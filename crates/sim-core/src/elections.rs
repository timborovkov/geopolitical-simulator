//! Electoral authorities administer announced polls under explicit scenario mandates.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Definition rows are not political priority in rules 24 and later.
pub(crate) fn ordered_opposition<'a>(
    rules: u32,
    parties: &'a [PartyDefinition],
    incumbent: &PartyId,
) -> Vec<&'a PartyDefinition> {
    let mut opposition: Vec<_> = parties
        .iter()
        .filter(|party| &party.id != incumbent)
        .collect();
    if rules >= 24 {
        opposition.sort_by(|a, b| a.id.cmp(&b.id));
    }
    opposition
}

pub(crate) fn opposition_leader<'a>(
    rules: u32,
    opposition: &[&'a PartyDefinition],
    seats: &BTreeMap<PartyId, u16>,
) -> Option<&'a PartyDefinition> {
    if rules >= 24 {
        opposition.iter().copied().max_by(|a, b| {
            seats[&a.id]
                .cmp(&seats[&b.id])
                .then_with(|| b.id.cmp(&a.id))
        })
    } else {
        // Preserve the published last-encountered winner for equal seat totals.
        opposition
            .iter()
            .copied()
            .max_by_key(|party| seats[&party.id])
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElectionRequestId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElectoralMandate {
    pub authority: OfficeId,
    pub country: CountryId,
    pub title: String,
    pub snap_forms: Vec<Regime>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElectionDefinition {
    pub mandates: Vec<ElectoralMandate>,
    pub decision_days: DaySpan,
    pub campaign_days: DaySpan,
    pub minimum_interval_days: DaySpan,
    pub regular_term_years: u16,
    pub administration_cost_bps: BasisPoints,
    pub source: String,
    pub source_version: String,
}
impl ElectionDefinition {
    pub fn mandate(&self, id: &CountryId) -> Option<&ElectoralMandate> {
        self.mandates.iter().find(|m| &m.country == id)
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=90).contains(&self.decision_days.units())
            || !(30..=180).contains(&self.campaign_days.units())
            || !(1..=365).contains(&self.minimum_interval_days.units())
            || !(1..=4).contains(&self.regular_term_years)
            || !(1..=100).contains(&self.administration_cost_bps.units_i64())
            || self.source_version.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("Invalid electoral mandate timing, costs or provenance".into());
        }
        let mut countries = BTreeSet::new();
        let mut offices = BTreeSet::new();
        for m in &self.mandates {
            if !countries.insert(&m.country)
                || !offices.insert(&m.authority)
                || m.authority.0.is_empty()
                || m.title.is_empty()
                || content.country(&m.country).is_none()
                || m.snap_forms
                    .iter()
                    .enumerate()
                    .any(|(i, form)| !form.competitive() || m.snap_forms[..i].contains(form))
                || content
                    .cabinet
                    .as_ref()
                    .is_some_and(|cabinet| cabinet.portfolios.iter().any(|p| p.id == m.authority))
            {
                return Err("Invalid, duplicate or cabinet-controlled electoral authority".into());
            }
            if content.identities.as_ref().is_some_and(|i| {
                i.offices
                    .iter()
                    .find(|o| o.id == m.authority)
                    .is_some_and(|o| {
                        o.country != m.country
                            || o.title != m.title
                            || o.role != OfficeRole::IndependentInstitution
                    })
            }) {
                return Err("Electoral authority conflicts with a reference office mandate".into());
            }
        }
        if countries != content.countries.iter().map(|c| &c.id).collect() {
            return Err("Every country needs a distinct electoral authority mandate".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ElectionCaseStatus {
    Pending,
    Announced,
    Conducted,
    Withdrawn,
    Refused { reason: String },
    Superseded,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElectionCase {
    pub id: ElectionRequestId,
    pub proposed_day: u32,
    pub decision_day: u32,
    pub vote_day: u32,
    pub sponsor: crate::GovernmentId,
    pub form: Regime,
    pub paid: Money,
    pub status: ElectionCaseStatus,
    pub resolved_day: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElectoralPoll {
    pub day: u32,
    pub request: Option<ElectionRequestId>,
    pub government_before: crate::GovernmentId,
    pub government_after: crate::GovernmentId,
    pub governing_party: PartyId,
    pub seats: BTreeMap<PartyId, u16>,
    pub contested: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElectoralAuthority {
    pub regime: Regime,
    pub government: crate::GovernmentId,
    pub next_request_day: u32,
    pub cases: Vec<ElectionCase>,
    pub polls: Vec<ElectoralPoll>,
}
impl ElectoralAuthority {
    pub fn new(rules: &ElectionDefinition, regime: Regime, start: u32) -> Self {
        Self {
            regime,
            government: crate::GovernmentId(1),
            next_request_day: start + rules.minimum_interval_days.units(),
            cases: Vec::new(),
            polls: Vec::new(),
        }
    }
    pub fn pending(&self) -> Option<&ElectionCase> {
        self.cases
            .iter()
            .find(|c| c.status == ElectionCaseStatus::Pending)
    }
    pub fn announced(&self) -> Option<&ElectionCase> {
        self.cases
            .iter()
            .find(|c| c.status == ElectionCaseStatus::Announced)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElectionPreview {
    pub authority: String,
    pub fee: Money,
    pub decision_day: u32,
    pub vote_day: u32,
    pub previous_vote_day: Option<u32>,
}
impl World {
    fn election_jurisdiction(&self, content: &Content, id: &CountryId) -> Result<(), String> {
        let rules = content
            .elections
            .as_ref()
            .ok_or("This campaign has no independent electoral authority workflow")?;
        let mandate = rules
            .mandate(id)
            .ok_or("Missing electoral authority mandate")?;
        let form = self.governing_regime(content, id)?;
        if !mandate.snap_forms.contains(form) {
            return Err("The government has no power to request an early poll under its current electoral mandate".into());
        }
        self.governing_rules(content, id)?.check(
            self.countries.get(id).ok_or("Unknown country")?,
            "elections",
            1,
        )
    }
    pub fn preview_election(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<ElectionPreview, String> {
        self.election_jurisdiction(content, id)?;
        let rules = content.elections.as_ref().expect("rules");
        let c = &self.countries[id];
        let state = c
            .electoral_authority
            .as_ref()
            .ok_or("Missing electoral authority state")?;
        if state.pending().is_some() || state.announced().is_some() {
            return Err("An election request or announced poll is already active".into());
        }
        if self.day.units() < state.next_request_day {
            return Err(format!(
                "The electoral authority's minimum interval runs until {}",
                date(state.next_request_day)
            ));
        }
        let decision_day = self
            .day
            .units()
            .checked_add(rules.decision_days.units())
            .ok_or("Election decision date overflow")?;
        let vote_day = decision_day
            .checked_add(rules.campaign_days.units())
            .filter(|d| *d <= MAX_DAY)
            .ok_or("The required review and campaign period would exceed the campaign calendar")?;
        if c.next_election.is_some_and(|day| day <= vote_day) {
            return Err(
                "An early-election request cannot postpone or replace an earlier scheduled poll"
                    .into(),
            );
        }
        let fee = Money(
            (content.country(id).expect("country").gdp * rules.administration_cost_bps.units_i64()
                / 10000)
                .max(1),
        );
        if fee.0 > c.available_cash().0 {
            return Err("Electoral administration requires uncommitted treasury".into());
        }
        Ok(ElectionPreview {
            authority: rules.mandate(id).expect("mandate").title.clone(),
            fee,
            decision_day,
            vote_day,
            previous_vote_day: c.next_election,
        })
    }
    pub(crate) fn request_election(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: ElectionRequestId,
    ) -> Result<(), String> {
        let preview = self.preview_election(content, id)?;
        let form = self.governing_regime(content, id)?.clone();
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            preview.fee.0,
            "Electoral registration and review administration",
        );
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += preview.fee.0;
        let state = c.electoral_authority.as_mut().expect("authority");
        state.next_request_day = self.day.units()
            + content
                .elections
                .as_ref()
                .expect("rules")
                .minimum_interval_days
                .units();
        state.cases.push(ElectionCase {
            id: request,
            proposed_day: self.day.units(),
            decision_day: preview.decision_day,
            vote_day: preview.vote_day,
            sponsor: c.government_number,
            form,
            paid: preview.fee,
            status: ElectionCaseStatus::Pending,
            resolved_day: None,
        });
        self.event(id,true,format!("Early-election request #{} submitted to {}. Paid {}m administration; independent review {}, proposed poll {}. The current poll date remains in force until approval. Administration and request interval remain after withdrawal or refusal.",request.0,preview.authority,preview.fee.0,date(preview.decision_day),date(preview.vote_day)));
        Ok(())
    }
    pub(crate) fn withdraw_election(
        &mut self,
        id: &CountryId,
        request: ElectionRequestId,
    ) -> Result<(), String> {
        let state = self
            .countries
            .get_mut(id)
            .and_then(|c| c.electoral_authority.as_mut())
            .ok_or("No electoral authority workflow")?;
        let case = state
            .cases
            .iter_mut()
            .find(|c| c.id == request)
            .ok_or("Unknown election request for this government")?;
        if case.status != ElectionCaseStatus::Pending {
            return Err("Only an unapproved request can be withdrawn; an announced poll cannot be cancelled by government order".into());
        }
        case.status = ElectionCaseStatus::Withdrawn;
        case.resolved_day = Some(self.day.units());
        self.event(id,true,format!("Early-election request #{} withdrawn before approval. Administration remains spent and the electoral interval remains in force.",request.0));
        Ok(())
    }
    pub(crate) fn sync_electoral_mandate(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.elections else {
            return;
        };
        let form = self.governing_regime(content, id).expect("form").clone();
        let c = self.countries.get_mut(id).expect("country");
        let state = c.electoral_authority.as_mut().expect("authority");
        let changed = state.regime != form;
        let successor = state.government != c.government_number;
        let earlier = state
            .announced()
            .is_some_and(|case| c.next_election.is_some_and(|day| day < case.vote_day));
        if !changed && !successor && !earlier {
            return;
        }
        let mut cancelled = false;
        for case in &mut state.cases {
            if (changed || successor) && case.status == ElectionCaseStatus::Pending
                || (changed || earlier) && case.status == ElectionCaseStatus::Announced
            {
                case.status = ElectionCaseStatus::Superseded;
                case.resolved_day = Some(self.day.units());
                cancelled = true;
            }
        }
        state.regime = form;
        state.government = c.government_number;
        if changed {
            state.next_request_day = state
                .next_request_day
                .max(self.day.units() + rules.minimum_interval_days.units());
        }
        if cancelled {
            self.event(id,true,if earlier { "An earlier mandatory confidence poll superseded the announced election. The authority retains the earlier legal date; paid administration remains spent." } else { "Electoral mandate or sponsoring cabinet changed. Superseded requests and polls remain in history; paid administration remains spent. A government order alone cannot cancel an announced poll." });
        }
    }
    pub(crate) fn resolve_election_requests(&mut self, content: &Content, id: &CountryId) {
        if content.elections.is_none() {
            return;
        }
        let Some(case) = self.countries[id]
            .electoral_authority
            .as_ref()
            .expect("authority")
            .pending()
            .filter(|c| c.decision_day <= self.day.units())
            .cloned()
        else {
            return;
        };
        let reason = if self.countries[id].government_number != case.sponsor {
            Some("The sponsoring cabinet has changed".into())
        } else if self.countries[id]
            .next_election
            .is_some_and(|day| day <= case.vote_day)
        {
            Some("An earlier poll is already required; government cannot postpone it".into())
        } else {
            self.election_jurisdiction(content, id).err()
        };
        let c = self.countries.get_mut(id).expect("country");
        let request = c
            .electoral_authority
            .as_mut()
            .expect("authority")
            .cases
            .iter_mut()
            .find(|r| r.id == case.id)
            .expect("request");
        request.resolved_day = Some(self.day.units());
        if let Some(reason) = reason {
            request.status = ElectionCaseStatus::Refused {
                reason: reason.clone(),
            };
            self.event(id,true,format!("Electoral authority refused request #{}: {reason}. Paid administration remains spent; the prior poll schedule remains in force.",case.id.0));
        } else {
            request.status = ElectionCaseStatus::Announced;
            c.next_election = Some(case.vote_day);
            self.event(id,true,format!("Electoral authority approved request #{} and announced a poll for {}. The government cannot withdraw this announced poll. The modeled allocation follows household opinion and the scenario's 100-seat rule, not a certified national electoral law.",case.id.0,date(case.vote_day)));
        }
    }
    pub(crate) fn record_electoral_poll(
        &mut self,
        content: &Content,
        id: &CountryId,
        government_before: crate::GovernmentId,
    ) {
        let Some(rules) = &content.elections else {
            return;
        };
        let c = self.countries.get_mut(id).expect("country");
        let state = c.electoral_authority.as_mut().expect("authority");
        let case = state
            .cases
            .iter_mut()
            .find(|r| r.status == ElectionCaseStatus::Announced && r.vote_day == self.day.units());
        let request = case.map(|r| {
            r.status = ElectionCaseStatus::Conducted;
            r.resolved_day = Some(self.day.units());
            r.id
        });
        for pending in state
            .cases
            .iter_mut()
            .filter(|c| c.status == ElectionCaseStatus::Pending)
        {
            pending.status = ElectionCaseStatus::Superseded;
            pending.resolved_day = Some(self.day.units());
        }
        state.polls.push(ElectoralPoll {
            day: self.day.units(),
            request,
            government_before,
            government_after: c.government_number,
            governing_party: c.ruling_party.clone(),
            seats: c.seats.clone(),
            contested: content.country(id).expect("country").parties.len() > 1,
        });
        state.next_request_day = state
            .next_request_day
            .max(self.day.units() + rules.minimum_interval_days.units());
        c.next_election = date(self.day.units())
            .parse::<CivilDate>()
            .expect("date")
            .anniversary(rules.regular_term_years)
            .and_then(CivilDate::campaign_day)
            .ok();
        self.event(id,true,format!("Electoral authority recorded the {} modeled poll and its 100-seat allocation. Next regular poll {}. This scenario does not model turnout, electoral districts or electoral-integrity certification.",date(self.day.units()),self.countries[id].next_election.map_or_else(||"falls outside the playable calendar".into(),date)));
    }
    pub(crate) fn ai_election_request(&mut self, content: &Content, id: &CountryId) {
        if content.elections.is_none()
            || !self.day.units().is_multiple_of(90)
            || self.countries[id].approval < 70
            || self.countries[id]
                .next_election
                .is_some_and(|day| day.saturating_sub(self.day.units()) <= 365)
        {
            return;
        }
        if self.preview_election(content, id).is_ok() {
            let command = self.command(id.clone(), Action::RequestElection);
            if let Err(reason) = self.apply(content, command, crate::developer::CommandOrigin::Ai) {
                self.event(
                    id,
                    false,
                    format!("AI election request declined by shared rules: {reason}"),
                );
            }
        }
    }
}
impl World {
    pub(crate) fn validate_electoral_authority(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let country = &self.countries[id];
        let (state, rules) = match (&country.electoral_authority, &content.elections) {
            (None, None) => return Ok(()),
            (Some(state), Some(rules)) => (state, rules),
            _ => {
                return Err("Electoral authority state differs from campaign rules/content".into());
            }
        };
        let start = content.start_day()?;
        let mandate = rules.mandate(id).ok_or("Missing electoral mandate")?;
        if state.regime != *self.governing_regime(content, id)?
            || state.government != country.government_number
            || state.next_request_day < start
            || state.next_request_day > MAX_DAY + rules.minimum_interval_days.units()
        {
            return Err("Invalid electoral authority identity or interval".into());
        }
        let proposals: Vec<_> = self
            .journal
            .iter()
            .filter(|e| {
                &e.command.issuer == id && matches!(e.command.action, Action::RequestElection)
            })
            .collect();
        if proposals.len() != state.cases.len()
            || state
                .cases
                .iter()
                .filter(|c| {
                    matches!(
                        c.status,
                        ElectionCaseStatus::Pending | ElectionCaseStatus::Announced
                    )
                })
                .count()
                > 1
        {
            return Err(
                "Election cases do not match the request journal or have overlapping mandates"
                    .into(),
            );
        }
        let fee = Money(
            (content.country(id).expect("country").gdp * rules.administration_cost_bps.units_i64()
                / 10000)
                .max(1),
        );
        let mut interval = start + rules.minimum_interval_days.units();
        for (case, origin) in state.cases.iter().zip(proposals) {
            let command = &origin.command;
            let form = country
                .constitution
                .as_ref()
                .and_then(|c| {
                    c.history
                        .iter()
                        .rev()
                        .find(|change| change.day <= command.day)
                })
                .map_or(&content.country(id).expect("country").regime, |change| {
                    &change.to
                });
            if case.id.0 != command.id.0
                || case.proposed_day != command.day
                || case.form != *form
                || !mandate.snap_forms.contains(form)
                || case.proposed_day.checked_add(rules.decision_days.units())
                    != Some(case.decision_day)
                || case.decision_day.checked_add(rules.campaign_days.units()) != Some(case.vote_day)
                || case.vote_day > MAX_DAY
                || case.sponsor == crate::GovernmentId(0)
                || case.sponsor > country.government_number
                || case.paid != fee
            {
                return Err("Invalid electoral request origin, jurisdiction, dates or cost".into());
            }
            interval = interval.max(case.proposed_day + rules.minimum_interval_days.units());
            let resolved = case.resolved_day;
            let withdrawn=self.journal.iter().any(|entry| &entry.command.issuer==id && matches!(entry.command.action,Action::WithdrawElectionRequest{request} if request==case.id) && Some(entry.command.day)==resolved);
            if (case.status == ElectionCaseStatus::Withdrawn) != withdrawn {
                return Err("Election withdrawal differs from the command journal".into());
            }
            let valid = match &case.status {
                ElectionCaseStatus::Pending => {
                    resolved.is_none()
                        && case.decision_day > self.day.units()
                        && case.sponsor == state.government
                        && case.form == state.regime
                }
                ElectionCaseStatus::Announced => {
                    resolved == Some(case.decision_day)
                        && case.decision_day <= self.day.units()
                        && case.vote_day > self.day.units()
                        && country.next_election == Some(case.vote_day)
                        && case.form == state.regime
                }
                ElectionCaseStatus::Conducted => {
                    resolved == Some(case.vote_day)
                        && state
                            .polls
                            .iter()
                            .any(|p| p.request == Some(case.id) && p.day == case.vote_day)
                }
                ElectionCaseStatus::Withdrawn => resolved.is_some_and(|day| {
                    day >= case.proposed_day && day < case.decision_day && day <= self.day.units()
                }),
                ElectionCaseStatus::Refused { reason } => {
                    !reason.is_empty()
                        && resolved == Some(case.decision_day)
                        && case.decision_day <= self.day.units()
                }
                ElectionCaseStatus::Superseded => resolved.is_some_and(|day| {
                    day >= case.proposed_day
                        && day <= self.day.units()
                        && (case.sponsor < state.government
                            || country
                                .next_election
                                .is_some_and(|poll| poll <= case.vote_day)
                            || state
                                .polls
                                .iter()
                                .any(|p| p.day >= day && p.day <= case.vote_day))
                }),
            };
            if !valid {
                return Err("Invalid electoral authority decision or announcement".into());
            }
        }
        for poll in &state.polls {
            if poll.day < start
                || poll.day > self.day.units()
                || poll.government_before == crate::GovernmentId(0)
                || poll.government_after < poll.government_before
                || poll.government_after > country.government_number
                || poll.government_after.saturating_sub(poll.government_before) > 1
                || poll.seats.values().map(|v| u32::from(*v)).sum::<u32>() != 100
                || poll.seats.keys().collect::<BTreeSet<_>>() != country.seats.keys().collect()
                || !poll.seats.contains_key(&poll.governing_party)
                || poll.contested != (content.country(id).expect("country").parties.len() > 1)
                || poll.request.is_some_and(|request| {
                    !state.cases.iter().any(|case| {
                        case.id == request
                            && case.status == ElectionCaseStatus::Conducted
                            && case.vote_day == poll.day
                    })
                })
            {
                return Err("Invalid recorded modeled poll".into());
            }
            interval = interval.max(poll.day + rules.minimum_interval_days.units());
        }
        if state.polls.windows(2).any(|p| p[0].day >= p[1].day) {
            return Err("Invalid poll chronology".into());
        }
        if let Some(change) = country.constitution.as_ref().and_then(|c| c.history.last()) {
            interval = interval.max(change.day + rules.minimum_interval_days.units());
        }
        if state.next_request_day != interval {
            return Err(
                "Electoral interval differs from proposals, polls or constitutional changes".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedPoll {
    pub day: u32,
    pub governing_party: PartyId,
    pub seats: BTreeMap<PartyId, u16>,
    pub contested: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicElectionReport {
    pub authority: OfficeId,
    pub title: String,
    pub next_poll: Option<u32>,
    pub last_poll: Option<PublishedPoll>,
}
impl World {
    pub(crate) fn published_election(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Option<PublicElectionReport> {
        let mandate = content.elections.as_ref()?.mandate(id)?;
        let country = self.countries.get(id)?;
        let state = country.electoral_authority.as_ref()?;
        Some(PublicElectionReport {
            authority: mandate.authority.clone(),
            title: mandate.title.clone(),
            next_poll: country.next_election.filter(|day| *day <= MAX_DAY),
            last_poll: state.polls.last().map(|p| PublishedPoll {
                day: p.day,
                governing_party: p.governing_party.clone(),
                seats: p.seats.clone(),
                contested: p.contested,
            }),
        })
    }
}

#[cfg(test)]
mod party_order_tests {
    use super::*;

    fn parties() -> Vec<PartyDefinition> {
        [
            ("AUR-1", "Incumbent", 60),
            ("AUR-2", "Zeta", 20),
            ("AUR-3", "Alpha", 20),
        ]
        .into_iter()
        .map(|(id, name, seats)| PartyDefinition {
            id: id.into(),
            name: name.into(),
            leader: format!("Leader {id}"),
            ideology: "Authored fixture".into(),
            seats,
        })
        .collect()
    }

    #[test]
    fn current_opposition_order_uses_ids_instead_of_rows_or_names() {
        let original = parties();
        let mut reversed = original.clone();
        reversed.reverse();
        for rows in [&original, &reversed] {
            let opposition = ordered_opposition(24, rows, &"AUR-1".into());
            assert_eq!(
                opposition
                    .iter()
                    .map(|party| party.id.0.as_str())
                    .collect::<Vec<_>>(),
                ["AUR-2", "AUR-3"]
            );
        }
    }

    #[test]
    fn current_successor_prefers_more_seats_then_the_lowest_party_id() {
        let rows = parties();
        let opposition = ordered_opposition(24, &rows, &"AUR-1".into());
        let mut seats = BTreeMap::from([("AUR-2".into(), 30), ("AUR-3".into(), 30)]);
        assert_eq!(
            opposition_leader(24, &opposition, &seats).unwrap().id,
            PartyId::from("AUR-2")
        );
        seats.insert("AUR-3".into(), 31);
        assert_eq!(
            opposition_leader(24, &opposition, &seats).unwrap().id,
            PartyId::from("AUR-3")
        );
        assert!(opposition_leader(24, &[], &seats).is_none());
    }

    #[test]
    fn legacy_rows_and_last_equal_seat_winners_remain_unchanged() {
        let original = parties();
        let mut reversed = original.clone();
        reversed.reverse();
        let seats = BTreeMap::from([("AUR-2".into(), 30), ("AUR-3".into(), 30)]);
        for rules in [2, 23] {
            for (rows, expected) in [(&original, "AUR-3"), (&reversed, "AUR-2")] {
                let opposition = ordered_opposition(rules, rows, &"AUR-1".into());
                assert_eq!(
                    opposition_leader(rules, &opposition, &seats).unwrap().id,
                    PartyId::from(expected)
                );
            }
        }
    }
}
