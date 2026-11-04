//! Versioned, validated content, separate from authoritative formulas.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CountryId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Regime {
    Parliamentary,
    Presidential,
    SemiPresidential,
    SingleParty,
    AbsoluteMonarchy,
    Theocracy,
    Personalist,
    Military,
}
impl Regime {
    pub fn competitive(&self) -> bool {
        matches!(
            self,
            Self::Parliamentary | Self::Presidential | Self::SemiPresidential
        )
    }
    pub fn route(&self) -> &'static str {
        match self {
            Self::Parliamentary | Self::Presidential | Self::SemiPresidential => {
                "Legislative vote (50 support required)"
            }
            Self::SingleParty => "Party leadership decision (40 elite backing required)",
            Self::AbsoluteMonarchy => "Royal decree (40 elite backing required)",
            Self::Theocracy => "Religious council approval (40 elite backing required)",
            Self::Personalist => "Executive decree (40 elite backing required)",
            Self::Military => "Military council decision (40 elite backing required)",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartyDefinition {
    pub id: String,
    pub name: String,
    pub leader: String,
    pub ideology: String,
    pub seats: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CountryDefinition {
    pub id: CountryId,
    pub name: String,
    pub capital: String,
    pub regime: Regime,
    pub executive: String,
    pub office: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governing_office: Option<crate::OfficeId>,
    pub currency: String,
    pub population: u64,
    pub gdp: i64,
    pub tax: i32,
    pub services: i32,
    pub welfare: i32,
    pub energy: i32,
    pub food: i32,
    pub capacity: i32,
    pub approval: i32,
    pub longitude: i32,
    pub latitude: i32,
    pub ruling_party: String,
    pub coalition: Vec<String>,
    pub parties: Vec<PartyDefinition>,
    pub households: Vec<crate::HouseholdDefinition>,
    pub government: crate::GovernmentDefinition,
    pub quality: String,
    pub identity_source: String,
    pub economic_source: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyDefinition {
    pub id: String,
    pub name: String,
    pub min: i32,
    pub max: i32,
    pub days: u32,
    pub cost_bps: i64,
    pub description: String,
    pub groups: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    pub id: String,
    pub quality: String,
    pub description: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoricalFact {
    pub country: CountryId,
    pub field: String,
    pub observation_period: String,
    pub original_value: String,
    pub original_units: String,
    pub applied_value: i64,
    pub source: String,
    pub source_version: String,
    pub quality: String,
    pub transformation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Content {
    pub schema: u32,
    #[serde(
        default = "crate::markets::legacy_rules",
        skip_serializing_if = "crate::markets::is_legacy_rules"
    )]
    pub rules: u32,
    pub id: String,
    pub name: String,
    pub start: String,
    pub description: String,
    pub sources: Vec<Source>,
    #[serde(default)]
    pub facts: Vec<HistoricalFact>,
    pub countries: Vec<CountryDefinition>,
    pub policies: Vec<PolicyDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identities: Option<crate::IdentityCatalogue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markets: Option<crate::MarketDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finance: Option<crate::FinanceDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constitutions: Option<crate::ConstitutionDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commitments: Option<crate::CommitmentDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cabinet: Option<crate::CabinetDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elections: Option<crate::ElectionDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subnational: Option<crate::SubnationalDefinition>,
}
impl Content {
    pub fn bundled() -> Result<Self, String> {
        Self::from_json(include_str!("../../../data/initial-2000-observed.json"))
    }
    /// Preserved rules-8 roster before expanded population/GDP observations.
    pub fn electoral_bundled() -> Result<Self, String> {
        Self::from_json(include_str!("../../../data/initial-2000-elections.json"))
    }
    /// Preserved rules-7 pack for campaigns begun before independent electoral mandates.
    pub fn cabinet_bundled() -> Result<Self, String> {
        Self::from_json(include_str!("../../../data/initial-2000-cabinet.json"))
    }
    /// Preserved rules-6 pack for campaigns begun before ministerial appointments.
    pub fn commitment_bundled() -> Result<Self, String> {
        Self::from_json(include_str!("../../../data/initial-2000-commitments.json"))
    }
    /// Preserved rules-5 pack for campaigns begun before policy lifecycle controls.
    pub fn constitution_bundled() -> Result<Self, String> {
        Self::from_json(include_str!(
            "../../../data/initial-2000-constitutions.json"
        ))
    }
    /// Preserved rules-4 pack for campaigns begun before constitutional changes.
    pub fn finance_bundled() -> Result<Self, String> {
        Self::from_json(include_str!("../../../data/initial-2000-finance.json"))
    }
    /// Preserved rules-3 pack for campaigns begun before financing authority.
    pub fn market_bundled() -> Result<Self, String> {
        Self::from_json(include_str!("../../../data/initial-2000-markets.json"))
    }
    /// Original rules-2 data is retained verbatim for existing campaigns.
    pub fn classic_bundled() -> Result<Self, String> {
        Self::from_json(include_str!("../../../data/initial-2000.json"))
    }
    pub fn bundled_for_hash(hash: u64) -> Result<Self, String> {
        for pack in [
            Self::bundled()?,
            Self::electoral_bundled()?,
            Self::cabinet_bundled()?,
            Self::commitment_bundled()?,
            Self::constitution_bundled()?,
            Self::finance_bundled()?,
            Self::market_bundled()?,
            Self::classic_bundled()?,
        ] {
            if pack.hash() == hash {
                return Ok(pack);
            }
        }
        Err(format!(
            "Save requires content {hash:016x}, which is not a bundled pack. Launch with --content and the exact original pack; no replacement was applied."
        ))
    }
    pub fn from_json(json: &str) -> Result<Self, String> {
        let header: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("Content JSON: {e}"))?;
        if !matches!(header.get("schema").and_then(|v| v.as_u64()), Some(2..=9)) {
            return Err("Unsupported content schema: expected version 2 through 9".into());
        }
        let mut content: Self =
            serde_json::from_str(json).map_err(|e| format!("Content JSON: {e}"))?;
        content.assemble_bookmark()?;
        content.validate()?;
        Ok(content)
    }
    pub fn country(&self, id: &CountryId) -> Option<&CountryDefinition> {
        self.countries.iter().find(|c| &c.id == id)
    }
    pub fn policy(&self, id: &str) -> Option<&PolicyDefinition> {
        self.policies.iter().find(|p| p.id == id)
    }
    pub fn hash(&self) -> u64 {
        crate::stable_hash(&serde_json::to_vec(self).expect("content serialization"))
    }
    pub fn start_day(&self) -> Result<u32, String> {
        crate::parse_date(&self.start)
    }
    pub fn at_bookmark(&self, start: &str) -> Result<Self, String> {
        let mut content = self.clone();
        content.start = start.into();
        content.assemble_bookmark()?;
        content.validate()?;
        Ok(content)
    }
    fn assemble_bookmark(&mut self) -> Result<(), String> {
        self.start_day()?;
        self.review_day()?;
        if let Some(identities) = &self.identities {
            identities.validate(self)?;
            let date = self.start.parse()?;
            for country in &mut self.countries {
                identities.resolve(country, date)?;
            }
        }
        Ok(())
    }
    pub fn bookmark_offices(&self, country: &CountryId) -> Vec<crate::BookmarkOffice> {
        self.identities
            .as_ref()
            .map_or_else(Vec::new, |identities| {
                identities.offices_at(country, self.start.parse().expect("validated bookmark"))
            })
    }
    pub fn review_day(&self) -> Result<u32, String> {
        self.start
            .parse::<crate::CivilDate>()?
            .anniversary(5)?
            .campaign_day()
            .map_err(|_| "Bookmark must permit a five-year review no later than 2100-01-01".into())
    }
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.schema, 2..=9) || self.countries.is_empty() {
            return Err("Unsupported content schema, bookmark or empty roster".into());
        }
        self.start_day()?;
        self.review_day()?;
        match (self.rules, &self.markets, &self.finance, &self.constitutions) {
            (2, None, None, None) => {}
            (3, Some(markets), None, None) if self.schema == 3 => markets.validate(self)?,
            (4, Some(markets), Some(finance), None) if self.schema == 4 => {
                markets.validate(self)?; finance.validate(self)?;
            }
            (5..=9, Some(markets), Some(finance), Some(constitutions)) if self.schema == self.rules => {
                markets.validate(self)?; finance.validate(self)?; constitutions.validate(self)?;
            }
            _ => return Err("Supported rules/content definitions are 2 (fixed prices), 3 (markets), 4 (financing authority), 5 (constitutional transitions), 6 (commitment lifecycle), 7 (ministerial appointments), 8 (electoral authorities), and 9 (subnational mandates)".into()),
        }
        match (&self.commitments, self.rules) {
            (Some(rules), 6..=9) => rules.validate(self)?,
            (None, 2..=5) => {}
            _ => return Err("Commitment lifecycle requires rules/content 6".into()),
        }
        match (&self.cabinet, self.rules) {
            (Some(rules), 7..=9) => rules.validate(self)?,
            (None, 2..=6) => {}
            _ => return Err("Ministerial appointments require rules/content 7".into()),
        }
        match (&self.elections, self.rules) {
            (Some(rules), 8 | 9) => rules.validate(self)?,
            (None, 2..=7) => {}
            _ => return Err("Electoral authority workflow requires rules/content 8".into()),
        }
        match (&self.subnational, self.rules) {
            (Some(rules), 9) => rules.validate(self)?,
            (None, 2..=8) => {}
            _ => return Err("Subnational mandates require rules/content 9".into()),
        }
        if let Some(identities) = &self.identities {
            identities.validate(self)?;
            let date = self.start.parse()?;
            for country in &self.countries {
                let mut resolved = country.clone();
                identities.resolve(&mut resolved, date)?;
                if resolved.executive != country.executive
                    || resolved.office != country.office
                    || resolved.currency != country.currency
                {
                    return Err(
                        "Stale bookmark identity cache; assemble the content for its start date"
                            .into(),
                    );
                }
            }
        } else if self
            .countries
            .iter()
            .any(|country| country.governing_office.is_some())
        {
            return Err("Governing-office reference requires an identity catalogue".into());
        }
        let sources: BTreeSet<_> = self.sources.iter().map(|s| &s.id).collect();
        if sources.len() != self.sources.len()
            || self.sources.iter().any(|source| {
                source.id.is_empty() || source.quality.is_empty() || source.description.is_empty()
            })
        {
            return Err("Duplicate or incomplete source definition".into());
        }
        let mut ids = BTreeSet::new();
        let mut party_ids = BTreeSet::new();
        for c in &self.countries {
            if c.id.0.is_empty()
                || !ids.insert(&c.id)
                || c.name.is_empty()
                || c.population < 1000
                || c.population > 2_000_000_000
                || !(1_000..=100_000_000).contains(&c.gdp)
                || !(10..=45).contains(&c.tax)
                || !(4..=18).contains(&c.services)
                || !(0..=12).contains(&c.welfare)
                || !(1..=200).contains(&c.energy)
                || !(1..=200).contains(&c.food)
                || !(1..=100).contains(&c.capacity)
                || !(0..=100).contains(&c.approval)
                || !(-180..=180).contains(&c.longitude)
                || !(-90..=90).contains(&c.latitude)
                || c.executive.is_empty()
                || !sources.contains(&c.identity_source)
                || !sources.contains(&c.economic_source)
            {
                return Err(format!("Invalid country fields or provenance: {}", c.id.0));
            }
            let mut groups = BTreeSet::new();
            if c.households.len() != 4
                || c.households.iter().any(|h| {
                    !groups.insert(&h.id)
                        || !["workers", "public", "vulnerable", "owners"].contains(&h.id.as_str())
                        || h.name.is_empty()
                        || !(1..=100).contains(&h.population_share)
                        || !(0..=100).contains(&h.income_share)
                        || !(0..=100).contains(&h.savings_share)
                        || !(1..=100).contains(&h.working_age_share)
                })
                || c.households.iter().map(|h| h.population_share).sum::<i64>() != 100
                || c.households.iter().map(|h| h.income_share).sum::<i64>() != 100
                || c.households.iter().map(|h| h.savings_share).sum::<i64>() != 100
                || !(1..=100).contains(&c.government.threshold)
                || !(0..=100).contains(&c.government.council_threshold)
                || c.government.consent_days > 60
                || c.government
                    .leadership_review_days
                    .is_some_and(|days| !(30..=1827).contains(&days))
                || c.regime.competitive()
                    != (c.government.route == crate::DecisionRoute::Legislature)
            {
                return Err(format!(
                    "Invalid household partition or government rules: {}",
                    c.id.0
                ));
            }
            let parties: BTreeSet<_> = c.parties.iter().map(|p| &p.id).collect();
            if c.parties.is_empty()
                || c.parties.iter().map(|p| u32::from(p.seats)).sum::<u32>() != 100
                || !parties.contains(&c.ruling_party)
                || c.coalition.is_empty()
                || !c.coalition.contains(&c.ruling_party)
                || c.coalition.iter().collect::<BTreeSet<_>>().len() != c.coalition.len()
                || c.coalition.iter().any(|p| !parties.contains(p))
                || c.parties
                    .iter()
                    .any(|p| p.id.is_empty() || !party_ids.insert(&p.id))
            {
                return Err(format!("Invalid parties, coalition or seats: {}", c.id.0));
            }
        }
        let mut fact_keys = BTreeSet::new();
        let bookmark_year = self.start.parse::<crate::CivilDate>()?.year();
        for fact in &self.facts {
            let c = self
                .country(&fact.country)
                .ok_or("Observation references absent country")?;
            if !fact_keys.insert((&fact.country, &fact.field))
                || !sources.contains(&fact.source)
                || fact.observation_period.len() != 4
                || !fact
                    .observation_period
                    .bytes()
                    .all(|byte| byte.is_ascii_digit())
                || fact
                    .observation_period
                    .parse::<u16>()
                    .ok()
                    .is_none_or(|year| year == 0 || year > bookmark_year)
                || fact.source_version.is_empty()
                || fact.quality.is_empty()
                || fact.original_value.is_empty()
                || fact.transformation.is_empty()
                || match fact.field.as_str() {
                    "population" => {
                        fact.applied_value != c.population as i64
                            || fact.original_units != "persons"
                    }
                    "gdp" => fact.applied_value != c.gdp || fact.original_units != "current USD",
                    _ => true,
                }
            {
                return Err("Invalid dated observation, units or applied transformation".into());
            }
        }
        let mut policies = BTreeSet::new();
        let expected = [
            "income_tax",
            "services",
            "welfare",
            "energy",
            "agriculture",
            "administration",
            "coalition",
        ];
        for p in &self.policies {
            if !expected.contains(&p.id.as_str())
                || !policies.insert(p.id.as_str())
                || p.min > p.max
                || p.days == 0
                || p.days > 365
                || !(0..=1000).contains(&p.cost_bps)
                || match p.id.as_str() {
                    "income_tax" => p.min < 10 || p.max > 45,
                    "services" => p.min < 4 || p.max > 18,
                    "welfare" => p.min < 0 || p.max > 12,
                    _ => p.min != 1 || p.max != 1,
                }
            {
                return Err(format!("Invalid policy: {}", p.id));
            }
        }
        if policies.len() != expected.len() {
            return Err("Missing required policy definitions".into());
        }
        Ok(())
    }
}
