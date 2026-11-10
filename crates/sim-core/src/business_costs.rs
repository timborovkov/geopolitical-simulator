//! Cost valuation for the existing aggregate civilian businesses. Values are
//! carrying costs, never market revaluations, new cash or private public assets.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessCostDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub food_factor_bps: BasisPoints,
    pub energy_factor_bps: BasisPoints,
}
impl BusinessCostDefinition {
    pub(crate) fn validate(&self, content: &Content) -> Result<(), String> {
        if self.source_version.is_empty()
            || self.description.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
            || u64::from(self.food_factor_bps.units()) + u64::from(self.energy_factor_bps.units())
                > 10000
        {
            return Err("Business cost allocation needs bounded, attributed assumptions".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessCosts {
    pub factors_paid: Money,
    pub food_factors: Money,
    pub energy_factors: Money,
    pub food_imports: Money,
    pub energy_imports: Money,
    pub food_sold: Money,
    pub energy_used: Money,
    pub food_spoiled: Money,
    pub food_import_loss: Money,
    pub energy_import_loss: Money,
}
fn money(n: i128) -> Result<Money, String> {
    i64::try_from(n)
        .map(Money)
        .map_err(|_| "Business valuation exceeds the accounting range".into())
}
fn sum(a: Money, b: Money) -> Result<Money, String> {
    money(i128::from(a.0) + i128::from(b.0))
}
impl BusinessCosts {
    pub fn food_inventory(&self) -> Result<Money, String> {
        money(
            i128::from(self.food_factors.0) + i128::from(self.food_imports.0)
                - i128::from(self.food_sold.0)
                - i128::from(self.food_spoiled.0)
                - i128::from(self.food_import_loss.0),
        )
    }
    pub fn energy_inventory(&self) -> Result<Money, String> {
        money(
            i128::from(self.energy_factors.0) + i128::from(self.energy_imports.0)
                - i128::from(self.energy_used.0)
                - i128::from(self.energy_import_loss.0),
        )
    }
    pub fn period_factors(&self) -> Result<Money, String> {
        money(
            i128::from(self.factors_paid.0)
                - i128::from(self.food_factors.0)
                - i128::from(self.energy_factors.0),
        )
    }
    fn validate(&self) -> Result<(), String> {
        if [
            self.factors_paid,
            self.food_factors,
            self.energy_factors,
            self.food_imports,
            self.energy_imports,
            self.food_sold,
            self.energy_used,
            self.food_spoiled,
            self.food_import_loss,
            self.energy_import_loss,
        ]
        .iter()
        .any(|m| !(0..=1_000_000_000_000_000_000).contains(&m.0))
            || self.food_inventory()?.0 < 0
            || self.energy_inventory()?.0 < 0
            || self.period_factors()?.0 < 0
        {
            return Err("Invalid cumulative business production costs".into());
        }
        Ok(())
    }
}

/// Food and energy retain different physical types even in a cost account.
/// ```compile_fail
/// use sim_core::{Food, Energy, ValuedBusinessStock};
/// fn wrong_good(food: ValuedBusinessStock<Food>) {
///     let energy: ValuedBusinessStock<Energy> = food;
/// }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValuedBusinessStock<Q> {
    pub physical: MarketSettlement<Q>,
    pub opening_value: Money,
    pub production_cost: Money,
    pub import_cost: Money,
    pub unstocked_import_cost: Money,
    pub used_cost: Money,
    pub lost_cost: Money,
    pub closing_value: Money,
}
fn value_stock<Q: Copy>(
    p: MarketSettlement<Q>,
    units: fn(Q) -> i64,
    opening: Money,
    production: Money,
    imports: Money,
) -> Result<ValuedBusinessStock<Q>, String> {
    let quantities = [
        p.opening_stock,
        p.production,
        p.imports,
        p.supply,
        p.delivered,
        p.loss,
        p.closing_stock,
    ];
    if quantities
        .iter()
        .any(|q| !(0..=1_000_000_000_000).contains(&units(*q)))
        || [opening, production, imports]
            .iter()
            .any(|m| !(0..=1_000_000_000_000_000_000).contains(&m.0))
        || (units(p.opening_stock) == 0 && opening.0 != 0)
        || (units(p.production) == 0 && production.0 != 0)
        || i128::from(units(p.opening_stock))
            + i128::from(units(p.production))
            + i128::from(units(p.imports))
            + i128::from(p.reserve_release.map_or(0, units))
            != i128::from(units(p.supply))
        || i128::from(units(p.delivered))
            + i128::from(units(p.loss))
            + i128::from(units(p.closing_stock))
            != i128::from(units(p.supply))
    {
        return Err("Business stock quantity and acquisition cost do not reconcile".into());
    }
    let unstocked = if units(p.imports) == 0 {
        imports
    } else {
        Money(0)
    };
    let total = i128::from(opening.0) + i128::from(production.0) + i128::from(imports.0)
        - i128::from(unstocked.0);
    let closing = if units(p.supply) == 0 {
        0
    } else {
        total * i128::from(units(p.closing_stock)) / i128::from(units(p.supply))
    };
    let removed = total - closing;
    let outgoing = i128::from(units(p.delivered)) + i128::from(units(p.loss));
    let used = if outgoing == 0 {
        0
    } else {
        removed * i128::from(units(p.delivered)) / outgoing
    };
    Ok(ValuedBusinessStock {
        physical: p,
        opening_value: opening,
        production_cost: production,
        import_cost: imports,
        unstocked_import_cost: unstocked,
        used_cost: money(used)?,
        lost_cost: money(removed - used)?,
        closing_value: money(closing)?,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessCostDay {
    pub day: CampaignDay,
    pub before: BusinessCosts,
    pub factors_paid: Money,
    pub food: ValuedBusinessStock<Food>,
    pub energy: ValuedBusinessStock<Energy>,
}
impl BusinessCostDay {
    fn after(&self) -> Result<BusinessCosts, String> {
        let mut a = self.before.clone();
        a.factors_paid = sum(a.factors_paid, self.factors_paid)?;
        a.food_factors = sum(a.food_factors, self.food.production_cost)?;
        a.energy_factors = sum(a.energy_factors, self.energy.production_cost)?;
        a.food_imports = sum(a.food_imports, self.food.import_cost)?;
        a.energy_imports = sum(a.energy_imports, self.energy.import_cost)?;
        a.food_sold = sum(a.food_sold, self.food.used_cost)?;
        a.energy_used = sum(a.energy_used, self.energy.used_cost)?;
        a.food_spoiled = sum(a.food_spoiled, self.food.lost_cost)?;
        a.food_import_loss = sum(a.food_import_loss, self.food.unstocked_import_cost)?;
        a.energy_import_loss = sum(a.energy_import_loss, self.energy.unstocked_import_cost)?;
        a.validate()?;
        Ok(a)
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessProfitTotals {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payroll_obligations: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_capital: Option<PrivateCapitalTotals>,
    pub sales: Money,
    pub public_cash: Money,
    pub deferred_capital: Money,
    pub receivables: Money,
    pub period_factors: Money,
    pub food_sold_cost: Money,
    pub energy_used_cost: Money,
    pub food_spoiled_cost: Money,
    pub unstocked_import_cost: Money,
    pub inventory_value: Money,
    pub distributions: Money,
    pub cash_change: Money,
}
impl BusinessProfitTotals {
    pub(crate) fn validate_checkpoint(&self) -> Result<(), String> {
        if self
            .payroll_obligations
            .is_some_and(|m| !(0..=1_000_000_000_000_000).contains(&m.0))
        {
            return Err("Invalid payroll checkpoint".into());
        }
        if let Some(a) = &self.private_capital {
            a.validate()?;
        }
        if [
            self.sales,
            self.public_cash,
            self.deferred_capital,
            self.receivables,
            self.period_factors,
            self.food_sold_cost,
            self.energy_used_cost,
            self.food_spoiled_cost,
            self.unstocked_import_cost,
            self.inventory_value,
            self.distributions,
        ]
        .iter()
        .any(|m| !(0..=1_000_000_000_000_000_000).contains(&m.0))
            || self.deferred_capital.0 > self.public_cash.0
            || i128::from(self.result()?.0)
                != i128::from(self.cash_change.0)
                    + i128::from(self.distributions.0)
                    + i128::from(self.inventory_value.0)
                    + i128::from(self.receivables.0)
                    - i128::from(self.deferred_capital.0)
                    - i128::from(self.payroll_obligations.map_or(0, |m| m.0))
                    + i128::from(self.private_capital.as_ref().map_or(0, |a| a.net_assets.0))
        {
            return Err("Business earnings checkpoint is invalid".into());
        }
        Ok(())
    }
    pub fn revenue(&self) -> Result<Money, String> {
        money(
            i128::from(self.sales.0) + i128::from(self.public_cash.0)
                - i128::from(self.deferred_capital.0)
                + i128::from(self.receivables.0),
        )
    }
    pub fn expenses(&self) -> Result<Money, String> {
        money(
            i128::from(
                self.private_capital
                    .as_ref()
                    .map_or(0, |a| a.depreciation.0),
            ) + i128::from(self.payroll_obligations.map_or(0, |m| m.0))
                + i128::from(self.period_factors.0)
                + i128::from(self.food_sold_cost.0)
                + i128::from(self.energy_used_cost.0)
                + i128::from(self.food_spoiled_cost.0)
                + i128::from(self.unstocked_import_cost.0),
        )
    }
    pub fn result(&self) -> Result<Money, String> {
        money(i128::from(self.revenue()?.0) - i128::from(self.expenses()?.0))
    }
    pub fn change_since(&self, a: &Self) -> Result<Self, String> {
        Ok(Self {
            payroll_obligations: match (self.payroll_obligations, a.payroll_obligations) {
                (Some(now), Some(old)) => Some(money(i128::from(now.0) - i128::from(old.0))?),
                (None, None) => None,
                _ => return Err("Payroll checkpoint version mismatch".into()),
            },
            private_capital: match (&self.private_capital, &a.private_capital) {
                (None, None) => None,
                (Some(now), Some(old)) => Some(now.change_since(old)),
                _ => return Err("Business capital checkpoint version mismatch".into()),
            },
            sales: money(i128::from(self.sales.0) - i128::from(a.sales.0))?,
            public_cash: money(i128::from(self.public_cash.0) - i128::from(a.public_cash.0))?,
            deferred_capital: money(
                i128::from(self.deferred_capital.0) - i128::from(a.deferred_capital.0),
            )?,
            receivables: money(i128::from(self.receivables.0) - i128::from(a.receivables.0))?,
            period_factors: money(
                i128::from(self.period_factors.0) - i128::from(a.period_factors.0),
            )?,
            food_sold_cost: money(
                i128::from(self.food_sold_cost.0) - i128::from(a.food_sold_cost.0),
            )?,
            energy_used_cost: money(
                i128::from(self.energy_used_cost.0) - i128::from(a.energy_used_cost.0),
            )?,
            food_spoiled_cost: money(
                i128::from(self.food_spoiled_cost.0) - i128::from(a.food_spoiled_cost.0),
            )?,
            unstocked_import_cost: money(
                i128::from(self.unstocked_import_cost.0) - i128::from(a.unstocked_import_cost.0),
            )?,
            inventory_value: money(
                i128::from(self.inventory_value.0) - i128::from(a.inventory_value.0),
            )?,
            distributions: money(i128::from(self.distributions.0) - i128::from(a.distributions.0))?,
            cash_change: money(i128::from(self.cash_change.0) - i128::from(a.cash_change.0))?,
        })
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessCostBook {
    pub costs: BusinessCosts,
    pub day_opening: BusinessProfitTotals,
    pub last: Option<BusinessCostDay>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessCapitalAdvance {
    pub commitment: CommitmentId,
    pub amount: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessProfitDetail {
    pub cumulative: BusinessProfitTotals,
    pub current_day: BusinessProfitTotals,
    pub costs: BusinessCosts,
    pub latest_costs: Option<BusinessCostDay>,
    pub pending_capital: Vec<BusinessCapitalAdvance>,
    pub delivered_contract_capital: Money,
    pub retained_withdrawal_capital: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessProfitReport {
    pub country: CountryId,
    pub day: CampaignDay,
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned: Option<BusinessProfitDetail>,
}
impl Country {
    pub(crate) fn business_capital(
        &self,
        content: &Content,
    ) -> Result<(Vec<BusinessCapitalAdvance>, Money, Money), String> {
        let mut pending = Vec::new();
        let (mut delivered, mut withdrawn) = (0_i128, 0_i128);
        for r in self
            .commitments
            .as_ref()
            .ok_or("Missing business contract records")?
        {
            // A common 10% administration component is earned at proposal, even
            // on routes paying the entire contract before delivery. Amendments
            // are separate administrative services. Both are attributed rules.
            let fee = (r.project.cost.0 / 10).max(1);
            let amendment = (i128::from(r.project.cost.0)
                * i128::from(
                    content
                        .commitments
                        .as_ref()
                        .ok_or("Missing contract terms")?
                        .amendment_fee_bps
                        .units(),
                )
                / 10000)
                .max(1);
            let capital = money(
                i128::from(r.paid.0) - i128::from(fee) - amendment * i128::from(r.amendments),
            )?;
            if capital.0 < 0 {
                return Err("Business contract fees exceed actual receipts".into());
            }
            if capital.0 == 0 {
                continue;
            }
            if self
                .projects
                .iter()
                .any(|p| p.id == r.id && p.stage == PolicyStage::Implementing)
            {
                pending.push(BusinessCapitalAdvance {
                    commitment: r.id,
                    amount: capital,
                });
            } else if r.stage() == CommitmentStage::Withdrawn {
                withdrawn += i128::from(capital.0);
            } else if !r.direct_effects.is_empty() {
                delivered += i128::from(capital.0);
            } else {
                return Err(
                    "Funded business contract has no delivery or pending obligation".into(),
                );
            }
        }
        Ok((pending, money(delivered)?, money(withdrawn)?))
    }
    fn business_profit_totals(&self, content: &Content) -> Result<BusinessProfitTotals, String> {
        let b = self
            .business
            .as_ref()
            .ok_or("Missing civilian business account")?;
        let a = &self
            .business_costs
            .as_ref()
            .ok_or("Missing business costs")?
            .costs;
        let (pending, _, _) = self.business_capital(content)?;
        let deferred = money(pending.iter().map(|p| i128::from(p.amount.0)).sum())?;
        if deferred.0 > b.public_receipts.0 {
            return Err("Deferred public capital exceeds actual business receipts".into());
        }
        let t = BusinessProfitTotals {
            payroll_obligations: self
                .private_payroll
                .as_ref()
                .map(|s| s.unpaid())
                .transpose()?,
            private_capital: self
                .private_investment
                .as_ref()
                .map(|a| a.totals())
                .transpose()?,
            sales: b.operating_receipts,
            public_cash: b.public_receipts,
            deferred_capital: deferred,
            receivables: self.arrears_firms,
            period_factors: a.period_factors()?,
            food_sold_cost: a.food_sold,
            energy_used_cost: a.energy_used,
            food_spoiled_cost: a.food_spoiled,
            unstocked_import_cost: sum(a.food_import_loss, a.energy_import_loss)?,
            inventory_value: sum(a.food_inventory()?, a.energy_inventory()?)?,
            distributions: b.distributions,
            cash_change: money(i128::from(self.accounts.firms.0) - i128::from(b.opening_cash.0))?,
        };
        let bridge = i128::from(t.private_capital.as_ref().map_or(0, |a| a.net_assets.0))
            + i128::from(t.cash_change.0)
            + i128::from(t.distributions.0)
            + i128::from(t.inventory_value.0)
            + i128::from(t.receivables.0)
            - i128::from(t.deferred_capital.0)
            - i128::from(t.payroll_obligations.map_or(0, |m| m.0));
        if i128::from(t.result()?.0) != bridge {
            return Err(
                "Business profit, cash, inventory, receivables and advances do not reconcile"
                    .into(),
            );
        }
        Ok(t)
    }
}
impl World {
    pub(crate) fn begin_business_cost_day(&mut self, content: &Content) -> Result<(), String> {
        if content.business_costs.is_none() {
            return Ok(());
        }
        for c in self.countries.values_mut() {
            let totals = c.business_profit_totals(content)?;
            c.business_costs.as_mut().expect("cost book").day_opening = totals;
        }
        Ok(())
    }
    pub(crate) fn settle_business_costs(
        &mut self,
        content: &Content,
        id: &CountryId,
        energy_import_cost: Money,
        food_import_cost: Money,
    ) -> Result<(), String> {
        let Some(d) = &content.business_costs else {
            return Ok(());
        };
        let c = &self.countries[id];
        let before = c
            .business_costs
            .as_ref()
            .ok_or("Missing cost book")?
            .costs
            .clone();
        let m = c.markets.as_ref().ok_or("Missing costed markets")?;
        let f = m
            .food
            .last
            .as_ref()
            .ok_or("Missing food settlement")?
            .clone();
        let e = m
            .energy
            .last
            .as_ref()
            .ok_or("Missing energy settlement")?
            .clone();
        let wages = c.report.wages;
        let current_factors = c
            .private_payroll
            .as_ref()
            .and_then(|s| s.last.as_ref())
            .map(|day| day.current_factors_paid())
            .transpose()?
            .unwrap_or(wages);
        let food_factor = Money(if f.production == Food::ZERO {
            0
        } else {
            (i128::from(current_factors.0) * i128::from(d.food_factor_bps.units()) / 10000) as i64
        });
        let energy_factor = Money(if e.production == Energy::ZERO {
            0
        } else {
            (i128::from(current_factors.0) * i128::from(d.energy_factor_bps.units()) / 10000) as i64
        });
        let day = BusinessCostDay {
            day: self.day,
            factors_paid: wages,
            food: value_stock(
                f,
                Food::units,
                before.food_inventory()?,
                food_factor,
                food_import_cost,
            )?,
            energy: value_stock(
                e,
                Energy::units,
                before.energy_inventory()?,
                energy_factor,
                energy_import_cost,
            )?,
            before,
        };
        let book = self
            .countries
            .get_mut(id)
            .expect("country")
            .business_costs
            .as_mut()
            .expect("cost book");
        book.costs = day.after()?;
        book.last = Some(day);
        Ok(())
    }
    pub(crate) fn validate_business_costs(&self, content: &Content) -> Result<(), String> {
        self.validate_private_investment(content)?;
        self.validate_private_payroll(content)?;
        for (id, c) in &self.countries {
            let Some(book) = &c.business_costs else {
                if content.business_costs.is_some() {
                    return Err("Missing business cost book".into());
                }
                continue;
            };
            let definition = content
                .business_costs
                .as_ref()
                .ok_or("Unexpected business cost book")?;
            if !matches!(self.rules, 37..=40) {
                return Err("Business costs require rules 37 through 40".into());
            }
            book.costs.validate()?;
            if let Some(a) = &book.day_opening.private_capital {
                a.validate()?;
            }
            if book.day_opening.private_capital.is_some() != content.private_investment.is_some() {
                return Err("Business capital checkpoint does not match content".into());
            }
            self.validate_commitments(content, id)?;
            c.validate_business_cash(self, content)?;
            let o = &book.day_opening;
            if [
                o.sales,
                o.public_cash,
                o.deferred_capital,
                o.receivables,
                o.period_factors,
                o.food_sold_cost,
                o.energy_used_cost,
                o.food_spoiled_cost,
                o.unstocked_import_cost,
                o.inventory_value,
                o.distributions,
            ]
            .iter()
            .any(|m| !(0..=1_000_000_000_000_000_000).contains(&m.0))
                || !(-1_000_000_000_000_000_000..=1_000_000_000_000_000_000)
                    .contains(&o.cash_change.0)
                || o.deferred_capital.0 > o.public_cash.0
                || i128::from(o.result()?.0)
                    != i128::from(o.private_capital.as_ref().map_or(0, |a| a.net_assets.0))
                        + i128::from(o.cash_change.0)
                        + i128::from(o.distributions.0)
                        + i128::from(o.inventory_value.0)
                        + i128::from(o.receivables.0)
                        - i128::from(o.deferred_capital.0)
                        - i128::from(o.payroll_obligations.map_or(0, |m| m.0))
                || !(0..=1_000_000_000_000).contains(&c.arrears_firms.0)
            {
                return Err("Invalid opening business income account".into());
            }
            let b = c
                .business
                .as_ref()
                .ok_or("Missing costed business sector")?;
            if i128::from(b.payments.0) - i128::from(b.distributions.0)
                != i128::from(
                    c.private_investment
                        .as_ref()
                        .map(|a| a.totals())
                        .transpose()?
                        .map_or(0, |a| a.acquisitions.0),
                ) + i128::from(book.costs.factors_paid.0)
                    + i128::from(book.costs.food_imports.0)
                    + i128::from(book.costs.energy_imports.0)
            {
                return Err("Business costs omit or duplicate actual payments".into());
            }
            match &book.last {
                None if self.day == content.start_day()?
                    && book.costs == BusinessCosts::default()
                    && book.day_opening
                        == BusinessProfitTotals {
                            payroll_obligations: c
                                .private_payroll
                                .as_ref()
                                .map(|s| s.unpaid())
                                .transpose()?,
                            private_capital: c
                                .private_investment
                                .as_ref()
                                .map(|s| s.totals())
                                .transpose()?,
                            ..BusinessProfitTotals::default()
                        } => {}
                Some(day) if day.day == self.day && self.day > content.start_day()? => {
                    day.before.validate()?;
                    if !(0..=1_000_000_000_000).contains(&day.factors_paid.0) {
                        return Err("Invalid daily factor cost".into());
                    }
                    let m = c.markets.as_ref().ok_or("Missing valued markets")?;
                    let f = m.food.last.as_ref().ok_or("Missing valued food")?;
                    let e = m.energy.last.as_ref().ok_or("Missing valued energy")?;
                    let fc = Money(if f.production == Food::ZERO {
                        0
                    } else {
                        (i128::from(
                            c.private_payroll
                                .as_ref()
                                .and_then(|s| s.last.as_ref())
                                .map(|r| r.current_factors_paid())
                                .transpose()?
                                .unwrap_or(day.factors_paid)
                                .0,
                        ) * i128::from(definition.food_factor_bps.units())
                            / 10000) as i64
                    });
                    let ec = Money(if e.production == Energy::ZERO {
                        0
                    } else {
                        (i128::from(
                            c.private_payroll
                                .as_ref()
                                .and_then(|s| s.last.as_ref())
                                .map(|r| r.current_factors_paid())
                                .transpose()?
                                .unwrap_or(day.factors_paid)
                                .0,
                        ) * i128::from(definition.energy_factor_bps.units())
                            / 10000) as i64
                    });
                    if f.day != self.day
                        || e.day != self.day
                        || f.closing_stock != c.food_stock
                        || e.closing_stock != c.energy_stock
                        || day.factors_paid != c.report.wages
                        || &day.food.physical != f
                        || &day.energy.physical != e
                        || day.food
                            != value_stock(
                                f.clone(),
                                Food::units,
                                day.before.food_inventory()?,
                                fc,
                                day.food.import_cost,
                            )?
                        || day.energy
                            != value_stock(
                                e.clone(),
                                Energy::units,
                                day.before.energy_inventory()?,
                                ec,
                                day.energy.import_cost,
                            )?
                        || book.costs != day.after()?
                        || (c.food_stock == Food::ZERO && book.costs.food_inventory()?.0 != 0)
                        || (c.energy_stock == Energy::ZERO && book.costs.energy_inventory()?.0 != 0)
                    {
                        return Err(format!(
                            "Business costs disagree with {id:?} physical settlement"
                        ));
                    }
                    // Public receipts and receivables can legitimately move both
                    // ways today. Opening cumulative operating costs cannot.
                    if book.day_opening.private_capital
                        != c.private_investment
                            .as_ref()
                            .map(|s| s.totals_at(CampaignDay::new(self.day.units() - 1)))
                            .transpose()?
                    {
                        return Err("Private opening asset checkpoint does not reconcile".into());
                    }
                    if book.day_opening.distributions.0 > b.distributions.0
                        || book.day_opening.sales.0 > b.operating_receipts.0
                        || book.day_opening.public_cash.0 > b.public_receipts.0
                        || book.day_opening.period_factors != day.before.period_factors()?
                        || book.day_opening.food_sold_cost != day.before.food_sold
                        || book.day_opening.energy_used_cost != day.before.energy_used
                        || book.day_opening.food_spoiled_cost != day.before.food_spoiled
                        || book.day_opening.unstocked_import_cost
                            != sum(day.before.food_import_loss, day.before.energy_import_loss)?
                        || book.day_opening.inventory_value
                            != sum(day.before.food_inventory()?, day.before.energy_inventory()?)?
                    {
                        return Err("Invalid opening business profit checkpoint".into());
                    }
                }
                _ => return Err("Business cost receipt is missing or misdated".into()),
            }
            c.business_profit_totals(content)?;
        }
        Ok(())
    }
    pub fn business_profit_report(
        &self,
        content: &Content,
        viewer: Viewer,
        country: &CountryId,
    ) -> Result<BusinessProfitReport, String> {
        if self.content_hash != content.hash() {
            return Err("Profit inspection requires exact campaign content".into());
        }
        let c = self
            .countries
            .get(country)
            .ok_or("Unknown business country")?;
        let mut r = BusinessProfitReport {
            country: country.clone(),
            day: self.day,
            source: content.business_costs.as_ref().map(|d| d.source.clone()),
            owned: None,
        };
        if content.business_costs.is_none()
            || country != &self.player
            || viewer != Viewer::Government(self.player.clone())
        {
            return Ok(r);
        }
        self.validate_business_costs(content)?;
        let book = c.business_costs.as_ref().expect("validated cost book");
        let cumulative = c.business_profit_totals(content)?;
        let (pending, delivered, withdrawn) = c.business_capital(content)?;
        r.owned = Some(BusinessProfitDetail {
            current_day: cumulative.change_since(&book.day_opening)?,
            cumulative,
            costs: book.costs.clone(),
            latest_costs: book.last.clone(),
            pending_capital: pending,
            delivered_contract_capital: delivered,
            retained_withdrawal_capital: withdrawn,
        });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cash_paid_without_a_whole_import_receipt_is_a_loss_not_an_asset() {
        let p = MarketSettlement {
            day: CampaignDay::new(1),
            price_bps: BasisPoints::new(10000),
            physical_need: Food::new(1).unwrap(),
            requested_use: Food::new(1).unwrap(),
            opening_stock: Food::ZERO,
            production: Food::ZERO,
            imports: Food::ZERO,
            reserve_release: Some(Food::ZERO),
            supply: Food::ZERO,
            delivered: Food::ZERO,
            loss: Food::ZERO,
            closing_stock: Food::ZERO,
        };
        let valued = value_stock(p, Food::units, Money(0), Money(0), Money(3)).unwrap();
        assert_eq!(valued.unstocked_import_cost, Money(3));
        assert_eq!(valued.closing_value, Money(0));
        assert_eq!(valued.used_cost, Money(0));
        assert_eq!(valued.lost_cost, Money(0));
        let a = BusinessCosts {
            food_imports: Money(3),
            food_import_loss: Money(3),
            ..Default::default()
        };
        a.validate().unwrap();
        assert_eq!(a.food_inventory().unwrap(), Money(0));
    }
}
