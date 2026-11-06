use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-stable-elections-test.json"
    ))
    .unwrap()
}
fn check(world: &World, content: &Content) {
    world.validate(content).unwrap();
    let bytes = persistence::encode(world, content).unwrap();
    let restored = persistence::decode(&bytes, content).unwrap();
    assert_eq!(persistence::encode(&restored, content).unwrap(), bytes);
    assert_eq!(restored, persistence::replay(&restored, content).unwrap());
}

#[test]
fn all_party_row_permutations_produce_the_same_remainder_seats_and_successor() {
    let content = fixture();
    let id = CountryId("AUR".into());
    let parties = content.country(&id).unwrap().parties.clone();
    let mut expected = None;
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut pack = content.clone();
        pack.countries
            .iter_mut()
            .find(|country| country.id == id)
            .unwrap()
            .parties = order
            .into_iter()
            .map(|index| parties[index].clone())
            .collect();
        pack.validate().unwrap();
        let mut world =
            World::new_scenario(&pack, id.clone(), 17, Scenario::FiscalSupplyCrisis).unwrap();
        world.advance(&pack, sim_core::DaySpan::new(361)).unwrap();
        let country = world.player_country();
        assert_eq!(country.government_number, sim_core::GovernmentId(2));
        assert_eq!(country.ruling_party, PartyId::from("AUR-2"));
        assert_eq!(
            country.seats,
            [
                ("AUR-1".into(), 15),
                ("AUR-2".into(), 43),
                ("AUR-3".into(), 42)
            ]
            .into()
        );
        assert_eq!(
            country.coalition,
            vec![PartyId::from("AUR-2"), PartyId::from("AUR-3")]
        );
        check(&world, &pack);
        // Pack identity includes its original row order; authoritative results do not.
        world.content_hash = content.hash();
        if let Some(expected) = &expected {
            assert_eq!(&world, expected);
        } else {
            expected = Some(world);
        }
    }
}

#[test]
fn an_equal_seat_poll_uses_the_lowest_id_and_keeps_developer_control_replayable() {
    let content = fixture();
    let id = CountryId("AUR".into());
    for reverse in [false, true] {
        let mut pack = content.clone();
        if reverse {
            pack.countries
                .iter_mut()
                .find(|country| country.id == id)
                .unwrap()
                .parties
                .reverse();
        }
        pack.validate().unwrap();
        let mut world =
            World::new_scenario(&pack, id.clone(), 17, Scenario::FiscalSupplyCrisis).unwrap();
        world.advance(&pack, sim_core::DaySpan::new(360)).unwrap();
        assert_eq!(world.player_country().next_election, Some(361));
        world
            .issue_developer(
                &pack,
                id.clone(),
                DeveloperEdit {
                    field: DeveloperField::Approval,
                    value: 19,
                },
            )
            .unwrap();
        world.advance(&pack, sim_core::DaySpan::new(1)).unwrap();
        assert!(world.is_developer_modified());
        assert_eq!(
            world.player_country().seats,
            [
                ("AUR-1".into(), 16),
                ("AUR-2".into(), 42),
                ("AUR-3".into(), 42)
            ]
            .into()
        );
        assert_eq!(world.player_country().ruling_party, PartyId::from("AUR-2"));
        check(&world, &pack);
    }
}

fn legacy_fixture(mut content: Content) -> Content {
    let id = CountryId("AUR".into());
    let country = content
        .countries
        .iter_mut()
        .find(|country| country.id == id)
        .unwrap();
    country.parties[0].seats = 60;
    country.parties[1].seats = 20;
    let mut third = country.parties[1].clone();
    third.id = "AUR-3".into();
    third.name = "Aurora alternative three".into();
    third.leader = "Fictional delegate three".into();
    country.parties.push(third);
    if let Some(preferences) = &mut content.party_preferences {
        let mut profile = preferences
            .profiles
            .iter()
            .find(|profile| profile.country == id && profile.party == "AUR-2")
            .unwrap()
            .clone();
        profile.party = "AUR-3".into();
        for position in &mut profile.positions {
            position.preferred = None;
        }
        preferences.profiles.push(profile);
    }
    content.validate().unwrap();
    content
}

#[test]
fn classic_and_rules23_counterexamples_keep_their_pre_change_audit_hashes() {
    for (source, day, hashes) in [
        (
            include_str!("../../../data/synthetic-test.json"),
            730,
            [0xdb87ff0b6422b70d, 0x0325a5f68f95d3cf],
        ),
        (
            include_str!("../../../data/synthetic-project-events-test.json"),
            361,
            [0x382b3463deb50a16, 0xe7615bdc6c719872],
        ),
    ] {
        let original = legacy_fixture(Content::from_json(source).unwrap());
        for (reverse, expected) in [false, true].into_iter().zip(hashes) {
            let mut content = original.clone();
            if reverse {
                content
                    .countries
                    .iter_mut()
                    .find(|country| country.id.0 == "AUR")
                    .unwrap()
                    .parties
                    .reverse();
            }
            content.validate().unwrap();
            let mut world = World::new_scenario(
                &content,
                CountryId("AUR".into()),
                17,
                Scenario::FiscalSupplyCrisis,
            )
            .unwrap();
            world
                .advance(&content, sim_core::DaySpan::new(day))
                .unwrap();
            assert_eq!(world.hash(), expected);
            assert_eq!(
                world.player_country().ruling_party,
                PartyId::from(if reverse { "AUR-3" } else { "AUR-2" })
            );
            check(&world, &content);
        }
    }
}
