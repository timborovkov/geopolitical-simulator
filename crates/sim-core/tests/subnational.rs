use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-subnational-test.json"
    ))
    .unwrap()
}
fn start(c: &Content, id: &str) -> World {
    World::new(c, CountryId(id.into()), 17).unwrap()
}
fn region(id: &str, key: &str) -> RegionId {
    RegionId(format!("region:{id}:{key}"))
}
fn grant() -> RegionalOrderKind {
    RegionalOrderKind::Grant {
        boost: 2,
        days: sim_core::DaySpan::new(30),
    }
}
fn check(w: &World, c: &Content) {
    w.validate(c).unwrap();
    let bytes = persistence::encode(w, c).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        10
    );
    assert_eq!(persistence::decode(&bytes, c).unwrap(), *w);
    persistence::replay(w, c).unwrap();
}
#[test]
fn protected_tax_entitlements_and_a_paid_local_grant_reconcile_without_creating_cash() {
    let c = content();
    let mut w = start(&c, "AUR");
    let r = region("AUR", "province");
    let opening = w.player_country().accounts.total();
    let preview = w
        .preview_regional_order(&c, &w.player, &r, &grant())
        .unwrap();
    assert_eq!(preview.fee, Money(20));
    assert_eq!(preview.principal, Money(90));
    assert_eq!(preview.daily_supplement, Money(3));
    assert_eq!(preview.end_day, Some(43));
    let hash = w.hash();
    w.preview_regional_order(&c, &w.player, &r, &grant())
        .unwrap();
    assert_eq!(w.hash(), hash);
    w.issue(
        &c,
        Action::RegionalOrder {
            region: r,
            kind: grant(),
        },
    )
    .unwrap();
    assert_eq!(w.player_country().accounts.treasury, Money(4980));
    assert_eq!(w.player_country().available_cash(), Money(4890));
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let state = w.player_country().subnational.as_ref().unwrap();
    assert_eq!(state.orders[0].status, RegionalOrderStatus::Implementing);
    assert_eq!(state.orders[0].spent, Money(3));
    assert_eq!(state.orders[0].remaining, Money(87));
    assert_eq!(state.orders[0].escrow, Money(0));
    assert_eq!(
        state
            .regions
            .iter()
            .map(|r| r.last.as_ref().unwrap().tax_receipts.0)
            .sum::<i64>(),
        w.player_country().report.revenue.0 * 20 / 100
    );
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(29)).unwrap();
    let order = &w.player_country().subnational.as_ref().unwrap().orders[0];
    assert_eq!(order.status, RegionalOrderStatus::Completed);
    assert_eq!(order.spent, Money(90));
    assert_eq!(order.remaining, Money(0));
    assert_eq!(w.player_country().accounts.total(), opening);
    assert_eq!(w.player_country().energy_capacity, 80);
    check(&w, &c);
}
#[test]
fn autonomous_services_cannot_be_ordered_and_delegated_services_use_current_consent() {
    let c = content();
    let mut w = start(&c, "AUR");
    let before = w.hash();
    assert!(
        w.issue(
            &c,
            Action::RegionalOrder {
                region: region("AUR", "province"),
                kind: RegionalOrderKind::Directive { level: 3 }
            }
        )
        .unwrap_err()
        .contains("own service budget")
    );
    assert_eq!(w.hash(), before);
    assert!(
        w.issue(
            &c,
            Action::RegionalOrder {
                region: region("BOR", "city"),
                kind: grant()
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), before);
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "city"),
            kind: RegionalOrderKind::Directive { level: 3 },
        },
    )
    .unwrap();
    assert_eq!(
        w.player_country().subnational.as_ref().unwrap().regions[1].service_level,
        2
    );
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    assert_eq!(
        w.player_country().subnational.as_ref().unwrap().regions[1].service_level,
        3
    );
    check(&w, &c);
    let mut royal = start(&c, "COR");
    royal
        .issue(
            &c,
            Action::RegionalOrder {
                region: region("COR", "city"),
                kind: RegionalOrderKind::Directive { level: 3 },
            },
        )
        .unwrap();
    assert_eq!(
        royal.player_country().subnational.as_ref().unwrap().orders[0].status,
        RegionalOrderStatus::Delivered
    );
    check(&royal, &c);
}
#[test]
fn withdrawal_and_refusal_release_only_escrow_and_keep_administration_and_cooldown() {
    let c = content();
    let mut w = start(&c, "AUR");
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "province"),
            kind: grant(),
        },
    )
    .unwrap();
    w.issue(
        &c,
        Action::CancelRegionalOrder {
            order: RegionalOrderId(1),
        },
    )
    .unwrap();
    assert_eq!(w.player_country().available_cash(), Money(4980));
    let before = w.hash();
    assert!(
        w.issue(
            &c,
            Action::RegionalOrder {
                region: region("AUR", "province"),
                kind: grant()
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), before);
    check(&w, &c);
    let mut refused = start(&c, "AUR");
    refused
        .issue(
            &c,
            Action::RegionalOrder {
                region: region("AUR", "province"),
                kind: grant(),
            },
        )
        .unwrap();
    refused
        .issue(
            &c,
            Action::EmergencyBorrow {
                amount: Money(10000),
            },
        )
        .unwrap();
    refused.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let order = &refused
        .player_country()
        .subnational
        .as_ref()
        .unwrap()
        .orders[0];
    assert!(matches!(order.status, RegionalOrderStatus::Refused { .. }));
    assert_eq!(order.escrow, Money(0));
    assert_eq!(order.spent, Money(0));
    assert!(!refused.is_developer_modified());
    check(&refused, &c);
}

#[test]
fn national_spending_and_debt_repayment_cannot_consume_local_tax_entitlements() {
    let c = content();
    let mut w = start(&c, "AUR");
    w.advance(&c, sim_core::DaySpan::new(90)).unwrap();
    let protected = w.player_country().regional_protected_cash();
    assert!(protected.0 > 0);
    let available = w.player_country().available_cash();
    w.issue(&c, Action::Repay { amount: available }).unwrap();
    assert_eq!(w.player_country().accounts.treasury, protected);
    assert_eq!(w.player_country().available_cash(), Money(0));
    let hash = w.hash();
    for action in [
        Action::Repay { amount: Money(1) },
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
        Action::Appoint {
            office: OfficeId("portfolio:AUR:supply".into()),
            person: PersonId("generated:AUR:supply:professional".into()),
        },
        Action::RegionalOrder {
            region: region("AUR", "city"),
            kind: grant(),
        },
    ] {
        assert!(w.issue(&c, action).is_err());
        assert_eq!(w.hash(), hash);
    }
    let outlook = w.funding_outlook(&c, 30, None).unwrap();
    assert_eq!(outlook.available_cash, Money(0));
    let forecast = w
        .player_country()
        .forecast(c.country(&w.player).unwrap(), c.income_taxes.as_ref());
    assert_eq!(
        outlook.daily_balance.0,
        forecast.daily_revenue.0 - forecast.daily_revenue.0 * 20 / 100 - forecast.daily_spending.0
            + 5
            - forecast.daily_interest.0
    );
    check(&w, &c);
}
#[test]
fn accepted_local_funding_survives_a_costed_constitutional_successor_and_cannot_be_reclaimed() {
    let c = content();
    let mut w = start(&c, "AUR");
    // Build effective backing through ordinary agreements, without developer edits.
    for _ in 0..3 {
        let preview = w.preview(&c, &w.player, &"coalition".into(), 1).unwrap();
        w.issue(
            &c,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
        w.advance(&c, preview.delay).unwrap();
    }
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "province"),
            kind: RegionalOrderKind::Grant {
                boost: 2,
                days: sim_core::DaySpan::new(90),
            },
        },
    )
    .unwrap();
    let request = RegionalOrderId(w.next_command.0 - 1);
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let before = w.hash();
    assert!(
        w.issue(&c, Action::CancelRegionalOrder { order: request })
            .is_err()
    );
    assert_eq!(w.hash(), before);
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let order = w
        .player_country()
        .subnational
        .as_ref()
        .unwrap()
        .orders
        .iter()
        .find(|o| o.id == request)
        .unwrap();
    assert_eq!(order.status, RegionalOrderStatus::Implementing);
    assert_eq!(order.spent.0, order.daily_supplement.0 * 15);
    assert!(!w.is_developer_modified());
    check(&w, &c);
}
#[test]
fn pending_local_offers_lose_their_sponsor_without_cancelling_other_local_mandates() {
    let mut c = content();
    c.subnational.as_mut().unwrap().grant_review_days = DaySpan::new(30);
    c.validate().unwrap();
    let mut w = start(&c, "AUR");
    for _ in 0..3 {
        let preview = w.preview(&c, &w.player, &"coalition".into(), 1).unwrap();
        w.issue(
            &c,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
        w.advance(&c, preview.delay).unwrap();
    }
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "province"),
            kind: grant(),
        },
    )
    .unwrap();
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let order = &w.player_country().subnational.as_ref().unwrap().orders[0];
    assert_eq!(order.status, RegionalOrderStatus::Superseded);
    assert_eq!(order.escrow, Money(0));
    assert!(w.player_country().regional_protected_cash().0 > 0);
    check(&w, &c);
}
#[test]
fn territorial_provenance_private_cash_calendar_and_saved_state_are_validated() {
    let c = content();
    let mut bad = c.clone();
    let share = &mut bad.subnational.as_mut().unwrap().charters[0].regions[0].population_share_bps;
    *share = share.checked_add(BasisPoints::new(1)).unwrap();
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.subnational.as_mut().unwrap().charters[0].regions[0].authority =
        bad.elections.as_ref().unwrap().mandates[0]
            .authority
            .clone();
    assert!(bad.validate().is_err());
    let mut w = start(&c, "AUR");
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "province"),
            kind: grant(),
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let public = serde_json::to_string(&w.project(&c, Viewer::Observer).unwrap()).unwrap();
    for private in [
        "tax_received",
        "escrow",
        "paid_supplement",
        "paid_administration",
    ] {
        assert!(!public.contains(private));
    }
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player)
        .unwrap()
        .subnational
        .as_mut()
        .unwrap()
        .regions[0]
        .cash
        .0 += 1;
    assert!(bad.validate(&c).is_err());
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player)
        .unwrap()
        .subnational
        .as_mut()
        .unwrap()
        .orders[0]
        .baseline_level = 3;
    assert!(bad.validate(&c).is_err());
    let mut invalid_date = w.clone();
    invalid_date
        .countries
        .get_mut(&w.player)
        .unwrap()
        .subnational
        .as_mut()
        .unwrap()
        .orders[0]
        .proposed_day = u32::MAX;
    assert!(invalid_date.validate(&c).is_err());
    let mut bytes: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    bytes["schema"] = 9.into();
    assert!(persistence::decode(&serde_json::to_vec(&bytes).unwrap(), &c).is_err());
    check(&w, &c);
    let later = c.at_bookmark("2095-01-01").unwrap();
    let mut late = start(&later, "AUR");
    late.advance(&later, sim_core::DaySpan::new(MAX_DAY - late.day - 5))
        .unwrap();
    assert!(
        late.preview_regional_order(&later, &late.player, &region("AUR", "city"), &grant())
            .unwrap_err()
            .contains("calendar")
    );
    check(&late, &later);
}
#[test]
fn ai_uses_shared_costed_offers_and_earlier_campaigns_keep_their_published_checksums() {
    let c = content();
    let mut w = start(&c, "BOR");
    w.advance(&c, sim_core::DaySpan::new(180)).unwrap();
    assert!(
        w.journal
            .iter()
            .any(|entry| entry.ai && matches!(entry.command.action, Action::RegionalOrder { .. }))
    );
    check(&w, &c);
    let earlier = Content::cabinet_bundled().unwrap();
    let mut old = World::new(&earlier, CountryId("GBR".into()), 17).unwrap();
    old.issue(
        &earlier,
        Action::Appoint {
            office: OfficeId("portfolio:GBR:supply".into()),
            person: PersonId("generated:GBR:supply:professional".into()),
        },
    )
    .unwrap();
    assert_eq!(old.hash(), 0x27b6cdc4d32c00b3);
    assert!(
        old.issue(
            &earlier,
            Action::RegionalOrder {
                region: region("GBR", "province"),
                kind: grant()
            }
        )
        .is_err()
    );
    assert_eq!(old.hash(), 0x27b6cdc4d32c00b3);
    persistence::replay(&old, &earlier).unwrap();
}

#[test]
fn a_local_absorption_mandate_can_refuse_a_grant_even_when_national_backing_remains_sufficient() {
    let c = content();
    let mut w = start(&c, "AUR");
    while w.player_country().subnational.as_ref().unwrap().regions[0]
        .cash
        .0
        < 538
    {
        assert!(w.day < 1000);
        w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    }
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "province"),
            kind: grant(),
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let order = &w.player_country().subnational.as_ref().unwrap().orders[0];
    match &order.status {
        RegionalOrderStatus::Refused { reason } => {
            assert!(reason.contains("Local cash absorption"))
        }
        other => panic!("Expected a local refusal, got {other:?}"),
    }
    assert_eq!(order.escrow, Money(0));
    assert_eq!(order.spent, Money(0));
    assert!(w.player_country().legislative_support >= 51);
    check(&w, &c);
}

#[test]
fn an_instant_royal_order_after_settlement_keeps_the_dated_report_and_can_save_immediately() {
    let c = content();
    let mut w = start(&c, "COR");
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    let settled = w.player_country().subnational.as_ref().unwrap().regions[1]
        .last
        .clone();
    let before = w.player_country().accounts.treasury;
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("COR", "city"),
            kind: RegionalOrderKind::Directive { level: 3 },
        },
    )
    .unwrap();
    assert_eq!(w.player_country().accounts.treasury.0, before.0 - 20);
    assert_eq!(
        w.player_country().subnational.as_ref().unwrap().regions[1].service_level,
        3
    );
    assert_eq!(
        w.player_country().subnational.as_ref().unwrap().regions[1].last,
        settled
    );
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    let current = w.player_country().subnational.as_ref().unwrap().regions[1]
        .last
        .as_ref()
        .unwrap();
    assert_eq!(current.requested_base, Money(3));
    assert_eq!(current.day, 2);
    check(&w, &c);
}
