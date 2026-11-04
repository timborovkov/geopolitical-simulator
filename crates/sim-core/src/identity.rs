//! Dated reference identities initialize a bookmark; they do not schedule campaign outcomes.
use crate::{CivilDate, Content, CountryDefinition, CountryId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
        #[serde(transparent)]
        pub struct $name(pub String);
    };
}
identity!(PersonId);
identity!(OfficeId);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordQuality {
    Observed,
    Reconciled,
    Interpolated,
    Inferred,
    Authored,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validity {
    pub valid_from: CivilDate,
    pub valid_to: Option<CivilDate>,
}
impl Validity {
    pub fn contains(&self, date: CivilDate) -> bool {
        self.valid_from <= date && self.valid_to.is_none_or(|end| date < end)
    }
    pub(crate) fn valid(&self) -> bool {
        self.valid_to.is_none_or(|end| self.valid_from < end)
    }
    pub(crate) fn overlaps(&self, other: &Self) -> bool {
        self.valid_to.is_none_or(|end| other.valid_from < end)
            && other.valid_to.is_none_or(|end| self.valid_from < end)
    }
    fn encloses(&self, other: &Self) -> bool {
        self.valid_from <= other.valid_from
            && self
                .valid_to
                .is_none_or(|end| other.valid_to.is_some_and(|other_end| other_end <= end))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonDefinition {
    pub id: PersonId,
    pub name: String,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfficeRole {
    HeadOfState,
    HeadOfGovernment,
    Legislature,
    Cabinet,
    Ministry,
    LocalGovernment,
    Court,
    IndependentInstitution,
    PartyBody,
    Council,
    Other,
}
impl OfficeRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::HeadOfState => "Head of state",
            Self::HeadOfGovernment => "Head of government",
            Self::Legislature => "Legislature",
            Self::Cabinet => "Cabinet",
            Self::Ministry => "Ministry",
            Self::LocalGovernment => "Local government",
            Self::Court => "Court",
            Self::IndependentInstitution => "Independent institution",
            Self::PartyBody => "Party body",
            Self::Council => "Council",
            Self::Other => "Other office",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficeDefinition {
    pub id: OfficeId,
    pub country: CountryId,
    pub title: String,
    pub role: OfficeRole,
    pub required: bool,
    pub validity: Validity,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OfficeHolder {
    Person {
        person: PersonId,
    },
    Collective {
        name: String,
        members: Vec<PersonId>,
    },
    Vacant,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficeTerm {
    pub id: String,
    pub office: OfficeId,
    pub holder: OfficeHolder,
    pub validity: Validity,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrencyTerm {
    pub id: String,
    pub country: CountryId,
    pub code: String,
    pub validity: Validity,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityCatalogue {
    pub people: Vec<PersonDefinition>,
    pub offices: Vec<OfficeDefinition>,
    pub terms: Vec<OfficeTerm>,
    pub currencies: Vec<CurrencyTerm>,
}
pub struct BookmarkOffice {
    pub id: OfficeId,
    pub title: String,
    pub role: OfficeRole,
    pub holder: String,
    pub term: Option<OfficeTerm>,
}
impl IdentityCatalogue {
    pub fn holder_name(&self, holder: &OfficeHolder) -> String {
        match holder {
            OfficeHolder::Person { person } => self
                .people
                .iter()
                .find(|p| &p.id == person)
                .map_or_else(|| "Unknown person".into(), |p| p.name.clone()),
            OfficeHolder::Collective { name, .. } => name.clone(),
            OfficeHolder::Vacant => "Vacant".into(),
        }
    }
    pub fn offices_at(&self, country: &CountryId, date: CivilDate) -> Vec<BookmarkOffice> {
        self.offices
            .iter()
            .filter(|office| &office.country == country && office.validity.contains(date))
            .map(|office| {
                let term = self
                    .terms
                    .iter()
                    .find(|term| term.office == office.id && term.validity.contains(date));
                BookmarkOffice {
                    id: office.id.clone(),
                    title: office.title.clone(),
                    role: office.role,
                    holder: term.map_or_else(
                        || "Unknown incumbent".into(),
                        |term| self.holder_name(&term.holder),
                    ),
                    term: term.cloned(),
                }
            })
            .collect()
    }
    pub(crate) fn resolve(
        &self,
        country: &mut CountryDefinition,
        date: CivilDate,
    ) -> Result<(), String> {
        if let Some(id) = &country.governing_office {
            let office = self
                .offices
                .iter()
                .find(|office| {
                    &office.id == id
                        && office.country == country.id
                        && office.validity.contains(date)
                })
                .ok_or("Governing office is absent, inactive or belongs to another country")?;
            let term = self
                .terms
                .iter()
                .find(|term| &term.office == id && term.validity.contains(date))
                .ok_or("Governing office has no explicit incumbent/vacancy at the bookmark")?;
            country.executive = self.holder_name(&term.holder);
            country.office = office.title.clone();
        }
        if self
            .currencies
            .iter()
            .any(|term| term.country == country.id)
        {
            country.currency = self
                .currencies
                .iter()
                .find(|term| term.country == country.id && term.validity.contains(date))
                .ok_or("Primary-currency chronology has a gap at the bookmark")?
                .code
                .clone();
        }
        Ok(())
    }
    pub(crate) fn validate(&self, content: &Content) -> Result<(), String> {
        let date: CivilDate = content.start.parse()?;
        let sources: BTreeSet<_> = content
            .sources
            .iter()
            .map(|source| source.id.as_str())
            .collect();
        let mut people = BTreeSet::new();
        for person in &self.people {
            if person.id.0.is_empty()
                || !people.insert(&person.id)
                || person.name.trim().is_empty()
                || !sources.contains(person.source.as_str())
                || person.source_version.is_empty()
            {
                return Err("Invalid dated person identity or provenance".into());
            }
        }
        let mut periods: BTreeMap<&OfficeId, Vec<&Validity>> = BTreeMap::new();
        for term in &self.terms {
            periods
                .entry(&term.office)
                .or_default()
                .push(&term.validity);
        }
        let mut offices = BTreeMap::new();
        for office in &self.offices {
            if office.id.0.is_empty()
                || offices.insert(&office.id, office).is_some()
                || content.country(&office.country).is_none()
                || office.title.trim().is_empty()
                || !office.validity.valid()
                || !sources.contains(office.source.as_str())
                || office.source_version.is_empty()
            {
                return Err(
                    "Invalid dated office identity, country, interval or provenance".into(),
                );
            }
            if office.required
                && office.validity.contains(date)
                && periods
                    .get(&office.id)
                    .into_iter()
                    .flatten()
                    .filter(|period| period.contains(date))
                    .count()
                    != 1
            {
                return Err(format!(
                    "Required office {} needs exactly one incumbent, collective or explicit vacancy at the bookmark",
                    office.id.0
                ));
            }
        }
        let mut records = BTreeSet::new();
        for term in &self.terms {
            let office = offices
                .get(&term.office)
                .ok_or("Term references an absent office")?;
            let valid_holder = match &term.holder {
                OfficeHolder::Person { person } => people.contains(person),
                OfficeHolder::Collective { name, members } => {
                    !name.trim().is_empty()
                        && members.iter().all(|person| people.contains(person))
                        && members.iter().collect::<BTreeSet<_>>().len() == members.len()
                }
                OfficeHolder::Vacant => true,
            };
            if term.id.is_empty()
                || !records.insert(term.id.as_str())
                || !valid_holder
                || !term.validity.valid()
                || !office.validity.encloses(&term.validity)
                || !sources.contains(term.source.as_str())
                || term.source_version.is_empty()
            {
                return Err(format!("Invalid or overlapping office term {}", term.id));
            }
        }
        for (office, intervals) in &mut periods {
            intervals.sort_by_key(|period| period.valid_from);
            if intervals.windows(2).any(|pair| pair[0].overlaps(pair[1])) {
                return Err(format!("Overlapping office terms for {}", office.0));
            }
        }
        let mut currency_periods: BTreeMap<&CountryId, Vec<&Validity>> = BTreeMap::new();
        for term in &self.currencies {
            if term.id.is_empty()
                || !records.insert(term.id.as_str())
                || content.country(&term.country).is_none()
                || !(3..=12).contains(&term.code.len())
                || !term
                    .code
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
                || !term.validity.valid()
                || !sources.contains(term.source.as_str())
                || term.source_version.is_empty()
            {
                return Err(format!(
                    "Invalid or overlapping primary-currency term {}",
                    term.id
                ));
            }
            currency_periods
                .entry(&term.country)
                .or_default()
                .push(&term.validity);
        }
        for (country, intervals) in &mut currency_periods {
            intervals.sort_by_key(|period| period.valid_from);
            if intervals.windows(2).any(|pair| pair[0].overlaps(pair[1])) {
                return Err(format!(
                    "Overlapping primary-currency terms for {}",
                    country.0
                ));
            }
        }
        Ok(())
    }
}
