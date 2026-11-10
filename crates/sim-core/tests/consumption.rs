use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-delivered-consumption-test.json"
    ))
    .unwrap()
}
fn earlier(c: &Content) -> Content {
    let mut v = serde_json::to_value(c).unwrap();
    v["schema"] = 35.into();
    v["rules"] = 35.into();
    v.as_object_mut().unwrap().remove("consumption");
    Content::from_json(&v.to_string()).unwrap()
}
fn world(c: &Content, scenario: Scenario) -> World {
    World::new_scenario(c, CountryId("AUR".into()), 17, scenario).unwrap()
}
#[test]
fn shortages_keep_food_cash_in_each_original_group_without_repricing_or_creating_money() {
    let c = content();
    let old = earlier(&c);
    let mut w = world(&c, Scenario::FiscalSupplyCrisis);
    let mut control = world(&old, Scenario::FiscalSupplyCrisis);
    w.advance(&c, DaySpan::new(1)).unwrap();
    control.advance(&old, DaySpan::new(1)).unwrap();
    let a = w.player_country();
    let b = control.player_country();
    let kept: i64 = a
        .households
        .iter()
        .map(|h| h.delivery.as_ref().unwrap().cash_kept.0)
        .sum();
    assert!(kept > 0);
    let quote = a.markets.as_ref().unwrap().food.last.as_ref().unwrap();
    assert_eq!(Some(quote), b.markets.as_ref().unwrap().food.last.as_ref());
    assert_eq!(a.accounts.households.0 - b.accounts.households.0, kept);
    assert_eq!(b.accounts.firms.0 - a.accounts.firms.0, kept);
    assert_eq!(a.accounts.treasury, b.accounts.treasury);
    assert_eq!(a.accounts.external, b.accounts.external);
    for (h, previous) in a.households.iter().zip(&b.households) {
        let r = h.delivery.as_ref().unwrap();
        assert_eq!(h.cash.0 - previous.cash.0, r.cash_kept.0);
        assert_eq!(previous.consumption.0 - h.consumption.0, r.cash_kept.0);
        assert_eq!(h.income, previous.income);
        assert_eq!(h.tax, previous.tax);
        assert_eq!(h.food_coverage, previous.food_coverage);
    }
    assert_eq!(a.accounts.total(), a.opening_money.0);
    w.validate(&c).unwrap();
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
}
#[test]
fn fully_delivered_orders_pay_the_same_cash_as_the_preserved_basket_rule() {
    let mut c = content();
    for d in &mut c.countries {
        d.food = Percent::new(100);
        d.energy = Percent::new(100);
    }
    c.validate().unwrap();
    let old = earlier(&c);
    let mut w = world(&c, Scenario::Sandbox);
    let mut control = world(&old, Scenario::Sandbox);
    w.advance(&c, DaySpan::new(1)).unwrap();
    control.advance(&old, DaySpan::new(1)).unwrap();
    let a = w.player_country();
    assert_eq!(a.accounts, control.player_country().accounts);
    for h in &a.households {
        let r = h.delivery.as_ref().unwrap();
        assert_eq!(r.food_need, r.delivered_food);
        assert_eq!(r.cash_kept, Money(0));
        assert_eq!(r.food_paid, r.food_budget);
        assert_eq!(h.consumption, r.planned_cash);
    }
}
#[test]
fn zero_food_supply_collects_no_food_payment_and_preserves_exact_food_cash() {
    let mut c = content();
    for d in &mut c.countries {
        d.food = Percent::new(1);
    }
    c.markets
        .as_mut()
        .unwrap()
        .goods
        .iter_mut()
        .find(|g| g.kind == GoodKind::Food)
        .unwrap()
        .import_limit_bps = BasisPoints::new(0);
    c.validate().unwrap();
    // Pick a recorded supply-stream shock to exhaust a valid positive opening
    // capacity, rather than constructing an invalid zero-capacity content pack.
    let seed = (0..100)
        .find(|seed| stable_hash(format!("{seed}:AUR:0:supply-v2").as_bytes()).is_multiple_of(3))
        .unwrap();
    let mut w = World::new(&c, CountryId("AUR".into()), seed).unwrap();
    w.advance(&c, DaySpan::new(1)).unwrap();
    let a = w.player_country();
    assert_eq!(a.report.food_consumed, Food::ZERO);
    assert_eq!(a.report.food_imported, Food::ZERO);
    assert!(
        a.households
            .iter()
            .any(|h| h.delivery.as_ref().unwrap().cash_kept.0 > 0)
    );
    for h in &a.households {
        let r = h.delivery.as_ref().unwrap();
        assert_eq!(r.food_paid, Money(0));
        assert_eq!(r.cash_kept, r.food_budget);
        assert_eq!(h.consumption, r.other_paid);
        assert_eq!(r.planned_cash.0, h.consumption.0 + r.cash_kept.0);
    }
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
}
#[test]
fn receipts_resume_through_prices_owner_distributions_and_retained_ledger_expiry() {
    let c = content();
    let mut w = world(&c, Scenario::FiscalSupplyCrisis);
    w.advance(&c, DaySpan::new(29)).unwrap();
    let bytes = persistence::encode(&w, &c).unwrap();
    let mut restored = persistence::decode(&bytes, &c).unwrap();
    w.advance(&c, DaySpan::new(66)).unwrap();
    restored.advance(&c, DaySpan::new(66)).unwrap();
    assert_eq!(w, restored);
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
    assert_eq!(persistence::snapshot_schema(&w), 37);
    assert!(
        w.player_country()
            .business
            .as_ref()
            .unwrap()
            .last_decision
            .is_some()
    );
    let before = persistence::encode(&w, &c).unwrap();
    let owner = w
        .consumption_report(&c, Viewer::Government(w.player.clone()), &w.player)
        .unwrap();
    assert_eq!(owner.owned.as_ref().unwrap().groups.len(), 4);
    for viewer in [
        Viewer::Observer,
        Viewer::Government(CountryId("BOR".into())),
    ] {
        let p = w.consumption_report(&c, viewer, &w.player).unwrap();
        assert!(p.owned.is_none());
        assert!(!serde_json::to_string(&p).unwrap().contains("food_paid"));
    }
    assert!(
        w.consumption_report(
            &c,
            Viewer::Government(w.player.clone()),
            &CountryId("BOR".into())
        )
        .unwrap()
        .owned
        .is_none()
    );
    assert_eq!(persistence::encode(&w, &c).unwrap(), before);
}
#[test]
fn malformed_delivery_receipts_cannot_advance_or_change_existing_state() {
    let c = content();
    let mut w = world(&c, Scenario::FiscalSupplyCrisis);
    w.advance(&c, DaySpan::new(1)).unwrap();
    let id = w.player.clone();
    for variant in 0..9 {
        let mut forged = w.clone();
        let h = &mut forged.countries.get_mut(&id).unwrap().households[0];
        let r = h.delivery.as_mut().unwrap();
        match variant {
            0 => r.food_paid.0 += 1,
            1 => r.cash_kept.0 += 1,
            2 => r.day = CampaignDay::new(0),
            3 => r.delivered_food = Food::new(r.affordable_food.units() + 1).unwrap(),
            4 => r.planned_cash.0 += 1,
            5 => r.food_budget.0 += 1,
            6 => r.desired_spending.0 += 1,
            7 => h.delivery = None,
            _ => {
                forged
                    .countries
                    .get_mut(&id)
                    .unwrap()
                    .markets
                    .as_mut()
                    .unwrap()
                    .food
                    .next_price_bps = BasisPoints::new(0)
            }
        };
        assert!(forged.validate(&c).is_err(), "variant {variant}");
        let before = forged.clone();
        assert!(
            forged.advance(&c, DaySpan::new(1)).is_err(),
            "variant {variant}"
        );
        assert_eq!(forged, before);
    }
    let old = earlier(&c);
    let previous = world(&old, Scenario::Sandbox);
    assert!(
        previous
            .player_country()
            .households
            .iter()
            .all(|h| h.delivery.is_none())
    );
    assert!(
        !String::from_utf8(persistence::encode(&previous, &old).unwrap())
            .unwrap()
            .contains("\"delivery\":")
    );
    let bundled = Content::consumption_bundled().unwrap();
    assert_eq!(Content::bundled_for_hash(bundled.hash()).unwrap(), bundled);
}

#[test]
fn purchasing_power_shortfall_is_distinct_from_physical_supply_shortfall() {
    let mut c = content();
    for d in &mut c.countries {
        d.food = Percent::new(200);
        d.energy = Percent::new(200);
        d.welfare = Percent::new(0);
        let poor = d
            .households
            .iter()
            .position(|h| h.id == "vulnerable")
            .unwrap();
        let owners = d.households.iter().position(|h| h.id == "owners").unwrap();
        let income = d.households[poor].income_share;
        let savings = d.households[poor].savings_share;
        d.households[poor].income_share = 0;
        d.households[poor].savings_share = 0;
        d.households[owners].income_share += income;
        d.households[owners].savings_share += savings;
    }
    c.validate().unwrap();
    let mut w = world(&c, Scenario::Sandbox);
    w.advance(&c, DaySpan::new(1)).unwrap();
    let p = w
        .player_country()
        .households
        .iter()
        .find(|h| h.id == "vulnerable")
        .unwrap()
        .delivery
        .as_ref()
        .unwrap();
    assert!(p.food_need.units() > 0);
    assert!(p.desired_spending.0 > 0);
    assert_eq!(p.planned_cash, Money(0));
    assert_eq!(p.affordable_food, Food::ZERO);
    assert_eq!(p.delivered_food, Food::ZERO);
    let market = w
        .player_country()
        .markets
        .as_ref()
        .unwrap()
        .food
        .last
        .as_ref()
        .unwrap();
    assert!(market.supply.units() >= market.physical_need.units());
    assert_eq!(market.delivered, market.requested_use);
    assert!(market.requested_use.units() < market.physical_need.units());
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
}
#[test]
fn later_bookmarks_and_zero_assigned_food_units_keep_valid_receipts() {
    for tiny in [false, true] {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../data/synthetic-delivered-consumption-test.json"
        ))
        .unwrap();
        value["start"] = "2001-02-10".into();
        if tiny {
            value["countries"][0]["population"] = 1000.into();
        }
        let mut c = Content::from_json(&value.to_string()).unwrap();
        for d in &mut c.countries {
            d.food = Percent::new(200);
            d.energy = Percent::new(200);
        }
        c.validate().unwrap();
        let mut w = world(&c, Scenario::Sandbox);
        let opening = w
            .consumption_report(&c, Viewer::Government(w.player.clone()), &w.player)
            .unwrap();
        assert!(opening.owned.unwrap().groups.is_empty());
        w.advance(&c, DaySpan::new(1)).unwrap();
        if tiny {
            assert!(
                w.player_country().households.iter().any(|h| h
                    .delivery
                    .as_ref()
                    .unwrap()
                    .food_need
                    == Food::ZERO)
            );
        }
        assert!(
            w.player_country()
                .households
                .iter()
                .all(|h| h.delivery.as_ref().unwrap().day == w.day)
        );
        assert_eq!(persistence::replay(&w, &c).unwrap(), w);
    }
}

#[test]
fn an_ordinary_paid_welfare_programme_relaxes_cash_rationing_when_food_is_abundant() {
    let mut c = content();
    for d in &mut c.countries {
        d.food = Percent::new(200);
        d.energy = Percent::new(200);
        d.welfare = Percent::new(0);
        let poor = d
            .households
            .iter()
            .position(|h| h.id == "vulnerable")
            .unwrap();
        let owners = d.households.iter().position(|h| h.id == "owners").unwrap();
        let income = d.households[poor].income_share;
        let savings = d.households[poor].savings_share;
        d.households[poor].income_share = 0;
        d.households[poor].savings_share = 0;
        d.households[owners].income_share += income;
        d.households[owners].savings_share += savings;
    }
    c.validate().unwrap();
    let mut w = world(&c, Scenario::Sandbox);
    w.advance(&c, DaySpan::new(1)).unwrap();
    let poor = HouseholdGroupId::from("vulnerable");
    let before = w
        .player_country()
        .household(&poor)
        .unwrap()
        .delivery
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(before.delivered_food, Food::ZERO);
    w.issue(
        &c,
        Action::Enact {
            policy: "welfare".into(),
            value: 12,
        },
    )
    .unwrap();
    for _ in 0..200 {
        if w.player_country().welfare == 12 {
            break;
        }
        w.advance(&c, DaySpan::new(1)).unwrap();
    }
    assert_eq!(w.player_country().welfare, 12);
    let after = w
        .player_country()
        .household(&poor)
        .unwrap()
        .delivery
        .as_ref()
        .unwrap();
    assert!(after.delivered_food.units() > before.delivered_food.units());
    assert!(after.planned_cash.0 > before.planned_cash.0);
    let market = w
        .player_country()
        .markets
        .as_ref()
        .unwrap()
        .food
        .last
        .as_ref()
        .unwrap();
    assert_eq!(market.delivered, market.requested_use);
    assert!(market.supply.units() >= market.physical_need.units());
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
}
