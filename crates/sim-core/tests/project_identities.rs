use sim_core::*;

fn propose(world: &mut World, content: &Content) -> CommitmentId {
    world
        .issue(
            content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30,
            },
        )
        .unwrap();
    world.player_country().projects[0].id
}

fn roundtrip(world: &World, content: &Content) {
    let encoded = persistence::encode(world, content).unwrap();
    let restored = persistence::decode(&encoded, content).unwrap();
    assert_eq!(persistence::encode(&restored, content).unwrap(), encoded);
    assert_eq!(restored, persistence::replay(&restored, content).unwrap());
}

#[test]
fn project_identity_keeps_numeric_json_and_published_campaign_hash() {
    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let id = propose(&mut world, &content);
    assert_eq!(id, CommitmentId(1));
    assert_eq!(serde_json::to_string(&id).unwrap(), "1");
    assert_eq!(id.to_string(), "1");
    let json = serde_json::to_value(world.player_country()).unwrap();
    assert_eq!(json["projects"][0]["id"], 1);
    assert_eq!(json["commitments"][0]["id"], 1);
    assert_eq!(json["commitments"][0]["project"]["id"], 1);
    world.advance(&content, 60).unwrap();
    assert_eq!(world.hash(), 0x4d601b2b61093b11);
    assert_eq!(world.player_country().project(&id).unwrap().value, 30);
    roundtrip(&world, &content);
}

#[test]
fn scoped_project_lookup_survives_amendment_and_leaves_history_after_withdrawal() {
    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let id = propose(&mut world, &content);
    let before = world.clone();
    assert!(
        world
            .player_country()
            .project(&CommitmentId(u64::MAX))
            .is_none()
    );
    let foreign = world
        .countries
        .iter()
        .find(|(country, _)| **country != world.player)
        .unwrap()
        .1;
    assert!(foreign.project(&id).is_none());
    assert_eq!(world, before);
    world
        .issue(
            &content,
            Action::AmendCommitment {
                request: id,
                value: 31,
            },
        )
        .unwrap();
    assert_eq!(world.player_country().project(&id).unwrap().value, 31);
    let record = &world.player_country().commitments.as_ref().unwrap()[0];
    assert_eq!(record.id, id);
    assert_eq!(
        record.project,
        *world.player_country().project(&id).unwrap()
    );
    roundtrip(&world, &content);
    world
        .issue(&content, Action::CancelCommitment { request: id })
        .unwrap();
    assert!(world.player_country().project(&id).is_none());
    assert_eq!(
        world.player_country().commitments.as_ref().unwrap()[0].id,
        id
    );
    roundtrip(&world, &content);
    let next = propose(&mut world, &content);
    assert_ne!(next, id);
    assert!(world.player_country().project(&id).is_none());
    roundtrip(&world, &content);
}

#[test]
fn classic_pending_project_roundtrips_without_lifecycle_records() {
    let content = Content::classic_bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let id = propose(&mut world, &content);
    let project = world.player_country().project(&id).unwrap().clone();
    assert!(world.player_country().commitments.is_none());
    assert_eq!(serde_json::to_value(&project).unwrap()["id"], id.0);
    roundtrip(&world, &content);
    world.advance(&content, project.due_day).unwrap();
    assert!(world.player_country().project(&id).is_none());
    assert_eq!(world.player_country().tax, 30);
    roundtrip(&world, &content);
}
