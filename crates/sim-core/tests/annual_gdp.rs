use sim_core::*;

// Published rules-25 numeric and state goldens stay pinned when the new-game default changes.

#[test]
fn annual_output_keeps_numeric_content_and_rejects_negative_or_out_of_range_calibration() {
    assert!(AnnualGdp::new(-1).is_err());
    let maximum = AnnualGdp::new(i64::MAX).unwrap();
    assert_eq!(maximum.units(), i64::MAX);
    assert_eq!(format!("{maximum:?}"), i64::MAX.to_string());
    assert_eq!(format!("{maximum}"), i64::MAX.to_string());
    assert_eq!(
        serde_json::to_string(&maximum).unwrap(),
        i64::MAX.to_string()
    );
    for invalid in ["-1", "1.5", "\"1000\"", "9223372036854775808"] {
        assert!(serde_json::from_str::<AnnualGdp>(invalid).is_err());
    }
    let content = Content::statistical_history_bundled().unwrap();
    assert_eq!(content.hash(), 0x0a0defa128f0a947);
    let original = serde_json::to_value(&content).unwrap();
    let index = original["countries"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c["id"] == "GBR")
        .unwrap();
    assert_eq!(original["countries"][index]["gdp"], 1_665_534);
    for value in [
        serde_json::json!(-1),
        serde_json::json!(0),
        serde_json::json!(999),
        serde_json::json!(100_000_001),
        serde_json::json!(i64::MAX),
    ] {
        let mut invalid = original.clone();
        invalid["countries"][index]["gdp"] = value;
        assert!(Content::from_json(&serde_json::to_string(&invalid).unwrap()).is_err());
    }
    for value in [0, 999, 100_000_001, i64::MAX] {
        let mut invalid = content.clone();
        invalid.countries[index].gdp = AnnualGdp::new(value).unwrap();
        assert!(World::new(&invalid, CountryId("GBR".into()), 17).is_err());
    }
    assert_eq!(
        Content::from_json(&serde_json::to_string(&original).unwrap())
            .unwrap()
            .hash(),
        content.hash()
    );
}

#[test]
fn annual_calibration_preserves_regional_rounding_loan_tax_and_reserve_delivery_goldens() {
    assert_eq!(
        regional_daily_cost(AnnualGdp::new(1_000_000).unwrap(), 1000, 4),
        Money(10)
    );
    assert_eq!(
        regional_daily_cost(AnnualGdp::new(1000).unwrap(), 1, 1),
        Money(1)
    );
    assert_eq!(
        regional_daily_cost(AnnualGdp::new(1000).unwrap(), 1, 0),
        Money(0)
    );
    let content = Content::statistical_history_bundled().unwrap();
    let player = CountryId("GBR".into());
    for (scenario, initial, settled) in [
        (Scenario::Sandbox, 0x86a7fade232d09c2, 0x4dc16fa9a2ffdc92),
        (
            Scenario::FiscalSupplyCrisis,
            0x94af62ddb1c72fe3,
            0x8e4efe93c3374421,
        ),
    ] {
        let mut world = World::new_scenario(&content, player.clone(), 17, scenario).unwrap();
        assert_eq!(world.hash(), initial);
        world.advance(&content, DaySpan::new(31)).unwrap();
        assert_eq!(world.hash(), settled);
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
    let mut world = World::new(&content, player.clone(), 17).unwrap();
    world
        .issue(
            &content,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
    world
        .issue(
            &content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30,
            },
        )
        .unwrap();
    world.advance(&content, DaySpan::new(44)).unwrap();
    assert_eq!(world.hash(), 0xe61cccb2a0420c9f);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    let mut reserve = World::new(&content, player, 17).unwrap();
    reserve
        .issue(
            &content,
            Action::ProcureReserve {
                good: GoodKind::Energy,
                days: NeedDays::new(1),
            },
        )
        .unwrap();
    assert_eq!(reserve.hash(), 0xedc5157cfed04b92);
    reserve.advance(&content, DaySpan::new(20)).unwrap();
    assert_eq!(reserve.hash(), 0x476bea2742f65dcd);
    assert_eq!(persistence::replay(&reserve, &content).unwrap(), reserve);
}
