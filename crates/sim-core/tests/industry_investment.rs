use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-industry-investment-test.json"
    ))
    .unwrap()
}
fn proposal(world: &mut World, content: &Content, policy: &str) {
    let command = world.command(
        world.player.clone(),
        Action::Enact {
            policy: policy.into(),
            value: 1,
        },
    );
    world.execute(content, command).unwrap();
}
#[test]
fn paid_industry_commissions_only_after_consent_time_and_materials_then_adds_next_day_assets() {
    let content = content();
    let id = CountryId("AUR".into());
    let sector = SectorId("construction".into());
    let mut world = World::new(&content, id.clone(), 17).unwrap();
    world.validate(&content).unwrap();
    assert_eq!(persistence::snapshot_schema(&world), 33);
    let initial = world.player_country().available_cash();
    let cost = world
        .preview(&content, &id, &"industry_construction".into(), 1)
        .unwrap()
        .cost;
    proposal(&mut world, &content, "industry_construction");
    assert_eq!(
        world.player_country().available_cash(),
        Money(initial.0 - cost.0)
    );
    let opening = content.production.as_ref().unwrap().countries[&id].installed[&sector];
    let mut operated = false;
    for _ in 0..300 {
        world.advance(&content, DaySpan::new(1)).unwrap();
        world.validate(&content).unwrap();
        let current = world.player_country().industry.as_ref().unwrap().levels[&sector];
        let last = world.production.as_ref().unwrap().countries[&id]
            .last
            .as_ref()
            .unwrap();
        if current == 1 {
            assert_eq!(last.request.capacities[&sector].installed_batches, opening);
            assert_eq!(last.industry_opening.as_ref().unwrap().levels[&sector], 0);
            operated = true;
            break;
        }
        assert_eq!(current, 0);
        assert_eq!(last.request.capacities[&sector].installed_batches, opening);
    }
    assert!(operated, "Funded industry must actually operate");
    let mut forged = world.clone();
    let effects = &mut forged
        .countries
        .get_mut(&id)
        .unwrap()
        .commitments
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r.project.policy == "industry_construction")
        .unwrap()
        .direct_effects;
    effects[0].before = 3;
    effects[0].after = 3;
    assert!(forged.validate(&content).is_err());
    let snapshot = persistence::encode(&world, &content).unwrap();
    let mut resumed = persistence::decode(&snapshot, &content).unwrap();
    world.advance(&content, DaySpan::new(1)).unwrap();
    resumed.advance(&content, DaySpan::new(1)).unwrap();
    assert_eq!(world, resumed);
    let last = world.production.as_ref().unwrap().countries[&id]
        .last
        .as_ref()
        .unwrap();
    assert!(last.request.capacities[&sector].installed_batches.0 > opening.0);
    assert_eq!(
        persistence::replay(&world, &content).unwrap().hash(),
        world.hash()
    );
}
#[test]
fn investment_definitions_and_installed_history_refuse_invention_and_legacy_injection() {
    let content = content();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world
        .countries
        .get_mut(&world.player.clone())
        .unwrap()
        .industry
        .as_mut()
        .unwrap()
        .levels
        .insert(SectorId("construction".into()), 1);
    assert!(world.validate(&content).is_err());
    let mut raw = serde_json::to_value(Content::production_bundled().unwrap()).unwrap();
    raw["industry"] = serde_json::to_value(content.industry.as_ref().unwrap()).unwrap();
    assert!(Content::from_json(&raw.to_string()).is_err());
    let mut raw = serde_json::to_value(&content).unwrap();
    raw["industry"]["countries"]["AUR"]["construction"] = serde_json::json!(0);
    assert!(Content::from_json(&raw.to_string()).is_err());
}
#[test]
fn carried_industry_upkeep_conserves_fractional_claims_and_forecasts_exact_intervals() {
    let content = content();
    let definition = content.industry.as_ref().unwrap();
    let mut state = IndustryInvestmentState::opening(definition);
    state.levels.insert(SectorId("construction".into()), 1);
    let gdp = content.countries[0].gdp;
    let projection = state.upkeep_projection(gdp);
    let mut paid = Money(0);
    for _ in 0..365 {
        let p = state.upkeep_projection(gdp);
        let (due, carry) = p.assessment(DaySpan::new(1)).unwrap();
        paid.0 += due.0;
        state.upkeep_remainder = carry;
    }
    assert_eq!(paid, projection.due(DaySpan::new(365)).unwrap());
    assert_eq!(paid.0, gdp.units() * 10 / 10000);
    assert_eq!(state.upkeep_remainder, 0);
    let mut forecast = World::new(&content, CountryId("AUR".into()), 17)
        .unwrap()
        .player_country()
        .budget
        .forecast
        .clone();
    forecast.daily_spending.0 += projection.due(DaySpan::new(1)).unwrap().0;
    forecast.industry_upkeep = Some(projection.clone());
    assert!(forecast.valid());
    assert_eq!(
        forecast.spending_for(DaySpan::new(365)).0,
        (forecast.daily_spending.0 - projection.due(DaySpan::new(1)).unwrap().0) * 365 + paid.0
    );
}

#[test]
fn paid_material_shortage_waits_past_earliest_date_survives_reload_and_withdrawal_keeps_sunk_cost()
{
    let mut content = content();
    let definition = content.production.as_mut().unwrap();
    definition
        .construction_policy_weights
        .insert("industry_construction".into(), 2190);
    for opening in definition.countries.values_mut() {
        opening
            .storage
            .get_mut(&definition.assembly_good)
            .unwrap()
            .capacity = ProductionQuantity(2190);
    }
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    proposal(&mut world, &content, "industry_construction");
    let project = world.player_country().projects[0].clone();
    world
        .advance(&content, project.due_day.elapsed_since(world.day).unwrap())
        .unwrap();
    world.validate(&content).unwrap();
    assert_eq!(
        world.player_country().industry.as_ref().unwrap().levels[&SectorId("construction".into())],
        0
    );
    assert_eq!(
        world.player_country().projects[0].stage,
        PolicyStage::Implementing
    );
    assert_eq!(world.player_country().projects[0].reserved, Money(0));
    let snapshot = persistence::encode(&world, &content).unwrap();
    let mut resumed = persistence::decode(&snapshot, &content).unwrap();
    let stock = world
        .construction_material_plan(
            &content,
            Viewer::Government(world.player.clone()),
            &world.player,
            &"industry_construction".into(),
        )
        .unwrap()
        .unwrap();
    assert!(stock.available.0 < stock.required.0);
    let mut withdrawn = world.clone();
    let before = withdrawn.player_country().accounts.treasury;
    withdrawn
        .issue(
            &content,
            Action::CancelCommitment {
                request: project.id,
            },
        )
        .unwrap();
    assert_eq!(withdrawn.player_country().accounts.treasury, before);
    assert_eq!(
        withdrawn
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .level_count(),
        0
    );
    for _ in 0..200 {
        world.advance(&content, DaySpan::new(1)).unwrap();
        resumed.advance(&content, DaySpan::new(1)).unwrap();
        if world
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .level_count()
            > 0
        {
            break;
        }
    }
    assert_eq!(world, resumed);
    assert_eq!(
        world
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .level_count(),
        1
    );
    world.validate(&content).unwrap();
    assert_eq!(
        world.hash(),
        persistence::replay(&world, &content).unwrap().hash()
    );
}

#[test]
fn nonplayer_pays_for_observed_service_capacity_shortage_through_the_shared_commands() {
    let mut content = content();
    let id = CountryId("BOR".into());
    let sector = SectorId("services".into());
    content
        .production
        .as_mut()
        .unwrap()
        .countries
        .get_mut(&id)
        .unwrap()
        .installed
        .insert(sector.clone(), ProductionBatches(0));
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    for _ in 0..300 {
        world.advance(&content, DaySpan::new(1)).unwrap();
        if world.countries[&id].industry.as_ref().unwrap().levels[&sector] > 0 {
            break;
        }
    }
    assert!(world.journal.iter().any(|entry|entry.ai && entry.command.issuer==id && matches!(&entry.command.action,Action::Enact{policy,..} if policy=="industry_services")));
    assert!(
        world.countries[&id].industry.as_ref().unwrap().levels[&sector] > 0,
        "AI should actually commission a costed service expansion"
    );
    world.validate(&content).unwrap();
    assert_eq!(
        world.hash(),
        persistence::replay(&world, &content).unwrap().hash()
    );
}

#[test]
fn missing_physical_maintenance_limits_added_equipment_after_the_next_settlement() {
    let mut content = content();
    let id = CountryId("AUR".into());
    let service = SectorId("services".into());
    let construction = SectorId("construction".into());
    content
        .production
        .as_mut()
        .unwrap()
        .countries
        .get_mut(&id)
        .unwrap()
        .installed
        .insert(service, ProductionBatches(0));
    content.validate().unwrap();
    let mut world = World::new(&content, id.clone(), 17).unwrap();
    proposal(&mut world, &content, "industry_construction");
    for _ in 0..300 {
        world.advance(&content, DaySpan::new(1)).unwrap();
        if world
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .level_count()
            == 1
        {
            break;
        }
    }
    assert_eq!(
        world
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .level_count(),
        1
    );
    world.advance(&content, DaySpan::new(1)).unwrap();
    let last = world.production.as_ref().unwrap().countries[&id]
        .last
        .as_ref()
        .unwrap();
    assert_eq!(
        last.request.capacities[&construction].installed_batches,
        ProductionBatches(2)
    );
    assert_eq!(last.maintenance_percent, 0);
    world.advance(&content, DaySpan::new(1)).unwrap();
    let last = world.production.as_ref().unwrap().countries[&id]
        .last
        .as_ref()
        .unwrap();
    assert_eq!(
        last.request.capacities[&construction].installed_batches,
        ProductionBatches(1)
    );
    let report = world
        .production_report(&content, Viewer::Government(id.clone()), &id)
        .unwrap()
        .unwrap()
        .owned
        .unwrap()
        .industry
        .unwrap();
    assert_eq!(
        report.nominal.installed[&construction],
        ProductionBatches(2)
    );
    assert_eq!(
        report.effective.installed[&construction],
        ProductionBatches(1)
    );
    assert!(
        world
            .production_report(&content, Viewer::Observer, &id)
            .unwrap()
            .unwrap()
            .owned
            .is_none()
    );
    let mut forged = world.clone();
    forged
        .countries
        .get_mut(&id)
        .unwrap()
        .industry
        .as_mut()
        .unwrap()
        .upkeep_remainder += 1;
    let before = forged.clone();
    assert!(forged.advance(&content, DaySpan::new(1)).is_err());
    assert_eq!(forged, before);
    world.validate(&content).unwrap();
}

#[test]
fn early_construction_investment_reduces_actual_material_wait_for_a_later_civilian_programme() {
    let mut content = content();
    let definition = content.production.as_mut().unwrap();
    definition
        .construction_policy_weights
        .insert("energy".into(), 2190);
    for opening in definition.countries.values_mut() {
        opening
            .storage
            .get_mut(&definition.assembly_good)
            .unwrap()
            .capacity = ProductionQuantity(2190);
    }
    content.validate().unwrap();
    let mut ordinary = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let mut invested = ordinary.clone();
    proposal(&mut invested, &content, "industry_construction");
    for world in [&mut ordinary, &mut invested] {
        world.advance(&content, DaySpan::new(30)).unwrap();
        proposal(world, &content, "energy");
    }
    let mut operations = Vec::new();
    for world in [&mut ordinary, &mut invested] {
        for _ in 0..300 {
            world.advance(&content, DaySpan::new(1)).unwrap();
            if world.player_country().energy_capacity
                > world.player_country().opening_energy_capacity
            {
                break;
            }
        }
        let operation = world
            .player_country()
            .commitments
            .as_ref()
            .unwrap()
            .iter()
            .find(|r| r.project.policy == "energy")
            .unwrap()
            .milestones
            .iter()
            .find(|m| m.stage == CommitmentStage::Operating)
            .unwrap()
            .day;
        operations.push(operation);
        world.validate(&content).unwrap();
    }
    assert!(
        operations[1] < operations[0],
        "Actual commissioned construction must relieve the observed material bottleneck: {operations:?}"
    );
}

#[test]
fn new_format_and_counter_overflow_refuse_reinterpretation_and_rollback_the_entire_day() {
    let mut content = content();
    let definition = content.production.as_mut().unwrap();
    definition
        .framework
        .sectors
        .get_mut(&SectorId("manufacturing".into()))
        .unwrap()
        .outputs_per_batch
        .insert(GoodId("tools".into()), ProductionQuantity(u64::MAX));
    for country in definition.countries.values_mut() {
        country
            .stocks
            .insert(GoodId("tools".into()), ProductionQuantity(0));
        country
            .storage
            .get_mut(&GoodId("tools".into()))
            .unwrap()
            .capacity = ProductionQuantity(0);
        country
            .installed
            .insert(SectorId("manufacturing".into()), ProductionBatches(1));
    }
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(1)).unwrap();
    world.validate(&content).unwrap();
    let snapshot = persistence::encode(&world, &content).unwrap();
    let mut raw: serde_json::Value = serde_json::from_slice(&snapshot).unwrap();
    raw["schema"] = serde_json::json!(32);
    assert!(
        persistence::decode(&serde_json::to_vec(&raw).unwrap(), &content)
            .unwrap_err()
            .contains("33")
    );
    let before = world.clone();
    assert!(
        world
            .advance(&content, DaySpan::new(1))
            .unwrap_err()
            .contains("overflow")
    );
    assert_eq!(world, before);
}

#[test]
fn huge_nominal_capacity_does_not_overflow_when_actual_output_is_limited_by_one_equipment_unit() {
    let mut content = Content::from_json(include_str!(
        "../../../data/synthetic-sector-production-test.json"
    ))
    .unwrap();
    let definition = content.production.as_mut().unwrap();
    let recipe = definition
        .framework
        .sectors
        .get_mut(&SectorId("manufacturing".into()))
        .unwrap();
    recipe
        .outputs_per_batch
        .insert(GoodId("tools".into()), ProductionQuantity(u64::MAX));
    recipe
        .inputs_per_batch
        .insert(GoodId("raw_materials".into()), ProductionQuantity(u64::MAX));
    for country in definition.countries.values_mut() {
        country.installed.insert(
            SectorId("manufacturing".into()),
            ProductionBatches(u64::MAX),
        );
        country
            .capital
            .insert(CapitalKindId("equipment".into()), CapitalServices(1));
        for (good, opening, capacity) in [
            ("tools", 0, 0),
            ("construction_materials", 0, 10),
            ("raw_materials", u64::MAX, u64::MAX),
        ] {
            let id = GoodId(good.into());
            country
                .stocks
                .insert(id.clone(), ProductionQuantity(opening));
            country.storage.get_mut(&id).unwrap().capacity = ProductionQuantity(capacity);
        }
    }
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(1)).unwrap();
    world.validate(&content).unwrap();
    let inventory = &world.production.as_ref().unwrap().countries[&world.player].inventory;
    assert_eq!(
        inventory.positions[&GoodId("tools".into())].totals.produced,
        ProductionQuantity(u64::MAX)
    );
    assert_eq!(
        inventory.positions[&GoodId("raw_materials".into())]
            .totals
            .industrial_use,
        ProductionQuantity(u64::MAX)
    );
    world.advance(&content, DaySpan::new(2)).unwrap();
    world.validate(&content).unwrap();
    assert_eq!(
        world.production.as_ref().unwrap().countries[&world.player]
            .inventory
            .positions[&GoodId("tools".into())]
            .totals
            .produced,
        ProductionQuantity(u64::MAX)
    );
}

#[test]
fn optional_industry_preserves_the_entire_opening_world_after_removing_only_new_asset_and_forecast_fields()
 {
    let old_content = Content::production_bundled().unwrap();
    let new_content = Content::industry_bundled().unwrap();
    assert_eq!(old_content.countries, new_content.countries);
    assert_eq!(old_content.facts, new_content.facts);
    let old = World::new(&old_content, CountryId("GBR".into()), 17).unwrap();
    let new = World::new(&new_content, CountryId("GBR".into()), 17).unwrap();
    new.validate(&new_content).unwrap();
    let expected = serde_json::to_value(&old).unwrap();
    let mut normalized = serde_json::to_value(&new).unwrap();
    normalized["rules"] = expected["rules"].clone();
    normalized["content_hash"] = expected["content_hash"].clone();
    for country in normalized["countries"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        assert!(
            country
                .as_object_mut()
                .unwrap()
                .remove("industry")
                .is_some()
        );
        assert!(
            country["budget"]["forecast"]
                .as_object_mut()
                .unwrap()
                .remove("industry_upkeep")
                .is_some()
        );
    }
    assert_eq!(
        normalized, expected,
        "Investment adds no opening people, jobs, cash, goods, capacity or authority"
    );
}
