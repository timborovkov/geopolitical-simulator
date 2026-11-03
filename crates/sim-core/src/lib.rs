//! Deterministic, daily geopolitical simulation. No wall clock, rendering or I/O.
mod budget;
mod calendar;
mod constitution;
mod content;
mod developer;
mod finance;
mod governance;
mod households;
mod identity;
mod knowledge;
mod living_standards;
mod markets;
mod model;
mod outlook;
pub mod persistence;
mod rules;
mod units;
pub use budget::*;
pub use calendar::*;
pub use constitution::*;
pub use content::*;
pub use developer::*;
pub use finance::*;
pub use governance::*;
pub use households::*;
pub use identity::*;
pub use knowledge::*;
pub use living_standards::*;
pub use markets::*;
pub use model::*;
pub use outlook::*;
pub use units::*;

/// FNV-1a, fixed byte order. A reproducibility checksum, not a security primitive.
pub fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
