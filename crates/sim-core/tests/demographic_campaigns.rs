use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-demographic-balances-test.json"
    ))
    .unwrap()
}
#[test]
fn daily_population_flows_mirror_households_without_creating_capacity_and_replay_exactly() {
    let content = fixture();
    let original = Content::from_json(include_str!(
        "../../../data/synthetic-governing-institutions-test.json"
    ))
    .unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let mut previous = World::new(&original, world.player.clone(), 17).unwrap();
    world.advance(&content, DaySpan::new(1)).unwrap();
    previous.advance(&original, DaySpan::new(1)).unwrap();
    assert_ne!(
        world.player_country().population,
        content.countries[0].population
    );
    assert_eq!(
        world.player_country().report.energy_produced,
        previous.player_country().report.energy_produced
    );
    assert_eq!(
        world.player_country().report.food_produced,
        previous.player_country().report.food_produced
    );
    assert_eq!(
        world.player_country().energy_capacity,
        previous.player_country().energy_capacity
    );
    assert_eq!(
        world.player_country().food_capacity,
        previous.player_country().food_capacity
    );
    world.advance(&content, DaySpan::new(89)).unwrap();
    world.validate(&content).unwrap();
    let state = world.demographics.as_ref().unwrap();
    for (id, country) in &world.countries {
        assert_eq!(country.population, state.country_population(id).unwrap());
        for group in &country.households {
            assert_eq!(
                group.population,
                state.balances.countries[id][&group.id]
                    .people
                    .total()
                    .unwrap()
            );
            assert_eq!(
                group.working_age,
                state.balances.countries[id][&group.id].people.working_age
            );
        }
    }
    let bytes = persistence::encode(&world, &content).unwrap();
    assert_eq!(persistence::snapshot_schema(&world), 30);
    let mut restored = persistence::decode(&bytes, &content).unwrap();
    assert_eq!(persistence::replay(&restored, &content).unwrap(), world);
    restored.advance(&content, DaySpan::new(40)).unwrap();
    world.advance(&content, DaySpan::new(40)).unwrap();
    assert_eq!(restored, world);
    world.validate(&content).unwrap();
}
#[test]
fn foreign_reports_use_dated_public_estimates_and_owned_age_flows_require_the_controlled_viewer() {
    let content = fixture();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(20)).unwrap();
    let player = world.player.clone();
    let owner = world
        .demographic_report(&content, Viewer::Government(player.clone()), &player)
        .unwrap()
        .unwrap();
    assert!(owner.owned_groups.is_some() && owner.next_attempt.is_some());
    for viewer in [
        Viewer::Observer,
        Viewer::Government(CountryId("BOR".into())),
    ] {
        let public = world
            .demographic_report(&content, viewer, &player)
            .unwrap()
            .unwrap();
        assert!(public.owned_groups.is_none() && public.next_attempt.is_none());
        assert_eq!(public.published, owner.published);
    }
    let foreign = world
        .demographic_report(
            &content,
            Viewer::Government(player),
            &CountryId("BOR".into()),
        )
        .unwrap()
        .unwrap();
    assert!(foreign.owned_groups.is_none());
    let public = world.public_report(&content, &world.player).unwrap();
    let published = public.population_measurement.unwrap();
    assert_eq!(public.population, published.population);
    assert_eq!(published.day, content.start_day().unwrap());
    assert_ne!(published.population, world.player_country().population);
    assert_eq!(published.population, owner.published[0].population);
}
#[test]
fn missing_or_foreign_population_state_and_incorrect_envelopes_are_rejected() {
    let content = fixture();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.demographics = None;
    assert!(world.validate(&content).is_err());
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.countries.get_mut(&world.player).unwrap().households[0]
        .working_age
        .0 += 1;
    assert!(world.validate(&content).is_err());
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let mut save: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&world, &content).unwrap()).unwrap();
    save["schema"] = 29.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&save).unwrap(), &content)
            .unwrap_err()
            .contains("format 30")
    );
    let old = Content::bundled().unwrap();
    let old_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert!(old_world.demographics.is_none());
    assert!(
        serde_json::to_value(&old_world)
            .unwrap()
            .get("demographics")
            .is_none()
    );
    assert_eq!(persistence::snapshot_schema(&old_world), 29);
    let mut invalid = serde_json::to_value(&content).unwrap();
    invalid["rules"] = 28.into();
    invalid["schema"] = 28.into();
    assert!(Content::from_json(&invalid.to_string()).is_err());
}
#[test]
fn reserve_contracts_retain_opening_quantity_and_price_while_residents_change() {
    let content = fixture();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let preview = world
        .preview_reserve(&content, &world.player, GoodKind::Food, NeedDays::new(1))
        .unwrap();
    world
        .issue(
            &content,
            Action::ProcureReserve {
                good: GoodKind::Food,
                days: NeedDays::new(1),
            },
        )
        .unwrap();
    world.advance(&content, DaySpan::new(120)).unwrap();
    assert_ne!(
        world.player_country().population,
        content.countries[0].population
    );
    let order = &world.player_country().reserves.as_ref().unwrap().orders[0];
    assert_eq!(order.quantity, preview.quantity);
    assert_eq!(order.quoted_cost, preview.cost);
    world.validate(&content).unwrap();
    let bytes = persistence::encode(&world, &content).unwrap();
    assert_eq!(persistence::decode(&bytes, &content).unwrap(), world);
}

#[test]
fn ordinary_long_run_decline_can_reach_zero_without_phantom_income_or_division_by_zero() {
    let mut value = serde_json::to_value(fixture()).unwrap();
    value["facts"] = serde_json::json!([]);
    value["demographics"]["corridors"] = serde_json::json!([]);
    for country in value["countries"].as_array_mut().unwrap() {
        country["population"] = 1000.into();
        country["energy"] = 100.into();
        country["food"] = 100.into();
    }
    for profiles in value["demographics"]["profiles"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        for profile in profiles.as_object_mut().unwrap().values_mut() {
            profile["rates"]["birth_per_working_age_bps"] = 0.into();
            for age in [
                "young_mortality_bps",
                "working_mortality_bps",
                "older_mortality_bps",
            ] {
                profile["rates"][age] = 10000.into();
            }
        }
    }
    let content = Content::from_json(&value.to_string()).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(3660)).unwrap();
    world.advance(&content, DaySpan::new(2340)).unwrap();
    assert!(world.countries.values().all(|c| c.population.0 == 0));
    world.validate(&content).unwrap();
    for country in world.countries.values() {
        assert_eq!(country.report.output, Money(0));
        assert_eq!(country.report.wages, Money(0));
        assert_eq!(country.report.consumption, Money(0));
        assert!(
            country
                .income_tax_report
                .as_ref()
                .unwrap()
                .groups
                .iter()
                .all(|g| g.assessment.gross == Money(0))
        );
        assert!(
            country
                .households
                .iter()
                .all(|g| g.employed.0 == 0 && g.basket_cost == Money(0))
        );
        assert_eq!(country.approval, 0);
        let forecasts =
            country.forecast_for_content(content.country(&country.id).unwrap(), &content);
        assert_eq!(forecasts.daily_revenue, Money(0));
    }
    let bytes = persistence::encode(&world, &content).unwrap();
    assert_eq!(persistence::decode(&bytes, &content).unwrap(), world);
}

#[test]
fn optional_roster_preserves_every_opening_input_and_resolves_its_exact_save_hash() {
    let content = Content::demographic_bundled().unwrap();
    let old = Content::governing_institutions_bundled().unwrap();
    assert_eq!(content.countries, old.countries);
    assert_eq!(content.policies, old.policies);
    assert_eq!(content.facts, old.facts);
    assert_eq!(&content.sources[..old.sources.len()], &old.sources);
    assert_eq!(content.legislation, old.legislation);
    assert_eq!(content.executive_identities, old.executive_identities);
    assert_eq!(content.governing_institutions, old.governing_institutions);
    assert_eq!(Content::bundled_for_hash(content.hash()).unwrap(), content);
    assert_eq!(Content::bundled_for_hash(old.hash()).unwrap(), old);
    for country in &content.countries {
        let mut world = World::new(&content, country.id.clone(), 17).unwrap();
        world.advance(&content, DaySpan::new(1)).unwrap();
        world.validate(&content).unwrap();
    }
}

#[test]
fn paid_administration_changes_precision_only_at_the_next_attempt_after_delivery() {
    let content = fixture();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let before = world.player_country().administration;
    let capacity = statistical_capacity(
        content
            .demographics
            .as_ref()
            .unwrap()
            .statistical_capacities[&world.player],
        Percent::new(before as u32),
    )
    .unwrap();
    let preview = world
        .preview(&content, &world.player, &"administration".into(), 1)
        .unwrap();
    let treasury = world.player_country().accounts.treasury;
    world
        .issue(
            &content,
            Action::Enact {
                policy: "administration".into(),
                value: 1,
            },
        )
        .unwrap();
    assert!(world.player_country().accounts.treasury.0 < treasury.0);
    world.advance(&content, preview.delay).unwrap();
    assert_eq!(world.player_country().administration, (before + 8).min(100));
    let archive = &world.demographics.as_ref().unwrap().archives[&world.player];
    assert!(
        archive
            .measurements
            .iter()
            .all(|m| m.published.statistical_capacity == capacity)
    );
    let next = archive.next_attempt.unwrap();
    world
        .advance(&content, DaySpan::new(next.units() - world.day.units()))
        .unwrap();
    let latest = world.demographics.as_ref().unwrap().archives[&world.player]
        .measurements
        .last()
        .unwrap();
    assert!(latest.published.statistical_capacity.units() > capacity.units());
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn paid_construction_can_expand_opening_capacity_past_the_preserved_limit() {
    let mut value = serde_json::to_value(fixture()).unwrap();
    value["countries"][0]["energy"] = 200.into();
    let content = Content::from_json(&value.to_string()).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(30)).unwrap();
    let preview = world
        .preview(&content, &world.player, &"energy".into(), 1)
        .unwrap();
    let before = world.player_country().accounts.treasury;
    world
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    assert!(world.player_country().accounts.treasury.0 < before.0);
    assert_eq!(world.player_country().energy_capacity, 200);
    world.advance(&content, preview.delay).unwrap();
    assert_eq!(world.player_country().energy_capacity, 215);
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
