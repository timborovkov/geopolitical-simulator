//! Distinct physical quantities. These are authored game units, not tonnes or MWh.
//! One day's baseline requirement is one unit per thousand residents.
//!
//! Mixing commodities is a compile-time error:
//! ```compile_fail
//! use sim_core::{Energy, Food};
//! let food = Food::new(5).unwrap();
//! food.checked_add(Energy::new(3).unwrap());
//! ```
use serde::{Deserialize, Serialize};

macro_rules! quantity {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq, PartialOrd, Ord)]
        #[serde(transparent)]
        pub struct $name(i64);

        impl $name {
            pub const ZERO: Self = Self(0);
            pub fn new(units: i64) -> Result<Self, &'static str> {
                if units < 0 {
                    Err("Physical quantities cannot be negative")
                } else {
                    Ok(Self(units))
                }
            }
            pub fn units(self) -> i64 {
                self.0
            }
            pub fn checked_add(self, other: Self) -> Option<Self> {
                self.0.checked_add(other.0).map(Self)
            }
            pub fn checked_sub(self, other: Self) -> Option<Self> {
                self.0.checked_sub(other.0).filter(|v| *v >= 0).map(Self)
            }
            pub fn saturating_sub(self, other: Self) -> Self {
                Self((self.0 - other.0).max(0))
            }
            /// Nonnegative rational scaling; rounds down once, with a wide
            /// intermediate. Invalid ratios and final overflow return None.
            pub fn scaled(self, numerator: i64, denominator: i64) -> Option<Self> {
                if numerator < 0 || denominator <= 0 {
                    return None;
                }
                let value = i128::from(self.0) * i128::from(numerator) / i128::from(denominator);
                i64::try_from(value).ok().map(Self)
            }
            /// Delivered fraction, bounded to 100%; zero demand is satisfied.
            pub fn coverage(self, demand: Self) -> i32 {
                if demand.0 == 0 {
                    100
                } else {
                    (i128::from(self.0.min(demand.0)) * 100 / i128::from(demand.0)) as i32
                }
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::new(i64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
quantity!(Energy);
quantity!(Food);
