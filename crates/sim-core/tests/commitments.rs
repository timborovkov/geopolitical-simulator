use sim_core::*;
fn fixture() -> Content {
    Content::from_json(include_str!("../../../data/synthetic-commitment-test.json")).unwrap()
}
fn start(content: &Content, id: &str) -> World {
    World::new(content, CountryId(id.into()), 17).unwrap()
}
fn enact(w: &mut World, c: &Content, policy: &str, value: i32) -> CommitmentId {
    let id = CommitmentId(w.next_project);
    w.issue(
        c,
        Action::Enact {
            policy: policy.into(),
            value,
        },
    )
    .unwrap();
    id
}
fn check(w: &World, c: &Content) {
    w.validate(c).unwrap();
    let bytes = persistence::encode(w, c).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        7
    );
    assert_eq!(persistence::decode(&bytes, c).unwrap(), *w);
    persistence::replay(w, c).unwrap();
}
#[test]
fn amendment_pays_restarts_consent_and_preserves_direct_and_observed_history() {
    let c = fixture();
    let mut w = start(&c, "AUR");
    let forecast = w.preview(&c, &w.player, &"income_tax".into(), 32).unwrap();
    let id = enact(&mut w, &c, "income_tax", 30);
    w.advance(&c, sim_core::DaySpan::new(3)).unwrap();
    let p = w.preview_amendment(&c, &w.player, id, 32).unwrap();
    assert_eq!(p.annual_budget_change, forecast.annual_budget_change);
    assert_eq!(p.household_impacts, forecast.household_impacts);
    assert_eq!(p.fee, Money(2));
    assert_eq!(p.consent_day, 17);
    let before = w.player_country().accounts.treasury;
    let hash = w.hash();
    w.preview_amendment(&c, &w.player, id, 32).unwrap();
    assert_eq!(w.hash(), hash);
    w.issue(
        &c,
        Action::AmendCommitment {
            request: id,
            value: 32,
        },
    )
    .unwrap();
    assert_eq!(w.player_country().accounts.treasury.0, before.0 - 2);
    assert_eq!(w.player_country().committed_cash, Money(18));
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(p.consent_day - w.day))
        .unwrap();
    let r = &w.player_country().commitments.as_ref().unwrap()[0];
    assert_eq!(r.stage(), CommitmentStage::Implementing);
    assert_eq!(r.paid, Money(22));
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(p.due_day - w.day))
        .unwrap();
    assert_eq!(w.player_country().tax, 32);
    assert!(w.player_country().projects.is_empty());
    let r = &w.player_country().commitments.as_ref().unwrap()[0];
    assert_eq!(
        r.direct_effects,
        vec![DirectPolicyEffect {
            field: "income_tax".into(),
            before: 24,
            after: 32
        }]
    );
    assert_eq!(r.stage(), CommitmentStage::Operating);
    let baseline = r.baseline.clone();
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(30)).unwrap();
    let r = &w.player_country().commitments.as_ref().unwrap()[0];
    assert_eq!(r.stage(), CommitmentStage::Evaluated);
    assert_eq!(r.evaluation.as_ref().unwrap().before, baseline);
    assert_eq!(
        r.evaluation.as_ref().unwrap().after,
        w.player_country().report
    );
    check(&w, &c);
}
#[test]
fn withdrawal_releases_only_reservations_and_cannot_cancel_a_finished_programme() {
    let c = fixture();
    let mut w = start(&c, "AUR");
    let id = enact(&mut w, &c, "energy", 1);
    let preview = w.preview_cancellation(&c, &w.player, id).unwrap();
    assert_eq!(preview.released, Money(450));
    assert_eq!(preview.sunk, Money(50));
    let before = w.player_country().accounts.clone();
    w.issue(&c, Action::CancelCommitment { request: id })
        .unwrap();
    assert_eq!(w.player_country().accounts, before);
    assert_eq!(w.player_country().committed_cash, Money(0));
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(200)).unwrap();
    assert_eq!(w.player_country().energy_capacity, 80);
    let id = enact(&mut w, &c, "energy", 1);
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let before = w.player_country().accounts.clone();
    assert_eq!(
        w.preview_cancellation(&c, &w.player, id).unwrap(),
        CancellationPreview {
            released: Money(0),
            sunk: Money(500)
        }
    );
    w.issue(&c, Action::CancelCommitment { request: id })
        .unwrap();
    assert_eq!(w.player_country().accounts, before);
    check(&w, &c);
    let id = enact(&mut w, &c, "income_tax", 28);
    let delay = w.player_country().projects[0].due_day - w.day;
    w.advance(&c, sim_core::DaySpan::new(delay)).unwrap();
    let hash = w.hash();
    assert!(
        w.issue(&c, Action::CancelCommitment { request: id })
            .is_err()
    );
    assert_eq!(w.hash(), hash);
    check(&w, &c);
}
#[test]
fn amendments_validate_backing_levels_limits_ownership_and_cash_atomically() {
    let c = fixture();
    let mut w = start(&c, "AUR");
    let id = enact(&mut w, &c, "income_tax", 28);
    for value in [28, 24, 46] {
        let hash = w.hash();
        assert!(
            w.issue(&c, Action::AmendCommitment { request: id, value })
                .is_err()
        );
        assert_eq!(hash, w.hash());
    }
    assert!(
        w.preview_amendment(&c, &CountryId("BOR".into()), id, 30)
            .is_err()
    );
    for value in [29, 30, 31] {
        w.issue(&c, Action::AmendCommitment { request: id, value })
            .unwrap();
    }
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::AmendCommitment {
                request: id,
                value: 32
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), hash);
    check(&w, &c);
    let mut poor = c.clone();
    poor.commitments.as_mut().unwrap().amendment_fee_bps = 10000;
    let mut w = start(&poor, "AUR");
    let id = enact(&mut w, &poor, "income_tax", 30);
    w.issue_developer(
        &poor,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::LegislativeSupport,
            value: 0,
        },
    )
    .unwrap();
    let hash = w.hash();
    assert!(
        w.issue(
            &poor,
            Action::AmendCommitment {
                request: id,
                value: 32
            }
        )
        .is_err()
    );
    assert_eq!(hash, w.hash());
    // The sponsor may withdraw an unapproved request even after losing support.
    w.issue(&poor, Action::CancelCommitment { request: id })
        .unwrap();
    check(&w, &poor);
}
#[test]
fn amendment_cannot_spend_reserved_capital_and_reviews_expose_overlapping_deliveries() {
    let c = fixture();
    let mut w = start(&c, "AUR");
    let id = enact(&mut w, &c, "income_tax", 30);
    let available = w.player_country().accounts.treasury.0 - w.player_country().committed_cash.0;
    w.issue(
        &c,
        Action::Repay {
            amount: Money(available - 1),
        },
    )
    .unwrap();
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::AmendCommitment {
                request: id,
                value: 32
            }
        )
        .is_err()
    );
    assert_eq!(hash, w.hash());
    assert_eq!(w.player_country().committed_cash, Money(18));
    check(&w, &c);
    let mut w = start(&c, "AUR");
    enact(&mut w, &c, "income_tax", 30);
    enact(&mut w, &c, "welfare", 6);
    w.advance(&c, sim_core::DaySpan::new(89)).unwrap();
    let r = &w.player_country().commitments.as_ref().unwrap()[0];
    assert_eq!(r.evaluation.as_ref().unwrap().other_deliveries, 1);
    check(&w, &c);
}
#[test]
fn refusal_is_retained_and_replacement_repeals_only_the_previous_level() {
    let c = fixture();
    let mut w = start(&c, "AUR");
    enact(&mut w, &c, "income_tax", 28);
    w.issue_developer(
        &c,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::LegislativeSupport,
            value: 0,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    assert_eq!(
        w.player_country().commitments.as_ref().unwrap()[0].stage(),
        CommitmentStage::Refused
    );
    check(&w, &c);
    w.issue_developer(
        &c,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::LegislativeSupport,
            value: 80,
        },
    )
    .unwrap();
    enact(&mut w, &c, "income_tax", 30);
    w.advance(&c, sim_core::DaySpan::new(59)).unwrap();
    w.advance(&c, sim_core::DaySpan::new(30)).unwrap();
    enact(&mut w, &c, "income_tax", 24);
    w.advance(&c, sim_core::DaySpan::new(59)).unwrap();
    let records = w.player_country().commitments.as_ref().unwrap();
    assert_eq!(records[1].stage(), CommitmentStage::Repealed);
    assert!(records[1].evaluation.is_some());
    assert_eq!(records[2].stage(), CommitmentStage::Operating);
    assert_eq!(w.player_country().tax, 24);
    check(&w, &c);
}
#[test]
fn funded_withdrawal_requires_current_power_and_unapproved_programmes_cannot_amend_capacity() {
    let c = fixture();
    let mut w = start(&c, "AUR");
    let id = enact(&mut w, &c, "energy", 1);
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::AmendCommitment {
                request: id,
                value: 1
            }
        )
        .is_err()
    );
    assert_eq!(hash, w.hash());
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    w.issue_developer(
        &c,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::LegislativeSupport,
            value: 0,
        },
    )
    .unwrap();
    let hash = w.hash();
    assert!(
        w.issue(&c, Action::CancelCommitment { request: id })
            .is_err()
    );
    assert_eq!(hash, w.hash());
    check(&w, &c);
}
#[test]
fn immediate_route_after_regime_change_funds_an_amendment_without_a_stale_pending_date() {
    let mut c = fixture();
    let rules = c.constitutions.as_mut().unwrap();
    rules.lawful.days = DaySpan::new(1);
    rules.lawful.legislative_threshold = 51;
    rules.lawful.elite_threshold = 0;
    rules.lawful.council_threshold = 0;
    rules.lawful.approval_threshold = 0;
    c.validate().unwrap();
    let mut w = start(&c, "AUR");
    let id = enact(&mut w, &c, "income_tax", 28);
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::AbsoluteMonarchy,
            contested: false,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    w.issue(
        &c,
        Action::AmendCommitment {
            request: id,
            value: 30,
        },
    )
    .unwrap();
    assert_eq!(
        w.player_country().projects[0].stage,
        PolicyStage::Implementing
    );
    assert_eq!(w.player_country().projects[0].reserved, Money(0));
    check(&w, &c);
}
#[test]
fn legacy_rules_keep_hashes_and_reject_lifecycle_orders_and_malformed_state() {
    let c = Content::constitution_bundled().unwrap();
    let mut old = start(&c, "GBR");
    let hash = old.hash();
    assert!(
        old.issue(
            &c,
            Action::CancelCommitment {
                request: CommitmentId(1)
            }
        )
        .is_err()
    );
    assert_eq!(old.hash(), hash);
    let c = fixture();
    let mut w = start(&c, "AUR");
    enact(&mut w, &c, "energy", 1);
    check(&w, &c);
    let mut orphan = start(&c, "AUR");
    orphan.countries.get_mut(&orphan.player).unwrap().projects =
        w.player_country().projects.clone();
    assert!(orphan.validate(&c).is_err());
    let mut broken = w.clone();
    broken
        .countries
        .get_mut(&w.player)
        .unwrap()
        .commitments
        .as_mut()
        .unwrap()[0]
        .milestones
        .clear();
    assert!(broken.validate(&c).is_err());
    let mut broken = w.clone();
    broken
        .countries
        .get_mut(&w.player)
        .unwrap()
        .commitments
        .as_mut()
        .unwrap()[0]
        .paid
        .0 += 1;
    assert!(broken.validate(&c).is_err());
    let mut broken = w.clone();
    broken.countries.get_mut(&w.player).unwrap().commitments = None;
    assert!(broken.validate(&c).is_err());
    let mut save: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    save["schema"] = 6.into();
    assert!(persistence::decode(&serde_json::to_vec(&save).unwrap(), &c).is_err());
    let view = w.project(&c, Viewer::Observer).unwrap();
    assert!(view.own.is_none());
    assert!(
        !serde_json::to_string(&view)
            .unwrap()
            .contains("proposal_command")
    );
    w.day = MAX_DAY;
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30
            }
        )
        .is_err()
    );
    assert_eq!(hash, w.hash());
}
