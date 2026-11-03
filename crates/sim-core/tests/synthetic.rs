use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap()
}

#[test]
fn synthetic_world_is_independent_of_historical_identities_and_runs_every_governing_route() {
    let content = fixture();
    assert!(content.facts.is_empty());
    assert_eq!(content.countries.len(), 3);
    let mut routes = Vec::new();
    for d in &content.countries {
        assert_eq!(d.identity_source, "synthetic-original");
        assert_eq!(d.economic_source, "synthetic-original");
        assert_eq!(d.population, 1_000_000);
        assert_eq!(d.gdp, 100_000);
        let mut world = World::new(&content, d.id.clone(), 17).unwrap();
        let preview = world.preview(&content, &d.id, "energy", 1).unwrap();
        assert_eq!(preview.consent_delay, d.government.consent_days);
        routes.push(d.government.route.clone());
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        world.advance(&content, 7).unwrap();
        let snapshot = persistence::encode(&world, &content).unwrap();
        let mut restored = persistence::decode(&snapshot, &content).unwrap();
        for _ in 0..24 {
            restored.advance(&content, 30).unwrap();
            world.advance(&content, 30).unwrap();
            restored.validate(&content).unwrap();
            assert_eq!(restored.hash(), world.hash());
        }
        assert!(world.player_country().energy_capacity > d.energy);
        persistence::replay(&world, &content).unwrap();
    }
    assert!(routes.contains(&DecisionRoute::Legislature));
    assert!(routes.contains(&DecisionRoute::PartyCouncil));
    assert!(routes.contains(&DecisionRoute::RoyalDecree));
}

#[test]
fn a_one_country_pack_needs_no_hidden_reference_to_the_historical_roster() {
    let mut content = fixture();
    content.countries.truncate(1);
    content.countries[0].id = CountryId("TEST-STATE".into());
    let mut world = World::new(&content, content.countries[0].id.clone(), 0).unwrap();
    world.advance(&content, 366).unwrap();
    world.validate(&content).unwrap();
    assert_eq!(world.countries.len(), 1);
    assert!(world.journal.iter().all(|entry| !entry.ai));
    persistence::replay(&world, &content).unwrap();
}
