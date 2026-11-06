use sim_core::*;

#[test]
fn the_income_roster_preserves_prior_identity_and_delivers_tax_policy_under_every_government() {
    let c = Content::income_bundled().unwrap();
    let old = Content::fiscal_bundled().unwrap();
    assert_eq!((c.schema, c.rules), (13, 13));
    assert_eq!(c.countries, old.countries);
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
    assert_eq!(c.undertakings, old.undertakings);
    assert_eq!(c.fiscal_plans, old.fiscal_plans);
    let mut policies = c.policies.clone();
    let tax = policies.iter_mut().find(|p| p.id == "income_tax").unwrap();
    let prior_tax = old.policy(&"income_tax".into()).unwrap();
    tax.name = prior_tax.name.clone();
    tax.description = prior_tax.description.clone();
    assert_eq!(policies, old.policies);
    let rules = c.income_taxes.as_ref().unwrap();
    assert!(
        c.sources
            .iter()
            .any(|s| s.id == rules.source && s.quality == "authored_scenario_model")
    );
    assert!(c.description.contains("not historical filing law"));
    for country in &c.countries {
        let mut w = World::new(&c, country.id.clone(), 17).unwrap();
        assert_eq!(persistence::snapshot_schema(&w), 14);
        assert!(w.player_country().income_tax_report.is_none());
        w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
        let rate = if w.player_country().tax < 45 {
            w.player_country().tax + 1
        } else {
            44
        };
        if let Err(reason) = w.preview(&c, &w.player, &"income_tax".into(), rate) {
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
            w.advance(&c, support.delay).unwrap();
        }
        let preview = w
            .preview(&c, &w.player, &"income_tax".into(), rate)
            .unwrap();
        let settled = w.player_country().income_tax_report.clone();
        w.issue(
            &c,
            Action::Enact {
                policy: "income_tax".into(),
                value: rate,
            },
        )
        .unwrap();
        assert_eq!(w.player_country().income_tax_report, settled);
        w.advance(&c, preview.delay).unwrap();
        let country = w.player_country();
        let report = country.income_tax_report.as_ref().unwrap();
        assert_eq!(report.standard_rate, rate);
        assert_eq!(report.paid(), country.report.revenue);
        assert_eq!(
            report
                .groups
                .iter()
                .map(|g| g.assessment.gross.0)
                .sum::<i64>(),
            country.report.wages.0
        );
        let public = country
            .households
            .iter()
            .find(|h| h.id == "public")
            .unwrap();
        let assessed = report.groups.iter().find(|g| g.group == "public").unwrap();
        assert!(public.income.0 > assessed.assessment.gross.0);
        w.validate(&c).unwrap();
        assert_eq!(persistence::replay(&w, &c).unwrap(), w);
    }
    for pack in [
        c,
        old,
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
fn new_pending_tax_and_preserved_fiscal_saves_recover_without_relabeling_or_losing_reports() {
    let c = Content::income_bundled().unwrap();
    let old = Content::fiscal_bundled().unwrap();
    let id = CountryId("GBR".into());
    let mut prior = World::new(&old, id.clone(), 17).unwrap();
    prior
        .issue(
            &old,
            Action::ProposeBudget {
                year: 2000,
                targets: FiscalTargets {
                    income_tax: 29,
                    services: 10,
                    welfare: 3,
                },
            },
        )
        .unwrap();
    assert_eq!(prior.hash(), 0xc49e391e8a8a2b40);
    let mut w = World::new(&c, id, 17).unwrap();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    w.issue(
        &c,
        Action::Enact {
            policy: "income_tax".into(),
            value: 32,
        },
    )
    .unwrap();
    let dir = std::env::temp_dir().join(format!("geosim-income-starter-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("campaign.json");
    persistence::save(&path, &w, &c).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(14));
    assert_eq!(candidates[1].schema, Some(13));
    let (resolved, restored) = persistence::restore_bundled(&path, &backup).unwrap();
    assert_eq!(resolved, old);
    assert_eq!(restored, prior);
    assert_eq!(std::fs::read(&backup).unwrap(), older);
    assert_eq!(
        std::fs::read(dir.join("campaign.json.before-recovery-1")).unwrap(),
        primary
    );
    assert_eq!(persistence::replay(&restored, &resolved).unwrap(), restored);
    let mut loaded = persistence::decode(&primary, &c).unwrap();
    w.advance(&c, sim_core::DaySpan::new(366)).unwrap();
    loaded.advance(&c, sim_core::DaySpan::new(366)).unwrap();
    assert_eq!(loaded, w);
    assert_eq!(
        w.player_country()
            .income_tax_report
            .as_ref()
            .unwrap()
            .standard_rate,
        32
    );
    assert_eq!(w.player_country().budget_reviews.len(), 1);
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
    std::fs::remove_dir_all(dir).unwrap();
}
