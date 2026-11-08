//! Authored auxiliary infrastructure production, separate from the existing
//! national civilian Food/Energy commodities and their settled inventories.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionCountryOpening {
    /// Dimensionless, immutable authored calibration of common recipe labor.
    pub labor_requirement_multiplier: u64,
    pub installed: BTreeMap<SectorId, ProductionBatches>,
    pub capital: BTreeMap<CapitalKindId, CapitalServices>,
    pub stocks: BTreeMap<GoodId, ProductionQuantity>,
    pub storage: BTreeMap<GoodId, ProductionStorageRule>,
    pub service_units_per_percent: ProductionQuantity,
    pub maintenance_units_per_step: ProductionQuantity,
    pub construction_unit: ProductionQuantity,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub framework: SectorProductionDefinition,
    pub assembly_good: GoodId,
    pub service_good: GoodId,
    pub service_sector: SectorId,
    /// Share of actual employed residents reserved for the auxiliary network;
    /// remaining workers belong to the older abstract civilian economy.
    pub worker_share: BasisPoints,
    pub construction_policy_weights: BTreeMap<PolicyId, u32>,
    pub countries: BTreeMap<CountryId, ProductionCountryOpening>,
}
impl ProductionDefinition {
    pub fn framework_for(&self, id: &CountryId) -> Result<SectorProductionDefinition, String> {
        let country = self.countries.get(id).ok_or("Missing production country")?;
        let mut framework = self.framework.clone();
        for recipe in framework.sectors.values_mut() {
            recipe.labor_per_batch.0 = recipe
                .labor_per_batch
                .0
                .checked_mul(country.labor_requirement_multiplier)
                .ok_or("Production labor calibration overflow")?;
        }
        framework.validate()?;
        Ok(framework)
    }
    pub fn construction_required(
        &self,
        id: &CountryId,
        policy: &PolicyId,
    ) -> Result<Option<ProductionQuantity>, String> {
        let Some(weight) = self.construction_policy_weights.get(policy) else {
            return Ok(None);
        };
        let country = self.countries.get(id).ok_or("Missing production country")?;
        country
            .construction_unit
            .0
            .checked_mul(u64::from(*weight))
            .map(ProductionQuantity)
            .map(Some)
            .ok_or_else(|| "Construction materials quote overflow".into())
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        self.framework.validate()?;
        if self.source != self.framework.source
            || self.source_version != self.framework.source_version
            || self.description.trim().is_empty()
            || self.description.len() > 4096
            || !content
                .sources
                .iter()
                .any(|source| source.id == self.source)
            || !(1..=9000).contains(&self.worker_share.units())
            || self
                .framework
                .sectors
                .values()
                .any(|recipe| recipe.labor_per_batch.0 == 0)
            || self.assembly_good == self.service_good
            || !self.framework.goods.contains_key(&self.assembly_good)
            || !self.framework.goods.contains_key(&self.service_good)
            || self.countries.keys().ne(content
                .countries
                .iter()
                .map(|c| &c.id)
                .collect::<std::collections::BTreeSet<_>>())
            || self.construction_policy_weights.is_empty()
            || self.construction_policy_weights.iter().any(|(id, weight)| {
                *weight == 0
                    || *weight > 100000
                    || content.policy(id).is_none()
                    || !matches!(id.as_str(), "energy" | "agriculture" | "administration")
            })
        {
            return Err("Production definitions need exact attributed country/good/policy coverage and bounded worker allocation".into());
        }
        let service = self
            .framework
            .sectors
            .get(&self.service_sector)
            .ok_or("Missing service sector")?;
        if service.industry != IndustryKind::Services
            || !service.outputs_per_batch.contains_key(&self.service_good)
            || self
                .framework
                .sectors
                .iter()
                .filter(|(_, recipe)| recipe.outputs_per_batch.contains_key(&self.service_good))
                .count()
                != 1
            || !self.framework.sectors.values().any(|recipe| {
                recipe.industry == IndustryKind::Construction
                    && recipe.outputs_per_batch.contains_key(&self.assembly_good)
            })
            || self
                .framework
                .sectors
                .values()
                .any(|recipe| recipe.inputs_per_batch.contains_key(&self.assembly_good))
        {
            return Err(
                "Service and construction bindings require their explicit production recipes"
                    .into(),
            );
        }
        for (id, country) in &self.countries {
            let framework = self.framework_for(id)?;
            if country.labor_requirement_multiplier == 0
                || country.installed.keys().ne(framework.sectors.keys())
                || country.capital.keys().ne(framework.capital_kinds.iter())
                || country.stocks.keys().ne(framework.goods.keys())
                || country.storage.keys().ne(framework.goods.keys())
                || country.service_units_per_percent.0 == 0
                || country.maintenance_units_per_step.0 == 0
                || country.construction_unit.0 == 0
                || country
                    .service_units_per_percent
                    .0
                    .checked_mul(100)
                    .is_none()
                || country
                    .maintenance_units_per_step
                    .0
                    .checked_mul(4000)
                    .is_none()
                || country
                    .storage
                    .values()
                    .any(|rule| rule.loss_bps.units() > 10000)
                || country.storage[&self.service_good].capacity.0 != 0
                || country.storage[&self.service_good].loss_bps.units() != 0
                || country
                    .stocks
                    .iter()
                    .any(|(good, q)| q.0 > country.storage[good].capacity.0)
            {
                return Err(
                    "Production country assets/stocks/storage do not match the pinned framework"
                        .into(),
                );
            }
            for policy in self.construction_policy_weights.keys() {
                if self
                    .construction_required(id, policy)?
                    .expect("known policy")
                    .0
                    > country.storage[&self.assembly_good].capacity.0
                {
                    return Err(
                        "A construction quote exceeds its entire physical storage capacity".into(),
                    );
                }
            }
            ProductionInventory::new(&framework, content.start_day()?, country.stocks.clone())?;
        }
        Ok(())
    }
}
