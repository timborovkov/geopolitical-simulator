use sim_core::*;

#[test]
fn preserved_interest_preserves_the_previous_roster_and_resolves_all_sixteen_packs() {
    let current = Content::interest_bundled().unwrap();
    let old = Content::cash_bundled().unwrap();
    assert_eq!((current.schema, current.rules), (17, 17));
    assert_eq!(current.countries.len(), 15);
    assert_eq!(current.sources.len(), old.sources.len() + 1);
    assert_eq!(
        &current.sources[..old.sources.len()],
        old.sources.as_slice()
    );
    let finance = current.finance.as_ref().unwrap();
    assert_eq!(
        finance.interest_accrual,
        Some(InterestAccrualRule::Actual365Carry)
    );
    assert_eq!(finance.maturity_days, Some(DaySpan::new(365)));
    assert!(
        current
            .sources
            .iter()
            .any(|s| s.id == finance.source && s.quality == "authored_scenario_model")
    );
    let mut normalized = current.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    let f = normalized.finance.as_mut().unwrap();
    f.interest_accrual = None;
    f.source = old.finance.as_ref().unwrap().source.clone();
    f.source_version = old.finance.as_ref().unwrap().source_version.clone();
    assert_eq!(
        normalized, old,
        "only the authored interest convention and metadata may change"
    );
    for pack in [
        current,
        old,
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
fn all_seeded_governments_accrue_actual_principal_days_and_resume_after_manual_redemption() {
    let content = Content::interest_bundled().unwrap();
    for d in &content.countries {
        let mut w = World::new(&content, d.id.clone(), 17).unwrap();
        let opening = w.player_country().debt;
        w.advance(&content, sim_core::DaySpan::new(30)).unwrap();
        let s = w
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .interest_accrual
            .as_ref()
            .unwrap();
        assert_eq!(s.principal_days, opening.0 as u64 * 30, "{}", d.id.0);
        assert_eq!(
            s.whole_assessed().0,
            s.paid.0 + w.player_country().arrears_external.0
        );
        let amount = Money(w.player_country().available_cash().0.min(100));
        assert!(amount.0 > 0);
        w.issue(&content, Action::Repay { amount }).unwrap();
        w.validate(&content).unwrap();
        let bytes = persistence::encode(&w, &content).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
            18
        );
        let mut resumed = persistence::decode(&bytes, &content).unwrap();
        w.advance(&content, sim_core::DaySpan::new(1)).unwrap();
        resumed
            .advance(&content, sim_core::DaySpan::new(1))
            .unwrap();
        assert_eq!(w, resumed);
        let s = w
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .interest_accrual
            .as_ref()
            .unwrap();
        assert_eq!(s.principal_days, opening.0 as u64 * 31 - amount.0 as u64);
        assert_eq!(
            s.last.as_ref().unwrap().principal,
            Money(opening.0 - amount.0)
        );
        assert_eq!(
            s.whole_assessed().0,
            s.paid.0 + w.player_country().arrears_external.0
        );
        assert_eq!(
            w.player_country().budget.expected_cash(),
            w.player_country().accounts.treasury.0
        );
        assert_eq!(persistence::replay(&w, &content).unwrap(), w);
    }
}

#[test]
fn mixed_formats_18_and_17_keep_the_previous_cash_recovery_golden_and_exact_originals() {
    let old = Content::cash_bundled().unwrap();
    let mut prior = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    let amount = prior.player_country().available_cash();
    prior.issue(&old, Action::Repay { amount }).unwrap();
    let request = CashHoldId(prior.next_command.0);
    prior
        .issue(
            &old,
            Action::HoldCash {
                target: Money(333),
                days: sim_core::DaySpan::new(90),
            },
        )
        .unwrap();
    prior.advance(&old, sim_core::DaySpan::new(1)).unwrap();
    prior.issue(&old, Action::ReleaseCash { request }).unwrap();
    prior
        .issue(
            &old,
            Action::Enact {
                policy: "income_tax".into(),
                value: 31,
            },
        )
        .unwrap();
    prior.advance(&old, sim_core::DaySpan::new(108)).unwrap();
    assert_eq!(prior.hash(), 0xbb0378bc7ed563a4);
    let content = Content::interest_bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    world.advance(&content, sim_core::DaySpan::new(3)).unwrap();
    let directory =
        std::env::temp_dir().join(format!("geosim-interest-starter-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &world, &content).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(18));
    assert_eq!(candidates[1].schema, Some(17));
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
    world.advance(&content, sim_core::DaySpan::new(40)).unwrap();
    resumed
        .advance(&content, sim_core::DaySpan::new(40))
        .unwrap();
    assert_eq!(world, resumed);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    let fixture = Content::from_json(include_str!(
        "../../../data/synthetic-interest-accrual-test.json"
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
