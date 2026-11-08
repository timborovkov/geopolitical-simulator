//! Deterministic, daily geopolitical simulation. No wall clock, rendering or I/O.
mod ai_inspection;
mod annual_gdp;
mod budget;
mod cabinet;
mod calendar;
mod campaign_clock;
mod cash_management;
mod command_receipts;
mod commitments;
mod constitution;
mod content;
mod demographic_content;
mod demographic_runtime;
mod demographic_state;
mod demographics;
mod developer;
mod elections;
mod event_references;
mod executive_identity;
mod finance;
mod fiscal_plans;
mod governance;
mod households;
mod identity;
mod ids;
mod income_taxes;
mod institution_context;
mod interest;
mod knowledge;
mod legislation;
mod living_standards;
mod markets;
mod model;
mod outlook;
mod party_preferences;
pub mod persistence;
mod population_estimates;
mod project_events;
mod random;
mod rates;
pub mod replay_trace;
mod reserves;
mod rules;
mod sovereignty;
mod statistical_history;
mod stock_cover;
mod subnational;
mod technologies;
mod territorial;
mod undertakings;
mod units;
pub use ai_inspection::*;
pub use annual_gdp::*;
pub use budget::*;
pub use cabinet::*;
pub use calendar::*;
pub use campaign_clock::*;
pub use cash_management::*;
pub use command_receipts::*;
pub use commitments::*;
pub use constitution::*;
pub use content::*;
pub use demographic_content::*;
pub use demographic_runtime::*;
pub use demographic_state::*;
pub use demographics::*;
pub use developer::*;
pub use elections::*;
pub use event_references::*;
pub use executive_identity::*;
pub use finance::*;
pub use fiscal_plans::*;
pub use governance::*;
pub use households::*;
pub use identity::*;
pub use ids::*;
pub use income_taxes::*;
pub use institution_context::*;
pub use interest::*;
pub use knowledge::*;
pub use legislation::*;
pub use living_standards::*;
pub use markets::*;
pub use model::*;
pub use outlook::*;
pub use party_preferences::*;
pub use population_estimates::*;
pub use project_events::*;
pub use rates::*;
pub use reserves::*;
pub use sovereignty::*;
pub use statistical_history::*;
pub use stock_cover::*;
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
