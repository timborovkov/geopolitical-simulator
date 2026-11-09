use sim_core::*;

#[test]
fn retained_daily_business_flows_reconcile_to_independently_observed_cash_and_wages() {
    let content = Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    let mut balances = vec![world.player_country().accounts.firms];
    let mut wages = vec![Money(0)];
    let mut consumption = vec![Money(0)];
    for _ in 0..35 {
        world.advance(&content, DaySpan::new(1)).unwrap();
        balances.push(world.player_country().accounts.firms);
        wages.push(world.player_country().report.wages);
        consumption.push(world.player_country().report.consumption);
    }
    let before = persistence::encode(&world, &content).unwrap();
    for period in [1, 7, 30] {
        let report = world
            .business_statement(
                &content,
                Viewer::Government(world.player.clone()),
                &world.player,
                DaySpan::new(period),
            )
            .unwrap();
        let owned = report.owned.unwrap();
        assert_eq!(owned.counterparties, BUSINESS_COUNTERPARTIES);
        let first = 36 - period as usize;
        assert_eq!(owned.opening_cash, balances[first - 1]);
        assert_eq!(owned.closing_cash, balances[35]);
        assert_eq!(owned.days.len(), period as usize);
        assert_eq!(
            owned.payments[0].0,
            wages[first..].iter().map(|m| m.0).sum::<i64>()
        );
        assert_eq!(
            owned.receipts[0].0,
            consumption[first..].iter().map(|m| m.0).sum::<i64>()
        );
        for row in owned.days {
            let day = row.day.units() as usize;
            assert_eq!(row.opening_cash, balances[day - 1]);
            assert_eq!(row.closing_cash, balances[day]);
            assert_eq!(row.payments[0], wages[day]);
            assert_eq!(row.receipts[0], consumption[day]);
            assert_eq!(
                row.opening_cash.0 + row.receipts.iter().map(|v| v.0).sum::<i64>()
                    - row.payments.iter().map(|v| v.0).sum::<i64>(),
                row.closing_cash.0
            );
        }
    }
    assert_eq!(before, persistence::encode(&world, &content).unwrap());
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn bookmark_and_same_day_capital_are_preserved_without_fabricated_earlier_history() {
    for text in [
        include_str!("../../../data/synthetic-test.json"),
        include_str!("../../../data/synthetic-dated-test.json"),
    ] {
        let content = Content::from_json(text).unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let opening = world.player_country().accounts.firms;
        let start = world.day;
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        let report = world
            .business_statement(
                &content,
                Viewer::Government(world.player.clone()),
                &world.player,
                DaySpan::new(30),
            )
            .unwrap();
        assert_eq!(report.first_day, start);
        let owned = report.owned.unwrap();
        assert_eq!(owned.days.len(), 1);
        assert_eq!(owned.opening_cash, opening);
        assert_eq!(owned.closing_cash, world.player_country().accounts.firms);
        assert!(owned.receipts[1].0 > 0);
        assert_eq!(owned.payments, [Money(0); 3]);
        let loaded =
            persistence::decode(&persistence::encode(&world, &content).unwrap(), &content).unwrap();
        assert_eq!(
            loaded
                .business_statement(
                    &content,
                    Viewer::Government(world.player.clone()),
                    &world.player,
                    DaySpan::new(30)
                )
                .unwrap()
                .owned,
            Some(owned)
        );
    }
}

#[test]
fn government_industrial_purchases_are_not_business_import_costs() {
    let content = Content::from_json(include_str!(
        "../../../data/synthetic-industrial-trade-test.json"
    ))
    .unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let unit = content.production.as_ref().unwrap().countries[&world.player]
        .construction_unit
        .0;
    world
        .issue(
            &content,
            Action::OrderIndustrialTrade {
                good: GoodId("tools".into()),
                direction: IndustrialTradeDirection::Import,
                quantity: ProductionQuantity(12 * unit),
            },
        )
        .unwrap();
    let order = world
        .player_country()
        .industrial_trade
        .as_ref()
        .unwrap()
        .orders[0]
        .clone();
    world
        .advance(&content, DaySpan::new(order.review_day.units() + 3))
        .unwrap();
    assert!(
        world
            .player_country()
            .industrial_trade
            .as_ref()
            .unwrap()
            .orders[0]
            .cash()
            .0
            > 0
    );
    let owned = world
        .business_statement(
            &content,
            Viewer::Government(world.player.clone()),
            &world.player,
            DaySpan::new(30),
        )
        .unwrap()
        .owned
        .unwrap();
    assert!(owned.receipts[1].0 >= order.fee.0);
    assert!(
        !owned
            .transactions
            .iter()
            .any(|e| e.reason.starts_with("Industrial import contract")
                || e.reason.starts_with("Industrial export contract"))
    );
    assert!(
        owned
            .transactions
            .iter()
            .any(|e| e.reason.starts_with("Industrial trade administration"))
    );
}

#[test]
fn foreign_and_observer_statements_omit_all_private_accounts_and_transactions() {
    let content = Content::bundled().unwrap();
    let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let foreign = CountryId("USA".into());
    for (viewer, id) in [
        (Viewer::Observer, world.player.clone()),
        (Viewer::Government(foreign.clone()), world.player.clone()),
        (Viewer::Government(world.player.clone()), foreign),
    ] {
        let report = world
            .business_statement(&content, viewer, &id, DaySpan::new(30))
            .unwrap();
        assert!(report.owned.is_none());
        let json = serde_json::to_string(&report).unwrap();
        assert!(
            !json.contains("owned") && !json.contains("cash") && !json.contains("transactions")
        );
    }
    for period in [0, 31, u32::MAX] {
        assert!(
            world
                .business_statement(
                    &content,
                    Viewer::Observer,
                    &world.player,
                    DaySpan::new(period)
                )
                .is_err()
        );
    }
    let mut other = content.clone();
    other.name.push_str(" changed");
    assert!(
        world
            .business_statement(&other, Viewer::Observer, &world.player, DaySpan::new(1))
            .is_err()
    );
}

#[test]
fn malformed_cash_statements_return_errors_without_overflow_or_mutation() {
    let content = Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
    for (from, to, amount) in [
        (Account::Households, Account::Firms, i64::MAX),
        (Account::Firms, Account::Households, i64::MAX),
        (Account::Firms, Account::Firms, 1),
        (Account::Firms, Account::External, -1),
    ] {
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.ledger.push(LedgerEntry {
            day: world.day,
            country: world.player.clone(),
            from,
            to,
            amount: Money(amount),
            reason: "Malformed test transfer".into(),
        });
        let before = world.clone();
        assert!(
            world
                .business_statement(
                    &content,
                    Viewer::Government(world.player.clone()),
                    &world.player,
                    DaySpan::new(1)
                )
                .is_err()
        );
        assert_eq!(world, before);
    }
}
