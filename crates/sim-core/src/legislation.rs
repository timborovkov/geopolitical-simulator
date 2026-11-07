//! Authored statutory permission, with paid drafting, consent and implementation.
//! This is an institutional prototype, not a historical legal-text simulation.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct LawId(pub String);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct LawBillId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LawDefinition {
    pub id: LawId,
    pub name: String,
    pub description: String,
    pub available_from: CivilDate,
    pub eligible_countries: Vec<CountryId>,
    pub source: String,
    pub source_version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyLawRequirement {
    pub policy: PolicyId,
    pub minimum_value: i32,
    pub laws: Vec<LawId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegislationDefinition {
    pub definitions: Vec<LawDefinition>,
    pub policy_requirements: Vec<PolicyLawRequirement>,
    pub cost_bps: BasisPoints,
    pub upfront_bps: BasisPoints,
    pub implementation_days: DaySpan,
    pub cooldown_days: DaySpan,
    pub ai_targets: BTreeMap<CountryId, Vec<LawId>>,
}
impl LegislationDefinition {
    pub fn law(&self, id: &LawId) -> Option<&LawDefinition> {
        self.definitions.iter().find(|law| &law.id == id)
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=128).contains(&self.definitions.len())
            || !(1..=1000).contains(&self.cost_bps.units())
            || !(1..=10000).contains(&self.upfront_bps.units())
            || !(1..=365).contains(&self.implementation_days.units())
            || !(1..=365).contains(&self.cooldown_days.units())
            || self.policy_requirements.is_empty()
            || self.policy_requirements.len() > content.policies.len()
        {
            return Err(
                "Legislation needs bounded definitions, costs, periods and programme requirements"
                    .into(),
            );
        }
        let mut ids = BTreeSet::new();
        for law in &self.definitions {
            let countries: BTreeSet<_> = law.eligible_countries.iter().collect();
            if law.id.0.is_empty()
                || law.id.0.len() > 64
                || !law
                    .id
                    .0
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
                || !ids.insert(&law.id)
                || law.name.trim().is_empty()
                || law.name.len() > 128
                || law.description.trim().is_empty()
                || law.description.len() > 2048
                || law.available_from > CivilDate::new(2100, 1, 1)?
                || countries.is_empty()
                || countries.len() != law.eligible_countries.len()
                || countries.iter().any(|id| content.country(id).is_none())
                || !content.sources.iter().any(|s| s.id == law.source)
                || law.source_version.trim().is_empty()
                || law.source_version.len() > 128
            {
                return Err(
                    "Invalid law identity, enactment date, country scope or provenance".into(),
                );
            }
        }
        let mut policies = BTreeSet::new();
        for requirement in &self.policy_requirements {
            let policy = content
                .policy(&requirement.policy)
                .ok_or("Unknown statutory programme")?;
            let required: BTreeSet<_> = requirement.laws.iter().collect();
            if !policies.insert(&requirement.policy)
                || !(policy.min..=policy.max).contains(&requirement.minimum_value)
                || !(1..=8).contains(&required.len())
                || required.len() != requirement.laws.len()
                || required.iter().any(|id| !ids.contains(id))
            {
                return Err("Invalid or duplicate programme/law requirement".into());
            }
        }
        for (country, targets) in &self.ai_targets {
            let unique: BTreeSet<_> = targets.iter().collect();
            if content.country(country).is_none()
                || !(1..=8).contains(&targets.len())
                || unique.len() != targets.len()
                || targets.iter().any(|id| {
                    self.law(id)
                        .is_none_or(|law| !law.eligible_countries.contains(country))
                })
            {
                return Err("Invalid legislation AI target".into());
            }
        }
        Ok(())
    }
    fn costs(&self, gdp: AnnualGdp) -> Result<(Money, Money, Money), String> {
        let total = Money(
            self.cost_bps
                .scale(Money(gdp.units()))
                .ok_or("Law cost overflow")?
                .0
                .max(1),
        );
        let upfront = Money(
            self.upfront_bps
                .scale(total)
                .ok_or("Law drafting cost overflow")?
                .0
                .max(1),
        );
        let reserved = Money(
            total
                .0
                .checked_sub(upfront.0)
                .ok_or("Invalid law cost split")?,
        );
        Ok((total, upfront, reserved))
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LawBillStatus {
    AwaitingConsent,
    Implementing,
    Enacted,
    Refused,
    Withdrawn,
    Superseded,
}
impl LawBillStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::AwaitingConsent => "Awaiting consent",
            Self::Implementing => "Implementing",
            Self::Enacted => "Enacted",
            Self::Refused => "Refused",
            Self::Withdrawn => "Withdrawn",
            Self::Superseded => "Superseded",
        }
    }
    pub fn pending(self) -> bool {
        matches!(self, Self::AwaitingConsent | Self::Implementing)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LawBill {
    pub id: LawBillId,
    pub law: LawId,
    pub sponsor: GovernmentId,
    pub proposed_day: CampaignDay,
    pub review_day: CampaignDay,
    pub enactment_day: CampaignDay,
    pub authority: GovernmentDefinition,
    pub total_cost: Money,
    pub upfront: Money,
    pub paid: Money,
    pub reserved: Money,
    pub status: LawBillStatus,
    pub closed_day: Option<CampaignDay>,
    pub explanation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnactedLaw {
    pub authority: GovernmentDefinition,
    pub paid: Money,
    pub bill: LawBillId,
    pub government: GovernmentId,
    pub day: CampaignDay,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegislationState {
    pub enacted: BTreeMap<LawId, EnactedLaw>,
    pub bills: Vec<LawBill>,
    pub next_request: BTreeMap<LawId, CampaignDay>,
}
impl LegislationState {
    pub fn reserved_cash(&self) -> Money {
        Money(self.bills.iter().map(|bill| bill.reserved.0).sum())
    }
    pub fn bill(&self, id: LawBillId) -> Option<&LawBill> {
        self.bills.iter().find(|bill| bill.id == id)
    }
    fn prune(&mut self) {
        while self.bills.iter().filter(|b| !b.status.pending()).count() > 128 {
            let index = self
                .bills
                .iter()
                .position(|b| !b.status.pending())
                .expect("closed bill");
            self.bills.remove(index);
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LawPreview {
    pub law: LawId,
    pub total_cost: Money,
    pub upfront: Money,
    pub reserved: Money,
    pub review_day: CampaignDay,
    pub enactment_day: CampaignDay,
    pub authority: GovernmentDefinition,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LawEligibility {
    pub law: LawId,
    pub name: String,
    pub enacted: bool,
}
impl World {
    pub fn policy_law_eligibility(
        &self,
        content: &Content,
        country: &CountryId,
        policy: &PolicyId,
        value: i32,
    ) -> Result<Vec<LawEligibility>, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Law eligibility requires the campaign's exact content".into());
        }
        let c = self.countries.get(country).ok_or("Unknown law country")?;
        let p = content
            .policy(policy)
            .ok_or("Unknown statutory programme")?;
        if !(p.min..=p.max).contains(&value) {
            return Err("Programme target is outside its defined range".into());
        }
        let Some(rules) = &content.legislation else {
            return Ok(Vec::new());
        };
        let state = c.legislation.as_ref().ok_or("Missing legislation state")?;
        Ok(rules
            .policy_requirements
            .iter()
            .filter(|r| &r.policy == policy && value >= r.minimum_value)
            .flat_map(|r| &r.laws)
            .map(|id| LawEligibility {
                law: id.clone(),
                name: rules.law(id).expect("validated law").name.clone(),
                enacted: state.enacted.contains_key(id),
            })
            .collect())
    }
    pub(crate) fn check_policy_laws(
        &self,
        content: &Content,
        country: &CountryId,
        policy: &PolicyId,
        value: i32,
    ) -> Result<(), String> {
        self.check_policy_laws_at(content, country, policy, value, self.day)
    }
    pub(crate) fn check_policy_laws_at(
        &self,
        content: &Content,
        country: &CountryId,
        policy: &PolicyId,
        value: i32,
        day: CampaignDay,
    ) -> Result<(), String> {
        let Some(rules) = &content.legislation else {
            return Ok(());
        };
        let state = self
            .countries
            .get(country)
            .and_then(|c| c.legislation.as_ref())
            .ok_or("Missing legislation state")?;
        for requirement in rules
            .policy_requirements
            .iter()
            .filter(|r| &r.policy == policy && value >= r.minimum_value)
        {
            for law in &requirement.laws {
                if state.enacted.get(law).is_none_or(|entry| entry.day > day) {
                    return Err(format!(
                        "{} must be enacted before commissioning this programme target; pending consent grants no statutory permission",
                        rules.law(law).ok_or("Unknown statutory prerequisite")?.name
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn preview_law(
        &self,
        content: &Content,
        country: &CountryId,
        law: &LawId,
    ) -> Result<LawPreview, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Law proposal requires the campaign's exact content".into());
        }
        let rules = content
            .legislation
            .as_ref()
            .ok_or("This scenario has no statutory workflow")?;
        let definition = rules.law(law).ok_or("Unknown law")?;
        if !definition.eligible_countries.contains(country) {
            return Err(
                "This statute is unavailable to that country in the loaded scenario".into(),
            );
        }
        let today: CivilDate = date(self.day).parse()?;
        if today < definition.available_from {
            return Err(format!(
                "This statutory proposal becomes available on {}",
                definition.available_from
            ));
        }
        let c = self.countries.get(country).ok_or("Unknown law country")?;
        let state = c.legislation.as_ref().ok_or("Missing legislation state")?;
        if state.enacted.contains_key(law) {
            return Err("This statute is already enacted".into());
        }
        if state.bills.iter().any(|bill| bill.status.pending()) {
            return Err("Complete or withdraw the government's pending bill first".into());
        }
        if state
            .next_request
            .get(law)
            .is_some_and(|day| self.day < *day)
        {
            return Err("The statute's drafting cooldown remains in force".into());
        }
        let authority = self.governing_rules(content, country)?.clone();
        authority.check(c, "legislation", 1)?;
        let review_day = self
            .day
            .checked_add(authority.consent_days)
            .filter(|d| d.units() <= MAX_DAY)
            .ok_or("Statutory consent exceeds the campaign calendar")?;
        let enactment_day = review_day
            .checked_add(rules.implementation_days)
            .filter(|d| d.units() <= MAX_DAY)
            .ok_or("Statutory implementation exceeds the campaign calendar")?;
        self.day
            .checked_add(rules.cooldown_days)
            .filter(|d| d.units() <= MAX_DAY)
            .ok_or("Statutory drafting cooldown exceeds the campaign calendar")?;
        let (total_cost, upfront, reserved) = rules.costs(
            content
                .country(country)
                .ok_or("Missing law country definition")?
                .gdp,
        )?;
        if c.available_cash().0 < total_cost.0 {
            return Err(format!(
                "Statute needs {}m of actual uncommitted cash: {}m drafting and {}m protected implementation funds; future receipts and pending loans are unavailable",
                total_cost.0, upfront.0, reserved.0
            ));
        }
        Ok(LawPreview {
            law: law.clone(),
            total_cost,
            upfront,
            reserved,
            review_day,
            enactment_day,
            authority,
        })
    }
    pub(crate) fn propose_law(
        &mut self,
        content: &Content,
        country: &CountryId,
        id: LawBillId,
        law: &LawId,
    ) -> Result<(), String> {
        let p = self.preview_law(content, country, law)?;
        let paid = self.pay(
            country,
            Account::Treasury,
            Account::Firms,
            p.upfront.0,
            "Statutory drafting administration",
        );
        if paid != p.upfront.0 {
            return Err("Statutory drafting could not be fully funded".into());
        }
        let c = self.countries.get_mut(country).expect("country");
        c.budget.capital.0 += paid;
        c.committed_cash.0 += p.reserved.0;
        let state = c.legislation.as_mut().expect("legislation");
        state.next_request.insert(
            law.clone(),
            self.day
                .checked_add(content.legislation.as_ref().expect("rules").cooldown_days)
                .expect("previewed cooldown"),
        );
        state.bills.push(LawBill {
            id,
            law: law.clone(),
            sponsor: c.government_number,
            proposed_day: self.day,
            review_day: p.review_day,
            enactment_day: p.enactment_day,
            authority: p.authority,
            total_cost: p.total_cost,
            upfront: p.upfront,
            paid: p.upfront,
            reserved: p.reserved,
            status: LawBillStatus::AwaitingConsent,
            closed_day: None,
            explanation: "Paid drafting; awaiting governing consent".into(),
        });
        state.prune();
        self.event(country, true, format!("Statutory bill #{} proposed: {}. Paid {}m drafting; reserved {}m actual implementation cash. Consent {}; earliest enactment {}. Acceptance grants no legal permission, services or capacity.",id.0,content.legislation.as_ref().expect("rules").law(law).expect("law").name,p.upfront.0,p.reserved.0,date(p.review_day),date(p.enactment_day)));
        self.resolve_legislation(content, country);
        Ok(())
    }
    fn close_law_bill(
        &mut self,
        country: &CountryId,
        id: LawBillId,
        status: LawBillStatus,
        explanation: String,
    ) {
        let c = self.countries.get_mut(country).expect("country");
        let state = c.legislation.as_mut().expect("legislation");
        let bill = state.bills.iter_mut().find(|b| b.id == id).expect("bill");
        let released = bill.reserved;
        c.committed_cash.0 -= released.0;
        bill.reserved = Money(0);
        bill.status = status;
        bill.closed_day = Some(self.day);
        bill.explanation = explanation.clone();
        state.prune();
        self.event(country, true, format!("Statutory bill #{} {}: {}. Released {}m unspent reservation; paid drafting/implementation remains spent.",id.0,status.label(),explanation,released.0));
    }
    pub(crate) fn withdraw_law(
        &mut self,
        country: &CountryId,
        request: LawBillId,
    ) -> Result<(), String> {
        let c = self.countries.get(country).ok_or("Unknown law country")?;
        let bill = c
            .legislation
            .as_ref()
            .and_then(|s| s.bill(request))
            .ok_or("No such retained bill for this government")?;
        if !bill.status.pending() || bill.sponsor != c.government_number {
            return Err(
                "Only a pending bill sponsored by the current government can be withdrawn".into(),
            );
        }
        self.close_law_bill(
            country,
            request,
            LawBillStatus::Withdrawn,
            "Withdrawn by the sponsoring government; drafting cooldown remains".into(),
        );
        Ok(())
    }
    pub(crate) fn resolve_legislation(&mut self, content: &Content, country: &CountryId) {
        let Some(state) = self.countries[country].legislation.as_ref() else {
            return;
        };
        let pending: Vec<_> = state
            .bills
            .iter()
            .filter(|b| b.status.pending())
            .cloned()
            .collect();
        for bill in pending {
            if bill.sponsor != self.countries[country].government_number {
                self.close_law_bill(country, bill.id, LawBillStatus::Superseded, "Sponsoring government changed before enactment; adopted statutes remain in force".into());
                continue;
            }
            if bill.status == LawBillStatus::AwaitingConsent && self.day >= bill.review_day {
                let check = self
                    .governing_rules(content, country)
                    .and_then(|authority| {
                        authority.check(&self.countries[country], "legislation", 1)
                    });
                if let Err(reason) = check {
                    self.close_law_bill(country, bill.id, LawBillStatus::Refused, reason);
                    continue;
                }
                let record = self
                    .countries
                    .get_mut(country)
                    .expect("country")
                    .legislation
                    .as_mut()
                    .expect("legislation")
                    .bills
                    .iter_mut()
                    .find(|b| b.id == bill.id)
                    .expect("bill");
                record.status = LawBillStatus::Implementing;
                record.explanation =
                    "Consent obtained; funded statutory implementation is pending".into();
                self.event(country,true,format!("Statutory bill #{} received governing consent. {}m remains protected for implementation on {}; no statutory permission is granted yet.",bill.id.0,bill.reserved.0,date(bill.enactment_day)));
            }
            let implementing = self.countries[country]
                .legislation
                .as_ref()
                .expect("state")
                .bill(bill.id)
                .is_some_and(|b| b.status == LawBillStatus::Implementing);
            if implementing && self.day >= bill.enactment_day {
                self.countries
                    .get_mut(country)
                    .expect("country")
                    .committed_cash
                    .0 -= bill.reserved.0;
                let paid = self.pay(
                    country,
                    Account::Treasury,
                    Account::Firms,
                    bill.reserved.0,
                    "Approved statutory implementation",
                );
                let c = self.countries.get_mut(country).expect("country");
                c.budget.capital.0 += paid;
                let state = c.legislation.as_mut().expect("state");
                let record = state
                    .bills
                    .iter_mut()
                    .find(|b| b.id == bill.id)
                    .expect("bill");
                record.reserved = Money(0);
                record.paid.0 += paid;
                if paid != bill.reserved.0 {
                    self.close_law_bill(
                        country,
                        bill.id,
                        LawBillStatus::Refused,
                        "Actual implementation funds were insufficient; no statute was enacted"
                            .into(),
                    );
                    continue;
                }
                record.status = LawBillStatus::Enacted;
                record.closed_day = Some(self.day);
                record.explanation =
                    "Consent and funded implementation completed; programme permission is enacted"
                        .into();
                state.enacted.insert(
                    bill.law.clone(),
                    EnactedLaw {
                        bill: bill.id,
                        government: bill.sponsor,
                        day: self.day,
                        authority: bill.authority.clone(),
                        paid: bill.total_cost,
                    },
                );
                state.prune();
                self.event(country,true,format!("Statutory bill #{} enacted: {}. Total {}m paid; statutory permission is active. Programme funding, consent and delivery remain separate; no services or capacity are awarded by enactment.",bill.id.0,content.legislation.as_ref().expect("rules").law(&bill.law).expect("law").name,bill.total_cost.0));
            }
        }
    }
    pub(crate) fn notice_legislation_dates(&mut self, content: &Content, country: &CountryId) {
        let Some(rules) = &content.legislation else {
            return;
        };
        let today: CivilDate = date(self.day).parse().expect("campaign date");
        for law in rules
            .definitions
            .iter()
            .filter(|law| law.available_from == today && law.eligible_countries.contains(country))
        {
            self.event(country, true, format!("Statutory proposal date reached: {}. A funded bill can now be considered under the governing consent and implementation rules; permission is granted only after enactment.",law.name));
        }
    }
    pub(crate) fn ai_legislation(&mut self, content: &Content, country: &CountryId) {
        let Some(rules) = &content.legislation else {
            return;
        };
        let Some(targets) = rules.ai_targets.get(country) else {
            return;
        };
        let state = self.countries[country]
            .legislation
            .as_ref()
            .expect("legislation");
        if state.bills.iter().any(|bill| bill.status.pending()) {
            return;
        }
        for law in targets {
            if state.enacted.contains_key(law) {
                continue;
            }
            if self.preview_law(content, country, law).is_ok() {
                let command =
                    self.command(country.clone(), Action::ProposeLaw { law: law.clone() });
                match self.apply(content,command,crate::developer::CommandOrigin::Ai) {
                    Ok(()) => self.event(country,false,"AI statutory review: proposed a configured legal authority through ordinary paid consent and implementation rules; no programme output is granted."),
                    Err(reason) => self.event(country,false,format!("AI statutory proposal declined by shared rules: {reason}")),
                }
                return;
            }
        }
    }
    pub(crate) fn validate_legislation(
        &self,
        content: &Content,
        country: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[country];
        let (Some(rules), Some(state)) = (&content.legislation, &c.legislation) else {
            return if content.legislation.is_none() && c.legislation.is_none() {
                Ok(())
            } else {
                Err("Legislation state/content mismatch".into())
            };
        };
        if state.bills.iter().filter(|b| b.status.pending()).count() > 1
            || state.bills.iter().filter(|b| !b.status.pending()).count() > 128
        {
            return Err("Too many retained or pending statutory bills".into());
        }
        let mut ids = BTreeSet::new();
        for bill in &state.bills {
            let law = rules.law(&bill.law).ok_or("Unknown bill statute")?;
            let command = self
                .journal_entry(&CommandId(bill.id.0))
                .ok_or("Statutory bill lacks its proposal command")?;
            let costs = rules.costs(content.country(country).expect("country").gdp)?;
            let known_authority =
                bill.authority == content.country(country).expect("country").government
                    || content.constitutions.as_ref().is_some_and(|r| {
                        r.templates.iter().any(|t| t.government == bill.authority)
                    });
            let available: CivilDate = date(bill.proposed_day).parse()?;
            if !ids.insert(bill.id)
                || !matches!(&command.command.action,Action::ProposeLaw{law}if law==&bill.law)
                || &command.command.issuer != country
                || command.command.day != bill.proposed_day
                || bill.sponsor.0 == 0
                || bill.sponsor > c.government_number
                || !known_authority
                || !law.eligible_countries.contains(country)
                || available < law.available_from
                || bill.proposed_day > self.day
                || bill.proposed_day.units() < (content.start_day()?).units()
                || bill.proposed_day.checked_add(bill.authority.consent_days)
                    != Some(bill.review_day)
                || bill.review_day.checked_add(rules.implementation_days)
                    != Some(bill.enactment_day)
                || bill.enactment_day.units() > MAX_DAY
                || (bill.total_cost, bill.upfront) != (costs.0, costs.1)
                || bill.paid.0 < bill.upfront.0
                || bill.paid.0 > bill.total_cost.0
                || bill.reserved.0 < 0
                || bill.explanation.is_empty()
                || bill.explanation.len() > 4096
                || state.next_request.get(&bill.law).is_none_or(|day| {
                    bill.proposed_day
                        .checked_add(rules.cooldown_days)
                        .is_none_or(|required| *day < required)
                })
            {
                return Err("Invalid statutory bill identity, authority, dates or finances".into());
            }
            if bill.status.pending() {
                if bill.closed_day.is_some()
                    || bill.paid != bill.upfront
                    || bill.reserved != costs.2
                    || (bill.status == LawBillStatus::AwaitingConsent
                        && self.day >= bill.review_day)
                    || (bill.status == LawBillStatus::Implementing
                        && (self.day < bill.review_day || self.day >= bill.enactment_day))
                {
                    return Err("Invalid pending statutory bill".into());
                }
            } else if bill.reserved != Money(0)
                || bill
                    .closed_day
                    .is_none_or(|day| day < bill.proposed_day || day > self.day)
                || (bill.status == LawBillStatus::Enacted
                    && (bill.paid != bill.total_cost
                        || bill.closed_day != Some(bill.enactment_day)
                        || state
                            .enacted
                            .get(&bill.law)
                            .is_none_or(|entry| entry.bill != bill.id)))
            {
                return Err("Invalid completed statutory bill".into());
            }
        }
        for entry in self
            .journal
            .iter()
            .filter(|entry| &entry.command.issuer == country)
        {
            match &entry.command.action {
                Action::Enact { policy, value } => {
                    self.check_policy_laws_at(content, country, policy, *value, entry.command.day)?
                }
                Action::AmendCommitment { request, value } => {
                    let record = c
                        .commitment(request)
                        .ok_or("Statutory amendment lacks its programme record")?;
                    self.check_policy_laws_at(
                        content,
                        country,
                        &record.project.policy,
                        *value,
                        entry.command.day,
                    )?;
                }
                _ => {}
            }
        }
        for (law_id, entry) in &state.enacted {
            let law = rules.law(law_id).ok_or("Unknown enacted statute")?;
            let command = self
                .journal_entry(&CommandId(entry.bill.0))
                .ok_or("Enacted statute lacks its original proposal")?;
            let known_authority =
                entry.authority == content.country(country).expect("country").government
                    || content.constitutions.as_ref().is_some_and(|r| {
                        r.templates.iter().any(|t| t.government == entry.authority)
                    });
            if !known_authority
                || entry.paid
                    != rules
                        .costs(content.country(country).expect("country").gdp)?
                        .0
                || command
                    .command
                    .day
                    .checked_add(entry.authority.consent_days)
                    .and_then(|day| day.checked_add(rules.implementation_days))
                    != Some(entry.day)
                || state.bill(entry.bill).is_some_and(|bill| {
                    bill.status != LawBillStatus::Enacted
                        || bill.closed_day != Some(entry.day)
                        || bill.sponsor != entry.government
                        || bill.paid != entry.paid
                })
                || &command.command.issuer != country
                || !matches!(&command.command.action,Action::ProposeLaw{law}if law==law_id)
                || !law.eligible_countries.contains(country)
                || entry.government.0 == 0
                || entry.government > c.government_number
                || entry.day > self.day
                || entry.day.units() < command.command.day.units()
            {
                return Err("Invalid enacted statute identity or chronology".into());
            }
        }
        if state
            .next_request
            .iter()
            .any(|(id, day)| rules.law(id).is_none() || day.units() > MAX_DAY)
        {
            return Err("Invalid statutory drafting cooldown".into());
        }
        Ok(())
    }
}
