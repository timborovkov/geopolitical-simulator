//! Projection boundary. The host authenticates a viewer before requesting a view.
//! Foreign reports contain published aggregates, never treasury, households or AI plans.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Viewer {
    Government(CountryId),
    Observer,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedObservation {
    pub day: u32,
    pub debt: Money,
    pub approval: i32,
    pub needs: i32,
    pub daily_output: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicCountryReport {
    pub country: CountryId,
    pub name: String,
    pub executive: String,
    pub population: Population,
    pub opening_gdp: Money,
    pub initial_data_quality: String,
    /// None means no statistical release yet; absence is not a zero observation.
    pub latest: Option<PublishedObservation>,
    pub history: Vec<PublishedObservation>,
    pub seats: BTreeMap<String, u16>,
    pub governing_parties: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerView {
    pub day: u32,
    pub rules: u32,
    pub content_hash: u64,
    pub developer_modified: bool,
    /// A government sees only its own private account state. Observers see none.
    pub own: Option<Country>,
    pub reports: BTreeMap<CountryId, PublicCountryReport>,
}
impl World {
    pub fn public_report(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<PublicCountryReport, String> {
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let d = content.country(id).ok_or("Missing country definition")?;
        let history: Vec<_> = c
            .history
            .iter()
            .map(|o| PublishedObservation {
                day: o.day,
                debt: o.debt,
                approval: o.approval,
                needs: o.needs,
                daily_output: o.output,
            })
            .collect();
        Ok(PublicCountryReport {
            country: id.clone(),
            name: d.name.clone(),
            executive: c.executive.clone(),
            population: c.population,
            opening_gdp: Money(d.gdp),
            initial_data_quality: d.quality.clone(),
            latest: history.last().cloned(),
            history,
            seats: c.seats.clone(),
            governing_parties: c.coalition.clone(),
        })
    }
    /// This function provides data minimization, not authentication. Trusted hosts
    /// assign Viewer from their authenticated role; clients cannot choose a role.
    pub fn project(&self, content: &Content, viewer: Viewer) -> Result<PlayerView, String> {
        let own = match viewer {
            Viewer::Observer => None,
            Viewer::Government(id) => Some(
                self.countries
                    .get(&id)
                    .ok_or("Unknown viewer government")?
                    .clone(),
            ),
        };
        let mut reports = BTreeMap::new();
        for id in self.countries.keys() {
            reports.insert(id.clone(), self.public_report(content, id)?);
        }
        Ok(PlayerView {
            day: self.day,
            rules: self.rules,
            content_hash: self.content_hash,
            developer_modified: self.is_developer_modified(),
            own,
            reports,
        })
    }
}
