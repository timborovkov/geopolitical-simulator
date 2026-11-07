use sim_core::*;

#[test]
fn dated_bookmark_records_keep_absolute_stamps_through_publication_restore_and_replay() {
    let content =
        Content::from_json(include_str!("../../../data/synthetic-dated-test.json")).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let opening = CampaignDay::new(parse_date("2001-07-15").unwrap());
    assert_eq!(world.day, opening);
    world
        .issue(
            &content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30,
            },
        )
        .unwrap();
    assert_eq!(world.journal.last().unwrap().command.day, opening);
    assert!(!world.ledger.is_empty());
    assert!(world.ledger.iter().all(|entry| entry.day == opening));
    assert!(world.events.iter().all(|event| event.day == opening));
    world.advance(&content, DaySpan::new(30)).unwrap();
    let through = opening.checked_add(DaySpan::new(30)).unwrap();
    assert_eq!(world.day, through);
    let view = world
        .project(&content, Viewer::Government(world.player.clone()))
        .unwrap();
    assert_eq!(view.day, through);
    let latest = view.reports[&world.player]
        .latest
        .as_ref()
        .expect("dated release");
    assert!(latest.day >= opening && latest.day <= through);
    assert!(
        world
            .player_country()
            .history
            .iter()
            .any(|point| point.day == latest.day)
    );
    let encoded = persistence::encode(&world, &content).unwrap();
    let raw: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(raw["schema"], 3);
    assert_eq!(raw["world"]["day"], through.units());
    assert_eq!(
        raw["world"]["journal"][0]["command"]["day"],
        opening.units()
    );
    let mut restored = persistence::decode(&encoded, &content).unwrap();
    assert_eq!(restored, world);
    assert_eq!(persistence::encode(&restored, &content).unwrap(), encoded);
    for next in [&mut world, &mut restored] {
        next.advance(&content, DaySpan::new(17)).unwrap();
    }
    assert_eq!(restored, world);
    assert_eq!(persistence::replay(&restored, &content).unwrap(), restored);
}
