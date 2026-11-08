//! Private resident stocks and bounded, dated publication history. Measurement
//! never changes resident stocks or consumes the economic random stream.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const RETAINED_POPULATION_RELEASES: usize = 120;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CountryPopulationArchive {
    pub measurements: Vec<PopulationMeasurement>,
    pub last_attempt: CampaignDay,
    pub attempt_capacity: Percent,
    pub next_attempt: Option<CampaignDay>,
    pub next_release: CensusReleaseId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemographyState {
    pub balances: DemographicBalances,
    pub archives: BTreeMap<CountryId, CountryPopulationArchive>,
}
impl DemographyState {
    pub fn new(
        content: &Content,
        definition: &DemographicDefinition,
        administration: &BTreeMap<CountryId, Percent>,
    ) -> Result<Self, String> {
        let balances = definition.initial_balances(content)?;
        let mut result = Self {
            balances,
            archives: BTreeMap::new(),
        };
        for country in &content.countries {
            let archive = CountryPopulationArchive {
                measurements: vec![],
                last_attempt: result.balances.day,
                attempt_capacity: Percent::new(0),
                next_attempt: None,
                next_release: CensusReleaseId(1),
            };
            result.archives.insert(country.id.clone(), archive);
            result.measure(content, definition, administration, &country.id)?;
        }
        result.validate(content, definition, result.balances.day)?;
        Ok(result)
    }
    /// The whole global demographic day, including due releases, commits once.
    pub fn advance_day(
        &mut self,
        content: &Content,
        definition: &DemographicDefinition,
        administration: &BTreeMap<CountryId, Percent>,
        day: CampaignDay,
    ) -> Result<Vec<MigrationTransfer>, String> {
        self.validate(content, definition, self.balances.day)?;
        if administration
            .keys()
            .ne(definition.statistical_capacities.keys())
            || administration.values().any(|value| value.units() > 100)
        {
            return Err("Invalid daily administrative capacity coverage or bounds".into());
        }
        let mut next = self.clone();
        let transfers =
            next.balances
                .advance_day(&definition.rates(), &definition.corridors, day)?;
        for country in &content.countries {
            if next.archives[&country.id]
                .next_attempt
                .is_some_and(|due| day >= due)
            {
                next.measure(content, definition, administration, &country.id)?;
            }
        }
        next.validate(content, definition, day)?;
        *self = next;
        Ok(transfers)
    }
    fn measure(
        &mut self,
        content: &Content,
        definition: &DemographicDefinition,
        administration: &BTreeMap<CountryId, Percent>,
        id: &CountryId,
    ) -> Result<(), String> {
        if administration
            .keys()
            .ne(definition.statistical_capacities.keys())
        {
            return Err(
                "Statistical measurement requires each country's actual administrative capacity"
                    .into(),
            );
        }
        let capacity =
            statistical_capacity(definition.statistical_capacities[id], administration[id])?;
        let population = self.country_population(id)?;
        let opening = content
            .country(id)
            .ok_or("Unknown population reference country")?
            .population;
        let archive = self
            .archives
            .get_mut(id)
            .ok_or("Missing population archive")?;
        if capacity.units() > 0 {
            let measurement = PopulationMeasurement::new(
                archive.next_release,
                self.balances.day,
                population,
                opening,
                capacity,
            )?;
            archive.next_release = CensusReleaseId(
                archive
                    .next_release
                    .0
                    .checked_add(1)
                    .ok_or("Population release identity exhausted")?,
            );
            archive.measurements.push(measurement);
            if archive.measurements.len() > RETAINED_POPULATION_RELEASES {
                archive.measurements.remove(0);
            }
        }
        archive.last_attempt = self.balances.day;
        archive.attempt_capacity = capacity;
        archive.next_attempt = self
            .balances
            .day
            .checked_add(census_interval(definition.census_days, capacity)?)
            .filter(|day| day.units() <= MAX_DAY);
        Ok(())
    }
    pub fn country_population(&self, id: &CountryId) -> Result<Population, String> {
        let groups = self
            .balances
            .countries
            .get(id)
            .ok_or("Unknown demographic country")?;
        let total = groups.values().try_fold(0_u64, |sum, group| {
            sum.checked_add(group.people.total()?.0)
                .ok_or_else(|| "Resident count overflow".to_string())
        })?;
        Ok(Population(total))
    }
    /// Returns published records only. World-level viewer checks select private
    /// owned stocks separately; callers cannot mistake this for current truth.
    pub fn published(&self, id: &CountryId) -> Result<Vec<PublishedPopulationEstimate>, String> {
        Ok(self
            .archives
            .get(id)
            .ok_or("Unknown population archive")?
            .measurements
            .iter()
            .map(|m| m.published.clone())
            .collect())
    }
    pub fn validate(
        &self,
        content: &Content,
        definition: &DemographicDefinition,
        day: CampaignDay,
    ) -> Result<(), String> {
        definition.validate_balances(content, &self.balances, day)?;
        if self.archives.keys().ne(self.balances.countries.keys()) {
            return Err("Population archives must cover exactly the campaign countries".into());
        }
        let start = content.start_day()?;
        for (id, archive) in &self.archives {
            if self.country_population(id)?.0 > 1_000_000_000_000 {
                return Err("Resident counts exceed supported physical/accounting bounds".into());
            }
            let opening = content
                .country(id)
                .ok_or("Unknown population reference")?
                .population;
            if archive.measurements.len() > RETAINED_POPULATION_RELEASES
                || archive.last_attempt < start
                || archive.last_attempt > day
                || archive.next_release.0 == 0
                || archive
                    .next_attempt
                    .is_some_and(|due| due <= day || due.units() > MAX_DAY)
                || archive.attempt_capacity.units() > definition.statistical_capacities[id].units()
                || archive.next_attempt
                    != archive
                        .last_attempt
                        .checked_add(census_interval(
                            definition.census_days,
                            archive.attempt_capacity,
                        )?)
                        .filter(|date| date.units() <= MAX_DAY)
            {
                return Err("Invalid population archive dates, retention or identity".into());
            }
            if archive.measurements.len() < RETAINED_POPULATION_RELEASES
                && archive
                    .measurements
                    .first()
                    .is_some_and(|m| m.published.id.0 != 1)
            {
                return Err(
                    "Untruncated population history must begin with its first release".into(),
                );
            }
            if archive.attempt_capacity.units() > 0
                && !archive.measurements.last().is_some_and(|m| {
                    m.published.day == archive.last_attempt
                        && m.published.statistical_capacity == archive.attempt_capacity
                })
            {
                return Err("A successful statistical attempt requires its dated release".into());
            }
            let mut previous: Option<(u64, CampaignDay)> = None;
            for measurement in &archive.measurements {
                measurement.validate(opening)?;
                if measurement.published.day == day
                    && measurement.actual != self.country_population(id)?
                    || measurement.published.day == start && measurement.actual != opening
                {
                    return Err("Population measurement differs from its known dated stocks".into());
                }
                let release = &measurement.published;
                if measurement.actual.0 > 1_000_000_000_000
                    || release.day < start
                    || release.day > archive.last_attempt
                    || previous.is_some_and(|(id, day)| {
                        id.checked_add(1) != Some(release.id.0) || release.day <= day
                    })
                    || release.statistical_capacity.units()
                        > definition.statistical_capacities[id].units()
                {
                    return Err("Invalid population release sequence or scenario capacity".into());
                }
                previous = Some((release.id.0, release.day));
            }
            if let Some((id, _)) = previous {
                if id.checked_add(1) != Some(archive.next_release.0) {
                    return Err("Next population release identity differs from its history".into());
                }
            } else if archive.next_release.0 != 1 {
                return Err("An unpublished archive cannot consume release identities".into());
            }
        }
        Ok(())
    }
}
