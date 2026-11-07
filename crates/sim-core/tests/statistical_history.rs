use sim_core::*;
use std::collections::BTreeMap;
fn pack() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-statistical-history-test.json"
    ))
    .unwrap()
}
fn point(day: u32, value: i64) -> Observation {
    Observation {
        day: sim_core::CampaignDay::new(day),
        treasury: Money(value),
        debt: Money(value + 10),
        approval: 51,
        needs: 99,
        output: Money(value + 20),
    }
}
#[test]
fn archive_rounding_dates_and_bounds_are_explicit_and_refusal_is_atomic() {
    let mut archive = StatisticalArchive::default();
    for sample in [
        point(30, 100),
        point(60, 101),
        point(90, 103),
        point(366, 200),
    ] {
        archive.retire(&sample).unwrap();
    }
    assert_eq!(archive.years.len(), 2);
    let first = &archive.years[0];
    assert_eq!(
        (first.year, first.first_day, first.last_day, first.samples),
        (2000, CampaignDay::new(30), CampaignDay::new(90), 3)
    );
    assert_eq!(first.mean().unwrap().treasury, Money(101));
    assert_eq!(archive.years[1].year, 2001);
    let bytes = serde_json::to_vec(&archive).unwrap();
    assert_eq!(
        serde_json::from_slice::<StatisticalArchive>(&bytes).unwrap(),
        archive
    );
    for invalid in [
        point(90, 100),
        point(MAX_DAY + 1, 100),
        point(400, -1),
        Observation {
            treasury: Money(i64::MAX),
            ..point(400, 100)
        },
    ] {
        let before = archive.clone();
        assert!(archive.retire(&invalid).is_err());
        assert_eq!(archive, before);
    }
    let mut invalid = archive.years[0].clone();
    invalid.samples = 0;
    assert!(invalid.mean().is_none());
    invalid.samples = 3;
    invalid.approval_sum = i64::MAX;
    assert!(invalid.mean().is_none());
    let mut invalid_date = archive.years[0].clone();
    invalid_date.first_day = sim_core::CampaignDay::new(0);
    invalid_date.last_day = sim_core::CampaignDay::new(u32::MAX);
    assert!(invalid_date.mean().is_none());
    let released = serde_json::to_string(&archive.published()).unwrap();
    assert!(!released.contains("treasury") && !released.contains("_sum"));
}

#[test]
fn twelve_year_retention_matches_the_original_campaign_and_independent_retired_samples() {
    let content = pack();
    let legacy = Content::from_json(include_str!(
        "../../../data/synthetic-stable-elections-test.json"
    ))
    .unwrap();
    let id = CountryId("AUR".into());
    let mut world = World::new(&content, id.clone(), 17).unwrap();
    let mut original = World::new(&legacy, id, 17).unwrap();
    let mut samples = BTreeMap::new();
    for _ in 0..142 {
        world.advance(&content, DaySpan::new(30)).unwrap();
        original.advance(&legacy, DaySpan::new(30)).unwrap();
        for sample in &original.player_country().history {
            samples.insert(sample.day, sample.clone());
        }
    }
    world.validate(&content).unwrap();
    let recent = &world.player_country().history;
    assert_eq!(recent.len(), 120);
    assert_eq!(*recent, original.player_country().history);
    let retired: Vec<_> = samples
        .values()
        .filter(|sample| sample.day < recent[0].day)
        .collect();
    assert!(!retired.is_empty());
    let archive = world.player_country().statistical_archive.as_ref().unwrap();
    assert_eq!(
        archive
            .years
            .iter()
            .map(|s| s.samples as usize)
            .sum::<usize>(),
        retired.len()
    );
    for bucket in &archive.years {
        let rows: Vec<_> = retired
            .iter()
            .filter(|p| date(p.day).parse::<CivilDate>().unwrap().year() == bucket.year)
            .collect();
        assert_eq!(bucket.samples as usize, rows.len());
        assert_eq!(bucket.first_day, rows.first().unwrap().day);
        assert_eq!(bucket.last_day, rows.last().unwrap().day);
        let n = rows.len() as i64;
        let mean = bucket.mean().unwrap();
        assert_eq!(
            mean.treasury,
            Money(rows.iter().map(|r| r.treasury.0).sum::<i64>() / n)
        );
        assert_eq!(
            mean.debt,
            Money(rows.iter().map(|r| r.debt.0).sum::<i64>() / n)
        );
        assert_eq!(
            mean.output,
            Money(rows.iter().map(|r| r.output.0).sum::<i64>() / n)
        );
        assert_eq!(
            mean.approval,
            i32::try_from(rows.iter().map(|r| i64::from(r.approval)).sum::<i64>() / n).unwrap()
        );
    }
    let mut normalized = world.clone();
    normalized.rules = original.rules;
    normalized.content_hash = original.content_hash;
    for country in normalized.countries.values_mut() {
        country.statistical_archive = None;
    }
    assert_eq!(
        normalized, original,
        "Archiving may not alter economics, government, commands or events"
    );
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut restored = persistence::decode(&bytes, &content).unwrap();
    assert_eq!(persistence::encode(&restored, &content).unwrap(), bytes);
    assert_eq!(persistence::snapshot_schema(&world), 26);
    world.advance(&content, DaySpan::new(90)).unwrap();
    restored.advance(&content, DaySpan::new(90)).unwrap();
    assert_eq!(restored, world);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    let observer = world.project(&content, Viewer::Observer).unwrap();
    assert!(observer.own.is_none());
    let report = &observer.reports[&world.player];
    let released = report.archived_reviews.as_ref().unwrap();
    assert!(!released.is_empty());
    assert!(
        !serde_json::to_string(released)
            .unwrap()
            .contains("treasury")
    );
    assert!(
        released
            .iter()
            .all(|summary| summary.last_day < report.history[0].day)
    );
}

#[test]
fn forged_archive_ranges_missing_state_and_legacy_injection_are_rejected() {
    let content = pack();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(3660)).unwrap();
    world.advance(&content, DaySpan::new(600)).unwrap();
    for case in 0..6 {
        let mut bad = world.clone();
        let country = bad.countries.get_mut(&bad.player.clone()).unwrap();
        if case == 0 {
            country.statistical_archive = None;
        } else {
            let recent = country.history[0].day;
            let summary = &mut country.statistical_archive.as_mut().unwrap().years[0];
            match case {
                1 => summary.samples = 0,
                2 => summary.last_day = recent,
                3 => summary.treasury_sum = Money(i64::MAX),
                4 => summary.year = 2099,
                _ => {
                    summary.first_day = sim_core::CampaignDay::new(0);
                    summary.last_day = sim_core::CampaignDay::new(u32::MAX);
                }
            }
        }
        assert!(bad.validate(&content).is_err(), "case {case}");
    }
    let legacy = Content::stable_elections_bundled().unwrap();
    let mut old = World::new(&legacy, CountryId("GBR".into()), 17).unwrap();
    old.countries
        .get_mut(&old.player.clone())
        .unwrap()
        .statistical_archive = Some(StatisticalArchive::default());
    assert!(old.validate(&legacy).is_err());
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    envelope["schema"] = 25.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&envelope).unwrap(), &content)
            .unwrap_err()
            .contains("format 26")
    );
}

#[test]
fn optional_starter_preserves_the_roster_and_resolves_its_exact_new_format() {
    let preserved = Content::bookmark_identities_bundled().unwrap();
    let content = Content::statistical_history_bundled().unwrap();
    assert_eq!(
        serde_json::to_value(&preserved.countries).unwrap(),
        serde_json::to_value(&content.countries).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&preserved.policies).unwrap(),
        serde_json::to_value(&content.policies).unwrap()
    );
    assert_eq!(
        Content::bundled_for_hash(content.hash()).unwrap().hash(),
        content.hash()
    );
    for definition in &content.countries {
        let world = World::new(&content, definition.id.clone(), 17).unwrap();
        assert_eq!(
            world.player_country().statistical_archive,
            Some(StatisticalArchive::default())
        );
        assert_eq!(persistence::snapshot_schema(&world), 26);
        let bytes = persistence::encode(&world, &content).unwrap();
        assert_eq!(
            persistence::encode(&persistence::decode(&bytes, &content).unwrap(), &content).unwrap(),
            bytes
        );
        assert!(persistence::decode(&bytes, &preserved).is_err());
    }
}
