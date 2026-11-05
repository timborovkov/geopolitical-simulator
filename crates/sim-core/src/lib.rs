//! Deterministic, daily geopolitical simulation. No wall clock, rendering or I/O.
mod budget;
mod cabinet;
mod calendar;
mod cash_management;
mod commitments;
mod constitution;
mod content;
mod developer;
mod elections;
mod event_references;
mod finance;
mod fiscal_plans;
mod governance;
mod households;
mod identity;
mod ids;
mod income_taxes;
mod interest;
mod knowledge;
mod living_standards;
mod markets;
mod model;
mod outlook;
mod party_preferences;
pub mod persistence;
mod rates;
mod reserves;
mod rules;
mod sovereignty;
mod subnational;
mod technologies;
mod territorial;
mod undertakings;
mod units;
pub use budget::*;
pub use cabinet::*;
pub use calendar::*;
pub use cash_management::*;
pub use commitments::*;
pub use constitution::*;
pub use content::*;
pub use developer::*;
pub use elections::*;
pub use event_references::*;
pub use finance::*;
pub use fiscal_plans::*;
pub use governance::*;
pub use households::*;
pub use identity::*;
pub use ids::*;
pub use income_taxes::*;
pub use interest::*;
pub use knowledge::*;
pub use living_standards::*;
pub use markets::*;
pub use model::*;
pub use outlook::*;
pub use party_preferences::*;
pub use rates::*;
pub use reserves::*;
pub use sovereignty::*;
pub use units::*;

/// FNV-1a, fixed byte order. A reproducibility checksum, not a security primitive.
pub fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

pub use subnational::*;
pub use territorial::*;
pub use undertakings::*;

pub use technologies::*;
