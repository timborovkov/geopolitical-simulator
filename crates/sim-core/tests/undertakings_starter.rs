use sim_core::*;
#[test]
fn the_undertaking_starter_preserves_all_country_observations_and_territorial_rights() {
    let c = Content::undertaking_bundled().unwrap();
    let old = Content::territorial_bundled().unwrap();
    assert_eq!(c.rules, 11);
    assert_eq!(c.schema, 11);
    assert_eq!(c.countries, old.countries);
    assert_eq!(c.policies, old.policies);
    assert_eq!(c.identities, old.identities);
    assert_eq!(c.facts, old.facts);
    assert_eq!(c.markets, old.markets);
    assert_eq!(c.finance, old.finance);
    assert_eq!(c.constitutions, old.constitutions);
    assert_eq!(c.commitments, old.commitments);
    assert_eq!(c.cabinet, old.cabinet);
    assert_eq!(c.elections, old.elections);
    assert_eq!(c.subnational, old.subnational);
    assert_eq!(c.territorial, old.territorial);
    let rules = c.undertakings.as_ref().unwrap();
    assert_eq!(rules.templates.len(), 3);
    assert!(
        c.sources
            .iter()
            .any(|s| s.id == rules.source && s.quality == "authored_scenario_model")
    );
    assert!(c.description.contains("not historical campaign promises"));
    for country in &c.countries {
        let mut w = World::new(&c, country.id.clone(), 17).unwrap();
        if let Err(reason) = w.preview(&c, &w.player, &"administration".into(), 1) {
            assert!(reason.contains("Negotiate"), "{}: {reason}", country.id.0);
            let support = w.preview(&c, &w.player, &"coalition".into(), 1).unwrap();
            w.issue(
                &c,
                Action::Enact {
                    policy: "coalition".into(),
                    value: 1,
                },
            )
            .unwrap();
            w.advance(&c, sim_core::DaySpan::new(support.delay))
                .unwrap();
        }
        w.issue(
            &c,
            Action::Enact {
                policy: "administration".into(),
                value: 1,
            },
        )
        .unwrap();
        let project = &w.player_country().projects[0];
        let p = w
            .preview_undertaking(&c, &w.player, "project-deadline", Some(project.id))
            .unwrap();
        assert!(
            p.expected_delivery.unwrap() <= p.deadline_day,
            "{} administration programme must fit the authored linked deadline",
            country.id.0
        );
    }
    for pack in [
        c,
        old,
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
fn compatible_undertakings_share_one_real_delivery_and_the_previous_roster_recovers_unchanged() {
    let c = Content::undertaking_bundled().unwrap();
    let old = Content::territorial_bundled().unwrap();
    let id = CountryId("GBR".into());
    let mut w = World::new(&c, id.clone(), 17).unwrap();
    w.issue(
        &c,
        Action::DeclareUndertaking {
            template: "energy-pledge".into(),
            project: None,
        },
    )
    .unwrap();
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    let source = w.player_country().projects[0].id;
    let due = w.player_country().projects[0].due_day;
    let initial = w.player_country().energy_capacity;
    w.issue(
        &c,
        Action::DeclareUndertaking {
            template: "project-deadline".into(),
            project: Some(source),
        },
    )
    .unwrap();
    assert_eq!(w.player_country().projects.len(), 1);
    let mut prior = World::new(&old, id, 17).unwrap();
    prior
        .issue(
            &old,
            Action::RegionalOrder {
                region: RegionId("region:GBR:regional".into()),
                kind: RegionalOrderKind::Grant { boost: 1, days: 30 },
            },
        )
        .unwrap();
    assert_eq!(prior.hash(), 0xfa090e3f40bb7662);
    let dir =
        std::env::temp_dir().join(format!("geosim-undertaking-starter-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("campaign.json");
    persistence::save(&path, &w, &c).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(12));
    assert_eq!(candidates[1].schema, Some(11));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, old);
    assert_eq!(restored, prior);
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(dir.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    persistence::replay(&restored, &resolved).unwrap();
    let mut loaded = persistence::decode(&primary, &c).unwrap();
    w.advance(&c, sim_core::DaySpan::new(due)).unwrap();
    loaded.advance(&c, sim_core::DaySpan::new(due)).unwrap();
    assert_eq!(w, loaded);
    assert_eq!(w.player_country().energy_capacity, initial + 15);
    let state = w.player_country().undertakings.as_ref().unwrap();
    assert_eq!(state.credibility, 70);
    assert!(
        state
            .records
            .iter()
            .all(|r| r.status == UndertakingStatus::Fulfilled { evidence: source })
    );
    w.validate(&c).unwrap();
    persistence::replay(&w, &c).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
