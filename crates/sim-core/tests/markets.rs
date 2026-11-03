use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!("../../../data/synthetic-market-test.json")).unwrap()
}
fn world(content: &Content) -> World {
    World::new(content, CountryId("AUR".into()), 17).unwrap()
}

#[test]
fn prices_respond_with_a_lag_and_reprice_baskets_without_creating_cash() {
    let content = fixture();
    let mut world = world(&content);
    let opening_cash = world.player_country().accounts.total();
    for field in [DeveloperField::EnergyCapacity, DeveloperField::FoodCapacity] {
        world
            .issue_developer(
                &content,
                world.player.clone(),
                DeveloperEdit { field, value: 10 },
            )
            .unwrap();
    }
    world.advance(&content, 1).unwrap();
    let markets = world.player_country().markets.as_ref().unwrap();
    assert_eq!(markets.food.last.as_ref().unwrap().price_bps, 10000);
    let following_price = markets.food.next_price_bps;
    assert!(following_price > 10000);
    let first_basket: i64 = world
        .player_country()
        .households
        .iter()
        .map(|h| h.basket_cost.0)
        .sum();
    world.advance(&content, 1).unwrap();
    assert_eq!(
        world
            .player_country()
            .markets
            .as_ref()
            .unwrap()
            .food
            .last
            .as_ref()
            .unwrap()
            .price_bps,
        following_price
    );
    world.advance(&content, 10).unwrap();
    let later_basket: i64 = world
        .player_country()
        .households
        .iter()
        .map(|h| h.basket_cost.0)
        .sum();
    assert!(later_basket > first_basket);
    assert_eq!(world.player_country().accounts.total(), opening_cash);
    world.validate(&content).unwrap();
    assert_eq!(
        persistence::replay(&world, &content).unwrap().hash(),
        world.hash()
    );
}

#[test]
fn physical_need_requested_use_and_delivery_are_separate() {
    let mut content = fixture();
    let country = &mut content.countries[0];
    country.food = 200;
    country.energy = 200;
    let vulnerable = country
        .households
        .iter()
        .position(|h| h.id == "vulnerable")
        .unwrap();
    let owners = country
        .households
        .iter()
        .position(|h| h.id == "owners")
        .unwrap();
    let moved = country.households[vulnerable].savings_share;
    country.households[vulnerable].savings_share = 0;
    country.households[owners].savings_share += moved;
    let mut world = world(&content);
    world.advance(&content, 1).unwrap();
    let food = &world.player_country().markets.as_ref().unwrap().food;
    let last = food.last.as_ref().unwrap();
    assert!(last.supply >= last.physical_need);
    assert!(last.requested_use < last.physical_need);
    assert_eq!(last.delivered, last.requested_use);
    assert!(food.next_price_bps < last.price_bps);
    assert!(world.player_country().report.food_coverage < 100);
    world.advance(&content, 30).unwrap();
    world.validate(&content).unwrap();
    persistence::replay(&world, &content).unwrap();
}

#[test]
fn bounded_price_adjustment_handles_shortage_surplus_and_zero_demand() {
    let content = fixture();
    let rule = content.markets.as_ref().unwrap().good(GoodKind::Food);
    assert_eq!(next_market_price(10000, 100, 100, rule).unwrap(), 10000);
    assert!(next_market_price(10000, 90, 100, rule).unwrap() > 10000);
    assert!(next_market_price(10000, 110, 100, rule).unwrap() < 10000);
    assert_eq!(
        next_market_price(rule.maximum_price_bps, 0, 100, rule).unwrap(),
        rule.maximum_price_bps
    );
    assert_eq!(
        next_market_price(rule.minimum_price_bps, 100, 0, rule).unwrap(),
        rule.minimum_price_bps
    );
    assert_eq!(next_market_price(10000, 0, 0, rule).unwrap(), 10000);
    assert!(next_market_price(10000, -1, 100, rule).is_err());
    for price in [rule.minimum_price_bps, 10000, rule.maximum_price_bps] {
        for supply in [0, 90, 100, 110, 10000] {
            let next = next_market_price(price, supply, 100, rule).unwrap();
            assert!(
                u64::from(price.abs_diff(next)) * 10000
                    <= u64::from(price) * u64::from(rule.adjustment_bps)
            );
        }
    }
    let mut invalid = rule.clone();
    invalid.adjustment_bps = u32::MAX;
    assert!(next_market_price(10000, i64::MAX, 1, &invalid).is_err());
}

#[test]
fn market_content_provenance_ranges_and_rules_are_validated() {
    let content = fixture();
    assert_eq!(content.rules, 3);
    for change in 0..7 {
        let mut invalid = content.clone();
        match change {
            0 => invalid.rules = 2,
            1 => invalid.markets = None,
            2 => {
                invalid.markets.as_mut().unwrap().goods[1].id =
                    invalid.markets.as_ref().unwrap().goods[0].id.clone()
            }
            3 => invalid.markets.as_mut().unwrap().goods[0].loss_bps = 1,
            4 => invalid.markets.as_mut().unwrap().source = "missing".into(),
            5 => invalid.markets.as_mut().unwrap().goods[1].minimum_price_bps = 0,
            _ => invalid.schema = 2,
        }
        assert!(invalid.validate().is_err());
    }
}

#[test]
fn markets_resume_at_a_later_bookmark_and_do_not_depend_on_roster_order() {
    let content = fixture().at_bookmark("2001-07-15").unwrap();
    let mut original = world(&content);
    original.advance(&content, 180).unwrap();
    let bytes = persistence::encode(&original, &content).unwrap();
    let header: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(header["schema"], 4);
    let mut restored = persistence::decode(&bytes, &content).unwrap();
    original.advance(&content, 30).unwrap();
    restored.advance(&content, 30).unwrap();
    assert_eq!(restored.hash(), original.hash());
    persistence::replay(&original, &content).unwrap();
    let mut reordered_content = content.clone();
    reordered_content.countries.reverse();
    let mut reordered = world(&reordered_content);
    reordered.advance(&reordered_content, 210).unwrap();
    assert_eq!(reordered.countries, original.countries);
    for schema in [2, 3] {
        let mut invalid = header.clone();
        invalid["schema"] = schema.into();
        assert!(persistence::decode(&serde_json::to_vec(&invalid).unwrap(), &content).is_err());
    }
    let mut invalid = original.clone();
    invalid
        .countries
        .get_mut(&invalid.player.clone())
        .unwrap()
        .markets
        .as_mut()
        .unwrap()
        .food
        .last
        .as_mut()
        .unwrap()
        .closing_stock = Food::ZERO;
    // Force a mismatch even when the original closing stock happened to be zero.
    invalid
        .countries
        .get_mut(&invalid.player.clone())
        .unwrap()
        .food_stock = Food::new(123456).unwrap();
    assert!(persistence::encode(&invalid, &content).is_err());
}

#[test]
fn ordinary_rules_two_world_hash_and_format_three_writer_remain_unchanged() {
    let content = Content::bundled().unwrap();
    assert_eq!(content.rules, 2);
    let mut world = World::new(&content, CountryId("GBR".into()), 2000).unwrap();
    world.advance(&content, 360).unwrap();
    assert_eq!(world.hash(), 0xc3994a175e880d20);
    assert!(
        world
            .countries
            .values()
            .all(|country| country.markets.is_none())
    );
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut header: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(header["schema"], 3);
    header["schema"] = 2.into();
    assert_eq!(
        persistence::decode(&serde_json::to_vec(&header).unwrap(), &content)
            .unwrap()
            .hash(),
        world.hash()
    );
}
