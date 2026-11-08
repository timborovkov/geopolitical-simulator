use sim_core::*;

fn fictional() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-sector-production-test.json"
    ))
    .unwrap()
}
#[test]
fn auxiliary_network_settles_real_stocks_funded_workers_and_exact_resume_in_both_packs() {
    for (content, id) in [
        (fictional(), "AUR"),
        (Content::production_bundled().unwrap(), "GBR"),
    ] {
        let mut world = World::new(&content, CountryId(id.into()), 17).unwrap();
        assert_eq!(persistence::snapshot_schema(&world), 32);
        for _ in 0..5 {
            world.advance(&content, DaySpan::new(1)).unwrap();
            world.validate(&content).unwrap();
        }
        let definition = content.production.as_ref().unwrap();
        let state = &world.production.as_ref().unwrap().countries[&world.player];
        let last = state.last.as_ref().unwrap();
        assert!(
            last.physical
                .production
                .sectors
                .iter()
                .any(|sector| sector.produced.0 > 0)
        );
        assert!(last.physical.production.labor_available.0 <= last.request.workers.0);
        assert_eq!(
            last.funding.private_income_paid,
            world.player_country().report.wages
        );
        assert!(
            state.inventory.positions[&definition.assembly_good]
                .quantity
                .0
                > 0
        );
        assert_eq!(
            state.inventory.positions[&definition.service_good].quantity,
            ProductionQuantity(0)
        );
        let encoded = persistence::encode(&world, &content).unwrap();
        let mut resumed = persistence::decode(&encoded, &content).unwrap();
        world.advance(&content, DaySpan::new(7)).unwrap();
        resumed.advance(&content, DaySpan::new(7)).unwrap();
        assert_eq!(resumed, world);
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
}
#[test]
fn prior_governing_campaign_retains_independently_recorded_opening_and_day31_hashes() {
    // Measured with the published demographic checkpoint's production binary.
    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(world.hash(), 0xba69e6436891ac61);
    assert!(world.production.is_none());
    world.advance(&content, DaySpan::new(31)).unwrap();
    assert_eq!(world.hash(), 0x5142fa0daffcb500);
    assert_eq!(persistence::snapshot_schema(&world), 29);
}
#[test]
fn paid_energy_construction_waits_for_real_packages_then_operates_and_replays() {
    let mut content = fictional();
    let id = CountryId("AUR".into());
    let mut initial = World::new(&content, id.clone(), 17).unwrap();
    initial.advance(&content, DaySpan::new(30)).unwrap();
    let earliest = initial
        .preview(&content, &id, &"energy".into(), 1)
        .unwrap()
        .delay;
    let required = u64::from(initial.day.units() + earliest.units() + 40) * 10;
    let definition = content.production.as_mut().unwrap();
    for weight in definition.construction_policy_weights.values_mut() {
        *weight = 1;
    }
    let opening = definition.countries.get_mut(&id).unwrap();
    opening.construction_unit = ProductionQuantity(required);
    opening
        .installed
        .insert(SectorId("construction".into()), ProductionBatches(1));
    opening
        .storage
        .get_mut(&definition.assembly_good)
        .unwrap()
        .capacity = ProductionQuantity(required);
    content.validate().unwrap();
    let mut world = World::new(&content, id.clone(), 17).unwrap();
    world.advance(&content, DaySpan::new(30)).unwrap();
    let capacity = world.player_country().energy_capacity;
    world
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    let project = world.player_country().projects[0].clone();
    world.advance(&content, earliest).unwrap();
    world.validate(&content).unwrap();
    assert_eq!(world.player_country().energy_capacity, capacity);
    assert!(world.player_country().project(&project.id).is_some());
    assert_eq!(
        world.player_country().commitment(&project.id).unwrap().paid,
        project.cost
    );
    assert!(world.events.iter().any(|event| {
        event.country == id
            && event
                .text
                .contains("awaits prefabricated assembly packages")
    }));
    let notice = world
        .events
        .iter()
        .find(|event| {
            event.country == id
                && event
                    .text
                    .contains("awaits prefabricated assembly packages")
        })
        .unwrap();
    assert!(
        !notice.text.contains(&required.to_string()),
        "public delay notices do not expose private stock quantities"
    );
    let encoded = persistence::encode(&world, &content).unwrap();
    let mut resumed = persistence::decode(&encoded, &content).unwrap();
    for _ in 0..1000 {
        if world.player_country().project(&project.id).is_none() {
            break;
        }
        world.advance(&content, DaySpan::new(1)).unwrap();
        resumed.advance(&content, DaySpan::new(1)).unwrap();
        assert_eq!(resumed, world);
    }
    assert!(world.player_country().project(&project.id).is_none());
    assert!(world.day > project.due_day);
    assert_eq!(world.player_country().energy_capacity, capacity + 15);
    let production = &world.production.as_ref().unwrap().countries[&id];
    assert!(
        production
            .last
            .as_ref()
            .unwrap()
            .capital_uses
            .iter()
            .any(|used| used.commitment == project.id
                && used.quantity == ProductionQuantity(required))
    );
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn missing_capital_limits_actual_service_inputs_without_creating_or_destroying_cash() {
    let content = fictional();
    let id = CountryId("AUR".into());
    let mut constrained = content.clone();
    for capital in constrained
        .production
        .as_mut()
        .unwrap()
        .countries
        .get_mut(&id)
        .unwrap()
        .capital
        .values_mut()
    {
        *capital = CapitalServices(0);
    }
    constrained.validate().unwrap();
    let mut normal = World::new(&content, id.clone(), 17).unwrap();
    let mut stopped = World::new(&constrained, id.clone(), 17).unwrap();
    normal.advance(&content, DaySpan::new(1)).unwrap();
    stopped.advance(&constrained, DaySpan::new(1)).unwrap();
    assert_eq!(
        normal.player_country().accounts,
        stopped.player_country().accounts
    );
    assert!(stopped.player_country().report.wages.0 > 0);
    assert!(normal.player_country().report.service_delivery > 0);
    assert_eq!(stopped.player_country().report.service_delivery, 0);
    let last = stopped.production.as_ref().unwrap().countries[&id]
        .last
        .as_ref()
        .unwrap();
    assert!(last.funding.service_paid.0 > 0);
    assert!(
        last.physical
            .production
            .sectors
            .iter()
            .all(|sector| sector.produced.0 == 0)
    );
    assert_eq!(last.service_delivered, ProductionQuantity(0));
    stopped.validate(&constrained).unwrap();
}
#[test]
fn unmaintained_paid_additions_remain_installed_but_do_not_create_civilian_output() {
    let mut content = fictional();
    let id = CountryId("AUR".into());
    let definition = content.production.as_mut().unwrap();
    let required = definition
        .construction_required(&id, &"energy".into())
        .unwrap()
        .unwrap();
    let opening = definition.countries.get_mut(&id).unwrap();
    opening
        .stocks
        .insert(definition.assembly_good.clone(), required);
    for capital in opening.capital.values_mut() {
        *capital = CapitalServices(0);
    }
    content.validate().unwrap();
    let mut world = World::new(&content, id.clone(), 17).unwrap();
    world.advance(&content, DaySpan::new(30)).unwrap();
    let initial_capacity = world.player_country().energy_capacity;
    world
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    let due = world.player_country().projects[0].due_day;
    world
        .advance(&content, due.elapsed_since(world.day).unwrap())
        .unwrap();
    assert_eq!(
        world.player_country().energy_capacity,
        initial_capacity + 15
    );
    world.advance(&content, DaySpan::new(2)).unwrap();
    let country = world.player_country();
    let last = world.production.as_ref().unwrap().countries[&id]
        .last
        .as_ref()
        .unwrap();
    assert_eq!(last.maintenance_percent, 0);
    assert_eq!(last.maintenance_delivered, ProductionQuantity(0));
    let basis = (content.country(&id).unwrap().population.0 / 1000) as i64;
    assert_eq!(
        country.report.energy_produced.units(),
        basis * i64::from((initial_capacity - country.report.shock).max(0)) / 100
    );
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn versioned_production_preserves_opening_facts_and_rejects_silent_old_format_reinterpretation() {
    let content = Content::production_bundled().unwrap();
    let previous = Content::social_memberships_bundled().unwrap();
    assert_eq!(content.countries, previous.countries);
    assert_eq!(content.policies, previous.policies);
    assert_eq!(content.facts, previous.facts);
    assert_eq!(content.demographics, previous.demographics);
    assert_eq!(content.social_memberships, previous.social_memberships);
    assert_eq!(Content::bundled_for_hash(content.hash()).unwrap(), content);
    assert_eq!(
        Content::bundled_for_hash(previous.hash()).unwrap(),
        previous
    );
    let mut raw = serde_json::to_value(&content).unwrap();
    raw.as_object_mut().unwrap().remove("production");
    assert!(Content::from_json(&raw.to_string()).is_err());
    let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let encoded = persistence::encode(&world, &content).unwrap();
    let mut header: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    header["schema"] = 31.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&header).unwrap(), &content)
            .unwrap_err()
            .contains("32")
    );
}
