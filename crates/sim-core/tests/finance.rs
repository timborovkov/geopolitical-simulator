use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!("../../../data/synthetic-finance-test.json")).unwrap()
}
fn world(content: &Content, id: &str) -> World {
    World::new(content, CountryId(id.into()), 17).unwrap()
}
fn borrow(world: &mut World, content: &Content, amount: i64) -> LoanId {
    let id = LoanId(world.next_command.0);
    world
        .issue(
            content,
            Action::Borrow {
                amount: Money(amount),
            },
        )
        .unwrap();
    id
}
fn replay(world: &World, content: &Content) {
    world.validate(content).unwrap();
    persistence::replay(world, content).unwrap();
}
fn rejected(world: &mut World, content: &Content, action: Action) {
    let before = world.hash();
    assert!(world.issue(content, action).is_err());
    assert_eq!(world.hash(), before);
}

#[test]
fn consent_withholds_cash_until_the_decision_and_survives_save_resume() {
    let content = fixture();
    let mut world = world(&content, "AUR");
    let cash = world.player_country().accounts.treasury;
    let debt = world.player_country().debt;
    let id = borrow(&mut world, &content, 10000);
    assert_eq!(world.player_country().accounts.treasury, cash);
    assert_eq!(world.player_country().debt, debt);
    assert_eq!(world.player_country().budget.borrowing, Money(0));
    rejected(&mut world, &content, Action::Borrow { amount: Money(1) });
    world.advance(&content, sim_core::DaySpan::new(13)).unwrap();
    let bytes = persistence::encode(&world, &content).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        5
    );
    let mut resumed = persistence::decode(&bytes, &content).unwrap();
    resumed
        .advance(&content, sim_core::DaySpan::new(1))
        .unwrap();
    world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(world.hash(), resumed.hash());
    let country = world.player_country();
    assert_eq!(country.debt.0, debt.0 + 10000);
    assert_eq!(country.budget.borrowing, Money(10000));
    assert!(country.financing.as_ref().unwrap().pending.is_none());
    assert_eq!(country.financing.as_ref().unwrap().instruments[1].id, id);
    assert!(world.ledger.iter().any(|e| e.country == world.player
        && e.day == 14
        && e.from == Account::External
        && e.to == Account::Treasury
        && e.amount == Money(10000)));
    replay(&world, &content);
}

#[test]
fn authority_is_checked_at_proposal_and_again_at_disbursement() {
    let content = fixture();
    let mut world = world(&content, "AUR");
    borrow(&mut world, &content, 10000);
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
    world.advance(&content, sim_core::DaySpan::new(14)).unwrap();
    assert_eq!(world.player_country().budget.borrowing, Money(0));
    assert_eq!(world.player_country().debt, Money(100000 / 3));
    assert!(
        world
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .pending
            .is_none()
    );
    assert!(
        world
            .events
            .iter()
            .any(|e| e.text.contains("refused at decision"))
    );
    rejected(&mut world, &content, Action::Borrow { amount: Money(100) });
    replay(&world, &content);
}

#[test]
fn cancellation_creates_no_debt_and_wrong_ids_are_atomic() {
    let content = fixture();
    let mut world = world(&content, "BOR");
    let id = borrow(&mut world, &content, 5000);
    rejected(
        &mut world,
        &content,
        Action::CancelBorrow {
            request: LoanId(id.0 + 1),
        },
    );
    world
        .issue(&content, Action::CancelBorrow { request: id })
        .unwrap();
    rejected(&mut world, &content, Action::CancelBorrow { request: id });
    world.advance(&content, sim_core::DaySpan::new(21)).unwrap();
    assert_eq!(world.player_country().debt, Money(100000 / 3));
    assert_eq!(world.player_country().budget.borrowing, Money(0));
    replay(&world, &content);
}

#[test]
fn authorized_decree_is_immediate_and_principal_redemption_is_fifo() {
    let content = fixture();
    let mut world = world(&content, "COR");
    for _ in 0..4 {
        borrow(&mut world, &content, 10000);
    }
    assert_eq!(world.player_country().accounts.treasury, Money(45000));
    world
        .issue(
            &content,
            Action::Repay {
                amount: Money(34333),
            },
        )
        .unwrap();
    let state = world.player_country().financing.as_ref().unwrap();
    assert_eq!(state.instruments[0].outstanding, Money(0));
    assert_eq!(state.instruments[1].outstanding, Money(9000));
    assert_eq!(world.player_country().debt, Money(39000));
    assert_eq!(
        world.player_country().accounts.total(),
        world.player_country().opening_money.0
    );
    replay(&world, &content);
}

#[test]
fn contested_borrowing_bypasses_legal_consent_but_is_a_costly_ordinary_game_action() {
    let mut content = fixture();
    content.countries[2].government.threshold = 100;
    let mut world = world(&content, "COR");
    rejected(
        &mut world,
        &content,
        Action::Borrow {
            amount: Money(10000),
        },
    );
    let before = world.player_country().clone();
    world
        .issue(
            &content,
            Action::EmergencyBorrow {
                amount: Money(10000),
            },
        )
        .unwrap();
    let country = world.player_country();
    assert_eq!(
        country.accounts.treasury.0,
        before.accounts.treasury.0 + 10000
    );
    assert_eq!(country.approval, before.approval - 15);
    assert_eq!(country.legislative_support, before.legislative_support - 20);
    assert_eq!(country.elite_support, before.elite_support - 10);
    assert_eq!(country.council_support, before.council_support - 20);
    assert!(!world.is_developer_modified());
    assert!(matches!(
        country.financing.as_ref().unwrap().instruments[1].origin,
        DebtOrigin::AcceptedRequest { contested: true }
    ));
    rejected(
        &mut world,
        &content,
        Action::EmergencyBorrow { amount: Money(1) },
    );
    replay(&world, &content);
}

#[test]
fn pending_credit_is_rechecked_after_an_emergency_draws_down_headroom() {
    let mut content = fixture();
    content
        .finance
        .as_mut()
        .unwrap()
        .emergency
        .as_mut()
        .unwrap()
        .legislative_cost = 0;
    let mut world = world(&content, "AUR");
    for _ in 0..11 {
        borrow(&mut world, &content, 10000);
        world.advance(&content, sim_core::DaySpan::new(14)).unwrap();
    }
    let pending = borrow(&mut world, &content, 5000);
    world
        .issue(
            &content,
            Action::EmergencyBorrow {
                amount: Money(5000),
            },
        )
        .unwrap();
    let debt = world.player_country().debt;
    world.advance(&content, sim_core::DaySpan::new(14)).unwrap();
    assert_eq!(world.player_country().debt, debt);
    assert_eq!(debt.0, 148333);
    assert!(
        world
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .instruments
            .iter()
            .all(|i| i.id != pending)
    );
    assert!(
        world
            .events
            .iter()
            .any(|e| e.text.contains("refused at decision: Creditors refuse"))
    );
    replay(&world, &content);
}

#[test]
fn malformed_terms_pending_dates_and_downgraded_envelopes_are_rejected() {
    let content = fixture();
    let mut world = world(&content, "AUR");
    borrow(&mut world, &content, 10000);
    let mut malformed = world.clone();
    malformed
        .countries
        .get_mut(&world.player)
        .unwrap()
        .financing
        .as_mut()
        .unwrap()
        .pending
        .as_mut()
        .unwrap()
        .decision_day = u32::MAX;
    assert!(malformed.validate(&content).is_err());
    let mut malformed = world.clone();
    malformed
        .countries
        .get_mut(&world.player)
        .unwrap()
        .financing
        .as_mut()
        .unwrap()
        .instruments[0]
        .outstanding
        .0 -= 1;
    assert!(malformed.validate(&content).is_err());
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for schema in [2, 3, 4] {
        envelope["schema"] = schema.into();
        assert!(
            persistence::decode(&serde_json::to_vec(&envelope).unwrap(), &content)
                .unwrap_err()
                .contains("format 5")
        );
    }
    let mut invalid = content.clone();
    invalid.finance.as_mut().unwrap().authorities[1].id =
        invalid.finance.as_ref().unwrap().authorities[0].id.clone();
    assert!(invalid.validate().is_err());
}

#[test]
fn party_route_and_ai_use_the_same_consent_workflow() {
    let content = fixture();
    let mut world = world(&content, "BOR");
    borrow(&mut world, &content, 1000);
    world.advance(&content, sim_core::DaySpan::new(20)).unwrap();
    assert_eq!(world.player_country().budget.borrowing, Money(0));
    world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(world.player_country().budget.borrowing, Money(1000));
    world
        .advance(&content, sim_core::DaySpan::new(339))
        .unwrap();
    assert!(world.journal.iter().any(|entry| entry.ai));
    replay(&world, &content);
}

#[test]
fn successor_government_must_seek_its_own_financing_mandate() {
    let content = fixture();
    let mut world = world(&content, "AUR");
    // A real confidence-triggered election occurs before the loan decision.
    world.advance(&content, sim_core::DaySpan::new(89)).unwrap();
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
    world.advance(&content, sim_core::DaySpan::new(61)).unwrap();
    let election = world.player_country().next_election.unwrap();
    assert!(election > world.day.units());
    world
        .advance(
            &content,
            sim_core::DaySpan::new(election - world.day.units() - 1),
        )
        .unwrap();
    world
        .issue_developer(
            &content,
            world.player.clone(),
            DeveloperEdit {
                field: DeveloperField::LegislativeSupport,
                value: 100,
            },
        )
        .unwrap();
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
    let request = borrow(&mut world, &content, 1000);
    let sponsor = world.player_country().government_number;
    world.advance(&content, sim_core::DaySpan::new(14)).unwrap();
    assert!(world.player_country().government_number > sponsor);
    assert!(
        world
            .events
            .iter()
            .any(|e| e.text.contains("Sponsoring government has changed"))
    );
    assert!(
        world
            .player_country()
            .financing
            .as_ref()
            .unwrap()
            .instruments
            .iter()
            .all(|i| i.id != request)
    );
    replay(&world, &content);
}

#[test]
fn a_cash_constrained_ai_borrows_through_authority_and_waits_for_cash() {
    let mut content = fixture();
    let definition = &mut content.countries[0];
    definition.tax = Percent::new(10);
    definition.services = Percent::new(18);
    definition.welfare = Percent::new(12);
    let mut world = world(&content, "COR");
    world.advance(&content, sim_core::DaySpan::new(90)).unwrap();
    let issuer = CountryId("AUR".into());
    let request = world
        .journal
        .iter()
        .find(|entry| {
            entry.ai
                && entry.command.issuer == issuer
                && matches!(entry.command.action, Action::Borrow { .. })
        })
        .unwrap();
    let request = request.clone();
    let decision = request.command.day + 14;
    if decision > world.day.units() {
        world
            .advance(
                &content,
                sim_core::DaySpan::new(decision - world.day.units()),
            )
            .unwrap();
    }
    let state = world.countries[&issuer].financing.as_ref().unwrap();
    let instrument = state
        .instruments
        .iter()
        .find(|i| i.id.0 == request.command.id.0)
        .unwrap();
    assert_eq!(instrument.issued_day, decision);
    assert!(matches!(
        instrument.origin,
        DebtOrigin::AcceptedRequest { contested: false }
    ));
    replay(&world, &content);
}
