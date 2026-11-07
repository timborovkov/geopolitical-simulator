//! Rules-4 financing: lawful consent, explicit contested acts and debt records.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct LoanId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CreditorId(pub String);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct AuthorityId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BorrowingAuthority {
    pub id: AuthorityId,
    pub label: String,
    pub route: DecisionRoute,
    pub consent_days: DaySpan,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmergencyFinance {
    pub council_threshold: i32,
    pub approval_cost: i32,
    pub legislative_cost: i32,
    pub elite_cost: i32,
    pub council_cost: i32,
    pub cooldown_days: DaySpan,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinanceDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest_accrual: Option<InterestAccrualRule>,
    pub authorities: Vec<BorrowingAuthority>,
    /// Authored term for newly disbursed loans; opening calibration stays perpetual.
    /// ```compile_fail
    /// use sim_core::Content;
    /// fn cannot_assign_a_date(content: &mut Content, date: u32) {
    ///     content.finance.as_mut().unwrap().maturity_days = Some(date);
    /// }
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maturity_days: Option<DaySpan>,
    pub emergency: Option<EmergencyFinance>,
    pub source: String,
    pub source_version: String,
}
impl FinanceDefinition {
    pub fn authority(&self, route: &DecisionRoute) -> Option<&BorrowingAuthority> {
        self.authorities
            .iter()
            .find(|authority| &authority.route == route)
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if (content.rules >= 17) != self.interest_accrual.is_some() {
            return Err("Carried daily interest requires rules/content 17".into());
        }
        if (content.rules >= 15) != self.maturity_days.is_some()
            || self
                .maturity_days
                .is_some_and(|days| !(30..=3650).contains(&days.units()))
        {
            return Err("Dated loan principal requires rules 15 and a 30–3650 day term".into());
        }
        if self.authorities.len() > 6
            || self.source_version.is_empty()
            || !content
                .sources
                .iter()
                .any(|source| source.id == self.source)
        {
            return Err(
                "Financing requires bounded authority definitions and source provenance".into(),
            );
        }
        for (index, authority) in self.authorities.iter().enumerate() {
            if authority.id.0.is_empty()
                || authority.label.is_empty()
                || authority.consent_days.units() > 180
                || self.authorities[..index]
                    .iter()
                    .any(|other| other.id == authority.id || other.route == authority.route)
            {
                return Err("Invalid or duplicate borrowing authority".into());
            }
        }
        if let Some(emergency) = &self.emergency
            && (!(0..=100).contains(&emergency.council_threshold)
                || [
                    emergency.approval_cost,
                    emergency.legislative_cost,
                    emergency.elite_cost,
                    emergency.council_cost,
                ]
                .into_iter()
                .any(|cost| !(0..=100).contains(&cost))
                || !(30..=365).contains(&emergency.cooldown_days.units()))
        {
            return Err("Invalid emergency-financing constraints or political costs".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportingCurrency {
    Constant2000Credit,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebtOrigin {
    OpeningCalibration,
    AcceptedRequest { contested: bool },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebtInstrument {
    pub id: LoanId,
    pub creditor: CreditorId,
    pub currency: ReportingCurrency,
    pub original_principal: Money,
    pub outstanding: Money,
    /// Opening calibration is recognized at the bookmark, not a historical
    /// claim that the country's actual debt originated on that date.
    pub issued_day: crate::CampaignDay,
    /// Annual basis points; four percent is 400 basis points, not four.
    /// ```compile_fail
    /// use sim_core::{DebtInstrument, Percent};
    /// fn cannot_assign_percent_count(loan: &mut DebtInstrument) {
    ///     loan.annual_rate_bps = Percent::new(4).units();
    /// }
    /// ```
    pub annual_rate_bps: BasisPoints,
    /// None denotes perpetual principal, including the opening calibration.
    pub maturity_day: Option<crate::CampaignDay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maturity_repaid: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_maturity_payment_day: Option<crate::CampaignDay>,
    pub origin: DebtOrigin,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BorrowingRequest {
    pub id: LoanId,
    pub amount: Money,
    pub requested_day: crate::CampaignDay,
    pub decision_day: crate::CampaignDay,
    pub authority: AuthorityId,
    pub sponsor_government: crate::GovernmentId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Financing {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interest_accrual: Option<AccruedInterest>,
    pub instruments: Vec<DebtInstrument>,
    pub pending: Option<BorrowingRequest>,
    pub emergency_count: u32,
    pub next_emergency_day: crate::CampaignDay,
}
fn creditor(country: &CountryId) -> CreditorId {
    CreditorId(format!("external:{}", country.0))
}
impl Financing {
    pub fn new(country: &CountryDefinition, day: crate::CampaignDay) -> Self {
        Self {
            interest_accrual: None,
            instruments: vec![DebtInstrument {
                id: LoanId(0),
                creditor: creditor(&country.id),
                currency: ReportingCurrency::Constant2000Credit,
                original_principal: Money(country.gdp.units() / 3),
                outstanding: Money(country.gdp.units() / 3),
                issued_day: day,
                annual_rate_bps: BasisPoints::new(400),
                maturity_day: None,
                maturity_repaid: None,
                last_maturity_payment_day: None,
                origin: DebtOrigin::OpeningCalibration,
            }],
            pending: None,
            emergency_count: 0,
            next_emergency_day: day,
        }
    }
    pub fn with_rules(
        country: &CountryDefinition,
        day: crate::CampaignDay,
        rules: &FinanceDefinition,
    ) -> Self {
        let mut state = Self::new(country, day);
        state.interest_accrual = rules
            .interest_accrual
            .as_ref()
            .map(|_| AccruedInterest::default());
        state
    }
    /// Principal already due or becoming due by this date; no interest or perpetual debt.
    /// ```compile_fail
    /// use sim_core::{DaySpan, Financing};
    /// fn wrong_horizon(financing: &Financing) {
    ///     financing.principal_due_through(DaySpan::new(90));
    /// }
    /// ```
    pub fn principal_due_through(&self, day: crate::CampaignDay) -> Money {
        Money(
            self.instruments
                .iter()
                .filter(|loan| loan.maturity_day.is_some_and(|due| due <= day))
                .map(|loan| loan.outstanding.0)
                .sum(),
        )
    }
    pub(crate) fn repay(&mut self, amount: Money) {
        let mut remaining = amount.0;
        for instrument in &mut self.instruments {
            let paid = remaining.min(instrument.outstanding.0);
            instrument.outstanding.0 -= paid;
            remaining -= paid;
            if remaining == 0 {
                break;
            }
        }
        debug_assert_eq!(remaining, 0, "validated debt portfolio covers repayment");
    }
    pub(crate) fn validate(
        &self,
        world: &World,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let country = &world.countries[id];
        let definition = content.country(id).ok_or("Missing country for financing")?;
        let rules = content.finance.as_ref().ok_or("Missing financing rules")?;
        let start = content.start_day()?;
        if self.instruments.is_empty()
            || self.instruments.len() > world.journal.len() + 1
            || !matches!(self.instruments[0].origin, DebtOrigin::OpeningCalibration)
        {
            return Err("Invalid debt-instrument count".into());
        }
        let mut ids = BTreeSet::new();
        let mut total = 0_i64;
        let mut previous_issue = start;
        let mut repayments = world
            .journal
            .iter()
            .filter(|entry| &entry.command.issuer == id)
            .try_fold(0_i64, |sum, entry| {
                if let Action::Repay { amount } = entry.command.action {
                    sum.checked_add(amount.0)
                        .ok_or("Principal repayment overflow")
                } else {
                    Ok(sum)
                }
            })?;
        for (index, instrument) in self.instruments.iter().enumerate() {
            if !ids.insert(instrument.id)
                || instrument.creditor != creditor(id)
                || instrument.original_principal.0 <= 0
                || instrument.outstanding.0 < 0
                || instrument.outstanding.0 > instrument.original_principal.0
                || instrument.annual_rate_bps.units() != 400
                || instrument.maturity_day
                    != if index == 0 {
                        None
                    } else {
                        rules.maturity_days.and_then(|days| {
                            instrument
                                .issued_day
                                .checked_add(crate::DaySpan::new(days.units()))
                        })
                    }
                || instrument
                    .maturity_day
                    .is_some_and(|day| day > crate::CampaignDay::new(MAX_DAY))
                || instrument.issued_day < previous_issue
                || instrument.issued_day > world.day
            {
                return Err("Invalid debt terms, chronology, creditor or principal".into());
            }
            let scheduled = instrument.maturity_repaid.map_or(0, |paid| paid.0);
            if instrument.maturity_repaid.is_some() != instrument.maturity_day.is_some()
                || scheduled < 0
                || scheduled > instrument.original_principal.0
                || (scheduled > 0) != instrument.last_maturity_payment_day.is_some()
                || instrument.last_maturity_payment_day.is_some_and(|day| {
                    day > world.day || instrument.maturity_day.is_none_or(|due| day < due)
                })
            {
                return Err("Invalid dated principal repayment history".into());
            }
            let redeemed = repayments.min(instrument.original_principal.0 - scheduled);
            if instrument.outstanding.0 != instrument.original_principal.0 - scheduled - redeemed {
                return Err(
                    "Debt redemption differs from journaled FIFO principal repayments".into(),
                );
            }
            repayments -= redeemed;
            previous_issue = instrument.issued_day;
            total = total
                .checked_add(instrument.outstanding.0)
                .ok_or("Debt portfolio overflow")?;
            match &instrument.origin {
                DebtOrigin::OpeningCalibration
                    if index == 0
                        && instrument.id == LoanId(0)
                        && instrument.original_principal.0 == definition.gdp.units() / 3
                        && instrument.issued_day == start => {}
                DebtOrigin::AcceptedRequest { contested }
                    if index > 0
                        && instrument.id.0 > 0
                        && instrument.original_principal.0 <= definition.gdp.units() / 10 =>
                {
                    let entry = world
                        .journal
                        .get(instrument.id.0 as usize - 1)
                        .ok_or("Missing debt-origin command")?;
                    let amount = match &entry.command.action {
                        Action::Borrow { amount } if !contested => *amount,
                        Action::EmergencyBorrow { amount } if *contested => *amount,
                        _ => return Err("Debt origin differs from its accepted request".into()),
                    };
                    let delay = if *contested {
                        0
                    } else {
                        rules
                            .authority(
                                &world
                                    .governing_rules_at(content, id, entry.command.day)?
                                    .route,
                            )
                            .ok_or("Debt lacks legal authority")?
                            .consent_days
                            .units()
                    };
                    if &entry.command.issuer != id
                        || amount != instrument.original_principal
                        || entry.command.day.checked_add(crate::DaySpan::new(delay))
                            != Some(instrument.issued_day)
                    {
                        return Err(
                            "Debt principal, issuer or approval date differs from the journal"
                                .into(),
                        );
                    }
                }
                _ => return Err("Invalid opening debt or debt origin".into()),
            }
        }
        match (&self.interest_accrual, &rules.interest_accrual) {
            (None, None) => {}
            (Some(state), Some(_)) => state.validate(world, content, id)?,
            _ => return Err("Interest state differs from the campaign's financing rules".into()),
        }
        if repayments != 0 || total != country.debt.0 {
            return Err("Debt instruments do not reconcile to public principal".into());
        }
        if let Some(request) = &self.pending {
            let entry = request
                .id
                .0
                .checked_sub(1)
                .and_then(|index| world.journal.get(index as usize))
                .ok_or("Missing pending-loan command")?;
            let authority = rules
                .authority(&world.governing_rules(content, id)?.route)
                .ok_or("Pending loan lacks legal authority")?;
            if request.amount.0 <= 0
                || request.amount.0 > definition.gdp.units() / 10
                || authority.consent_days.units() == 0
                || request.authority != authority.id
                || request.requested_day != entry.command.day
                || Some(request.decision_day)
                    != request.requested_day.checked_add(authority.consent_days)
                || request.decision_day <= world.day
                || request.decision_day > crate::CampaignDay::new(MAX_DAY)
                || request.requested_day < start
                || request.sponsor_government == crate::GovernmentId(0)
                || request.sponsor_government > country.government_number
                || &entry.command.issuer != id
                || !matches!(entry.command.action, Action::Borrow { amount } if amount == request.amount)
                || self
                    .instruments
                    .iter()
                    .any(|instrument| instrument.id == request.id)
            {
                return Err("Invalid pending borrowing request".into());
            }
        }
        let emergencies: Vec<_> = world
            .journal
            .iter()
            .filter(|entry| {
                &entry.command.issuer == id
                    && matches!(entry.command.action, Action::EmergencyBorrow { .. })
            })
            .collect();
        if emergencies
            .iter()
            .any(|entry| !ids.contains(&LoanId(entry.command.id.0)))
        {
            return Err("Accepted emergency borrowing has no debt instrument".into());
        }
        let expected_next = if let Some(last) = emergencies.last() {
            last.command
                .day
                .checked_add(
                    rules
                        .emergency
                        .as_ref()
                        .ok_or("Contested borrowing lacks rules")?
                        .cooldown_days,
                )
                .ok_or("Emergency deadline overflow")?
        } else {
            start
        };
        if self.emergency_count as usize != emergencies.len()
            || self.next_emergency_day != expected_next
        {
            return Err("Emergency-financing audit differs from accepted commands".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct BorrowingPreview {
    pub amount: Money,
    pub authority: AuthorityId,
    pub route: String,
    pub consent_days: DaySpan,
    pub decision_day: crate::CampaignDay,
}

impl World {
    fn check_credit(&self, content: &Content, id: &CountryId, amount: Money) -> Result<(), String> {
        let country = self.countries.get(id).ok_or("Unknown borrowing country")?;
        if content
            .finance
            .as_ref()
            .and_then(|r| r.maturity_days)
            .is_some_and(|term| {
                self.day
                    .units()
                    .checked_add(term.units())
                    .is_none_or(|day| day > MAX_DAY)
            })
        {
            return Err("Loan maturity would exceed the campaign calendar".into());
        }
        let definition = content
            .country(id)
            .ok_or("Missing borrowing country definition")?;
        if amount.0 <= 0 || amount.0 > definition.gdp.units() / 10 {
            return Err(
                "Borrowing must be positive and at most 10% of opening GDP per issue".into(),
            );
        }
        if country
            .debt
            .0
            .checked_add(amount.0)
            .is_none_or(|debt| debt > definition.gdp.units() * 3 / 2)
        {
            return Err("Creditors refuse: debt ceiling is 150% of opening GDP".into());
        }
        if amount.0 > country.accounts.external.0 {
            return Err("External creditor lacks available funds".into());
        }
        Ok(())
    }
    pub fn preview_borrowing(
        &self,
        content: &Content,
        id: &CountryId,
        amount: Money,
    ) -> Result<BorrowingPreview, String> {
        self.check_credit(content, id, amount)?;
        let rules = content
            .finance
            .as_ref()
            .ok_or("This campaign uses immediate classic financing")?;
        let government = self.governing_rules(content, id)?;
        let country = &self.countries[id];
        if country
            .financing
            .as_ref()
            .ok_or("Missing financing state")?
            .pending
            .is_some()
        {
            return Err("A borrowing request already awaits a decision".into());
        }
        let authority = rules
            .authority(&government.route)
            .ok_or("This government has no legal borrowing authority")?;
        government.check(country, "borrow", 0)?;
        let decision_day = self
            .day
            .units()
            .checked_add(authority.consent_days.units())
            .filter(|day| *day <= MAX_DAY)
            .ok_or("No time remains for institutional loan approval")?;
        if rules.maturity_days.is_some_and(|term| {
            decision_day
                .checked_add(term.units())
                .is_none_or(|day| day > MAX_DAY)
        }) {
            return Err("Loan maturity would exceed the campaign calendar".into());
        }
        Ok(BorrowingPreview {
            amount,
            authority: authority.id.clone(),
            route: authority.label.clone(),
            consent_days: authority.consent_days,
            decision_day: crate::CampaignDay::new(decision_day),
        })
    }
    pub fn preview_emergency_borrowing(
        &self,
        content: &Content,
        id: &CountryId,
        amount: Money,
    ) -> Result<EmergencyFinance, String> {
        self.check_credit(content, id, amount)?;
        let rule = content
            .finance
            .as_ref()
            .and_then(|rules| rules.emergency.as_ref())
            .ok_or("Contested emergency financing is unavailable")?;
        let country = &self.countries[id];
        let financing = country
            .financing
            .as_ref()
            .ok_or("Missing financing state")?;
        if self.day.units() < (financing.next_emergency_day).units() {
            return Err(format!(
                "Emergency financing is on cooldown until {}",
                date(financing.next_emergency_day)
            ));
        }
        if country.council_support < rule.council_threshold {
            return Err(format!(
                "Contested financing requires effective council backing {}; current {}",
                rule.council_threshold, country.council_support
            ));
        }
        Ok(rule.clone())
    }
    fn disburse_loan(
        &mut self,
        content: &Content,
        id: &CountryId,
        loan: LoanId,
        amount: Money,
        contested: bool,
    ) {
        self.pay(
            id,
            Account::External,
            Account::Treasury,
            amount.0,
            if contested {
                "Contested emergency loan principal received"
            } else {
                "Approved external loan principal received"
            },
        );
        let country = self.countries.get_mut(id).expect("borrowing country");
        country.debt.0 += amount.0;
        country.budget.borrowing.0 += amount.0;
        country
            .financing
            .as_mut()
            .expect("financing state")
            .instruments
            .push(DebtInstrument {
                id: loan,
                creditor: creditor(id),
                currency: ReportingCurrency::Constant2000Credit,
                original_principal: amount,
                outstanding: amount,
                issued_day: self.day,
                annual_rate_bps: BasisPoints::new(400),
                maturity_day: (content
                    .finance
                    .as_ref()
                    .and_then(|r| r.maturity_days)
                    .map(|days| self.day.units() + days.units()))
                .map(crate::CampaignDay::new),
                maturity_repaid: content
                    .finance
                    .as_ref()
                    .and_then(|r| r.maturity_days)
                    .map(|_| Money(0)),
                last_maturity_payment_day: None,
                origin: DebtOrigin::AcceptedRequest { contested },
            });
        let terms = content
            .finance
            .as_ref()
            .and_then(|r| r.maturity_days)
            .map_or_else(
                || "voluntary principal repayment".into(),
                |days| {
                    format!(
                        "principal due {}",
                        date(crate::CampaignDay::new(self.day.units() + days.units()))
                    )
                },
            );
        self.event(id, true, format!("Loan #{} disbursed: {}m from the finite external creditor at 4% annual interest, with {terms}.{}", loan.0, amount.0, if contested { " Constitutional borrowing consent was bypassed through an explicit contested act." } else { " Institutional authority and financing were checked at disbursement." }));
    }
    /// Called after current tax/local settlement and before interest and national purchases.
    pub(crate) fn settle_maturities(&mut self, id: &CountryId) {
        let upcoming: Vec<_> =
            self.countries[id]
                .financing
                .as_ref()
                .map_or_else(Vec::new, |state| {
                    state
                        .instruments
                        .iter()
                        .filter(|loan| {
                            loan.outstanding.0 > 0
                                && loan.maturity_day
                                    == (self.day.units().checked_add(30))
                                        .map(crate::CampaignDay::new)
                        })
                        .map(|loan| (loan.id, loan.outstanding.0, loan.maturity_day.unwrap()))
                        .collect()
                });
        for (loan, principal, day) in upcoming {
            self.event(id,true,format!("Loan #{}: {principal}m principal is due in 30 days on {}. Review the repayment calendar and available cash; borrowing consent and refinancing are not guaranteed.",loan.0,date(day)));
        }
        let due: Vec<_> = self.countries[id]
            .financing
            .as_ref()
            .map_or_else(Vec::new, |state| {
                state
                    .instruments
                    .iter()
                    .filter(|loan| {
                        loan.outstanding.0 > 0
                            && loan.maturity_day.is_some_and(|day| day <= self.day)
                    })
                    .map(|loan| (loan.id, loan.outstanding.0, loan.maturity_day.unwrap()))
                    .collect()
            });
        for (loan, outstanding, maturity) in due {
            let paid = self.pay(
                id,
                Account::Treasury,
                Account::External,
                outstanding,
                "Maturing external principal repaid",
            );
            if paid > 0 {
                let country = self.countries.get_mut(id).expect("country");
                country.debt.0 -= paid;
                country.budget.principal_repaid.0 += paid;
                let record = country
                    .financing
                    .as_mut()
                    .unwrap()
                    .instruments
                    .iter_mut()
                    .find(|row| row.id == loan)
                    .unwrap();
                record.outstanding.0 -= paid;
                record.maturity_repaid.as_mut().unwrap().0 += paid;
                record.last_maturity_payment_day = Some(self.day);
            }
            if self.day == maturity || paid == outstanding {
                self.event(id, true, format!("Loan #{} principal due {}: paid {paid}m; {}m remains overdue principal. Overdue principal remains in debt and is not counted again as operating arrears. Refinancing requires a separately authorized loan; no automatic rollover.",loan.0,date(maturity),outstanding-paid));
            }
        }
    }
    pub(crate) fn request_borrowing(
        &mut self,
        content: &Content,
        id: &CountryId,
        loan: LoanId,
        amount: Money,
    ) -> Result<(), String> {
        let preview = self.preview_borrowing(content, id, amount)?;
        if preview.consent_days.units() == 0 {
            self.disburse_loan(content, id, loan, amount, false);
        } else {
            let country = self.countries.get_mut(id).expect("country");
            country.financing.as_mut().expect("financing state").pending = Some(BorrowingRequest {
                id: loan,
                amount,
                requested_day: self.day,
                decision_day: preview.decision_day,
                authority: preview.authority,
                sponsor_government: country.government_number,
            });
            self.event(id, true, format!("Requested loan #{}: {}m via {}. Decision due {}; pending funds are not treasury cash. Authority, debt ceiling and creditor cash will be rechecked.", loan.0, amount.0, preview.route, date(preview.decision_day)));
        }
        Ok(())
    }
    pub(crate) fn emergency_borrowing(
        &mut self,
        content: &Content,
        id: &CountryId,
        loan: LoanId,
        amount: Money,
    ) -> Result<(), String> {
        let rule = self.preview_emergency_borrowing(content, id, amount)?;
        let country = self.countries.get_mut(id).expect("country");
        let financing = country.financing.as_mut().expect("financing state");
        let count = financing
            .emergency_count
            .checked_add(1)
            .ok_or("Emergency counter exhausted")?;
        let next = self
            .day
            .units()
            .checked_add(rule.cooldown_days.units())
            .ok_or("Emergency cooldown overflow")?;
        financing.emergency_count = count;
        financing.next_emergency_day = crate::CampaignDay::new(next);
        country.approval = (country.approval - rule.approval_cost).max(0);
        for household in &mut country.households {
            household.approval = (household.approval - rule.approval_cost).max(0);
        }
        country.legislative_support = (country.legislative_support - rule.legislative_cost).max(0);
        country.elite_support = (country.elite_support - rule.elite_cost).max(0);
        country.council_support = (country.council_support - rule.council_cost).max(0);
        self.disburse_loan(content, id, loan, amount, true);
        self.event(id, true, format!("Contested financing costs: approval -{}, legislature -{}, elite -{}, council -{} (floored at zero). Next emergency borrowing is available after {}.", rule.approval_cost, rule.legislative_cost, rule.elite_cost, rule.council_cost, date(crate::CampaignDay::new(next))));
        Ok(())
    }
    pub(crate) fn cancel_borrowing(&mut self, id: &CountryId, loan: LoanId) -> Result<(), String> {
        let financing = self
            .countries
            .get_mut(id)
            .and_then(|country| country.financing.as_mut())
            .ok_or("This campaign has no borrowing-request workflow")?;
        if financing
            .pending
            .as_ref()
            .is_none_or(|request| request.id != loan)
        {
            return Err("No matching pending borrowing request".into());
        }
        financing.pending = None;
        self.event(
            id,
            true,
            format!(
                "Cancelled pending loan #{}. No principal was disbursed and no debt was created.",
                loan.0
            ),
        );
        Ok(())
    }
    pub(crate) fn resolve_borrowing(&mut self, content: &Content, id: &CountryId) {
        let request = self.countries[id]
            .financing
            .as_ref()
            .and_then(|state| state.pending.clone());
        let Some(request) = request.filter(|request| request.decision_day <= self.day) else {
            return;
        };
        let result = if self.countries[id].government_number != request.sponsor_government {
            Err("Sponsoring government has changed; the successor must submit its own borrowing request".into())
        } else {
            self.check_credit(content, id, request.amount)
                .and_then(|()| {
                    self.governing_rules(content, id)?
                        .check(&self.countries[id], "borrow", 0)
                })
        };
        self.countries
            .get_mut(id)
            .expect("country")
            .financing
            .as_mut()
            .expect("financing")
            .pending = None;
        match result {
            Ok(()) => self.disburse_loan(content, id, request.id, request.amount, false),
            Err(reason) => self.event(
                id,
                true,
                format!(
                    "Loan #{} refused at decision: {reason}. No principal or debt was created.",
                    request.id.0
                ),
            ),
        }
    }
}
