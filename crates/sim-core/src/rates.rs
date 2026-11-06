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
    ($name:ident $(, $debug:ident)?) => {
        #[derive(
            Clone, Copy, $($debug,)? Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
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
scalar!(Percent, Debug);
scalar!(BasisPoints, Debug);
// A duration, not an absolute simulation date or a calendar-year mandate.
scalar!(DaySpan);
impl DaySpan {
    /// Scale a whole-day duration, rounding down once. A zero denominator or
    /// a result beyond the supported duration range returns None.
    pub fn scaled(self, numerator: u32, denominator: u32) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        u32::try_from(u64::from(self.0) * u64::from(numerator) / u64::from(denominator))
            .ok()
            .map(Self::new)
    }
}
// Configured durations retain the published numeric diagnostic and JSON forms.
impl std::fmt::Debug for DaySpan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
impl std::fmt::Display for DaySpan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

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

impl TryFrom<i32> for Percent {
    type Error = std::num::TryFromIntError;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        u32::try_from(value).map(Self)
    }
}

/// Compose rate and period factors before rounding money towards zero once.
/// Reduced fractions retain precision between factors; overflow returns None.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoneyScale {
    numerator: u64,
    denominator: u64,
}
impl MoneyScale {
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };
    pub fn percent(self, rate: Percent) -> Option<Self> {
        self.multiply(u64::from(rate.units()), 100)
    }
    pub fn basis_points(self, rate: BasisPoints) -> Option<Self> {
        self.multiply(u64::from(rate.units()), u64::from(BasisPoints::DENOMINATOR))
    }
    /// Uniformly spread an amount over a nonzero number of days.
    pub fn spread_over(self, period: DaySpan) -> Option<Self> {
        self.multiply(1, u64::from(period.units()))
    }
    pub fn apply(self, amount: Money) -> Option<Money> {
        let scaled = i128::from(amount.0).checked_mul(i128::from(self.numerator))?
            / i128::from(self.denominator);
        i64::try_from(scaled).ok().map(Money)
    }
    fn multiply(self, numerator: u64, denominator: u64) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        // Cross-cancel before multiplication so equal factors do not overflow.
        fn gcd(mut a: u64, mut b: u64) -> u64 {
            while b != 0 {
                let remainder = a % b;
                a = b;
                b = remainder;
            }
            a
        }
        let common = gcd(numerator, denominator);
        let numerator = numerator / common;
        let denominator = denominator / common;
        let left = gcd(self.numerator, denominator);
        let right = gcd(numerator, self.denominator);
        Some(Self {
            numerator: (self.numerator / left).checked_mul(numerator / right)?,
            denominator: (self.denominator / right).checked_mul(denominator / left)?,
        })
    }
}
