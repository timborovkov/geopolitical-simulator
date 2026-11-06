use sim_core::*;

#[test]
fn population_definition_units_preserve_all_published_content_and_opening_hashes() {
    let known: [(u64, u64); 23] = [
        (0xd705e31066ca7139, 0xf091646f6ad90870), // initial-2000-stable-elections.json
        (0x1313c347522a1843, 0xddf0ce62932e6af7), // initial-2000-bookmark-identities.json
        (0x2ece4c40d10ad679, 0xb74fd50f30b753b9), // initial-2000-project-events.json
        (0xcf3ddf9118d0c3c5, 0x0fc2d7e7ebdb9688), // initial-2000-event-references.json
        (0x0600d1bb1d644211, 0x343d72b6cdfb6c83), // initial-2000-national-reserves.json
        (0x9fb1977bcdc0603d, 0xc29f309743ff5b94), // initial-2000-party-preferences.json
        (0xc2e0834e28a83d71, 0xd5f9f1f64662dcd3), // initial-2000-technology-eligibility.json
        (0x0edde4ec57fa5f24, 0x915461409527fc51), // initial-2000-interest-accrual.json
        (0x1b51e7e02875ba21, 0x9f82b952372c133e), // initial-2000-cash-management.json
        (0x7b478d7ac573b0d6, 0x9119e39d0e04ff98), // initial-2000-debt-maturities.json
        (0xe12fae3b10bfc8e4, 0xd6f2b6fa9cc1384a), // initial-2000-fiscal-ai.json
        (0x498e3b183dbfc5b9, 0x04f66253df6a20fb), // initial-2000-income-bands.json
        (0xeb9bf1a5f22fe1ba, 0xae1f4297ad56f091), // initial-2000-fiscal-plans.json
        (0x35cba912aef530d2, 0xc458a46931b0fc9a), // initial-2000-undertakings.json
        (0x15d7fb419182f288, 0xd18da06871917ca8), // initial-2000-territorial.json
        (0xeec309b3141d6040, 0x2347dad481b5fdbc), // initial-2000-observed.json
        (0x5de71d91a48970b1, 0x1d508dc584b0f8d7), // initial-2000-elections.json
        (0x10f122fce2c1f875, 0xb8245403a26de3cd), // initial-2000-cabinet.json
        (0x7c8be167d291cb7e, 0x70e8365503655966), // initial-2000-commitments.json
        (0x2815dc054721e23e, 0x40bb72c0babdf470), // initial-2000-constitutions.json
        (0x51adda7efdd77a3b, 0x7e490f30b9b20db4), // initial-2000-finance.json
        (0x18cc3056d4775745, 0x0ae9ffc0960ece36), // initial-2000-markets.json
        (0x4ab24c451ec6e733, 0xabc4a3d35c40ec9f), // initial-2000.json
    ];
    for (content_hash, world_hash) in known {
        let content = Content::bundled_for_hash(content_hash).unwrap();
        assert_eq!(content.hash(), content_hash);
        let country = CountryId("GBR".into());
        let population: Population = content.country(&country).unwrap().population;
        let world = World::new(&content, country, 17).unwrap();
        assert_eq!(world.player_country().population, population);
        assert_eq!(world.hash(), world_hash);
        let bytes = persistence::encode(&world, &content).unwrap();
        assert_eq!(
            persistence::encode(&persistence::decode(&bytes, &content).unwrap(), &content).unwrap(),
            bytes
        );
    }
    let raw = include_str!("../../../data/synthetic-test.json");
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!(999),
        serde_json::json!(2_000_000_001_u64),
        serde_json::json!(1_000.5),
        serde_json::json!("1000000"),
    ] {
        let mut data: serde_json::Value = serde_json::from_str(raw).unwrap();
        data["countries"][0]["population"] = invalid;
        assert!(Content::from_json(&data.to_string()).is_err());
    }
}

#[test]
fn uneven_typed_census_initializes_exact_cohorts_and_retains_the_published_progression() {
    let mut content =
        Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
    let country = CountryId("AUR".into());
    content
        .countries
        .iter_mut()
        .find(|definition| definition.id == country)
        .unwrap()
        .population = Population(1_000_003);
    content.validate().unwrap();
    let mut world = World::new(&content, country.clone(), 17).unwrap();
    assert_eq!(
        world
            .player_country()
            .households
            .iter()
            .map(|group| u128::from(group.population.0))
            .sum::<u128>(),
        1_000_003
    );
    world.advance(&content, DaySpan::new(61)).unwrap();
    assert_eq!(
        world.player_country().population,
        content.country(&country).unwrap().population
    );
    assert_eq!(world.hash(), 0x1de9f0670f03a23a);
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
    let bytes = persistence::encode(&world, &content).unwrap();
    assert_eq!(world, persistence::decode(&bytes, &content).unwrap());
}
