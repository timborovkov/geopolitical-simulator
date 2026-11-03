//! Reproducible crisis playthrough using ordinary, validated government actions.
//! This is a worked strategy, not a simulation bypass or a universal optimal AI.
use sim_core::*;
fn propose(world: &mut World, content: &Content, policy: &str, value: i32) {
    if world.preview(content, &world.player, policy, value).is_ok() {
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
fn play(content: &Content, country: &str, seed: u64) -> Result<(), String> {
    let id = CountryId(country.into());
    let definition = content.country(&id).ok_or("country")?;
    let mut world = World::new_scenario(content, id, seed, Scenario::FiscalSupplyCrisis)?;
    let review_day = content.review_day()?;
    while world.day < review_day {
        let c = world.player_country();
        let forecast = c.forecast(definition);
        let gap = forecast.daily_spending.0 + forecast.daily_interest.0 - forecast.daily_revenue.0;
        if c.accounts.treasury.0 - c.committed_cash.0 < gap.max(1) * 90 {
            let amount = Money(definition.gdp / 10);
            if c.debt.0 + amount.0 <= definition.gdp * 3 / 2
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
        let support = if definition.regime.competitive() {
            c.legislative_support
        } else {
            c.elite_support
        };
        if support < definition.government.threshold + 5
            || c.council_support < definition.government.council_threshold + 5
        {
            propose(&mut world, content, "coalition", 1);
        }
        // Fund essential services at an affordable level while widening receipts.
        for (policy, value) in [("income_tax", 35), ("services", 8), ("welfare", 8)] {
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
        if c.accounts.treasury.0 - c.committed_cash.0 > definition.gdp / 8
            && c.debt.0 > definition.gdp / 5
        {
            let amount = Money((definition.gdp / 20).min(c.debt.0));
            world.issue(content, Action::Repay { amount })?;
        }
        world.advance(content, (review_day - world.day).min(30))?;
    }
    world.validate(content)?;
    persistence::replay(&world, content)?;
    let c = world.player_country();
    if c.report.energy_coverage < 90
        || c.report.food_coverage < 90
        || c.approval < 50
        || c.arrears.0 != 0
        || c.debt.0 > definition.gdp
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
        c.debt.0 * 100 / definition.gdp,
        world.hash()
    );
    Ok(())
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let content = match args.as_slice() {
        [] => Content::bundled()?,
        [flag, path] if flag == "--content" => {
            Content::from_json(&std::fs::read_to_string(path).map_err(|error| error.to_string())?)?
        }
        _ => return Err("Usage: recovery [--content FILE]".into()),
    };
    for definition in &content.countries {
        let country = definition.id.0.as_str();
        play(&content, country, 17)?;
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
