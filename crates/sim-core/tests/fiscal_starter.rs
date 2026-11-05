use sim_core::*;
fn targets(c: &Country) -> FiscalTargets {
    FiscalTargets {
        income_tax: if c.tax < 45 { c.tax + 1 } else { 44 },
        services: if c.services < 18 { c.services + 1 } else { 17 },
        welfare: if c.welfare > 0 { c.welfare - 1 } else { 1 },
    }
}
#[test]
fn the_seeded_fiscal_roster_preserves_all_prior_data_and_each_country_can_deliver_a_funded_budget()
{
    let c = Content::fiscal_bundled().unwrap();
    let old = Content::undertaking_bundled().unwrap();
    assert_eq!(c.rules, 12);
    assert_eq!(c.schema, 12);
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
    assert_eq!(c.undertakings, old.undertakings);
    let rules = c.fiscal_plans.as_ref().unwrap();
    assert!(
        c.sources
            .iter()
            .any(|s| s.id == rules.source && s.quality == "authored_scenario_model")
    );
    assert!(c.description.contains("not historical budget laws"));
    for country in &c.countries {
        let mut w = World::new(&c, country.id.clone(), 17).unwrap();
        let t = targets(w.player_country());
        if let Err(reason) = w.preview_budget(&c, &w.player, 2000, &t) {
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
        let preview = w.preview_budget(&c, &w.player, 2000, &t).unwrap();
        let before = w.player_country().accounts.treasury;
        let id = BudgetBillId(w.next_command);
        w.issue(
            &c,
            Action::ProposeBudget {
                year: 2000,
                targets: t.clone(),
            },
        )
        .unwrap();
        assert_eq!(
            w.player_country().accounts.treasury.0,
            before.0 - preview.fee.0
        );
        assert_eq!(
            w.player_country().committed_cash,
            preview.implementation_cost
        );
        w.advance(&c, preview.review_day - w.day).unwrap();
        let bill = w
            .player_country()
            .fiscal_plans
            .as_ref()
            .unwrap()
            .bills
            .iter()
            .find(|b| b.id == id)
            .unwrap();
        assert_eq!(bill.status, BudgetBillStatus::Enacted, "{}", country.id.0);
        assert_eq!(bill.programmes.len(), 3);
        let due = w
            .player_country()
            .projects
            .iter()
            .map(|p| p.due_day)
            .max()
            .unwrap();
        w.advance(&c, due - w.day).unwrap();
        assert_eq!(
            FiscalTargets::current(w.player_country()),
            t,
            "{}",
            country.id.0
        );
        w.validate(&c).unwrap();
        assert_eq!(persistence::replay(&w, &c).unwrap(), w);
    }
    for pack in [
        c,
        old,
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
fn a_pending_default_budget_recovers_without_changing_the_preserved_shared_undertaking() {
    let c = Content::fiscal_bundled().unwrap();
    let old = Content::undertaking_bundled().unwrap();
    let id = CountryId("GBR".into());
    let mut w = World::new(&c, id.clone(), 17).unwrap();
    let t = targets(w.player_country());
    w.issue(
        &c,
        Action::ProposeBudget {
            year: 2000,
            targets: t.clone(),
        },
    )
    .unwrap();
    let due = w.preview_budget(&c, &w.player, 2000, &t).unwrap_err();
    assert!(due.contains("pending"));
    let mut prior = World::new(&old, id, 17).unwrap();
    prior
        .issue(
            &old,
            Action::DeclareUndertaking {
                template: "energy-pledge".into(),
                project: None,
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
    prior
        .issue(
            &old,
            Action::DeclareUndertaking {
                template: "project-deadline".into(),
                project: Some(CommitmentId(1)),
            },
        )
        .unwrap();
    assert_eq!(prior.hash(), 0x4e9a67357ae59c65);
    assert_eq!(prior.player_country().budget.opening_day_settled, None);
    assert_eq!(w.player_country().budget.opening_day_settled, Some(false));
    let dir = std::env::temp_dir().join(format!("geosim-fiscal-starter-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("campaign.json");
    persistence::save(&path, &w, &c).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(13));
    assert_eq!(candidates[1].schema, Some(12));
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
    w.advance(&c, 366).unwrap();
    loaded.advance(&c, 366).unwrap();
    assert_eq!(loaded, w);
    assert_eq!(FiscalTargets::current(w.player_country()), t);
    assert_eq!(w.player_country().budget.opening_day_settled, Some(true));
    assert_eq!(w.player_country().budget_reviews.len(), 1);
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
    std::fs::remove_dir_all(dir).unwrap();
}
