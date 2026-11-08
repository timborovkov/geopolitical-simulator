//! Physical production planning for a single day. Inputs come exclusively from
//! opening stocks: today's outputs become inputs on the next day. This explicit
//! boundary prevents circular recipes or iteration order from creating goods.
//! This planner does not settle wages, purchases, consumption or investment.
use crate::{GoodId, Population, stable_hash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A production establishment/aggregate is not a commodity identity.
/// ```compile_fail
/// use sim_core::{GoodId, SectorId};
/// let sector: SectorId = GoodId("food".into());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct SectorId(pub String);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CapitalKindId(pub String);
/// Worker-days are a flow of labor service, not resident people.
/// ```compile_fail
/// use sim_core::{LaborDays, Population};
/// let work: LaborDays = Population(100);
/// ```
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct LaborDays(pub u64);
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CapitalServices(pub u64);
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ProductionBatches(pub u64);
/// Counts are meaningful only with their catalogue good/unit. Never sum these
/// values across different goods; the stock and flow maps retain the good key.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ProductionQuantity(pub u64);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndustryKind {
    Agriculture,
    Extraction,
    Manufacturing,
    Construction,
    Services,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionGood {
    pub name: String,
    pub unit: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectorRecipe {
    pub name: String,
    pub industry: IndustryKind,
    /// Smaller numbers receive scarce resources first; ties use stable IDs.
    pub priority: u16,
    pub labor_per_batch: LaborDays,
    pub capital_per_batch: BTreeMap<CapitalKindId, CapitalServices>,
    pub inputs_per_batch: BTreeMap<GoodId, ProductionQuantity>,
    pub outputs_per_batch: BTreeMap<GoodId, ProductionQuantity>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectorProductionDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub goods: BTreeMap<GoodId, ProductionGood>,
    pub capital_kinds: BTreeSet<CapitalKindId>,
    pub sectors: BTreeMap<SectorId, SectorRecipe>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectorCapacity {
    pub installed_batches: ProductionBatches,
    pub requested_batches: ProductionBatches,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProductionConstraint {
    InstalledCapacity,
    Labor,
    Capital(CapitalKindId),
    Input(GoodId),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionLimit {
    pub constraint: ProductionConstraint,
    pub maximum_batches: ProductionBatches,
    /// Binding means this constraint prevented fulfilling the request. Several
    /// constraints can bind simultaneously, including a zero available input.
    pub binding: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectorProduction {
    pub sector: SectorId,
    pub requested: ProductionBatches,
    pub produced: ProductionBatches,
    pub labor_used: LaborDays,
    pub capital_used: BTreeMap<CapitalKindId, CapitalServices>,
    pub inputs: BTreeMap<GoodId, ProductionQuantity>,
    pub outputs: BTreeMap<GoodId, ProductionQuantity>,
    pub limits: Vec<ProductionLimit>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionGoodFlow {
    pub opening: ProductionQuantity,
    pub consumed: ProductionQuantity,
    pub produced: ProductionQuantity,
    /// Before storage losses, trade or final consumption, all separate phases.
    pub closing: ProductionQuantity,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionDay {
    /// Actual allocation order is explicit and independent of display labels.
    pub sectors: Vec<SectorProduction>,
    pub goods: BTreeMap<GoodId, ProductionGoodFlow>,
    pub labor_available: LaborDays,
    pub labor_unused: LaborDays,
    pub capital_available: BTreeMap<CapitalKindId, CapitalServices>,
    pub capital_unused: BTreeMap<CapitalKindId, CapitalServices>,
}
fn atom(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b':')
        })
}
impl SectorProductionDefinition {
    /// Exact deterministic definition identity, including units and provenance.
    /// A stock cannot be reinterpreted under a different recipe/catalogue.
    pub fn hash(&self) -> u64 {
        stable_hash(&serde_json::to_vec(self).expect("production definitions serialize"))
    }
    /// Recompute the complete receipt from its opening resources. This rejects
    /// forged stock/factor flows and changed allocation order; callers retain
    /// the separately attributed definition, installed capacities and workers.
    pub fn validate_day(
        &self,
        day: &ProductionDay,
        capacity: &BTreeMap<SectorId, SectorCapacity>,
        workers: Population,
    ) -> Result<(), String> {
        let opening = day
            .goods
            .iter()
            .map(|(id, flow)| (id.clone(), flow.opening))
            .collect();
        if &self.plan_day(
            capacity,
            &opening,
            workers,
            day.labor_available,
            &day.capital_available,
        )? != day
        {
            return Err("Production receipt differs from its opening resources and recipes".into());
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.source.trim().is_empty()
            || self.source.len() > 256
            || self.source_version.trim().is_empty()
            || self.source_version.len() > 128
            || self.description.trim().is_empty()
            || self.description.len() > 4096
            || self.goods.is_empty()
            || self.goods.len() > 128
            || self.capital_kinds.len() > 64
            || self.sectors.is_empty()
            || self.sectors.len() > 128
        {
            return Err("Production requires bounded explicit catalogues and provenance".into());
        }
        for (id, good) in &self.goods {
            if !atom(&id.0)
                || good.name.trim().is_empty()
                || good.name.len() > 256
                || good.unit.trim().is_empty()
                || good.unit.len() > 128
            {
                return Err("Invalid production good identity, name or unit".into());
            }
        }
        if self.capital_kinds.iter().any(|id| !atom(&id.0)) {
            return Err("Invalid production capital identity".into());
        }
        for (id, sector) in &self.sectors {
            if !atom(&id.0)
                || sector.name.trim().is_empty()
                || sector.name.len() > 256
                || sector.outputs_per_batch.is_empty()
                || sector.labor_per_batch.0 == 0
                    && sector.capital_per_batch.is_empty()
                    && sector.inputs_per_batch.is_empty()
                || sector
                    .capital_per_batch
                    .iter()
                    .any(|(id, q)| q.0 == 0 || !self.capital_kinds.contains(id))
                || sector
                    .inputs_per_batch
                    .iter()
                    .chain(&sector.outputs_per_batch)
                    .any(|(id, q)| q.0 == 0 || !self.goods.contains_key(id))
            {
                return Err("Invalid production recipe or unbounded free output".into());
            }
        }
        Ok(())
    }
    /// Pure planning: failure, including overflow, cannot alter input stocks.
    /// At most one worker-day per available modeled worker is permitted.
    pub fn plan_day(
        &self,
        capacity: &BTreeMap<SectorId, SectorCapacity>,
        opening: &BTreeMap<GoodId, ProductionQuantity>,
        workers: Population,
        labor: LaborDays,
        capital: &BTreeMap<CapitalKindId, CapitalServices>,
    ) -> Result<ProductionDay, String> {
        self.validate()?;
        if capacity.keys().ne(self.sectors.keys())
            || opening.keys().ne(self.goods.keys())
            || capital.keys().ne(self.capital_kinds.iter())
            || labor.0 > workers.0
        {
            return Err(
                "Production needs exact catalogue coverage and available worker-days".into(),
            );
        }
        let mut available = opening.clone();
        let mut unused_labor = labor;
        let mut unused_capital = capital.clone();
        let mut result = ProductionDay {
            sectors: vec![],
            goods: opening
                .iter()
                .map(|(id, q)| {
                    (
                        id.clone(),
                        ProductionGoodFlow {
                            opening: *q,
                            consumed: ProductionQuantity(0),
                            produced: ProductionQuantity(0),
                            closing: ProductionQuantity(0),
                        },
                    )
                })
                .collect(),
            labor_available: labor,
            labor_unused: labor,
            capital_available: capital.clone(),
            capital_unused: capital.clone(),
        };
        let mut order: Vec<_> = self.sectors.iter().collect();
        order.sort_by_key(|(id, recipe)| (recipe.priority, *id));
        for (id, recipe) in order {
            let request = &capacity[id];
            let mut limits = vec![(
                ProductionConstraint::InstalledCapacity,
                request.installed_batches.0,
            )];
            if let Some(maximum) = unused_labor.0.checked_div(recipe.labor_per_batch.0) {
                limits.push((ProductionConstraint::Labor, maximum));
            }
            for (kind, amount) in &recipe.capital_per_batch {
                limits.push((
                    ProductionConstraint::Capital(kind.clone()),
                    unused_capital[kind].0 / amount.0,
                ));
            }
            for (good, amount) in &recipe.inputs_per_batch {
                limits.push((
                    ProductionConstraint::Input(good.clone()),
                    available[good].0 / amount.0,
                ));
            }
            let batches = limits
                .iter()
                .fold(request.requested_batches.0, |n, (_, max)| n.min(*max));
            let scale = |n: u64| n.checked_mul(batches).ok_or("Production quantity overflow");
            let labor_used = LaborDays(scale(recipe.labor_per_batch.0)?);
            unused_labor.0 -= labor_used.0;
            let mut capital_used = BTreeMap::new();
            for (kind, per_batch) in &recipe.capital_per_batch {
                let used = CapitalServices(scale(per_batch.0)?);
                unused_capital.get_mut(kind).expect("validated capital").0 -= used.0;
                capital_used.insert(kind.clone(), used);
            }
            let mut inputs = BTreeMap::new();
            for (good, per_batch) in &recipe.inputs_per_batch {
                let used = ProductionQuantity(scale(per_batch.0)?);
                available.get_mut(good).expect("validated good").0 -= used.0;
                let flow = result.goods.get_mut(good).expect("validated good");
                flow.consumed.0 = flow
                    .consumed
                    .0
                    .checked_add(used.0)
                    .ok_or("Production input overflow")?;
                inputs.insert(good.clone(), used);
            }
            let mut outputs = BTreeMap::new();
            for (good, per_batch) in &recipe.outputs_per_batch {
                let produced = ProductionQuantity(scale(per_batch.0)?);
                let flow = result.goods.get_mut(good).expect("validated good");
                flow.produced.0 = flow
                    .produced
                    .0
                    .checked_add(produced.0)
                    .ok_or("Production output overflow")?;
                outputs.insert(good.clone(), produced);
            }
            result.sectors.push(SectorProduction {
                sector: id.clone(),
                requested: request.requested_batches,
                produced: ProductionBatches(batches),
                labor_used,
                capital_used,
                inputs,
                outputs,
                limits: limits
                    .into_iter()
                    .map(|(constraint, maximum)| ProductionLimit {
                        constraint,
                        maximum_batches: ProductionBatches(maximum),
                        binding: batches < request.requested_batches.0 && batches == maximum,
                    })
                    .collect(),
            });
        }
        for flow in result.goods.values_mut() {
            flow.closing.0 = flow
                .opening
                .0
                .checked_sub(flow.consumed.0)
                .and_then(|q| q.checked_add(flow.produced.0))
                .ok_or("Production stock reconciliation overflow")?;
        }
        result.labor_unused = unused_labor;
        result.capital_unused = unused_capital;
        Ok(result)
    }
}
