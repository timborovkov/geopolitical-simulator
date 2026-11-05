use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!("../../../data/synthetic-election-test.json")).unwrap()
}
fn start(c: &Content, id: &str) -> World {
    World::new(c, CountryId(id.into()), 17).unwrap()
}
fn request(w: &mut World, c: &Content) -> ElectionRequestId {
    let id = ElectionRequestId(w.next_command);
    w.issue(c, Action::RequestElection).unwrap();
    id
}
fn check(w: &World, c: &Content) {
    w.validate(c).unwrap();
    let bytes = persistence::encode(w, c).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        9
    );
    assert_eq!(persistence::decode(&bytes, c).unwrap(), *w);
    persistence::replay(w, c).unwrap();
}
#[test]
fn authority_announces_and_conducts_a_fixed_poll_that_government_cannot_withdraw() {
    let c = content();
    let mut w = start(&c, "AUR");
    let opening = w.hash();
    assert!(w.issue(&c, Action::RequestElection).is_err());
    assert_eq!(opening, w.hash());
    w.advance(&c, 365).unwrap();
    let p = w.preview_election(&c, &w.player).unwrap();
    assert_eq!(p.decision_day, 372);
    assert_eq!(p.vote_day, 432);
    assert_eq!(p.fee, Money(50));
    assert_eq!(p.previous_vote_day, Some(730));
    let hash = w.hash();
    w.preview_election(&c, &w.player).unwrap();
    assert_eq!(hash, w.hash());
    let before = w.player_country().accounts.treasury;
    let id = request(&mut w, &c);
    assert_eq!(w.player_country().accounts.treasury.0, before.0 - 50);
    assert_eq!(w.player_country().next_election, Some(730));
    assert_eq!(
        w.public_report(&c, &w.player)
            .unwrap()
            .electoral_authority
            .unwrap()
            .next_poll,
        Some(730)
    );
    check(&w, &c);
    w.advance(&c, 7).unwrap();
    assert_eq!(w.player_country().next_election, Some(432));
    assert_eq!(
        w.public_report(&c, &w.player)
            .unwrap()
            .electoral_authority
            .unwrap()
            .next_poll,
        Some(432)
    );
    for action in [
        Action::WithdrawElectionRequest { request: id },
        Action::RequestElection,
    ] {
        let hash = w.hash();
        assert!(w.issue(&c, action).is_err());
        assert_eq!(hash, w.hash());
    }
    check(&w, &c);
    w.advance(&c, 60).unwrap();
    let state = w.player_country().electoral_authority.as_ref().unwrap();
    assert_eq!(state.cases[0].status, ElectionCaseStatus::Conducted);
    assert_eq!(state.polls[0].day, 432);
    assert_eq!(state.polls[0].request, Some(id));
    assert_eq!(
        state.polls[0]
            .seats
            .values()
            .map(|s| u32::from(*s))
            .sum::<u32>(),
        100
    );
    let next = date(432)
        .parse::<CivilDate>()
        .unwrap()
        .anniversary(4)
        .unwrap()
        .campaign_day()
        .unwrap();
    assert_eq!(w.player_country().next_election, Some(next));
    assert_eq!(
        w.public_report(&c, &w.player)
            .unwrap()
            .electoral_authority
            .unwrap()
            .last_poll
            .unwrap()
            .day,
        432
    );
    check(&w, &c);
}
#[test]
fn presidential_and_royal_governments_cannot_order_polls_outside_their_mandate() {
    let c = content();
    for id in ["BOR", "COR"] {
        let mut w = start(&c, id);
        let hash = w.hash();
        let error = w.preview_election(&c, &w.player).unwrap_err();
        assert!(error.contains("no power"));
        assert!(w.issue(&c, Action::RequestElection).is_err());
        assert_eq!(hash, w.hash());
        check(&w, &c);
    }
}
#[test]
fn withdrawal_and_refusal_retain_administration_and_the_statutory_interval() {
    let c = content();
    let mut withdrawn = start(&c, "AUR");
    withdrawn.advance(&c, 365).unwrap();
    let id = request(&mut withdrawn, &c);
    let before = withdrawn.player_country().accounts.clone();
    withdrawn
        .issue(&c, Action::WithdrawElectionRequest { request: id })
        .unwrap();
    assert_eq!(withdrawn.player_country().accounts, before);
    assert_eq!(withdrawn.player_country().next_election, Some(730));
    let hash = withdrawn.hash();
    assert!(withdrawn.issue(&c, Action::RequestElection).is_err());
    assert_eq!(hash, withdrawn.hash());
    check(&withdrawn, &c);
    let mut refused = start(&c, "AUR");
    refused.advance(&c, 365).unwrap();
    request(&mut refused, &c);
    // An ordinary contested financing action spends political backing before review.
    refused
        .issue(
            &c,
            Action::EmergencyBorrow {
                amount: Money(10000),
            },
        )
        .unwrap();
    assert!(refused.player_country().legislative_support < 51);
    refused.advance(&c, 7).unwrap();
    assert!(matches!(
        refused
            .player_country()
            .electoral_authority
            .as_ref()
            .unwrap()
            .cases[0]
            .status,
        ElectionCaseStatus::Refused { .. }
    ));
    assert_eq!(refused.player_country().next_election, Some(730));
    assert!(!refused.is_developer_modified());
    check(&refused, &c);
}
#[test]
fn an_earlier_mandatory_confidence_poll_supersedes_an_announced_poll() {
    let mut c = content();
    let rules = c.elections.as_mut().unwrap();
    rules.minimum_interval_days = 1;
    rules.campaign_days = 180;
    c.countries[0].parties[0].seats = 40;
    c.countries[0].parties[1].seats = 60;
    c.validate().unwrap();
    let mut w = start(&c, "AUR");
    w.issue(
        &c,
        Action::Enact {
            policy: "coalition".into(),
            value: 1,
        },
    )
    .unwrap();
    w.advance(&c, 60).unwrap();
    request(&mut w, &c);
    w.advance(&c, 7).unwrap();
    assert_eq!(w.player_country().next_election, Some(247));
    check(&w, &c);
    w.advance(&c, 113).unwrap();
    assert_eq!(w.day, 180);
    assert_eq!(w.player_country().next_election, Some(181));
    assert_eq!(
        w.player_country()
            .electoral_authority
            .as_ref()
            .unwrap()
            .cases[0]
            .status,
        ElectionCaseStatus::Superseded
    );
    check(&w, &c);
    w.advance(&c, 1).unwrap();
    let poll = &w
        .player_country()
        .electoral_authority
        .as_ref()
        .unwrap()
        .polls[0];
    assert_eq!(poll.day, 181);
    assert!(poll.request.is_none());
    assert!(!w.is_developer_modified());
    check(&w, &c);
}
#[test]
fn a_costed_contested_constitutional_change_can_supersede_the_electoral_mandate() {
    let mut c = content();
    c.elections.as_mut().unwrap().minimum_interval_days = 1;
    let mut w = start(&c, "AUR");
    for _ in 0..3 {
        let p = w.preview(&c, &w.player, &"coalition".into(), 1).unwrap();
        w.issue(
            &c,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
        w.advance(&c, p.delay).unwrap();
    }
    let id = request(&mut w, &c);
    w.advance(&c, 7).unwrap();
    assert_eq!(
        w.player_country()
            .electoral_authority
            .as_ref()
            .unwrap()
            .announced()
            .unwrap()
            .id,
        id
    );
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, 14).unwrap();
    assert_eq!(
        w.governing_regime(&c, &w.player).unwrap(),
        &Regime::Military
    );
    assert!(w.player_country().next_election.is_none());
    assert_eq!(
        w.player_country()
            .electoral_authority
            .as_ref()
            .unwrap()
            .cases[0]
            .status,
        ElectionCaseStatus::Superseded
    );
    let hash = w.hash();
    assert!(w.issue(&c, Action::RequestElection).is_err());
    assert_eq!(hash, w.hash());
    assert!(!w.is_developer_modified());
    check(&w, &c);
}
#[test]
fn request_cash_calendar_and_existing_poll_deadlines_are_enforced() {
    let c = content();
    let mut w = start(&c, "AUR");
    w.advance(&c, 365).unwrap();
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
    assert!(w.issue(&c, Action::RequestElection).is_err());
    assert_eq!(w.hash(), hash);
    check(&w, &c);
    let mut near = start(&c, "AUR");
    near.advance(&c, 700).unwrap();
    assert!(
        near.preview_election(&c, &near.player)
            .unwrap_err()
            .contains("postpone")
    );
    let later = c.at_bookmark("2095-01-01").unwrap();
    let mut near = start(&later, "AUR");
    near.advance(&later, MAX_DAY - near.day - 10).unwrap();
    assert!(
        near.preview_election(&later, &near.player)
            .unwrap_err()
            .contains("calendar")
    );
    check(&near, &later);
}
#[test]
fn independent_mandates_private_requests_saved_state_and_ai_share_validated_rules() {
    let c = content();
    let mut bad = c.clone();
    bad.elections.as_mut().unwrap().mandates[0].authority =
        bad.cabinet.as_ref().unwrap().portfolios[0].id.clone();
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.elections.as_mut().unwrap().campaign_days = 0;
    assert!(bad.validate().is_err());
    let mut w = start(&c, "AUR");
    w.advance(&c, 365).unwrap();
    request(&mut w, &c);
    let public = serde_json::to_string(&w.project(&c, Viewer::Observer).unwrap()).unwrap();
    assert!(!public.contains("proposed_day"));
    assert!(!public.contains("sponsor"));
    assert!(!public.contains("paid"));
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player)
        .unwrap()
        .electoral_authority
        .as_mut()
        .unwrap()
        .cases[0]
        .vote_day += 1;
    assert!(bad.validate(&c).is_err());
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player)
        .unwrap()
        .electoral_authority = None;
    assert!(bad.validate(&c).is_err());
    let mut saved: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    saved["schema"] = 8.into();
    assert!(persistence::decode(&serde_json::to_vec(&saved).unwrap(), &c).is_err());
    check(&w, &c);
    let mut ai = start(&c, "BOR");
    ai.advance(&c, 1400).unwrap();
    assert!(ai.journal.iter().any(|e| e.ai
        && e.command.issuer.0 == "AUR"
        && matches!(e.command.action, Action::RequestElection)));
    check(&ai, &c);
}

#[test]
fn an_announced_poll_on_a_confidence_review_day_actually_counts_the_seats() {
    let mut c = content();
    let rules = c.elections.as_mut().unwrap();
    rules.minimum_interval_days = 1;
    rules.decision_days = 60;
    rules.campaign_days = 60;
    c.countries[0].parties[0].seats = 40;
    c.countries[0].parties[1].seats = 60;
    c.validate().unwrap();
    let mut w = start(&c, "AUR");
    w.issue(
        &c,
        Action::Enact {
            policy: "coalition".into(),
            value: 1,
        },
    )
    .unwrap();
    w.advance(&c, 60).unwrap();
    request(&mut w, &c);
    w.advance(&c, 60).unwrap();
    assert_eq!(w.player_country().next_election, Some(180));
    w.advance(&c, 60).unwrap();
    let state = w.player_country().electoral_authority.as_ref().unwrap();
    assert_eq!(state.polls.len(), 1);
    assert_eq!(state.polls[0].day, 180);
    assert_ne!(
        state.polls[0].seats[&"AUR-1".into()],
        40,
        "A recorded poll must run the allocation, not retain the opening seats after a confidence deferral"
    );
    w.advance(&c, 1).unwrap();
    assert_eq!(
        w.player_country()
            .electoral_authority
            .as_ref()
            .unwrap()
            .polls
            .len(),
        1
    );
    check(&w, &c);
}
