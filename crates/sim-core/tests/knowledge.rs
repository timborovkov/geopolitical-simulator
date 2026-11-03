use sim_core::*;
#[test]
fn government_projection_excludes_foreign_accounts_and_future_truth() {
    let content = Content::bundled().unwrap();
    let mut w = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    w.advance(&content, 31).unwrap();
    let view = w
        .project(&content, Viewer::Government(w.player.clone()))
        .unwrap();
    assert_eq!(view.own.as_ref().unwrap().id, w.player);
    assert_eq!(view.reports.len(), 15);
    let china = &view.reports[&CountryId("CHN".into())];
    assert_eq!(china.latest.as_ref().unwrap().day, 30);
    let report = serde_json::to_value(china).unwrap();
    for key in [
        "treasury",
        "accounts",
        "households",
        "elite_support",
        "projects",
        "seed",
        "journal",
        "ledger",
    ] {
        assert!(report.get(key).is_none());
    }
    let as_json = serde_json::to_value(&view).unwrap();
    assert!(as_json.get("seed").is_none());
    assert!(as_json.get("journal").is_none());
    let unchanged = china.latest.clone();
    w.advance(&content, 1).unwrap();
    assert_eq!(
        w.public_report(&content, &china.country).unwrap().latest,
        unchanged
    );
}
#[test]
fn observer_has_only_public_reports_and_absent_releases_stay_unknown() {
    let c = Content::bundled().unwrap();
    let w = World::new(&c, CountryId("GBR".into()), 42).unwrap();
    let view = w.project(&c, Viewer::Observer).unwrap();
    assert!(view.own.is_none());
    assert!(
        view.reports
            .values()
            .all(|r| r.latest.is_none() && r.history.is_empty())
    );
    assert!(
        w.project(&c, Viewer::Government(CountryId("absent".into())))
            .is_err()
    );
}

#[test]
fn presentation_and_projection_changes_preserve_version_two_campaign_replay() {
    let c = Content::classic_bundled().unwrap();
    let mut w = World::new(&c, CountryId("GBR".into()), 2000).unwrap();
    w.advance(&c, 360).unwrap();
    assert_eq!(w.hash(), 0xc3994a175e880d20);
    let before = w.hash();
    let _ = w.project(&c, Viewer::Observer).unwrap();
    assert_eq!(w.hash(), before);
    let loaded =
        sim_core::persistence::decode(&sim_core::persistence::encode(&w, &c).unwrap(), &c).unwrap();
    assert_eq!(loaded.hash(), before);
}
