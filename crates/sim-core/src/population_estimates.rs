//! A bounded, deterministic census-precision model, separate from resident stocks.
//! Its intervals describe scenario rounding at a dated release, not statistical
//! confidence or a guarantee about population after that measurement date.
use crate::*;
use serde::{Deserialize, Serialize};

/// Release references are independent of government command identities.
/// ```compile_fail
/// use sim_core::{CensusReleaseId, CommandId};
/// let release: CensusReleaseId = CommandId(1);
/// ```
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CensusReleaseId(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishedPopulationEstimate {
    /// Scoped to the country containing the release.
    pub id: CensusReleaseId,
    pub day: CampaignDay,
    pub population: Population,
    pub lower: Population,
    pub upper: Population,
    pub resolution: Population,
    pub statistical_capacity: Percent,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PopulationMeasurement {
    /// Private simulation stock. A normal foreign report never includes it.
    pub actual: Population,
    pub published: PublishedPopulationEstimate,
}
impl PopulationMeasurement {
    pub fn new(
        id: CensusReleaseId,
        day: CampaignDay,
        actual: Population,
        opening_reference: Population,
        statistical_capacity: Percent,
    ) -> Result<Self, String> {
        if statistical_capacity.units() == 0
            || statistical_capacity.units() > 100
            || day.units() > MAX_DAY
            || id.0 == 0
        {
            return Err("A dated population release needs statistical capacity in 1..=100 and a nonzero identity".into());
        }
        let resolution = population_resolution(opening_reference, statistical_capacity)?;
        let q = u128::from(resolution.0);
        let rounded = (u128::from(actual.0) + q / 2) / q * q;
        let point =
            u64::try_from(rounded).map_err(|_| "Population estimate exceeds count bounds")?;
        let lower = point.saturating_sub(resolution.0 / 2);
        let upper = point
            .checked_add((resolution.0 - 1) / 2)
            .ok_or("Population interval overflow")?;
        let result = Self {
            actual,
            published: PublishedPopulationEstimate {
                id,
                day,
                population: Population(point),
                lower: Population(lower),
                upper: Population(upper),
                resolution,
                statistical_capacity,
            },
        };
        result.validate(opening_reference)?;
        Ok(result)
    }
    pub fn validate(&self, opening_reference: Population) -> Result<(), String> {
        let p = &self.published;
        if p.statistical_capacity.units() == 0
            || p.statistical_capacity.units() > 100
            || p.day.units() > MAX_DAY
            || p.id.0 == 0
            || p.resolution != population_resolution(opening_reference, p.statistical_capacity)?
        {
            return Err("Invalid population measurement identity, date or precision".into());
        }
        let rebuilt_point = (u128::from(self.actual.0) + u128::from(p.resolution.0) / 2)
            / u128::from(p.resolution.0)
            * u128::from(p.resolution.0);
        if rebuilt_point != u128::from(p.population.0)
            || p.lower.0 != p.population.0.saturating_sub(p.resolution.0 / 2)
            || p.upper.0
                != p.population
                    .0
                    .checked_add((p.resolution.0 - 1) / 2)
                    .ok_or("Population interval overflow")?
            || self.actual.0 < p.lower.0
            || self.actual.0 > p.upper.0
        {
            return Err("Population release does not reconcile with its recorded measurement and rounding interval".into());
        }
        Ok(())
    }
}
fn population_resolution(opening: Population, capacity: Percent) -> Result<Population, String> {
    if capacity.units() > 100 {
        return Err("Statistical capacity exceeds 100 percent".into());
    }
    let resolution = u128::from(opening.0) * u128::from(100 - capacity.units()) / 10000;
    Ok(Population(
        u64::try_from(resolution)
            .map_err(|_| "Population resolution overflow")?
            .max(1),
    ))
}
/// Administrative delivery can improve releases, without altering world truth.
pub fn statistical_capacity(ceiling: Percent, administration: Percent) -> Result<Percent, String> {
    if ceiling.units() > 100 || administration.units() > 100 {
        return Err("Statistical and administrative capacities must be within 0..=100".into());
    }
    Ok(Percent::new(ceiling.units() * administration.units() / 100))
}
/// A lower capacity lengthens this publication interval. A zero capacity permits
/// a retry but does not authorize manufacturing a zero-population observation.
pub fn census_interval(base: DaySpan, capacity: Percent) -> Result<DaySpan, String> {
    if !(7..=180).contains(&base.units()) || capacity.units() > 100 {
        return Err(
            "Census interval requires 7..=180 base days and capacity within 0..=100".into(),
        );
    }
    let extra = (base.units() * (100 - capacity.units())).div_ceil(100);
    base.checked_add(DaySpan::new(extra))
        .ok_or_else(|| "Census interval overflow".into())
}
