use sim_core::*;

#[test]
fn physical_arithmetic_rejects_overflow_and_preserves_integer_rounding() {
    let maximum = Energy::new(i64::MAX).unwrap();
    assert!(maximum.checked_add(Energy::new(1).unwrap()).is_none());
    assert!(maximum.scaled(2, 1).is_none());
    assert_eq!(maximum.scaled(2, 2), Some(maximum));
    assert_eq!(Food::new(101).unwrap().scaled(1, 100).unwrap().units(), 1);
    assert!(Food::new(1).unwrap().scaled(1, 0).is_none());
    assert!(Food::new(1).unwrap().scaled(-1, 1).is_none());
    assert!(Food::ZERO.checked_sub(Food::new(1).unwrap()).is_none());
    assert_eq!(Food::ZERO.saturating_sub(Food::new(1).unwrap()), Food::ZERO);
    assert_eq!(Food::new(9).unwrap().coverage(Food::new(10).unwrap()), 90);
    assert_eq!(maximum.coverage(Energy::new(1).unwrap()), 100);
    assert_eq!(Food::ZERO.coverage(Food::ZERO), 100);
}

#[test]
fn quantities_keep_legacy_json_scalars_and_reject_invalid_values() {
    assert_eq!(
        serde_json::to_string(&Food::new(42).unwrap()).unwrap(),
        "42"
    );
    assert_eq!(serde_json::from_str::<Energy>("42").unwrap().units(), 42);
    for malformed in ["-1", "1.5", "9223372036854775808", "null", "{\"units\":42}"] {
        assert!(serde_json::from_str::<Food>(malformed).is_err());
        assert!(serde_json::from_str::<Energy>(malformed).is_err());
    }
}

#[test]
fn outside_campaign_bounds_is_rejected_even_with_a_valid_quantity_type() {
    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 2000).unwrap();
    world.advance(&content, 30).unwrap();
    let mut invalid = world.clone();
    invalid
        .countries
        .get_mut(&world.player)
        .unwrap()
        .report
        .food_imported = Food::new(i64::MAX).unwrap();
    assert!(invalid.validate(&content).is_err());
    assert!(persistence::encode(&invalid, &content).is_err());
    let bytes = persistence::encode(&world, &content).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["world"]["countries"]["GBR"]["food_stock"].is_i64());
    assert_eq!(
        persistence::decode(&bytes, &content).unwrap().hash(),
        world.hash()
    );
    persistence::replay(&world, &content).unwrap();
}
