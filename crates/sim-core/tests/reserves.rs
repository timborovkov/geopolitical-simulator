use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-national-reserves-test.json"
    ))
    .unwrap()
}
fn id() -> CountryId {
    CountryId("AUR".into())
}
fn seed() -> u64 {
    (0..500)
        .find(|s| {
            let first = stable_hash(format!("{s}:AUR:0:supply-v2").as_bytes());
            let next = stable_hash(format!("{s}:AUR:1:supply-v2").as_bytes());
            !first.is_multiple_of(3) && next.is_multiple_of(3) && 10 + next % 16 >= 20
        })
        .unwrap()
}
fn world(c: &Content) -> World {
    World::new(c, id(), seed()).unwrap()
}
fn issue(w: &mut World, c: &Content, g: GoodKind, days: u32) -> ReserveOrderId {
    let request = ReserveOrderId(w.next_command.0);
    w.issue(
        c,
        Action::ProcureReserve {
            good: g,
            days: sim_core::NeedDays::new(days),
        },
    )
    .unwrap();
    request
}
fn verify(w: &World, c: &Content) {
    w.validate(c).unwrap();
    assert_eq!(*w, persistence::replay(w, c).unwrap());
}
#[test]
fn proposal_is_paid_reserved_and_delayed_then_deliveries_reconcile_cash_and_spare_quota() {
    let c = content();
    let mut w = world(&c);
    let opening = w.player_country().accounts.clone();
    let p = w
        .preview_reserve(&c, &id(), GoodKind::Energy, NeedDays::new(1))
        .unwrap();
    let r = issue(&mut w, &c, GoodKind::Energy, 1);
    assert_eq!(
        w.player_country().reserves.as_ref().unwrap().energy.stored,
        Energy::ZERO
    );
    assert_eq!(
        w.player_country().accounts.treasury.0,
        opening.treasury.0 - p.fee.0
    );
    assert_eq!(w.player_country().committed_cash, p.cost);
    w.advance(&c, sim_core::DaySpan::new(p.review_day - 1))
        .unwrap();
    assert_eq!(
        w.player_country().reserves.as_ref().unwrap().orders[0].filled,
        0
    );
    let mut paid = 0;
    while w.day.units() < 60 {
        w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
        w.validate(&c).unwrap();
        let player = w.player_country();
        let state = player.reserves.as_ref().unwrap();
        let f = state.energy.last.as_ref().unwrap();
        assert!(f.purchased.units() + player.report.energy_imported.units() <= 250);
        paid += w
            .ledger
            .iter()
            .filter(|e| {
                e.day == w.day.units()
                    && e.country == id()
                    && e.reason == "National reserve commodity procurement"
            })
            .map(|e| e.amount.0)
            .sum::<i64>();
    }
    let o = w
        .player_country()
        .reserves
        .as_ref()
        .unwrap()
        .orders
        .iter()
        .find(|o| o.id == r)
        .unwrap();
    assert_eq!(o.status, ReserveStatus::Completed);
    assert_eq!(o.paid, p.cost);
    assert_eq!(paid, p.cost.0);
    assert_eq!(o.reserved, Money(0));
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert_eq!(
        w.player_country()
            .reserves
            .as_ref()
            .unwrap()
            .energy
            .stored
            .units(),
        1000
    );
    verify(&w, &c);
}
#[test]
fn reserves_reduce_actual_post_import_shortfall_and_do_not_create_output_before_use() {
    let c = content();
    let mut w = world(&c);
    let mut baseline = world(&c);
    issue(&mut w, &c, GoodKind::Energy, 1);
    w.advance(&c, sim_core::DaySpan::new(90)).unwrap();
    baseline.advance(&c, sim_core::DaySpan::new(90)).unwrap();
    assert_eq!(
        w.player_country().report.output,
        baseline.player_country().report.output
    );
    assert_eq!(
        w.player_country()
            .reserves
            .as_ref()
            .unwrap()
            .energy
            .released,
        Energy::ZERO
    );
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    baseline.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    assert!(
        w.player_country().report.energy_coverage
            > baseline.player_country().report.energy_coverage
    );
    assert!(w.player_country().report.output.0 > baseline.player_country().report.output.0);
    let s = w.player_country().reserves.as_ref().unwrap();
    assert!(s.energy.last.as_ref().unwrap().released.units() > 0);
    assert!(w.events.iter().any(|e| e.country == id()
        && e.day == w.day.units()
        && e.important
        && e.text.contains("National reserves are covering")));
    assert_eq!(
        s.energy.purchased.units() - s.energy.released.units(),
        s.energy.stored.units()
    );
    verify(&w, &c);
}
#[test]
fn food_spoilage_and_release_keep_national_and_civilian_stock_books_separate() {
    let c = content();
    let mut w = world(&c);
    issue(&mut w, &c, GoodKind::Food, 1);
    w.advance(&c, sim_core::DaySpan::new(90)).unwrap();
    let before = w.player_country().reserves.as_ref().unwrap();
    assert!(before.food.lost.units() > 0);
    assert_eq!(before.food.released, Food::ZERO);
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    let s = w.player_country().reserves.as_ref().unwrap();
    assert!(s.food.last.as_ref().unwrap().released.units() > 0);
    assert_eq!(
        s.food.purchased.units() - s.food.released.units() - s.food.lost.units(),
        s.food.stored.units()
    );
    let m = w
        .player_country()
        .markets
        .as_ref()
        .unwrap()
        .food
        .last
        .as_ref()
        .unwrap();
    assert_eq!(
        m.opening_stock.units()
            + m.production.units()
            + m.imports.units()
            + m.reserve_release.unwrap().units(),
        m.supply.units()
    );
    verify(&w, &c);
}
#[test]
fn unfed_civilian_physical_needs_prevent_national_procurement_and_expiry_releases_unspent_cash() {
    let mut c = content();
    for d in &mut c.countries {
        d.energy = 40;
        d.food = 40;
    }
    let mut w = world(&c);
    let p = w
        .preview_reserve(&c, &id(), GoodKind::Energy, NeedDays::new(1))
        .unwrap();
    issue(&mut w, &c, GoodKind::Energy, 1);
    w.advance(&c, sim_core::DaySpan::new(p.expiry_day + 1))
        .unwrap();
    let o = &w.player_country().reserves.as_ref().unwrap().orders[0];
    assert_eq!(o.status, ReserveStatus::Expired);
    assert_eq!(o.filled, 0);
    assert_eq!(o.paid, Money(0));
    assert_eq!(o.reserved, Money(0));
    assert_eq!(w.player_country().committed_cash, Money(0));
    verify(&w, &c);
}
#[test]
fn cancellation_keeps_delivered_goods_and_paid_costs_and_returns_only_unspent_reservations() {
    let c = content();
    let mut w = world(&c);
    let request = issue(&mut w, &c, GoodKind::Energy, 2);
    w.advance(&c, sim_core::DaySpan::new(20)).unwrap();
    let before = w.player_country().clone();
    let o = before.reserves.as_ref().unwrap().orders[0].clone();
    assert!(o.filled > 0 && o.filled < o.quantity);
    w.issue(&c, Action::CancelReserve { request }).unwrap();
    let after = w.player_country();
    assert_eq!(after.accounts, before.accounts);
    assert_eq!(
        after.reserves.as_ref().unwrap().energy.stored,
        before.reserves.as_ref().unwrap().energy.stored
    );
    assert_eq!(
        after.committed_cash.0,
        before.committed_cash.0 - o.reserved.0
    );
    assert_eq!(after.reserves.as_ref().unwrap().orders[0].paid, o.paid);
    verify(&w, &c);
    let h = w.hash();
    assert!(w.issue(&c, Action::CancelReserve { request }).is_err());
    assert_eq!(w.hash(), h);
}
#[test]
fn support_is_rechecked_at_consent_and_refusal_does_not_deliver_goods() {
    let mut c = content();
    let d = c.countries.iter_mut().find(|d| d.id == id()).unwrap();
    d.government.consent_days = DaySpan::new(60);
    d.parties[0].seats = 51;
    d.parties[1].seats = 49;
    d.tax = 10;
    d.services = 4;
    d.welfare = 0;
    let profiles = &mut c.party_preferences.as_mut().unwrap().profiles;
    for p in &mut profiles
        .iter_mut()
        .find(|p| p.party.0 == "AUR-1")
        .unwrap()
        .positions
    {
        let def = c.policies.iter().find(|d| d.id == p.policy).unwrap();
        p.preferred = Some(PreferredPolicyRange {
            min: def.max,
            max: def.max,
        });
    }
    c.validate().unwrap();
    let mut w = world(&c);
    let p = w
        .preview_reserve(&c, &id(), GoodKind::Energy, NeedDays::new(1))
        .unwrap();
    issue(&mut w, &c, GoodKind::Energy, 1);
    w.advance(&c, sim_core::DaySpan::new(p.review_day)).unwrap();
    let o = &w.player_country().reserves.as_ref().unwrap().orders[0];
    assert!(matches!(o.status, ReserveStatus::Refused { .. }));
    assert_eq!(o.filled, 0);
    assert_eq!(o.paid, Money(0));
    assert_eq!(o.reserved, Money(0));
    verify(&w, &c);
}
#[test]
fn rounding_is_prepaid_cumulatively_and_fixed_price_contracts_do_not_reprice_delivered_goods() {
    let mut c = content();
    let energy = c
        .markets
        .as_mut()
        .unwrap()
        .goods
        .iter_mut()
        .find(|g| g.kind == GoodKind::Energy)
        .unwrap();
    energy.import_limit_bps = BasisPoints::new(2100);
    let mut w = world(&c);
    issue(&mut w, &c, GoodKind::Energy, 1);
    let price = w.player_country().reserves.as_ref().unwrap().orders[0]
        .price_bps
        .units();
    for _ in 0..110 {
        w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
        w.validate(&c).unwrap();
        let o = &w.player_country().reserves.as_ref().unwrap().orders[0];
        assert_eq!(o.price_bps.units(), price);
        assert!(
            i128::from(o.paid.0) * i128::from(o.quantity)
                >= i128::from(o.quoted_cost.0) * i128::from(o.filled)
        );
    }
    verify(&w, &c);
}
#[test]
fn bad_definitions_requests_and_private_foreign_projections_fail_without_mutation() {
    let original: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/synthetic-national-reserves-test.json"
    ))
    .unwrap();
    for (field, value) in [
        ("capacity_days", 0),
        ("capacity_days", 11),
        ("order_days", 29),
        ("order_days", 366),
        ("fee_bps", 0),
        ("ai_target_days", 4),
    ] {
        let mut v = original.clone();
        v["reserves"][field] = value.into();
        assert!(Content::from_json(&v.to_string()).is_err());
    }
    let mut v = original.clone();
    v["reserves"]["source"] = "missing".into();
    assert!(Content::from_json(&v.to_string()).is_err());
    let mut v = original.clone();
    v.as_object_mut().unwrap().remove("reserves");
    assert!(Content::from_json(&v.to_string()).is_err());
    for base in [
        original.clone(),
        serde_json::to_value(Content::bundled().unwrap()).unwrap(),
    ] {
        for field in ["energy", "food"] {
            let mut invalid = base.clone();
            invalid["countries"][0][field] = 201.into();
            assert!(
                Content::from_json(&invalid.to_string()).is_err(),
                "legacy 200-point {field} capacity ceiling changed"
            );
        }
    }
    let c = content();
    let mut w = world(&c);
    let h = w.hash();
    for days in [0, 4, u32::MAX] {
        assert!(
            w.issue(
                &c,
                Action::ProcureReserve {
                    good: GoodKind::Energy,
                    days: sim_core::NeedDays::new(days)
                }
            )
            .is_err()
        );
        assert_eq!(w.hash(), h);
    }
    let mut other = c.clone();
    other.reserves.as_mut().unwrap().fee_bps = BasisPoints::new(2);
    assert!(
        w.preview_reserve(&other, &id(), GoodKind::Energy, NeedDays::new(1))
            .is_err()
    );
    assert_eq!(w.hash(), h);
    issue(&mut w, &c, GoodKind::Energy, 1);
    let h = w.hash();
    assert!(
        w.issue(
            &c,
            Action::ProcureReserve {
                good: GoodKind::Energy,
                days: sim_core::NeedDays::new(1)
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), h);
    let observer = serde_json::to_string(&w.project(&c, Viewer::Observer).unwrap()).unwrap();
    for private in ["quoted_cost", "last_fill_day", "reserves", "committed_cash"] {
        assert!(!observer.contains(private));
    }
    verify(&w, &c);
}
#[test]
fn save_22_continues_pending_and_partial_orders_and_rejects_forged_stock_and_funding() {
    let c = content();
    let mut w = world(&c);
    issue(&mut w, &c, GoodKind::Energy, 2);
    w.advance(&c, sim_core::DaySpan::new(7)).unwrap();
    let bytes = persistence::encode(&w, &c).unwrap();
    let header: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(header["schema"], 22);
    let mut h = header;
    h["schema"] = 21.into();
    assert!(persistence::decode(&serde_json::to_vec(&h).unwrap(), &c).is_err());
    let mut restored = persistence::decode(&bytes, &c).unwrap();
    w.advance(&c, sim_core::DaySpan::new(15)).unwrap();
    restored.advance(&c, sim_core::DaySpan::new(15)).unwrap();
    assert_eq!(w, restored);
    let bytes = persistence::encode(&w, &c).unwrap();
    let mut restored = persistence::decode(&bytes, &c).unwrap();
    w.advance(&c, sim_core::DaySpan::new(100)).unwrap();
    restored.advance(&c, sim_core::DaySpan::new(100)).unwrap();
    assert_eq!(w, restored);
    verify(&w, &c);
    let mut bad = w.clone();
    bad.countries
        .get_mut(&id())
        .unwrap()
        .reserves
        .as_mut()
        .unwrap()
        .energy
        .stored = Energy::new(12345).unwrap();
    assert!(bad.validate(&c).is_err());
    let mut bad = w.clone();
    bad.countries
        .get_mut(&id())
        .unwrap()
        .reserves
        .as_mut()
        .unwrap()
        .orders[0]
        .paid
        .0 += 1;
    assert!(bad.validate(&c).is_err());
    assert_eq!(Content::party_bundled().unwrap().rules, 20);
    assert!(Content::bundled_for_hash(c.hash()).is_err());
}
#[test]
fn ai_uses_the_same_funded_contracts_and_replay_path() {
    let c = content();
    let mut w = World::new(&c, CountryId("COR".into()), seed()).unwrap();
    w.advance(&c, sim_core::DaySpan::new(1000)).unwrap();
    assert!(
        w.journal
            .iter()
            .any(|e| e.ai && matches!(e.command.action, Action::ProcureReserve { .. }))
    );
    assert!(
        w.countries
            .values()
            .any(|c| c.reserves.as_ref().unwrap().energy.purchased.units() > 0)
    );
    assert!(!w.is_developer_modified());
    verify(&w, &c);
}

#[test]
fn zero_delay_royal_consent_still_waits_for_settlement_and_has_exact_delivery_window() {
    let mut c = content();
    for d in &mut c.countries {
        d.energy = 40;
    }
    let mut w = World::new(&c, CountryId("COR".into()), seed()).unwrap();
    let p = w
        .preview_reserve(&c, &w.player, GoodKind::Energy, NeedDays::new(1))
        .unwrap();
    assert_eq!(p.review_day, w.day.units());
    assert_eq!(p.earliest_delivery_day, w.day.units() + 1);
    assert_eq!(
        p.expiry_day,
        p.earliest_delivery_day + c.reserves.as_ref().unwrap().order_days.units() - 1
    );
    w.issue(
        &c,
        Action::ProcureReserve {
            good: GoodKind::Energy,
            days: sim_core::NeedDays::new(1),
        },
    )
    .unwrap();
    assert_eq!(
        w.player_country().reserves.as_ref().unwrap().energy.stored,
        Energy::ZERO
    );
    assert_eq!(
        w.player_country().reserves.as_ref().unwrap().orders[0].status,
        ReserveStatus::Active
    );
    w.advance(&c, sim_core::DaySpan::new(p.expiry_day)).unwrap();
    assert_eq!(
        w.player_country().reserves.as_ref().unwrap().orders[0].status,
        ReserveStatus::Active
    );
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(
        w.player_country().reserves.as_ref().unwrap().orders[0].status,
        ReserveStatus::Expired
    );
    verify(&w, &c);
}
#[test]
fn procurement_spending_and_pending_reservations_cross_fiscal_years_without_rebooking_administration()
 {
    let mut c = content();
    c.start = "2000-12-31".into();
    c.validate().unwrap();
    let mut w = world(&c);
    let p = w
        .preview_reserve(&c, &id(), GoodKind::Energy, NeedDays::new(1))
        .unwrap();
    issue(&mut w, &c, GoodKind::Energy, 1);
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(
        w.player_country()
            .budget_reviews
            .last()
            .unwrap()
            .period
            .capital,
        p.fee
    );
    assert_eq!(w.player_country().budget.capital, Money(0));
    assert_eq!(w.player_country().committed_cash, p.cost);
    w.advance(&c, sim_core::DaySpan::new(45)).unwrap();
    let paid = w.player_country().reserves.as_ref().unwrap().orders[0].paid;
    assert!(paid.0 > 0);
    assert!(w.player_country().budget.payments.0 >= paid.0);
    assert_eq!(w.player_country().budget.capital, Money(0));
    verify(&w, &c);
}

#[test]
fn reserve_purchases_preserve_policy_capital_local_funds_and_temporary_cash_retention() {
    let c = content();
    let mut w = world(&c);
    w.advance(&c, sim_core::DaySpan::new(30)).unwrap();
    w.issue(
        &c,
        Action::HoldCash {
            target: Money(1000),
            days: sim_core::DaySpan::new(90),
        },
    )
    .unwrap();
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    issue(&mut w, &c, GoodKind::Food, 1);
    for _ in 0..45 {
        w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
        w.validate(&c).unwrap();
        let p = w.player_country();
        assert!(
            p.accounts.treasury.0
                >= p.committed_cash.0 + p.regional_protected_cash().0 + p.retained_cash().0
        );
    }
    let p = w.player_country();
    assert!(p.regional_protected_cash().0 > 0);
    assert!(p.retained_cash().0 > 0);
    assert!(p.reserves.as_ref().unwrap().food.purchased.units() > 0);
    assert!(p.projects.iter().any(|p| p.policy == "energy"));
    assert_eq!(
        p.committed_cash.0,
        p.projects.iter().map(|p| p.reserved.0).sum::<i64>()
            + p.reserves.as_ref().unwrap().reserved_cash().0
    );
    let h = w.hash();
    assert!(
        w.issue(
            &c,
            Action::ProcureReserve {
                good: GoodKind::Food,
                days: sim_core::NeedDays::new(3)
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), h);
    verify(&w, &c);
}
