use sim_core::*;
fn fixture() -> (Content, World) {
    let c = Content::from_json(include_str!(
        "../../../data/synthetic-business-finance-test.json"
    ))
    .unwrap();
    let w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    (c, w)
}
#[test]
fn dividends_use_closed_earnings_and_preserve_both_cash_requirements() {
    let (c, mut w) = fixture();
    let mut total = 0;
    for _ in 0..4 {
        w.advance(&c, DaySpan::new(29)).unwrap();
        let prior = w
            .business_profit_report(&c, Viewer::Government(w.player.clone()), &w.player)
            .unwrap()
            .owned
            .unwrap()
            .cumulative;
        w.advance(&c, DaySpan::new(1)).unwrap();
        let b = w.player_country().business.as_ref().unwrap();
        let d = b.last_decision.as_ref().unwrap();
        let p = d.earnings_protection.as_ref().unwrap();
        assert_eq!(p.checkpoint, prior);
        assert_eq!(p.closed_day.units() + 1, d.day.units());
        assert_eq!(
            p.retained_earnings.0,
            prior.result().unwrap().0 - prior.distributions.0
        );
        assert_eq!(
            p.available_cash.0,
            (d.opening_cash.0 - d.protected_cash.0 - p.client_advances.0).max(0)
        );
        let limit = (p.retained_earnings.0.max(0) * 25 / 100).min(p.available_cash.0);
        assert_eq!(d.paid.0, limit);
        total += d.paid.0;
        w.validate(&c).unwrap();
    }
    assert!(total > 0);
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
    let bytes = persistence::encode(&w, &c).unwrap();
    assert_eq!(w, persistence::decode(&bytes, &c).unwrap());
    assert_eq!(persistence::snapshot_schema(&w), 39);
}
#[test]
fn malformed_checkpoints_refuse_advance_and_commands_atomically() {
    let (c, mut w) = fixture();
    w.advance(&c, DaySpan::new(30)).unwrap();
    for variant in 0..8 {
        let mut bad = w.clone();
        let d = bad
            .countries
            .get_mut(&w.player)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .last_decision
            .as_mut()
            .unwrap();
        match variant {
            0 => d.earnings_protection = None,
            1 => d.earnings_protection.as_mut().unwrap().retained_earnings.0 += 1,
            2 => d.earnings_protection.as_mut().unwrap().available_cash.0 += 1,
            3 => d.earnings_protection.as_mut().unwrap().client_advances = Money(-1),
            4 => d.earnings_protection.as_mut().unwrap().closed_day = d.day,
            5 => {
                d.earnings_protection
                    .as_mut()
                    .unwrap()
                    .checkpoint
                    .cash_change
                    .0 += 1
            }
            6 => {
                d.earnings_protection
                    .as_mut()
                    .unwrap()
                    .checkpoint
                    .distributions
                    .0 += 1
            }
            _ => {
                d.earnings_protection
                    .as_mut()
                    .unwrap()
                    .checkpoint
                    .food_sold_cost = Money(i64::MAX)
            }
        }
        let before = bad.clone();
        assert!(bad.validate(&c).is_err(), "{variant}");
        assert!(bad.advance(&c, DaySpan::new(1)).is_err(), "{variant}");
        assert_eq!(before, bad);
        assert!(
            bad.issue(
                &c,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1
                }
            )
            .is_err(),
            "{variant}"
        );
        assert_eq!(before, bad);
    }
}
#[test]
fn payout_checkpoint_is_private_and_current_bundle_resolves_exactly() {
    let (c, mut w) = fixture();
    w.advance(&c, DaySpan::new(30)).unwrap();
    let before = persistence::encode(&w, &c).unwrap();
    for v in [
        Viewer::Observer,
        Viewer::Government(CountryId("BOR".into())),
    ] {
        let r = w.business_ownership_report(&c, v, &w.player).unwrap();
        assert!(r.owned.is_none());
        assert!(
            !serde_json::to_string(&r)
                .unwrap()
                .contains("earnings_protection")
        );
    }
    assert_eq!(before, persistence::encode(&w, &c).unwrap());
    let bundled = Content::business_finance_bundled().unwrap();
    assert_eq!(bundled, Content::bundled_for_hash(bundled.hash()).unwrap());
    assert_eq!(Content::recommended_bundled().unwrap(), bundled);
    let mut missing = c.clone();
    missing.business_finance = None;
    assert!(missing.validate().is_err());
    let mut old = Content::business_costs_bundled().unwrap();
    old.business_finance = c.business_finance;
    assert!(old.validate().is_err());
}
#[test]
fn funded_undelivered_contract_is_reserved_until_delivery_or_withdrawal() {
    let (c, mut w) = fixture();
    w.issue(
        &c,
        Action::Enact {
            policy: "welfare".into(),
            value: 6,
        },
    )
    .unwrap();
    w.advance(&c, DaySpan::new(29)).unwrap();
    let prior = w
        .business_profit_report(&c, Viewer::Government(w.player.clone()), &w.player)
        .unwrap()
        .owned
        .unwrap();
    assert!(prior.cumulative.deferred_capital.0 > 0);
    w.advance(&c, DaySpan::new(1)).unwrap();
    let d = w
        .player_country()
        .business
        .as_ref()
        .unwrap()
        .last_decision
        .as_ref()
        .unwrap();
    let p = d.earnings_protection.as_ref().unwrap();
    assert_eq!(p.client_advances, prior.cumulative.deferred_capital);
    assert!(d.paid.0 <= p.available_cash.0);
    let request = prior.pending_capital[0].commitment;
    w.issue(&c, Action::CancelCommitment { request }).unwrap();
    w.advance(&c, DaySpan::new(30)).unwrap();
    let p = w
        .player_country()
        .business
        .as_ref()
        .unwrap()
        .last_decision
        .as_ref()
        .unwrap()
        .earnings_protection
        .as_ref()
        .unwrap();
    assert_eq!(p.client_advances, Money(0));
    assert_eq!(p.checkpoint.deferred_capital, Money(0));
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
}
