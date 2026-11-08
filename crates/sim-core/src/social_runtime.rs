//! Owner-only projections of overlapping roles, without changing resident or
//! financial accounts, policy powers or historical membership claims.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortSocialCounts {
    pub cohort: HouseholdGroupId,
    pub population: Population,
    pub members: BTreeMap<SocialGroupId, Population>,
    pub any_selected: Population,
    pub all_selected: Population,
    pub none_selected: Population,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnedSocialCounts {
    /// Each person once, rather than a sum of overlapping role counts.
    pub population: Population,
    pub members: BTreeMap<SocialGroupId, Population>,
    pub any_selected: Population,
    pub all_selected: Population,
    pub none_selected: Population,
    pub cohorts: Vec<CohortSocialCounts>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocialMembershipReport {
    pub country: CountryId,
    pub day: CampaignDay,
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub groups: Vec<SocialGroupDefinition>,
    pub selected: BTreeSet<SocialGroupId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned: Option<OwnedSocialCounts>,
}
fn add_population(left: Population, right: Population) -> Result<Population, String> {
    Ok(Population(
        left.0
            .checked_add(right.0)
            .ok_or("Social membership count overflow")?,
    ))
}
impl World {
    /// Foreign/observer views expose attributed definitions only. They do not
    /// calculate or publish current private household ages, jobs or memberships.
    pub fn social_membership_report(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &CountryId,
        selected: &BTreeSet<SocialGroupId>,
    ) -> Result<Option<SocialMembershipReport>, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Social inspection requires the campaign's exact content".into());
        }
        let country = self
            .countries
            .get(id)
            .ok_or("Unknown social-membership country")?;
        let Some(definition) = &content.social_memberships else {
            return Ok(None);
        };
        if selected
            .iter()
            .any(|id| !definition.catalogue.groups.contains_key(id))
        {
            return Err("Unknown social group in selection".into());
        }
        let owner = id == &self.player && viewer == Viewer::Government(self.player.clone());
        let owned = if owner {
            let balances = &self
                .demographics
                .as_ref()
                .ok_or("Social scopes require demographic campaign stocks")?
                .balances;
            let stocks = balances
                .countries
                .get(id)
                .ok_or("Missing social population stocks")?;
            let mut result = OwnedSocialCounts {
                population: Population(0),
                members: definition
                    .catalogue
                    .groups
                    .keys()
                    .map(|id| (id.clone(), Population(0)))
                    .collect(),
                any_selected: Population(0),
                all_selected: Population(0),
                none_selected: Population(0),
                cohorts: vec![],
            };
            for household in &country.households {
                let ages = stocks
                    .get(&household.id)
                    .ok_or("Missing household demographic stocks")?
                    .people;
                if ages.total()? != household.population
                    || ages.working_age != household.working_age
                {
                    return Err("Social projection requires reconciled household stocks".into());
                }
                let profiles = definition
                    .cohort(id, &household.id)
                    .ok_or("Missing household social profiles")?;
                let projected = definition
                    .catalogue
                    .project(ages, household.employed, profiles)?;
                let mut members = BTreeMap::new();
                for group in definition.catalogue.groups.keys() {
                    let count = projected.members(group)?;
                    result
                        .members
                        .insert(group.clone(), add_population(result.members[group], count)?);
                    members.insert(group.clone(), count);
                }
                let any_selected = projected.count_any(selected)?;
                let all_selected = projected.count_all(selected)?;
                let none_selected = projected.count_none(selected)?;
                result.population = add_population(result.population, projected.population)?;
                result.any_selected = add_population(result.any_selected, any_selected)?;
                result.all_selected = add_population(result.all_selected, all_selected)?;
                result.none_selected = add_population(result.none_selected, none_selected)?;
                result.cohorts.push(CohortSocialCounts {
                    cohort: household.id.clone(),
                    population: projected.population,
                    members,
                    any_selected,
                    all_selected,
                    none_selected,
                });
            }
            if result.population != country.population
                || result.members[&definition.catalogue.employment_group].0
                    != country.households.iter().map(|h| h.employed.0).sum::<u64>()
            {
                return Err("Joint social memberships do not reconcile to unique country residents and jobs".into());
            }
            Some(result)
        } else {
            None
        };
        Ok(Some(SocialMembershipReport {
            country: id.clone(),
            day: self.day,
            source: definition.source.clone(),
            source_version: definition.source_version.clone(),
            description: definition.description.clone(),
            groups: definition.catalogue.groups.values().cloned().collect(),
            selected: selected.clone(),
            owned,
        }))
    }
}
