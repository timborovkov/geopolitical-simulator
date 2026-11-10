//! Attributed private employment contracts and non-cash wage claims.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivatePayrollDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub wage_share_bps: BasisPoints,
    pub countries: BTreeMap<CountryId, BTreeMap<HouseholdGroupId, PrivateEmploymentShares>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateEmploymentShares {
    pub private_jobs_bps: BasisPoints,
    pub public_jobs_bps: BasisPoints,
}
impl PrivatePayrollDefinition {
    pub(crate) fn validate(&self, content: &Content) -> Result<(), String> {
        if self.source_version.is_empty()
            || self.description.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
            || !(1..=10000).contains(&self.wage_share_bps.units())
            || self.countries.len() != content.countries.len()
        {
            return Err(
                "Private payroll requires bounded, attributed employment assumptions".into(),
            );
        }
        for c in &content.countries {
            let groups = self
                .countries
                .get(&c.id)
                .ok_or("Missing employment country")?;
            if groups.len() != c.households.len() || groups.len() > 64 {
                return Err("Employment must cover each household group exactly once".into());
            }
            for h in &c.households {
                let s = groups
                    .get(&h.id)
                    .ok_or("Missing household employment shares")?;
                if u64::from(s.private_jobs_bps.units()) + u64::from(s.public_jobs_bps.units())
                    > 10000
                {
                    return Err("Employment shares exceed the working-age population".into());
                }
            }
        }
        Ok(())
    }
}
fn checked(n: i128) -> Result<Money, String> {
    i64::try_from(n)
        .map(Money)
        .map_err(|_| "Payroll amount exceeds accounting range".into())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateEmploymentContract {
    pub group: HouseholdGroupId,
    pub opening_jobs: Population,
    pub jobs: Population,
    pub full_daily_payroll: Money,
    pub assessed: Money,
    pub paid: Money,
    pub unpaid: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivatePayrollReceipt {
    pub group: HouseholdGroupId,
    pub jobs: Population,
    pub opening_unpaid: Money,
    pub due: Money,
    pub paid: Money,
    pub arrears_paid: Money,
    pub current_paid: Money,
    pub closing_unpaid: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivatePayrollDay {
    pub operating_receipts_before: Money,
    pub public_receipts_before: Money,
    pub payments_before: Money,
    pub opening_cash: Money,
    pub public_service_due: Money,
    pub public_service_paid: Money,
    pub day: CampaignDay,
    pub groups: Vec<PrivatePayrollReceipt>,
    pub nonwage_paid: Money,
}
impl PrivatePayrollDay {
    pub fn paid(&self) -> Result<Money, String> {
        checked(self.groups.iter().map(|g| i128::from(g.paid.0)).sum())
    }
    pub fn arrears_paid(&self) -> Result<Money, String> {
        checked(
            self.groups
                .iter()
                .map(|g| i128::from(g.arrears_paid.0))
                .sum(),
        )
    }
    pub fn current_factors_paid(&self) -> Result<Money, String> {
        checked(
            self.groups
                .iter()
                .map(|g| i128::from(g.current_paid.0))
                .sum::<i128>()
                + i128::from(self.nonwage_paid.0),
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivatePayrollState {
    pub as_of: CampaignDay,
    pub contracts: Vec<PrivateEmploymentContract>,
    pub last: Option<PrivatePayrollDay>,
}
impl PrivatePayrollState {
    pub fn unpaid(&self) -> Result<Money, String> {
        checked(self.contracts.iter().map(|c| i128::from(c.unpaid.0)).sum())
    }
}
impl World {
    pub(crate) fn seed_private_payroll(&mut self, content: &Content) -> Result<(), String> {
        let Some(d) = &content.private_payroll else {
            return Ok(());
        };
        for (id, c) in &mut self.countries {
            let opening = content.country(id).ok_or("Unknown payroll country")?;
            let wages = checked(
                i128::from(opening.gdp.units() / 365 * 70 / 100)
                    * i128::from(d.wage_share_bps.units())
                    / 10000,
            )?;
            let amounts = apportion(
                wages.0,
                &opening
                    .households
                    .iter()
                    .map(|h| h.income_share)
                    .collect::<Vec<_>>(),
            );
            let contracts = c
                .households
                .iter()
                .zip(amounts)
                .map(|(h, amount)| {
                    let shares = &d.countries[id][&h.id];
                    let jobs = Population(
                        (u128::from(h.working_age.0) * u128::from(shares.private_jobs_bps.units())
                            / 10000) as u64,
                    );
                    PrivateEmploymentContract {
                        group: h.id.clone(),
                        opening_jobs: jobs,
                        jobs,
                        full_daily_payroll: Money(if jobs.0 == 0 { 0 } else { amount }),
                        assessed: Money(0),
                        paid: Money(0),
                        unpaid: Money(0),
                    }
                })
                .collect();
            c.private_payroll = Some(PrivatePayrollState {
                as_of: self.day,
                contracts,
                last: None,
            });
        }
        let ids: Vec<_> = self.countries.keys().cloned().collect();
        for id in ids {
            self.private_employment_counts(content, &id, Money(1), Money(1));
        }
        Ok(())
    }
    /// Wage claims are settled before non-wage factor income. Each household's
    /// older claims receive its payment first; available cash is apportioned
    /// over all currently eligible claims, with stable largest remainders.
    pub(crate) fn settle_private_payroll(
        &mut self,
        content: &Content,
        id: &CountryId,
        output: Money,
        weights: &[i64],
    ) -> Result<(Money, Money), String> {
        let d = content
            .private_payroll
            .as_ref()
            .ok_or("Missing payroll rules")?;
        let c = &self.countries[id];
        let s = c
            .private_payroll
            .as_ref()
            .ok_or("Missing employment contracts")?;
        let opening_cash = c.accounts.firms;
        let business = c
            .business
            .as_ref()
            .ok_or("Missing payroll business account")?;
        let (operating_receipts_before, public_receipts_before, payments_before) = (
            business.operating_receipts,
            business.public_receipts,
            business.payments,
        );
        let mut groups = Vec::new();
        let mut claims = Vec::new();
        for (contract, h) in s.contracts.iter().zip(&c.households) {
            let jobs = Population(contract.opening_jobs.0.min(h.working_age.0));
            let due = if contract.opening_jobs.0 == 0 {
                Money(0)
            } else {
                checked(
                    i128::from(contract.full_daily_payroll.0) * i128::from(jobs.0)
                        / i128::from(contract.opening_jobs.0),
                )?
            };
            let claim = checked(i128::from(contract.unpaid.0) + i128::from(due.0))?;
            claims.push(if h.population.0 == 0 { 0 } else { claim.0 });
            groups.push(PrivatePayrollReceipt {
                group: h.id.clone(),
                jobs,
                opening_unpaid: contract.unpaid,
                due,
                paid: Money(0),
                arrears_paid: Money(0),
                current_paid: Money(0),
                closing_unpaid: claim,
            });
        }
        let total = checked(claims.iter().map(|n| i128::from(*n)).sum())?;
        let paid = if total.0 == 0 {
            0
        } else {
            self.pay_income(
                id,
                Account::Firms,
                total.0,
                &claims,
                "Contractual private wages and prior wage claims",
            )
        };
        let payments = if total.0 == 0 {
            vec![0; claims.len()]
        } else {
            apportion(paid, &claims)
        };
        for (r, payment) in groups.iter_mut().zip(payments) {
            r.paid = Money(payment);
            r.arrears_paid = Money(payment.min(r.opening_unpaid.0));
            r.current_paid = Money(payment - r.arrears_paid.0);
            r.closing_unpaid.0 -= payment;
        }
        let nonwage_due = checked(
            i128::from(output.0 * 70 / 100) * i128::from(10000 - d.wage_share_bps.units()) / 10000,
        )?;
        let nonwage_paid = self.pay_income(
            id,
            Account::Firms,
            nonwage_due.0,
            weights,
            "Non-wage private factor income",
        );
        let day = PrivatePayrollDay {
            operating_receipts_before,
            public_receipts_before,
            payments_before,
            opening_cash,
            public_service_due: Money(0),
            public_service_paid: Money(0),
            day: self.day,
            groups,
            nonwage_paid: Money(nonwage_paid),
        };
        let current = day.current_factors_paid()?;
        let total_paid = checked(i128::from(paid) + i128::from(nonwage_paid))?;
        let s = self
            .countries
            .get_mut(id)
            .expect("country")
            .private_payroll
            .as_mut()
            .expect("payroll");
        for (contract, receipt) in s.contracts.iter_mut().zip(&day.groups) {
            contract.jobs = receipt.jobs;
            contract.assessed =
                checked(i128::from(contract.assessed.0) + i128::from(receipt.due.0))?;
            contract.paid = checked(i128::from(contract.paid.0) + i128::from(receipt.paid.0))?;
            contract.unpaid = receipt.closing_unpaid;
        }
        s.as_of = self.day;
        let opening_unpaid = checked(
            day.groups
                .iter()
                .map(|r| i128::from(r.opening_unpaid.0))
                .sum(),
        )?;
        s.last = Some(day);
        let closing_unpaid = s.unpaid()?;
        if opening_unpaid.0 == 0 && closing_unpaid.0 > 0 {
            self.event(id,true,"Private businesses could not pay all contractual wages. Unpaid household claims persist and protect cash before payouts or equipment investment.");
        }
        if opening_unpaid.0 > 0 && closing_unpaid.0 == 0 {
            self.event(id,false,"Private businesses have settled all retained wage claims in cash. These payments do not fund production a second time.");
        }
        Ok((total_paid, current))
    }
    pub(crate) fn private_employment_counts(
        &mut self,
        content: &Content,
        id: &CountryId,
        service_due: Money,
        service_paid: Money,
    ) {
        let Some(d) = &content.private_payroll else {
            return;
        };
        let c = self.countries.get_mut(id).expect("country");
        let s = c.private_payroll.as_mut().expect("payroll");
        if let Some(day) = &mut s.last {
            day.public_service_due = service_due;
            day.public_service_paid = service_paid;
        }
        for (h, contract) in c.households.iter_mut().zip(&s.contracts) {
            // Private jobs are contractual positions, including unpaid workers.
            // Public positions remain an authored funding-based estimate, not a
            // second private payroll or independently counted set of residents.
            let public_target = u128::from(h.working_age.0)
                * u128::from(d.countries[id][&h.id].public_jobs_bps.units())
                / 10000;
            let public_target =
                public_target.min(u128::from(h.working_age.0.saturating_sub(contract.jobs.0)));
            let public_paid = if service_due.0 <= 0 {
                0
            } else {
                public_target * service_paid.0.max(0) as u128 / service_due.0 as u128
            };
            h.employed = Population(
                (u128::from(contract.jobs.0) + public_paid).min(u128::from(h.working_age.0)) as u64,
            );
        }
    }
    pub fn private_payroll_report(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &CountryId,
    ) -> Result<Option<PrivatePayrollState>, String> {
        if self.content_hash != content.hash() {
            return Err("Payroll inspection requires exact content".into());
        }
        let c = self.countries.get(id).ok_or("Unknown payroll country")?;
        if viewer != Viewer::Government(self.player.clone()) || id != &self.player {
            return Ok(None);
        }
        self.validate_private_payroll(content)?;
        Ok(c.private_payroll.clone())
    }
    pub(crate) fn validate_private_payroll(&self, content: &Content) -> Result<(), String> {
        for (id, c) in &self.countries {
            match (&content.private_payroll, &c.private_payroll) {
                (None, None) => continue,
                (Some(definition), Some(s)) if self.rules == 40 => {
                    let opening = content.country(id).ok_or("Unknown employment country")?;
                    let opening_households =
                        crate::households::initial_households(opening, Money(0));
                    let total = checked(
                        i128::from(opening.gdp.units() / 365 * 70 / 100)
                            * i128::from(definition.wage_share_bps.units())
                            / 10000,
                    )?;
                    let wage_bills = apportion(
                        total.0,
                        &opening
                            .households
                            .iter()
                            .map(|h| h.income_share)
                            .collect::<Vec<_>>(),
                    );
                    if s.as_of != self.day
                        || s.contracts.len() != c.households.len()
                        || c.households.len() != opening.households.len()
                    {
                        return Err("Invalid payroll date or cohort count".into());
                    }
                    for (index, (contract, h)) in s.contracts.iter().zip(&c.households).enumerate()
                    {
                        if h.id != opening.households[index].id {
                            return Err("Employment household identity differs from content".into());
                        }
                        let jobs = Population(
                            (u128::from(opening_households[index].working_age.0)
                                * u128::from(
                                    definition.countries[id][&h.id].private_jobs_bps.units(),
                                )
                                / 10000) as u64,
                        );
                        if contract.opening_jobs != jobs
                            || contract.full_daily_payroll
                                != Money(if jobs.0 == 0 { 0 } else { wage_bills[index] })
                            || contract.jobs.0 != jobs.0.min(h.working_age.0)
                        {
                            return Err(
                                "Employment differs from attributed contracts and workforce".into(),
                            );
                        }
                        if contract.group != h.id
                            || contract.jobs.0 > contract.opening_jobs.0
                            || contract.jobs.0 > h.working_age.0
                            || [
                                contract.full_daily_payroll,
                                contract.assessed,
                                contract.paid,
                                contract.unpaid,
                            ]
                            .iter()
                            .any(|m| !(0..=1_000_000_000_000_000).contains(&m.0))
                            || i128::from(contract.assessed.0) - i128::from(contract.paid.0)
                                != i128::from(contract.unpaid.0)
                        {
                            return Err("Private payroll claims do not reconcile".into());
                        }
                    }
                    match &s.last {
                        None if self.day == content.start_day()?
                            && s.contracts
                                .iter()
                                .all(|g| g.assessed.0 == 0 && g.paid.0 == 0) => {}
                        Some(day)
                            if day.day == self.day && day.groups.len() == s.contracts.len() =>
                        {
                            let business = c
                                .business
                                .as_ref()
                                .ok_or("Missing payroll business account")?;
                            if [
                                day.operating_receipts_before,
                                day.public_receipts_before,
                                day.payments_before,
                            ]
                            .iter()
                            .any(|m| !(0..=1_000_000_000_000_000_000).contains(&m.0))
                                || day.operating_receipts_before.0 > business.operating_receipts.0
                                || day.public_receipts_before.0 > business.public_receipts.0
                                || i128::from(day.payments_before.0)
                                    + i128::from(day.paid()?.0)
                                    + i128::from(day.nonwage_paid.0)
                                    > i128::from(business.payments.0)
                                || i128::from(day.opening_cash.0)
                                    != i128::from(business.opening_cash.0)
                                        + i128::from(day.operating_receipts_before.0)
                                        + i128::from(day.public_receipts_before.0)
                                        - i128::from(day.payments_before.0)
                            {
                                return Err("Payroll cash checkpoint does not reconcile".into());
                            }
                            if !(0..=1_000_000_000_000_000_000).contains(&day.opening_cash.0)
                                || !(0..=1_000_000_000_000).contains(&day.public_service_due.0)
                                || !(0..=day.public_service_due.0)
                                    .contains(&day.public_service_paid.0)
                            {
                                return Err("Invalid payroll cash or public funding receipt".into());
                            }
                            let claims: Vec<i64> = day
                                .groups
                                .iter()
                                .zip(&c.households)
                                .map(|(r, h)| {
                                    if h.population.0 == 0 {
                                        Ok(0)
                                    } else {
                                        checked(
                                            i128::from(r.opening_unpaid.0) + i128::from(r.due.0),
                                        )
                                        .map(|m| m.0)
                                    }
                                })
                                .collect::<Result<_, String>>()?;
                            if claims.iter().any(|m| *m < 0) {
                                return Err("Negative eligible wage claim".into());
                            }
                            let total = checked(claims.iter().map(|m| i128::from(*m)).sum())?;
                            let paid = total.0.min(day.opening_cash.0);
                            let expected = if total.0 == 0 {
                                vec![0; claims.len()]
                            } else {
                                apportion(paid, &claims)
                            };
                            let nonwage_due = checked(
                                i128::from(c.report.output.0 * 70 / 100)
                                    * i128::from(10000 - definition.wage_share_bps.units())
                                    / 10000,
                            )?;
                            if day.nonwage_paid.0 != nonwage_due.0.min(day.opening_cash.0 - paid) {
                                return Err(
                                    "Non-wage settlement differs from output and available cash"
                                        .into(),
                                );
                            }
                            for ((r, g), (payment, h)) in day
                                .groups
                                .iter()
                                .zip(&s.contracts)
                                .zip(expected.into_iter().zip(&c.households))
                            {
                                let public_target = (u128::from(h.working_age.0)
                                    * u128::from(
                                        definition.countries[id][&h.id].public_jobs_bps.units(),
                                    )
                                    / 10000)
                                    .min(u128::from(h.working_age.0.saturating_sub(g.jobs.0)));
                                let public_paid = if day.public_service_due.0 == 0 {
                                    0
                                } else {
                                    public_target * day.public_service_paid.0 as u128
                                        / day.public_service_due.0 as u128
                                };
                                if r.paid.0 != payment
                                    || h.employed.0 != g.jobs.0 + public_paid as u64
                                {
                                    return Err("Payroll allocation or employment differs from actual funding".into());
                                }

                                let due = if g.opening_jobs.0 == 0 {
                                    Money(0)
                                } else {
                                    checked(
                                        i128::from(g.full_daily_payroll.0) * i128::from(g.jobs.0)
                                            / i128::from(g.opening_jobs.0),
                                    )?
                                };
                                if r.due != due {
                                    return Err(
                                        "Wage bill differs from contracted jobs and rate".into()
                                    );
                                }
                                if r.group != g.group
                                    || r.jobs != g.jobs
                                    || r.closing_unpaid != g.unpaid
                                    || [
                                        r.opening_unpaid,
                                        r.due,
                                        r.paid,
                                        r.arrears_paid,
                                        r.current_paid,
                                        r.closing_unpaid,
                                    ]
                                    .iter()
                                    .any(|m| !(0..=1_000_000_000_000_000).contains(&m.0))
                                    || i128::from(r.opening_unpaid.0) + i128::from(r.due.0)
                                        - i128::from(r.paid.0)
                                        != i128::from(r.closing_unpaid.0)
                                    || r.arrears_paid.0 != r.paid.0.min(r.opening_unpaid.0)
                                    || r.current_paid.0 != r.paid.0 - r.arrears_paid.0
                                    || r.due.0 > g.assessed.0
                                    || r.paid.0 > g.paid.0
                                {
                                    return Err("Private payroll receipt is invalid".into());
                                }
                            }
                            let opening_unpaid = checked(
                                day.groups
                                    .iter()
                                    .map(|g| i128::from(g.opening_unpaid.0))
                                    .sum(),
                            )?;
                            if c.business_costs
                                .as_ref()
                                .ok_or("Missing payroll cost book")?
                                .day_opening
                                .payroll_obligations
                                != Some(opening_unpaid)
                            {
                                return Err(
                                    "Opening payroll checkpoint differs from wage receipts".into(),
                                );
                            }
                            for (reason, expected) in [
                                (
                                    "Contractual private wages and prior wage claims",
                                    day.paid()?,
                                ),
                                ("Non-wage private factor income", day.nonwage_paid),
                            ] {
                                let actual = checked(
                                    self.ledger
                                        .iter()
                                        .filter(|e| {
                                            &e.country == id
                                                && e.day == self.day
                                                && e.from == Account::Firms
                                                && e.to == Account::Households
                                                && e.reason == reason
                                        })
                                        .map(|e| i128::from(e.amount.0))
                                        .sum(),
                                )?;
                                if actual != expected {
                                    return Err(
                                        "Payroll cash differs from the actual ledger".into()
                                    );
                                }
                            }
                            if !(0..=1_000_000_000_000_000).contains(&day.nonwage_paid.0)
                                || day.paid()?.0 + day.nonwage_paid.0 != c.report.wages.0
                            {
                                return Err("Private payroll omits actual factor payments".into());
                            }
                        }
                        _ => {
                            return Err(format!(
                                "Private payroll receipt missing or misdated for {id:?}"
                            ));
                        }
                    }
                }
                _ => return Err("Private payroll requires its exact rules".into()),
            }
        }
        Ok(())
    }
}
