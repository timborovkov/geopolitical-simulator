//! Authored ministerial appointments. Skill affects preparation time, not output.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppointmentId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortfolioDefinition {
    pub id: OfficeId,
    pub country: CountryId,
    pub title: String,
    pub policies: Vec<PolicyId>,
    pub minimum_skill: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MinisterCandidate {
    pub person: PersonId,
    pub office: OfficeId,
    pub name: String,
    pub skill: i32,
    pub acceptability: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CabinetDefinition {
    pub portfolios: Vec<PortfolioDefinition>,
    pub candidates: Vec<MinisterCandidate>,
    pub appointment_cost_bps: BasisPoints,
    pub dismissal_cost_bps: BasisPoints,
    pub cooldown_days: DaySpan,
    pub vacancy_preparation_days: DaySpan,
    pub source: String,
    pub source_version: String,
}
impl CabinetDefinition {
    pub fn office(&self, id: &OfficeId) -> Option<&PortfolioDefinition> {
        self.portfolios.iter().find(|p| &p.id == id)
    }
    pub fn candidate(&self, id: &PersonId) -> Option<&MinisterCandidate> {
        self.candidates.iter().find(|p| &p.person == id)
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=100).contains(&self.appointment_cost_bps.units_i64())
            || !(1..=100).contains(&self.dismissal_cost_bps.units_i64())
            || !(1..=365).contains(&self.cooldown_days.units())
            || !(1..=90).contains(&self.vacancy_preparation_days.units())
            || self.source_version.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("Invalid cabinet costs, preparation rules or provenance".into());
        }
        let mut offices = BTreeSet::new();
        let mut policies = BTreeSet::new();
        for p in &self.portfolios {
            if p.id.0.is_empty()
                || p.title.is_empty()
                || !offices.insert(&p.id)
                || content.country(&p.country).is_none()
                || !(1..=100).contains(&p.minimum_skill)
                || p.policies.is_empty()
                || p.policies.iter().any(|policy| {
                    content.policy(policy).is_none()
                        || policy == "coalition"
                        || !policies.insert((&p.country, policy))
                })
            {
                return Err("Invalid, duplicate or overlapping ministry jurisdiction".into());
            }
        }
        if let Some(identities) = &content.identities {
            for portfolio in &self.portfolios {
                if identities
                    .offices
                    .iter()
                    .find(|office| office.id == portfolio.id)
                    .is_some_and(|office| {
                        office.country != portfolio.country
                            || office.title != portfolio.title
                            || !matches!(office.role, OfficeRole::Ministry | OfficeRole::Cabinet)
                    })
                {
                    return Err(
                        "Portfolio conflicts with a reference office identity or protected role"
                            .into(),
                    );
                }
            }
            for candidate in &self.candidates {
                if identities
                    .people
                    .iter()
                    .find(|person| person.id == candidate.person)
                    .is_some_and(|person| person.name != candidate.name)
                {
                    return Err(
                        "Cabinet candidate conflicts with a reference person identity".into(),
                    );
                }
            }
        }
        if content.countries.iter().any(|c| {
            content
                .policies
                .iter()
                .filter(|p| p.id != "coalition")
                .any(|p| !policies.contains(&(&c.id, &p.id)))
        }) {
            return Err("Every implemented policy needs a portfolio in each country".into());
        }
        let mut people = BTreeSet::new();
        for candidate in &self.candidates {
            let office = self
                .office(&candidate.office)
                .ok_or("Candidate office is missing")?;
            if candidate.person.0.is_empty()
                || candidate.name.is_empty()
                || !people.insert(&candidate.person)
                || !(office.minimum_skill..=100).contains(&candidate.skill)
                || !(0..=100).contains(&candidate.acceptability)
            {
                return Err(
                    "Invalid cabinet candidate identity, qualification or acceptability".into(),
                );
            }
        }
        if self
            .portfolios
            .iter()
            .any(|office| !self.candidates.iter().any(|p| p.office == office.id))
        {
            return Err("Every portfolio needs an eligible candidate".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MinisterialTerm {
    pub appointment: AppointmentId,
    pub office: OfficeId,
    pub person: PersonId,
    pub since: crate::CampaignDay,
    pub until: Option<crate::CampaignDay>,
    pub government: crate::GovernmentId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingAppointment {
    pub id: AppointmentId,
    pub office: OfficeId,
    pub person: PersonId,
    pub proposed_day: crate::CampaignDay,
    pub decision_day: crate::CampaignDay,
    pub sponsor: crate::GovernmentId,
    pub paid: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cabinet {
    pub government: crate::GovernmentId,
    pub since: crate::CampaignDay,
    pub terms: Vec<MinisterialTerm>,
    pub pending: Vec<PendingAppointment>,
    pub next_request_day: BTreeMap<OfficeId, crate::CampaignDay>,
}
impl Cabinet {
    pub fn new(rules: &CabinetDefinition, id: &CountryId, day: crate::CampaignDay) -> Self {
        Self {
            government: crate::GovernmentId(1),
            since: day,
            terms: Vec::new(),
            pending: Vec::new(),
            next_request_day: rules
                .portfolios
                .iter()
                .filter(|p| &p.country == id)
                .map(|p| (p.id.clone(), day))
                .collect(),
        }
    }
    pub fn minister(&self, office: &OfficeId) -> Option<&MinisterialTerm> {
        self.terms
            .iter()
            .rev()
            .find(|term| &term.office == office && term.until.is_none())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppointmentPreview {
    pub fee: Money,
    pub decision_day: crate::CampaignDay,
    pub preparation_days: DaySpan,
    pub elite_cost: i32,
    pub council_cost: i32,
    pub route: String,
}
impl World {
    pub fn policy_preparation_days(
        &self,
        content: &Content,
        id: &CountryId,
        policy: &PolicyId,
    ) -> DaySpan {
        let Some(rules) = &content.cabinet else {
            return DaySpan::ZERO;
        };
        let Some(office) = rules
            .portfolios
            .iter()
            .find(|p| &p.country == id && p.policies.iter().any(|p| p == policy))
        else {
            return DaySpan::ZERO;
        };
        self.countries[id]
            .cabinet
            .as_ref()
            .and_then(|c| c.minister(&office.id))
            .and_then(|t| rules.candidate(&t.person))
            .map_or(rules.vacancy_preparation_days, |p| {
                rules
                    .vacancy_preparation_days
                    .scaled((100 - p.skill) as u32, 100)
                    .expect("validated candidate skill")
            })
    }
    pub fn preview_appointment(
        &self,
        content: &Content,
        id: &CountryId,
        office: &OfficeId,
        person: &PersonId,
    ) -> Result<AppointmentPreview, String> {
        let rules = content
            .cabinet
            .as_ref()
            .ok_or("This campaign has no ministerial appointment rules")?;
        let definition = rules
            .office(office)
            .filter(|p| &p.country == id)
            .ok_or("This portfolio does not belong to this government")?;
        let candidate = rules
            .candidate(person)
            .filter(|p| &p.office == office && p.skill >= definition.minimum_skill)
            .ok_or("Candidate is not eligible for this portfolio")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let cabinet = c.cabinet.as_ref().ok_or("Missing cabinet state")?;
        if cabinet.pending.iter().any(|p| &p.office == office) {
            return Err("This portfolio already has a pending appointment".into());
        }
        if cabinet
            .minister(office)
            .is_some_and(|t| &t.person == person)
        {
            return Err("This candidate already holds the portfolio".into());
        }
        if cabinet
            .next_request_day
            .get(office)
            .is_none_or(|day| *day > self.day)
        {
            return Err("This portfolio's appointment cooldown has not elapsed".into());
        }
        let government = self.governing_rules(content, id)?;
        government.check(c, "appointments", 1)?;
        let fee = Money(
            (content
                .country(id)
                .ok_or("Missing country definition")?
                .gdp
                .units()
                * rules.appointment_cost_bps.units_i64()
                / 10000)
                .max(1),
        );
        if fee.0 > c.available_cash().0 {
            return Err("Appointment administration requires uncommitted treasury".into());
        }
        let decision_day = self
            .day
            .units()
            .checked_add(government.consent_days.units())
            .filter(|d| *d <= MAX_DAY)
            .ok_or("Appointment would exceed the campaign calendar")?;
        self.day
            .units()
            .checked_add(rules.cooldown_days.units())
            .filter(|d| *d <= MAX_DAY)
            .ok_or("Appointment cooldown would exceed the campaign calendar")?;
        let cost = (100 - candidate.acceptability) / 10;
        Ok(AppointmentPreview {
            fee,
            decision_day: crate::CampaignDay::new(decision_day),
            preparation_days: rules
                .vacancy_preparation_days
                .scaled((100 - candidate.skill) as u32, 100)
                .expect("validated candidate skill"),
            elite_cost: cost,
            council_cost: cost,
            route: government.label(),
        })
    }
    pub(crate) fn request_appointment(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: AppointmentId,
        office: &OfficeId,
        person: &PersonId,
    ) -> Result<(), String> {
        let preview = self.preview_appointment(content, id, office, person)?;
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            preview.fee.0,
            "Ministerial appointment administration",
        );
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += preview.fee.0;
        let cabinet = c.cabinet.as_mut().expect("cabinet");
        cabinet.next_request_day.insert(
            office.clone(),
            crate::CampaignDay::new(
                self.day.units()
                    + content
                        .cabinet
                        .as_ref()
                        .expect("rules")
                        .cooldown_days
                        .units(),
            ),
        );
        cabinet.pending.push(PendingAppointment {
            id: request,
            office: office.clone(),
            person: person.clone(),
            proposed_day: self.day,
            decision_day: preview.decision_day,
            sponsor: c.government_number,
            paid: preview.fee,
        });
        self.event(id,true,format!("Appointment #{} proposed: {} to {}. Paid {}m administration; decision {}. Current authority and cabinet sponsorship will be rechecked. Withdrawal/refusal does not refund administration or remove the cooldown.",request.0,content.cabinet.as_ref().expect("rules").candidate(person).expect("candidate").name,content.cabinet.as_ref().expect("rules").office(office).expect("office").title,preview.fee.0,date(preview.decision_day)));
        self.resolve_appointments(content, id);
        Ok(())
    }
    pub(crate) fn cancel_appointment(
        &mut self,
        id: &CountryId,
        request: AppointmentId,
    ) -> Result<(), String> {
        let cabinet = self
            .countries
            .get_mut(id)
            .and_then(|c| c.cabinet.as_mut())
            .ok_or("No cabinet appointment workflow")?;
        if !cabinet.pending.iter().any(|p| p.id == request) {
            return Err("No such pending appointment for this government".into());
        }
        cabinet.pending.retain(|p| p.id != request);
        self.event(id,true,format!("Appointment #{} withdrawn. Administration remains spent and the portfolio cooldown remains in force.",request.0));
        Ok(())
    }
    pub fn preview_dismissal(
        &self,
        content: &Content,
        id: &CountryId,
        office: &OfficeId,
    ) -> Result<Money, String> {
        let rules = content
            .cabinet
            .as_ref()
            .ok_or("No cabinet appointment workflow")?;
        rules
            .office(office)
            .filter(|p| &p.country == id)
            .ok_or("This portfolio does not belong to this government")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let cabinet = c.cabinet.as_ref().ok_or("Missing cabinet")?;
        if cabinet.minister(office).is_none() || cabinet.pending.iter().any(|p| &p.office == office)
        {
            return Err("Dismissal needs an occupied portfolio with no pending replacement".into());
        }
        self.governing_rules(content, id)?
            .check(c, "appointments", 1)?;
        self.day
            .units()
            .checked_add(rules.cooldown_days.units())
            .filter(|day| *day <= MAX_DAY)
            .ok_or("Dismissal cooldown would exceed the campaign calendar")?;
        let fee = Money(
            (content.country(id).expect("country").gdp.units()
                * rules.dismissal_cost_bps.units_i64()
                / 10000)
                .max(1),
        );
        if fee.0 > c.available_cash().0 {
            return Err("Dismissal administration requires uncommitted treasury".into());
        }
        Ok(fee)
    }
    pub(crate) fn dismiss_minister(
        &mut self,
        content: &Content,
        id: &CountryId,
        office: &OfficeId,
    ) -> Result<(), String> {
        let fee = self.preview_dismissal(content, id, office)?;
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            fee.0,
            "Ministerial dismissal administration",
        );
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += fee.0;
        let cabinet = c.cabinet.as_mut().expect("cabinet");
        cabinet
            .terms
            .iter_mut()
            .filter(|t| &t.office == office && t.until.is_none())
            .for_each(|t| t.until = Some(self.day));
        cabinet.next_request_day.insert(
            office.clone(),
            crate::CampaignDay::new(
                self.day.units()
                    + content
                        .cabinet
                        .as_ref()
                        .expect("rules")
                        .cooldown_days
                        .units(),
            ),
        );
        self.event(id,true,format!("Minister dismissed from {}. Paid {}m administration. Permanent civil service now prepares new proposals with the configured vacancy delay; funded projects retain their delivery dates.",content.cabinet.as_ref().expect("rules").office(office).expect("office").title,fee.0));
        Ok(())
    }
    pub(crate) fn resolve_appointments(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.cabinet else {
            return;
        };
        let requests: Vec<_> = self.countries[id]
            .cabinet
            .as_ref()
            .expect("cabinet")
            .pending
            .iter()
            .filter(|p| p.decision_day <= self.day)
            .cloned()
            .collect();
        for p in requests {
            let c = &self.countries[id];
            let reason = if c.government_number != p.sponsor {
                Some("The sponsoring cabinet has changed".to_string())
            } else {
                self.governing_rules(content, id)
                    .expect("government")
                    .check(c, "appointments", 1)
                    .err()
            };
            let c = self.countries.get_mut(id).expect("country");
            let cabinet = c.cabinet.as_mut().expect("cabinet");
            cabinet.pending.retain(|r| r.id != p.id);
            if let Some(reason) = reason {
                self.event(
                    id,
                    true,
                    format!(
                        "Appointment #{} refused: {reason}. Paid administration remains spent.",
                        p.id.0
                    ),
                );
                continue;
            }
            cabinet
                .terms
                .iter_mut()
                .filter(|t| t.office == p.office && t.until.is_none())
                .for_each(|t| t.until = Some(self.day));
            cabinet.terms.push(MinisterialTerm {
                appointment: p.id,
                office: p.office.clone(),
                person: p.person.clone(),
                since: self.day,
                until: None,
                government: c.government_number,
            });
            let candidate = rules.candidate(&p.person).expect("candidate");
            let cost = (100 - candidate.acceptability) / 10;
            c.elite_support = (c.elite_support - cost).max(0);
            c.council_support = (c.council_support - cost).max(0);
            self.event(id,true,format!("Appointment #{} approved: {} now holds {}. Elite and council backing each fall {cost} points. Skill affects preparation time for new proposals only; existing projects and productive capacity do not change.",p.id.0,candidate.name,rules.office(&p.office).expect("office").title));
        }
    }
    pub(crate) fn cabinet_succession(&mut self, id: &CountryId) {
        let c = self.countries.get_mut(id).expect("country");
        let Some(cabinet) = &mut c.cabinet else {
            return;
        };
        if cabinet.government == c.government_number {
            return;
        }
        cabinet.government = c.government_number;
        cabinet.since = self.day;
        cabinet
            .terms
            .iter_mut()
            .filter(|t| t.until.is_none())
            .for_each(|t| t.until = Some(self.day));
        cabinet.pending.clear();
        for day in cabinet.next_request_day.values_mut() {
            *day = self.day;
        }
        self.event(id,true,"Successor cabinet ended prior ministerial terms and cancelled unapproved appointments. Administration remains spent; the new government may appoint its own ministers. Permanent civil service handles new preparation until appointments complete.");
    }
    pub(crate) fn ai_appointments(&mut self, content: &Content, id: &CountryId) {
        let Some(rules) = &content.cabinet else {
            return;
        };
        let state = self.countries[id].cabinet.as_ref().expect("cabinet");
        let mut candidates: Vec<_> = rules
            .candidates
            .iter()
            .filter(|p| {
                rules
                    .office(&p.office)
                    .is_some_and(|office| &office.country == id)
                    && state.minister(&p.office).is_none()
            })
            .collect();
        candidates.sort_by_key(|p| (std::cmp::Reverse(p.skill), p.person.clone()));
        for candidate in candidates {
            if self
                .preview_appointment(content, id, &candidate.office, &candidate.person)
                .is_ok()
            {
                let command = self.command(
                    id.clone(),
                    Action::Appoint {
                        office: candidate.office.clone(),
                        person: candidate.person.clone(),
                    },
                );
                if let Err(reason) =
                    self.apply(content, command, crate::developer::CommandOrigin::Ai)
                {
                    self.event(
                        id,
                        false,
                        format!("AI appointment declined by shared rules: {reason}"),
                    );
                }
                break;
            }
        }
    }
}
impl World {
    pub(crate) fn validate_cabinet(&self, content: &Content, id: &CountryId) -> Result<(), String> {
        let country = &self.countries[id];
        let (state, rules) = match (&country.cabinet, &content.cabinet) {
            (None, None) => return Ok(()),
            (Some(state), Some(rules)) => (state, rules),
            _ => return Err("Cabinet state differs from campaign rules/content".into()),
        };
        let start = content.start_day()?;
        let offices: BTreeSet<_> = rules
            .portfolios
            .iter()
            .filter(|p| &p.country == id)
            .map(|p| &p.id)
            .collect();
        if state.government != country.government_number
            || state.since < start
            || state.since > self.day
            || state.government == crate::GovernmentId(1) && state.since != start
            || state.next_request_day.keys().collect::<BTreeSet<_>>() != offices
            || state
                .next_request_day
                .values()
                .any(|d| *d < state.since || *d > crate::CampaignDay::new(MAX_DAY))
        {
            return Err("Invalid cabinet identity or portfolio clocks".into());
        }
        let appointment = |request: AppointmentId,
                           office: &OfficeId,
                           person: &PersonId|
         -> Result<&Command, String> {
            let command = request
                .0
                .checked_sub(1)
                .and_then(|index| self.journal.get(index as usize))
                .map(|entry| &entry.command)
                .ok_or("Appointment origin is missing")?;
            if &command.issuer != id
                || !matches!(&command.action,Action::Appoint {office:origin,person:nominee} if origin==office && nominee==person)
                || !offices.contains(office)
                || !rules.candidate(person).is_some_and(|p| &p.office == office)
            {
                return Err("Appointment origin, jurisdiction or eligibility mismatch".into());
            }
            Ok(command)
        };
        let mut ids = BTreeSet::new();
        let mut terms: BTreeMap<&OfficeId, Vec<&MinisterialTerm>> = BTreeMap::new();
        for term in &state.terms {
            let origin = appointment(term.appointment, &term.office, &term.person)?;
            let delay = self
                .governing_rules_at(content, id, origin.day)?
                .consent_days
                .units();
            if !ids.insert(term.appointment.0)
                || origin.day.checked_add(crate::DaySpan::new(delay)) != Some(term.since)
                || term.since > self.day
                || term
                    .until
                    .is_some_and(|day| day < term.since || day > self.day)
                || term.government == crate::GovernmentId(0)
                || term.government > state.government
                || term.until.is_none()
                    && (term.government != state.government || term.since < state.since)
            {
                return Err("Invalid ministerial term dates or cabinet sponsorship".into());
            }
            terms.entry(&term.office).or_default().push(term);
        }
        for terms in terms.values() {
            if terms.windows(2).any(|pair| {
                pair[0].since > pair[1].since || pair[0].until.is_none_or(|day| day > pair[1].since)
            }) {
                return Err("Overlapping ministerial terms".into());
            }
        }
        let fee = Money(
            (content.country(id).expect("country").gdp.units()
                * rules.appointment_cost_bps.units_i64()
                / 10000)
                .max(1),
        );
        let mut pending = BTreeSet::new();
        for p in &state.pending {
            let origin = appointment(p.id, &p.office, &p.person)?;
            let delay = self
                .governing_rules_at(content, id, origin.day)?
                .consent_days
                .units();
            if !ids.insert(p.id.0)
                || !pending.insert(&p.office)
                || p.proposed_day != origin.day
                || origin.day.checked_add(crate::DaySpan::new(delay)) != Some(p.decision_day)
                || p.decision_day <= self.day
                || p.decision_day > crate::CampaignDay::new(MAX_DAY)
                || p.sponsor != state.government
                || p.proposed_day < state.since
                || p.paid != fee
                || state.next_request_day[&p.office]
                    != crate::CampaignDay::new(
                        (p.proposed_day).units() + rules.cooldown_days.units(),
                    )
            {
                return Err("Invalid pending ministerial appointment".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedMinister {
    pub office: OfficeId,
    pub title: String,
    pub holder: Option<String>,
}
impl World {
    pub(crate) fn published_ministers(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Option<Vec<PublishedMinister>> {
        let rules = content.cabinet.as_ref()?;
        let state = self.countries.get(id)?.cabinet.as_ref()?;
        Some(
            rules
                .portfolios
                .iter()
                .filter(|p| &p.country == id)
                .map(|office| PublishedMinister {
                    office: office.id.clone(),
                    title: office.title.clone(),
                    holder: state
                        .minister(&office.id)
                        .and_then(|term| rules.candidate(&term.person))
                        .map(|p| p.name.clone()),
                })
                .collect(),
        )
    }
}
