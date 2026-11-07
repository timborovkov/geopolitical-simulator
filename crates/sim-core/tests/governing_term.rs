use sim_core::{persistence, *};
fn setup(id: &str) -> (Content, World) {
    let c = Content::bundled().unwrap();
    let w = World::new(&c, CountryId(id.into()), 2000).unwrap();
    (c, w)
}
#[test]
fn source_observations_and_household_definitions_are_validated() {
    let c = Content::bundled().unwrap();
    assert_eq!(c.facts.len(), 29);
    for id in ["GBR", "CHN", "SAU"] {
        let d = c.country(&CountryId(id.into())).unwrap();
        assert_eq!(d.quality, "mixed_observed_and_authored");
    }
    let mut bad = c.clone();
    bad.facts[0].applied_value += 1;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.countries[0].households[0].population_share += 1;
    assert!(bad.validate().is_err());
    assert_eq!(apportion(7, &[1, 1, 1]), vec![3, 2, 2]);
}
#[test]
fn proposal_reserves_cash_consent_pays_capital_and_rechecks_authority() {
    let (c, mut w) = setup("GBR");
    let preview = w.preview(&c, &w.player, &"energy".into(), 1).unwrap();
    let before = w.player_country().accounts.treasury.0;
    w.issue(
        &c,
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
    assert_eq!(
        w.player_country().committed_cash.0,
        preview.cost.0 - preview.upfront_payment.0
    );
    let reserved = w.player_country().committed_cash.0;
    w.advance(
        &c,
        sim_core::DaySpan::new(preview.consent_delay.units() - 1),
    )
    .unwrap();
    let mut restored = persistence::decode(&persistence::encode(&w, &c).unwrap(), &c).unwrap();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    restored.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(w.hash(), restored.hash());
    assert_eq!(w.player_country().committed_cash.0, 0);
    assert_eq!(
        w.player_country().projects[0].stage,
        PolicyStage::Implementing
    );
    assert!(w.player_country().budget.capital.0 >= reserved);
    let (c, mut rejected) = setup("GBR");
    rejected
        .issue(
            &c,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    let player = rejected.player.clone();
    rejected
        .countries
        .get_mut(&player)
        .unwrap()
        .legislative_support = 0;
    rejected.advance(&c, preview.consent_delay).unwrap();
    assert!(rejected.player_country().projects.is_empty());
    assert_eq!(rejected.player_country().committed_cash.0, 0);
    assert_eq!(
        rejected.player_country().budget.capital,
        preview.upfront_payment
    );
    rejected.validate(&c).unwrap();
}
#[test]
fn royal_party_and_parliamentary_approval_differ() {
    let (c, mut uk) = setup("GBR");
    let id = uk.player.clone();
    uk.countries.get_mut(&id).unwrap().legislative_support = 50;
    assert!(uk.preview(&c, &id, &"energy".into(), 1).is_err());
    let (c, mut china) = setup("CHN");
    let id = china.player.clone();
    china.countries.get_mut(&id).unwrap().council_support = 49;
    assert!(china.preview(&c, &id, &"energy".into(), 1).is_err());
    let (c, mut saudi) = setup("SAU");
    let id = saudi.player.clone();
    saudi.countries.get_mut(&id).unwrap().council_support = 60;
    assert!(saudi.preview(&c, &id, &"welfare".into(), 9).is_err());
    assert_eq!(
        saudi
            .preview(&c, &id, &"welfare".into(), 7)
            .unwrap()
            .consent_delay
            .units(),
        0
    );
    assert!(saudi.preview(&c, &id, &"coalition".into(), 1).is_ok());
}
#[test]
fn welfare_reaches_target_groups_and_taxes_change_disposable_income() {
    let (c, mut baseline) = setup("GBR");
    let mut welfare = baseline.clone();
    let mut tax = baseline.clone();
    welfare
        .issue(
            &c,
            Action::Enact {
                policy: "welfare".into(),
                value: 8,
            },
        )
        .unwrap();
    tax.issue(
        &c,
        Action::Enact {
            policy: "income_tax".into(),
            value: 40,
        },
    )
    .unwrap();
    baseline.advance(&c, sim_core::DaySpan::new(180)).unwrap();
    welfare.advance(&c, sim_core::DaySpan::new(180)).unwrap();
    tax.advance(&c, sim_core::DaySpan::new(180)).unwrap();
    let vulnerable = |w: &World| {
        w.player_country()
            .households
            .iter()
            .find(|h| h.id == "vulnerable")
            .unwrap()
            .clone()
    };
    assert!(vulnerable(&welfare).income.0 > vulnerable(&baseline).income.0);
    assert!(vulnerable(&welfare).cash.0 > vulnerable(&baseline).cash.0);
    let workers = |w: &World| {
        w.player_country()
            .households
            .iter()
            .find(|h| h.id == "workers")
            .unwrap()
            .clone()
    };
    assert!(workers(&tax).tax.0 > workers(&baseline).tax.0);
    assert!(
        workers(&tax).income.0 - workers(&tax).tax.0
            < workers(&baseline).income.0 - workers(&baseline).tax.0
    );
    for w in [&baseline, &welfare, &tax] {
        w.validate(&c).unwrap();
    }
}
#[test]
fn annual_execution_reconciles_and_a_new_year_preserves_the_review() {
    let (c, mut w) = setup("GBR");
    w.issue(
        &c,
        Action::Borrow {
            amount: Money(1000),
        },
    )
    .unwrap();
    w.issue(&c, Action::Repay { amount: Money(300) }).unwrap();
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(366)).unwrap();
    w.validate(&c).unwrap();
    let state = w.player_country();
    assert_eq!(state.budget_reviews.len(), 1);
    let review = &state.budget_reviews[0];
    assert_eq!(review.period.expected_cash(), review.closing_cash.0);
    assert_eq!(review.period.borrowing.0, 1000);
    assert_eq!(review.period.principal_repaid.0, 300);
    assert!(review.period.capital.0 > 0);
    assert_eq!(state.budget.start_day, sim_core::CampaignDay::new(366));
    persistence::replay(&w, &c).unwrap();
}
#[test]
fn reserved_cash_cannot_be_used_for_repayment() {
    let (c, mut w) = setup("GBR");
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    let cash = w.player_country().accounts.treasury.0;
    let before = w.hash();
    assert!(
        w.issue(
            &c,
            Action::Repay {
                amount: Money(cash)
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), before);
}
#[test]
fn three_reference_crisis_terms_transition_and_replay_through_pending_programs() {
    for id in ["GBR", "CHN", "SAU"] {
        let c = Content::bundled().unwrap();
        let mut w = World::new_scenario(&c, CountryId(id.into()), 17, Scenario::FiscalSupplyCrisis)
            .unwrap();
        w.issue(
            &c,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
        let preview_days = w.player_country().projects[0].due_day;
        let bytes = persistence::encode(&w, &c).unwrap();
        let mut resumed = persistence::decode(&bytes, &c).unwrap();
        w.advance(&c, sim_core::DaySpan::new((preview_days).units()))
            .unwrap();
        resumed
            .advance(&c, sim_core::DaySpan::new((preview_days).units()))
            .unwrap();
        assert_eq!(w.hash(), resumed.hash());
        w.advance(&c, sim_core::DaySpan::new(1827 - preview_days.units()))
            .unwrap();
        resumed
            .advance(&c, sim_core::DaySpan::new(1827 - preview_days.units()))
            .unwrap();
        assert_eq!(w.hash(), resumed.hash());
        w.validate(&c).unwrap();
        assert!(
            w.player_country().government_number > GovernmentId(1),
            "{id}: no political transition"
        );
        assert!(w.player_country().arrears.0 > 0, "{id}: no fiscal crisis");
        assert!(
            w.player_country()
                .households
                .iter()
                .any(|h| h.food_coverage < 90)
        );
        assert!(w.milestone_reported);
        persistence::replay(&w, &c).unwrap();
    }
}
#[test]
fn version_one_and_malformed_budget_saves_fail_cleanly() {
    let (c, mut w) = setup("GBR");
    let old = br#"{"schema":1,"world":{}}"#;
    assert!(
        persistence::decode(old, &c)
            .unwrap_err()
            .contains("Unsupported save version")
    );
    w.countries
        .get_mut(&w.player.clone())
        .unwrap()
        .budget
        .revenue = Money(i64::MAX);
    assert!(persistence::encode(&w, &c).is_err());
}
