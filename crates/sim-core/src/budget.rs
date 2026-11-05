//! Fiscal projections are fixed assumptions, distinct from realized cash execution.
use crate::*;
use serde::{Deserialize, Serialize};
/// Preserve the existing daily-GDP rounding boundary before applying a policy rate.
pub(crate) fn daily_policy_spending(daily_gdp: Money, level: i32) -> Money {
    Percent::try_from(level)
        .expect("validated spending level")
        .scale(daily_gdp)
        .expect("bounded daily policy spending")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetForecast {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest_accrual: Option<InterestProjection>,
    pub daily_revenue: Money,
    pub daily_spending: Money,
    /// With accrual, whole dues on the next settlement using the opening carry.
    pub daily_interest: Money,
}
impl BudgetForecast {
    pub fn valid(&self) -> bool {
        [
            self.daily_revenue.0,
            self.daily_spending.0,
            self.daily_interest.0,
        ]
        .iter()
        .all(|v| (0..=1_000_000_000).contains(v))
            && self
                .interest_accrual
                .as_ref()
                .is_none_or(|p| p.valid() && self.daily_interest == p.due(1).unwrap_or(Money(-1)))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetPeriod {
    pub start_day: u32,
    pub end_day: u32,
    pub opening_cash: Money,
    /// Calendar execution: initial opening has no flow; later years settle January 1.
    /// None preserves the earlier elapsed-period accounting convention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening_day_settled: Option<bool>,
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
            && self.forecast.valid()
    }
    pub fn expected_cash(&self) -> i64 {
        self.opening_cash.0 + self.revenue.0 + self.borrowing.0
            - self.payments.0
            - self.capital.0
            - self.principal_repaid.0
    }
    pub fn expected_revenue(&self, day: u32) -> i64 {
        let days = match self.opening_day_settled {
            None => day.min(self.end_day).saturating_sub(self.start_day),
            Some(included) => {
                day.min(self.end_day - 1).saturating_sub(self.start_day)
                    + u32::from(included && day >= self.start_day)
            }
        };
        self.forecast.daily_revenue.0 * i64::from(days)
    }
    pub fn forecast_closing_cash(&self) -> i64 {
        let days =
            self.end_day - self.start_day - u32::from(self.opening_day_settled == Some(false));
        if let Some(interest) = &self.forecast.interest_accrual {
            return self.opening_cash.0
                + i64::from(days)
                    * (self.forecast.daily_revenue.0 - self.forecast.daily_spending.0)
                - interest.due(days).expect("validated budget projection").0;
        }
        self.opening_cash.0
            + i64::from(
                self.end_day - self.start_day - u32::from(self.opening_day_settled == Some(false)),
            ) * (self.forecast.daily_revenue.0
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
    /// Display label for the forecast; this field is not an entity selector.
    pub group: String,
    pub daily_disposable_change: Money,
}
impl Country {
    pub(crate) fn forecast_private_income(&self, d: &CountryDefinition) -> Vec<i64> {
        let daily = d.gdp / 365;
        let output = daily * i64::from((self.energy_capacity + 25).min(100)) / 100;
        apportion(
            output * 70 / 100,
            &d.households
                .iter()
                .map(|h| h.income_share)
                .collect::<Vec<_>>(),
        )
    }
    pub(crate) fn national_daily_balance(
        &self,
        d: &CountryDefinition,
        forecast: &BudgetForecast,
    ) -> Money {
        let operating = forecast.daily_revenue.0
            - self.regional_tax_entitlement(forecast.daily_revenue.0)
            - forecast.daily_spending.0
            + self.regional_planned_spending(d);
        if let Some(interest) = &forecast.interest_accrual {
            Money(
                (operating * i64::from(INTEREST_DENOMINATOR) - interest.daily_numerator as i64)
                    .div_euclid(i64::from(INTEREST_DENOMINATOR)),
            )
        } else {
            Money(operating - forecast.daily_interest.0)
        }
    }
    /// Conditional unchanged-policy forecast using this campaign's tax definition.
    pub fn forecast(
        &self,
        d: &CountryDefinition,
        taxes: Option<&IncomeTaxDefinition>,
    ) -> BudgetForecast {
        let daily = d.gdp / 365;
        let income = self.forecast_private_income(d);
        let revenue = income
            .iter()
            .enumerate()
            .map(|(index, amount)| {
                taxes.map_or_else(
                    || {
                        crate::income_taxes::flat_withholding(
                            Money(*amount),
                            self.tax,
                            self.administration,
                        )
                        .0
                    },
                    |t| {
                        t.assess(
                            d,
                            self.households[index].population,
                            Money(*amount),
                            self.tax,
                            self.administration,
                        )
                        .expect("validated tax definition and bounded forecast")
                        .collectible
                        .0
                    },
                )
            })
            .sum();
        let maintenance = daily
            * i64::from(
                (self.energy_capacity - self.opening_energy_capacity).max(0)
                    + (self.food_capacity - self.opening_food_capacity).max(0),
            )
            / 15
            / 1000;
        let interest_accrual = self
            .financing
            .as_ref()
            .and_then(|f| f.interest_accrual.as_ref())
            .map(|state| {
                InterestProjection::new(self.debt, state.remainder)
                    .expect("validated principal and interest remainder")
            });
        BudgetForecast {
            daily_interest: interest_accrual.as_ref().map_or_else(
                || crate::interest::minimum_daily_interest(self.debt),
                |p| p.due(1).expect("bounded daily projection"),
            ),
            interest_accrual,
            daily_revenue: Money(revenue),
            daily_spending: Money(
                daily_policy_spending(Money(daily), self.services).0
                    + daily_policy_spending(Money(daily), self.welfare).0
                    + maintenance
                    + self.regional_planned_spending(d),
            ),
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
            opening_day_settled: content.fiscal_plans.as_ref().map(|_| true),
            forecast: c.forecast(d, content.income_taxes.as_ref()),
            revenue: Money(0),
            payments: Money(0),
            capital: Money(0),
            borrowing: Money(0),
            principal_repaid: Money(0),
        };
        self.event(id, true, message);
    }
}
