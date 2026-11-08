use sim_core::*;

#[test]
fn dated_public_precision_can_differ_from_private_stock_without_exposing_that_stock() {
    let measured = PopulationMeasurement::new(
        CensusReleaseId(1),
        CampaignDay::new(42),
        Population(1_000_123),
        Population(1_000_000),
        Percent::new(90),
    )
    .unwrap();
    assert_eq!(measured.published.resolution, Population(1000));
    assert_eq!(measured.published.population, Population(1_000_000));
    assert_ne!(measured.published.population, measured.actual);
    assert_eq!(
        (measured.published.lower, measured.published.upper),
        (Population(999_500), Population(1_000_499))
    );
    let public = serde_json::to_value(&measured.published).unwrap();
    assert!(public.get("actual").is_none());
    assert_eq!(public["day"], 42);
    let restored: PopulationMeasurement =
        serde_json::from_slice(&serde_json::to_vec(&measured).unwrap()).unwrap();
    restored.validate(Population(1_000_000)).unwrap();
    assert_eq!(restored, measured);
}
#[test]
fn integer_rounding_intervals_contain_every_recorded_count_including_zero_and_perfect_capacity() {
    for capacity in [1, 10, 50, 99, 100] {
        for reference in [0, 1, 10, 10_000, 1_000_000] {
            for actual in 0..1000 {
                let result = PopulationMeasurement::new(
                    CensusReleaseId(7),
                    CampaignDay::new(0),
                    Population(actual),
                    Population(reference),
                    Percent::new(capacity),
                )
                .unwrap();
                assert!(result.published.lower.0 <= actual && actual <= result.published.upper.0);
                if capacity == 100 {
                    assert_eq!(result.published.population, Population(actual));
                    assert_eq!(result.published.lower, result.published.upper);
                }
            }
        }
    }
    let small = PopulationMeasurement::new(
        CensusReleaseId(1),
        CampaignDay::new(0),
        Population(1),
        Population(1_000_000),
        Percent::new(1),
    )
    .unwrap();
    assert_eq!(small.published.population, Population(0));
    assert!(small.published.upper.0 > 0);
}
#[test]
fn administrative_capacity_improves_precision_and_frequency_but_zero_capacity_does_not_invent_a_release()
 {
    let lower = statistical_capacity(Percent::new(90), Percent::new(50)).unwrap();
    let higher = statistical_capacity(Percent::new(90), Percent::new(100)).unwrap();
    assert_eq!(lower, Percent::new(45));
    assert_eq!(higher, Percent::new(90));
    assert_eq!(
        census_interval(DaySpan::new(30), lower).unwrap(),
        DaySpan::new(47)
    );
    assert_eq!(
        census_interval(DaySpan::new(30), higher).unwrap(),
        DaySpan::new(33)
    );
    assert!(
        PopulationMeasurement::new(
            CensusReleaseId(1),
            CampaignDay::new(0),
            Population(1_000_123),
            Population(1_000_000),
            Percent::new(0)
        )
        .is_err()
    );
    assert!(statistical_capacity(Percent::new(101), Percent::new(50)).is_err());
    assert!(census_interval(DaySpan::new(0), higher).is_err());
}
#[test]
fn malformed_measurement_metadata_dates_and_count_overflow_are_rejected() {
    let base = PopulationMeasurement::new(
        CensusReleaseId(1),
        CampaignDay::new(15),
        Population(50_123),
        Population(100_000),
        Percent::new(90),
    )
    .unwrap();
    for variant in 0..4 {
        let mut malformed = base.clone();
        match variant {
            0 => malformed.published.population.0 += 1,
            1 => malformed.published.upper.0 += 1,
            2 => malformed.published.id = CensusReleaseId(0),
            _ => malformed.published.day = CampaignDay::new(MAX_DAY + 1),
        }
        assert!(malformed.validate(Population(100_000)).is_err());
    }
    assert!(
        PopulationMeasurement::new(
            CensusReleaseId(1),
            CampaignDay::new(0),
            Population(u64::MAX),
            Population(20_000),
            Percent::new(50)
        )
        .is_err()
    );
}
