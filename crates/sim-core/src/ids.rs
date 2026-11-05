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

// Preserve existing policy diagnostics while retaining a distinct Rust type.
impl std::fmt::Debug for PolicyId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, formatter)
    }
}
