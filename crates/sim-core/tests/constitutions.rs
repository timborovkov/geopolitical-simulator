use sim_core::*;

fn setup() -> (Content, World) {
    let content = Content::from_json(include_str!(
        "../../../data/synthetic-constitution-test.json"
    ))
    .unwrap();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    (content, world)
}
fn support(world: &mut World, content: &Content) {
    let delay = world
        .preview(content, &world.player, &"coalition".into(), 1)
        .unwrap()
        .delay;
    world
        .issue(
            content,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
    world.advance(content, delay).unwrap();
}
fn propose(world: &mut World, content: &Content, target: Regime, contested: bool) -> TransitionId {
    let id = TransitionId(world.next_command.0);
    world
        .issue(content, Action::Reconstitute { target, contested })
        .unwrap();
    id
}
fn rejected(world: &mut World, content: &Content, action: Action) {
    let before = world.hash();
    assert!(world.issue(content, action).is_err());
    assert_eq!(world.hash(), before);
}
fn replay(world: &World, content: &Content) {
    world.validate(content).unwrap();
    persistence::replay(world, content).unwrap();
}

#[test]
fn lawful_change_and_restoration_switch_real_powers_preserve_debt_and_resume_pending_decisions() {
    let (content, mut world) = setup();
    world
        .issue(
            &content,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
    support(&mut world, &content);
    let request = propose(&mut world, &content, Regime::SingleParty, false);
    assert_eq!(
        world.governing_regime(&content, &world.player).unwrap(),
        &Regime::Parliamentary
    );
    assert_eq!(world.player_country().committed_cash, Money(400));
    world.advance(&content, 89).unwrap();
    let bytes = persistence::encode(&world, &content).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        6
    );
    let mut resumed = persistence::decode(&bytes, &content).unwrap();
    world.advance(&content, 1).unwrap();
    resumed.advance(&content, 1).unwrap();
    assert_eq!(world.hash(), resumed.hash());
    assert_eq!(
        world.governing_regime(&content, &world.player).unwrap(),
        &Regime::SingleParty
    );
    assert_eq!(
        world
            .governing_rules(&content, &world.player)
            .unwrap()
            .route,
        DecisionRoute::PartyCouncil
    );
    assert_eq!(
        world
            .player_country()
            .constitution
            .as_ref()
            .unwrap()
            .history[0]
            .request,
        request
    );
    assert!(world.player_country().next_election.is_none());
    assert_eq!(world.player_country().government_number, 2);
    assert_eq!(world.player_country().committed_cash, Money(0));
    // The old loan keeps its 14-day legislative origin; the new government
    // really uses the 21-day party borrowing route.
    assert_eq!(
        world
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .instruments[1]
            .issued_day,
        14
    );
    assert_eq!(
        world
            .preview_borrowing(&content, &world.player, Money(1000))
            .unwrap()
            .consent_days,
        21
    );
    world
        .issue(
            &content,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
    support(&mut world, &content);
    let next = world
        .player_country()
        .constitution
        .as_ref()
        .unwrap()
        .next_request_day;
    world.advance(&content, next - world.day).unwrap();
    propose(&mut world, &content, Regime::Parliamentary, false);
    world.advance(&content, 90).unwrap();
    assert_eq!(
        world.governing_regime(&content, &world.player).unwrap(),
        &Regime::Parliamentary
    );
    assert_eq!(world.player_country().government_number, 3);
    assert_eq!(world.player_country().next_election, Some(world.day + 365));
    assert_eq!(
        world.player_country().accounts.total(),
        world.player_country().opening_money.0
    );
    assert!(!world.is_developer_modified());
    replay(&world, &content);
}

#[test]
fn withdrawal_releases_only_unspent_reservations_and_keeps_inertia_and_sunk_cost() {
    let (content, mut world) = setup();
    support(&mut world, &content);
    let before = world.player_country().accounts.treasury.0;
    let request = propose(&mut world, &content, Regime::AbsoluteMonarchy, false);
    assert_eq!(world.player_country().accounts.treasury.0, before - 100);
    assert_eq!(world.player_country().committed_cash, Money(400));
    let treasury = world.player_country().accounts.treasury;
    rejected(&mut world, &content, Action::Repay { amount: treasury });
    rejected(
        &mut world,
        &content,
        Action::CancelConstitution {
            request: TransitionId(request.0 + 1),
        },
    );
    world
        .issue(&content, Action::CancelConstitution { request })
        .unwrap();
    assert_eq!(world.player_country().accounts.treasury.0, before - 100);
    assert_eq!(world.player_country().committed_cash, Money(0));
    rejected(
        &mut world,
        &content,
        Action::Reconstitute {
            target: Regime::AbsoluteMonarchy,
            contested: false,
        },
    );
    world.advance(&content, 90).unwrap();
    assert_eq!(
        world.governing_regime(&content, &world.player).unwrap(),
        &Regime::Parliamentary
    );
    assert!(
        world
            .player_country()
            .constitution
            .as_ref()
            .unwrap()
            .history
            .is_empty()
    );
    replay(&world, &content);
}

#[test]
fn lost_backing_refuses_a_pending_transition_and_releases_reserved_cash() {
    let (content, mut world) = setup();
    support(&mut world, &content);
    propose(&mut world, &content, Regime::Military, false);
    world.advance(&content, 89).unwrap();
    world
        .issue_developer(
            &content,
            world.player.clone(),
            DeveloperEdit {
                field: DeveloperField::LegislativeSupport,
                value: 0,
            },
        )
        .unwrap();
    world.advance(&content, 1).unwrap();
    assert_eq!(
        world.governing_regime(&content, &world.player).unwrap(),
        &Regime::Parliamentary
    );
    assert_eq!(world.player_country().committed_cash, Money(0));
    assert!(
        world
            .events
            .iter()
            .any(|event| event.text.contains("refused at decision"))
    );
    replay(&world, &content);
}

#[test]
fn a_contested_transition_is_a_normal_costly_political_action_with_a_real_power_gate() {
    let (content, _) = setup();
    let mut world = World::new_scenario(
        &content,
        CountryId("AUR".into()),
        17,
        Scenario::FiscalSupplyCrisis,
    )
    .unwrap();
    rejected(
        &mut world,
        &content,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    );
    for _ in 0..3 {
        if world.day < 100 {
            world
                .issue(
                    &content,
                    Action::Borrow {
                        amount: Money(10000),
                    },
                )
                .unwrap();
        }
        support(&mut world, &content);
    }
    assert!(world.player_country().approval < 55);
    assert!(
        world
            .preview_constitution(&content, &world.player, &Regime::Military, false)
            .is_err()
    );
    let before = world.player_country().clone();
    propose(&mut world, &content, Regime::Military, true);
    assert_eq!(
        world.player_country().accounts.treasury.0,
        before.accounts.treasury.0 - 1000
    );
    world.advance(&content, 14).unwrap();
    assert_eq!(
        world.governing_regime(&content, &world.player).unwrap(),
        &Regime::Military
    );
    assert_eq!(
        world.player_country().approval,
        (before.approval - 20).max(0)
    );
    assert_eq!(
        world.player_country().elite_support,
        before.elite_support - 15
    );
    assert_eq!(
        world.player_country().council_support,
        before.council_support - 20
    );
    assert!(!world.is_developer_modified());
    assert_eq!(
        world
            .governing_rules(&content, &world.player)
            .unwrap()
            .route,
        DecisionRoute::MilitaryCouncil
    );
    assert_eq!(
        world
            .preview_borrowing(&content, &world.player, Money(1000))
            .unwrap()
            .consent_days,
        7
    );
    replay(&world, &content);
}

#[test]
fn constitutional_succession_cancels_old_pending_loan_mandates_without_creating_debt() {
    let (content, mut world) = setup();
    support(&mut world, &content);
    propose(&mut world, &content, Regime::AbsoluteMonarchy, false);
    world.advance(&content, 89).unwrap();
    world
        .issue(
            &content,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
    let debt = world.player_country().debt;
    world.advance(&content, 1).unwrap();
    assert_eq!(
        world.governing_regime(&content, &world.player).unwrap(),
        &Regime::AbsoluteMonarchy
    );
    assert_eq!(world.player_country().debt, debt);
    assert!(
        world
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .pending
            .is_none()
    );
    assert_eq!(
        world
            .preview_borrowing(&content, &world.player, Money(1000))
            .unwrap()
            .consent_days,
        0
    );
    replay(&world, &content);
}

#[test]
fn invalid_templates_history_pending_dates_and_old_envelopes_fail_cleanly() {
    let (content, mut world) = setup();
    rejected(
        &mut world,
        &content,
        Action::Reconstitute {
            target: Regime::Parliamentary,
            contested: false,
        },
    );
    rejected(
        &mut world,
        &content,
        Action::Reconstitute {
            target: Regime::SingleParty,
            contested: false,
        },
    );
    support(&mut world, &content);
    propose(&mut world, &content, Regime::SingleParty, false);
    let mut bad = world.clone();
    bad.countries
        .get_mut(&world.player)
        .unwrap()
        .constitution
        .as_mut()
        .unwrap()
        .pending
        .as_mut()
        .unwrap()
        .decision_day = u32::MAX;
    assert!(bad.validate(&content).is_err());
    let mut bad = world.clone();
    bad.countries
        .get_mut(&world.player)
        .unwrap()
        .constitution
        .as_mut()
        .unwrap()
        .regime = Regime::Military;
    assert!(bad.validate(&content).is_err());
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut header: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    header["schema"] = 5.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&header).unwrap(), &content)
            .unwrap_err()
            .contains("format 6")
    );
    let mut bad = content.clone();
    bad.constitutions.as_mut().unwrap().templates[1].regime = Regime::Parliamentary;
    assert!(bad.validate().is_err());
}

#[test]
fn ai_preferences_use_shared_lawful_rules_and_public_reports_show_the_active_regime() {
    let (content, mut world) = setup();
    world.advance(&content, 1827).unwrap();
    let entry = world
        .journal
        .iter()
        .find(|entry| {
            entry.ai
                && matches!(
                    entry.command.action,
                    Action::Reconstitute {
                        contested: false,
                        ..
                    }
                )
        })
        .expect("Authored AI preference should become feasible during the campaign");
    let id = &entry.command.issuer;
    let country = &world.countries[id];
    assert!(!country.constitution.as_ref().unwrap().history.is_empty());
    let report = world.public_report(&content, id).unwrap();
    assert_eq!(
        &report.regime,
        world.governing_regime(&content, id).unwrap()
    );
    assert_ne!(report.regime, content.country(id).unwrap().regime);
    assert_eq!(
        &report.government,
        world.governing_rules(&content, id).unwrap()
    );
    let public = serde_json::to_string(&report).unwrap();
    assert!(!public.contains("pending"));
    assert!(!public.contains("sponsor_government"));
    assert!(!public.contains("council_support"));
    assert!(!public.contains("accounts"));
    replay(&world, &content);
}

#[test]
fn a_successor_cabinet_does_not_inherit_an_unapproved_constitutional_mandate() {
    let (content, mut world) = setup();
    world
        .issue(
            &content,
            Action::Borrow {
                amount: Money(10000),
            },
        )
        .unwrap();
    support(&mut world, &content);
    world.advance(&content, 650 - world.day).unwrap();
    support(&mut world, &content);
    propose(&mut world, &content, Regime::SingleParty, false);
    let due = world
        .player_country()
        .constitution
        .as_ref()
        .unwrap()
        .pending
        .as_ref()
        .unwrap()
        .decision_day;
    let election = world.player_country().next_election.unwrap();
    assert!(election < due);
    world.advance(&content, election - world.day - 1).unwrap();
    world
        .issue_developer(
            &content,
            world.player.clone(),
            DeveloperEdit {
                field: DeveloperField::Approval,
                value: 0,
            },
        )
        .unwrap();
    world.advance(&content, due - world.day).unwrap();
    assert!(world.player_country().government_number > 1);
    assert!(
        world
            .player_country()
            .constitution
            .as_ref()
            .unwrap()
            .history
            .is_empty()
    );
    assert_eq!(world.player_country().committed_cash, Money(0));
    assert!(
        world
            .events
            .iter()
            .any(|event| event.text.contains("Sponsoring cabinet changed"))
    );
    replay(&world, &content);
}

#[test]
fn every_configured_form_applies_its_policy_consent_borrowing_and_review_clocks() {
    let (content, _) = setup();
    for template in &content.constitutions.as_ref().unwrap().templates {
        if template.regime == Regime::Parliamentary {
            continue;
        }
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        support(&mut world, &content);
        propose(&mut world, &content, template.regime.clone(), false);
        world.advance(&content, 90).unwrap();
        assert_eq!(
            world.governing_regime(&content, &world.player).unwrap(),
            &template.regime
        );
        assert_eq!(
            world.governing_rules(&content, &world.player).unwrap(),
            &template.government
        );
        assert_eq!(
            world.player_country().next_election.is_some(),
            template.regime.competitive()
        );
        assert_eq!(
            world.player_country().next_leadership_review,
            template
                .government
                .leadership_review_days
                .map(|days| world.day + days)
        );
        let preview = world
            .preview(&content, &world.player, &"energy".into(), 1)
            .unwrap();
        assert_eq!(preview.consent_delay, template.government.consent_days);
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        assert_eq!(
            world.player_country().projects[0].consent_day,
            world.day + template.government.consent_days
        );
        let borrowing = world
            .preview_borrowing(&content, &world.player, Money(1))
            .unwrap();
        assert_eq!(
            borrowing.consent_days,
            content
                .finance
                .as_ref()
                .unwrap()
                .authority(&template.government.route)
                .unwrap()
                .consent_days
        );
        world
            .issue(&content, Action::Borrow { amount: Money(1) })
            .unwrap();
        world
            .advance(&content, borrowing.consent_days.max(1))
            .unwrap();
        assert!(
            world
                .player_country()
                .financing
                .as_ref()
                .unwrap()
                .pending
                .is_none()
        );
        assert_eq!(world.player_country().id.0, "AUR");
        assert_eq!(
            content.country(&world.player).unwrap().regime,
            Regime::Parliamentary
        );
        replay(&world, &content);
    }
}
