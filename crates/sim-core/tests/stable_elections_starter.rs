use sim_core::*;

#[test]
fn stable_starter_preserves_historical_definitions_and_resolves_all_twenty_two_packs() {
    let current = Content::stable_elections_bundled().unwrap();
    let old = Content::programme_events_bundled().unwrap();
    assert_eq!((current.schema, current.rules), (24, 24));
    assert_eq!(current.hash(), 0xd705e31066ca7139);
    assert_eq!(old.hash(), 0x2ece4c40d10ad679);
    let mut normalized = current.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    assert_eq!(normalized, old);
    assert_eq!(
        &current.sources[..old.sources.len()],
        old.sources.as_slice()
    );
    assert_eq!(current.sources.len(), old.sources.len() + 1);
    for country in &current.countries {
        let mut world = World::new(&current, country.id.clone(), 17).unwrap();
        let prior = World::new(&old, country.id.clone(), 17).unwrap();
        assert_eq!(persistence::snapshot_schema(&world), 25);
        world.rules = prior.rules;
        world.content_hash = prior.content_hash;
        assert_eq!(world, prior);
    }
    let packs = [
        current,
        old,
        Content::event_reference_bundled().unwrap(),
        Content::reserve_bundled().unwrap(),
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
    assert_eq!(packs.len(), 22);
    for pack in packs {
        assert_eq!(Content::bundled_for_hash(pack.hash()).unwrap(), pack);
    }
}

#[test]
fn current_receipts_and_pre_poll_resources_match_the_preserved_rules23_campaign() {
    let current = Content::stable_elections_bundled().unwrap();
    let old = Content::programme_events_bundled().unwrap();
    let id = CountryId("GBR".into());
    let mut world = World::new(&current, id.clone(), 17).unwrap();
    assert_eq!(world.hash(), 0xf091646f6ad90870);
    let mut prior = World::new(&old, id, 17).unwrap();
    let action = Action::Enact {
        policy: "income_tax".into(),
        value: 30,
    };
    assert_eq!(
        world.issue_with_receipt(&current, action.clone()).unwrap(),
        prior.issue_with_receipt(&old, action).unwrap()
    );
    world.advance(&current, sim_core::DaySpan::new(60)).unwrap();
    prior.advance(&old, sim_core::DaySpan::new(60)).unwrap();
    assert_eq!(world.hash(), 0x5aeea655f8b8a596);
    assert_eq!(prior.hash(), 0x8bf8784e005cff53);
    assert_eq!(world, persistence::replay(&world, &current).unwrap());
    world.rules = prior.rules;
    world.content_hash = prior.content_hash;
    assert_eq!(world, prior);
}

#[test]
fn formats25_and24_recover_exact_originals_and_reject_a_downgraded_new_header() {
    let current = Content::stable_elections_bundled().unwrap();
    let old = Content::programme_events_bundled().unwrap();
    let id = CountryId("GBR".into());
    let mut world = World::new(&current, id.clone(), 17).unwrap();
    world
        .issue(
            &current,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30,
            },
        )
        .unwrap();
    let mut prior = World::new(&old, id, 17).unwrap();
    prior
        .issue(
            &old,
            Action::Enact {
                policy: "income_tax".into(),
                value: 31,
            },
        )
        .unwrap();
    let directory = std::env::temp_dir().join(format!(
        "geosim-stable-election-starter-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &world, &current).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(25));
    assert_eq!(candidates[1].schema, Some(24));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!((resolved, restored), (old, prior));
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(directory.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    let mut resumed = persistence::decode(&primary, &current).unwrap();
    world
        .advance(&current, sim_core::DaySpan::new(108))
        .unwrap();
    resumed
        .advance(&current, sim_core::DaySpan::new(108))
        .unwrap();
    assert_eq!(world, resumed);
    assert_eq!(world, persistence::replay(&world, &current).unwrap());
    let mut header: serde_json::Value = serde_json::from_slice(&primary).unwrap();
    header["schema"] = 24.into();
    let invalid = serde_json::to_vec(&header).unwrap();
    let bad = directory.join("downgraded.json");
    std::fs::write(&bad, &invalid).unwrap();
    assert!(
        persistence::load(&bad, &current)
            .unwrap_err()
            .contains("snapshot format 25")
    );
    assert_eq!(std::fs::read(&bad).unwrap(), invalid);
    std::fs::remove_dir_all(directory).unwrap();
}
