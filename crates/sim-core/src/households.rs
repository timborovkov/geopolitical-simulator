//! Disjoint household groups: cash is a partition of the household-sector account.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseholdDefinition {
    pub id: String,
    pub name: String,
    pub population_share: i64,
    pub income_share: i64,
    pub savings_share: i64,
    pub working_age_share: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Household {
    pub id: String,
    pub population: Population,
    pub working_age: Population,
    pub employed: Population,
    pub cash: Money,
    pub arrears: Money,
    pub income: Money,
    pub tax: Money,
    pub consumption: Money,
    pub basket_cost: Money,
    pub food_coverage: i32,
    pub approval: i32,
}
/// Largest remainder allocation. Exact sum; stable input order breaks ties.
pub fn apportion(total: i64, weights: &[i64]) -> Vec<i64> {
    let sum: i64 = weights.iter().sum();
    assert!(total >= 0 && sum > 0 && weights.iter().all(|w| *w >= 0));
    let mut values: Vec<i64> = weights
        .iter()
        .map(|w| ((i128::from(total) * i128::from(*w)) / i128::from(sum)) as i64)
        .collect();
    let mut order: Vec<_> = (0..weights.len()).collect();
    order.sort_by_key(|i| {
        std::cmp::Reverse((i128::from(total) * i128::from(weights[*i])) % i128::from(sum))
    });
    let remainder = total - values.iter().sum::<i64>();
    for i in order.into_iter().take(remainder as usize) {
        values[i] += 1;
    }
    values
}
pub(crate) fn initial_households(d: &CountryDefinition, cash: Money) -> Vec<Household> {
    let population = apportion(
        d.population as i64,
        &d.households
            .iter()
            .map(|h| h.population_share)
            .collect::<Vec<_>>(),
    );
    let savings = apportion(
        cash.0,
        &d.households
            .iter()
            .map(|h| h.savings_share)
            .collect::<Vec<_>>(),
    );
    d.households
        .iter()
        .enumerate()
        .map(|(i, h)| Household {
            id: h.id.clone(),
            population: Population(population[i] as u64),
            working_age: Population((population[i] * h.working_age_share / 100) as u64),
            employed: Population((population[i] * h.working_age_share * 90 / 10000) as u64),
            cash: Money(savings[i]),
            arrears: Money(0),
            income: Money(0),
            tax: Money(0),
            consumption: Money(0),
            basket_cost: Money(0),
            food_coverage: 100,
            approval: d.approval,
        })
        .collect()
}
impl World {
    pub(crate) fn pay_income(
        &mut self,
        id: &CountryId,
        from: Account,
        amount: i64,
        weights: &[i64],
        reason: &str,
    ) -> i64 {
        self.transfer(id, from, Account::Households, amount, reason, Some(weights))
    }
    pub(crate) fn pay_group(
        &mut self,
        id: &CountryId,
        index: usize,
        to: Account,
        amount: i64,
        reason: &str,
    ) -> i64 {
        let c = self.countries.get_mut(id).expect("country");
        let paid = c.accounts.transfer(
            Account::Households,
            to,
            amount.max(0).min(c.households[index].cash.0),
        );
        c.households[index].cash.0 -= paid;
        if paid > 0 {
            self.ledger.push(LedgerEntry {
                day: self.day,
                country: id.clone(),
                from: Account::Households,
                to,
                amount: Money(paid),
                reason: format!("{}: {reason}", c.households[index].id),
            });
        }
        paid
    }
    pub(crate) fn settle_households(
        &mut self,
        content: &Content,
        id: &CountryId,
        food_available: Food,
        demand: Food,
    ) -> (Money, Food) {
        let d = content.country(id).expect("definition");
        let weights: Vec<_> = self.countries[id]
            .households
            .iter()
            .map(|h| h.population.0 as i64)
            .collect();
        let baskets = apportion((d.gdp / 365 * 60 / 100).max(1), &weights);
        let needs = apportion(demand.units(), &weights);
        let mut affordable = Vec::new();
        let mut consumption = 0;
        for (index, basket) in baskets.iter().enumerate() {
            let h = &self.countries[id].households[index];
            let desired = (*basket).max((h.income.0 - h.tax.0) * 90 / 100).max(1);
            let paid = self.pay_group(
                id,
                index,
                Account::Firms,
                desired,
                "Household consumption basket",
            );
            let c = self.countries.get_mut(id).expect("country");
            c.households[index].consumption = Money(paid);
            c.households[index].basket_cost = Money(*basket);
            affordable.push(if *basket == 0 {
                needs[index]
            } else {
                needs[index] * paid.min(*basket) / basket
            });
            consumption += paid;
        }
        let total_affordable: i64 = affordable.iter().sum();
        let delivered =
            food_available.min(Food::new(total_affordable).expect("nonnegative affordable demand"));
        let allocated = if total_affordable > 0 {
            apportion(delivered.units(), &affordable)
        } else {
            vec![0; affordable.len()]
        };
        for (i, h) in self
            .countries
            .get_mut(id)
            .expect("country")
            .households
            .iter_mut()
            .enumerate()
        {
            h.food_coverage = if needs[i] == 0 {
                100
            } else {
                (allocated[i] * 100 / needs[i]) as i32
            };
        }
        (Money(consumption), delivered)
    }
}
