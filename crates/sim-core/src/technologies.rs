//! Authored programme eligibility, distinct from research or adoption simulation.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct TechnologyId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TechnologyDefinition {
    pub id: TechnologyId,
    pub name: String,
    /// Earliest scenario date on which this prerequisite permits commissioning.
    /// This is not a prediction of discovery or automatic historical adoption.
    pub available_from: CivilDate,
    pub eligible_countries: Vec<CountryId>,
    pub source: String,
    pub source_version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyTechnologyRequirement {
    pub policy: String,
    pub technologies: Vec<TechnologyId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TechnologyRules {
    pub definitions: Vec<TechnologyDefinition>,
    pub policy_requirements: Vec<PolicyTechnologyRequirement>,
}
impl TechnologyRules {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if !(1..=128).contains(&self.definitions.len())
            || self.policy_requirements.is_empty()
            || self.policy_requirements.len() > content.policies.len()
        {
            return Err(
                "Technology eligibility needs bounded definitions and policy prerequisites".into(),
            );
        }
        let mut ids = BTreeSet::new();
        for technology in &self.definitions {
            let countries: BTreeSet<_> = technology.eligible_countries.iter().collect();
            if technology.id.0.is_empty()
                || technology.id.0.len() > 64
                || !technology
                    .id
                    .0
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
                || !ids.insert(&technology.id)
                || technology.name.trim().is_empty()
                || technology.name.len() > 128
                || technology.available_from > CivilDate::new(2100, 1, 1)?
                || countries.is_empty()
                || countries.len() != technology.eligible_countries.len()
                || countries.iter().any(|id| content.country(id).is_none())
                || !content.sources.iter().any(|s| s.id == technology.source)
                || technology.source_version.trim().is_empty()
            {
                return Err(
                    "Invalid technology ID, date, country eligibility or provenance".into(),
                );
            }
        }
        let mut policies = BTreeSet::new();
        for requirement in &self.policy_requirements {
            let required: BTreeSet<_> = requirement.technologies.iter().collect();
            if content.policy(&requirement.policy).is_none()
                || !policies.insert(&requirement.policy)
                || !(1..=8).contains(&required.len())
                || required.len() != requirement.technologies.len()
                || required.iter().any(|id| !ids.contains(id))
            {
                return Err("Unknown or duplicate policy/technology prerequisite".into());
            }
        }
        Ok(())
    }
    pub fn technology(&self, id: &TechnologyId) -> Option<&TechnologyDefinition> {
        self.definitions.iter().find(|t| t.id == *id)
    }

    fn eligibility(
        &self,
        country: &CountryId,
        policy: &str,
        day: u32,
    ) -> Result<Vec<TechnologyEligibility>, String> {
        if day > MAX_DAY {
            return Err("Technology eligibility date exceeds the campaign calendar".into());
        }
        let today: CivilDate = date(day).parse()?;
        let requirement = self.policy_requirements.iter().find(|r| r.policy == policy);
        requirement
            .into_iter()
            .flat_map(|r| &r.technologies)
            .map(|id| {
                let technology = self
                    .technology(id)
                    .ok_or("Unknown technology prerequisite")?;
                Ok(TechnologyEligibility {
                    technology: id.clone(),
                    name: technology.name.clone(),
                    available_from: technology.available_from,
                    country_eligible: technology.eligible_countries.contains(country),
                    date_eligible: technology.available_from <= today,
                })
            })
            .collect()
    }
    pub(crate) fn check_at(
        &self,
        country: &CountryId,
        policy: &str,
        day: u32,
    ) -> Result<(), String> {
        self.eligibility(country, policy, day)?
            .iter()
            .try_for_each(TechnologyEligibility::ensure_available)
    }
}

/// Public scenario information. Availability grants no cash, capacity or research.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TechnologyEligibility {
    pub technology: TechnologyId,
    pub name: String,
    pub available_from: CivilDate,
    pub country_eligible: bool,
    pub date_eligible: bool,
}
impl TechnologyEligibility {
    pub fn available(&self) -> bool {
        self.country_eligible && self.date_eligible
    }
    fn ensure_available(&self) -> Result<(), String> {
        if !self.country_eligible {
            return Err(format!(
                "{} is not eligible for this country in the loaded scenario",
                self.name
            ));
        }
        if !self.date_eligible {
            return Err(format!(
                "{} is required; programme commissioning becomes eligible on {}",
                self.name, self.available_from
            ));
        }
        Ok(())
    }
}
impl World {
    pub fn policy_technology_eligibility(
        &self,
        content: &Content,
        country: &CountryId,
        policy: &str,
    ) -> Result<Vec<TechnologyEligibility>, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Technology eligibility requires the campaign's exact content".into());
        }
        if !self.countries.contains_key(country)
            || content.country(country).is_none()
            || content.policy(policy).is_none()
        {
            return Err("Unknown country or policy for technology eligibility".into());
        }
        let Some(rules) = &content.technologies else {
            return Ok(Vec::new());
        };
        rules.eligibility(country, policy, self.day)
    }
    pub(crate) fn check_policy_technologies(
        &self,
        content: &Content,
        country: &CountryId,
        policy: &str,
    ) -> Result<(), String> {
        if content.technologies.is_none() {
            return Ok(());
        }
        for technology in self.policy_technology_eligibility(content, country, policy)? {
            technology.ensure_available()?;
        }
        Ok(())
    }
    pub(crate) fn notice_technology_dates(&mut self, content: &Content, country: &CountryId) {
        let Some(rules) = &content.technologies else {
            return;
        };
        let today: CivilDate = date(self.day).parse().expect("campaign date");
        for technology in rules
            .definitions
            .iter()
            .filter(|t| t.available_from == today && t.eligible_countries.contains(country))
        {
            let policies: Vec<_> = rules
                .policy_requirements
                .iter()
                .filter(|r| r.technologies.contains(&technology.id))
                .map(|r| {
                    content
                        .policy(&r.policy)
                        .expect("validated policy")
                        .name
                        .as_str()
                })
                .collect();
            if !policies.is_empty() {
                self.event(country, true, format!("Technology eligibility date reached: {}. Eligible programmes: {}. Other prerequisites, funding, governing consent and implementation are still required; no capacity or research is granted.",technology.name,policies.join(", ")));
            }
        }
    }
}
