use sim_core::*;

fn fixture() -> (Content, World, LawId) {
    let content = Content::from_json(include_str!(
        "../../../data/synthetic-legislation-test.json"
    ))
    .unwrap();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    (content, world, LawId("high_service_authority".into()))
}
#[test]
fn paid_statutory_consent_and_implementation_unlock_permission_without_awarding_services() {
    let (content, mut world, law) = fixture();
    let services = PolicyId::from("services");
    let initial_services = world.player_country().services;
    assert!(
        world
            .preview(&content, &world.player, &services, 14)
            .unwrap_err()
            .contains("must be enacted")
    );
    let opening = world.player_country().accounts.clone();
    let preview = world.preview_law(&content, &world.player, &law).unwrap();
    assert_eq!(
        (preview.total_cost, preview.upfront, preview.reserved),
        (Money(20), Money(5), Money(15))
    );
    assert_eq!(
        (preview.review_day.units(), preview.enactment_day.units()),
        (14, 21)
    );
    world
        .issue(&content, Action::ProposeLaw { law: law.clone() })
        .unwrap();
    let id = LawBillId(1);
    let c = world.player_country();
    assert_eq!(c.accounts.treasury.0, opening.treasury.0 - 5);
    assert_eq!(c.accounts.firms.0, opening.firms.0 + 5);
    assert_eq!(c.committed_cash, Money(15));
    assert_eq!(
        c.legislation.as_ref().unwrap().bill(id).unwrap().status,
        LawBillStatus::AwaitingConsent
    );
    world.validate(&content).unwrap();
    assert_eq!(persistence::snapshot_schema(&world), 27);
    let bytes = persistence::encode(&world, &content).unwrap();
    world = persistence::decode(&bytes, &content).unwrap();
    assert_eq!(persistence::encode(&world, &content).unwrap(), bytes);
    world.advance(&content, DaySpan::new(14)).unwrap();
    assert_eq!(
        world
            .player_country()
            .legislation
            .as_ref()
            .unwrap()
            .bill(id)
            .unwrap()
            .status,
        LawBillStatus::Implementing
    );
    assert!(
        !world
            .policy_law_eligibility(&content, &world.player, &services, 14)
            .unwrap()[0]
            .enacted
    );
    world.advance(&content, DaySpan::new(7)).unwrap();
    let c = world.player_country();
    let state = c.legislation.as_ref().unwrap();
    let bill = state.bill(id).unwrap();
    assert_eq!(
        (bill.status, bill.paid, bill.reserved),
        (LawBillStatus::Enacted, Money(20), Money(0))
    );
    assert!(state.enacted.contains_key(&law));
    assert_eq!(c.committed_cash, Money(0));
    assert_eq!(c.services, initial_services);
    assert!(
        world
            .policy_law_eligibility(&content, &world.player, &services, 14)
            .unwrap()[0]
            .enacted
    );
    assert!(
        world
            .preview(&content, &world.player, &services, 14)
            .is_ok()
    );
    assert!(
        world
            .preview_law(&content, &world.player, &law)
            .unwrap_err()
            .contains("already enacted")
    );
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn withdrawal_and_refusal_release_only_unspent_cash_and_retain_drafting_costs() {
    let (content, mut world, law) = fixture();
    world
        .issue(&content, Action::ProposeLaw { law: law.clone() })
        .unwrap();
    let treasury = world.player_country().accounts.treasury;
    world
        .issue(
            &content,
            Action::WithdrawLaw {
                request: LawBillId(1),
            },
        )
        .unwrap();
    assert_eq!(world.player_country().accounts.treasury, treasury);
    assert_eq!(world.player_country().committed_cash, Money(0));
    let bill = world
        .player_country()
        .legislation
        .as_ref()
        .unwrap()
        .bill(LawBillId(1))
        .unwrap();
    assert_eq!(
        (bill.status, bill.paid, bill.reserved),
        (LawBillStatus::Withdrawn, Money(5), Money(0))
    );
    let before = world.hash();
    assert!(
        world
            .issue(&content, Action::ProposeLaw { law: law.clone() })
            .is_err()
    );
    assert_eq!(world.hash(), before);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    let (_, mut refused, _) = fixture();
    refused.issue(&content, Action::ProposeLaw { law }).unwrap();
    refused
        .countries
        .get_mut(&refused.player)
        .unwrap()
        .legislative_support = 0;
    refused.advance(&content, DaySpan::new(14)).unwrap();
    refused.validate(&content).unwrap();
    let bill = refused
        .player_country()
        .legislation
        .as_ref()
        .unwrap()
        .bill(LawBillId(1))
        .unwrap();
    assert_eq!(
        (bill.status, bill.paid, bill.reserved),
        (LawBillStatus::Refused, Money(5), Money(0))
    );
    assert!(
        refused
            .player_country()
            .legislation
            .as_ref()
            .unwrap()
            .enacted
            .is_empty()
    );
}
#[test]
fn statutory_pack_preserves_the_preceding_default_and_old_pending_world_hashes() {
    // Published rules-25 goldens remain an explicit compatibility fixture.
    let default = Content::statistical_history_bundled().unwrap();
    assert_eq!(default.hash(), 0x0a0defa128f0a947);
    let mut world = World::new(&default, CountryId("GBR".into()), 17).unwrap();
    world
        .issue(
            &default,
            Action::ProcureReserve {
                good: GoodKind::Energy,
                days: NeedDays::new(1),
            },
        )
        .unwrap();
    world.advance(&default, DaySpan::new(20)).unwrap();
    assert_eq!(world.hash(), 0x476bea2742f65dcd);
    assert!(world.player_country().legislation.is_none());
    assert_eq!(persistence::snapshot_schema(&world), 26);
    let new = Content::legislation_bundled().unwrap();
    assert_eq!(new.rules, 26);
    assert_eq!(new.countries, default.countries);
    assert_eq!(
        Content::bundled_for_hash(new.hash()).unwrap().hash(),
        new.hash()
    );
    assert_eq!(persistence::replay(&world, &default).unwrap(), world);
}

#[test]
fn programme_budget_and_amendment_paths_cannot_bypass_statutory_permission_or_actual_cash() {
    let (content, mut world, law) = fixture();
    let before = world.hash();
    let targets = FiscalTargets {
        income_tax: world.player_country().tax,
        services: 14,
        welfare: world.player_country().welfare,
    };
    assert!(
        world
            .issue(
                &content,
                Action::ProposeBudget {
                    year: 2000,
                    targets
                }
            )
            .unwrap_err()
            .contains("must be enacted")
    );
    assert_eq!(world.hash(), before);
    world
        .issue(
            &content,
            Action::Enact {
                policy: "services".into(),
                value: 12,
            },
        )
        .unwrap();
    let request = world.player_country().projects[0].id;
    let before = world.hash();
    assert!(
        world
            .issue(&content, Action::AmendCommitment { request, value: 14 })
            .unwrap_err()
            .contains("must be enacted")
    );
    assert_eq!(world.hash(), before);
    let (_, mut empty, _) = fixture();
    let c = empty.countries.get_mut(&empty.player).unwrap();
    let available = c.available_cash().0;
    c.accounts.treasury.0 -= available;
    c.accounts.external.0 += available;
    c.budget.capital.0 += available;
    empty.validate(&content).unwrap();
    let before = empty.hash();
    assert!(
        empty
            .issue(&content, Action::ProposeLaw { law: law.clone() })
            .unwrap_err()
            .contains("actual uncommitted cash")
    );
    assert_eq!(empty.hash(), before);
}

#[test]
fn sponsoring_government_changes_cancel_pending_work_and_preserve_enacted_statutes() {
    let (content, mut pending, law) = fixture();
    pending
        .issue(&content, Action::ProposeLaw { law: law.clone() })
        .unwrap();
    let c = pending.countries.get_mut(&pending.player).unwrap();
    c.government_number = c.government_number.checked_add(1).unwrap();
    pending.advance(&content, DaySpan::new(1)).unwrap();
    pending.validate(&content).unwrap();
    let bill = pending
        .player_country()
        .legislation
        .as_ref()
        .unwrap()
        .bill(LawBillId(1))
        .unwrap();
    assert_eq!(
        (bill.status, bill.paid, bill.reserved),
        (LawBillStatus::Superseded, Money(5), Money(0))
    );
    assert_eq!(pending.player_country().committed_cash, Money(0));
    let (_, mut enacted, _) = fixture();
    enacted
        .issue(&content, Action::ProposeLaw { law: law.clone() })
        .unwrap();
    enacted.advance(&content, DaySpan::new(21)).unwrap();
    let c = enacted.countries.get_mut(&enacted.player).unwrap();
    c.government_number = c.government_number.checked_add(1).unwrap();
    enacted.advance(&content, DaySpan::new(1)).unwrap();
    enacted.validate(&content).unwrap();
    assert!(
        enacted
            .player_country()
            .legislation
            .as_ref()
            .unwrap()
            .enacted
            .contains_key(&law)
    );
}

#[test]
fn dated_proposal_notice_does_not_grant_permission_and_malformed_legal_state_is_rejected() {
    let (mut content, _, law) = fixture();
    content.legislation.as_mut().unwrap().definitions[0].available_from =
        CivilDate::new(2000, 1, 15).unwrap();
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let before = world.hash();
    assert!(
        world
            .issue(&content, Action::ProposeLaw { law: law.clone() })
            .is_err()
    );
    assert_eq!(world.hash(), before);
    // Other countries can produce important events before this statutory date.
    while world.day.units() < 14 {
        let elapsed = world.advance_to_event(&content).unwrap();
        assert!(elapsed.units() > 0);
        assert!(world.day.units() <= 14);
    }
    assert!(
        world.events.iter().any(
            |e| e.country == world.player && e.text.contains("Statutory proposal date reached")
        )
    );
    assert!(
        world
            .player_country()
            .legislation
            .as_ref()
            .unwrap()
            .enacted
            .is_empty()
    );
    world
        .issue(&content, Action::ProposeLaw { law: law.clone() })
        .unwrap();
    world.advance(&content, DaySpan::new(21)).unwrap();
    world.validate(&content).unwrap();
    let mut fake = world.clone();
    fake.countries
        .get_mut(&fake.player)
        .unwrap()
        .legislation
        .as_mut()
        .unwrap()
        .bills[0]
        .status = LawBillStatus::Withdrawn;
    assert!(fake.validate(&content).is_err());
    let mut fake = world.clone();
    fake.countries
        .get_mut(&fake.player)
        .unwrap()
        .legislation
        .as_mut()
        .unwrap()
        .enacted
        .get_mut(&law)
        .unwrap()
        .day = CampaignDay::new(14);
    assert!(fake.validate(&content).is_err());
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut wrong: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    wrong["schema"] = serde_json::json!(26);
    assert!(
        persistence::decode(&serde_json::to_vec(&wrong).unwrap(), &content)
            .unwrap_err()
            .contains("format 27")
    );
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn reserved_statutory_work_crosses_year_end_without_rebooking_drafting_or_releasing_capital() {
    let (mut content, _, law) = fixture();
    content.start = "2000-12-20".into();
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.issue(&content, Action::ProposeLaw { law }).unwrap();
    world.advance(&content, DaySpan::new(7)).unwrap();
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut restored = persistence::decode(&bytes, &content).unwrap();
    world.advance(&content, DaySpan::new(14)).unwrap();
    restored.advance(&content, DaySpan::new(14)).unwrap();
    assert_eq!(restored, world);
    world.validate(&content).unwrap();
    let c = world.player_country();
    assert_eq!(c.budget.capital, Money(15));
    assert_eq!(c.budget_reviews[0].period.capital, Money(5));
    assert_eq!(c.committed_cash, Money(0));
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn configured_ai_statutes_use_shared_paid_commands_and_legal_definitions_validate() {
    let (content, mut world, _) = fixture();
    world.advance(&content, DaySpan::new(120)).unwrap();
    world.validate(&content).unwrap();
    let proposals: Vec<_> = world
        .journal
        .iter()
        .filter(|e| matches!(e.command.action, Action::ProposeLaw { .. }))
        .collect();
    assert!(!proposals.is_empty());
    assert!(
        proposals
            .iter()
            .all(|e| e.ai && e.command.issuer != world.player)
    );
    assert!(
        world
            .countries
            .iter()
            .filter(|(id, _)| *id != &world.player)
            .any(|(_, c)| !c.legislation.as_ref().unwrap().enacted.is_empty())
    );
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    let mut bad = content.clone();
    let duplicate = bad.legislation.as_ref().unwrap().definitions[0].clone();
    bad.legislation
        .as_mut()
        .unwrap()
        .definitions
        .push(duplicate);
    assert!(bad.validate().is_err());
    let mut bad = content.clone();
    bad.legislation.as_mut().unwrap().policy_requirements[0].minimum_value = 19;
    assert!(bad.validate().is_err());
    let mut bad = content.clone();
    bad.legislation.as_mut().unwrap().definitions[0].source = "missing".into();
    assert!(bad.validate().is_err());
    let mut bad = content.clone();
    bad.legislation.as_mut().unwrap().cost_bps = BasisPoints::new(u32::MAX);
    assert!(bad.validate().is_err());
}

#[test]
fn funded_drafting_rolls_back_after_actual_event_reference_exhaustion() {
    let (content, mut world, _) = fixture();
    let template = world
        .events
        .iter()
        .find(|e| e.country == world.player)
        .unwrap()
        .clone();
    world.events = (0..400).map(|_| template.clone()).collect();
    for value in world.event_sequences.as_mut().unwrap().values_mut() {
        *value = 1;
    }
    let law = LawId("high_service_authority".into());
    let preview = world
        .preview_law(&content, &world.player, &law)
        .expect("Paid proposal must be eligible before exhausting references");
    let count = world
        .events
        .iter()
        .filter(|e| e.country == world.player)
        .count() as u64;
    for (index, event) in world
        .events
        .iter_mut()
        .filter(|e| e.country == world.player)
        .enumerate()
    {
        event.id = Some(EventId {
            country: world.player.clone(),
            sequence: u64::MAX - count + index as u64,
        });
    }
    world
        .event_sequences
        .as_mut()
        .unwrap()
        .insert(world.player.clone(), u64::MAX);
    world.validate(&content).unwrap();
    let before = world.clone();
    let error = world
        .issue(&content, Action::ProposeLaw { law })
        .unwrap_err();
    assert!(error.contains("reference space exhausted"), "{error}");
    assert_eq!(world, before);
    world.validate(&content).unwrap();
    assert_eq!((preview.upfront, preview.reserved), (Money(5), Money(15)));
}
