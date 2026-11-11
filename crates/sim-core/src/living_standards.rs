//! Read-only indicators of modeled group means, not individual survey estimates.
use crate::{BasisPoints, Household, HouseholdGroupId, Money, Population};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LivingStandards {
    pub population: Population,
    pub working_age: Population,
    pub employed: Population,
    pub disposable_income: Money,
    pub essential_basket: Money,
    pub cash: Money,
    /// Whole groups whose realized daily income after tax is below their basket.
    /// Savings may still fund consumption: this is not a poverty headcount.
    pub income_below_basket: Population,
    /// Whole groups with modeled food delivery below the mandate's 90% threshold.
    pub food_below_target: Population,
    /// Population-weighted Gini of group-mean disposable income, in 0..=10000.
    /// Excludes within-group inequality. Undefined for zero total income,
    /// negative disposable income, or income assigned to a zero-person group.
    pub group_mean_gini_bps: Option<BasisPoints>,
}

pub fn living_standards(groups: &[Household]) -> Result<LivingStandards, String> {
    let mut result = LivingStandards {
        population: Population(0),
        working_age: Population(0),
        employed: Population(0),
        disposable_income: Money(0),
        essential_basket: Money(0),
        cash: Money(0),
        income_below_basket: Population(0),
        food_below_target: Population(0),
        group_mean_gini_bps: None,
    };
    let mut incomes = Vec::new();
    let overflow = || "Living-standard indicator arithmetic overflow".to_string();
    for group in groups {
        if [
            group.income.0,
            group.tax.0,
            group.basket_cost.0,
            group.cash.0,
        ]
        .into_iter()
        .any(|amount| amount < 0)
            || group.employed.0 > group.working_age.0
            || group.working_age.0 > group.population.0
        {
            return Err("Invalid household quantities for living-standard indicators".into());
        }
        let income = group
            .income
            .0
            .checked_sub(group.tax.0)
            .ok_or_else(overflow)?;
        result.population.0 = result
            .population
            .0
            .checked_add(group.population.0)
            .ok_or_else(overflow)?;
        result.working_age.0 = result
            .working_age
            .0
            .checked_add(group.working_age.0)
            .ok_or_else(overflow)?;
        result.employed.0 = result
            .employed
            .0
            .checked_add(group.employed.0)
            .ok_or_else(overflow)?;
        result.disposable_income.0 = result
            .disposable_income
            .0
            .checked_add(income)
            .ok_or_else(overflow)?;
        result.essential_basket.0 = result
            .essential_basket
            .0
            .checked_add(group.basket_cost.0)
            .ok_or_else(overflow)?;
        result.cash.0 = result
            .cash
            .0
            .checked_add(group.cash.0)
            .ok_or_else(overflow)?;
        if group.basket_cost.0 > 0 && income < group.basket_cost.0 {
            result.income_below_basket.0 = result
                .income_below_basket
                .0
                .checked_add(group.population.0)
                .ok_or_else(overflow)?;
        }
        if group.food_coverage < 90 {
            result.food_below_target.0 = result
                .food_below_target
                .0
                .checked_add(group.population.0)
                .ok_or_else(overflow)?;
        }
        incomes.push((i128::from(group.population.0), i128::from(income)));
    }
    if result.population.0 > 0
        && result.disposable_income.0 > 0
        && incomes
            .iter()
            .all(|&(people, income)| income >= 0 && (people > 0 || income == 0))
    {
        // p_i p_j |y_i/p_i - y_j/p_j| = |y_i p_j - y_j p_i|,
        // avoiding per-person division and rounding before aggregation.
        let mut numerator = 0_i128;
        for &(people, income) in &incomes {
            for &(other_people, other_income) in &incomes {
                let difference = income
                    .checked_mul(other_people)
                    .ok_or_else(overflow)?
                    .checked_sub(other_income.checked_mul(people).ok_or_else(overflow)?)
                    .ok_or_else(overflow)?
                    .checked_abs()
                    .ok_or_else(overflow)?;
                numerator = numerator.checked_add(difference).ok_or_else(overflow)?;
            }
        }
        let denominator = i128::from(result.population.0)
            .checked_mul(i128::from(result.disposable_income.0))
            .and_then(|value| value.checked_mul(2))
            .ok_or_else(overflow)?;
        let gini = numerator.checked_mul(10000).ok_or_else(overflow)? / denominator;
        result.group_mean_gini_bps = Some(BasisPoints::new(u32::from(
            u16::try_from(gini).map_err(|_| overflow())?,
        )));
    }
    Ok(result)
}

/// A boundary of the Lorenz curve, ordered by mean disposable income per resident.
/// Shares round down to whole basis points; the final boundary is exactly 100%.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncomeShareBoundary {
    pub group: HouseholdGroupId,
    pub population: Population,
    pub disposable_income: Money,
    pub cumulative_population: Population,
    pub cumulative_income: Money,
    pub population_share: BasisPoints,
    pub income_share: BasisPoints,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupIncomeDistribution {
    pub population: Population,
    pub disposable_income: Money,
    pub group_mean_gini_bps: BasisPoints,
    pub boundaries: Vec<IncomeShareBoundary>,
}

/// The same group means and undefined cases as `living_standards`, with no
/// assumption about individual income or inequality within a cohort.
pub fn group_income_distribution(
    groups: &[Household],
) -> Result<Option<GroupIncomeDistribution>, String> {
    let indicators = living_standards(groups)?;
    let Some(gini) = indicators.group_mean_gini_bps else {
        return Ok(None);
    };
    let mut identities = std::collections::BTreeSet::new();
    if groups.iter().any(|group| !identities.insert(&group.id)) {
        return Err("Income distribution requires distinct household group IDs".into());
    }
    let mut ordered: Vec<_> = groups
        .iter()
        .filter(|group| group.population.0 > 0)
        .collect();
    // The existing Gini calculation has checked these exact cross-products.
    // Sort rational means without per-person rounding; IDs resolve equal means.
    ordered.sort_by(|a, b| {
        (i128::from(a.income.0 - a.tax.0) * i128::from(b.population.0))
            .cmp(&(i128::from(b.income.0 - b.tax.0) * i128::from(a.population.0)))
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut people = 0_u64;
    let mut income = 0_i64;
    let mut boundaries = Vec::with_capacity(ordered.len());
    for group in ordered {
        let disposable = group.income.0 - group.tax.0;
        people = people
            .checked_add(group.population.0)
            .ok_or("Income distribution population overflow")?;
        income = income
            .checked_add(disposable)
            .ok_or("Income distribution cash overflow")?;
        boundaries.push(IncomeShareBoundary {
            group: group.id.clone(),
            population: group.population,
            disposable_income: Money(disposable),
            cumulative_population: Population(people),
            cumulative_income: Money(income),
            population_share: BasisPoints::new(
                (i128::from(people) * 10000 / i128::from(indicators.population.0)) as u32,
            ),
            income_share: BasisPoints::new(
                (i128::from(income) * 10000 / i128::from(indicators.disposable_income.0)) as u32,
            ),
        });
    }
    Ok(Some(GroupIncomeDistribution {
        population: indicators.population,
        disposable_income: indicators.disposable_income,
        group_mean_gini_bps: gini,
        boundaries,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn group(people: u64, income: i64) -> Household {
        Household {
            delivery: None,
            id: "test".into(),
            population: Population(people),
            working_age: Population(people),
            employed: Population(people),
            cash: Money(100),
            arrears: Money(0),
            income: Money(income),
            tax: Money(0),
            consumption: Money(0),
            basket_cost: Money(50),
            food_coverage: 100,
            approval: 50,
        }
    }
    #[test]
    fn weighted_group_mean_gini_handles_population_weights_and_cohort_splits() {
        assert_eq!(
            living_standards(&[group(1, 100), group(3, 300)])
                .unwrap()
                .group_mean_gini_bps,
            Some(BasisPoints::new(0))
        );
        let original = living_standards(&[group(1, 0), group(1, 100)]).unwrap();
        assert_eq!(original.group_mean_gini_bps, Some(BasisPoints::new(5000)));
        assert_eq!(
            living_standards(&[group(1, 100), group(1, 0)])
                .unwrap()
                .group_mean_gini_bps,
            original.group_mean_gini_bps
        );
        assert_eq!(
            living_standards(&[group(1, 0), group(1, 0), group(2, 200)])
                .unwrap()
                .group_mean_gini_bps,
            original.group_mean_gini_bps
        );
        assert_eq!(
            living_standards(&[group(0, 100), group(2, 200)])
                .unwrap()
                .group_mean_gini_bps,
            None
        );
        assert_eq!(
            living_standards(&[group(1, 0)])
                .unwrap()
                .group_mean_gini_bps,
            None
        );
    }
    #[test]
    fn income_pressure_is_distinct_from_savings_and_food_delivery() {
        let mut low_income = group(10, 20);
        low_income.cash = Money(1000);
        let mut scarce_food = group(30, 300);
        scarce_food.food_coverage = 60;
        let indicators = living_standards(&[low_income, scarce_food]).unwrap();
        assert_eq!(indicators.income_below_basket, Population(10));
        assert_eq!(indicators.food_below_target, Population(30));
        assert_eq!(indicators.cash, Money(1100));
        assert_eq!(indicators.population, Population(40));
    }
    #[test]
    fn invalid_and_overflowing_aggregates_fail_without_wrapping() {
        assert!(living_standards(&[group(1, i64::MAX), group(1, i64::MAX)]).is_err());
        assert!(living_standards(&[group(u64::MAX, 1), group(1, 1)]).is_err());
        assert!(living_standards(&[group(1, -1)]).is_err());
    }

    fn identified(id: &str, people: u64, income: i64) -> Household {
        let mut result = group(people, income);
        result.id = id.into();
        result
    }
    #[test]
    fn lorenz_boundaries_use_population_weights_and_reconcile_with_existing_gini() {
        let groups = [identified("rich", 1, 100), identified("poor", 3, 0)];
        let before = groups.clone();
        let curve = group_income_distribution(&groups).unwrap().unwrap();
        assert_eq!(curve.group_mean_gini_bps, BasisPoints::new(7500));
        assert_eq!(curve.boundaries[0].group.0, "poor");
        assert_eq!(curve.boundaries[0].population_share, BasisPoints::new(7500));
        assert_eq!(curve.boundaries[0].income_share, BasisPoints::new(0));
        let last = curve.boundaries.last().unwrap();
        assert_eq!(last.population_share, BasisPoints::new(10000));
        assert_eq!(last.income_share, BasisPoints::new(10000));
        assert_eq!(last.cumulative_population, curve.population);
        assert_eq!(last.cumulative_income, curve.disposable_income);
        // Twice the exact area below the curve is 1/4; Gini is 1 - 1/4.
        let mut prior_income = 0_i128;
        let area: i128 = curve
            .boundaries
            .iter()
            .map(|point| {
                let next = i128::from(point.cumulative_income.0);
                let area = i128::from(point.population.0) * (prior_income + next);
                prior_income = next;
                area
            })
            .sum();
        assert_eq!(
            10000
                - area * 10000
                    / (i128::from(curve.population.0) * i128::from(curve.disposable_income.0)),
            i128::from(curve.group_mean_gini_bps.units())
        );
        assert_eq!(groups, before);
    }
    #[test]
    fn lorenz_order_is_stable_for_equal_means_and_zero_person_groups_are_not_boundaries() {
        let groups = [
            identified("z", 3, 300),
            identified("empty", 0, 0),
            identified("a", 1, 100),
        ];
        let curve = group_income_distribution(&groups).unwrap().unwrap();
        assert_eq!(curve.group_mean_gini_bps, BasisPoints::new(0));
        assert_eq!(
            curve
                .boundaries
                .iter()
                .map(|p| p.group.0.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "z"]
        );
        assert!(
            curve
                .boundaries
                .iter()
                .all(|p| p.population_share == p.income_share)
        );
        let mut reversed = groups.clone();
        reversed.reverse();
        assert_eq!(group_income_distribution(&reversed).unwrap(), Some(curve));
    }
    #[test]
    fn undefined_and_invalid_distribution_inputs_never_invent_shares() {
        assert!(group_income_distribution(&[]).unwrap().is_none());
        assert!(
            group_income_distribution(&[identified("a", 1, 0)])
                .unwrap()
                .is_none()
        );
        assert!(
            group_income_distribution(&[identified("a", 0, 100)])
                .unwrap()
                .is_none()
        );
        let mut negative = identified("a", 1, 10);
        negative.tax = Money(20);
        assert!(group_income_distribution(&[negative]).unwrap().is_none());
        assert!(
            group_income_distribution(&[identified("a", 1, i64::MAX), identified("b", 1, 1)])
                .is_err()
        );
        assert!(
            group_income_distribution(&[identified("a", 1, 10), identified("a", 1, 20)]).is_err()
        );
    }
}
