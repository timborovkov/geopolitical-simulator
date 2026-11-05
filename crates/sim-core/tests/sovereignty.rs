use sim_core::*;
fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-sovereignty-test.json"
    ))
    .unwrap()
}
fn id(s: &str) -> CountryId {
    CountryId(s.into())
}
fn strait(c: &Content) -> TerritoryReference {
    c.sovereignty
        .as_ref()
        .unwrap()
        .territories_at(c.start.parse().unwrap())
        .into_iter()
        .find(|r| r.territory.id.0 == "strait")
        .unwrap()
        .reference
        .unwrap()
}

#[test]
fn dated_claims_control_and_recognition_are_independent_at_exclusive_boundaries() {
    let c = fixture();
    let opening = strait(&c);
    assert_eq!(
        opening.sovereignty,
        LegalSovereignty::Sovereign { country: id("AUR") }
    );
    assert_eq!(
        opening.control,
        TerritorialControl::Occupation {
            occupier: id("BOR")
        }
    );
    let before = c.at_bookmark("2000-12-31").unwrap();
    assert_eq!(strait(&before), opening);
    let later = c.at_bookmark("2001-01-01").unwrap();
    let row = strait(&later);
    assert_eq!(
        row.sovereignty,
        LegalSovereignty::Disputed {
            claimants: vec![id("AUR"), id("BOR")]
        }
    );
    assert_eq!(
        row.control,
        TerritorialControl::CivilAdministration { country: id("COR") }
    );
    assert_eq!(
        c.countries, later.countries,
        "reference changes cannot rewrite countries, governments or map markers"
    );
    let report = later.bookmark_sovereignty(&id("COR")).unwrap().unwrap();
    assert!(
        report
            .territories
            .iter()
            .any(|r| r.territory.id.0 == "strait")
    );
    let unknown = c
        .sovereignty
        .as_ref()
        .unwrap()
        .territories_at(c.start.parse().unwrap())
        .pop()
        .unwrap()
        .reference
        .unwrap();
    assert_eq!(unknown.sovereignty, LegalSovereignty::Unknown);
    assert_eq!(unknown.control, TerritorialControl::Unknown);
}
#[test]
fn recognition_is_directional_and_absence_is_unknown_rather_than_refusal() {
    let c = fixture();
    let r = c.sovereignty.as_ref().unwrap();
    let date = c.start.parse().unwrap();
    assert_eq!(
        r.recognition_at(&id("COR"), &id("AUR"), date),
        RecognitionPosition::Recognized
    );
    assert_eq!(
        r.recognition_at(&id("AUR"), &id("COR"), date),
        RecognitionPosition::Unknown
    );
    assert_eq!(
        r.recognition_at(&id("BOR"), &id("AUR"), date),
        RecognitionPosition::NotRecognized
    );
    assert_eq!(
        r.recognition_at(&id("AUR"), &id("BOR"), date),
        RecognitionPosition::Unknown
    );
    assert_eq!(
        r.recognition_at(&id("BOR"), &id("AUR"), "2001-01-01".parse().unwrap()),
        RecognitionPosition::Recognized
    );
    assert!(c.bookmark_sovereignty(&id("MISSING")).is_err());
    let plain = Content::bundled().unwrap();
    assert_eq!(plain.bookmark_sovereignty(&id("GBR")).unwrap(), None);
}
#[test]
fn malformed_chronology_identifiers_countries_provenance_and_unversioned_records_are_rejected() {
    let original: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/synthetic-sovereignty-test.json"
    ))
    .unwrap();
    for case in 0..19 {
        let mut v = original.clone();
        match case {
            0 => {
                v["sovereignty"]["territories"][1]["id"] =
                    v["sovereignty"]["territories"][0]["id"].clone()
            }
            1 => v["sovereignty"]["references"][0]["territory"] = "missing".into(),
            2 => v["sovereignty"]["references"][0]["sovereignty"]["country"] = "BAD".into(),
            3 => v["sovereignty"]["references"][3]["control"]["occupier"] = "BAD".into(),
            4 => {
                v["sovereignty"]["references"][4]["sovereignty"]["claimants"] =
                    serde_json::json!(["AUR", "AUR"])
            }
            5 => {
                v["sovereignty"]["references"][4]["sovereignty"]["claimants"] =
                    serde_json::json!(["AUR"])
            }
            6 => v["sovereignty"]["references"][0]["source"] = "missing".into(),
            7 => v["sovereignty"]["recognition"][0]["source_version"] = " ".into(),
            8 => v["sovereignty"]["references"][4]["validity"]["valid_from"] = "2000-12-31".into(),
            9 => v["sovereignty"]["references"][3]["validity"]["valid_to"] = "1990-01-01".into(),
            10 => v["sovereignty"]["references"][0]["validity"]["valid_from"] = "2000-01-02".into(),
            11 => {
                v["sovereignty"]["recognition"][2]["validity"]["valid_from"] = "2000-12-31".into()
            }
            12 => v["sovereignty"]["recognition"][0]["observer"] = "AUR".into(),
            13 => v["sovereignty"]["recognition"][0]["subject"] = "BAD".into(),
            14 => {
                v["rules"] = 18.into();
                v["schema"] = 18.into();
            }
            15 => v["sovereignty"] = serde_json::Value::Null,
            16 => v["sovereignty"]["territories"][0]["id"] = "Bad ID".into(),
            17 => {
                v["sovereignty"]["recognition"][0]["id"] =
                    v["sovereignty"]["references"][0]["id"].clone()
            }
            18 => v["sovereignty"]["references"][0]["validity"]["valid_from"] = "2000-02-30".into(),
            _ => unreachable!(),
        }
        assert!(Content::from_json(&v.to_string()).is_err(), "case {case}");
    }
    let mut gap = fixture();
    gap.sovereignty.as_mut().unwrap().references[0]
        .validity
        .valid_to = Some("2001-01-01".parse().unwrap());
    gap.validate().unwrap();
    assert!(gap.at_bookmark("2001-01-01").is_err());
    let rows = gap
        .sovereignty
        .as_ref()
        .unwrap()
        .territories_at("2001-01-01".parse().unwrap());
    assert!(
        rows[0].reference.is_none(),
        "a later gap stays unknown rather than fabricated control"
    );
}
#[test]
fn public_reports_freeze_bookmark_references_without_private_state_or_future_transfers() {
    let c = fixture();
    let mut w = World::new(&c, id("AUR"), 17).unwrap();
    let opening = w
        .public_report(&c, &id("BOR"))
        .unwrap()
        .bookmark_sovereignty
        .unwrap();
    let hash = w.hash();
    let projected = w.project(&c, Viewer::Observer).unwrap();
    assert!(projected.own.is_none());
    let report = serde_json::to_string(&projected.reports[&id("BOR")]).unwrap();
    assert!(report.contains("occupation"));
    assert!(!report.contains("strait_later"));
    for secret in [
        "treasury",
        "accounts",
        "projects",
        "ai_traces",
        "shock_state",
    ] {
        assert!(!report.contains(secret), "private {secret}");
    }
    assert_eq!(w.hash(), hash);
    let mut changed = World::new(&c, id("AUR"), 17).unwrap();
    for _ in 0..4 {
        if changed
            .preview_constitution(&c, &changed.player, &Regime::SingleParty, false)
            .is_ok()
        {
            break;
        }
        let delay = changed
            .preview(&c, &changed.player, &"coalition".into(), 1)
            .unwrap()
            .delay;
        changed
            .issue(
                &c,
                Action::Enact {
                    policy: "coalition".into(),
                    value: 1,
                },
            )
            .unwrap();
        changed.advance(&c, sim_core::DaySpan::new(delay)).unwrap();
    }
    let preview = changed
        .preview_constitution(&c, &changed.player, &Regime::SingleParty, false)
        .unwrap();
    changed
        .issue(
            &c,
            Action::Reconstitute {
                target: Regime::SingleParty,
                contested: false,
            },
        )
        .unwrap();
    changed
        .advance(
            &c,
            sim_core::DaySpan::new(preview.decision_day - changed.day),
        )
        .unwrap();
    assert_eq!(
        changed.governing_regime(&c, &id("AUR")).unwrap(),
        &Regime::SingleParty
    );
    assert_eq!(
        changed
            .public_report(&c, &id("BOR"))
            .unwrap()
            .bookmark_sovereignty,
        Some(opening.clone())
    );
    assert_eq!(changed.player, id("AUR"));
    assert_eq!(persistence::replay(&changed, &c).unwrap(), changed);
    w.advance(&c, sim_core::DaySpan::new(367)).unwrap();
    assert_eq!(
        w.public_report(&c, &id("BOR"))
            .unwrap()
            .bookmark_sovereignty,
        Some(opening)
    );
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
}
#[test]
fn reference_metadata_grants_no_cash_capacity_or_forced_political_change() {
    let c = fixture();
    let old = Content::from_json(include_str!(
        "../../../data/synthetic-technology-eligibility-test.json"
    ))
    .unwrap();
    for country in ["AUR", "BOR", "COR"] {
        let mut w = World::new(&c, id(country), 17).unwrap();
        let mut prior = World::new(&old, id(country), 17).unwrap();
        w.advance(&c, sim_core::DaySpan::new(30)).unwrap();
        prior.advance(&old, sim_core::DaySpan::new(30)).unwrap();
        for (world, pack) in [(&mut w, &c), (&mut prior, &old)] {
            world
                .issue(
                    pack,
                    Action::Enact {
                        policy: "energy".into(),
                        value: 1,
                    },
                )
                .unwrap();
            world.advance(pack, sim_core::DaySpan::new(370)).unwrap();
        }
        let mut normalized = w.clone();
        normalized.rules = prior.rules;
        normalized.content_hash = prior.content_hash;
        assert_eq!(
            normalized, prior,
            "{country}: references must not alter accounts, AI, decisions, events or capacity"
        );
        assert_eq!(persistence::replay(&w, &c).unwrap(), w);
    }
}
#[test]
fn format_20_continues_exactly_and_old_envelopes_cannot_hide_new_reference_content() {
    let c = fixture();
    let mut w = World::new(&c, id("AUR"), 17).unwrap();
    w.advance(&c, sim_core::DaySpan::new(30)).unwrap();
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    let bytes = persistence::encode(&w, &c).unwrap();
    let mut header: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(header["schema"], 20);
    let mut restored = persistence::decode(&bytes, &c).unwrap();
    w.advance(&c, sim_core::DaySpan::new(400)).unwrap();
    restored.advance(&c, sim_core::DaySpan::new(400)).unwrap();
    assert_eq!(w, restored);
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
    header["schema"] = 19.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&header).unwrap(), &c)
            .unwrap_err()
            .contains("format 20")
    );
    assert!(
        Content::bundled_for_hash(c.hash()).is_err(),
        "optional catalogue requires the original exact pack"
    );
    let old = Content::technology_bundled().unwrap();
    let old_world = World::new(&old, id("GBR"), 17).unwrap();
    assert_eq!(old_world.hash(), 0xd5f9f1f64662dcd3);
    assert_eq!(persistence::snapshot_schema(&old_world), 19);
}
