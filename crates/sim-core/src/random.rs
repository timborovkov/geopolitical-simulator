//! Versioned, indexed deterministic samples for authoritative rules.
//!
//! Supply is currently the only stochastic subsystem. Its stream is addressed
//! by campaign seed, stable country ID and zero-based 90-day period. The seed
//! and absolute day in snapshots reconstruct this index; no mutable cursor is
//! consumed by another country, UI action or AI decision. This is a hash-derived
//! sample, not a general-purpose statistical or cryptographic PRNG.
use crate::CountryId;

/// Preserve the published FNV-1a-64 mapping and exact UTF-8 key encoding.
/// Changing either the sample or its distribution requires a new rules version.
pub(crate) fn supply_sample_v2(seed: u64, country: &CountryId, period: u32) -> u64 {
    crate::stable_hash(format!("{seed}:{}:{period}:supply-v2", country.0).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supply_v2_vectors_pin_algorithm_encoding_and_index() {
        for (seed, country, period, expected) in [
            (0, "AUR", 0, 0x7ab78c3ff00a12cf),
            (17, "GBR", 0, 0x875845cb24ea6e54),
            (17, "GBR", 1, 0x7214a4f5c60a8bbf),
            (u64::MAX, "USA", 405, 0xc6fecf3d29f9ea10),
        ] {
            assert_eq!(
                supply_sample_v2(seed, &CountryId(country.into()), period),
                expected
            );
        }
    }
}
