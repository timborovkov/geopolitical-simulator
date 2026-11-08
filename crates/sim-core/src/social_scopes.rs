//! Age/employment eligibility for joint membership projections. The four base
//! populations are disjoint and remain independent of overlapping memberships.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SocialPopulationScope {
    Young,
    WorkingEmployed,
    WorkingOther,
    Older,
}
impl SocialPopulationScope {
    pub const ALL: [Self; 4] = [
        Self::Young,
        Self::WorkingEmployed,
        Self::WorkingOther,
        Self::Older,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Young => "Young residents (0–14)",
            Self::WorkingEmployed => "Employed working-age residents",
            Self::WorkingOther => "Other working-age residents",
            Self::Older => "Older residents (65+)",
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SocialEligibility {
    AnyResident,
    AgeFifteenPlus,
    EmployedResident,
}
impl SocialEligibility {
    fn allows(self, scope: SocialPopulationScope) -> bool {
        match self {
            Self::AnyResident => true,
            Self::AgeFifteenPlus => scope != SocialPopulationScope::Young,
            Self::EmployedResident => scope == SocialPopulationScope::WorkingEmployed,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopedSocialCatalogue {
    pub groups: BTreeMap<SocialGroupId, SocialGroupDefinition>,
    pub eligibility: BTreeMap<SocialGroupId, SocialEligibility>,
    /// This membership is exactly the existing employed resident stock. Its
    /// identity/name is configurable; behavior never branches on display text.
    pub employment_group: SocialGroupId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopedSocialMemberships {
    pub population: Population,
    pub scopes: BTreeMap<SocialPopulationScope, OverlappingMemberships>,
}
impl ScopedSocialCatalogue {
    pub fn validate_profiles(
        &self,
        profiles: &BTreeMap<SocialPopulationScope, SocialMembershipProfile>,
    ) -> Result<(), String> {
        if self.groups.keys().ne(self.eligibility.keys())
            || self.eligibility.get(&self.employment_group)
                != Some(&SocialEligibility::EmployedResident)
            || profiles.keys().copied().collect::<BTreeSet<_>>()
                != SocialPopulationScope::ALL.into_iter().collect()
        {
            return Err("Social profiles require every disjoint age/employment scope and explicit group eligibility".into());
        }
        for (scope, profile) in profiles {
            profile.validate(&self.groups)?;
            for cell in &profile.cells {
                if cell.memberships.contains(&self.employment_group)
                    != (*scope == SocialPopulationScope::WorkingEmployed)
                    || cell
                        .memberships
                        .iter()
                        .any(|id| !self.eligibility[id].allows(*scope))
                {
                    return Err("Joint memberships conflict with age/employment eligibility or the complete employment membership".into());
                }
            }
        }
        Ok(())
    }
    pub fn project(
        &self,
        ages: AgeStocks,
        employed: Population,
        profiles: &BTreeMap<SocialPopulationScope, SocialMembershipProfile>,
    ) -> Result<ScopedSocialMemberships, String> {
        self.validate_profiles(profiles)?;
        let other = ages
            .working_age
            .0
            .checked_sub(employed.0)
            .ok_or("Employed residents exceed working-age stocks")?;
        let population = ages.total()?;
        let mut scopes = BTreeMap::new();
        for (scope, stock) in [
            (SocialPopulationScope::Young, ages.young),
            (SocialPopulationScope::WorkingEmployed, employed),
            (SocialPopulationScope::WorkingOther, Population(other)),
            (SocialPopulationScope::Older, ages.older),
        ] {
            scopes.insert(scope, profiles[&scope].project(stock, &self.groups)?);
        }
        let result = ScopedSocialMemberships { population, scopes };
        if result.members(&self.employment_group)? != employed
            || result.count_all(&BTreeSet::new())? != population
        {
            return Err("Scoped memberships do not reconcile to resident/employment stocks".into());
        }
        Ok(result)
    }
}
impl ScopedSocialMemberships {
    pub fn validate(
        &self,
        catalogue: &ScopedSocialCatalogue,
        profiles: &BTreeMap<SocialPopulationScope, SocialMembershipProfile>,
        ages: AgeStocks,
        employed: Population,
    ) -> Result<(), String> {
        if &catalogue.project(ages, employed, profiles)? != self {
            return Err("Scoped social membership differs from attributed resident stocks and joint profiles".into());
        }
        Ok(())
    }
    fn selected(&self, groups: &BTreeSet<SocialGroupId>, all: bool) -> Result<Population, String> {
        let total = self.scopes.values().try_fold(0_u64, |sum, scope| {
            let count = if all {
                scope.count_all(groups)?
            } else {
                scope.count_any(groups)?
            };
            sum.checked_add(count.0)
                .ok_or_else(|| "Scoped membership count overflow".to_string())
        })?;
        if total > self.population.0 {
            return Err("Scoped group selection exceeds unique residents".into());
        }
        Ok(Population(total))
    }
    pub fn members(&self, group: &SocialGroupId) -> Result<Population, String> {
        self.count_any(&BTreeSet::from([group.clone()]))
    }
    pub fn count_any(&self, groups: &BTreeSet<SocialGroupId>) -> Result<Population, String> {
        self.selected(groups, false)
    }
    pub fn count_all(&self, groups: &BTreeSet<SocialGroupId>) -> Result<Population, String> {
        self.selected(groups, true)
    }
    pub fn count_none(&self, groups: &BTreeSet<SocialGroupId>) -> Result<Population, String> {
        Ok(Population(
            self.population
                .0
                .checked_sub(self.count_any(groups)?.0)
                .ok_or("Scoped membership union exceeds residents")?,
        ))
    }
}
