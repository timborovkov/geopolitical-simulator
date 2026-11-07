//! Optional rules-6 policy lifecycle. Paid work is sunk; reservations remain cash.
use crate::*;
use serde::{Deserialize, Serialize};

/// Stable identity shared by an active project and its commitment history.
/// The serialized identity remains a numeric value in every campaign version.
///
/// ```compile_fail
/// use sim_core::{Country, LoanId};
/// fn wrong_family(country: &Country, loan: LoanId) {
///     country.project(&loan);
/// }
/// ```
///
/// ```compile_fail
/// use sim_core::PolicyInstance;
/// fn raw_number(project: &mut PolicyInstance) {
///     project.id = 1_u64;
/// }
/// ```
#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommitmentId(pub u64);
impl std::fmt::Display for CommitmentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitmentDefinition {
    pub amendment_fee_bps: BasisPoints,
    pub maximum_amendments: u32,
    pub evaluation_days: DaySpan,
    pub source: String,
    pub source_version: String,
}
impl CommitmentDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=10000).contains(&self.amendment_fee_bps.units_i64())
            || !(1..=10).contains(&self.maximum_amendments)
            || !(1..=365).contains(&self.evaluation_days.units())
            || self.source_version.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("Invalid commitment costs, review interval or provenance".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommitmentStage {
    Proposed,
    Amended,
    Approved,
    Funded,
    Implementing,
    Operating,
    Evaluated,
    Repealed,
    Refused,
    Withdrawn,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitmentMilestone {
    pub day: crate::CampaignDay,
    pub stage: CommitmentStage,
    pub value: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectPolicyEffect {
    pub field: String,
    pub before: i32,
    pub after: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitmentEvaluation {
    pub day: crate::CampaignDay,
    /// Last settled day before delivery; opening-state reports are explicitly absent.
    pub before: Option<DailyReport>,
    pub after: DailyReport,
    pub approval_before: i32,
    pub approval_after: i32,
    pub other_deliveries: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitmentRecord {
    pub id: CommitmentId,
    pub proposal_command: CommandId,
    pub project: PolicyInstance,
    pub paid: Money,
    pub amendments: u32,
    pub milestones: Vec<CommitmentMilestone>,
    pub direct_effects: Vec<DirectPolicyEffect>,
    pub review_due: Option<u32>,
    pub baseline: Option<DailyReport>,
    pub approval_at_delivery: Option<i32>,
    pub evaluation: Option<CommitmentEvaluation>,
}
impl CommitmentRecord {
    pub fn stage(&self) -> CommitmentStage {
        self.milestones.last().expect("record milestone").stage
    }
    fn mark(&mut self, day: u32, stage: CommitmentStage) {
        self.milestones.push(CommitmentMilestone {
            day: crate::CampaignDay::new(day),
            stage,
            value: self.project.value,
        });
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmendmentPreview {
    pub fee: Money,
    pub consent_day: u32,
    pub due_day: u32,
    pub route: String,
    pub preparation_days: DaySpan,
    pub annual_budget_change: Money,
    pub household_impacts: Vec<HouseholdImpact>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancellationPreview {
    pub released: Money,
    pub sunk: Money,
}
fn level(c: &Country, policy: &PolicyId) -> i32 {
    match policy.as_str() {
        "income_tax" => c.tax,
        "services" => c.services,
        "welfare" => c.welfare,
        _ => 0,
    }
}
impl World {
    pub fn preview_amendment(
        &self,
        content: &Content,
        id: &CountryId,
        request: CommitmentId,
        value: i32,
    ) -> Result<AmendmentPreview, String> {
        let rules = content
            .commitments
            .as_ref()
            .ok_or("This campaign has no commitment amendment rules")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let record = c
            .commitments
            .as_ref()
            .and_then(|records| records.iter().find(|r| r.id == request))
            .ok_or("Unknown commitment for this government")?;
        let p = c
            .project(&request)
            .ok_or("Only a pending commitment can be amended")?;
        if p.stage != PolicyStage::AwaitingConsent {
            return Err(
                "Funded implementation cannot be amended; withdraw or let it operate".into(),
            );
        }
        if !matches!(p.policy.as_str(), "income_tax" | "services" | "welfare") {
            return Err("This programme has no adjustable level".into());
        }
        let definition = content.policy(&p.policy).ok_or("Unknown policy")?;
        if !(definition.min..=definition.max).contains(&value)
            || value == p.value
            || value == level(c, &p.policy)
        {
            return Err("Choose a different permitted level that is not already operating".into());
        }
        if record.amendments >= rules.maximum_amendments {
            return Err("This proposal has reached its amendment limit".into());
        }
        let government = self.governing_rules(content, id)?;
        government.check(c, p.policy.as_str(), value)?;
        self.check_policy_laws(content, id, &p.policy, value)?;
        let fee = Money((p.cost.0 * rules.amendment_fee_bps.units_i64() / 10000).max(1));
        if fee.0 > c.available_cash().0 {
            return Err("Amendment administration requires uncommitted treasury cash".into());
        }
        let consent_day = self
            .day
            .units()
            .checked_add(government.consent_days.units())
            .ok_or("Consent date overflow")?;
        let preparation_days = self.policy_preparation_days(content, id, &p.policy);
        let implementation = definition
            .days
            .checked_add(DaySpan::new((100 - c.administration) as u32 / 2))
            .and_then(|days| days.checked_add(preparation_days))
            .ok_or("Implementation duration overflow")?;
        let due_day = consent_day
            .checked_add(implementation.units())
            .filter(|day| *day <= MAX_DAY)
            .ok_or("Implementation would exceed the campaign calendar")?;
        let (annual_budget_change, household_impacts) = crate::rules::policy_forecast(
            content.country(id).ok_or("Missing country definition")?,
            c,
            &p.policy,
            value,
            content.income_taxes.as_ref(),
        );
        Ok(AmendmentPreview {
            fee,
            consent_day,
            due_day,
            route: government.label(),
            preparation_days,
            annual_budget_change,
            household_impacts,
        })
    }
    pub fn preview_cancellation(
        &self,
        content: &Content,
        id: &CountryId,
        request: CommitmentId,
    ) -> Result<CancellationPreview, String> {
        content
            .commitments
            .as_ref()
            .ok_or("This campaign has no commitment withdrawal rules")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let record = c
            .commitments
            .as_ref()
            .and_then(|records| records.iter().find(|r| r.id == request))
            .ok_or("Unknown commitment for this government")?;
        let p = c
            .project(&request)
            .ok_or("Only a pending commitment can be withdrawn")?;
        if p.stage == PolicyStage::Implementing {
            self.governing_rules(content, id)?
                .check(c, p.policy.as_str(), p.value)?;
        }
        Ok(CancellationPreview {
            released: p.reserved,
            sunk: record.paid,
        })
    }
    pub(crate) fn amend_commitment(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: CommitmentId,
        value: i32,
    ) -> Result<(), String> {
        let preview = self.preview_amendment(content, id, request, value)?;
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            preview.fee.0,
            "Policy amendment administration payment",
        );
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += preview.fee.0;
        let p = c
            .projects
            .iter_mut()
            .find(|p| p.id == request)
            .expect("project");
        p.value = value;
        p.start_day = preview.consent_day;
        p.consent_day = preview.consent_day;
        p.due_day = preview.due_day;
        let r = c
            .commitments
            .as_mut()
            .expect("records")
            .iter_mut()
            .find(|r| r.id == request)
            .expect("record");
        r.paid.0 += preview.fee.0;
        r.amendments += 1;
        r.project = p.clone();
        r.mark(self.day.units(), CommitmentStage::Amended);
        // An intervening constitutional change may now permit immediate consent.
        self.resolve_consents(content, id);
        self.event_for_commitment(id, request, CommitmentStage::Amended, format!("Commitment #{} amended to {value}. Paid {}m administration; consent {} and operating due {}. Existing capital reservation is unchanged.",request.0,preview.fee.0,date(preview.consent_day),date(preview.due_day)));
        Ok(())
    }
    pub(crate) fn cancel_commitment(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: CommitmentId,
    ) -> Result<(), String> {
        let preview = self.preview_cancellation(content, id, request)?;
        let c = self.countries.get_mut(id).expect("country");
        c.committed_cash.0 -= preview.released.0;
        c.projects.retain(|p| p.id != request);
        let r = c
            .commitments
            .as_mut()
            .expect("records")
            .iter_mut()
            .find(|r| r.id == request)
            .expect("record");
        r.project.reserved = Money(0);
        r.mark(self.day.units(), CommitmentStage::Withdrawn);
        self.event_for_commitment(id, request, CommitmentStage::Withdrawn, format!("Commitment #{} withdrawn. Released {}m reserved cash; {}m already paid remains spent. No operating effect was delivered.", request.0,preview.released.0,preview.sunk.0));
        Ok(())
    }
    pub(crate) fn record_proposal(&mut self, id: &CountryId, command: CommandId) {
        let c = self.countries.get_mut(id).expect("country");
        let Some(records) = &mut c.commitments else {
            return;
        };
        let p = c.projects.last().expect("new project").clone();
        let mut r = CommitmentRecord {
            id: p.id,
            proposal_command: command,
            paid: Money(p.cost.0 - p.reserved.0),
            project: p,
            amendments: 0,
            milestones: Vec::new(),
            direct_effects: Vec::new(),
            review_due: None,
            baseline: None,
            approval_at_delivery: None,
            evaluation: None,
        };
        r.mark(self.day.units(), CommitmentStage::Proposed);
        if r.project.stage == PolicyStage::Implementing {
            for stage in [
                CommitmentStage::Approved,
                CommitmentStage::Funded,
                CommitmentStage::Implementing,
            ] {
                r.mark(self.day.units(), stage);
            }
        }
        records.push(r);
    }
    pub(crate) fn record_consent(
        &mut self,
        id: &CountryId,
        project: &PolicyInstance,
        approved: bool,
    ) {
        let c = self.countries.get_mut(id).expect("country");
        let Some(records) = &mut c.commitments else {
            return;
        };
        let r = records
            .iter_mut()
            .find(|r| r.id == project.id)
            .expect("record");
        r.project.reserved = Money(0);
        if approved {
            r.paid.0 += project.reserved.0;
            r.project.stage = PolicyStage::Implementing;
            for stage in [
                CommitmentStage::Approved,
                CommitmentStage::Funded,
                CommitmentStage::Implementing,
            ] {
                r.mark(self.day.units(), stage);
            }
        } else {
            r.mark(self.day.units(), CommitmentStage::Refused);
        }
    }
    pub(crate) fn record_delivery(
        &mut self,
        content: &Content,
        id: &CountryId,
        project: &PolicyInstance,
        before: &Country,
    ) {
        let c = self.countries.get_mut(id).expect("country");
        let Some(records) = &mut c.commitments else {
            return;
        };
        let effects = match project.policy.as_str() {
            "income_tax" => vec![("income_tax", before.tax, c.tax)],
            "services" => vec![("services", before.services, c.services)],
            "welfare" => vec![("welfare", before.welfare, c.welfare)],
            "energy" => vec![("energy_capacity", before.energy_capacity, c.energy_capacity)],
            "agriculture" => vec![("food_capacity", before.food_capacity, c.food_capacity)],
            "administration" => vec![("administration", before.administration, c.administration)],
            "coalition" => vec![
                (
                    "legislative_support",
                    before.legislative_support,
                    c.legislative_support,
                ),
                ("elite_support", before.elite_support, c.elite_support),
                ("council_support", before.council_support, c.council_support),
            ],
            _ => unreachable!("validated policy"),
        };
        if matches!(
            project.policy.as_str(),
            "income_tax" | "services" | "welfare"
        ) {
            for r in records.iter_mut().filter(|r| {
                r.project.policy == project.policy
                    && matches!(
                        r.stage(),
                        CommitmentStage::Operating | CommitmentStage::Evaluated
                    )
            }) {
                r.mark(self.day.units(), CommitmentStage::Repealed);
                r.review_due = None;
            }
        }
        let r = records
            .iter_mut()
            .find(|r| r.id == project.id)
            .expect("record");
        r.project = project.clone();
        r.direct_effects = effects
            .into_iter()
            .map(|(field, before, after)| DirectPolicyEffect {
                field: field.into(),
                before,
                after,
            })
            .collect();
        r.baseline = (self.day.units() > content.start_day().expect("start") + 1)
            .then(|| before.report.clone());
        r.approval_at_delivery = Some(before.approval);
        r.review_due = self
            .day
            .units()
            .checked_add(
                content
                    .commitments
                    .as_ref()
                    .expect("rules")
                    .evaluation_days
                    .units(),
            )
            .filter(|day| *day <= MAX_DAY);
        r.mark(self.day.units(), CommitmentStage::Operating);
    }
    pub(crate) fn evaluate_commitments(&mut self, id: &CountryId) {
        let c = self.countries.get_mut(id).expect("country");
        let Some(records) = &mut c.commitments else {
            return;
        };
        if !records
            .iter()
            .any(|r| r.review_due == Some(self.day.units()))
        {
            return;
        }
        let deliveries: Vec<_> = records
            .iter()
            .flat_map(|r| r.milestones.iter())
            .filter(|m| m.stage == CommitmentStage::Operating)
            .map(|m| m.day)
            .collect();
        let mut evaluated = Vec::new();
        for r in records
            .iter_mut()
            .filter(|r| r.review_due == Some(self.day.units()))
        {
            let delivery = r
                .milestones
                .iter()
                .find(|m| m.stage == CommitmentStage::Operating)
                .expect("delivery")
                .day;
            r.evaluation = Some(CommitmentEvaluation {
                day: self.day,
                before: r.baseline.clone(),
                after: c.report.clone(),
                approval_before: r.approval_at_delivery.expect("approval"),
                approval_after: c.approval,
                other_deliveries: deliveries
                    .iter()
                    .filter(|day| **day >= delivery && **day <= self.day)
                    .count()
                    .saturating_sub(1) as u32,
            });
            r.review_due = None;
            r.mark(self.day.units(), CommitmentStage::Evaluated);
            evaluated.push(r.id);
        }
        for request in evaluated {
            self.event_for_commitment(id,request,CommitmentStage::Evaluated,format!("Commitment #{request} outcome review available: direct setting changes and observed settlement. Other programmes, prices and external shocks can also change outcomes; this is not a causal estimate."));
        }
    }
}
impl World {
    pub(crate) fn validate_commitments(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[id];
        let (records, rules) = match (&c.commitments, &content.commitments) {
            (None, None) => return Ok(()),
            (Some(records), Some(rules)) => (records, rules),
            _ => return Err("Commitment records differ from campaign rules/content".into()),
        };
        let proposals: Vec<_> = self
            .journal
            .iter()
            .filter(|e| matches!(e.command.action, Action::Enact { .. }))
            .enumerate()
            .filter(|(_, e)| &e.command.issuer == id)
            .collect();
        if c.projects
            .iter()
            .any(|p| !records.iter().any(|r| r.id == p.id && r.project == *p))
            || proposals.len() != records.len()
            || self.next_project
                != self
                    .journal
                    .iter()
                    .filter(|e| matches!(e.command.action, Action::Enact { .. }))
                    .count() as u64
                    + 1
        {
            return Err("Commitment records do not cover the proposal journal".into());
        }
        for (r, (ordinal, entry)) in records.iter().zip(proposals) {
            let Action::Enact { policy, value } = &entry.command.action else {
                unreachable!()
            };
            let definition = content.policy(policy).ok_or("Unknown commitment policy")?;
            let cost = Money(
                (content.country(id).expect("country").gdp.units()
                    * definition.cost_bps.units_i64()
                    / 10000)
                    .max(1),
            );
            if r.milestones.is_empty()
                || r.id.0 != ordinal as u64 + 1
                || r.project.id != r.id
                || r.proposal_command != entry.command.id
                || r.project.policy != *policy
                || r.project.cost != cost
                || r.project.start_day != r.project.consent_day
                || r.project.consent_day < entry.command.day.units()
                || r.project.due_day < r.project.consent_day
                || r.project.due_day > MAX_DAY
                || !(definition.min..=definition.max).contains(&r.project.value)
                || r.milestones[0]
                    != (CommitmentMilestone {
                        day: entry.command.day,
                        stage: CommitmentStage::Proposed,
                        value: *value,
                    })
                || r.milestones.windows(2).any(|m| m[0].day > m[1].day)
                || r.milestones.iter().any(|m| {
                    m.day > self.day || !(definition.min..=definition.max).contains(&m.value)
                })
                || r.milestones.last().expect("milestone").value != r.project.value
            {
                return Err("Invalid commitment identity, dates, costs or milestone values".into());
            }
            let amendments: Vec<_> = self.journal.iter().filter(|e| &e.command.issuer==id && matches!(e.command.action,Action::AmendCommitment{request,..} if request==r.id)).collect();
            let marks: Vec<_> = r
                .milestones
                .iter()
                .filter(|m| m.stage == CommitmentStage::Amended)
                .collect();
            if r.amendments as usize!=amendments.len() || marks.len()!=amendments.len() || r.amendments>rules.maximum_amendments
                || marks.iter().zip(amendments).any(|(m,e)| e.command.day!=m.day || !matches!(e.command.action,Action::AmendCommitment{value,..} if value==m.value))
            { return Err("Invalid commitment amendment history".into()); }
            let mut expected = vec![CommitmentStage::Proposed];
            expected.extend(std::iter::repeat_n(
                CommitmentStage::Amended,
                r.amendments as usize,
            ));
            let funded = r
                .milestones
                .iter()
                .any(|m| m.stage == CommitmentStage::Funded);
            if funded {
                expected.extend([
                    CommitmentStage::Approved,
                    CommitmentStage::Funded,
                    CommitmentStage::Implementing,
                ]);
            }
            let operating = r
                .milestones
                .iter()
                .find(|m| m.stage == CommitmentStage::Operating);
            if let Some(m) = operating {
                expected.push(CommitmentStage::Operating);
                if !funded
                    || m.day != crate::CampaignDay::new(r.project.due_day)
                    || r.approval_at_delivery.is_none()
                    || r.direct_effects.is_empty()
                {
                    return Err("Invalid commitment delivery record".into());
                }
                let valid_effects: Vec<_> = match policy.as_str() {
                    "income_tax" => vec![("income_tax", 10, 45, None)],
                    "services" => vec![("services", 4, 18, None)],
                    "welfare" => vec![("welfare", 0, 12, None)],
                    "energy" => vec![("energy_capacity", 0, 200, Some(15))],
                    "agriculture" => vec![("food_capacity", 0, 200, Some(15))],
                    "administration" => vec![("administration", 0, 100, Some(8))],
                    "coalition" => vec![
                        ("legislative_support", 0, 100, Some(12)),
                        ("elite_support", 0, 100, Some(12)),
                        ("council_support", 0, 100, Some(12)),
                    ],
                    _ => return Err("Unknown direct policy effect".into()),
                };
                if r.direct_effects.len() != valid_effects.len()
                    || r.direct_effects.iter().zip(valid_effects).any(
                        |(e, (field, min, max, increment))| {
                            e.field != field
                                || !(min..=max).contains(&e.before)
                                || e.after
                                    != increment
                                        .map_or(r.project.value, |n| (e.before + n).min(max))
                        },
                    )
                {
                    return Err("Invalid direct policy effect".into());
                }
            } else if !r.direct_effects.is_empty()
                || r.review_due.is_some()
                || r.evaluation.is_some()
                || r.baseline.is_some()
                || r.approval_at_delivery.is_some()
            {
                return Err("Undelivered commitment contains outcome evidence".into());
            }
            if let Some(e) = &r.evaluation {
                expected.push(CommitmentStage::Evaluated);
                if operating.and_then(|m| m.day.checked_add(rules.evaluation_days)) != Some(e.day)
                    || e.day > self.day
                    || e.before != r.baseline
                    || Some(e.approval_before) != r.approval_at_delivery
                    || !(0..=100).contains(&e.approval_after)
                {
                    return Err("Invalid commitment outcome review date or baseline".into());
                }
            }
            match r.stage() {
                CommitmentStage::Refused | CommitmentStage::Withdrawn => expected.push(r.stage()),
                CommitmentStage::Repealed if operating.is_some() => {
                    expected.push(CommitmentStage::Repealed)
                }
                _ => {}
            }
            if r.milestones.iter().map(|m| m.stage).collect::<Vec<_>>() != expected {
                return Err("Invalid commitment lifecycle order".into());
            }
            let pending = matches!(
                r.stage(),
                CommitmentStage::Proposed
                    | CommitmentStage::Amended
                    | CommitmentStage::Implementing
            );
            if pending != c.projects.iter().any(|p| p.id == r.id)
                || (pending && !c.projects.iter().any(|p| p == &r.project))
                || (!pending && r.project.reserved.0 != 0)
                || r.paid.0
                    != cost.0
                        - if funded {
                            0
                        } else {
                            cost.0 - (cost.0 / 10).max(1)
                        }
                        + i64::from(r.amendments)
                            * (cost.0 * rules.amendment_fee_bps.units_i64() / 10000).max(1)
            {
                return Err("Commitment funding or pending project disagrees with history".into());
            }
            let cancelled = self.journal.iter().any(|e| {
                &e.command.issuer == id
                    && matches!(e.command.action,Action::CancelCommitment{request} if request==r.id)
                    && e.command.day == r.milestones.last().expect("mark").day
            });
            if (r.stage() == CommitmentStage::Withdrawn) != cancelled {
                return Err("Commitment withdrawal differs from journal".into());
            }
            let expected_review = if r.stage() == CommitmentStage::Operating {
                operating
                    .and_then(|m| m.day.checked_add(rules.evaluation_days))
                    .filter(|day| *day <= crate::CampaignDay::new(MAX_DAY))
            } else {
                None
            };
            if r.review_due != expected_review.map(crate::CampaignDay::units)
                || r.review_due.is_some_and(|day| day <= self.day.units())
            {
                return Err("Invalid pending commitment review".into());
            }
        }
        Ok(())
    }
}
