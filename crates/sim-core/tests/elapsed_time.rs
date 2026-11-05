use sim_core::*;

#[test]
fn typed_event_elapsed_time_and_follow_on_steps_keep_pre_change_campaign_hashes() {
    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    for action in [
        Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        },
        Action::Borrow {
            amount: Money(1000),
        },
        Action::HoldCash {
            target: Money(100),
            days: 30,
        },
        Action::ReleaseCash {
            request: CashHoldId(3),
        },
        Action::ProcureReserve {
            good: GoodKind::Food,
            days: 1,
        },
    ] {
        world.issue(&content, action).unwrap();
    }
    let before = persistence::encode(&world, &content).unwrap();
    world.advance(&content, DaySpan::ZERO).unwrap();
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    let start = world.day;
    let elapsed: DaySpan = world.advance_to_event(&content).unwrap();
    assert_eq!(elapsed, DaySpan::new(14));
    assert_eq!(world.day - start, elapsed.units());
    assert_eq!(world.hash(), 0xbf9cebfd31a16e85);
    let resumed =
        persistence::decode(&persistence::encode(&world, &content).unwrap(), &content).unwrap();
    assert_eq!(world, resumed);
    world.advance(&content, DaySpan::new(94)).unwrap();
    assert_eq!(world.hash(), 0x9d112c7a163317b6);
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}
