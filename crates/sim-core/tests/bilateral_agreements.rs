use sim_core::*;

fn fixture() -> (Content, World) {
    let content = Content::diplomacy_bundled().unwrap();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    (content, world)
}
fn terms(duration: u32, notice: u32) -> NonAggressionTerms {
    NonAggressionTerms {
        duration: DaySpan::new(duration),
        notice: DaySpan::new(notice),
    }
}
fn issue(world: &mut World, content: &Content, action: TreatyAction) {
    world.issue(content, Action::Diplomacy { action }).unwrap();
}
fn offer(world: &mut World, content: &Content, with: &str, duration: u32, notice: u32) {
    issue(
        world,
        content,
        TreatyAction::Offer {
            with: CountryId(with.into()),
            terms: terms(duration, notice),
        },
    );
}
fn record(world: &World) -> &TreatyRecord {
    &world.diplomacy.as_ref().unwrap().agreements[&TreatyId(1)]
}
fn owned(content: &Content, world: &World) -> Vec<TreatyView> {
    world
        .treaties_view(
            content,
            Viewer::Government(world.player.clone()),
            &world.player,
        )
        .unwrap()
}
fn exact(world: &World, content: &Content) {
    world.validate(content).unwrap();
    let bytes = persistence::encode(world, content).unwrap();
    let loaded = persistence::decode(&bytes, content).unwrap();
    assert_eq!(persistence::encode(&loaded, content).unwrap(), bytes);
    let replay = persistence::replay(world, content).unwrap();
    assert_eq!(persistence::encode(&replay, content).unwrap(), bytes);
}
fn activate(content: &Content, world: &mut World, duration: u32, notice: u32) {
    offer(world, content, "BOR", duration, notice);
    world.advance(content, DaySpan::new(3)).unwrap();
    assert_eq!(record(world).stage, TreatyStage::AwaitingRatification);
    issue(
        world,
        content,
        TreatyAction::Ratify {
            agreement: TreatyId(1),
            revision: TreatyRevision(1),
        },
    );
    world.advance(content, DaySpan::new(25)).unwrap();
    assert_eq!(world.day, CampaignDay::new(28));
    assert_eq!(record(world).stage, TreatyStage::Active);
}

#[test]
fn paid_signatures_and_two_domestic_reviews_precede_next_day_activation_and_expiry() {
    let (content, mut world) = fixture();
    offer(&mut world, &content, "BOR", 180, 30);
    assert_eq!(record(&world).administration_paid[&world.player], Money(10));
    assert_eq!(record(&world).stage, TreatyStage::Negotiating);
    exact(&world, &content);
    world.advance(&content, DaySpan::new(3)).unwrap();
    assert_eq!(record(&world).signatures.len(), 2);
    assert!(record(&world).activated_on.is_none());
    exact(&world, &content);
    issue(
        &mut world,
        &content,
        TreatyAction::Ratify {
            agreement: TreatyId(1),
            revision: TreatyRevision(1),
        },
    );
    let pending = persistence::encode(&world, &content).unwrap();
    let mut resumed = persistence::decode(&pending, &content).unwrap();
    world.advance(&content, DaySpan::new(24)).unwrap();
    resumed.advance(&content, DaySpan::new(24)).unwrap();
    assert_eq!(
        persistence::encode(&world, &content).unwrap(),
        persistence::encode(&resumed, &content).unwrap()
    );
    assert_eq!(world.day, CampaignDay::new(27));
    assert_eq!(record(&world).stage, TreatyStage::Scheduled);
    assert_eq!(record(&world).activation_day, Some(CampaignDay::new(28)));
    assert_eq!(
        record(&world)
            .administration_paid
            .values()
            .map(|paid| paid.0)
            .sum::<i64>(),
        40
    );
    assert!(
        record(&world)
            .ratifications
            .values()
            .all(|request| request.passed.is_some())
    );
    world.advance(&content, DaySpan::new(1)).unwrap();
    assert_eq!(record(&world).stage, TreatyStage::Active);
    assert_eq!(record(&world).expires_at, Some(CampaignDay::new(208)));
    world.advance(&content, DaySpan::new(179)).unwrap();
    assert_eq!(record(&world).stage, TreatyStage::Active);
    world.advance(&content, DaySpan::new(1)).unwrap();
    assert_eq!(record(&world).stage, TreatyStage::Expired);
    assert_eq!(record(&world).closed_on, Some(CampaignDay::new(208)));
    for country in world.countries.values() {
        assert_eq!(country.accounts.total(), country.opening_money.0);
    }
    exact(&world, &content);
}

#[test]
fn counteroffers_invalidate_old_signatures_and_stale_orders_do_not_charge_or_mutate() {
    let (content, mut world) = fixture();
    offer(&mut world, &content, "BOR", 730, 0);
    world.advance(&content, DaySpan::new(3)).unwrap();
    assert_eq!(record(&world).revision, TreatyRevision(2));
    assert_eq!(record(&world).terms, terms(365, 30));
    assert_eq!(record(&world).offered_by, CountryId("BOR".into()));
    assert_eq!(record(&world).signatures.len(), 1);
    let before = persistence::encode(&world, &content).unwrap();
    assert!(
        world
            .issue(
                &content,
                Action::Diplomacy {
                    action: TreatyAction::Sign {
                        agreement: TreatyId(1),
                        revision: TreatyRevision(1)
                    }
                }
            )
            .is_err()
    );
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    issue(
        &mut world,
        &content,
        TreatyAction::Sign {
            agreement: TreatyId(1),
            revision: TreatyRevision(2),
        },
    );
    let public = world
        .treaties_view(&content, Viewer::Observer, &world.player)
        .unwrap();
    assert!(public[0].milestones.iter().all(|milestone| !matches!(
        milestone.outcome,
        TreatyOutcome::Offered | TreatyOutcome::Counteroffered
    )));
    issue(
        &mut world,
        &content,
        TreatyAction::Ratify {
            agreement: TreatyId(1),
            revision: TreatyRevision(2),
        },
    );
    world.advance(&content, DaySpan::new(25)).unwrap();
    assert_eq!(record(&world).stage, TreatyStage::Active);
    assert_eq!(record(&world).administration_paid[&world.player], Money(30));
    assert_eq!(
        record(&world).administration_paid[&CountryId("BOR".into())],
        Money(20)
    );
    exact(&world, &content);
}

#[test]
fn refused_private_offers_retain_fees_and_pair_cooldown_and_never_become_public() {
    let (content, mut world) = fixture();
    offer(&mut world, &content, "COR", 180, 30);
    world.advance(&content, DaySpan::new(3)).unwrap();
    assert_eq!(record(&world).stage, TreatyStage::Refused);
    assert_eq!(owned(&content, &world).len(), 1);
    for viewer in [
        Viewer::Observer,
        Viewer::Government(CountryId("COR".into())),
    ] {
        for country in ["AUR", "COR"] {
            assert!(
                world
                    .treaties_view(&content, viewer.clone(), &CountryId(country.into()))
                    .unwrap()
                    .is_empty()
            );
        }
    }
    world.advance(&content, DaySpan::new(29)).unwrap();
    let before = persistence::encode(&world, &content).unwrap();
    assert!(
        world
            .issue(
                &content,
                Action::Diplomacy {
                    action: TreatyAction::Offer {
                        with: CountryId("COR".into()),
                        terms: terms(180, 30)
                    }
                }
            )
            .is_err()
    );
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    world.advance(&content, DaySpan::new(1)).unwrap();
    offer(&mut world, &content, "COR", 180, 30);
    assert_eq!(world.diplomacy.as_ref().unwrap().next_id, TreatyId(3));
    assert_eq!(record(&world).administration_paid[&world.player], Money(10));
    exact(&world, &content);
}

#[test]
fn notices_preserve_obligations_until_the_deadline_and_natural_expiry_wins_a_tie() {
    for (notice, expected) in [(0, TreatyStage::Terminated), (30, TreatyStage::Expired)] {
        let (mut content, _) = fixture();
        content
            .diplomacy
            .as_mut()
            .unwrap()
            .ai
            .get_mut(&CountryId("BOR".into()))
            .unwrap()
            .minimum_notice = DaySpan::ZERO;
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        activate(&content, &mut world, 30, notice);
        let fees = record(&world).administration_paid.clone();
        let cash = world.player_country().available_cash();
        world
            .issue(&content, Action::Repay { amount: cash })
            .unwrap();
        assert_eq!(world.player_country().available_cash(), Money(0));
        issue(
            &mut world,
            &content,
            TreatyAction::Terminate {
                agreement: TreatyId(1),
                revision: TreatyRevision(1),
            },
        );
        assert_eq!(record(&world).administration_paid, fees);
        if notice > 0 {
            assert_eq!(record(&world).stage, TreatyStage::Notice);
            let saved = persistence::encode(&world, &content).unwrap();
            let mut resumed = persistence::decode(&saved, &content).unwrap();
            world.advance(&content, DaySpan::new(29)).unwrap();
            assert_eq!(record(&world).stage, TreatyStage::Notice);
            world.advance(&content, DaySpan::new(1)).unwrap();
            resumed.advance(&content, DaySpan::new(30)).unwrap();
            assert_eq!(
                persistence::encode(&world, &content).unwrap(),
                persistence::encode(&resumed, &content).unwrap()
            );
        }
        assert_eq!(record(&world).stage, expected);
        assert_eq!(
            record(&world)
                .milestones
                .iter()
                .filter(|milestone| matches!(
                    milestone.outcome,
                    TreatyOutcome::Expired | TreatyOutcome::Terminated
                ))
                .count(),
            1
        );
        exact(&world, &content);
    }
}

#[test]
fn unanswered_and_unsubmitted_deadlines_close_before_ai_and_keep_paid_costs() {
    let (mut content, _) = fixture();
    content.diplomacy.as_mut().unwrap().ai_cadence = DaySpan::new(30);
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    offer(&mut world, &content, "BOR", 180, 30);
    world.advance(&content, DaySpan::new(30)).unwrap();
    assert_eq!(record(&world).stage, TreatyStage::TimedOut);
    assert_eq!(record(&world).closed_on, Some(CampaignDay::new(30)));
    assert_eq!(record(&world).signatures.len(), 1);
    exact(&world, &content);
    let (content, mut world) = fixture();
    offer(&mut world, &content, "BOR", 180, 30);
    world.advance(&content, DaySpan::new(33)).unwrap();
    assert_eq!(record(&world).stage, TreatyStage::TimedOut);
    assert_eq!(record(&world).closed_on, Some(CampaignDay::new(33)));
    assert!(
        record(&world).ratifications[&CountryId("BOR".into())]
            .passed
            .is_some()
    );
    assert_eq!(record(&world).administration_paid[&world.player], Money(10));
    exact(&world, &content);
}

#[test]
fn withdrawing_a_pending_ratification_keeps_fees_and_invalid_control_or_cash_cannot_sign() {
    let (content, mut world) = fixture();
    offer(&mut world, &content, "BOR", 180, 30);
    let before = persistence::encode(&world, &content).unwrap();
    let foreign = world.command(
        CountryId("BOR".into()),
        Action::Diplomacy {
            action: TreatyAction::Sign {
                agreement: TreatyId(1),
                revision: TreatyRevision(1),
            },
        },
    );
    assert!(world.execute(&content, foreign).is_err());
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    world.advance(&content, DaySpan::new(3)).unwrap();
    assert!(world.journal.iter().any(|entry| entry.ai
        && entry.command.issuer == CountryId("BOR".into())
        && matches!(
            entry.command.action,
            Action::Diplomacy {
                action: TreatyAction::Sign { .. }
            }
        )));
    issue(
        &mut world,
        &content,
        TreatyAction::Ratify {
            agreement: TreatyId(1),
            revision: TreatyRevision(1),
        },
    );
    let fees = record(&world).administration_paid.clone();
    issue(
        &mut world,
        &content,
        TreatyAction::Withdraw {
            agreement: TreatyId(1),
            revision: TreatyRevision(1),
        },
    );
    assert_eq!(record(&world).stage, TreatyStage::Withdrawn);
    assert_eq!(record(&world).administration_paid, fees);
    exact(&world, &content);
    let (content, mut world) = fixture();
    world
        .issue(
            &content,
            Action::Repay {
                amount: world.player_country().available_cash(),
            },
        )
        .unwrap();
    let before = persistence::encode(&world, &content).unwrap();
    assert!(
        world
            .issue(
                &content,
                Action::Diplomacy {
                    action: TreatyAction::Offer {
                        with: CountryId("BOR".into()),
                        terms: terms(180, 30)
                    }
                }
            )
            .is_err()
    );
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
}

#[test]
fn government_succession_lapses_pending_signatures_but_preserves_active_state_obligations() {
    let (mut content, _) = fixture();
    let route = &mut content.constitutions.as_mut().unwrap().lawful;
    route.days = DaySpan::new(2);
    route.legislative_threshold = 0;
    route.elite_threshold = 0;
    route.council_threshold = 0;
    route.approval_threshold = 0;
    for active in [false, true] {
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        if active {
            activate(&content, &mut world, 180, 30);
        } else {
            offer(&mut world, &content, "BOR", 180, 30);
        }
        world
            .issue(
                &content,
                Action::Reconstitute {
                    target: Regime::Military,
                    contested: false,
                },
            )
            .unwrap();
        world.advance(&content, DaySpan::new(2)).unwrap();
        assert_eq!(
            world.governing_regime(&content, &world.player).unwrap(),
            &Regime::Military
        );
        assert_eq!(
            record(&world).stage,
            if active {
                TreatyStage::Active
            } else {
                TreatyStage::SponsorshipLapsed
            }
        );
        exact(&world, &content);
    }
}

#[test]
fn actual_review_rechecks_authority_and_explicit_developer_stress_remains_replayable() {
    let (content, mut world) = fixture();
    offer(&mut world, &content, "BOR", 180, 30);
    world.advance(&content, DaySpan::new(3)).unwrap();
    issue(
        &mut world,
        &content,
        TreatyAction::Ratify {
            agreement: TreatyId(1),
            revision: TreatyRevision(1),
        },
    );
    world
        .issue_developer(
            &content,
            world.player.clone(),
            DeveloperEdit {
                field: DeveloperField::LegislativeSupport,
                value: 0,
            },
        )
        .unwrap();
    world.advance(&content, DaySpan::new(14)).unwrap();
    assert_eq!(record(&world).stage, TreatyStage::RatificationFailed);
    assert!(world.is_developer_modified());
    assert_eq!(record(&world).administration_paid[&world.player], Money(20));
    exact(&world, &content);
}

#[test]
fn projections_and_typed_event_links_are_read_only_and_do_not_unlock_foreign_negotiations() {
    let (content, mut world) = fixture();
    offer(&mut world, &content, "BOR", 180, 30);
    let before = persistence::encode(&world, &content).unwrap();
    let own = owned(&content, &world);
    assert_eq!(
        own[0].owned.as_ref().unwrap().administration_paid,
        Money(10)
    );
    for viewer in [
        Viewer::Observer,
        Viewer::Government(CountryId("BOR".into())),
    ] {
        assert!(
            world
                .treaties_view(&content, viewer, &world.player)
                .unwrap()
                .is_empty()
        );
    }
    assert!(
        world
            .treaties_view(
                &content,
                Viewer::Government(world.player.clone()),
                &CountryId("BOR".into())
            )
            .unwrap()
            .is_empty()
    );
    let event = world
        .events
        .iter()
        .find(|event| {
            matches!(event.context, Some(EventContext::Treaty { .. }))
                && event.country == world.player
        })
        .unwrap()
        .id
        .as_ref()
        .unwrap();
    assert_eq!(
        world
            .event_treaty(&content, Viewer::Government(world.player.clone()), event)
            .unwrap(),
        Some(TreatyId(1))
    );
    assert!(
        world
            .event_treaty(&content, Viewer::Observer, event)
            .unwrap()
            .is_none()
    );
    let foreign = world
        .events
        .iter()
        .find(|event| {
            event.country == CountryId("BOR".into())
                && matches!(event.context, Some(EventContext::Treaty { .. }))
        })
        .unwrap()
        .id
        .as_ref()
        .unwrap();
    assert!(
        world
            .event_treaty(
                &content,
                Viewer::Government(CountryId("BOR".into())),
                foreign
            )
            .unwrap()
            .is_none()
    );
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    world.advance(&content, DaySpan::new(3)).unwrap();
    let public = world
        .treaties_view(&content, Viewer::Observer, &CountryId("BOR".into()))
        .unwrap();
    assert_eq!(public.len(), 1);
    assert!(public[0].owned.is_none());
    assert!(public[0].offer_deadline.is_none());
    assert!(
        public[0]
            .milestones
            .iter()
            .all(|milestone| milestone.day >= public[0].signed_on.unwrap())
    );
    exact(&world, &content);
}

#[test]
fn malformed_records_and_content_cannot_be_loaded_advanced_or_used_for_a_partial_command() {
    let (content, mut world) = fixture();
    activate(&content, &mut world, 180, 30);
    for case in 0..17 {
        let mut bad = world.clone();
        let state = bad.diplomacy.as_mut().unwrap();
        let record = state.agreements.get_mut(&TreatyId(1)).unwrap();
        match case {
            0 => state.next_id = TreatyId(u64::MAX),
            1 => record.id = TreatyId(2),
            2 => record.parties.reverse(),
            3 => record.terms.duration = DaySpan::new(29),
            4 => record.stage = TreatyStage::Scheduled,
            5 => record.signed_on = None,
            6 => {
                record.signatures.remove(&CountryId("BOR".into()));
            }
            7 => record.ratifications.get_mut(&bad.player).unwrap().review = CampaignDay::new(999),
            8 => record.ratifications.get_mut(&bad.player).unwrap().passed = Some(bad.day),
            9 => record.administration_paid.get_mut(&bad.player).unwrap().0 += 1,
            10 => record.milestones[0].id = TreatyMilestoneId(999),
            11 => record.next_milestone = TreatyMilestoneId(u32::MAX),
            12 => record.activation_day = Some(CampaignDay::new(29)),
            13 => record.closed_on = Some(bad.day),
            14 => record.milestones.last_mut().unwrap().command = Some(CommandId(1)),
            15 => bad.diplomacy = None,
            _ => record.offered_by = CountryId("COR".into()),
        }
        let before = serde_json::to_vec(&bad).unwrap();
        assert!(bad.validate(&content).is_err(), "case {case}");
        assert!(
            bad.treaties_view(
                &content,
                Viewer::Government(bad.player.clone()),
                &bad.player
            )
            .is_err(),
            "case {case}"
        );
        assert!(
            bad.issue(
                &content,
                Action::Diplomacy {
                    action: TreatyAction::Terminate {
                        agreement: TreatyId(1),
                        revision: TreatyRevision(1)
                    }
                }
            )
            .is_err(),
            "case {case}"
        );
        assert_eq!(serde_json::to_vec(&bad).unwrap(), before, "case {case}");
        assert!(
            bad.advance(&content, DaySpan::new(1)).is_err(),
            "case {case}"
        );
        assert_eq!(serde_json::to_vec(&bad).unwrap(), before, "case {case}");
    }
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut wrong: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    wrong["schema"] = 41.into();
    assert!(persistence::decode(&serde_json::to_vec(&wrong).unwrap(), &content).is_err());
    let mut invalid = content.clone();
    invalid.diplomacy.as_mut().unwrap().maximum_revisions = 17;
    assert!(invalid.validate().is_err());
    let mut invalid = content.clone();
    invalid.diplomacy = None;
    assert!(invalid.validate().is_err());
}

#[test]
fn event_exhaustion_rolls_back_both_participants_fees_records_and_allocators() {
    let (content, mut world) = fixture();
    let player = world.player.clone();
    world
        .event_sequences
        .as_mut()
        .unwrap()
        .insert(player.clone(), u64::MAX);
    // Artificial allocator boundary: retain a full, consistently numbered
    // history so the command reaches allocation, rather than malformed input.
    let template = world
        .events
        .iter()
        .find(|event| event.country == player)
        .unwrap()
        .clone();
    world.events = (0..400)
        .map(|index| {
            let mut event = template.clone();
            event.id.as_mut().unwrap().sequence = u64::MAX - 400 + index;
            event
        })
        .collect();
    let before = persistence::encode(&world, &content).unwrap();
    assert!(
        world
            .issue(
                &content,
                Action::Diplomacy {
                    action: TreatyAction::Offer {
                        with: CountryId("BOR".into()),
                        terms: terms(180, 30)
                    }
                }
            )
            .is_err()
    );
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
}

#[test]
fn archived_rules_remain_byte_compatible_and_definition_order_does_not_reorder_treaties() {
    let old = Content::business_finance_bundled().unwrap();
    let mut old_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(old_world.hash(), 0xb1a34e0e7596b9a9);
    let bytes = persistence::encode(&old_world, &old).unwrap();
    assert!(
        serde_json::to_value(&old_world)
            .unwrap()
            .get("diplomacy")
            .is_none()
    );
    assert!(
        serde_json::to_value(&old)
            .unwrap()
            .get("diplomacy")
            .is_none()
    );
    assert!(
        old_world
            .issue(
                &old,
                Action::Diplomacy {
                    action: TreatyAction::Offer {
                        with: CountryId("USA".into()),
                        terms: terms(180, 30)
                    }
                }
            )
            .is_err()
    );
    assert_eq!(persistence::encode(&old_world, &old).unwrap(), bytes);
    let (content, mut world) = fixture();
    let mut reordered = content.clone();
    reordered.countries.reverse();
    let mut other = World::new(&reordered, CountryId("AUR".into()), 17).unwrap();
    activate(&content, &mut world, 180, 30);
    activate(&reordered, &mut other, 180, 30);
    assert_eq!(world.diplomacy, other.diplomacy);
    exact(&other, &reordered);
}

#[test]
fn duplicate_pairs_and_real_archive_revision_limits_refuse_before_spending_or_allocating() {
    let (mut content, _) = fixture();
    content.diplomacy.as_mut().unwrap().maximum_records = 1;
    content.diplomacy.as_mut().unwrap().maximum_revisions = 2;
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    offer(&mut world, &content, "BOR", 730, 0);
    world.advance(&content, DaySpan::new(3)).unwrap();
    assert_eq!(record(&world).revision, TreatyRevision(2));
    let before = persistence::encode(&world, &content).unwrap();
    let counter = world
        .issue(
            &content,
            Action::Diplomacy {
                action: TreatyAction::Counteroffer {
                    agreement: TreatyId(1),
                    revision: TreatyRevision(2),
                    terms: terms(180, 30),
                },
            },
        )
        .unwrap_err();
    assert!(counter.contains("revision capacity"));
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    assert!(
        world
            .issue(
                &content,
                Action::Diplomacy {
                    action: TreatyAction::Offer {
                        with: CountryId("BOR".into()),
                        terms: terms(180, 30),
                    }
                }
            )
            .is_err()
    );
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    issue(
        &mut world,
        &content,
        TreatyAction::Withdraw {
            agreement: TreatyId(1),
            revision: TreatyRevision(2),
        },
    );
    world.advance(&content, DaySpan::new(30)).unwrap();
    let before = persistence::encode(&world, &content).unwrap();
    let exhausted = world
        .issue(
            &content,
            Action::Diplomacy {
                action: TreatyAction::Offer {
                    with: CountryId("BOR".into()),
                    terms: terms(180, 30),
                },
            },
        )
        .unwrap_err();
    assert!(exhausted.contains("archive capacity"));
    assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    assert_eq!(world.diplomacy.as_ref().unwrap().next_id, TreatyId(2));
    exact(&world, &content);
}
