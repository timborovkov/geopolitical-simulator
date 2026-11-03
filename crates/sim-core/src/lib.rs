//! Deterministic, daily geopolitical simulation. No wall clock, rendering or I/O.
mod budget;
mod calendar;
mod content;
mod developer;
mod governance;
mod households;
mod identity;
mod knowledge;
mod model;
pub mod persistence;
mod rules;
mod units;
pub use budget::*;
pub use calendar::*;
pub use content::*;
pub use developer::*;
pub use governance::*;
pub use households::*;
pub use identity::*;
pub use knowledge::*;
pub use model::*;
pub use units::*;

/// FNV-1a, fixed byte order. A reproducibility checksum, not a security primitive.
pub fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
