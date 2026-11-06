use sim_core::*;

#[test]
fn basis_point_fields_keep_numeric_wire_units_and_reject_invalid_signed_input() {
    let maximum = BasisPoints::new(u32::MAX);
    assert_eq!(maximum.units_i64(), 4_294_967_295);
    assert_eq!(format!("{maximum:?}"), "4294967295");
    assert_eq!(format!("{maximum}"), "4294967295");
    assert_eq!(serde_json::to_string(&maximum).unwrap(), "4294967295");
    let content = Content::bundled().unwrap();
    assert_eq!(content.hash(), 0x0a0defa128f0a947);
    let original = serde_json::to_value(&content).unwrap();
    let policies = original["policies"].as_array().unwrap();
    let index = policies
        .iter()
        .position(|p| p["id"] == "income_tax")
        .unwrap();
    assert_eq!(policies[index]["cost_bps"], 2);
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!("2"),
        serde_json::json!(4_294_967_296_u64),
    ] {
        let mut bad = original.clone();
        bad["policies"][index]["cost_bps"] = invalid;
        assert!(Content::from_json(&serde_json::to_string(&bad).unwrap()).is_err());
    }
    let mut bad = content.clone();
    bad.policies[index].cost_bps = BasisPoints::new(1001);
    assert!(bad.validate().is_err());
    assert_eq!(
        Content::from_json(&serde_json::to_string(&original).unwrap())
            .unwrap()
            .hash(),
        content.hash()
    );
}

#[test]
fn typed_rates_keep_paid_appointments_amendments_loans_and_delivery_exact() {
    let content = Content::bundled().unwrap();
    let id = CountryId("GBR".into());
    let tax = PolicyId::from("income_tax");
    let cabinet = content.cabinet.as_ref().unwrap();
    let office = cabinet
        .portfolios
        .iter()
        .find(|p| p.country == id && p.policies.contains(&tax))
        .unwrap();
    let person = cabinet
        .candidates
        .iter()
        .filter(|p| p.office == office.id)
        .max_by_key(|p| (p.skill, &p.person))
        .unwrap();
    let mut world = World::new(&content, id, 17).unwrap();
    world
        .issue(
            &content,
            Action::Appoint {
                office: office.id.clone(),
                person: person.person.clone(),
            },
        )
        .unwrap();
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
                policy: tax.clone(),
                value: 30,
            },
        )
        .unwrap();
    world.advance(&content, DaySpan::new(7)).unwrap();
    let request = world
        .player_country()
        .projects
        .iter()
        .find(|p| p.policy == tax)
        .unwrap()
        .id;
    world
        .issue(&content, Action::AmendCommitment { request, value: 32 })
        .unwrap();
    world.advance(&content, DaySpan::new(7)).unwrap();
    assert_eq!(
        world.policy_preparation_days(&content, &world.player, &tax),
        DaySpan::new(4)
    );
    world
        .issue(
            &content,
            Action::Enact {
                policy: PolicyId::from("services"),
                value: 8,
            },
        )
        .unwrap();
    // Captured with the preceding release executable, before basis-point fields changed.
    assert_eq!(world.hash(), 0x94cbad91666278ba);
    let loan = world
        .player_country()
        .financing
        .as_ref()
        .unwrap()
        .instruments
        .iter()
        .find(|l| l.original_principal == Money(1000))
        .unwrap();
    assert_eq!(loan.annual_rate_bps, BasisPoints::new(400));
    assert_eq!(
        world
            .player_country()
            .projects
            .iter()
            .find(|p| p.id == request)
            .unwrap()
            .due_day,
        85
    );
    let bytes = persistence::encode(&world, &content).unwrap();
    world = persistence::decode(&bytes, &content).unwrap();
    assert_eq!(persistence::encode(&world, &content).unwrap(), bytes);
    world.advance(&content, DaySpan::new(186)).unwrap();
    assert_eq!(world.hash(), 0x78a8fbc9461a8ac5);
    assert_eq!(
        (world.player_country().tax, world.player_country().services),
        (32, 8)
    );
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}
