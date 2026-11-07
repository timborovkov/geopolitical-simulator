//! Campaign clock indices, with elapsed time kept in `DaySpan`.
use crate::DaySpan;
use serde::{Deserialize, Serialize};

/// An absolute day index counted from 2000-01-01. Campaign-specific calendar
/// limits remain validation rules; existing numeric snapshot fields are retained.
#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CampaignDay(u32);
impl CampaignDay {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }
    pub const fn units(self) -> u32 {
        self.0
    }
    pub fn checked_add(self, elapsed: DaySpan) -> Option<Self> {
        self.0.checked_add(elapsed.units()).map(Self)
    }
    pub fn elapsed_since(self, earlier: Self) -> Option<DaySpan> {
        self.0.checked_sub(earlier.0).map(DaySpan::new)
    }
}
impl std::fmt::Debug for CampaignDay {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
impl std::fmt::Display for CampaignDay {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_indices_preserve_numeric_wire_units_and_checked_elapsed_composition() {
        let start = CampaignDay::new(561);
        let end = start.checked_add(DaySpan::new(30)).unwrap();
        assert_eq!(end.units(), 591);
        assert_eq!(end.elapsed_since(start), Some(DaySpan::new(30)));
        assert_eq!(start.elapsed_since(end), None);
        let maximum = CampaignDay::new(u32::MAX);
        assert!(maximum.checked_add(DaySpan::new(1)).is_none());
        assert_eq!(format!("{maximum:?}"), "4294967295");
        assert_eq!(format!("{maximum}"), "4294967295");
        assert_eq!(serde_json::to_string(&maximum).unwrap(), "4294967295");
        assert_eq!(serde_json::from_str::<CampaignDay>("561").unwrap(), start);
        for invalid in ["-1", "1.5", "\"561\"", "4294967296"] {
            assert!(serde_json::from_str::<CampaignDay>(invalid).is_err());
        }
    }
}
