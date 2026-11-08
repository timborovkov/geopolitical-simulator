//! Version-gated world integration and explicit private/public projection.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnedDemographicGroup {
    pub group: HouseholdGroupId,
    pub ages: AgeStocks,
    pub households: HouseholdUnits,
    pub flows: DemographicFlows,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemographicReport {
    pub country: CountryId,
    pub as_of: CampaignDay,
    pub published: Vec<PublishedPopulationEstimate>,
    pub owned_groups: Option<Vec<OwnedDemographicGroup>>,
    pub source: String,
    pub source_version: String,
    pub next_attempt: Option<CampaignDay>,
}
impl World {
    fn demographic_administration(&self) -> Result<BTreeMap<CountryId, Percent>, String> {
        self.countries
            .iter()
            .map(|(id, country)| {
                if !(0..=100).contains(&country.administration) {
                    return Err("Invalid administrative capacity for population measurement".into());
                }
                Ok((id.clone(), Percent::new(country.administration as u32)))
            })
            .collect()
    }
    pub(crate) fn seed_demographics(&mut self, content: &Content) -> Result<(), String> {
        if let Some(definition) = &content.demographics {
            self.demographics = Some(DemographyState::new(
                content,
                definition,
                &self.demographic_administration()?,
            )?);
        }
        Ok(())
    }
    pub(crate) fn step_demographics(&mut self, content: &Content) -> Result<(), String> {
        let Some(definition) = &content.demographics else {
            return Ok(());
        };
        let administration = self.demographic_administration()?;
        let state = self
            .demographics
            .as_mut()
            .ok_or("Missing demographic campaign state")?;
        state.advance_day(content, definition, &administration, self.day)?;
        for (id, country) in &mut self.countries {
            country.population = state.country_population(id)?;
            for group in &mut country.households {
                let stock = &state.balances.countries[id][&group.id];
                group.population = stock.people.total()?;
                group.working_age = stock.people.working_age;
                group.employed.0 = group.employed.0.min(group.working_age.0);
            }
        }
        Ok(())
    }
    pub(crate) fn validate_demographics(&self, content: &Content) -> Result<(), String> {
        match (&content.demographics, &self.demographics) {
            (None, None) => Ok(()),
            (Some(definition), Some(state)) => {
                state.validate(content, definition, self.day)?;
                for (id, country) in &self.countries {
                    if country.population != state.country_population(id)?
                        || country.population.0 > 1_000_000_000_000
                    {
                        return Err("Country residents differ from demographic stocks or supported count bounds".into());
                    }
                    let stocks = &state.balances.countries[id];
                    if country.households.len() != stocks.len() {
                        return Err("Household identities differ from demographic stocks".into());
                    }
                    for group in &country.households {
                        let stock = stocks
                            .get(&group.id)
                            .ok_or("Unknown demographic household group")?;
                        if group.population != stock.people.total()?
                            || group.working_age != stock.people.working_age
                        {
                            return Err("Household residents or working-age counts differ from demographic stocks".into());
                        }
                    }
                }
                Ok(())
            }
            _ => Err("Demographic state must match the exact campaign rules".into()),
        }
    }
    pub fn demographic_report(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &CountryId,
    ) -> Result<Option<DemographicReport>, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Demographic inspection requires the campaign's exact content".into());
        }
        if !self.countries.contains_key(id) {
            return Err("Unknown demographic country".into());
        }
        let (Some(definition), Some(state)) = (&content.demographics, &self.demographics) else {
            return Ok(None);
        };
        let owner = id == &self.player && viewer == Viewer::Government(self.player.clone());
        Ok(Some(DemographicReport {
            country: id.clone(),
            as_of: self.day,
            published: state.published(id)?,
            owned_groups: owner.then(|| {
                state.balances.countries[id]
                    .iter()
                    .map(|(id, group)| OwnedDemographicGroup {
                        group: id.clone(),
                        ages: group.people,
                        households: group.households,
                        flows: group.flows.clone(),
                    })
                    .collect()
            }),
            source: definition.source.clone(),
            source_version: definition.source_version.clone(),
            next_attempt: if owner {
                state.archives[id].next_attempt
            } else {
                None
            },
        }))
    }
    /// Population-sensitive income entitlements; pooled accounts do not migrate.
    pub(crate) fn demographic_income_weights(&self, content: &Content, id: &CountryId) -> Vec<i64> {
        let definition = content.country(id).expect("country definition");
        if content.demographics.is_none() {
            return definition
                .households
                .iter()
                .map(|h| h.income_share)
                .collect();
        }
        self.countries[id].population_income_weights(definition)
    }
}
