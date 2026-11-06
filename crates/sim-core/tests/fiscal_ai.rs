use sim_core::*;

fn gap_content() -> (Content, CountryId, i32) {
    let mut content = Content::from_json(include_str!(
        "../../../data/synthetic-national-fiscal-ai-test.json"
    ))
    .unwrap();
    for d in &mut content.countries {
        d.energy = 120;
        d.food = 120;
    }
    content.constitutions.as_mut().unwrap().ai_targets.clear();
    content.territorial.as_mut().unwrap().ai_targets.clear();
    let foreign = CountryId("BOR".into());
    let rate = (10..=45)
        .find(|rate| {
            content
                .countries
                .iter_mut()
                .find(|d| d.id == foreign)
                .unwrap()
                .tax = *rate;
            let probe = World::new(&content, foreign.clone(), 17).unwrap();
            let forecast = probe.player_country().forecast(
                content.country(&foreign).unwrap(),
                content.income_taxes.as_ref(),
            );
            forecast.daily_revenue.0 > forecast.daily_spending.0 + forecast.daily_interest.0
                && probe
                    .funding_outlook(&content, 0, None)
                    .unwrap()
                    .daily_balance
                    .0
                    < 0
        })
        .expect("authored national gap beneath a consolidated surplus");
    (content, foreign, rate)
}

#[test]
fn fiscal_ai_responds_to_national_obligations_without_spending_local_cash_or_duplicating_programmes()
 {
    let (content, foreign, rate) = gap_content();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let targets = FiscalTargets {
        income_tax: (rate + 2).min(45),
        services: 8,
        welfare: 3,
    };
    world
        .preview_budget(&content, &foreign, 2000, &targets)
        .unwrap();
    world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
    let country = &world.countries[&foreign];
    let bills = &country.fiscal_plans.as_ref().unwrap().bills;
    assert_eq!(
        bills.len(),
        1,
        "a consolidated surplus must not hide a national funding gap"
    );
    assert_eq!(bills[0].status, BudgetBillStatus::Pending);
    assert_eq!(bills[0].targets, targets);
    assert_eq!(
        country.tax, rate,
        "submission must not operate the new rate instantly"
    );
    assert_eq!(
        country.fiscal_plans.as_ref().unwrap().reserved_cash(),
        bills[0].implementation_cost
    );
    assert!(
        country.committed_cash.0 >= bills[0].implementation_cost.0,
        "other concurrent programmes keep their own reservations"
    );
    assert!(world.journal.iter().any(|e| e.ai
        && e.command.issuer == foreign
        && matches!(e.command.action, Action::ProposeBudget { .. })));
    world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
    let country = &world.countries[&foreign];
    assert_eq!(
        country.fiscal_plans.as_ref().unwrap().bills.len(),
        1,
        "funded fiscal programmes must prevent duplicate packages"
    );
    assert_eq!(
        country.fiscal_plans.as_ref().unwrap().bills[0].status,
        BudgetBillStatus::Enacted
    );
    assert_eq!(
        country.tax, rate,
        "funding still precedes the ordinary delivery date"
    );
    assert_eq!(persistence::snapshot_schema(&world), 15);
    let bytes = persistence::encode(&world, &content).unwrap();
    assert_eq!(persistence::decode(&bytes, &content).unwrap(), world);
    let mut old_envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    old_envelope["schema"] = 14.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&old_envelope).unwrap(), &content)
            .unwrap_err()
            .contains("15")
    );
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn national_fiscal_ai_cannot_bypass_legal_backing_or_implementation_cash() {
    for block_backing in [true, false] {
        let (mut content, foreign, rate) = gap_content();
        if !block_backing {
            for policy in &mut content.policies {
                if matches!(policy.id.as_str(), "income_tax" | "services" | "welfare") {
                    policy.cost_bps = BasisPoints::new(1000);
                }
            }
            content.validate().unwrap();
        }
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        if block_backing {
            for field in [
                DeveloperField::LegislativeSupport,
                DeveloperField::EliteSupport,
                DeveloperField::CouncilSupport,
            ] {
                world
                    .issue_developer(&content, foreign.clone(), DeveloperEdit { field, value: 0 })
                    .unwrap();
            }
        }
        let targets = FiscalTargets {
            income_tax: (rate + 2).min(45),
            services: 8,
            welfare: 3,
        };
        assert!(
            world
                .preview_budget(&content, &foreign, 2000, &targets)
                .is_err()
        );
        world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
        assert!(
            world.countries[&foreign]
                .fiscal_plans
                .as_ref()
                .unwrap()
                .bills
                .is_empty()
        );
        assert!(!world.journal.iter().any(|e| e.ai
            && e.command.issuer == foreign
            && matches!(e.command.action, Action::ProposeBudget { .. })));
        world.validate(&content).unwrap();
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
}

fn preserved_delivery() -> (Content, World) {
    let content = Content::income_bundled().unwrap();
    assert_eq!(content.rules, 13);
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    world
        .issue(
            &content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 32,
            },
        )
        .unwrap();
    world.advance(&content, sim_core::DaySpan::new(78)).unwrap();
    assert_eq!(world.hash(), 0x3b708c82007c6193);
    (content, world)
}

#[test]
fn preserved_rules13_keeps_its_original_ai_commands_and_delivered_tax_save() {
    let (content, world) = preserved_delivery();
    assert_eq!(persistence::snapshot_schema(&world), 14);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn preserved_national_fiscal_reviews_preserve_the_entire_prior_roster_and_all_thirteen_packs() {
    let content = Content::national_fiscal_bundled().unwrap();
    let old = Content::income_bundled().unwrap();
    assert_eq!((content.schema, content.rules), (14, 14));
    assert_eq!(content.sources.len(), old.sources.len() + 1);
    assert_eq!(
        &content.sources[..old.sources.len()],
        old.sources.as_slice()
    );
    assert!(content.sources.iter().any(
        |s| s.id == "authored-national-fiscal-ai-v1" && s.quality == "authored_scenario_model"
    ));
    let mut normalized = content.clone();
    normalized.schema = old.schema;
    normalized.rules = old.rules;
    normalized.id = old.id.clone();
    normalized.name = old.name.clone();
    normalized.description = old.description.clone();
    normalized.sources = old.sources.clone();
    assert_eq!(
        normalized, old,
        "AI review changes must not alter country, tax or institutional data"
    );
    for pack in [
        content,
        old,
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
fn mixed_format15_and_preserved_format14_recovery_keeps_the_old_campaign_and_reports_exact() {
    let content = Content::national_fiscal_bundled().unwrap();
    let (old, prior) = preserved_delivery();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    world
        .issue(
            &content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 32,
            },
        )
        .unwrap();
    let dir = std::env::temp_dir().join(format!("geosim-fiscal-ai-starter-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("campaign.json");
    persistence::save(&path, &world, &content).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(15));
    assert_eq!(candidates[1].schema, Some(14));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, old);
    assert_eq!(restored, prior);
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(dir.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    assert_eq!(persistence::replay(&restored, &resolved).unwrap(), restored);
    let mut loaded = persistence::decode(&primary, &content).unwrap();
    world
        .advance(&content, sim_core::DaySpan::new(366))
        .unwrap();
    loaded
        .advance(&content, sim_core::DaySpan::new(366))
        .unwrap();
    assert_eq!(loaded, world);
    assert_eq!(
        world
            .player_country()
            .income_tax_report
            .as_ref()
            .unwrap()
            .standard_rate,
        32
    );
    assert_eq!(world.player_country().budget_reviews.len(), 1);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    std::fs::remove_dir_all(dir).unwrap();
}
