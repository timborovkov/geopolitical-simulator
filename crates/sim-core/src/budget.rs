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
    pub industry_upkeep: Option<IndustryUpkeepProjection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest_accrual: Option<InterestProjection>,
    pub daily_revenue: Money,
    pub daily_spending: Money,
    /// With accrual, whole dues on the next settlement using the opening carry.
    pub daily_interest: Money,
}
impl BudgetForecast {
    /// Fixed daily assumptions plus exact carried industrial upkeep, avoiding
    /// multiplication of a rounded one-day installment.
    pub fn spending_for(&self, days: DaySpan) -> Money {
        let base = self.daily_spending.0 * i64::from(days.units());
        self.industry_upkeep.as_ref().map_or(Money(base), |p| {
            Money(
                base - p.due(DaySpan::new(1)).expect("valid industry projection").0
                    * i64::from(days.units())
                    + p.due(days).expect("valid industry projection").0,
            )
        })
    }
    pub fn valid(&self) -> bool {
        [
            self.daily_revenue.0,
            self.daily_spending.0,
            self.daily_interest.0,
        ]
        .iter()
        .all(|v| (0..=1_000_000_000).contains(v))
            && self.industry_upkeep.as_ref().is_none_or(|p| {
                p.valid()
                    && p.due(DaySpan::new(1))
                        .is_ok_and(|due| due.0 <= self.daily_spending.0)
            })
            && self.interest_accrual.as_ref().is_none_or(|p| {
                p.valid()
                    && self.daily_interest == p.due(crate::DaySpan::new(1)).unwrap_or(Money(-1))
            })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetPeriod {
    pub start_day: crate::CampaignDay,
    pub end_day: crate::CampaignDay,
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
            && self.end_day <= crate::CampaignDay::new(MAX_DAY + 366)
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
    pub fn expected_revenue(&self, day: crate::CampaignDay) -> i64 {
        let day = day.units();
        let days = match self.opening_day_settled {
            None => day
                .min((self.end_day).units())
                .saturating_sub((self.start_day).units()),
            Some(included) => {
                day.min((self.end_day).units() - 1)
                    .saturating_sub((self.start_day).units())
                    + u32::from(included && day >= (self.start_day).units())
            }
        };
        self.forecast.daily_revenue.0 * i64::from(days)
    }
    pub fn forecast_closing_cash(&self) -> i64 {
        let days = (self.end_day).units()
            - (self.start_day).units()
            - u32::from(self.opening_day_settled == Some(false));
        if let Some(interest) = &self.forecast.interest_accrual {
            return self.opening_cash.0 + i64::from(days) * self.forecast.daily_revenue.0
                - self.forecast.spending_for(DaySpan::new(days)).0
                - interest
                    .due(crate::DaySpan::new(days))
                    .expect("validated budget projection")
                    .0;
        }
        self.opening_cash.0
            + i64::from(
                (self.end_day).units()
                    - (self.start_day).units()
                    - u32::from(self.opening_day_settled == Some(false)),
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
        let daily = d.gdp.units() / 365;
        let output = daily * i64::from((self.energy_capacity + 25).min(100)) / 100;
        apportion(
            output * 70 / 100,
            &d.households
                .iter()
                .map(|h| h.income_share)
                .collect::<Vec<_>>(),
        )
    }
    pub(crate) fn population_income_weights(&self, d: &CountryDefinition) -> Vec<i64> {
        let opening = crate::households::initial_households(d, Money(0));
        d.households
            .iter()
            .zip(opening)
            .map(|(profile, opening)| {
                let population = self.household(&profile.id).expect("household").population.0;
                if population == 0 || profile.income_share == 0 {
                    return 0;
                }
                let weight = u128::from(population) * profile.income_share as u128 * 1_000_000
                    / u128::from(opening.population.0.max(1));
                weight.clamp(1, 1_000_000_000_000) as i64
            })
            .collect()
    }
    pub(crate) fn forecast_private_income_for_content(
        &self,
        d: &CountryDefinition,
        content: &Content,
    ) -> Vec<i64> {
        if content.demographics.is_none() {
            return self.forecast_private_income(d);
        }
        if self.population.0 == 0 {
            return vec![0; self.households.len()];
        }
        let imports = content
            .markets
            .as_ref()
            .expect("markets")
            .good(GoodKind::Energy)
            .import_limit_bps
            .units();
        let supply_bps = i128::from(self.energy_capacity) * 100 + i128::from(imports);
        let coverage =
            (supply_bps * i128::from(d.population.0) / i128::from(self.population.0)).min(10000);
        let output = i128::from(d.gdp.units() / 365) * coverage / 10000;
        let weights = self.population_income_weights(d);
        if weights.iter().all(|w| *w == 0) {
            return vec![0; weights.len()];
        }
        apportion((output * 70 / 100) as i64, &weights)
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
                (operating * i64::from(INTEREST_DENOMINATOR) - interest.daily_numerator as i64
                    + forecast
                        .industry_upkeep
                        .as_ref()
                        .map_or(0, |p| p.rounding_adjustment()))
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
        self.forecast_inner(d, taxes, None)
    }
    /// Uses explicit content rules for dynamic resident stocks; the preserved
    /// two-argument API retains the fixed-population forecast.
    pub fn forecast_for_content(&self, d: &CountryDefinition, content: &Content) -> BudgetForecast {
        self.forecast_inner(d, content.income_taxes.as_ref(), Some(content))
    }
    fn forecast_inner(
        &self,
        d: &CountryDefinition,
        taxes: Option<&IncomeTaxDefinition>,
        content: Option<&Content>,
    ) -> BudgetForecast {
        let dynamic = content.is_some_and(|c| c.demographics.is_some());
        let daily = d.gdp.units() / 365;
        let income = content.map_or_else(
            || self.forecast_private_income(d),
            |c| self.forecast_private_income_for_content(d, c),
        );
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
                        t.assess_for_population(
                            d,
                            self.households[index].population,
                            Money(*amount),
                            self.tax,
                            self.administration,
                            dynamic,
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
        let industry_upkeep = self.industry.as_ref().map(|s| s.upkeep_projection(d.gdp));
        let industry_due = industry_upkeep.as_ref().map_or(0, |p| {
            p.due(DaySpan::new(1)).expect("validated industry upkeep").0
        });
        BudgetForecast {
            industry_upkeep,
            daily_interest: interest_accrual.as_ref().map_or_else(
                || crate::interest::minimum_daily_interest(self.debt),
                |p| {
                    p.due(crate::DaySpan::new(1))
                        .expect("bounded daily projection")
                },
            ),
            interest_accrual,
            daily_revenue: Money(revenue),
            daily_spending: Money(
                if !dynamic
                    || self
                        .households
                        .iter()
                        .any(|h| h.id == "public" && h.population.0 > 0)
                {
                    daily_policy_spending(Money(daily), self.services).0
                } else {
                    0
                } + if !dynamic
                    || self
                        .households
                        .iter()
                        .any(|h| (h.id == "workers" || h.id == "vulnerable") && h.population.0 > 0)
                {
                    daily_policy_spending(Money(daily), self.welfare).0
                } else {
                    0
                } + maintenance
                    + industry_due
                    + self.regional_planned_spending(d),
            ),
        }
    }
}
pub(crate) fn next_fiscal_end(day: crate::CampaignDay) -> crate::CampaignDay {
    let day = day.units();
    let year = date(crate::CampaignDay::new(day))[..4]
        .parse::<u32>()
        .expect("year");
    crate::CampaignDay::new(
        (2000..=year)
            .map(|y| {
                if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                    366
                } else {
                    365
                }
            })
            .sum::<u32>(),
    )
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
            forecast: c.forecast_for_content(d, content),
            revenue: Money(0),
            payments: Money(0),
            capital: Money(0),
            borrowing: Money(0),
            principal_repaid: Money(0),
        };
        self.event(id, true, message);
    }
}
