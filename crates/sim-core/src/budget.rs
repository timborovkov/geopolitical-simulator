//! Fiscal projections are fixed assumptions, distinct from realized cash execution.
use crate::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetForecast {
    pub daily_revenue: Money,
    pub daily_spending: Money,
    pub daily_interest: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetPeriod {
    pub start_day: u32,
    pub end_day: u32,
    pub opening_cash: Money,
    pub forecast: BudgetForecast,
    pub revenue: Money,
    pub payments: Money,
    pub capital: Money,
    pub borrowing: Money,
    pub principal_repaid: Money,
}
impl BudgetPeriod {
    pub fn valid_ranges(&self) -> bool {
        self.start_day < self.end_day
            && self.end_day <= MAX_DAY + 366
            && [
                self.opening_cash.0,
                self.revenue.0,
                self.payments.0,
                self.capital.0,
                self.borrowing.0,
                self.principal_repaid.0,
            ]
            .iter()
            .all(|v| (0..=1_000_000_000_000).contains(v))
            && [
                self.forecast.daily_revenue.0,
                self.forecast.daily_spending.0,
                self.forecast.daily_interest.0,
            ]
            .iter()
            .all(|v| (0..=1_000_000_000).contains(v))
    }
    pub fn expected_cash(&self) -> i64 {
        self.opening_cash.0 + self.revenue.0 + self.borrowing.0
            - self.payments.0
            - self.capital.0
            - self.principal_repaid.0
    }
    pub fn expected_revenue(&self, day: u32) -> i64 {
        self.forecast.daily_revenue.0 * i64::from(day.min(self.end_day) - self.start_day)
    }
    pub fn forecast_closing_cash(&self) -> i64 {
        self.opening_cash.0
            + i64::from(self.end_day - self.start_day)
                * (self.forecast.daily_revenue.0
                    - self.forecast.daily_spending.0
                    - self.forecast.daily_interest.0)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetReview {
    pub period: BudgetPeriod,
    pub closing_cash: Money,
    pub arrears: Money,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseholdImpact {
    pub group: String,
    pub daily_disposable_change: Money,
}
impl Country {
    pub fn forecast(&self, d: &CountryDefinition) -> BudgetForecast {
        let daily = d.gdp / 365;
        let output = daily * i64::from((self.energy_capacity + 25).min(100)) / 100;
        let income = apportion(
            output * 70 / 100,
            &d.households
                .iter()
                .map(|h| h.income_share)
                .collect::<Vec<_>>(),
        );
        let revenue = income
            .iter()
            .map(|amount| amount * i64::from(self.tax) * i64::from(self.administration) / 10000)
            .sum();
        let maintenance = daily
            * i64::from(
                (self.energy_capacity - self.opening_energy_capacity).max(0)
                    + (self.food_capacity - self.opening_food_capacity).max(0),
            )
            / 15
            / 1000;
        BudgetForecast {
            daily_revenue: Money(revenue),
            daily_spending: Money(
                daily * i64::from(self.services) / 100
                    + daily * i64::from(self.welfare) / 100
                    + maintenance,
            ),
            daily_interest: Money((self.debt.0 * 4 / 100 / 365).max(i64::from(self.debt.0 > 0))),
        }
    }
}
pub(crate) fn next_fiscal_end(day: u32) -> u32 {
    let year = date(day)[..4].parse::<u32>().expect("year");
    (2000..=year)
        .map(|y| {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                366
            } else {
                365
            }
        })
        .sum()
}
impl World {
    pub(crate) fn review_budget(&mut self, content: &Content, id: &CountryId) {
        if self.countries[id].budget.end_day != self.day {
            return;
        }
        let c = self.countries.get_mut(id).expect("country");
        let d = content.country(id).expect("definition");
        c.budget_reviews.push(BudgetReview {
            period: c.budget.clone(),
            closing_cash: c.accounts.treasury,
            arrears: c.arrears,
        });
        if c.budget_reviews.len() > 10 {
            c.budget_reviews.remove(0);
        }
        let variance = c.budget.revenue.0 - c.budget.expected_revenue(self.day);
        let message = format!(
            "Fiscal year closed: revenue {}m ({variance:+}m against opening forecast), payments {}m, capital {}m, closing cash {}m, arrears {}m. Variance reflects supply, collection, cash-constrained wages and policy changes; financing is shown separately.",
            c.budget.revenue.0,
            c.budget.payments.0,
            c.budget.capital.0,
            c.accounts.treasury.0,
            c.arrears.0
        );
        c.budget = BudgetPeriod {
            start_day: self.day,
            end_day: next_fiscal_end(self.day),
            opening_cash: c.accounts.treasury,
            forecast: c.forecast(d),
            revenue: Money(0),
            payments: Money(0),
            capital: Money(0),
            borrowing: Money(0),
            principal_repaid: Money(0),
        };
        self.event(id, true, message);
    }
}
