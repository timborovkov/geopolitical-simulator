use sim_core::*;

#[test]
fn starter_preserves_the_roster_and_seeds_explicit_attributed_positions_without_free_resources() {
    let content = Content::bundled().unwrap();
    let old = Content::technology_bundled().unwrap();
    assert_eq!((content.schema, content.rules), (20, RULES_VERSION));
    assert_eq!(content.countries.len(), 15);
    assert_eq!(content.countries, old.countries);
    let rules = content.party_preferences.as_ref().unwrap();
    assert_eq!(rules.profiles.len(), 34);
    assert_eq!(rules.influences.len(), 15);
    assert_eq!(
        rules
            .profiles
            .iter()
            .filter(|p| p.quality == RecordQuality::Unknown)
            .count(),
        4
    );
    assert!(content.sovereignty.is_none());
    assert_eq!(
        &content.sources[..old.sources.len()],
        old.sources.as_slice()
    );
    assert_eq!(content.sources.len(), old.sources.len() + 1);
    assert!(
        content
            .sources
            .iter()
            .any(|s| s.id == rules.source && s.quality == "authored_scenario_model")
    );
    let mut normalized = content.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    normalized.party_preferences = None;
    assert_eq!(
        normalized, old,
        "only party preferences, attribution and pack metadata may change"
    );
    let mut routes = (0, 0, 0);
    for d in &content.countries {
        let mut world = World::new(&content, d.id.clone(), 17).unwrap();
        let prior = World::new(&old, d.id.clone(), 17).unwrap();
        let before = world.hash();
        let alignment = world
            .party_alignment_for(
                &content,
                &d.id,
                &FiscalTargets::current(world.player_country()),
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            alignment.penalty, 0,
            "opening platform is aligned for {}",
            d.id.0
        );
        match alignment.metric {
            Some(PartyBackingMetric::Legislative) => {
                routes.0 += 1;
                assert_eq!(alignment.known_positions_bps, 10000);
            }
            Some(PartyBackingMetric::PartyLeadership) => {
                routes.1 += 1;
                assert_eq!(alignment.known_positions_bps, 10000);
            }
            None => {
                routes.2 += 1;
                assert_eq!(alignment.max_penalty, 0);
            }
        }
        assert_eq!(world.hash(), before);
        world.content_hash = prior.content_hash;
        world.rules = prior.rules;
        assert_eq!(
            world, prior,
            "{} receives no cash, capacity, support or free orders",
            d.id.0
        );
    }
    assert_eq!(routes, (9, 2, 4));
    for pack in [
        content,
        old,
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
    ] {
        assert_eq!(Content::bundled_for_hash(pack.hash()).unwrap(), pack);
    }
}

#[test]
fn coalition_and_party_leadership_keep_distinct_seeded_tradeoffs_and_advisory_unknowns() {
    let content = Content::bundled().unwrap();
    let world = World::new(&content, CountryId("DEU".into()), 17).unwrap();
    let before = world.hash();
    let targets = FiscalTargets {
        income_tax: 45,
        services: 18,
        welfare: 12,
    };
    let alignment = world
        .party_alignment_for(&content, &world.player, &targets)
        .unwrap()
        .unwrap();
    assert!(alignment.penalty > 0);
    assert_eq!(alignment.parties.iter().filter(|p| p.weight > 0).count(), 2);
    assert_eq!(
        alignment
            .parties
            .iter()
            .find(|p| p.party.0 == "DEU:spd")
            .unwrap()
            .weight,
        45
    );
    assert_eq!(
        alignment
            .parties
            .iter()
            .find(|p| p.party.0 == "DEU:greens")
            .unwrap()
            .weight,
        10
    );
    assert!(
        world
            .preview(&content, &world.player, "income_tax", 45)
            .unwrap()
            .explanation
            .contains("monthly political reviews")
    );
    assert_eq!(world.hash(), before);
    for id in ["CHN", "CUB"] {
        let w = World::new(&content, CountryId(id.into()), 17).unwrap();
        let r = w
            .party_alignment_for(&content, &w.player, &targets)
            .unwrap()
            .unwrap();
        assert_eq!(r.metric, Some(PartyBackingMetric::PartyLeadership));
        assert!(r.penalty > 0);
        assert_eq!(r.parties.iter().filter(|p| p.weight == 1).count(), 1);
    }
    for id in ["SAU", "IRN", "PRK", "MMR"] {
        let w = World::new(&content, CountryId(id.into()), 17).unwrap();
        let r = w
            .party_alignment_for(&content, &w.player, &targets)
            .unwrap()
            .unwrap();
        assert_eq!(r.metric, None);
        assert_eq!(r.penalty, 0);
        assert!(r.parties.iter().any(|p| p.distance_bps.is_none()));
    }
}

#[test]
fn mixed_formats_21_and_19_retain_exact_old_campaigns_and_pending_new_policy_continuation() {
    let old = Content::technology_bundled().unwrap();
    let prior = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(prior.hash(), 0xd5f9f1f64662dcd3);
    let content = Content::bundled().unwrap();
    let mut current = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    current
        .issue(
            &content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 45,
            },
        )
        .unwrap();
    current.advance(&content, 7).unwrap();
    let directory =
        std::env::temp_dir().join(format!("geosim-party-starter-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &current, &content).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(21));
    assert_eq!(candidates[1].schema, Some(19));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, old);
    assert_eq!(restored, prior);
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(directory.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    let mut resumed = persistence::decode(&primary, &content).unwrap();
    current.advance(&content, 120).unwrap();
    resumed.advance(&content, 120).unwrap();
    assert_eq!(current, resumed);
    assert_eq!(persistence::replay(&current, &content).unwrap(), current);
    let fixture = Content::from_json(include_str!(
        "../../../data/synthetic-party-preferences-test.json"
    ))
    .unwrap();
    let optional = World::new(&fixture, CountryId("AUR".into()), 17).unwrap();
    let bytes = persistence::encode(&optional, &fixture).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert!(persistence::load_bundled(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(persistence::load(&path, &fixture).unwrap(), optional);
    std::fs::remove_dir_all(directory).unwrap();
}
