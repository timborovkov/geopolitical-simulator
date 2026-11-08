//! Aggregate demographic stocks and balanced flows. These are scenario assumptions,
//! not historical national rates or individual people/family histories.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const RATE_YEAR: u64 = 365 * 10_000;
const YOUNG_YEARS: u64 = 15;
const WORKING_YEARS: u64 = 50;

/// Aggregate household units do not count residents or create dwellings.
/// ```compile_fail
/// use sim_core::{HouseholdUnits, Population};
/// let households: HouseholdUnits = Population(1000);
/// ```
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct HouseholdUnits(pub u64);
/// An annual flow count, deliberately distinct from a stock of residents.
/// ```compile_fail
/// use sim_core::{PeoplePerYear, Population};
/// let annual_migration: PeoplePerYear = Population(1000);
/// ```
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct PeoplePerYear(pub u64);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct MigrationCorridorId(pub String);

/// Coarse, exhaustive bands: 0–14, 15–64, and 65+. Working age is not employment.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgeStocks {
    pub young: Population,
    pub working_age: Population,
    pub older: Population,
}
impl Default for AgeStocks {
    fn default() -> Self {
        Self {
            young: Population(0),
            working_age: Population(0),
            older: Population(0),
        }
    }
}
impl AgeStocks {
    pub fn total(self) -> Result<Population, String> {
        Ok(Population(add(
            add(self.young.0, self.working_age.0)?,
            self.older.0,
        )?))
    }
    fn plus(self, other: Self) -> Result<Self, String> {
        Ok(Self {
            young: Population(add(self.young.0, other.young.0)?),
            working_age: Population(add(self.working_age.0, other.working_age.0)?),
            older: Population(add(self.older.0, other.older.0)?),
        })
    }
    fn minus(self, other: Self) -> Result<Self, String> {
        Ok(Self {
            young: Population(sub(self.young.0, other.young.0)?),
            working_age: Population(sub(self.working_age.0, other.working_age.0)?),
            older: Population(sub(self.older.0, other.older.0)?),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortDemographicRates {
    /// Births per 10,000 working-age residents per 365 days, all sexes combined.
    pub birth_per_working_age_bps: BasisPoints,
    pub young_mortality_bps: BasisPoints,
    pub working_mortality_bps: BasisPoints,
    pub older_mortality_bps: BasisPoints,
    /// Target household units per working-age resident, expressed as a ratio.
    pub household_target_bps: BasisPoints,
    /// Annual adjustment rates of the gap/surplus, rather than of all residents.
    pub household_formation_bps: BasisPoints,
    pub household_dissolution_bps: BasisPoints,
}
impl CohortDemographicRates {
    pub fn validate(&self) -> Result<(), String> {
        if [
            self.birth_per_working_age_bps,
            self.young_mortality_bps,
            self.working_mortality_bps,
            self.older_mortality_bps,
            self.household_target_bps,
            self.household_formation_bps,
            self.household_dissolution_bps,
        ]
        .iter()
        .any(|r| r.units() > 10_000)
        {
            return Err(
                "Demographic ratios and annual rates must be within 0..=10000 basis points".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemographicCarries {
    pub births: u64,
    pub young_deaths: u64,
    pub working_deaths: u64,
    pub older_deaths: u64,
    pub young_aging: u64,
    pub working_aging: u64,
    pub household_formation: u64,
    pub household_dissolution: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemographicFlows {
    pub births: Population,
    pub deaths: AgeStocks,
    pub into_working_age: Population,
    pub into_older_age: Population,
    pub arrivals: AgeStocks,
    pub departures: AgeStocks,
    pub households_formed: HouseholdUnits,
    pub households_dissolved: HouseholdUnits,
    pub arriving_households: HouseholdUnits,
    pub departing_households: HouseholdUnits,
}
impl Default for DemographicFlows {
    fn default() -> Self {
        Self {
            births: Population(0),
            deaths: AgeStocks::default(),
            into_working_age: Population(0),
            into_older_age: Population(0),
            arrivals: AgeStocks::default(),
            departures: AgeStocks::default(),
            households_formed: HouseholdUnits(0),
            households_dissolved: HouseholdUnits(0),
            arriving_households: HouseholdUnits(0),
            departing_households: HouseholdUnits(0),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CohortDemography {
    pub opening: AgeStocks,
    pub opening_households: HouseholdUnits,
    pub people: AgeStocks,
    pub households: HouseholdUnits,
    pub flows: DemographicFlows,
    pub carries: DemographicCarries,
}
impl CohortDemography {
    pub fn new(people: AgeStocks, households: HouseholdUnits) -> Result<Self, String> {
        let result = Self {
            opening: people,
            opening_households: households,
            people,
            households,
            flows: DemographicFlows::default(),
            carries: DemographicCarries::default(),
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<(), String> {
        let total = self.people.total()?.0;
        let opening = self.opening.total()?.0;
        if self.households.0 > total
            || self.opening_households.0 > opening
            || (self.households.0 == 0) != (total == 0)
            || (self.opening_households.0 == 0) != (opening == 0)
        {
            return Err("Demographic household counts must be nonempty exactly when residents exist and cannot exceed residents".into());
        }
        let f = &self.flows;
        for (actual, initial, added, removed) in [
            (
                self.people.young.0,
                self.opening.young.0,
                vec![f.births.0, f.arrivals.young.0],
                vec![f.deaths.young.0, f.into_working_age.0, f.departures.young.0],
            ),
            (
                self.people.working_age.0,
                self.opening.working_age.0,
                vec![f.into_working_age.0, f.arrivals.working_age.0],
                vec![
                    f.deaths.working_age.0,
                    f.into_older_age.0,
                    f.departures.working_age.0,
                ],
            ),
            (
                self.people.older.0,
                self.opening.older.0,
                vec![f.into_older_age.0, f.arrivals.older.0],
                vec![f.deaths.older.0, f.departures.older.0],
            ),
            (
                self.households.0,
                self.opening_households.0,
                vec![f.households_formed.0, f.arriving_households.0],
                vec![f.households_dissolved.0, f.departing_households.0],
            ),
        ] {
            let expected = i128::from(initial) + added.into_iter().map(i128::from).sum::<i128>()
                - removed.into_iter().map(i128::from).sum::<i128>();
            if expected != i128::from(actual) {
                return Err(
                    "Demographic opening stocks and cumulative flows do not reconcile".into(),
                );
            }
        }
        let c = &self.carries;
        if [
            c.births,
            c.young_deaths,
            c.working_deaths,
            c.older_deaths,
            c.household_formation,
            c.household_dissolution,
        ]
        .iter()
        .any(|r| *r >= RATE_YEAR)
            || c.young_aging >= YOUNG_YEARS * 365
            || c.working_aging >= WORKING_YEARS * 365
        {
            return Err("Invalid demographic fractional carry".into());
        }
        Ok(())
    }
    fn normalize_households(&mut self) -> Result<(), String> {
        let people = self.people.total()?.0;
        let next = if people == 0 {
            0
        } else {
            self.households.0.clamp(1, people)
        };
        if next >= self.households.0 {
            self.flows.households_formed.0 =
                add(self.flows.households_formed.0, next - self.households.0)?;
        } else {
            self.flows.households_dissolved.0 =
                add(self.flows.households_dissolved.0, self.households.0 - next)?;
        }
        self.households = HouseholdUnits(next);
        Ok(())
    }
    fn step(&mut self, rates: &CohortDemographicRates) -> Result<(), String> {
        rates.validate()?;
        self.validate()?;
        let p = self.people;
        let births = count(
            p.working_age.0,
            rates.birth_per_working_age_bps.units(),
            RATE_YEAR,
            &mut self.carries.births,
        )?;
        let deaths = AgeStocks {
            young: Population(
                count(
                    p.young.0,
                    rates.young_mortality_bps.units(),
                    RATE_YEAR,
                    &mut self.carries.young_deaths,
                )?
                .min(p.young.0),
            ),
            working_age: Population(
                count(
                    p.working_age.0,
                    rates.working_mortality_bps.units(),
                    RATE_YEAR,
                    &mut self.carries.working_deaths,
                )?
                .min(p.working_age.0),
            ),
            older: Population(
                count(
                    p.older.0,
                    rates.older_mortality_bps.units(),
                    RATE_YEAR,
                    &mut self.carries.older_deaths,
                )?
                .min(p.older.0),
            ),
        };
        let survivors = p.minus(deaths)?;
        // Aging is limited to survivors present at the start of this day. Newborns
        // and newly aged-in residents cannot pass a second boundary on the same day.
        let young_aging = count(
            p.young.0,
            1,
            YOUNG_YEARS * 365,
            &mut self.carries.young_aging,
        )?
        .min(survivors.young.0);
        let working_aging = count(
            p.working_age.0,
            1,
            WORKING_YEARS * 365,
            &mut self.carries.working_aging,
        )?
        .min(survivors.working_age.0);
        self.people = AgeStocks {
            young: Population(add(sub(survivors.young.0, young_aging)?, births)?),
            working_age: Population(add(
                sub(survivors.working_age.0, working_aging)?,
                young_aging,
            )?),
            older: Population(add(survivors.older.0, working_aging)?),
        };
        self.flows.births.0 = add(self.flows.births.0, births)?;
        self.flows.deaths = self.flows.deaths.plus(deaths)?;
        self.flows.into_working_age.0 = add(self.flows.into_working_age.0, young_aging)?;
        self.flows.into_older_age.0 = add(self.flows.into_older_age.0, working_aging)?;
        self.normalize_households()?;
        let population = self.people.total()?.0;
        let desired = u64::try_from(
            u128::from(self.people.working_age.0) * u128::from(rates.household_target_bps.units())
                / 10_000,
        )
        .map_err(|_| "Demographic household target overflow")?
        .min(population)
        .max(u64::from(population > 0));
        if desired > self.households.0 {
            let formed = count(
                desired - self.households.0,
                rates.household_formation_bps.units(),
                RATE_YEAR,
                &mut self.carries.household_formation,
            )?
            .min(desired - self.households.0);
            self.households.0 = add(self.households.0, formed)?;
            self.flows.households_formed.0 = add(self.flows.households_formed.0, formed)?;
        } else {
            let dissolved = count(
                self.households.0 - desired,
                rates.household_dissolution_bps.units(),
                RATE_YEAR,
                &mut self.carries.household_dissolution,
            )?
            .min(self.households.0 - desired);
            self.households.0 = sub(self.households.0, dissolved)?;
            self.flows.households_dissolved.0 = add(self.flows.households_dissolved.0, dissolved)?;
        }
        self.validate()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DemographicLocation {
    pub country: CountryId,
    pub group: HouseholdGroupId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationCorridor {
    pub id: MigrationCorridorId,
    pub origin: DemographicLocation,
    pub destination: DemographicLocation,
    pub annual_people: PeoplePerYear,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationTransfer {
    pub corridor: MigrationCorridorId,
    pub origin: DemographicLocation,
    pub destination: DemographicLocation,
    pub people: AgeStocks,
    pub households: HouseholdUnits,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DemographicBalances {
    pub day: CampaignDay,
    pub countries: BTreeMap<CountryId, BTreeMap<HouseholdGroupId, CohortDemography>>,
    pub migration_carries: BTreeMap<MigrationCorridorId, u64>,
}
impl DemographicBalances {
    pub fn validate(&self) -> Result<(), String> {
        if self.day.units() > MAX_DAY || self.countries.is_empty() || self.countries.len() > 512 {
            return Err(
                "Demographic balances need bounded country/cohort stocks and a valid campaign day"
                    .into(),
            );
        }
        if self.countries.iter().any(|(country, groups)| {
            country.0.is_empty()
                || country.0.len() > 128
                || groups.len() > 64
                || groups.keys().any(|id| id.0.is_empty() || id.0.len() > 128)
        }) {
            return Err("Invalid demographic country/cohort identity".into());
        }
        let mut arriving = AgeStocks::default();
        let mut departing = AgeStocks::default();
        let mut households_in = 0;
        let mut households_out = 0;
        for groups in self.countries.values() {
            if groups.is_empty() {
                return Err("Demographic country needs cohort stocks".into());
            }
            for group in groups.values() {
                group.validate()?;
                arriving = arriving.plus(group.flows.arrivals)?;
                departing = departing.plus(group.flows.departures)?;
                households_in = add(households_in, group.flows.arriving_households.0)?;
                households_out = add(households_out, group.flows.departing_households.0)?;
            }
        }
        if arriving != departing
            || households_in != households_out
            || self.migration_carries.values().any(|r| *r >= 365)
        {
            return Err("Demographic migration arrivals and departures do not reconcile".into());
        }
        Ok(())
    }
    /// One complete day is transactional. Migration plans use the same pre-move
    /// stocks; incoming residents cannot emigrate again during this step.
    pub fn advance_day(
        &mut self,
        profiles: &BTreeMap<CountryId, BTreeMap<HouseholdGroupId, CohortDemographicRates>>,
        corridors: &[MigrationCorridor],
        day: CampaignDay,
    ) -> Result<Vec<MigrationTransfer>, String> {
        if self.day.checked_add(DaySpan::new(1)) != Some(day) || day.units() > MAX_DAY {
            return Err("Demography requires the next campaign day".into());
        }
        self.validate()?;
        if profiles.keys().ne(self.countries.keys()) {
            return Err("Demographic country profiles do not match stocks".into());
        }
        let mut next = self.clone();
        for (country, groups) in &mut next.countries {
            let rates = &profiles[country];
            if rates.keys().ne(groups.keys()) {
                return Err("Demographic cohort profiles do not match stocks".into());
            }
            for (id, group) in groups {
                group.step(&rates[id])?;
            }
        }
        let migrations = next.migrate(corridors)?;
        next.day = day;
        next.validate()?;
        *self = next;
        Ok(migrations)
    }
    fn migrate(
        &mut self,
        corridors: &[MigrationCorridor],
    ) -> Result<Vec<MigrationTransfer>, String> {
        if corridors.len() > 1024 {
            return Err("Too many demographic migration corridors".into());
        }
        let mut ordered = BTreeMap::new();
        for corridor in corridors {
            if corridor.id.0.is_empty()
                || corridor.id.0.len() > 128
                || !corridor.id.0.bytes().all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b':' | b'-')
                })
                || corridor.origin.country == corridor.destination.country
                || ordered.insert(corridor.id.clone(), corridor).is_some()
            {
                return Err("Invalid or duplicate migration corridor".into());
            }
            self.group(&corridor.origin)?;
            self.group(&corridor.destination)?;
        }
        if self
            .migration_carries
            .keys()
            .any(|id| !ordered.contains_key(id))
        {
            return Err("Unknown migration carry".into());
        }
        let mut remaining = self.countries.clone();
        let mut moves = Vec::new();
        for (id, corridor) in ordered {
            let wanted = count(
                corridor.annual_people.0,
                1,
                365,
                self.migration_carries.entry(id.clone()).or_default(),
            )?;
            let origin = remaining
                .get_mut(&corridor.origin.country)
                .and_then(|g| g.get_mut(&corridor.origin.group))
                .ok_or("Unknown migration origin")?;
            let people_before = origin.people.total()?.0;
            let actual = wanted.min(people_before);
            let people = split_people(actual, origin.people)?;
            let households = HouseholdUnits(if people_before == 0 {
                0
            } else {
                u64::try_from(
                    u128::from(origin.households.0) * u128::from(actual)
                        / u128::from(people_before),
                )
                .map_err(|_| "Migrating household overflow")?
            });
            origin.people = origin.people.minus(people)?;
            origin.households.0 = sub(origin.households.0, households.0)?;
            moves.push(MigrationTransfer {
                corridor: id,
                origin: corridor.origin.clone(),
                destination: corridor.destination.clone(),
                people,
                households,
            });
        }
        for flow in &moves {
            let origin = self.group_mut(&flow.origin)?;
            origin.people = origin.people.minus(flow.people)?;
            origin.households.0 = sub(origin.households.0, flow.households.0)?;
            origin.flows.departures = origin.flows.departures.plus(flow.people)?;
            origin.flows.departing_households.0 =
                add(origin.flows.departing_households.0, flow.households.0)?;
        }
        for flow in &moves {
            let destination = self.group_mut(&flow.destination)?;
            destination.people = destination.people.plus(flow.people)?;
            destination.households.0 = add(destination.households.0, flow.households.0)?;
            destination.flows.arrivals = destination.flows.arrivals.plus(flow.people)?;
            destination.flows.arriving_households.0 =
                add(destination.flows.arriving_households.0, flow.households.0)?;
        }
        for groups in self.countries.values_mut() {
            for group in groups.values_mut() {
                group.normalize_households()?;
            }
        }
        Ok(moves)
    }
    fn group(&self, key: &DemographicLocation) -> Result<&CohortDemography, String> {
        self.countries
            .get(&key.country)
            .and_then(|g| g.get(&key.group))
            .ok_or_else(|| "Unknown demographic country/cohort".into())
    }
    fn group_mut(&mut self, key: &DemographicLocation) -> Result<&mut CohortDemography, String> {
        self.countries
            .get_mut(&key.country)
            .and_then(|g| g.get_mut(&key.group))
            .ok_or_else(|| "Unknown demographic country/cohort".into())
    }
}
fn add(a: u64, b: u64) -> Result<u64, String> {
    a.checked_add(b)
        .ok_or_else(|| "Demographic count overflow".into())
}
fn sub(a: u64, b: u64) -> Result<u64, String> {
    a.checked_sub(b)
        .ok_or_else(|| "Demographic stock would become negative".into())
}
fn count(base: u64, rate: u32, denominator: u64, remainder: &mut u64) -> Result<u64, String> {
    if *remainder >= denominator || denominator == 0 {
        return Err("Invalid demographic carry denominator".into());
    }
    let numerator = u128::from(base)
        .checked_mul(u128::from(rate))
        .and_then(|n| n.checked_add(u128::from(*remainder)))
        .ok_or("Demographic flow overflow")?;
    let result = u64::try_from(numerator / u128::from(denominator))
        .map_err(|_| "Demographic flow count overflow")?;
    *remainder = (numerator % u128::from(denominator)) as u64;
    Ok(result)
}
fn split_people(total: u64, available: AgeStocks) -> Result<AgeStocks, String> {
    let sum = available.total()?.0;
    if total > sum {
        return Err("Migration exceeds origin residents".into());
    }
    if sum == 0 {
        return Ok(AgeStocks::default());
    }
    let weights = [
        available.young.0,
        available.working_age.0,
        available.older.0,
    ];
    let mut counts = weights.map(|w| (u128::from(total) * u128::from(w) / u128::from(sum)) as u64);
    let mut order = [0, 1, 2];
    order.sort_by_key(|i| {
        (
            std::cmp::Reverse(u128::from(total) * u128::from(weights[*i]) % u128::from(sum)),
            *i,
        )
    });
    let used = add(add(counts[0], counts[1])?, counts[2])?;
    for index in order.into_iter().take((total - used) as usize) {
        counts[index] += 1;
    }
    Ok(AgeStocks {
        young: Population(counts[0]),
        working_age: Population(counts[1]),
        older: Population(counts[2]),
    })
}
