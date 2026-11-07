//! Costed import contracts and separate national commodity buffers.
use crate::developer::CommandOrigin;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReserveDefinition {
    pub capacity_days: NeedDays,
    pub order_days: DaySpan,
    pub fee_bps: BasisPoints,
    pub ai_target_days: NeedDays,
    pub source: String,
    pub source_version: String,
}
impl ReserveDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=10).contains(&self.capacity_days.units())
            || !(30..=365).contains(&self.order_days.units())
            || !(1..=10).contains(&self.fee_bps.units())
            || !(1..=self.capacity_days.units()).contains(&self.ai_target_days.units())
            || self.source_version.trim().is_empty()
            || self.source_version.len() > 128
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("National reserves require bounded capacity, contract duration, fee and source provenance".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ReserveOrderId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReserveStatus {
    Pending,
    Active,
    Completed,
    Cancelled,
    Expired,
    Refused { reason: String },
}
impl ReserveStatus {
    pub fn open(&self) -> bool {
        matches!(self, Self::Pending | Self::Active)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReserveOrder {
    pub id: ReserveOrderId,
    pub good: GoodKind,
    pub days: NeedDays,
    pub quantity: i64,
    pub filled: i64,
    pub price_bps: BasisPoints,
    pub last_fill_day: Option<crate::CampaignDay>,
    pub last_fill_quantity: i64,
    pub quoted_cost: Money,
    pub paid: Money,
    pub reserved: Money,
    pub fee: Money,
    pub proposed_day: crate::CampaignDay,
    pub review_day: crate::CampaignDay,
    pub expiry_day: crate::CampaignDay,
    pub sponsor: crate::GovernmentId,
    pub status: ReserveStatus,
    pub resolved_day: Option<crate::CampaignDay>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReserveFlow<Q> {
    pub day: crate::CampaignDay,
    pub opening: Q,
    pub purchased: Q,
    pub released: Q,
    pub loss: Q,
    pub closing: Q,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReserveStock<Q> {
    pub stored: Q,
    pub purchased: Q,
    pub released: Q,
    pub lost: Q,
    pub last: Option<ReserveFlow<Q>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NationalReserves {
    pub energy: ReserveStock<Energy>,
    pub food: ReserveStock<Food>,
    pub orders: Vec<ReserveOrder>,
    pub last_release_day: Option<crate::CampaignDay>,
}
impl NationalReserves {
    pub fn new() -> Self {
        Self {
            energy: ReserveStock {
                stored: Energy::ZERO,
                purchased: Energy::ZERO,
                released: Energy::ZERO,
                lost: Energy::ZERO,
                last: None,
            },
            food: ReserveStock {
                stored: Food::ZERO,
                purchased: Food::ZERO,
                released: Food::ZERO,
                lost: Food::ZERO,
                last: None,
            },
            orders: Vec::new(),
            last_release_day: None,
        }
    }
    pub fn reserved_cash(&self) -> Money {
        Money(self.orders.iter().map(|o| o.reserved.0).sum())
    }
    pub fn stored(&self, good: GoodKind) -> i64 {
        match good {
            GoodKind::Energy => self.energy.stored.units(),
            GoodKind::Food => self.food.stored.units(),
        }
    }
}
impl Default for NationalReserves {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservePreview {
    pub quantity: i64,
    pub price_bps: BasisPoints,
    pub cost: Money,
    pub fee: Money,
    pub review_day: crate::CampaignDay,
    pub earliest_delivery_day: crate::CampaignDay,
    pub expiry_day: crate::CampaignDay,
    pub route: String,
}
/// Observed procurement constraints, never a promise about a future settlement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReserveSupply {
    pub day: crate::CampaignDay,
    pub physical_need: i64,
    pub civilian_supply: i64,
    pub civilian_imports: i64,
    pub import_capacity: i64,
    pub spare_import_capacity: i64,
}
impl ReserveSupply {
    pub fn explanation(&self) -> &'static str {
        if self.civilian_supply < self.physical_need {
            "Procurement could not fill: civilian physical needs were not covered. Improve supply first."
        } else if self.spare_import_capacity == 0 {
            "Procurement could not fill: civilian imports used the full import capacity. Improve domestic supply to free capacity."
        } else {
            "Spare import capacity was available after civilian physical needs. An accepted funded contract could fill within its remaining quantity."
        }
    }
}
fn ceiling_ratio(n: i128, d: i128) -> i64 {
    i64::try_from((n + d - 1) / d).expect("bounded reserve price")
}
fn contract_cost(gdp: AnnualGdp, need: i64, quantity: i64, price: u32) -> Money {
    Money(ceiling_ratio(
        i128::from((gdp.units() / 365).max(1)) * i128::from(quantity) * i128::from(price),
        i128::from(need) * 5 * 10000,
    ))
}
fn paid_for(order: &ReserveOrder, filled: i64) -> Money {
    Money(ceiling_ratio(
        i128::from(order.quoted_cost.0) * i128::from(filled),
        i128::from(order.quantity),
    ))
}
impl World {
    pub fn reserve_supply(
        &self,
        content: &Content,
        id: &CountryId,
        good: GoodKind,
    ) -> Result<Option<ReserveSupply>, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Reserve supply inspection requires the campaign's exact content".into());
        }
        content
            .reserves
            .as_ref()
            .ok_or("This pack has no national reserve procedure")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let markets = c.markets.as_ref().ok_or("Missing commodity markets")?;
        let observed = match good {
            GoodKind::Energy => markets.energy.last.as_ref().map(|m| {
                (
                    m.day,
                    m.physical_need.units(),
                    m.supply.units(),
                    m.imports.units(),
                )
            }),
            GoodKind::Food => markets.food.last.as_ref().map(|m| {
                (
                    m.day,
                    m.physical_need.units(),
                    m.supply.units(),
                    m.imports.units(),
                )
            }),
        };
        let Some((day, physical_need, civilian_supply, civilian_imports)) = observed else {
            return Ok(None);
        };
        let rule = content
            .markets
            .as_ref()
            .ok_or("Missing commodity market rules")?
            .good(good);
        let import_capacity = physical_need * i64::from(rule.import_limit_bps.units()) / 10000;
        Ok(Some(ReserveSupply {
            day,
            physical_need,
            civilian_supply,
            civilian_imports,
            import_capacity,
            spare_import_capacity: (import_capacity - civilian_imports).max(0),
        }))
    }
    pub fn preview_reserve(
        &self,
        content: &Content,
        id: &CountryId,
        good: GoodKind,
        days: NeedDays,
    ) -> Result<ReservePreview, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Reserve procurement requires the campaign's exact content".into());
        }
        let rules = content
            .reserves
            .as_ref()
            .ok_or("This pack has no national reserve procedure")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let d = content.country(id).ok_or("Unknown country definition")?;
        let state = c
            .reserves
            .as_ref()
            .ok_or("Missing national reserve state")?;
        if !(1..=rules.capacity_days.units()).contains(&days.units()) {
            return Err(format!(
                "Choose 1..{} days of baseline physical need",
                rules.capacity_days
            ));
        }
        let need = (c.population.0 / 1000) as i64;
        let quantity = need * i64::from(days.units());
        if state
            .orders
            .iter()
            .any(|o| o.good == good && o.status.open())
        {
            return Err("This commodity already has an open procurement contract".into());
        }
        if state.stored(good) + quantity > need * i64::from(rules.capacity_days.units()) {
            return Err("The order would exceed national storage capacity".into());
        }
        let government = self.governing_rules(content, id)?;
        government.check(c, "reserve", 1)?;
        let market = c
            .markets
            .as_ref()
            .ok_or("Reserve procurement requires commodity markets")?;
        let price_bps = match good {
            GoodKind::Energy => market.energy.next_price_bps.units(),
            GoodKind::Food => market.food.next_price_bps.units(),
        };
        let cost = contract_cost(d.gdp, need, quantity, price_bps);
        let fee = Money((d.gdp.units() * i64::from(rules.fee_bps.units()) / 10000).max(1));
        if c.available_cash().0 < cost.0 + fee.0 {
            return Err(format!(
                "Requires {}m uncommitted national cash: {}m administration and {}m reserved purchase budget",
                cost.0 + fee.0,
                fee.0,
                cost.0
            ));
        }
        let review_day = self
            .day
            .units()
            .checked_add(government.consent_days.units())
            .ok_or("Invalid review date")?;
        let earliest_delivery_day = review_day.max(self.day.units() + 1);
        let expiry_day = earliest_delivery_day
            .checked_add(rules.order_days.units() - 1)
            .filter(|day| *day <= MAX_DAY)
            .ok_or("Reserve contract exceeds the campaign calendar")?;
        Ok(ReservePreview {
            quantity,
            price_bps: BasisPoints::new(price_bps),
            cost,
            fee,
            review_day: crate::CampaignDay::new(review_day),
            earliest_delivery_day: crate::CampaignDay::new(earliest_delivery_day),
            expiry_day: crate::CampaignDay::new(expiry_day),
            route: government.label(),
        })
    }
    pub(crate) fn procure_reserve(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: ReserveOrderId,
        good: GoodKind,
        days: NeedDays,
    ) -> Result<(), String> {
        let p = self.preview_reserve(content, id, good, days)?;
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            p.fee.0,
            "National reserve procurement administration",
        );
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += p.fee.0;
        c.committed_cash.0 += p.cost.0;
        let state = c.reserves.as_mut().expect("reserves");
        if state.orders.len() >= 128
            && let Some(index) = state
                .orders
                .iter()
                .position(|o| !o.status.open() && o.last_fill_day != Some(self.day))
        {
            state.orders.remove(index);
        }
        state.orders.push(ReserveOrder {
            id: request,
            good,
            days,
            quantity: p.quantity,
            filled: 0,
            price_bps: p.price_bps,
            last_fill_day: None,
            last_fill_quantity: 0,
            quoted_cost: p.cost,
            paid: Money(0),
            reserved: p.cost,
            fee: p.fee,
            proposed_day: self.day,
            review_day: p.review_day,
            expiry_day: p.expiry_day,
            sponsor: c.government_number,
            status: if p.review_day == self.day {
                ReserveStatus::Active
            } else {
                ReserveStatus::Pending
            },
            resolved_day: None,
        });
        self.event(id,true,format!("Reserve contract #{} proposed: {:?}, {} units. Paid {}m administration; {}m reserved at a fixed quoted price. Consent {}; fills use only spare import capacity after civilian needs. Reserved cash is not delivered goods.",request.0,good,p.quantity,p.fee.0,p.cost.0,date((p.review_day).units())));
        Ok(())
    }
    fn close_reserve(&mut self, id: &CountryId, request: ReserveOrderId, status: ReserveStatus) {
        let c = self.countries.get_mut(id).expect("country");
        let order = c
            .reserves
            .as_mut()
            .expect("reserves")
            .orders
            .iter_mut()
            .find(|o| o.id == request)
            .expect("order");
        c.committed_cash.0 -= order.reserved.0;
        let released = order.reserved.0;
        order.reserved = Money(0);
        order.status = status.clone();
        order.resolved_day = Some(self.day);
        self.event(id,true,format!("Reserve contract #{} {:?}; released {}m unspent cash. Delivered stocks and paid administration remain; no delivered-goods refund.",request.0,status,released));
    }
    pub fn preview_cancel_reserve(
        &self,
        content: &Content,
        id: &CountryId,
        request: ReserveOrderId,
    ) -> Result<Money, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Reserve cancellation requires the campaign's exact content".into());
        }
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let order = c
            .reserves
            .as_ref()
            .and_then(|s| s.orders.iter().find(|o| o.id == request && o.status.open()))
            .ok_or("No open reserve contract with that ID")?;
        if order.status == ReserveStatus::Active {
            self.governing_rules(content, id)?.check(c, "reserve", 1)?;
        }
        Ok(order.reserved)
    }
    pub(crate) fn cancel_reserve(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: ReserveOrderId,
    ) -> Result<(), String> {
        self.preview_cancel_reserve(content, id, request)?;
        self.close_reserve(id, request, ReserveStatus::Cancelled);
        Ok(())
    }
    pub(crate) fn review_reserves(&mut self, content: &Content, id: &CountryId) {
        let Some(state) = self.countries[id].reserves.as_ref() else {
            return;
        };
        let orders: Vec<_> = state
            .orders
            .iter()
            .filter(|o| o.status.open())
            .cloned()
            .collect();
        for o in orders {
            if o.status == ReserveStatus::Pending {
                let result = if o.sponsor != self.countries[id].government_number {
                    Err("sponsoring government changed".into())
                } else if self.day.units() >= (o.review_day).units() {
                    self.governing_rules(content, id)
                        .expect("government")
                        .check(&self.countries[id], "reserve", 1)
                } else {
                    continue;
                };
                if let Err(reason) = result {
                    self.close_reserve(id, o.id, ReserveStatus::Refused { reason });
                    continue;
                }
                self.countries
                    .get_mut(id)
                    .unwrap()
                    .reserves
                    .as_mut()
                    .unwrap()
                    .orders
                    .iter_mut()
                    .find(|r| r.id == o.id)
                    .unwrap()
                    .status = ReserveStatus::Active;
                self.event(id,true,format!("Reserve contract #{} received consent; procurement may now fill from spare import capacity. It grants no immediate stocks.",o.id.0));
            }
            if self.day.units() > (o.expiry_day).units() {
                self.close_reserve(id, o.id, ReserveStatus::Expired);
            }
        }
    }
    pub(crate) fn begin_reserve_day(&mut self, id: &CountryId) {
        if let Some(state) = self.countries.get_mut(id).unwrap().reserves.as_mut() {
            state.energy.last = Some(ReserveFlow {
                day: self.day,
                opening: state.energy.stored,
                purchased: Energy::ZERO,
                released: Energy::ZERO,
                loss: Energy::ZERO,
                closing: state.energy.stored,
            });
            state.food.last = Some(ReserveFlow {
                day: self.day,
                opening: state.food.stored,
                purchased: Food::ZERO,
                released: Food::ZERO,
                loss: Food::ZERO,
                closing: state.food.stored,
            });
        }
    }
    pub(crate) fn release_reserve(
        &mut self,
        id: &CountryId,
        good: GoodKind,
        shortfall: i64,
    ) -> i64 {
        let Some(s) = self.countries.get_mut(id).unwrap().reserves.as_mut() else {
            return 0;
        };
        let amount = s.stored(good).min(shortfall);
        let notice = amount > 0
            && s.last_release_day
                .is_none_or(|day| day.units().saturating_add(1) < self.day.units());
        if amount > 0 {
            s.last_release_day = Some(self.day);
        }
        match good {
            GoodKind::Energy => {
                let q = Energy::new(amount).unwrap();
                s.energy.stored = s.energy.stored.checked_sub(q).unwrap();
                s.energy.released = s.energy.released.checked_add(q).unwrap();
                s.energy.last.as_mut().unwrap().released = q;
            }
            GoodKind::Food => {
                let q = Food::new(amount).unwrap();
                s.food.stored = s.food.stored.checked_sub(q).unwrap();
                s.food.released = s.food.released.checked_add(q).unwrap();
                s.food.last.as_mut().unwrap().released = q;
            }
        }
        let depleted = amount > 0 && s.stored(good) == 0;
        if notice {
            self.event(id,true,"National reserves are covering post-import physical shortfalls. Treasury shows food/energy releases and remaining stocks; household affordability still applies.");
        }
        if depleted {
            self.event(id,true,format!("National {good:?} reserve depleted. Procurement still needs cash, consent and spare import capacity; sustained supply shortfalls need further action."));
        }
        amount
    }
    pub(crate) fn finish_reserve_day(&mut self, content: &Content, id: &CountryId) {
        let Some(state) = self.countries[id].reserves.as_ref() else {
            return;
        };
        let orders: Vec<_> = state
            .orders
            .iter()
            .filter(|o| o.status == ReserveStatus::Active)
            .cloned()
            .collect();
        let c = &self.countries[id];
        let need = (c.population.0 / 1000) as i64;
        for o in orders {
            let c = &self.countries[id];
            let m = c.markets.as_ref().unwrap();
            let r = content.markets.as_ref().unwrap().good(o.good);
            let (supply, imports) = match o.good {
                GoodKind::Energy => {
                    let x = m.energy.last.as_ref().unwrap();
                    (x.supply.units(), x.imports.units())
                }
                GoodKind::Food => {
                    let x = m.food.last.as_ref().unwrap();
                    (x.supply.units(), x.imports.units())
                }
            };
            if supply < need {
                continue;
            }
            let quantity = (need * i64::from(r.import_limit_bps.units()) / 10000 - imports)
                .min(o.quantity - o.filled)
                .max(0);
            if quantity == 0 {
                continue;
            }
            let due = paid_for(&o, o.filled + quantity).0 - o.paid.0;
            self.countries.get_mut(id).unwrap().committed_cash.0 -= due;
            let paid = self.pay(
                id,
                Account::Treasury,
                Account::External,
                due,
                "National reserve commodity procurement",
            );
            assert_eq!(paid, due, "reserved procurement cash must remain funded");
            let c = self.countries.get_mut(id).unwrap();
            c.budget.payments.0 += paid;
            c.report.spending.0 += paid;
            let s = c.reserves.as_mut().unwrap();
            let r = s.orders.iter_mut().find(|r| r.id == o.id).unwrap();
            r.filled += quantity;
            r.last_fill_day = Some(self.day);
            r.last_fill_quantity = quantity;
            r.paid.0 += paid;
            r.reserved.0 -= paid;
            match o.good {
                GoodKind::Energy => {
                    let q = Energy::new(quantity).unwrap();
                    s.energy.stored = s.energy.stored.checked_add(q).unwrap();
                    s.energy.purchased = s.energy.purchased.checked_add(q).unwrap();
                    s.energy.last.as_mut().unwrap().purchased = q;
                }
                GoodKind::Food => {
                    let q = Food::new(quantity).unwrap();
                    s.food.stored = s.food.stored.checked_add(q).unwrap();
                    s.food.purchased = s.food.purchased.checked_add(q).unwrap();
                    s.food.last.as_mut().unwrap().purchased = q;
                }
            }
            if o.filled + quantity == o.quantity {
                self.close_reserve(id, o.id, ReserveStatus::Completed);
            }
        }
        let s = self
            .countries
            .get_mut(id)
            .unwrap()
            .reserves
            .as_mut()
            .unwrap();
        let loss = s
            .food
            .stored
            .scaled(
                i64::from(
                    content
                        .markets
                        .as_ref()
                        .unwrap()
                        .good(GoodKind::Food)
                        .loss_bps
                        .units(),
                ),
                10000,
            )
            .unwrap();
        s.food.stored = s.food.stored.checked_sub(loss).unwrap();
        s.food.lost = s.food.lost.checked_add(loss).unwrap();
        let f = s.food.last.as_mut().unwrap();
        f.loss = loss;
        f.closing = s.food.stored;
        s.energy.last.as_mut().unwrap().closing = s.energy.stored;
    }
    pub(crate) fn ai_reserves(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.reserves else {
            return;
        };
        let c = &self.countries[id];
        if c.arrears.0 > 0 || c.report.energy_coverage < 100 || c.report.food_coverage < 100 {
            return;
        }
        let d = content.country(id).unwrap();
        if c.population != d.population {
            return;
        }
        let need = (c.population.0 / 1000) as i64;
        let state = c.reserves.as_ref().unwrap();
        for good in [GoodKind::Energy, GoodKind::Food] {
            if state.stored(good) >= need / 2 {
                continue;
            }
            if let Ok(p) = self.preview_reserve(content, id, good, rules.ai_target_days)
                && c.available_cash().0 > d.gdp.units() / 20 + p.cost.0 + p.fee.0
            {
                let cmd = self.command(
                    id.clone(),
                    Action::ProcureReserve {
                        good,
                        days: rules.ai_target_days,
                    },
                );
                let _ = self.apply(content, cmd, CommandOrigin::Ai);
                return;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_stock<Q: Copy>(
    s: &ReserveStock<Q>,
    units: impl Fn(Q) -> i64,
    need: i64,
    rules: &ReserveDefinition,
    day: u32,
    start: u32,
    loss_bps: u32,
    imports: i64,
    supply: i64,
    import_bps: u32,
    purchased_today: i64,
) -> Result<(), String> {
    let stored = units(s.stored);
    let purchased = units(s.purchased);
    let released = units(s.released);
    let lost = units(s.lost);
    if !(0..=need * i64::from(rules.capacity_days.units())).contains(&stored)
        || [purchased, released, lost]
            .iter()
            .any(|v| !(0..=1_000_000_000_000_000).contains(v))
        || i128::from(purchased) - i128::from(released) - i128::from(lost) != i128::from(stored)
    {
        return Err("National reserve inventory does not reconcile".into());
    }
    let Some(f) = &s.last else {
        return if day == start && [stored, purchased, released, lost, purchased_today] == [0; 5] {
            Ok(())
        } else {
            Err("Missing national reserve settlement".into())
        };
    };
    let opening = units(f.opening);
    let bought = units(f.purchased);
    let used = units(f.released);
    let loss = units(f.loss);
    if f.day != crate::CampaignDay::new(day)
        || day <= start
        || [opening, bought, used, loss]
            .iter()
            .any(|v| !(0..=1_000_000_000).contains(v))
        || opening > need * i64::from(rules.capacity_days.units())
        || used > opening.min(need)
        || used != opening.min((need - (supply - used)).max(0))
        || bought != purchased_today
        || bought + imports > need * i64::from(import_bps) / 10000
        || (bought > 0 && supply < need)
        || loss != (opening - used + bought) * i64::from(loss_bps) / 10000
        || opening - used + bought - loss != stored
        || units(f.closing) != stored
        || bought > purchased
        || used > released
        || loss > lost
    {
        return Err("National reserve daily stock/priority/quota settlement is invalid".into());
    }
    Ok(())
}
impl World {
    pub(crate) fn validate_reserves(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[id];
        let (rules, state) = match (&content.reserves, &c.reserves) {
            (None, None) => return Ok(()),
            (Some(r), Some(s)) => (r, s),
            _ => return Err("National reserve state differs from campaign content".into()),
        };
        let released_today = state
            .energy
            .last
            .as_ref()
            .is_some_and(|f| f.released.units() > 0)
            || state
                .food
                .last
                .as_ref()
                .is_some_and(|f| f.released.units() > 0);
        if state.last_release_day.is_some_and(|day| {
            day <= crate::CampaignDay::new(content.start_day().unwrap()) || day > self.day
        }) || (state.energy.released.units() > 0 || state.food.released.units() > 0)
            != state.last_release_day.is_some()
            || released_today != (state.last_release_day == Some(self.day))
        {
            return Err("Invalid national reserve release notice date".into());
        }
        if state.orders.len() > 128 {
            return Err("Reserve order history exceeds its retention limit".into());
        }
        let d = content.country(id).unwrap();
        if c.population != d.population {
            return Err("Reserve population differs from fixed country definition".into());
        }
        let need = (c.population.0 / 1000) as i64;
        let mut previous = 0;
        let mut open = std::collections::BTreeSet::new();
        for o in &state.orders {
            let command = self
                .journal
                .get(o.id.0.saturating_sub(1) as usize)
                .ok_or("Missing reserve procurement command")?;
            let market = content.markets.as_ref().unwrap().good(o.good);
            if o.id.0 <= previous
                || command.command.issuer != *id
                || command.command.day != crate::CampaignDay::new((o.proposed_day).units())
                || !matches!(command.command.action,Action::ProcureReserve{good,days} if good==o.good&&days==o.days)
                || !(1..=rules.capacity_days.units()).contains(&o.days.units())
                || o.quantity != need * i64::from(o.days.units())
                || !(0..=o.quantity).contains(&o.filled)
                || !(market.minimum_price_bps.units()..=market.maximum_price_bps.units())
                    .contains(&o.price_bps.units())
                || o.quoted_cost != contract_cost(d.gdp, need, o.quantity, o.price_bps.units())
                || o.fee != Money((d.gdp.units() * i64::from(rules.fee_bps.units()) / 10000).max(1))
                || o.paid != paid_for(o, o.filled)
                || o.review_day < o.proposed_day
                || (o.review_day).units() - (o.proposed_day).units() > 365
                || o.expiry_day
                    != crate::CampaignDay::new(
                        (o.review_day
                            .max(crate::CampaignDay::new((o.proposed_day).units() + 1)))
                        .units()
                            + rules.order_days.units()
                            - 1,
                    )
                || o.expiry_day > crate::CampaignDay::new(MAX_DAY)
                || o.sponsor == crate::GovernmentId(0)
                || o.sponsor > c.government_number
                || o.reserved.0
                    != if o.status.open() {
                        o.quoted_cost.0 - o.paid.0
                    } else {
                        0
                    }
                || o.resolved_day
                    .is_some_and(|x| x < o.proposed_day || x > self.day)
                || o.status.open() != o.resolved_day.is_none()
                || o.last_fill_day
                    .is_some_and(|x| x < o.review_day || x > self.day || x > o.expiry_day)
                || (o.filled == 0) != (o.last_fill_day.is_none())
                || o.last_fill_quantity < 0
                || o.last_fill_quantity > o.filled
                || (o.last_fill_day.is_some() && o.last_fill_quantity == 0)
            {
                return Err(
                    "Invalid reserve procurement history, funding or command reference".into(),
                );
            }
            if o.status.open()
                && !open.insert(match o.good {
                    GoodKind::Energy => 0,
                    GoodKind::Food => 1,
                })
            {
                return Err("Overlapping national reserve contracts".into());
            }
            match &o.status {
                ReserveStatus::Pending if o.filled!=0=>return Err("Pending reserve contract already received goods".into()),
                ReserveStatus::Active if o.review_day>self.day||self.day.units()>(o.expiry_day).units()||o.filled==o.quantity=>return Err("Invalid active reserve contract".into()),
                ReserveStatus::Completed if o.filled!=o.quantity||o.resolved_day!=o.last_fill_day=>return Err("Invalid completed reserve contract".into()),
                ReserveStatus::Expired if o.resolved_day!=Some(crate::CampaignDay::new((o.expiry_day).units()+1))=>return Err("Invalid reserve expiry".into()),
                ReserveStatus::Refused{reason} if o.filled!=0||reason.trim().is_empty()=>return Err("Invalid reserve refusal".into()),
                ReserveStatus::Cancelled if !self.journal.iter().any(|e|e.command.issuer==*id&&Some(e.command.day.units())==(o.resolved_day).map(|day| day.units())&&matches!(e.command.action,Action::CancelReserve{request} if request==o.id))=>return Err("Missing reserve cancellation command".into()),
                _=>{}
            }
            previous = o.id.0;
        }
        let bought = |good| {
            state
                .orders
                .iter()
                .filter(|o| o.good == good && o.last_fill_day == Some(self.day))
                .map(|o| o.last_fill_quantity)
                .sum()
        };
        let market = content.markets.as_ref().unwrap();
        let quotes = c.markets.as_ref().unwrap();
        let e = market.good(GoodKind::Energy);
        let f = market.good(GoodKind::Food);
        validate_stock(
            &state.energy,
            Energy::units,
            need,
            rules,
            self.day.units(),
            content.start_day()?,
            0,
            c.report.energy_imported.units(),
            quotes.energy.last.as_ref().map_or(0, |x| x.supply.units()),
            e.import_limit_bps.units(),
            bought(GoodKind::Energy),
        )?;
        validate_stock(
            &state.food,
            Food::units,
            need,
            rules,
            self.day.units(),
            content.start_day()?,
            f.loss_bps.units(),
            c.report.food_imported.units(),
            quotes.food.last.as_ref().map_or(0, |x| x.supply.units()),
            f.import_limit_bps.units(),
            bought(GoodKind::Food),
        )
    }
}
