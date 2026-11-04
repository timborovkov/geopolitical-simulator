use sim_core::*;

#[test]
fn default_dated_loans_preserve_the_entire_previous_roster_and_resolve_all_fourteen_packs() {
    let content = Content::bundled().unwrap();
    let old = Content::national_fiscal_bundled().unwrap();
    assert_eq!(content.rules, RULES_VERSION);
    assert_eq!((content.schema, content.rules), (15, 15));
    assert_eq!(content.finance.as_ref().unwrap().maturity_days, Some(365));
    assert_eq!(content.sources.len(), old.sources.len() + 1);
    assert_eq!(
        &content.sources[..old.sources.len()],
        old.sources.as_slice()
    );
    assert!(
        content
            .sources
            .iter()
            .any(|source| source.id == "authored-dated-loan-principal-v1"
                && source.quality == "authored_scenario_model")
    );
    let mut normalized = content.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    let finance = normalized.finance.as_mut().unwrap();
    finance.maturity_days = None;
    finance.source = old.finance.as_ref().unwrap().source.clone();
    finance.source_version = old.finance.as_ref().unwrap().source_version.clone();
    assert_eq!(
        normalized, old,
        "only authored new-loan terms and metadata may change"
    );
    for pack in [
        content,
        old,
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
fn each_seeded_government_can_issue_a_dated_loan_through_its_ordinary_authority_and_replay_maturity()
 {
    let content = Content::bundled().unwrap();
    for definition in &content.countries {
        let mut world = World::new(&content, definition.id.clone(), 17).unwrap();
        let amount = Money((definition.gdp / 100).max(1));
        for _ in 0..4 {
            if world
                .preview_borrowing(&content, &world.player, amount)
                .is_ok()
            {
                break;
            }
            let support = world
                .preview(&content, &world.player, "coalition", 1)
                .unwrap();
            world
                .issue(
                    &content,
                    Action::Enact {
                        policy: "coalition".into(),
                        value: 1,
                    },
                )
                .unwrap();
            world.advance(&content, support.delay).unwrap();
        }
        let preview = world
            .preview_borrowing(&content, &world.player, amount)
            .unwrap();
        let id = LoanId(world.next_command);
        world.issue(&content, Action::Borrow { amount }).unwrap();
        world.advance(&content, preview.consent_days).unwrap();
        let loan = world
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .instruments
            .iter()
            .find(|loan| loan.id == id)
            .unwrap();
        assert_eq!(loan.issued_day, world.day);
        assert_eq!(loan.maturity_day, Some(world.day + 365));
        assert_eq!(loan.maturity_repaid, Some(Money(0)));
        assert!(
            world
                .player_country()
                .financing
                .as_ref()
                .unwrap()
                .instruments[0]
                .maturity_day
                .is_none()
        );
        world.advance(&content, 364).unwrap();
        assert_eq!(
            world
                .player_country()
                .financing
                .as_ref()
                .unwrap()
                .instruments
                .iter()
                .find(|loan| loan.id == id)
                .unwrap()
                .outstanding,
            amount
        );
        let before = world.player_country().debt.0;
        world.advance(&content, 1).unwrap();
        let loan = world
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .instruments
            .iter()
            .find(|loan| loan.id == id)
            .unwrap();
        let paid = loan.maturity_repaid.unwrap().0;
        assert!((0..=amount.0).contains(&paid));
        assert_eq!(loan.outstanding.0, amount.0 - paid);
        assert_eq!(world.player_country().debt.0, before - paid);
        assert_eq!(world.player_country().budget.principal_repaid, Money(paid));
        assert_eq!(
            world.player_country().budget.expected_cash(),
            world.player_country().accounts.treasury.0
        );
        world.validate(&content).unwrap();
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
}
#[test]
fn mixed_format16_and_preserved_format15_recovery_retains_original_bytes_and_exact_content() {
    let content = Content::bundled().unwrap();
    let old = Content::national_fiscal_bundled().unwrap();
    let mut prior = World::new(&old, CountryId("FRA".into()), 17).unwrap();
    prior.advance(&old, 138).unwrap();
    assert_eq!(prior.hash(), 0x4be2fd07e49920b2);
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    world.advance(&content, 1).unwrap();
    world
        .issue(
            &content,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
    let directory =
        std::env::temp_dir().join(format!("geosim-dated-debt-starter-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &world, &content).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(16));
    assert_eq!(candidates[1].schema, Some(15));
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
    world.advance(&content, 380).unwrap();
    resumed.advance(&content, 380).unwrap();
    assert_eq!(resumed, world);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    let fixture = Content::from_json(include_str!(
        "../../../data/synthetic-debt-maturities-test.json"
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
