//! Optional, temporary protection of actual national cash for a later decision.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CashManagementDefinition {
    pub max_target_bps: i64,
    pub max_days: DaySpan,
    pub cooldown_days: DaySpan,
    pub approval_cost: i32,
    pub source: String,
    pub source_version: String,
}

impl CashManagementDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=100).contains(&self.max_target_bps)
            || !(1..=90).contains(&self.max_days.units())
            || !(90..=365).contains(&self.cooldown_days.units())
            || !(1..=20).contains(&self.approval_cost)
            || self.source_version.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("Cash retention requires bounded targets, duration, political cost and source provenance".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CashHoldId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CashHoldEnd {
    Released { command: CommandId },
    Expired,
    GovernmentChanged,
}
impl CashHoldEnd {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Released { .. } => "released by government",
            Self::Expired => "duration expired",
            Self::GovernmentChanged => "sponsoring government changed",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CashHoldRecord {
    pub id: CashHoldId,
    pub target: Money,
    pub start_day: u32,
    pub expiry_day: u32,
    pub sponsor_government: crate::GovernmentId,
    pub funded_notice_day: Option<u32>,
    pub ended: Option<(u32, CashHoldEnd)>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CashManagement {
    pub records: Vec<CashHoldRecord>,
    pub retained: Money,
    pub next_allowed_day: u32,
}
impl CashManagement {
    pub fn active(&self) -> Option<&CashHoldRecord> {
        self.records.last().filter(|r| r.ended.is_none())
    }
}
impl Country {
    /// Reservations and protected local balances have first claim. An unfunded
    /// target never becomes a balance, a transfer or permission to spend.
    pub fn retained_cash(&self) -> Money {
        self.cash_management
            .as_ref()
            .map_or(Money(0), |state| state.retained)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CashHoldPreview {
    pub target: Money,
    pub retained_now: Money,
    pub expiry_day: u32,
    pub next_allowed_day: u32,
    pub approval_cost: i32,
}
impl World {
    pub fn preview_cash_hold(
        &self,
        content: &Content,
        id: &CountryId,
        target: Money,
        days: u32,
    ) -> Result<CashHoldPreview, String> {
        if self.content_hash != content.hash() {
            return Err("Cash retention requires the campaign's exact content".into());
        }
        let rules = content
            .cash_management
            .as_ref()
            .ok_or("This campaign has no temporary cash-retention procedure")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let state = c
            .cash_management
            .as_ref()
            .ok_or("Missing cash-management state")?;
        let d = content.country(id).ok_or("Missing country definition")?;
        if state.active().is_some() {
            return Err("Release the active retention before creating another".into());
        }
        if self.day < state.next_allowed_day {
            return Err(format!(
                "Cash retention may be used again on {}",
                date(state.next_allowed_day)
            ));
        }
        if !(1..=(d.gdp * rules.max_target_bps / 10000).max(1)).contains(&target.0)
            || !(1..=rules.max_days.units()).contains(&days)
        {
            return Err(format!(
                "Target must be 1..{}m and duration 1..{} days",
                (d.gdp * rules.max_target_bps / 10000).max(1),
                rules.max_days.units()
            ));
        }
        let expiry_day = self
            .day
            .checked_add(days)
            .filter(|day| *day <= MAX_DAY)
            .ok_or("Retention exceeds the campaign calendar")?;
        let next_allowed_day = self
            .day
            .checked_add(rules.cooldown_days.units())
            .filter(|day| *day <= MAX_DAY)
            .ok_or("Retention cooldown exceeds the campaign calendar")?;
        Ok(CashHoldPreview {
            target,
            retained_now: Money(target.0.min(c.available_cash().0.max(0))),
            expiry_day,
            next_allowed_day,
            approval_cost: rules.approval_cost,
        })
    }
    pub(crate) fn hold_cash(
        &mut self,
        content: &Content,
        id: &CountryId,
        request: CashHoldId,
        target: Money,
        days: u32,
    ) -> Result<(), String> {
        let preview = self.preview_cash_hold(content, id, target, days)?;
        let c = self.countries.get_mut(id).expect("country");
        let state = c.cash_management.as_mut().expect("cash management");
        state.records.push(CashHoldRecord {
            id: request,
            target,
            start_day: self.day,
            expiry_day: preview.expiry_day,
            sponsor_government: c.government_number,
            funded_notice_day: (preview.retained_now == target).then_some(self.day),
            ended: None,
        });
        state.next_allowed_day = preview.next_allowed_day;
        state.retained = preview.retained_now;
        c.approval = (c.approval - preview.approval_cost).max(0);
        for h in &mut c.households {
            h.approval = (h.approval - preview.approval_cost).max(0);
        }
        self.event(id, true, format!("Temporary cash retention #{}: target {}m until {}. Approval -{}. Only actual uncommitted national cash is protected; unpaid operating claims remain arrears and dated principal remains due. Release before funding a decision.", request.0, target.0, date(preview.expiry_day), preview.approval_cost));
        Ok(())
    }
    pub(crate) fn release_cash(
        &mut self,
        id: &CountryId,
        request: CashHoldId,
        command: CommandId,
    ) -> Result<(), String> {
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let active = c
            .cash_management
            .as_ref()
            .and_then(CashManagement::active)
            .ok_or("No active cash retention")?;
        if active.id != request {
            return Err("This cash retention is no longer active".into());
        }
        let amount = c.retained_cash();
        self.end_cash_hold(id, CashHoldEnd::Released { command });
        self.event(id, true, format!("Cash retention #{} released {}m of actual cash. The unfilled target creates no money; policy costs, authority and delays still apply.", request.0, amount.0));
        Ok(())
    }
    fn end_cash_hold(&mut self, id: &CountryId, reason: CashHoldEnd) {
        let state = self
            .countries
            .get_mut(id)
            .expect("country")
            .cash_management
            .as_mut()
            .expect("cash management");
        state.records.last_mut().expect("active").ended = Some((self.day, reason));
        state.retained = Money(0);
    }
    pub(crate) fn resolve_cash_hold(&mut self, id: &CountryId) {
        let c = &self.countries[id];
        let Some(active) = c.cash_management.as_ref().and_then(CashManagement::active) else {
            return;
        };
        let reason = if active.sponsor_government != c.government_number {
            Some(CashHoldEnd::GovernmentChanged)
        } else if self.day >= active.expiry_day {
            Some(CashHoldEnd::Expired)
        } else {
            None
        };
        if let Some(reason) = reason {
            self.end_cash_hold(id, reason.clone());
            self.event(id, true, format!("Temporary cash retention ended: {}. Actual retained cash is available again; unpaid obligations remain.", reason.label()));
        }
    }
    pub(crate) fn notice_cash_hold(&mut self, id: &CountryId) {
        let c = &self.countries[id];
        if let Some(active) = c.cash_management.as_ref().and_then(CashManagement::active) {
            let extra = (active.target.0 - c.retained_cash().0).min(c.available_cash().0.max(0));
            self.countries
                .get_mut(id)
                .expect("country")
                .cash_management
                .as_mut()
                .expect("cash management")
                .retained
                .0 += extra;
        }
        let c = &self.countries[id];
        let Some(active) = c.cash_management.as_ref().and_then(CashManagement::active) else {
            return;
        };
        if active.funded_notice_day.is_none() && c.retained_cash() == active.target {
            let target = active.target;
            self.countries
                .get_mut(id)
                .expect("country")
                .cash_management
                .as_mut()
                .expect("cash management")
                .records
                .last_mut()
                .expect("active")
                .funded_notice_day = Some(self.day);
            self.event(id, true, format!("Retention target funded with {}m actual cash. Release it before proposing your decision; ordinary costs and authority still apply. Retention expires automatically.", target.0));
        }
    }
    /// Only campaigns opting into retention use this fallback. It never grants
    /// money or bypasses the ordinary funding, backing or implementation gates.
    pub(crate) fn ai_cash_management(&mut self, content: &Content, id: &CountryId) -> bool {
        let Some(rules) = &content.cash_management else {
            return false;
        };
        let c = self.countries[id].clone();
        let active = c
            .cash_management
            .as_ref()
            .and_then(CashManagement::active)
            .cloned();
        if !c.projects.is_empty() || c.fiscal_plans.as_ref().is_some_and(FiscalPlans::pending) {
            return active.is_some();
        }
        let government = self.governing_rules(content, id).expect("government");
        let forecast = c.forecast(
            content.country(id).expect("country"),
            content.income_taxes.as_ref(),
        );
        let candidates = if government.check(&c, "borrow", 0).is_err() {
            vec![("coalition", 1)]
        } else if c
            .national_daily_balance(content.country(id).expect("country"), &forecast)
            .0
            < 0
            || c.arrears.0 > 0
        {
            vec![
                ("services", (c.services - 3).max(4)),
                ("welfare", (c.welfare - 3).max(0)),
                ("income_tax", (c.tax + 3).min(45)),
            ]
        } else {
            vec![]
        };
        for (policy, value) in candidates {
            let Ok(preview) =
                self.preview_policy_funding(content, id, &policy.into(), value, false, false)
            else {
                continue;
            };
            if policy != "coalition" && preview.annual_budget_change.0 >= 0 {
                continue;
            }
            if let Some(active) = &active {
                if c.retained_cash().0 + c.available_cash().0 < preview.cost.0 {
                    continue;
                }
                // Check the actual release and proposal together on a clone. The
                // original world is replaced only if both ordinary commands pass.
                let mut trial = self.clone();
                let release = trial.command(id.clone(), Action::ReleaseCash { request: active.id });
                if trial
                    .apply(content, release, crate::developer::CommandOrigin::Ai)
                    .is_err()
                {
                    continue;
                }
                let proposal = trial.command(
                    id.clone(),
                    Action::Enact {
                        policy: policy.into(),
                        value,
                    },
                );
                if trial
                    .apply(content, proposal, crate::developer::CommandOrigin::Ai)
                    .is_ok()
                {
                    trial.event(id, false, "AI released actual retained receipts and funded a recovery programme through ordinary policy rules. Deferred obligations remain payable.");
                    *self = trial;
                    return true;
                }
            } else {
                // Existing funding and pending/available credit have priority.
                if c.available_cash().0 >= preview.cost.0
                    || c.financing.as_ref().is_some_and(|f| f.pending.is_some())
                    || self
                        .preview_borrowing(
                            content,
                            id,
                            Money(content.country(id).expect("country").gdp / 20),
                        )
                        .is_ok()
                {
                    return false;
                }
                if self
                    .preview_cash_hold(content, id, preview.cost, rules.max_days.units())
                    .is_ok()
                {
                    let command = self.command(
                        id.clone(),
                        Action::HoldCash {
                            target: preview.cost,
                            days: rules.max_days.units(),
                        },
                    );
                    if self
                        .apply(content, command, crate::developer::CommandOrigin::Ai)
                        .is_ok()
                    {
                        self.event(id, false, format!("AI is retaining real receipts for {policy}; credit is blocked and no future receipt is treated as money. Deferring payments may increase arrears."));
                        return true;
                    }
                }
            }
        }
        active.is_some()
    }
    pub(crate) fn validate_cash_management(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[id];
        let (rules, state) = match (&content.cash_management, &c.cash_management) {
            (None, None) => return Ok(()),
            (Some(rules), Some(state)) => (rules, state),
            _ => return Err("Cash-management state differs from campaign content".into()),
        };
        let holds: Vec<_> = self
            .journal
            .iter()
            .filter(|e| {
                e.command.issuer == *id && matches!(e.command.action, Action::HoldCash { .. })
            })
            .collect();
        let releases: Vec<_> = self
            .journal
            .iter()
            .filter(|e| {
                e.command.issuer == *id && matches!(e.command.action, Action::ReleaseCash { .. })
            })
            .collect();
        if holds.len() != state.records.len() {
            return Err("Cash retention history differs from its command journal".into());
        }
        if state
            .active()
            .is_some_and(|r| r.funded_notice_day.is_some() && state.retained != r.target)
        {
            return Err("Funded active retention must retain the actual target balance".into());
        }
        if state.retained.0 < 0
            || state.retained.0 > state.active().map_or(0, |r| r.target.0)
            || i128::from(state.retained.0)
                + i128::from(c.committed_cash.0)
                + i128::from(c.regional_protected_cash().0)
                > i128::from(c.accounts.treasury.0)
        {
            return Err("Retained cash exceeds actual uncommitted national cash or target".into());
        }
        let mut next_allowed = 0;
        let mut previous_end = content.start_day()?;
        let mut release_count = 0;
        for (index, (record, entry)) in state.records.iter().zip(holds).enumerate() {
            let Action::HoldCash { target, days } = entry.command.action else {
                unreachable!()
            };
            let max_target =
                (content.country(id).expect("country").gdp * rules.max_target_bps / 10000).max(1);
            if record.id.0 != entry.command.id.0
                || record.target != target
                || !(1..=max_target).contains(&target.0)
                || !(1..=rules.max_days.units()).contains(&days)
                || record.start_day != entry.command.day
                || record.start_day < next_allowed
                || record.start_day < previous_end
                || record.start_day.checked_add(days) != Some(record.expiry_day)
                || record.expiry_day > MAX_DAY
                || record.sponsor_government == crate::GovernmentId(0)
                || record.sponsor_government > c.government_number
                || record.funded_notice_day.is_some_and(|day| {
                    day < record.start_day || day >= record.expiry_day || day > self.day
                })
            {
                return Err("Invalid cash retention terms, sponsor or chronology".into());
            }
            next_allowed = record
                .start_day
                .checked_add(rules.cooldown_days.units())
                .filter(|day| *day <= MAX_DAY)
                .ok_or("Invalid cash retention cooldown")?;
            match &record.ended {
                None if index + 1 == state.records.len()
                    && self.day < record.expiry_day
                    && record.sponsor_government == c.government_number =>
                {
                    previous_end = self.day;
                }
                Some((day, reason))
                    if *day >= record.start_day
                        && *day <= self.day
                        && record.funded_notice_day.is_none_or(|notice| notice <= *day) =>
                {
                    match reason {
                        CashHoldEnd::Expired if *day == record.expiry_day => {}
                        CashHoldEnd::GovernmentChanged if *day <= record.expiry_day && record.sponsor_government < c.government_number => {}
                        CashHoldEnd::Released { command } if *day < record.expiry_day && releases.iter().any(|e| e.command.id == *command && e.command.day == *day && matches!(e.command.action, Action::ReleaseCash { request } if request == record.id)) => { release_count += 1; }
                        _ => return Err("Invalid cash retention ending".into()),
                    }
                    previous_end = *day;
                }
                _ => return Err("Invalid active cash retention or end date".into()),
            }
        }
        if state.next_allowed_day != next_allowed || release_count != releases.len() {
            return Err("Cash retention cooldown/release journal mismatch".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ai_collects_actual_cash_and_uses_shared_commands_when_credit_is_unavailable() {
        let mut content = Content::from_json(include_str!(
            "../../../data/synthetic-cash-management-test.json"
        ))
        .unwrap();
        content.finance.as_mut().unwrap().authorities.clear();
        content.finance.as_mut().unwrap().emergency = None;
        for country in &mut content.countries {
            country.tax = 10;
            country.services = 18;
            country.welfare = 12;
        }
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let foreign = CountryId("BOR".into());
        for _ in 0..365 {
            if world.countries[&foreign]
                .cash_management
                .as_ref()
                .unwrap()
                .active()
                .is_some()
            {
                break;
            }
            world.advance(&content, crate::DaySpan::new(1)).unwrap();
        }
        let debt = world.countries[&foreign].debt;

        assert!(
            world.countries[&foreign]
                .cash_management
                .as_ref()
                .unwrap()
                .active()
                .is_some(),
            "day {} cash {:?} projects {:?} fiscal {:?} state {:?}; events {:?}",
            world.day,
            world.countries[&foreign].available_cash(),
            world.countries[&foreign].projects,
            world.countries[&foreign].fiscal_plans,
            world.countries[&foreign].cash_management,
            world
                .events
                .iter()
                .filter(|e| e.country == foreign)
                .rev()
                .take(8)
                .collect::<Vec<_>>()
        );
        assert!(world.countries[&foreign].retained_cash().0 < 80);
        for _ in 0..60 {
            world.advance(&content, crate::DaySpan::new(1)).unwrap();
            if world.countries[&foreign]
                .cash_management
                .as_ref()
                .unwrap()
                .active()
                .is_none()
            {
                break;
            }
        }
        assert!(
            world.countries[&foreign]
                .cash_management
                .as_ref()
                .unwrap()
                .active()
                .is_none()
        );
        assert!(world.journal.iter().any(|e| e.ai
            && e.command.issuer == foreign
            && matches!(e.command.action, Action::ReleaseCash { .. })));
        assert!(world.journal.iter().any(|e| e.ai
            && e.command.issuer == foreign
            && matches!(e.command.action, Action::Enact { .. })));
        assert_eq!(world.countries[&foreign].debt, debt);
        assert!(world.countries[&foreign].arrears.0 > 0);
        world.validate(&content).unwrap();
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
}
