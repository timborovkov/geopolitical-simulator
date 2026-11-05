use sim_core::*;

#[test]
fn preserved_technology_preserves_the_complete_roster_and_resolves_all_seventeen_packs() {
    let current = Content::technology_bundled().unwrap();
    let old = Content::interest_bundled().unwrap();
    assert_eq!((current.schema, current.rules), (18, 18));
    assert_eq!(current.countries.len(), 15);
    assert_eq!(current.sources.len(), old.sources.len() + 1);
    assert_eq!(
        &current.sources[..old.sources.len()],
        old.sources.as_slice()
    );
    let rules = current.technologies.as_ref().unwrap();
    assert_eq!(rules.definitions.len(), 2);
    let bookmark: CivilDate = current.start.parse().unwrap();
    for technology in &rules.definitions {
        assert_eq!(technology.available_from, bookmark);
        assert_eq!(technology.eligible_countries.len(), current.countries.len());
        assert!(
            current
                .countries
                .iter()
                .all(|d| technology.eligible_countries.contains(&d.id))
        );
        assert!(
            current
                .sources
                .iter()
                .any(|s| s.id == technology.source && s.quality == "authored_scenario_model")
        );
    }
    let mut normalized = current.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    normalized.technologies = None;
    assert_eq!(
        normalized, old,
        "only attributed technology eligibility and pack metadata may change"
    );
    for pack in [
        current,
        old,
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
fn starting_eligibility_grants_no_resources_or_free_delivery_across_all_fifteen_governments() {
    let content = Content::technology_bundled().unwrap();
    let old = Content::interest_bundled().unwrap();
    for d in &content.countries {
        let mut current = World::new(&content, d.id.clone(), 17).unwrap();
        let mut prior = World::new(&old, d.id.clone(), 17).unwrap();
        for policy in ["energy", "agriculture"] {
            assert!(
                current
                    .policy_technology_eligibility(&content, &current.player, policy)
                    .unwrap()
                    .iter()
                    .all(TechnologyEligibility::available)
            );
        }
        if let Err(reason) = current.preview(&content, &current.player, "energy", 1) {
            assert_eq!(
                prior.preview(&old, &prior.player, "energy", 1).unwrap_err(),
                reason
            );
            let support = prior.preview(&old, &prior.player, "coalition", 1).unwrap();
            current
                .issue(
                    &content,
                    Action::Enact {
                        policy: "coalition".into(),
                        value: 1,
                    },
                )
                .unwrap();
            prior
                .issue(
                    &old,
                    Action::Enact {
                        policy: "coalition".into(),
                        value: 1,
                    },
                )
                .unwrap();
            current.advance(&content, support.delay).unwrap();
            prior.advance(&old, support.delay).unwrap();
        }
        let preview = current
            .preview(&content, &current.player, "energy", 1)
            .unwrap();
        let baseline = prior.preview(&old, &prior.player, "energy", 1).unwrap();
        assert_eq!(
            (preview.cost, preview.delay, preview.upfront_payment),
            (baseline.cost, baseline.delay, baseline.upfront_payment)
        );
        current
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        prior
            .issue(
                &old,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        current.advance(&content, preview.delay + 30).unwrap();
        prior.advance(&old, baseline.delay + 30).unwrap();
        current.validate(&content).unwrap();
        prior.validate(&old).unwrap();
        let mut normalized = current.clone();
        normalized.rules = prior.rules;
        normalized.content_hash = prior.content_hash;
        assert_eq!(
            normalized, prior,
            "{} must retain accounts, capacity, delivery, opinions, AI commands and events when prerequisites already pass",
            d.id.0
        );
    }
    let mut restricted = content.clone();
    let country = restricted.countries[0].id.clone();
    restricted.technologies.as_mut().unwrap().definitions[0]
        .eligible_countries
        .retain(|id| id != &country);
    restricted.validate().unwrap();
    let mut world = World::new(&restricted, country, 17).unwrap();
    let hash = world.hash();
    assert!(
        world
            .preview(&restricted, &world.player, "energy", 1)
            .unwrap_err()
            .contains("not eligible")
    );
    assert!(world.policy_funding_plan(&restricted, "energy", 1).is_err());
    assert!(
        world
            .issue(
                &restricted,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1
                }
            )
            .is_err()
    );
    assert_eq!(world.hash(), hash);
}

#[test]
fn mixed_formats_19_and_18_preserve_the_previous_interest_golden_and_exact_recovery_bytes() {
    let old = Content::interest_bundled().unwrap();
    let mut prior = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    prior
        .issue(
            &old,
            Action::Borrow {
                amount: Money(10000),
            },
        )
        .unwrap();
    prior.advance(&old, 379).unwrap();
    assert_eq!(prior.hash(), 0x1d07acd01d53d45f);
    let content = Content::technology_bundled().unwrap();
    let mut current = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    current
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    current.advance(&content, 7).unwrap();
    let directory =
        std::env::temp_dir().join(format!("geosim-technology-starter-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &current, &content).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(19));
    assert_eq!(candidates[1].schema, Some(18));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, old);
    assert_eq!(restored, prior);
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(directory.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    assert_eq!(persistence::replay(&restored, &resolved).unwrap(), restored);
    let mut resumed = persistence::decode(&primary, &content).unwrap();
    current.advance(&content, 30).unwrap();
    resumed.advance(&content, 30).unwrap();
    assert_eq!(current, resumed);
    assert_eq!(persistence::replay(&current, &content).unwrap(), current);
    let fixture = Content::from_json(include_str!(
        "../../../data/synthetic-technology-eligibility-test.json"
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
