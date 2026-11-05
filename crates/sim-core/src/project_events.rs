//! Explicit event subjects for rules-23 programme inspection; never inferred from text.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventContext {
    General,
    Commitment {
        request: CommitmentId,
        stage: CommitmentStage,
    },
}
impl World {
    pub(crate) fn event_for_commitment(
        &mut self,
        country: &CountryId,
        request: CommitmentId,
        stage: CommitmentStage,
        text: impl Into<String>,
    ) {
        self.event_with_context(
            country,
            true,
            text,
            EventContext::Commitment { request, stage },
        );
    }

    pub(crate) fn validate_project_events(&self) -> Result<(), String> {
        for event in &self.events {
            self.commitment_for_event(event)?;
        }
        Ok(())
    }

    fn commitment_for_event(&self, event: &Event) -> Result<Option<&CommitmentRecord>, String> {
        if self.rules < 23 {
            return if event.context.is_none() {
                Ok(None)
            } else {
                Err("Programme event links require rules 23".into())
            };
        }
        match event.context {
            Some(EventContext::General) => Ok(None),
            Some(EventContext::Commitment { request, stage }) => {
                if !matches!(
                    stage,
                    CommitmentStage::Proposed
                        | CommitmentStage::Amended
                        | CommitmentStage::Implementing
                        | CommitmentStage::Operating
                        | CommitmentStage::Evaluated
                        | CommitmentStage::Refused
                        | CommitmentStage::Withdrawn
                ) {
                    return Err("Unsupported programme event stage".into());
                }
                let record = self
                    .countries
                    .get(&event.country)
                    .and_then(|country| country.commitment(&request))
                    .ok_or("Programme event does not identify an owned record")?;
                if !record
                    .milestones
                    .iter()
                    .any(|m| m.day == event.day && m.stage == stage)
                {
                    return Err("Programme event differs from recorded lifecycle".into());
                }
                Ok(Some(record))
            }
            None => Err("Missing programme event context".into()),
        }
    }

    /// Resolve an event's explicit programme link only within the viewer's own records.
    /// Unavailable, expired and foreign references share the same empty result.
    pub fn event_commitment(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &EventId,
    ) -> Result<Option<&CommitmentRecord>, String> {
        let Some(event) = self.event_by_reference(content, viewer, id)? else {
            return Ok(None);
        };
        self.commitment_for_event(event)
    }
}
