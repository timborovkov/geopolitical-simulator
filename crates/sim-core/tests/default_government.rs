use sim_core::*;

#[test]
fn integrated_default_follows_paid_permission_funded_delivery_and_observed_review() {
    let content = Content::bundled().unwrap();
    assert_eq!(content.rules, RULES_VERSION);
    assert_eq!(content.rules, 28);
    assert_eq!(
        content.hash(),
        Content::governing_institutions_bundled().unwrap().hash()
    );
    for definition in &content.countries {
        let world = World::new(&content, definition.id.clone(), 17).unwrap();
        let country = world.player_country();
        assert!(country.executive_identity.is_some());
        assert!(country.legislation.is_some());
        assert!(country.statistical_archive.is_some());
        assert!(
            world
                .governing_institutions_view(
                    &content,
                    Viewer::Government(world.player.clone()),
                    &world.player
                )
                .is_ok()
        );
        assert_eq!(persistence::snapshot_schema(&world), 29);
    }
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let requirement = content.legislation.as_ref().unwrap().policy_requirements[0].clone();
    let law = requirement.laws[0].clone();
    let initial_services = world.player_country().services;
    let before = world.clone();
    assert!(
        world
            .preview(
                &content,
                &world.player,
                &requirement.policy,
                requirement.minimum_value
            )
            .unwrap_err()
            .contains("must be enacted")
    );
    assert_eq!(world, before);
    let preview = world.preview_law(&content, &world.player, &law).unwrap();
    let drafting_cash = world.player_country().accounts.treasury;
    let bill_command = world.next_command;
    world
        .issue(&content, Action::ProposeLaw { law: law.clone() })
        .unwrap();
    assert_eq!(
        world.player_country().accounts.treasury,
        Money(drafting_cash.0 - preview.upfront.0)
    );
    assert_eq!(world.player_country().committed_cash, preview.reserved);
    assert!(
        world
            .journal_entry(&bill_command)
            .unwrap()
            .governing_context
            .is_some()
    );
    world
        .advance(
            &content,
            preview.enactment_day.elapsed_since(world.day).unwrap(),
        )
        .unwrap();
    assert!(
        world
            .player_country()
            .legislation
            .as_ref()
            .unwrap()
            .enacted
            .contains_key(&law)
    );
    assert_eq!(world.player_country().services, initial_services);
    let before_programme = world.player_country().accounts.treasury;
    let programme = world
        .preview(
            &content,
            &world.player,
            &requirement.policy,
            requirement.minimum_value,
        )
        .unwrap();
    let proposal_command = world.next_command;
    world
        .issue(
            &content,
            Action::Enact {
                policy: requirement.policy.clone(),
                value: requirement.minimum_value,
            },
        )
        .unwrap();
    assert_eq!(
        world.player_country().accounts.treasury,
        Money(before_programme.0 - programme.upfront_payment.0)
    );
    assert_eq!(world.player_country().services, initial_services);
    assert!(
        world
            .journal_entry(&proposal_command)
            .unwrap()
            .governing_context
            .is_some()
    );
    let due = world
        .player_country()
        .projects
        .iter()
        .find(|p| p.policy == requirement.policy)
        .unwrap()
        .due_day;
    world
        .advance(&content, due.elapsed_since(world.day).unwrap())
        .unwrap();
    assert_eq!(world.player_country().services, requirement.minimum_value);
    let review = world
        .player_country()
        .commitments
        .as_ref()
        .unwrap()
        .iter()
        .find(|record| record.proposal_command == proposal_command)
        .unwrap()
        .review_due
        .unwrap();
    world
        .advance(&content, review.elapsed_since(world.day).unwrap())
        .unwrap();
    assert!(
        world
            .player_country()
            .commitments
            .as_ref()
            .unwrap()
            .iter()
            .find(|record| record.proposal_command == proposal_command)
            .unwrap()
            .evaluation
            .is_some()
    );
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    assert_eq!(
        persistence::decode(&persistence::encode(&world, &content).unwrap(), &content).unwrap(),
        world
    );
}

#[test]
fn integrated_default_loads_and_continues_the_preserved_pending_default_without_migration() {
    let current = Content::bundled().unwrap();
    let preserved = Content::statistical_history_bundled().unwrap();
    assert_eq!(
        Content::bundled_for_hash(preserved.hash()).unwrap(),
        preserved
    );
    let mut world = World::new(&preserved, CountryId("GBR".into()), 17).unwrap();
    world
        .issue(
            &preserved,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30,
            },
        )
        .unwrap();
    world.advance(&preserved, DaySpan::new(7)).unwrap();
    let directory =
        std::env::temp_dir().join(format!("geosim-integrated-default-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("pending-v26.json");
    persistence::save(&path, &world, &preserved).unwrap();
    let original = std::fs::read(&path).unwrap();
    let (resolved, mut restored) = persistence::load_bundled(&path).unwrap();
    assert_eq!(resolved, preserved);
    assert_eq!(restored, world);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(persistence::decode(&original, &current).is_err());
    assert_eq!(persistence::snapshot_schema(&restored), 26);
    assert!(restored.player_country().executive_identity.is_none());
    assert!(restored.player_country().legislation.is_none());
    assert!(
        restored
            .journal
            .iter()
            .all(|entry| entry.governing_context.is_none())
    );
    persistence::save(&path, &restored, &resolved).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let due = restored.player_country().projects[0].due_day;
    restored
        .advance(&resolved, due.elapsed_since(restored.day).unwrap())
        .unwrap();
    assert_eq!(restored.player_country().tax, 30);
    restored.validate(&resolved).unwrap();
    assert_eq!(persistence::replay(&restored, &resolved).unwrap(), restored);
    std::fs::remove_dir_all(directory).unwrap();
}
