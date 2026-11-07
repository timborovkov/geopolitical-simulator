//! Territorial settlements can change through lawful consent or explicit contested acts.
//! Internal fiscal transfers change discretion, never the consolidated money stock.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct TerritorialOrderId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritorialTemplate {
    pub form: TerritorialForm,
    pub tax_pool_bps: BasisPoints,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequisitionRule {
    pub administration_cost_bps: BasisPoints,
    pub elite_threshold: i32,
    pub council_threshold: i32,
    pub approval_cost: i32,
    pub legislative_cost: i32,
    pub elite_cost: i32,
    pub council_cost: i32,
    pub cooldown_days: DaySpan,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritorialDefinition {
    pub templates: Vec<TerritorialTemplate>,
    pub lawful: TransitionRoute,
    pub contested: Option<TransitionRoute>,
    pub cooldown_days: DaySpan,
    pub local_consent_approval: i32,
    pub compensation_bps: BasisPoints,
    pub requisition: Option<RequisitionRule>,
    pub ai_targets: BTreeMap<CountryId, TerritorialForm>,
    pub source: String,
    pub source_version: String,
}
impl TerritorialDefinition {
    pub fn template(&self, form: &TerritorialForm) -> Option<&TerritorialTemplate> {
        self.templates.iter().find(|t| &t.form == form)
    }
    pub fn route(&self, contested: bool) -> Result<&TransitionRoute, String> {
        if contested {
            self.contested
                .as_ref()
                .ok_or("Contested territorial change is unavailable".into())
        } else {
            Ok(&self.lawful)
        }
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if content.subnational.is_none()
            || self.templates.len() != 3
            || self.source_version.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
            || !(30..=730).contains(&self.cooldown_days.units())
            || !(0..=100).contains(&self.local_consent_approval)
            || !(1..=1000).contains(&self.compensation_bps.units_i64())
            || self
                .ai_targets
                .keys()
                .any(|country| content.country(country).is_none())
        {
            return Err(
                "Invalid territorial templates, consent, compensation or provenance".into(),
            );
        }
        for (index, template) in self.templates.iter().enumerate() {
            if !(0..=4000).contains(&template.tax_pool_bps.units_i64())
                || self.templates[..index]
                    .iter()
                    .any(|other| other.form == template.form)
            {
                return Err("Duplicate or invalid territorial template".into());
            }
        }
        for route in std::iter::once(&self.lawful).chain(self.contested.iter()) {
            if !(1..=365).contains(&route.days.units())
                || !(1..=1000).contains(&route.cost_bps.units_i64())
                || !(1..=10000).contains(&route.upfront_bps.units_i64())
                || [
                    route.legislative_threshold,
                    route.elite_threshold,
                    route.council_threshold,
                    route.approval_threshold,
                    route.approval_cost,
                    route.legislative_cost,
                    route.elite_cost,
                    route.council_cost,
                ]
                .iter()
                .any(|v| !(0..=100).contains(v))
            {
                return Err("Invalid territorial route, backing or costs".into());
            }
        }
        if let Some(rule) = &self.requisition
            && (!(1..=1000).contains(&rule.administration_cost_bps.units_i64())
                || !(30..=730).contains(&rule.cooldown_days.units())
                || [
                    rule.elite_threshold,
                    rule.council_threshold,
                    rule.approval_cost,
                    rule.legislative_cost,
                    rule.elite_cost,
                    rule.council_cost,
                ]
                .iter()
                .any(|v| !(0..=100).contains(v)))
        {
            return Err("Invalid contested requisition powers or costs".into());
        }
        Ok(())
    }
    pub fn apply_template(
        &self,
        base: &TerritorialCharter,
        target: &TerritorialForm,
    ) -> Result<Vec<RegionalMandate>, String> {
        let template = self
            .template(target)
            .ok_or("Missing territorial template")?;
        let weights: Vec<_> = base
            .regions
            .iter()
            .map(|r| r.population_share_bps.units_i64())
            .collect();
        let shares = apportion(template.tax_pool_bps.units_i64(), &weights);
        let fallback = base
            .regions
            .iter()
            .max_by(|a, b| {
                a.population_share_bps
                    .units_i64()
                    .cmp(&b.population_share_bps.units_i64())
                    .then_with(|| a.id.cmp(&b.id))
            })
            .ok_or("Empty territorial charter")?
            .id
            .clone();
        let has_regional = base.regions.iter().any(|r| {
            matches!(
                r.tier,
                RegionalTier::Region | RegionalTier::AutonomousJurisdiction
            )
        });
        Ok(base
            .regions
            .iter()
            .zip(shares)
            .map(|(original, tax)| {
                let mut region = original.clone();
                region.tax_entitlement_bps =
                    BasisPoints::new(u32::try_from(tax).expect("bounded territorial entitlement"));
                region.autonomous_services = match target {
                    TerritorialForm::Federation => true,
                    TerritorialForm::DelegatedUnitary => false,
                    TerritorialForm::DecentralizedUnitary => {
                        if has_regional {
                            matches!(
                                region.tier,
                                RegionalTier::Region | RegionalTier::AutonomousJurisdiction
                            )
                        } else {
                            region.id == fallback
                        }
                    }
                };
                region
            })
            .collect())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalCompensation {
    pub region: RegionId,
    pub amount: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerritorialReformStatus {
    Pending,
    Enacted,
    Withdrawn,
    Superseded,
    Refused { reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritorialReform {
    pub id: TerritorialOrderId,
    pub from: TerritorialForm,
    pub target: TerritorialForm,
    pub contested: bool,
    pub proposed_day: u32,
    pub decision_day: u32,
    pub sponsor: crate::GovernmentId,
    pub admin_cost: Money,
    pub paid_admin: Money,
    pub reserved: Money,
    pub compensations: Vec<RegionalCompensation>,
    pub status: TerritorialReformStatus,
    pub resolved_day: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritorialChange {
    pub request: TerritorialOrderId,
    pub from: TerritorialForm,
    pub to: TerritorialForm,
    pub day: u32,
    pub contested: bool,
    pub compensations: Vec<RegionalCompensation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegionalRequisition {
    pub id: TerritorialOrderId,
    pub day: u32,
    pub region: RegionId,
    pub amount: Money,
    pub fee: Money,
    pub coffer_before: Money,
    pub national_available_before: Money,
    pub sponsor: crate::GovernmentId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritorialState {
    pub next_reform_day: u32,
    pub next_requisition_day: u32,
    pub reforms: Vec<TerritorialReform>,
    pub history: Vec<TerritorialChange>,
    pub requisitions: Vec<RegionalRequisition>,
}
impl TerritorialState {
    pub fn new(start: u32) -> Self {
        Self {
            next_reform_day: start,
            next_requisition_day: start,
            reforms: Vec::new(),
            history: Vec::new(),
            requisitions: Vec::new(),
        }
    }
    pub fn reserved_cash(&self) -> Money {
        Money(
            i64::try_from(
                self.reforms
                    .iter()
                    .map(|r| i128::from(r.reserved.0))
                    .sum::<i128>(),
            )
            .unwrap_or(i64::MAX),
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerritorialPreview {
    pub from: TerritorialForm,
    pub target: TerritorialForm,
    pub contested: bool,
    pub admin_cost: Money,
    pub upfront: Money,
    pub reserved: Money,
    pub decision_day: u32,
    pub rights_lost: Vec<RegionId>,
    pub compensations: Vec<RegionalCompensation>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequisitionPreview {
    pub region: RegionId,
    pub amount: Money,
    pub fee: Money,
    pub coffer: Money,
    pub national_available: Money,
}
impl World {
    pub fn territorial_form_at(
        &self,
        content: &Content,
        id: &CountryId,
        day: u32,
    ) -> Result<TerritorialForm, String> {
        let base = content
            .subnational
            .as_ref()
            .and_then(|r| r.charter(id))
            .ok_or("Missing territorial charter")?;
        Ok(self
            .countries
            .get(id)
            .and_then(|c| c.territorial.as_ref())
            .and_then(|s| s.history.iter().rev().find(|change| change.day <= day))
            .map_or_else(|| base.form.clone(), |change| change.to.clone()))
    }
    pub(crate) fn territorial_revision(
        &self,
        content: &Content,
        id: &CountryId,
        day: u32,
    ) -> Option<u32> {
        content.territorial.as_ref()?;
        let state = self.countries.get(id)?.territorial.as_ref()?;
        Some(
            state
                .history
                .iter()
                .filter(|change| change.day <= day)
                .count() as u32,
        )
    }
    pub fn regional_mandate_at(
        &self,
        content: &Content,
        id: &CountryId,
        region: &RegionId,
        day: u32,
    ) -> Result<RegionalMandate, String> {
        let base = content
            .subnational
            .as_ref()
            .and_then(|r| r.charter(id))
            .ok_or("Missing territorial charter")?;
        if let Some(rules) = &content.territorial
            && let Some(change) = self
                .countries
                .get(id)
                .and_then(|c| c.territorial.as_ref())
                .and_then(|s| s.history.iter().rev().find(|change| change.day <= day))
        {
            return rules
                .apply_template(base, &change.to)?
                .into_iter()
                .find(|r| &r.id == region)
                .ok_or("Unknown budget jurisdiction".into());
        }
        base.regions
            .iter()
            .find(|r| &r.id == region)
            .cloned()
            .ok_or("Unknown budget jurisdiction".into())
    }
    pub(crate) fn regional_internal_adjustment(&self, id: &CountryId, region: &RegionId) -> i128 {
        self.countries
            .get(id)
            .and_then(|c| c.territorial.as_ref())
            .map_or(0, |state| {
                state
                    .history
                    .iter()
                    .flat_map(|h| &h.compensations)
                    .filter(|payment| &payment.region == region)
                    .map(|payment| i128::from(payment.amount.0))
                    .sum::<i128>()
                    - state
                        .requisitions
                        .iter()
                        .filter(|claim| &claim.region == region)
                        .map(|claim| i128::from(claim.amount.0))
                        .sum::<i128>()
            })
    }
    fn check_territorial_backing(
        &self,
        content: &Content,
        id: &CountryId,
        contested: bool,
    ) -> Result<(), String> {
        let rules = content
            .territorial
            .as_ref()
            .ok_or("This campaign has no territorial reform workflow")?;
        let route = rules.route(contested)?;
        let c = self.countries.get(id).ok_or("Unknown government")?;
        let government = self.governing_rules(content, id)?;
        if !contested {
            government.check(c, "territorial_charter", 1)?;
        }
        let backing = if !contested && government.route == DecisionRoute::Legislature {
            c.legislative_support
        } else {
            c.elite_support
        };
        let needed = if !contested && government.route == DecisionRoute::Legislature {
            route.legislative_threshold
        } else {
            route.elite_threshold
        };
        if backing < needed
            || c.council_support < route.council_threshold
            || c.approval < route.approval_threshold
        {
            return Err(format!(
                "Territorial reform requires backing {needed}, council {}, approval {}; current {backing}/{}/{}",
                route.council_threshold, route.approval_threshold, c.council_support, c.approval
            ));
        }
        Ok(())
    }
    fn territorial_compensations(
        &self,
        content: &Content,
        id: &CountryId,
        target: &TerritorialForm,
        contested: bool,
        day: u32,
    ) -> Result<(Vec<RegionId>, Vec<RegionalCompensation>), String> {
        let rules = content
            .territorial
            .as_ref()
            .ok_or("Missing territorial procedures")?;
        let base = content
            .subnational
            .as_ref()
            .and_then(|r| r.charter(id))
            .ok_or("Missing territorial charter")?;
        let future = rules.apply_template(base, target)?;
        let amount = Money(
            (content.country(id).ok_or("Missing country")?.gdp.units()
                * rules.compensation_bps.units_i64()
                / 10000)
                .max(1),
        );
        let mut lost = Vec::new();
        let mut payments = Vec::new();
        for next in future {
            let previous = self.regional_mandate_at(content, id, &next.id, day)?;
            if previous.autonomous_services && !next.autonomous_services
                || next.tax_entitlement_bps.units_i64() < previous.tax_entitlement_bps.units_i64()
            {
                lost.push(next.id.clone());
                if !contested {
                    payments.push(RegionalCompensation {
                        region: next.id,
                        amount,
                    });
                }
            }
        }
        Ok((lost, payments))
    }
    fn check_local_charter_consent(
        &self,
        content: &Content,
        id: &CountryId,
        losses: &[RegionId],
        contested: bool,
    ) -> Result<(), String> {
        if contested || losses.is_empty() {
            return Ok(());
        }
        let needed = content
            .territorial
            .as_ref()
            .expect("rules")
            .local_consent_approval;
        let approval = self.countries[id]
            .households
            .iter()
            .find(|h| h.id == "public")
            .ok_or("Missing public-service group")?
            .approval;
        if approval < needed {
            return Err(format!(
                "Lawful removal of local powers needs public-service group approval {needed}; current {approval}. This is an authored local-consent proxy, not a regional election or force simulation"
            ));
        }
        Ok(())
    }
    pub fn preview_territorial_reform(
        &self,
        content: &Content,
        id: &CountryId,
        target: &TerritorialForm,
        contested: bool,
    ) -> Result<TerritorialPreview, String> {
        if self.content_hash != content.hash() {
            return Err("Territorial preview requires the exact campaign content".into());
        }
        self.check_territorial_backing(content, id, contested)?;
        let rules = content.territorial.as_ref().expect("rules");
        let route = rules.route(contested)?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let state = c
            .territorial
            .as_ref()
            .ok_or("Missing territorial history")?;
        if self.day.units() < state.next_reform_day {
            return Err(format!(
                "Territorial reform interval lasts until {}",
                date(state.next_reform_day)
            ));
        }
        if state
            .reforms
            .iter()
            .any(|r| r.status == TerritorialReformStatus::Pending)
        {
            return Err("A territorial charter review is already active".into());
        }
        let base = content
            .subnational
            .as_ref()
            .and_then(|r| r.charter(id))
            .expect("charter");
        let future = rules.apply_template(base, target)?;
        let mut differs = false;
        for next in &future {
            let previous = self.regional_mandate_at(content, id, &next.id, self.day.units())?;
            differs |= next.autonomous_services != previous.autonomous_services
                || next.tax_entitlement_bps.units_i64() != previous.tax_entitlement_bps.units_i64();
        }
        let from = self.territorial_form_at(content, id, self.day.units())?;
        if !differs && from == *target {
            return Err("This territorial settlement is already in force".into());
        }
        let (rights_lost, compensations) =
            self.territorial_compensations(content, id, target, contested, self.day.units())?;
        self.check_local_charter_consent(content, id, &rights_lost, contested)?;
        let decision_day = self
            .day
            .units()
            .checked_add(route.days.units())
            .filter(|day| *day <= MAX_DAY)
            .ok_or("Territorial review exceeds the campaign calendar")?;
        let admin_cost = Money(
            (content.country(id).expect("country").gdp.units() * route.cost_bps.units_i64()
                / 10000)
                .max(1),
        );
        let upfront = Money((admin_cost.0 * route.upfront_bps.units_i64() / 10000).max(1));
        let reserved = Money(
            admin_cost.0 - upfront.0
                + compensations
                    .iter()
                    .map(|payment| payment.amount.0)
                    .sum::<i64>(),
        );
        if c.available_cash().0 < upfront.0 + reserved.0 {
            return Err("Territorial reform requires national uncommitted cash for administration and all promised local compensation".into());
        }
        Ok(TerritorialPreview {
            from,
            target: target.clone(),
            contested,
            admin_cost,
            upfront,
            reserved,
            decision_day,
            rights_lost,
            compensations,
        })
    }
    pub fn preview_regional_requisition(
        &self,
        content: &Content,
        id: &CountryId,
        region: &RegionId,
        amount: Money,
    ) -> Result<RequisitionPreview, String> {
        if self.content_hash != content.hash() {
            return Err("Requisition preview requires the exact campaign content".into());
        }
        let rules = content
            .territorial
            .as_ref()
            .and_then(|r| r.requisition.as_ref())
            .ok_or("This campaign has no contested requisition route")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let state = c
            .territorial
            .as_ref()
            .ok_or("Missing territorial history")?;
        if self.day.units() < state.next_requisition_day {
            return Err(format!(
                "Contested requisition interval lasts until {}",
                date(state.next_requisition_day)
            ));
        }
        if c.elite_support < rules.elite_threshold || c.council_support < rules.council_threshold {
            return Err(format!(
                "Contested requisition requires elite {}, council {}; current {}/{}",
                rules.elite_threshold, rules.council_threshold, c.elite_support, c.council_support
            ));
        }
        let local = c
            .subnational
            .as_ref()
            .and_then(|s| s.regions.iter().find(|r| &r.id == region))
            .ok_or("This local base account is outside your country")?;
        if amount.0 <= 0 || amount.0 > local.cash.0 {
            return Err("Requisition must be positive and covered by the local base coffer; accepted grants and escrow are protected".into());
        }
        let fee = Money(
            (content.country(id).ok_or("Missing country")?.gdp.units()
                * rules.administration_cost_bps.units_i64()
                / 10000)
                .max(1),
        );
        if c.available_cash().0 + amount.0 < fee.0 {
            return Err(
                "National cash plus the contested base-fund claim must cover administration".into(),
            );
        }
        Ok(RequisitionPreview {
            region: region.clone(),
            amount,
            fee,
            coffer: local.cash,
            national_available: c.available_cash(),
        })
    }
}
impl World {
    pub(crate) fn request_territorial_reform(
        &mut self,
        content: &Content,
        id: &CountryId,
        target: &TerritorialForm,
        contested: bool,
        request: TerritorialOrderId,
    ) -> Result<(), String> {
        let preview = self.preview_territorial_reform(content, id, target, contested)?;
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            preview.upfront.0,
            "Territorial charter review administration",
        );
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += preview.upfront.0;
        c.committed_cash.0 += preview.reserved.0;
        let state = c.territorial.as_mut().expect("territorial");
        state.next_reform_day = self.day.units()
            + content
                .territorial
                .as_ref()
                .expect("rules")
                .cooldown_days
                .units();
        state.reforms.push(TerritorialReform {
            id: request,
            from: preview.from,
            target: preview.target,
            contested,
            proposed_day: self.day.units(),
            decision_day: preview.decision_day,
            sponsor: c.government_number,
            admin_cost: preview.admin_cost,
            paid_admin: preview.upfront,
            reserved: preview.reserved,
            compensations: preview.compensations,
            status: TerritorialReformStatus::Pending,
            resolved_day: None,
        });
        self.event(id,true,format!("Territorial charter request #{} submitted through the {} route. Paid {}m administration; {}m reserved for remaining administration/local compensation. Review {}. Withdrawal or refusal retains paid costs and the reform interval; existing local funds and accepted service grants remain protected.",request.0,if contested {"contested effective-power"} else {"lawful consent"},preview.upfront.0,preview.reserved.0,date(preview.decision_day)));
        Ok(())
    }
    pub(crate) fn withdraw_territorial_reform(
        &mut self,
        id: &CountryId,
        request: TerritorialOrderId,
    ) -> Result<(), String> {
        let c = self.countries.get_mut(id).ok_or("Unknown government")?;
        let record = c
            .territorial
            .as_mut()
            .and_then(|state| state.reforms.iter_mut().find(|r| r.id == request))
            .ok_or("Unknown territorial request for this government")?;
        if record.status != TerritorialReformStatus::Pending {
            return Err("Only a pending territorial review can be withdrawn".into());
        }
        c.committed_cash.0 -= record.reserved.0;
        record.reserved = Money(0);
        record.status = TerritorialReformStatus::Withdrawn;
        record.resolved_day = Some(self.day.units());
        self.event(id,true,format!("Territorial request #{} withdrawn. Remaining reservations released; paid administration and the reform interval remain.",request.0));
        Ok(())
    }
    pub(crate) fn territorial_succession(&mut self, id: &CountryId) {
        let c = self.countries.get_mut(id).expect("country");
        let Some(state) = &mut c.territorial else {
            return;
        };
        let mut changed = false;
        for record in &mut state.reforms {
            if record.status == TerritorialReformStatus::Pending
                && record.sponsor != c.government_number
            {
                c.committed_cash.0 -= record.reserved.0;
                record.reserved = Money(0);
                record.status = TerritorialReformStatus::Superseded;
                record.resolved_day = Some(self.day.units());
                changed = true;
            }
        }
        if changed {
            self.event(id,true,"Successor cabinet superseded the pending territorial charter review. Reservations released; paid administration and the reform interval remain. Enacted local rights, funds and grant contracts continue.");
        }
    }
    fn territorial_political_costs(
        &mut self,
        id: &CountryId,
        approval: i32,
        legislative: i32,
        elite: i32,
        council: i32,
    ) {
        let c = self.countries.get_mut(id).expect("country");
        c.approval = (c.approval - approval).clamp(0, 100);
        c.legislative_support = (c.legislative_support - legislative).clamp(0, 100);
        c.elite_support = (c.elite_support - elite).clamp(0, 100);
        c.council_support = (c.council_support - council).clamp(0, 100);
        for h in &mut c.households {
            h.approval = (h.approval - approval).clamp(0, 100);
        }
    }
    pub(crate) fn resolve_territorial_reforms(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.territorial else {
            return;
        };
        let Some(record) = self.countries[id]
            .territorial
            .as_ref()
            .expect("state")
            .reforms
            .iter()
            .find(|r| {
                r.status == TerritorialReformStatus::Pending && r.decision_day <= self.day.units()
            })
            .cloned()
        else {
            return;
        };
        let route = rules.route(record.contested).expect("route");
        let (losses, _) = self
            .territorial_compensations(
                content,
                id,
                &record.target,
                record.contested,
                self.day.units(),
            )
            .expect("rights");
        let decision = if record.sponsor != self.countries[id].government_number {
            Err("The sponsoring cabinet changed".into())
        } else {
            self.check_territorial_backing(content, id, record.contested)
                .and_then(|()| {
                    self.check_local_charter_consent(content, id, &losses, record.contested)
                })
        };
        let c = self.countries.get_mut(id).expect("country");
        c.committed_cash.0 -= record.reserved.0;
        let pending = c
            .territorial
            .as_mut()
            .expect("state")
            .reforms
            .iter_mut()
            .find(|r| r.id == record.id)
            .expect("record");
        pending.reserved = Money(0);
        pending.resolved_day = Some(self.day.units());
        if let Err(reason) = decision {
            pending.status = TerritorialReformStatus::Refused {
                reason: reason.clone(),
            };
            self.event(id,true,format!("Territorial request #{} refused: {reason}. Unspent reservations released; paid administration and reform interval remain.",record.id.0));
            return;
        }
        let remaining_admin = record.admin_cost.0 - record.paid_admin.0;
        let paid = self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            remaining_admin,
            "Enacted territorial charter administration",
        );
        debug_assert_eq!(paid, remaining_admin);
        let base = content
            .subnational
            .as_ref()
            .and_then(|r| r.charter(id))
            .expect("charter");
        let future = rules
            .apply_template(base, &record.target)
            .expect("template");
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += paid;
        for next in &future {
            c.subnational
                .as_mut()
                .expect("local accounts")
                .regions
                .iter_mut()
                .find(|r| r.id == next.id)
                .expect("region")
                .tax_entitlement_bps = next.tax_entitlement_bps;
        }
        for payment in &record.compensations {
            c.subnational
                .as_mut()
                .expect("local accounts")
                .regions
                .iter_mut()
                .find(|r| r.id == payment.region)
                .expect("region")
                .cash
                .0 += payment.amount.0;
        }
        let state = c.territorial.as_mut().expect("state");
        let pending = state
            .reforms
            .iter_mut()
            .find(|r| r.id == record.id)
            .expect("record");
        pending.status = TerritorialReformStatus::Enacted;
        pending.paid_admin = record.admin_cost;
        state.history.push(TerritorialChange {
            request: record.id,
            from: record.from,
            to: record.target.clone(),
            day: self.day.units(),
            contested: record.contested,
            compensations: record.compensations.clone(),
        });
        self.territorial_political_costs(
            id,
            route.approval_cost,
            route.legislative_cost,
            route.elite_cost,
            route.council_cost,
        );
        self.event(id,true,format!("Territorial request #{} enacted: {:?}. New autonomy and collected-tax rights now apply. {}m local compensation was reclassified within consolidated public cash, not paid outside it. Existing local base cash and accepted service grants remain with their authorities; pending regional mandates must use the new charter.",record.id.0,record.target,record.compensations.iter().map(|p|p.amount.0).sum::<i64>()));
    }
    pub(crate) fn requisition_regional_funds(
        &mut self,
        content: &Content,
        id: &CountryId,
        region: &RegionId,
        amount: Money,
        request: TerritorialOrderId,
    ) -> Result<(), String> {
        let preview = self.preview_regional_requisition(content, id, region, amount)?;
        let rule = content
            .territorial
            .as_ref()
            .and_then(|r| r.requisition.as_ref())
            .expect("requisition");
        // The contested act explicitly reclassifies only an existing BASE coffer.
        // Accepted grant balances and pending escrow remain separate and protected.
        self.countries
            .get_mut(id)
            .expect("country")
            .subnational
            .as_mut()
            .expect("local accounts")
            .regions
            .iter_mut()
            .find(|r| &r.id == region)
            .expect("region")
            .cash
            .0 -= amount.0;
        let paid = self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            preview.fee.0,
            "Contested local-fund requisition administration",
        );
        debug_assert_eq!(paid, preview.fee.0);
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += paid;
        let state = c.territorial.as_mut().expect("state");
        state.next_requisition_day = self.day.units() + rule.cooldown_days.units();
        state.requisitions.push(RegionalRequisition {
            id: request,
            day: self.day.units(),
            region: region.clone(),
            amount,
            fee: preview.fee,
            coffer_before: preview.coffer,
            national_available_before: preview.national_available,
            sponsor: c.government_number,
        });
        self.territorial_political_costs(
            id,
            rule.approval_cost,
            rule.legislative_cost,
            rule.elite_cost,
            rule.council_cost,
        );
        self.event(id,true,format!("Contested requisition #{} violated {}'s local mandate: {}m of existing base funds reclassified to national discretion; {}m administration paid. No consolidated principal was created. Accepted service grants and escrow remain protected. Political costs: approval -{}, legislature -{}, elite -{}, council -{}; requisition interval {} days.",request.0,region.0,amount.0,preview.fee.0,rule.approval_cost,rule.legislative_cost,rule.elite_cost,rule.council_cost,rule.cooldown_days.units()));
        Ok(())
    }
    pub(crate) fn ai_territorial_turn(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.territorial else {
            return;
        };
        if !self.day.units().is_multiple_of(180) {
            return;
        }
        if self.countries[id]
            .constitution
            .as_ref()
            .is_none_or(|state| state.pending.is_none())
            && let Some(target) = rules.ai_targets.get(id)
            && self
                .preview_territorial_reform(content, id, target, false)
                .is_ok()
        {
            let command = self.command(
                id.clone(),
                Action::ReformTerritory {
                    target: target.clone(),
                    contested: false,
                },
            );
            if let Err(reason) = self.apply(content, command, crate::developer::CommandOrigin::Ai) {
                self.event(
                    id,
                    false,
                    format!("AI territorial reform declined by shared rules: {reason}"),
                );
            }
            return;
        }
        if self.countries[id].available_cash().0 > 0 || self.countries[id].arrears.0 == 0 {
            return;
        }
        for region in &content
            .subnational
            .as_ref()
            .and_then(|r| r.charter(id))
            .expect("charter")
            .regions
        {
            let local = self.countries[id]
                .subnational
                .as_ref()
                .expect("local accounts")
                .regions
                .iter()
                .find(|r| r.id == region.id)
                .expect("region");
            let amount = local.cash;
            if self
                .preview_regional_requisition(content, id, &region.id, amount)
                .is_ok()
            {
                let command = self.command(
                    id.clone(),
                    Action::RequisitionRegionalFunds {
                        region: region.id.clone(),
                        amount,
                    },
                );
                if let Err(reason) =
                    self.apply(content, command, crate::developer::CommandOrigin::Ai)
                {
                    self.event(
                        id,
                        false,
                        format!("AI contested requisition declined by shared rules: {reason}"),
                    );
                }
                break;
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedTerritorialChange {
    pub day: u32,
    pub form: TerritorialForm,
    pub contested: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedRequisition {
    pub day: u32,
    pub region: RegionId,
    pub amount: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritorialReport {
    pub form: TerritorialForm,
    pub last_change: Option<PublishedTerritorialChange>,
    pub last_requisition: Option<PublishedRequisition>,
}
impl World {
    pub(crate) fn published_territorial(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Option<TerritorialReport> {
        content.territorial.as_ref()?;
        let state = self.countries.get(id)?.territorial.as_ref()?;
        Some(TerritorialReport {
            form: self
                .territorial_form_at(content, id, self.day.units())
                .ok()?,
            last_change: state
                .history
                .last()
                .map(|change| PublishedTerritorialChange {
                    day: change.day,
                    form: change.to.clone(),
                    contested: change.contested,
                }),
            last_requisition: state.requisitions.last().map(|claim| PublishedRequisition {
                day: claim.day,
                region: claim.region.clone(),
                amount: claim.amount,
            }),
        })
    }
}
impl World {
    pub(crate) fn validate_territorial(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[id];
        let (state, rules) = match (&c.territorial, &content.territorial) {
            (None, None) => return Ok(()),
            (Some(state), Some(rules)) => (state, rules),
            _ => return Err("Territorial state differs from campaign procedures".into()),
        };
        let start = content.start_day()?;
        let d = content.country(id).expect("country");
        let reform_origins: Vec<_> = self
            .journal
            .iter()
            .filter(|entry| {
                &entry.command.issuer == id
                    && matches!(entry.command.action, Action::ReformTerritory { .. })
            })
            .collect();
        let requisition_origins: Vec<_> = self
            .journal
            .iter()
            .filter(|entry| {
                &entry.command.issuer == id
                    && matches!(
                        entry.command.action,
                        Action::RequisitionRegionalFunds { .. }
                    )
            })
            .collect();
        let reform_clock = match reform_origins.last() {
            Some(entry) => entry
                .command
                .day
                .checked_add(rules.cooldown_days.units())
                .ok_or("Territorial interval overflow")?,
            None => start,
        };
        let requisition_clock = match requisition_origins.last() {
            Some(entry) => entry
                .command
                .day
                .checked_add(
                    rules
                        .requisition
                        .as_ref()
                        .ok_or("Requisition journal without procedures")?
                        .cooldown_days
                        .units(),
                )
                .ok_or("Requisition interval overflow")?,
            None => start,
        };
        if state.reforms.len() != reform_origins.len()
            || state.requisitions.len() != requisition_origins.len()
            || state.next_reform_day != reform_clock
            || state.next_requisition_day != requisition_clock
            || state
                .reforms
                .iter()
                .filter(|r| r.status == TerritorialReformStatus::Pending)
                .count()
                > 1
        {
            return Err("Territorial records or intervals differ from accepted commands".into());
        }
        for (record, origin) in state.reforms.iter().zip(&reform_origins) {
            let command = &origin.command;
            let route = rules.route(record.contested)?;
            let matches = matches!(&command.action,Action::ReformTerritory{target,contested} if target==&record.target&&contested==&record.contested);
            let (losses, compensations) = self.territorial_compensations(
                content,
                id,
                &record.target,
                record.contested,
                command.day,
            )?;
            let cost = Money((d.gdp.units() * route.cost_bps.units_i64() / 10000).max(1));
            let upfront = Money((cost.0 * route.upfront_bps.units_i64() / 10000).max(1));
            let reservation =
                Money(cost.0 - upfront.0 + compensations.iter().map(|p| p.amount.0).sum::<i64>());
            if !matches
                || record.id.0 != command.id.0
                || record.proposed_day != command.day
                || command.day.checked_add(route.days.units()) != Some(record.decision_day)
                || record.decision_day > MAX_DAY
                || record.from != self.territorial_form_at(content, id, command.day)?
                || record.admin_cost != cost
                || record.compensations != compensations
                || record.sponsor == crate::GovernmentId(0)
                || record.sponsor > c.government_number
            {
                return Err(
                    "Invalid territorial reform origin, date, powers or financial promise".into(),
                );
            }
            let base = content
                .subnational
                .as_ref()
                .and_then(|r| r.charter(id))
                .expect("charter");
            let mut differs = record.from != record.target;
            for next in rules.apply_template(base, &record.target)? {
                let previous = self.regional_mandate_at(content, id, &next.id, command.day)?;
                differs |= previous.autonomous_services != next.autonomous_services
                    || previous.tax_entitlement_bps.units_i64()
                        != next.tax_entitlement_bps.units_i64();
            }
            if !differs {
                return Err("Territorial reform records a settlement already in force".into());
            }
            if !record.contested && losses.len() != record.compensations.len() {
                return Err("Local rights losses require explicit funded compensation".into());
            }
            let withdrawal=self.journal.iter().filter(|entry|&entry.command.issuer==id&&Some(entry.command.day)==record.resolved_day&&matches!(entry.command.action,Action::WithdrawTerritorialReform{request} if request==record.id)).count();
            if withdrawal != usize::from(record.status == TerritorialReformStatus::Withdrawn) {
                return Err("Territorial withdrawal differs from accepted commands".into());
            }
            let final_day = record.resolved_day.is_some_and(|day| {
                day >= record.proposed_day && day <= record.decision_day && day <= self.day.units()
            });
            let valid = match &record.status {
                TerritorialReformStatus::Pending => {
                    record.resolved_day.is_none()
                        && record.decision_day > self.day.units()
                        && record.sponsor == c.government_number
                        && record.paid_admin == upfront
                        && record.reserved == reservation
                }
                TerritorialReformStatus::Enacted => {
                    record.resolved_day == Some(record.decision_day)
                        && record.decision_day <= self.day.units()
                        && record.paid_admin == cost
                        && record.reserved.0 == 0
                }
                TerritorialReformStatus::Withdrawn => {
                    final_day
                        && record
                            .resolved_day
                            .is_some_and(|day| day < record.decision_day)
                        && record.paid_admin == upfront
                        && record.reserved.0 == 0
                }
                TerritorialReformStatus::Refused { reason } => {
                    !reason.is_empty()
                        && record.resolved_day == Some(record.decision_day)
                        && record.decision_day <= self.day.units()
                        && record.paid_admin == upfront
                        && record.reserved.0 == 0
                }
                TerritorialReformStatus::Superseded => {
                    final_day
                        && record.sponsor < c.government_number
                        && record.paid_admin == upfront
                        && record.reserved.0 == 0
                }
            };
            if !valid {
                return Err(
                    "Invalid territorial decision, sponsorship or reservation state".into(),
                );
            }
        }
        for pair in state.reforms.windows(2) {
            if pair[0]
                .proposed_day
                .checked_add(rules.cooldown_days.units())
                .is_none_or(|day| pair[1].proposed_day < day)
                || pair[0]
                    .resolved_day
                    .is_none_or(|day| day > pair[1].proposed_day)
            {
                return Err("Overlapping territorial charter reviews or reform intervals".into());
            }
        }
        let enacted: Vec<_> = state
            .reforms
            .iter()
            .filter(|r| r.status == TerritorialReformStatus::Enacted)
            .collect();
        if enacted.len() != state.history.len() {
            return Err("Territorial history differs from enacted reviews".into());
        }
        let mut previous = content
            .subnational
            .as_ref()
            .and_then(|r| r.charter(id))
            .expect("charter")
            .form
            .clone();
        for (change, record) in state.history.iter().zip(enacted) {
            if change.request != record.id
                || change.day != record.decision_day
                || change.from != record.from
                || change.from != previous
                || change.to != record.target
                || change.contested != record.contested
                || change.compensations != record.compensations
            {
                return Err(
                    "Invalid dated territorial settlement or local compensation history".into(),
                );
            }
            previous = change.to.clone();
        }
        if state
            .history
            .windows(2)
            .any(|pair| pair[0].day >= pair[1].day)
        {
            return Err("Territorial settlement dates are not ordered".into());
        }
        for (claim, origin) in state.requisitions.iter().zip(&requisition_origins) {
            let rule = rules
                .requisition
                .as_ref()
                .ok_or("Requisition without explicit contested powers")?;
            let command = &origin.command;
            let matches = matches!(&command.action,Action::RequisitionRegionalFunds{region,amount} if region==&claim.region&&amount==&claim.amount);
            if !matches
                || claim.id.0 != command.id.0
                || claim.day != command.day
                || claim.sponsor == crate::GovernmentId(0)
                || claim.sponsor > c.government_number
                || content
                    .subnational
                    .as_ref()
                    .and_then(|r| r.mandate(id, &claim.region))
                    .is_none()
                || [
                    claim.amount.0,
                    claim.coffer_before.0,
                    claim.national_available_before.0,
                ]
                .iter()
                .any(|value| !(0..=1_000_000_000_000).contains(value))
                || claim.amount.0 == 0
                || claim.amount.0 > claim.coffer_before.0
                || claim.fee
                    != Money(
                        (d.gdp.units() * rule.administration_cost_bps.units_i64() / 10000).max(1),
                    )
                || claim.fee.0 > claim.national_available_before.0 + claim.amount.0
            {
                return Err(
                    "Invalid contested base-fund requisition origin, resources or administration"
                        .into(),
                );
            }
        }
        if let Some(rule) = &rules.requisition
            && state.requisitions.windows(2).any(|pair| {
                pair[0]
                    .day
                    .checked_add(rule.cooldown_days.units())
                    .is_none_or(|day| pair[1].day < day)
            })
        {
            return Err("Contested requisition interval violated".into());
        }
        Ok(())
    }
    pub(crate) fn regional_post_settlement_adjustment(
        &self,
        id: &CountryId,
        region: &RegionId,
    ) -> i128 {
        -self
            .countries
            .get(id)
            .and_then(|c| c.territorial.as_ref())
            .map_or(0, |state| {
                state
                    .requisitions
                    .iter()
                    .filter(|claim| claim.day == self.day.units() && &claim.region == region)
                    .map(|claim| i128::from(claim.amount.0))
                    .sum::<i128>()
            })
    }
}
