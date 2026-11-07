use sim_core::*;

fn content_at(start: &str) -> Result<Content, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/synthetic-test.json")).unwrap();
    value["start"] = start.into();
    value["countries"].as_array_mut().unwrap().truncate(1);
    Content::from_json(&value.to_string())
}

#[test]
fn civil_dates_cover_historical_terms_and_calendar_anniversaries() {
    let historical: CivilDate = "1900-02-28".parse().unwrap();
    assert!(historical.campaign_day().is_err());
    assert!("1900-02-29".parse::<CivilDate>().is_err());
    assert!("0000-01-01".parse::<CivilDate>().is_err());
    assert!("2400-02-29".parse::<CivilDate>().is_ok());
    let leap: CivilDate = "2000-02-29".parse().unwrap();
    assert_eq!(leap.anniversary(5).unwrap().to_string(), "2005-02-28");
    assert_eq!(leap.anniversary(4).unwrap().to_string(), "2004-02-29");
    assert!(
        CivilDate::new(9999, 12, 31)
            .unwrap()
            .anniversary(1)
            .is_err()
    );
    assert_eq!(
        serde_json::to_string(&historical).unwrap(),
        "\"1900-02-28\""
    );
    assert!(serde_json::from_str::<CivilDate>("\"1900-02-29\"").is_err());
}

#[test]
fn a_midyear_bookmark_initializes_clocks_fiscal_periods_and_pending_saves() {
    let content = content_at("2001-07-15").unwrap();
    let start = parse_date("2001-07-15").unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    assert_eq!(world.day.units(), start);
    assert_eq!(world.player_country().budget.start_day, start);
    assert_eq!(
        world.player_country().budget.end_day,
        parse_date("2002-01-01").unwrap()
    );
    assert_eq!(world.player_country().next_election, Some(start + 730));
    world
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    world.advance(&content, sim_core::DaySpan::new(7)).unwrap();
    let mut restored =
        persistence::decode(&persistence::encode(&world, &content).unwrap(), &content).unwrap();
    let remaining = parse_date("2002-01-01").unwrap() - world.day.units();
    restored
        .advance(&content, sim_core::DaySpan::new(remaining))
        .unwrap();
    world
        .advance(&content, sim_core::DaySpan::new(remaining))
        .unwrap();
    world.validate(&content).unwrap();
    assert_eq!(restored.hash(), world.hash());
    assert_eq!(
        world.player_country().budget_reviews[0].period.start_day,
        start
    );
    assert_eq!(world.player_country().budget.start_day, world.day.units());
    persistence::replay(&world, &content).unwrap();
}

#[test]
fn a_later_bookmark_receives_five_calendar_years_before_its_review() {
    let content = content_at("2001-01-01").unwrap();
    assert_eq!(
        content.review_day().unwrap(),
        parse_date("2006-01-01").unwrap()
    );
    assert_eq!(
        content.review_day().unwrap() - content.start_day().unwrap(),
        1826
    );
    let mut world = World::new(&content, CountryId("AUR".into()), 9).unwrap();
    world
        .advance(
            &content,
            sim_core::DaySpan::new(content.review_day().unwrap() - world.day.units() - 1),
        )
        .unwrap();
    assert!(!world.milestone_reported);
    world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    assert!(world.milestone_reported);
    assert!(
        world
            .events
            .iter()
            .any(|event| event.day == world.day && event.text.starts_with("Five-year review:"))
    );
    world.validate(&content).unwrap();
    persistence::replay(&world, &content).unwrap();
}

#[test]
fn bookmarks_outside_the_supported_full_mandate_range_fail_before_starting() {
    assert!(content_at("2095-01-01").is_ok());
    for invalid in ["1999-12-31", "2095-01-02", "2100-01-01", "2001-02-29"] {
        assert!(content_at(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn annual_calibration_observations_must_not_come_from_a_future_bookmark_year() {
    let mut content = content_at("2001-07-15").unwrap();
    content.facts.push(HistoricalFact {
        country: CountryId("AUR".into()),
        field: "population".into(),
        observation_period: "2001".into(),
        original_value: "1000000".into(),
        original_units: "persons".into(),
        applied_value: 1_000_000,
        source: "synthetic-original".into(),
        source_version: "fixture-v1".into(),
        quality: "authored_synthetic_fixture".into(),
        transformation: "Fictional annual calibration for validation only".into(),
    });
    content.validate().unwrap();
    for invalid in ["2002", "2001-Q1", "0000", "----"] {
        content.facts[0].observation_period = invalid.into();
        assert!(content.validate().is_err(), "{invalid}");
    }
}
