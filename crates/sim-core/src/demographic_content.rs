//! Scenario calibration for aggregate population accounting. Opening reference
//! populations remain distinct from these authored transition assumptions.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortDemographicDefinition {
    pub rates: CohortDemographicRates,
    /// Split only the non-working-age opening residents between 0–14 and 65+.
    pub young_share_of_dependants_bps: BasisPoints,
    /// Aggregate household units per resident; these are not buildings/assets.
    pub initial_households_per_person_bps: BasisPoints,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemographicDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub profiles: BTreeMap<CountryId, BTreeMap<HouseholdGroupId, CohortDemographicDefinition>>,
    pub corridors: Vec<MigrationCorridor>,
    pub statistical_capacities: BTreeMap<CountryId, Percent>,
    pub census_days: DaySpan,
    /// Capacity percentages refer to opening physical daily need, not growing
    /// civilian demand. Further construction is still separately funded.
    pub maximum_capacity: Percent,
}
impl DemographicDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if self.description.trim().is_empty()
            || self.description.len() > 4096
            || self.source_version.trim().is_empty()
            || self.source_version.len() > 128
            || !content.sources.iter().any(|s| s.id == self.source)
            || !(200..=2000).contains(&self.maximum_capacity.units())
            || self.corridors.len() > 1024
            || !(7..=180).contains(&self.census_days.units())
            || self
                .statistical_capacities
                .values()
                .any(|c| c.units() > 100)
        {
            return Err("Invalid demographic calibration, provenance or capacity bound".into());
        }
        let countries: BTreeSet<_> = content.countries.iter().map(|c| &c.id).collect();
        if self.statistical_capacities.keys().collect::<BTreeSet<_>>() != countries {
            return Err("Every country needs an explicit statistical-capacity calibration".into());
        }
        if self.profiles.keys().collect::<BTreeSet<_>>() != countries {
            return Err("Every country needs an explicit demographic profile".into());
        }
        for country in &content.countries {
            let profiles = &self.profiles[&country.id];
            if profiles.keys().collect::<BTreeSet<_>>()
                != country
                    .households
                    .iter()
                    .map(|h| &h.id)
                    .collect::<BTreeSet<_>>()
            {
                return Err("Every household group needs an explicit demographic profile".into());
            }
            for profile in profiles.values() {
                profile.rates.validate()?;
                if profile.young_share_of_dependants_bps.units() > 10000
                    || !(1..=10000).contains(&profile.initial_households_per_person_bps.units())
                {
                    return Err("Invalid opening demographic ratios".into());
                }
            }
        }
        let mut ids = BTreeSet::new();
        for corridor in &self.corridors {
            if corridor.id.0.is_empty()
                || corridor.id.0.len() > 128
                || !corridor.id.0.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b':')
                })
                || !ids.insert(&corridor.id)
                || corridor.origin.country == corridor.destination.country
                || [&corridor.origin, &corridor.destination].iter().any(|key| {
                    content
                        .country(&key.country)
                        .and_then(|c| c.household(&key.group))
                        .is_none()
                })
            {
                return Err("Migration corridors require unique stable identities and known country/cohort endpoints in different countries".into());
            }
        }
        Ok(())
    }
    pub fn rates(&self) -> BTreeMap<CountryId, BTreeMap<HouseholdGroupId, CohortDemographicRates>> {
        self.profiles
            .iter()
            .map(|(country, profiles)| {
                (
                    country.clone(),
                    profiles
                        .iter()
                        .map(|(group, profile)| (group.clone(), profile.rates.clone()))
                        .collect(),
                )
            })
            .collect()
    }
    /// Called after the containing content has passed validation. No cash, assets
    /// or opening resources are supplied by demographic initialization.
    pub fn initial_balances(&self, content: &Content) -> Result<DemographicBalances, String> {
        self.validate(content)?;
        let mut countries = BTreeMap::new();
        for definition in &content.countries {
            let mut cohorts = BTreeMap::new();
            for household in crate::households::initial_households(definition, Money(0)) {
                let profile = &self.profiles[&definition.id][&household.id];
                let dependants = household
                    .population
                    .0
                    .checked_sub(household.working_age.0)
                    .ok_or("Opening working-age count exceeds residents")?;
                let young = (u128::from(dependants)
                    * u128::from(profile.young_share_of_dependants_bps.units())
                    / 10000) as u64;
                let families = ((u128::from(household.population.0)
                    * u128::from(profile.initial_households_per_person_bps.units())
                    / 10000) as u64)
                    .max(u64::from(household.population.0 > 0));
                let people = AgeStocks {
                    young: Population(young),
                    working_age: household.working_age,
                    older: Population(dependants - young),
                };
                cohorts.insert(
                    household.id,
                    CohortDemography::new(people, HouseholdUnits(families))?,
                );
            }
            countries.insert(definition.id.clone(), cohorts);
        }
        let result = DemographicBalances {
            day: content.start_day()?,
            countries,
            migration_carries: self.corridors.iter().map(|c| (c.id.clone(), 0)).collect(),
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate_balances(
        &self,
        content: &Content,
        balances: &DemographicBalances,
        day: CampaignDay,
    ) -> Result<(), String> {
        balances.validate()?;
        if balances.day != day {
            return Err("Demographic balances must match the authoritative campaign day".into());
        }
        let opening = self.initial_balances(content)?;
        if balances.countries.keys().ne(opening.countries.keys())
            || balances
                .migration_carries
                .keys()
                .ne(opening.migration_carries.keys())
        {
            return Err("Demographic identities differ from scenario definitions".into());
        }
        for (country, groups) in &opening.countries {
            let actual = &balances.countries[country];
            if actual.keys().ne(groups.keys()) {
                return Err(
                    "Demographic household identities differ from scenario definitions".into(),
                );
            }
            for (id, initial) in groups {
                if actual[id].opening != initial.opening
                    || actual[id].opening_households != initial.opening_households
                {
                    return Err("Demographic opening stocks differ from the attributed scenario calibration".into());
                }
            }
        }
        Ok(())
    }
}
