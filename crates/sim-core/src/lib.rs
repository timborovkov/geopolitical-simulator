//! Deterministic, daily geopolitical simulation. No wall clock, rendering or I/O.
mod budget;
mod content;
mod governance;
mod households;
mod model;
pub mod persistence;
mod rules;
pub use budget::*;
pub use content::*;
pub use governance::*;
pub use households::*;
pub use model::*;

/// FNV-1a, fixed byte order. A reproducibility checksum, not a security primitive.
pub fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
