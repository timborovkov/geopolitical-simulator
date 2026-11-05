use sim_core::{persistence, *};
fn setup(id: &str) -> (Content, World) {
    let content = Content::bundled().unwrap();
    let world = World::new(&content, CountryId(id.into()), 2000).unwrap();
    (content, world)
}
#[test]
fn content_validates_roster_regimes_and_references() {
    let c = Content::bundled().unwrap();
    assert_eq!(c.countries.len(), 15);
    assert_eq!(c.policies.len(), 7);
    assert!(c.countries.iter().any(|d| d.regime == Regime::Military));
    let mut invalid = c.clone();
    invalid.countries[0].parties[0].seats += 1;
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.countries[0].economic_source = "absent".into();
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.countries[0].id = c.countries[1].id.clone();
    assert!(invalid.validate().is_err());
    let mut invalid = c.clone();
    invalid.schema = 99;
    assert!(invalid.validate().is_err());
}
#[test]
fn gregorian_calendar_includes_leap_day() {
    assert_eq!(date(0), "2000-01-01");
    assert_eq!(date(59), "2000-02-29");
    assert_eq!(date(60), "2000-03-01");
    assert_eq!(date(366), "2001-01-01");
    assert_eq!(date(1827), "2005-01-01");
    assert_eq!(date(36525), "2100-01-01");
}
#[test]
fn rejected_commands_never_mutate_state() {
    let (content, mut w) = setup("GBR");
    let before = w.hash();
    for action in [
        Action::Borrow { amount: Money(-10) },
        Action::Borrow {
            amount: Money(i64::MAX),
        },
        Action::Repay {
            amount: Money(i64::MAX),
        },
        Action::Enact {
            policy: "missing".into(),
            value: 1,
        },
        Action::Enact {
            policy: "income_tax".into(),
            value: 99,
        },
    ] {
        assert!(w.issue(&content, action).is_err());
        assert_eq!(w.hash(), before);
    }
    let foreign = w.command(CountryId("CHN".into()), Action::Borrow { amount: Money(1) });
    assert!(w.execute(&content, foreign).is_err());
    assert_eq!(w.hash(), before);
    let mut stale = w.command(w.player.clone(), Action::Borrow { amount: Money(1) });
    stale.day = 1;
    assert!(w.execute(&content, stale).is_err());
    assert_eq!(w.hash(), before);
    let command = w.command(w.player.clone(), Action::Borrow { amount: Money(1) });
    w.execute(&content, command.clone()).unwrap();
    let after = w.hash();
    assert!(w.execute(&content, command).is_err());
    assert_eq!(w.hash(), after);
}
#[test]
fn legal_routes_apply_to_all_regimes_and_support_can_be_negotiated() {
    let content = Content::bundled().unwrap();
    for d in &content.countries {
        let mut w = World::new(&content, d.id.clone(), 42).unwrap();
        let c = w.countries.get_mut(&d.id).unwrap();
        c.legislative_support = 20;
        c.elite_support = 20;
        assert!(w.preview(&content, &d.id, &"energy".into(), 1).is_err());
        let delivery_day = w
            .preview(&content, &d.id, &"coalition".into(), 1)
            .unwrap()
            .delay;
        w.issue(
            &content,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
        w.advance(&content, delivery_day).unwrap();
        assert!(w.events.iter().any(|e| e.country == d.id
            && e.day == delivery_day
            && e.text.contains("coalition is operating")));
        w.advance(&content, 200 - delivery_day).unwrap();
        // The bounded news feed may expire; accepted delivery evidence is permanent.
        let delivery = w
            .player_country()
            .commitments
            .as_ref()
            .unwrap()
            .iter()
            .find(|r| r.project.policy == "coalition")
            .unwrap();
        assert!(
            delivery
                .milestones
                .iter()
                .any(|m| m.stage == CommitmentStage::Operating && m.day == delivery_day)
        );
        assert_eq!(delivery.direct_effects.len(), 3);
        assert!(
            delivery
                .direct_effects
                .iter()
                .all(|e| e.after == (e.before + 12).min(100))
        );
        w.validate(&content).unwrap();
    }
}
#[test]
fn policy_has_cost_delay_no_duplicate_and_save_resumes_exactly() {
    let (content, mut w) = setup("GBR");
    let before = w.player_country().accounts.treasury.0;
    let preview = w.preview(&content, &w.player, &"energy".into(), 1).unwrap();
    w.issue(
        &content,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    assert_eq!(
        w.player_country().accounts.treasury.0,
        before - preview.upfront_payment.0
    );
    assert_eq!(w.player_country().energy_capacity, 90);
    assert!(
        w.issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1
            }
        )
        .is_err()
    );
    w.advance(&content, preview.delay - 1).unwrap();
    assert_eq!(w.player_country().energy_capacity, 90);
    let bytes = persistence::encode(&w, &content).unwrap();
    let mut loaded = persistence::decode(&bytes, &content).unwrap();
    w.advance(&content, 1).unwrap();
    loaded.advance(&content, 1).unwrap();
    assert_eq!(w.player_country().energy_capacity, 105);
    assert_eq!(loaded.hash(), w.hash());
    w.advance(&content, 100).unwrap();
    loaded.advance(&content, 100).unwrap();
    assert_eq!(loaded.hash(), w.hash());
    persistence::replay(&w, &content).unwrap();
}
#[test]
fn sector_cash_and_physical_goods_reconcile_every_day() {
    let (content, mut w) = setup("GBR");
    w.issue(
        &content,
        Action::Borrow {
            amount: Money(1000),
        },
    )
    .unwrap();
    w.issue(&content, Action::Repay { amount: Money(500) })
        .unwrap();
    for _ in 0..400 {
        let old = w.clone();
        w.advance(&content, 1).unwrap();
        for (id, c) in &w.countries {
            let previous = &old.countries[id];
            let r = &c.report;
            assert_eq!(c.accounts.total(), c.opening_money.0);
            assert_eq!(
                previous
                    .energy_stock
                    .checked_add(r.energy_produced)
                    .unwrap()
                    .checked_add(r.energy_imported)
                    .unwrap()
                    .checked_add(
                        c.reserves.as_ref().map_or(Energy::ZERO, |s| s
                            .energy
                            .last
                            .as_ref()
                            .unwrap()
                            .released)
                    )
                    .unwrap(),
                r.energy_used.checked_add(c.energy_stock).unwrap()
            );
            assert_eq!(
                previous
                    .food_stock
                    .checked_add(r.food_produced)
                    .unwrap()
                    .checked_add(r.food_imported)
                    .unwrap()
                    .checked_add(
                        c.reserves.as_ref().map_or(Food::ZERO, |s| s
                            .food
                            .last
                            .as_ref()
                            .unwrap()
                            .released)
                    )
                    .unwrap(),
                r.food_consumed
                    .checked_add(r.food_lost)
                    .unwrap()
                    .checked_add(c.food_stock)
                    .unwrap()
            );
            assert_eq!(
                c.arrears.0,
                c.arrears_households.0 + c.arrears_external.0 + c.arrears_firms.0
            );
        }
        w.validate(&content).unwrap();
    }
}
#[test]
fn unfunded_services_create_liabilities_and_reduce_delivery() {
    let (content, mut w) = setup("GBR");
    // Developer fixture transfers cash; normal play cannot edit accounts.
    let c = w.countries.get_mut(&w.player.clone()).unwrap();
    c.accounts.external.0 += c.accounts.treasury.0;
    c.accounts.treasury.0 = 0;
    c.budget.opening_cash = Money(0);
    c.services = 18;
    c.welfare = 12;
    w.advance(&content, 1).unwrap();
    let c = w.player_country();
    assert!(c.arrears.0 > 0);
    assert!(c.report.service_delivery < 92);
    assert!(c.arrears_households.0 > 0);
    w.validate(&content).unwrap();
    w.issue(
        &content,
        Action::Borrow {
            amount: Money(10000),
        },
    )
    .unwrap();
    w.advance(&content, 13).unwrap();
    assert!(
        w.player_country().report.unpaid.0 > 0,
        "Pending loan is not spendable"
    );
    w.advance(&content, 1).unwrap();
    assert_eq!(w.player_country().report.unpaid.0, 0);
}
#[test]
fn save_damage_versions_content_mismatch_and_atomic_backup() {
    let (content, mut w) = setup("GBR");
    let mut bytes = persistence::encode(&w, &content).unwrap();
    bytes[0] = b'!';
    assert!(persistence::decode(&bytes, &content).is_err());
    let mut modified: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &content).unwrap()).unwrap();
    modified["world"]["countries"]["GBR"]["approval"] = serde_json::json!(7);
    assert!(persistence::decode(&serde_json::to_vec(&modified).unwrap(), &content).is_err());
    let mut extreme = w.clone();
    extreme
        .countries
        .get_mut(&extreme.player.clone())
        .unwrap()
        .accounts
        .treasury = Money(i64::MAX);
    assert!(persistence::encode(&extreme, &content).is_err());
    let mut other = content.clone();
    other.id = "other-pack".into();
    assert!(persistence::decode(&persistence::encode(&w, &content).unwrap(), &other).is_err());
    let dir = std::env::temp_dir().join(format!("geosim-save-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("campaign.json");
    persistence::save(&path, &w, &content).unwrap();
    let original = w.hash();
    w.advance(&content, 1).unwrap();
    persistence::save(&path, &w, &content).unwrap();
    assert_eq!(persistence::load(&path, &content).unwrap().hash(), w.hash());
    assert_eq!(
        persistence::load(&path.with_extension("json.bak"), &content)
            .unwrap()
            .hash(),
        original
    );
    std::fs::write(path.with_extension("json.tmp"), "interrupted").unwrap();
    assert!(persistence::save(&path, &w, &content).is_err());
    assert_eq!(persistence::load(&path, &content).unwrap().hash(), w.hash());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn elections_change_government_country_continues_and_ai_is_active() {
    let (content, mut w) = setup("USA");
    w.advance(&content, 729).unwrap();
    let c = w.countries.get_mut(&w.player.clone()).unwrap();
    c.approval = 0;
    for h in &mut c.households {
        h.approval = 0;
    }
    w.advance(&content, 1).unwrap();
    let c = w.player_country();
    assert_eq!(c.government_number, 2);
    assert_ne!(c.ruling_party, "USA:democratic");
    assert_eq!(w.player.0, "USA");
    assert!(w.journal.iter().any(|e| e.ai));
    w.validate(&content).unwrap();
}
#[test]
fn five_year_campaign_is_reproducible_with_actions_and_milestone() {
    let (content, mut w) = setup("GBR");
    w.issue(
        &content,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    w.advance(&content, 150).unwrap();
    w.issue(
        &content,
        Action::Enact {
            policy: "agriculture".into(),
            value: 1,
        },
    )
    .unwrap();
    w.advance(&content, 1677).unwrap();
    w.validate(&content).unwrap();
    assert!(w.milestone_reported);
    assert!(w.events.iter().any(|e| e.text.contains("Five-year review")));
    assert_eq!(persistence::replay(&w, &content).unwrap().hash(), w.hash());
    assert!(w.player_country().accounts.treasury.0 > 0);
    assert!(w.player_country().report.wages.0 > 0);
}
#[test]
fn ten_year_campaigns_across_the_roster_preserve_invariants() {
    let content = Content::bundled().unwrap();
    for d in &content.countries {
        let mut world = World::new(&content, d.id.clone(), 123).unwrap();
        world.advance(&content, 3660).unwrap();
        world.validate(&content).unwrap();
        assert!(world.countries.values().all(|c| c.history.len() <= 120));
        assert!(world.ledger.iter().all(|e| e.day + 30 >= world.day));
    }
}
