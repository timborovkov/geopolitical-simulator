use sim_core::*;

#[test]
fn preserved_cash_management_preserves_the_complete_prior_roster_and_resolves_fifteen_packs() {
    let content = Content::cash_bundled().unwrap();
    let old = Content::debt_bundled().unwrap();
    assert_eq!((content.schema, content.rules), (16, 16));
    let rules = content.cash_management.as_ref().unwrap();
    assert_eq!(
        (
            rules.max_target_bps.units_i64(),
            rules.max_days.units(),
            rules.cooldown_days.units(),
            rules.approval_cost
        ),
        (100, 90, 90, 3)
    );
    assert_eq!(content.sources.len(), old.sources.len() + 1);
    assert_eq!(
        &content.sources[..old.sources.len()],
        old.sources.as_slice()
    );
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
    normalized.cash_management = None;
    assert_eq!(
        normalized, old,
        "only the authored cash procedure and metadata may change"
    );
    for pack in [
        content,
        old,
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
fn seeded_cash_recovery_respects_actual_receipts_and_keeps_zero_receipt_targets_unfunded() {
    let content = Content::cash_bundled().unwrap();
    let (mut funded, mut no_receipts) = (0, 0);
    for d in &content.countries {
        let mut w = World::new(&content, d.id.clone(), 17).unwrap();
        let rate = (w.player_country().tax + 1).min(45);
        let (policy, value) = if w
            .preview(&content, &w.player, &"income_tax".into(), rate)
            .is_ok()
        {
            ("income_tax", rate)
        } else {
            ("coalition", 1)
        };
        let cost = w
            .preview(&content, &w.player, &policy.into(), value)
            .unwrap()
            .cost;
        let available = w.player_country().available_cash();
        w.issue(&content, Action::Repay { amount: available })
            .unwrap();
        assert_eq!(w.player_country().available_cash(), Money(0));
        assert!(
            w.preview(&content, &w.player, &policy.into(), value)
                .is_err()
        );
        let request = CashHoldId(w.next_command.0);
        let cash = w.player_country().accounts.treasury;
        let debt = w.player_country().debt;
        w.issue(
            &content,
            Action::HoldCash {
                target: cost,
                days: 90,
            },
        )
        .unwrap();
        assert_eq!(w.player_country().retained_cash(), Money(0));
        assert_eq!(w.player_country().accounts.treasury, cash);
        let mut receipts = 0;
        for _ in 0..89 {
            if w.player_country().retained_cash() == cost {
                break;
            }
            w.advance(&content, sim_core::DaySpan::new(1)).unwrap();
            receipts += w.player_country().report.revenue.0;
        }
        if receipts == 0 {
            no_receipts += 1;
            assert_eq!(w.player_country().retained_cash(), Money(0));
            assert_eq!(w.player_country().available_cash(), Money(0));
            assert!(
                w.preview(&content, &w.player, &policy.into(), value)
                    .is_err()
            );
            assert!(
                w.player_country().cash_management.as_ref().unwrap().records[0]
                    .funded_notice_day
                    .is_none()
            );
            w.advance(&content, sim_core::DaySpan::new(1)).unwrap();
            assert!(
                w.player_country()
                    .cash_management
                    .as_ref()
                    .unwrap()
                    .active()
                    .is_none()
            );
            assert_eq!(w.player_country().retained_cash(), Money(0));
            assert_eq!(persistence::replay(&w, &content).unwrap(), w);
            continue;
        }
        funded += 1;
        assert_eq!(
            w.player_country().retained_cash(),
            cost,
            "{} must actually collect its decision cost",
            d.id.0
        );
        assert_eq!(w.player_country().debt, debt, "retention must not borrow");
        let available = w.player_country().available_cash();
        let arrears = w.player_country().arrears;
        w.issue(&content, Action::ReleaseCash { request }).unwrap();
        assert_eq!(
            w.player_country().available_cash(),
            Money(available.0 + cost.0)
        );
        assert_eq!(w.player_country().arrears, arrears);
        let preview = w
            .preview(&content, &w.player, &policy.into(), value)
            .unwrap();
        w.issue(
            &content,
            Action::Enact {
                policy: policy.into(),
                value,
            },
        )
        .unwrap();
        assert_eq!(
            w.player_country().committed_cash,
            Money(cost.0 - preview.upfront_payment.0)
        );
        assert_eq!(
            w.player_country().budget.expected_cash(),
            w.player_country().accounts.treasury.0
        );
        let bytes = persistence::encode(&w, &content).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
            17
        );
        assert_eq!(persistence::decode(&bytes, &content).unwrap(), w);
        assert_eq!(persistence::replay(&w, &content).unwrap(), w);
    }
    assert_eq!((funded, no_receipts), (14, 1));
}

#[test]
fn mixed_format17_and_preserved_format16_recovery_keeps_originals_and_the_previous_golden() {
    let content = Content::cash_bundled().unwrap();
    let old = Content::debt_bundled().unwrap();
    let mut prior = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    prior
        .issue(
            &old,
            Action::Borrow {
                amount: Money(10000),
            },
        )
        .unwrap();
    prior.advance(&old, sim_core::DaySpan::new(379)).unwrap();
    assert_eq!(prior.hash(), 0xd532c2368aeeecca);
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    world
        .issue(
            &content,
            Action::HoldCash {
                target: Money(1000),
                days: 90,
            },
        )
        .unwrap();
    world.advance(&content, sim_core::DaySpan::new(3)).unwrap();
    let directory =
        std::env::temp_dir().join(format!("geosim-cash-starter-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("campaign.json");
    persistence::save(&path, &world, &content).unwrap();
    let primary = std::fs::read(&path).unwrap();
    let backup = path.with_extension("json.bak");
    let older = persistence::encode(&prior, &old).unwrap();
    std::fs::write(&backup, &older).unwrap();
    let candidates = persistence::recovery_candidates_bundled(&path);
    assert_eq!(candidates[0].schema, Some(17));
    assert_eq!(candidates[1].schema, Some(16));
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
    world.advance(&content, sim_core::DaySpan::new(90)).unwrap();
    resumed
        .advance(&content, sim_core::DaySpan::new(90))
        .unwrap();
    assert_eq!(world, resumed);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    let fixture = Content::from_json(include_str!(
        "../../../data/synthetic-cash-management-test.json"
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
