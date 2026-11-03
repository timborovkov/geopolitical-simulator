use sim_core::*;

fn fixture() -> (Content, World) {
    let content = Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    (content, world)
}
fn edit(field: DeveloperField, value: i32) -> DeveloperEdit {
    DeveloperEdit { field, value }
}

#[test]
fn normal_commands_and_invalid_developer_edits_are_atomic_rejections() {
    let (content, mut world) = fixture();
    let original = world.hash();
    let action = Action::Developer {
        target: CountryId("BOR".into()),
        edit: edit(DeveloperField::Tax, 35),
    };
    assert!(world.issue(&content, action.clone()).is_err());
    assert!(
        world
            .execute(&content, world.command(world.player.clone(), action))
            .is_err()
    );
    assert!(
        world
            .issue_developer(
                &content,
                CountryId("missing".into()),
                edit(DeveloperField::Tax, 35)
            )
            .is_err()
    );
    for field in DeveloperField::ALL {
        assert_eq!(DeveloperField::from_name(field.name()).unwrap(), field);
        for value in [field.bounds().start() - 1, field.bounds().end() + 1] {
            assert!(
                world
                    .issue_developer(&content, world.player.clone(), edit(field, value))
                    .is_err()
            );
            assert_eq!(world.hash(), original);
        }
    }
    assert!(!world.is_developer_modified());
    assert_eq!(world.hash(), original);
}

#[test]
fn mixed_command_origins_save_resume_and_replay_without_leaking_private_state() {
    let (content, mut world) = fixture();
    world
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    world.advance(&content, 31).unwrap();
    assert!(world.journal.iter().any(|entry| entry.ai));
    let target = CountryId("BOR".into());
    let accounts = world.countries[&target].accounts.clone();
    for (field, value) in [(DeveloperField::Tax, 35), (DeveloperField::Approval, 77)] {
        world
            .issue_developer(&content, target.clone(), edit(field, value))
            .unwrap();
    }
    assert_eq!(world.countries[&target].accounts, accounts);
    assert!(
        world.countries[&target]
            .households
            .iter()
            .all(|h| h.approval == 77)
    );
    assert!(world.is_developer_modified());
    assert!(world.journal.iter().filter(|e| e.developer).all(|e| !e.ai));
    let view = world
        .project(&content, Viewer::Government(world.player.clone()))
        .unwrap();
    assert!(view.developer_modified);
    assert_eq!(view.own.unwrap().id, world.player);
    let public = serde_json::to_value(&view.reports[&target]).unwrap();
    for private in ["accounts", "projects", "households", "ai_plan"] {
        assert!(public.get(private).is_none());
    }
    let mut loaded =
        persistence::decode(&persistence::encode(&world, &content).unwrap(), &content).unwrap();
    world.advance(&content, 60).unwrap();
    loaded.advance(&content, 60).unwrap();
    assert_eq!(loaded.hash(), world.hash());
    assert_eq!(
        persistence::replay(&world, &content).unwrap().hash(),
        world.hash()
    );

    let mut malformed = world.clone();
    malformed
        .journal
        .iter_mut()
        .find(|entry| entry.developer)
        .unwrap()
        .developer = false;
    assert!(malformed.validate(&content).is_err());
    let mut malformed = world.clone();
    malformed
        .journal
        .iter_mut()
        .find(|entry| entry.developer)
        .unwrap()
        .ai = true;
    assert!(malformed.validate(&content).is_err());
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&world, &content).unwrap()).unwrap();
    legacy["schema"] = 2.into();
    assert!(persistence::decode(&serde_json::to_vec(&legacy).unwrap(), &content).is_err());
}

#[test]
fn successful_developer_campaign_never_receives_normal_mandate_certification() {
    let (mut content, _) = fixture();
    content.countries.truncate(1);
    let mut world = World::new(&content, content.countries[0].id.clone(), 17).unwrap();
    for (field, value) in [
        (DeveloperField::Tax, 35),
        (DeveloperField::Administration, 100),
        (DeveloperField::Services, 8),
        (DeveloperField::Welfare, 8),
        (DeveloperField::EnergyCapacity, 200),
        (DeveloperField::FoodCapacity, 200),
    ] {
        world
            .issue_developer(&content, world.player.clone(), edit(field, value))
            .unwrap();
    }
    world
        .advance(&content, content.review_day().unwrap() - world.day - 1)
        .unwrap();
    world
        .issue_developer(
            &content,
            world.player.clone(),
            edit(DeveloperField::Approval, 100),
        )
        .unwrap();
    world.advance(&content, 1).unwrap();
    let review = world
        .events
        .iter()
        .find(|e| e.text.starts_with("Five-year review:"))
        .unwrap();
    assert!(
        review
            .text
            .contains("targets met in a developer-modified campaign"),
        "{}",
        review.text
    );
    assert!(
        !world
            .events
            .iter()
            .any(|e| e.text.contains("mandate achieved"))
    );
    persistence::replay(&world, &content).unwrap();
}

#[test]
fn exhausted_ids_fail_before_payment_or_mutation() {
    let (content, mut world) = fixture();
    world.next_command = u64::MAX;
    let original = world.hash();
    assert!(
        world
            .issue_developer(
                &content,
                world.player.clone(),
                edit(DeveloperField::Tax, 35)
            )
            .is_err()
    );
    assert_eq!(world.hash(), original);
    world.next_command = 1;
    world.next_project = u64::MAX;
    let original = world.hash();
    assert!(
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1
                }
            )
            .is_err()
    );
    assert_eq!(world.hash(), original);
}

#[test]
fn format_two_conversion_preserves_exact_original_and_can_be_recovered() {
    let (content, mut world) = fixture();
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&world, &content).unwrap()).unwrap();
    legacy["schema"] = 2.into();
    let original = serde_json::to_vec_pretty(&legacy).unwrap();
    let dir = std::env::temp_dir().join(format!("geosim-v2-migration-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("campaign.json");
    std::fs::write(&path, &original).unwrap();
    assert_eq!(
        persistence::load(&path, &content).unwrap().hash(),
        world.hash()
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(
        persistence::recovery_candidates(&path, &content)[0].schema,
        Some(2)
    );
    world.advance(&content, 1).unwrap();
    persistence::save(&path, &world, &content).unwrap();
    let archive = dir.join("campaign.json.format-v2-1");
    assert_eq!(std::fs::read(&archive).unwrap(), original);
    assert_eq!(
        std::fs::read(path.with_extension("json.bak")).unwrap(),
        original
    );
    assert_eq!(
        persistence::recovery_candidates(&path, &content)[0].schema,
        Some(3)
    );
    let restored = persistence::restore(&path, &path.with_extension("json.bak"), &content).unwrap();
    assert_eq!(
        restored.hash(),
        persistence::decode(&original, &content).unwrap().hash()
    );
    assert_eq!(std::fs::read(&archive).unwrap(), original);
    assert_eq!(
        std::fs::read(path.with_extension("json.bak")).unwrap(),
        original
    );
    persistence::save(&path, &world, &content).unwrap();
    assert_eq!(std::fs::read(&archive).unwrap(), original);
    for unsupported in [1, 99] {
        legacy["schema"] = unsupported.into();
        assert!(persistence::decode(&serde_json::to_vec(&legacy).unwrap(), &content).is_err());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn autosave_names_are_confined_and_separate_content_and_scenarios() {
    let (_, mut world) = fixture();
    for id in ["../../outside", "世界/../../../", "", "A"].map(|id| id.repeat(80)) {
        world.player = CountryId(id);
        let path = persistence::autosave_path(&world);
        assert_eq!(path.parent().unwrap(), std::path::Path::new("saves"));
        assert_eq!(path.components().count(), 2);
        assert!(path.file_name().unwrap().len() < 120);
        let original = path;
        world.content_hash ^= 1;
        assert_ne!(original, persistence::autosave_path(&world));
        world.content_hash ^= 1;
        world.scenario = Scenario::FiscalSupplyCrisis;
        assert_ne!(original, persistence::autosave_path(&world));
        world.scenario = Scenario::Sandbox;
    }
}
