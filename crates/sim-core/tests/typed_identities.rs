use sim_core::*;

#[test]
fn typed_policy_and_party_references_keep_published_json_and_campaign_bytes() {
    let content = Content::event_reference_bundled().unwrap();
    assert_eq!(content.hash(), 0xcf3ddf9118d0c3c5);
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(world.hash(), 0x0fc2d7e7ebdb9688);
    let prior = Content::reserve_bundled().unwrap();
    assert_eq!(
        World::new(&prior, CountryId("GBR".into()), 17)
            .unwrap()
            .hash(),
        0x343d72b6cdfb6c83
    );
    assert_eq!(
        serde_json::to_string(&PolicyId::from("income_tax")).unwrap(),
        r#""income_tax""#
    );
    assert_eq!(
        serde_json::to_string(&PartyId::from("GBR:labour")).unwrap(),
        r#""GBR:labour""#
    );
    let action: Action =
        serde_json::from_str(r#"{"Enact":{"policy":"income_tax","value":30}}"#).unwrap();
    assert_eq!(
        serde_json::to_string(&action).unwrap(),
        r#"{"Enact":{"policy":"income_tax","value":30}}"#
    );
    world.issue(&content, action).unwrap();
    world.advance(&content, sim_core::DaySpan::new(60)).unwrap();
    assert_eq!(world.hash(), 0x4d601b2b61093b11);
    let encoded = persistence::encode(&world, &content).unwrap();
    let restored = persistence::decode(&encoded, &content).unwrap();
    assert_eq!(persistence::encode(&restored, &content).unwrap(), encoded);
    assert_eq!(restored, persistence::replay(&restored, &content).unwrap());
    let json = serde_json::to_value(&restored).unwrap();
    let seats = &json["countries"]["GBR"]["seats"];
    for party in &content.countries[0].parties {
        assert!(seats.get(party.id.as_str()).is_some());
    }
}

#[test]
fn renaming_and_reordering_rosters_preserve_identity_and_names_cannot_issue_commands() {
    let mut content = Content::event_reference_bundled().unwrap();
    let country = CountryId("GBR".into());
    let policy = PolicyId::from("income_tax");
    let party = content.country(&country).unwrap().ruling_party.clone();
    let foreign_party = content.countries[1].ruling_party.clone();
    content
        .policies
        .iter_mut()
        .find(|p| p.id == policy)
        .unwrap()
        .name = "Renamed fiscal proposal".into();
    let definition = content
        .countries
        .iter_mut()
        .find(|d| d.id == country)
        .unwrap();
    definition
        .parties
        .iter_mut()
        .find(|p| p.id == party)
        .unwrap()
        .name = "Renamed governing party".into();
    definition.parties.reverse();
    content.policies.reverse();
    content.validate().unwrap();
    assert_eq!(
        content.policy(&policy).unwrap().name,
        "Renamed fiscal proposal"
    );
    let definition = content.country(&country).unwrap();
    assert_eq!(
        definition.party(&party).unwrap().name,
        "Renamed governing party"
    );
    assert!(definition.party(&foreign_party).is_none());
    assert!(
        definition
            .party(&PartyId::from("Renamed governing party"))
            .is_none()
    );
    let mut world = World::new(&content, country, 17).unwrap();
    let opening = world.clone();
    let error = world
        .issue(
            &content,
            Action::Enact {
                policy: "Renamed fiscal proposal".into(),
                value: 30,
            },
        )
        .unwrap_err();
    assert!(error.contains("Unknown policy"));
    assert_eq!(world, opening);
    let delay = world
        .preview(&content, &world.player, &policy, 30)
        .unwrap()
        .delay
        .units();
    world
        .issue(
            &content,
            Action::Enact {
                policy: policy.clone(),
                value: 30,
            },
        )
        .unwrap();
    assert_eq!(world.player_country().projects[0].policy, policy);
    assert_eq!(world.player_country().ruling_party, party);
    world
        .advance(&content, sim_core::DaySpan::new(delay))
        .unwrap();
    assert_eq!(world.player_country().tax, 30);
    world.validate(&content).unwrap();
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}
