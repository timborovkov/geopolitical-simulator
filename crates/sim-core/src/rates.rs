//! Explicit rate and elapsed-time units. Conversion never silently rounds.
//!
//! Percentages and basis points are different units:
//! ```compile_fail
//! use sim_core::{BasisPoints, Percent};
//! let rate: BasisPoints = Percent::new(4);
//! ```
//! Elapsed days cannot be added to a rate:
//! ```compile_fail
//! use sim_core::{BasisPoints, DaySpan};
//! DaySpan::new(30).checked_add(BasisPoints::new(400));
//! ```
use crate::Money;
use serde::{Deserialize, Serialize};

macro_rules! scalar {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
        )]
        #[serde(transparent)]
        pub struct $name(u32);
        impl $name {
            pub const ZERO: Self = Self(0);
            pub const fn new(units: u32) -> Self {
                Self(units)
            }
            pub const fn units(self) -> u32 {
                self.0
            }
            pub fn checked_add(self, other: Self) -> Option<Self> {
                self.0.checked_add(other.0).map(Self)
            }
            pub fn checked_sub(self, other: Self) -> Option<Self> {
                self.0.checked_sub(other.0).map(Self)
            }
        }
    };
}
// Rates may exceed 100%. Policy-specific limits remain command/content rules.
scalar!(Percent);
scalar!(BasisPoints);
// A duration, not an absolute simulation date or a calendar-year mandate.
scalar!(DaySpan);

impl Percent {
    pub fn as_basis_points(self) -> Option<BasisPoints> {
        self.0.checked_mul(100).map(BasisPoints)
    }
    /// Signed money scaling rounds towards zero once, after wide multiplication.
    pub fn scale(self, amount: Money) -> Option<Money> {
        scale_money(amount, self.0, 100)
    }
}
impl BasisPoints {
    pub const DENOMINATOR: u32 = 10_000;
    pub fn as_exact_percent(self) -> Option<Percent> {
        self.0.is_multiple_of(100).then_some(Percent(self.0 / 100))
    }
    /// Signed money scaling rounds towards zero once, after wide multiplication.
    pub fn scale(self, amount: Money) -> Option<Money> {
        scale_money(amount, self.0, Self::DENOMINATOR)
    }
    /// Simple accrual on nonnegative principal, without compounding. Whole units
    /// round down; the exact residual is retained. Carry must use this period's
    /// denominator. A zero period, negative principal or overflow returns None.
    pub fn accrue(
        self,
        principal: Money,
        elapsed: DaySpan,
        period: DaySpan,
        carry: Option<MoneyRemainder>,
    ) -> Option<ProratedMoney> {
        if principal.0 < 0 || period == DaySpan::ZERO {
            return None;
        }
        let denominator = u64::from(Self::DENOMINATOR) * u64::from(period.0);
        let carry = match carry {
            Some(carry) if carry.denominator != denominator => return None,
            Some(carry) => carry.numerator,
            None => 0,
        };
        let numerator = i128::from(principal.0)
            .checked_mul(i128::from(self.0))?
            .checked_mul(i128::from(elapsed.0))?
            .checked_add(i128::from(carry))?;
        Some(ProratedMoney {
            whole: Money(i64::try_from(numerator / i128::from(denominator)).ok()?),
            remainder: MoneyRemainder {
                numerator: (numerator % i128::from(denominator)) as u64,
                denominator,
            },
        })
    }
}
fn scale_money(amount: Money, rate: u32, denominator: u32) -> Option<Money> {
    let value = i128::from(amount.0) * i128::from(rate) / i128::from(denominator);
    i64::try_from(value).ok().map(Money)
}

/// A sub-unit claim, never spendable cash. The denominator is part of its unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoneyRemainder {
    numerator: u64,
    denominator: u64,
}
impl MoneyRemainder {
    pub fn new(numerator: u64, denominator: u64) -> Option<Self> {
        (denominator > 0 && numerator < denominator).then_some(Self {
            numerator,
            denominator,
        })
    }
    pub fn numerator(self) -> u64 {
        self.numerator
    }
    pub fn denominator(self) -> u64 {
        self.denominator
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProratedMoney {
    pub whole: Money,
    pub remainder: MoneyRemainder,
}
