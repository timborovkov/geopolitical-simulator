//! Inventory coverage measured against a baseline daily physical requirement.
use serde::{Deserialize, Serialize};

/// A stock quantity expressed as multiples of a baseline daily physical need.
/// Buying three need-days requests three daily needs; it does not advance time
/// or promise three days of delivery. Food and energy remain distinct stocks.
#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct NeedDays(u32);
impl NeedDays {
    pub const fn new(units: u32) -> Self {
        Self(units)
    }
    pub const fn units(self) -> u32 {
        self.0
    }
}
impl std::fmt::Debug for NeedDays {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
impl std::fmt::Display for NeedDays {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
