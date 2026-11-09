//! Realized business-sector cash, derived from retained transfers without changing a campaign.
//! Capital receipts remain cash receipts: this statement does not claim accrual profit.
use crate::*;
use serde::{Deserialize, Serialize};

/// Counterparty order shared by daily totals and period totals.
pub const BUSINESS_COUNTERPARTIES: [Account; 3] =
    [Account::Households, Account::Treasury, Account::External];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessCashDay {
    pub day: CampaignDay,
    pub opening_cash: Money,
    pub receipts: [Money; 3],
    pub payments: [Money; 3],
    pub closing_cash: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessStatement {
    /// Array order for both period totals and every daily receipt/payment row.
    pub counterparties: [Account; 3],
    pub opening_cash: Money,
    pub receipts: [Money; 3],
    pub payments: [Money; 3],
    pub closing_cash: Money,
    pub days: Vec<BusinessCashDay>,
    pub transactions: Vec<LedgerEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessStatementReport {
    pub country: CountryId,
    pub first_day: CampaignDay,
    pub last_day: CampaignDay,
    /// Only the active government's authenticated view contains current business accounts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned: Option<BusinessStatement>,
}

fn money(value: i128) -> Result<Money, String> {
    i64::try_from(value)
        .map(Money)
        .map_err(|_| "Business cash statement exceeds the accounting range".into())
}
fn total(values: &[Money; 3]) -> i128 {
    values.iter().map(|v| i128::from(v.0)).sum()
}
fn checked_values(values: [i128; 3]) -> Result<[Money; 3], String> {
    Ok([money(values[0])?, money(values[1])?, money(values[2])?])
}
impl World {
    /// An inclusive 1–30 day window, clipped to the campaign bookmark, never
    /// extrapolated beyond the ordinary ledger's guaranteed retention.
    pub fn business_statement(
        &self,
        content: &Content,
        viewer: Viewer,
        country: &CountryId,
        days: DaySpan,
    ) -> Result<BusinessStatementReport, String> {
        if self.content_hash != content.hash() {
            return Err("Business statements require the campaign's exact content".into());
        }
        if !(1..=30).contains(&days.units()) {
            return Err("Choose a business statement period of 1 through 30 days".into());
        }
        let state = self
            .countries
            .get(country)
            .ok_or("Unknown business country")?;
        let first_day = CampaignDay::new(
            self.day
                .units()
                .saturating_sub(days.units() - 1)
                .max(content.start_day()?.units()),
        );
        if first_day > self.day {
            return Err("Business statement precedes its campaign bookmark".into());
        }
        let mut report = BusinessStatementReport {
            country: country.clone(),
            first_day,
            last_day: self.day,
            owned: None,
        };
        if country != &self.player || viewer != Viewer::Government(self.player.clone()) {
            return Ok(report);
        }
        let count = (self.day.units() - first_day.units() + 1) as usize;
        let mut receipts = vec![[0_i128; 3]; count];
        let mut payments = receipts.clone();
        let mut transactions = Vec::new();
        for entry in self.ledger.iter().filter(|e| {
            &e.country == country
                && e.day >= first_day
                && e.day <= self.day
                && (e.from == Account::Firms || e.to == Account::Firms)
        }) {
            let incoming = entry.to == Account::Firms;
            let counterparty = if incoming { entry.from } else { entry.to };
            let index = BUSINESS_COUNTERPARTIES
                .iter()
                .position(|a| *a == counterparty)
                .ok_or("Business cash statement contains an internal self-transfer")?;
            if entry.amount.0 <= 0 {
                return Err("Business cash statement requires positive recorded transfers".into());
            }
            let day = (entry.day.units() - first_day.units()) as usize;
            let values = if incoming {
                &mut receipts
            } else {
                &mut payments
            };
            values[day][index] += i128::from(entry.amount.0);
            transactions.push(entry.clone());
        }
        let mut closing = i128::from(state.accounts.firms.0);
        let mut rows = Vec::with_capacity(count);
        let mut all_receipts = [0_i128; 3];
        let mut all_payments = [0_i128; 3];
        for day in (0..count).rev() {
            let received = checked_values(receipts[day])?;
            let paid = checked_values(payments[day])?;
            let opening = closing - total(&received) + total(&paid);
            if opening < 0 || closing < 0 {
                return Err("Business transfers do not reconcile to nonnegative daily cash".into());
            }
            rows.push(BusinessCashDay {
                day: CampaignDay::new(first_day.units() + day as u32),
                opening_cash: money(opening)?,
                receipts: received,
                payments: paid,
                closing_cash: money(closing)?,
            });
            for index in 0..3 {
                all_receipts[index] += receipts[day][index];
                all_payments[index] += payments[day][index];
            }
            closing = opening;
        }
        rows.reverse();
        report.owned = Some(BusinessStatement {
            counterparties: BUSINESS_COUNTERPARTIES,
            opening_cash: money(closing)?,
            receipts: checked_values(all_receipts)?,
            payments: checked_values(all_payments)?,
            closing_cash: state.accounts.firms,
            days: rows,
            transactions,
        });
        Ok(report)
    }
}
