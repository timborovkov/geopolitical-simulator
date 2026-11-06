//! Authored subnational mandates and protected public subaccounts.
//! Public cash remains consolidated; national orders cannot spend local entitlements.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct RegionId(pub String);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RegionalOrderId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegionalTier {
    Region,
    Municipality,
    AutonomousJurisdiction,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerritorialForm {
    Federation,
    DecentralizedUnitary,
    DelegatedUnitary,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalMandate {
    pub id: RegionId,
    pub authority: OfficeId,
    pub name: String,
    pub tier: RegionalTier,
    pub population_share_bps: BasisPoints,
    pub tax_entitlement_bps: BasisPoints,
    pub autonomous_services: bool,
    pub service_level: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritorialCharter {
    pub country: CountryId,
    pub form: TerritorialForm,
    pub regions: Vec<RegionalMandate>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubnationalDefinition {
    pub charters: Vec<TerritorialCharter>,
    pub grant_review_days: DaySpan,
    pub cooldown_days: DaySpan,
    pub administration_cost_bps: BasisPoints,
    pub absorption_days: DaySpan,
    pub source: String,
    pub source_version: String,
}
impl SubnationalDefinition {
    pub fn charter(&self, country: &CountryId) -> Option<&TerritorialCharter> {
        self.charters
            .iter()
            .find(|charter| &charter.country == country)
    }
    pub fn mandate(&self, country: &CountryId, region: &RegionId) -> Option<&RegionalMandate> {
        self.charter(country)?
            .regions
            .iter()
            .find(|mandate| &mandate.id == region)
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=90).contains(&self.grant_review_days.units())
            || !(30..=365).contains(&self.cooldown_days.units())
            || !(1..=100).contains(&self.administration_cost_bps.units_i64())
            || !(30..=365).contains(&self.absorption_days.units())
            || self.source_version.is_empty()
            || !content
                .sources
                .iter()
                .any(|source| source.id == self.source)
        {
            return Err("Invalid subnational review, costs or provenance".into());
        }
        let mut countries = BTreeSet::new();
        let mut regions = BTreeSet::new();
        let mut authorities = BTreeSet::new();
        for charter in &self.charters {
            if !countries.insert(&charter.country)
                || content.country(&charter.country).is_none()
                || match charter.form {
                    TerritorialForm::Federation | TerritorialForm::DecentralizedUnitary => !charter
                        .regions
                        .iter()
                        .any(|region| region.autonomous_services),
                    TerritorialForm::DelegatedUnitary => charter
                        .regions
                        .iter()
                        .any(|region| region.autonomous_services),
                }
                || charter.regions.is_empty()
                || charter.regions.len() > 8
                || charter
                    .regions
                    .iter()
                    .map(|r| i128::from(r.population_share_bps.units_i64()))
                    .sum::<i128>()
                    != 10000
                || charter
                    .regions
                    .iter()
                    .map(|r| i128::from(r.tax_entitlement_bps.units_i64()))
                    .sum::<i128>()
                    > 4000
            {
                return Err(
                    "Invalid territorial charter or nonpartitioning budget jurisdictions".into(),
                );
            }
            for region in &charter.regions {
                if !regions.insert(&region.id)
                    || !authorities.insert(&region.authority)
                    || region.id.0.is_empty()
                    || region.authority.0.is_empty()
                    || region.name.is_empty()
                    || !(1..=10000).contains(&region.population_share_bps.units_i64())
                    || !(0..=4000).contains(&region.tax_entitlement_bps.units_i64())
                    || !(0..=8).contains(&region.service_level)
                    || content.cabinet.as_ref().is_some_and(|cabinet| {
                        cabinet.portfolios.iter().any(|p| p.id == region.authority)
                    })
                    || content.elections.as_ref().is_some_and(|elections| {
                        elections
                            .mandates
                            .iter()
                            .any(|m| m.authority == region.authority)
                    })
                    || content.identities.as_ref().is_some_and(|identities| {
                        identities.offices.iter().any(|office| {
                            office.id == region.authority
                                && (office.country != charter.country
                                    || office.title != region.name
                                    || office.role != OfficeRole::LocalGovernment)
                        })
                    })
                {
                    return Err(
                        "Invalid or conflicting subnational authority or entitlement".into(),
                    );
                }
            }
        }
        if countries != content.countries.iter().map(|c| &c.id).collect() {
            return Err("Each country requires its own territorial charter".into());
        }
        Ok(())
    }
}
/// Annual GDP-point spending weighted by a disjoint budget jurisdiction's population.
/// This purchases public-service household work; it creates no physical capacity.
pub fn regional_daily_cost(gdp: i64, share_bps: i64, level: i32) -> Money {
    if level == 0 {
        Money(0)
    } else {
        Money((gdp * share_bps * i64::from(level) / 365 / 10000 / 100).max(1))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalSettlement {
    pub day: u32,
    pub tax_receipts: Money,
    pub requested_base: Money,
    pub paid_base: Money,
    pub paid_supplement: Money,
    pub closing_cash: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalBudget {
    pub id: RegionId,
    pub population_share_bps: BasisPoints,
    pub tax_entitlement_bps: BasisPoints,
    pub service_level: i32,
    pub cash: Money,
    pub tax_received: Money,
    pub base_spent: Money,
    pub next_order_day: u32,
    pub last: Option<RegionalSettlement>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegionalOrderKind {
    Grant { boost: i32, days: u32 },
    Directive { level: i32 },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegionalOrderStatus {
    Pending,
    Implementing,
    Completed,
    Delivered,
    Withdrawn,
    Superseded,
    Refused { reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalOrder {
    pub id: RegionalOrderId,
    pub region: RegionId,
    pub kind: RegionalOrderKind,
    pub proposed_day: u32,
    pub decision_day: u32,
    pub end_day: Option<u32>,
    pub sponsor: crate::GovernmentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charter_revision: Option<u32>,
    pub baseline_level: i32,
    pub paid_administration: Money,
    pub principal: Money,
    pub daily_supplement: Money,
    pub escrow: Money,
    pub remaining: Money,
    pub spent: Money,
    pub status: RegionalOrderStatus,
    pub resolved_day: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubnationalState {
    pub regions: Vec<RegionalBudget>,
    pub orders: Vec<RegionalOrder>,
}
impl SubnationalState {
    pub fn new(charter: &TerritorialCharter, start: u32) -> Self {
        Self {
            regions: charter
                .regions
                .iter()
                .map(|r| RegionalBudget {
                    id: r.id.clone(),
                    population_share_bps: r.population_share_bps,
                    tax_entitlement_bps: r.tax_entitlement_bps,
                    service_level: r.service_level,
                    cash: Money(0),
                    tax_received: Money(0),
                    base_spent: Money(0),
                    next_order_day: start,
                    last: None,
                })
                .collect(),
            orders: Vec::new(),
        }
    }
    pub fn protected_cash(&self) -> Money {
        let total = self
            .regions
            .iter()
            .map(|r| i128::from(r.cash.0))
            .sum::<i128>()
            + self
                .orders
                .iter()
                .map(|o| i128::from(o.escrow.0) + i128::from(o.remaining.0))
                .sum::<i128>();
        Money(i64::try_from(total).unwrap_or(i64::MAX))
    }
}
impl Country {
    pub fn regional_protected_cash(&self) -> Money {
        self.subnational
            .as_ref()
            .map_or(Money(0), SubnationalState::protected_cash)
    }
    pub fn available_cash(&self) -> Money {
        Money(
            self.accounts
                .treasury
                .0
                .saturating_sub(self.committed_cash.0)
                .saturating_sub(self.regional_protected_cash().0)
                .saturating_sub(self.retained_cash().0),
        )
    }
    pub(crate) fn regional_planned_spending(&self, d: &CountryDefinition) -> i64 {
        self.subnational.as_ref().map_or(0, |state| {
            state
                .regions
                .iter()
                .map(|r| {
                    regional_daily_cost(d.gdp, r.population_share_bps.units_i64(), r.service_level)
                        .0
                })
                .sum::<i64>()
                + state
                    .orders
                    .iter()
                    .filter(|o| o.status == RegionalOrderStatus::Implementing)
                    .map(|o| o.daily_supplement.0)
                    .sum::<i64>()
        })
    }
    pub(crate) fn regional_tax_entitlement(&self, revenue: i64) -> i64 {
        revenue
            * self.subnational.as_ref().map_or(0, |state| {
                state
                    .regions
                    .iter()
                    .map(|r| r.tax_entitlement_bps.units_i64())
                    .sum::<i64>()
            })
            / 10000
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionalPreview {
    pub region: RegionId,
    pub fee: Money,
    pub principal: Money,
    pub daily_supplement: Money,
    pub decision_day: u32,
    pub end_day: Option<u32>,
}
impl World {
    pub fn preview_regional_order(
        &self,
        content: &Content,
        id: &CountryId,
        region: &RegionId,
        kind: &RegionalOrderKind,
    ) -> Result<RegionalPreview, String> {
        let rules = content
            .subnational
            .as_ref()
            .ok_or("This campaign has no subnational mandate workflow")?;
        let mandate = self.regional_mandate_at(content, id, region, self.day)?;
        let country = self.countries.get(id).ok_or("Unknown country")?;
        let state = country
            .subnational
            .as_ref()
            .ok_or("Missing subnational accounts")?;
        let local = state
            .regions
            .iter()
            .find(|r| &r.id == region)
            .ok_or("Missing local authority")?;
        let government = self.governing_rules(content, id)?;
        government.check(country, "subnational", 1)?;
        if self.day < local.next_order_day {
            return Err(format!(
                "This jurisdiction's order interval lasts until {}",
                date(local.next_order_day)
            ));
        }
        if state.orders.iter().any(|o| {
            &o.region == region
                && matches!(
                    o.status,
                    RegionalOrderStatus::Pending | RegionalOrderStatus::Implementing
                )
        }) {
            return Err("A regional decision or funded supplement is already active".into());
        }
        let d = content.country(id).expect("country");
        let (delay, principal, daily_supplement, end_duration) = match kind {
            RegionalOrderKind::Grant { boost, days } => {
                if !(1..=3).contains(boost)
                    || !(30..=180).contains(days)
                    || local.service_level + boost > 10
                {
                    return Err(
                        "Grant supplements require 1–3 levels for 30–180 days, within level 10"
                            .into(),
                    );
                }
                let base = regional_daily_cost(
                    d.gdp,
                    mandate.population_share_bps.units_i64(),
                    local.service_level,
                )
                .0;
                if local.cash.0 > base.max(1) * i64::from(rules.absorption_days.units()) {
                    return Err("The local authority's cash mandate does not permit another funded supplement yet".into());
                }
                let daily = regional_daily_cost(
                    d.gdp,
                    mandate.population_share_bps.units_i64(),
                    local.service_level + boost,
                )
                .0 - base;
                if daily <= 0 {
                    return Err(
                        "The proposed supplement is below the model's money resolution".into(),
                    );
                }
                (
                    rules.grant_review_days.units(),
                    Money(daily * i64::from(*days)),
                    Money(daily),
                    Some(*days),
                )
            }
            RegionalOrderKind::Directive { level } => {
                if mandate.autonomous_services {
                    return Err("This authority controls its own service budget; the national government may offer a grant but cannot order its service level".into());
                }
                if !(0..=8).contains(level) || *level == local.service_level {
                    return Err(
                        "A delegated service level must differ and remain within 0–8".into(),
                    );
                }
                (government.consent_days.units(), Money(0), Money(0), None)
            }
        };
        let decision_day = self
            .day
            .checked_add(delay)
            .filter(|day| *day <= MAX_DAY)
            .ok_or("Regional review exceeds the campaign calendar")?;
        let end_day = match end_duration {
            Some(days) => Some(
                decision_day
                    .checked_add(days - 1)
                    .filter(|day| *day <= MAX_DAY)
                    .ok_or("Funded regional service term exceeds the campaign calendar")?,
            ),
            None => None,
        };
        let fee = Money((d.gdp * rules.administration_cost_bps.units_i64() / 10000).max(1));
        if country.available_cash().0 < fee.0 + principal.0 {
            return Err("Regional offers require national uncommitted cash, excluding local entitlements and existing escrow".into());
        }
        Ok(RegionalPreview {
            region: region.clone(),
            fee,
            principal,
            daily_supplement,
            decision_day,
            end_day,
        })
    }
    pub(crate) fn request_regional_order(
        &mut self,
        content: &Content,
        id: &CountryId,
        region: &RegionId,
        kind: &RegionalOrderKind,
        order: RegionalOrderId,
    ) -> Result<(), String> {
        let preview = self.preview_regional_order(content, id, region, kind)?;
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            preview.fee.0,
            "Subnational decision administration",
        );
        let charter_revision = self.territorial_revision(content, id, self.day);
        let country = self.countries.get_mut(id).expect("country");
        country.budget.capital.0 += preview.fee.0;
        let state = country.subnational.as_mut().expect("subnational");
        let local = state
            .regions
            .iter_mut()
            .find(|r| &r.id == region)
            .expect("region");
        local.next_order_day = self.day
            + content
                .subnational
                .as_ref()
                .expect("rules")
                .cooldown_days
                .units();
        state.orders.push(RegionalOrder {
            id: order,
            region: region.clone(),
            kind: kind.clone(),
            proposed_day: self.day,
            decision_day: preview.decision_day,
            end_day: preview.end_day,
            sponsor: country.government_number,
            charter_revision,
            baseline_level: local.service_level,
            paid_administration: preview.fee,
            principal: preview.principal,
            daily_supplement: preview.daily_supplement,
            escrow: preview.principal,
            remaining: Money(0),
            spent: Money(0),
            status: RegionalOrderStatus::Pending,
            resolved_day: None,
        });
        self.event(id,true,format!("Regional order #{} for {} submitted. Paid {}m administration; {}m remains protected in public escrow until review {}. National and local public cash stay consolidated; this is not an external payment. Withdrawal/refusal releases escrow but retains fees and the order interval.",order.0,region.0,preview.fee.0,preview.principal.0,date(preview.decision_day)));
        if preview.decision_day == self.day {
            self.resolve_regional_orders(content, id);
        }
        Ok(())
    }
    pub(crate) fn cancel_regional_order(
        &mut self,
        id: &CountryId,
        order: RegionalOrderId,
    ) -> Result<(), String> {
        let record = self
            .countries
            .get_mut(id)
            .and_then(|c| c.subnational.as_mut())
            .and_then(|s| s.orders.iter_mut().find(|o| o.id == order))
            .ok_or("Unknown regional order for this government")?;
        if record.status != RegionalOrderStatus::Pending {
            return Err("Only a pending regional order can be withdrawn; an accepted local grant is outside national spending control".into());
        }
        record.escrow = Money(0);
        record.status = RegionalOrderStatus::Withdrawn;
        record.resolved_day = Some(self.day);
        self.event(id,true,format!("Regional order #{} withdrawn. Public escrow released to national availability; administration and interval remain.",order.0));
        Ok(())
    }
    pub(crate) fn regional_succession(&mut self, content: &Content, id: &CountryId) {
        let revision = self.territorial_revision(content, id, self.day);
        let country = self.countries.get_mut(id).expect("country");
        let Some(state) = &mut country.subnational else {
            return;
        };
        let mut changed = false;
        let mut charter_changed = false;
        for order in &mut state.orders {
            if order.status == RegionalOrderStatus::Pending
                && (order.sponsor != country.government_number
                    || order.charter_revision != revision)
            {
                charter_changed |= order.charter_revision != revision;
                order.status = RegionalOrderStatus::Superseded;
                order.resolved_day = Some(self.day);
                order.escrow = Money(0);
                changed = true;
            }
        }
        if charter_changed {
            self.event(id, true, "New territorial charter superseded pending regional offers and released their escrow. Accepted local grant contracts and existing local cash continue.");
        } else if changed {
            self.event(id,true,"Successor cabinet superseded pending regional offers and released their escrow. Accepted local grants and tax entitlements continue under their separate mandates.");
        }
    }
    pub(crate) fn resolve_regional_orders(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.subnational else {
            return;
        };
        let due: Vec<_> = self.countries[id]
            .subnational
            .as_ref()
            .expect("subnational")
            .orders
            .iter()
            .filter(|o| o.status == RegionalOrderStatus::Pending && o.decision_day <= self.day)
            .cloned()
            .collect();
        for order in due {
            let c = &self.countries[id];
            let mandate = rules.mandate(id, &order.region).expect("mandate");
            let state = c.subnational.as_ref().expect("state");
            let local = state
                .regions
                .iter()
                .find(|r| r.id == order.region)
                .expect("region");
            let ordinary = self
                .governing_rules(content, id)
                .expect("government")
                .check(c, "subnational", 1);
            let refusal = if c.government_number != order.sponsor {
                Some("The sponsoring cabinet changed".into())
            } else if let Err(reason) = ordinary {
                Some(reason)
            } else if matches!(order.kind, RegionalOrderKind::Grant { .. })
                && local.cash.0
                    > regional_daily_cost(
                        content.country(id).expect("country").gdp,
                        mandate.population_share_bps.units_i64(),
                        local.service_level,
                    )
                    .0
                    .max(1)
                        * i64::from(rules.absorption_days.units())
            {
                Some("Local cash absorption mandate no longer permits this grant".into())
            } else {
                None
            };
            let c = self.countries.get_mut(id).expect("country");
            let state = c.subnational.as_mut().expect("state");
            let record = state
                .orders
                .iter_mut()
                .find(|o| o.id == order.id)
                .expect("order");
            record.resolved_day = Some(self.day);
            record.escrow = Money(0);
            if let Some(reason) = refusal {
                record.status = RegionalOrderStatus::Refused {
                    reason: reason.clone(),
                };
                self.event(id,true,format!("Regional order #{} refused: {reason}. Escrow released; administration remains spent.",order.id.0));
            } else {
                match order.kind {
                    RegionalOrderKind::Grant { .. } => {
                        record.status = RegionalOrderStatus::Implementing;
                        record.remaining = record.principal;
                        self.event(id,true,format!("Local authority accepted grant #{} for {}. Protected funding purchases {}m/day of additional public-service work through {}. The national government cannot reclaim or redirect accepted funds.",order.id.0,order.region.0,order.daily_supplement.0,date(order.end_day.expect("term"))));
                    }
                    RegionalOrderKind::Directive { level } => {
                        record.status = RegionalOrderStatus::Delivered;
                        state
                            .regions
                            .iter_mut()
                            .find(|r| r.id == order.region)
                            .expect("region")
                            .service_level = level;
                        self.event(id,true,format!("Delegated regional service order #{} delivered: {} now uses level {level}. Actual purchases remain limited by that jurisdiction's protected funds; no physical capacity is created.",order.id.0,order.region.0));
                    }
                }
            }
        }
    }
    pub(crate) fn settle_subnational(
        &mut self,
        content: &Content,
        id: &CountryId,
        revenue: i64,
    ) -> i64 {
        let Some(rules) = &content.subnational else {
            return 0;
        };
        let charter = rules.charter(id).expect("charter");
        let d = content.country(id).expect("country");
        let shares: Vec<_> = self.countries[id]
            .subnational
            .as_ref()
            .expect("subnational")
            .regions
            .iter()
            .map(|r| r.tax_entitlement_bps.units_i64())
            .collect();
        let total_share: i64 = shares.iter().sum();
        let receipts = if total_share == 0 {
            vec![0; shares.len()]
        } else {
            apportion(revenue * total_share / 10000, &shares)
        };
        let public_weights: Vec<_> = d
            .households
            .iter()
            .map(|h| i64::from(h.id == "public"))
            .collect();
        let mut total = 0;
        for (mandate, receipt) in charter.regions.iter().zip(receipts) {
            let state = self
                .countries
                .get_mut(id)
                .expect("country")
                .subnational
                .as_mut()
                .expect("state");
            let local = state
                .regions
                .iter_mut()
                .find(|r| r.id == mandate.id)
                .expect("region");
            local.cash.0 += receipt;
            local.tax_received.0 += receipt;
            let requested = regional_daily_cost(
                d.gdp,
                mandate.population_share_bps.units_i64(),
                local.service_level,
            );
            let base = local.cash.0.min(requested.0);
            local.cash.0 -= base;
            local.base_spent.0 += base;
            let mut supplement = 0;
            let mut completed = Vec::new();
            for order in state
                .orders
                .iter_mut()
                .filter(|o| o.region == mandate.id && o.status == RegionalOrderStatus::Implementing)
            {
                let amount = order.daily_supplement.0.min(order.remaining.0);
                order.remaining.0 -= amount;
                order.spent.0 += amount;
                supplement += amount;
                if self.day == order.end_day.expect("grant term") {
                    order.status = RegionalOrderStatus::Completed;
                    order.resolved_day = Some(self.day);
                    completed.push(order.id);
                }
            }
            local.last = Some(RegionalSettlement {
                day: self.day,
                tax_receipts: Money(receipt),
                requested_base: requested,
                paid_base: Money(base),
                paid_supplement: Money(supplement),
                closing_cash: local.cash,
            });
            let paid = self.pay_income(
                id,
                Account::Treasury,
                base + supplement,
                &public_weights,
                &format!("Regional public-service purchases: {}", mandate.id.0),
            );
            debug_assert_eq!(
                paid,
                base + supplement,
                "Released local funds must remain fully payable"
            );
            total += paid;
            for order in completed {
                self.event(id,true,format!("Regional grant #{} completed its funded service term. Actual public-service household payments remain in accounts; this is not certification of service quality or staffing.",order.0));
            }
        }
        total
    }
    pub(crate) fn ai_regional_order(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.subnational else {
            return;
        };
        if !self.day.is_multiple_of(90) {
            return;
        }
        let kind = RegionalOrderKind::Grant { boost: 1, days: 30 };
        for mandate in &rules.charter(id).expect("charter").regions {
            let local = self.countries[id]
                .subnational
                .as_ref()
                .expect("state")
                .regions
                .iter()
                .find(|r| r.id == mandate.id)
                .expect("region");
            if local.cash.0
                >= regional_daily_cost(
                    content.country(id).expect("country").gdp,
                    mandate.population_share_bps.units_i64(),
                    local.service_level,
                )
                .0
                .max(1)
                    * 10
            {
                continue;
            }
            if self
                .preview_regional_order(content, id, &mandate.id, &kind)
                .is_ok()
            {
                let command = self.command(
                    id.clone(),
                    Action::RegionalOrder {
                        region: mandate.id.clone(),
                        kind: kind.clone(),
                    },
                );
                if let Err(reason) =
                    self.apply(content, command, crate::developer::CommandOrigin::Ai)
                {
                    self.event(
                        id,
                        false,
                        format!("AI regional offer declined by shared rules: {reason}"),
                    );
                }
                break;
            }
        }
    }
}

impl World {
    pub(crate) fn validate_subnational(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[id];
        let (state, rules) = match (&c.subnational, &content.subnational) {
            (None, None) => return Ok(()),
            (Some(state), Some(rules)) => (state, rules),
            _ => return Err("Subnational state differs from content/rules".into()),
        };
        let charter = rules.charter(id).ok_or("Missing territorial charter")?;
        let d = content.country(id).expect("country");
        let start = content.start_day()?;
        let origins: Vec<_> = self
            .journal
            .iter()
            .filter(|entry| {
                &entry.command.issuer == id
                    && matches!(entry.command.action, Action::RegionalOrder { .. })
            })
            .collect();
        if !(0..=1_000_000_000).contains(&c.report.revenue.0)
            || origins.len() != state.orders.len()
            || state.regions.len() != charter.regions.len()
            || c.available_cash().0 < 0
        {
            return Err("Invalid protected public cash or regional order coverage".into());
        }
        let shares: Vec<_> = state
            .regions
            .iter()
            .map(|r| r.tax_entitlement_bps.units_i64())
            .collect();
        let total_share: i64 = shares.iter().sum();
        let receipts = if total_share == 0 {
            vec![0; shares.len()]
        } else {
            apportion(c.report.revenue.0 * total_share / 10000, &shares)
        };
        for ((local, mandate), receipt) in state.regions.iter().zip(&charter.regions).zip(receipts)
        {
            let mandate = self.regional_mandate_at(content, id, &mandate.id, self.day)?;
            let proposals: Vec<_> = state
                .orders
                .iter()
                .filter(|o| o.region == local.id)
                .collect();
            let clock = match proposals.last() {
                Some(order) => order
                    .proposed_day
                    .checked_add(rules.cooldown_days.units())
                    .ok_or("Regional order interval date overflow")?,
                None => start,
            };
            let level = proposals
                .iter()
                .rev()
                .find_map(|o| match (&o.kind, &o.status) {
                    (RegionalOrderKind::Directive { level }, RegionalOrderStatus::Delivered) => {
                        Some(*level)
                    }
                    _ => None,
                })
                .unwrap_or(mandate.service_level);
            // A zero-delay player decree is issued after this day's settlement.
            // Keep the historical plan distinct from the current purchasing level.
            let settled_level = proposals
                .iter()
                .rev()
                .find_map(|order| match (&order.kind, &order.status) {
                    (RegionalOrderKind::Directive { level }, RegionalOrderStatus::Delivered)
                        if order.decision_day <= self.day
                            && (order.decision_day < self.day
                                || order.proposed_day < order.decision_day) =>
                    {
                        Some(*level)
                    }
                    _ => None,
                })
                .unwrap_or(mandate.service_level);
            if !(0..=8).contains(&local.service_level)
                || !(0..=8).contains(&settled_level)
                || local.id != mandate.id
                || local.population_share_bps.units_i64()
                    != mandate.population_share_bps.units_i64()
                || local.tax_entitlement_bps.units_i64() != mandate.tax_entitlement_bps.units_i64()
                || local.service_level != level
                || local.next_order_day != clock
                || proposals
                    .iter()
                    .filter(|o| {
                        matches!(
                            o.status,
                            RegionalOrderStatus::Pending | RegionalOrderStatus::Implementing
                        )
                    })
                    .count()
                    > 1
                || proposals.windows(2).any(|pair| {
                    pair[0]
                        .proposed_day
                        .checked_add(rules.cooldown_days.units())
                        .is_none_or(|clock| pair[1].proposed_day < clock)
                })
                || [local.cash.0, local.tax_received.0, local.base_spent.0]
                    .iter()
                    .any(|v| !(0..=1_000_000_000_000).contains(v))
                || i128::from(local.tax_received.0) - i128::from(local.base_spent.0)
                    + self.regional_internal_adjustment(id, &local.id)
                    != i128::from(local.cash.0)
            {
                return Err(
                    "Invalid regional entitlement, service authority, cash identity or interval"
                        .into(),
                );
            }
            match &local.last {
                None if self.day == start
                    && local.cash.0 == 0
                    && local.tax_received.0 == 0
                    && local.base_spent.0 == 0 => {}
                Some(last) if self.day > start => {
                    let ledger = self
                        .ledger
                        .iter()
                        .rev()
                        .take_while(|entry| entry.day == self.day)
                        .filter(|e| {
                            &e.country == id
                                && e.day == self.day
                                && e.from == Account::Treasury
                                && e.to == Account::Households
                                && e.reason
                                    == format!("Regional public-service purchases: {}", local.id.0)
                        })
                        .map(|e| i128::from(e.amount.0))
                        .sum::<i128>();
                    let supplements = state
                        .orders
                        .iter()
                        .filter(|o| {
                            o.region == local.id
                                && o.decision_day <= self.day
                                && o.end_day.is_some_and(|end| self.day <= end)
                                && matches!(
                                    o.status,
                                    RegionalOrderStatus::Implementing
                                        | RegionalOrderStatus::Completed
                                )
                        })
                        .map(|o| i128::from(o.daily_supplement.0))
                        .sum::<i128>();
                    if last.day != self.day
                        || last.tax_receipts != Money(receipt)
                        || last.requested_base
                            != regional_daily_cost(
                                d.gdp,
                                mandate.population_share_bps.units_i64(),
                                settled_level,
                            )
                        || last.paid_base.0 < 0
                        || last.paid_base.0 > last.requested_base.0
                        || last.paid_base.0 > local.base_spent.0
                        || last.tax_receipts.0 > local.tax_received.0
                        || i128::from(last.paid_supplement.0) != supplements
                        || i128::from(last.closing_cash.0)
                            + self.regional_post_settlement_adjustment(id, &local.id)
                            != i128::from(local.cash.0)
                        || ledger
                            != i128::from(last.paid_base.0) + i128::from(last.paid_supplement.0)
                    {
                        return Err("Regional settlement does not reconcile to its mandate or original transfers".into());
                    }
                }
                _ => return Err("Missing or premature regional settlement".into()),
            }
        }
        for (order, origin) in state.orders.iter().zip(origins) {
            let command = &origin.command;
            let mandate = self.regional_mandate_at(content, id, &order.region, command.day)?;
            let kind = match &command.action {
                Action::RegionalOrder { region, kind }
                    if region == &order.region && kind == &order.kind =>
                {
                    kind
                }
                _ => return Err("Regional order differs from original command".into()),
            };
            let previous_level = state
                .orders
                .iter()
                .rev()
                .filter(|previous| {
                    previous.id.0 < order.id.0
                        && previous.resolved_day.is_some_and(|day| day <= command.day)
                })
                .find_map(|previous| match (&previous.kind, &previous.status) {
                    (RegionalOrderKind::Directive { level }, RegionalOrderStatus::Delivered)
                        if previous.region == order.region =>
                    {
                        Some(*level)
                    }
                    _ => None,
                })
                .unwrap_or(mandate.service_level);
            if order.baseline_level != previous_level {
                return Err("Regional baseline differs from delivered service orders".into());
            }
            let delay = match kind {
                RegionalOrderKind::Grant { .. } => rules.grant_review_days.units(),
                RegionalOrderKind::Directive { .. } => self
                    .governing_rules_at(content, id, command.day)?
                    .consent_days
                    .units(),
            };
            if order.id.0 != command.id.0
                || order.proposed_day != command.day
                || order.charter_revision != self.territorial_revision(content, id, command.day)
                || command.day.checked_add(delay) != Some(order.decision_day)
                || order.decision_day > MAX_DAY
                || order.sponsor == crate::GovernmentId(0)
                || order.sponsor > c.government_number
                || !(0..=8).contains(&order.baseline_level)
                || order.paid_administration
                    != Money((d.gdp * rules.administration_cost_bps.units_i64() / 10000).max(1))
                || [
                    order.principal.0,
                    order.escrow.0,
                    order.remaining.0,
                    order.spent.0,
                    order.daily_supplement.0,
                ]
                .iter()
                .any(|v| !(0..=1_000_000_000_000).contains(v))
            {
                return Err("Invalid regional order origin, cost, dates or sponsorship".into());
            }
            match kind {
                RegionalOrderKind::Grant { boost, days } => {
                    if !(1..=3).contains(boost) || !(30..=180).contains(days) {
                        return Err("Invalid regional grant dimensions".into());
                    }
                    let daily = regional_daily_cost(
                        d.gdp,
                        mandate.population_share_bps.units_i64(),
                        order.baseline_level + boost,
                    )
                    .0 - regional_daily_cost(
                        d.gdp,
                        mandate.population_share_bps.units_i64(),
                        order.baseline_level,
                    )
                    .0;
                    if !(1..=3).contains(boost)
                        || !(30..=180).contains(days)
                        || order.baseline_level + boost > 10
                        || daily <= 0
                        || order.daily_supplement != Money(daily)
                        || order.principal != Money(daily * i64::from(*days))
                        || order.decision_day.checked_add(days - 1) != order.end_day
                        || order.end_day.is_none_or(|day| day > MAX_DAY)
                    {
                        return Err(
                            "Invalid funded regional grant term or financial commitment".into()
                        );
                    }
                }
                RegionalOrderKind::Directive { level } => {
                    if mandate.autonomous_services
                        || !(0..=8).contains(level)
                        || *level == order.baseline_level
                        || order.end_day.is_some()
                        || order.principal.0 != 0
                        || order.daily_supplement.0 != 0
                    {
                        return Err("Regional directive exceeds national jurisdiction or changes grant money".into());
                    }
                }
            }
            let withdrawn=self.journal.iter().any(|entry| &entry.command.issuer==id && Some(entry.command.day)==order.resolved_day && matches!(entry.command.action,Action::CancelRegionalOrder {order:request} if request==order.id));
            if withdrawn != (order.status == RegionalOrderStatus::Withdrawn) {
                return Err("Regional withdrawal differs from original commands".into());
            }
            let released = order.escrow.0 == 0 && order.remaining.0 == 0 && order.spent.0 == 0;
            let valid = match &order.status {
                RegionalOrderStatus::Pending => {
                    order.decision_day > self.day
                        && order.resolved_day.is_none()
                        && order.escrow == order.principal
                        && order.remaining.0 == 0
                        && order.spent.0 == 0
                        && order.sponsor == c.government_number
                        && order.charter_revision
                            == self.territorial_revision(content, id, self.day)
                }
                RegionalOrderStatus::Implementing => {
                    matches!(kind, RegionalOrderKind::Grant { .. })
                        && order.decision_day <= self.day
                        && order.end_day.is_some_and(|end| self.day < end)
                        && order.resolved_day == Some(order.decision_day)
                        && order.escrow.0 == 0
                        && order.spent.0
                            == order.daily_supplement.0
                                * i64::from(self.day - order.decision_day + 1)
                        && order.remaining.0 == order.principal.0 - order.spent.0
                }
                RegionalOrderStatus::Completed => {
                    matches!(kind, RegionalOrderKind::Grant { .. })
                        && order
                            .end_day
                            .is_some_and(|end| end <= self.day && order.resolved_day == Some(end))
                        && order.escrow.0 == 0
                        && order.remaining.0 == 0
                        && order.spent == order.principal
                }
                RegionalOrderStatus::Delivered => {
                    matches!(kind, RegionalOrderKind::Directive { .. })
                        && order.decision_day <= self.day
                        && order.resolved_day == Some(order.decision_day)
                        && released
                }
                RegionalOrderStatus::Withdrawn => {
                    released
                        && order.resolved_day.is_some_and(|day| {
                            day >= order.proposed_day && day < order.decision_day && day <= self.day
                        })
                }
                RegionalOrderStatus::Refused { reason } => {
                    released
                        && !reason.is_empty()
                        && order.resolved_day == Some(order.decision_day)
                        && order.decision_day <= self.day
                }
                RegionalOrderStatus::Superseded => {
                    released
                        && (order.sponsor < c.government_number
                            || order.resolved_day.is_some_and(|day| {
                                order.charter_revision
                                    != self.territorial_revision(content, id, day)
                            }))
                        && order
                            .resolved_day
                            .is_some_and(|day| day >= order.proposed_day && day <= self.day)
                }
            };
            if !valid {
                return Err("Invalid regional decision, grant drawdown or escrow state".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedRegionalAuthority {
    pub region: RegionId,
    pub authority: OfficeId,
    pub name: String,
    pub tier: RegionalTier,
    pub autonomous_services: bool,
    pub service_level: i32,
}
impl World {
    pub(crate) fn published_regions(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Option<Vec<PublishedRegionalAuthority>> {
        let charter = content.subnational.as_ref()?.charter(id)?;
        let state = self.countries.get(id)?.subnational.as_ref()?;
        Some(
            charter
                .regions
                .iter()
                .map(|mandate| PublishedRegionalAuthority {
                    region: mandate.id.clone(),
                    authority: mandate.authority.clone(),
                    name: mandate.name.clone(),
                    tier: mandate.tier.clone(),
                    autonomous_services: self
                        .regional_mandate_at(content, id, &mandate.id, self.day)
                        .expect("regional mandate")
                        .autonomous_services,
                    service_level: state
                        .regions
                        .iter()
                        .find(|r| r.id == mandate.id)
                        .expect("region")
                        .service_level,
                })
                .collect(),
        )
    }
}
