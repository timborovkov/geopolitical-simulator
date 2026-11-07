use sim_core::{replay_trace::*, *};
use std::io::{self, Cursor, Write};

fn fixture() -> (Content, World) {
    let content = Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world
        .issue(&content, Action::Borrow { amount: Money(50) })
        .unwrap();
    world.advance(&content, DaySpan::new(2)).unwrap();
    world
        .issue_developer(
            &content,
            CountryId("BOR".into()),
            DeveloperEdit {
                field: DeveloperField::Tax,
                value: 35,
            },
        )
        .unwrap();
    world.advance(&content, DaySpan::new(2)).unwrap();
    (content, world)
}
fn trace(world: &World, content: &Content) -> Vec<u8> {
    let mut bytes = Vec::new();
    let summary = write_replay_trace(world, content, &mut bytes).unwrap();
    assert!(matches!(
        summary.footer.outcome,
        ReplayTraceOutcome::Complete { .. }
    ));
    bytes
}
fn records(bytes: &[u8]) -> Vec<ReplayTraceRecord> {
    String::from_utf8(bytes.to_vec())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn encode(records: &[ReplayTraceRecord]) -> Vec<u8> {
    records
        .iter()
        .flat_map(|record| {
            let mut bytes = serde_json::to_vec(record).unwrap();
            bytes.push(b'\n');
            bytes
        })
        .collect()
}
fn compare(a: &[u8], b: &[u8]) -> Result<ReplayTraceComparison, String> {
    compare_replay_traces(Cursor::new(a), Cursor::new(b))
}

#[test]
fn streaming_trace_covers_opening_daily_ticks_and_manual_origins_without_mutation() {
    let (content, world) = fixture();
    let before = serde_json::to_vec(&world).unwrap();
    let a = trace(&world, &content);
    assert_eq!(a, trace(&world, &content));
    assert_eq!(before, serde_json::to_vec(&world).unwrap());
    let comparison = compare(&a, &a).unwrap();
    assert_eq!(comparison.first_difference, None);
    assert_eq!(comparison.compared_points, 7); // Opening, four days, two explicit commands.
    assert_eq!(
        comparison.expected_outcome,
        ReplayTraceOutcome::Complete {
            final_hash: world.hash(),
            snapshot_matches: true
        }
    );
    assert_eq!(
        sim_core::persistence::replay(&world, &content)
            .unwrap()
            .hash(),
        world.hash()
    );
    let points: Vec<_> = records(&a)
        .into_iter()
        .filter_map(|record| match record {
            ReplayTraceRecord::Point { point } => Some(point),
            _ => None,
        })
        .collect();
    assert_eq!(points[0].boundary, ReplayBoundary::Opening);
    assert_eq!(
        points[1].boundary,
        ReplayBoundary::Command(world.journal[0].command.id)
    );
    assert!(
        points
            .iter()
            .any(|p| p.day == CampaignDay::new(2)
                && matches!(p.boundary, ReplayBoundary::Command(_)))
    );
    assert!(
        points
            .iter()
            .all(|p| p.components.iter().any(|c| c.component
                == ReplayComponent::Country {
                    country: CountryId("BOR".into()),
                    domain: ReplayDomain::Funds
                }))
    );
}

#[test]
fn comparison_finds_a_transient_first_boundary_even_when_final_snapshots_match() {
    let (content, world) = fixture();
    let a = trace(&world, &content);
    let mut altered = records(&a);
    let component = ReplayComponent::Country {
        country: CountryId("BOR".into()),
        domain: ReplayDomain::Funds,
    };
    let ReplayTraceRecord::Point { point } = &mut altered[3] else {
        panic!("daily point")
    };
    assert_eq!(point.boundary, ReplayBoundary::Day);
    point.world_hash ^= 1;
    point
        .components
        .iter_mut()
        .find(|entry| entry.component == component)
        .unwrap()
        .hash ^= 1;
    let comparison = compare(&a, &encode(&altered)).unwrap();
    let first = comparison.first_difference.unwrap();
    assert_eq!(first.sequence, 3);
    assert_eq!(first.expected.as_ref().unwrap().0, CampaignDay::new(1));
    assert_eq!(first.components, vec![component]);
    assert_eq!(comparison.expected_outcome, comparison.actual_outcome);
    assert_eq!(comparison.compared_points, 7);
    // A malformed tail must still fail after a legitimate early difference.
    altered.pop();
    assert!(
        compare(&a, &encode(&altered))
            .unwrap_err()
            .contains("footer")
    );
}

#[test]
fn trace_reader_rejects_incomplete_incompatible_and_malformed_evidence() {
    let (content, world) = fixture();
    let a = trace(&world, &content);
    let original = records(&a);
    for kind in 0..10 {
        let mut bad = original.clone();
        match kind {
            0 => {
                bad.pop();
            }
            1 => {
                bad.push(original[0].clone());
            }
            2 => {
                if let ReplayTraceRecord::Header { header } = &mut bad[0] {
                    header.version += 1;
                }
            }
            3 => {
                if let ReplayTraceRecord::Header { header } = &mut bad[0] {
                    header.identity.seed += 1;
                }
            }
            4 => {
                if let ReplayTraceRecord::Point { point } = &mut bad[1] {
                    point.sequence += 1;
                }
            }
            5 => {
                if let ReplayTraceRecord::Point { point } = &mut bad[3] {
                    point.day = CampaignDay::new(2);
                }
            }
            6 => {
                if let ReplayTraceRecord::Point { point } = &mut bad[1] {
                    point.components.reverse();
                }
            }
            7 => {
                if let ReplayTraceRecord::Footer { footer } = bad.last_mut().unwrap() {
                    footer.points += 1;
                }
            }
            8 => {
                if let ReplayTraceRecord::Footer { footer } = bad.last_mut().unwrap() {
                    footer.outcome = ReplayTraceOutcome::Complete {
                        final_hash: 0,
                        snapshot_matches: true,
                    };
                }
            }
            9 => {
                if let ReplayTraceRecord::Point { point } = &mut bad[2] {
                    point.boundary = ReplayBoundary::Command(CommandId(0));
                }
            }
            _ => unreachable!(),
        }
        assert!(compare(&a, &encode(&bad)).is_err(), "malformed case {kind}");
    }
    assert!(compare(&a, b"{broken}\n").is_err());
    assert!(
        compare(&a, &vec![b' '; 1024 * 1024 + 1])
            .unwrap_err()
            .contains("1 MiB")
    );
}

#[test]
fn snapshot_disagreement_and_failed_replay_are_distinct_from_execution_comparison() {
    let (content, world) = fixture();
    let a = trace(&world, &content);
    let mut changed = world.clone();
    changed
        .events
        .last_mut()
        .unwrap()
        .text
        .push_str(" inspected");
    changed.validate(&content).unwrap();
    assert!(sim_core::persistence::replay(&changed, &content).is_err());
    let b = trace(&changed, &content);
    let comparison = compare(&a, &b).unwrap();
    assert_eq!(comparison.first_difference, None);
    assert_eq!(
        comparison.actual_outcome,
        ReplayTraceOutcome::Complete {
            final_hash: world.hash(),
            snapshot_matches: false
        }
    );
    let mut rejected = world.clone();
    rejected.journal[0].command.action = Action::Repay {
        amount: Money(1_000_000_000_000),
    };
    rejected.validate(&content).unwrap();
    let before = serde_json::to_vec(&rejected).unwrap();
    let mut failed_bytes = Vec::new();
    let summary = write_replay_trace(&rejected, &content, &mut failed_bytes).unwrap();
    assert!(matches!(
        summary.footer.outcome,
        ReplayTraceOutcome::Failed { .. }
    ));
    assert_eq!(summary.footer.points, 1);
    assert_eq!(
        compare(&failed_bytes, &failed_bytes)
            .unwrap()
            .first_difference,
        None
    );
    assert_eq!(serde_json::to_vec(&rejected).unwrap(), before);
    // Simulate an engine reporting a failed replay after a validated prefix.
    let mut failed = records(&a);
    failed.truncate(4);
    failed.push(ReplayTraceRecord::Footer {
        footer: ReplayTraceFooter {
            points: 3,
            outcome: ReplayTraceOutcome::Failed {
                error: "recorded AI command divergence".into(),
            },
        },
    });
    let comparison = compare(&a, &encode(&failed)).unwrap();
    assert_eq!(comparison.first_difference.unwrap().sequence, 4);
    assert!(matches!(
        comparison.actual_outcome,
        ReplayTraceOutcome::Failed { .. }
    ));
}

#[test]
fn writer_failure_leaves_incomplete_evidence_and_original_campaign_untouched() {
    struct FailingWriter {
        bytes: Vec<u8>,
        remaining: usize,
    }
    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::other("deliberate output failure"));
            }
            let size = bytes.len().min(self.remaining);
            self.bytes.extend_from_slice(&bytes[..size]);
            self.remaining -= size;
            Ok(size)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let (content, world) = fixture();
    let before = serde_json::to_vec(&world).unwrap();
    let mut writer = FailingWriter {
        bytes: Vec::new(),
        remaining: 2000,
    };
    assert!(
        write_replay_trace(&world, &content, &mut writer)
            .unwrap_err()
            .contains("deliberate output failure")
    );
    assert_eq!(before, serde_json::to_vec(&world).unwrap());
    let good = trace(&world, &content);
    assert!(compare(&good, &writer.bytes).is_err());
}
