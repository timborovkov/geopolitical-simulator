use sim_core::*;

fn without_references(mut world: World, prior: &World) -> World {
    world.rules = prior.rules;
    world.content_hash = prior.content_hash;
    world.event_sequences = None;
    for event in &mut world.events {
        event.id = None;
    }
    world
}

#[test]
fn seeded_event_references_preserve_every_definition_and_opening_resource() {
    let content = Content::event_reference_bundled().unwrap();
    let old = Content::reserve_bundled().unwrap();
    assert_eq!((content.schema, content.rules), (22, 22));
    assert_eq!(content.countries.len(), 15);
    assert_eq!(
        content.party_preferences.as_ref().unwrap().profiles.len(),
        34
    );
    assert_eq!(
        &content.sources[..old.sources.len()],
        old.sources.as_slice()
    );
    assert_eq!(content.sources.len(), old.sources.len() + 1);
    assert!(
        content
            .sources
            .iter()
            .any(|source| source.id == "authored-starter-event-references-v1"
                && source.quality == "authored_scenario_model")
    );
    let mut normalized = content.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    assert_eq!(normalized, old);
    for country in &content.countries {
        let world = World::new(&content, country.id.clone(), 17).unwrap();
        let prior = World::new(&old, country.id.clone(), 17).unwrap();
        let id = EventId {
            country: country.id.clone(),
            sequence: 1,
        };
        assert_eq!(world.events[0].id, Some(id.clone()));
        assert_eq!(world.event_sequences.as_ref().unwrap().len(), 15);
        assert_eq!(
            world
                .event_by_reference(&content, Viewer::Government(country.id.clone()), &id)
                .unwrap(),
            Some(&world.events[0])
        );
        assert_eq!(without_references(world, &prior), prior);
    }
    let packs = [
        content,
        old,
        Content::party_bundled().unwrap(),
        Content::technology_bundled().unwrap(),
        Content::interest_bundled().unwrap(),
        Content::cash_bundled().unwrap(),
        Content::debt_bundled().unwrap(),
        Content::national_fiscal_bundled().unwrap(),
        Content::income_bundled().unwrap(),
        Content::fiscal_bundled().unwrap(),
        Content::undertaking_bundled().unwrap(),
        Content::territorial_bundled().unwrap(),
        Content::observed_bundled().unwrap(),
        Content::electoral_bundled().unwrap(),
        Content::cabinet_bundled().unwrap(),
        Content::commitment_bundled().unwrap(),
        Content::constitution_bundled().unwrap(),
        Content::finance_bundled().unwrap(),
        Content::market_bundled().unwrap(),
        Content::classic_bundled().unwrap(),
    ];
    assert_eq!(packs.len(), 20);
    for pack in packs {
        assert_eq!(Content::bundled_for_hash(pack.hash()).unwrap(), pack);
    }
}

#[test]
fn every_seeded_government_retains_private_references_without_altering_policy_or_ai_outcomes() {
    let content = Content::event_reference_bundled().unwrap();
    let old = Content::reserve_bundled().unwrap();
    for country in &content.countries {
        let mut world = World::new(&content, country.id.clone(), 17).unwrap();
        let mut prior = World::new(&old, country.id.clone(), 17).unwrap();
        let opening = world.clone();
        let mut wrong_content = content.clone();
        wrong_content.description.push_str(" different");
        assert!(world.advance(&wrong_content, 1).is_err());
        assert!(world.advance(&content, 3661).is_err());
        assert_eq!(world, opening);
        let action = Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        };
        assert_eq!(
            world.issue(&content, action.clone()),
            prior.issue(&old, action)
        );
        world.advance(&content, 60).unwrap();
        prior.advance(&old, 60).unwrap();
        world.validate(&content).unwrap();
        let before = world.clone();
        let view = world
            .project(&content, Viewer::Government(country.id.clone()))
            .unwrap();
        assert!(!view.own_events.as_ref().unwrap().is_empty());
        let own_events = view.own_events.unwrap();
        assert!(own_events.iter().all(|event| event.country == country.id));
        for event in own_events
            .iter()
            .take(1)
            .chain(own_events.iter().rev().take(1))
        {
            assert_eq!(event.country, country.id);
            assert_eq!(
                world
                    .event_by_reference(
                        &content,
                        Viewer::Government(country.id.clone()),
                        event.id.as_ref().unwrap()
                    )
                    .unwrap(),
                Some(event)
            );
        }
        for event in world
            .events
            .iter()
            .filter(|event| event.country != country.id)
            .take(1)
        {
            assert!(
                world
                    .event_by_reference(
                        &content,
                        Viewer::Government(country.id.clone()),
                        event.id.as_ref().unwrap()
                    )
                    .unwrap()
                    .is_none()
            );
        }
        assert!(
            world
                .project(&content, Viewer::Observer)
                .unwrap()
                .own_events
                .is_none()
        );
        assert_eq!(world, before);
        assert_eq!(
            without_references(world.clone(), &prior),
            prior,
            "{} economic, political and AI outcomes changed",
            country.id.0
        );
        assert_eq!(world, persistence::replay(&world, &content).unwrap());
    }
}

#[test]
fn mixed_formats_23_and_22_preserve_reference_identity_and_exact_original_recovery_bytes() {
    let content = Content::event_reference_bundled().unwrap();
    let old = Content::reserve_bundled().unwrap();
    let prior = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(prior.hash(), 0x343d72b6cdfb6c83);
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    world
        .issue(
            &content,
            Action::ProcureReserve {
                good: GoodKind::Energy,
                days: 1,
            },
        )
        .unwrap();
    world.advance(&content, 7).unwrap();
    let reference = world.events[1].id.clone().unwrap();
    let original_event = world.events[1].clone();
    let directory = std::env::temp_dir().join(format!(
        "geosim-event-reference-starter-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &world, &content).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(23));
    assert_eq!(candidates[1].schema, Some(22));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, old);
    assert_eq!(restored, prior);
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(directory.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    let mut resumed = persistence::decode(&primary, &content).unwrap();
    assert_eq!(
        resumed
            .event_by_reference(
                &content,
                Viewer::Government(resumed.player.clone()),
                &reference
            )
            .unwrap(),
        Some(&original_event)
    );
    world.advance(&content, 90).unwrap();
    resumed.advance(&content, 90).unwrap();
    assert_eq!(world, resumed);
    assert!(
        world
            .player_country()
            .reserves
            .as_ref()
            .unwrap()
            .energy
            .purchased
            .units()
            > 0
    );
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
    std::fs::remove_dir_all(directory).unwrap();
}
