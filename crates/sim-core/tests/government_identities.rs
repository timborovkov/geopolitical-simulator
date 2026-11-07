use sim_core::*;

fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-stable-elections-test.json"
    ))
    .unwrap()
}

#[test]
fn mandate_identity_is_numeric_country_scoped_and_checked_without_implicit_conversion() {
    for number in [0, 1, 2, u32::MAX] {
        let id = GovernmentId(number);
        assert_eq!(serde_json::to_value(id).unwrap(), number);
        assert_eq!(
            serde_json::from_value::<GovernmentId>(serde_json::json!(number)).unwrap(),
            id
        );
        assert_eq!(format!("{id:?}"), number.to_string());
    }
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!(4_294_967_296_u64),
        serde_json::json!("1"),
    ] {
        assert!(serde_json::from_value::<GovernmentId>(invalid).is_err());
    }
    assert_eq!(GovernmentId(u32::MAX).checked_add(1), None);
    assert_eq!(GovernmentId(1).checked_add(1), Some(GovernmentId(2)));
    assert_eq!(GovernmentId(2).saturating_sub(GovernmentId(1)), 1);
    let pack = content();
    let world = World::new(&pack, CountryId("AUR".into()), 17).unwrap();
    let a = &world.countries[&CountryId("AUR".into())];
    let b = &world.countries[&CountryId("BOR".into())];
    // The scalar is scoped to its owning country, as are existing loan IDs.
    assert_eq!(a.government_number, b.government_number);
    assert_ne!((&a.id, a.government_number), (&b.id, b.government_number));
}

#[test]
fn typed_pending_sponsors_survive_restoration_and_expire_on_the_actual_successor() {
    let pack = content();
    let mut world = World::new(&pack, CountryId("AUR".into()), 17).unwrap();
    let delay = world
        .preview(&pack, &world.player, &"coalition".into(), 1)
        .unwrap()
        .delay
        .units();
    world
        .issue(
            &pack,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
    world.advance(&pack, DaySpan::new(delay)).unwrap();
    world
        .issue(
            &pack,
            Action::Reconstitute {
                target: Regime::SingleParty,
                contested: false,
            },
        )
        .unwrap();
    let decision = world
        .player_country()
        .constitution
        .as_ref()
        .unwrap()
        .pending
        .as_ref()
        .unwrap()
        .decision_day;
    world
        .advance(&pack, DaySpan::new(decision - world.day.units() - 1))
        .unwrap();
    world
        .issue(
            &pack,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
    world
        .issue(
            &pack,
            Action::HoldCash {
                target: Money(100),
                days: sim_core::DaySpan::new(30),
            },
        )
        .unwrap();
    let country = world.player_country();
    let sponsor: GovernmentId = country
        .financing
        .as_ref()
        .unwrap()
        .pending
        .as_ref()
        .unwrap()
        .sponsor_government;
    assert_eq!(sponsor, GovernmentId(1));
    assert_eq!(
        country
            .constitution
            .as_ref()
            .unwrap()
            .pending
            .as_ref()
            .unwrap()
            .sponsor_government,
        sponsor
    );
    assert_eq!(
        country
            .cash_management
            .as_ref()
            .unwrap()
            .records
            .last()
            .unwrap()
            .sponsor_government,
        sponsor
    );
    let bytes = persistence::encode(&world, &pack).unwrap();
    let encoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(encoded["world"]["countries"]["AUR"]["government_number"], 1);
    assert_eq!(
        encoded["world"]["countries"]["AUR"]["financing"]["pending"]["sponsor_government"],
        1
    );
    let mut restored = persistence::decode(&bytes, &pack).unwrap();
    assert_eq!(persistence::encode(&restored, &pack).unwrap(), bytes);
    world.advance(&pack, DaySpan::new(1)).unwrap();
    restored.advance(&pack, DaySpan::new(1)).unwrap();
    assert_eq!(world, restored);
    let country = world.player_country();
    assert_eq!(country.government_number, GovernmentId(2));
    assert_eq!(
        country
            .constitution
            .as_ref()
            .unwrap()
            .history
            .last()
            .unwrap()
            .government_number,
        GovernmentId(2)
    );
    assert_eq!(
        country.cabinet.as_ref().unwrap().government,
        GovernmentId(2)
    );
    assert_eq!(
        country.electoral_authority.as_ref().unwrap().government,
        GovernmentId(2)
    );
    assert!(country.financing.as_ref().unwrap().pending.is_none());
    assert!(matches!(
        country
            .cash_management
            .as_ref()
            .unwrap()
            .records
            .last()
            .unwrap()
            .ended,
        Some((_, CashHoldEnd::GovernmentChanged))
    ));
    world.validate(&pack).unwrap();
    assert_eq!(world, persistence::replay(&world, &pack).unwrap());
}

#[test]
fn exhausted_successor_identity_refuses_the_tick_without_wrapping_or_mutating_it() {
    let pack = content();
    let id = CountryId("BOR".into());
    let mut world = World::new(&pack, id.clone(), 17).unwrap();
    world.advance(&pack, DaySpan::new(29)).unwrap();
    // Inject an exhausted allocator to exercise the error path; this is not
    // presented as a normal campaign or as a replayable historical mandate.
    let country = world.countries.get_mut(&id).unwrap();
    country.government_number = GovernmentId(u32::MAX);
    country.elite_support = 0;
    let before = world.clone();
    let error = world.advance(&pack, DaySpan::new(1)).unwrap_err();
    assert!(error.contains("Government identity exhausted"), "{error}");
    assert_eq!(world, before);
}
