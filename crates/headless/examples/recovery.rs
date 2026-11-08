//! Reproducible crisis playthrough using ordinary, validated government actions.
//! This is a worked strategy, not a simulation bypass or a universal optimal AI.
use sim_core::*;
fn propose(world: &mut World, content: &Content, policy: &str, value: i32) {
    if world
        .preview(content, &world.player, &policy.into(), value)
        .is_ok()
    {
        world
            .issue(
                content,
                Action::Enact {
                    policy: policy.into(),
                    value,
                },
            )
            .expect("revalidated example action");
    }
}
fn play(
    content: &Content,
    country: &str,
    seed: u64,
    contested_transition: bool,
) -> Result<(), String> {
    let id = CountryId(country.into());
    let definition = content.country(&id).ok_or("country")?;
    let mut world = World::new_scenario(content, id, seed, Scenario::FiscalSupplyCrisis)?;
    if contested_transition {
        let rules = content
            .constitutions
            .as_ref()
            .ok_or("--contested-transition requires a constitutional-transition content pack")?;
        let target = if definition.regime == Regime::Military {
            Regime::Personalist
        } else {
            Regime::Military
        };
        let mut loans = 0;
        for _ in 0..3 {
            let amount = Money(definition.gdp.units() / 10);
            if loans < 2
                && world
                    .preview_borrowing(content, &world.player, amount)
                    .is_ok()
            {
                world.issue(content, Action::Borrow { amount })?;
                loans += 1;
            }
            if world
                .preview(content, &world.player, &"coalition".into(), 1)
                .is_err()
                && let Some(pending) = world
                    .player_country()
                    .financing
                    .as_ref()
                    .and_then(|state| state.pending.as_ref())
            {
                world.advance(
                    content,
                    sim_core::DaySpan::new((pending.decision_day).units() - world.day.units()),
                )?;
            }
            let preview = world.preview(content, &world.player, &"coalition".into(), 1)?;
            world.issue(
                content,
                Action::Enact {
                    policy: "coalition".into(),
                    value: 1,
                },
            )?;
            world.advance(content, preview.delay)?;
        }
        // Minority cabinets may only gain borrowing authority after talks.
        // Accepted requests still supply no cash until their actual decision.
        while loans < 2 {
            let amount = Money(definition.gdp.units() / 10);
            let preview = world.preview_borrowing(content, &world.player, amount)?;
            world.issue(content, Action::Borrow { amount })?;
            loans += 1;
            world.advance(content, preview.consent_days)?;
        }
        // Short dated loans can mature during the support negotiations. Obtain
        // any needed replacement cash through the same ordinary consent route.
        if content
            .finance
            .as_ref()
            .is_some_and(|finance| finance.maturity_days.is_some())
        {
            let cost =
                (definition.gdp.units() * rules.route(true)?.cost_bps.units_i64() / 10000).max(1);
            for _ in 0..3 {
                if world.player_country().available_cash().0 >= cost {
                    break;
                }
                let amount = Money(definition.gdp.units() / 10);
                let preview = world.preview_borrowing(content, &world.player, amount)?;
                world.issue(content, Action::Borrow { amount })?;
                world.advance(content, preview.consent_days)?;
            }
        }
        world.issue(
            content,
            Action::Reconstitute {
                target: target.clone(),
                contested: true,
            },
        )?;
        world.advance(content, rules.route(true)?.days)?;
        if world.governing_regime(content, &world.player)? != &target
            || world.is_developer_modified()
        {
            return Err("Contested transition did not enact as an ordinary game action".into());
        }
        println!(
            "{country}: {:?} to {:?} through a costed contested route; country continues on {} without developer edits",
            definition.regime,
            target,
            date(world.day)
        );
    }
    let review_day = content.review_day()?;
    while world.day.units() < (review_day).units() {
        let c = world.player_country();
        let gap = -world.funding_outlook(content, 0, None)?.daily_balance.0;
        let upcoming_principal = c.financing.as_ref().map_or(0, |state| {
            state
                .principal_due_through(sim_core::CampaignDay::new(
                    world.day.units().saturating_add(90),
                ))
                .0
        });
        if c.available_cash().0 < gap.max(1) * 90 + upcoming_principal {
            let amount = Money(definition.gdp.units() / 10);
            if c.debt.0 + amount.0 <= definition.gdp.units() * 3 / 2
                && c.accounts.external.0 >= amount.0
                && (content.finance.is_none()
                    || world
                        .preview_borrowing(content, &world.player, amount)
                        .is_ok())
            {
                world.issue(content, Action::Borrow { amount })?;
            }
        }
        let c = world.player_country();
        let government = world.governing_rules(content, &world.player)?;
        let support = if world
            .governing_regime(content, &world.player)?
            .competitive()
        {
            c.legislative_support
        } else {
            c.elite_support
        };
        if support < government.threshold + 5
            || c.council_support < government.council_threshold + 5
        {
            propose(&mut world, content, "coalition", 1);
        }
        // Fund essential services at an affordable level while widening receipts.
        for (policy, value) in [
            (
                "income_tax",
                if content.income_taxes.is_some() {
                    40
                } else {
                    35
                },
            ),
            (
                "services",
                if content
                    .finance
                    .as_ref()
                    .is_some_and(|rules| rules.maturity_days.is_some())
                {
                    6
                } else {
                    8
                },
            ),
            (
                "welfare",
                if content
                    .finance
                    .as_ref()
                    .is_some_and(|rules| rules.maturity_days.is_some())
                {
                    6
                } else {
                    8
                },
            ),
        ] {
            propose(&mut world, content, policy, value);
        }
        for (policy, needed) in [
            ("energy", world.player_country().energy_capacity < 110),
            ("agriculture", world.player_country().food_capacity < 100),
            ("administration", world.player_country().administration < 85),
        ] {
            if needed {
                propose(&mut world, content, policy, 1);
            }
        }
        let c = world.player_country();
        // Hold cash for dated principal instead of voluntarily retiring perpetual
        // calibration debt with cash needed for a near-term contractual payment.
        let scheduled_principal = c.financing.as_ref().map_or(0, |state| {
            state
                .principal_due_through(sim_core::CampaignDay::new(MAX_DAY))
                .0
        });
        if c.available_cash().0 > definition.gdp.units() / 8 + scheduled_principal
            && c.debt.0 > definition.gdp.units() / 5
        {
            let amount = Money((definition.gdp.units() / 20).min(c.debt.0));
            world.issue(content, Action::Repay { amount })?;
        }
        world.advance(
            content,
            sim_core::DaySpan::new(((review_day).units() - world.day.units()).min(30)),
        )?;
    }
    world.validate(content)?;
    persistence::replay(&world, content)?;
    let c = world.player_country();
    if c.report.energy_coverage < 90
        || c.report.food_coverage < 90
        || c.approval < 50
        || c.arrears.0 != 0
        || c.financing
            .as_ref()
            .is_some_and(|state| state.principal_due_through(world.day).0 > 0)
        || c.debt.0 > definition.gdp.units()
    {
        return Err(format!(
            "{country} recovery failed: energy {}%, food {}%, approval {}%, arrears {}m, debt {}m",
            c.report.energy_coverage, c.report.food_coverage, c.approval, c.arrears.0, c.debt.0
        ));
    }
    println!(
        "{country} seed {seed}: five-year mandate achieved · energy {}% · food {}% · approval {}% · debt {}% GDP · no arrears · replay {:016x}",
        c.report.energy_coverage,
        c.report.food_coverage,
        c.approval,
        c.debt.0 * 100 / definition.gdp.units(),
        world.hash()
    );
    Ok(())
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut content_path = None;
    let mut contested_transition = false;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--content" if content_path.is_none() => {
                content_path = Some(arguments.next().ok_or("Missing content path")?);
            }
            "--contested-transition" if !contested_transition => {
                contested_transition = true;
            }
            _ => return Err("Usage: recovery [--content FILE] [--contested-transition]".into()),
        }
    }
    let content = if let Some(path) = content_path {
        Content::from_json(&std::fs::read_to_string(path).map_err(|error| error.to_string())?)?
    } else {
        Content::recommended_bundled()?
    };
    for definition in &content.countries {
        let country = definition.id.0.as_str();
        play(&content, country, 17, contested_transition)?;
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
