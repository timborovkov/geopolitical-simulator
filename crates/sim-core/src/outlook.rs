//! Read-only funding scenarios. Negative balances mean a funding gap, never
//! permission to overdraw an account. No proposed loan is treated as approved.
use crate::*;

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
    pub points: Vec<FundingPoint>,
}
impl World {
    /// Owner-only, unchanged-policy scenario excluding shocks, price changes,
    /// arrears repayments, new commitments and scheduled policy delivery.
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
        let forecast = country.forecast(definition);
        let balance = forecast.daily_revenue.0
            - country.regional_tax_entitlement(forecast.daily_revenue.0)
            - forecast.daily_spending.0
            + country.regional_planned_spending(definition)
            - forecast.daily_interest.0;
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
            additional_interest =
                after_loan.forecast(definition).daily_interest.0 - forecast.daily_interest.0;
        }
        let mut points = Vec::with_capacity(horizon as usize + 1);
        for offset in 0..=horizon {
            let day = self.day + offset;
            let baseline = i128::from(available) + i128::from(balance) * i128::from(offset);
            let mut if_approved = baseline;
            if let Some(assumption) = loan.filter(|assumption| day >= assumption.decision_day) {
                // A future approval precedes that day's settlement/interest;
                // an immediate manual draw is after today's settlement.
                let interest_days =
                    day - assumption.decision_day + u32::from(assumption.decision_day > self.day);
                if_approved += i128::from(assumption.amount.0)
                    - i128::from(additional_interest) * i128::from(interest_days);
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
            points,
        })
    }
}
