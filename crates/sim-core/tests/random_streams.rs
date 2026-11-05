use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap()
}

#[test]
fn supply_history_is_independent_of_other_countries_and_definition_order() {
    let full = fixture();
    let mut reversed = full.clone();
    reversed.countries.reverse();
    reversed.validate().unwrap();
    let aur = CountryId("AUR".into());
    let mut isolated = full.clone();
    isolated.countries.retain(|country| country.id == aur);
    isolated.validate().unwrap();
    for seed in [0, 17, 42] {
        let contents = [&full, &reversed, &isolated];
        let mut worlds = contents.map(|content| World::new(content, aur.clone(), seed).unwrap());
        let mut observed = Vec::new();
        for day in 1..=200 {
            for (world, content) in worlds.iter_mut().zip(contents) {
                world.advance(content, 1).unwrap();
            }
            let shock = worlds[0].player_country().report.shock;
            assert_eq!(
                worlds[1].player_country().report.shock,
                shock,
                "seed {seed}, day {day}"
            );
            assert_eq!(
                worlds[2].player_country().report.shock,
                shock,
                "seed {seed}, day {day}"
            );
            observed.push(shock);
        }
        let expected = match seed {
            0 => [0, 0, 0],
            17 => [15, 0, 25],
            42 => [0, 10, 11],
            _ => unreachable!(),
        };
        assert_eq!([observed[0], observed[90], observed[180]], expected);
        assert!(observed[20..90].iter().all(|shock| *shock == 0));
        assert!(observed[110..180].iter().all(|shock| *shock == 0));
    }
}

#[test]
fn quarter_boundary_samples_survive_snapshot_replay_and_chunked_advancement() {
    let content = Content::bundled().unwrap();
    let mut continuous = World::new(&content, CountryId("GBR".into()), 0).unwrap();
    continuous.advance(&content, 111).unwrap();
    let mut resumed = World::new(&content, CountryId("GBR".into()), 0).unwrap();
    for target in [89, 90, 91, 110, 111] {
        resumed.advance(&content, target - resumed.day).unwrap();
        let bytes = persistence::encode(&resumed, &content).unwrap();
        resumed = persistence::decode(&bytes, &content).unwrap();
        assert_eq!(persistence::encode(&resumed, &content).unwrap(), bytes);
        assert_eq!(resumed, persistence::replay(&resumed, &content).unwrap());
        match target {
            89 | 90 | 111 => assert_eq!(resumed.player_country().report.shock, 0),
            91 | 110 => assert_eq!(resumed.player_country().report.shock, 11),
            _ => unreachable!(),
        }
    }
    assert_eq!(resumed.hash(), continuous.hash());
    assert_eq!(resumed, continuous);
}
