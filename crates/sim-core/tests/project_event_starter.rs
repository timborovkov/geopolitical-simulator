use sim_core::*;

fn without_context(mut world: World, prior: &World) -> World {
    world.rules = prior.rules;
    world.content_hash = prior.content_hash;
    for event in &mut world.events {
        event.context = None;
    }
    world
}

#[test]
fn linked_starter_preserves_definitions_resources_and_all_twenty_previous_pack_hashes() {
    let content = Content::programme_events_bundled().unwrap();
    let old = Content::event_reference_bundled().unwrap();
    assert_eq!((content.schema, content.rules), (23, 23));
    assert_eq!(content.countries.len(), 15);
    assert_eq!(old.hash(), 0xcf3ddf9118d0c3c5);
    assert_eq!(content.hash(), 0x2ece4c40d10ad679);
    assert_eq!(
        World::new(&content, CountryId("GBR".into()), 17)
            .unwrap()
            .hash(),
        0xb74fd50f30b753b9
    );
    let mut normalized = content.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    assert_eq!(normalized, old);
    assert_eq!(
        &content.sources[..old.sources.len()],
        old.sources.as_slice()
    );
    assert_eq!(content.sources.len(), old.sources.len() + 1);
    for country in &content.countries {
        let world = World::new(&content, country.id.clone(), 17).unwrap();
        let prior = World::new(&old, country.id.clone(), 17).unwrap();
        assert_eq!(world.events[0].context, Some(EventContext::General));
        assert_eq!(without_context(world, &prior), prior);
    }
    let packs = [
        content,
        old,
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
    assert_eq!(packs.len(), 21);
    for pack in packs {
        assert_eq!(Content::bundled_for_hash(pack.hash()).unwrap(), pack);
    }
}

#[test]
fn every_seeded_government_has_linked_outcomes_with_identical_policy_ai_and_private_resources() {
    let c = Content::programme_events_bundled().unwrap();
    let old = Content::event_reference_bundled().unwrap();
    for country in &c.countries {
        let mut w = World::new(&c, country.id.clone(), 17).unwrap();
        let mut prior = World::new(&old, country.id.clone(), 17).unwrap();
        let action = Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        };
        let accepted = w.issue(&c, action.clone());
        assert_eq!(accepted, prior.issue(&old, action));
        w.advance(&c, 60).unwrap();
        prior.advance(&old, 60).unwrap();
        assert_eq!(without_context(w.clone(), &prior), prior);
        w.validate(&c).unwrap();
        if accepted.is_ok() {
            assert!(w.events.iter().any(|event| event.country == w.player
                && matches!(event.context, Some(EventContext::Commitment { .. }))));
        }
        for event in w.events.iter().filter(|e| {
            e.country == w.player && matches!(e.context, Some(EventContext::Commitment { .. }))
        }) {
            let record = w
                .event_commitment(
                    &c,
                    Viewer::Government(w.player.clone()),
                    event.id.as_ref().unwrap(),
                )
                .unwrap()
                .unwrap();
            assert_eq!(record.project.policy, "income_tax");
        }
        let owner = w.project(&c, Viewer::Government(w.player.clone())).unwrap();
        assert!(
            owner
                .own_events
                .unwrap()
                .iter()
                .all(|e| e.country == w.player)
        );
        assert!(
            w.project(&c, Viewer::Observer)
                .unwrap()
                .own_events
                .is_none()
        );
    }
}

#[test]
fn formats24_and23_recover_without_losing_originals_and_pending_links_resume_exactly() {
    let c = Content::programme_events_bundled().unwrap();
    let old = Content::event_reference_bundled().unwrap();
    let mut w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
    w.issue(
        &c,
        Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        },
    )
    .unwrap();
    let reference = w.events.last().unwrap().id.clone().unwrap();
    let prior = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "geosim-project-event-starter-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &w, &c).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(24));
    assert_eq!(candidates[1].schema, Some(23));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!((resolved, restored), (old, prior));
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(directory.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    let mut resumed = persistence::decode(&primary, &c).unwrap();
    assert_eq!(
        resumed
            .event_commitment(&c, Viewer::Government(resumed.player.clone()), &reference)
            .unwrap()
            .unwrap()
            .id,
        CommitmentId(1)
    );
    w.advance(&c, 108).unwrap();
    resumed.advance(&c, 108).unwrap();
    assert_eq!(w, resumed);
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
    std::fs::remove_dir_all(directory).unwrap();
}
