use sim_core::*;

#[test]
fn receipts_identify_owned_outcomes_without_changing_published_state() {
    let content = Content::programme_events_bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let mut ordinary = world.clone();
    let action = Action::Enact {
        policy: "income_tax".into(),
        value: 30,
    };
    ordinary.issue(&content, action.clone()).unwrap();
    let command = world.command(world.player.clone(), action);
    let incoming: Command = serde_json::from_slice(&serde_json::to_vec(&command).unwrap()).unwrap();
    let receipt = world.execute_with_receipt(&content, incoming).unwrap();
    assert_eq!(receipt.command, command.id);
    assert_eq!(receipt.outcomes, vec!["GBR#2".parse().unwrap()]);
    assert_eq!(world, ordinary);
    let bytes = persistence::encode(&world, &content).unwrap();
    for id in &receipt.outcomes {
        let outcome = world
            .event_by_reference(&content, Viewer::Government(world.player.clone()), id)
            .unwrap()
            .unwrap();
        assert_eq!(outcome.day, command.day);
        assert!(matches!(
            outcome.context,
            Some(EventContext::Commitment {
                stage: CommitmentStage::Proposed,
                ..
            })
        ));
        assert!(
            world
                .event_by_reference(&content, Viewer::Observer, id)
                .unwrap()
                .is_none()
        );
        assert!(
            world
                .event_by_reference(&content, Viewer::Government(CountryId("USA".into())), id)
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(persistence::encode(&world, &content).unwrap(), bytes);
    world.advance(&content, sim_core::DaySpan::new(60)).unwrap();
    assert_eq!(world.hash(), 0x8bf8784e005cff53);
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}

#[test]
fn failed_envelopes_return_no_receipt_and_preserve_bytes_and_allocators() {
    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let action = Action::Enact {
        policy: "income_tax".into(),
        value: 30,
    };
    let valid = world.command(world.player.clone(), action);
    for variant in 0..5 {
        let before = persistence::encode(&world, &content).unwrap();
        let mut bad = valid.clone();
        match variant {
            0 => bad.id.0 += 1,
            1 => bad.day = bad.day.checked_add(DaySpan::new(1)).unwrap(),
            2 => bad.issuer = CountryId("USA".into()),
            3 => {
                bad.action = Action::Enact {
                    policy: "absent".into(),
                    value: 30,
                }
            }
            _ => {
                bad.action = Action::Developer {
                    target: world.player.clone(),
                    edit: DeveloperEdit {
                        field: DeveloperField::Approval,
                        value: 99,
                    },
                }
            }
        }
        assert!(world.execute_with_receipt(&content, bad).is_err());
        assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    }
    let receipt = world.execute_with_receipt(&content, valid.clone()).unwrap();
    assert_eq!(receipt.command, valid.id);
    assert_eq!(receipt.outcomes, vec!["GBR#2".parse().unwrap()]);
    let before = persistence::encode(&world, &content).unwrap();
    assert!(world.execute_with_receipt(&content, valid).is_err());
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
}

#[test]
fn receipts_keep_legacy_outcomes_unreferenced_and_current_general_events_addressable() {
    let legacy = Content::classic_bundled().unwrap();
    let mut older = World::new(&legacy, CountryId("GBR".into()), 17).unwrap();
    let mut ordinary = older.clone();
    let action = Action::Borrow { amount: Money(100) };
    ordinary.issue(&legacy, action.clone()).unwrap();
    let receipt = older.issue_with_receipt(&legacy, action).unwrap();
    assert!(receipt.outcomes.is_empty());
    assert_eq!(older, ordinary);
    assert!(older.events.last().unwrap().text.contains("Borrowed"));
    assert_eq!(older, persistence::replay(&older, &legacy).unwrap());

    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let receipt = world
        .issue_with_receipt(&content, Action::Borrow { amount: Money(100) })
        .unwrap();
    assert!(!receipt.outcomes.is_empty());
    for id in &receipt.outcomes {
        let event = world
            .event_by_reference(&content, Viewer::Government(world.player.clone()), id)
            .unwrap()
            .unwrap();
        assert_eq!(event.context, Some(EventContext::General));
    }
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}
