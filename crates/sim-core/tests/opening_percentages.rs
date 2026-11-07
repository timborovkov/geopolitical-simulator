use sim_core::*;

// Published rules-25 numeric and state goldens stay pinned when the new-game default changes.

#[test]
fn authored_opening_percentages_keep_numeric_content_and_validate_their_own_bounds() {
    let content = Content::statistical_history_bundled().unwrap();
    assert_eq!(content.hash(), 0x0a0defa128f0a947);
    let original = serde_json::to_value(&content).unwrap();
    assert_eq!(format!("{:?}", Percent::new(28)), "28");
    assert_eq!(format!("{}", Percent::new(200)), "200");
    assert_eq!(serde_json::to_string(&Percent::new(200)).unwrap(), "200");
    let countries = original["countries"].as_array().unwrap();
    let index = countries.iter().position(|c| c["id"] == "GBR").unwrap();
    assert_eq!(countries[index]["tax"], 28);
    for field in [
        "tax", "services", "welfare", "energy", "food", "capacity", "approval",
    ] {
        for invalid in [
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!("28"),
            serde_json::json!(4_294_967_296_u64),
        ] {
            let mut value = original.clone();
            value["countries"][index][field] = invalid;
            assert!(
                Content::from_json(&serde_json::to_string(&value).unwrap()).is_err(),
                "invalid {field}"
            );
        }
        // A representable rate is still subject to the field's institutional limits.
        let mut value = original.clone();
        value["countries"][index][field] = serde_json::json!(u32::MAX);
        assert!(
            Content::from_json(&serde_json::to_string(&value).unwrap()).is_err(),
            "unbounded {field}"
        );
    }
    let mut expanded = content.clone();
    let country = expanded
        .countries
        .iter_mut()
        .find(|c| c.id.0 == "GBR")
        .unwrap();
    country.energy = Percent::new(200);
    country.food = Percent::new(200);
    expanded.validate().unwrap(); // Production potential can exceed one day's need.
    assert!(World::new(&expanded, CountryId("GBR".into()), 17).is_ok());
    expanded.countries[index].tax = Percent::new(u32::MAX);
    assert!(World::new(&expanded, CountryId("GBR".into()), 17).is_err());
    assert_eq!(
        Content::from_json(&serde_json::to_string(&original).unwrap())
            .unwrap()
            .hash(),
        content.hash()
    );
}

#[test]
fn typed_opening_rates_preserve_normal_and_crisis_initialization_settlement_and_replay() {
    let content = Content::statistical_history_bundled().unwrap();
    let id = CountryId("GBR".into());
    for (scenario, initial, settled) in [
        (Scenario::Sandbox, 0x86a7fade232d09c2, 0x4dc16fa9a2ffdc92),
        (
            Scenario::FiscalSupplyCrisis,
            0x94af62ddb1c72fe3,
            0x8e4efe93c3374421,
        ),
    ] {
        let mut world = World::new_scenario(&content, id.clone(), 17, scenario).unwrap();
        assert_eq!(world.hash(), initial);
        let c = world.player_country();
        if scenario == Scenario::Sandbox {
            let d = content.country(&id).unwrap();
            assert_eq!(c.tax, d.tax.units() as i32);
            assert_eq!(c.administration, d.capacity.units() as i32);
            assert!(
                c.households
                    .iter()
                    .all(|h| h.approval == d.approval.units() as i32)
            );
        } else {
            assert_eq!((c.tax, c.services, c.welfare), (10, 18, 12));
            assert_eq!(
                (
                    c.energy_capacity,
                    c.food_capacity,
                    c.administration,
                    c.approval
                ),
                (40, 40, 60, 20)
            );
        }
        world.advance(&content, DaySpan::new(31)).unwrap();
        assert_eq!(world.hash(), settled);
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
}
