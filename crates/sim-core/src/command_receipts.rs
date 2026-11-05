//! Transient acknowledgements from the normal command authority.
use crate::*;
use serde::Serialize;

/// The accepted command and its retained immediate outcome references.
///
/// Receipts do not grant access to events or predict delayed delivery. Older
/// campaigns without event identities return an empty outcome list. References
/// remain subject to the campaign, owner and Chronicle retention boundaries.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CommandReceipt {
    pub command: u64,
    pub outcomes: Vec<EventId>,
}

impl World {
    /// Validate a normal command envelope and acknowledge its recorded outcomes.
    /// Trusted AI, budget and developer origins cannot be supplied in an envelope.
    pub fn execute_with_receipt(
        &mut self,
        content: &Content,
        command: Command,
    ) -> Result<CommandReceipt, String> {
        let id = command.id;
        let issuer = command.issuer.clone();
        let first = self
            .event_sequences
            .as_ref()
            .and_then(|sequences| sequences.get(&issuer))
            .copied();
        self.execute(content, command)?;
        let outcomes = self
            .events
            .iter()
            .filter_map(|event| event.id.as_ref())
            .filter(|event| {
                event.country == issuer && first.is_some_and(|first| event.sequence >= first)
            })
            .cloned()
            .collect();
        Ok(CommandReceipt {
            command: id,
            outcomes,
        })
    }

    /// Submit a current player action through the same envelope authority.
    pub fn issue_with_receipt(
        &mut self,
        content: &Content,
        action: Action,
    ) -> Result<CommandReceipt, String> {
        self.execute_with_receipt(content, self.command(self.player.clone(), action))
    }
}
