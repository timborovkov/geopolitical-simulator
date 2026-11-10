use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-business-costs-test.json"
    ))
    .unwrap()
}
fn world(c: &Content) -> World {
    World::new(c, CountryId("AUR".into()), 17).unwrap()
}
fn detail(w: &World, c: &Content) -> BusinessProfitDetail {
    w.business_profit_report(c, Viewer::Government(w.player.clone()), &w.player)
        .unwrap()
        .owned
        .unwrap()
}
fn bridge(w: &World, c: &Content) {
    w.validate(c).unwrap();
    let d = detail(w, c);
    let a = &d.cumulative;
    assert_eq!(
        a.result().unwrap().0,
        a.cash_change.0 + a.distributions.0 + a.inventory_value.0 + a.receivables.0
            - a.deferred_capital.0
    );
    assert_eq!(
        w.player_country().accounts.total(),
        w.player_country().opening_money.0
    );
}
#[test]
fn cost_books_change_no_cash_goods_income_or_payouts_and_resume_after_ledger_expiry() {
    let c = content();
    let mut value = serde_json::to_value(&c).unwrap();
    value["schema"] = 36.into();
    value["rules"] = 36.into();
    value.as_object_mut().unwrap().remove("business_costs");
    let old = Content::from_json(&value.to_string()).unwrap();
    let mut a = World::new_scenario(
        &c,
        CountryId("AUR".into()),
        17,
        Scenario::FiscalSupplyCrisis,
    )
    .unwrap();
    let mut b = World::new_scenario(
        &old,
        CountryId("AUR".into()),
        17,
        Scenario::FiscalSupplyCrisis,
    )
    .unwrap();
    a.advance(&c, DaySpan::new(29)).unwrap();
    b.advance(&old, DaySpan::new(29)).unwrap();
    let bytes = persistence::encode(&a, &c).unwrap();
    let mut loaded = persistence::decode(&bytes, &c).unwrap();
    a.advance(&c, DaySpan::new(66)).unwrap();
    loaded.advance(&c, DaySpan::new(66)).unwrap();
    b.advance(&old, DaySpan::new(66)).unwrap();
    assert_eq!(a, loaded);
    assert_eq!(a.production, b.production);
    assert_eq!(a.ledger, b.ledger);
    for (id, country) in &a.countries {
        let other = &b.countries[id];
        assert_eq!(country.accounts, other.accounts);
        assert_eq!(country.report, other.report);
        assert_eq!(country.business, other.business);
        assert_eq!(country.households, other.households);
        assert_eq!(country.food_stock, other.food_stock);
        assert_eq!(country.energy_stock, other.energy_stock);
    }
    bridge(&a, &c);
    assert!(detail(&a, &c).cumulative.distributions.0 > 0);
    assert_eq!(a, persistence::replay(&a, &c).unwrap());
    assert_eq!(persistence::snapshot_schema(&a), 38);
    let before = persistence::encode(&a, &c).unwrap();
    for viewer in [
        Viewer::Observer,
        Viewer::Government(CountryId("BOR".into())),
    ] {
        let r = a.business_profit_report(&c, viewer, &a.player).unwrap();
        assert!(r.owned.is_none());
        assert!(
            !serde_json::to_string(&r)
                .unwrap()
                .contains("inventory_value")
        );
    }
    assert!(
        a.business_profit_report(
            &c,
            Viewer::Government(a.player.clone()),
            &CountryId("BOR".into())
        )
        .unwrap()
        .owned
        .is_none()
    );
    assert_eq!(before, persistence::encode(&a, &c).unwrap());
}
#[test]
fn unsold_stock_carries_actual_cost_and_spoilage_expenses_it_without_market_revaluation() {
    let mut c = content();
    for d in &mut c.countries {
        d.food = Percent::new(200);
        d.energy = Percent::new(200);
    }
    c.validate().unwrap();
    let mut w = world(&c);
    w.advance(&c, DaySpan::new(1)).unwrap();
    let first = detail(&w, &c).latest_costs.unwrap();
    let f = &first.food;
    assert!(f.production_cost.0 > 0);
    assert!(f.closing_value.0 > 0);
    assert!(f.physical.loss.units() > 0);
    assert!(f.lost_cost.0 > 0);
    assert_eq!(f.import_cost, Money(0));
    assert_eq!(
        f.opening_value.0 + f.production_cost.0 + f.import_cost.0,
        f.used_cost.0 + f.lost_cost.0 + f.closing_value.0
    );
    assert!(
        w.player_country()
            .markets
            .as_ref()
            .unwrap()
            .food
            .next_price_bps
            .units()
            < f.physical.price_bps.units()
    );
    w.advance(&c, DaySpan::new(1)).unwrap();
    let second = detail(&w, &c).latest_costs.unwrap();
    assert_eq!(second.food.opening_value, first.food.closing_value);
    assert_eq!(second.energy.opening_value, first.energy.closing_value);
    assert_eq!(
        second.food.physical.opening_stock,
        first.food.physical.closing_stock
    );
    bridge(&w, &c);
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
}
#[test]
fn proposal_administration_and_paid_capital_are_earned_at_different_stages() {
    let c = content();
    let mut w = world(&c);
    w.issue(
        &c,
        Action::Enact {
            policy: "welfare".into(),
            value: 6,
        },
    )
    .unwrap();
    let p = w.player_country().projects[0].clone();
    let fee = (p.cost.0 / 10).max(1);
    let capital = p.cost.0 - fee;
    let a = detail(&w, &c);
    assert_eq!(a.cumulative.public_cash, Money(fee));
    assert_eq!(a.cumulative.revenue().unwrap(), Money(fee));
    assert_eq!(a.cumulative.deferred_capital, Money(0));
    w.advance(&c, DaySpan::new(p.consent_day.units())).unwrap();
    let a = detail(&w, &c);
    assert_eq!(a.cumulative.deferred_capital, Money(capital));
    assert_eq!(a.pending_capital[0].commitment, p.id);
    assert_eq!(a.delivered_contract_capital, Money(0));
    w.advance(&c, DaySpan::new(p.due_day.units() - w.day.units() - 1))
        .unwrap();
    let before = detail(&w, &c);
    assert_eq!(before.cumulative.deferred_capital, Money(capital));
    w.advance(&c, DaySpan::new(1)).unwrap();
    let after = detail(&w, &c);
    assert_eq!(w.player_country().welfare, 6);
    assert_eq!(after.cumulative.deferred_capital, Money(0));
    assert_eq!(after.delivered_contract_capital, Money(capital));
    assert_eq!(
        after.current_day.revenue().unwrap().0,
        after.current_day.sales.0
            + after.current_day.public_cash.0
            + after.current_day.receivables.0
            + capital
    );
    bridge(&w, &c);
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
}
#[test]
fn immediate_consent_and_funded_withdrawal_preserve_advances_and_sunk_termination_income() {
    let c = content();
    let mut fast = world(&c);
    fast.issue(
        &c,
        Action::Enact {
            policy: "coalition".into(),
            value: 1,
        },
    )
    .unwrap();
    let p = fast.player_country().projects[0].clone();
    assert_eq!(p.stage, PolicyStage::Implementing);
    let capital = p.cost.0 - (p.cost.0 / 10).max(1);
    let a = detail(&fast, &c);
    assert_eq!(a.cumulative.public_cash, p.cost);
    assert_eq!(a.cumulative.deferred_capital, Money(capital));
    assert_eq!(a.cumulative.result().unwrap(), Money(p.cost.0 - capital));
    let cash = fast.player_country().accounts.clone();
    fast.issue(&c, Action::CancelCommitment { request: p.id })
        .unwrap();
    let a = detail(&fast, &c);
    assert_eq!(fast.player_country().accounts, cash);
    assert_eq!(a.cumulative.deferred_capital, Money(0));
    assert_eq!(a.retained_withdrawal_capital, Money(capital));
    assert_eq!(a.delivered_contract_capital, Money(0));
    bridge(&fast, &c);
    let mut pending = world(&c);
    pending
        .issue(
            &c,
            Action::Enact {
                policy: "welfare".into(),
                value: 6,
            },
        )
        .unwrap();
    let request = pending.player_country().projects[0].id;
    pending
        .issue(&c, Action::AmendCommitment { request, value: 7 })
        .unwrap();
    pending
        .issue(&c, Action::CancelCommitment { request })
        .unwrap();
    let a = detail(&pending, &c);
    assert_eq!(a.retained_withdrawal_capital, Money(0));
    assert_eq!(a.cumulative.deferred_capital, Money(0));
    assert_eq!(a.cumulative.revenue().unwrap(), a.cumulative.public_cash);
    bridge(&pending, &c);
    assert_eq!(fast, persistence::replay(&fast, &c).unwrap());
    assert_eq!(pending, persistence::replay(&pending, &c).unwrap());
}
#[test]
fn maintenance_receivable_collection_is_not_a_second_sale() {
    // Authored high-spending/low-tax exercise; all actual withholding, programme
    // consent, cash retention, arrears and collection use ordinary game rules.
    let mut c = content();
    for d in &mut c.countries {
        d.gdp = AnnualGdp::new(1_000_000).unwrap();
        d.services = Percent::new(18);
        d.welfare = Percent::new(12);
        d.tax = Percent::new(20);
        d.food = Percent::new(200);
        d.energy = Percent::new(100);
    }
    c.policies
        .iter_mut()
        .find(|p| p.id == "energy")
        .unwrap()
        .days = DaySpan::new(1);
    c.validate().unwrap();
    let mut w = world(&c);
    w.advance(&c, DaySpan::new(30)).unwrap();
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    for _ in 0..300 {
        if w.player_country().energy_capacity > 100
            && (2_000..=10_000).contains(&w.player_country().available_cash().0)
        {
            break;
        }
        w.advance(&c, DaySpan::new(1)).unwrap();
    }
    assert!(w.player_country().energy_capacity > 100);
    let target = w.player_country().available_cash();
    assert!(
        (2_000..=10_000).contains(&target.0),
        "cash target {target:?}"
    );
    let request = CashHoldId(w.next_command.0);
    w.issue(
        &c,
        Action::HoldCash {
            target,
            days: DaySpan::new(10),
        },
    )
    .unwrap();
    w.advance(&c, DaySpan::new(1)).unwrap();
    let before = detail(&w, &c);
    assert!(before.cumulative.receivables.0 > 0);
    w.issue(&c, Action::ReleaseCash { request }).unwrap();
    w.advance(&c, DaySpan::new(1)).unwrap();
    let after = detail(&w, &c);
    assert_eq!(after.cumulative.receivables, Money(0));
    assert!(after.current_day.receivables.0 < 0);
    assert_eq!(
        after.current_day.revenue().unwrap().0,
        after.current_day.sales.0 + after.current_day.public_cash.0
            - before.cumulative.receivables.0
    );
    bridge(&w, &c);
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
}
#[test]
fn malformed_cost_receipts_or_opening_accounts_refuse_commands_and_ticks_atomically() {
    let c = content();
    let mut invalid = c.clone();
    let rules = invalid.business_costs.as_mut().unwrap();
    rules.food_factor_bps = BasisPoints::new(u32::MAX);
    rules.energy_factor_bps = BasisPoints::new(u32::MAX);
    assert!(invalid.validate().is_err());
    let mut w = world(&c);
    w.advance(&c, DaySpan::new(1)).unwrap();
    let id = w.player.clone();
    for variant in 0..8 {
        let mut forged = w.clone();
        let b = forged
            .countries
            .get_mut(&id)
            .unwrap()
            .business_costs
            .as_mut()
            .unwrap();
        match variant {
            0 => b.costs.factors_paid.0 += 1,
            1 => b.last.as_mut().unwrap().food.closing_value.0 += 1,
            2 => b.last.as_mut().unwrap().food.import_cost.0 += 1,
            3 => b.last.as_mut().unwrap().food.physical.production = Food::new(999999).unwrap(),
            4 => b.last.as_mut().unwrap().day = CampaignDay::new(0),
            5 => b.last = None,
            6 => b.day_opening.public_cash = Money(-1),
            _ => b.day_opening.distributions = Money(1),
        };
        assert!(forged.validate(&c).is_err(), "variant {variant}");
        let before = forged.clone();
        assert!(
            forged.advance(&c, DaySpan::new(1)).is_err(),
            "variant {variant}"
        );
        assert_eq!(forged, before);
        assert!(
            forged
                .issue(
                    &c,
                    Action::Enact {
                        policy: "coalition".into(),
                        value: 1
                    }
                )
                .is_err(),
            "variant {variant}"
        );
        assert_eq!(forged, before);
    }
}
#[test]
fn later_bookmarks_tiny_quantities_and_exact_bundle_identity_remain_valid() {
    let bundled = Content::business_costs_bundled().unwrap();
    assert_eq!(Content::bundled_for_hash(bundled.hash()).unwrap(), bundled);
    for tiny in [false, true] {
        let mut v = serde_json::to_value(content()).unwrap();
        v["start"] = "2001-02-10".into();
        if tiny {
            v["countries"][0]["population"] = 1000.into();
        }
        let c = Content::from_json(&v.to_string()).unwrap();
        let mut w = world(&c);
        assert!(detail(&w, &c).latest_costs.is_none());
        w.advance(&c, DaySpan::new(2)).unwrap();
        bridge(&w, &c);
        assert_eq!(w, persistence::replay(&w, &c).unwrap());
    }
}
#[test]
fn public_reserve_supply_is_not_a_private_cash_purchase_or_auxiliary_asset() {
    let c = content();
    let seed = (0..500)
        .find(|s| {
            let first = stable_hash(format!("{s}:AUR:0:supply-v2").as_bytes());
            let next = stable_hash(format!("{s}:AUR:1:supply-v2").as_bytes());
            !first.is_multiple_of(3) && next.is_multiple_of(3) && 10 + next % 16 >= 20
        })
        .unwrap();
    let mut w = World::new(&c, CountryId("AUR".into()), seed).unwrap();
    w.issue(
        &c,
        Action::ProcureReserve {
            good: GoodKind::Food,
            days: NeedDays::new(1),
        },
    )
    .unwrap();
    w.advance(&c, DaySpan::new(91)).unwrap();
    let d = detail(&w, &c);
    let f = d.latest_costs.unwrap().food;
    assert!(f.physical.reserve_release.unwrap().units() > 0);
    assert!(
        w.player_country()
            .reserves
            .as_ref()
            .unwrap()
            .food
            .purchased
            .units()
            > 0
    );
    assert_eq!(
        f.opening_value.0 + f.production_cost.0 + f.import_cost.0,
        f.used_cost.0 + f.lost_cost.0 + f.closing_value.0 + f.unstocked_import_cost.0
    );
    bridge(&w, &c);
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
}
