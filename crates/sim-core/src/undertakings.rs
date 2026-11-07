//! Costed domestic undertakings reuse policy milestones and actual delivery evidence.
//! Declaring a promise does not enact its policy or supply its resources.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct UndertakingId(pub u64);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum UndertakingKind {
    PublicPledge,
    SupportAgreement,
    ProjectDeadline,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalDirection {
    AtLeast,
    AtMost,
}
impl GoalDirection {
    fn met(self, actual: i32, target: i32) -> bool {
        match self {
            Self::AtLeast => actual >= target,
            Self::AtMost => actual <= target,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoliticalEffect {
    pub approval: i32,
    pub legislative: i32,
    pub elite: i32,
    pub council: i32,
}
impl PoliticalEffect {
    fn valid(&self) -> bool {
        [self.approval, self.legislative, self.elite, self.council]
            .iter()
            .all(|v| (-30..=30).contains(v))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UndertakingTemplate {
    pub id: String,
    pub name: String,
    pub kind: UndertakingKind,
    pub policy: Option<PolicyId>,
    pub direction: GoalDirection,
    pub delta: i32,
    pub review_days: DaySpan,
    pub deadline_days: DaySpan,
    pub administration_cost_bps: BasisPoints,
    pub minimum_credibility: i32,
    pub elite_threshold: i32,
    pub council_threshold: i32,
    pub acceptance: PoliticalEffect,
    pub breach: PoliticalEffect,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UndertakingDefinition {
    pub templates: Vec<UndertakingTemplate>,
    pub cooldown_days: DaySpan,
    pub maximum_open: usize,
    pub source: String,
    pub source_version: String,
}
fn policy_field(policy: &PolicyId) -> Option<&'static str> {
    match policy.as_str() {
        "income_tax" => Some("income_tax"),
        "services" => Some("services"),
        "welfare" => Some("welfare"),
        "energy" => Some("energy_capacity"),
        "agriculture" => Some("food_capacity"),
        "administration" => Some("administration"),
        _ => None,
    }
}
fn policy_level(c: &Country, policy: &PolicyId) -> Option<i32> {
    match policy.as_str() {
        "income_tax" => Some(c.tax),
        "services" => Some(c.services),
        "welfare" => Some(c.welfare),
        "energy" => Some(c.energy_capacity),
        "agriculture" => Some(c.food_capacity),
        "administration" => Some(c.administration),
        _ => None,
    }
}
fn target_in_range(content: &Content, policy: &PolicyId, value: i32) -> bool {
    match policy.as_str() {
        "energy" | "agriculture" => (0..=200).contains(&value),
        "administration" => (0..=100).contains(&value),
        _ => content
            .policy(policy)
            .is_some_and(|p| (p.min..=p.max).contains(&value)),
    }
}
impl UndertakingDefinition {
    pub fn template(&self, id: &str) -> Option<&UndertakingTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if content.commitments.is_none()
            || self.templates.is_empty()
            || self.templates.len() > 24
            || !(1..=3).contains(&self.maximum_open)
            || !(30..=730).contains(&self.cooldown_days.units())
            || self.source_version.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("Invalid undertaking lifecycle, bounds or provenance".into());
        }
        let mut ids = BTreeSet::new();
        for t in &self.templates {
            let goal = if t.kind == UndertakingKind::ProjectDeadline {
                t.policy.is_none() && t.delta == 0
            } else {
                t.policy
                    .as_ref()
                    .is_some_and(|p| policy_field(p).is_some() && content.policy(p).is_some())
                    && match t.direction {
                        GoalDirection::AtLeast => (1..=50).contains(&t.delta),
                        GoalDirection::AtMost => (-50..=-1).contains(&t.delta),
                    }
            };
            if t.id.is_empty()
                || t.name.is_empty()
                || !ids.insert(&t.id)
                || !goal
                || !(1..=90).contains(&t.review_days.units())
                || !(1..=730).contains(&t.deadline_days.units())
                || !(1..=1000).contains(&t.administration_cost_bps.units_i64())
                || [
                    t.minimum_credibility,
                    t.elite_threshold,
                    t.council_threshold,
                ]
                .iter()
                .any(|v| !(0..=100).contains(v))
                || !t.acceptance.valid()
                || !t.breach.valid()
            {
                return Err("Invalid undertaking template, goal, effects or dates".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyDeliveryGoal {
    pub policy: PolicyId,
    pub direction: GoalDirection,
    pub target: i32,
}
impl PolicyDeliveryGoal {
    pub fn description(&self, content: &Content) -> String {
        let name = match self.policy.as_str() {
            "energy" => "Energy capacity",
            "agriculture" => "Food capacity",
            "administration" => "Administrative capacity",
            _ => content
                .policy(&self.policy)
                .map_or(self.policy.as_str(), |p| p.name.as_str()),
        };
        let direction = match self.direction {
            GoalDirection::AtLeast => "at least",
            GoalDirection::AtMost => "at most",
        };
        let units = match self.policy.as_str() {
            "income_tax" => "%",
            "services" | "welfare" => "% of opening GDP",
            _ => " points",
        };
        format!("{name} {direction} {}{units}", self.target)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum UndertakingStatus {
    Pending,
    Active,
    Fulfilled { evidence: CommitmentId },
    Breached,
    Withdrawn,
    Superseded,
    Refused { reason: String },
}
impl UndertakingStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Pending => "Awaiting review".into(),
            Self::Active => "Accepted; awaiting delivery".into(),
            Self::Fulfilled { evidence } => format!("Fulfilled by policy #{}", evidence.0),
            Self::Breached => "Deadline missed".into(),
            Self::Withdrawn => "Withdrawn".into(),
            Self::Superseded => "Closed after cabinet succession".into(),
            Self::Refused { reason } => format!("Refused: {reason}"),
        }
    }
    fn open(&self) -> bool {
        matches!(self, Self::Pending | Self::Active)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UndertakingRecord {
    pub id: UndertakingId,
    pub template: String,
    pub kind: UndertakingKind,
    pub goal: PolicyDeliveryGoal,
    pub baseline: i32,
    pub linked_project: Option<CommitmentId>,
    pub declared_day: u32,
    pub review_day: u32,
    pub deadline_day: u32,
    pub sponsor: crate::GovernmentId,
    pub fee: Money,
    pub accepted_day: Option<u32>,
    pub resolved_day: Option<u32>,
    pub status: UndertakingStatus,
    pub milestones: Vec<CommitmentMilestone>,
}
impl UndertakingRecord {
    fn mark(&mut self, day: u32, stage: CommitmentStage) {
        self.milestones.push(CommitmentMilestone {
            day,
            stage,
            value: self.goal.target,
        });
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Undertakings {
    pub credibility: i32,
    pub next_declaration: BTreeMap<UndertakingKind, u32>,
    pub records: Vec<UndertakingRecord>,
}
impl Undertakings {
    pub fn new(start: u32) -> Self {
        Self {
            credibility: 60,
            next_declaration: [
                UndertakingKind::PublicPledge,
                UndertakingKind::SupportAgreement,
                UndertakingKind::ProjectDeadline,
            ]
            .into_iter()
            .map(|k| (k, start))
            .collect(),
            records: Vec::new(),
        }
    }
    fn computed_credibility(&self) -> i32 {
        let score = 60_i128
            + self
                .records
                .iter()
                .map(|r| match r.status {
                    UndertakingStatus::Fulfilled { .. } => 5,
                    UndertakingStatus::Breached => -10,
                    UndertakingStatus::Withdrawn if r.accepted_day.is_some() => -8,
                    UndertakingStatus::Superseded if r.accepted_day.is_some() => -3,
                    _ => 0,
                })
                .sum::<i128>();
        score.clamp(0, 100) as i32
    }
    fn refresh_credibility(&mut self) {
        self.credibility = self.computed_credibility();
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndertakingPreview {
    pub kind: UndertakingKind,
    pub goal: PolicyDeliveryGoal,
    pub fee: Money,
    pub review_day: u32,
    pub deadline_day: u32,
    pub expected_delivery: Option<u32>,
    pub bottleneck: Option<String>,
}
impl World {
    fn undertaking_backing(&self, id: &CountryId, t: &UndertakingTemplate) -> Result<(), String> {
        let c = self.countries.get(id).ok_or("Unknown government")?;
        if let Some(state) = &c.undertakings
            && state.credibility < t.minimum_credibility
        {
            return Err(format!(
                "This undertaking needs credibility {}; current {}",
                t.minimum_credibility, state.credibility
            ));
        }
        if c.elite_support < t.elite_threshold || c.council_support < t.council_threshold {
            return Err(format!(
                "Undertaking review needs elite {}, council {}; current {}/{}",
                t.elite_threshold, t.council_threshold, c.elite_support, c.council_support
            ));
        }
        Ok(())
    }
    pub fn preview_undertaking(
        &self,
        content: &Content,
        id: &CountryId,
        template: &str,
        project: Option<CommitmentId>,
    ) -> Result<UndertakingPreview, String> {
        if self.content_hash != content.hash() {
            return Err("Undertaking preview requires exact campaign content".into());
        }
        let rules = content
            .undertakings
            .as_ref()
            .ok_or("This campaign has no delivery undertakings")?;
        let t = rules
            .template(template)
            .ok_or("Unknown undertaking template")?;
        let c = self.countries.get(id).ok_or("Unknown government")?;
        let state = c
            .undertakings
            .as_ref()
            .ok_or("Missing undertaking history")?;
        self.undertaking_backing(id, t)?;
        if self.day.units() < state.next_declaration[&t.kind] {
            return Err(format!(
                "This undertaking interval lasts until {}",
                date(state.next_declaration[&t.kind])
            ));
        }
        if state.records.iter().filter(|r| r.status.open()).count() >= rules.maximum_open
            || state
                .records
                .iter()
                .any(|r| r.kind == t.kind && r.status.open())
        {
            return Err("An undertaking of this kind is already open, or the open-commitment limit is reached".into());
        }
        let goal = if t.kind == UndertakingKind::ProjectDeadline {
            let request = project
                .ok_or("Choose an existing uncompleted policy commitment for this deadline")?;
            let record = c
                .commitments
                .as_ref()
                .and_then(|rs| rs.iter().find(|r| r.id == request))
                .ok_or("Unknown owned policy commitment")?;
            if !c.projects.iter().any(|p| p.id == record.project.id)
                || policy_field(&record.project.policy).is_none()
            {
                return Err(
                    "A delivery deadline requires an actual pending non-negotiation policy project"
                        .into(),
                );
            }
            let current = policy_level(c, &record.project.policy).expect("policy");
            let target = match record.project.policy.as_str() {
                "energy" | "agriculture" => (current + 15).min(200),
                "administration" => (current + 8).min(100),
                _ => record.project.value,
            };
            if target == current {
                return Err(
                    "The attached project already matches the current operating setting".into(),
                );
            }
            PolicyDeliveryGoal {
                policy: record.project.policy.clone(),
                direction: if target > current {
                    GoalDirection::AtLeast
                } else {
                    GoalDirection::AtMost
                },
                target,
            }
        } else {
            if project.is_some() {
                return Err(
                    "Only a project-deadline template accepts an attached commitment".into(),
                );
            }
            let policy = t.policy.clone().expect("template");
            let current = policy_level(c, &policy).expect("policy");
            let target = current
                .checked_add(t.delta)
                .ok_or("Undertaking target overflow")?;
            if !target_in_range(content, &policy, target) {
                return Err("The promised target exceeds the policy or capacity bounds".into());
            }
            PolicyDeliveryGoal {
                policy,
                direction: t.direction,
                target,
            }
        };
        let review_day = self
            .day
            .units()
            .checked_add(t.review_days.units())
            .ok_or("Undertaking review overflow")?;
        let deadline_day = review_day
            .checked_add(t.deadline_days.units())
            .filter(|d| *d <= MAX_DAY)
            .ok_or("Undertaking deadline exceeds the campaign calendar")?;
        let fee = Money(
            (content.country(id).expect("country").gdp.units()
                * t.administration_cost_bps.units_i64()
                / 10000)
                .max(1),
        );
        if c.available_cash().0 < fee.0 {
            return Err(
                "Insufficient national uncommitted cash for undertaking administration".into(),
            );
        }
        let (expected_delivery, bottleneck) = if let Some(p) = c
            .projects
            .iter()
            .find(|p| project.map_or(p.policy == goal.policy, |r| r == p.id))
        {
            (Some(p.due_day),Some("Existing project date is conditional on consent, funding, amendments and withdrawal; the undertaking cannot implement it".into()))
        } else {
            let value = if matches!(
                goal.policy.as_str(),
                "energy" | "agriculture" | "administration"
            ) {
                1
            } else {
                goal.target
            };
            match self.preview(content,id,&goal.policy,value){Ok(p)=>(self.day.units().checked_add(p.delay.units()),Some("Indicative date for a separate policy order issued now under current backing and resources; no policy has been ordered".into())),Err(reason)=>(None,Some(reason))}
        };
        Ok(UndertakingPreview {
            kind: t.kind,
            goal,
            fee,
            review_day,
            deadline_day,
            expected_delivery,
            bottleneck,
        })
    }
    fn undertaking_evidence(
        &self,
        id: &CountryId,
        record: &UndertakingRecord,
        through: u32,
    ) -> Option<(CommitmentId, u32)> {
        let accepted = record.accepted_day?;
        if through < accepted {
            return None;
        }
        let from = record.declared_day;
        let field = policy_field(&record.goal.policy)?;
        self.countries[id]
            .commitments
            .as_ref()?
            .iter()
            .filter_map(|policy| {
                if policy.project.policy != record.goal.policy
                    || record.linked_project.is_some_and(|r| r != policy.id)
                {
                    return None;
                }
                let delivered = policy
                    .milestones
                    .iter()
                    .find(|m| m.stage == CommitmentStage::Operating)?
                    .day;
                if delivered < from
                    || delivered > through.min(record.deadline_day)
                    || !policy.direct_effects.iter().any(|e| {
                        e.field == field && record.goal.direction.met(e.after, record.goal.target)
                    })
                {
                    return None;
                }
                Some((policy.id, delivered))
            })
            .min_by_key(|(id, day)| (*day, id.0))
    }
    fn undertaking_effect(&mut self, id: &CountryId, effect: &PoliticalEffect) {
        let c = self.countries.get_mut(id).expect("country");
        c.approval = (c.approval + effect.approval).clamp(0, 100);
        c.legislative_support = (c.legislative_support + effect.legislative).clamp(0, 100);
        c.elite_support = (c.elite_support + effect.elite).clamp(0, 100);
        c.council_support = (c.council_support + effect.council).clamp(0, 100);
        for h in &mut c.households {
            h.approval = (h.approval + effect.approval).clamp(0, 100);
        }
    }
}
impl World {
    pub(crate) fn declare_undertaking(
        &mut self,
        content: &Content,
        id: &CountryId,
        template: &str,
        project: Option<CommitmentId>,
        request: UndertakingId,
    ) -> Result<(), String> {
        let preview = self.preview_undertaking(content, id, template, project)?;
        let baseline = policy_level(&self.countries[id], &preview.goal.policy).expect("policy");
        let rules = content.undertakings.as_ref().expect("rules");
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            preview.fee.0,
            "Delivery undertaking administration",
        );
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += preview.fee.0;
        let state = c.undertakings.as_mut().expect("undertakings");
        state
            .next_declaration
            .insert(preview.kind, self.day.units() + rules.cooldown_days.units());
        let mut record = UndertakingRecord {
            id: request,
            template: template.into(),
            kind: preview.kind,
            goal: preview.goal.clone(),
            baseline,
            linked_project: project,
            declared_day: self.day.units(),
            review_day: preview.review_day,
            deadline_day: preview.deadline_day,
            sponsor: c.government_number,
            fee: preview.fee,
            accepted_day: None,
            resolved_day: None,
            status: UndertakingStatus::Pending,
            milestones: Vec::new(),
        };
        record.mark(self.day.units(), CommitmentStage::Proposed);
        state.records.push(record);
        self.event(id,true,format!("Undertaking #{} submitted: {}. Paid {}m administration; review {}, delivery deadline {}. This does not order the policy or reserve its implementation resources. Withdrawal/refusal retains paid administration and the undertaking interval.",request.0,preview.goal.description(content),preview.fee.0,date(preview.review_day),date(preview.deadline_day)));
        Ok(())
    }
    pub(crate) fn withdraw_undertaking(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: UndertakingId,
    ) -> Result<(), String> {
        let record = self
            .countries
            .get(id)
            .and_then(|c| c.undertakings.as_ref())
            .and_then(|s| s.records.iter().find(|r| r.id == request))
            .ok_or("Unknown owned undertaking")?
            .clone();
        if !record.status.open() {
            return Err("Only pending or active undertakings can be withdrawn".into());
        }
        if record.accepted_day.is_some() {
            let t = content
                .undertakings
                .as_ref()
                .and_then(|r| r.template(&record.template))
                .expect("template");
            self.undertaking_effect(id, &t.breach);
        }
        let state = self
            .countries
            .get_mut(id)
            .expect("country")
            .undertakings
            .as_mut()
            .expect("state");
        let r = state
            .records
            .iter_mut()
            .find(|r| r.id == request)
            .expect("record");
        r.status = UndertakingStatus::Withdrawn;
        r.resolved_day = Some(self.day.units());
        r.mark(self.day.units(), CommitmentStage::Withdrawn);
        state.refresh_credibility();
        self.event(id,true,format!("Undertaking #{} withdrawn. Paid administration and the interval remain. {}",request.0,if record.accepted_day.is_some(){"An accepted promise was broken: the published political costs and credibility loss apply."}else{"The proposed undertaking had not been accepted; no breach effect applies."}));
        Ok(())
    }
    pub(crate) fn undertaking_succession(&mut self, id: &CountryId) {
        let c = self.countries.get_mut(id).expect("country");
        let Some(state) = &mut c.undertakings else {
            return;
        };
        let mut changed = false;
        for r in &mut state.records {
            if r.status.open() && r.sponsor != c.government_number {
                r.status = UndertakingStatus::Superseded;
                r.resolved_day = Some(self.day.units());
                r.mark(self.day.units(), CommitmentStage::Withdrawn);
                changed = true;
            }
        }
        if changed {
            state.refresh_credibility();
            self.event(id,true,"Successor cabinet closed the previous cabinet's pending and active undertakings. Existing policy projects continue; paid administration and declaration intervals remain. Accepted unfulfilled undertakings reduce the country-level credibility record.");
        }
    }
    pub(crate) fn resolve_undertaking_reviews(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.undertakings else {
            return;
        };
        let due: Vec<_> = self.countries[id]
            .undertakings
            .as_ref()
            .expect("state")
            .records
            .iter()
            .filter(|r| r.status == UndertakingStatus::Pending && r.review_day <= self.day.units())
            .cloned()
            .collect();
        for record in due {
            let t = rules.template(&record.template).expect("template");
            let decision = self.undertaking_backing(id, t);
            let state = self
                .countries
                .get_mut(id)
                .expect("country")
                .undertakings
                .as_mut()
                .expect("state");
            let r = state
                .records
                .iter_mut()
                .find(|r| r.id == record.id)
                .expect("record");
            match decision {
                Ok(()) => {
                    r.accepted_day = Some(self.day.units());
                    r.status = UndertakingStatus::Active;
                    r.mark(self.day.units(), CommitmentStage::Approved);
                    r.mark(self.day.units(), CommitmentStage::Operating);
                    self.undertaking_effect(id, &t.acceptance);
                    self.event(id,true,format!("Undertaking #{} accepted: {} by {}. Political acceptance effects now apply; the promised policy still requires its own consent, cash and delivery. Missing or withdrawing the accepted target incurs the stated breach effects.",record.id.0,record.goal.description(content),date(record.deadline_day)));
                }
                Err(reason) => {
                    r.status = UndertakingStatus::Refused {
                        reason: reason.clone(),
                    };
                    r.resolved_day = Some(self.day.units());
                    r.mark(self.day.units(), CommitmentStage::Refused);
                    self.event(id,true,format!("Undertaking #{} refused: {reason}. Administration and declaration interval remain.",record.id.0));
                }
            }
        }
    }
    pub(crate) fn evaluate_undertakings(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.undertakings else {
            return;
        };
        let active: Vec<_> = self.countries[id]
            .undertakings
            .as_ref()
            .expect("state")
            .records
            .iter()
            .filter(|r| r.status == UndertakingStatus::Active)
            .cloned()
            .collect();
        for record in active {
            let evidence = self.undertaking_evidence(id, &record, self.day.units());
            if evidence.is_none() && self.day.units() < record.deadline_day {
                continue;
            }
            let status = if let Some((evidence, _)) = evidence {
                UndertakingStatus::Fulfilled { evidence }
            } else {
                UndertakingStatus::Breached
            };
            if status == UndertakingStatus::Breached {
                self.undertaking_effect(
                    id,
                    &rules.template(&record.template).expect("template").breach,
                );
            }
            let state = self
                .countries
                .get_mut(id)
                .expect("country")
                .undertakings
                .as_mut()
                .expect("state");
            let r = state
                .records
                .iter_mut()
                .find(|r| r.id == record.id)
                .expect("record");
            r.status = status;
            r.resolved_day = Some(self.day.units());
            r.mark(self.day.units(), CommitmentStage::Evaluated);
            state.refresh_credibility();
            if let Some((evidence, _)) = evidence {
                self.event(id,true,format!("Undertaking #{} fulfilled by actual policy commitment #{}. This records a delivered setting/capacity target, not a claim that household outcomes were caused by the promise. Credibility updated.",record.id.0,evidence.0));
            } else {
                self.event(id,true,format!("Undertaking #{} breached: no qualifying policy delivery met {} by {}. The published political costs apply and credibility falls; pending policy projects continue under their own terms.",record.id.0,record.goal.description(content),date(record.deadline_day)));
            }
        }
    }
    pub(crate) fn ai_undertaking_turn(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.undertakings else {
            return;
        };
        if !self.day.units().is_multiple_of(90) {
            return;
        }
        for template in &rules.templates {
            let project = if template.kind == UndertakingKind::ProjectDeadline {
                self.countries[id]
                    .projects
                    .iter()
                    .find(|p| policy_field(&p.policy).is_some())
                    .map(|p| p.id)
            } else {
                None
            };
            if let Ok(preview) = self.preview_undertaking(content, id, &template.id, project)
                && preview
                    .expected_delivery
                    .is_some_and(|due| due <= preview.deadline_day)
            {
                let command = self.command(
                    id.clone(),
                    Action::DeclareUndertaking {
                        template: template.id.clone(),
                        project,
                    },
                );
                if let Err(reason) =
                    self.apply(content, command, crate::developer::CommandOrigin::Ai)
                {
                    self.event(
                        id,
                        false,
                        format!("AI undertaking declined by shared rules: {reason}"),
                    );
                }
                break;
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedUndertaking {
    pub id: UndertakingId,
    pub kind: UndertakingKind,
    pub goal: PolicyDeliveryGoal,
    pub accepted_day: u32,
    pub deadline_day: u32,
    pub status: UndertakingStatus,
    pub resolved_day: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicUndertakingReport {
    pub credibility: i32,
    pub records: Vec<PublishedUndertaking>,
}
impl World {
    pub(crate) fn published_undertakings(&self, id: &CountryId) -> Option<PublicUndertakingReport> {
        let state = self.countries.get(id)?.undertakings.as_ref()?;
        Some(PublicUndertakingReport {
            credibility: state.credibility,
            records: state
                .records
                .iter()
                .filter_map(|r| {
                    Some(PublishedUndertaking {
                        id: r.id,
                        kind: r.kind,
                        goal: r.goal.clone(),
                        accepted_day: r.accepted_day?,
                        deadline_day: r.deadline_day,
                        status: r.status.clone(),
                        resolved_day: r.resolved_day,
                    })
                })
                .collect(),
        })
    }
    pub(crate) fn policy_level_at(
        &self,
        content: &Content,
        id: &CountryId,
        policy: &PolicyId,
        day: u32,
        before_command: CommandId,
    ) -> Result<i32, String> {
        let d = content.country(id).ok_or("Unknown country")?;
        let mut level = match policy.as_str() {
            "income_tax" => d.tax.units() as i32,
            "services" => d.services.units() as i32,
            "welfare" => d.welfare.units() as i32,
            "energy" => d.energy.units() as i32,
            "agriculture" => d.food.units() as i32,
            "administration" => d.capacity.units() as i32,
            _ => return Err("Unknown undertaking policy".into()),
        };
        if self.scenario == Scenario::FiscalSupplyCrisis && id == &self.player {
            level = match policy.as_str() {
                "income_tax" => 10,
                "services" => 18,
                "welfare" => 12,
                "energy" | "agriculture" => 40,
                "administration" => 60,
                _ => unreachable!(),
            };
        }
        let field = policy_field(policy).expect("policy");
        let mut changes = Vec::new();
        for r in self.countries[id]
            .commitments
            .as_ref()
            .ok_or("Missing policy history")?
        {
            if let Some(delivered) = r
                .milestones
                .iter()
                .find(|m| m.stage == CommitmentStage::Operating)
                && delivered.day <= day
                && let Some(effect) = r.direct_effects.iter().find(|e| e.field == field)
            {
                changes.push((delivered.day, 0_u8, r.id.0, effect.after));
            }
        }
        for entry in &self.journal {
            if entry.command.day > day
                || entry.command.day == day && entry.command.id >= before_command
            {
                continue;
            }
            if let Action::Developer { target, edit } = &entry.command.action
                && target == id
                && (edit.field.name() == field
                    || policy == "income_tax" && edit.field == DeveloperField::Tax)
            {
                changes.push((entry.command.day, 1_u8, entry.command.id.0, edit.value));
            }
        }
        changes.sort_by_key(|(day, phase, id, _)| (*day, *phase, *id));
        for (_, _, _, after) in changes {
            level = after;
        }
        Ok(level)
    }
    fn undertaking_project_value_at(
        &self,
        id: &CountryId,
        record: &CommitmentRecord,
        before_command: CommandId,
    ) -> Result<i32, String> {
        let mut value = None;
        for entry in self
            .journal
            .iter()
            .filter(|e| &e.command.issuer == id && e.command.id < before_command)
        {
            match &entry.command.action {
                Action::Enact { value: v, .. } if entry.command.id == record.proposal_command => {
                    value = Some(*v)
                }
                Action::AmendCommitment { request, value: v } if *request == record.id => {
                    value = Some(*v)
                }
                _ => {}
            }
        }
        value.ok_or("Missing undertaking project origin".into())
    }
    pub(crate) fn validate_undertakings(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[id];
        let (state, rules) = match (&c.undertakings, &content.undertakings) {
            (None, None) => return Ok(()),
            (Some(s), Some(r)) => (s, r),
            _ => return Err("Undertaking history differs from campaign definitions".into()),
        };
        let origins: Vec<_> = self
            .journal
            .iter()
            .filter(|e| {
                &e.command.issuer == id
                    && matches!(e.command.action, Action::DeclareUndertaking { .. })
            })
            .collect();
        if origins.len() != state.records.len()
            || state.credibility != state.computed_credibility()
            || state.next_declaration.len() != 3
            || state.records.iter().filter(|r| r.status.open()).count() > rules.maximum_open
        {
            return Err("Invalid undertaking coverage, credibility or open limit".into());
        }
        for kind in [
            UndertakingKind::PublicPledge,
            UndertakingKind::SupportAgreement,
            UndertakingKind::ProjectDeadline,
        ] {
            let records: Vec<_> = state.records.iter().filter(|r| r.kind == kind).collect();
            let expected = records.last().map_or(Ok(content.start_day()?), |r| {
                r.declared_day
                    .checked_add(rules.cooldown_days.units())
                    .ok_or("Undertaking interval overflow")
            })?;
            if state.next_declaration.get(&kind) != Some(&expected)
                || records.iter().filter(|r| r.status.open()).count() > 1
            {
                return Err("Invalid undertaking interval or concurrent kind".into());
            }
            for pair in records.windows(2) {
                if pair[0]
                    .declared_day
                    .checked_add(rules.cooldown_days.units())
                    .is_none_or(|d| pair[1].declared_day < d)
                    || pair[0]
                        .resolved_day
                        .is_none_or(|d| d > pair[1].declared_day)
                {
                    return Err("Overlapping undertaking intervals or commitments".into());
                }
            }
        }
        for (r, origin) in state.records.iter().zip(origins) {
            let command = &origin.command;
            let t = rules
                .template(&r.template)
                .ok_or("Unknown undertaking template in history")?;
            if !matches!(&command.action,Action::DeclareUndertaking{template,project} if template==&r.template&&project==&r.linked_project)
                || r.id.0 != command.id.0
                || r.kind != t.kind
                || r.declared_day != command.day
                || command.day.checked_add(t.review_days.units()) != Some(r.review_day)
                || r.review_day.checked_add(t.deadline_days.units()) != Some(r.deadline_day)
                || r.deadline_day > MAX_DAY
                || r.sponsor == crate::GovernmentId(0)
                || r.sponsor > c.government_number
                || r.fee
                    != Money(
                        (content.country(id).expect("country").gdp.units()
                            * t.administration_cost_bps.units_i64()
                            / 10000)
                            .max(1),
                    )
                || !target_in_range(content, &r.goal.policy, r.goal.target)
                || r.baseline
                    != self.policy_level_at(content, id, &r.goal.policy, command.day, command.id)?
            {
                return Err(
                    "Invalid undertaking origin, dates, goal baseline, fee or sponsorship".into(),
                );
            }
            let expected_goal = if t.kind == UndertakingKind::ProjectDeadline {
                let request = r
                    .linked_project
                    .ok_or("Missing attached policy commitment")?;
                let project = c
                    .commitments
                    .as_ref()
                    .and_then(|rs| rs.iter().find(|p| p.id == request))
                    .ok_or("Unknown attached policy commitment")?;
                if project.proposal_command>=command.id||project.project.policy!=r.goal.policy||self.journal.iter().any(|entry|entry.command.issuer==*id&&entry.command.id<command.id&&matches!(entry.command.action,Action::CancelCommitment{request} if request==project.id))||project.milestones.iter().any(|m|matches!(m.stage,CommitmentStage::Operating|CommitmentStage::Refused)&&m.day<=command.day){return Err("Undertaking attached a nonpending or unrelated project".into());}
                let target = match r.goal.policy.as_str() {
                    "energy" | "agriculture" => (r.baseline + 15).min(200),
                    "administration" => (r.baseline + 8).min(100),
                    _ => self.undertaking_project_value_at(id, project, command.id)?,
                };
                PolicyDeliveryGoal {
                    policy: project.project.policy.clone(),
                    direction: if target > r.baseline {
                        GoalDirection::AtLeast
                    } else {
                        GoalDirection::AtMost
                    },
                    target,
                }
            } else {
                if r.linked_project.is_some() {
                    return Err("Unrelated undertaking has an attached project".into());
                }
                PolicyDeliveryGoal {
                    policy: t.policy.clone().expect("template"),
                    direction: t.direction,
                    target: r
                        .baseline
                        .checked_add(t.delta)
                        .ok_or("Undertaking target overflow")?,
                }
            };
            if r.goal != expected_goal || r.goal.direction.met(r.baseline, r.goal.target) {
                return Err(
                    "Undertaking target differs from its authored declaration or is already met"
                        .into(),
                );
            }
            let withdrawn:Vec<_>=self.journal.iter().filter(|e|&e.command.issuer==id&&matches!(e.command.action,Action::WithdrawUndertaking{request} if request==r.id)).collect();
            if withdrawn.len() != usize::from(r.status == UndertakingStatus::Withdrawn)
                || withdrawn
                    .first()
                    .is_some_and(|e| Some(e.command.day) != r.resolved_day)
            {
                return Err("Undertaking withdrawal differs from commands".into());
            }
            let accepted = r.accepted_day == Some(r.review_day) && r.review_day <= self.day.units();
            let final_day = r
                .resolved_day
                .is_some_and(|d| d >= r.declared_day && d <= self.day.units());
            // Constitutional succession runs before today's policy deliveries.
            // A funded project can deliver later on the same calendar day after
            // the former cabinet's undertaking has already been closed.
            let superseded_before_delivery = c.constitution.as_ref().is_some_and(|state| {
                state.history.iter().any(|change| {
                    Some(change.day) == r.resolved_day
                        && r.sponsor.checked_add(1) == Some(change.government_number)
                })
            });
            let evidence = self.undertaking_evidence(id, r, self.day.units());
            let valid = match &r.status {
                UndertakingStatus::Pending => {
                    r.accepted_day.is_none()
                        && r.resolved_day.is_none()
                        && r.review_day > self.day.units()
                        && r.sponsor == c.government_number
                }
                UndertakingStatus::Active => {
                    accepted
                        && r.resolved_day.is_none()
                        && r.deadline_day > self.day.units()
                        && r.sponsor == c.government_number
                        && evidence.is_none()
                }
                UndertakingStatus::Fulfilled { evidence: source } => {
                    accepted
                        && evidence.is_some_and(|(p, day)| {
                            p == *source && r.resolved_day == Some(day.max(r.review_day))
                        })
                }
                UndertakingStatus::Breached => {
                    accepted
                        && r.resolved_day == Some(r.deadline_day)
                        && r.deadline_day <= self.day.units()
                        && evidence.is_none()
                }
                UndertakingStatus::Refused { reason } => {
                    !reason.is_empty()
                        && r.accepted_day.is_none()
                        && r.resolved_day == Some(r.review_day)
                        && r.review_day <= self.day.units()
                }
                UndertakingStatus::Withdrawn => {
                    final_day
                        && if accepted {
                            r.resolved_day
                                .is_some_and(|d| d >= r.review_day && d < r.deadline_day)
                                && self
                                    .undertaking_evidence(id, r, r.resolved_day.expect("day"))
                                    .is_none()
                        } else {
                            r.accepted_day.is_none()
                                && r.resolved_day.is_some_and(|d| d < r.review_day)
                        }
                }
                UndertakingStatus::Superseded => {
                    final_day
                        && r.sponsor < c.government_number
                        && if accepted {
                            r.resolved_day.is_some_and(|d| {
                                (if superseded_before_delivery {
                                    d > r.review_day && d <= r.deadline_day
                                } else {
                                    d >= r.review_day && d < r.deadline_day
                                }) && self
                                    .undertaking_evidence(
                                        id,
                                        r,
                                        d.saturating_sub(u32::from(superseded_before_delivery)),
                                    )
                                    .is_none()
                            })
                        } else {
                            r.accepted_day.is_none()
                                && r.resolved_day.is_some_and(|d| {
                                    d < r.review_day
                                        || superseded_before_delivery && d == r.review_day
                                })
                        }
                }
            };
            if !valid {
                return Err(
                    "Invalid undertaking acceptance, outcome, evidence or resolution date".into(),
                );
            }
            let mut milestones = vec![CommitmentMilestone {
                day: r.declared_day,
                stage: CommitmentStage::Proposed,
                value: r.goal.target,
            }];
            if accepted {
                for stage in [CommitmentStage::Approved, CommitmentStage::Operating] {
                    milestones.push(CommitmentMilestone {
                        day: r.review_day,
                        stage,
                        value: r.goal.target,
                    });
                }
            }
            if let Some(day) = r.resolved_day {
                let stage = match r.status {
                    UndertakingStatus::Refused { .. } => CommitmentStage::Refused,
                    UndertakingStatus::Withdrawn | UndertakingStatus::Superseded => {
                        CommitmentStage::Withdrawn
                    }
                    _ => CommitmentStage::Evaluated,
                };
                milestones.push(CommitmentMilestone {
                    day,
                    stage,
                    value: r.goal.target,
                });
            }
            if r.milestones != milestones {
                return Err(
                    "Undertaking lifecycle differs from shared commitment milestones".into(),
                );
            }
        }
        Ok(())
    }
}
