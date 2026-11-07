use sim_core::*;
fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-executive-identities-test.json"
    ))
    .unwrap()
}
#[test]
fn explicit_opening_people_unknown_and_collective_holders_seed_stable_read_only_references() {
    let content = fixture();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.validate(&content).unwrap();
    let before = world.clone();
    let person = world
        .executive_identity_view(&content, &CountryId("AUR".into()))
        .unwrap()
        .unwrap();
    assert_eq!(
        person.holder,
        ExecutiveHolder::Person {
            person: PersonId("person:AUR:opening-executive".into())
        }
    );
    assert_eq!(person.government, GovernmentId(1));
    assert_eq!(person.tenures[0].tenure.id, ExecutiveTenureId(1));
    let unknown = world
        .executive_identity_view(&content, &CountryId("BOR".into()))
        .unwrap()
        .unwrap();
    assert_eq!(unknown.holder, ExecutiveHolder::Unknown);
    assert_eq!(unknown.quality, RecordQuality::Unknown);
    let council = world
        .executive_identity_view(&content, &CountryId("COR".into()))
        .unwrap()
        .unwrap();
    assert_eq!(
        council.holder,
        ExecutiveHolder::Collective {
            body: ExecutiveBodyId("reference:COR:opening-governing-body".into())
        }
    );
    assert!(matches!(
        council.body_origin,
        Some(ExecutiveBodyOrigin::Reference { .. })
    ));
    assert!(council.members.is_empty()); // Unknown members are not invented.
    assert_eq!(before, world);
    assert_eq!(persistence::snapshot_schema(&world), 28);
    let bytes = persistence::encode(&world, &content).unwrap();
    assert_eq!(persistence::decode(&bytes, &content).unwrap(), world);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn the_optional_reference_layer_preserves_every_prior_default_byte_and_historical_calibration() {
    let old = Content::bundled().unwrap();
    assert_eq!(old.rules, 25);
    assert_eq!(old.hash(), 0x0a0defa128f0a947);
    let world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(world.hash(), 0x86a7fade232d09c2);
    assert!(
        world
            .executive_identity_view(&old, &world.player)
            .unwrap()
            .is_none()
    );
    let new = Content::executive_identities_bundled().unwrap();
    let preceding = Content::legislation_bundled().unwrap();
    assert_eq!(new.countries, preceding.countries);
    assert_eq!(new.rules, 27);
    assert_eq!(
        Content::bundled_for_hash(new.hash()).unwrap().hash(),
        new.hash()
    );
    let live = World::new(&new, CountryId("GBR".into()), 17).unwrap();
    live.validate(&new).unwrap();
    let view = live
        .executive_identity_view(&new, &live.player)
        .unwrap()
        .unwrap();
    assert!(matches!(view.holder, ExecutiveHolder::Person { .. }));
    assert_eq!(view.name, "Tony Blair");
    assert_eq!(view.opening_office, OfficeId("GBR:governing-office".into()));
    assert_eq!(
        persistence::decode(&persistence::encode(&live, &new).unwrap(), &new).unwrap(),
        live
    );
}
#[test]
fn election_replacement_allocates_a_body_independent_of_its_display_name_and_mandate() {
    let mut raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/synthetic-executive-identities-test.json"
    ))
    .unwrap();
    let country_index = raw["countries"]
        .as_array()
        .unwrap()
        .iter()
        .position(|country| country["id"] == "AUR")
        .unwrap();
    let party = raw["countries"][country_index]["parties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|party| party["id"] == "AUR-2")
        .unwrap();
    let label = format!("{} cabinet 2", party["name"].as_str().unwrap());
    let person_index = raw["identities"]["people"]
        .as_array()
        .unwrap()
        .iter()
        .position(|person| person["id"] == "person:AUR:opening-executive")
        .unwrap();
    // The sourced person deliberately has exactly the future cabinet label.
    raw["identities"]["people"][person_index]["name"] = serde_json::json!(label);
    raw["countries"][country_index]["executive"] = serde_json::json!(label);
    let content = Content::from_json(&raw.to_string()).unwrap();
    let mut world = World::new_scenario(
        &content,
        CountryId("AUR".into()),
        17,
        Scenario::FiscalSupplyCrisis,
    )
    .unwrap();
    let before = world
        .player_country()
        .executive_identity
        .as_ref()
        .unwrap()
        .holder
        .clone();
    let due = world.player_country().next_election.unwrap();
    while world.player_country().government_number == GovernmentId(1)
        && world.day.units() < (due).units()
    {
        world.advance(&content, DaySpan::new(1)).unwrap();
    }
    world.validate(&content).unwrap();
    let state = world.player_country().executive_identity.as_ref().unwrap();
    assert_eq!(world.player_country().government_number, GovernmentId(2));
    assert_eq!(
        state.holder,
        ExecutiveHolder::Collective {
            body: ExecutiveBodyId("generated:AUR:1".into())
        }
    );
    assert_ne!(state.holder, before);
    assert_eq!(world.player_country().executive, label);
    assert_eq!(state.tenures[0].until_day, Some(world.day));
    assert_eq!(state.holder_since, world.day);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn reference_dates_do_not_replace_live_people_and_vacant_is_distinct_from_unknown() {
    let mut raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/synthetic-executive-identities-test.json"
    ))
    .unwrap();
    let original_term = raw["identities"]["terms"]
        .as_array()
        .unwrap()
        .iter()
        .position(|term| term["office"] == "office:AUR:governing-reference")
        .unwrap();
    let mut future_person = raw["identities"]["people"][0].clone();
    future_person["id"] = serde_json::json!("person:AUR:future-executive");
    future_person["name"] = serde_json::json!("Fictional future steward");
    raw["identities"]["people"]
        .as_array_mut()
        .unwrap()
        .push(future_person);
    let mut future = raw["identities"]["terms"][original_term].clone();
    future["id"] = serde_json::json!("term:AUR:future-executive");
    future["holder"] = serde_json::json!({"kind":"person","person":"person:AUR:future-executive"});
    future["validity"]["valid_from"] = serde_json::json!("2000-02-01");
    raw["identities"]["terms"][original_term]["validity"]["valid_to"] =
        serde_json::json!("2000-02-01");
    raw["identities"]["terms"]
        .as_array_mut()
        .unwrap()
        .push(future);
    let content = Content::from_json(&raw.to_string()).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let holder = world
        .player_country()
        .executive_identity
        .as_ref()
        .unwrap()
        .holder
        .clone();
    world.advance(&content, DaySpan::new(35)).unwrap();
    world.validate(&content).unwrap();
    assert_eq!(
        world
            .player_country()
            .executive_identity
            .as_ref()
            .unwrap()
            .holder,
        holder
    );
    let later = content.at_bookmark("2000-02-02").unwrap();
    let later_world = World::new(&later, CountryId("AUR".into()), 17).unwrap();
    assert_eq!(
        later_world
            .player_country()
            .executive_identity
            .as_ref()
            .unwrap()
            .holder,
        ExecutiveHolder::Person {
            person: PersonId("person:AUR:future-executive".into())
        }
    );
    let mut vacant_raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/synthetic-executive-identities-test.json"
    ))
    .unwrap();
    vacant_raw["identities"]["terms"][original_term]["holder"] =
        serde_json::json!({"kind":"vacant"});
    vacant_raw["countries"][0]["executive"] = serde_json::json!(
        fixture()
            .identities
            .as_ref()
            .unwrap()
            .holder_name(&OfficeHolder::Vacant)
    );
    let vacant = Content::from_json(&vacant_raw.to_string()).unwrap();
    let vacant_world = World::new(&vacant, CountryId("AUR".into()), 17).unwrap();
    vacant_world.validate(&vacant).unwrap();
    assert_eq!(
        vacant_world
            .player_country()
            .executive_identity
            .as_ref()
            .unwrap()
            .holder,
        ExecutiveHolder::Vacant
    );
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn transitional_authorities_have_distinct_ids_and_pruning_tenures_never_reuses_bodies() {
    let mut content = fixture();
    content
        .executive_identities
        .as_mut()
        .unwrap()
        .retained_closed_tenures = 2;
    let rules = content.constitutions.as_mut().unwrap();
    rules.cooldown_days = DaySpan::new(30);
    let mut routes = vec![&mut rules.lawful];
    if let Some(route) = &mut rules.contested {
        routes.push(route);
    }
    for route in routes {
        route.days = DaySpan::new(1);
        route.cost_bps = BasisPoints::new(1);
        route.upfront_bps = BasisPoints::new(2500);
        route.legislative_threshold = 0;
        route.elite_threshold = 0;
        route.council_threshold = 0;
        route.approval_threshold = 0;
        route.approval_cost = 0;
        route.legislative_cost = 0;
        route.elite_cost = 0;
        route.council_cost = 0;
    }
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    for (index, target) in [
        Regime::Presidential,
        Regime::AbsoluteMonarchy,
        Regime::Presidential,
        Regime::AbsoluteMonarchy,
    ]
    .into_iter()
    .enumerate()
    {
        world
            .issue(
                &content,
                Action::Reconstitute {
                    target,
                    contested: false,
                },
            )
            .unwrap();
        let bytes = persistence::encode(&world, &content).unwrap();
        assert_eq!(persistence::decode(&bytes, &content).unwrap(), world);
        world.advance(&content, DaySpan::new(30)).unwrap();
        world.validate(&content).unwrap();
        let state = world.player_country().executive_identity.as_ref().unwrap();
        assert_eq!(
            state.holder,
            ExecutiveHolder::Collective {
                body: ExecutiveBodyId(format!("generated:AUR:{}", index + 1))
            }
        );
        assert_eq!(
            world.player_country().government_number,
            GovernmentId(index as u32 + 2)
        );
    }
    let state = world.player_country().executive_identity.as_ref().unwrap();
    assert_eq!(state.tenures.len(), 3);
    assert_eq!(state.tenures[0].id, ExecutiveTenureId(3));
    assert_eq!(state.bodies.len(), 4);
    assert!(
        state
            .bodies
            .contains_key(&ExecutiveBodyId("generated:AUR:1".into()))
    );
    assert_eq!(state.next_generated_body, 5);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn exhausted_body_allocator_rolls_back_the_actual_executive_replacement_day() {
    let content = fixture();
    let mut world = World::new_scenario(
        &content,
        CountryId("AUR".into()),
        17,
        Scenario::FiscalSupplyCrisis,
    )
    .unwrap();
    world
        .countries
        .get_mut(&world.player)
        .unwrap()
        .executive_identity
        .as_mut()
        .unwrap()
        .next_generated_body = u64::MAX;
    world.validate(&content).unwrap();
    let limit = world.player_country().next_election.unwrap();
    let mut failed = false;
    while world.day.units() <= (limit).units() {
        let before = world.clone();
        match world.advance(&content, DaySpan::new(1)) {
            Ok(()) => {}
            Err(error) => {
                assert!(
                    error.contains("Executive body identity exhausted"),
                    "{error}"
                );
                assert_eq!(world, before);
                world.validate(&content).unwrap();
                failed = true;
                break;
            }
        }
    }
    assert!(
        failed,
        "A real replacement must exercise the exhausted allocator"
    );
}

#[test]
fn malformed_holder_keys_labels_dates_and_old_envelopes_are_rejected() {
    let content = fixture();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    for variant in 0..6 {
        let mut bad = world.clone();
        let country = bad.countries.get_mut(&bad.player).unwrap();
        let state = country.executive_identity.as_mut().unwrap();
        match variant {
            0 => state.next_generated_body = 0,
            1 => {
                state.holder = ExecutiveHolder::Person {
                    person: PersonId("unknown".into()),
                }
            }
            2 => state.government = GovernmentId(2),
            3 => state.tenures[0].from_day = CampaignDay::new(1),
            4 => country.executive = "Unrelated display name".into(),
            5 => state.tenures[0].id = ExecutiveTenureId(2),
            _ => unreachable!(),
        }
        assert!(
            persistence::encode(&bad, &content).is_err(),
            "variant {variant}"
        );
    }
    let mut raw: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&world, &content).unwrap()).unwrap();
    raw["schema"] = serde_json::json!(27);
    assert!(
        persistence::decode(&serde_json::to_vec(&raw).unwrap(), &content)
            .unwrap_err()
            .contains("format 28")
    );
    let mut bad_content = content.clone();
    bad_content
        .executive_identities
        .as_mut()
        .unwrap()
        .opening_offices
        .insert(
            CountryId("AUR".into()),
            OfficeId("office:BOR:governing-reference".into()),
        );
    assert!(bad_content.validate().is_err());
    let mut bad_content = content.clone();
    bad_content
        .executive_identities
        .as_mut()
        .unwrap()
        .retained_closed_tenures = 0;
    assert!(bad_content.validate().is_err());
}

#[test]
fn live_identity_bookkeeping_preserves_all_preceding_economic_political_and_ai_state() {
    let old = Content::legislation_bundled().unwrap();
    let new = Content::executive_identities_bundled().unwrap();
    let player = CountryId("GBR".into());
    let mut reference =
        World::new_scenario(&old, player.clone(), 17, Scenario::FiscalSupplyCrisis).unwrap();
    let mut tracked = World::new_scenario(&new, player, 17, Scenario::FiscalSupplyCrisis).unwrap();
    reference.advance(&old, DaySpan::new(800)).unwrap();
    tracked.advance(&new, DaySpan::new(800)).unwrap();
    reference.validate(&old).unwrap();
    tracked.validate(&new).unwrap();
    let expected = serde_json::to_value(&reference).unwrap();
    let mut normalized = serde_json::to_value(&tracked).unwrap();
    normalized["rules"] = expected["rules"].clone();
    normalized["content_hash"] = expected["content_hash"].clone();
    for country in normalized["countries"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        country
            .as_object_mut()
            .unwrap()
            .remove("executive_identity");
    }
    assert_eq!(normalized, expected);
    assert_eq!(persistence::replay(&tracked, &new).unwrap(), tracked);
}

#[test]
fn elite_replacement_preserves_reference_collective_and_rejects_corrupt_generated_bodies() {
    let content = fixture();
    let mut world = World::new_scenario(
        &content,
        CountryId("COR".into()),
        17,
        Scenario::FiscalSupplyCrisis,
    )
    .unwrap();
    for _ in 0..800 {
        if world.player_country().government_number != GovernmentId(1) {
            break;
        }
        world.advance(&content, DaySpan::new(1)).unwrap();
    }
    let state = world.player_country().executive_identity.as_ref().unwrap();
    assert_eq!(state.government, GovernmentId(2));
    assert!(state.bodies.contains_key(&ExecutiveBodyId(
        "reference:COR:opening-governing-body".into()
    )));
    let body = ExecutiveBodyId("generated:COR:1".into());
    assert!(matches!(
        state.bodies[&body].origin,
        ExecutiveBodyOrigin::Generated {
            body_kind: ExecutiveBodyKind::LeadershipCouncil,
            ..
        }
    ));
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    for variant in 0..5 {
        let mut bad = world.clone();
        let state = bad
            .countries
            .get_mut(&bad.player)
            .unwrap()
            .executive_identity
            .as_mut()
            .unwrap();
        let generated = state.bodies.get_mut(&body).unwrap();
        match variant {
            0 => generated
                .members
                .push(PersonId("person:AUR:opening-executive".into())),
            1 => generated.created_day = CampaignDay::new(world.day.units() + 1),
            2 => generated.id = ExecutiveBodyId("generated:COR:99".into()),
            3 => generated.name = "Unattributed council".into(),
            4 => {
                state.bodies.remove(&ExecutiveBodyId(
                    "reference:COR:opening-governing-body".into(),
                ));
            }
            _ => unreachable!(),
        }
        assert!(
            persistence::encode(&bad, &content).is_err(),
            "variant {variant}"
        );
    }
}
