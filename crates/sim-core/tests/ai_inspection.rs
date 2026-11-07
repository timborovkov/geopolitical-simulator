use sim_core::*;

fn fixture() -> (Content, World) {
    let content = Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world
        .issue(&content, Action::Borrow { amount: Money(50) })
        .unwrap();
    world.advance(&content, DaySpan::new(391)).unwrap();
    (content, world)
}
#[test]
fn developer_ai_view_is_bounded_country_scoped_and_preserves_recorded_origin_and_wording() {
    let (content, world) = fixture();
    let before = serde_json::to_vec(&world).unwrap();
    let foreign = CountryId("BOR".into());
    let full = world.developer_ai_log(&foreign, 500).unwrap();
    assert!(!full.player_controlled);
    assert!(full.accepted_total >= 3);
    assert!(full.reviews_available >= 3);
    assert!(
        full.commands
            .iter()
            .all(|e| e.ai && e.command.issuer == foreign)
    );
    assert!(
        full.reviews
            .iter()
            .all(|e| e.country == foreign && e.text.starts_with("AI "))
    );
    assert!(
        full.commands
            .windows(2)
            .all(|p| p[0].command.id > p[1].command.id)
    );
    assert!(full.reviews.windows(2).all(|p| p[0].day >= p[1].day));
    let limited = world.developer_ai_log(&foreign, 1).unwrap();
    assert_eq!(limited.accepted_total, full.accepted_total);
    assert_eq!(limited.reviews_available, full.reviews_available);
    assert_eq!(limited.commands, full.commands[..1]);
    assert_eq!(limited.reviews, full.reviews[..1]);
    for entry in &full.commands {
        assert_eq!(*entry, &world.journal[entry.command.id.0 as usize - 1]);
    }
    assert!(
        full.reviews
            .iter()
            .all(|entry| world.events.contains(entry))
    );
    let own = world.developer_ai_log(&world.player, 100).unwrap();
    assert!(own.player_controlled);
    assert_eq!(own.accepted_total, 0);
    assert!(own.commands.is_empty());
    assert_eq!(before, serde_json::to_vec(&world).unwrap());
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn missing_review_messages_are_not_invented_and_bad_inspection_requests_are_read_only() {
    let (content, mut world) = fixture();
    let foreign = CountryId("BOR".into());
    let original_commands = world
        .developer_ai_log(&foreign, 500)
        .unwrap()
        .accepted_total;
    // Legacy event retention can remove explanations while the journal survives.
    world
        .events
        .retain(|e| !(e.country == foreign && e.text.starts_with("AI ")));
    world.validate(&content).unwrap();
    let before = serde_json::to_vec(&world).unwrap();
    let log = world.developer_ai_log(&foreign, 500).unwrap();
    assert_eq!(log.accepted_total, original_commands);
    assert_eq!(log.reviews_available, 0);
    assert!(log.reviews.is_empty());
    assert_eq!(
        serde_json::to_value(&log).unwrap()["reviews"],
        serde_json::json!([])
    );
    for limit in [0, 501, usize::MAX] {
        assert!(world.developer_ai_log(&foreign, limit).is_err());
    }
    assert!(
        world
            .developer_ai_log(&CountryId("missing".into()), 100)
            .is_err()
    );
    assert_eq!(before, serde_json::to_vec(&world).unwrap());
}
