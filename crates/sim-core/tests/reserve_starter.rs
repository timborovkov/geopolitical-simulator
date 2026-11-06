use sim_core::*;

#[test]
fn seeded_reserves_preserve_all_prior_definitions_and_grant_no_opening_resources() {
    let content = Content::reserve_bundled().unwrap();
    let old = Content::party_bundled().unwrap();
    assert_eq!((content.schema, content.rules), (21, 21));
    assert_eq!(content.countries.len(), 15);
    assert_eq!(
        content.party_preferences.as_ref().unwrap().profiles.len(),
        34
    );
    let rules = content.reserves.as_ref().unwrap();
    assert_eq!(
        (
            rules.capacity_days,
            rules.order_days.units(),
            rules.fee_bps,
            rules.ai_target_days
        ),
        (3, 90, 1, 1)
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
            .any(|s| s.id == rules.source && s.quality == "authored_scenario_model")
    );
    let mut normalized = content.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    normalized.reserves = None;
    assert_eq!(normalized, old);
    for d in &content.countries {
        let mut current = World::new(&content, d.id.clone(), 17).unwrap();
        let prior = World::new(&old, d.id.clone(), 17).unwrap();
        assert_eq!(
            current.player_country().reserves,
            Some(NationalReserves::new())
        );
        for good in [GoodKind::Energy, GoodKind::Food] {
            assert!(
                current
                    .reserve_supply(&content, &d.id, good)
                    .unwrap()
                    .is_none()
            );
        }
        current.content_hash = prior.content_hash;
        current.rules = prior.rules;
        for c in current.countries.values_mut() {
            c.reserves = None;
        }
        assert_eq!(
            current, prior,
            "{} receives no money, capacity, backing or stocks",
            d.id.0
        );
    }
    assert_eq!(Content::bundled_for_hash(content.hash()).unwrap(), content);
    assert_eq!(Content::bundled_for_hash(old.hash()).unwrap(), old);
}

#[test]
fn dated_supply_feedback_explains_both_physical_shortfalls_and_quota_bottlenecks_read_only() {
    let mut content = Content::reserve_bundled().unwrap();
    for c in &mut content.countries {
        c.energy = 40;
        c.food = 40;
    }
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
    world.advance(&content, sim_core::DaySpan::new(15)).unwrap();
    let before = world.clone();
    for good in [GoodKind::Energy, GoodKind::Food] {
        let s = world
            .reserve_supply(&content, &world.player, good)
            .unwrap()
            .unwrap();
        assert_eq!(s.day, world.day);
        assert!(s.civilian_supply < s.physical_need);
        assert!(s.explanation().contains("physical needs were not covered"));
        assert_eq!(
            s.spare_import_capacity,
            s.import_capacity - s.civilian_imports
        );
    }
    assert_eq!(
        world.player_country().reserves.as_ref().unwrap().orders[0].filled,
        0
    );
    assert_eq!(world, before);
    let mut wrong = content.clone();
    wrong.reserves.as_mut().unwrap().fee_bps = 2;
    assert!(
        world
            .reserve_supply(&wrong, &world.player, GoodKind::Food)
            .is_err()
    );
    let mut quota_content = Content::from_json(include_str!(
        "../../../data/synthetic-national-reserves-test.json"
    ))
    .unwrap();
    for c in &mut quota_content.countries {
        c.energy = 75;
        c.food = 65;
    }
    let seed = (0..100)
        .find(|seed| !stable_hash(format!("{seed}:AUR:0:supply-v2").as_bytes()).is_multiple_of(3))
        .unwrap();
    let mut quota_world = World::new(&quota_content, CountryId("AUR".into()), seed).unwrap();
    quota_world
        .advance(&quota_content, sim_core::DaySpan::new(1))
        .unwrap();
    for good in [GoodKind::Energy, GoodKind::Food] {
        let s = quota_world
            .reserve_supply(&quota_content, &quota_world.player, good)
            .unwrap()
            .unwrap();
        assert_eq!(s.civilian_supply, s.physical_need);
        assert_eq!(s.spare_import_capacity, 0);
        assert_eq!(s.civilian_imports, s.import_capacity);
        assert!(s.explanation().contains("full import capacity"));
    }
    let mut spare_content = quota_content.clone();
    for c in &mut spare_content.countries {
        c.energy = 90;
        c.food = 90;
    }
    let mut spare_world = World::new(&spare_content, CountryId("AUR".into()), seed).unwrap();
    spare_world
        .advance(&spare_content, sim_core::DaySpan::new(1))
        .unwrap();
    let s = spare_world
        .reserve_supply(&spare_content, &spare_world.player, GoodKind::Energy)
        .unwrap()
        .unwrap();
    assert!(s.spare_import_capacity > 0);
    assert!(
        s.explanation()
            .contains("Spare import capacity was available")
    );
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}

#[test]
fn new_reserve_contract_and_preserved_party_save_recover_together_without_rewriting_originals() {
    let content = Content::reserve_bundled().unwrap();
    let old = Content::party_bundled().unwrap();
    let prior = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(prior.hash(), 0xc29f309743ff5b94);
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
    world.advance(&content, sim_core::DaySpan::new(7)).unwrap();
    let directory =
        std::env::temp_dir().join(format!("geosim-reserve-starter-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &world, &content).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(22));
    assert_eq!(candidates[1].schema, Some(21));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, old);
    assert_eq!(restored, prior);
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(directory.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    let mut resumed = persistence::decode(&primary, &content).unwrap();
    world.advance(&content, sim_core::DaySpan::new(90)).unwrap();
    resumed
        .advance(&content, sim_core::DaySpan::new(90))
        .unwrap();
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
