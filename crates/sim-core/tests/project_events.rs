use sim_core::*;

fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-project-events-test.json"
    ))
    .unwrap()
}
fn world(content: &Content) -> World {
    World::new(content, CountryId("AUR".into()), 17).unwrap()
}
fn propose(world: &mut World, content: &Content, policy: &str, value: i32) -> CommitmentId {
    let request = CommitmentId(world.next_project);
    world
        .issue(
            content,
            Action::Enact {
                policy: policy.into(),
                value,
            },
        )
        .unwrap();
    request
}
fn check(world: &World, content: &Content) {
    world.validate(content).unwrap();
    let bytes = persistence::encode(world, content).unwrap();
    let restored = persistence::decode(&bytes, content).unwrap();
    assert_eq!(persistence::encode(&restored, content).unwrap(), bytes);
    assert_eq!(restored, persistence::replay(&restored, content).unwrap());
    assert_eq!(persistence::snapshot_schema(world), 24);
}
fn linked(world: &World, request: CommitmentId, stage: CommitmentStage) -> &Event {
    world
        .events
        .iter()
        .find(|event| event.context == Some(EventContext::Commitment { request, stage }))
        .unwrap()
}

#[test]
fn exact_same_day_decisions_delivery_and_review_resolve_owned_programmes() {
    let c = content();
    let mut w = world(&c);
    let request = propose(&mut w, &c, "income_tax", 30);
    w.issue(&c, Action::AmendCommitment { request, value: 32 })
        .unwrap();
    let other = propose(&mut w, &c, "services", 12);
    assert_ne!(request, other);
    for (id, stage) in [
        (request, CommitmentStage::Proposed),
        (request, CommitmentStage::Amended),
        (other, CommitmentStage::Proposed),
    ] {
        let event = linked(&w, id, stage);
        assert_eq!(event.day, sim_core::CampaignDay::new(0));
        assert_eq!(
            w.event_commitment(
                &c,
                Viewer::Government(w.player.clone()),
                event.id.as_ref().unwrap()
            )
            .unwrap()
            .unwrap()
            .id,
            id
        );
    }
    let consent = w.player_country().project(&request).unwrap().consent_day;
    w.advance(&c, sim_core::DaySpan::new((consent).units()))
        .unwrap();
    linked(&w, request, CommitmentStage::Implementing);
    let due = w.player_country().project(&request).unwrap().due_day;
    w.advance(&c, sim_core::DaySpan::new((due).units() - w.day.units()))
        .unwrap();
    assert_eq!(w.player_country().tax, 32);
    linked(&w, request, CommitmentStage::Operating);
    let review = w
        .player_country()
        .commitment(&request)
        .unwrap()
        .review_due
        .unwrap();
    w.advance(&c, sim_core::DaySpan::new((review).units() - w.day.units()))
        .unwrap();
    let event = linked(&w, request, CommitmentStage::Evaluated);
    assert_eq!(
        w.event_commitment(
            &c,
            Viewer::Government(w.player.clone()),
            event.id.as_ref().unwrap()
        )
        .unwrap(),
        w.player_country().commitment(&request)
    );
    check(&w, &c);
}

#[test]
fn withdrawal_and_refusal_link_history_and_rejected_orders_create_no_event() {
    let c = content();
    let mut w = world(&c);
    let opening_tax = w.player_country().tax;
    let request = propose(&mut w, &c, "income_tax", 30);
    let before = w.clone();
    assert!(
        w.issue(&c, Action::AmendCommitment { request, value: 99 })
            .is_err()
    );
    assert_eq!(w, before);
    w.issue(&c, Action::CancelCommitment { request }).unwrap();
    linked(&w, request, CommitmentStage::Withdrawn);
    assert!(w.player_country().project(&request).is_none());
    let refused = propose(&mut w, &c, "income_tax", 32);
    w.issue_developer(
        &c,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::LegislativeSupport,
            value: 0,
        },
    )
    .unwrap();
    let consent = w.player_country().project(&refused).unwrap().consent_day;
    w.advance(&c, sim_core::DaySpan::new((consent).units()))
        .unwrap();
    let event = linked(&w, refused, CommitmentStage::Refused);
    assert_eq!(
        w.event_commitment(
            &c,
            Viewer::Government(w.player.clone()),
            event.id.as_ref().unwrap()
        )
        .unwrap()
        .unwrap()
        .stage(),
        CommitmentStage::Refused
    );
    assert_eq!(w.player_country().tax, opening_tax);
    check(&w, &c);
}

#[test]
fn lookup_uses_typed_context_not_names_or_text_and_excludes_foreign_and_observer_views() {
    let mut c = content();
    c.policies
        .iter_mut()
        .find(|p| p.id == "income_tax")
        .unwrap()
        .name = "Renamed fiscal programme".into();
    c.policies.reverse();
    c.validate().unwrap();
    let mut w = world(&c);
    let request = propose(&mut w, &c, "income_tax", 30);
    let event = linked(&w, request, CommitmentStage::Proposed)
        .id
        .clone()
        .unwrap();
    let before = w.clone();
    assert_eq!(
        w.event_commitment(&c, Viewer::Government(w.player.clone()), &event)
            .unwrap()
            .unwrap()
            .project
            .policy,
        "income_tax"
    );
    assert!(
        w.event_commitment(&c, Viewer::Government(CountryId("BOR".into())), &event)
            .unwrap()
            .is_none()
    );
    assert!(
        w.event_commitment(&c, Viewer::Observer, &event)
            .unwrap()
            .is_none()
    );
    assert!(
        w.event_commitment(
            &c,
            Viewer::Government(w.player.clone()),
            &EventId {
                country: w.player.clone(),
                sequence: u64::MAX
            }
        )
        .unwrap()
        .is_none()
    );
    let owner = w.project(&c, Viewer::Government(w.player.clone())).unwrap();
    assert!(
        owner
            .own_events
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
    assert_eq!(w, before);
    check(&w, &c);
    // Presentation text is never a selector; changing it cannot redirect a link.
    w.events
        .iter_mut()
        .find(|e| e.id.as_ref() == Some(&event))
        .unwrap()
        .text = "Translated unrelated display text".into();
    assert_eq!(
        w.event_commitment(&c, Viewer::Government(w.player.clone()), &event)
            .unwrap()
            .unwrap()
            .id,
        request
    );
}

#[test]
fn malformed_contexts_and_old_envelopes_fail_without_charging_a_command() {
    let c = content();
    assert!(Content::bundled_for_hash(c.hash()).is_err());
    let mut w = world(&c);
    let request = propose(&mut w, &c, "income_tax", 30);
    let index = w
        .events
        .iter()
        .position(|e| {
            e.context
                == Some(EventContext::Commitment {
                    request,
                    stage: CommitmentStage::Proposed,
                })
        })
        .unwrap();
    for context in [
        None,
        Some(EventContext::Commitment {
            request: CommitmentId(u64::MAX),
            stage: CommitmentStage::Proposed,
        }),
        Some(EventContext::Commitment {
            request,
            stage: CommitmentStage::Operating,
        }),
        Some(EventContext::Commitment {
            request,
            stage: CommitmentStage::Funded,
        }),
    ] {
        let mut broken = w.clone();
        broken.events[index].context = context;
        let before = broken.clone();
        assert!(
            broken
                .event_commitment(
                    &c,
                    Viewer::Government(broken.player.clone()),
                    broken.events[index].id.as_ref().unwrap()
                )
                .is_err()
        );
        assert!(broken.validate(&c).is_err());
        assert!(persistence::encode(&broken, &c).is_err());
        assert!(
            broken
                .issue(&c, Action::CancelCommitment { request })
                .is_err()
        );
        assert_eq!(broken, before);
    }
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    envelope["schema"] = 23.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&envelope).unwrap(), &c)
            .unwrap_err()
            .contains("format 24")
    );
    let old = Content::event_reference_bundled().unwrap();
    let mut prior = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(prior.hash(), 0x0fc2d7e7ebdb9688);
    assert!(prior.events.iter().all(|e| e.context.is_none()));
    prior.events[0].context = Some(EventContext::General);
    assert!(prior.validate(&old).is_err());
}

#[test]
fn expired_event_links_do_not_expire_programme_history_or_reuse_event_ids() {
    let c = content();
    let mut w = world(&c);
    let request = propose(&mut w, &c, "income_tax", 30);
    let reference = linked(&w, request, CommitmentStage::Proposed)
        .id
        .clone()
        .unwrap();
    for _ in 0..205 {
        let request = ReserveOrderId(w.next_command.0);
        w.issue(
            &c,
            Action::ProcureReserve {
                good: GoodKind::Food,
                days: sim_core::NeedDays::new(1),
            },
        )
        .unwrap();
        w.issue(&c, Action::CancelReserve { request }).unwrap();
    }
    assert_eq!(w.events.len(), 400);
    assert!(
        w.event_commitment(&c, Viewer::Government(w.player.clone()), &reference)
            .unwrap()
            .is_none()
    );
    assert_eq!(w.player_country().commitment(&request).unwrap().id, request);
    w.issue(&c, Action::AmendCommitment { request, value: 32 })
        .unwrap();
    let amended = linked(&w, request, CommitmentStage::Amended)
        .id
        .clone()
        .unwrap();
    assert!(amended.sequence > reference.sequence);
    check(&w, &c);
}
