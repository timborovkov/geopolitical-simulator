//! Aggregate household-owned enterprises. Cash distributions are not accrual profit.
use crate::*;
use serde::{Deserialize, Serialize};

/// A stable enterprise reference, independent of a government identity.
/// ```compile_fail
/// use sim_core::{BusinessId, CountryId};
/// let enterprise: BusinessId = CountryId("GBR".into());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct BusinessId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub distribution_bps: BasisPoints,
}
impl BusinessDefinition {
    pub(crate) fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=10000).contains(&self.distribution_bps.units())
            || self.source_version.is_empty()
            || self.description.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err(
                "Business distributions require a bounded rate and attributed assumptions".into(),
            );
        }
        Ok(())
    }
}
/// Attributed earnings-based payout policy. The distribution rate remains in
/// BusinessDefinition; no government action can order a private distribution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessFinanceDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
}
impl BusinessFinanceDefinition {
    pub(crate) fn validate(&self, content: &Content) -> Result<(), String> {
        if self.source_version.is_empty()
            || self.description.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
        {
            return Err("Business finance requires attributed payout assumptions".into());
        }
        Ok(())
    }
}
/// A completed-day earnings checkpoint and the actual cash constraints at review.
/// Retained earnings and cash availability are deliberately separate quantities.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessEarningsProtection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payroll_creditors: Option<Money>,
    pub closed_day: CampaignDay,
    pub checkpoint: BusinessProfitTotals,
    pub retained_earnings: Money,
    pub client_advances: Money,
    pub available_cash: Money,
}
impl BusinessEarningsProtection {
    fn available(cash: Money, buffer: Money, advances: Money) -> Money {
        Money((i128::from(cash.0) - i128::from(buffer.0) - i128::from(advances.0)).max(0) as i64)
    }
    fn available_with_payroll(
        cash: Money,
        buffer: Money,
        advances: Money,
        payroll: Option<Money>,
    ) -> Money {
        if payroll.is_none() {
            return Self::available(cash, buffer, advances);
        }
        Money(
            (i128::from(cash.0)
                - i128::from(buffer.0)
                - i128::from(advances.0)
                - i128::from(payroll.map_or(0, |m| m.0)))
            .max(0) as i64,
        )
    }
    fn validate(
        &self,
        decision: &BusinessDistribution,
        business: &BusinessSector,
    ) -> Result<(), String> {
        let a = &self.checkpoint;
        let expenses = a.expenses()?;
        if let Some(capital) = &a.private_capital {
            capital.validate()?;
        }
        let result = a.result()?;
        let fields = [
            a.sales,
            a.public_cash,
            a.deferred_capital,
            a.receivables,
            a.period_factors,
            a.food_sold_cost,
            a.energy_used_cost,
            a.food_spoiled_cost,
            a.unstocked_import_cost,
            a.inventory_value,
            a.distributions,
            self.client_advances,
            self.available_cash,
        ];
        if self
            .payroll_creditors
            .is_some_and(|m| !(0..=1_000_000_000_000_000).contains(&m.0))
            || fields
                .iter()
                .any(|m| !(0..=1_000_000_000_000_000_000).contains(&m.0))
            || self.closed_day.units().checked_add(1) != Some(decision.day.units())
            || a.distributions != decision.distributions_before
            || a.sales.0 > decision.operating_receipts_before.0
            || a.public_cash.0 > decision.public_receipts_before.0
            || a.deferred_capital.0 > a.public_cash.0
            || (i128::from(expenses.0)
                - i128::from(a.private_capital.as_ref().map_or(0, |x| x.depreciation.0))
                - i128::from(a.payroll_obligations.map_or(0, |m| m.0)))
                > i128::from(decision.payments_before.0) - i128::from(a.distributions.0)
            || i128::from(business.opening_cash.0) + i128::from(a.cash_change.0) < 0
            || i128::from(result.0)
                != i128::from(a.private_capital.as_ref().map_or(0, |x| x.net_assets.0))
                    + i128::from(a.cash_change.0)
                    + i128::from(a.distributions.0)
                    + i128::from(a.inventory_value.0)
                    + i128::from(a.receivables.0)
                    - i128::from(a.deferred_capital.0)
                    - i128::from(a.payroll_obligations.map_or(0, |m| m.0))
            || i128::from(self.retained_earnings.0)
                != i128::from(result.0) - i128::from(a.distributions.0)
            || self.client_advances.0 > decision.public_receipts_before.0
            || self.available_cash
                != Self::available_with_payroll(
                    decision.opening_cash,
                    decision.protected_cash,
                    self.client_advances,
                    self.payroll_creditors,
                )
        {
            return Err("Business earnings checkpoint does not reconcile".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessOwnerPayment {
    pub group: HouseholdGroupId,
    pub amount: Money,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BusinessDistributionReason {
    Paid,
    OperatingLoss,
    RetainedLoss,
    ClientAdvancesProtected,
    PaymentBuffer,
    OwnersAbsent,
    WholeCreditRounding,
}
impl BusinessDistributionReason {
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Paid => "Cash distribution paid to household owners",
            Self::RetainedLoss => "No positive retained modeled earnings at the previous day close",
            Self::ClientAdvancesProtected => {
                "Cash retained for unfinished client contracts and the payment buffer"
            }
            Self::OperatingLoss => "No positive undistributed operating cash",
            Self::PaymentBuffer => "Cash retained for the recent payment buffer",
            Self::OwnersAbsent => "An owning cohort has no residents; ownership is retained",
            Self::WholeCreditRounding => "Distribution rounds below 1m in the reporting unit",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessDistribution {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub earnings_protection: Option<BusinessEarningsProtection>,
    pub operating_receipts_before: Money,
    pub payments_before: Money,
    pub public_receipts_before: Money,
    pub distributions_before: Money,
    pub reason: BusinessDistributionReason,
    pub day: CampaignDay,
    pub opening_cash: Money,
    /// Actual trailing 30-day business payments, including earlier distributions.
    pub protected_cash: Money,
    /// Signed cash earned from households/external counterpart less all payments.
    pub undistributed_cash: Money,
    pub paid: Money,
    pub owners: Vec<BusinessOwnerPayment>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessSector {
    pub id: BusinessId,
    pub opening_cash: Money,
    /// Household sales and external receipts, never Treasury receipts.
    pub operating_receipts: Money,
    /// All payments, including owner distributions. This is a cash account, not costs.
    pub payments: Money,
    /// Government cash receipts, separately tracked from household/external sales.
    /// Legacy cash payouts exclude all; earnings payouts recognize earned services only.
    pub public_receipts: Money,
    pub distributions: Money,
    pub last_decision: Option<BusinessDistribution>,
}
impl BusinessSector {
    pub(crate) fn opening(country: &CountryId, cash: Money) -> Self {
        Self {
            id: BusinessId(format!("{}:civilian-enterprises", country.0)),
            opening_cash: cash,
            operating_receipts: Money(0),
            payments: Money(0),
            public_receipts: Money(0),
            distributions: Money(0),
            last_decision: None,
        }
    }
    pub fn undistributed_cash(&self) -> Money {
        Money(self.operating_receipts.0 - self.payments.0)
    }
}
impl Country {
    pub(crate) fn record_business_transfer(&mut self, from: Account, to: Account, paid: i64) {
        let Some(business) = &mut self.business else {
            return;
        };
        // Valid counters <= 10^18 and individual transfers <= 10^12 leave ample
        // i64 headroom. End-of-day validation rejects an exhausted counter atomically.
        if to == Account::Firms {
            if from == Account::Treasury {
                business.public_receipts.0 += paid;
            } else {
                business.operating_receipts.0 += paid;
            }
        }
        if from == Account::Firms {
            business.payments.0 += paid;
        }
    }
    pub(crate) fn validate_business_cash(
        &self,
        world: &World,
        content: &Content,
    ) -> Result<(), String> {
        let Some(b) = &self.business else {
            return if content.businesses.is_none() {
                Ok(())
            } else {
                Err("Missing business-sector account".into())
            };
        };
        if content.businesses.is_none()
            || !matches!(world.rules, 35..=40)
            || b.id != BusinessId(format!("{}:civilian-enterprises", self.id.0))
            || [
                b.opening_cash,
                b.operating_receipts,
                b.payments,
                b.public_receipts,
                b.distributions,
            ]
            .iter()
            .any(|m| !(0..=1_000_000_000_000_000_000).contains(&m.0))
            || b.opening_cash.0
                != content
                    .country(&self.id)
                    .ok_or("Missing business country")?
                    .gdp
                    .units()
                    / 2
            || b.distributions.0 > b.payments.0
            || (content.business_finance.is_none() && b.distributions.0 > b.operating_receipts.0)
            || i128::from(b.opening_cash.0)
                + i128::from(b.operating_receipts.0)
                + i128::from(b.public_receipts.0)
                - i128::from(b.payments.0)
                != i128::from(self.accounts.firms.0)
        {
            return Err("Business cash account does not reconcile".into());
        }
        Ok(())
    }
    pub(crate) fn validate_business(&self, world: &World, content: &Content) -> Result<(), String> {
        self.validate_business_cash(world, content)?;
        let Some(b) = &self.business else {
            return Ok(());
        };
        let elapsed = world.day.units() - content.start_day()?.units();
        match &b.last_decision {
            None if elapsed < 30 => {}
            Some(decision) if elapsed >= 30 => {
                let expected = world.day.units() - elapsed % 30;
                let definition = content
                    .country(&self.id)
                    .ok_or("Missing business country")?;
                let rate = i64::from(
                    content
                        .businesses
                        .as_ref()
                        .expect("validated definition")
                        .distribution_bps
                        .units(),
                );
                let (base, available) =
                    match (&decision.earnings_protection, &content.business_finance) {
                        (Some(protection), Some(_)) => {
                            protection.validate(decision, b)?;
                            if protection.checkpoint.payroll_obligations.is_some()
                                != content.private_payroll.is_some()
                            {
                                return Err("Payout payroll checkpoint version mismatch".into());
                            }
                            let creditors = self
                                .private_payroll
                                .as_ref()
                                .map(|s| s.unpaid())
                                .transpose()?;
                            if protection.payroll_creditors.is_some() != creditors.is_some()
                                || protection
                                    .payroll_creditors
                                    .is_some_and(|m| !(0..=1_000_000_000_000_000).contains(&m.0))
                                || (decision.day == world.day
                                    && protection.payroll_creditors != creditors)
                            {
                                return Err(
                                    "Payout payroll protection differs from wage claims".into()
                                );
                            }

                            if protection.checkpoint.private_capital
                                != self
                                    .private_investment
                                    .as_ref()
                                    .map(|s| s.totals_at(protection.closed_day))
                                    .transpose()?
                            {
                                return Err(
                                    "Payout fixed assets differ from closed equipment history"
                                        .into(),
                                );
                            }
                            (protection.retained_earnings, protection.available_cash)
                        }
                        (None, None) => (
                            decision.undistributed_cash,
                            Money(
                                decision
                                    .opening_cash
                                    .0
                                    .saturating_sub(decision.protected_cash.0)
                                    .max(0),
                            ),
                        ),
                        _ => return Err("Business payout policy/checkpoint mismatch".into()),
                    };
                let maximum = (i128::from(base.0.max(0)) * i128::from(rate) / 10000)
                    .min(i128::from(available.0));
                let ownership_weights: Vec<_> = definition
                    .households
                    .iter()
                    .map(|h| h.savings_share)
                    .collect();
                let expected_owner_payments = apportion(decision.paid.0.max(0), &ownership_weights);
                if decision.day.units() != expected
                    || decision.operating_receipts_before.0 < 0
                    || decision.operating_receipts_before.0 > b.operating_receipts.0
                    || decision.payments_before.0 < 0
                    || i128::from(decision.payments_before.0) + i128::from(decision.paid.0)
                        > i128::from(b.payments.0)
                    || decision.public_receipts_before.0 < 0
                    || decision.public_receipts_before.0 > b.public_receipts.0
                    || decision.distributions_before.0 < 0
                    || i128::from(decision.distributions_before.0) + i128::from(decision.paid.0)
                        != i128::from(b.distributions.0)
                    || i128::from(decision.undistributed_cash.0)
                        != i128::from(decision.operating_receipts_before.0)
                            - i128::from(decision.payments_before.0)
                    || i128::from(decision.opening_cash.0)
                        != i128::from(b.opening_cash.0)
                            + i128::from(decision.operating_receipts_before.0)
                            + i128::from(decision.public_receipts_before.0)
                            - i128::from(decision.payments_before.0)
                    || decision.protected_cash.0 > decision.payments_before.0
                    || decision.opening_cash.0 < 0
                    || decision.protected_cash.0 < 0
                    || decision.paid.0 < 0
                    || i128::from(decision.paid.0) > maximum
                    || (decision.paid.0 > 0)
                        != (decision.reason == BusinessDistributionReason::Paid)
                    || decision.paid.0 > b.distributions.0
                    || decision.owners.len() != definition.households.len()
                    || decision
                        .owners
                        .iter()
                        .zip(&definition.households)
                        .any(|(p, h)| p.group != h.id || p.amount.0 < 0)
                    || decision
                        .owners
                        .iter()
                        .zip(&expected_owner_payments)
                        .any(|(p, amount)| p.amount.0 != *amount)
                    || decision
                        .owners
                        .iter()
                        .map(|p| i128::from(p.amount.0))
                        .sum::<i128>()
                        != i128::from(decision.paid.0)
                {
                    return Err("Invalid autonomous business distribution".into());
                }
            }
            _ => return Err("Business decision date does not match its campaign".into()),
        }
        Ok(())
    }
}
impl World {
    pub(crate) fn validate_business_cash_accounts(&self, content: &Content) -> Result<(), String> {
        for country in self.countries.values() {
            country.validate_business_cash(self, content)?;
        }
        Ok(())
    }
    pub(crate) fn validate_business_accounts(&self, content: &Content) -> Result<(), String> {
        self.validate_private_investment(content)?;
        self.validate_private_payroll(content)?;
        for country in self.countries.values() {
            country.validate_business(self, content)?;
        }
        Ok(())
    }
    /// Bounded enterprise agency, independent of government consent. Every 30
    /// elapsed days, share a fraction of earned cash (legacy) or prior closed-day
    /// retained earnings (finance policy), while retaining actual
    /// trailing payments. Opening capital and every Treasury receipt stay outside
    /// earned cash in legacy rules. Finance rules also protect unfinished client
    /// advances. Household transfers enter ordinary withholding this day.
    pub(crate) fn distribute_business_cash(
        &mut self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let Some(definition) = &content.businesses else {
            return Ok(());
        };
        let elapsed = self.day.units() - content.start_day()?.units();
        if elapsed == 0 || !elapsed.is_multiple_of(30) {
            return Ok(());
        }
        let c = &self.countries[id];
        let business = c.business.as_ref().ok_or("Missing autonomous businesses")?;
        let operating_receipts_before = business.operating_receipts;
        let payments_before = business.payments;
        let public_receipts_before = business.public_receipts;
        let distributions_before = business.distributions;
        let undistributed_cash = business.undistributed_cash();
        let opening_cash = c.accounts.firms;
        let first = self.day.units().saturating_sub(29);
        let buffer: i128 = self
            .ledger
            .iter()
            .filter(|e| &e.country == id && e.from == Account::Firms && e.day.units() >= first)
            .map(|e| i128::from(e.amount.0))
            .sum();
        let protected_cash = Money(
            i64::try_from(buffer)
                .map_err(|_| "Business cash buffer exceeds the accounting range")?,
        );
        let weights: Vec<_> = content
            .country(id)
            .expect("country")
            .households
            .iter()
            .map(|h| h.savings_share)
            .collect();
        // Ownership does not move to other cohorts when an owning cohort has no
        // residents. Hold distributions pending an eventual ownership mechanism.
        let all_owners_present = c
            .households
            .iter()
            .zip(&weights)
            .all(|(h, weight)| *weight == 0 || h.population.0 > 0);
        let earnings_protection = if content.business_finance.is_some() {
            let checkpoint = c
                .business_costs
                .as_ref()
                .ok_or("Missing completed business earnings")?
                .day_opening
                .clone();
            let retained =
                i128::from(checkpoint.result()?.0) - i128::from(checkpoint.distributions.0);
            let (pending, _, _) = c.business_capital(content)?;
            let advances: i128 = pending.iter().map(|p| i128::from(p.amount.0)).sum();
            let client_advances = Money(
                i64::try_from(advances).map_err(|_| "Client advances exceed accounting range")?,
            );
            Some(BusinessEarningsProtection {
                closed_day: CampaignDay::new(self.day.units() - 1),
                checkpoint,
                retained_earnings: Money(
                    i64::try_from(retained)
                        .map_err(|_| "Retained earnings exceed accounting range")?,
                ),
                payroll_creditors: c.private_payroll.as_ref().map(|s| s.unpaid()).transpose()?,
                client_advances,
                available_cash: BusinessEarningsProtection::available_with_payroll(
                    opening_cash,
                    protected_cash,
                    client_advances,
                    c.private_payroll.as_ref().map(|s| s.unpaid()).transpose()?,
                ),
            })
        } else {
            None
        };
        let (base, available) = earnings_protection.as_ref().map_or(
            (
                undistributed_cash,
                Money(opening_cash.0.saturating_sub(protected_cash.0).max(0)),
            ),
            |p| (p.retained_earnings, p.available_cash),
        );
        let amount = if all_owners_present {
            (i128::from(base.0.max(0)) * i128::from(definition.distribution_bps.units()) / 10000)
                .min(i128::from(available.0)) as i64
        } else {
            0
        };
        let reason = if !all_owners_present {
            BusinessDistributionReason::OwnersAbsent
        } else if base.0 <= 0 {
            if earnings_protection.is_some() {
                BusinessDistributionReason::RetainedLoss
            } else {
                BusinessDistributionReason::OperatingLoss
            }
        } else if opening_cash.0 <= protected_cash.0 {
            BusinessDistributionReason::PaymentBuffer
        } else if available.0 == 0 {
            BusinessDistributionReason::ClientAdvancesProtected
        } else if amount == 0 {
            BusinessDistributionReason::WholeCreditRounding
        } else {
            BusinessDistributionReason::Paid
        };
        let paid = self.pay_income(
            id,
            Account::Firms,
            amount,
            &weights,
            "Autonomous business owner cash distribution",
        );
        let owners = apportion(paid, &weights)
            .into_iter()
            .zip(&self.countries[id].households)
            .map(|(amount, h)| BusinessOwnerPayment {
                group: h.id.clone(),
                amount: Money(amount),
            })
            .collect();
        let business = self
            .countries
            .get_mut(id)
            .expect("country")
            .business
            .as_mut()
            .expect("business");
        business.distributions.0 += paid;
        business.last_decision = Some(BusinessDistribution {
            earnings_protection,
            operating_receipts_before,
            payments_before,
            public_receipts_before,
            distributions_before,
            reason,
            day: self.day,
            opening_cash,
            protected_cash,
            undistributed_cash,
            paid: Money(paid),
            owners,
        });
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BusinessOwnershipReport {
    pub country: CountryId,
    pub source: Option<String>,
    /// Private current accounts and decisions are omitted from foreign/observer views.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned: Option<BusinessSector>,
}
impl World {
    pub fn business_ownership_report(
        &self,
        content: &Content,
        viewer: Viewer,
        country: &CountryId,
    ) -> Result<BusinessOwnershipReport, String> {
        if self.content_hash != content.hash() {
            return Err("Business ownership requires exact campaign content".into());
        }
        let c = self
            .countries
            .get(country)
            .ok_or("Unknown business country")?;
        Ok(BusinessOwnershipReport {
            country: country.clone(),
            source: content
                .business_finance
                .as_ref()
                .map(|d| d.source.clone())
                .or_else(|| content.businesses.as_ref().map(|d| d.source.clone())),
            owned: if country == &self.player && viewer == Viewer::Government(self.player.clone()) {
                c.business.clone()
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Content, World) {
        let content =
            Content::from_json(include_str!("../../../data/synthetic-businesses-test.json"))
                .unwrap();
        let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        (content, world)
    }
    #[test]
    fn liquidity_reserves_add_and_cannot_wrap_at_large_valid_amounts() {
        assert_eq!(
            BusinessEarningsProtection::available(Money(100), Money(35), Money(45)),
            Money(20)
        );
        assert_eq!(
            BusinessEarningsProtection::available(Money(100), Money(35), Money(75)),
            Money(0)
        );
        assert_eq!(
            BusinessEarningsProtection::available(Money(100), Money(i64::MAX), Money(i64::MAX)),
            Money(0)
        );
    }
    #[test]
    fn positive_earned_cash_cannot_breach_recent_payment_buffer() {
        let (content, mut world) = fixture();
        let id = world.player.clone();
        world.advance(&content, DaySpan::new(29)).unwrap();
        let gdp = content.country(&id).unwrap().gdp.units();
        // An explicit test-only sales/payroll shock, settled through actual cash
        // accounts. Positive earned cash need not imply sufficient liquidity.
        world.pay(
            &id,
            Account::External,
            Account::Firms,
            gdp,
            "Test sales shock",
        );
        world.pay_income(
            &id,
            Account::Firms,
            gdp * 9 / 10,
            &[1, 1, 1, 1],
            "Test payroll shock",
        );
        world.advance(&content, DaySpan::new(1)).unwrap();
        let decision = world
            .player_country()
            .business
            .as_ref()
            .unwrap()
            .last_decision
            .as_ref()
            .unwrap();
        assert!(decision.undistributed_cash.0 > 0);
        assert!(decision.protected_cash.0 > decision.opening_cash.0);
        assert_eq!(decision.paid, Money(0));
        world.validate(&content).unwrap();
    }
    #[test]
    fn loss_carry_blocks_payout_even_with_public_capital_and_large_bank_balance() {
        let (content, mut world) = fixture();
        let id = world.player.clone();
        world.advance(&content, DaySpan::new(29)).unwrap();
        let gdp = content.country(&id).unwrap().gdp.units();
        world.pay(
            &id,
            Account::Firms,
            Account::External,
            gdp / 4,
            "Test operating loss",
        );
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "income_tax".into(),
                    value: world.player_country().tax + 1,
                },
            )
            .unwrap();
        world.advance(&content, DaySpan::new(1)).unwrap();
        let b = world.player_country().business.as_ref().unwrap();
        let decision = b.last_decision.as_ref().unwrap();
        assert!(decision.opening_cash.0 > 0);
        assert!(decision.undistributed_cash.0 < 0);
        assert_eq!(decision.paid, Money(0));
        assert!(b.public_receipts.0 > 0);
        world.validate(&content).unwrap();
    }
}
