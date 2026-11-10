use sim_core::*;
fn fixture() -> (Content, World) {
    let c = Content::from_json(include_str!(
        "../../../data/synthetic-private-investment-test.json"
    ))
    .unwrap();
    let w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    (c, w)
}
#[test]
fn autonomous_equipment_is_paid_installed_and_reconciles_without_public_conversion() {
    let (c, mut w) = fixture();
    let opening = w.player_country().clone();
    w.advance(&c, DaySpan::new(360)).unwrap();
    let s = w.player_country().private_investment.as_ref().unwrap();
    assert!(
        !s.equipment.is_empty(),
        "Profitable import substitution should lead to actual investment"
    );
    let totals = s.totals().unwrap();
    assert!(totals.acquisitions.0 > 0 && totals.depreciation.0 > 0 && totals.net_assets.0 > 0);
    assert_eq!(
        w.player_country().opening_energy_capacity,
        opening.opening_energy_capacity
    );
    assert_eq!(
        w.player_country().opening_food_capacity,
        opening.opening_food_capacity
    );
    let profit = w
        .business_profit_report(&c, Viewer::Government(w.player.clone()), &w.player)
        .unwrap()
        .owned
        .unwrap()
        .cumulative;
    assert_eq!(profit.private_capital, Some(totals.clone()));
    assert_eq!(
        profit.result().unwrap().0,
        profit.cash_change.0
            + profit.distributions.0
            + profit.inventory_value.0
            + profit.receivables.0
            - profit.deferred_capital.0
            + totals.net_assets.0
    );
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
    assert_eq!(
        w,
        persistence::decode(&persistence::encode(&w, &c).unwrap(), &c).unwrap()
    );
    assert_eq!(persistence::snapshot_schema(&w), 40);
}
#[test]
fn equipment_receipts_dates_and_capital_checkpoints_refuse_atomic_mutation() {
    let (c, mut w) = fixture();
    w.advance(&c, DaySpan::new(360)).unwrap();
    assert!(
        !w.player_country()
            .private_investment
            .as_ref()
            .unwrap()
            .equipment
            .is_empty()
    );
    for variant in 0..12 {
        let mut bad = w.clone();
        let state = bad
            .countries
            .get_mut(&w.player)
            .unwrap()
            .private_investment
            .as_mut()
            .unwrap();
        match variant {
            0 => state.equipment[0].cost.0 += 1,
            1 => state.equipment[0].commissioned = state.equipment[0].acquired,
            2 => state.equipment[0].retired = state.equipment[0].commissioned,
            3 => state.as_of = CampaignDay::new(0),
            4 => state.equipment[0].id.0 += 1,
            5 => state.last_review.as_mut().unwrap().available_cash.0 += 1,
            6 => state.last_review.as_mut().unwrap().retained_earnings.0 += 1,
            7 => state.last_review.as_mut().unwrap().payment_buffer.0 += 1,
            8 => state.last_review.as_mut().unwrap().price.0 += 1,
            9 => state.last_review.as_mut().unwrap().checkpoint.cash_change.0 += 1,
            10 => state.last_review.as_mut().unwrap().distributions_before.0 += 1,
            _ => {
                bad.countries
                    .get_mut(&w.player)
                    .unwrap()
                    .business_costs
                    .as_mut()
                    .unwrap()
                    .day_opening
                    .private_capital
                    .as_mut()
                    .unwrap()
                    .net_assets
                    .0 += 1
            }
        }
        let original = bad.clone();
        assert!(bad.advance(&c, DaySpan::new(1)).is_err());
        assert_eq!(bad, original);
        assert!(
            bad.issue(
                &c,
                Action::Enact {
                    policy: "welfare".into(),
                    value: 6
                }
            )
            .is_err()
        );
        assert_eq!(bad, original);
        assert!(bad.validate(&c).is_err());
    }
}
#[test]
fn private_reports_and_legacy_bytes_do_not_gain_equipment() {
    let (c, w) = fixture();
    assert!(
        w.private_investment_report(&c, Viewer::Observer, &w.player)
            .unwrap()
            .is_none()
    );
    assert!(
        w.private_investment_report(
            &c,
            Viewer::Government(w.player.clone()),
            &CountryId("BOR".into())
        )
        .unwrap()
        .is_none()
    );
    let old = Content::business_finance_bundled().unwrap();
    let world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(world.hash(), 0xb1a34e0e7596b9a9);
    assert_eq!(Content::recommended_bundled().unwrap(), old);
    assert!(world.player_country().private_investment.is_none());
    assert!(Content::bundled_for_hash(c.hash()).is_err());
    let attributed = Content::private_investment_bundled().unwrap();
    assert_eq!(
        Content::bundled_for_hash(attributed.hash()).unwrap(),
        attributed
    );
}

#[test]
fn unfinished_client_capital_stays_reserved_during_private_investment() {
    let (c, mut w) = fixture();
    w.issue(
        &c,
        Action::Enact {
            policy: "welfare".into(),
            value: 6,
        },
    )
    .unwrap();
    w.advance(&c, DaySpan::new(30)).unwrap();
    let s = w.player_country().private_investment.as_ref().unwrap();
    let r = s.last_review.as_ref().unwrap();
    assert!(r.client_advances.0 > 0);
    let paid = if r.reason == PrivateInvestmentReason::Purchased {
        r.price.0
    } else {
        0
    };
    if paid > 0 {
        assert!(
            i128::from(r.opening_cash.0) - i128::from(paid)
                >= i128::from(r.payment_buffer.0) + i128::from(r.client_advances.0)
        );
    }
    assert_eq!(
        r.client_advances,
        w.business_profit_report(&c, Viewer::Government(w.player.clone()), &w.player)
            .unwrap()
            .owned
            .unwrap()
            .cumulative
            .deferred_capital
    );
    assert_eq!(
        r.available_cash.0,
        (r.opening_cash.0 - r.payment_buffer.0 - r.client_advances.0).max(0)
    );
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
}
