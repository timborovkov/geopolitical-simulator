//! Read-only indicators of modeled group means, not individual survey estimates.
use crate::{Household, Money, Population};

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
    pub group_mean_gini_bps: Option<u16>,
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
        result.group_mean_gini_bps = Some(u16::try_from(gini).map_err(|_| overflow())?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn group(people: u64, income: i64) -> Household {
        Household {
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
            Some(0)
        );
        let original = living_standards(&[group(1, 0), group(1, 100)]).unwrap();
        assert_eq!(original.group_mean_gini_bps, Some(5000));
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
}
