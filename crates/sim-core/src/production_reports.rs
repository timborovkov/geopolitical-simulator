//! Normal owner projections and public definitions for auxiliary infrastructure.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstructionMaterialPlan {
    pub country: CountryId,
    pub day: CampaignDay,
    pub policy: PolicyId,
    pub good: GoodId,
    pub name: String,
    pub unit: String,
    pub required: ProductionQuantity,
    /// Current stock, not a reservation or guaranteed future delivery.
    pub available: ProductionQuantity,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingConstruction {
    pub commitment: CommitmentId,
    pub policy: PolicyId,
    pub name: String,
    pub stage: PolicyStage,
    pub earliest_day: CampaignDay,
    pub materials: ConstructionMaterialPlan,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionSectorReference {
    pub id: SectorId,
    pub name: String,
    pub industry: IndustryKind,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnedProductionReport {
    pub inventory: ProductionInventory,
    pub storage: BTreeMap<GoodId, ProductionStorageRule>,
    pub worker_share: BasisPoints,
    pub last: Option<NationalProductionDay>,
    pub pending: Vec<PendingConstruction>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionReport {
    pub country: CountryId,
    pub day: CampaignDay,
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub goods: BTreeMap<GoodId, ProductionGood>,
    pub sectors: Vec<ProductionSectorReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned: Option<OwnedProductionReport>,
}
impl World {
    fn check_production_content(&self, content: &Content) -> Result<(), String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Production inspection requires the campaign's exact content".into());
        }
        Ok(())
    }
    pub fn construction_material_plan(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &CountryId,
        policy: &PolicyId,
    ) -> Result<Option<ConstructionMaterialPlan>, String> {
        self.check_production_content(content)?;
        if id != &self.player || viewer != Viewer::Government(self.player.clone()) {
            return Err(
                "Construction material planning is private to the controlled government".into(),
            );
        }
        content
            .policy(policy)
            .ok_or("Unknown construction programme")?;
        let Some(definition) = &content.production else {
            return Ok(None);
        };
        let Some(required) = definition.construction_required(id, policy)? else {
            return Ok(None);
        };
        let available = self
            .construction_available(content, id)
            .ok_or("Missing construction inventory")?;
        let good = &definition.framework.goods[&definition.assembly_good];
        Ok(Some(ConstructionMaterialPlan {
            country: id.clone(),
            day: self.day,
            policy: policy.clone(),
            good: definition.assembly_good.clone(),
            name: good.name.clone(),
            unit: good.unit.clone(),
            required,
            available,
        }))
    }
    /// Foreign/observer views expose scenario definitions without private live
    /// inventory, capacity, labor, funding or pending-project information.
    pub fn production_report(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &CountryId,
    ) -> Result<Option<ProductionReport>, String> {
        self.check_production_content(content)?;
        let country = self.countries.get(id).ok_or("Unknown production country")?;
        let Some(definition) = &content.production else {
            return Ok(None);
        };
        let owned = if id == &self.player && viewer == Viewer::Government(self.player.clone()) {
            let state = self
                .production
                .as_ref()
                .ok_or("Missing production state")?
                .countries
                .get(id)
                .ok_or("Missing production country")?;
            let mut pending = Vec::new();
            for project in &country.projects {
                if let Some(materials) =
                    self.construction_material_plan(content, viewer.clone(), id, &project.policy)?
                {
                    pending.push(PendingConstruction {
                        commitment: project.id,
                        policy: project.policy.clone(),
                        name: content
                            .policy(&project.policy)
                            .ok_or("Unknown construction programme")?
                            .name
                            .clone(),
                        stage: project.stage.clone(),
                        earliest_day: project.due_day,
                        materials,
                    });
                }
            }
            pending.sort_by_key(|project| project.commitment);
            Some(OwnedProductionReport {
                inventory: state.inventory.clone(),
                storage: definition.countries[id].storage.clone(),
                worker_share: definition.worker_share,
                last: state.last.clone(),
                pending,
            })
        } else {
            None
        };
        Ok(Some(ProductionReport {
            country: id.clone(),
            day: self.day,
            source: definition.source.clone(),
            source_version: definition.source_version.clone(),
            description: definition.description.clone(),
            goods: definition.framework.goods.clone(),
            sectors: definition
                .framework
                .sectors
                .iter()
                .map(|(id, sector)| ProductionSectorReference {
                    id: id.clone(),
                    name: sector.name.clone(),
                    industry: sector.industry,
                })
                .collect(),
            owned,
        }))
    }
}
