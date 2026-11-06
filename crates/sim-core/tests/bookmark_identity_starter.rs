use sim_core::*;

#[test]
fn opening_crosswalk_preserves_all_prior_definitions_and_opening_world_resources() {
    let pack = Content::bookmark_identities_bundled().unwrap();
    let prior = Content::stable_elections_bundled().unwrap();
    assert_eq!(pack.rules, prior.rules);
    let mut normalized = pack.clone();
    normalized.id = prior.id.clone();
    normalized.name = prior.name.clone();
    normalized.description = prior.description.clone();
    normalized.sources = prior.sources.clone();
    normalized.identities = None;
    for country in &mut normalized.countries {
        country.governing_office = None;
    }
    assert_eq!(normalized, prior);
    assert_eq!(Content::bundled_for_hash(pack.hash()).unwrap(), pack);
    for definition in &pack.countries {
        let mut world = World::new(&pack, definition.id.clone(), 17).unwrap();
        let expected = World::new(&prior, definition.id.clone(), 17).unwrap();
        assert_eq!(persistence::snapshot_schema(&world), 25);
        world.content_hash = expected.content_hash;
        assert_eq!(world, expected);
    }
    let identities = pack.identities.as_ref().unwrap();
    assert_eq!(
        (
            identities.people.len(),
            identities.offices.len(),
            identities.terms.len(),
            identities.currencies.len()
        ),
        (37, 49, 49, 15)
    );
    assert!(
        identities
            .people
            .iter()
            .all(|person| person.quality == RecordQuality::Authored)
    );
}

#[test]
fn opening_records_preserve_distinct_executives_collective_missingness_and_exclusive_dates() {
    let pack = Content::bookmark_identities_bundled().unwrap();
    let identities = pack.identities.as_ref().unwrap();
    assert!(pack.at_bookmark("2000-01-02").is_err());
    let term = |office: &str| {
        identities
            .terms
            .iter()
            .find(|term| term.office.0 == office)
            .unwrap()
    };
    assert_eq!(
        term("GBR:governing-office").holder,
        term("GBR:labour:leadership").holder
    );
    assert_ne!(
        term("FRA:governing-office").holder,
        term("FRA:ps:leadership").holder
    );
    assert!(
        matches!(&term("DEU:greens:leadership").holder,OfficeHolder::Collective{name,members} if name=="Collective leadership"&&members.is_empty())
    );
    assert!(
        !identities
            .people
            .iter()
            .any(|person| person.name == "Collective leadership")
    );
    assert!(
        identities
            .terms
            .iter()
            .all(|term| term.validity.contains("2000-01-01".parse().unwrap())
                && !term.validity.contains("2000-01-02".parse().unwrap()))
    );
    for country in &pack.countries {
        let offices = pack.bookmark_offices(&country.id);
        let governing = offices
            .iter()
            .find(|office| Some(&office.id) == country.governing_office.as_ref())
            .unwrap();
        assert_eq!(governing.holder, country.executive);
    }
    // Definition order and renamed labels do not change reference identities.
    let mut renamed = pack.clone();
    let catalogue = renamed.identities.as_mut().unwrap();
    catalogue.people.reverse();
    catalogue.offices.reverse();
    catalogue.terms.reverse();
    let person = catalogue
        .people
        .iter_mut()
        .find(|person| person.id.0 == "GBR:labour:opening-person")
        .unwrap();
    person.name = "Revised display label".into();
    let renamed = Content::from_json(&serde_json::to_string(&renamed).unwrap()).unwrap();
    assert_eq!(
        renamed.country(&CountryId("GBR".into())).unwrap().executive,
        "Revised display label"
    );
    assert_eq!(
        renamed
            .identities
            .as_ref()
            .unwrap()
            .terms
            .iter()
            .find(|term| term.office.0 == "GBR:governing-office")
            .unwrap()
            .holder,
        term("GBR:governing-office").holder
    );
}

#[test]
fn reference_metadata_never_schedules_a_handover_and_campaign_saves_replay() {
    let pack = Content::bookmark_identities_bundled().unwrap();
    let prior = Content::stable_elections_bundled().unwrap();
    let country = CountryId("GBR".into());
    let mut world = World::new(&pack, country.clone(), 17).unwrap();
    let mut expected = World::new(&prior, country, 17).unwrap();
    let action = Action::Enact {
        policy: "income_tax".into(),
        value: 30,
    };
    world.issue(&pack, action.clone()).unwrap();
    expected.issue(&prior, action).unwrap();
    world.advance(&pack, DaySpan::new(108)).unwrap();
    expected.advance(&prior, DaySpan::new(108)).unwrap();
    assert_eq!(world, persistence::replay(&world, &pack).unwrap());
    let bytes = persistence::encode(&world, &pack).unwrap();
    assert_eq!(world, persistence::decode(&bytes, &pack).unwrap());
    let mut normalized = world;
    normalized.content_hash = expected.content_hash;
    assert_eq!(normalized, expected);
}
