use sim_core::{persistence, *};
use std::{
    io::{self, Write},
    path::Path,
};

fn status(world: &World, content: &Content) {
    let c = world.player_country();
    let d = content.country(&world.player).expect("player");
    if world.is_developer_modified() {
        println!("DEVELOPER-MODIFIED CAMPAIGN · ordinary mandate certification is disabled.");
    }
    println!(
        "{} | {} | {} | hash {:016x}",
        date(world.day),
        d.name,
        c.executive,
        world.hash()
    );
    println!(
        "Cash {}m | debt {}m ({}% GDP) | arrears {}m | approval {}% | legislative {} | elite {}",
        c.accounts.treasury.0,
        c.debt.0,
        c.debt.0 * 100 / d.gdp,
        c.arrears.0,
        c.approval,
        c.legislative_support,
        c.elite_support
    );
    println!(
        "Tax {}% | services {}% GDP | welfare {}% GDP | energy {}% | food {}% | administration {}%",
        c.tax, c.services, c.welfare, c.energy_capacity, c.food_capacity, c.administration
    );
    if world.day == content.start_day().expect("validated bookmark") {
        println!("Opening state: advance one day for realized fiscal and supply flows.");
    } else {
        println!(
            "Last day: revenue {}m | spending {}m | interest {}m | energy delivered {}% | food delivered {}%",
            c.report.revenue.0,
            c.report.spending.0,
            c.report.interest.0,
            c.report.energy_coverage,
            c.report.food_coverage
        );
    }
    println!(
        "Reserved {}m | council/cadre backing {} | authority {}",
        c.committed_cash.0,
        c.council_support,
        d.government.label()
    );
    for p in &c.projects {
        println!(
            "Pending {} = {} · paid {}m · due {}",
            p.policy,
            p.value,
            p.cost.0,
            date(p.due_day)
        );
    }
}
fn number<T: std::str::FromStr>(text: Option<&&str>) -> Result<T, String> {
    text.ok_or("Missing numeric argument")?
        .parse()
        .map_err(|_| "Invalid number".into())
}
fn play(world: &mut World, content: &Content, developer: bool) -> Result<(), String> {
    println!(
        "Statecraft · {} · bookmark {}. Type help; data lists observation sources and authored assumptions.",
        content.name, content.start
    );
    if developer {
        println!(
            "DEVELOPER HOST ENABLED: dev COUNTRY FIELD VALUE bypasses policy costs/delays and marks the campaign. Type dev-fields for available bounded fields."
        );
    }
    status(world, content);
    loop {
        print!("statecraft> ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        if io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            break;
        }
        let words: Vec<_> = line.split_whitespace().collect();
        let result = (|| -> Result<(), String> {
            match words.first().copied().unwrap_or("") {
                "" => {}
                "quit" | "exit" => return Ok(()),
                "help" => {
                    println!(
                        "status | countries | policies | preview POLICY VALUE | enact POLICY VALUE\nadvance DAYS (max 3660) | until YYYY-MM-DD | next | borrow MILLIONS | repay MILLIONS\nhouseholds | budget | data | events [COUNTRY] | ledger | history | parties | save PATH | load PATH | replay | quit\nObjective at five years: energy/food ≥90%, approval ≥50%, no arrears, debt ≤opening GDP."
                    );
                    if developer {
                        println!(
                            "dev-fields | dev-inspect COUNTRY | dev COUNTRY FIELD VALUE (explicit developer edits)"
                        );
                    }
                }
                "dev-fields" if developer => {
                    for field in DeveloperField::ALL {
                        println!("{} · {:?}", field.name(), field.bounds());
                    }
                }
                "dev-inspect" if developer => {
                    let target = CountryId(words.get(1).ok_or("Missing country ID")?.to_string());
                    let country = world.countries.get(&target).ok_or("Unknown country")?;
                    println!(
                        "LIVE WORLD TRUTH · developer view; includes private foreign accounts."
                    );
                    println!(
                        "{}",
                        serde_json::to_string_pretty(country).map_err(|error| error.to_string())?
                    );
                }
                "dev" => {
                    if !developer {
                        return Err("Developer edits are disabled; restart with --developer to use the explicit developer host".into());
                    }
                    let target = CountryId(words.get(1).ok_or("Missing country ID")?.to_string());
                    let field =
                        DeveloperField::from_name(words.get(2).ok_or("Missing developer field")?)?;
                    let value = number(words.get(3))?;
                    world.issue_developer(content, target, DeveloperEdit { field, value })?;
                    println!(
                        "Developer edit accepted. Campaign permanently marked developer-modified."
                    );
                }
                "status" => status(world, content),
                "countries" => {
                    for d in &content.countries {
                        println!("{} · {} · {:?} · {}", d.id.0, d.name, d.regime, d.executive);
                    }
                }
                "policies" => {
                    for p in &content.policies {
                        println!(
                            "{} [{}..{}] · {} · {}",
                            p.id, p.min, p.max, p.name, p.description
                        );
                    }
                }
                "preview" | "enact" => {
                    let policy = words.get(1).ok_or("Missing policy ID")?.to_string();
                    let value = number(words.get(2))?;
                    if words[0] == "preview" {
                        println!(
                            "{:?}",
                            world.preview(content, &world.player, &policy, value)?
                        );
                    } else {
                        world.issue(content, Action::Enact { policy, value })?;
                        println!("Commitment accepted.");
                    }
                }
                "advance" => {
                    world.advance(content, number(words.get(1))?)?;
                    status(world, content);
                }
                "until" => {
                    let target = parse_date(words.get(1).ok_or("Missing date YYYY-MM-DD")?)?;
                    if target < world.day {
                        return Err("Cannot rewind; load an earlier snapshot".into());
                    }
                    while world.day < target {
                        world.advance(content, (target - world.day).min(3660))?;
                    }
                    status(world, content);
                }
                "next" => {
                    println!("Advanced {} days", world.advance_to_event(content)?);
                    status(world, content);
                }
                "borrow" => world.issue(
                    content,
                    Action::Borrow {
                        amount: Money(number(words.get(1))?),
                    },
                )?,
                "repay" => world.issue(
                    content,
                    Action::Repay {
                        amount: Money(number(words.get(1))?),
                    },
                )?,
                "events" => {
                    let id = words.get(1).map_or(world.player.0.as_str(), |s| *s);
                    if id == world.player.0 {
                        for e in world.events.iter().filter(|e| e.country == world.player) {
                            println!("{} · {}", date(e.day), e.text);
                        }
                    } else {
                        let report = world.public_report(content, &CountryId(id.into()))?;
                        println!(
                            "{} · {} · public information only; foreign government decisions and private accounts are excluded",
                            report.name, report.executive
                        );
                        if let Some(release) = report.latest {
                            println!(
                                "Published {}: approval {}%, needs {}%, debt {}m, daily output {}m",
                                date(release.day),
                                release.approval,
                                release.needs,
                                release.debt.0,
                                release.daily_output.0
                            );
                        } else {
                            println!("No statistical release yet");
                        }
                    }
                }
                "ledger" => {
                    for e in world.ledger.iter().filter(|e| e.country == world.player) {
                        println!(
                            "{} · {:?} → {:?} · {}m · {}",
                            date(e.day),
                            e.from,
                            e.to,
                            e.amount.0,
                            e.reason
                        );
                    }
                }
                "households" => {
                    let c = world.player_country();
                    let d = content.country(&world.player).expect("country");
                    if world.day == content.start_day()? {
                        println!(
                            "Opening state: advance one day for realized income and basket indicators."
                        );
                    } else {
                        let indicators = living_standards(&c.households)?;
                        let total = indicators.population.0.max(1) as f64;
                        println!(
                            "{} · population in groups with income below basket {:.1}% · population in groups with food below 90% {:.1}% · group-mean income Gini (0–1) {}",
                            date(world.day),
                            indicators.income_below_basket.0 as f64 * 100.0 / total,
                            indicators.food_below_target.0 as f64 * 100.0 / total,
                            indicators.group_mean_gini_bps.map_or_else(
                                || "undefined".into(),
                                |gini| format!("{:.3}", f64::from(gini) / 10000.0)
                            )
                        );
                        println!(
                            "Group means exclude within-group inequality. Income pressure is not a poverty headcount: savings can fund food; food supply can fail despite income."
                        );
                    }
                    for (h, definition) in c.households.iter().zip(&d.households) {
                        println!(
                            "{} · population {} · employed {}/{} · cash {}m · disposable {}m/day · basket {}m/day · food {}% · approval {}% · arrears {}m",
                            definition.name,
                            h.population.0,
                            h.employed.0,
                            h.working_age.0,
                            h.cash.0,
                            h.income.0 - h.tax.0,
                            h.basket_cost.0,
                            h.food_coverage,
                            h.approval,
                            h.arrears.0
                        );
                    }
                }
                "budget" => {
                    let c = world.player_country();
                    let p = &c.budget;
                    println!(
                        "Fiscal period {} → {}: opening cash {}m; forecast closing cash {}m",
                        date(p.start_day),
                        date(p.end_day),
                        p.opening_cash.0,
                        p.forecast_closing_cash()
                    );
                    println!(
                        "Revenue {}m vs {}m forecast to date; payments {}m, capital {}m, financing {}m, reconciled cash {}m; reserved {}m",
                        p.revenue.0,
                        p.expected_revenue(world.day),
                        p.payments.0,
                        p.capital.0,
                        p.borrowing.0 - p.principal_repaid.0,
                        p.expected_cash(),
                        c.committed_cash.0
                    );
                    for review in &c.budget_reviews {
                        println!(
                            "Closed {} · revenue {}m · payments {}m · closing cash {}m · arrears {}m",
                            date(review.period.end_day),
                            review.period.revenue.0,
                            review.period.payments.0,
                            review.closing_cash.0,
                            review.arrears.0
                        );
                    }
                }
                "data" => {
                    println!(
                        "Bookmark primary currency: {} (identity metadata; values use game reporting credits).",
                        content.country(&world.player).expect("player").currency
                    );
                    for f in content.facts.iter().filter(|f| f.country == world.player) {
                        println!(
                            "{}: {} {} ({}) → {} · {}@{} · {}",
                            f.field,
                            f.original_value,
                            f.original_units,
                            f.observation_period,
                            f.applied_value,
                            f.source,
                            f.source_version,
                            f.transformation
                        );
                    }
                    println!(
                        "Inspect each source and quality label; seed values without observations may use authored assumptions."
                    );
                    for office in content.bookmark_offices(&world.player) {
                        println!(
                            "Bookmark {} · {} · {}: {}",
                            content.start,
                            office.role.label(),
                            office.title,
                            office.holder
                        );
                        if let Some(term) = office.term {
                            println!(
                                "Reference from {} until {} (exclusive) · {:?} · source {}@{}; future records do not force campaign transitions.",
                                term.validity.valid_from,
                                term.validity
                                    .valid_to
                                    .map_or_else(|| "open-ended".into(), |date| date.to_string()),
                                term.quality,
                                term.source,
                                term.source_version
                            );
                        }
                    }
                }
                "history" => {
                    for o in &world.player_country().history {
                        println!(
                            "{} · cash {}m · debt {}m · approval {}% · needs {}%",
                            date(o.day),
                            o.treasury.0,
                            o.debt.0,
                            o.approval,
                            o.needs
                        );
                    }
                }
                "parties" => {
                    let d = content.country(&world.player).expect("player");
                    println!("{}", d.government.label());
                    for p in &d.parties {
                        println!(
                            "{} · {} · {} · seats {}",
                            p.id,
                            p.name,
                            p.ideology,
                            world.player_country().seats[&p.id]
                        );
                    }
                }
                "save" => {
                    persistence::save(
                        Path::new(words.get(1).ok_or("Missing path")?),
                        world,
                        content,
                    )?;
                    println!(
                        "Saved snapshot format {}. Compatible older primaries are archived before conversion.",
                        persistence::SAVE_SCHEMA
                    );
                }
                "load" => {
                    *world =
                        persistence::load(Path::new(words.get(1).ok_or("Missing path")?), content)?;
                    println!("Loaded.");
                }
                "replay" => {
                    persistence::replay(world, content)?;
                    println!("Replay hash verified.");
                }
                _ => return Err("Unknown command; type help".into()),
            }
            Ok(())
        })();
        if let Err(e) = result {
            println!("Rejected: {e}");
        }
        if matches!(words.first(), Some(&"quit" | &"exit")) {
            break;
        }
    }
    Ok(())
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut country = "GBR".to_string();
    let mut scenario = Scenario::Sandbox;
    let mut seed = 2000;
    let mut days = 360;
    let mut interactive = false;
    let mut verify = false;
    let mut developer = false;
    let mut save_path = None;
    let mut load_path = None;
    let mut content_path = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--interactive" => interactive = true,
            "--verify" => verify = true,
            "--developer" => developer = true,
            "--help" | "-h" => {
                println!(
                    "headless [--country GBR] [--scenario sandbox|crisis] [--seed 2000] [--days 360] [--interactive] [--verify] [--save FILE] [--load FILE] [--content FILE] [--developer]"
                );
                return Ok(());
            }
            flag @ ("--country" | "--seed" | "--days" | "--save" | "--load" | "--content"
            | "--scenario") => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("Missing value for {flag}"))?;
                match flag {
                    "--country" => country = value.clone(),
                    "--scenario" => {
                        scenario = match value.as_str() {
                            "sandbox" => Scenario::Sandbox,
                            "crisis" => Scenario::FiscalSupplyCrisis,
                            _ => return Err("Scenario must be sandbox or crisis".into()),
                        }
                    }
                    "--seed" => seed = value.parse().map_err(|_| "Invalid seed")?,
                    "--days" => days = value.parse().map_err(|_| "Invalid days")?,
                    "--save" => save_path = Some(value.clone()),
                    "--load" => load_path = Some(value.clone()),
                    "--content" => content_path = Some(value.clone()),
                    _ => unreachable!(),
                }
            }
            other => return Err(format!("Unknown option {other}")),
        }
        i += 1;
    }
    let content = if let Some(path) = content_path {
        Content::from_json(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)?
    } else {
        Content::bundled()?
    };
    let mut world = if let Some(path) = load_path {
        persistence::load(Path::new(&path), &content)?
    } else {
        World::new_scenario(&content, CountryId(country), seed, scenario)?
    };
    if interactive {
        play(&mut world, &content, developer)?;
    } else {
        world.advance(&content, days)?;
        status(&world, &content);
    }
    world.validate(&content)?;
    if verify {
        persistence::replay(&world, &content)?;
        println!("Deterministic replay verified.");
    }
    if let Some(path) = save_path {
        persistence::save(Path::new(&path), &world, &content)?;
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
