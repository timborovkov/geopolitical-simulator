use sim_core::*;

fn count(words: &[&str], expected: usize, usage: &str) -> Result<(), String> {
    if words.len() == expected {
        Ok(())
    } else {
        Err(format!("Usage: {usage}"))
    }
}
fn number(value: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| "Use a nonnegative whole day count".into())
}
fn reference(id: &str, revision: &str) -> Result<(TreatyId, TreatyRevision), String> {
    let id = id
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or("Use a positive pact ID")?;
    let revision = revision
        .parse::<u32>()
        .ok()
        .filter(|revision| *revision > 0)
        .ok_or("Use the positive current revision")?;
    Ok((TreatyId(id), TreatyRevision(revision)))
}
fn action(words: &[&str]) -> Result<TreatyAction, String> {
    match words.first().copied() {
        Some("offer") => {
            count(words, 4, "offer COUNTRY DURATION_DAYS NOTICE_DAYS")?;
            Ok(TreatyAction::Offer {
                with: CountryId(words[1].into()),
                terms: NonAggressionTerms {
                    duration: DaySpan::new(number(words[2])?),
                    notice: DaySpan::new(number(words[3])?),
                },
            })
        }
        Some("counter") => {
            count(words, 5, "counter ID REVISION DURATION_DAYS NOTICE_DAYS")?;
            let (agreement, revision) = reference(words[1], words[2])?;
            Ok(TreatyAction::Counteroffer {
                agreement,
                revision,
                terms: NonAggressionTerms {
                    duration: DaySpan::new(number(words[3])?),
                    notice: DaySpan::new(number(words[4])?),
                },
            })
        }
        Some(kind @ ("sign" | "ratify" | "reject" | "withdraw" | "notice")) => {
            count(words, 3, "sign|ratify|reject|withdraw|notice ID REVISION")?;
            let (agreement, revision) = reference(words[1], words[2])?;
            Ok(match kind {
                "sign" => TreatyAction::Sign {
                    agreement,
                    revision,
                },
                "ratify" => TreatyAction::Ratify {
                    agreement,
                    revision,
                },
                "reject" => TreatyAction::Reject {
                    agreement,
                    revision,
                },
                "withdraw" => TreatyAction::Withdraw {
                    agreement,
                    revision,
                },
                _ => TreatyAction::Terminate {
                    agreement,
                    revision,
                },
            })
        }
        _ => Err("Choose offer, counter, sign, ratify, reject, withdraw or notice".into()),
    }
}

pub fn handle(world: &mut World, content: &Content, words: &[&str]) -> Result<(), String> {
    if words[0] == "treaties" {
        if words.len() > 2 {
            return Err("Usage: treaties [COUNTRY]".into());
        }
        let country = words
            .get(1)
            .map_or_else(|| world.player.clone(), |value| CountryId((*value).into()));
        let viewer = if country == world.player {
            Viewer::Government(world.player.clone())
        } else {
            Viewer::Observer
        };
        let records = world.treaties_view(content, viewer, &country)?;
        println!("Bilateral agreements · {} · {}", country.0, date(world.day));
        println!(
            "This exercise records non-aggression promises and procedures. Military enforcement is not available."
        );
        if records.is_empty() {
            println!(
                "No agreements visible to this view. Unaccepted offers are private to their participants."
            );
        }
        for record in records {
            println!(
                "Pact #{} revision {} · {} / {} · {}",
                record.id.0,
                record.revision.0,
                record.parties[0].0,
                record.parties[1].0,
                record.stage.label()
            );
            println!(
                "Duration {} days from activation · notice {} days · current offering party {}",
                record.terms.duration.units(),
                record.terms.notice.units(),
                record.offered_by.0
            );
            if let Some(deadline) = record.offer_deadline {
                println!("Respond before {}", date(deadline));
            }
            if record.stage == TreatyStage::AwaitingRatification
                && let Some(deadline) = record.ratification_submission_deadline
            {
                println!(
                    "Submit each domestic ratification before {}; on-time scheduled reviews may finish later",
                    date(deadline)
                );
            }
            if let Some(day) = record.signed_on {
                println!("Both signatures recorded {}", date(day));
            }
            if let Some(day) = record.activation_day {
                println!(
                    "Scheduled activation {} · actually activated {}",
                    date(day),
                    record
                        .activated_on
                        .map(date)
                        .unwrap_or_else(|| "not activated".into())
                );
            }
            if let Some(day) = record.expires_at {
                println!("Natural expiry {} (end excluded)", date(day));
            }
            if let Some(notice) = &record.notice {
                println!(
                    "Notice by {} on {} · operative until {}",
                    notice.by.0,
                    date(notice.day),
                    date(notice.until)
                );
            }
            if let Some(day) = record.closed_on {
                println!("Closed {}", date(day));
            }
            if record.signed_on.is_some() {
                for request in &record.ratifications {
                    println!(
                        "{} domestic ratification: {}",
                        request.country.0,
                        request
                            .passed
                            .map(|day| format!("passed {}", date(day)))
                            .or_else(|| {
                                if record.stage.closed() {
                                    Some("not ratified before closure".into())
                                } else {
                                    request
                                        .review
                                        .map(|day| format!("submitted; review {}", date(day)))
                                }
                            })
                            .unwrap_or_else(|| "not submitted".into())
                    );
                }
            }
            if let Some(own) = &record.owned {
                println!(
                    "Your paid administration {}m · charge per new paid submission {}m · available national cash {}m",
                    own.administration_paid.0, own.service_charge.0, own.available_national_cash.0
                );
                println!(
                    "Your current authority: {}. Paid charges are retained after withdrawal, refusal or lapse.",
                    own.authority
                );
            }
            for milestone in record.milestones.iter().rev().take(12).rev() {
                println!(
                    "  {} · revision {} · {}{}",
                    date(milestone.day),
                    milestone.revision.0,
                    milestone.outcome.label(),
                    milestone
                        .actor
                        .as_ref()
                        .map(|actor| format!(" · {}", actor.0))
                        .unwrap_or_default()
                );
            }
        }
        return Ok(());
    }
    let preview = words[0] == "pact-preview";
    let action = if preview {
        action(&words[1..])?
    } else {
        let kind = words[0]
            .strip_prefix("pact-")
            .ok_or("Unknown pact command")?;
        let mut request = vec![kind];
        request.extend_from_slice(&words[1..]);
        action(&request)?
    };
    if preview {
        let plan = world.preview_treaty(content, &action)?;
        println!(
            "{} · pact #{} revision {}",
            action.label(),
            plan.agreement.0,
            plan.revision.0
        );
        println!(
            "Immediate administration {}m · {}",
            plan.immediate_cost.0, plan.authority
        );
        println!(
            "After this command: {} · duration {} days · notice {} days",
            plan.stage_after_command.label(),
            plan.terms.duration.units(),
            plan.terms.notice.units()
        );
        if let Some(day) = plan.ratification_review {
            println!("Conditional domestic review {}", date(day));
        }
        println!("{}", plan.explanation);
    } else {
        super::issue(world, content, Action::Diplomacy { action })?;
    }
    Ok(())
}
