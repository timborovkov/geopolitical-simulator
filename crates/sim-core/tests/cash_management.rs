use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-cash-management-test.json"
    ))
    .unwrap()
}
fn exhausted() -> (Content, World) {
    let mut c = fixture();
    c.finance.as_mut().unwrap().authorities.clear();
    c.finance.as_mut().unwrap().emergency = None;
    c.countries.iter_mut().for_each(|d| {
        d.services = 18;
        d.welfare = 12;
    });
    c.validate().unwrap();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    for _ in 0..180 {
        if w.player_country().available_cash().0 == 0 {
            break;
        }
        w.advance(&c, 1).unwrap();
    }
    assert_eq!(w.player_country().available_cash(), Money(0));
    (c, w)
}
fn hold(w: &mut World, c: &Content, amount: i64, days: u32) -> CashHoldId {
    let id = CashHoldId(w.next_command);
    w.issue(
        c,
        Action::HoldCash {
            target: Money(amount),
            days,
        },
    )
    .unwrap();
    id
}
fn replay(w: &World, c: &Content) {
    w.validate(c).unwrap();
    let bytes = persistence::encode(w, c).unwrap();
    assert_eq!(persistence::decode(&bytes, c).unwrap(), *w);
    assert_eq!(persistence::replay(w, c).unwrap(), *w);
}
#[test]
fn real_receipts_can_fund_a_recovery_decision_without_credit_or_forgiving_arrears() {
    let (c, mut w) = exhausted();
    assert!(w.preview_borrowing(&c, &w.player, Money(1000)).is_err());
    assert!(w.preview(&c, &w.player, "services", 6).is_err());
    let debt = w.player_country().debt;
    let approval = w.player_country().approval;
    let cash = w.player_country().accounts.treasury;
    let id = hold(&mut w, &c, 80, 90);
    assert_eq!(w.player_country().accounts.treasury, cash);
    assert_eq!(w.player_country().retained_cash(), Money(0));
    assert_eq!(w.player_country().approval, (approval - 3).max(0));
    w.advance(&c, 1).unwrap();
    assert!(w.player_country().retained_cash().0 > 0);
    assert!(w.player_country().retained_cash().0 < 80);
    assert!(
        w.preview(&c, &w.player, "services", 6).is_err(),
        "a partial target cannot fund the full programme"
    );
    let partially_funded = persistence::encode(&w, &c).unwrap();
    let mut resumed = persistence::decode(&partially_funded, &c).unwrap();
    for _ in 0..30 {
        if w.player_country().retained_cash() == Money(80) {
            break;
        }
        w.advance(&c, 1).unwrap();
        resumed.advance(&c, 1).unwrap();
    }
    assert_eq!(w, resumed);
    assert_eq!(w.player_country().retained_cash(), Money(80));
    assert!(w.player_country().arrears.0 > 0);
    assert_eq!(w.player_country().debt, debt);
    let ctry = w.player_country();
    assert!(
        ctry.regional_protected_cash().0 + ctry.committed_cash.0 + 80 <= ctry.accounts.treasury.0
    );
    let actual = ctry.accounts.treasury;
    let arrears = ctry.arrears;
    w.issue(&c, Action::ReleaseCash { request: id }).unwrap();
    assert_eq!(w.player_country().accounts.treasury, actual);
    assert_eq!(w.player_country().arrears, arrears);
    assert_eq!(w.player_country().available_cash(), Money(80));
    assert_eq!(
        w.preview(&c, &w.player, "services", 6).unwrap().cost,
        Money(80)
    );
    w.issue(
        &c,
        Action::Enact {
            policy: "services".into(),
            value: 6,
        },
    )
    .unwrap();
    assert_eq!(w.player_country().available_cash(), Money(0));
    assert_eq!(
        w.player_country().budget.expected_cash(),
        w.player_country().accounts.treasury.0
    );
    replay(&w, &c);
}
#[test]
fn reservations_and_local_claims_remain_payable_while_retention_is_active() {
    let c = fixture();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.issue(
        &c,
        Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        },
    )
    .unwrap();
    let reserved = w.player_country().committed_cash;
    assert!(reserved.0 > 0);
    hold(&mut w, &c, 1000, 90);
    let held = w.player_country().retained_cash();
    assert!(held.0 > 0);
    w.advance(&c, 14).unwrap();
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert_eq!(w.player_country().retained_cash(), held);
    assert_eq!(
        w.player_country().projects[0].stage,
        PolicyStage::Implementing
    );
    assert_eq!(
        w.player_country().commitments.as_ref().unwrap()[0].paid,
        Money(20)
    );
    assert!(
        w.player_country()
            .subnational
            .as_ref()
            .unwrap()
            .regions
            .iter()
            .all(|r| r.last.as_ref().is_some_and(|last| last.paid_base.0 > 0))
    );
    replay(&w, &c);
}
#[test]
fn expiry_cooldown_rejections_and_successor_governments_release_only_actual_cash() {
    let c = fixture();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    for action in [
        Action::HoldCash {
            target: Money(1001),
            days: 90,
        },
        Action::HoldCash {
            target: Money(1),
            days: 91,
        },
        Action::ReleaseCash {
            request: CashHoldId(999),
        },
    ] {
        let old = w.clone();
        assert!(w.issue(&c, action).is_err());
        assert_eq!(w, old);
    }
    let id = hold(&mut w, &c, 100, 2);
    let old = w.clone();
    assert!(
        w.issue(
            &c,
            Action::HoldCash {
                target: Money(1),
                days: 1
            }
        )
        .is_err()
    );
    assert_eq!(w, old);
    let mut baseline = w.clone();
    baseline
        .issue(&c, Action::ReleaseCash { request: id })
        .unwrap();
    w.advance(&c, 2).unwrap();
    baseline.advance(&c, 2).unwrap();
    assert_eq!(w.player_country().retained_cash(), Money(0));
    assert!(matches!(
        w.player_country().cash_management.as_ref().unwrap().records[0].ended,
        Some((2, CashHoldEnd::Expired))
    ));
    assert_eq!(
        w.player_country().accounts.treasury,
        baseline.player_country().accounts.treasury
    );
    assert!(w.preview_cash_hold(&c, &w.player, Money(10), 2).is_err());
    w.advance(&c, 88).unwrap();
    hold(&mut w, &c, 100, 90);
    replay(&w, &c);
    // A normal contested transition changes the sponsoring government and
    // must release retention on the same date, before further settlement.
    let mut successor = World::new(&c, CountryId("COR".into()), 17).unwrap();
    for _ in 0..2 {
        successor
            .issue(
                &c,
                Action::Enact {
                    policy: "coalition".into(),
                    value: 1,
                },
            )
            .unwrap();
        while !successor.player_country().projects.is_empty() {
            successor.advance(&c, 1).unwrap();
        }
    }
    successor
        .issue(
            &c,
            Action::Reconstitute {
                target: Regime::Parliamentary,
                contested: true,
            },
        )
        .unwrap();
    hold(&mut successor, &c, 100, 90);
    for _ in 0..90 {
        if successor.player_country().government_number > 1 {
            break;
        }
        successor.advance(&c, 1).unwrap();
    }
    assert!(successor.player_country().government_number > 1);
    assert_eq!(successor.player_country().retained_cash(), Money(0));
    assert!(matches!(
        successor
            .player_country()
            .cash_management
            .as_ref()
            .unwrap()
            .records[0]
            .ended,
        Some((_, CashHoldEnd::GovernmentChanged))
    ));
    replay(&successor, &c);
}
#[test]
fn snapshot_and_content_validation_reject_invalid_terms_and_false_cash() {
    let c = fixture();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    hold(&mut w, &c, 100, 90);
    let bytes = persistence::encode(&w, &c).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["schema"], 17);
    json["schema"] = 16.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&json).unwrap(), &c)
            .unwrap_err()
            .contains("format 17")
    );
    let mut corrupt = w.clone();
    corrupt
        .countries
        .get_mut(&w.player)
        .unwrap()
        .cash_management
        .as_mut()
        .unwrap()
        .retained = Money(101);
    assert!(corrupt.validate(&c).is_err());
    let mut corrupt = w.clone();
    corrupt
        .countries
        .get_mut(&w.player)
        .unwrap()
        .cash_management
        .as_mut()
        .unwrap()
        .records
        .clear();
    assert!(corrupt.validate(&c).is_err());
    let mut bad = c.clone();
    bad.cash_management.as_mut().unwrap().max_target_bps = 101;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.rules = 15;
    bad.schema = 15;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.cash_management = None;
    assert!(bad.validate().is_err());
    replay(&w, &c);
}

#[test]
fn deferred_matured_principal_stays_in_debt_and_is_not_operating_arrears() {
    let mut c = fixture();
    for d in &mut c.countries {
        d.services = 18;
        d.welfare = 12;
    }
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.issue(
        &c,
        Action::Borrow {
            amount: Money(1000),
        },
    )
    .unwrap();
    w.advance(&c, 193).unwrap();
    assert_eq!(w.player_country().available_cash(), Money(0));
    let id = hold(&mut w, &c, 500, 90);
    let mut baseline = w.clone();
    baseline
        .issue(&c, Action::ReleaseCash { request: id })
        .unwrap();
    let debt = w.player_country().debt;
    let previous_arrears = w.player_country().arrears;
    w.advance(&c, 1).unwrap();
    baseline.advance(&c, 1).unwrap();
    assert_eq!(w.player_country().debt, debt);
    assert_eq!(
        w.player_country()
            .financing
            .as_ref()
            .unwrap()
            .principal_due_through(w.day),
        Money(1000)
    );
    assert!(w.player_country().retained_cash().0 > 0);
    assert!(baseline.player_country().debt.0 < debt.0);
    assert!(
        w.player_country().arrears.0 - previous_arrears.0 < 1000,
        "unpaid principal must not be duplicated as operating arrears"
    );
    let before = w.hash();
    assert_eq!(
        w.funding_outlook(&c, 1, None).unwrap().available_cash,
        Money(0)
    );
    assert_eq!(w.hash(), before);
    replay(&w, &c);
}
