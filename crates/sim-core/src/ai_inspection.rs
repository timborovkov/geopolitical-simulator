//! Read-only host inspection of recorded AI activity, separate from player knowledge.
use crate::{CountryId, Event, JournalEntry, World};
use serde::Serialize;

/// A bounded view of existing records. Commands are structurally automated;
/// `budget_bill` distinguishes reviewed foreign-budget dispatch from AI origin.
/// Reviews retain their original wording, with no reconstructed candidate scores
/// or inferred association between an event and a particular command.
#[derive(Debug, Serialize)]
pub struct DeveloperAiLog<'a> {
    pub country: &'a CountryId,
    pub player_controlled: bool,
    pub accepted_total: usize,
    pub reviews_available: usize,
    /// Newest first; at most the requested limit in each collection.
    pub commands: Vec<&'a JournalEntry>,
    /// Only retained messages with the current engine's explicit "AI " prefix.
    pub reviews: Vec<&'a Event>,
}
impl World {
    /// Live host truth. Clients must require their explicit developer capability
    /// before exposing this view; it is not a government-knowledge projection.
    pub fn developer_ai_log(
        &self,
        id: &CountryId,
        limit: usize,
    ) -> Result<DeveloperAiLog<'_>, String> {
        if !(1..=500).contains(&limit) {
            return Err("AI log limit must be 1..500 records per collection".into());
        }
        let country = &self.countries.get(id).ok_or("Unknown AI log country")?.id;
        let commands = self
            .journal
            .iter()
            .rev()
            .filter(|entry| entry.ai && &entry.command.issuer == id);
        let reviews = self
            .events
            .iter()
            .rev()
            .filter(|event| &event.country == id && event.text.starts_with("AI "));
        Ok(DeveloperAiLog {
            country,
            player_controlled: id == &self.player,
            accepted_total: commands.clone().count(),
            reviews_available: reviews.clone().count(),
            commands: commands.take(limit).collect(),
            reviews: reviews.take(limit).collect(),
        })
    }
}
