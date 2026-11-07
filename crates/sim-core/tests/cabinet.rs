use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!("../../../data/synthetic-cabinet-test.json")).unwrap()
}
fn start(c: &Content, id: &str) -> World {
    World::new(c, CountryId(id.into()), 17).unwrap()
}
fn office(id: &str, portfolio: &str) -> OfficeId {
    OfficeId(format!("portfolio:{id}:{portfolio}"))
}
fn candidate(id: &str, portfolio: &str, kind: &str) -> PersonId {
    PersonId(format!("generated:{id}:{portfolio}:{kind}"))
}
fn appoint(w: &mut World, c: &Content, portfolio: &str, kind: &str) -> AppointmentId {
    let id = w.player.0.clone();
    let request = AppointmentId(w.next_command.0);
    w.issue(
        c,
        Action::Appoint {
            office: office(&id, portfolio),
            person: candidate(&id, portfolio, kind),
        },
    )
    .unwrap();
    request
}
fn check(w: &World, c: &Content) {
    w.validate(c).unwrap();
    let bytes = persistence::encode(w, c).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        8
    );
    assert_eq!(persistence::decode(&bytes, c).unwrap(), *w);
    persistence::replay(w, c).unwrap();
}
#[test]
fn appointment_reduces_only_future_preparation_without_moving_funded_deadlines_or_capacity() {
    let c = content();
    let mut w = start(&c, "AUR");
    assert_eq!(
        w.policy_preparation_days(&c, &w.player, &"energy".into())
            .units(),
        30
    );
    let original = w.preview(&c, &w.player, &"energy".into(), 1).unwrap();
    assert_eq!(original.delay.units(), 149);
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    let request = appoint(&mut w, &c, "supply", "professional");
    assert_eq!(w.player_country().budget.capital, Money(70)); // 50 proposal + 20 appointment.
    assert_eq!(
        w.player_country().cabinet.as_ref().unwrap().pending[0].id,
        request
    );
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    assert_eq!(
        w.policy_preparation_days(&c, &w.player, &"energy".into())
            .units(),
        4
    );
    assert_eq!(w.player_country().projects[0].due_day, 149);
    assert_eq!(w.player_country().energy_capacity, 80);
    assert_eq!(w.player_country().administration, 70);
    assert_eq!(
        w.preview(&c, &w.player, &"agriculture".into(), 1)
            .unwrap()
            .delay
            .units(),
        153
    );
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(135)).unwrap();
    assert_eq!(w.player_country().energy_capacity, 95);
    check(&w, &c);
}
#[test]
fn professional_and_consensus_candidates_have_a_visible_political_and_timing_tradeoff() {
    let c = content();
    let mut professional = start(&c, "AUR");
    let mut consensus = professional.clone();
    appoint(&mut professional, &c, "finance", "professional");
    appoint(&mut consensus, &c, "finance", "consensus");
    professional
        .advance(&c, sim_core::DaySpan::new(14))
        .unwrap();
    consensus.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    assert_eq!(
        professional
            .policy_preparation_days(&c, &professional.player, &"income_tax".into())
            .units(),
        4
    );
    assert_eq!(
        consensus
            .policy_preparation_days(&c, &consensus.player, &"income_tax".into())
            .units(),
        13
    );
    assert_eq!(professional.player_country().elite_support, 59);
    assert_eq!(consensus.player_country().elite_support, 64);
    assert_eq!(professional.player_country().council_support, 59);
    assert_eq!(consensus.player_country().council_support, 64);
    assert_eq!(
        professional.player_country().accounts,
        consensus.player_country().accounts
    );
    check(&professional, &c);
    check(&consensus, &c);
}
#[test]
fn all_three_authority_routes_apply_and_immediate_decrees_leave_no_stale_pending_request() {
    let c = content();
    for (id, delay) in [("AUR", 14), ("BOR", 21), ("COR", 0)] {
        let mut w = start(&c, id);
        let office = office(id, "finance");
        let nominee = candidate(id, "finance", "professional");
        let p = w
            .preview_appointment(&c, &w.player, &office, &nominee)
            .unwrap();
        assert_eq!(p.decision_day, delay);
        assert_eq!(p.fee, Money(20));
        let hash = w.hash();
        w.preview_appointment(&c, &w.player, &office, &nominee)
            .unwrap();
        assert_eq!(w.hash(), hash);
        appoint(&mut w, &c, "finance", "professional");
        w.advance(&c, sim_core::DaySpan::new(delay)).unwrap();
        assert!(
            w.player_country()
                .cabinet
                .as_ref()
                .unwrap()
                .pending
                .is_empty()
        );
        assert!(
            w.player_country()
                .cabinet
                .as_ref()
                .unwrap()
                .minister(&office)
                .is_some()
        );
        check(&w, &c);
    }
}
#[test]
fn cancellation_and_refusal_keep_costs_and_cooldown_and_rejections_are_atomic() {
    let c = content();
    let mut w = start(&c, "AUR");
    let request = appoint(&mut w, &c, "finance", "professional");
    let before = w.player_country().accounts.clone();
    w.issue(&c, Action::CancelAppointment { request }).unwrap();
    assert_eq!(w.player_country().accounts, before);
    for action in [
        Action::CancelAppointment { request },
        Action::Appoint {
            office: office("AUR", "finance"),
            person: candidate("AUR", "finance", "consensus"),
        },
        Action::Appoint {
            office: office("BOR", "supply"),
            person: candidate("BOR", "supply", "professional"),
        },
        Action::Appoint {
            office: office("AUR", "supply"),
            person: candidate("AUR", "finance", "professional"),
        },
    ] {
        let hash = w.hash();
        assert!(w.issue(&c, action).is_err());
        assert_eq!(w.hash(), hash);
    }
    check(&w, &c);
    w.advance(&c, sim_core::DaySpan::new(90)).unwrap();
    appoint(&mut w, &c, "finance", "consensus");
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
    assert!(
        w.player_country()
            .cabinet
            .as_ref()
            .unwrap()
            .pending
            .is_empty()
    );
    assert!(
        w.player_country()
            .cabinet
            .as_ref()
            .unwrap()
            .terms
            .is_empty()
    );
    assert_eq!(w.player_country().budget.capital, Money(40));
    check(&w, &c);
}
#[test]
fn successor_cabinet_closes_terms_cancels_requests_and_retains_commissioned_programmes() {
    let mut c = content();
    let route = &mut c.constitutions.as_mut().unwrap().lawful;
    route.days = DaySpan::new(1);
    route.legislative_threshold = 51;
    route.elite_threshold = 0;
    route.elite_cost = 0;
    route.council_threshold = 0;
    route.approval_threshold = 0;
    c.validate().unwrap();
    let mut w = start(&c, "AUR");
    appoint(&mut w, &c, "supply", "professional");
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(14)).unwrap();
    let due = w.player_country().projects[0].due_day;
    appoint(&mut w, &c, "finance", "consensus");
    w.issue(
        &c,
        Action::Enact {
            policy: "income_tax".into(),
            value: 28,
        },
    )
    .unwrap();
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::AbsoluteMonarchy,
            contested: false,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    let state = w.player_country().cabinet.as_ref().unwrap();
    assert_eq!(state.government, sim_core::GovernmentId(2));
    assert_eq!(state.since, 15);
    assert_eq!(state.terms[0].until, Some(15));
    assert!(state.pending.is_empty());
    assert_eq!(w.player_country().projects[0].due_day, due);
    assert_eq!(
        w.policy_preparation_days(&c, &w.player, &"energy".into())
            .units(),
        30
    );
    appoint(&mut w, &c, "finance", "professional");
    assert!(
        w.player_country()
            .cabinet
            .as_ref()
            .unwrap()
            .minister(&office("AUR", "finance"))
            .is_some()
    );
    let amended = w.player_country().projects[1].id;
    let preview = w.preview_amendment(&c, &w.player, amended, 32).unwrap();
    assert_eq!(preview.preparation_days.units(), 4);
    assert_eq!(preview.consent_day, 15);
    w.issue(
        &c,
        Action::AmendCommitment {
            request: amended,
            value: 32,
        },
    )
    .unwrap();
    assert_eq!(w.player_country().projects[1].due_day, preview.due_day);
    assert_eq!(
        w.player_country().projects[1].stage,
        PolicyStage::Implementing
    );
    check(&w, &c);
}
#[test]
fn dismissal_requires_current_power_costs_cash_and_restores_civil_service_preparation() {
    let c = content();
    let mut w = start(&c, "AUR");
    appoint(&mut w, &c, "finance", "professional");
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
    let action = Action::DismissMinister {
        office: office("AUR", "finance"),
    };
    let hash = w.hash();
    assert!(w.issue(&c, action.clone()).is_err());
    assert_eq!(hash, w.hash());
    w.issue_developer(
        &c,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::LegislativeSupport,
            value: 70,
        },
    )
    .unwrap();
    assert_eq!(
        w.preview_dismissal(&c, &w.player, &office("AUR", "finance"))
            .unwrap(),
        Money(10)
    );
    let before = w.player_country().accounts.treasury;
    w.issue(&c, action).unwrap();
    assert_eq!(w.player_country().accounts.treasury.0, before.0 - 10);
    assert_eq!(
        w.policy_preparation_days(&c, &w.player, &"income_tax".into())
            .units(),
        30
    );
    check(&w, &c);
}
#[test]
fn definitions_state_snapshot_boundaries_privacy_and_ai_appointments_are_validated() {
    let c = content();
    let mut bad = c.clone();
    bad.cabinet.as_mut().unwrap().portfolios[0].policies.clear();
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.cabinet.as_mut().unwrap().candidates[0].skill = 0;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.identities = Some(IdentityCatalogue {
        people: vec![PersonDefinition {
            id: bad.cabinet.as_ref().unwrap().candidates[0].person.clone(),
            name: "Conflicting actor name".into(),
            source: "synthetic-original".into(),
            source_version: "test-v1".into(),
            quality: RecordQuality::Authored,
        }],
        offices: vec![],
        terms: vec![],
        currencies: vec![],
    });
    assert!(
        bad.validate()
            .unwrap_err()
            .contains("reference person identity")
    );
    let mut bad = c.clone();
    let portfolio = bad.cabinet.as_ref().unwrap().portfolios[0].clone();
    bad.identities = Some(IdentityCatalogue {
        people: vec![],
        offices: vec![OfficeDefinition {
            id: portfolio.id,
            country: portfolio.country,
            title: portfolio.title,
            role: OfficeRole::HeadOfState,
            required: false,
            validity: Validity {
                valid_from: "1990-01-01".parse().unwrap(),
                valid_to: None,
            },
            source: "synthetic-original".into(),
            source_version: "test-v1".into(),
            quality: RecordQuality::Authored,
        }],
        terms: vec![],
        currencies: vec![],
    });
    assert!(bad.validate().unwrap_err().contains("protected role"));
    let mut w = start(&c, "AUR");
    appoint(&mut w, &c, "finance", "professional");
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player)
        .unwrap()
        .cabinet
        .as_mut()
        .unwrap()
        .pending[0]
        .paid = Money(0);
    assert!(bad.validate(&c).is_err());
    let mut bad = w.clone();
    bad.countries.get_mut(&w.player).unwrap().cabinet = None;
    assert!(bad.validate(&c).is_err());
    let mut saved: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    saved["schema"] = 7.into();
    assert!(persistence::decode(&serde_json::to_vec(&saved).unwrap(), &c).is_err());
    let public = serde_json::to_string(&w.project(&c, Viewer::Observer).unwrap()).unwrap();
    assert!(!public.contains("PendingAppointment"));
    assert!(!public.contains("acceptability"));
    assert!(!public.contains("sponsor"));
    w.advance(&c, sim_core::DaySpan::new(120)).unwrap();
    assert!(
        !w.countries[&CountryId("BOR".into())]
            .cabinet
            .as_ref()
            .unwrap()
            .terms
            .is_empty()
    );
    assert!(
        w.public_report(&c, &w.player)
            .unwrap()
            .ministers
            .unwrap()
            .iter()
            .any(|p| p.holder.is_some())
    );
    check(&w, &c);
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player)
        .unwrap()
        .cabinet
        .as_mut()
        .unwrap()
        .terms[0]
        .since += 1;
    assert!(bad.validate(&c).is_err());
}

#[test]
fn appointment_cannot_spend_reserved_capital_or_schedule_past_the_campaign_limit() {
    let c = content();
    let mut w = start(&c, "AUR");
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
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
            Action::Appoint {
                office: office("AUR", "supply"),
                person: candidate("AUR", "supply", "professional")
            }
        )
        .is_err()
    );
    assert_eq!(hash, w.hash());
    check(&w, &c);
    let later = c.at_bookmark("2095-01-01").unwrap();
    let mut w = start(&later, "COR");
    w.advance(&later, sim_core::DaySpan::new(MAX_DAY - w.day.units() - 10))
        .unwrap();
    let hash = w.hash();
    assert!(
        w.issue(
            &later,
            Action::Appoint {
                office: office("COR", "supply"),
                person: candidate("COR", "supply", "professional")
            }
        )
        .is_err()
    );
    assert_eq!(hash, w.hash());
    check(&w, &later);
}
