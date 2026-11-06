use sim_core::*;

fn setup() -> (Content, World) {
    let content = Content::stable_elections_bundled().unwrap();
    let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    (content, world)
}
fn check(world: &World, content: &Content) {
    world.validate(content).unwrap();
    let bytes = persistence::encode(world, content).unwrap();
    let resumed = persistence::decode(&bytes, content).unwrap();
    assert_eq!(persistence::encode(&resumed, content).unwrap(), bytes);
    assert_eq!(resumed, persistence::replay(&resumed, content).unwrap());
}

#[test]
fn typed_receipts_and_cash_release_keep_the_pre_change_campaign_and_numeric_wire_format() {
    let (content, mut world) = setup();
    let command = world.command(
        world.player.clone(),
        Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        },
    );
    let encoded = serde_json::to_value(&command).unwrap();
    assert_eq!(encoded["id"], 1);
    let incoming = serde_json::from_value(encoded).unwrap();
    let receipt = world.execute_with_receipt(&content, incoming).unwrap();
    let accepted: CommandId = receipt.command;
    assert_eq!(accepted, CommandId(1));
    let before = persistence::encode(&world, &content).unwrap();
    assert_eq!(world.journal_entry(&accepted).unwrap().command, command);
    assert!(world.journal_entry(&CommandId(0)).is_none());
    assert!(world.journal_entry(&world.next_command).is_none());
    assert!(world.journal_entry(&CommandId(u64::MAX)).is_none());
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    assert_eq!(serde_json::to_value(&receipt).unwrap()["command"], 1);
    assert_eq!(
        world.player_country().commitments.as_ref().unwrap()[0].proposal_command,
        accepted
    );
    for action in [
        Action::Borrow {
            amount: Money(1000),
        },
        Action::HoldCash {
            target: Money(100),
            days: sim_core::DaySpan::new(30),
        },
        Action::ReleaseCash {
            request: CashHoldId(3),
        },
        Action::ProcureReserve {
            good: GoodKind::Food,
            days: 1,
        },
    ] {
        world.issue(&content, action).unwrap();
    }
    let record = &world
        .player_country()
        .cash_management
        .as_ref()
        .unwrap()
        .records[0];
    assert!(matches!(
        record.ended,
        Some((
            0,
            CashHoldEnd::Released {
                command: CommandId(4)
            }
        ))
    ));
    world.advance(&content, sim_core::DaySpan::new(14)).unwrap();
    assert_eq!(world.hash(), 0xbf9cebfd31a16e85);
    check(&world, &content);
    world.advance(&content, sim_core::DaySpan::new(94)).unwrap();
    assert_eq!(world.hash(), 0x9d112c7a163317b6);
    check(&world, &content);
}

#[test]
fn typed_budget_dispatch_keeps_pre_change_links_and_long_continuation_hash() {
    let (content, mut world) = setup();
    world
        .issue(
            &content,
            Action::ProposeBudget {
                year: 2000,
                targets: FiscalTargets {
                    income_tax: 30,
                    services: 10,
                    welfare: 5,
                },
            },
        )
        .unwrap();
    world
        .advance(&content, sim_core::DaySpan::new(430))
        .unwrap();
    assert_eq!(world.hash(), 0x5141a6d1566843de);
    let bill = &world.player_country().fiscal_plans.as_ref().unwrap().bills[0];
    assert_eq!(bill.status, BudgetBillStatus::Enacted);
    assert_eq!(bill.programmes.len(), 3);
    for (programme, number) in bill.programmes.iter().zip(2..=4) {
        let command: CommandId = programme.command;
        assert_eq!(command, CommandId(number));
        let record = world
            .player_country()
            .commitments
            .as_ref()
            .unwrap()
            .iter()
            .find(|record| record.id == programme.project)
            .unwrap();
        assert_eq!(record.proposal_command, command);
        let entry = world.journal_entry(&command).unwrap();
        assert_eq!(entry.command.id, command);
        assert_eq!(entry.budget_bill, Some(bill.id));
    }
    check(&world, &content);
}
