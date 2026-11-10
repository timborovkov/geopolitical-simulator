//! Batch household clearing: planned cash and affordable demand precede physical
//! allocation; businesses receive only actual food charges and the abstract other basket.
use crate::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsumptionDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
}
impl ConsumptionDefinition {
    pub(crate) fn validate(&self, content: &Content) -> Result<(), String> {
        if self.source_version.is_empty()
            || self.description.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("Delivered consumption requires attributed assumptions".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseholdConsumption {
    pub day: CampaignDay,
    pub desired_spending: Money,
    pub planned_cash: Money,
    pub food_budget: Money,
    pub food_paid: Money,
    pub other_paid: Money,
    pub cash_kept: Money,
    pub food_need: Food,
    pub affordable_food: Food,
    pub delivered_food: Food,
}
impl HouseholdConsumption {
    pub(crate) fn empty(day: CampaignDay) -> Self {
        Self {
            day,
            desired_spending: Money(0),
            planned_cash: Money(0),
            food_budget: Money(0),
            food_paid: Money(0),
            other_paid: Money(0),
            cash_kept: Money(0),
            food_need: Food::ZERO,
            affordable_food: Food::ZERO,
            delivered_food: Food::ZERO,
        }
    }
}
fn quoted_food_budget(essential_cash: i64, share: u32, price: u32) -> i64 {
    let food = i128::from(share) * i128::from(price);
    let total = i128::from(10000 - share) * 10000 + food;
    (i128::from(essential_cash) * food / total) as i64
}
fn charged_food(budget: i64, affordable: i64, delivered: i64) -> i64 {
    if affordable == 0 {
        0
    } else {
        (i128::from(budget) * i128::from(delivered) / i128::from(affordable)) as i64
    }
}
impl World {
    pub(crate) fn settle_delivered_consumption(
        &mut self,
        content: &Content,
        id: &CountryId,
        supply: Food,
        baskets: &[i64],
        needs: &[i64],
    ) -> (Money, Food, Food) {
        let definition = content.markets.as_ref().expect("configured markets");
        let price = self.countries[id]
            .markets
            .as_ref()
            .expect("markets")
            .food
            .next_price_bps
            .units();
        let mut plans: Vec<_> = self.countries[id]
            .households
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let mut p = HouseholdConsumption::empty(self.day);
                if h.population.0 == 0 {
                    return p;
                }
                p.desired_spending =
                    Money(baskets[i].max((h.income.0 - h.tax.0) * 90 / 100).max(1));
                p.planned_cash = Money(p.desired_spending.0.min(h.cash.0));
                p.food_budget = Money(quoted_food_budget(
                    p.planned_cash.0.min(baskets[i]),
                    definition.food_budget_share_bps.units(),
                    price,
                ));
                p.food_need = Food::new(needs[i]).expect("needs");
                p.affordable_food = Food::new(if baskets[i] == 0 {
                    needs[i]
                } else {
                    (i128::from(needs[i]) * i128::from(p.planned_cash.0.min(baskets[i]))
                        / i128::from(baskets[i])) as i64
                })
                .expect("affordable need");
                p
            })
            .collect();
        let affordable: Vec<_> = plans.iter().map(|p| p.affordable_food.units()).collect();
        let total: i64 = affordable.iter().sum();
        let delivered = supply.units().min(total);
        let allocation = if total == 0 {
            vec![0; plans.len()]
        } else {
            apportion(delivered, &affordable)
        };
        let mut paid = 0;
        for (i, p) in plans.iter_mut().enumerate() {
            p.delivered_food = Food::new(allocation[i]).expect("delivery");
            p.food_paid = Money(charged_food(
                p.food_budget.0,
                p.affordable_food.units(),
                allocation[i],
            ));
            p.other_paid = Money(p.planned_cash.0 - p.food_budget.0);
            let actual = self.pay_group(
                id,
                i,
                Account::Firms,
                p.food_paid.0 + p.other_paid.0,
                "Household basket: delivered food and other consumption",
            );
            p.cash_kept = Money(p.planned_cash.0 - actual);
            paid += actual;
            let h = &mut self.countries.get_mut(id).expect("country").households[i];
            h.basket_cost = Money(baskets[i]);
            h.consumption = Money(actual);
            h.food_coverage = if needs[i] == 0 {
                100
            } else {
                (allocation[i] * 100 / needs[i]) as i32
            };
            h.delivery = Some(p.clone());
        }
        (
            Money(paid),
            Food::new(delivered).expect("delivered"),
            Food::new(total).expect("affordable"),
        )
    }
    pub(crate) fn validate_consumption(&self, content: &Content) -> Result<(), String> {
        for id in self.countries.keys() {
            self.validate_country_consumption(content, id)?;
        }
        Ok(())
    }
    pub(crate) fn validate_country_consumption(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = self
            .countries
            .get(id)
            .ok_or("Unknown consumption country")?;
        if self.rules >= 36 {
            let rules = content
                .markets
                .as_ref()
                .ok_or("Missing consumption quote definition")?;
            let quote = c
                .markets
                .as_ref()
                .ok_or("Missing consumption quote")?
                .food
                .next_price_bps
                .units();
            let food = rules.good(GoodKind::Food);
            if !(food.minimum_price_bps.units()..=food.maximum_price_bps.units()).contains(&quote) {
                return Err("Invalid next consumption quote".into());
            }
        }
        if self.rules < 36 || self.day == content.start_day()? {
            return if c.households.iter().all(|h| h.delivery.is_none()) {
                Ok(())
            } else {
                Err("Unexpected delivered-consumption receipt".into())
            };
        }
        let markets = content
            .markets
            .as_ref()
            .ok_or("Missing consumption market definition")?;
        let last = c
            .markets
            .as_ref()
            .and_then(|m| m.food.last.as_ref())
            .ok_or("Missing settled consumption market")?;
        if last.day != self.day || !(1..=100000).contains(&last.price_bps.units()) {
            return Err("Invalid consumption settlement date or quote".into());
        }
        if c.households.len()
            != content
                .country(id)
                .ok_or("Missing consumption definition")?
                .households
                .len()
            || c.households
                .iter()
                .any(|h| h.population.0 > 1_000_000_000_000)
            || [
                last.physical_need,
                last.requested_use,
                last.delivered,
                last.supply,
            ]
            .iter()
            .any(|q| !(0..=1_000_000_000_000).contains(&q.units()))
        {
            return Err("Invalid consumption quantities or cohort partition".into());
        }
        let weights: Vec<_> = c.households.iter().map(|h| h.population.0 as i64).collect();
        let needs = if weights.iter().all(|w| *w == 0) {
            vec![0; weights.len()]
        } else {
            apportion(last.physical_need.units(), &weights)
        };
        let mut affordable = Vec::new();
        let mut delivered = Vec::new();
        let mut cash = 0_i128;
        for (i, h) in c.households.iter().enumerate() {
            let p = h
                .delivery
                .as_ref()
                .ok_or("Missing household consumption receipt")?;
            if [p.food_need, p.affordable_food, p.delivered_food]
                .iter()
                .any(|q| !(0..=1_000_000_000_000).contains(&q.units()))
            {
                return Err("Invalid household food quantities".into());
            }
            if p.day != self.day
                || [
                    h.cash,
                    h.income,
                    h.tax,
                    h.basket_cost,
                    h.consumption,
                    p.desired_spending,
                    p.planned_cash,
                    p.food_budget,
                    p.food_paid,
                    p.other_paid,
                    p.cash_kept,
                ]
                .iter()
                .any(|m| !(0..=1_000_000_000_000).contains(&m.0))
                || h.tax.0 > h.income.0
            {
                return Err("Invalid household consumption money or date".into());
            }
            let desired = if h.population.0 == 0 {
                0
            } else {
                h.basket_cost
                    .0
                    .max((h.income.0 - h.tax.0) * 90 / 100)
                    .max(1)
            };
            let budget = desired.min(h.cash.0 + h.consumption.0);
            let food_budget = quoted_food_budget(
                budget.min(h.basket_cost.0),
                markets.food_budget_share_bps.units(),
                last.price_bps.units(),
            );
            let requested = if h.population.0 == 0 {
                0
            } else if h.basket_cost.0 == 0 {
                needs[i]
            } else {
                (i128::from(needs[i]) * i128::from(budget.min(h.basket_cost.0))
                    / i128::from(h.basket_cost.0)) as i64
            };
            if p.desired_spending.0 != desired
                || p.planned_cash.0 != budget
                || p.food_budget.0 != food_budget
                || p.food_need.units() != needs[i]
                || p.affordable_food.units() != requested
                || p.delivered_food.units() > requested
                || p.food_paid.0 != charged_food(food_budget, requested, p.delivered_food.units())
                || p.other_paid.0 != budget - food_budget
                || p.cash_kept.0 != food_budget - p.food_paid.0
                || h.consumption.0 != p.food_paid.0 + p.other_paid.0
                || h.food_coverage
                    != if needs[i] == 0 {
                        100
                    } else {
                        (p.delivered_food.units() * 100 / needs[i]) as i32
                    }
            {
                return Err("Household demand, delivery and cash do not reconcile".into());
            }
            affordable.push(requested);
            delivered.push(p.delivered_food.units());
            cash += i128::from(h.consumption.0);
        }
        let total: i64 = affordable.iter().sum();
        let expected = if total == 0 {
            vec![0; affordable.len()]
        } else {
            apportion(last.delivered.units(), &affordable)
        };
        if affordable.iter().sum::<i64>() != last.requested_use.units()
            || delivered != expected
            || cash != i128::from(c.report.consumption.0)
        {
            return Err("Country consumption does not reconcile to settled food and cash".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HouseholdConsumptionLine {
    pub group: HouseholdGroupId,
    pub name: String,
    pub receipt: HouseholdConsumption,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsumptionDetail {
    pub groups: Vec<HouseholdConsumptionLine>,
    pub food_supply: Option<Food>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsumptionReport {
    pub country: CountryId,
    pub day: CampaignDay,
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned: Option<ConsumptionDetail>,
}
impl World {
    pub fn consumption_report(
        &self,
        content: &Content,
        viewer: Viewer,
        country: &CountryId,
    ) -> Result<ConsumptionReport, String> {
        if self.content_hash != content.hash() {
            return Err("Consumption inspection requires exact campaign content".into());
        }
        let c = self
            .countries
            .get(country)
            .ok_or("Unknown consumption country")?;
        let mut report = ConsumptionReport {
            country: country.clone(),
            day: self.day,
            source: content.consumption.as_ref().map(|d| d.source.clone()),
            owned: None,
        };
        if self.rules < 36
            || country != &self.player
            || viewer != Viewer::Government(self.player.clone())
        {
            return Ok(report);
        }
        self.validate_country_consumption(content, country)?;
        report.owned = Some(ConsumptionDetail {
            food_supply: c
                .markets
                .as_ref()
                .and_then(|m| m.food.last.as_ref())
                .map(|s| s.supply),
            groups: c
                .households
                .iter()
                .zip(&content.country(country).expect("country").households)
                .filter_map(|(h, d)| {
                    h.delivery.clone().map(|receipt| HouseholdConsumptionLine {
                        group: h.id.clone(),
                        name: d.name.clone(),
                        receipt,
                    })
                })
                .collect(),
        });
        Ok(report)
    }
}
