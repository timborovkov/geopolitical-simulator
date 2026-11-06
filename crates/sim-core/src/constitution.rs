//! Optional rules-5 constitutional transitions. These authored political routes
//! are distinct from the bookmark's reference regime and from developer edits.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct TransitionId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegimeTemplate {
    pub regime: Regime,
    pub title: String,
    pub government: GovernmentDefinition,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransitionRoute {
    pub days: DaySpan,
    pub cost_bps: i64,
    pub upfront_bps: i64,
    pub legislative_threshold: i32,
    pub elite_threshold: i32,
    pub council_threshold: i32,
    pub approval_threshold: i32,
    pub approval_cost: i32,
    pub legislative_cost: i32,
    pub elite_cost: i32,
    pub council_cost: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstitutionDefinition {
    pub templates: Vec<RegimeTemplate>,
    pub lawful: TransitionRoute,
    pub contested: Option<TransitionRoute>,
    pub cooldown_days: DaySpan,
    /// Fictional scenario preferences, not a universal historical trajectory.
    pub ai_targets: BTreeMap<CountryId, Regime>,
    pub source: String,
    pub source_version: String,
}
impl ConstitutionDefinition {
    pub fn template(&self, regime: &Regime) -> Option<&RegimeTemplate> {
        self.templates
            .iter()
            .find(|template| &template.regime == regime)
    }
    pub fn route(&self, contested: bool) -> Result<&TransitionRoute, String> {
        if contested {
            self.contested
                .as_ref()
                .ok_or("Contested constitutional change is unavailable".into())
        } else {
            Ok(&self.lawful)
        }
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if self.templates.len() != 8
            || self.source_version.is_empty()
            || !content
                .sources
                .iter()
                .any(|source| source.id == self.source)
            || !(30..=730).contains(&self.cooldown_days.units())
            || self
                .ai_targets
                .keys()
                .any(|id| content.country(id).is_none())
        {
            return Err("Constitutions need eight unique templates, bounded inertia and valid provenance/AI targets".into());
        }
        for (index, template) in self.templates.iter().enumerate() {
            let rules = &template.government;
            if template.title.is_empty()
                || self.templates[..index]
                    .iter()
                    .any(|other| other.regime == template.regime)
                || !(1..=100).contains(&rules.threshold)
                || !(0..=100).contains(&rules.council_threshold)
                || rules.consent_days.units() > 180
                || rules
                    .leadership_review_days
                    .is_some_and(|days| !(30..=1827).contains(&days.units()))
                || template.regime.competitive() != (rules.route == DecisionRoute::Legislature)
            {
                return Err("Invalid constitution template or governing route".into());
            }
        }
        for route in std::iter::once(&self.lawful).chain(self.contested.iter()) {
            if !(1..=365).contains(&route.days.units())
                || !(1..=1000).contains(&route.cost_bps)
                || !(1..=10000).contains(&route.upfront_bps)
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
                .any(|value| !(0..=100).contains(value))
            {
                return Err("Invalid constitutional consent, costs or backing constraints".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstitutionChange {
    pub request: TransitionId,
    pub from: Regime,
    pub to: Regime,
    pub day: u32,
    pub contested: bool,
    pub government_number: crate::GovernmentId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstitutionalRequest {
    pub id: TransitionId,
    pub from: Regime,
    pub target: Regime,
    pub contested: bool,
    pub proposed_day: u32,
    pub decision_day: u32,
    pub sponsor_government: crate::GovernmentId,
    pub cost: Money,
    pub upfront: Money,
    pub reserved: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Constitution {
    pub regime: Regime,
    pub since: u32,
    pub history: Vec<ConstitutionChange>,
    pub pending: Option<ConstitutionalRequest>,
    pub next_request_day: u32,
}
impl Constitution {
    pub fn new(regime: Regime, day: u32) -> Self {
        Self {
            regime,
            since: day,
            history: Vec::new(),
            pending: None,
            next_request_day: day,
        }
    }
    pub fn reserved_cash(&self) -> Money {
        self.pending
            .as_ref()
            .map_or(Money(0), |request| request.reserved)
    }
    pub(crate) fn validate(
        &self,
        world: &World,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let rules = content
            .constitutions
            .as_ref()
            .ok_or("Missing constitution definitions")?;
        let definition = content.country(id).ok_or("Missing constitution country")?;
        let country = &world.countries[id];
        let start = content.start_day()?;
        let mut current = definition.regime.clone();
        let mut previous_day = start;
        let mut previous_government = crate::GovernmentId(1);
        let mut ids = BTreeSet::new();
        if self.history.len() > world.journal.len() {
            return Err("Invalid constitution history length".into());
        }
        for change in &self.history {
            let command = transition_command(world, id, change.request)?;
            let (target, contested) = match &command.action {
                Action::Reconstitute { target, contested } => (target, *contested),
                _ => return Err("Constitution change lacks a proposal command".into()),
            };
            if !ids.insert(change.request)
                || change.from != current
                || target != &change.to
                || contested != change.contested
                || change.to == current
                || rules.template(target).is_none()
                || command
                    .day
                    .checked_add(rules.route(contested)?.days.units())
                    != Some(change.day)
                || change.day < previous_day
                || change.day > world.day
                || change.government_number <= previous_government
                || change.government_number > country.government_number
            {
                return Err("Constitution history differs from its chronology, government or command origin".into());
            }
            current = change.to.clone();
            previous_day = change.day;
            previous_government = change.government_number;
        }
        if current != self.regime || self.since != previous_day {
            return Err("Current constitution differs from its accepted history".into());
        }
        if let Some(request) = &self.pending {
            let command = transition_command(world, id, request.id)?;
            let route = rules.route(request.contested)?;
            let cost = (definition.gdp * route.cost_bps / 10000).max(1);
            let upfront = (cost * route.upfront_bps / 10000).max(1);
            if !matches!(&command.action, Action::Reconstitute { target, contested } if target == &request.target && *contested == request.contested)
                || request.from != self.regime
                || request.target == self.regime
                || rules.template(&request.target).is_none()
                || request.proposed_day != command.day
                || command.day.checked_add(route.days.units()) != Some(request.decision_day)
                || request.decision_day <= world.day
                || request.decision_day > MAX_DAY
                || request.sponsor_government == crate::GovernmentId(0)
                || request.sponsor_government > country.government_number
                || request.cost != Money(cost)
                || request.upfront != Money(upfront)
                || request.reserved != Money(cost - upfront)
                || ids.contains(&request.id)
            {
                return Err("Invalid pending constitutional request or reservation".into());
            }
        }
        let last_request = world.journal.iter().rev().find(|entry| {
            &entry.command.issuer == id
                && matches!(entry.command.action, Action::Reconstitute { .. })
        });
        let expected = last_request.map_or(Ok(start), |entry| {
            entry
                .command
                .day
                .checked_add(rules.cooldown_days.units())
                .ok_or("Constitution cooldown overflow")
        })?;
        if self.next_request_day != expected {
            return Err("Constitutional inertia differs from accepted requests".into());
        }
        Ok(())
    }
}
fn transition_command<'a>(
    world: &'a World,
    issuer: &CountryId,
    id: TransitionId,
) -> Result<&'a Command, String> {
    let entry =
        id.0.checked_sub(1)
            .and_then(|index| world.journal.get(index as usize))
            .ok_or("Missing constitutional command")?;
    if &entry.command.issuer != issuer {
        return Err("Constitution command has another issuer".into());
    }
    Ok(&entry.command)
}

#[derive(Clone, Debug)]
pub struct ConstitutionalPreview {
    pub target: Regime,
    pub contested: bool,
    pub decision_day: u32,
    pub cost: Money,
    pub upfront: Money,
    pub reserved: Money,
    pub political_costs: TransitionRoute,
}
impl World {
    pub fn governing_regime<'a>(
        &'a self,
        content: &'a Content,
        id: &CountryId,
    ) -> Result<&'a Regime, String> {
        let country = self.countries.get(id).ok_or("Unknown governing country")?;
        Ok(country.constitution.as_ref().map_or(
            &content
                .country(id)
                .ok_or("Missing governing definition")?
                .regime,
            |state| &state.regime,
        ))
    }
    pub fn governing_rules<'a>(
        &'a self,
        content: &'a Content,
        id: &CountryId,
    ) -> Result<&'a GovernmentDefinition, String> {
        self.governing_rules_at(content, id, self.day)
    }
    pub(crate) fn governing_rules_at<'a>(
        &'a self,
        content: &'a Content,
        id: &CountryId,
        day: u32,
    ) -> Result<&'a GovernmentDefinition, String> {
        let country = self.countries.get(id).ok_or("Unknown governing country")?;
        if let Some(change) = country
            .constitution
            .as_ref()
            .and_then(|state| state.history.iter().rev().find(|change| change.day <= day))
        {
            return Ok(&content
                .constitutions
                .as_ref()
                .and_then(|rules| rules.template(&change.to))
                .ok_or("Missing active constitution template")?
                .government);
        }
        Ok(&content
            .country(id)
            .ok_or("Missing governing definition")?
            .government)
    }
    fn check_constitution_backing(
        &self,
        content: &Content,
        id: &CountryId,
        contested: bool,
    ) -> Result<(), String> {
        let country = self
            .countries
            .get(id)
            .ok_or("Unknown constitutional country")?;
        let route = content
            .constitutions
            .as_ref()
            .ok_or("This campaign has no constitutional transition workflow")?
            .route(contested)?;
        let government = self.governing_rules(content, id)?;
        if !contested {
            government.check(country, "constitution", 0)?;
        }
        let backing = if !contested && government.route == DecisionRoute::Legislature {
            country.legislative_support
        } else {
            country.elite_support
        };
        let needed = if !contested && government.route == DecisionRoute::Legislature {
            route.legislative_threshold
        } else {
            route.elite_threshold
        };
        if backing < needed
            || country.council_support < route.council_threshold
            || country.approval < route.approval_threshold
        {
            return Err(format!(
                "{} constitutional change requires backing {needed}, council {}, approval {}; current {backing}/{}/{}",
                if contested {
                    "Contested effective-power route"
                } else {
                    "Lawful"
                },
                route.council_threshold,
                route.approval_threshold,
                country.council_support,
                country.approval
            ));
        }
        Ok(())
    }
    pub fn preview_constitution(
        &self,
        content: &Content,
        id: &CountryId,
        target: &Regime,
        contested: bool,
    ) -> Result<ConstitutionalPreview, String> {
        if self.content_hash != content.hash() {
            return Err("Constitution preview needs the campaign's exact content".into());
        }
        let rules = content
            .constitutions
            .as_ref()
            .ok_or("This campaign has no constitutional transition workflow")?;
        let country = self
            .countries
            .get(id)
            .ok_or("Unknown constitutional country")?;
        let state = country
            .constitution
            .as_ref()
            .ok_or("Missing current constitution")?;
        if state.pending.is_some() {
            return Err("A constitutional decision is already pending".into());
        }
        if self.day < state.next_request_day {
            return Err(format!(
                "Constitutional inertia: next request after {}",
                date(state.next_request_day)
            ));
        }
        if target == &state.regime || rules.template(target).is_none() {
            return Err("Choose a different configured constitution".into());
        }
        self.check_constitution_backing(content, id, contested)?;
        let route = rules.route(contested)?;
        let decision_day = self
            .day
            .checked_add(route.days.units())
            .filter(|day| *day <= MAX_DAY)
            .ok_or("No time remains for the constitutional decision")?;
        let cost = Money(
            (content.country(id).ok_or("Missing country")?.gdp * route.cost_bps / 10000).max(1),
        );
        if country.available_cash().0 < cost.0 {
            return Err(format!(
                "Constitutional transition requires {}m of uncommitted cash",
                cost.0
            ));
        }
        let upfront = Money((cost.0 * route.upfront_bps / 10000).max(1));
        Ok(ConstitutionalPreview {
            target: target.clone(),
            contested,
            decision_day,
            cost,
            upfront,
            reserved: Money(cost.0 - upfront.0),
            political_costs: route.clone(),
        })
    }
    pub(crate) fn request_constitution(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: TransitionId,
        target: &Regime,
        contested: bool,
    ) -> Result<(), String> {
        let preview = self.preview_constitution(content, id, target, contested)?;
        let next_request_day = self
            .day
            .checked_add(
                content
                    .constitutions
                    .as_ref()
                    .expect("rules")
                    .cooldown_days
                    .units(),
            )
            .ok_or("Constitution cooldown overflow")?;
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            preview.upfront.0,
            "Constitutional proposal administration",
        );
        let country = self.countries.get_mut(id).expect("country");
        country.budget.capital.0 += preview.upfront.0;
        country.committed_cash.0 += preview.reserved.0;
        let state = country.constitution.as_mut().expect("constitution");
        state.pending = Some(ConstitutionalRequest {
            id: request,
            from: state.regime.clone(),
            target: target.clone(),
            contested,
            proposed_day: self.day,
            decision_day: preview.decision_day,
            sponsor_government: country.government_number,
            cost: preview.cost,
            upfront: preview.upfront,
            reserved: preview.reserved,
        });
        state.next_request_day = next_request_day;
        self.event(id, true, format!("{} constitutional request #{}: {:?}. Paid {}m, reserved {}m. Decision {}; backing and sponsorship will be rechecked. Political costs apply if enacted. Cancellation/refusal retains administration costs and constitutional cooldown.", if contested { "CONTESTED" } else { "Lawful" }, request.0, target, preview.upfront.0, preview.reserved.0, date(preview.decision_day)));
        Ok(())
    }
    pub(crate) fn cancel_constitution(
        &mut self,
        id: &CountryId,
        request: TransitionId,
    ) -> Result<(), String> {
        let country = self
            .countries
            .get_mut(id)
            .ok_or("Unknown constitutional country")?;
        let state = country
            .constitution
            .as_mut()
            .ok_or("This campaign has no constitutional transition workflow")?;
        if state
            .pending
            .as_ref()
            .is_none_or(|pending| pending.id != request)
        {
            return Err("No matching pending constitutional request".into());
        }
        let pending = state.pending.take().expect("matching request");
        country.committed_cash.0 -= pending.reserved.0;
        self.event(id, true, format!("Withdrew constitutional request #{}. Released {}m of reserved cash; {}m administration cost remains spent. Constitutional cooldown remains in force.", request.0, pending.reserved.0, pending.upfront.0));
        Ok(())
    }
    pub(crate) fn resolve_constitution(&mut self, content: &Content, id: &CountryId) {
        let request = self.countries[id]
            .constitution
            .as_ref()
            .and_then(|state| state.pending.clone());
        let Some(request) = request.filter(|request| request.decision_day <= self.day) else {
            return;
        };
        let result = if self.countries[id].government_number != request.sponsor_government {
            Err("Sponsoring cabinet changed; its constitutional mandate expired".into())
        } else {
            self.check_constitution_backing(content, id, request.contested)
                .and_then(|()| {
                    self.countries[id]
                        .government_number
                        .checked_add(1)
                        .ok_or("Government number exhausted".into())
                })
        };
        let country = self.countries.get_mut(id).expect("country");
        country.committed_cash.0 -= request.reserved.0;
        country.constitution.as_mut().expect("constitution").pending = None;
        let next_government = match result {
            Ok(number) => number,
            Err(reason) => {
                self.event(id, true, format!("Constitutional request #{} refused at decision: {reason}. Released {}m reserved cash; administration costs remain spent.", request.id.0, request.reserved.0));
                return;
            }
        };
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            request.reserved.0,
            "Approved constitutional transition administration",
        );
        let rules = content.constitutions.as_ref().expect("constitution rules");
        let template = rules.template(&request.target).expect("target template");
        let costs = rules.route(request.contested).expect("route");
        let country = self.countries.get_mut(id).expect("country");
        country.budget.capital.0 += request.reserved.0;
        country.government_number = next_government;
        country.confidence_crises = 0;
        country.next_election = request.target.competitive().then_some(self.day + 365);
        country.next_leadership_review = template
            .government
            .leadership_review_days
            .map(|days| self.day + days.units());
        country.executive = format!(
            "Generated transitional {} #{}",
            template.title, next_government
        );
        country.approval = (country.approval - costs.approval_cost).max(0);
        for household in &mut country.households {
            household.approval = (household.approval - costs.approval_cost).max(0);
        }
        country.legislative_support = (country.legislative_support - costs.legislative_cost).max(0);
        country.elite_support = (country.elite_support - costs.elite_cost).max(0);
        country.council_support = (country.council_support - costs.council_cost).max(0);
        let state = country.constitution.as_mut().expect("constitution");
        state.regime = request.target.clone();
        state.since = self.day;
        state.history.push(ConstitutionChange {
            request: request.id,
            from: request.from.clone(),
            to: request.target.clone(),
            day: self.day,
            contested: request.contested,
            government_number: next_government,
        });
        let cancelled_loan = country
            .financing
            .as_mut()
            .and_then(|state| state.pending.take());
        self.event(id, true, format!("{} constitution enacted: {:?} to {:?}. Government #{} uses {}. Approval -{}, legislature -{}, elite -{}, council -{} (floor zero). Existing resource accounts, debt and funded programs continue; country play continues.", if request.contested { "CONTESTED" } else { "Lawful" }, request.from, request.target, next_government, template.government.label(), costs.approval_cost, costs.legislative_cost, costs.elite_cost, costs.council_cost));
        if let Some(loan) = cancelled_loan {
            self.event(id, true, format!("Pending loan #{} cancelled after constitutional succession. No principal was disbursed; the new government must seek its own borrowing mandate.", loan.id.0));
        }
    }
}
