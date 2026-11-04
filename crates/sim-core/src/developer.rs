//! Trusted developer-host entry point. Never route remote/player input here.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandOrigin {
    Player,
    Ai,
    Developer,
    Budget(BudgetBillId),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeveloperField {
    Tax,
    Services,
    Welfare,
    EnergyCapacity,
    FoodCapacity,
    Administration,
    Approval,
    LegislativeSupport,
    EliteSupport,
    CouncilSupport,
}
impl DeveloperField {
    pub const ALL: [Self; 10] = [
        Self::Tax,
        Self::Services,
        Self::Welfare,
        Self::EnergyCapacity,
        Self::FoodCapacity,
        Self::Administration,
        Self::Approval,
        Self::LegislativeSupport,
        Self::EliteSupport,
        Self::CouncilSupport,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Tax => "tax",
            Self::Services => "services",
            Self::Welfare => "welfare",
            Self::EnergyCapacity => "energy_capacity",
            Self::FoodCapacity => "food_capacity",
            Self::Administration => "administration",
            Self::Approval => "approval",
            Self::LegislativeSupport => "legislative_support",
            Self::EliteSupport => "elite_support",
            Self::CouncilSupport => "council_support",
        }
    }
    pub fn from_name(name: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| format!("Unknown developer field {name}"))
    }
    pub fn bounds(self) -> std::ops::RangeInclusive<i32> {
        match self {
            Self::Tax => 10..=45,
            Self::Services => 4..=18,
            Self::Welfare => 0..=12,
            Self::EnergyCapacity | Self::FoodCapacity => 1..=200,
            Self::Administration => 1..=100,
            _ => 0..=100,
        }
    }
    pub fn read(self, country: &Country) -> i32 {
        match self {
            Self::Tax => country.tax,
            Self::Services => country.services,
            Self::Welfare => country.welfare,
            Self::EnergyCapacity => country.energy_capacity,
            Self::FoodCapacity => country.food_capacity,
            Self::Administration => country.administration,
            Self::Approval => country.approval,
            Self::LegislativeSupport => country.legislative_support,
            Self::EliteSupport => country.elite_support,
            Self::CouncilSupport => country.council_support,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeveloperEdit {
    pub field: DeveloperField,
    pub value: i32,
}
impl DeveloperEdit {
    pub fn validate(&self) -> Result<(), String> {
        if !self.field.bounds().contains(&self.value) {
            return Err(format!(
                "Developer {} must be within {:?}",
                self.field.name(),
                self.field.bounds()
            ));
        }
        Ok(())
    }
    pub(crate) fn apply(&self, country: &mut Country) {
        match self.field {
            DeveloperField::Tax => country.tax = self.value,
            DeveloperField::Services => country.services = self.value,
            DeveloperField::Welfare => country.welfare = self.value,
            DeveloperField::EnergyCapacity => country.energy_capacity = self.value,
            DeveloperField::FoodCapacity => country.food_capacity = self.value,
            DeveloperField::Administration => country.administration = self.value,
            DeveloperField::Approval => {
                country.approval = self.value;
                for household in &mut country.households {
                    household.approval = self.value;
                }
            }
            DeveloperField::LegislativeSupport => country.legislative_support = self.value,
            DeveloperField::EliteSupport => country.elite_support = self.value,
            DeveloperField::CouncilSupport => country.council_support = self.value,
        }
    }
}
impl World {
    /// Developer capability is assigned by the trusted host, not player input.
    /// Edits bypass policy costs/delays, are journaled and permanently mark this
    /// campaign. Normal execute/issue and AI cannot use this command origin.
    pub fn issue_developer(
        &mut self,
        content: &Content,
        target: CountryId,
        edit: DeveloperEdit,
    ) -> Result<(), String> {
        let command = self.command(self.player.clone(), Action::Developer { target, edit });
        self.apply(content, command, CommandOrigin::Developer)
    }
    pub fn is_developer_modified(&self) -> bool {
        self.journal.iter().any(|entry| entry.developer)
    }
}
pub(crate) fn is_false(value: &bool) -> bool {
    !value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_origin_cannot_issue_developer_commands() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let original = world.hash();
        let command = world.command(
            CountryId("CHN".into()),
            Action::Developer {
                target: CountryId("CHN".into()),
                edit: DeveloperEdit {
                    field: DeveloperField::Tax,
                    value: 35,
                },
            },
        );
        assert!(world.apply(&content, command, CommandOrigin::Ai).is_err());
        assert_eq!(world.hash(), original);
    }
}
