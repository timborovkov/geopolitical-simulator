//! Campaign-scoped event references and transactional allocation for newer rules.
use crate::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, str::FromStr};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventId {
    pub country: CountryId,
    pub sequence: u64,
}
impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.country.0, self.sequence)
    }
}
impl FromStr for EventId {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        let (country, sequence) = value
            .trim()
            .rsplit_once('#')
            .ok_or("Use a country and event number, for example AUR#12")?;
        if country.is_empty()
            || country.chars().any(|c| c.is_whitespace() || c.is_control())
            || country.len() > 128
            || sequence.is_empty()
            || !sequence.bytes().all(|c| c.is_ascii_digit())
        {
            return Err("Use a country and positive event number, for example AUR#12".into());
        }
        let sequence = sequence
            .parse::<u64>()
            .map_err(|_| "Event number is outside its supported range")?;
        if sequence == 0 {
            return Err("Event numbers start at 1".into());
        }
        Ok(Self {
            country: CountryId(country.into()),
            sequence,
        })
    }
}
impl World {
    pub(crate) fn allocate_event_reference(
        &mut self,
        country: &CountryId,
    ) -> Result<Option<EventId>, String> {
        if self.rules < 22 {
            return Ok(None);
        }
        let next = self
            .event_sequences
            .as_mut()
            .and_then(|s| s.get_mut(country))
            .ok_or_else(|| format!("Missing event reference sequence for {}", country.0))?;
        let sequence = *next;
        *next = next
            .checked_add(1)
            .ok_or_else(|| format!("Event reference space exhausted for {}", country.0))?;
        Ok(Some(EventId {
            country: country.clone(),
            sequence,
        }))
    }

    pub(crate) fn validate_event_references(&self, content: &Content) -> Result<(), String> {
        self.validate_event_references_for_content(content, content.hash())
    }

    pub(crate) fn validate_event_references_for_content(
        &self,
        content: &Content,
        content_hash: u64,
    ) -> Result<(), String> {
        self.validate_project_events()?;
        if let Some(reason) = &self.event_error {
            return Err(reason.clone());
        }
        if self.rules < 22 {
            return if self.event_sequences.is_none() && self.events.iter().all(|e| e.id.is_none()) {
                Ok(())
            } else {
                Err("Stable event references require rules 22".into())
            };
        }
        if self.rules != content.rules || self.content_hash != content_hash {
            return Err("Event references require the campaign's exact content".into());
        }
        let sequences = self
            .event_sequences
            .as_ref()
            .ok_or("Missing event reference sequences")?;
        if sequences.keys().ne(self.countries.keys()) {
            return Err("Event reference countries differ from the campaign".into());
        }
        let emitted = sequences
            .values()
            .try_fold(0_u128, |sum, next| {
                next.checked_sub(1)
                    .and_then(|n| sum.checked_add(u128::from(n)))
            })
            .ok_or("Invalid event reference sequence")?;
        if self.events.len() as u128 != emitted.min(400) {
            return Err("Retained event references do not reconcile with their sequences".into());
        }
        let start = content.start_day()?;
        let mut previous = start;
        for event in &self.events {
            if event.day < previous
                || event.day > self.day
                || event.text.is_empty()
                || event.text.len() > 65536
                || !self.countries.contains_key(&event.country)
            {
                return Err("Invalid retained event date, country or text".into());
            }
            previous = event.day;
        }
        let mut expected: BTreeMap<_, _> = sequences.clone();
        for event in self.events.iter().rev() {
            let next = expected
                .get_mut(&event.country)
                .ok_or("Unknown event reference country")?;
            *next = next
                .checked_sub(1)
                .filter(|n| *n > 0)
                .ok_or("Invalid retained event reference number")?;
            if event.id.as_ref()
                != Some(&EventId {
                    country: event.country.clone(),
                    sequence: *next,
                })
            {
                return Err("Duplicate, missing or out-of-order event reference".into());
            }
        }
        Ok(())
    }

    /// Newer rules roll back a command or complete daily step if ID allocation fails.
    pub(crate) fn event_transaction<T>(
        &mut self,
        content: &Content,
        operation: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        self.event_transaction_for_content(content, content.hash(), operation)
    }

    /// Reuse the hash of an immutable content pack already checked at the boundary.
    pub(crate) fn event_transaction_for_content<T>(
        &mut self,
        content: &Content,
        content_hash: u64,
        operation: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        self.validate_event_references_for_content(content, content_hash)?;
        let before = self.clone();
        let result = operation(self).and_then(|value| {
            self.validate_event_references_for_content(content, content_hash)?;
            Ok(value)
        });
        if result.is_err() {
            *self = before;
        }
        result
    }

    /// A reference never grants access to another government's private event.
    pub fn event_by_reference(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &EventId,
    ) -> Result<Option<&Event>, String> {
        if self.rules != content.rules || self.content_hash != content.hash() {
            return Err("Event lookup requires the campaign's exact content".into());
        }
        if self.rules < 22 || self.event_sequences.is_none() {
            return Err("This campaign has no stable event references".into());
        }
        match viewer {
            Viewer::Observer => Ok(None),
            Viewer::Government(country) => {
                if !self.countries.contains_key(&country) {
                    return Err("Unknown viewer government".into());
                }
                Ok(self
                    .events
                    .iter()
                    .find(|e| id.country == country && e.id.as_ref() == Some(id)))
            }
        }
    }
}
