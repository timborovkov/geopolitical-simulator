//! Bilateral agreement records. Signatures and expected activation are not operation.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct TreatyId(pub u64);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct TreatyRevision(pub u32);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct TreatyMilestoneId(pub u32);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TreatyAction {
    Offer {
        with: CountryId,
        terms: NonAggressionTerms,
    },
    Counteroffer {
        agreement: TreatyId,
        revision: TreatyRevision,
        terms: NonAggressionTerms,
    },
    /// Agreement identities cannot be replaced by command-envelope identities.
    /// ```compile_fail
    /// use sim_core::{CommandId, TreatyAction, TreatyRevision};
    /// let wrong = TreatyAction::Sign { agreement: CommandId(1), revision: TreatyRevision(1) };
    /// ```
    Sign {
        agreement: TreatyId,
        revision: TreatyRevision,
    },
    Ratify {
        agreement: TreatyId,
        revision: TreatyRevision,
    },
    Reject {
        agreement: TreatyId,
        revision: TreatyRevision,
    },
    Withdraw {
        agreement: TreatyId,
        revision: TreatyRevision,
    },
    Terminate {
        agreement: TreatyId,
        revision: TreatyRevision,
    },
}
impl TreatyAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Offer { .. } => "Submit non-aggression offer",
            Self::Counteroffer { .. } => "Submit revised terms",
            Self::Sign { .. } => "Sign current terms",
            Self::Ratify { .. } => "Submit domestic ratification",
            Self::Reject { .. } => "Reject current offer",
            Self::Withdraw { .. } => "Withdraw before activation",
            Self::Terminate { .. } => "Give termination notice",
        }
    }
    pub(crate) fn target(&self) -> Option<(TreatyId, TreatyRevision)> {
        match self {
            Self::Offer { .. } => None,
            Self::Counteroffer {
                agreement,
                revision,
                ..
            }
            | Self::Sign {
                agreement,
                revision,
            }
            | Self::Ratify {
                agreement,
                revision,
            }
            | Self::Reject {
                agreement,
                revision,
            }
            | Self::Withdraw {
                agreement,
                revision,
            }
            | Self::Terminate {
                agreement,
                revision,
            } => Some((*agreement, *revision)),
        }
    }
    pub(crate) fn paid_service(&self) -> bool {
        matches!(
            self,
            Self::Offer { .. }
                | Self::Counteroffer { .. }
                | Self::Sign { .. }
                | Self::Ratify { .. }
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NonAggressionTerms {
    /// A term is elapsed time, never an absolute campaign date.
    /// ```compile_fail
    /// use sim_core::{CampaignDay, DaySpan, NonAggressionTerms};
    /// let wrong = NonAggressionTerms { duration: CampaignDay::new(180), notice: DaySpan::new(30) };
    /// ```
    pub duration: DaySpan,
    pub notice: DaySpan,
}
impl NonAggressionTerms {
    pub fn validate(&self, definition: &DiplomacyDefinition) -> Result<(), String> {
        if self.duration < definition.minimum_duration
            || self.duration > definition.maximum_duration
            || self.notice > self.duration
        {
            return Err(
                "Choose a supported pact duration and notice no longer than its duration".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TreatyStance {
    Consider,
    Decline,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreatyAiPolicy {
    pub stance: TreatyStance,
    pub maximum_duration: DaySpan,
    pub minimum_notice: DaySpan,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiplomacyDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub minimum_duration: DaySpan,
    pub maximum_duration: DaySpan,
    pub response_days: DaySpan,
    pub pair_cooldown: DaySpan,
    pub ai_cadence: DaySpan,
    pub administration_bps: BasisPoints,
    pub maximum_records: u32,
    pub maximum_revisions: u32,
    pub ai: BTreeMap<CountryId, TreatyAiPolicy>,
}
impl DiplomacyDefinition {
    pub(crate) fn validate(&self, content: &Content) -> Result<(), String> {
        if self.source_version.is_empty()
            || self.description.is_empty()
            || !content
                .sources
                .iter()
                .any(|source| source.id == self.source)
            || !(30..=3650).contains(&self.minimum_duration.units())
            || self.maximum_duration < self.minimum_duration
            || self.maximum_duration.units() > 3650
            || !(1..=90).contains(&self.response_days.units())
            || !(1..=365).contains(&self.pair_cooldown.units())
            || !(1..=30).contains(&self.ai_cadence.units())
            || !(1..=100).contains(&self.administration_bps.units())
            || !(1..=4096).contains(&self.maximum_records)
            || !(1..=16).contains(&self.maximum_revisions)
            || self.ai.len() != content.countries.len()
        {
            return Err(
                "Diplomacy needs bounded, attributed terms, fees and country decision policies"
                    .into(),
            );
        }
        for country in &content.countries {
            let policy = self
                .ai
                .get(&country.id)
                .ok_or("Missing treaty decision policy")?;
            if policy.maximum_duration < self.minimum_duration
                || policy.maximum_duration > self.maximum_duration
                || policy.minimum_notice > policy.maximum_duration
            {
                return Err("Invalid treaty decision-policy term limits".into());
            }
        }
        Ok(())
    }

    pub fn administration(&self, country: &CountryDefinition) -> Result<Money, String> {
        let fee = i128::from(country.gdp.units()) * i128::from(self.administration_bps.units())
            / i128::from(BasisPoints::DENOMINATOR);
        i64::try_from(fee.max(1))
            .map(Money)
            .map_err(|_| "Treaty administration exceeds the accounting range".into())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TreatyStage {
    Negotiating,
    AwaitingRatification,
    Scheduled,
    Active,
    Notice,
    Expired,
    Terminated,
    Refused,
    Withdrawn,
    TimedOut,
    RatificationFailed,
    SponsorshipLapsed,
}
impl TreatyStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Negotiating => "Offer awaiting a response",
            Self::AwaitingRatification => "Signed; domestic ratification required",
            Self::Scheduled => "Ratified; activation scheduled",
            Self::Active => "Active",
            Self::Notice => "Active during termination notice",
            Self::Expired => "Expired",
            Self::Terminated => "Terminated after notice",
            Self::Refused => "Refused",
            Self::Withdrawn => "Withdrawn before activation",
            Self::TimedOut => "Response or submission window expired",
            Self::RatificationFailed => "Domestic ratification failed",
            Self::SponsorshipLapsed => "Pending government sponsorship lapsed",
        }
    }
    pub fn closed(self) -> bool {
        !matches!(
            self,
            Self::Negotiating
                | Self::AwaitingRatification
                | Self::Scheduled
                | Self::Active
                | Self::Notice
        )
    }
    pub fn operative(self) -> bool {
        matches!(self, Self::Active | Self::Notice)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TreatyOutcome {
    Offered,
    Counteroffered,
    Signed,
    RatificationSubmitted,
    Ratified,
    ActivationScheduled,
    Activated,
    NoticeGiven,
    Expired,
    Terminated,
    Refused,
    Withdrawn,
    TimedOut,
    RatificationFailed,
    SponsorshipLapsed,
}
impl TreatyOutcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Offered => "Non-aggression offer submitted",
            Self::Counteroffered => "Revised terms offered; earlier signatures invalidated",
            Self::Signed => "Both parties signed; domestic ratification remains",
            Self::RatificationSubmitted => "Domestic ratification submitted for review",
            Self::Ratified => "One government completed domestic ratification",
            Self::ActivationScheduled => "Both ratifications passed; activation scheduled",
            Self::Activated => "Non-aggression pact activated",
            Self::NoticeGiven => "Termination notice given; pact remains active during notice",
            Self::Expired => "Non-aggression pact expired",
            Self::Terminated => "Non-aggression pact terminated after notice",
            Self::Refused => "Non-aggression offer refused",
            Self::Withdrawn => "Proposal withdrawn before activation",
            Self::TimedOut => "Offer or ratification-submission window expired",
            Self::RatificationFailed => "Domestic ratification did not pass",
            Self::SponsorshipLapsed => "Pending proposal lapsed after government succession",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreatySignature {
    pub government: GovernmentId,
    pub day: CampaignDay,
    pub command: CommandId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreatyRatification {
    pub government: GovernmentId,
    pub submitted: CampaignDay,
    pub review: CampaignDay,
    pub passed: Option<CampaignDay>,
    pub command: CommandId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreatyNotice {
    pub by: CountryId,
    pub day: CampaignDay,
    pub until: CampaignDay,
    pub command: CommandId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreatyMilestone {
    pub id: TreatyMilestoneId,
    pub day: CampaignDay,
    pub revision: TreatyRevision,
    pub outcome: TreatyOutcome,
    pub actor: Option<CountryId>,
    pub command: Option<CommandId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreatyRecord {
    pub id: TreatyId,
    pub parties: [CountryId; 2],
    pub revision: TreatyRevision,
    pub offered_by: CountryId,
    pub terms: NonAggressionTerms,
    pub created: CampaignDay,
    pub offered_on: CampaignDay,
    pub offer_deadline: CampaignDay,
    pub signed_on: Option<CampaignDay>,
    pub activation_day: Option<CampaignDay>,
    pub activated_on: Option<CampaignDay>,
    pub expires_at: Option<CampaignDay>,
    pub closed_on: Option<CampaignDay>,
    pub stage: TreatyStage,
    pub proposal_command: CommandId,
    pub signatures: BTreeMap<CountryId, TreatySignature>,
    pub ratifications: BTreeMap<CountryId, TreatyRatification>,
    pub administration_paid: BTreeMap<CountryId, Money>,
    pub notice: Option<TreatyNotice>,
    pub next_milestone: TreatyMilestoneId,
    pub milestones: Vec<TreatyMilestone>,
}
impl TreatyRecord {
    pub fn participant(&self, country: &CountryId) -> bool {
        self.parties.contains(country)
    }

    pub(crate) fn milestone(
        &mut self,
        day: CampaignDay,
        outcome: TreatyOutcome,
        actor: Option<CountryId>,
        command: Option<CommandId>,
    ) -> Result<TreatyMilestoneId, String> {
        if self.milestones.len() >= 64 {
            return Err("Treaty milestone capacity exhausted".into());
        }
        let id = self.next_milestone;
        if id.0 == 0 {
            return Err("Treaty milestone identities start at one".into());
        }
        let next =
            id.0.checked_add(1)
                .ok_or("Treaty milestone identity exhausted")?;
        self.milestones.push(TreatyMilestone {
            id,
            day,
            revision: self.revision,
            outcome,
            actor,
            command,
        });
        self.next_milestone = TreatyMilestoneId(next);
        Ok(id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiplomacyState {
    pub next_id: TreatyId,
    pub agreements: BTreeMap<TreatyId, TreatyRecord>,
}
impl Default for DiplomacyState {
    fn default() -> Self {
        Self {
            next_id: TreatyId(1),
            agreements: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreatyPreview {
    pub parties: [CountryId; 2],
    pub agreement: TreatyId,
    pub revision: TreatyRevision,
    pub terms: NonAggressionTerms,
    pub immediate_cost: Money,
    pub authority: String,
    pub stage_after_command: TreatyStage,
    pub ratification_review: Option<CampaignDay>,
    pub explanation: String,
}

struct TreatyPlan {
    record: TreatyRecord,
    next_id: Option<TreatyId>,
    fee: Money,
    milestone: TreatyMilestoneId,
}

pub(crate) fn treaty_date(day: CampaignDay, elapsed: DaySpan) -> Result<CampaignDay, String> {
    day.checked_add(elapsed)
        .filter(|date| date.units() <= MAX_DAY)
        .ok_or_else(|| "Agreement date exceeds the campaign calendar".into())
}

impl World {
    fn treaty_definition<'a>(
        &self,
        content: &'a Content,
    ) -> Result<&'a DiplomacyDefinition, String> {
        if self.content_hash != content.hash() {
            return Err("Treaty decisions require the campaign's exact content".into());
        }
        content
            .diplomacy
            .as_ref()
            .ok_or_else(|| "This campaign has no bilateral agreement rules".into())
    }

    fn check_treaty_sponsorship(&self, record: &TreatyRecord) -> Result<(), String> {
        if record.signatures.iter().any(|(party, signature)| {
            self.countries
                .get(party)
                .is_none_or(|country| country.government_number != signature.government)
        }) {
            return Err("Pending government sponsorship changed; the next daily review will lapse this proposal".into());
        }
        Ok(())
    }

    fn check_treaty_calendar(
        &self,
        content: &Content,
        parties: &[CountryId; 2],
        terms: &NonAggressionTerms,
    ) -> Result<(), String> {
        let rules = self.treaty_definition(content)?;
        let consent = parties.iter().try_fold(1_u32, |maximum, party| {
            self.governing_rules(content, party)
                .map(|government| maximum.max(government.consent_days.units()))
        })?;
        let remaining = rules
            .response_days
            .checked_add(rules.response_days)
            .and_then(|days| days.checked_add(DaySpan::new(consent)))
            .and_then(|days| days.checked_add(DaySpan::new(1)))
            .and_then(|days| days.checked_add(terms.duration))
            .ok_or("Agreement timetable overflow")?;
        treaty_date(self.day, remaining)?;
        Ok(())
    }

    fn plan_treaty_action(
        &self,
        content: &Content,
        actor: &CountryId,
        command: CommandId,
        action: &TreatyAction,
    ) -> Result<TreatyPlan, String> {
        let rules = self.treaty_definition(content)?;
        let state = self
            .diplomacy
            .as_ref()
            .ok_or("Missing bilateral agreement state")?;
        let country = self
            .countries
            .get(actor)
            .ok_or("Unknown treaty government")?;
        let definition = content
            .country(actor)
            .ok_or("Missing treaty government definition")?;
        let government = self.governing_rules(content, actor)?;
        let fee = if action.paid_service() {
            government.check(country, "treaty", 0)?;
            let fee = rules.administration(definition)?;
            if country
                .budget
                .capital
                .0
                .checked_add(fee.0)
                .is_none_or(|total| total > 1_000_000_000_000)
            {
                return Err(
                    "Treaty administration exceeds the supported annual spending range".into(),
                );
            }
            if country.available_cash().0 < fee.0 {
                return Err(format!(
                    "Treaty administration requires {}m of available national cash",
                    fee.0
                ));
            }
            fee
        } else {
            Money(0)
        };
        let mut next_id = None;
        let mut record = if let TreatyAction::Offer { with, terms } = action {
            if with == actor || !self.countries.contains_key(with) {
                return Err("Choose another existing country for a bilateral pact".into());
            }
            terms.validate(rules)?;
            let mut parties = [actor.clone(), with.clone()];
            parties.sort();
            self.check_treaty_calendar(content, &parties, terms)?;
            if state.agreements.len() >= rules.maximum_records as usize {
                return Err("Agreement archive capacity exhausted".into());
            }
            for prior in state
                .agreements
                .values()
                .filter(|prior| prior.parties == parties)
            {
                if !prior.stage.closed() {
                    return Err("This pair already has a live pact or negotiation".into());
                }
                let closed = prior.closed_on.ok_or("Closed pact has no closure date")?;
                if self.day < treaty_date(closed, rules.pair_cooldown)? {
                    return Err("This pair's agreement cooldown has not elapsed".into());
                }
            }
            let id = state.next_id;
            if id.0 == 0 || state.agreements.contains_key(&id) {
                return Err("Invalid agreement identity allocator".into());
            }
            next_id = Some(TreatyId(
                id.0.checked_add(1)
                    .ok_or("Agreement identities exhausted")?,
            ));
            TreatyRecord {
                id,
                parties,
                revision: TreatyRevision(1),
                offered_by: actor.clone(),
                terms: terms.clone(),
                created: self.day,
                offered_on: self.day,
                offer_deadline: treaty_date(self.day, rules.response_days)?,
                signed_on: None,
                activation_day: None,
                activated_on: None,
                expires_at: None,
                closed_on: None,
                stage: TreatyStage::Negotiating,
                proposal_command: command,
                signatures: BTreeMap::from([(
                    actor.clone(),
                    TreatySignature {
                        government: country.government_number,
                        day: self.day,
                        command,
                    },
                )]),
                ratifications: BTreeMap::new(),
                administration_paid: BTreeMap::new(),
                notice: None,
                next_milestone: TreatyMilestoneId(1),
                milestones: Vec::new(),
            }
        } else {
            let (id, revision) = action.target().ok_or("Missing agreement identity")?;
            let record = state.agreements.get(&id).ok_or("Agreement unavailable")?;
            if !record.participant(actor) {
                return Err("Agreement unavailable".into());
            }
            if record.revision != revision {
                return Err("Agreement revision changed; inspect the current terms".into());
            }
            if record.stage.closed() {
                return Err("This agreement is already closed".into());
            }
            record.clone()
        };
        let outcome = match action {
            TreatyAction::Offer { .. } => TreatyOutcome::Offered,
            TreatyAction::Counteroffer { terms, .. } => {
                if record.stage != TreatyStage::Negotiating
                    || &record.offered_by == actor
                    || self.day >= record.offer_deadline
                {
                    return Err(
                        "Only the receiving party may counter a current unanswered offer".into(),
                    );
                }
                self.check_treaty_sponsorship(&record)?;
                terms.validate(rules)?;
                if terms == &record.terms {
                    return Err("A counteroffer must change the terms".into());
                }
                if record.revision.0 >= rules.maximum_revisions {
                    return Err("Agreement revision capacity exhausted".into());
                }
                self.check_treaty_calendar(content, &record.parties, terms)?;
                record.revision = TreatyRevision(
                    record
                        .revision
                        .0
                        .checked_add(1)
                        .ok_or("Agreement revisions exhausted")?,
                );
                record.terms = terms.clone();
                record.offered_by = actor.clone();
                record.offered_on = self.day;
                record.offer_deadline = treaty_date(self.day, rules.response_days)?;
                record.signatures.clear();
                record.signatures.insert(
                    actor.clone(),
                    TreatySignature {
                        government: country.government_number,
                        day: self.day,
                        command,
                    },
                );
                record.ratifications.clear();
                TreatyOutcome::Counteroffered
            }
            TreatyAction::Sign { .. } => {
                if record.stage != TreatyStage::Negotiating
                    || &record.offered_by == actor
                    || record.signatures.contains_key(actor)
                    || self.day >= record.offer_deadline
                {
                    return Err(
                        "Only the receiving party may sign a current unanswered offer".into(),
                    );
                }
                self.check_treaty_sponsorship(&record)?;
                record.signatures.insert(
                    actor.clone(),
                    TreatySignature {
                        government: country.government_number,
                        day: self.day,
                        command,
                    },
                );
                record.signed_on = Some(self.day);
                record.stage = TreatyStage::AwaitingRatification;
                TreatyOutcome::Signed
            }
            TreatyAction::Ratify { .. } => {
                if record.stage != TreatyStage::AwaitingRatification
                    || !record.signatures.contains_key(actor)
                    || record.ratifications.contains_key(actor)
                {
                    return Err(
                        "Both signatures and an unsubmitted domestic ratification are required"
                            .into(),
                    );
                }
                self.check_treaty_sponsorship(&record)?;
                let signed = record
                    .signed_on
                    .ok_or("Signed proposal has no signature date")?;
                if self.day >= treaty_date(signed, rules.response_days)? {
                    return Err("The domestic ratification submission window expired".into());
                }
                record.ratifications.insert(
                    actor.clone(),
                    TreatyRatification {
                        government: country.government_number,
                        submitted: self.day,
                        review: treaty_date(
                            self.day,
                            DaySpan::new(government.consent_days.units().max(1)),
                        )?,
                        passed: None,
                        command,
                    },
                );
                TreatyOutcome::RatificationSubmitted
            }
            TreatyAction::Reject { .. } => {
                if record.stage != TreatyStage::Negotiating
                    || &record.offered_by == actor
                    || self.day >= record.offer_deadline
                {
                    return Err(
                        "Only the receiving party may reject a current unanswered offer".into(),
                    );
                }
                record.stage = TreatyStage::Refused;
                record.closed_on = Some(self.day);
                TreatyOutcome::Refused
            }
            TreatyAction::Withdraw { .. } => {
                if !matches!(
                    record.stage,
                    TreatyStage::Negotiating
                        | TreatyStage::AwaitingRatification
                        | TreatyStage::Scheduled
                ) {
                    return Err(
                        "An operative pact requires termination notice rather than withdrawal"
                            .into(),
                    );
                }
                record.stage = TreatyStage::Withdrawn;
                record.closed_on = Some(self.day);
                TreatyOutcome::Withdrawn
            }
            TreatyAction::Terminate { .. } => {
                if record.stage != TreatyStage::Active || record.notice.is_some() {
                    return Err(
                        "Termination notice requires an active pact without an existing notice"
                            .into(),
                    );
                }
                let expires = record.expires_at.ok_or("Active pact has no expiry")?;
                let until = self
                    .day
                    .checked_add(record.terms.notice)
                    .ok_or("Termination notice date overflow")?
                    .min(expires);
                record.notice = Some(TreatyNotice {
                    by: actor.clone(),
                    day: self.day,
                    until,
                    command,
                });
                if until == self.day {
                    record.stage = TreatyStage::Terminated;
                    record.closed_on = Some(self.day);
                    TreatyOutcome::Terminated
                } else {
                    record.stage = TreatyStage::Notice;
                    TreatyOutcome::NoticeGiven
                }
            }
        };
        if fee.0 > 0 {
            let paid = record
                .administration_paid
                .entry(actor.clone())
                .or_insert(Money(0));
            paid.0 = paid
                .0
                .checked_add(fee.0)
                .ok_or("Agreement administration total overflow")?;
        }
        let milestone = record.milestone(self.day, outcome, Some(actor.clone()), Some(command))?;
        Ok(TreatyPlan {
            record,
            next_id,
            fee,
            milestone,
        })
    }

    pub fn preview_treaty(
        &self,
        content: &Content,
        action: &TreatyAction,
    ) -> Result<TreatyPreview, String> {
        self.validate_treaties(content)?;
        let plan = self.plan_treaty_action(content, &self.player, self.next_command, action)?;
        let explanation = match action {
            TreatyAction::Terminate { .. } => format!("No new administration charge. The pact remains operative until {} unless natural expiry ends it first. Paid earlier charges are retained.", date(plan.record.notice.as_ref().ok_or("Missing planned notice")?.until)),
            TreatyAction::Withdraw { .. } => "No new administration charge. This closes the pending proposal before activation; earlier charges and pair cooldown remain.".into(),
            TreatyAction::Reject { .. } => "No new administration charge. This refuses the unanswered offer; earlier charges and pair cooldown remain.".into(),
            TreatyAction::Ratify { .. } => "Administration is paid now for domestic review and is not refundable. Actual ratification rechecks backing and government sponsorship. Both domestic ratifications must pass before next-day activation is scheduled.".into(),
            _ => "Administration is paid now and is not refundable. This order records a signature to these terms, not an active pact. Both signatures and both separately paid domestic ratifications are required before scheduled activation.".into(),
        };
        Ok(TreatyPreview {
            parties: plan.record.parties,
            agreement: plan.record.id,
            revision: plan.record.revision,
            terms: plan.record.terms,
            immediate_cost: plan.fee,
            authority: self.governing_rules(content, &self.player)?.label(),
            stage_after_command: plan.record.stage,
            ratification_review: if matches!(action, TreatyAction::Ratify { .. }) {
                plan.record
                    .ratifications
                    .get(&self.player)
                    .map(|request| request.review)
            } else {
                None
            },
            explanation,
        })
    }

    fn emit_treaty_milestone(
        &mut self,
        agreement: TreatyId,
        milestone: TreatyMilestoneId,
    ) -> Result<(), String> {
        let record = self
            .diplomacy
            .as_ref()
            .and_then(|state| state.agreements.get(&agreement))
            .ok_or("Missing agreement outcome")?;
        let entry = record
            .milestones
            .iter()
            .find(|entry| entry.id == milestone)
            .ok_or("Missing treaty milestone")?;
        let parties = record.parties.clone();
        let text = format!(
            "Pact #{} · revision {} · {}. Participants: {} and {}.",
            agreement.0,
            entry.revision.0,
            entry.outcome.label(),
            parties[0].0,
            parties[1].0
        );
        for party in &parties {
            self.event_with_context(
                party,
                true,
                text.clone(),
                EventContext::Treaty {
                    agreement,
                    milestone,
                },
            );
        }
        Ok(())
    }

    pub(crate) fn apply_treaty_action(
        &mut self,
        content: &Content,
        actor: &CountryId,
        command: CommandId,
        action: &TreatyAction,
    ) -> Result<(), String> {
        let plan = self.plan_treaty_action(content, actor, command, action)?;
        if plan.fee.0 > 0 {
            let paid = self.pay(
                actor,
                Account::Treasury,
                Account::Firms,
                plan.fee.0,
                &format!(
                    "Treaty #{} revision {}: earned diplomatic administration",
                    plan.record.id.0, plan.record.revision.0
                ),
            );
            if paid != plan.fee.0 {
                return Err("Treaty administration could not be paid in full".into());
            }
            let budget = &mut self
                .countries
                .get_mut(actor)
                .ok_or("Unknown treaty payer")?
                .budget;
            budget.capital.0 = budget
                .capital
                .0
                .checked_add(plan.fee.0)
                .ok_or("Treaty budget expense overflow")?;
        }
        let id = plan.record.id;
        let state = self
            .diplomacy
            .as_mut()
            .ok_or("Missing bilateral agreement state")?;
        if let Some(next) = plan.next_id {
            state.next_id = next;
        }
        state.agreements.insert(id, plan.record);
        self.emit_treaty_milestone(id, plan.milestone)
    }

    pub(crate) fn lapse_treaty_sponsorship(&mut self) -> Result<(), String> {
        let ids: Vec<_> = self
            .diplomacy
            .as_ref()
            .map(|state| {
                state
                    .agreements
                    .values()
                    .filter(|record| {
                        !record.stage.closed()
                            && !record.stage.operative()
                            && self.check_treaty_sponsorship(record).is_err()
                    })
                    .map(|record| record.id)
                    .collect()
            })
            .unwrap_or_default();
        for id in ids {
            let record = self
                .diplomacy
                .as_mut()
                .and_then(|state| state.agreements.get_mut(&id))
                .ok_or("Missing pending treaty")?;
            record.stage = TreatyStage::SponsorshipLapsed;
            record.closed_on = Some(self.day);
            let milestone =
                record.milestone(self.day, TreatyOutcome::SponsorshipLapsed, None, None)?;
            self.emit_treaty_milestone(id, milestone)?;
        }
        Ok(())
    }

    pub(crate) fn review_treaties(&mut self, content: &Content) -> Result<(), String> {
        let Some(rules) = &content.diplomacy else {
            return Ok(());
        };
        self.lapse_treaty_sponsorship()?;
        let ids: Vec<_> = self
            .diplomacy
            .as_ref()
            .ok_or("Missing treaty state")?
            .agreements
            .keys()
            .copied()
            .collect();
        for id in ids {
            let mut record = self
                .diplomacy
                .as_ref()
                .and_then(|state| state.agreements.get(&id))
                .ok_or("Missing treaty record")?
                .clone();
            if record.stage.closed() {
                continue;
            }
            let mut outcomes = Vec::new();
            let timed_out = (record.stage == TreatyStage::Negotiating
                && self.day >= record.offer_deadline)
                || (record.stage == TreatyStage::AwaitingRatification
                    && self.day
                        >= treaty_date(
                            record.signed_on.ok_or("Missing signature date")?,
                            rules.response_days,
                        )?
                    && record
                        .parties
                        .iter()
                        .any(|party| !record.ratifications.contains_key(party)));
            if timed_out {
                record.stage = TreatyStage::TimedOut;
                record.closed_on = Some(self.day);
                outcomes.push(record.milestone(self.day, TreatyOutcome::TimedOut, None, None)?);
            } else if record.stage == TreatyStage::AwaitingRatification {
                for party in record.parties.clone() {
                    let Some(request) = record.ratifications.get(&party) else {
                        continue;
                    };
                    if request.passed.is_some() || request.review > self.day {
                        continue;
                    }
                    let country = self
                        .countries
                        .get(&party)
                        .ok_or("Unknown treaty ratifier")?;
                    if country.government_number != request.government
                        || self
                            .governing_rules(content, &party)?
                            .check(country, "treaty", 0)
                            .is_err()
                    {
                        record.stage = TreatyStage::RatificationFailed;
                        record.closed_on = Some(self.day);
                        outcomes.push(record.milestone(
                            self.day,
                            TreatyOutcome::RatificationFailed,
                            Some(party),
                            None,
                        )?);
                        break;
                    }
                    record
                        .ratifications
                        .get_mut(&party)
                        .ok_or("Missing ratification")?
                        .passed = Some(self.day);
                    outcomes.push(record.milestone(
                        self.day,
                        TreatyOutcome::Ratified,
                        Some(party),
                        None,
                    )?);
                }
                if !record.stage.closed()
                    && record.ratifications.len() == 2
                    && record
                        .ratifications
                        .values()
                        .all(|request| request.passed.is_some())
                {
                    let activation = treaty_date(self.day, DaySpan::new(1))?;
                    record.activation_day = Some(activation);
                    record.expires_at = Some(treaty_date(activation, record.terms.duration)?);
                    record.stage = TreatyStage::Scheduled;
                    outcomes.push(record.milestone(
                        self.day,
                        TreatyOutcome::ActivationScheduled,
                        None,
                        None,
                    )?);
                }
            } else if record.stage == TreatyStage::Scheduled {
                let activation = record
                    .activation_day
                    .ok_or("Scheduled treaty has no activation date")?;
                if activation < self.day {
                    return Err("Treaty activation was not processed at its daily boundary".into());
                }
                if activation == self.day {
                    record.stage = TreatyStage::Active;
                    record.activated_on = Some(self.day);
                    outcomes.push(record.milestone(
                        self.day,
                        TreatyOutcome::Activated,
                        None,
                        None,
                    )?);
                }
            } else if record.stage.operative() {
                if self.day >= record.expires_at.ok_or("Active treaty has no expiry")? {
                    record.stage = TreatyStage::Expired;
                    record.closed_on = Some(self.day);
                    outcomes.push(record.milestone(
                        self.day,
                        TreatyOutcome::Expired,
                        None,
                        None,
                    )?);
                } else if record
                    .notice
                    .as_ref()
                    .is_some_and(|notice| self.day >= notice.until)
                {
                    record.stage = TreatyStage::Terminated;
                    record.closed_on = Some(self.day);
                    outcomes.push(record.milestone(
                        self.day,
                        TreatyOutcome::Terminated,
                        None,
                        None,
                    )?);
                }
            }
            self.diplomacy
                .as_mut()
                .ok_or("Missing treaty state")?
                .agreements
                .insert(id, record);
            for milestone in outcomes {
                self.emit_treaty_milestone(id, milestone)?;
            }
        }
        Ok(())
    }

    pub(crate) fn treaty_ai_turn(&mut self, content: &Content) -> Result<(), String> {
        let Some(rules) = &content.diplomacy else {
            return Ok(());
        };
        if !(self.day.units() - content.start_day()?.units())
            .is_multiple_of(rules.ai_cadence.units())
        {
            return Ok(());
        }
        let actors: Vec<_> = self
            .countries
            .keys()
            .filter(|id| *id != &self.player)
            .cloned()
            .collect();
        for actor in actors {
            let policy = rules.ai.get(&actor).ok_or("Missing treaty AI policy")?;
            let candidates: Vec<_> = self
                .diplomacy
                .as_ref()
                .ok_or("Missing treaty state")?
                .agreements
                .values()
                .filter(|record| record.participant(&actor) && !record.stage.closed())
                .filter_map(|record| {
                    let reference = (record.id, record.revision);
                    if record.stage == TreatyStage::Negotiating && record.offered_by != actor {
                        if policy.stance == TreatyStance::Decline {
                            Some(TreatyAction::Reject {
                                agreement: reference.0,
                                revision: reference.1,
                            })
                        } else if record.terms.duration <= policy.maximum_duration
                            && record.terms.notice >= policy.minimum_notice
                        {
                            Some(TreatyAction::Sign {
                                agreement: reference.0,
                                revision: reference.1,
                            })
                        } else if record.revision.0 < rules.maximum_revisions {
                            let duration = record.terms.duration.min(policy.maximum_duration);
                            let notice =
                                record.terms.notice.min(duration).max(policy.minimum_notice);
                            Some(TreatyAction::Counteroffer {
                                agreement: reference.0,
                                revision: reference.1,
                                terms: NonAggressionTerms {
                                    duration: duration.max(notice),
                                    notice,
                                },
                            })
                        } else {
                            Some(TreatyAction::Reject {
                                agreement: reference.0,
                                revision: reference.1,
                            })
                        }
                    } else if record.stage == TreatyStage::AwaitingRatification
                        && !record.ratifications.contains_key(&actor)
                    {
                        Some(TreatyAction::Ratify {
                            agreement: reference.0,
                            revision: reference.1,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            for action in candidates {
                if self
                    .plan_treaty_action(content, &actor, self.next_command, &action)
                    .is_err()
                {
                    continue;
                }
                let command = self.command(actor.clone(), Action::Diplomacy { action });
                // An eligible order's transactional failure must fail this daily
                // boundary, rather than silently swallowing exhausted event IDs.
                self.apply(content, command, crate::developer::CommandOrigin::Ai)?;
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    #[test]
    fn a_third_government_cannot_probe_an_unaccepted_foreign_offer_by_id() {
        let content = Content::diplomacy_bundled().unwrap();
        let mut world = World::new(&content, CountryId("COR".into()), 17).unwrap();
        // Trusted test actor creates the foreign negotiation through the same
        // paid command handler; this does not grant the player actor control.
        let command = world.command(
            CountryId("AUR".into()),
            Action::Diplomacy {
                action: TreatyAction::Offer {
                    with: CountryId("BOR".into()),
                    terms: NonAggressionTerms {
                        duration: DaySpan::new(180),
                        notice: DaySpan::new(30),
                    },
                },
            },
        );
        world
            .apply(&content, command, crate::developer::CommandOrigin::Ai)
            .unwrap();
        let before = crate::persistence::encode(&world, &content).unwrap();
        let mut failures = Vec::new();
        for id in [1, 999] {
            failures.push(
                world
                    .issue(
                        &content,
                        Action::Diplomacy {
                            action: TreatyAction::Sign {
                                agreement: TreatyId(id),
                                revision: TreatyRevision(1),
                            },
                        },
                    )
                    .unwrap_err(),
            );
            assert_eq!(
                crate::persistence::encode(&world, &content).unwrap(),
                before
            );
        }
        assert_eq!(failures, vec!["Agreement unavailable"; 2]);
        assert!(
            world
                .treaties_view(&content, Viewer::Observer, &CountryId("AUR".into()))
                .unwrap()
                .is_empty()
        );
        assert!(
            world
                .treaties_view(
                    &content,
                    Viewer::Government(CountryId("AUR".into())),
                    &CountryId("AUR".into())
                )
                .unwrap()
                .is_empty()
        );
        let event = world
            .events
            .iter()
            .find(|event| event.country == CountryId("AUR".into()))
            .unwrap()
            .id
            .as_ref()
            .unwrap();
        assert!(
            world
                .event_treaty(&content, Viewer::Government(CountryId("AUR".into())), event)
                .unwrap()
                .is_none()
        );
    }
}
