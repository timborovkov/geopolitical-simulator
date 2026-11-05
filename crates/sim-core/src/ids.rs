//! Distinct software identities; display names never select an entity.
use serde::{Deserialize, Serialize};

/// A stable policy reference, distinct from a country or political party.
///
/// ```compile_fail
/// use sim_core::{Content, PartyId};
/// let content = Content::bundled().unwrap();
/// let party = PartyId("GBR:labour".into());
/// content.policy(&party);
/// ```
///
/// ```compile_fail
/// use sim_core::{Action, CountryId};
/// let action = Action::Enact { policy: CountryId("GBR".into()), value: 30 };
/// ```
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct PolicyId(pub String);

/// A stable political-party reference; it is independent of the party's name.
///
/// ```compile_fail
/// use sim_core::{Content, PolicyId};
/// let mut content = Content::bundled().unwrap();
/// content.countries[0].party(&PolicyId("services".into()));
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct PartyId(pub String);

/// A stable population group key, resolved within a country and independent of its display label.
///
/// ```compile_fail
/// use sim_core::{CountryDefinition, CurrencyId};
/// fn wrong_family(country: &CountryDefinition) {
///     country.household(&CurrencyId::from("USD"));
/// }
/// ```
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct HouseholdGroupId(pub String);

/// A primary-currency identity code in bookmark metadata. Money remains in the
/// game's reporting unit; this reference does not imply exchange-rate conversion.
///
/// ```compile_fail
/// use sim_core::{CountryDefinition, HouseholdGroupId};
/// fn wrong_family(country: &mut CountryDefinition) {
///     country.currency = HouseholdGroupId::from("workers");
/// }
/// ```
///
/// ```compile_fail
/// use sim_core::{CurrencyId, ReportingCurrency};
/// let reporting_unit: ReportingCurrency = CurrencyId::from("USD");
/// ```
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CurrencyId(pub String);

macro_rules! string_id {
    ($id:ident) => {
        impl $id {
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }
        impl From<String> for $id {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
        impl From<&str> for $id {
            fn from(value: &str) -> Self {
                Self(value.into())
            }
        }
        impl std::fmt::Display for $id {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
        impl PartialEq<str> for $id {
            fn eq(&self, value: &str) -> bool {
                self.as_str() == value
            }
        }
        impl PartialEq<&str> for $id {
            fn eq(&self, value: &&str) -> bool {
                self.as_str() == *value
            }
        }
    };
}
string_id!(PolicyId);
string_id!(PartyId);
string_id!(HouseholdGroupId);
string_id!(CurrencyId);

// Preserve existing policy diagnostics while retaining a distinct Rust type.
impl std::fmt::Debug for PolicyId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, formatter)
    }
}

// Keep published cohort/currency diagnostics identical to their string encoding.
impl std::fmt::Debug for HouseholdGroupId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, formatter)
    }
}
impl std::fmt::Debug for CurrencyId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, formatter)
    }
}

/// A campaign-global command reference, distinct from projects and request IDs.
/// The transparent number preserves every published save and envelope encoding.
///
/// ```compile_fail
/// use sim_core::{Command, LoanId};
/// fn wrong_family(command: &mut Command) {
///     command.id = LoanId(1);
/// }
/// ```
///
/// ```compile_fail
/// use sim_core::{CommandReceipt, CommitmentId};
/// fn wrong_family(receipt: &mut CommandReceipt) {
///     receipt.command = CommitmentId(1);
/// }
/// ```
///
/// ```compile_fail
/// use sim_core::{World, CommitmentId};
/// fn wrong_family(world: &World) {
///     world.journal_entry(&CommitmentId(1));
/// }
/// ```
#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CommandId(pub u64);

impl CommandId {
    pub fn checked_add(self, count: u64) -> Option<Self> {
        self.0.checked_add(count).map(Self)
    }
}
impl std::fmt::Display for CommandId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, formatter)
    }
}
impl std::fmt::Debug for CommandId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, formatter)
    }
}
