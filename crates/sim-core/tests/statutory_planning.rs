use sim_core::*;

fn fixture() -> (Content, World) {
    let content = Content::from_json(include_str!(
        "../../../data/synthetic-legislation-test.json"
    ))
    .unwrap();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    (content, world)
}
fn plan(content: &Content, world: &World, value: i32) -> Vec<PolicyLawPlan> {
    let before = world.clone();
    let result = world
        .policy_law_plan(
            content,
            Viewer::Government(world.player.clone()),
            &"services".into(),
            value,
        )
        .unwrap();
    assert_eq!(&before, world);
    result
}

#[test]
fn paid_bill_progress_is_read_only_and_permission_still_precedes_separate_programme_funding() {
    let (content, mut world) = fixture();
    let law = LawId("high_service_authority".into());
    assert!(plan(&content, &world, 13).is_empty());
    let opening = plan(&content, &world, 14);
    let LawPermissionProgress::Proposable(preview) = &opening[0].progress else {
        panic!("available paid proposal")
    };
    assert_eq!(
        (preview.total_cost, preview.upfront, preview.reserved),
        (Money(20), Money(5), Money(15))
    );
    world.issue(&content, Action::ProposeLaw { law }).unwrap();
    assert!(matches!(
        plan(&content, &world, 14)[0].progress,
        LawPermissionProgress::Pending {
            bill: LawBillId(1),
            status: LawBillStatus::AwaitingConsent,
            paid: Money(5),
            reserved: Money(15),
            ..
        }
    ));
    assert!(
        world
            .preview(&content, &world.player, &"services".into(), 14)
            .is_err()
    );
    world.advance(&content, DaySpan::new(14)).unwrap();
    assert!(matches!(
        plan(&content, &world, 14)[0].progress,
        LawPermissionProgress::Pending {
            status: LawBillStatus::Implementing,
            ..
        }
    ));
    world.advance(&content, DaySpan::new(7)).unwrap();
    assert!(
        matches!(plan(&content, &world, 14)[0].progress, LawPermissionProgress::Enacted { bill: LawBillId(1), day } if day == CampaignDay::new(21))
    );
    assert_eq!(world.player_country().services, 9);
    assert!(world.player_country().projects.is_empty());
    assert!(
        world
            .preview(&content, &world.player, &"services".into(), 14)
            .is_ok()
    );
    let bytes = persistence::encode(&world, &content).unwrap();
    let restored = persistence::decode(&bytes, &content).unwrap();
    assert_eq!(plan(&content, &restored, 14), plan(&content, &world, 14));
    assert_eq!(persistence::replay(&restored, &content).unwrap(), restored);
    let mut withdrawn = World::new(&content, world.player.clone(), 17).unwrap();
    withdrawn
        .issue(
            &content,
            Action::ProposeLaw {
                law: LawId("high_service_authority".into()),
            },
        )
        .unwrap();
    withdrawn
        .issue(
            &content,
            Action::WithdrawLaw {
                request: LawBillId(1),
            },
        )
        .unwrap();
    assert!(
        matches!(&plan(&content, &withdrawn, 14)[0].progress, LawPermissionProgress::Blocked(reason) if reason.contains("cooldown"))
    );
    assert_eq!(withdrawn.player_country().committed_cash, Money(0));
}

#[test]
fn ordinary_proposal_failures_remain_distinct_from_pending_and_enacted_permission() {
    let (original, _) = fixture();
    for variant in ["funding", "future", "country", "backing"] {
        let mut value = serde_json::to_value(&original).unwrap();
        value["legislation"]["ai_targets"] = serde_json::json!({});
        match variant {
            "funding" => value["legislation"]["cost_bps"] = 1000.into(),
            "future" => {
                value["legislation"]["definitions"][0]["available_from"] = "2000-02-01".into()
            }
            "country" => {
                value["legislation"]["definitions"][0]["eligible_countries"] =
                    serde_json::json!(["BOR"])
            }
            "backing" => value["countries"][0]["government"]["threshold"] = 99.into(),
            _ => unreachable!(),
        }
        let content = Content::from_json(&value.to_string()).unwrap();
        let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let reason = world
            .preview_law(
                &content,
                &world.player,
                &LawId("high_service_authority".into()),
            )
            .unwrap_err();
        assert_eq!(
            plan(&content, &world, 14)[0].progress,
            LawPermissionProgress::Blocked(reason),
            "{variant}"
        );
    }
}

#[test]
fn multiple_required_laws_follow_ids_and_thresholds_after_definition_reordering_and_renaming() {
    let (original, _) = fixture();
    let mut value = serde_json::to_value(&original).unwrap();
    let mut second = value["legislation"]["definitions"][0].clone();
    second["id"] = "staffing_authority".into();
    second["name"] = "Illustrative staffing authority".into();
    second["available_from"] = "2000-02-01".into();
    value["legislation"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(second);
    value["legislation"]["policy_requirements"][0]["laws"]
        .as_array_mut()
        .unwrap()
        .push("staffing_authority".into());
    let content = Content::from_json(&value.to_string()).unwrap();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let expected = plan(&content, &world, 14);
    assert_eq!(expected.len(), 2);
    assert!(matches!(
        expected[0].progress,
        LawPermissionProgress::Proposable(_)
    ));
    assert!(matches!(
        expected[1].progress,
        LawPermissionProgress::Blocked(_)
    ));
    value["legislation"]["definitions"]
        .as_array_mut()
        .unwrap()
        .reverse();
    value["legislation"]["definitions"][0]["name"] = "Renamed staffing reference".into();
    value["legislation"]["policy_requirements"][0]["laws"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let reordered = Content::from_json(&value.to_string()).unwrap();
    let world = World::new(&reordered, world.player.clone(), 17).unwrap();
    let actual = plan(&reordered, &world, 14);
    for previous in expected {
        let current = actual.iter().find(|row| row.law == previous.law).unwrap();
        assert_eq!(current.progress, previous.progress);
    }
    assert_eq!(actual[0].name, "Renamed staffing reference");
    assert!(plan(&reordered, &world, 13).is_empty());
}

#[test]
fn planning_rejects_foreign_viewers_mismatched_content_and_invalid_targets_and_preserves_earlier_packs()
 {
    let (content, world) = fixture();
    let before = world.clone();
    for viewer in [
        Viewer::Observer,
        Viewer::Government(CountryId("BOR".into())),
    ] {
        assert!(
            world
                .policy_law_plan(&content, viewer, &"services".into(), 14)
                .unwrap_err()
                .contains("private")
        );
    }
    let viewer = Viewer::Government(world.player.clone());
    for (policy, value) in [("missing", 14), ("services", 100)] {
        assert!(
            world
                .policy_law_plan(&content, viewer.clone(), &policy.into(), value)
                .is_err()
        );
    }
    let old = Content::statistical_history_bundled().unwrap();
    assert!(
        world
            .policy_law_plan(&old, viewer, &"services".into(), 14)
            .is_err()
    );
    assert_eq!(world, before);
    let older_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert!(plan(&old, &older_world, 14).is_empty());
    assert_eq!(persistence::snapshot_schema(&older_world), 26);
}
