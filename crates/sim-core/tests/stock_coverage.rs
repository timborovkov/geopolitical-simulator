use sim_core::*;

// Preserve published rules-25 hashes instead of regenerating them for a new default.

#[test]
fn physical_need_coverage_preserves_procurement_costs_timing_and_real_stock() {
    let maximum = NeedDays::new(u32::MAX);
    assert_eq!(maximum.units(), u32::MAX);
    assert_eq!(format!("{maximum:?}"), "4294967295");
    assert_eq!(format!("{maximum}"), "4294967295");
    assert_eq!(serde_json::to_string(&maximum).unwrap(), "4294967295");
    for invalid in ["-1", "1.5", "\"1\"", "4294967296"] {
        assert!(serde_json::from_str::<NeedDays>(invalid).is_err());
    }
    let content = Content::statistical_history_bundled().unwrap();
    assert_eq!(content.hash(), 0x0a0defa128f0a947);
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let initial = world.hash();
    for days in [0, 4, u32::MAX] {
        assert!(
            world
                .issue(
                    &content,
                    Action::ProcureReserve {
                        good: GoodKind::Energy,
                        days: NeedDays::new(days),
                    },
                )
                .is_err()
        );
        assert_eq!(world.hash(), initial);
    }
    world
        .issue(
            &content,
            Action::ProcureReserve {
                good: GoodKind::Energy,
                days: NeedDays::new(1),
            },
        )
        .unwrap();
    // Captured from the preceding release before stock-coverage fields changed.
    assert_eq!(world.hash(), 0xedc5157cfed04b92);
    let reserves = world.player_country().reserves.as_ref().unwrap();
    assert_eq!(reserves.energy.stored, Energy::ZERO);
    let order = &reserves.orders[0];
    assert_eq!(order.days, NeedDays::new(1));
    assert_eq!(order.quantity, 58892);
    assert_eq!(order.reserved, Money(913));
    assert_eq!(order.fee, Money(166));
    assert_eq!(order.review_day, sim_core::CampaignDay::new(14));
    assert_eq!(order.expiry_day, sim_core::CampaignDay::new(103));
    let bytes = persistence::encode(&world, &content).unwrap();
    world = persistence::decode(&bytes, &content).unwrap();
    assert_eq!(persistence::encode(&world, &content).unwrap(), bytes);
    world.advance(&content, DaySpan::new(14)).unwrap();
    assert_eq!(world.hash(), 0x742157cdb0c51291);
    let reserves = world.player_country().reserves.as_ref().unwrap();
    assert_eq!(reserves.orders[0].status, ReserveStatus::Active);
    assert_eq!(reserves.energy.stored, Energy::new(8833).unwrap());
    assert_eq!(reserves.orders[0].paid, Money(137));
    assert_eq!(reserves.orders[0].reserved, Money(776));
    world.advance(&content, DaySpan::new(30)).unwrap();
    assert_eq!(world.hash(), 0xad2d84e8ab210c5d);
    let reserves = world.player_country().reserves.as_ref().unwrap();
    assert_eq!(reserves.orders[0].status, ReserveStatus::Completed);
    assert_eq!(
        reserves.orders[0].resolved_day,
        Some(sim_core::CampaignDay::new(20))
    );
    assert_eq!(reserves.energy.stored, Energy::new(58892).unwrap());
    assert_eq!(reserves.orders[0].paid, Money(913));
    assert_eq!(reserves.orders[0].reserved, Money(0));
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}
