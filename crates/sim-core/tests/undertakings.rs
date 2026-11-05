use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-undertakings-test.json"
    ))
    .unwrap()
}
fn start(c: &Content, id: &str) -> World {
    World::new(c, CountryId(id.into()), 17).unwrap()
}
fn declare(
    w: &mut World,
    c: &Content,
    template: &str,
    project: Option<CommitmentId>,
) -> UndertakingId {
    let id = UndertakingId(w.next_command);
    w.issue(
        c,
        Action::DeclareUndertaking {
            template: template.into(),
            project,
        },
    )
    .unwrap();
    id
}
fn enact(w: &mut World, c: &Content, policy: &str, value: i32) -> CommitmentId {
    w.issue(
        c,
        Action::Enact {
            policy: policy.into(),
            value,
        },
    )
    .unwrap();
    w.player_country()
        .commitments
        .as_ref()
        .unwrap()
        .last()
        .unwrap()
        .id
}
fn record(w: &World, id: UndertakingId) -> &UndertakingRecord {
    w.player_country()
        .undertakings
        .as_ref()
        .unwrap()
        .records
        .iter()
        .find(|r| r.id == id)
        .unwrap()
}
fn check(w: &World, c: &Content) {
    w.validate(c).unwrap();
    let bytes = persistence::encode(w, c).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        12
    );
    assert_eq!(persistence::decode(&bytes, c).unwrap(), *w);
    persistence::replay(w, c).unwrap();
}
#[test]
fn a_public_pledge_costs_administration_but_only_an_actual_policy_delivery_fulfils_it() {
    let c = content();
    let mut w = start(&c, "AUR");
    let hash = w.hash();
    let p = w
        .preview_undertaking(&c, &w.player, "energy-pledge", None)
        .unwrap();
    assert_eq!(p.fee, Money(20));
    assert_eq!(p.goal.target, 95);
    assert_eq!(p.review_day, 7);
    assert_eq!(p.deadline_day, 187);
    assert_eq!(w.hash(), hash);
    let id = declare(&mut w, &c, "energy-pledge", None);
    assert!(w.player_country().projects.is_empty());
    assert_eq!(w.player_country().energy_capacity, 80);
    assert_eq!(w.player_country().accounts.treasury, Money(4980));
    check(&w, &c);
    let source = enact(&mut w, &c, "energy", 1);
    let due = w.player_country().projects[0].due_day;
    w.advance(&c, due).unwrap();
    assert_eq!(
        record(&w, id).status,
        UndertakingStatus::Fulfilled { evidence: source }
    );
    assert_eq!(record(&w, id).resolved_day, Some(due));
    assert_eq!(
        w.player_country()
            .undertakings
            .as_ref()
            .unwrap()
            .credibility,
        65
    );
    assert!(
        record(&w, id)
            .milestones
            .iter()
            .any(|m| m.stage == CommitmentStage::Operating)
    );
    check(&w, &c);
}
#[test]
fn an_unimplemented_support_agreement_grants_backing_then_breaches_at_its_fixed_deadline() {
    let c = content();
    let mut w = start(&c, "AUR");
    let id = declare(&mut w, &c, "service-agreement", None);
    w.advance(&c, 14).unwrap();
    assert_eq!(record(&w, id).status, UndertakingStatus::Active);
    assert_eq!(w.player_country().legislative_support, 72);
    assert_eq!(w.player_country().elite_support, 77);
    assert_eq!(w.player_country().services, 9);
    assert!(w.player_country().projects.is_empty());
    check(&w, &c);
    w.advance(&c, 179).unwrap();
    let before = w.player_country().clone();
    w.advance(&c, 1).unwrap();
    assert_eq!(record(&w, id).status, UndertakingStatus::Breached);
    assert_eq!(record(&w, id).resolved_day, Some(194));
    assert_eq!(
        w.player_country().legislative_support,
        (before.legislative_support - 18).max(0)
    );
    assert_eq!(
        w.player_country()
            .undertakings
            .as_ref()
            .unwrap()
            .credibility,
        50
    );
    assert!(!w.is_developer_modified());
    check(&w, &c);
}
#[test]
fn withdrawal_keeps_fees_and_intervals_and_only_accepted_promises_have_breach_costs() {
    let c = content();
    let mut w = start(&c, "AUR");
    let id = declare(&mut w, &c, "service-agreement", None);
    w.issue(&c, Action::WithdrawUndertaking { request: id })
        .unwrap();
    assert_eq!(w.player_country().accounts.treasury, Money(4950));
    assert_eq!(
        w.player_country()
            .undertakings
            .as_ref()
            .unwrap()
            .credibility,
        60
    );
    assert!(
        w.preview_undertaking(&c, &w.player, "service-agreement", None)
            .is_err()
    );
    check(&w, &c);
    let mut w = start(&c, "AUR");
    let id = declare(&mut w, &c, "service-agreement", None);
    w.advance(&c, 14).unwrap();
    w.issue(&c, Action::WithdrawUndertaking { request: id })
        .unwrap();
    assert_eq!(w.player_country().legislative_support, 54);
    assert_eq!(
        w.player_country()
            .undertakings
            .as_ref()
            .unwrap()
            .credibility,
        52
    );
    check(&w, &c);
}
#[test]
fn a_project_deadline_reuses_the_owned_policy_commitment_and_never_orders_a_second_project() {
    let c = content();
    let mut w = start(&c, "AUR");
    assert!(
        w.preview_undertaking(&c, &w.player, "project-deadline", None)
            .is_err()
    );
    let source = enact(&mut w, &c, "energy", 1);
    let due = w.player_country().projects[0].due_day;
    let id = declare(&mut w, &c, "project-deadline", Some(source));
    assert_eq!(w.player_country().projects.len(), 1);
    assert_eq!(record(&w, id).linked_project, Some(source));
    check(&w, &c);
    w.advance(&c, due).unwrap();
    assert_eq!(
        record(&w, id).status,
        UndertakingStatus::Fulfilled { evidence: source }
    );
    check(&w, &c);
}
#[test]
fn amending_or_withdrawing_the_attached_project_does_not_move_its_promised_target_or_deadline() {
    for withdraw in [false, true] {
        let c = content();
        let mut w = start(&c, "AUR");
        let source = enact(&mut w, &c, "income_tax", 26);
        let id = declare(&mut w, &c, "project-deadline", Some(source));
        let deadline = record(&w, id).deadline_day;
        if withdraw {
            w.issue(&c, Action::CancelCommitment { request: source })
                .unwrap();
        } else {
            w.issue(
                &c,
                Action::AmendCommitment {
                    request: source,
                    value: 25,
                },
            )
            .unwrap();
        }
        assert_eq!(record(&w, id).goal.target, 26);
        assert_eq!(record(&w, id).deadline_day, deadline);
        check(&w, &c);
        w.advance(&c, deadline).unwrap();
        assert_eq!(record(&w, id).status, UndertakingStatus::Breached);
        check(&w, &c);
    }
}
#[test]
fn delivery_during_review_is_recognized_when_the_undertaking_is_accepted() {
    let mut c = content();
    c.cabinet.as_mut().unwrap().vacancy_preparation_days = 1;
    c.policies
        .iter_mut()
        .find(|p| p.id == "energy")
        .unwrap()
        .days = 1;
    c.undertakings.as_mut().unwrap().templates[0].review_days = 30;
    let mut w = start(&c, "COR");
    let id = declare(&mut w, &c, "energy-pledge", None);
    let source = enact(&mut w, &c, "energy", 1);
    let due = w.player_country().projects[0].due_day;
    assert!(due < 30);
    w.advance(&c, due).unwrap();
    assert_eq!(record(&w, id).status, UndertakingStatus::Pending);
    check(&w, &c);
    w.advance(&c, 30 - due).unwrap();
    assert_eq!(
        record(&w, id).status,
        UndertakingStatus::Fulfilled { evidence: source }
    );
    assert_eq!(record(&w, id).resolved_day, Some(30));
    check(&w, &c);
}
#[test]
fn review_rechecks_backing_after_another_accepted_agreement_is_withdrawn() {
    let mut c = content();
    let templates = &mut c.undertakings.as_mut().unwrap().templates;
    templates[1].review_days = 1;
    templates[0].elite_threshold = 70;
    let mut w = start(&c, "AUR");
    let support = declare(&mut w, &c, "service-agreement", None);
    w.advance(&c, 1).unwrap();
    let pledge = declare(&mut w, &c, "energy-pledge", None);
    w.issue(&c, Action::WithdrawUndertaking { request: support })
        .unwrap();
    w.advance(&c, 7).unwrap();
    assert!(matches!(
        record(&w, pledge).status,
        UndertakingStatus::Refused { .. }
    ));
    assert!(record(&w, pledge).accepted_day.is_none());
    assert_eq!(
        w.player_country()
            .undertakings
            .as_ref()
            .unwrap()
            .credibility,
        52
    );
    check(&w, &c);
}
#[test]
fn successor_cabinets_close_pending_and_active_undertakings_while_policy_projects_continue() {
    let mut c = content();
    let route = c
        .constitutions
        .as_mut()
        .unwrap()
        .contested
        .as_mut()
        .unwrap();
    route.elite_threshold = 50;
    route.council_threshold = 40;
    c.undertakings.as_mut().unwrap().templates[1].review_days = 30;
    let mut w = start(&c, "AUR");
    let source = enact(&mut w, &c, "energy", 1);
    w.advance(&c, 14).unwrap(); // Existing capital is funded before succession.
    let pledge = declare(&mut w, &c, "energy-pledge", None);
    let agreement = declare(&mut w, &c, "service-agreement", None);
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, 14).unwrap();
    assert_eq!(record(&w, pledge).status, UndertakingStatus::Superseded);
    assert!(record(&w, pledge).accepted_day.is_some());
    assert_eq!(record(&w, agreement).status, UndertakingStatus::Superseded);
    assert!(record(&w, agreement).accepted_day.is_none());
    assert_eq!(
        w.player_country()
            .undertakings
            .as_ref()
            .unwrap()
            .credibility,
        57
    );
    assert!(w.player_country().projects.iter().any(|p| p.id == source.0));
    check(&w, &c);
}
#[test]
fn only_accepted_undertakings_and_their_outcomes_are_published() {
    let c = content();
    let mut w = start(&c, "AUR");
    let before = w.public_report(&c, &w.player).unwrap();
    declare(&mut w, &c, "energy-pledge", None);
    assert_eq!(w.public_report(&c, &w.player).unwrap(), before);
    w.advance(&c, 7).unwrap();
    let view = w.project(&c, Viewer::Observer).unwrap();
    assert!(view.own.is_none());
    let report = view.reports[&w.player].undertakings.as_ref().unwrap();
    assert_eq!(report.records.len(), 1);
    let json = serde_json::to_value(report).unwrap();
    for hidden in ["fee", "sponsor", "elite_support", "linked_project"] {
        assert!(!json.to_string().contains(hidden));
    }
    check(&w, &c);
}
#[test]
fn malformed_dates_targets_fees_links_and_credibility_fail_validation_without_panicking() {
    let c = content();
    let mut w = start(&c, "AUR");
    let id = declare(&mut w, &c, "energy-pledge", None);
    check(&w, &c);
    for mutation in 0..6 {
        let mut bad = w.clone();
        let s = bad
            .countries
            .get_mut(&w.player)
            .unwrap()
            .undertakings
            .as_mut()
            .unwrap();
        match mutation {
            0 => s.records[0].declared_day = u32::MAX,
            1 => s.records[0].baseline = i32::MAX,
            2 => s.records[0].fee = Money(i64::MIN),
            3 => s.records[0].linked_project = Some(CommitmentId(u64::MAX)),
            4 => s.credibility = 100,
            _ => s.records[0].goal.target = -1,
        };
        assert!(bad.validate(&c).is_err(), "mutation {mutation}");
    }
    let mut bytes: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    bytes["schema"] = 11.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&bytes).unwrap(), &c)
            .unwrap_err()
            .contains("format 12")
    );
    assert_eq!(record(&w, id).status, UndertakingStatus::Pending);
}
#[test]
fn ai_declares_costed_undertakings_through_shared_commands_and_replays() {
    let c = content();
    let mut w = start(&c, "COR");
    w.advance(&c, 365).unwrap();
    assert!(
        w.journal
            .iter()
            .any(|e| e.ai && matches!(e.command.action, Action::DeclareUndertaking { .. }))
    );
    assert!(!w.is_developer_modified());
    check(&w, &c);
}
#[test]
fn promises_cannot_cross_the_campaign_calendar() {
    let c = content().at_bookmark("2095-01-01").unwrap();
    let mut w = start(&c, "AUR");
    w.advance(&c, MAX_DAY - w.day - 1).unwrap();
    let hash = w.hash();
    assert!(
        w.preview_undertaking(&c, &w.player, "energy-pledge", None)
            .unwrap_err()
            .contains("calendar")
    );
    assert_eq!(w.hash(), hash);
    check(&w, &c);
}

#[test]
fn a_morning_constitutional_successor_closes_a_pledge_before_a_funded_project_delivers_that_day() {
    let mut c = content();
    let mut w = start(&c, "AUR");
    // Use source-defined timing so the two real processes meet on one day.
    let due = w.preview(&c, &w.player, &"energy".into(), 1).unwrap().delay;
    let route = c
        .constitutions
        .as_mut()
        .unwrap()
        .contested
        .as_mut()
        .unwrap();
    route.days = due;
    route.elite_threshold = 50;
    route.council_threshold = 40;
    w = start(&c, "AUR");
    let pledge = declare(&mut w, &c, "energy-pledge", None);
    let source = enact(&mut w, &c, "energy", 1);
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, due).unwrap();
    assert_eq!(record(&w, pledge).status, UndertakingStatus::Superseded);
    assert_eq!(
        w.player_country()
            .commitments
            .as_ref()
            .unwrap()
            .iter()
            .find(|r| r.id == source)
            .unwrap()
            .direct_effects[0]
            .after,
        95
    );
    check(&w, &c);
}

#[test]
fn direct_developer_capacity_edits_are_not_policy_delivery_evidence_and_later_baselines_remain_saveable()
 {
    let c = content();
    let mut w = start(&c, "AUR");
    let pledge = declare(&mut w, &c, "energy-pledge", None);
    w.advance(&c, 7).unwrap();
    w.issue_developer(
        &c,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::EnergyCapacity,
            value: 95,
        },
    )
    .unwrap();
    w.advance(&c, 180).unwrap();
    assert_eq!(record(&w, pledge).status, UndertakingStatus::Breached);
    assert!(w.is_developer_modified());
    let later = declare(&mut w, &c, "energy-pledge", None);
    assert_eq!(record(&w, later).baseline, 95);
    assert_eq!(record(&w, later).goal.target, 110);
    check(&w, &c);
}

#[test]
fn a_lower_tax_pledge_uses_the_operating_rate_and_actual_downward_delivery_evidence() {
    let mut c = content();
    let template = &mut c.undertakings.as_mut().unwrap().templates[0];
    template.policy = Some("income_tax".into());
    template.direction = GoalDirection::AtMost;
    template.delta = -2;
    let mut w = start(&c, "AUR");
    let pledge = declare(&mut w, &c, "energy-pledge", None);
    assert_eq!(record(&w, pledge).goal.target, 22);
    let source = enact(&mut w, &c, "income_tax", 22);
    let due = w.player_country().projects[0].due_day;
    w.advance(&c, due).unwrap();
    assert_eq!(w.player_country().tax, 22);
    assert_eq!(
        record(&w, pledge).status,
        UndertakingStatus::Fulfilled { evidence: source }
    );
    check(&w, &c);
}
