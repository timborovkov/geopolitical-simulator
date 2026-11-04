//! Reviewed fiscal packages fund real policy programmes. Forecasts authorize no cash.
use crate::developer::CommandOrigin;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetBillId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FiscalPlanDefinition {
    pub review_days: u32,
    pub cooldown_days: u32,
    pub annual_window_days: u32,
    pub administration_cost_bps: i64,
    pub source: String,
    pub source_version: String,
}
impl FiscalPlanDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if content.commitments.is_none()
            || !(1..=90).contains(&self.review_days)
            || !(1..=365).contains(&self.cooldown_days)
            || !(90..=365).contains(&self.annual_window_days)
            || !(1..=100).contains(&self.administration_cost_bps)
            || self.source_version.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("Invalid fiscal-plan review, costs, calendar or provenance".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FiscalTargets {
    pub income_tax: i32,
    pub services: i32,
    pub welfare: i32,
}
impl FiscalTargets {
    pub fn current(c: &Country) -> Self {
        Self {
            income_tax: c.tax,
            services: c.services,
            welfare: c.welfare,
        }
    }
    pub fn entries(&self) -> [(&'static str, i32); 3] {
        [
            ("income_tax", self.income_tax),
            ("services", self.services),
            ("welfare", self.welfare),
        ]
    }
    pub fn is_fiscal(policy: &str) -> bool {
        matches!(policy, "income_tax" | "services" | "welfare")
    }
    pub fn valid(&self, content: &Content) -> bool {
        self.entries().iter().all(|(p, v)| {
            content
                .policy(p)
                .is_some_and(|d| (d.min..=d.max).contains(v))
        })
    }
    fn changed(&self, baseline: &Self) -> Vec<(String, i32)> {
        self.entries()
            .into_iter()
            .zip(baseline.entries())
            .filter(|((_, v), (_, b))| v != b)
            .map(|((p, v), _)| (p.into(), v))
            .collect()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BudgetBillStatus {
    Pending,
    Enacted,
    Withdrawn,
    Superseded,
    Refused { reason: String },
}
impl BudgetBillStatus {
    pub fn label(&self) -> String {
        match self {
            Self::Pending => "Awaiting institutional review".into(),
            Self::Enacted => "Enacted; follow linked policy delivery".into(),
            Self::Withdrawn => "Withdrawn; administration retained".into(),
            Self::Superseded => "Closed after cabinet succession".into(),
            Self::Refused { reason } => format!("Refused: {reason}"),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetProgramme {
    pub policy: String,
    pub command: u64,
    pub project: CommitmentId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetBill {
    pub id: BudgetBillId,
    pub year: u32,
    pub proposed_day: u32,
    pub review_day: u32,
    pub effective_day: u32,
    pub sponsor: u32,
    pub baseline: FiscalTargets,
    pub targets: FiscalTargets,
    pub fee: Money,
    pub implementation_cost: Money,
    pub baseline_forecast: BudgetForecast,
    pub planned_forecast: BudgetForecast,
    pub status: BudgetBillStatus,
    pub resolved_day: Option<u32>,
    pub programmes: Vec<BudgetProgramme>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FiscalPlans {
    pub next_proposal: u32,
    pub bills: Vec<BudgetBill>,
}
impl FiscalPlans {
    pub fn new(day: u32) -> Self {
        Self {
            next_proposal: day,
            bills: Vec::new(),
        }
    }
    pub fn pending(&self) -> bool {
        self.bills
            .iter()
            .any(|b| b.status == BudgetBillStatus::Pending)
    }
    pub fn reserved_cash(&self) -> Money {
        Money(
            self.bills
                .iter()
                .filter(|b| b.status == BudgetBillStatus::Pending)
                .map(|b| b.implementation_cost.0)
                .sum(),
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BudgetPreview {
    pub fee: Money,
    pub implementation_cost: Money,
    pub review_day: u32,
    pub effective_day: u32,
    pub programmes: Vec<(String, i32, u32)>,
    pub baseline_forecast: BudgetForecast,
    pub planned_forecast: BudgetForecast,
    pub baseline_national_balance: Money,
    pub planned_national_balance: Money,
    pub route: String,
}
/// Only enacted settings are published. Deliberations, cash and private project IDs stay private.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedFiscalPlan {
    pub year: u32,
    pub enacted_day: u32,
    pub first_scheduled_delivery: u32,
    pub targets: FiscalTargets,
}
fn year(day: u32) -> u32 {
    date(day)[..4].parse().expect("calendar year")
}
fn year_start(year: u32) -> Result<u32, String> {
    parse_date(&format!("{year:04}-01-01"))
}
fn programme_cost(
    content: &Content,
    id: &CountryId,
    targets: &FiscalTargets,
    baseline: &FiscalTargets,
) -> Money {
    let gdp = content.country(id).expect("country").gdp;
    Money(
        targets
            .changed(baseline)
            .iter()
            .map(|(p, _)| (gdp * content.policy(p).expect("policy").cost_bps / 10000).max(1))
            .sum(),
    )
}
impl World {
    pub fn preview_budget(
        &self,
        content: &Content,
        id: &CountryId,
        fiscal_year: u32,
        targets: &FiscalTargets,
    ) -> Result<BudgetPreview, String> {
        let rules = content
            .fiscal_plans
            .as_ref()
            .ok_or("This campaign has no fiscal-plan procedure")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let state = c.fiscal_plans.as_ref().ok_or("Missing fiscal-plan state")?;
        if state.pending() {
            return Err("Withdraw or resolve the pending budget before submitting another".into());
        }
        if self.day < state.next_proposal {
            return Err(format!(
                "Budget preparation interval ends {}",
                date(state.next_proposal)
            ));
        }
        if !targets.valid(content) {
            return Err("Budget targets exceed permitted policy levels".into());
        }
        let current_year = year(self.day);
        let effective_day = if fiscal_year == current_year {
            self.day
        } else if fiscal_year == current_year + 1 && fiscal_year <= 2099 {
            let start = year_start(fiscal_year)?;
            if start - self.day > rules.annual_window_days {
                return Err(
                    "Next-year budgets open only in the configured annual preparation window"
                        .into(),
                );
            }
            start
        } else {
            return Err(
                "Choose the current fiscal year (supplementary) or next fiscal year (annual)"
                    .into(),
            );
        };
        if fiscal_year > 2099 {
            return Err("A fiscal plan must fit within the campaign calendar".into());
        }
        if c.projects
            .iter()
            .any(|p| FiscalTargets::is_fiscal(&p.policy))
        {
            return Err("Wait for or withdraw existing fiscal programmes before proposing a combined budget".into());
        }
        let baseline = FiscalTargets::current(c);
        let changed = targets.changed(&baseline);
        if changed.is_empty() {
            return Err("Choose at least one tax or spending change".into());
        }
        let g = self.governing_rules(content, id)?;
        let review_day = self
            .day
            .checked_add(rules.review_days.max(g.consent_days))
            .filter(|d| *d <= MAX_DAY)
            .ok_or("Budget review exceeds the campaign calendar")?;
        if review_day >= year_start(fiscal_year + 1)? {
            return Err("Budget review would occur after the selected fiscal year; choose a next-year annual plan".into());
        }
        let mut programmes = Vec::new();
        for (p, v) in changed {
            let preview = self.preview(content, id, &p, v)?;
            let delivery = review_day
                .checked_add(preview.delay - preview.consent_delay)
                .map(|d| d.max(effective_day))
                .filter(|d| *d <= MAX_DAY)
                .ok_or("Budget programme delivery exceeds the campaign calendar")?;
            programmes.push((p, v, delivery));
        }
        self.next_command
            .checked_add(4)
            .ok_or("Command ID space exhausted")?;
        self.next_project
            .checked_add(3)
            .ok_or("Project ID space exhausted")?;
        let cost = programme_cost(content, id, targets, &baseline);
        let d = content.country(id).ok_or("Missing country definition")?;
        let fee = Money((d.gdp * rules.administration_cost_bps / 10000).max(1));
        if cost.0 + fee.0 > c.available_cash().0 {
            return Err(format!(
                "Budget needs {}m uncommitted cash: {}m administration plus {}m reserved implementation; available {}m",
                cost.0 + fee.0,
                fee.0,
                cost.0,
                c.available_cash().0
            ));
        }
        let mut planned = c.clone();
        planned.tax = targets.income_tax;
        planned.services = targets.services;
        planned.welfare = targets.welfare;
        let baseline_forecast = c.forecast(d, content.income_taxes.as_ref());
        let planned_forecast = planned.forecast(d, content.income_taxes.as_ref());
        let baseline_national_balance = c.national_daily_balance(d, &baseline_forecast);
        let planned_national_balance = planned.national_daily_balance(d, &planned_forecast);
        Ok(BudgetPreview {
            fee,
            implementation_cost: cost,
            review_day,
            effective_day,
            programmes,
            baseline_forecast,
            planned_forecast,
            baseline_national_balance,
            planned_national_balance,
            route: g.label(),
        })
    }
    pub(crate) fn propose_budget(
        &mut self,
        content: &Content,
        id: &CountryId,
        bill: BudgetBillId,
        fiscal_year: u32,
        targets: &FiscalTargets,
    ) -> Result<(), String> {
        let p = self.preview_budget(content, id, fiscal_year, targets)?;
        self.pay(
            id,
            Account::Treasury,
            Account::Firms,
            p.fee.0,
            "Budget proposal administration",
        );
        let c = self.countries.get_mut(id).expect("country");
        c.budget.capital.0 += p.fee.0;
        c.committed_cash.0 += p.implementation_cost.0;
        let baseline = FiscalTargets::current(c);
        let sponsor = c.government_number;
        let state = c.fiscal_plans.as_mut().expect("fiscal plans");
        state.next_proposal =
            self.day + content.fiscal_plans.as_ref().expect("rules").cooldown_days;
        state.bills.push(BudgetBill {
            id: bill,
            year: fiscal_year,
            proposed_day: self.day,
            review_day: p.review_day,
            effective_day: p.effective_day,
            sponsor,
            baseline,
            targets: targets.clone(),
            fee: p.fee,
            implementation_cost: p.implementation_cost,
            baseline_forecast: p.baseline_forecast,
            planned_forecast: p.planned_forecast,
            status: BudgetBillStatus::Pending,
            resolved_day: None,
            programmes: Vec::new(),
        });
        self.event(id,true,format!("Budget #{} proposed for {fiscal_year}. Paid {}m administration; {}m implementation cash reserved. Review {}; no setting changes before actual delivery or {}. Recurring spending still needs daily cash.",bill.0,p.fee.0,p.implementation_cost.0,date(p.review_day),date(p.effective_day)));
        Ok(())
    }
    pub(crate) fn withdraw_budget(
        &mut self,
        id: &CountryId,
        bill: BudgetBillId,
    ) -> Result<(), String> {
        let c = self.countries.get_mut(id).ok_or("Unknown country")?;
        let b = c
            .fiscal_plans
            .as_mut()
            .and_then(|s| s.bills.iter_mut().find(|b| b.id == bill))
            .ok_or("Unknown budget proposal")?;
        if b.status != BudgetBillStatus::Pending {
            return Err("Only an unapproved budget may be withdrawn; enacted programmes have their own withdrawal rules".into());
        }
        c.committed_cash.0 -= b.implementation_cost.0;
        b.status = BudgetBillStatus::Withdrawn;
        b.resolved_day = Some(self.day);
        self.event(id,true,format!("Budget #{} withdrawn. Implementation reservation released; administration and preparation interval retained.",bill.0));
        Ok(())
    }
    pub(crate) fn budget_succession(&mut self, id: &CountryId) {
        let c = self.countries.get_mut(id).expect("country");
        let Some(state) = &mut c.fiscal_plans else {
            return;
        };
        let mut closed = Vec::new();
        for b in state
            .bills
            .iter_mut()
            .filter(|b| b.status == BudgetBillStatus::Pending && b.sponsor != c.government_number)
        {
            c.committed_cash.0 -= b.implementation_cost.0;
            b.status = BudgetBillStatus::Superseded;
            b.resolved_day = Some(self.day);
            closed.push(b.id.0);
        }
        for b in closed {
            self.event(id,true,format!("Budget #{b} closed after cabinet succession. Unspent implementation reservation released; enacted programmes continue."));
        }
    }
    pub(crate) fn resolve_budget_bills(&mut self, content: &Content, id: &CountryId) {
        self.budget_succession(id);
        let Some(b) = self.countries[id]
            .fiscal_plans
            .as_ref()
            .and_then(|s| {
                s.bills
                    .iter()
                    .find(|b| b.status == BudgetBillStatus::Pending && b.review_day == self.day)
            })
            .cloned()
        else {
            return;
        };
        // Resolve as an atomic package; refusal never leaves half a budget funded.
        let mut trial = self.clone();
        let c = trial.countries.get_mut(id).expect("country");
        c.committed_cash.0 -= b.implementation_cost.0;
        let stored = c
            .fiscal_plans
            .as_mut()
            .expect("plans")
            .bills
            .iter_mut()
            .find(|x| x.id == b.id)
            .expect("bill");
        stored.status = BudgetBillStatus::Enacted;
        stored.resolved_day = Some(self.day);
        let result = (|| -> Result<(), String> {
            if FiscalTargets::current(&trial.countries[id]) != b.baseline {
                return Err("Operating fiscal settings changed during review".into());
            }
            for (policy, value) in b.targets.changed(&b.baseline) {
                let command = trial.next_command;
                let project = CommitmentId(trial.next_project);
                trial.apply(
                    content,
                    Command {
                        id: command,
                        day: trial.day,
                        issuer: id.clone(),
                        action: Action::Enact {
                            policy: policy.clone(),
                            value,
                        },
                    },
                    CommandOrigin::Budget(b.id),
                )?;
                trial
                    .countries
                    .get_mut(id)
                    .expect("country")
                    .fiscal_plans
                    .as_mut()
                    .expect("plans")
                    .bills
                    .iter_mut()
                    .find(|x| x.id == b.id)
                    .expect("bill")
                    .programmes
                    .push(BudgetProgramme {
                        policy,
                        command,
                        project,
                    });
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                *self = trial;
                self.event(id,true,format!("Budget #{} enacted. Reserved implementation paid for its linked programmes; settings change only when each programme operates. Recurring bills remain subject to daily funding.",b.id.0));
            }
            Err(reason) => {
                let c = self.countries.get_mut(id).expect("country");
                c.committed_cash.0 -= b.implementation_cost.0;
                let b = c
                    .fiscal_plans
                    .as_mut()
                    .expect("plans")
                    .bills
                    .iter_mut()
                    .find(|x| x.id == b.id)
                    .expect("bill");
                b.status = BudgetBillStatus::Refused {
                    reason: reason.clone(),
                };
                b.resolved_day = Some(self.day);
                self.event(id,true,format!("Budget review refused: {reason}. Implementation reservation released; proposal administration retained."));
            }
        }
    }
    pub(crate) fn preview_budget_programme(
        &self,
        content: &Content,
        id: &CountryId,
        bill: BudgetBillId,
        policy: &str,
        value: i32,
    ) -> Result<PolicyPreview, String> {
        let b = self.countries[id]
            .fiscal_plans
            .as_ref()
            .and_then(|s| s.bills.iter().find(|b| b.id == bill))
            .ok_or("Unknown budget dispatch")?;
        if b.status != BudgetBillStatus::Enacted
            || b.resolved_day != Some(self.day)
            || !b
                .targets
                .changed(&b.baseline)
                .contains(&(policy.into(), value))
            || b.programmes.iter().any(|p| p.policy == policy)
        {
            return Err("Programme is not authorized by this enacted budget".into());
        }
        let mut p = self.preview_policy(content, id, policy, value, true)?;
        p.delay = (p.delay - p.consent_delay).max(b.effective_day.saturating_sub(self.day));
        p.consent_delay = 0;
        p.route = format!(
            "Enacted budget #{} (institutional consent satisfied)",
            bill.0
        );
        p.upfront_payment = p.cost;
        Ok(p)
    }
    pub fn published_fiscal_plans(&self, id: &CountryId) -> Option<Vec<PublishedFiscalPlan>> {
        Some(
            self.countries
                .get(id)?
                .fiscal_plans
                .as_ref()?
                .bills
                .iter()
                .filter(|b| b.status == BudgetBillStatus::Enacted)
                .map(|b| PublishedFiscalPlan {
                    year: b.year,
                    enacted_day: b.resolved_day.expect("resolved bill"),
                    first_scheduled_delivery: b
                        .programmes
                        .iter()
                        .filter_map(|p| {
                            self.countries[id]
                                .commitments
                                .as_ref()?
                                .iter()
                                .find(|r| r.id == p.project)
                                .map(|r| r.project.due_day)
                        })
                        .min()
                        .expect("enacted programmes"),
                    targets: b.targets.clone(),
                })
                .collect(),
        )
    }
    pub(crate) fn ai_budget_turn(&mut self, content: &Content, id: &CountryId) {
        if content.fiscal_plans.is_none() {
            return;
        }
        let c = &self.countries[id];
        let d = content.country(id).expect("country");
        let f = c.forecast(d, content.income_taxes.as_ref());
        let balance = if content.rules >= 14 {
            c.national_daily_balance(d, &f).0
        } else {
            f.daily_revenue.0 - f.daily_spending.0 - f.daily_interest.0
        };
        if balance >= 0 {
            return;
        }
        let targets = FiscalTargets {
            income_tax: (c.tax + 2).min(45),
            services: c.services.saturating_sub(1).max(4),
            welfare: c.welfare.saturating_sub(1).max(0),
        };
        let fiscal_year = year(self.day);
        if self
            .preview_budget(content, id, fiscal_year, &targets)
            .is_ok()
        {
            let _ = self.apply(
                content,
                Command {
                    id: self.next_command,
                    day: self.day,
                    issuer: id.clone(),
                    action: Action::ProposeBudget {
                        year: fiscal_year,
                        targets,
                    },
                },
                CommandOrigin::Ai,
            );
        }
    }
}
impl World {
    pub(crate) fn validate_fiscal_plans(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[id];
        let (state, rules) = match (&c.fiscal_plans, &content.fiscal_plans) {
            (None, None) => return Ok(()),
            (Some(s), Some(r)) => (s, r),
            _ => return Err("Fiscal-plan state differs from content".into()),
        };
        let proposals: Vec<_> = self
            .journal
            .iter()
            .filter(|e| {
                &e.command.issuer == id && matches!(e.command.action, Action::ProposeBudget { .. })
            })
            .collect();
        if proposals.len() != state.bills.len()
            || state
                .bills
                .iter()
                .filter(|b| b.status == BudgetBillStatus::Pending)
                .count()
                > 1
            || Some(state.next_proposal)
                != state.bills.last().map_or(Some(content.start_day()?), |b| {
                    b.proposed_day.checked_add(rules.cooldown_days)
                })
        {
            return Err("Fiscal-plan journal, interval or pending count mismatch".into());
        }
        let fee = Money(
            (content.country(id).expect("country").gdp * rules.administration_cost_bps / 10000)
                .max(1),
        );
        let mut previous_next = content.start_day()?;
        let mut dispatched = Vec::new();
        for (b, e) in state.bills.iter().zip(proposals) {
            let Action::ProposeBudget {
                year: fiscal_year,
                targets,
            } = &e.command.action
            else {
                unreachable!()
            };
            if b.proposed_day != e.command.day || !(2000..=2099).contains(&b.year) {
                return Err("Invalid fiscal-plan proposal date or year".into());
            }
            let annual = b.year == year(b.proposed_day) + 1;
            let expected_effective = if annual {
                year_start(b.year)?
            } else {
                b.proposed_day
            };
            let opening = &content.country(id).expect("country").government;
            let government = c
                .constitution
                .as_ref()
                .and_then(|s| {
                    s.history
                        .iter()
                        .rfind(|h| h.day <= b.proposed_day && h.government_number <= b.sponsor)
                })
                .and_then(|h| content.constitutions.as_ref()?.template(&h.to))
                .map_or(opening, |t| &t.government);
            let expected_review = b.proposed_day + rules.review_days.max(government.consent_days);
            let changed = b.targets.changed(&b.baseline);
            if b.id.0 != e.command.id
                || b.proposed_day != e.command.day
                || b.year != *fiscal_year
                || b.targets != *targets
                || !b.targets.valid(content)
                || !b.baseline.valid(content)
                || changed.is_empty()
                || b.year > 2099
                || (!annual && b.year != year(b.proposed_day))
                || b.effective_day != expected_effective
                || (annual && b.effective_day - b.proposed_day > rules.annual_window_days)
                || b.proposed_day < previous_next
                || b.fee != fee
                || b.implementation_cost != programme_cost(content, id, &b.targets, &b.baseline)
                || b.sponsor == 0
                || b.sponsor > c.government_number
                || b.review_day != expected_review
                || b.review_day > MAX_DAY
                || b.review_day >= year_start(b.year + 1)?
            {
                return Err(
                    "Invalid fiscal-plan identity, targets, cost, sponsor or calendar".into(),
                );
            }
            previous_next = b.proposed_day + rules.cooldown_days;
            for f in [&b.baseline_forecast, &b.planned_forecast] {
                if [f.daily_revenue.0, f.daily_spending.0, f.daily_interest.0]
                    .iter()
                    .any(|n| !(0..=1_000_000_000).contains(n))
                {
                    return Err("Invalid budget forecast assumptions".into());
                }
            }
            if b.planned_forecast.daily_interest != b.baseline_forecast.daily_interest {
                return Err("A fiscal proposal cannot create forecast financing".into());
            }
            if b.status == BudgetBillStatus::Pending {
                if b.resolved_day.is_some()
                    || b.review_day <= self.day
                    || b.sponsor != c.government_number
                    || !b.programmes.is_empty()
                    || c.projects
                        .iter()
                        .any(|p| FiscalTargets::is_fiscal(&p.policy))
                {
                    return Err(
                        "Invalid pending budget reservation or fiscal policy overlap".into(),
                    );
                }
            } else {
                let resolved = b
                    .resolved_day
                    .ok_or("Closed budget lacks a resolution date")?;
                if resolved < b.proposed_day || resolved > self.day || resolved > b.review_day {
                    return Err("Invalid budget resolution date".into());
                }
                if matches!(
                    b.status,
                    BudgetBillStatus::Enacted | BudgetBillStatus::Refused { .. }
                ) && resolved != b.review_day
                {
                    return Err("Budget review resolved on the wrong day".into());
                }
                if b.status == BudgetBillStatus::Superseded && b.sponsor >= c.government_number {
                    return Err("Budget supersession lacks a successor cabinet".into());
                }
                if b.status == BudgetBillStatus::Enacted {
                    if b.programmes.len() != changed.len() {
                        return Err("Enacted budget lacks its funded programmes".into());
                    }
                    for (p, (policy, value)) in b.programmes.iter().zip(&changed) {
                        let entry = self
                            .journal
                            .get(p.command.saturating_sub(1) as usize)
                            .ok_or("Missing budget programme journal")?;
                        let record = c
                            .commitments
                            .as_ref()
                            .and_then(|r| r.iter().find(|r| r.id == p.project))
                            .ok_or("Missing budget programme commitment")?;
                        if p.policy != *policy
                            || entry.budget_bill != Some(b.id)
                            || &entry.command.issuer != id
                            || entry.command.day != resolved
                            || entry.command.action
                                != (Action::Enact {
                                    policy: policy.clone(),
                                    value: *value,
                                })
                            || record.proposal_command != p.command
                            || record.project.value != *value
                            || record.amendments != 0
                            || p.command <= b.id.0
                            || record.project.consent_day != resolved
                            || record.project.due_day < b.effective_day
                            || !record
                                .milestones
                                .iter()
                                .any(|m| m.stage == CommitmentStage::Funded && m.day == resolved)
                            || record.project.policy != *policy
                        {
                            return Err(
                                "Budget programme funding, enactment or provenance mismatch".into(),
                            );
                        }
                        dispatched.push(p.command);
                    }
                } else if !b.programmes.is_empty() {
                    return Err("An unapproved budget contains funded programmes".into());
                }
            }
            let withdrawal = self.journal.iter().find(|e| {
                &e.command.issuer == id
                    && matches!(e.command.action,Action::WithdrawBudget{request} if request==b.id)
            });
            if (b.status == BudgetBillStatus::Withdrawn) != withdrawal.is_some()
                || withdrawal.is_some_and(|e| Some(e.command.day) != b.resolved_day)
            {
                return Err("Budget withdrawal differs from journal".into());
            }
        }
        let scheduled: Vec<_> = self
            .journal
            .iter()
            .filter(|e| &e.command.issuer == id && e.budget_bill.is_some())
            .map(|e| e.command.id)
            .collect();
        if scheduled != dispatched {
            return Err("Unowned or reordered budget programme dispatch".into());
        }
        Ok(())
    }
}
