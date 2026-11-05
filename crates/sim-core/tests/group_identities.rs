use sim_core::*;

fn check(world: &World, content: &Content) {
    let bytes = persistence::encode(world, content).unwrap();
    let restored = persistence::decode(&bytes, content).unwrap();
    assert_eq!(persistence::encode(&restored, content).unwrap(), bytes);
    assert_eq!(restored, persistence::replay(&restored, content).unwrap());
}

#[test]
fn typed_group_and_currency_references_keep_published_strings_and_campaign_bytes() {
    let content = Content::bundled().unwrap();
    assert_eq!(content.hash(), 0x2ece4c40d10ad679);
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(world.hash(), 0xb74fd50f30b753b9);
    assert_eq!(
        serde_json::to_string(&HouseholdGroupId::from("workers")).unwrap(),
        r#""workers""#
    );
    assert_eq!(
        serde_json::to_string(&CurrencyId::from("GBP")).unwrap(),
        r#""GBP""#
    );
    assert_eq!(
        format!("{:?}", HouseholdGroupId::from("workers")),
        r#""workers""#
    );
    assert_eq!(format!("{:?}", CurrencyId::from("GBP")), r#""GBP""#);
    assert_eq!(
        content.country(&world.player).unwrap().currency,
        CurrencyId::from("GBP")
    );
    let definition = serde_json::to_value(content.country(&world.player).unwrap()).unwrap();
    assert_eq!(definition["currency"], "GBP");
    world
        .issue(
            &content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30,
            },
        )
        .unwrap();
    world.advance(&content, 60).unwrap();
    assert_eq!(world.hash(), 0x8bf8784e005cff53);
    let state = serde_json::to_value(world.player_country()).unwrap();
    for group in &content.country(&world.player).unwrap().households {
        assert!(
            state["households"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == group.id.as_str())
        );
        assert!(
            state["income_tax_report"]["groups"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["group"] == group.id.as_str())
        );
    }
    check(&world, &content);
}

#[test]
fn renamed_and_reordered_group_definitions_still_resolve_tax_records_by_identity() {
    let mut content = Content::bundled().unwrap();
    for country in &mut content.countries {
        country.households.reverse();
        for group in &mut country.households {
            group.name = format!("Renamed cohort {}", group.id);
        }
    }
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    let workers = HouseholdGroupId::from("workers");
    let definition = content.country(&world.player).unwrap();
    assert_eq!(
        definition.household(&workers).unwrap().name,
        "Renamed cohort workers"
    );
    assert_eq!(
        world.player_country().household(&workers).unwrap().id,
        workers
    );
    assert!(
        definition
            .household(&HouseholdGroupId::from("Renamed cohort workers"))
            .is_none()
    );
    let before = world.clone();
    assert!(
        world
            .player_country()
            .household(&HouseholdGroupId::from("unknown"))
            .is_none()
    );
    let forecast = world
        .preview_income_taxes(&content, &world.player, 30)
        .unwrap();
    for (group, _) in &forecast {
        assert!(definition.household(group).is_some());
        assert!(world.player_country().household(group).is_some());
    }
    assert_eq!(world, before);
    world.advance(&content, 1).unwrap();
    for line in &world
        .player_country()
        .income_tax_report
        .as_ref()
        .unwrap()
        .groups
    {
        assert!(
            definition
                .household(&line.group)
                .unwrap()
                .name
                .starts_with("Renamed cohort ")
        );
        assert_eq!(
            world.player_country().household(&line.group).unwrap().tax,
            line.paid
        );
    }
    let preview = world
        .preview(&content, &world.player, &PolicyId::from("income_tax"), 30)
        .unwrap();
    assert!(
        preview
            .household_impacts
            .iter()
            .all(|impact| impact.group.starts_with("Renamed cohort "))
    );
    check(&world, &content);
}

#[test]
fn dated_currency_identities_roundtrip_without_converting_reporting_money() {
    let original = include_str!("../../../data/synthetic-dated-test.json");
    let early = Content::from_json(original).unwrap();
    let mut later_json: serde_json::Value = serde_json::from_str(original).unwrap();
    later_json["start"] = "2001-07-16".into();
    let later = Content::from_json(&serde_json::to_string(&later_json).unwrap()).unwrap();
    let country = CountryId("AUR".into());
    assert_eq!(
        early.country(&country).unwrap().currency,
        CurrencyId::from("AUT")
    );
    assert_eq!(
        later.country(&country).unwrap().currency,
        CurrencyId::from("NEW-AUT")
    );
    let first = World::new(&early, country.clone(), 17).unwrap();
    let second = World::new(&later, country, 17).unwrap();
    assert_eq!(
        first.player_country().accounts,
        second.player_country().accounts
    );
    assert_eq!(first.player_country().debt, second.player_country().debt);
    let catalogue = later.identities.as_ref().unwrap();
    let term = catalogue
        .currencies
        .iter()
        .find(|term| term.code == "NEW-AUT")
        .unwrap();
    let json = serde_json::to_value(term).unwrap();
    assert_eq!(json["code"], "NEW-AUT");
    assert_eq!(serde_json::from_value::<CurrencyTerm>(json).unwrap(), *term);
    check(&first, &early);
    check(&second, &later);
}
