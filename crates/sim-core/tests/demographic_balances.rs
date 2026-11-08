use sim_core::*;
use std::collections::BTreeMap;

fn people(young: u64, working: u64, older: u64) -> AgeStocks {
    AgeStocks {
        young: Population(young),
        working_age: Population(working),
        older: Population(older),
    }
}
fn rates() -> CohortDemographicRates {
    CohortDemographicRates {
        birth_per_working_age_bps: BasisPoints::new(300),
        young_mortality_bps: BasisPoints::new(50),
        working_mortality_bps: BasisPoints::new(60),
        older_mortality_bps: BasisPoints::new(450),
        household_target_bps: BasisPoints::new(6500),
        household_formation_bps: BasisPoints::new(800),
        household_dissolution_bps: BasisPoints::new(600),
    }
}
fn location(country: &str) -> DemographicLocation {
    DemographicLocation {
        country: CountryId(country.into()),
        group: "working".into(),
    }
}
fn corridor(id: &str, from: &str, to: &str, annual: u64) -> MigrationCorridor {
    MigrationCorridor {
        id: MigrationCorridorId(id.into()),
        origin: location(from),
        destination: location(to),
        annual_people: PeoplePerYear(annual),
    }
}
fn setup() -> (
    DemographicBalances,
    BTreeMap<CountryId, BTreeMap<HouseholdGroupId, CohortDemographicRates>>,
) {
    let mut countries = BTreeMap::new();
    let mut profiles = BTreeMap::new();
    for country in ["AUR", "BOR", "COR"] {
        let key = CountryId(country.into());
        countries.insert(
            key.clone(),
            BTreeMap::from([(
                "working".into(),
                CohortDemography::new(people(2000, 6000, 2000), HouseholdUnits(3000)).unwrap(),
            )]),
        );
        profiles.insert(key, BTreeMap::from([("working".into(), rates())]));
    }
    (
        DemographicBalances {
            day: CampaignDay::new(0),
            countries,
            migration_carries: BTreeMap::new(),
        },
        profiles,
    )
}
#[test]
fn one_century_balances_births_deaths_age_transitions_families_and_bilateral_migration_exactly() {
    let (mut world, profiles) = setup();
    let corridors = [
        corridor("aur-bor", "AUR", "BOR", 120),
        corridor("bor-aur", "BOR", "AUR", 40),
    ];
    for day in 1..=MAX_DAY {
        world
            .advance_day(&profiles, &corridors, CampaignDay::new(day))
            .unwrap();
    }
    let mut population = 0_u128;
    let mut expected = 0_i128;
    let mut families_formed = 0;
    let mut aged = 0;
    for groups in world.countries.values() {
        for cohort in groups.values() {
            population += u128::from(cohort.people.total().unwrap().0);
            expected += i128::from(cohort.opening.total().unwrap().0)
                + i128::from(cohort.flows.births.0)
                - i128::from(cohort.flows.deaths.total().unwrap().0);
            families_formed += cohort.flows.households_formed.0;
            aged += cohort.flows.into_older_age.0;
        }
    }
    assert_eq!(population as i128, expected);
    assert!(families_formed > 0 && aged > 0);
    let bytes = serde_json::to_vec(&world).unwrap();
    let restored: DemographicBalances = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, world);
}
#[test]
fn migration_uses_pre_move_stocks_and_reference_order_never_changes_balanced_results() {
    let (mut original, mut profiles) = setup();
    for (country, groups) in &mut original.countries {
        groups.insert(
            "working".into(),
            CohortDemography::new(
                if country.0 == "AUR" {
                    people(100, 0, 0)
                } else {
                    people(0, 0, 0)
                },
                HouseholdUnits(if country.0 == "AUR" { 10 } else { 0 }),
            )
            .unwrap(),
        );
        let r = profiles
            .get_mut(country)
            .unwrap()
            .get_mut(&"working".into())
            .unwrap();
        r.birth_per_working_age_bps = BasisPoints::new(0);
        r.young_mortality_bps = BasisPoints::new(0);
        r.working_mortality_bps = BasisPoints::new(0);
        r.older_mortality_bps = BasisPoints::new(0);
        r.household_formation_bps = BasisPoints::new(0);
        r.household_dissolution_bps = BasisPoints::new(0);
    }
    let mut a = original.clone();
    let mut b = original;
    let mut corridors = vec![
        corridor("a-route", "AUR", "BOR", 36500),
        corridor("b-route", "BOR", "COR", 36500),
    ];
    a.advance_day(&profiles, &corridors, CampaignDay::new(1))
        .unwrap();
    corridors.reverse();
    b.advance_day(&profiles, &corridors, CampaignDay::new(1))
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(
        a.countries[&CountryId("BOR".into())][&"working".into()]
            .people
            .total()
            .unwrap(),
        Population(100)
    );
    assert_eq!(
        a.countries[&CountryId("COR".into())][&"working".into()]
            .people
            .total()
            .unwrap(),
        Population(0)
    );
    a.advance_day(&profiles, &corridors, CampaignDay::new(2))
        .unwrap();
    assert_eq!(
        a.countries[&CountryId("COR".into())][&"working".into()]
            .people
            .total()
            .unwrap(),
        Population(100)
    );
}
#[test]
fn damaged_carries_unknown_corridors_wrong_dates_and_overflow_roll_back_the_whole_demographic_day()
{
    let (original, profiles) = setup();
    for day in [0, 2] {
        let mut world = original.clone();
        assert!(
            world
                .advance_day(&profiles, &[], CampaignDay::new(day))
                .is_err()
        );
        assert_eq!(world, original);
    }
    let mut world = original.clone();
    let bad = corridor("missing", "AUR", "UNKNOWN", 1);
    assert!(
        world
            .advance_day(&profiles, &[bad], CampaignDay::new(1))
            .is_err()
    );
    assert_eq!(world, original);
    let mut damaged = original.clone();
    damaged
        .countries
        .get_mut(&CountryId("AUR".into()))
        .unwrap()
        .get_mut(&"working".into())
        .unwrap()
        .carries
        .births = 3650000;
    let before = damaged.clone();
    assert!(
        damaged
            .advance_day(&profiles, &[], CampaignDay::new(1))
            .is_err()
    );
    assert_eq!(damaged, before);
    let mut overflow = original;
    overflow
        .countries
        .get_mut(&CountryId("AUR".into()))
        .unwrap()
        .insert(
            "working".into(),
            CohortDemography::new(people(0, u64::MAX, 0), HouseholdUnits(1)).unwrap(),
        );
    let before = overflow.clone();
    assert!(
        overflow
            .advance_day(&profiles, &[], CampaignDay::new(1))
            .is_err()
    );
    assert_eq!(overflow, before);
}

#[test]
fn zero_stocks_and_invalid_profiles_or_corridor_identities_have_explicit_balanced_behavior() {
    let (mut original, profiles) = setup();
    for groups in original.countries.values_mut() {
        groups.insert(
            "working".into(),
            CohortDemography::new(people(0, 0, 0), HouseholdUnits(0)).unwrap(),
        );
    }
    let flows = original
        .advance_day(
            &profiles,
            &[corridor("empty-flow", "AUR", "BOR", u64::MAX)],
            CampaignDay::new(1),
        )
        .unwrap();
    assert_eq!(flows[0].people.total().unwrap(), Population(0));
    assert_eq!(flows[0].households, HouseholdUnits(0));
    original.validate().unwrap();
    let (world, mut invalid) = setup();
    invalid
        .get_mut(&CountryId("AUR".into()))
        .unwrap()
        .get_mut(&"working".into())
        .unwrap()
        .birth_per_working_age_bps = BasisPoints::new(10001);
    let mut next = world.clone();
    assert!(
        next.advance_day(&invalid, &[], CampaignDay::new(1))
            .is_err()
    );
    assert_eq!(next, world);
    let mut duplicate = vec![
        corridor("same", "AUR", "BOR", 365),
        corridor("same", "BOR", "COR", 365),
    ];
    assert!(
        next.advance_day(&profiles, &duplicate, CampaignDay::new(1))
            .is_err()
    );
    assert_eq!(next, world);
    duplicate[1].id = MigrationCorridorId("different".into());
    duplicate[1].destination = location("BOR");
    assert!(
        next.advance_day(&profiles, &duplicate, CampaignDay::new(1))
            .is_err()
    );
    assert_eq!(next, world);
    let mut malformed = world;
    malformed
        .countries
        .get_mut(&CountryId("AUR".into()))
        .unwrap()
        .get_mut(&"working".into())
        .unwrap()
        .flows
        .births = Population(1);
    assert!(malformed.validate().is_err());
}
