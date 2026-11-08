//! Nonexclusive social memberships projected from an explicit joint partition.
//! People are counted in disjoint cells once; membership totals are not a
//! population partition and must never be added to national resident accounts.
use crate::{BasisPoints, HouseholdGroupId, Population};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

/// A social membership is independent of a household's economic cohort.
/// ```compile_fail
/// use sim_core::{SocialGroupId, HouseholdGroupId};
/// let parents: SocialGroupId = HouseholdGroupId("workers".into());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SocialGroupId(pub String);
/// A joint cell can contain several memberships, and is not a party identity.
/// ```compile_fail
/// use sim_core::{SocialCellId, PartyId};
/// let joint: SocialCellId = PartyId("party".into());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SocialCellId(pub String);
impl From<&str> for SocialGroupId {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}
impl From<&str> for SocialCellId {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}
impl fmt::Display for SocialGroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl fmt::Display for SocialCellId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocialGroupDefinition {
    pub id: SocialGroupId,
    pub name: String,
    pub description: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocialCellDefinition {
    pub id: SocialCellId,
    /// Empty means no membership in this catalogue, not an unknown reference.
    pub memberships: BTreeSet<SocialGroupId>,
    pub share: BasisPoints,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocialMembershipProfile {
    pub cells: Vec<SocialCellDefinition>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocialCellCount {
    pub id: SocialCellId,
    pub memberships: BTreeSet<SocialGroupId>,
    pub population: Population,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlappingMemberships {
    /// Unique residents, independent of how many memberships each one has.
    pub population: Population,
    pub known_groups: BTreeSet<SocialGroupId>,
    pub cells: Vec<SocialCellCount>,
}
fn stable_atom(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b':' | b'_' | b'-')
        })
}
impl SocialMembershipProfile {
    pub fn validate(
        &self,
        catalogue: &BTreeMap<SocialGroupId, SocialGroupDefinition>,
    ) -> Result<(), String> {
        if catalogue.len() > 64 || self.cells.is_empty() || self.cells.len() > 128 {
            return Err(
                "Social membership profiles require bounded catalogues and nonempty joint cells"
                    .into(),
            );
        }
        for (id, group) in catalogue {
            if id != &group.id
                || !stable_atom(&id.0)
                || group.name.trim().is_empty()
                || group.name.len() > 256
                || group.description.trim().is_empty()
                || group.description.len() > 2048
            {
                return Err("Invalid social group identity or description".into());
            }
        }
        let mut ids = BTreeSet::new();
        let mut joints = BTreeSet::new();
        let mut shares = 0_u32;
        for cell in &self.cells {
            if !stable_atom(&cell.id.0)
                || !ids.insert(&cell.id)
                || !joints.insert(&cell.memberships)
                || cell.share.units() > 10000
                || cell
                    .memberships
                    .iter()
                    .any(|id| !catalogue.contains_key(id))
            {
                return Err("Joint membership cells need unique stable identities/combinations, known groups and bounded shares".into());
            }
            shares = shares
                .checked_add(cell.share.units())
                .ok_or("Social membership share overflow")?;
        }
        if shares != 10000 {
            return Err(
                "Joint membership shares must account for every resident exactly once".into(),
            );
        }
        Ok(())
    }
    /// Deterministic largest remainder, breaking ties by stable cell identity.
    /// These are explicit joint shares, not an assumption that memberships are
    /// independent or historical national group prevalence is known.
    pub fn project(
        &self,
        population: Population,
        catalogue: &BTreeMap<SocialGroupId, SocialGroupDefinition>,
    ) -> Result<OverlappingMemberships, String> {
        self.validate(catalogue)?;
        let mut cells: Vec<_> = self
            .cells
            .iter()
            .map(|cell| {
                let numerator = u128::from(population.0) * u128::from(cell.share.units());
                (cell, (numerator / 10000) as u64, numerator % 10000)
            })
            .collect();
        cells.sort_by(|a, b| a.0.id.cmp(&b.0.id));
        let used: u128 = cells.iter().map(|(_, count, _)| u128::from(*count)).sum();
        let remainder = u128::from(population.0)
            .checked_sub(used)
            .ok_or("Joint membership allocation exceeds residents")?;
        if remainder > cells.len() as u128 {
            return Err("Invalid joint membership remainder".into());
        }
        let mut order: Vec<_> = (0..cells.len()).collect();
        order.sort_by(|a, b| {
            cells[*b]
                .2
                .cmp(&cells[*a].2)
                .then_with(|| cells[*a].0.id.cmp(&cells[*b].0.id))
        });
        for index in order.into_iter().take(remainder as usize) {
            cells[index].1 = cells[index]
                .1
                .checked_add(1)
                .ok_or("Joint membership count overflow")?;
        }
        Ok(OverlappingMemberships {
            population,
            known_groups: catalogue.keys().cloned().collect(),
            cells: cells
                .into_iter()
                .map(|(cell, count, _)| SocialCellCount {
                    id: cell.id.clone(),
                    memberships: cell.memberships.clone(),
                    population: Population(count),
                })
                .collect(),
        })
    }
}
impl OverlappingMemberships {
    pub fn validate(
        &self,
        profile: &SocialMembershipProfile,
        catalogue: &BTreeMap<SocialGroupId, SocialGroupDefinition>,
    ) -> Result<(), String> {
        if &profile.project(self.population, catalogue)? != self {
            return Err("Membership counts differ from their attributed joint partition".into());
        }
        Ok(())
    }
    fn selected(&self, groups: &BTreeSet<SocialGroupId>, all: bool) -> Result<Population, String> {
        if !groups.is_subset(&self.known_groups) {
            return Err("Unknown social group; absence is not an observed zero".into());
        }
        let total = self
            .cells
            .iter()
            .filter(|cell| {
                if all {
                    groups.is_subset(&cell.memberships)
                } else {
                    !groups.is_disjoint(&cell.memberships)
                }
            })
            .try_fold(0_u64, |sum, cell| {
                sum.checked_add(cell.population.0)
                    .ok_or("Selected membership count overflow")
            })?;
        if total > self.population.0 {
            return Err("Selected membership count exceeds unique residents".into());
        }
        Ok(Population(total))
    }
    pub fn members(&self, group: &SocialGroupId) -> Result<Population, String> {
        self.count_any(&BTreeSet::from([group.clone()]))
    }
    /// A union counts each resident once. An empty selection contains no one.
    pub fn count_any(&self, groups: &BTreeSet<SocialGroupId>) -> Result<Population, String> {
        self.selected(groups, false)
    }
    /// An intersection counts cells satisfying every selected membership. With
    /// no conditions, it contains every resident, including unaffiliated cells.
    pub fn count_all(&self, groups: &BTreeSet<SocialGroupId>) -> Result<Population, String> {
        self.selected(groups, true)
    }
    pub fn count_none(&self, groups: &BTreeSet<SocialGroupId>) -> Result<Population, String> {
        Ok(Population(
            self.population
                .0
                .checked_sub(self.count_any(groups)?.0)
                .ok_or("Membership union exceeds residents")?,
        ))
    }
}
/// A projection can be attached to a disjoint economic cohort without replacing
/// that cohort's identity or its role in population and cash reconciliation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortSocialMemberships {
    pub cohort: HouseholdGroupId,
    pub memberships: OverlappingMemberships,
}
