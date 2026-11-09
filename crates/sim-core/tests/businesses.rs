use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!("../../../data/synthetic-businesses-test.json")).unwrap()
}
fn world(content: &Content) -> World {
    World::new(content, CountryId("AUR".into()), 17).unwrap()
}
#[test]
fn autonomous_distributions_conserve_cash_enter_tax_and_resume_exactly() {
    let content = content();
    let mut w = world(&content);
    w.advance(&content, DaySpan::new(29)).unwrap();
    assert!(
        w.player_country()
            .business
            .as_ref()
            .unwrap()
            .last_decision
            .is_none()
    );
    let bytes = persistence::encode(&w, &content).unwrap();
    let mut restored = persistence::decode(&bytes, &content).unwrap();
    w.advance(&content, DaySpan::new(91)).unwrap();
    restored.advance(&content, DaySpan::new(91)).unwrap();
    assert_eq!(w, restored);
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
    let b = w.player_country().business.as_ref().unwrap();

    assert!(b.distributions.0 > 0);
    let decision = b.last_decision.as_ref().unwrap();
    assert_eq!(decision.day, CampaignDay::new(120));
    assert!(
        decision.opening_cash.0 - decision.paid.0 >= decision.protected_cash.0
            || decision.paid.0 == 0
    );
    let c = w.player_country();
    assert_eq!(
        c.income_tax_report
            .as_ref()
            .unwrap()
            .groups
            .iter()
            .map(|g| g.assessment.gross.0)
            .sum::<i64>(),
        c.report.wages.0 + decision.paid.0
    );
    assert_eq!(
        b.opening_cash.0 + b.operating_receipts.0 + b.public_receipts.0 - b.payments.0,
        c.accounts.firms.0
    );
    assert_eq!(c.opening_money.0, c.accounts.total());
    let actual = w
        .ledger
        .iter()
        .filter(|e| {
            e.country == w.player
                && e.day == w.day
                && e.reason == "Autonomous business owner cash distribution"
        })
        .map(|e| e.amount.0)
        .sum::<i64>();
    assert_eq!(actual, decision.paid.0);
    assert!(
        w.player_country()
            .households
            .iter()
            .all(|h| h.tax.0 <= h.income.0)
    );
}
#[test]
fn government_capital_is_excluded_and_public_assets_keep_their_identity() {
    let content = content();
    let mut w = world(&content);
    let opening = w.production.clone();
    w.issue(
        &content,
        Action::Enact {
            policy: "income_tax".into(),
            value: w.player_country().tax + 1,
        },
    )
    .unwrap();
    let c = w.player_country();
    let b = c.business.as_ref().unwrap();
    assert!(b.public_receipts.0 > 0);
    assert_eq!(b.undistributed_cash(), Money(0));
    assert_eq!(w.production, opening);
    assert_eq!(b.opening_cash.0 + b.public_receipts.0, c.accounts.firms.0);
    w.advance(&content, DaySpan::new(90)).unwrap();
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}
#[test]
fn private_projection_and_version_boundaries_reject_forged_accounts() {
    let content = content();
    let mut w = world(&content);
    w.advance(&content, DaySpan::new(30)).unwrap();
    let bytes = persistence::encode(&w, &content).unwrap();
    assert_eq!(persistence::snapshot_schema(&w), 36);
    for viewer in [
        Viewer::Observer,
        Viewer::Government(CountryId("BOR".into())),
    ] {
        let report = w
            .business_ownership_report(&content, viewer, &w.player)
            .unwrap();
        assert!(report.owned.is_none());
        assert!(
            !serde_json::to_string(&report)
                .unwrap()
                .contains("operating_receipts")
        );
    }
    assert!(
        w.business_ownership_report(
            &content,
            Viewer::Government(w.player.clone()),
            &CountryId("BOR".into())
        )
        .unwrap()
        .owned
        .is_none()
    );
    assert_eq!(persistence::encode(&w, &content).unwrap(), bytes);
    let id = w.player.clone();
    for variant in 0..7 {
        let mut forged = w.clone();
        let c = forged.countries.get_mut(&id).unwrap();
        let b = c.business.as_mut().unwrap();
        match variant {
            0 => b.operating_receipts.0 += 1,
            1 => b.id = BusinessId("wrong-enterprise".into()),
            2 => b.last_decision.as_mut().unwrap().owners[0].amount.0 += 1,
            3 => {
                b.last_decision.as_mut().unwrap().reason =
                    if b.last_decision.as_ref().unwrap().paid.0 > 0 {
                        BusinessDistributionReason::OperatingLoss
                    } else {
                        BusinessDistributionReason::Paid
                    }
            }
            4 => {
                b.last_decision
                    .as_mut()
                    .unwrap()
                    .operating_receipts_before
                    .0 += 1
            }
            5 => b.last_decision.as_mut().unwrap().day = CampaignDay::new(29),
            _ => c.business = None,
        }
        assert!(forged.validate(&content).is_err(), "forgery {variant}");
        let before = forged.clone();
        assert!(
            forged.advance(&content, DaySpan::new(1)).is_err(),
            "forgery {variant}"
        );
        assert_eq!(forged, before);
    }
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["schema"] = 35.into();
    assert!(persistence::decode(&serde_json::to_vec(&value).unwrap(), &content).is_err());
    let mut bad = content.clone();
    bad.businesses = None;
    assert!(bad.validate().is_err());
    let previous = Content::industrial_trade_bundled().unwrap();
    let old = World::new(&previous, CountryId("GBR".into()), 17).unwrap();
    assert!(old.player_country().business.is_none());
    assert!(
        !String::from_utf8(persistence::encode(&old, &previous).unwrap())
            .unwrap()
            .contains("\"business\":")
    );
    let bundled = Content::businesses_bundled().unwrap();
    assert_eq!(Content::bundled_for_hash(bundled.hash()).unwrap(), bundled);
}

#[test]
fn review_cadence_is_relative_to_later_bookmarks() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/synthetic-businesses-test.json")).unwrap();
    value["start"] = "2001-02-10".into();
    let content = Content::from_json(&value.to_string()).unwrap();
    let mut w = world(&content);
    let start = w.day;
    w.advance(&content, DaySpan::new(29)).unwrap();
    assert!(
        w.player_country()
            .business
            .as_ref()
            .unwrap()
            .last_decision
            .is_none()
    );
    w.advance(&content, DaySpan::new(1)).unwrap();
    assert_eq!(
        w.player_country()
            .business
            .as_ref()
            .unwrap()
            .last_decision
            .as_ref()
            .unwrap()
            .day
            .units(),
        start.units() + 30
    );
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}

#[test]
fn exhausted_business_counters_roll_back_complete_daily_steps() {
    let content = content();
    let mut w = world(&content);
    let id = w.player.clone();
    let b = w.countries.get_mut(&id).unwrap().business.as_mut().unwrap();
    b.operating_receipts = Money(1_000_000_000_000_000_000);
    b.payments = b.operating_receipts;
    w.validate(&content).unwrap();
    let before = w.clone();
    assert!(
        w.advance(&content, DaySpan::new(1))
            .unwrap_err()
            .contains("Business cash account")
    );
    assert_eq!(w, before);
}
