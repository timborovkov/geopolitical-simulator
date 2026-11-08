//! Attributed joint-membership calibrations. No profile silently substitutes
//! independence assumptions, missing populations or historical prevalence.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type SocialScopeProfiles = BTreeMap<SocialPopulationScope, SocialMembershipProfile>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocialMembershipDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub catalogue: ScopedSocialCatalogue,
    pub profiles: BTreeMap<CountryId, BTreeMap<HouseholdGroupId, SocialScopeProfiles>>,
}
impl SocialMembershipDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !content
            .sources
            .iter()
            .any(|source| source.id == self.source)
            || self.source_version.trim().is_empty()
            || self.source_version.len() > 128
            || self.description.trim().is_empty()
            || self.description.len() > 4096
        {
            return Err(
                "Social membership calibration requires explicit bounded provenance".into(),
            );
        }
        if self.profiles.keys().collect::<BTreeSet<_>>()
            != content
                .countries
                .iter()
                .map(|country| &country.id)
                .collect()
        {
            return Err("Every country needs explicit joint social membership profiles".into());
        }
        for country in &content.countries {
            let profiles = &self.profiles[&country.id];
            if profiles.keys().collect::<BTreeSet<_>>()
                != country.households.iter().map(|group| &group.id).collect()
            {
                return Err("Every disjoint household cohort needs explicit age/employment membership profiles".into());
            }
            for scopes in profiles.values() {
                self.catalogue.validate_profiles(scopes)?;
            }
        }
        Ok(())
    }
    pub fn cohort(
        &self,
        country: &CountryId,
        group: &HouseholdGroupId,
    ) -> Option<&SocialScopeProfiles> {
        self.profiles.get(country)?.get(group)
    }
}
