//! Reconciled physical inventory around the production planner. Final use is
//! supplied by the later consumption/construction/trade phases, not generated
//! from desired demand or expected cash. This module creates no money or jobs.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionStorageRule {
    /// Zero capacity permits same-day service use; unused output is discarded.
    pub capacity: ProductionQuantity,
    pub loss_bps: BasisPoints,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionStockTotals {
    pub produced: ProductionQuantity,
    pub imported: ProductionQuantity,
    pub exported: ProductionQuantity,
    pub industrial_use: ProductionQuantity,
    pub final_use: ProductionQuantity,
    pub discarded: ProductionQuantity,
    pub spoiled: ProductionQuantity,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionStockPosition {
    pub quantity: ProductionQuantity,
    /// Carried fractional loss in ten-thousandths of this good's own unit.
    /// This is not a probability or a share of a different commodity.
    pub loss_carry: u32,
    pub totals: ProductionStockTotals,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionInventory {
    pub definition_hash: u64,
    pub start_day: CampaignDay,
    pub day: CampaignDay,
    pub opening: BTreeMap<GoodId, ProductionQuantity>,
    pub positions: BTreeMap<GoodId, ProductionStockPosition>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionDayRequest {
    pub capacities: BTreeMap<SectorId, SectorCapacity>,
    pub workers: Population,
    pub labor: LaborDays,
    pub capital: BTreeMap<CapitalKindId, CapitalServices>,
    /// Actual paid external transfers, supplied by the cash/trade phase. The
    /// inventory engine neither finances nor authorizes them. Imports arrive
    /// after production, so recipes cannot spend them earlier in this day.
    pub imports: BTreeMap<GoodId, ProductionQuantity>,
    pub exports: BTreeMap<GoodId, ProductionQuantity>,
    /// Actual funded/deliverable final use. Unmet desired demand belongs in a
    /// separate report, and cannot silently overdraw physical stocks.
    pub final_use: BTreeMap<GoodId, ProductionQuantity>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionStorageFlow {
    pub after_production: ProductionQuantity,
    pub imports: ProductionQuantity,
    pub exports: ProductionQuantity,
    pub final_use: ProductionQuantity,
    pub discarded: ProductionQuantity,
    pub spoiled: ProductionQuantity,
    pub closing: ProductionQuantity,
    pub opening_loss_carry: u32,
    pub closing_loss_carry: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductionInventoryDay {
    pub day: CampaignDay,
    pub production: ProductionDay,
    pub storage: BTreeMap<GoodId, ProductionStorageFlow>,
}
fn add_quantity(
    a: ProductionQuantity,
    b: ProductionQuantity,
) -> Result<ProductionQuantity, String> {
    a.0.checked_add(b.0)
        .map(ProductionQuantity)
        .ok_or_else(|| "Cumulative production-stock flow overflow".into())
}
impl ProductionInventory {
    /// Consume already stored goods after this day's settlement, for example
    /// an actually funded capital delivery. This is a separate physical phase;
    /// it advances no clock and cannot spend tomorrow's output.
    pub fn withdraw_final(
        &self,
        definition: &SectorProductionDefinition,
        amounts: &BTreeMap<GoodId, ProductionQuantity>,
    ) -> Result<Self, String> {
        self.validate(definition)?;
        if amounts.keys().any(|id| !definition.goods.contains_key(id))
            || self.day == self.start_day && amounts.values().any(|q| q.0 > 0)
        {
            return Err("Capital use requires known goods and a completed daily settlement".into());
        }
        let mut result = self.clone();
        for (id, quantity) in amounts {
            let stock = result.positions.get_mut(id).expect("validated good");
            stock.quantity.0 = stock
                .quantity
                .0
                .checked_sub(quantity.0)
                .ok_or("Capital use exceeds delivered physical stocks")?;
            stock.totals.final_use = add_quantity(stock.totals.final_use, *quantity)?;
            if stock.quantity.0 == 0 {
                stock.loss_carry = 0;
            }
        }
        result.validate(definition)?;
        Ok(result)
    }
    pub fn new(
        definition: &SectorProductionDefinition,
        day: CampaignDay,
        opening: BTreeMap<GoodId, ProductionQuantity>,
    ) -> Result<Self, String> {
        let result = Self {
            definition_hash: definition.hash(),
            start_day: day,
            day,
            positions: opening
                .iter()
                .map(|(id, q)| {
                    (
                        id.clone(),
                        ProductionStockPosition {
                            quantity: *q,
                            loss_carry: 0,
                            totals: Default::default(),
                        },
                    )
                })
                .collect(),
            opening,
        };
        result.validate(definition)?;
        Ok(result)
    }
    pub fn validate(&self, definition: &SectorProductionDefinition) -> Result<(), String> {
        definition.validate()?;
        if self.definition_hash != definition.hash()
            || self.day.units() > MAX_DAY
            || self.start_day > self.day
            || self.opening.keys().ne(definition.goods.keys())
            || self.positions.keys().ne(definition.goods.keys())
        {
            return Err("Production inventory requires the exact dated commodity catalogue".into());
        }
        for (id, position) in &self.positions {
            let totals = &position.totals;
            let incoming = u128::from(self.opening[id].0)
                + u128::from(totals.produced.0)
                + u128::from(totals.imported.0);
            let outgoing = u128::from(position.quantity.0)
                + u128::from(totals.industrial_use.0)
                + u128::from(totals.exported.0)
                + u128::from(totals.final_use.0)
                + u128::from(totals.discarded.0)
                + u128::from(totals.spoiled.0);
            if incoming != outgoing
                || position.loss_carry >= 10000
                || position.quantity.0 == 0 && position.loss_carry != 0
                || self.day == self.start_day
                    && (position.quantity != self.opening[id]
                        || position.loss_carry != 0
                        || position.totals != ProductionStockTotals::default())
            {
                return Err(
                    "Production stock/cumulative flow or fractional loss does not reconcile".into(),
                );
            }
        }
        Ok(())
    }
    /// Plan and commit one complete day into a new value. Errors cannot leave
    /// a partially consumed good, advanced date or modified loss carry behind.
    /// Final use precedes storage-capacity disposal and fractional spoilage.
    pub fn apply_day(
        &self,
        definition: &SectorProductionDefinition,
        storage: &BTreeMap<GoodId, ProductionStorageRule>,
        request: &ProductionDayRequest,
    ) -> Result<(Self, ProductionInventoryDay), String> {
        self.validate(definition)?;
        if storage.keys().ne(definition.goods.keys())
            || request.final_use.keys().ne(definition.goods.keys())
            || request.imports.keys().ne(definition.goods.keys())
            || request.exports.keys().ne(definition.goods.keys())
            || storage.values().any(|rule| rule.loss_bps.units() > 10000)
        {
            return Err("Production final-use/storage rules require exact bounded coverage".into());
        }
        let next_day = self
            .day
            .checked_add(DaySpan::new(1))
            .filter(|day| day.units() <= MAX_DAY)
            .ok_or("Production inventory calendar exhausted")?;
        let quantities = self
            .positions
            .iter()
            .map(|(id, position)| (id.clone(), position.quantity))
            .collect();
        let production = definition.plan_day(
            &request.capacities,
            &quantities,
            request.workers,
            request.labor,
            &request.capital,
        )?;
        let mut result = self.clone();
        let mut storage_flows = BTreeMap::new();
        for (id, flow) in &production.goods {
            let final_use = request.final_use[id];
            let imports = request.imports[id];
            let exports = request.exports[id];
            let after_imports = flow
                .closing
                .0
                .checked_add(imports.0)
                .ok_or("Production import stock overflow")?;
            let after_exports = after_imports
                .checked_sub(exports.0)
                .ok_or("Exports exceed physical production stocks")?;
            let remaining = after_exports
                .checked_sub(final_use.0)
                .ok_or("Final use exceeds physical production stocks")?;
            let rule = &storage[id];
            let discarded = ProductionQuantity(remaining.saturating_sub(rule.capacity.0));
            let stored = remaining.min(rule.capacity.0);
            let position = result.positions.get_mut(id).expect("validated stock");
            let opening_loss_carry = position.loss_carry;
            let loss_numerator = u128::from(stored) * u128::from(rule.loss_bps.units())
                + u128::from(opening_loss_carry);
            let spoiled = ProductionQuantity(
                u64::try_from(loss_numerator / 10000).map_err(|_| "Storage loss overflow")?,
            );
            let closing =
                ProductionQuantity(stored.checked_sub(spoiled.0).ok_or("Excess storage loss")?);
            let closing_loss_carry = if closing.0 == 0 {
                0
            } else {
                (loss_numerator % 10000) as u32
            };
            position.quantity = closing;
            position.loss_carry = closing_loss_carry;
            let totals = &mut position.totals;
            totals.produced = add_quantity(totals.produced, flow.produced)?;
            totals.imported = add_quantity(totals.imported, imports)?;
            totals.exported = add_quantity(totals.exported, exports)?;
            totals.industrial_use = add_quantity(totals.industrial_use, flow.consumed)?;
            totals.final_use = add_quantity(totals.final_use, final_use)?;
            totals.discarded = add_quantity(totals.discarded, discarded)?;
            totals.spoiled = add_quantity(totals.spoiled, spoiled)?;
            storage_flows.insert(
                id.clone(),
                ProductionStorageFlow {
                    after_production: flow.closing,
                    imports,
                    exports,
                    final_use,
                    discarded,
                    spoiled,
                    closing,
                    opening_loss_carry,
                    closing_loss_carry,
                },
            );
        }
        result.day = next_day;
        result.validate(definition)?;
        Ok((
            result,
            ProductionInventoryDay {
                day: next_day,
                production,
                storage: storage_flows,
            },
        ))
    }
}
