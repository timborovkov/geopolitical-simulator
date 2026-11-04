use sim_core::{persistence, *};
use std::{
    io::{self, Write},
    path::Path,
};

fn status(world: &World, content: &Content) {
    let c = world.player_country();
    let d = content.country(&world.player).expect("player");
    if let Some(markets) = &c.markets {
        println!(
            "Relative price markets · rules {} · opening index 1.00",
            world.rules
        );
        if let (Some(food), Some(energy)) = (&markets.food.last, &markets.energy.last) {
            println!(
                "Settlement {} prices: food {:.4}, energy {:.4}; next settlement: food {:.4}, energy {:.4}",
                date(food.day),
                f64::from(food.price_bps) / 10000.0,
                f64::from(energy.price_bps) / 10000.0,
                f64::from(markets.food.next_price_bps) / 10000.0,
                f64::from(markets.energy.next_price_bps) / 10000.0
            );
            println!(
                "Food physical need {} · affordable demand {} · supply {} · delivered {} · loss {} · closing stock {}",
                food.physical_need,
                food.requested_use,
                food.supply,
                food.delivered,
                food.loss,
                food.closing_stock
            );
        } else {
            println!("Awaiting first market settlement; opening price indices are references.");
        }
    }
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
        "{} {}% | services {}% GDP | welfare {}% GDP | energy {}% | food {}% | administration {}%",
        if content.income_taxes.is_some() {
            "Standard private-income marginal rate"
        } else {
            "Tax"
        },
        c.tax,
        c.services,
        c.welfare,
        c.energy_capacity,
        c.food_capacity,
        c.administration
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
        world
            .governing_rules(content, &world.player)
            .expect("government")
            .label()
    );
    for p in &c.projects {
        let paid = c
            .commitments
            .as_ref()
            .and_then(|records| records.iter().find(|r| r.id.0 == p.id))
            .map_or(p.cost.0 - p.reserved.0, |r| r.paid.0);
        println!(
            "Pending #{} · {} = {} · {} · paid {}m · reserved {}m · due {}",
            p.id,
            p.policy,
            p.value,
            p.stage.label(),
            paid,
            p.reserved.0,
            date(p.due_day)
        );
    }
}
fn number<T: std::str::FromStr>(text: Option<&&str>) -> Result<T, String> {
    text.ok_or("Missing numeric argument")?
        .parse()
        .map_err(|_| "Invalid number".into())
}
fn play(
    world: &mut World,
    content: &mut Content,
    developer: bool,
    bundled_catalog: bool,
) -> Result<(), String> {
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
                        "status | countries | policies | preview POLICY VALUE | enact POLICY VALUE\nadvance DAYS (max 3660) | until YYYY-MM-DD | next | borrow MILLIONS | repay MILLIONS\nincome-tax-bands | income-tax-report | preview-income-tax STANDARD_RATE\nfiscal-plans | preview-budget YEAR TAX SERVICES WELFARE | propose-budget YEAR TAX SERVICES WELFARE | withdraw-budget ID\nundertaking-templates | undertakings | preview-undertaking TEMPLATE [PROJECT] | declare-undertaking TEMPLATE [PROJECT] | withdraw-undertaking ID\nterritory | preview-territory FORM lawful|contested | reform-territory FORM lawful|contested | withdraw-territory ID | preview-requisition REGION MILLIONS | requisition REGION MILLIONS\nregions | preview-region-grant REGION BOOST DAYS | region-grant REGION BOOST DAYS | preview-region-services REGION LEVEL | region-services REGION LEVEL | cancel-region ID\nelections | preview-election | request-election | withdraw-election ID\ncabinet | appoint OFFICE PERSON | preview-appointment OFFICE PERSON | dismiss OFFICE | preview-dismissal OFFICE | cancel-appointment ID\nconstitutions | preview-constitution FORM lawful|contested | reconstitute FORM lawful|contested | withdraw-constitution ID\ncommitments | amend ID LEVEL | preview-amend ID LEVEL | withdraw ID | preview-withdraw ID\ncash-retention | preview-retention MILLIONS DAYS | retain-cash MILLIONS DAYS | release-cash ID\nfinance | outlook [DAYS] | preview-loan MILLIONS | emergency-borrow MILLIONS | cancel-loan ID\nhouseholds | budget | data | public-report COUNTRY | events [COUNTRY] | ledger | history | parties | save PATH | load PATH | replay | quit\nObjective at five years: energy/food ≥90%, approval ≥50%, no arrears, debt ≤opening GDP."
                    );
                    if content.finance.as_ref().is_some_and(|rules|rules.maturity_days.is_some()) {
                        println!("The five-year objective also requires zero overdue loan principal; inspect finance for contractual dates.");
                    }
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
                        let report = world.public_report(content, &d.id)?;
                        println!(
                            "{} · {} · {:?} · {}",
                            d.id.0, d.name, report.regime, report.executive
                        );
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
                        let mut preview = world.preview(content, &world.player, &policy, value)?;
                        if policy == "coalition" {
                            preview.route =
                                "Direct governing negotiation; backing changes after delivery"
                                    .into();
                        }
                        println!("{preview:?}");
                    } else {
                        world.issue(content, Action::Enact { policy, value })?;
                        println!("Commitment accepted.");
                    }
                }
                "commitments" => {
                    let records = world
                        .player_country()
                        .commitments
                        .as_ref()
                        .ok_or("This campaign has no commitment lifecycle")?;
                    for record in records {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(record).map_err(|e| e.to_string())?
                        );
                    }
                    println!(
                        "Direct delivered changes are recorded separately. Outcome reviews compare observed settlement days; other policies, prices and external shocks also affect them. They are not causal estimates."
                    );
                }
                "amend" | "preview-amend" => {
                    let request = CommitmentId(number(words.get(1))?);
                    let value = number(words.get(2))?;
                    let preview =
                        world.preview_amendment(content, &world.player, request, value)?;
                    if words[0] == "preview-amend" {
                        println!("{preview:?}");
                    } else {
                        world.issue(content, Action::AmendCommitment { request, value })?;
                    }
                }
                "withdraw" | "preview-withdraw" => {
                    let request = CommitmentId(number(words.get(1))?);
                    let preview = world.preview_cancellation(content, &world.player, request)?;
                    if words[0] == "preview-withdraw" {
                        println!("{preview:?}");
                    } else {
                        world.issue(content, Action::CancelCommitment { request })?;
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
                "income-tax-bands" => println!("{}",serde_json::to_string_pretty(content.income_taxes.as_ref().ok_or("This campaign uses the earlier flat private-income model")?).map_err(|e|e.to_string())?),
                "income-tax-report" => println!("{}",serde_json::to_string_pretty(world.player_country().income_tax_report.as_ref().ok_or("No realized band withholding; use a band-enabled campaign and advance one day")?).map_err(|e|e.to_string())?),
                "preview-income-tax" => println!("{:?}",world.preview_income_taxes(content,&world.player,number(words.get(1))?)?),
                "fiscal-plans" => println!(
                    "{}",
                    serde_json::to_string_pretty(
                        world
                            .player_country()
                            .fiscal_plans
                            .as_ref()
                            .ok_or("No fiscal-plan workflow")?
                    )
                    .map_err(|e| e.to_string())?
                ),
                "preview-budget" | "propose-budget" => {
                    let year = number(words.get(1))?;
                    let targets = FiscalTargets {
                        income_tax: number(words.get(2))?,
                        services: number(words.get(3))?,
                        welfare: number(words.get(4))?,
                    };
                    if words[0] == "preview-budget" {
                        println!(
                            "{:?}",
                            world.preview_budget(content, &world.player, year, &targets)?
                        );
                    } else {
                        world.issue(content, Action::ProposeBudget { year, targets })?;
                    }
                }
                "withdraw-budget" => world.issue(
                    content,
                    Action::WithdrawBudget {
                        request: BudgetBillId(number(words.get(1))?),
                    },
                )?,
                "undertaking-templates" => println!(
                    "{}",
                    serde_json::to_string_pretty(
                        content
                            .undertakings
                            .as_ref()
                            .ok_or("No undertaking workflow")?
                    )
                    .map_err(|e| e.to_string())?
                ),
                "undertakings" => println!(
                    "{}",
                    serde_json::to_string_pretty(
                        world
                            .player_country()
                            .undertakings
                            .as_ref()
                            .ok_or("No undertaking history")?
                    )
                    .map_err(|e| e.to_string())?
                ),
                "preview-undertaking" | "declare-undertaking" => {
                    let template = words
                        .get(1)
                        .ok_or("Missing template; use undertaking-templates")?
                        .to_string();
                    let project = words
                        .get(2)
                        .map(|v| {
                            v.parse::<u64>()
                                .map(CommitmentId)
                                .map_err(|_| "Invalid policy commitment ID")
                        })
                        .transpose()?;
                    if words[0] == "preview-undertaking" {
                        println!(
                            "{:?}",
                            world.preview_undertaking(
                                content,
                                &world.player,
                                &template,
                                project
                            )?
                        );
                    } else {
                        world.issue(content, Action::DeclareUndertaking { template, project })?;
                    }
                }
                "withdraw-undertaking" => world.issue(
                    content,
                    Action::WithdrawUndertaking {
                        request: UndertakingId(
                            words
                                .get(1)
                                .ok_or("Missing undertaking ID")?
                                .parse()
                                .map_err(|_| "Invalid undertaking ID")?,
                        ),
                    },
                )?,
                "territory" => {
                    let state = world
                        .player_country()
                        .territorial
                        .as_ref()
                        .ok_or("No territorial reform workflow")?;
                    println!(
                        "Current settlement {:?}",
                        world.territorial_form_at(content, &world.player, world.day)?
                    );
                    println!(
                        "{}",
                        serde_json::to_string_pretty(state).map_err(|e| e.to_string())?
                    );
                }
                "preview-territory" | "reform-territory" => {
                    let target: TerritorialForm =
                        serde_json::from_value(serde_json::Value::String(
                            words
                                .get(1)
                                .ok_or(
                                    "Use federation, decentralized_unitary or delegated_unitary",
                                )?
                                .to_string(),
                        ))
                        .map_err(|e| e.to_string())?;
                    let contested = match words.get(2).copied() {
                        Some("lawful") => false,
                        Some("contested") => true,
                        _ => return Err("Choose lawful or contested".into()),
                    };
                    if words[0] == "preview-territory" {
                        println!(
                            "{:?}",
                            world.preview_territorial_reform(
                                content,
                                &world.player,
                                &target,
                                contested
                            )?
                        );
                    } else {
                        world.issue(content, Action::ReformTerritory { target, contested })?;
                    }
                }
                "withdraw-territory" => world.issue(
                    content,
                    Action::WithdrawTerritorialReform {
                        request: TerritorialOrderId(
                            words
                                .get(1)
                                .ok_or("Missing request ID")?
                                .parse()
                                .map_err(|_| "Invalid request ID")?,
                        ),
                    },
                )?,
                "preview-requisition" | "requisition" => {
                    let region = RegionId(words.get(1).ok_or("Missing region")?.to_string());
                    let amount = Money(
                        words
                            .get(2)
                            .ok_or("Missing millions")?
                            .parse()
                            .map_err(|_| "Invalid millions")?,
                    );
                    if words[0] == "preview-requisition" {
                        println!(
                            "{:?}",
                            world.preview_regional_requisition(
                                content,
                                &world.player,
                                &region,
                                amount
                            )?
                        );
                    } else {
                        world
                            .issue(content, Action::RequisitionRegionalFunds { region, amount })?;
                    }
                }
                "regions" => {
                    let rules = content
                        .subnational
                        .as_ref()
                        .ok_or("This campaign has no subnational mandate workflow")?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            rules
                                .charter(&world.player)
                                .ok_or("Missing territorial charter")?
                        )
                        .map_err(|error| error.to_string())?
                    );
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            world
                                .player_country()
                                .subnational
                                .as_ref()
                                .ok_or("Missing local accounts")?
                        )
                        .map_err(|error| error.to_string())?
                    );
                    for original in &rules.charter(&world.player).expect("charter").regions {
                        println!(
                            "Current mandate: {:?}",
                            world.regional_mandate_at(
                                content,
                                &world.player,
                                &original.id,
                                world.day
                            )?
                        );
                    }
                    println!(
                        "Consolidated public cash {}m; protected local funds/escrow {}m; national uncommitted cash {}m. National government cannot reclaim accepted grants or order autonomous base service levels.",
                        world.player_country().accounts.treasury.0,
                        world.player_country().regional_protected_cash().0,
                        world.player_country().available_cash().0
                    );
                }
                "preview-region-grant" | "region-grant" => {
                    let region = RegionId(words.get(1).ok_or("Missing region ID")?.to_string());
                    let kind = RegionalOrderKind::Grant {
                        boost: number(words.get(2))?,
                        days: number(words.get(3))?,
                    };
                    if words[0] == "preview-region-grant" {
                        println!(
                            "{:?}",
                            world.preview_regional_order(content, &world.player, &region, &kind)?
                        );
                    } else {
                        world.issue(content, Action::RegionalOrder { region, kind })?;
                    }
                }
                "preview-region-services" | "region-services" => {
                    let region = RegionId(words.get(1).ok_or("Missing region ID")?.to_string());
                    let kind = RegionalOrderKind::Directive {
                        level: number(words.get(2))?,
                    };
                    if words[0] == "preview-region-services" {
                        println!(
                            "{:?}",
                            world.preview_regional_order(content, &world.player, &region, &kind)?
                        );
                    } else {
                        world.issue(content, Action::RegionalOrder { region, kind })?;
                    }
                }
                "cancel-region" => world.issue(
                    content,
                    Action::CancelRegionalOrder {
                        order: RegionalOrderId(number(words.get(1))?),
                    },
                )?,
                "elections" => {
                    let rules = content
                        .elections
                        .as_ref()
                        .ok_or("This campaign has no electoral authority workflow")?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            rules.mandate(&world.player).ok_or("Missing mandate")?
                        )
                        .map_err(|e| e.to_string())?
                    );
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            world
                                .player_country()
                                .electoral_authority
                                .as_ref()
                                .ok_or("Missing authority")?
                        )
                        .map_err(|e| e.to_string())?
                    );
                    println!(
                        "Announced polls cannot be withdrawn by government order. Model results use 100 seats; no turnout, national electoral law or integrity certification is modeled."
                    );
                }
                "preview-election" => {
                    println!("{:?}", world.preview_election(content, &world.player)?)
                }
                "request-election" => world.issue(content, Action::RequestElection)?,
                "withdraw-election" => world.issue(
                    content,
                    Action::WithdrawElectionRequest {
                        request: ElectionRequestId(number(words.get(1))?),
                    },
                )?,
                "cabinet" => {
                    let rules = content
                        .cabinet
                        .as_ref()
                        .ok_or("This campaign has no ministerial appointment rules")?;
                    let state = world
                        .player_country()
                        .cabinet
                        .as_ref()
                        .ok_or("Missing cabinet")?;
                    println!(
                        "Generated candidates and rules are authored; skill affects future preparation only."
                    );
                    for office in rules
                        .portfolios
                        .iter()
                        .filter(|p| p.country == world.player)
                    {
                        println!(
                            "{} · {} · holder {}",
                            office.id.0,
                            office.title,
                            state
                                .minister(&office.id)
                                .and_then(|t| rules.candidate(&t.person))
                                .map_or("Permanent civil service", |p| p.name.as_str())
                        );
                        for p in rules.candidates.iter().filter(|p| p.office == office.id) {
                            println!(
                                "  {} · {} · skill {} · political acceptability {}",
                                p.person.0, p.name, p.skill, p.acceptability
                            );
                        }
                    }
                    println!(
                        "{}",
                        serde_json::to_string_pretty(state).map_err(|e| e.to_string())?
                    );
                }
                "appoint" | "preview-appointment" => {
                    let office = OfficeId(words.get(1).ok_or("Missing portfolio ID")?.to_string());
                    let person = PersonId(words.get(2).ok_or("Missing candidate ID")?.to_string());
                    let preview =
                        world.preview_appointment(content, &world.player, &office, &person)?;
                    if words[0] == "appoint" {
                        world.issue(content, Action::Appoint { office, person })?;
                    } else {
                        println!("{preview:?}");
                    }
                }
                "dismiss" | "preview-dismissal" => {
                    let office = OfficeId(words.get(1).ok_or("Missing portfolio ID")?.to_string());
                    let fee = world.preview_dismissal(content, &world.player, &office)?;
                    if words[0] == "dismiss" {
                        world.issue(content, Action::DismissMinister { office })?;
                    } else {
                        println!(
                            "Dismissal administration {}m; civil-service preparation and a new appointment cooldown apply.",
                            fee.0
                        );
                    }
                }
                "cancel-appointment" => world.issue(
                    content,
                    Action::CancelAppointment {
                        request: AppointmentId(number(words.get(1))?),
                    },
                )?,
                "constitutions" => {
                    let rules = content
                        .constitutions
                        .as_ref()
                        .ok_or("This campaign has no constitutional transition workflow")?;
                    let state = world
                        .player_country()
                        .constitution
                        .as_ref()
                        .ok_or("Missing constitution")?;
                    println!(
                        "Current {:?} since {} · next request {}",
                        state.regime,
                        date(state.since),
                        date(state.next_request_day)
                    );
                    for template in &rules.templates {
                        println!(
                            "{} · {} · {}",
                            serde_json::to_value(&template.regime)
                                .map_err(|error| error.to_string())?
                                .as_str()
                                .unwrap_or("unknown"),
                            template.title,
                            template.government.label()
                        );
                    }
                    if let Some(request) = &state.pending {
                        println!(
                            "Pending #{} to {:?} · contested {} · decision {} · paid {}m, reserved {}m",
                            request.id.0,
                            request.target,
                            request.contested,
                            date(request.decision_day),
                            request.upfront.0,
                            request.reserved.0
                        );
                    }
                    for change in &state.history {
                        println!(
                            "{} · {:?} to {:?} · contested {} · government #{}",
                            date(change.day),
                            change.from,
                            change.to,
                            change.contested,
                            change.government_number
                        );
                    }
                }
                "preview-constitution" | "reconstitute" => {
                    let target: Regime = serde_json::from_value(serde_json::Value::String(
                        words
                            .get(1)
                            .ok_or("Missing governing form; use constitutions")?
                            .to_string(),
                    ))
                    .map_err(|_| "Unknown governing form; use constitutions")?;
                    let contested = match words.get(2).copied().unwrap_or("lawful") {
                        "lawful" => false,
                        "contested" => true,
                        _ => return Err("Choose lawful or contested explicitly".into()),
                    };
                    if words[0] == "preview-constitution" {
                        let preview = world.preview_constitution(
                            content,
                            &world.player,
                            &target,
                            contested,
                        )?;
                        println!(
                            "{:?} · contested {} · decision {} · {}m total: {}m upfront, {}m reserved · approval/legislature/elite/council costs {}/{}/{}/{} if enacted. Cancellation/refusal retains paid costs and cooldown.",
                            target,
                            contested,
                            date(preview.decision_day),
                            preview.cost.0,
                            preview.upfront.0,
                            preview.reserved.0,
                            preview.political_costs.approval_cost,
                            preview.political_costs.legislative_cost,
                            preview.political_costs.elite_cost,
                            preview.political_costs.council_cost
                        );
                    } else {
                        world.issue(content, Action::Reconstitute { target, contested })?;
                    }
                }
                "withdraw-constitution" => world.issue(
                    content,
                    Action::CancelConstitution {
                        request: TransitionId(number(words.get(1))?),
                    },
                )?,
                "outlook" => {
                    let horizon = if words.len() > 1 {
                        number(words.get(1))?
                    } else {
                        90_u32.min(MAX_DAY - world.day)
                    };
                    let loan = world
                        .player_country()
                        .financing
                        .as_ref()
                        .and_then(|state| state.pending.as_ref())
                        .map(|request| LoanAssumption {
                            amount: request.amount,
                            decision_day: request.decision_day,
                        });
                    let plan = world.funding_outlook(content, horizon, loan)?;
                    println!(
                        "Unchanged-policy funding scenario, not available money or an approval guarantee. Excludes shocks, price changes, arrears repayment and pending program delivery."
                    );
                    println!(
                        "Available cash {}m · balance {:+}m/day · deficit runway {}",
                        plan.available_cash.0,
                        plan.daily_balance.0,
                        plan.covered_days.map_or_else(
                            || "balanced or positive run rate".into(),
                            |days| format!("{days} fully covered days")
                        )
                    );
                    if world.player_country().cash_management.as_ref().is_some_and(|s|s.active().is_some()) { println!("Retained cash {}m excluded. Projection holds retention fixed; future capture and automatic release are not simulated.",world.player_country().retained_cash().0); }
                    if content.finance.as_ref().is_some_and(|rules|rules.maturity_days.is_some()) {
                        println!("Dated principal within horizon: {}m; conditional total {}m. Subtracts full due principal (existing overdue principal from tomorrow); recurring interest is held fixed after repayment, conservatively overstating it. No automatic refinancing.",plan.principal_due.0,plan.conditional_principal_due.0);
                    }
                    for point in plan.points.iter().filter(|point| {
                        point.day == world.day
                            || point.day == world.day + horizon
                            || loan.is_some_and(|loan| point.day == loan.decision_day)
                    }) {
                        println!(
                            "{}: without borrowing {}m · conditional approval {}m (negative = funding gap)",
                            date(point.day),
                            point.without_loan.0,
                            point.if_approved.0
                        );
                    }
                    if let Some(loan) = loan {
                        println!(
                            "Pending {}m on {} IF approved · additional interest {}m/day",
                            loan.amount.0,
                            date(loan.decision_day),
                            plan.additional_daily_interest.0
                        );
                    }
                }
                "cash-retention" => {
                    let c = world.player_country();
                    let state = c.cash_management.as_ref().ok_or("This campaign has no temporary cash retention")?;
                    println!("Actual retained cash {}m; spendable {}m; target is not money. Deferring operating payments creates arrears; matured principal remains due. Release before funding a normal decision.", c.retained_cash().0, c.available_cash().0);
                    println!("{}", serde_json::to_string_pretty(state).map_err(|e|e.to_string())?);
                }
                "preview-retention" => {
                    let preview = world.preview_cash_hold(content, &world.player, Money(number(words.get(1))?), number(words.get(2))?)?;
                    println!("Target {}m; actual protected now {}m; approval -{}; expiry {}; reuse {}. Unfilled target is not cash. Wages, welfare and interest may become arrears; matured principal may remain unpaid. Local claims and project reservations remain protected.", preview.target.0, preview.retained_now.0, preview.approval_cost, date(preview.expiry_day), date(preview.next_allowed_day));
                }
                "retain-cash" => world.issue(content, Action::HoldCash { target: Money(number(words.get(1))?), days: number(words.get(2))? })?,
                "release-cash" => world.issue(content, Action::ReleaseCash { request: CashHoldId(number(words.get(1))?) })?,
                "finance" => {
                    if let Some(financing) = &world.player_country().financing {
                        println!("Borrowing authority · pending funds are not treasury cash");
                        if let Some(request) = &financing.pending {
                            println!(
                                "Pending #{}: {}m via {}, decision {} · cancel-loan {}",
                                request.id.0,
                                request.amount.0,
                                request.authority.0,
                                date(request.decision_day),
                                request.id.0
                            );
                        }
                        println!("Principal due/overdue now {}m; including next 30 days {}m. Outstanding matured principal remains debt, not duplicated operating arrears.",financing.principal_due_through(world.day).0,financing.principal_due_through(world.day.saturating_add(30)).0);
                        for loan in &financing.instruments {
                            println!(
                                "Debt #{} · {} · {:?} · original {}m · outstanding {}m · recognized/issued {} · fixed 4% annual, voluntary FIFO redemption · {:?}",
                                loan.id.0,
                                loan.creditor.0,
                                loan.currency,
                                loan.original_principal.0,
                                loan.outstanding.0,
                                date(loan.issued_day),
                                loan.origin
                            );
                            if let Some(day) = loan.maturity_day {
                                println!("  Principal due {} · automatic repayment {}m · remaining {}m{}",date(day),loan.maturity_repaid.unwrap_or(Money(0)).0,loan.outstanding.0,if day <= world.day && loan.outstanding.0 > 0 {" OVERDUE"} else {""});
                            } else {println!("  Perpetual principal, no mandatory maturity date.");}
                        }
                        println!(
                            "Emergency acts {} · next eligible {} (backing and credit checks still apply)",
                            financing.emergency_count,
                            date(financing.next_emergency_day)
                        );
                    } else {
                        println!(
                            "This campaign uses immediate legacy financing; financing authority is available in new rules-4 campaigns."
                        );
                    }
                }
                "preview-loan" => {
                    let amount = Money(number(words.get(1))?);
                    match world.preview_borrowing(content, &world.player, amount) {
                        Ok(preview) => println!(
                            "{}m via {} · decision {} · consent {} days · authority and creditor funds rechecked before cash is released",
                            amount.0,
                            preview.route,
                            date(preview.decision_day),
                            preview.consent_days
                        ),
                        Err(reason) => println!("Ordinary borrowing unavailable: {reason}"),
                    }
                    if let Some(term) = content.finance.as_ref().and_then(|rules|rules.maturity_days) {
                        println!("New principal matures {term} days after actual disbursement. Separate refinancing requires authority and finite creditor funds.");
                    }
                    match world.preview_emergency_borrowing(content, &world.player, amount) {
                        Ok(rule) => println!(
                            "Contested emergency route: immediate cash; approval -{}, legislature -{}, elite -{}, council -{}, cooldown {} days. Use emergency-borrow explicitly.",
                            rule.approval_cost,
                            rule.legislative_cost,
                            rule.elite_cost,
                            rule.council_cost,
                            rule.cooldown_days
                        ),
                        Err(reason) => println!("Emergency route unavailable: {reason}"),
                    }
                }
                "emergency-borrow" => world.issue(
                    content,
                    Action::EmergencyBorrow {
                        amount: Money(number(words.get(1))?),
                    },
                )?,
                "cancel-loan" => world.issue(
                    content,
                    Action::CancelBorrow {
                        request: LoanId(
                            number::<i64>(words.get(1))?
                                .try_into()
                                .map_err(|_| "Loan ID must be positive")?,
                        ),
                    },
                )?,
                "repay" => world.issue(
                    content,
                    Action::Repay {
                        amount: Money(number(words.get(1))?),
                    },
                )?,
                "public-report" => {
                    let id=CountryId((*words.get(1).ok_or("Usage: public-report COUNTRY")?).into());
                    let report=world.public_report(content,&id)?;
                    println!("{}",serde_json::to_string_pretty(&report).map_err(|error|error.to_string())?);
                }
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
                    let d = content.country(&world.player).expect("player");
                    println!(
                        "Bookmark primary currency: {} (identity metadata; values use game reporting credits).",
                        content.country(&world.player).expect("player").currency
                    );
                    for (field, value) in [("population", d.population as i64), ("gdp", d.gdp)] {
                        if !content
                            .facts
                            .iter()
                            .any(|fact| fact.country == world.player && fact.field == field)
                        {
                            println!(
                                "{field}: {value} · authored calibration; no observation record in this pack."
                            );
                        }
                    }
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
                    println!("{}", world.governing_rules(content, &world.player)?.label());
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
                        persistence::snapshot_schema(world)
                    );
                }
                "load" => {
                    let path = Path::new(words.get(1).ok_or("Missing path")?);
                    if bundled_catalog {
                        let (resolved, loaded) = persistence::load_bundled(path)?;
                        *content = resolved;
                        *world = loaded;
                    } else {
                        *world = persistence::load(path, content)?;
                    }
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
    let bundled_catalog = content_path.is_none();
    let mut content = if let Some(path) = content_path {
        Content::from_json(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)?
    } else {
        Content::bundled()?
    };
    let mut world = if let Some(path) = load_path {
        if bundled_catalog {
            let (resolved, loaded) = persistence::load_bundled(Path::new(&path))?;
            content = resolved;
            loaded
        } else {
            persistence::load(Path::new(&path), &content)?
        }
    } else {
        World::new_scenario(&content, CountryId(country), seed, scenario)?
    };
    if interactive {
        play(&mut world, &mut content, developer, bundled_catalog)?;
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
