use sim_core::*;

// Preserve published rules-25 hashes instead of regenerating them for a new default.

#[test]
fn whole_day_scaling_retains_units_rounding_and_checked_bounds() {
    let vacancy = DaySpan::new(30);
    assert_eq!(vacancy.scaled(15, 100), Some(DaySpan::new(4)));
    assert_eq!(vacancy.scaled(45, 100), Some(DaySpan::new(13)));
    assert_eq!(vacancy.scaled(0, 100), Some(DaySpan::ZERO));
    assert_eq!(vacancy.scaled(1, 0), None);
    let maximum = DaySpan::new(u32::MAX);
    assert_eq!(maximum.scaled(u32::MAX, u32::MAX), Some(maximum));
    assert_eq!(maximum.scaled(2, 1), None);
    assert_eq!(maximum.checked_add(DaySpan::new(1)), None);
    assert_eq!(DaySpan::ZERO.checked_sub(DaySpan::new(1)), None);
    assert_eq!(format!("{vacancy:?}"), "30");
    assert_eq!(format!("{vacancy}"), "30");
    assert_eq!(serde_json::to_string(&maximum).unwrap(), "4294967295");
    for invalid in ["-1", "4294967296", "1.5", "\"30\""] {
        assert!(serde_json::from_str::<DaySpan>(invalid).is_err());
    }
}

#[test]
fn configured_durations_keep_original_numeric_content_pending_delivery_and_replay() {
    let content = Content::statistical_history_bundled().unwrap();
    // Captured with the release executable before the duration conversion.
    assert_eq!(content.hash(), 0x0a0defa128f0a947);
    let json = serde_json::to_value(&content).unwrap();
    assert_eq!(json["countries"][0]["government"]["consent_days"], 14);
    assert_eq!(json["finance"]["maturity_days"], 365);
    assert_eq!(json["cabinet"]["vacancy_preparation_days"], 30);
    let restored = Content::from_json(&serde_json::to_string(&json).unwrap()).unwrap();
    assert_eq!(restored.hash(), content.hash());

    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let policy = PolicyId::from("income_tax");
    let preview = world.preview(&content, &world.player, &policy, 30).unwrap();
    assert_eq!(preview.consent_delay, DaySpan::new(14));
    assert_eq!(preview.delay, DaySpan::new(78));
    world
        .issue(&content, Action::Enact { policy, value: 30 })
        .unwrap();
    world.advance(&content, DaySpan::new(7)).unwrap();
    assert_eq!(world.hash(), 0x07e398663b1c5328);
    let project = &world.player_country().projects[0];
    assert_eq!(project.consent_day, sim_core::CampaignDay::new(14));
    assert_eq!(project.due_day, sim_core::CampaignDay::new(78));
    assert_eq!(project.reserved, Money(300));
    let encoded = persistence::encode(&world, &content).unwrap();
    world = persistence::decode(&encoded, &content).unwrap();
    assert_eq!(persistence::encode(&world, &content).unwrap(), encoded);
    world.advance(&content, DaySpan::new(101)).unwrap();
    assert_eq!(world.hash(), 0x65f26cfaa6d0aa93);
    assert_eq!(world.player_country().tax, 30);
    assert_eq!(world, persistence::replay(&world, &content).unwrap());

    let mut invalid = content.clone();
    invalid.countries[0].government.consent_days = DaySpan::new(u32::MAX);
    assert!(invalid.validate().is_err());
}
