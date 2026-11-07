use sim_core::*;

fn raw() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../data/synthetic-dated-test.json")).unwrap()
}
fn content() -> Content {
    Content::from_json(&raw().to_string()).unwrap()
}
fn aurora(content: &Content) -> &CountryDefinition {
    content.country(&CountryId("AUR".into())).unwrap()
}

#[test]
fn half_open_terms_assemble_people_offices_and_currency_at_each_bookmark() {
    let content = content();
    assert_eq!(aurora(&content).executive, "Fictional Steward B");
    assert_eq!(aurora(&content).office, "First Minister of Aurora");
    assert_eq!(aurora(&content).currency, "AUT");
    let before = content.at_bookmark("2001-07-14").unwrap();
    assert_eq!(aurora(&before).executive, "Fictional Steward A");
    let after = content.at_bookmark("2001-07-16").unwrap();
    assert_eq!(aurora(&after).executive, "Fictional Steward C");
    assert_eq!(aurora(&after).currency, "NEW-AUT");
    assert_eq!(aurora(&content).executive, "Fictional Steward B");
    assert_ne!(before.hash(), after.hash());
}

#[test]
fn collective_vacant_and_unknown_incumbents_remain_distinct() {
    let content = content();
    let offices = content.bookmark_offices(&CountryId("AUR".into()));
    assert_eq!(offices.len(), 4);
    let council = offices
        .iter()
        .find(|office| office.title == "Aurora Council")
        .unwrap();
    assert_eq!(council.holder, "Fictional Aurora Council");
    assert_eq!(council.role, OfficeRole::HeadOfState);
    assert!(matches!(
        council.term.as_ref().unwrap().holder,
        OfficeHolder::Collective { .. }
    ));
    let vacancy = offices
        .iter()
        .find(|office| office.title == "Assembly Speaker")
        .unwrap();
    assert_eq!(vacancy.holder, "Vacant");
    assert!(vacancy.term.is_some());
    let unknown = offices
        .iter()
        .find(|office| office.title == "Independent Auditor")
        .unwrap();
    assert_eq!(unknown.holder, "Unknown incumbent");
    assert!(unknown.term.is_none());
}

#[test]
fn future_reference_terms_do_not_force_campaign_successions_or_currency_changes() {
    let content = content();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(date(world.day.units()), "2001-07-16");
    assert_eq!(world.player_country().executive, "Fictional Steward B");
    assert_eq!(aurora(&content).currency, "AUT");
    world.validate(&content).unwrap();
    persistence::replay(&world, &content).unwrap();
    let incompatible = content.at_bookmark("2001-07-16").unwrap();
    assert!(world.validate(&incompatible).is_err());
}

#[test]
fn conflicts_gaps_broken_references_and_provenance_fail_content_assembly() {
    for variant in 0..10 {
        let mut value = raw();
        match variant {
            0 => value["identities"]["terms"][1]["validity"]["valid_to"] = "2001-07-17".into(),
            1 => value["identities"]["terms"][1]["holder"]["person"] = "absent".into(),
            2 => value["identities"]["offices"][0]["country"] = "BOR".into(),
            3 => {
                value["identities"]["terms"]
                    .as_array_mut()
                    .unwrap()
                    .remove(1);
            }
            4 => value["identities"]["currencies"][0]["validity"]["valid_to"] = "2001-07-14".into(),
            5 => value["identities"]["people"][1]["id"] = "person:aurora:a".into(),
            6 => value["identities"]["terms"][1]["source"] = "absent".into(),
            7 => value["identities"]["offices"][0]["validity"]["valid_from"] = "2001-07-15".into(),
            8 => {
                let source = value["sources"][0].clone();
                value["sources"].as_array_mut().unwrap().push(source);
            }
            _ => value["identities"]["currencies"][0]["validity"]["valid_to"] = "2001-07-17".into(),
        }
        assert!(
            Content::from_json(&value.to_string()).is_err(),
            "variant {variant}"
        );
    }
}

#[test]
fn office_lookup_and_conflict_detection_do_not_depend_on_record_order() {
    let mut value = raw();
    value["identities"]["terms"]
        .as_array_mut()
        .unwrap()
        .reverse();
    value["identities"]["currencies"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let content = Content::from_json(&value.to_string()).unwrap();
    assert_eq!(aurora(&content).executive, "Fictional Steward B");
    let term = value["identities"]["terms"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|term| term["id"] == "term:aurora:b")
        .unwrap();
    term["validity"]["valid_to"] = "2001-07-17".into();
    assert!(Content::from_json(&value.to_string()).is_err());
}

#[test]
fn a_mutated_compiled_identity_is_rejected_and_reassembly_restores_consistency() {
    let mut content = content();
    content.countries[0].executive = "Unresolved stale identity".into();
    assert!(content.validate().is_err());
    let repaired = content.at_bookmark(&content.start).unwrap();
    assert_eq!(aurora(&repaired).executive, "Fictional Steward B");
    repaired.validate().unwrap();
}
