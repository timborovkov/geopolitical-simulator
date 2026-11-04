//! Read-only funding scenarios. Negative balances mean a funding gap, never
//! permission to overdraw an account. No proposed loan is treated as approved.
use crate::*;

/// Owner-only requirements for a future decision. This is neither an accepted
/// proposal nor a claim on future receipts; submission still needs full cash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyFundingPlan {
    pub policy: String,
    pub value: i32,
    pub preview: PolicyPreview,
    pub available_cash: Money,
    pub retained_cash: Money,
    pub shortfall: Money,
}
impl World {
    pub fn policy_funding_plan(
        &self,
        content: &Content,
        policy: &str,
        value: i32,
    ) -> Result<PolicyFundingPlan, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Funding plans require the campaign's exact content".into());
        }
        let preview =
            self.preview_policy_funding(content, &self.player, policy, value, false, false)?;
        let country = self.player_country();
        let available_cash = country.available_cash();
        if available_cash.0 < 0 {
            return Err("Invalid available treasury for funding plan".into());
        }
        Ok(PolicyFundingPlan {
            policy: policy.into(),
            value,
            shortfall: Money((preview.cost.0 - available_cash.0).max(0)),
            preview,
            available_cash,
            retained_cash: country.retained_cash(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoanAssumption {
    pub amount: Money,
    pub decision_day: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FundingPoint {
    pub day: u32,
    pub without_loan: Money,
    pub if_approved: Money,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FundingOutlook {
    pub available_cash: Money,
    pub daily_balance: Money,
    /// Whole days of the current deficit covered by available cash. None means
    /// a balanced or positive run rate, not a guarantee against future shortfalls.
    pub covered_days: Option<u64>,
    pub additional_daily_interest: Money,
    pub loan: Option<LoanAssumption>,
    pub principal_due: Money,
    pub conditional_principal_due: Money,
    pub points: Vec<FundingPoint>,
}
impl World {
    /// Owner-only, unchanged-policy scenario excluding shocks, price changes,
    /// arrears repayments, new commitments and scheduled policy delivery.
    /// Subtracts contractual dated principal, holding recurring interest fixed
    /// after repayment as a conservative unchanged-run-rate assumption.
    pub fn funding_outlook(
        &self,
        content: &Content,
        horizon: u32,
        loan: Option<LoanAssumption>,
    ) -> Result<FundingOutlook, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Funding outlook requires the campaign's exact content".into());
        }
        if horizon > 365
            || self
                .day
                .checked_add(horizon)
                .is_none_or(|day| day > MAX_DAY)
        {
            return Err("Funding outlook must stay within 365 days and the campaign limit".into());
        }
        let country = self
            .countries
            .get(&self.player)
            .ok_or("Missing controlled country")?;
        let definition = content
            .country(&self.player)
            .ok_or("Missing controlled-country definition")?;
        if !(0..=1_000_000_000_000).contains(&country.debt.0) {
            return Err("Invalid debt for funding outlook".into());
        }
        let available = country.available_cash().0;
        if available < 0 {
            return Err("Invalid available treasury for funding outlook".into());
        }
        let forecast = country.forecast(definition, content.income_taxes.as_ref());
        let balance = country.national_daily_balance(definition, &forecast).0;
        let mut additional_interest = 0;
        if let Some(assumption) = loan {
            if assumption.amount.0 <= 0
                || assumption.amount.0 > definition.gdp / 10
                || assumption.decision_day < self.day
                || assumption.decision_day > MAX_DAY
            {
                return Err("Invalid hypothetical loan amount or decision date".into());
            }
            let mut after_loan = country.clone();
            after_loan.debt.0 = country
                .debt
                .0
                .checked_add(assumption.amount.0)
                .ok_or("Hypothetical debt overflow")?;
            additional_interest = after_loan
                .forecast(definition, content.income_taxes.as_ref())
                .daily_interest
                .0
                - forecast.daily_interest.0;
        }
        let principal_due = country.financing.as_ref().map_or(Money(0), |state| {
            state.principal_due_through(self.day + horizon)
        });
        let assumed_maturity = loan.and_then(|assumption| {
            content
                .finance
                .as_ref()
                .and_then(|r| r.maturity_days)
                .and_then(|term| assumption.decision_day.checked_add(term))
        });
        if assumed_maturity.is_some_and(|day| day > MAX_DAY) {
            return Err("Hypothetical loan maturity exceeds the campaign calendar".into());
        }
        let conditional_principal_due = Money(
            principal_due.0
                + loan
                    .filter(|_| assumed_maturity.is_some_and(|day| day <= self.day + horizon))
                    .map_or(0, |assumption| assumption.amount.0),
        );
        let mut points = Vec::with_capacity(horizon as usize + 1);
        for offset in 0..=horizon {
            let day = self.day + offset;
            let due = if offset == 0 {
                0
            } else {
                country
                    .financing
                    .as_ref()
                    .map_or(0, |state| state.principal_due_through(day).0)
            };
            let baseline =
                i128::from(available) + i128::from(balance) * i128::from(offset) - i128::from(due);
            let mut if_approved = baseline;
            if let Some(assumption) = loan.filter(|assumption| day >= assumption.decision_day) {
                // A future approval precedes that day's settlement/interest;
                // an immediate manual draw is after today's settlement.
                let interest_days =
                    day - assumption.decision_day + u32::from(assumption.decision_day > self.day);
                if_approved += i128::from(assumption.amount.0)
                    - i128::from(additional_interest) * i128::from(interest_days);
            }
            if offset > 0 && assumed_maturity.is_some_and(|due| due <= day) {
                if_approved -= i128::from(loan.expect("assumed maturity requires loan").amount.0);
            }
            points.push(FundingPoint {
                day,
                without_loan: Money(
                    baseline
                        .try_into()
                        .map_err(|_| "Funding balance overflow")?,
                ),
                if_approved: Money(
                    if_approved
                        .try_into()
                        .map_err(|_| "Conditional funding balance overflow")?,
                ),
            });
        }
        Ok(FundingOutlook {
            available_cash: Money(available),
            daily_balance: Money(balance),
            covered_days: (balance < 0)
                .then(|| (i128::from(available) / -i128::from(balance)) as u64),
            additional_daily_interest: Money(additional_interest),
            loan,
            principal_due,
            conditional_principal_due,
            points,
        })
    }
}
