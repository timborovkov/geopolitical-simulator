//! Paid public auxiliary-stock trade with an explicitly bounded external counterparty.
//! Prices are authored per country-specific lot; this is not a private-firm profit model.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndustrialTradeDirection {
    Import,
    Export,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct IndustrialTradeId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustrialTradeGood {
    pub ask_bps: BasisPoints,
    pub bid_bps: BasisPoints,
    pub import_lots_per_day: u32,
    pub export_lots_per_day: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustrialTradeDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub order_days: DaySpan,
    pub fee_bps: BasisPoints,
    pub price_lots: u32,
    pub maximum_orders: u32,
    pub goods: BTreeMap<GoodId, IndustrialTradeGood>,
}
impl IndustrialTradeDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        let production = content
            .production
            .as_ref()
            .ok_or("Industrial trade requires production")?;
        if self.description.trim().is_empty()
            || self.description.len() > 4096
            || self.source_version.trim().is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
            || !(1..=90).contains(&self.order_days.units())
            || !(1..=100).contains(&self.fee_bps.units())
            || !(1..=100).contains(&self.price_lots)
            || !(1..=256).contains(&self.maximum_orders)
            || self.goods.is_empty()
            || self.goods.len() > 16
        {
            return Err(
                "Industrial trade needs attributed, bounded price and contract definitions".into(),
            );
        }
        for (good, rule) in &self.goods {
            if !production.framework.goods.contains_key(good)
                || good == &production.service_good
                || !production
                    .framework
                    .sectors
                    .values()
                    .any(|r| r.outputs_per_batch.contains_key(good))
                || rule.bid_bps.units() == 0
                || rule.bid_bps.units() >= rule.ask_bps.units()
                || rule.ask_bps.units() > 1000
                || !(1..=100).contains(&rule.import_lots_per_day)
                || !(1..=100).contains(&rule.export_lots_per_day)
            {
                return Err("Industrial trade requires storable produced goods, positive spread and bounded quotas".into());
            }
            for country in &content.countries {
                let opening = production
                    .countries
                    .get(&country.id)
                    .ok_or("Missing industrial trade production country")?;
                if country.gdp.units() <= 0
                    || opening.storage.get(good).is_none_or(|s| s.capacity.0 == 0)
                {
                    return Err("Industrial trade requires positive GDP and storage".into());
                }
                if self
                    .amount(
                        content,
                        &country.id,
                        good,
                        IndustrialTradeDirection::Import,
                        opening.storage[good].capacity,
                    )?
                    .0
                    > 1_000_000_000_000
                {
                    return Err("Industrial quote exceeds the accounting range".into());
                }
                opening
                    .construction_unit
                    .0
                    .checked_mul(u64::from(
                        rule.import_lots_per_day.max(rule.export_lots_per_day),
                    ))
                    .ok_or("Industrial trade quota overflow")?;
            }
        }
        Ok(())
    }
    pub fn amount(
        &self,
        content: &Content,
        id: &CountryId,
        good: &GoodId,
        direction: IndustrialTradeDirection,
        quantity: ProductionQuantity,
    ) -> Result<Money, String> {
        let rule = self
            .goods
            .get(good)
            .ok_or("This good has no external industrial market")?;
        let unit = content
            .production
            .as_ref()
            .ok_or("Missing production")?
            .countries
            .get(id)
            .ok_or("Unknown trade country")?
            .construction_unit
            .0;
        let rate = match direction {
            IndustrialTradeDirection::Import => rule.ask_bps.units(),
            IndustrialTradeDirection::Export => rule.bid_bps.units(),
        };
        let numerator = u128::try_from(
            content
                .country(id)
                .ok_or("Unknown trade country")?
                .gdp
                .units(),
        )
        .map_err(|_| "Invalid trade GDP")?
        .checked_mul(u128::from(rate))
        .and_then(|n| n.checked_mul(u128::from(quantity.0)))
        .ok_or("Industrial quote overflow")?;
        let denominator = u128::from(unit) * u128::from(self.price_lots) * 10000;
        if denominator == 0 {
            return Err("Industrial quote needs a positive unit".into());
        }
        let units = match direction {
            IndustrialTradeDirection::Import => numerator.div_ceil(denominator),
            IndustrialTradeDirection::Export => numerator / denominator,
        };
        i64::try_from(units)
            .map(Money)
            .map_err(|_| "Industrial cash quote overflow".into())
    }
    pub fn daily_limit(
        &self,
        content: &Content,
        id: &CountryId,
        good: &GoodId,
        direction: IndustrialTradeDirection,
    ) -> Result<ProductionQuantity, String> {
        let rule = self
            .goods
            .get(good)
            .ok_or("Unknown industrial trade good")?;
        let lots = match direction {
            IndustrialTradeDirection::Import => rule.import_lots_per_day,
            IndustrialTradeDirection::Export => rule.export_lots_per_day,
        };
        content
            .production
            .as_ref()
            .ok_or("Missing production")?
            .countries
            .get(id)
            .ok_or("Unknown trade country")?
            .construction_unit
            .0
            .checked_mul(u64::from(lots))
            .map(ProductionQuantity)
            .ok_or("Industrial quota overflow".into())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum IndustrialTradeStatus {
    Pending,
    Active,
    Completed,
    Cancelled,
    Expired,
    Refused { reason: String },
}
impl IndustrialTradeStatus {
    pub fn open(&self) -> bool {
        matches!(self, Self::Pending | Self::Active)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustrialTradeFill {
    pub day: CampaignDay,
    pub quantity: ProductionQuantity,
    pub cash: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustrialTradeOrder {
    pub id: IndustrialTradeId,
    pub good: GoodId,
    pub direction: IndustrialTradeDirection,
    pub quantity: ProductionQuantity,
    pub quoted_cash: Money,
    pub fee: Money,
    pub reserved: Money,
    pub proposed_day: CampaignDay,
    pub review_day: CampaignDay,
    pub expiry_day: CampaignDay,
    pub sponsor: GovernmentId,
    pub status: IndustrialTradeStatus,
    pub resolved_day: Option<CampaignDay>,
    pub fills: Vec<IndustrialTradeFill>,
}
impl IndustrialTradeOrder {
    pub fn filled(&self) -> ProductionQuantity {
        ProductionQuantity(
            self.fills
                .iter()
                .fold(0_u64, |n, f| n.saturating_add(f.quantity.0)),
        )
    }
    pub fn cash(&self) -> Money {
        Money(
            self.fills
                .iter()
                .fold(0_i64, |n, f| n.saturating_add(f.cash.0)),
        )
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustrialTradeState {
    pub orders: Vec<IndustrialTradeOrder>,
}
impl IndustrialTradeState {
    pub fn reserved_cash(&self) -> Money {
        Money(
            self.orders
                .iter()
                .fold(0_i64, |n, o| n.saturating_add(o.reserved.0)),
        )
    }
    pub fn daily_cash(&self, day: CampaignDay, direction: IndustrialTradeDirection) -> Money {
        Money(
            self.orders
                .iter()
                .filter(|o| o.direction == direction)
                .flat_map(|o| &o.fills)
                .filter(|f| f.day == day)
                .fold(0_i64, |n, f| n.saturating_add(f.cash.0)),
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustrialTradePreview {
    pub good: GoodId,
    pub direction: IndustrialTradeDirection,
    pub quantity: ProductionQuantity,
    pub quoted_cash: Money,
    pub fee: Money,
    pub daily_limit: ProductionQuantity,
    pub current_stock: ProductionQuantity,
    pub review_day: CampaignDay,
    pub expiry_day: CampaignDay,
    pub route: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustrialTradeReport {
    pub country: CountryId,
    pub day: CampaignDay,
    pub definition: IndustrialTradeDefinition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned: Option<IndustrialTradeState>,
}
pub(crate) struct IndustrialTradeFlows {
    pub imports: BTreeMap<GoodId, ProductionQuantity>,
    pub exports: BTreeMap<GoodId, ProductionQuantity>,
}
impl World {
    pub fn industrial_trade_report(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &CountryId,
    ) -> Result<Option<IndustrialTradeReport>, String> {
        if self.content_hash != content.hash() {
            return Err("Industrial trade requires the campaign's exact content".into());
        }
        let country = self
            .countries
            .get(id)
            .ok_or("Unknown industrial trade country")?;
        let Some(definition) = &content.industrial_trade else {
            return Ok(None);
        };
        Ok(Some(IndustrialTradeReport {
            country: id.clone(),
            day: self.day,
            definition: definition.clone(),
            owned: (id == &self.player && viewer == Viewer::Government(self.player.clone()))
                .then(|| country.industrial_trade.clone().expect("trade state")),
        }))
    }
    pub fn preview_industrial_trade(
        &self,
        content: &Content,
        id: &CountryId,
        good: &GoodId,
        direction: IndustrialTradeDirection,
        quantity: ProductionQuantity,
    ) -> Result<IndustrialTradePreview, String> {
        if self.content_hash != content.hash() {
            return Err("Industrial trade requires the campaign's exact content".into());
        }
        let definition = content
            .industrial_trade
            .as_ref()
            .ok_or("This pack has no industrial trade procedure")?;
        let c = self.countries.get(id).ok_or("Unknown trade country")?;
        let state = c
            .industrial_trade
            .as_ref()
            .ok_or("Missing industrial trade state")?;
        let production = content.production.as_ref().ok_or("Missing production")?;
        let storage = production.countries[id]
            .storage
            .get(good)
            .ok_or("Unknown industrial good")?;
        if quantity.0 == 0 || quantity.0 > storage.capacity.0 {
            return Err(
                "Choose a positive order no larger than this good's storage capacity".into(),
            );
        }
        if state.orders.iter().filter(|o| o.status.open()).count()
            >= definition.maximum_orders as usize
        {
            return Err("Maximum simultaneous industrial contracts reached".into());
        }
        if state
            .orders
            .iter()
            .any(|o| &o.good == good && o.direction == direction && o.status.open())
        {
            return Err(
                "This good already has an open industrial contract in that direction".into(),
            );
        }
        let authority = self.governing_rules(content, id)?;
        authority.check(c, "industrial_trade", 1)?;
        let stock = self
            .production
            .as_ref()
            .ok_or("Missing production inventory")?
            .countries[id]
            .inventory
            .positions[good]
            .quantity;
        if direction == IndustrialTradeDirection::Export && quantity.0 > stock.0 {
            return Err("An export proposal cannot exceed currently observed stocks; stocks are not reserved".into());
        }
        let quoted_cash = definition.amount(content, id, good, direction, quantity)?;
        if quoted_cash.0 <= 0 {
            return Err("Choose a quantity with a positive whole-credit cash quote".into());
        }
        let fee = Money(
            (content.country(id).ok_or("Unknown country")?.gdp.units()
                * definition.fee_bps.units_i64()
                / 10000)
                .max(1),
        );
        let required = fee.0
            + if direction == IndustrialTradeDirection::Import {
                quoted_cash.0
            } else {
                0
            };
        if c.available_cash().0 < required {
            return Err(format!(
                "Industrial trade requires {required}m uncommitted cash, including {}m nonrefundable administration",
                fee.0
            ));
        }
        let review_day = self
            .day
            .checked_add(DaySpan::new(authority.consent_days.units().max(1)))
            .ok_or("Industrial review calendar overflow")?;
        let expiry_day = review_day
            .checked_add(DaySpan::new(definition.order_days.units() - 1))
            .filter(|d| d.units() <= MAX_DAY)
            .ok_or("Industrial contract exceeds campaign calendar")?;
        Ok(IndustrialTradePreview {
            good: good.clone(),
            direction,
            quantity,
            quoted_cash,
            fee,
            daily_limit: definition.daily_limit(content, id, good, direction)?,
            current_stock: stock,
            review_day,
            expiry_day,
            route: authority.label(),
        })
    }
    pub(crate) fn order_industrial_trade(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: IndustrialTradeId,
        good: &GoodId,
        direction: IndustrialTradeDirection,
        quantity: ProductionQuantity,
    ) -> Result<(), String> {
        let quote = self.preview_industrial_trade(content, id, good, direction, quantity)?;
        let paid = self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            quote.fee.0,
            &format!("Industrial trade administration #{}", request.0),
        );
        if paid != quote.fee.0 {
            return Err("Industrial administration could not be paid".into());
        }
        let c = self.countries.get_mut(id).expect("country");
        let reserved = if direction == IndustrialTradeDirection::Import {
            quote.quoted_cash
        } else {
            Money(0)
        };
        c.committed_cash.0 += reserved.0;
        c.budget.capital.0 += quote.fee.0;
        c.industrial_trade
            .as_mut()
            .expect("trade state")
            .orders
            .push(IndustrialTradeOrder {
                id: request,
                good: good.clone(),
                direction,
                quantity,
                quoted_cash: quote.quoted_cash,
                fee: quote.fee,
                reserved,
                proposed_day: self.day,
                review_day: quote.review_day,
                expiry_day: quote.expiry_day,
                sponsor: c.government_number,
                status: IndustrialTradeStatus::Pending,
                resolved_day: None,
                fills: Vec::new(),
            });
        self.event(id,true,format!("Industrial {direction:?} contract #{} proposed for {} {} units: {}m fee paid, {}m reserved; earliest settlement {} after ordinary consent. Stocks and foreign capacity are not promised deliveries.",request.0,quantity.0,good.0,quote.fee.0,reserved.0,date(quote.review_day)));
        Ok(())
    }
    fn close_industrial_trade(
        &mut self,
        id: &CountryId,
        request: IndustrialTradeId,
        status: IndustrialTradeStatus,
    ) -> Result<(), String> {
        let c = self.countries.get_mut(id).ok_or("Unknown trade country")?;
        let order = c
            .industrial_trade
            .as_mut()
            .ok_or("Missing industrial trade state")?
            .orders
            .iter_mut()
            .find(|o| o.id == request)
            .ok_or("Unknown industrial contract")?;
        if !order.status.open() {
            return Err("Industrial contract is already closed".into());
        }
        c.committed_cash.0 = c
            .committed_cash
            .0
            .checked_sub(order.reserved.0)
            .ok_or("Industrial reservation underflow")?;
        let released = order.reserved;
        order.reserved = Money(0);
        order.status = status.clone();
        order.resolved_day = Some(self.day);
        self.event(id,true,format!("Industrial contract #{} {status:?}: {}m unspent reservation released. Administration, completed deliveries and payments remain.",request.0,released.0));
        Ok(())
    }
    pub(crate) fn cancel_industrial_trade(
        &mut self,
        id: &CountryId,
        request: IndustrialTradeId,
    ) -> Result<(), String> {
        self.close_industrial_trade(id, request, IndustrialTradeStatus::Cancelled)
    }
    pub(crate) fn review_industrial_trade(
        &mut self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let Some(state) = &self.countries[id].industrial_trade else {
            return Ok(());
        };
        let orders = state
            .orders
            .iter()
            .filter(|o| o.status.open())
            .cloned()
            .collect::<Vec<_>>();
        for order in orders {
            if order.sponsor != self.countries[id].government_number {
                self.close_industrial_trade(
                    id,
                    order.id,
                    IndustrialTradeStatus::Refused {
                        reason: "Sponsoring government changed".into(),
                    },
                )?;
                continue;
            }
            if self.day > order.expiry_day {
                self.close_industrial_trade(id, order.id, IndustrialTradeStatus::Expired)?;
                continue;
            }
            if order.status == IndustrialTradeStatus::Pending && self.day >= order.review_day {
                if let Err(reason) = self.governing_rules(content, id)?.check(
                    &self.countries[id],
                    "industrial_trade",
                    1,
                ) {
                    self.close_industrial_trade(
                        id,
                        order.id,
                        IndustrialTradeStatus::Refused { reason },
                    )?;
                    continue;
                }
                self.countries
                    .get_mut(id)
                    .expect("country")
                    .industrial_trade
                    .as_mut()
                    .expect("trade")
                    .orders
                    .iter_mut()
                    .find(|o| o.id == order.id)
                    .expect("order")
                    .status = IndustrialTradeStatus::Active;
                self.event(id,true,format!("Industrial contract #{} received consent; actual fills still require cash, stocks, storage and remaining external quota.",order.id.0));
            }
        }
        Ok(())
    }
    pub(crate) fn settle_industrial_trade(
        &mut self,
        content: &Content,
        id: &CountryId,
        plan: &ProductionDay,
        final_use: &BTreeMap<GoodId, ProductionQuantity>,
    ) -> Result<IndustrialTradeFlows, String> {
        let production = content.production.as_ref().ok_or("Missing production")?;
        let mut imports: BTreeMap<_, _> = production
            .framework
            .goods
            .keys()
            .map(|g| (g.clone(), ProductionQuantity(0)))
            .collect();
        let mut exports = imports.clone();
        let Some(definition) = &content.industrial_trade else {
            return Ok(IndustrialTradeFlows { imports, exports });
        };
        let orders = self.countries[id]
            .industrial_trade
            .as_ref()
            .ok_or("Missing industrial trade state")?
            .orders
            .iter()
            .filter(|o| o.status == IndustrialTradeStatus::Active)
            .cloned()
            .collect::<Vec<_>>();
        let mut available: BTreeMap<_, _> = plan
            .goods
            .iter()
            .map(|(g, f)| {
                Ok((
                    g.clone(),
                    f.closing
                        .0
                        .checked_sub(final_use[g].0)
                        .ok_or("Industrial final-use priority exceeds available stock")?,
                ))
            })
            .collect::<Result<_, String>>()?;
        let protected = self.countries[id]
            .projects
            .iter()
            .filter(|p| p.stage == PolicyStage::Implementing && p.due_day <= self.day)
            .try_fold(0_u64, |n, p| {
                n.checked_add(
                    production
                        .construction_required(id, &p.policy)?
                        .map_or(0, |q| q.0),
                )
                .ok_or_else(|| "Protected industrial construction sum overflow".to_string())
            })?;
        for direction in [
            IndustrialTradeDirection::Import,
            IndustrialTradeDirection::Export,
        ] {
            for order in orders.iter().filter(|o| o.direction == direction) {
                let used = match direction {
                    IndustrialTradeDirection::Import => imports[&order.good].0,
                    IndustrialTradeDirection::Export => exports[&order.good].0,
                };
                let limit = definition
                    .daily_limit(content, id, &order.good, direction)?
                    .0
                    .saturating_sub(used);
                let physical = match direction {
                    IndustrialTradeDirection::Import => production.countries[id].storage
                        [&order.good]
                        .capacity
                        .0
                        .saturating_sub(available[&order.good]),
                    IndustrialTradeDirection::Export => available[&order.good].saturating_sub(
                        if order.good == production.assembly_good {
                            protected
                        } else {
                            0
                        },
                    ),
                };
                let mut quantity = (order.quantity.0 - order.filled().0)
                    .min(limit)
                    .min(physical);
                if quantity == 0 {
                    continue;
                }
                let opening_cash = order.cash().0;
                let filled = order.filled().0;
                let delta = |q| {
                    definition
                        .amount(
                            content,
                            id,
                            &order.good,
                            direction,
                            ProductionQuantity(filled + q),
                        )
                        .map(|m| m.0 - opening_cash)
                };
                let cash_available = if direction == IndustrialTradeDirection::Import {
                    order.reserved.0
                } else {
                    self.countries[id].accounts.external.0
                };
                let (mut low, mut high) = (0, quantity);
                while low < high {
                    let mid = low + (high - low).div_ceil(2);
                    if delta(mid)? <= cash_available {
                        low = mid;
                    } else {
                        high = mid - 1;
                    }
                }
                quantity = low;
                if quantity == 0 {
                    continue;
                }
                let cash = Money(delta(quantity)?);
                if direction == IndustrialTradeDirection::Import {
                    self.countries
                        .get_mut(id)
                        .expect("country")
                        .committed_cash
                        .0 -= cash.0;
                }
                let (from, to, reason) = match direction {
                    IndustrialTradeDirection::Import => (
                        Account::Treasury,
                        Account::External,
                        format!("Industrial import contract #{}", order.id.0),
                    ),
                    IndustrialTradeDirection::Export => (
                        Account::External,
                        Account::Treasury,
                        format!("Industrial export contract #{}", order.id.0),
                    ),
                };
                if self.pay(id, from, to, cash.0, &reason) != cash.0 {
                    return Err("Industrial fill payment differs from its funded quote".into());
                }
                let c = self.countries.get_mut(id).expect("country");
                match direction {
                    IndustrialTradeDirection::Import => {
                        c.budget.payments.0 = c
                            .budget
                            .payments
                            .0
                            .checked_add(cash.0)
                            .ok_or("Industrial payment overflow")?;
                        imports.get_mut(&order.good).expect("good").0 += quantity;
                        *available.get_mut(&order.good).expect("good") += quantity;
                    }
                    IndustrialTradeDirection::Export => {
                        let receipts = c
                            .budget
                            .industrial_exports
                            .as_mut()
                            .ok_or("Missing industrial export budget")?;
                        receipts.0 = receipts
                            .0
                            .checked_add(cash.0)
                            .ok_or("Industrial receipts overflow")?;
                        exports.get_mut(&order.good).expect("good").0 += quantity;
                        *available.get_mut(&order.good).expect("good") -= quantity;
                    }
                }
                let current = c
                    .industrial_trade
                    .as_mut()
                    .expect("trade")
                    .orders
                    .iter_mut()
                    .find(|o| o.id == order.id)
                    .expect("order");
                if direction == IndustrialTradeDirection::Import {
                    current.reserved.0 -= cash.0;
                }
                current.fills.push(IndustrialTradeFill {
                    day: self.day,
                    quantity: ProductionQuantity(quantity),
                    cash,
                });
                if filled + quantity == order.quantity.0 {
                    self.close_industrial_trade(id, order.id, IndustrialTradeStatus::Completed)?;
                }
            }
        }
        Ok(IndustrialTradeFlows { imports, exports })
    }
}

impl World {
    /// Audit every contract against commands, physical stocks, cash and retained execution.
    /// Closed contracts remain visible for the campaign; only the ordinary ledger is pruned.
    pub(crate) fn validate_industrial_trade(&self, content: &Content) -> Result<(), String> {
        let Some(definition) = &content.industrial_trade else {
            if self
                .countries
                .values()
                .any(|c| c.industrial_trade.is_some())
            {
                return Err("Unexpected industrial trade state".into());
            }
            return Ok(());
        };
        let production = content
            .production
            .as_ref()
            .ok_or("Missing trade production")?;
        for (id, c) in &self.countries {
            let state = c
                .industrial_trade
                .as_ref()
                .ok_or("Missing industrial trade history")?;
            let physical = self
                .production
                .as_ref()
                .and_then(|p| p.countries.get(id))
                .ok_or("Missing trade inventory")?;
            let mut ids = BTreeSet::new();
            let mut open = BTreeSet::new();
            let mut previous = 0;
            let mut totals: BTreeMap<(GoodId, IndustrialTradeDirection), u64> = BTreeMap::new();
            let mut daily: BTreeMap<(CampaignDay, GoodId, IndustrialTradeDirection), u64> =
                BTreeMap::new();
            let mut expected_ledger = Vec::new();
            for order in &state.orders {
                let entry = self
                    .journal
                    .get(
                        order
                            .id
                            .0
                            .checked_sub(1)
                            .ok_or("Zero industrial contract ID")? as usize,
                    )
                    .ok_or("Missing industrial contract command")?;
                if order.id.0 <= previous
                    || !ids.insert(order.id)
                    || entry.command.issuer != *id
                    || entry.command.day != order.proposed_day
                    || !matches!(&entry.command.action,Action::OrderIndustrialTrade{good,direction,quantity} if good==&order.good && direction==&order.direction && quantity==&order.quantity)
                    || entry
                        .governing_context
                        .as_ref()
                        .is_none_or(|g| g.government != order.sponsor)
                {
                    return Err("Industrial contract differs from its originating command".into());
                }
                previous = order.id.0;
                let storage = production
                    .countries
                    .get(id)
                    .and_then(|p| p.storage.get(&order.good))
                    .ok_or("Unknown industrial storage")?;
                let authority = self.governing_rules_at(content, id, order.proposed_day)?;
                let review = order
                    .proposed_day
                    .checked_add(DaySpan::new(authority.consent_days.units().max(1)))
                    .ok_or("Industrial review overflow")?;
                let expiry = review
                    .checked_add(DaySpan::new(definition.order_days.units() - 1))
                    .ok_or("Industrial expiry overflow")?;
                let fee = Money(
                    (content
                        .country(id)
                        .ok_or("Unknown industrial country")?
                        .gdp
                        .units()
                        * definition.fee_bps.units_i64()
                        / 10000)
                        .max(1),
                );
                if order.quantity.0 == 0
                    || order.quantity.0 > storage.capacity.0
                    || order.proposed_day > self.day
                    || order.review_day != review
                    || order.expiry_day != expiry
                    || expiry.units() > MAX_DAY
                    || order.quoted_cash
                        != definition.amount(
                            content,
                            id,
                            &order.good,
                            order.direction,
                            order.quantity,
                        )?
                    || order.quoted_cash.0 <= 0
                    || order.fee != fee
                    || order.fills.len() > definition.order_days.units() as usize
                {
                    return Err("Invalid industrial contract quote or calendar".into());
                }
                if order.status.open() && !open.insert((order.good.clone(), order.direction)) {
                    return Err("Duplicate open industrial market".into());
                }
                let mut quantity = 0_u64;
                let mut cash = 0_i64;
                let mut last = None;
                let retained =
                    |day: CampaignDay| day.units().saturating_add(30) >= self.day.units();
                if retained(order.proposed_day) {
                    expected_ledger.push((
                        order.proposed_day,
                        0,
                        2,
                        fee.0,
                        format!("Industrial trade administration #{}", order.id.0),
                    ));
                }
                for fill in &order.fills {
                    if fill.day < review
                        || fill.day > expiry
                        || fill.day > self.day
                        || last.is_some_and(|d| fill.day <= d)
                        || fill.quantity.0 == 0
                        || fill.cash.0 < 0
                    {
                        return Err("Invalid industrial fill calendar or quantity".into());
                    }
                    quantity = quantity
                        .checked_add(fill.quantity.0)
                        .ok_or("Industrial fill quantity overflow")?;
                    let quoted = definition
                        .amount(
                            content,
                            id,
                            &order.good,
                            order.direction,
                            ProductionQuantity(quantity),
                        )?
                        .0;
                    if quantity > order.quantity.0
                        || fill.cash.0
                            != quoted
                                .checked_sub(cash)
                                .ok_or("Industrial fill quote underflow")?
                    {
                        return Err("Industrial fill differs from cumulative cash quote".into());
                    }
                    cash = quoted;
                    last = Some(fill.day);
                    let value = daily
                        .entry((fill.day, order.good.clone(), order.direction))
                        .or_default();
                    *value = value
                        .checked_add(fill.quantity.0)
                        .ok_or("Industrial quota sum overflow")?;
                    if retained(fill.day) && fill.cash.0 > 0 {
                        let (from, to, label) = match order.direction {
                            IndustrialTradeDirection::Import => (0, 3, "import"),
                            IndustrialTradeDirection::Export => (3, 0, "export"),
                        };
                        expected_ledger.push((
                            fill.day,
                            from,
                            to,
                            fill.cash.0,
                            format!("Industrial {label} contract #{}", order.id.0),
                        ));
                    }
                }
                let total = totals
                    .entry((order.good.clone(), order.direction))
                    .or_default();
                *total = total
                    .checked_add(quantity)
                    .ok_or("Industrial lifetime quantity overflow")?;
                let reserved =
                    if order.status.open() && order.direction == IndustrialTradeDirection::Import {
                        order.quoted_cash.0 - cash
                    } else {
                        0
                    };
                if order.reserved.0 != reserved
                    || order.status.open() != order.resolved_day.is_none()
                    || order.resolved_day.is_some_and(|d| {
                        d < order.proposed_day || d > self.day || last.is_some_and(|f| f > d)
                    })
                {
                    return Err(
                        "Industrial resolution or reservation differs from execution".into(),
                    );
                }
                match &order.status {
                    IndustrialTradeStatus::Pending if !order.fills.is_empty() || self.day>=review => return Err("Overdue industrial consent".into()),
                    IndustrialTradeStatus::Active if self.day<review || self.day>expiry || quantity==order.quantity.0 => return Err("Invalid active industrial contract".into()),
                    IndustrialTradeStatus::Completed if quantity!=order.quantity.0 || order.resolved_day!=last => return Err("Incomplete industrial delivery marked complete".into()),
                    IndustrialTradeStatus::Expired if order.resolved_day!=expiry.checked_add(DaySpan::new(1)) => return Err("Industrial expiry date differs".into()),
                    IndustrialTradeStatus::Cancelled if !self.journal.iter().any(|e| e.command.issuer==*id && Some(e.command.day)==order.resolved_day && matches!(e.command.action,Action::CancelIndustrialTrade{request} if request==order.id)) => return Err("Industrial cancellation lacks command".into()),
                    IndustrialTradeStatus::Refused{reason} if reason.trim().is_empty() || reason.len()>4096 => return Err("Industrial refusal lacks reason".into()),
                    _=>{},
                }
            }
            if open.len() > definition.maximum_orders as usize {
                return Err("Too many simultaneous industrial contracts".into());
            }
            let journal_ids = self
                .journal
                .iter()
                .filter(|e| {
                    e.command.issuer == *id
                        && matches!(e.command.action, Action::OrderIndustrialTrade { .. })
                })
                .map(|e| IndustrialTradeId(e.command.id.0))
                .collect::<BTreeSet<_>>();
            if ids != journal_ids {
                return Err("Industrial history omits or invents a command".into());
            }
            for ((_, good, direction), quantity) in &daily {
                if *quantity > definition.daily_limit(content, id, good, *direction)?.0 {
                    return Err("Industrial fill exceeds external daily quota".into());
                }
            }
            for (good, position) in &physical.inventory.positions {
                if position.totals.imported.0
                    != *totals
                        .get(&(good.clone(), IndustrialTradeDirection::Import))
                        .unwrap_or(&0)
                    || position.totals.exported.0
                        != *totals
                            .get(&(good.clone(), IndustrialTradeDirection::Export))
                            .unwrap_or(&0)
                {
                    return Err(
                        "Industrial deliveries differ from lifetime physical stock totals".into(),
                    );
                }
                if let Some(last) = &physical.last {
                    let flow = last
                        .physical
                        .storage
                        .get(good)
                        .ok_or("Missing industrial storage flow")?;
                    let after_domestic = flow
                        .after_production
                        .0
                        .checked_sub(flow.final_use.0)
                        .ok_or("Industrial domestic use exceeds production")?;
                    let room = production.countries[id].storage[good]
                        .capacity
                        .0
                        .saturating_sub(after_domestic);
                    if flow.imports.0 > room {
                        return Err("Industrial imports exceed usable storage room".into());
                    }
                    let protected = if good == &production.assembly_good {
                        let pending = c
                            .projects
                            .iter()
                            .filter(|p| {
                                p.stage == PolicyStage::Implementing && p.due_day <= self.day
                            })
                            .try_fold(0_u128, |n, p| {
                                production
                                    .construction_required(id, &p.policy)
                                    .map(|q| n + u128::from(q.map_or(0, |q| q.0)))
                            })?;
                        pending
                            + last
                                .capital_uses
                                .iter()
                                .map(|p| u128::from(p.quantity.0))
                                .sum::<u128>()
                    } else {
                        0
                    };
                    if u128::from(flow.exports.0)
                        > (u128::from(after_domestic) + u128::from(flow.imports.0))
                            .saturating_sub(protected)
                    {
                        return Err(
                            "Industrial exports consume protected domestic construction stocks"
                                .into(),
                        );
                    }
                    if last.request.imports.get(good).map_or(0, |q| q.0)
                        != *daily
                            .get(&(self.day, good.clone(), IndustrialTradeDirection::Import))
                            .unwrap_or(&0)
                        || last.request.exports.get(good).map_or(0, |q| q.0)
                            != *daily
                                .get(&(self.day, good.clone(), IndustrialTradeDirection::Export))
                                .unwrap_or(&0)
                    {
                        return Err("Industrial deliveries differ from dated physical flows".into());
                    }
                }
            }
            for period in
                std::iter::once(&c.budget).chain(c.budget_reviews.iter().map(|r| &r.period))
            {
                let mut exports = 0_i64;
                let mut imports = 0_i64;
                let mut fees = 0_i64;
                for o in &state.orders {
                    if o.proposed_day >= period.start_day && o.proposed_day < period.end_day {
                        fees = fees.checked_add(o.fee.0).ok_or("Industrial fee overflow")?;
                    }
                    for f in &o.fills {
                        if f.day >= period.start_day && f.day < period.end_day {
                            let sum = if o.direction == IndustrialTradeDirection::Import {
                                &mut imports
                            } else {
                                &mut exports
                            };
                            *sum = sum
                                .checked_add(f.cash.0)
                                .ok_or("Industrial budget sum overflow")?;
                        }
                    }
                }
                if period.industrial_exports != Some(Money(exports))
                    || imports > period.payments.0
                    || fees > period.capital.0
                {
                    return Err("Industrial cash differs from budget execution".into());
                }
            }
            let account = |a: Account| match a {
                Account::Treasury => 0,
                Account::Households => 1,
                Account::Firms => 2,
                Account::External => 3,
            };
            let mut actual = self
                .ledger
                .iter()
                .filter(|e| {
                    e.country == *id
                        && (e.reason.starts_with("Industrial trade administration #")
                            || e.reason.starts_with("Industrial import contract #")
                            || e.reason.starts_with("Industrial export contract #"))
                })
                .map(|e| {
                    (
                        e.day,
                        account(e.from),
                        account(e.to),
                        e.amount.0,
                        e.reason.clone(),
                    )
                })
                .collect::<Vec<_>>();
            actual.sort();
            expected_ledger.sort();
            if actual != expected_ledger {
                return Err("Industrial fills or fees differ from retained cash ledger".into());
            }
        }
        Ok(())
    }
}

impl World {
    pub(crate) fn ai_industrial_trade(&mut self, content: &Content, id: &CountryId) {
        let Some(definition) = &content.industrial_trade else {
            return;
        };
        let c = &self.countries[id];
        if c.arrears.0 > 0 || c.report.food_coverage < 100 || c.report.energy_coverage < 100 {
            return;
        }
        let Some(last) = self
            .production
            .as_ref()
            .and_then(|p| p.countries.get(id))
            .and_then(|p| p.last.as_ref())
        else {
            return;
        };
        let production = content.production.as_ref().expect("production");
        let unit = production.countries[id].construction_unit.0;
        let mut candidates = BTreeMap::new();
        for sector in &last.physical.production.sectors {
            for limit in &sector.limits {
                if limit.binding
                    && let ProductionConstraint::Input(good) = &limit.constraint
                    && definition.goods.contains_key(good)
                {
                    candidates.insert(good.clone(), IndustrialTradeDirection::Import);
                }
            }
        }
        if c.projects.iter().any(|p| {
            p.stage == PolicyStage::Implementing
                && p.due_day <= self.day
                && production
                    .construction_required(id, &p.policy)
                    .is_ok_and(|q| q.is_some())
        }) {
            candidates.insert(
                production.assembly_good.clone(),
                IndustrialTradeDirection::Import,
            );
        }
        for (good, flow) in &last.physical.storage {
            if flow.discarded.0 > 0
                && definition.goods.contains_key(good)
                && good != &production.assembly_good
            {
                candidates
                    .entry(good.clone())
                    .or_insert(IndustrialTradeDirection::Export);
            }
        }
        for (good, direction) in candidates {
            let inventory = &self.production.as_ref().expect("production").countries[id].inventory;
            let stock = inventory.positions[&good].quantity.0;
            let capacity = production.countries[id].storage[&good].capacity.0;
            let units = if direction == IndustrialTradeDirection::Export {
                let deliverable = definition
                    .daily_limit(content, id, &good, direction)
                    .expect("validated industrial quota")
                    .0
                    .saturating_mul(u64::from(definition.order_days.units()));
                stock
                    .saturating_sub(
                        last.physical.production.goods[&good]
                            .consumed
                            .0
                            .saturating_mul(3),
                    )
                    .min(deliverable)
            } else if good == production.assembly_good {
                let needed = c
                    .projects
                    .iter()
                    .filter(|p| p.stage == PolicyStage::Implementing && p.due_day <= self.day)
                    .fold(0_u64, |n, p| {
                        n.saturating_add(
                            production
                                .construction_required(id, &p.policy)
                                .ok()
                                .flatten()
                                .map_or(0, |q| q.0),
                        )
                    });
                needed.saturating_sub(stock).min(capacity)
            } else {
                unit.saturating_mul(3).min(capacity)
            };
            if units == 0 {
                continue;
            }
            let quantity = ProductionQuantity(units);
            if let Ok(quote) =
                self.preview_industrial_trade(content, id, &good, direction, quantity)
            {
                if direction == IndustrialTradeDirection::Export
                    && quote.quoted_cash.0 <= quote.fee.0
                {
                    continue;
                }
                let cost = quote.fee.0
                    + if direction == IndustrialTradeDirection::Import {
                        quote.quoted_cash.0
                    } else {
                        0
                    };
                if c.available_cash().0
                    <= content.country(id).expect("country").gdp.units() / 20 + cost
                {
                    continue;
                }
                let command = self.command(
                    id.clone(),
                    Action::OrderIndustrialTrade {
                        good,
                        direction,
                        quantity,
                    },
                );
                let _ = self.apply(content, command, CommandOrigin::Ai);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_external_cash_clips_exports() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-industrial-trade-test.json"
        ))
        .unwrap();
        let id = CountryId("AUR".into());
        let mut world = World::new(&content, id.clone(), 17).unwrap();
        world
            .issue(
                &content,
                Action::OrderIndustrialTrade {
                    good: GoodId("biomass".into()),
                    direction: IndustrialTradeDirection::Export,
                    quantity: ProductionQuantity(100),
                },
            )
            .unwrap();
        let review = world.countries[&id]
            .industrial_trade
            .as_ref()
            .unwrap()
            .orders[0]
            .review_day;
        world
            .advance(&content, review.elapsed_since(world.day).unwrap())
            .unwrap();
        let before = world.countries[&id]
            .industrial_trade
            .as_ref()
            .unwrap()
            .orders[0]
            .cash();
        let last = world.production.as_ref().unwrap().countries[&id]
            .last
            .clone()
            .unwrap();
        let external = world.countries[&id].accounts.external.0;
        assert_eq!(
            world.pay(
                &id,
                Account::External,
                Account::Households,
                external,
                "Test counterparty cash allocation"
            ),
            external
        );
        let flows = world
            .settle_industrial_trade(
                &content,
                &id,
                &last.physical.production,
                &last.request.final_use,
            )
            .unwrap();
        assert!(
            flows.exports[&GoodId("biomass".into())].0
                < content
                    .industrial_trade
                    .as_ref()
                    .unwrap()
                    .daily_limit(
                        &content,
                        &id,
                        &GoodId("biomass".into()),
                        IndustrialTradeDirection::Export
                    )
                    .unwrap()
                    .0
        );
        assert!(
            world.countries[&id]
                .industrial_trade
                .as_ref()
                .unwrap()
                .orders[0]
                .cash()
                .0
                - before.0
                <= 1
        );
        assert!(world.countries[&id].accounts.external.0 >= 0);
    }
}
