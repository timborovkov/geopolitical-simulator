//! Optional daily interest accrual. Fractions are creditor claims, never cash.
use crate::*;
use serde::{Deserialize, Serialize};

pub const INTEREST_YEAR_DAYS: u32 = 365;
pub const INTEREST_RATE_BPS: u32 = 400;
pub const INTEREST_RATE: BasisPoints = BasisPoints::new(INTEREST_RATE_BPS);
pub const INTEREST_YEAR: DaySpan = DaySpan::new(INTEREST_YEAR_DAYS);
pub const INTEREST_DENOMINATOR: u32 = INTEREST_YEAR_DAYS * 10_000;
const MAX_PRINCIPAL: i64 = 1_000_000_000_000;

/// Preserved pre-carry convention: truncate the combined daily rate, then apply
/// the minimum whole charge for positive principal. No fractional carry is added.
pub(crate) fn minimum_daily_interest(principal: Money) -> Money {
    let whole = MoneyScale::ONE
        .basis_points(BasisPoints::new(400))
        .and_then(|scale| scale.spread_over(INTEREST_YEAR))
        .and_then(|scale| scale.apply(principal))
        .expect("bounded legacy interest");
    Money(whole.0.max(i64::from(principal.0 > 0)))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterestAccrualRule {
    #[serde(rename = "actual_365_carry")]
    Actual365Carry,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterestProjection {
    pub daily_numerator: u64,
    pub opening_remainder: u32,
}
impl InterestProjection {
    pub fn new(principal: Money, opening_remainder: u32) -> Result<Self, String> {
        if !(0..=MAX_PRINCIPAL).contains(&principal.0) || opening_remainder >= INTEREST_DENOMINATOR
        {
            return Err(
                "Interest requires bounded nonnegative principal and a sub-unit remainder".into(),
            );
        }
        Ok(Self {
            daily_numerator: principal.0 as u64 * u64::from(INTEREST_RATE_BPS),
            opening_remainder,
        })
    }
    pub fn valid(&self) -> bool {
        self.daily_numerator <= MAX_PRINCIPAL as u64 * u64::from(INTEREST_RATE_BPS)
            && self
                .daily_numerator
                .is_multiple_of(u64::from(INTEREST_RATE_BPS))
            && self.opening_remainder < INTEREST_DENOMINATOR
    }
    /// Whole cash units becoming due over the interval, with no daily minimum.
    pub fn due(&self, days: u32) -> Result<Money, String> {
        self.due_for(DaySpan::new(days))
    }
    /// Typed elapsed-day form of the scalar compatibility API.
    pub fn due_for(&self, days: DaySpan) -> Result<Money, String> {
        self.due_with_addition_for(days, Money(0), DaySpan::ZERO)
    }
    pub fn due_with_addition(
        &self,
        days: u32,
        additional: Money,
        additional_days: u32,
    ) -> Result<Money, String> {
        self.due_with_addition_for(
            DaySpan::new(days),
            additional,
            DaySpan::new(additional_days),
        )
    }
    pub fn due_with_addition_for(
        &self,
        days: DaySpan,
        additional: Money,
        additional_days: DaySpan,
    ) -> Result<Money, String> {
        if !self.valid() || !(0..=MAX_PRINCIPAL).contains(&additional.0) || additional_days > days {
            return Err("Invalid interest projection or conditional loan interval".into());
        }
        let carry = MoneyRemainder::new(
            u64::from(self.opening_remainder),
            u64::from(INTEREST_DENOMINATOR),
        )
        .expect("validated interest remainder");
        // valid() guarantees the legacy wire numerator is exactly principal × rate.
        let principal = Money((self.daily_numerator / u64::from(INTEREST_RATE.units())) as i64);
        let base = INTEREST_RATE
            .accrue(principal, days, INTEREST_YEAR, Some(carry))
            .ok_or("Interest projection overflow")?;
        // Retain the base fraction when adding a conditional loan's principal-days.
        // Adding two separately rounded cash amounts would lose creditor claims.
        let addition = INTEREST_RATE
            .accrue(
                additional,
                additional_days,
                INTEREST_YEAR,
                Some(base.remainder),
            )
            .ok_or("Interest projection overflow")?;
        Ok(Money(
            base.whole
                .0
                .checked_add(addition.whole.0)
                .ok_or("Interest projection overflow")?,
        ))
    }
    /// Display only; accounting and command decisions use the integer ratio.
    pub fn average_per_day(&self) -> f64 {
        self.daily_numerator as f64 / f64::from(INTEREST_DENOMINATOR)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterestAssessment {
    pub day: crate::CampaignDay,
    pub principal: Money,
    pub opening_remainder: u32,
    pub amount_due: Money,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccruedInterest {
    /// Sum of the actual principal at each daily assessment, in million-credit days.
    pub principal_days: u64,
    pub paid: Money,
    pub remainder: u32,
    pub last: Option<InterestAssessment>,
}
impl AccruedInterest {
    pub fn whole_assessed(&self) -> Money {
        Money(
            (i128::from(self.principal_days) * i128::from(INTEREST_RATE_BPS)
                / i128::from(INTEREST_DENOMINATOR)) as i64,
        )
    }
    fn assess(&mut self, principal: Money, day: u32) -> Money {
        let projection = InterestProjection::new(principal, self.remainder)
            .expect("validated principal and interest remainder");
        let amount_due = projection
            .due_for(DaySpan::new(1))
            .expect("bounded daily assessment");
        self.principal_days = self
            .principal_days
            .checked_add(principal.0 as u64)
            .expect("bounded campaign principal-days");
        self.last = Some(InterestAssessment {
            day: crate::CampaignDay::new(day),
            principal,
            opening_remainder: self.remainder,
            amount_due,
        });
        self.remainder = ((u64::from(self.remainder) + projection.daily_numerator)
            % u64::from(INTEREST_DENOMINATOR)) as u32;
        amount_due
    }
    pub(crate) fn validate(
        &self,
        world: &World,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &world.countries[id];
        let days = world.day.units() - content.start_day()?;
        let numerator = i128::from(self.principal_days) * i128::from(INTEREST_RATE_BPS);
        let whole = numerator / i128::from(INTEREST_DENOMINATOR);
        if i128::from(self.principal_days) > i128::from(days) * i128::from(MAX_PRINCIPAL)
            || self.paid.0 < 0
            || i128::from(self.paid.0) + i128::from(c.arrears_external.0) != whole
            || i128::from(self.remainder) != numerator % i128::from(INTEREST_DENOMINATOR)
        {
            return Err("Interest accrual does not reconcile to principal-days, paid cash and external arrears".into());
        }
        if days == 0 {
            return if self.last.is_none()
                && self.principal_days == 0
                && self.paid == Money(0)
                && self.remainder == 0
            {
                Ok(())
            } else {
                Err("Opening campaigns have no assessed interest".into())
            };
        }
        let last = self
            .last
            .as_ref()
            .ok_or("Missing daily interest assessment")?;
        let projection = InterestProjection::new(last.principal, last.opening_remainder)?;
        // Manual and monthly-AI finance commands dated today follow settlement.
        // Institutional approvals of earlier requests precede today's assessment.
        let repayments: i128 = world
            .journal
            .iter()
            .filter(|e| e.command.issuer == *id && e.command.day == world.day)
            .filter_map(|e| match e.command.action {
                Action::Repay { amount } => Some(i128::from(amount.0)),
                _ => None,
            })
            .sum();
        let issued_after: i128 = c
            .financing
            .as_ref()
            .expect("financing")
            .instruments
            .iter()
            .filter(|loan| {
                loan.id.0 > 0
                    && loan.issued_day == world.day
                    && world
                        .journal
                        .get(loan.id.0 as usize - 1)
                        .is_some_and(|e| e.command.day == world.day)
            })
            .map(|loan| i128::from(loan.original_principal.0))
            .sum();
        if last.day != world.day
            || i128::from(last.principal.0) != i128::from(c.debt.0) + repayments - issued_after
            || self.principal_days < last.principal.0 as u64
            || u64::from(last.opening_remainder)
                != (self.principal_days - last.principal.0 as u64) * u64::from(INTEREST_RATE_BPS)
                    % u64::from(INTEREST_DENOMINATOR)
            || projection.due_for(DaySpan::new(1))? != last.amount_due
            || c.report.interest.0 > last.amount_due.0
        {
            return Err(
                "Invalid interest assessment chronology, principal basis or amount due".into(),
            );
        }
        Ok(())
    }
}
impl World {
    pub(crate) fn assess_interest(&mut self, id: &CountryId) -> Money {
        let c = self.countries.get_mut(id).expect("country");
        if let Some(state) = c
            .financing
            .as_mut()
            .and_then(|f| f.interest_accrual.as_mut())
        {
            state.assess(c.debt, self.day.units())
        } else {
            minimum_daily_interest(c.debt)
        }
    }
    pub(crate) fn record_interest_payment(&mut self, id: &CountryId, paid: Money) {
        if let Some(state) = self
            .countries
            .get_mut(id)
            .expect("country")
            .financing
            .as_mut()
            .and_then(|f| f.interest_accrual.as_mut())
        {
            state.paid.0 = state
                .paid
                .0
                .checked_add(paid.0)
                .expect("bounded cumulative interest paid");
        }
    }
}
