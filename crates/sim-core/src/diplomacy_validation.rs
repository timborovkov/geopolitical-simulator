//! Agreement invariants and explicit event/journal links, independent of UI text.
use crate::diplomacy::treaty_date;
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

fn next_stage(stage: Option<TreatyStage>, outcome: TreatyOutcome) -> Result<TreatyStage, String> {
    use TreatyOutcome as O;
    use TreatyStage as S;
    Ok(match (stage, outcome) {
        (None, O::Offered) => S::Negotiating,
        (Some(S::Negotiating), O::Counteroffered) => S::Negotiating,
        (Some(S::Negotiating), O::Signed) => S::AwaitingRatification,
        (Some(S::AwaitingRatification), O::RatificationSubmitted | O::Ratified) => {
            S::AwaitingRatification
        }
        (Some(S::AwaitingRatification), O::ActivationScheduled) => S::Scheduled,
        (Some(S::Scheduled), O::Activated) => S::Active,
        (Some(S::Active), O::NoticeGiven) => S::Notice,
        (Some(S::Active | S::Notice), O::Expired) => S::Expired,
        (Some(S::Active | S::Notice), O::Terminated) => S::Terminated,
        (Some(S::Negotiating), O::Refused) => S::Refused,
        (Some(S::Negotiating | S::AwaitingRatification | S::Scheduled), O::Withdrawn) => {
            S::Withdrawn
        }
        (Some(S::Negotiating | S::AwaitingRatification), O::TimedOut) => S::TimedOut,
        (Some(S::AwaitingRatification), O::RatificationFailed) => S::RatificationFailed,
        (Some(S::Negotiating | S::AwaitingRatification | S::Scheduled), O::SponsorshipLapsed) => {
            S::SponsorshipLapsed
        }
        _ => return Err("Invalid treaty lifecycle transition".into()),
    })
}

impl World {
    pub(crate) fn valid_treaty_command_shape(
        &self,
        content: &Content,
        actor: &CountryId,
        action: &TreatyAction,
    ) -> bool {
        let Some(rules) = &content.diplomacy else {
            return false;
        };
        if self.rules != 41 || content.rules != 41 || !self.countries.contains_key(actor) {
            return false;
        }
        match action {
            TreatyAction::Offer { with, terms } => {
                with != actor && self.countries.contains_key(with) && terms.validate(rules).is_ok()
            }
            TreatyAction::Counteroffer {
                agreement,
                revision,
                terms,
            } => {
                agreement.0 > 0
                    && (1..rules.maximum_revisions).contains(&revision.0)
                    && terms.validate(rules).is_ok()
            }
            _ => action.target().is_some_and(|(id, revision)| {
                id.0 > 0 && (1..=rules.maximum_revisions).contains(&revision.0)
            }),
        }
    }

    fn treaty_journal(&self, id: CommandId) -> Result<&JournalEntry, String> {
        id.0.checked_sub(1)
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.journal.get(index))
            .filter(|entry| entry.command.id == id)
            .ok_or_else(|| "Treaty does not identify an accepted command".into())
    }

    pub(crate) fn validate_treaty_event(
        &self,
        event: &Event,
        id: TreatyId,
        milestone: TreatyMilestoneId,
    ) -> Result<(), String> {
        if self.rules != 41 {
            return Err("Treaty event links require agreement rules".into());
        }
        let record = self
            .diplomacy
            .as_ref()
            .and_then(|state| state.agreements.get(&id))
            .ok_or("Treaty event has no agreement")?;
        if !record.participant(&event.country)
            || !record
                .milestones
                .iter()
                .any(|entry| entry.id == milestone && entry.day == event.day)
        {
            return Err("Treaty event differs from its participant milestone".into());
        }
        Ok(())
    }

    pub(crate) fn validate_treaties(&self, content: &Content) -> Result<(), String> {
        let (rules, state) = match (&content.diplomacy, &self.diplomacy) {
            (None, None) if self.rules != 41 && content.rules != 41 => return Ok(()),
            (Some(rules), Some(state)) if self.rules == 41 && content.rules == 41 => (rules, state),
            _ => return Err("Agreement state differs from campaign rules/content".into()),
        };
        if self.content_hash != content.hash() {
            return Err("Agreement state requires exact content".into());
        }
        rules.validate(content)?;
        if state.agreements.len() > rules.maximum_records as usize
            || state.next_id.0 != state.agreements.len() as u64 + 1
        {
            return Err("Invalid agreement archive or next identity".into());
        }
        let start = content.start_day()?;
        let mut commands = BTreeSet::new();
        let mut live_pairs = BTreeSet::new();
        let mut previous_pairs = BTreeMap::<[CountryId; 2], Option<CampaignDay>>::new();
        let mut previous_proposal = CommandId(0);
        for (index, (id, record)) in state.agreements.iter().enumerate() {
            if id.0 != index as u64 + 1
                || &record.id != id
                || record.parties[0] >= record.parties[1]
                || record
                    .parties
                    .iter()
                    .any(|party| !self.countries.contains_key(party))
                || !record.participant(&record.offered_by)
                || !(1..=rules.maximum_revisions).contains(&record.revision.0)
                || record.created < start
                || record.offered_on < record.created
                || record.offered_on > self.day
                || record.offer_deadline != treaty_date(record.offered_on, rules.response_days)?
                || record.stage.closed() != record.closed_on.is_some()
                || record
                    .closed_on
                    .is_some_and(|day| day < record.offered_on || day > self.day)
                || record.milestones.is_empty()
                || record.milestones.len() > 64
                || record.next_milestone.0 != record.milestones.len() as u32 + 1
            {
                return Err("Invalid agreement identity, parties, dates or history".into());
            }
            record.terms.validate(rules)?;
            if record.proposal_command <= previous_proposal {
                return Err("Treaty allocation differs from command order".into());
            }
            previous_proposal = record.proposal_command;
            if let Some(prior) = previous_pairs.insert(record.parties.clone(), record.closed_on) {
                let closed = prior.ok_or("A pair negotiated over an existing live agreement")?;
                if record.created < treaty_date(closed, rules.pair_cooldown)? {
                    return Err("Treaty pair cooldown was bypassed".into());
                }
            }
            if !record.stage.closed() && !live_pairs.insert(record.parties.clone()) {
                return Err("Overlapping live agreements for one pair".into());
            }
            let mut stage = None;
            let mut revision = 0_u32;
            let mut prior_day = record.created;
            let mut paid = BTreeMap::<CountryId, Money>::new();
            let mut latest_terms = None;
            for (milestone_index, milestone) in record.milestones.iter().enumerate() {
                if milestone.id.0 != milestone_index as u32 + 1
                    || milestone.day < prior_day
                    || milestone.day > self.day
                    || milestone
                        .actor
                        .as_ref()
                        .is_some_and(|actor| !record.participant(actor))
                {
                    return Err("Invalid treaty milestone identity, actor or day".into());
                }
                let expected_revision = if matches!(
                    milestone.outcome,
                    TreatyOutcome::Offered | TreatyOutcome::Counteroffered
                ) {
                    revision.checked_add(1).ok_or("Treaty revision overflow")?
                } else {
                    revision
                };
                if milestone.revision.0 != expected_revision {
                    return Err("Treaty milestone revision differs from its terms".into());
                }
                revision = expected_revision;
                stage = Some(next_stage(stage, milestone.outcome)?);
                prior_day = milestone.day;
                if let Some(command_id) = milestone.command {
                    if !commands.insert(command_id) {
                        return Err("One command produced multiple treaty submissions".into());
                    }
                    let entry = self.treaty_journal(command_id)?;
                    let Action::Diplomacy { action } = &entry.command.action else {
                        return Err("Treaty milestone references another command family".into());
                    };
                    let actor = milestone
                        .actor
                        .as_ref()
                        .ok_or("Treaty submission lacks its actor")?;
                    if &entry.command.issuer != actor
                        || entry.command.day != milestone.day
                        || !self.valid_treaty_command_shape(content, actor, action)
                    {
                        return Err("Treaty milestone differs from its accepted command".into());
                    }
                    if milestone_index == 0 && milestone.day != record.created {
                        return Err("Treaty creation differs from its first milestone".into());
                    }
                    if matches!(
                        action,
                        TreatyAction::Counteroffer { .. }
                            | TreatyAction::Sign { .. }
                            | TreatyAction::Reject { .. }
                    ) {
                        let Some((offering, old_terms, offered_day)) = latest_terms else {
                            return Err("Treaty response has no current offer".into());
                        };
                        if actor == offering
                            || milestone.day >= treaty_date(offered_day, rules.response_days)?
                            || matches!(action, TreatyAction::Counteroffer { terms, .. } if terms == old_terms)
                        {
                            return Err(
                                "Treaty response bypassed its recipient, terms or deadline".into(),
                            );
                        }
                    }
                    if matches!(action, TreatyAction::Ratify { .. })
                        && record
                            .ratifications
                            .get(actor)
                            .is_none_or(|request| request.command != command_id)
                    {
                        return Err(
                            "Submitted domestic ratification has no retained request".into()
                        );
                    }
                    if matches!(action, TreatyAction::Sign { .. })
                        && record
                            .signatures
                            .get(actor)
                            .is_none_or(|signature| signature.command != command_id)
                    {
                        return Err("Accepted treaty signature is missing".into());
                    }
                    if matches!(action, TreatyAction::Terminate { .. })
                        && record
                            .notice
                            .as_ref()
                            .is_none_or(|notice| notice.command != command_id)
                    {
                        return Err("Accepted termination notice is missing".into());
                    }
                    let outcome_matches = match action {
                        TreatyAction::Offer { with, terms } => {
                            if command_id != record.proposal_command
                                || !record.participant(with)
                                || milestone_index != 0
                            {
                                return Err("Treaty creation differs from its offer".into());
                            }
                            latest_terms = Some((actor, terms, entry.command.day));
                            milestone.outcome == TreatyOutcome::Offered
                        }
                        TreatyAction::Counteroffer {
                            agreement,
                            revision,
                            terms,
                        } => {
                            if agreement != id
                                || revision.0.checked_add(1) != Some(milestone.revision.0)
                            {
                                return Err(
                                    "Counteroffer revision differs from its submission".into()
                                );
                            }
                            latest_terms = Some((actor, terms, entry.command.day));
                            milestone.outcome == TreatyOutcome::Counteroffered
                        }
                        other => {
                            if other.target() != Some((*id, milestone.revision)) {
                                return Err("Treaty command targets another record/revision".into());
                            }
                            matches!(
                                (other, milestone.outcome),
                                (TreatyAction::Sign { .. }, TreatyOutcome::Signed)
                                    | (
                                        TreatyAction::Ratify { .. },
                                        TreatyOutcome::RatificationSubmitted
                                    )
                                    | (TreatyAction::Reject { .. }, TreatyOutcome::Refused)
                                    | (TreatyAction::Withdraw { .. }, TreatyOutcome::Withdrawn)
                                    | (
                                        TreatyAction::Terminate { .. },
                                        TreatyOutcome::NoticeGiven | TreatyOutcome::Terminated
                                    )
                            )
                        }
                    };
                    if !outcome_matches {
                        return Err("Treaty command has the wrong outcome".into());
                    }
                    if action.paid_service() {
                        let fee = rules.administration(
                            content.country(actor).ok_or("Unknown treaty payer")?,
                        )?;
                        let total = paid.entry(actor.clone()).or_insert(Money(0));
                        total.0 = total
                            .0
                            .checked_add(fee.0)
                            .ok_or("Treaty fee total overflow")?;
                    }
                } else if !matches!(
                    milestone.outcome,
                    TreatyOutcome::Ratified
                        | TreatyOutcome::ActivationScheduled
                        | TreatyOutcome::Activated
                        | TreatyOutcome::Expired
                        | TreatyOutcome::Terminated
                        | TreatyOutcome::TimedOut
                        | TreatyOutcome::RatificationFailed
                        | TreatyOutcome::SponsorshipLapsed
                ) {
                    return Err("Treaty submission has no command".into());
                }
            }
            if record.closed_on.is_some_and(|day| {
                record
                    .milestones
                    .last()
                    .is_none_or(|milestone| milestone.day != day)
            }) {
                return Err("Agreement closure differs from its terminal milestone".into());
            }
            if stage != Some(record.stage)
                || revision != record.revision.0
                || paid != record.administration_paid
                || latest_terms.is_none_or(|(actor, terms, day)| {
                    actor != &record.offered_by
                        || terms != &record.terms
                        || day != record.offered_on
                })
            {
                return Err("Treaty current state differs from its recorded lifecycle".into());
            }
            self.validate_treaty_signatures(content, record)?;
            self.validate_treaty_dates(content, record)?;
        }
        for event in &self.events {
            if let Some(EventContext::Treaty {
                agreement,
                milestone,
            }) = event.context
            {
                self.validate_treaty_event(event, agreement, milestone)?;
            }
        }
        for entry in &self.journal {
            if matches!(entry.command.action, Action::Diplomacy { .. })
                && !commands.contains(&entry.command.id)
            {
                return Err("Accepted treaty command has no retained milestone".into());
            }
        }
        Ok(())
    }

    fn validate_treaty_signatures(
        &self,
        content: &Content,
        record: &TreatyRecord,
    ) -> Result<(), String> {
        if record.signed_on.is_some()
            != record
                .milestones
                .iter()
                .any(|milestone| milestone.outcome == TreatyOutcome::Signed)
        {
            return Err("Agreement lost or invented its two-signature milestone".into());
        }
        if record.signatures.is_empty()
            || record.signatures.len() > 2
            || record.ratifications.len() > 2
            || (record.signatures.len() == 2) != record.signed_on.is_some()
            || (record.stage == TreatyStage::Negotiating
                && (record.signatures.len() != 1
                    || !record.signatures.contains_key(&record.offered_by)))
        {
            return Err("Invalid treaty signatures or ratification ownership".into());
        }
        for (party, signature) in &record.signatures {
            let entry = self.treaty_journal(signature.command)?;
            if !record.participant(party)
                || &entry.command.issuer != party
                || entry.command.day != signature.day
                || entry
                    .governing_context
                    .as_ref()
                    .is_none_or(|context| context.government != signature.government)
                || signature.day < record.offered_on
                || signature.day > self.day
            {
                return Err("Treaty signature differs from its government submission".into());
            }
            let Action::Diplomacy { action } = &entry.command.action else {
                return Err("Signature has no treaty command".into());
            };
            let matches = match action {
                TreatyAction::Offer { terms, .. } => {
                    record.revision == TreatyRevision(1)
                        && signature.command == record.proposal_command
                        && terms == &record.terms
                }
                TreatyAction::Counteroffer {
                    agreement,
                    revision,
                    terms,
                } => {
                    agreement == &record.id
                        && revision.0.checked_add(1) == Some(record.revision.0)
                        && terms == &record.terms
                }
                TreatyAction::Sign {
                    agreement,
                    revision,
                } => agreement == &record.id && revision == &record.revision,
                _ => false,
            };
            if !matches {
                return Err("Treaty signature names stale or different terms".into());
            }
        }
        if let Some(signed) = record.signed_on
            && record
                .signatures
                .values()
                .map(|signature| signature.day)
                .max()
                != Some(signed)
        {
            return Err("Invalid two-signature date".into());
        }
        for (party, request) in &record.ratifications {
            let signature = record
                .signatures
                .get(party)
                .ok_or("Ratification has no signature")?;
            let signed = record
                .signed_on
                .ok_or("Ratification precedes both signatures")?;
            let entry = self.treaty_journal(request.command)?;
            let delay = DaySpan::new(
                self.governing_rules_at(content, party, request.submitted)?
                    .consent_days
                    .units()
                    .max(1),
            );
            if &entry.command.issuer != party
                || entry.command.day != request.submitted
                || !matches!(&entry.command.action, Action::Diplomacy { action: TreatyAction::Ratify { agreement, revision } } if agreement == &record.id && revision == &record.revision)
                || entry
                    .governing_context
                    .as_ref()
                    .is_none_or(|context| context.government != request.government)
                || request.government != signature.government
                || request.submitted < signed
                || request.submitted > self.day
                || request.submitted
                    >= treaty_date(
                        signed,
                        content
                            .diplomacy
                            .as_ref()
                            .ok_or("Missing treaty definition")?
                            .response_days,
                    )?
                || request.review != treaty_date(request.submitted, delay)?
                || request
                    .passed
                    .is_some_and(|day| day != request.review || day > self.day)
                || (!record.stage.closed()
                    && request.passed.is_none()
                    && request.review <= self.day)
            {
                return Err("Invalid domestic treaty ratification".into());
            }
            let passes = record
                .milestones
                .iter()
                .filter(|milestone| {
                    milestone.outcome == TreatyOutcome::Ratified
                        && milestone.actor.as_ref() == Some(party)
                })
                .collect::<Vec<_>>();
            if request.passed.map_or(!passes.is_empty(), |day| {
                passes.len() != 1 || passes[0].day != day
            }) {
                return Err("Ratification result differs from its milestone".into());
            }
        }
        if record
            .milestones
            .iter()
            .filter(|milestone| milestone.outcome == TreatyOutcome::Ratified)
            .count()
            != record
                .ratifications
                .values()
                .filter(|request| request.passed.is_some())
                .count()
        {
            return Err("Unowned treaty ratification result".into());
        }
        Ok(())
    }

    fn validate_treaty_dates(
        &self,
        content: &Content,
        record: &TreatyRecord,
    ) -> Result<(), String> {
        if record.activation_day.is_some() != record.expires_at.is_some() {
            return Err("Incomplete treaty activation timetable".into());
        }
        if let Some(activation) = record.activation_day
            && (record.ratifications.len() != 2
                || record
                    .ratifications
                    .values()
                    .any(|request| request.passed.is_none())
                || record
                    .ratifications
                    .values()
                    .filter_map(|request| request.passed)
                    .max()
                    .and_then(|day| day.checked_add(DaySpan::new(1)))
                    != Some(activation)
                || record.expires_at != Some(treaty_date(activation, record.terms.duration)?)
                || (record.stage == TreatyStage::Scheduled && activation <= self.day))
        {
            return Err("Activation does not follow both domestic ratifications".into());
        }
        let activated = matches!(
            record.stage,
            TreatyStage::Active
                | TreatyStage::Notice
                | TreatyStage::Expired
                | TreatyStage::Terminated
        );
        if record.activated_on.is_some() != activated
            || record
                .activated_on
                .is_some_and(|day| Some(day) != record.activation_day || day > self.day)
            || (record.stage.operative() && record.expires_at.is_none_or(|day| day <= self.day))
            || record
                .notice
                .as_ref()
                .is_some_and(|notice| !record.participant(&notice.by))
        {
            return Err("Invalid active agreement dates or notice owner".into());
        }
        if let Some(notice) = &record.notice {
            let activated = record.activated_on.ok_or("Notice predates operation")?;
            let expires = record.expires_at.ok_or("Notice has no natural expiry")?;
            let entry = self.treaty_journal(notice.command)?;
            if !matches!(
                record.stage,
                TreatyStage::Notice | TreatyStage::Terminated | TreatyStage::Expired
            ) || entry.command.issuer != notice.by
                || entry.command.day != notice.day
                || !matches!(&entry.command.action, Action::Diplomacy { action: TreatyAction::Terminate { agreement, revision } } if agreement == &record.id && revision == &record.revision)
                || notice.day < activated
                || notice.day >= expires
                || notice.day > self.day
                || notice
                    .day
                    .checked_add(record.terms.notice)
                    .map(|day| day.min(expires))
                    != Some(notice.until)
                || (record.stage == TreatyStage::Notice && notice.until <= self.day)
            {
                return Err("Invalid treaty termination notice".into());
            }
        } else if matches!(record.stage, TreatyStage::Notice | TreatyStage::Terminated) {
            return Err("Termination has no notice".into());
        }
        if record.stage == TreatyStage::Expired
            && (record.closed_on != record.expires_at
                || record
                    .notice
                    .as_ref()
                    .is_some_and(|notice| Some(notice.until) != record.expires_at))
        {
            return Err("Invalid natural treaty expiry".into());
        }
        if record.stage == TreatyStage::Terminated
            && record.notice.as_ref().is_none_or(|notice| {
                record.closed_on != Some(notice.until)
                    || record
                        .expires_at
                        .is_none_or(|expiry| notice.until >= expiry)
            })
        {
            return Err("Invalid notice termination or expiry tie".into());
        }
        if record.stage == TreatyStage::TimedOut {
            let deadline = if let Some(signed) = record.signed_on {
                if record.ratifications.len() == 2 {
                    return Err("Submitted ratifications were incorrectly timed out".into());
                }
                treaty_date(
                    signed,
                    content
                        .diplomacy
                        .as_ref()
                        .ok_or("Missing treaty rules")?
                        .response_days,
                )?
            } else {
                record.offer_deadline
            };
            if record.closed_on != Some(deadline) {
                return Err("Treaty timeout differs from its deadline".into());
            }
        }
        if !record.stage.closed()
            && record.stage == TreatyStage::Negotiating
            && self.day >= record.offer_deadline
        {
            return Err("Unprocessed offer expiry".into());
        }
        if record.stage == TreatyStage::AwaitingRatification
            && record.ratifications.len() < 2
            && record.signed_on.is_some_and(|signed| {
                signed
                    .checked_add(
                        content
                            .diplomacy
                            .as_ref()
                            .expect("validated treaty rules")
                            .response_days,
                    )
                    .is_none_or(|day| self.day >= day)
            })
        {
            return Err("Unprocessed ratification submission expiry".into());
        }
        Ok(())
    }
}
