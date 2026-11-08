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
/// Published review dates keep the same absolute-day type as their private source.
/// ```compile_fail
/// use sim_core::{PublishedObservation, DaySpan};
/// fn cannot_publish_a_duration(report: &mut PublishedObservation) { report.day = DaySpan::new(30); }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedObservation {
    pub day: crate::CampaignDay,
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
    pub regime: Regime,
    pub government: GovernmentDefinition,
    pub government_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ministers: Option<Vec<PublishedMinister>>,
    /// Dated bookmark references, separate from live government and private state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark_sovereignty: Option<BookmarkSovereigntyReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electoral_authority: Option<PublicElectionReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regional_authorities: Option<Vec<PublishedRegionalAuthority>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub territorial: Option<TerritorialReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undertakings: Option<PublicUndertakingReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fiscal_plans: Option<Vec<PublishedFiscalPlan>>,
    pub population: Population,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population_measurement: Option<crate::PublishedPopulationEstimate>,
    pub opening_gdp: Money,
    pub initial_data_quality: String,
    /// None means no statistical release yet; absence is not a zero observation.
    pub latest: Option<PublishedObservation>,
    pub history: Vec<PublishedObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_reviews: Option<Vec<crate::PublishedAnnualReview>>,
    pub seats: BTreeMap<PartyId, u16>,
    pub governing_parties: Vec<PartyId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_events: Option<Vec<Event>>,
    pub day: crate::CampaignDay,
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
        self.validate_report_content(content)?;
        self.public_report_validated(content, id)
    }
    fn validate_report_content(&self, content: &Content) -> Result<(), String> {
        if self.rules != content.rules || self.content_hash != content.hash() {
            return Err("Public reports require the campaign's exact content pack".into());
        }
        Ok(())
    }
    /// The caller has checked the content once for this immutable projection.
    fn public_report_validated(
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
            regime: self.governing_regime(content, id)?.clone(),
            government: self.governing_rules(content, id)?.clone(),
            government_title: c
                .constitution
                .as_ref()
                .and_then(|state| state.history.last())
                .and_then(|change| content.constitutions.as_ref()?.template(&change.to))
                .map_or_else(|| d.office.clone(), |template| template.title.clone()),
            ministers: self.published_ministers(content, id),
            bookmark_sovereignty: content.bookmark_sovereignty(id)?,
            electoral_authority: self.published_election(content, id),
            regional_authorities: self.published_regions(content, id),
            territorial: self.published_territorial(content, id),
            undertakings: self.published_undertakings(id),
            fiscal_plans: self.published_fiscal_plans(id),
            population: self.demographics.as_ref().map_or(c.population, |state| {
                state.archives[id]
                    .measurements
                    .last()
                    .map_or(d.population, |m| m.published.population)
            }),
            population_measurement: self.demographics.as_ref().and_then(|state| {
                state.archives[id]
                    .measurements
                    .last()
                    .map(|m| m.published.clone())
            }),
            opening_gdp: Money(d.gdp.units()),
            initial_data_quality: d.quality.clone(),
            archived_reviews: c
                .statistical_archive
                .as_ref()
                .map(crate::StatisticalArchive::published),
            latest: history.last().cloned(),
            history,
            seats: c.seats.clone(),
            governing_parties: c.coalition.clone(),
        })
    }
    /// This function provides data minimization, not authentication. Trusted hosts
    /// assign Viewer from their authenticated role; clients cannot choose a role.
    pub fn project(&self, content: &Content, viewer: Viewer) -> Result<PlayerView, String> {
        self.validate_report_content(content)?;
        let own = match viewer {
            Viewer::Observer => None,
            Viewer::Government(id) => Some(
                self.countries
                    .get(&id)
                    .ok_or("Unknown viewer government")?
                    .clone(),
            ),
        };
        let own_events = self
            .event_sequences
            .as_ref()
            .filter(|_| self.rules >= 22)
            .and_then(|_| {
                own.as_ref().map(|country| {
                    self.events
                        .iter()
                        .filter(|e| e.country == country.id)
                        .cloned()
                        .collect()
                })
            });
        let mut reports = BTreeMap::new();
        for id in self.countries.keys() {
            reports.insert(id.clone(), self.public_report_validated(content, id)?);
        }
        Ok(PlayerView {
            own_events,
            day: self.day,
            rules: self.rules,
            content_hash: self.content_hash,
            developer_modified: self.is_developer_modified(),
            own,
            reports,
        })
    }
}
