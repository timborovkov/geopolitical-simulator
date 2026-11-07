//! Annual output calibration in millions of constant-2000 USD-equivalent credits.
//! This is a monetary amount over the authored year, distinct from spendable cash.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct AnnualGdp(i64);
impl AnnualGdp {
    pub const fn new(units: i64) -> Result<Self, &'static str> {
        if units < 0 {
            Err("Annual GDP cannot be negative")
        } else {
            Ok(Self(units))
        }
    }
    /// Explicit extraction at existing period, ratio and display boundaries.
    pub const fn units(self) -> i64 {
        self.0
    }
}
impl<'de> Deserialize<'de> for AnnualGdp {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(i64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
impl std::fmt::Debug for AnnualGdp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
impl std::fmt::Display for AnnualGdp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
