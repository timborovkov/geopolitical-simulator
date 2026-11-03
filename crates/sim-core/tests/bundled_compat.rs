use sim_core::*;

#[test]
fn published_constitutional_pack_keeps_its_no_order_campaign_golden() {
    let content = Content::from_json(include_str!(
        "../../../data/initial-2000-constitutions.json"
    ))
    .unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 2000).unwrap();
    world.advance(&content, 360).unwrap();
    assert_eq!(world.hash(), 0x542bcedc081ce958);
    persistence::replay(&world, &content).unwrap();
}

#[test]
fn electoral_starter_preserves_prior_country_policy_and_reference_data() {
    let current = Content::bundled().unwrap();
    let classic = Content::classic_bundled().unwrap();
    assert_eq!(current.schema, 8);
    assert_eq!(current.rules, 8);
    assert_eq!(classic.rules, 2);
    assert_eq!(current.countries, classic.countries);
    assert_eq!(current.policies, classic.policies);
    assert_eq!(current.facts, classic.facts);
    assert_ne!(current.hash(), classic.hash());
    let market = Content::market_bundled().unwrap();
    assert_eq!(market.rules, 3);
    assert!(market.finance.is_none());
    assert_eq!(current.countries, market.countries);
    assert_eq!(current.policies, market.policies);
    assert_eq!(current.facts, market.facts);
    assert_eq!(current.markets, market.markets);
    let finance = Content::finance_bundled().unwrap();
    assert_eq!(finance.rules, 4);
    assert!(finance.constitutions.is_none());
    assert_eq!(current.countries, finance.countries);
    assert_eq!(current.policies, finance.policies);
    assert_eq!(current.facts, finance.facts);
    assert_eq!(current.finance, finance.finance);
    let constitution = Content::constitution_bundled().unwrap();
    assert_eq!(constitution.rules, 5);
    assert!(constitution.commitments.is_none());
    assert_eq!(current.countries, constitution.countries);
    assert_eq!(current.policies, constitution.policies);
    assert_eq!(current.facts, constitution.facts);
    assert_eq!(current.constitutions, constitution.constitutions);
    let commitment = Content::commitment_bundled().unwrap();
    assert_eq!(commitment.rules, 6);
    assert!(commitment.cabinet.is_none());
    assert_eq!(current.countries, commitment.countries);
    assert_eq!(current.policies, commitment.policies);
    assert_eq!(current.facts, commitment.facts);
    assert_eq!(current.commitments, commitment.commitments);
    let cabinet = Content::cabinet_bundled().unwrap();
    assert_eq!(cabinet.rules, 7);
    assert!(cabinet.elections.is_none());
    assert_eq!(current.countries, cabinet.countries);
    assert_eq!(current.cabinet, cabinet.cabinet);
    assert_eq!(current.elections.as_ref().unwrap().mandates.len(), 15);
    for pack in [
        current,
        cabinet,
        commitment,
        constitution,
        finance,
        market,
        classic,
    ] {
        assert_eq!(Content::bundled_for_hash(pack.hash()).unwrap(), pack);
    }
}

#[test]
fn bundled_recovery_selects_exact_pack_and_preserves_converted_backup_bytes() {
    let classic = Content::classic_bundled().unwrap();
    let current = Content::bundled().unwrap();
    let player = CountryId("GBR".into());
    let mut older = World::new(&classic, player.clone(), 17).unwrap();
    older
        .issue(
            &classic,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    older.advance(&classic, 31).unwrap();
    let mut header: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&older, &classic).unwrap()).unwrap();
    header["schema"] = 2.into();
    let original = serde_json::to_vec_pretty(&header).unwrap();
    let mut newer = World::new(&current, player, 17).unwrap();
    newer.advance(&current, 10).unwrap();
    let dir = std::env::temp_dir().join(format!("geosim-bundled-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mixed.json");
    persistence::save(&path, &newer, &current).unwrap();
    let newer_bytes = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    std::fs::write(&backup, &original).unwrap();
    let (resolved, loaded) = persistence::load_bundled(&backup).unwrap();
    assert_eq!(resolved.hash(), classic.hash());
    assert_eq!(loaded.hash(), older.hash());
    assert_eq!(std::fs::read(&backup).unwrap(), original);
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].rules, Some(8));
    assert_eq!(
        candidates[0].content_id.as_deref(),
        Some(current.id.as_str())
    );
    assert_eq!(candidates[1].schema, Some(2));
    assert_eq!(candidates[1].rules, Some(2));
    let (resolved, mut restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved.hash(), classic.hash());
    assert_eq!(restored.hash(), older.hash());
    assert_eq!(
        std::fs::read(dir.join("mixed.json.before-recovery-1")).unwrap(),
        newer_bytes
    );
    let archive = dir.join("mixed.json.bak.format-v2-1");
    assert_eq!(std::fs::read(&archive).unwrap(), original);
    assert_eq!(
        persistence::recovery_candidates_bundled(&path)[0].schema,
        Some(3)
    );
    restored.advance(&resolved, 1).unwrap();
    persistence::save(&path, &restored, &resolved).unwrap();
    assert_eq!(std::fs::read(&archive).unwrap(), original);
    persistence::replay(&restored, &resolved).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unknown_pack_is_not_substituted_by_a_similar_bundled_roster() {
    let mut custom = Content::classic_bundled().unwrap();
    custom.id = "custom-unbundled-2000".into();
    let world = World::new(&custom, CountryId("GBR".into()), 17).unwrap();
    let path = std::env::temp_dir().join(format!("geosim-unbundled-{}.json", std::process::id()));
    persistence::save(&path, &world, &custom).unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(
        persistence::load_bundled(&path)
            .unwrap_err()
            .contains("--content")
    );
    assert_eq!(
        persistence::load(&path, &custom).unwrap().hash(),
        world.hash()
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn preserved_market_pack_replays_and_recovers_without_financing_migration() {
    let market = Content::market_bundled().unwrap();
    let mut original = World::new(&market, CountryId("GBR".into()), 2000).unwrap();
    original.advance(&market, 360).unwrap();
    assert_eq!(original.hash(), 0x8ef1c8a84552a107);
    let current = Content::bundled().unwrap();
    let mut newer = World::new(&current, original.player.clone(), 42).unwrap();
    newer
        .issue(
            &current,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
    let dir = std::env::temp_dir().join(format!("geosim-market-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mixed.json");
    persistence::save(&path, &newer, &current).unwrap();
    let current_bytes = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let market_bytes = persistence::encode(&original, &market).unwrap();
    std::fs::write(&backup, &market_bytes).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].rules, Some(8));
    assert_eq!(candidates[0].schema, Some(9));
    assert_eq!(candidates[1].rules, Some(3));
    assert_eq!(candidates[1].schema, Some(4));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, market);
    assert_eq!(restored.hash(), original.hash());
    assert!(restored.countries.values().all(|c| c.financing.is_none()));
    assert_eq!(
        std::fs::read(dir.join("mixed.json.before-recovery-1")).unwrap(),
        current_bytes
    );
    assert_eq!(std::fs::read(&backup).unwrap(), market_bytes);
    assert!(persistence::load(&path, &current).is_err());
    persistence::replay(&restored, &resolved).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn preserved_financing_pack_keeps_its_golden_and_pending_loan_when_restored_from_current_rules() {
    let finance = Content::finance_bundled().unwrap();
    let mut original = World::new(&finance, CountryId("GBR".into()), 2000).unwrap();
    original.advance(&finance, 360).unwrap();
    assert_eq!(original.hash(), 0x9bcec519bfc3c4f4);
    let mut pending = World::new(&finance, original.player.clone(), 2000).unwrap();
    pending
        .issue(
            &finance,
            Action::Borrow {
                amount: Money(16655),
            },
        )
        .unwrap();
    let current = Content::bundled().unwrap();
    let current_world = World::new(&current, original.player.clone(), 42).unwrap();
    let dir = std::env::temp_dir().join(format!("geosim-finance-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mixed.json");
    persistence::save(&path, &current_world, &current).unwrap();
    let current_bytes = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let old_bytes = persistence::encode(&pending, &finance).unwrap();
    std::fs::write(&backup, &old_bytes).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].rules, Some(8));
    assert_eq!(candidates[0].schema, Some(9));
    assert_eq!(candidates[1].rules, Some(4));
    assert_eq!(candidates[1].schema, Some(5));
    let (resolved, mut restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, finance);
    assert!(restored.player_country().constitution.is_none());
    assert!(
        restored
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .pending
            .is_some()
    );
    restored.advance(&resolved, 14).unwrap();
    assert_eq!(restored.hash(), 0xd7d347d722332a8a);
    assert_eq!(
        std::fs::read(dir.join("mixed.json.before-recovery-1")).unwrap(),
        current_bytes
    );
    assert_eq!(std::fs::read(&backup).unwrap(), old_bytes);
    persistence::replay(&restored, &resolved).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn earlier_constitutional_pending_request_recovers_without_policy_lifecycle_migration() {
    let earlier = Content::constitution_bundled().unwrap();
    let mut pending = World::new(&earlier, CountryId("GBR".into()), 17).unwrap();
    pending
        .issue(
            &earlier,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
    pending.advance(&earlier, 49).unwrap();
    pending
        .issue(
            &earlier,
            Action::Reconstitute {
                target: Regime::SingleParty,
                contested: false,
            },
        )
        .unwrap();
    let current = Content::bundled().unwrap();
    let current_world = World::new(&current, pending.player.clone(), 42).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "geosim-constitution-recovery-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mixed.json");
    persistence::save(&path, &current_world, &current).unwrap();
    let current_bytes = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let earlier_bytes = persistence::encode(&pending, &earlier).unwrap();
    std::fs::write(&backup, &earlier_bytes).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(9));
    assert_eq!(candidates[1].schema, Some(6));
    let (resolved, mut restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, earlier);
    assert!(restored.player_country().commitments.is_none());
    assert!(
        restored
            .player_country()
            .constitution
            .as_ref()
            .unwrap()
            .pending
            .is_some()
    );
    restored.advance(&resolved, 90).unwrap();
    assert_eq!(restored.hash(), 0x1615216df479c2ba);
    assert_eq!(std::fs::read(&backup).unwrap(), earlier_bytes);
    assert_eq!(
        std::fs::read(dir.join("mixed.json.before-recovery-1")).unwrap(),
        current_bytes
    );
    persistence::replay(&restored, &resolved).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn published_commitment_starter_keeps_its_no_order_campaign_golden() {
    let content =
        Content::from_json(include_str!("../../../data/initial-2000-commitments.json")).unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 2000).unwrap();
    world.advance(&content, 360).unwrap();
    assert_eq!(world.hash(), 0xdaf70b17cc0d2ff0);
    persistence::replay(&world, &content).unwrap();
}

#[test]
fn earlier_commitment_amendment_recovers_without_new_preparation_delays() {
    let earlier = Content::commitment_bundled().unwrap();
    let mut pending = World::new(&earlier, CountryId("GBR".into()), 17).unwrap();
    pending
        .issue(
            &earlier,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30,
            },
        )
        .unwrap();
    pending.advance(&earlier, 3).unwrap();
    pending
        .issue(
            &earlier,
            Action::AmendCommitment {
                request: CommitmentId(1),
                value: 32,
            },
        )
        .unwrap();
    assert_eq!(pending.hash(), 0xbb5fb19faa310498);
    let current = Content::bundled().unwrap();
    let current_world = World::new(&current, pending.player.clone(), 42).unwrap();
    let dir =
        std::env::temp_dir().join(format!("geosim-commitment-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mixed.json");
    persistence::save(&path, &current_world, &current).unwrap();
    let current_bytes = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let earlier_bytes = persistence::encode(&pending, &earlier).unwrap();
    std::fs::write(&backup, &earlier_bytes).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(9));
    assert_eq!(candidates[1].schema, Some(7));
    let (resolved, mut restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, earlier);
    assert!(restored.player_country().cabinet.is_none());
    assert_eq!(restored.player_country().projects[0].due_day, 51);
    restored.advance(&resolved, 78).unwrap();
    assert_eq!(restored.hash(), 0x6e1b17c01398b1ec);
    assert_eq!(std::fs::read(&backup).unwrap(), earlier_bytes);
    assert_eq!(
        std::fs::read(dir.join("mixed.json.before-recovery-1")).unwrap(),
        current_bytes
    );
    persistence::replay(&restored, &resolved).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn published_cabinet_starter_keeps_its_no_order_campaign_golden() {
    let content =
        Content::from_json(include_str!("../../../data/initial-2000-cabinet.json")).unwrap();
    assert_eq!(content.cabinet.as_ref().unwrap().portfolios.len(), 45);
    assert_eq!(content.cabinet.as_ref().unwrap().candidates.len(), 90);
    let mut world = World::new(&content, CountryId("GBR".into()), 2000).unwrap();
    world.advance(&content, 360).unwrap();
    assert_eq!(world.hash(), 0xbec8276e0de88b7d);
    persistence::replay(&world, &content).unwrap();
}

#[test]
fn earlier_pending_minister_recovers_without_electoral_rules_or_changed_consent() {
    let earlier = Content::cabinet_bundled().unwrap();
    let mut pending = World::new(&earlier, CountryId("GBR".into()), 17).unwrap();
    pending
        .issue(
            &earlier,
            Action::Appoint {
                office: OfficeId("portfolio:GBR:supply".into()),
                person: PersonId("generated:GBR:supply:professional".into()),
            },
        )
        .unwrap();
    assert_eq!(pending.hash(), 0x27b6cdc4d32c00b3);
    let current = Content::bundled().unwrap();
    let world = World::new(&current, pending.player.clone(), 42).unwrap();
    let dir = std::env::temp_dir().join(format!("geosim-cabinet-recovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mixed.json");
    persistence::save(&path, &world, &current).unwrap();
    let current_bytes = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let earlier_bytes = persistence::encode(&pending, &earlier).unwrap();
    std::fs::write(&backup, &earlier_bytes).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(9));
    assert_eq!(candidates[1].schema, Some(8));
    let (resolved, mut restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, earlier);
    assert!(restored.player_country().electoral_authority.is_none());
    restored.advance(&resolved, 14).unwrap();
    assert_eq!(restored.hash(), 0x8061566401cf6588);
    assert_eq!(
        restored.policy_preparation_days(&resolved, &restored.player, "energy"),
        4
    );
    assert_eq!(std::fs::read(&backup).unwrap(), earlier_bytes);
    assert_eq!(
        std::fs::read(dir.join("mixed.json.before-recovery-1")).unwrap(),
        current_bytes
    );
    persistence::replay(&restored, &resolved).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn default_early_poll_preserves_a_retained_cabinet_and_its_commissioned_programme() {
    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    world
        .issue(
            &content,
            Action::Appoint {
                office: OfficeId("portfolio:GBR:supply".into()),
                person: PersonId("generated:GBR:supply:professional".into()),
            },
        )
        .unwrap();
    world.advance(&content, 365).unwrap();
    world
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    let due = world
        .player_country()
        .projects
        .iter()
        .find(|p| p.policy == "energy")
        .unwrap()
        .due_day;
    let government = world.player_country().government_number;
    world.issue(&content, Action::RequestElection).unwrap();
    world.advance(&content, 67).unwrap();
    assert_eq!(world.player_country().government_number, government);
    assert_eq!(
        world
            .player_country()
            .electoral_authority
            .as_ref()
            .unwrap()
            .polls
            .len(),
        1
    );
    assert_eq!(
        world.policy_preparation_days(&content, &world.player, "energy"),
        4
    );
    assert_eq!(
        world
            .player_country()
            .projects
            .iter()
            .find(|p| p.policy == "energy")
            .unwrap()
            .due_day,
        due
    );
    assert_eq!(world.player_country().energy_capacity, 90);
    world.advance(&content, due - world.day).unwrap();
    assert_eq!(world.player_country().energy_capacity, 105);
    persistence::replay(&world, &content).unwrap();
}
