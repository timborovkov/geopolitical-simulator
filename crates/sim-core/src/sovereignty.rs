//! Dated public references, independent of regimes, military simulation and map geometry.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct TerritoryId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritoryDefinition {
    pub id: TerritoryId,
    pub name: String,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
}

/// A recorded legal claim is distinct from control and third-party recognition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LegalSovereignty {
    Sovereign { country: CountryId },
    Disputed { claimants: Vec<CountryId> },
    Unknown,
}
impl LegalSovereignty {
    pub fn label(&self) -> String {
        match self {
            Self::Sovereign { country } => format!("Sovereignty recorded for {}", country.0),
            Self::Disputed { claimants } => format!(
                "Disputed claims: {}",
                claimants
                    .iter()
                    .map(|c| c.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Unknown => "Sovereignty unknown".into(),
        }
    }
    fn involves(&self, id: &CountryId) -> bool {
        match self {
            Self::Sovereign { country } => country == id,
            Self::Disputed { claimants } => claimants.contains(id),
            Self::Unknown => false,
        }
    }
}

/// Occupation is explicit; a different controller never implies annexation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerritorialControl {
    CivilAdministration { country: CountryId },
    Occupation { occupier: CountryId },
    Unknown,
}
impl TerritorialControl {
    pub fn label(&self) -> String {
        match self {
            Self::CivilAdministration { country } => format!("Civil administration: {}", country.0),
            Self::Occupation { occupier } => format!("Occupation recorded: {}", occupier.0),
            Self::Unknown => "Control unknown".into(),
        }
    }
    fn involves(&self, id: &CountryId) -> bool {
        match self {
            Self::CivilAdministration { country } => country == id,
            Self::Occupation { occupier } => occupier == id,
            Self::Unknown => false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerritoryReference {
    pub id: String,
    pub territory: TerritoryId,
    pub validity: Validity,
    pub sovereignty: LegalSovereignty,
    pub control: TerritorialControl,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecognitionPosition {
    Recognized,
    NotRecognized,
    Unknown,
}
impl RecognitionPosition {
    pub fn label(self) -> &'static str {
        match self {
            Self::Recognized => "Recognized",
            Self::NotRecognized => "Not recognized",
            Self::Unknown => "Unknown",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecognitionReference {
    pub id: String,
    pub observer: CountryId,
    pub subject: CountryId,
    pub position: RecognitionPosition,
    pub validity: Validity,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SovereigntyCatalogue {
    pub territories: Vec<TerritoryDefinition>,
    pub references: Vec<TerritoryReference>,
    pub recognition: Vec<RecognitionReference>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedTerritoryReference {
    pub territory: TerritoryDefinition,
    /// None means a reference gap at this date, never unoccupied or unclaimed land.
    pub reference: Option<TerritoryReference>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookmarkSovereigntyReport {
    pub bookmark: CivilDate,
    pub territories: Vec<PublishedTerritoryReference>,
    pub recognition: Vec<RecognitionReference>,
}
impl SovereigntyCatalogue {
    pub fn territories_at(&self, date: CivilDate) -> Vec<PublishedTerritoryReference> {
        self.territories
            .iter()
            .map(|territory| PublishedTerritoryReference {
                territory: territory.clone(),
                reference: self
                    .references
                    .iter()
                    .find(|r| r.territory == territory.id && r.validity.contains(date))
                    .cloned(),
            })
            .collect()
    }
    /// Missing recognition records are unknown, not implicit refusal or recognition.
    pub fn recognition_at(
        &self,
        observer: &CountryId,
        subject: &CountryId,
        date: CivilDate,
    ) -> RecognitionPosition {
        self.recognition
            .iter()
            .find(|r| &r.observer == observer && &r.subject == subject && r.validity.contains(date))
            .map_or(RecognitionPosition::Unknown, |r| r.position)
    }
    pub(crate) fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=4096).contains(&self.territories.len())
            || self.references.len() > 16384
            || self.recognition.len() > 16384
        {
            return Err(
                "Sovereignty reference catalogue exceeds bounded sizes or has no territories"
                    .into(),
            );
        }
        let valid_id = |id: &str| {
            !id.is_empty()
                && id.len() <= 64
                && id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        };
        let provenance = |source: &str, version: &str| {
            content.sources.iter().any(|s| s.id == source)
                && !version.trim().is_empty()
                && version.len() <= 128
        };
        let mut ids = BTreeSet::new();
        for territory in &self.territories {
            if !valid_id(&territory.id.0)
                || !ids.insert(&territory.id)
                || territory.name.trim().is_empty()
                || territory.name.len() > 128
                || !provenance(&territory.source, &territory.source_version)
            {
                return Err("Invalid territory ID, name or provenance".into());
            }
        }
        let mut record_ids = BTreeSet::new();
        let mut periods: BTreeMap<&TerritoryId, Vec<&Validity>> = BTreeMap::new();
        let known = |country: &CountryId| content.country(country).is_some();
        let valid_period = |p: &Validity| {
            p.valid()
                && p.valid_from <= CivilDate::new(2100, 1, 1).expect("valid limit")
                && p.valid_to
                    .is_none_or(|d| d <= CivilDate::new(2100, 1, 1).expect("valid limit"))
        };
        for r in &self.references {
            if !valid_id(&r.id)
                || !record_ids.insert(&r.id)
                || !ids.contains(&r.territory)
                || !valid_period(&r.validity)
                || !provenance(&r.source, &r.source_version)
            {
                return Err("Invalid territory reference, dates or provenance".into());
            }
            let legal = match &r.sovereignty {
                LegalSovereignty::Sovereign { country } => known(country),
                LegalSovereignty::Disputed { claimants } => {
                    (2..=content.countries.len()).contains(&claimants.len())
                        && claimants.iter().all(known)
                        && claimants.iter().collect::<BTreeSet<_>>().len() == claimants.len()
                }
                LegalSovereignty::Unknown => true,
            };
            let control = match &r.control {
                TerritorialControl::CivilAdministration { country } => known(country),
                TerritorialControl::Occupation { occupier } => known(occupier),
                TerritorialControl::Unknown => true,
            };
            if !legal || !control {
                return Err(
                    "Unknown country or duplicate/insufficient sovereignty claimants".into(),
                );
            }
            let prior = periods.entry(&r.territory).or_default();
            if prior.iter().any(|p| p.overlaps(&r.validity)) {
                return Err("Overlapping territory references".into());
            }
            prior.push(&r.validity);
        }
        let bookmark: CivilDate = content.start.parse()?;
        if self.territories.iter().any(|t| {
            !self
                .references
                .iter()
                .any(|r| r.territory == t.id && r.validity.contains(bookmark))
        }) {
            return Err(
                "Territory reference gap at the bookmark; record unknown status explicitly".into(),
            );
        }
        let mut recognition_periods: BTreeMap<(&CountryId, &CountryId), Vec<&Validity>> =
            BTreeMap::new();
        for r in &self.recognition {
            if !valid_id(&r.id)
                || !record_ids.insert(&r.id)
                || !known(&r.observer)
                || !known(&r.subject)
                || r.observer == r.subject
                || !valid_period(&r.validity)
                || !provenance(&r.source, &r.source_version)
            {
                return Err("Invalid recognition reference, countries, dates or provenance".into());
            }
            let prior = recognition_periods
                .entry((&r.observer, &r.subject))
                .or_default();
            if prior.iter().any(|p| p.overlaps(&r.validity)) {
                return Err("Overlapping recognition references".into());
            }
            prior.push(&r.validity);
        }
        Ok(())
    }
}
impl Content {
    /// Public scenario references frozen at the chosen bookmark, never campaign outcomes.
    pub fn bookmark_sovereignty(
        &self,
        country: &CountryId,
    ) -> Result<Option<BookmarkSovereigntyReport>, String> {
        if self.country(country).is_none() {
            return Err("Unknown country".into());
        }
        let Some(catalogue) = &self.sovereignty else {
            return Ok(None);
        };
        let bookmark = self.start.parse()?;
        Ok(Some(BookmarkSovereigntyReport {
            bookmark,
            territories: catalogue
                .territories_at(bookmark)
                .into_iter()
                .filter(|r| {
                    r.reference.as_ref().is_some_and(|r| {
                        r.sovereignty.involves(country) || r.control.involves(country)
                    })
                })
                .collect(),
            recognition: catalogue
                .recognition
                .iter()
                .filter(|r| {
                    (&r.observer == country || &r.subject == country)
                        && r.validity.contains(bookmark)
                })
                .cloned()
                .collect(),
        }))
    }
}
