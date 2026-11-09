//! Actual paid-income-limited auxiliary production. Existing civilian Food and
//! Energy remain distinct goods; no industrial receipt duplicates those stocks.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionFunding {
    pub private_income_paid: Money,
    pub service_due: Money,
    pub service_paid: Money,
    pub maintenance_due: Money,
    pub maintenance_paid: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapitalMaterialUse {
    pub commitment: CommitmentId,
    pub policy: PolicyId,
    pub quantity: ProductionQuantity,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NationalProductionDay {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub industry_opening: Option<IndustryOpeningFactors>,
    pub physical: ProductionInventoryDay,
    pub request: ProductionDayRequest,
    pub funding: ProductionFunding,
    pub service_required: ProductionQuantity,
    pub service_delivered: ProductionQuantity,
    pub maintenance_required: ProductionQuantity,
    pub maintenance_delivered: ProductionQuantity,
    pub service_percent: i32,
    pub maintenance_percent: i32,
    /// These uses occur after storage, never before this day's physical output.
    pub capital_uses: Vec<CapitalMaterialUse>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CountryProduction {
    pub inventory: ProductionInventory,
    pub last: Option<NationalProductionDay>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NationalProduction {
    pub countries: BTreeMap<CountryId, CountryProduction>,
}
fn quote(q: u64, numerator: u64, denominator: u64) -> Result<u64, String> {
    if denominator == 0 {
        return Err("Production ratio requires a positive denominator".into());
    }
    u64::try_from((u128::from(q) * u128::from(numerator)).div_ceil(u128::from(denominator)))
        .map_err(|_| "Production physical quote overflow".into())
}
fn fraction(actual: u64, required: u64) -> i32 {
    if required == 0 {
        100
    } else {
        ((u128::from(actual) * 100 / u128::from(required)).min(100)) as i32
    }
}
fn worker_pool(
    workers: Population,
    share: BasisPoints,
    paid: Money,
    benchmark: Money,
) -> LaborDays {
    let pool = u128::from(workers.0) * u128::from(share.units()) / 10000;
    let funded = if benchmark.0 <= 0 {
        0
    } else {
        (u128::from(paid.0.max(0) as u64) * 10000 / benchmark.0 as u128).min(10000)
    };
    LaborDays((pool * funded / 10000) as u64)
}
impl World {
    pub(crate) fn seed_production(&mut self, content: &Content) -> Result<(), String> {
        let Some(definition) = &content.production else {
            self.production = None;
            return Ok(());
        };
        let mut countries = BTreeMap::new();
        for (id, opening) in &definition.countries {
            countries.insert(
                id.clone(),
                CountryProduction {
                    inventory: ProductionInventory::new(
                        &definition.framework_for(id)?,
                        self.day,
                        opening.stocks.clone(),
                    )?,
                    last: None,
                },
            );
        }
        self.production = Some(NationalProduction { countries });
        Ok(())
    }
    /// Maintenance supplied yesterday constrains only funded additions to the
    /// older civilian capacity. Opening equipment starts authored as maintained.
    pub(crate) fn production_maintenance(&self, id: &CountryId) -> i32 {
        self.production
            .as_ref()
            .and_then(|p| p.countries.get(id))
            .and_then(|p| p.last.as_ref())
            .map_or(100, |day| day.maintenance_percent)
    }
    pub(crate) fn settle_production(
        &mut self,
        content: &Content,
        id: &CountryId,
        funding: ProductionFunding,
        industry_opening: Option<IndustryOpeningFactors>,
    ) -> Result<i32, String> {
        let Some(definition) = &content.production else {
            return Ok(100);
        };
        if funding.private_income_paid.0 < 0
            || funding.service_due.0 < 0
            || funding.service_paid.0 < 0
            || funding.maintenance_due.0 < 0
            || funding.maintenance_paid.0 < 0
            || funding.service_paid.0 > funding.service_due.0
            || funding.maintenance_paid.0 > funding.maintenance_due.0
        {
            return Err("Invalid actual production funding".into());
        }
        let country = &self.countries[id];
        let opening = &definition.countries[id];
        let framework = definition.framework_for(id)?;
        let assets = match (&content.industry, &industry_opening) {
            (Some(d), Some(f)) => d.assets(definition, id, &f.levels, f.maintenance_readiness)?,
            (None, None) => IndustryAssets {
                installed: opening.installed.clone(),
                capital: opening.capital.clone(),
            },
            _ => return Err("Missing industrial opening factors".into()),
        };
        let state = self
            .production
            .as_ref()
            .ok_or("Missing production state")?
            .countries
            .get(id)
            .ok_or("Missing production country")?;
        if state.inventory.day.checked_add(DaySpan::new(1)) != Some(self.day) {
            return Err("Production settlement must advance exactly one day".into());
        }
        let workers = Population(country.households.iter().try_fold(0_u64, |sum, h| {
            sum.checked_add(h.employed.0)
                .ok_or("Production worker overflow")
        })?);
        let daily = (content.country(id).ok_or("Country definition")?.gdp.units() / 365).max(1);
        let labor = worker_pool(
            workers,
            definition.worker_share,
            funding.private_income_paid,
            Money(daily * 70 / 100),
        );
        let opening_service_total = quote(
            opening.service_units_per_percent.0,
            country.services.max(0) as u64,
            1,
        )?;
        let service_total = quote(
            opening_service_total,
            country.population.0,
            content
                .country(id)
                .ok_or("Country definition")?
                .population
                .0,
        )?;
        let service_required = if funding.service_paid.0 == 0 {
            0
        } else {
            quote(
                service_total,
                funding.service_paid.0 as u64,
                funding.service_due.0 as u64,
            )?
        };
        let extra = (country.energy_capacity - country.opening_energy_capacity).max(0) as u64
            + (country.food_capacity - country.opening_food_capacity).max(0) as u64;
        let maintenance_required = quote(opening.maintenance_units_per_step.0, extra, 15)?
            .checked_add(
                opening
                    .maintenance_units_per_step
                    .0
                    .checked_mul(
                        industry_opening
                            .as_ref()
                            .map_or(0, |f| f.levels.values().map(|n| u64::from(*n)).sum()),
                    )
                    .ok_or("Industrial maintenance overflow")?,
            )
            .ok_or("Industrial maintenance overflow")?;
        let funded_maintenance = if funding.maintenance_due.0 == 0 {
            maintenance_required
        } else {
            quote(
                maintenance_required,
                funding.maintenance_paid.0 as u64,
                funding.maintenance_due.0 as u64,
            )?
        };
        let service_target = service_required
            .checked_add(funded_maintenance)
            .ok_or("Service requirement overflow")?;
        let mut capacities = BTreeMap::new();
        for (sector, recipe) in &framework.sectors {
            let requested = if sector == &definition.service_sector {
                service_target.div_ceil(recipe.outputs_per_batch[&definition.service_good].0)
            } else {
                recipe
                    .outputs_per_batch
                    .iter()
                    .map(|(good, amount)| {
                        opening.storage[good]
                            .capacity
                            .0
                            .saturating_sub(state.inventory.positions[good].quantity.0)
                            .div_ceil(amount.0)
                    })
                    .max()
                    .unwrap_or(0)
            };
            capacities.insert(
                sector.clone(),
                SectorCapacity {
                    installed_batches: assets.installed[sector],
                    requested_batches: ProductionBatches(if industry_opening.is_some() {
                        requested
                    } else {
                        requested.min(opening.installed[sector].0)
                    }),
                },
            );
        }
        let zeros: BTreeMap<_, _> = framework
            .goods
            .keys()
            .map(|id| (id.clone(), ProductionQuantity(0)))
            .collect();
        let mut request = ProductionDayRequest {
            capacities,
            workers,
            labor,
            capital: assets.capital,
            imports: zeros.clone(),
            exports: zeros.clone(),
            final_use: zeros,
        };
        let current = state
            .inventory
            .positions
            .iter()
            .map(|(good, pos)| (good.clone(), pos.quantity))
            .collect();
        let plan = framework.plan_day(
            &request.capacities,
            &current,
            workers,
            labor,
            &request.capital,
        )?;
        let available = plan.goods[&definition.service_good].closing.0;
        let service_delivered = service_required.min(available);
        let maintenance_delivered = funded_maintenance.min(available - service_delivered);
        request.final_use.insert(
            definition.service_good.clone(),
            ProductionQuantity(service_delivered + maintenance_delivered),
        );
        let (inventory, physical) =
            state
                .inventory
                .apply_day(&framework, &opening.storage, &request)?;
        let service_percent = fraction(service_delivered, service_required);
        let maintenance_percent = fraction(maintenance_delivered, maintenance_required);
        let state = self
            .production
            .as_mut()
            .expect("production")
            .countries
            .get_mut(id)
            .expect("country");
        state.inventory = inventory;
        state.last = Some(NationalProductionDay {
            industry_opening,
            physical,
            request,
            funding,
            service_required: ProductionQuantity(service_required),
            service_delivered: ProductionQuantity(service_delivered),
            maintenance_required: ProductionQuantity(maintenance_required),
            maintenance_delivered: ProductionQuantity(maintenance_delivered),
            service_percent,
            maintenance_percent,
            capital_uses: vec![],
        });
        Ok(service_percent)
    }
    pub(crate) fn consume_construction(
        &mut self,
        content: &Content,
        id: &CountryId,
        project: &PolicyInstance,
    ) -> Result<(), String> {
        let Some(definition) = &content.production else {
            return Ok(());
        };
        let Some(quantity) = definition.construction_required(id, &project.policy)? else {
            return Ok(());
        };
        let framework = definition.framework_for(id)?;
        let state = self
            .production
            .as_mut()
            .ok_or("Missing production state")?
            .countries
            .get_mut(id)
            .ok_or("Missing production country")?;
        let last = state
            .last
            .as_mut()
            .ok_or("Construction requires actual production settlement")?;
        if last.physical.day != self.day
            || last
                .capital_uses
                .iter()
                .any(|used| used.commitment == project.id)
        {
            return Err("Invalid duplicate/undated capital materials use".into());
        }
        state.inventory = state.inventory.withdraw_final(
            &framework,
            &BTreeMap::from([(definition.assembly_good.clone(), quantity)]),
        )?;
        last.capital_uses.push(CapitalMaterialUse {
            commitment: project.id,
            policy: project.policy.clone(),
            quantity,
        });
        Ok(())
    }
    pub(crate) fn construction_available(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Option<ProductionQuantity> {
        let definition = content.production.as_ref()?;
        self.production
            .as_ref()?
            .countries
            .get(id)?
            .inventory
            .positions
            .get(&definition.assembly_good)
            .map(|p| p.quantity)
    }
    pub(crate) fn validate_production(&self, content: &Content) -> Result<(), String> {
        let (definition, state) = match (&content.production, &self.production) {
            (None, None) => return Ok(()),
            (Some(definition), Some(state)) => (definition, state),
            _ => return Err("Production state differs from pinned campaign rules".into()),
        };
        if state.countries.keys().ne(self.countries.keys()) {
            return Err("Production country roster mismatch".into());
        }
        for (id, state) in &state.countries {
            let framework = definition.framework_for(id)?;
            state.inventory.validate(&framework)?;
            if state.inventory.day != self.day
                || state.inventory.start_day != content.start_day()?
                || state.inventory.opening != definition.countries[id].stocks
            {
                return Err("Production inventory date or opening mismatch".into());
            }
            if self.day == state.inventory.start_day {
                if state.last.is_some() {
                    return Err("Opening production cannot have invented history".into());
                }
                continue;
            }
            let last = state
                .last
                .as_ref()
                .ok_or("Missing production daily receipt")?;
            let country = &self.countries[id];
            let workers = Population(country.households.iter().try_fold(0_u64, |sum, h| {
                sum.checked_add(h.employed.0)
                    .ok_or("Production worker overflow")
            })?);
            let daily = (content.country(id).ok_or("Country definition")?.gdp.units() / 365).max(1);
            let assets = match (&content.industry, &last.industry_opening) {
                (None, None) => IndustryAssets {
                    installed: definition.countries[id].installed.clone(),
                    capital: definition.countries[id].capital.clone(),
                },
                (Some(industry), Some(factors)) => {
                    let state = country.industry.as_ref().ok_or("Missing industry state")?;
                    let mut opening = state.levels.clone();
                    for record in country.commitments.as_deref().unwrap_or_default() {
                        if let Some((sector, delta)) = industry.effect(&record.project.policy)
                            && record
                                .milestones
                                .iter()
                                .any(|m| m.stage == CommitmentStage::Operating && m.day == self.day)
                        {
                            let n = opening.get_mut(sector).ok_or("Industry sector")?;
                            *n = n
                                .checked_add_signed(-delta)
                                .ok_or("Industry operating attribution")?;
                        }
                    }
                    if factors.levels != opening {
                        return Err(
                            "Industry settlement predates today\'s actual commissioning".into()
                        );
                    }
                    let projection = IndustryUpkeepProjection {
                        gdp: content.country(id).ok_or("Industry definition")?.gdp,
                        levels: opening.values().sum(),
                        rate: state.maintenance_bps_per_level,
                        opening_remainder: factors.upkeep_opening_remainder,
                    };
                    let (due, carry) = projection.assessment(DaySpan::new(1))?;
                    if carry != state.upkeep_remainder || due.0 > last.funding.maintenance_due.0 {
                        return Err("Industry upkeep receipt/carry mismatch".into());
                    }
                    industry.assets(definition, id, &opening, factors.maintenance_readiness)?
                }
                _ => return Err("Industry receipt differs from pinned rules".into()),
            };
            let funding = &last.funding;
            if last.physical.day != self.day
                || funding.private_income_paid != country.report.wages
                || funding.private_income_paid.0 < 0
                || funding.service_due.0 < 0
                || funding.service_paid.0 < 0
                || funding.maintenance_due.0 < 0
                || funding.maintenance_paid.0 < 0
                || funding.service_paid.0 > funding.service_due.0
                || funding.maintenance_paid.0 > funding.maintenance_due.0
                || last.request.workers != workers
                || last.request.labor
                    != worker_pool(
                        workers,
                        definition.worker_share,
                        funding.private_income_paid,
                        Money(daily * 70 / 100),
                    )
                || !(0..=100).contains(&last.service_percent)
                || !(0..=100).contains(&last.maintenance_percent)
                || last.service_percent
                    != fraction(last.service_delivered.0, last.service_required.0)
                || last.maintenance_percent
                    != fraction(last.maintenance_delivered.0, last.maintenance_required.0)
                || last.service_delivered.0 > last.service_required.0
                || last.maintenance_delivered.0 > last.maintenance_required.0
                || last.request.capacities.keys().ne(framework.sectors.keys())
                || last.request.capital != assets.capital
                || last.request.capacities.iter().any(|(sector, cap)| {
                    cap.installed_batches != assets.installed[sector]
                        || content.industry.is_none()
                            && cap.requested_batches.0 > cap.installed_batches.0
                })
                || last.capital_uses.len() > content.policies.len()
                || last.physical.storage.keys().ne(framework.goods.keys())
            {
                return Err("Invalid production receipt/capacity/service outcomes".into());
            }
            framework.validate_day(
                &last.physical.production,
                &last.request.capacities,
                last.request.workers,
            )?;
            if last.physical.production.labor_available != last.request.labor
                || last.physical.production.capital_available != last.request.capital
            {
                return Err("Production factor receipt differs from funded allocation".into());
            }
            let mut projects = BTreeSet::new();
            let mut capital_use = 0_u64;
            for used in &last.capital_uses {
                if !projects.insert(used.commitment)
                    || definition.construction_required(id, &used.policy)? != Some(used.quantity)
                {
                    return Err("Invalid capital materials attribution".into());
                }
                let record = country
                    .commitment(&used.commitment)
                    .ok_or("Capital use has no funded commitment")?;
                if record.project.policy != used.policy
                    || !record
                        .milestones
                        .iter()
                        .any(|m| m.stage == CommitmentStage::Operating && m.day == self.day)
                {
                    return Err("Capital use does not match actual operation".into());
                }
                capital_use = capital_use
                    .checked_add(used.quantity.0)
                    .ok_or("Capital materials counter overflow")?;
            }
            let mut lifetime_capital_use = 0_u64;
            let mut expected_today = BTreeSet::new();
            for record in country.commitments.as_deref().unwrap_or_default() {
                let Some(quantity) =
                    definition.construction_required(id, &record.project.policy)?
                else {
                    continue;
                };
                if let Some(operating) = record
                    .milestones
                    .iter()
                    .find(|m| m.stage == CommitmentStage::Operating)
                {
                    lifetime_capital_use = lifetime_capital_use
                        .checked_add(quantity.0)
                        .ok_or("Capital materials history overflow")?;
                    if operating.day == self.day {
                        expected_today.insert(record.id);
                    }
                }
            }
            if expected_today != projects
                || state.inventory.positions[&definition.assembly_good]
                    .totals
                    .final_use
                    .0
                    != lifetime_capital_use
            {
                return Err(
                    "Physical capital consumption differs from funded delivery history".into(),
                );
            }
            for (good, stock) in &state.inventory.positions {
                let flow = &last.physical.storage[good];
                let used = if good == &definition.assembly_good {
                    capital_use
                } else {
                    0
                };
                if flow.closing.0.checked_sub(used) != Some(stock.quantity.0)
                    || stock.totals.imported.0 != 0
                    || stock.totals.exported.0 != 0
                    || good != &definition.assembly_good
                        && good != &definition.service_good
                        && stock.totals.final_use.0 != 0
                    || stock.quantity.0 > definition.countries[id].storage[good].capacity.0
                    || flow.opening_loss_carry >= 10000
                    || flow.closing_loss_carry >= 10000
                    || last.physical.production.goods[good].opening.0 == 0
                        && flow.opening_loss_carry != 0
                    || stock.loss_carry
                        != if stock.quantity.0 == 0 {
                            0
                        } else {
                            flow.closing_loss_carry
                        }
                    || flow.after_production != last.physical.production.goods[good].closing
                    || flow.imports.0 != 0
                    || flow.exports.0 != 0
                    || last.request.imports.get(good) != Some(&ProductionQuantity(0))
                    || last.request.exports.get(good) != Some(&ProductionQuantity(0))
                    || last.request.final_use.get(good) != Some(&flow.final_use)
                    || flow.final_use.0
                        != if good == &definition.service_good {
                            last.service_delivered
                                .0
                                .checked_add(last.maintenance_delivered.0)
                                .ok_or("Service flow overflow")?
                        } else {
                            0
                        }
                    || u128::from(flow.after_production.0)
                        != u128::from(flow.final_use.0)
                            + u128::from(flow.discarded.0)
                            + u128::from(flow.spoiled.0)
                            + u128::from(flow.closing.0)
                {
                    return Err("Production stock/day/capital flows do not reconcile".into());
                }
                let rule = &definition.countries[id].storage[good];
                let remaining = flow
                    .after_production
                    .0
                    .checked_sub(flow.final_use.0)
                    .ok_or("Excess production final use")?;
                let stored = remaining.min(rule.capacity.0);
                let numerator = u128::from(stored) * u128::from(rule.loss_bps.units())
                    + u128::from(flow.opening_loss_carry);
                let spoiled = (numerator / 10000) as u64;
                let closing = stored
                    .checked_sub(spoiled)
                    .ok_or("Excess production spoilage")?;
                let carry = if closing == 0 {
                    0
                } else {
                    (numerator % 10000) as u32
                };
                if flow.discarded.0 != remaining.saturating_sub(rule.capacity.0)
                    || flow.spoiled.0 != spoiled
                    || flow.closing.0 != closing
                    || flow.closing_loss_carry != carry
                    || stock.totals.produced.0 < last.physical.production.goods[good].produced.0
                    || stock.totals.industrial_use.0
                        < last.physical.production.goods[good].consumed.0
                    || stock.totals.final_use.0
                        < flow
                            .final_use
                            .0
                            .checked_add(used)
                            .ok_or("Capital/day flow overflow")?
                    || stock.totals.discarded.0 < flow.discarded.0
                    || stock.totals.spoiled.0 < flow.spoiled.0
                {
                    return Err("Production storage/cumulative receipt mismatch".into());
                }
                let elapsed = u128::from(
                    self.day
                        .elapsed_since(state.inventory.start_day)
                        .ok_or("Production date precedes opening")?
                        .units(),
                );
                let production_bound =
                    framework
                        .sectors
                        .iter()
                        .fold(0_u128, |sum, (sector, recipe)| {
                            sum.saturating_add(
                                self.industry_capacity_days(content, id, sector, elapsed)
                                    .saturating_mul(u128::from(
                                        recipe.outputs_per_batch.get(good).map_or(0, |q| q.0),
                                    )),
                            )
                        });
                let input_bound = framework
                    .sectors
                    .iter()
                    .fold(0_u128, |sum, (sector, recipe)| {
                        sum.saturating_add(
                            self.industry_capacity_days(content, id, sector, elapsed)
                                .saturating_mul(u128::from(
                                    recipe.inputs_per_batch.get(good).map_or(0, |q| q.0),
                                )),
                        )
                    });
                if u128::from(stock.totals.produced.0) > production_bound
                    || u128::from(stock.totals.industrial_use.0) > input_bound
                {
                    return Err("Production cumulative flows exceed installed daily capacity over actual elapsed days".into());
                }
            }
        }
        Ok(())
    }
}
