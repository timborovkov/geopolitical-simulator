use sim_core::*;
use std::collections::BTreeMap;

fn fixture() -> (Content, DemographicDefinition) {
    let content = Content::from_json(include_str!(
        "../../../data/synthetic-governing-institutions-test.json"
    ))
    .unwrap();
    let definition = DemographicDefinition {
        source: content.sources[0].id.clone(),
        source_version: "authored-test-1".into(),
        description: "Explicit fictional test assumptions; no historical vital statistics.".into(),
        profiles: content
            .countries
            .iter()
            .map(|country| {
                (
                    country.id.clone(),
                    country
                        .households
                        .iter()
                        .map(|group| {
                            (
                                group.id.clone(),
                                CohortDemographicDefinition {
                                    rates: CohortDemographicRates {
                                        birth_per_working_age_bps: BasisPoints::new(200),
                                        young_mortality_bps: BasisPoints::new(20),
                                        working_mortality_bps: BasisPoints::new(40),
                                        older_mortality_bps: BasisPoints::new(400),
                                        household_target_bps: BasisPoints::new(6000),
                                        household_formation_bps: BasisPoints::new(500),
                                        household_dissolution_bps: BasisPoints::new(500),
                                    },
                                    young_share_of_dependants_bps: BasisPoints::new(6000),
                                    initial_households_per_person_bps: BasisPoints::new(3000),
                                },
                            )
                        })
                        .collect(),
                )
            })
            .collect(),
        corridors: vec![],
        statistical_capacities: content
            .countries
            .iter()
            .map(|c| (c.id.clone(), Percent::new(80)))
            .collect(),
        census_days: DaySpan::new(30),
        maximum_capacity: Percent::new(400),
    };
    (content, definition)
}

#[test]
fn attributed_initial_stocks_match_existing_residents_and_working_age_without_changing_cash() {
    let (content, definition) = fixture();
    let original = content.clone();
    let mut balances = definition.initial_balances(&content).unwrap();
    for country in &content.countries {
        let world = World::new(&content, country.id.clone(), 17).unwrap();
        let current = world.player_country();
        let groups = &balances.countries[&country.id];
        assert_eq!(
            groups
                .values()
                .map(|g| g.people.total().unwrap().0)
                .sum::<u64>(),
            country.population.0
        );
        for household in &current.households {
            let cohort = &groups[&household.id];
            assert_eq!(cohort.people.total().unwrap(), household.population);
            assert_eq!(cohort.people.working_age, household.working_age);
            assert!(cohort.households.0 <= household.population.0);
        }
    }
    definition
        .validate_balances(&content, &balances, content.start_day().unwrap())
        .unwrap();
    let next = content
        .start_day()
        .unwrap()
        .checked_add(DaySpan::new(1))
        .unwrap();
    balances
        .advance_day(&definition.rates(), &definition.corridors, next)
        .unwrap();
    definition
        .validate_balances(&content, &balances, next)
        .unwrap();
    assert!(
        definition
            .validate_balances(&content, &balances, content.start_day().unwrap())
            .is_err()
    );
    assert_eq!(content, original);
    let restored: DemographicDefinition =
        serde_json::from_slice(&serde_json::to_vec(&definition).unwrap()).unwrap();
    assert_eq!(restored, definition);
}

#[test]
fn missing_profiles_provenance_unknown_endpoints_and_forged_opening_stocks_are_rejected() {
    let (content, original) = fixture();
    for variant in 0..8 {
        let mut definition = original.clone();
        let country = content.countries[0].id.clone();
        let group = content.countries[0].households[0].id.clone();
        match variant {
            0 => definition.source = "unattributed".into(),
            1 => definition.profiles.remove(&country).map(|_| ()).unwrap(),
            2 => definition
                .profiles
                .insert(country.clone(), BTreeMap::new())
                .map(|_| ())
                .unwrap(),
            3 => definition
                .statistical_capacities
                .insert(country.clone(), Percent::new(101))
                .map(|_| ())
                .unwrap(),
            4 => definition.census_days = DaySpan::new(0),
            5 => definition.maximum_capacity = Percent::new(199),
            6 => {
                definition
                    .profiles
                    .get_mut(&country)
                    .unwrap()
                    .get_mut(&group)
                    .unwrap()
                    .young_share_of_dependants_bps = BasisPoints::new(10001)
            }
            7 => definition.corridors.push(MigrationCorridor {
                id: MigrationCorridorId("unknown-endpoint".into()),
                origin: DemographicLocation {
                    country,
                    group: group.clone(),
                },
                destination: DemographicLocation {
                    country: CountryId("UNKNOWN".into()),
                    group,
                },
                annual_people: PeoplePerYear(100),
            }),
            _ => unreachable!(),
        }
        assert!(definition.validate(&content).is_err(), "variant {variant}");
        assert!(
            definition.initial_balances(&content).is_err(),
            "variant {variant}"
        );
    }
    let mut balances = original.initial_balances(&content).unwrap();
    let cohort = balances
        .countries
        .values_mut()
        .next()
        .unwrap()
        .values_mut()
        .next()
        .unwrap();
    // Internally balanced but inconsistent with the attributed opening reference.
    cohort.opening.young.0 += 1;
    cohort.people.young.0 += 1;
    balances.validate().unwrap();
    assert!(
        original
            .validate_balances(&content, &balances, content.start_day().unwrap())
            .is_err()
    );
}

#[test]
fn population_publications_are_dated_bounded_and_improve_only_after_actual_capacity_changes() {
    let (content, mut definition) = fixture();
    definition.census_days = DaySpan::new(7);
    let lower: BTreeMap<_, _> = content
        .countries
        .iter()
        .map(|c| (c.id.clone(), Percent::new(50)))
        .collect();
    let higher: BTreeMap<_, _> = content
        .countries
        .iter()
        .map(|c| (c.id.clone(), Percent::new(100)))
        .collect();
    let country = &content.countries[0].id;
    let mut state = DemographyState::new(&content, &definition, &lower).unwrap();
    assert_eq!(
        state.archives[country].measurements[0]
            .published
            .statistical_capacity,
        Percent::new(40)
    );
    let opening = state.published(country).unwrap();
    for day in 1..12 {
        state
            .advance_day(&content, &definition, &higher, CampaignDay::new(day))
            .unwrap();
    }
    assert_eq!(state.published(country).unwrap(), opening);
    state
        .advance_day(&content, &definition, &higher, CampaignDay::new(12))
        .unwrap();
    let archive = &state.archives[country];
    assert_eq!(
        archive
            .measurements
            .last()
            .unwrap()
            .published
            .statistical_capacity,
        Percent::new(80)
    );
    assert!(archive.measurements.last().unwrap().published.resolution.0 < opening[0].resolution.0);
    assert_eq!(archive.next_attempt, Some(CampaignDay::new(21)));
    for day in 13..=1200 {
        state
            .advance_day(&content, &definition, &higher, CampaignDay::new(day))
            .unwrap();
    }
    let archive = &state.archives[country];
    assert_eq!(archive.measurements.len(), 120);
    assert!(archive.measurements[0].published.id.0 > 1);
    let public = serde_json::to_string(&state.published(country).unwrap()).unwrap();
    assert!(
        !public.contains("actual") && !public.contains("people") && !public.contains("working_age")
    );
    let restored: DemographyState =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    restored
        .validate(&content, &definition, CampaignDay::new(1200))
        .unwrap();
    assert_eq!(state, restored);
    let mut damaged = restored.clone();
    let archive = damaged.archives.values_mut().next().unwrap();
    archive.measurements[0].published.id = CensusReleaseId(u64::MAX);
    archive.measurements[1].published.id = CensusReleaseId(0);
    assert!(
        damaged
            .validate(&content, &definition, CampaignDay::new(1200))
            .is_err()
    );
    let mut resumed = restored;
    for day in 1201..=1230 {
        resumed
            .advance_day(&content, &definition, &higher, CampaignDay::new(day))
            .unwrap();
        state
            .advance_day(&content, &definition, &higher, CampaignDay::new(day))
            .unwrap();
    }
    assert_eq!(state, resumed);
    // A release-ID failure after computing births must roll back the entire day.
    for archive in state.archives.values_mut() {
        assert_eq!(archive.measurements.len(), 120);
        for (index, measurement) in archive.measurements.iter_mut().enumerate() {
            measurement.published.id = CensusReleaseId(u64::MAX - 120 + index as u64);
        }
        archive.next_release = CensusReleaseId(u64::MAX);
    }
    state
        .validate(&content, &definition, state.balances.day)
        .unwrap();
    let due = state.archives[country].next_attempt.unwrap();
    while state.balances.day.units() + 1 < due.units() {
        let next = state.balances.day.checked_add(DaySpan::new(1)).unwrap();
        state
            .advance_day(&content, &definition, &higher, next)
            .unwrap();
    }
    let before = state.clone();
    assert!(
        state
            .advance_day(&content, &definition, &higher, due)
            .unwrap_err()
            .contains("exhausted")
    );
    assert_eq!(state, before);
}

#[test]
fn zero_capacity_never_invents_an_observation_and_invalid_day_or_archive_rolls_back() {
    let (content, definition) = fixture();
    let capacities: BTreeMap<_, _> = content
        .countries
        .iter()
        .map(|c| (c.id.clone(), Percent::new(0)))
        .collect();
    let mut state = DemographyState::new(&content, &definition, &capacities).unwrap();
    for day in 1..=61 {
        state
            .advance_day(&content, &definition, &capacities, CampaignDay::new(day))
            .unwrap();
    }
    for id in state.archives.keys() {
        assert!(state.published(id).unwrap().is_empty());
        assert_eq!(state.archives[id].next_release, CensusReleaseId(1));
        assert_eq!(state.archives[id].last_attempt, CampaignDay::new(60));
    }
    let before = state.clone();
    assert!(
        state
            .advance_day(&content, &definition, &capacities, CampaignDay::new(63))
            .is_err()
    );
    assert_eq!(state, before);
    assert!(
        state
            .advance_day(
                &content,
                &definition,
                &BTreeMap::new(),
                CampaignDay::new(62)
            )
            .is_err()
    );
    assert_eq!(state, before);
    state.archives.values_mut().next().unwrap().next_attempt = Some(CampaignDay::new(80));
    assert!(
        state
            .validate(&content, &definition, CampaignDay::new(61))
            .is_err()
    );
    let malformed = state.clone();
    assert!(
        state
            .advance_day(&content, &definition, &capacities, CampaignDay::new(62))
            .is_err()
    );
    assert_eq!(state, malformed);
}
