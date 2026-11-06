//! Authored party policy positions, separate from ideology labels and held backing.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreferredPolicyRange {
    pub min: i32,
    pub max: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartyPolicyPosition {
    pub policy: PolicyId,
    pub importance: u16,
    /// None is explicitly unknown; it does not mean a preferred level of zero.
    #[serde(deserialize_with = "required_preference")]
    pub preferred: Option<PreferredPolicyRange>,
}
fn required_preference<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PreferredPolicyRange>, D::Error> {
    Option::deserialize(deserializer)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartyEconomicProfile {
    pub country: CountryId,
    pub party: PartyId,
    pub positions: Vec<PartyPolicyPosition>,
    pub source: String,
    pub source_version: String,
    pub quality: RecordQuality,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CountryPartyInfluence {
    pub country: CountryId,
    pub legislative_max_penalty: i32,
    pub party_leadership_max_penalty: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartyPreferenceDefinition {
    pub profiles: Vec<PartyEconomicProfile>,
    pub influences: Vec<CountryPartyInfluence>,
    /// Provenance for the influence model, separate from each party's positions.
    pub source: String,
    pub source_version: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PartyBackingMetric {
    Legislative,
    PartyLeadership,
}
impl PartyBackingMetric {
    pub fn label(self) -> &'static str {
        match self {
            Self::Legislative => "legislative support",
            Self::PartyLeadership => "party-leadership/elite backing",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartyAlignmentContribution {
    pub party: PartyId,
    pub weight: u32,
    pub known_importance: u32,
    pub total_importance: u32,
    pub distance_bps: Option<BasisPoints>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PartyAlignmentReport {
    pub country: CountryId,
    pub metric: Option<PartyBackingMetric>,
    pub max_penalty: i32,
    pub penalty: i32,
    pub known_positions_bps: BasisPoints,
    pub parties: Vec<PartyAlignmentContribution>,
    pub source: String,
    pub source_version: String,
}
struct AlignmentScore {
    metric: Option<PartyBackingMetric>,
    cap: i32,
    penalty: i32,
    known: u32,
    parties: Vec<PartyAlignmentContribution>,
}
impl PartyPreferenceDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        let attributed = |source: &str, revision: &str| {
            !revision.trim().is_empty()
                && revision.len() <= 128
                && content.sources.iter().any(|s| s.id == source)
        };
        if !attributed(&self.source, &self.source_version) || self.profiles.len() > 8192 {
            return Err("Invalid party-preference model provenance or roster size".into());
        }
        let mut profiles = BTreeSet::new();
        for profile in &self.profiles {
            let country = content
                .country(&profile.country)
                .ok_or("Party profile references absent country")?;
            if profile.party.0.trim().is_empty()
                || profile.party.0.len() > 64
                || country.party(&profile.party).is_none()
                || !profiles.insert((profile.country.clone(), profile.party.clone()))
                || !attributed(&profile.source, &profile.source_version)
                || profile.positions.len() != 3
            {
                return Err(
                    "Party profiles need unique stable party references and attributed positions"
                        .into(),
                );
            }
            let mut policies = BTreeSet::new();
            for position in &profile.positions {
                let policy = content
                    .policy(&position.policy)
                    .ok_or("Party position references absent policy")?;
                if !FiscalTargets::is_fiscal(&position.policy)
                    || !policies.insert(&position.policy)
                    || !(1..=100).contains(&position.importance)
                    || policy.min >= policy.max
                {
                    return Err(
                        "Party positions require distinct fiscal axes and bounded importance"
                            .into(),
                    );
                }
                if let Some(range) = &position.preferred
                    && (range.min > range.max
                        || range.min < policy.min
                        || range.max > policy.max
                        || profile.quality == RecordQuality::Unknown)
                {
                    return Err(
                        "Known party positions need bounded preferred ranges and stated quality"
                            .into(),
                    );
                }
            }
        }
        let expected: BTreeSet<_> = content
            .countries
            .iter()
            .flat_map(|c| c.parties.iter().map(|p| (c.id.clone(), p.id.clone())))
            .collect();
        if profiles != expected {
            return Err(
                "Each registered party needs explicit known/unknown fiscal positions".into(),
            );
        }
        let mut countries = BTreeSet::new();
        for rule in &self.influences {
            if content.country(&rule.country).is_none()
                || !countries.insert(rule.country.clone())
                || !(0..=20).contains(&rule.legislative_max_penalty)
                || !(0..=20).contains(&rule.party_leadership_max_penalty)
            {
                return Err(
                    "Party influence needs unique country rules and 0–20 point limits".into(),
                );
            }
        }
        if countries != content.countries.iter().map(|c| c.id.clone()).collect() {
            return Err(
                "Each country needs an explicit party-influence rule, including advisory zeroes"
                    .into(),
            );
        }
        Ok(())
    }
    fn score(
        &self,
        content: &Content,
        definition: &CountryDefinition,
        country: &Country,
        route: &DecisionRoute,
        targets: &FiscalTargets,
        details: bool,
    ) -> Result<AlignmentScore, String> {
        if !targets.valid(content) {
            return Err("Party alignment requires supported fiscal policy levels".into());
        }
        let rule = self
            .influences
            .iter()
            .find(|r| r.country == country.id)
            .ok_or("Missing party influence rule")?;
        let (metric, cap) = match route {
            DecisionRoute::Legislature => (
                Some(PartyBackingMetric::Legislative),
                rule.legislative_max_penalty,
            ),
            DecisionRoute::PartyCouncil => (
                Some(PartyBackingMetric::PartyLeadership),
                rule.party_leadership_max_penalty,
            ),
            _ => (None, 0),
        };
        let mut total_weight = 0_u64;
        let mut weighted_distance = 0_u64;
        let mut weighted_known = 0_u64;
        let mut parties = Vec::new();
        for party in &definition.parties {
            let profile = self
                .profiles
                .iter()
                .find(|p| p.country == country.id && p.party == party.id)
                .ok_or("Missing party position profile")?;
            let weight = match metric {
                Some(PartyBackingMetric::Legislative) if country.coalition.contains(&party.id) => {
                    u32::from(*country.seats.get(&party.id).ok_or("Missing party seats")?)
                }
                Some(PartyBackingMetric::PartyLeadership) if country.ruling_party == party.id => 1,
                _ => 0,
            };
            let mut importance = 0_u32;
            let mut known = 0_u32;
            let mut distance = 0_u64;
            for position in &profile.positions {
                importance += u32::from(position.importance);
                if let Some(range) = &position.preferred {
                    known += u32::from(position.importance);
                    let value = targets
                        .entries()
                        .into_iter()
                        .find(|(id, _)| *id == position.policy)
                        .ok_or("Unsupported party policy axis")?
                        .1;
                    let policy = content
                        .policy(&position.policy)
                        .ok_or("Missing party policy definition")?;
                    let gap = (range.min - value).max(value - range.max).max(0);
                    let normalized = u64::try_from(gap).map_err(|_| "Negative party distance")?
                        * 10_000
                        / (policy.max - policy.min) as u64;
                    distance += u64::from(position.importance) * normalized;
                }
            }
            if importance == 0 {
                return Err("Party profile has no policy importance".into());
            }
            let distance_bps = distance / u64::from(importance);
            total_weight += u64::from(weight);
            weighted_distance += u64::from(weight) * distance_bps;
            weighted_known += u64::from(weight) * u64::from(known) * 10_000 / u64::from(importance);
            if details {
                parties.push(PartyAlignmentContribution {
                    party: party.id.clone(),
                    weight,
                    known_importance: known,
                    total_importance: importance,
                    distance_bps: (known > 0).then_some(BasisPoints::new(distance_bps as u32)),
                });
            }
        }
        let penalty = if total_weight == 0 {
            0
        } else {
            (weighted_distance * cap as u64 / (total_weight * 10_000)) as i32
        };
        let known = weighted_known.checked_div(total_weight).unwrap_or(0) as u32;
        Ok(AlignmentScore {
            metric,
            cap,
            penalty,
            known,
            parties,
        })
    }
    pub(crate) fn operating_penalty(
        &self,
        content: &Content,
        definition: &CountryDefinition,
        country: &Country,
        route: &DecisionRoute,
    ) -> Result<i32, String> {
        self.score(
            content,
            definition,
            country,
            route,
            &FiscalTargets::current(country),
            false,
        )
        .map(|score| score.penalty)
    }
    fn report(
        &self,
        content: &Content,
        definition: &CountryDefinition,
        country: &Country,
        route: &DecisionRoute,
        targets: &FiscalTargets,
    ) -> Result<PartyAlignmentReport, String> {
        let score = self.score(content, definition, country, route, targets, true)?;
        Ok(PartyAlignmentReport {
            country: country.id.clone(),
            metric: score.metric,
            max_penalty: score.cap,
            penalty: score.penalty,
            known_positions_bps: BasisPoints::new(score.known),
            parties: score.parties,
            source: self.source.clone(),
            source_version: self.source_version.clone(),
        })
    }
}
impl World {
    /// Public positions and sponsoring parties assessed against explicitly supplied
    /// targets. Does not publish the country's private backing or operating settings.
    pub fn party_alignment_for(
        &self,
        content: &Content,
        country: &CountryId,
        targets: &FiscalTargets,
    ) -> Result<Option<PartyAlignmentReport>, String> {
        if self.content_hash != content.hash() || self.rules != content.rules {
            return Err("Party alignment requires the campaign's exact content pack".into());
        }
        let c = self.countries.get(country).ok_or("Unknown country")?;
        let d = content
            .country(country)
            .ok_or("Missing country definition")?;
        let route = &self.governing_rules(content, country)?.route;
        content
            .party_preferences
            .as_ref()
            .map(|rules| rules.report(content, d, c, route, targets))
            .transpose()
    }
    pub(crate) fn party_preference_forecast_text(
        &self,
        content: &Content,
        country: &CountryId,
        targets: &FiscalTargets,
    ) -> Result<String, String> {
        if content.party_preferences.is_none() {
            return Ok(String::new());
        }
        let current = FiscalTargets::current(self.countries.get(country).ok_or("Unknown country")?);
        let before = self
            .party_alignment_for(content, country, &current)?
            .expect("preferences");
        let c = self.countries.get(country).ok_or("Unknown country")?;
        let after = content
            .party_preferences
            .as_ref()
            .expect("preferences")
            .report(
                content,
                content
                    .country(country)
                    .ok_or("Missing country definition")?,
                c,
                &self.governing_rules(content, country)?.route,
                targets,
            )?;
        let Some(metric) = after.metric else {
            return Ok(" Party positions are advisory for this governing route.".into());
        };
        Ok(format!(
            " Party alignment would reduce the {} target by {} points after operation (current operating reduction {}). Held backing moves at monthly political reviews alongside other opinion and funding effects. Known position coverage: {}.{:02}%.",
            metric.label(),
            after.penalty,
            before.penalty,
            after.known_positions_bps.units() / 100,
            after.known_positions_bps.units() % 100
        ))
    }
}
