use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-industrial-trade-test.json"
    ))
    .unwrap()
}
fn world(content: &Content) -> World {
    World::new(content, CountryId("AUR".into()), 17).unwrap()
}
fn order(
    world: &mut World,
    content: &Content,
    good: &str,
    direction: IndustrialTradeDirection,
    lots: u64,
) -> IndustrialTradeOrder {
    let quantity = ProductionQuantity(
        content.production.as_ref().unwrap().countries[&world.player]
            .construction_unit
            .0
            * lots,
    );
    world
        .issue(
            content,
            Action::OrderIndustrialTrade {
                good: GoodId(good.into()),
                direction,
                quantity,
            },
        )
        .unwrap();
    world
        .player_country()
        .industrial_trade
        .as_ref()
        .unwrap()
        .orders
        .last()
        .unwrap()
        .clone()
}
#[test]
fn paid_imports_use_consent_quotas_cash_and_exact_replay() {
    let content = content();
    let mut w = world(&content);
    let id = w.player.clone();
    let opening = w.player_country().accounts.treasury.0;
    let o = order(
        &mut w,
        &content,
        "tools",
        IndustrialTradeDirection::Import,
        12,
    );
    assert_eq!(w.player_country().accounts.treasury.0, opening - o.fee.0);
    assert_eq!(w.player_country().committed_cash, o.quoted_cash);
    assert!(
        w.player_country().industrial_trade.as_ref().unwrap().orders[0]
            .fills
            .is_empty()
    );
    w.validate(&content).unwrap();
    let before = persistence::encode(&w, &content).unwrap();
    let mut resumed = persistence::decode(&before, &content).unwrap();
    w.advance(
        &content,
        DaySpan::new(o.review_day.units() - w.day.units() + 4),
    )
    .unwrap();
    resumed
        .advance(&content, DaySpan::new(o.review_day.units() + 4))
        .unwrap();
    assert_eq!(w, resumed);
    let live = &w.player_country().industrial_trade.as_ref().unwrap().orders[0];
    assert!(live.filled().0 > 0);
    for fill in &live.fills {
        assert!(fill.day >= o.review_day);
        assert!(
            fill.quantity.0
                <= content
                    .industrial_trade
                    .as_ref()
                    .unwrap()
                    .daily_limit(&content, &id, &o.good, o.direction)
                    .unwrap()
                    .0
        );
    }
    assert_eq!(
        live.cash(),
        content
            .industrial_trade
            .as_ref()
            .unwrap()
            .amount(&content, &id, &o.good, o.direction, live.filled())
            .unwrap()
    );
    assert_eq!(
        w.production.as_ref().unwrap().countries[&id]
            .inventory
            .positions[&o.good]
            .totals
            .imported,
        live.filled()
    );
    assert_eq!(persistence::snapshot_schema(&w), 35);
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
    w.validate(&content).unwrap();
}
#[test]
fn cancellation_releases_only_unspent_cash_and_keeps_fee_and_partial_delivery() {
    let content = content();
    let mut w = world(&content);
    let o = order(
        &mut w,
        &content,
        "tools",
        IndustrialTradeDirection::Import,
        12,
    );
    w.advance(&content, DaySpan::new(o.review_day.units()))
        .unwrap();
    let filled = w.player_country().industrial_trade.as_ref().unwrap().orders[0].clone();
    assert!(filled.filled().0 > 0 && filled.filled().0 < o.quantity.0);
    let treasury = w.player_country().accounts.treasury;
    w.issue(&content, Action::CancelIndustrialTrade { request: o.id })
        .unwrap();
    assert_eq!(w.player_country().accounts.treasury, treasury);
    let cancelled = &w.player_country().industrial_trade.as_ref().unwrap().orders[0];
    assert_eq!(cancelled.status, IndustrialTradeStatus::Cancelled);
    assert_eq!(cancelled.reserved, Money(0));
    assert_eq!(cancelled.fills, filled.fills);
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
    let snapshot = w.clone();
    assert!(
        w.issue(&content, Action::CancelIndustrialTrade { request: o.id })
            .is_err()
    );
    assert_eq!(w, snapshot);
}
#[test]
fn export_receipts_are_asset_sales_and_preserve_total_money() {
    let content = content();
    let mut w = world(&content);
    let id = w.player.clone();
    let total = w.player_country().accounts.checked_total();
    let o = order(
        &mut w,
        &content,
        "biomass",
        IndustrialTradeDirection::Export,
        1,
    );
    assert_eq!(w.player_country().committed_cash, Money(0));
    w.advance(&content, DaySpan::new(o.review_day.units() + 3))
        .unwrap();
    let live = &w.player_country().industrial_trade.as_ref().unwrap().orders[0];
    assert!(live.filled().0 > 0 && live.cash().0 > 0);
    assert_eq!(
        w.player_country().budget.industrial_exports,
        Some(live.cash())
    );
    assert_eq!(w.player_country().accounts.checked_total(), total);
    assert_eq!(
        w.production.as_ref().unwrap().countries[&id]
            .inventory
            .positions[&o.good]
            .totals
            .exported,
        live.filled()
    );
    assert_eq!(
        w.player_country().budget.expected_cash(),
        w.player_country().accounts.treasury.0
    );
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}
#[test]
fn closed_history_does_not_block_future_orders_and_foreign_reports_hide_contracts() {
    let mut content = content();
    content.industrial_trade.as_mut().unwrap().maximum_orders = 1;
    content.validate().unwrap();
    let mut w = world(&content);
    for _ in 0..3 {
        let o = order(
            &mut w,
            &content,
            "tools",
            IndustrialTradeDirection::Import,
            1,
        );
        w.issue(&content, Action::CancelIndustrialTrade { request: o.id })
            .unwrap();
    }
    assert_eq!(
        w.player_country()
            .industrial_trade
            .as_ref()
            .unwrap()
            .orders
            .len(),
        3
    );
    w.validate(&content).unwrap();
    let foreign = content
        .countries
        .iter()
        .find(|c| c.id != w.player)
        .unwrap()
        .id
        .clone();
    assert!(
        w.industrial_trade_report(&content, Viewer::Government(w.player.clone()), &foreign)
            .unwrap()
            .unwrap()
            .owned
            .is_none()
    );
    let snapshot = persistence::encode(&w, &content).unwrap();
    w.industrial_trade_report(&content, Viewer::Government(w.player.clone()), &w.player)
        .unwrap();
    assert_eq!(snapshot, persistence::encode(&w, &content).unwrap());
}
#[test]
fn forged_trade_history_and_cash_are_refused_without_advancing() {
    let content = content();
    let mut w = world(&content);
    let o = order(
        &mut w,
        &content,
        "tools",
        IndustrialTradeDirection::Import,
        12,
    );
    w.advance(&content, DaySpan::new(o.review_day.units() + 1))
        .unwrap();
    w.validate(&content).unwrap();
    for field in 0..6 {
        let mut forged = w.clone();
        let trade = forged
            .countries
            .get_mut(&w.player)
            .unwrap()
            .industrial_trade
            .as_mut()
            .unwrap();
        match field {
            0 => trade.orders[0].fills[0].cash.0 += 1,
            1 => trade.orders[0].reserved.0 += 1,
            2 => trade.orders[0].fills[0].quantity.0 += 1,
            3 => trade.orders.clear(),
            4 => trade.orders[0].review_day = CampaignDay::new(0),
            _ => {
                let mut fill = trade.orders[0].fills[0].clone();
                fill.cash = Money(i64::MAX);
                let day = fill.day;
                trade.orders[0].fills = vec![fill.clone(), fill];
                assert_eq!(
                    trade.daily_cash(day, IndustrialTradeDirection::Import),
                    Money(i64::MAX)
                );
            }
        }
        assert!(forged.validate(&content).is_err());
        let before = forged.clone();
        assert!(forged.advance(&content, DaySpan::new(1)).is_err());
        assert_eq!(forged, before);
    }
}
#[test]
fn prices_have_a_spread_and_cumulative_rounding_cannot_make_cash() {
    let content = content();
    let w = world(&content);
    let definition = content.industrial_trade.as_ref().unwrap();
    for good in definition.goods.keys() {
        let mut previous = 0;
        for q in [0, 1, 2, 7, 20, 100, 1000] {
            let ask = definition
                .amount(
                    &content,
                    &w.player,
                    good,
                    IndustrialTradeDirection::Import,
                    ProductionQuantity(q),
                )
                .unwrap()
                .0;
            let bid = definition
                .amount(
                    &content,
                    &w.player,
                    good,
                    IndustrialTradeDirection::Export,
                    ProductionQuantity(q),
                )
                .unwrap()
                .0;
            assert!(ask >= bid && ask >= previous);
            previous = ask;
        }
    }
    let mut bad = content.clone();
    bad.production.as_mut().unwrap().countries.clear();
    assert!(bad.validate().is_err());
    let good = GoodId("tools".into());
    let before = w.clone();
    assert!(
        w.preview_industrial_trade(
            &content,
            &w.player,
            &good,
            IndustrialTradeDirection::Import,
            ProductionQuantity(u64::MAX)
        )
        .is_err()
    );
    assert_eq!(w, before);
}

#[test]
fn imports_relieve_a_paid_material_blocked_programme_without_creating_domestic_output() {
    let mut content = content();
    let id = CountryId("AUR".into());
    content
        .production
        .as_mut()
        .unwrap()
        .countries
        .get_mut(&id)
        .unwrap()
        .installed
        .insert(SectorId("construction".into()), ProductionBatches(0));
    content.validate().unwrap();
    let mut w = world(&content);
    w.advance(&content, DaySpan::new(30)).unwrap();
    w.issue(
        &content,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    let project = w.player_country().projects[0].clone();
    w.advance(&content, project.due_day.elapsed_since(w.day).unwrap())
        .unwrap();
    assert!(
        w.player_country()
            .projects
            .iter()
            .any(|p| p.id == project.id && p.stage == PolicyStage::Implementing)
    );
    let mut control = w.clone();
    let o = order(
        &mut w,
        &content,
        "assembly_packages",
        IndustrialTradeDirection::Import,
        100,
    );
    let days = o.review_day.units() - w.day.units() + 19;
    w.advance(&content, DaySpan::new(days)).unwrap();
    control.advance(&content, DaySpan::new(days)).unwrap();
    assert!(
        !w.player_country()
            .projects
            .iter()
            .any(|p| p.id == project.id)
    );
    assert!(
        control
            .player_country()
            .projects
            .iter()
            .any(|p| p.id == project.id)
    );
    assert!(w.player_country().energy_capacity > control.player_country().energy_capacity);
    assert_eq!(
        w.production.as_ref().unwrap().countries[&id]
            .inventory
            .positions[&o.good]
            .totals
            .produced,
        ProductionQuantity(0)
    );
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}
#[test]
fn industrial_inputs_arrive_after_production_and_supply_the_following_day() {
    let mut content = content();
    let id = CountryId("AUR".into());
    let opening = content
        .production
        .as_mut()
        .unwrap()
        .countries
        .get_mut(&id)
        .unwrap();
    opening
        .stocks
        .insert(GoodId("industrial_fuel".into()), ProductionQuantity(0));
    // Every recipe needs fuel, including extraction: this is a genuine opening-stock bottleneck.
    content.validate().unwrap();
    let mut w = world(&content);
    let o = order(
        &mut w,
        &content,
        "industrial_fuel",
        IndustrialTradeDirection::Import,
        12,
    );
    w.advance(&content, DaySpan::new(o.review_day.units()))
        .unwrap();
    let last = w.production.as_ref().unwrap().countries[&id]
        .last
        .as_ref()
        .unwrap();
    assert!(last.request.imports[&o.good].0 > 0);
    assert!(
        last.physical
            .production
            .sectors
            .iter()
            .all(|s| s.produced.0 == 0)
    );
    w.advance(&content, DaySpan::new(1)).unwrap();
    assert!(
        w.production.as_ref().unwrap().countries[&id]
            .last
            .as_ref()
            .unwrap()
            .physical
            .production
            .sectors
            .iter()
            .any(|s| s.produced.0 > 0)
    );
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}
#[test]
fn blocked_orders_expire_and_release_unspent_cash() {
    let mut content = content();
    let id = CountryId("AUR".into());
    let good = GoodId("tools".into());
    let production = content.production.as_mut().unwrap();
    for recipe in production.framework.sectors.values_mut() {
        recipe.inputs_per_batch.remove(&good);
    }
    production
        .countries
        .get_mut(&id)
        .unwrap()
        .stocks
        .insert(good.clone(), ProductionQuantity(300));
    production
        .countries
        .get_mut(&id)
        .unwrap()
        .storage
        .get_mut(&good)
        .unwrap()
        .loss_bps = BasisPoints::new(0);
    content.validate().unwrap();
    let mut w = world(&content);
    let o = order(
        &mut w,
        &content,
        "tools",
        IndustrialTradeDirection::Import,
        1,
    );
    w.advance(&content, DaySpan::new(o.expiry_day.units() + 1))
        .unwrap();
    let live = &w.player_country().industrial_trade.as_ref().unwrap().orders[0];
    assert_eq!(live.status, IndustrialTradeStatus::Expired);
    assert_eq!(live.filled(), ProductionQuantity(0));
    assert_eq!(live.reserved, Money(0));
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}

#[test]
fn trade_consent_is_rechecked_and_refusal_releases_purchase_cash() {
    let mut content = content();
    content
        .countries
        .iter_mut()
        .find(|c| c.id.0 == "AUR")
        .unwrap()
        .government
        .consent_days = DaySpan::new(2);
    content.validate().unwrap();
    let mut w = world(&content);
    let o = order(
        &mut w,
        &content,
        "tools",
        IndustrialTradeDirection::Import,
        1,
    );
    w.advance(&content, DaySpan::new(1)).unwrap();
    w.issue_developer(
        &content,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::LegislativeSupport,
            value: 0,
        },
    )
    .unwrap();
    w.advance(&content, o.review_day.elapsed_since(w.day).unwrap())
        .unwrap();
    let live = &w.player_country().industrial_trade.as_ref().unwrap().orders[0];
    assert!(matches!(live.status, IndustrialTradeStatus::Refused { .. }));
    assert_eq!(live.reserved, Money(0));
    assert!(live.fills.is_empty());
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}

#[test]
fn export_cash_crosses_calendar_budgets_without_becoming_forecast_tax_income() {
    let mut content = content();
    let d = content
        .countries
        .iter_mut()
        .find(|c| c.id.0 == "COR")
        .unwrap();
    d.tax = Percent::new(45);
    d.services = Percent::new(4);
    d.welfare = Percent::new(0);
    content.validate().unwrap();
    let mut w = World::new(&content, CountryId("COR".into()), 17).unwrap();
    w.advance(&content, DaySpan::new(350)).unwrap();
    let o = order(
        &mut w,
        &content,
        "biomass",
        IndustrialTradeDirection::Export,
        100,
    );
    w.advance(
        &content,
        DaySpan::new(o.expiry_day.units() + 1 - w.day.units()),
    )
    .unwrap();
    assert!(w.player_country().budget.industrial_exports.unwrap().0 > 0);
    assert!(
        w.player_country()
            .budget_reviews
            .last()
            .unwrap()
            .period
            .industrial_exports
            .unwrap()
            .0
            > 0
    );
    assert_eq!(
        w.player_country().budget.expected_cash(),
        w.player_country().accounts.treasury.0
    );
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
    let saved = persistence::encode(&w, &content).unwrap();
    let mut resumed = persistence::decode(&saved, &content).unwrap();
    w.advance(&content, DaySpan::new(3)).unwrap();
    resumed.advance(&content, DaySpan::new(3)).unwrap();
    assert_eq!(w, resumed);
}
#[test]
fn trade_starters_preserve_all_predecessor_opening_assets_and_reject_unattributed_markets() {
    let old = Content::industry_lifecycle_bundled().unwrap();
    let new = Content::industrial_trade_bundled().unwrap();
    let id = CountryId("GBR".into());
    let expected = serde_json::to_value(World::new(&old, id.clone(), 17).unwrap()).unwrap();
    let mut actual = serde_json::to_value(World::new(&new, id, 17).unwrap()).unwrap();
    actual["rules"] = expected["rules"].clone();
    actual["content_hash"] = expected["content_hash"].clone();
    for country in actual["countries"].as_object_mut().unwrap().values_mut() {
        country.as_object_mut().unwrap().remove("industrial_trade");
        country["budget"]
            .as_object_mut()
            .unwrap()
            .remove("industrial_exports");
    }
    assert_eq!(actual, expected);
    for mutation in 0..4 {
        let mut bad = new.clone();
        let definition = bad.industrial_trade.as_mut().unwrap();
        match mutation {
            0 => definition.source = "missing".into(),
            1 => definition.price_lots = 0,
            2 => {
                definition
                    .goods
                    .get_mut(&GoodId("tools".into()))
                    .unwrap()
                    .bid_bps = BasisPoints::new(10)
            }
            _ => definition
                .goods
                .insert(
                    GoodId("contracted_services".into()),
                    definition.goods[&GoodId("tools".into())].clone(),
                )
                .map(|_| ())
                .unwrap_or(()),
        }
        assert!(bad.validate().is_err());
    }
}

#[test]
fn ai_imports_observed_inputs_through_ordinary_paid_contracts() {
    let mut content = content();
    let foreign = CountryId("COR".into());
    content
        .production
        .as_mut()
        .unwrap()
        .countries
        .get_mut(&foreign)
        .unwrap()
        .stocks
        .insert(GoodId("industrial_fuel".into()), ProductionQuantity(0));
    let d = content
        .countries
        .iter_mut()
        .find(|c| c.id == foreign)
        .unwrap();
    d.tax = Percent::new(45);
    d.services = Percent::new(4);
    d.welfare = Percent::new(0);
    d.energy = Percent::new(120);
    d.food = Percent::new(120);
    d.capacity = Percent::new(100);
    content.validate().unwrap();
    let mut w = world(&content);
    w.advance(&content, DaySpan::new(35)).unwrap();
    let orders = &w.countries[&foreign]
        .industrial_trade
        .as_ref()
        .unwrap()
        .orders;
    assert!(
        orders
            .iter()
            .any(|o| o.direction == IndustrialTradeDirection::Import
                && o.good.0 == "industrial_fuel"
                && o.filled().0 > 0)
    );
    assert!(w.journal.iter().any(|e| e.ai
        && e.command.issuer == foreign
        && matches!(e.command.action, Action::OrderIndustrialTrade { .. })));
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}

#[test]
fn ai_batches_observed_surplus_so_sale_quotes_cover_nonrefundable_administration() {
    let mut content = content();
    let foreign = CountryId("COR".into());
    let d = content
        .countries
        .iter_mut()
        .find(|c| c.id == foreign)
        .unwrap();
    d.tax = Percent::new(45);
    d.services = Percent::new(4);
    d.welfare = Percent::new(0);
    d.energy = Percent::new(120);
    d.food = Percent::new(120);
    d.capacity = Percent::new(100);
    content.validate().unwrap();
    let mut w = world(&content);
    w.advance(&content, DaySpan::new(70)).unwrap();
    let sales = w.countries[&foreign]
        .industrial_trade
        .as_ref()
        .unwrap()
        .orders
        .iter()
        .filter(|o| o.direction == IndustrialTradeDirection::Export)
        .collect::<Vec<_>>();
    assert!(!sales.is_empty());
    assert!(sales.iter().all(|o| o.quoted_cash.0 > o.fee.0));
    assert!(sales.iter().any(|o| o.cash().0 > o.fee.0));
    w.validate(&content).unwrap();
    assert_eq!(persistence::replay(&w, &content).unwrap(), w);
}
