use sim_core::*;

// Preserve published rules-25 hashes instead of regenerating them for a new default.

#[test]
fn requested_elapsed_spans_keep_cash_expiry_grant_payments_and_replay_exact() {
    let content = Content::statistical_history_bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let initial = world.hash();
    for days in [0, 91, u32::MAX] {
        assert!(
            world
                .issue(
                    &content,
                    Action::HoldCash {
                        target: Money(50),
                        days: DaySpan::new(days)
                    },
                )
                .is_err()
        );
        assert_eq!(world.hash(), initial);
    }
    world
        .issue(
            &content,
            Action::HoldCash {
                target: Money(50),
                days: DaySpan::new(30),
            },
        )
        .unwrap();
    let region = content
        .subnational
        .as_ref()
        .unwrap()
        .charter(&world.player)
        .unwrap()
        .regions
        .iter()
        .find(|r| r.tier == RegionalTier::Region)
        .unwrap()
        .id
        .clone();
    let retained = world.hash();
    for days in [0, 29, 181, u32::MAX] {
        assert!(
            world
                .issue(
                    &content,
                    Action::RegionalOrder {
                        region: region.clone(),
                        kind: RegionalOrderKind::Grant {
                            boost: 1,
                            days: DaySpan::new(days)
                        },
                    },
                )
                .is_err()
        );
        assert_eq!(world.hash(), retained);
    }
    world
        .issue(
            &content,
            Action::RegionalOrder {
                region,
                kind: RegionalOrderKind::Grant {
                    boost: 1,
                    days: DaySpan::new(30),
                },
            },
        )
        .unwrap();
    // Captured with the preceding release before command duration fields changed.
    assert_eq!(world.hash(), 0x95e5bbd29aa9a89c);
    assert_eq!(world.player_country().retained_cash(), Money(50));
    let encoded = persistence::encode(&world, &content).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        value["world"]["journal"][0]["command"]["action"]["HoldCash"]["days"],
        30
    );
    world = persistence::decode(&encoded, &content).unwrap();
    assert_eq!(persistence::encode(&world, &content).unwrap(), encoded);
    world.advance(&content, DaySpan::new(14)).unwrap();
    assert_eq!(world.hash(), 0x5b42f91462fafef5);
    let budget = world.player_country().subnational.as_ref().unwrap();
    let order = budget.orders.first().unwrap();
    assert_eq!(order.decision_day, sim_core::CampaignDay::new(14));
    assert_eq!(order.end_day, Some(sim_core::CampaignDay::new(43)));
    assert_eq!(order.status, RegionalOrderStatus::Implementing);
    let paid = order.spent;
    assert!(paid.0 > 0);
    assert_eq!(order.escrow, Money(0));
    world.advance(&content, DaySpan::new(30)).unwrap();
    assert_eq!(world.hash(), 0x38ededc3a2db03a3);
    assert_eq!(world.player_country().retained_cash(), Money(0));
    let order = &world.player_country().subnational.as_ref().unwrap().orders[0];
    assert_eq!(order.status, RegionalOrderStatus::Completed);
    assert_eq!(order.spent, order.principal);
    assert_eq!(order.remaining, Money(0));
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}
