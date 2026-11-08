//! Paid public auxiliary capacity. Installation does not create people, income,
//! civilian goods or GDP; funded maintenance constrains added equipment.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const INDUSTRY_UPKEEP_DENOMINATOR: u64 = 365 * 10000;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustryInvestmentDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub policies: BTreeMap<PolicyId, SectorId>,
    pub maximum_levels: u32,
    pub maintenance_bps_per_level: BasisPoints,
    pub countries: BTreeMap<CountryId, BTreeMap<SectorId, ProductionBatches>>,
}
impl IndustryInvestmentDefinition {
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        let production = content
            .production
            .as_ref()
            .ok_or("Industry requires production")?;
        if self.description.trim().is_empty()
            || self.description.len() > 4096
            || self.source_version.trim().is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
            || !(1..=10).contains(&self.maximum_levels)
            || !(1..=100).contains(&self.maintenance_bps_per_level.units())
            || self.policies.len() > 16
            || self.policies.len() != production.framework.sectors.len()
            || self.policies.values().collect::<BTreeSet<_>>()
                != production.framework.sectors.keys().collect()
            || self.countries.keys().ne(production.countries.keys())
        {
            return Err("Industry needs bounded, attributed, exact country/sector coverage".into());
        }
        for (policy, sector) in &self.policies {
            if !policy.as_str().starts_with("industry_")
                || content.policy(policy).is_none()
                || !production.construction_policy_weights.contains_key(policy)
                || !production.framework.sectors.contains_key(sector)
            {
                return Err(
                    "Industry investment needs an explicit costed material-consuming policy".into(),
                );
            }
        }
        for (id, steps) in &self.countries {
            if steps.keys().ne(production.framework.sectors.keys())
                || steps.values().any(|q| q.0 == 0)
            {
                return Err("Industry needs positive country-specific build steps".into());
            }
            self.assets(
                production,
                id,
                &steps
                    .keys()
                    .map(|s| (s.clone(), self.maximum_levels))
                    .collect(),
                100,
            )?;
        }
        Ok(())
    }
    pub fn assets(
        &self,
        production: &ProductionDefinition,
        id: &CountryId,
        levels: &BTreeMap<SectorId, u32>,
        readiness: i32,
    ) -> Result<IndustryAssets, String> {
        let opening = production
            .countries
            .get(id)
            .ok_or("Missing industry country")?;
        let steps = self
            .countries
            .get(id)
            .ok_or("Missing industry build steps")?;
        if !(0..=100).contains(&readiness)
            || levels.keys().ne(production.framework.sectors.keys())
            || levels.values().any(|n| *n > self.maximum_levels)
        {
            return Err("Invalid industry levels/readiness".into());
        }
        let mut installed = opening.installed.clone();
        let mut capital = opening.capital.clone();
        for (sector, level) in levels {
            let added = steps[sector]
                .0
                .checked_mul(u64::from(*level))
                .ok_or("Industry capacity overflow")?;
            let effective = u64::try_from(u128::from(added) * readiness as u128 / 100)
                .map_err(|_| "Industry capacity overflow")?;
            installed
                .get_mut(sector)
                .ok_or("Missing industry sector")?
                .0 = installed[sector]
                .0
                .checked_add(effective)
                .ok_or("Industry capacity overflow")?;
            for (kind, per_batch) in &production.framework.sectors[sector].capital_per_batch {
                let amount = effective
                    .checked_mul(per_batch.0)
                    .ok_or("Industry capital overflow")?;
                let current = capital
                    .get_mut(kind)
                    .ok_or("Missing industry equipment kind")?;
                current.0 = current
                    .0
                    .checked_add(amount)
                    .ok_or("Industry capital overflow")?;
            }
        }
        Ok(IndustryAssets { installed, capital })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustryAssets {
    pub installed: BTreeMap<SectorId, ProductionBatches>,
    pub capital: BTreeMap<CapitalKindId, CapitalServices>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustryInvestmentState {
    pub levels: BTreeMap<SectorId, u32>,
    pub maintenance_bps_per_level: BasisPoints,
    /// Fractional creditor claim, not spendable cash; denominator is pinned.
    pub upkeep_remainder: u64,
}
impl IndustryInvestmentState {
    pub fn opening(definition: &IndustryInvestmentDefinition) -> Self {
        Self {
            levels: definition
                .policies
                .values()
                .map(|s| (s.clone(), 0))
                .collect(),
            maintenance_bps_per_level: definition.maintenance_bps_per_level,
            upkeep_remainder: 0,
        }
    }
    pub fn level_count(&self) -> u32 {
        self.levels.values().sum()
    }
    pub fn upkeep_projection(&self, gdp: AnnualGdp) -> IndustryUpkeepProjection {
        IndustryUpkeepProjection {
            gdp,
            levels: self.level_count(),
            rate: self.maintenance_bps_per_level,
            opening_remainder: self.upkeep_remainder,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustryUpkeepProjection {
    pub gdp: AnnualGdp,
    pub levels: u32,
    pub rate: BasisPoints,
    pub opening_remainder: u64,
}
impl IndustryUpkeepProjection {
    pub fn valid(&self) -> bool {
        (0..=1_000_000_000_000).contains(&self.gdp.units())
            && self.levels <= 160
            && (1..=100).contains(&self.rate.units())
            && self.opening_remainder < INDUSTRY_UPKEEP_DENOMINATOR
    }
    pub fn assessment(&self, days: DaySpan) -> Result<(Money, u64), String> {
        if !self.valid() {
            return Err("Invalid industry upkeep projection".into());
        }
        let rate = self
            .rate
            .units()
            .checked_mul(self.levels)
            .ok_or("Industry upkeep overflow")?;
        let amount = BasisPoints::new(rate)
            .accrue(
                Money(self.gdp.units()),
                days,
                DaySpan::new(365),
                MoneyRemainder::new(self.opening_remainder, INDUSTRY_UPKEEP_DENOMINATOR),
            )
            .ok_or("Industry upkeep overflow")?;
        Ok((amount.whole, amount.remainder.numerator()))
    }
    pub fn daily_numerator(&self) -> i64 {
        self.gdp.units() * i64::from(self.rate.units()) * i64::from(self.levels)
    }
    pub(crate) fn rounding_adjustment(&self) -> i64 {
        self.due(DaySpan::new(1))
            .expect("valid industry projection")
            .0
            * INDUSTRY_UPKEEP_DENOMINATOR as i64
            - self.daily_numerator()
    }
    pub fn due(&self, days: DaySpan) -> Result<Money, String> {
        self.assessment(days).map(|a| a.0)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndustryOpeningFactors {
    pub levels: BTreeMap<SectorId, u32>,
    pub maintenance_readiness: i32,
    pub upkeep_opening_remainder: u64,
}
impl World {
    pub(crate) fn assess_industry_upkeep(
        &mut self,
        content: &Content,
        id: &CountryId,
    ) -> Result<Money, String> {
        let Some(state) = self
            .countries
            .get_mut(id)
            .ok_or("Industry country")?
            .industry
            .as_mut()
        else {
            return Ok(Money(0));
        };
        let (due, carry) = state
            .upkeep_projection(content.country(id).ok_or("Industry definition")?.gdp)
            .assessment(DaySpan::new(1))?;
        state.upkeep_remainder = carry;
        Ok(due)
    }
    pub(crate) fn industry_factors(&self, id: &CountryId) -> Option<IndustryOpeningFactors> {
        self.countries[id]
            .industry
            .as_ref()
            .map(|s| IndustryOpeningFactors {
                levels: s.levels.clone(),
                maintenance_readiness: self.production_maintenance(id),
                upkeep_opening_remainder: s.upkeep_remainder,
            })
    }
    pub(crate) fn industry_capacity_days(
        &self,
        content: &Content,
        id: &CountryId,
        sector: &SectorId,
        elapsed: u128,
    ) -> u128 {
        let production = content.production.as_ref().expect("production");
        let mut result = u128::from(production.countries[id].installed[sector].0) * elapsed;
        if let Some(definition) = &content.industry {
            for record in self.countries[id]
                .commitments
                .as_deref()
                .unwrap_or_default()
            {
                if definition.policies.get(&record.project.policy) == Some(sector)
                    && let Some(op) = record
                        .milestones
                        .iter()
                        .find(|m| m.stage == CommitmentStage::Operating)
                {
                    result += u128::from(definition.countries[id][sector].0)
                        * u128::from(
                            self.day
                                .elapsed_since(op.day)
                                .expect("ordered operation")
                                .units(),
                        );
                }
            }
        }
        result
    }
    pub(crate) fn ai_industry_investment(&mut self, content: &Content, id: &CountryId) -> bool {
        let Some(definition) = &content.industry else {
            return false;
        };
        let Some(last) = self
            .production
            .as_ref()
            .and_then(|p| p.countries.get(id))
            .and_then(|p| p.last.as_ref())
        else {
            return false;
        };
        let production = content.production.as_ref().expect("industry production");
        let country = &self.countries[id];
        let mut wanted = Vec::new();
        if last.service_delivered.0 < last.service_required.0 {
            wanted.push(production.service_sector.clone());
        }
        if country.projects.iter().any(|p| {
            p.stage == PolicyStage::Implementing
                && p.due_day <= self.day
                && production
                    .construction_required(id, &p.policy)
                    .ok()
                    .flatten()
                    .is_some_and(|q| {
                        self.construction_available(content, id)
                            .is_some_and(|a| a.0 < q.0)
                    })
        }) {
            wanted.extend(
                production
                    .framework
                    .sectors
                    .iter()
                    .filter(|(_, r)| r.outputs_per_batch.contains_key(&production.assembly_good))
                    .map(|(s, _)| s.clone()),
            );
        }
        let mut seen = BTreeSet::new();
        let mut choice = None;
        while let Some(sector) = wanted.first().cloned() {
            wanted.remove(0);
            if !seen.insert(sector.clone()) {
                continue;
            }
            let Some(receipt) = last
                .physical
                .production
                .sectors
                .iter()
                .find(|r| r.sector == sector)
            else {
                continue;
            };
            let binding: Vec<_> = receipt.limits.iter().filter(|l| l.binding).collect();
            if binding
                .iter()
                .any(|l| l.constraint == ProductionConstraint::InstalledCapacity)
                && binding
                    .iter()
                    .all(|l| l.constraint == ProductionConstraint::InstalledCapacity)
                && let Some(policy) = definition
                    .policies
                    .iter()
                    .find(|(_, s)| **s == sector)
                    .map(|(p, _)| p.clone())
                && self.preview(content, id, &policy, 1).is_ok()
                && country.available_cash().0
                    >= content.country(id).expect("country").gdp.units() / 100
                        + self
                            .preview(content, id, &policy, 1)
                            .expect("preview")
                            .cost
                            .0
            {
                choice = Some(policy);
                break;
            }
            for limit in binding {
                if let ProductionConstraint::Input(good) = &limit.constraint {
                    wanted.extend(
                        production
                            .framework
                            .sectors
                            .iter()
                            .filter(|(_, r)| r.outputs_per_batch.contains_key(good))
                            .map(|(s, _)| s.clone()),
                    );
                }
            }
        }
        let Some(policy) = choice else {
            return false;
        };
        let command = self.command(id.clone(), Action::Enact { policy, value: 1 });
        match self.apply(content, command, crate::developer::CommandOrigin::Ai) {
            Ok(()) => {
                self.event(id,false,"AI commissioned costed industry investment for an observed service or material bottleneck through ordinary consent, funding and construction rules.");
                true
            }
            Err(_) => false,
        }
    }
    pub(crate) fn validate_industry(&self, content: &Content) -> Result<(), String> {
        for (id, country) in &self.countries {
            let (definition, state) = match (&content.industry, &country.industry) {
                (None, None) => continue,
                (Some(d), Some(s)) => (d, s),
                _ => return Err("Industry state differs from pinned rules".into()),
            };
            let production = content.production.as_ref().ok_or("Industry production")?;
            if state.maintenance_bps_per_level != definition.maintenance_bps_per_level
                || state.upkeep_remainder >= INDUSTRY_UPKEEP_DENOMINATOR
            {
                return Err("Industry upkeep differs from pinned rules".into());
            }
            definition.assets(production, id, &state.levels, 100)?;
            let mut level_days = 0_u128;
            let mut levels: BTreeMap<_, u32> = definition
                .policies
                .values()
                .map(|s| (s.clone(), 0))
                .collect();
            let mut deliveries: Vec<_> = country
                .commitments
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter_map(|record| {
                    let sector = definition.policies.get(&record.project.policy)?;
                    let operation = record
                        .milestones
                        .iter()
                        .find(|m| m.stage == CommitmentStage::Operating)?;
                    Some((operation.day, record.id, sector, record))
                })
                .collect();
            deliveries.sort_by_key(|(day, id, _, _)| (*day, *id));
            for (operation, _, sector, record) in deliveries {
                let before = levels.get_mut(sector).ok_or("Industry sector")?;
                if record.direct_effects.len() != 1
                    || record.direct_effects[0].field != format!("industry_level:{}", sector.0)
                    || record.direct_effects[0].before != *before as i32
                    || record.direct_effects[0].after != *before as i32 + 1
                {
                    return Err(
                        "Industry direct effects differ from ordered operating asset history"
                            .into(),
                    );
                }
                *before = before.checked_add(1).ok_or("Industry level overflow")?;
                level_days += u128::from(
                    self.day
                        .elapsed_since(operation)
                        .ok_or("Industry operation chronology")?
                        .units(),
                );
            }
            let expected_carry =
                (u128::from(content.country(id).ok_or("Industry GDP")?.gdp.units() as u64)
                    * u128::from(definition.maintenance_bps_per_level.units())
                    * level_days)
                    % u128::from(INDUSTRY_UPKEEP_DENOMINATOR);
            if u128::from(state.upkeep_remainder) != expected_carry {
                return Err(
                    "Industry upkeep carry differs from actual installed level-days".into(),
                );
            }
            for forecast in std::iter::once(&country.budget.forecast)
                .chain(country.budget_reviews.iter().map(|r| &r.period.forecast))
            {
                let projection = forecast
                    .industry_upkeep
                    .as_ref()
                    .ok_or("Missing industrial upkeep forecast")?;
                if !projection.valid()
                    || projection.gdp != content.country(id).ok_or("Industry GDP")?.gdp
                    || projection.rate != definition.maintenance_bps_per_level
                    || projection.levels > state.level_count()
                {
                    return Err("Industrial forecast differs from pinned assets and rates".into());
                }
            }
            if levels != state.levels {
                return Err(
                    "Installed industry differs from paid operating commitment history".into(),
                );
            }
            if self.day == content.start_day()?
                && (state.level_count() != 0 || state.upkeep_remainder != 0)
            {
                return Err("Opening industry contains invented investment".into());
            }
        }
        Ok(())
    }
}
