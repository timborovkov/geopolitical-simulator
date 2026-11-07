use sim_core::*;

fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-event-references-test.json"
    ))
    .unwrap()
}
fn world(c: &Content) -> World {
    World::new(c, CountryId("AUR".into()), 17).unwrap()
}
fn reference(country: &str, sequence: u64) -> EventId {
    EventId {
        country: CountryId(country.into()),
        sequence,
    }
}
fn with_pruned_history(c: &Content) -> World {
    let mut w = world(c);
    for _ in 0..205 {
        let request = ReserveOrderId(w.next_command.0);
        w.issue(
            c,
            Action::ProcureReserve {
                good: GoodKind::Food,
                days: sim_core::NeedDays::new(1),
            },
        )
        .unwrap();
        w.issue(c, Action::CancelReserve { request }).unwrap();
    }
    w.validate(c).unwrap();
    w
}

#[test]
fn references_are_country_scoped_private_and_read_only_through_commands_and_daily_events() {
    let c = content();
    let mut w = world(&c);
    assert_eq!(w.events[0].id, Some(reference("AUR", 1)));
    assert_eq!(
        w.event_sequences.as_ref().unwrap()[&CountryId("BOR".into())],
        1
    );
    let first = w.events[0].clone();
    let before = w.clone();
    assert!(
        w.issue(
            &c,
            Action::Enact {
                policy: "missing".into(),
                value: 1
            }
        )
        .is_err()
    );
    assert_eq!(w, before);
    w.issue(
        &c,
        Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(60)).unwrap();
    assert_eq!(
        w.event_by_reference(
            &c,
            Viewer::Government(w.player.clone()),
            &reference("AUR", 1)
        )
        .unwrap(),
        Some(&first)
    );
    let foreign = w
        .events
        .iter()
        .find(|e| e.country.0 == "BOR")
        .unwrap()
        .id
        .clone()
        .unwrap();
    let before = w.hash();
    assert!(
        w.event_by_reference(&c, Viewer::Government(w.player.clone()), &foreign)
            .unwrap()
            .is_none()
    );
    assert!(
        w.event_by_reference(&c, Viewer::Government(foreign.country.clone()), &foreign)
            .unwrap()
            .is_some()
    );
    assert!(
        w.event_by_reference(&c, Viewer::Observer, &foreign)
            .unwrap()
            .is_none()
    );
    let own = w.project(&c, Viewer::Government(w.player.clone())).unwrap();
    assert!(
        own.own_events
            .as_ref()
            .unwrap()
            .iter()
            .all(|e| e.country == w.player)
    );
    assert!(
        w.project(&c, Viewer::Observer)
            .unwrap()
            .own_events
            .is_none()
    );
    let observer = serde_json::to_value(w.project(&c, Viewer::Observer).unwrap()).unwrap();
    assert!(observer.get("own_events").is_none());
    assert!(observer.get("event_sequences").is_none());
    let mut other = c.clone();
    other.description.push_str(" other");
    assert!(
        w.event_by_reference(
            &other,
            Viewer::Government(w.player.clone()),
            &reference("AUR", 1)
        )
        .is_err()
    );
    assert_eq!(w.hash(), before);
    w.validate(&c).unwrap();
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
    let prior_content = Content::from_json(include_str!(
        "../../../data/synthetic-national-reserves-test.json"
    ))
    .unwrap();
    let mut prior = world(&prior_content);
    prior
        .issue(
            &prior_content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 30,
            },
        )
        .unwrap();
    prior
        .advance(&prior_content, sim_core::DaySpan::new(60))
        .unwrap();
    let mut normalized = w;
    normalized.rules = prior.rules;
    normalized.content_hash = prior.content_hash;
    normalized.event_sequences = None;
    for event in &mut normalized.events {
        event.id = None;
    }
    assert_eq!(
        normalized, prior,
        "Event references must not alter any economic, political or AI outcome"
    );
}

#[test]
fn format_23_resumes_pending_programmes_and_keeps_all_earlier_rules_and_hashes() {
    let c = content();
    let mut w = world(&c);
    w.issue(
        &c,
        Action::ProcureReserve {
            good: GoodKind::Energy,
            days: sim_core::NeedDays::new(1),
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(7)).unwrap();
    let bytes = persistence::encode(&w, &c).unwrap();
    let mut header: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(header["schema"], 23);
    header["schema"] = 22.into();
    assert!(persistence::decode(&serde_json::to_vec(&header).unwrap(), &c).is_err());
    let mut resumed = persistence::decode(&bytes, &c).unwrap();
    w.advance(&c, sim_core::DaySpan::new(90)).unwrap();
    resumed.advance(&c, sim_core::DaySpan::new(90)).unwrap();
    assert_eq!(w, resumed);
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
    let old = Content::reserve_bundled().unwrap();
    let old_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(old.rules, 21);
    assert_eq!(old_world.hash(), 0x343d72b6cdfb6c83);
    let legacy = serde_json::to_value(&old_world).unwrap();
    assert!(legacy.get("event_sequences").is_none());
    assert!(
        legacy["events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e.get("id").is_none())
    );
    assert!(Content::bundled_for_hash(c.hash()).is_err());
}

#[test]
fn retention_never_reuses_references_and_retained_entries_survive_filtering_and_reload() {
    let c = content();
    let mut w = with_pruned_history(&c);
    assert_eq!(w.events.len(), 400);
    assert!(
        w.event_by_reference(
            &c,
            Viewer::Government(w.player.clone()),
            &reference("AUR", 1)
        )
        .unwrap()
        .is_none()
    );
    let retained = w.events[200].clone();
    let expected_next = w.event_sequences.as_ref().unwrap()[&w.player];
    w.issue(
        &c,
        Action::ProcureReserve {
            good: GoodKind::Energy,
            days: sim_core::NeedDays::new(1),
        },
    )
    .unwrap();
    assert_eq!(
        w.events.last().unwrap().id,
        Some(reference("AUR", expected_next))
    );
    assert_eq!(
        w.event_by_reference(
            &c,
            Viewer::Government(w.player.clone()),
            retained.id.as_ref().unwrap()
        )
        .unwrap(),
        Some(&retained)
    );
    let restored = persistence::decode(&persistence::encode(&w, &c).unwrap(), &c).unwrap();
    assert_eq!(restored, w);
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
}

#[test]
fn malformed_or_legacy_reference_metadata_is_rejected_before_encoding() {
    let c = content();
    let mut base = world(&c);
    base.advance(&c, sim_core::DaySpan::new(30)).unwrap();
    for variant in 0..9 {
        let mut w = base.clone();
        match variant {
            0 => w.event_sequences = None,
            1 => {
                w.event_sequences
                    .as_mut()
                    .unwrap()
                    .remove(&CountryId("BOR".into()));
            }
            2 => {
                *w.event_sequences
                    .as_mut()
                    .unwrap()
                    .get_mut(&w.player)
                    .unwrap() = 0;
            }
            3 => w.events[0].id = None,
            4 => w.events[0].id.as_mut().unwrap().sequence = 0,
            5 => w.events[0].id.as_mut().unwrap().country = CountryId("BOR".into()),
            6 => w.events[0].day = w.day.units() + 1,
            7 => {
                w.events.pop();
            }
            _ => w.events[0].text.clear(),
        }
        assert!(
            persistence::encode(&w, &c).is_err(),
            "Malformed event metadata variant {variant}"
        );
    }
    let old = Content::reserve_bundled().unwrap();
    let mut old_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    old_world.events[0].id = Some(reference("GBR", 1));
    assert!(persistence::encode(&old_world, &old).is_err());
}

fn exhaust_counter(w: &mut World) {
    assert_eq!(w.events.len(), 400);
    let count = w.events.iter().filter(|e| e.country == w.player).count() as u64;
    for (index, event) in w
        .events
        .iter_mut()
        .filter(|e| e.country == w.player)
        .enumerate()
    {
        event.id = Some(EventId {
            country: w.player.clone(),
            sequence: u64::MAX - count + index as u64,
        });
    }
    *w.event_sequences
        .as_mut()
        .unwrap()
        .get_mut(&w.player)
        .unwrap() = u64::MAX;
}

#[test]
fn exhausted_reference_space_rolls_back_paid_commands_and_incomplete_daily_steps() {
    let c = content();
    let ordinary = with_pruned_history(&c);
    // Artificial boundary fixture: keep retained ordering valid so allocation is exercised.
    let mut exhausted = ordinary.clone();
    exhaust_counter(&mut exhausted);
    exhausted.validate(&c).unwrap();
    let before = exhausted.clone();
    let error = exhausted
        .issue(
            &c,
            Action::ProcureReserve {
                good: GoodKind::Energy,
                days: sim_core::NeedDays::new(1),
            },
        )
        .unwrap_err();
    assert!(error.contains("reference space exhausted"), "{error}");
    assert_eq!(exhausted, before);
    let mut annual = ordinary;
    annual.advance(&c, sim_core::DaySpan::new(365)).unwrap();
    exhaust_counter(&mut annual);
    annual.validate(&c).unwrap();
    let before = annual.clone();
    let error = annual.advance(&c, sim_core::DaySpan::new(1)).unwrap_err();
    assert!(error.contains("reference space exhausted"), "{error}");
    assert_eq!(annual, before);
}

#[test]
fn reference_parser_is_bounded_and_preserves_canonical_country_identity() {
    let id = reference("AUR", 12);
    assert_eq!(id.to_string().parse::<EventId>().unwrap(), id);
    for invalid in [
        "",
        "12",
        "#1",
        "AUR#",
        "AUR#0",
        "AU R#1",
        "AUR#-1",
        "AUR#+1",
        "AUR#1.0",
        "AUR#18446744073709551616",
    ] {
        assert!(invalid.parse::<EventId>().is_err(), "{invalid}");
    }
}
