use sim_core::*;
#[test]
fn the_territorial_starter_retains_all_reference_data_with_explicit_authored_local_definitions() {
    let c = Content::bundled().unwrap();
    let old = Content::observed_bundled().unwrap();
    assert_eq!(c.rules, RULES_VERSION);
    assert_eq!(c.rules, 10);
    assert_eq!(c.schema, 10);
    assert_eq!(c.countries, old.countries);
    assert_eq!(c.policies, old.policies);
    assert_eq!(c.facts, old.facts);
    assert_eq!(c.identities, old.identities);
    assert_eq!(c.markets, old.markets);
    assert_eq!(c.finance, old.finance);
    assert_eq!(c.constitutions, old.constitutions);
    assert_eq!(c.commitments, old.commitments);
    assert_eq!(c.cabinet, old.cabinet);
    assert_eq!(c.elections, old.elections);
    let locals = c.subnational.as_ref().unwrap();
    let rules = c.territorial.as_ref().unwrap();
    assert_eq!(locals.charters.len(), 15);
    for country in &c.countries {
        let charter = locals.charter(&country.id).unwrap();
        assert_eq!(charter.regions.len(), 2);
        assert_eq!(
            charter
                .regions
                .iter()
                .map(|r| r.population_share_bps)
                .sum::<i64>(),
            10000
        );
        assert_eq!(rules.ai_targets[&country.id], charter.form);
        assert_eq!(
            rules.apply_template(charter, &charter.form).unwrap(),
            charter.regions
        );
        assert!(
            charter
                .regions
                .iter()
                .all(|r| r.name.starts_with("Authored "))
        );
    }
    assert_eq!(rules.source, locals.source);
    assert_ne!(rules.source, old.elections.as_ref().unwrap().source);
    assert!(
        c.sources
            .iter()
            .any(|s| s.id == rules.source && s.quality == "authored_scenario_model")
    );
    assert!(
        c.description
            .contains("not historical administrative units")
    );
    for pack in [
        c,
        old,
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
fn default_local_grants_recover_without_migrating_a_preserved_observed_campaign() {
    let c = Content::bundled().unwrap();
    let old = Content::observed_bundled().unwrap();
    let mut w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
    let region = RegionId("region:GBR:regional".into());
    w.issue(
        &c,
        Action::RegionalOrder {
            region,
            kind: RegionalOrderKind::Grant { boost: 1, days: 30 },
        },
    )
    .unwrap();
    let mut earlier = World::new(&old, CountryId("PRK".into()), 17).unwrap();
    earlier.advance(&old, 1).unwrap();
    assert_eq!(earlier.hash(), 0xd0e75a0a36bdf7cf);
    let dir =
        std::env::temp_dir().join(format!("geosim-territorial-starter-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("campaign.json");
    persistence::save(&path, &w, &c).unwrap();
    let current_bytes = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let old_bytes = persistence::encode(&earlier, &old).unwrap();
    std::fs::write(&backup, &old_bytes).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(11));
    assert_eq!(candidates[1].schema, Some(9));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, old);
    assert_eq!(restored, earlier);
    assert_eq!(std::fs::read(&backup).unwrap(), old_bytes);
    assert_eq!(
        std::fs::read(dir.join("campaign.json.before-recovery-1")).unwrap(),
        current_bytes
    );
    persistence::replay(&restored, &resolved).unwrap();
    let bytes = persistence::encode(&w, &c).unwrap();
    let mut loaded = persistence::decode(&bytes, &c).unwrap();
    w.advance(&c, 14).unwrap();
    loaded.advance(&c, 14).unwrap();
    assert_eq!(w, loaded);
    assert_eq!(
        w.player_country().subnational.as_ref().unwrap().orders[0].status,
        RegionalOrderStatus::Implementing
    );
    persistence::replay(&w, &c).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
