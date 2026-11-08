use sim_core::{persistence, *};
use std::{
    io::{self, Write},
    path::Path,
};

fn issue(world: &mut World, content: &Content, action: Action) -> Result<(), String> {
    let receipt = world.issue_with_receipt(content, action)?;
    println!("Command #{} accepted.", receipt.command);
    for event in receipt.outcomes {
        println!("Outcome {event} · inspect: event {event}");
    }
    Ok(())
}

fn print_event(event: &Event) {
    if let Some(id) = &event.id {
        println!("{id} · {} · {}", date(event.day), event.text);
    } else {
        println!("{} · {}", date(event.day), event.text);
    }
    if let Some(EventContext::Commitment { request, stage }) = event.context {
        println!("Programme #{request} · recorded stage {stage:?} · inspect: commitment {request}");
    }
}

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
                f64::from(food.price_bps.units()) / 10000.0,
                f64::from(energy.price_bps.units()) / 10000.0,
                f64::from(markets.food.next_price_bps.units()) / 10000.0,
                f64::from(markets.energy.next_price_bps.units()) / 10000.0
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
        c.debt.0 * 100 / d.gdp.units(),
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
            .and_then(|records| records.iter().find(|r| r.id == p.id))
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
                        "social-groups [COUNTRY [GROUP...]] | population [COUNTRY] | institutions | command-context ID | executive | laws | plan-laws POLICY VALUE | preview-law ID | propose-law ID | withdraw-law BILL\nreserves | preview-reserve energy|food DAYS | procure-reserve energy|food DAYS | cancel-reserve ID\nstatus | countries | policies | party-preferences [COUNTRY] | party-alignment | sovereignty [COUNTRY] | technologies [POLICY] | preview POLICY VALUE | plan-policy POLICY VALUE | enact POLICY VALUE\nadvance DAYS (max 3660) | until YYYY-MM-DD | next | borrow MILLIONS | repay MILLIONS\nincome-tax-bands | income-tax-report | preview-income-tax STANDARD_RATE\nfiscal-plans | preview-budget YEAR TAX SERVICES WELFARE | propose-budget YEAR TAX SERVICES WELFARE | withdraw-budget ID\nundertaking-templates | undertakings | preview-undertaking TEMPLATE [PROJECT] | declare-undertaking TEMPLATE [PROJECT] | withdraw-undertaking ID\nterritory | preview-territory FORM lawful|contested | reform-territory FORM lawful|contested | withdraw-territory ID | preview-requisition REGION MILLIONS | requisition REGION MILLIONS\nregions | preview-region-grant REGION BOOST DAYS | region-grant REGION BOOST DAYS | preview-region-services REGION LEVEL | region-services REGION LEVEL | cancel-region ID\nelections | preview-election | request-election | withdraw-election ID\ncabinet | appoint OFFICE PERSON | preview-appointment OFFICE PERSON | dismiss OFFICE | preview-dismissal OFFICE | cancel-appointment ID\nconstitutions | preview-constitution FORM lawful|contested | reconstitute FORM lawful|contested | withdraw-constitution ID\ncommitment ID | commitments | amend ID LEVEL | preview-amend ID LEVEL | withdraw ID | preview-withdraw ID\ncash-retention | preview-retention MILLIONS DAYS | retain-cash MILLIONS DAYS | release-cash ID\nfinance | outlook [DAYS] | preview-loan MILLIONS | emergency-borrow MILLIONS | cancel-loan ID\nhouseholds | budget | data | public-report COUNTRY | events [COUNTRY] | event COUNTRY#NUMBER | ledger | history | parties | save PATH | load PATH | replay | quit\nObjective at five years: energy/food ≥90%, approval ≥50%, no arrears, debt ≤opening GDP."
                    );
                    if world.event_sequences.is_some() {println!("event COUNTRY#NUMBER · inspect one of your government's retained entries; references belong to this campaign. Accepted command replies identify their immediate outcome entries.");}
                    if content.finance.as_ref().is_some_and(|rules|rules.maturity_days.is_some()) {
                        println!("The five-year objective also requires zero overdue loan principal; inspect finance for contractual dates.");
                    }
                    if developer {
                        println!(
                            "dev-fields | dev-inspect COUNTRY | dev-ai-log COUNTRY [LIMIT] | dev COUNTRY FIELD VALUE (explicit developer edits)\ndev-trace-replay NEWFILE | dev-compare-traces REFERENCE ACTUAL (read-only replay evidence)"
                        );
                    }
                }
                "institutions" => {
                    match world.governing_institutions_view(content, Viewer::Government(world.player.clone()), &world.player)? {
                        None => println!("This campaign does not track governing institution identities."),
                        Some(view) => {
                            println!("{} · {} · government #{} · {:?}", view.primary.name, view.primary.id.0, view.context.government, view.context.route);
                            println!("Modeled backing {} / required {}. This is a support score, not a vote count.", view.backing, view.threshold);
                            if let Some(council) = view.council { println!("{} · {} · backing {} / base threshold {}", council.name, council.id.0, view.council_backing, view.council_threshold); }
                            println!("Authored gameplay institution · {} / {}. Special procedures may use different consent rules.", view.primary.source, view.primary.source_version);
                        }
                    }
                }
                "command-context" => {
                    let command = CommandId(words.get(1).ok_or("Missing command ID")?.parse().map_err(|_| "Command ID must be a positive integer")?);
                    match world.command_governing_context(content, Viewer::Government(world.player.clone()), command)? {
                        Some(context) => println!("Command #{} · governing context when submitted: government #{} · {:?} · {} · council {:?}. This records submission, not approval or votes.", command.0, context.government, context.route, context.primary.0, context.council.map(|id| id.0)),
                        None => println!("No governing context available for this command."),
                    }
                }
                "executive" => {
                    match world.executive_identity_view(content, &world.player)? {
                        None => println!("This campaign does not track live executive identities. Opening references remain available through data."),
                        Some(view) => {
                            println!("{} · holder since {} · government #{} from {}", view.name, date(view.holder_since), view.government.0, date(view.mandate_since));
                            println!("Holder: {:?} · source {} / {} · {:?}", view.holder, view.source, view.source_version, view.quality);
                            println!("Opening reference: {}. Current powers follow the governing constitution; future reference terms do not appoint live holders.", view.opening_office_title);
                            if let Some(origin) = &view.body_origin { println!("Collective origin: {origin:?}. Unlisted membership is unspecified."); }
                            for entry in view.tenures.iter().rev() {
                                let tenure = &entry.tenure;
                                println!("Government #{} · {} · {} to {}", tenure.government.0, entry.name, date(tenure.from_day), tenure.until_day.map(date).unwrap_or_else(|| "present".into()));
                            }
                        }
                    }
                }
                "laws" => {
                    let rules = content.legislation.as_ref().ok_or("This scenario has no statutory workflow")?;
                    let state = world.player_country().legislation.as_ref().ok_or("Missing legislation state")?;
                    println!("Statutory permission for your government · drafting, consent and implementation precede programme commissioning.");
                    for law in &rules.definitions {
                        println!("{} · {} · proposal from {} · {}",law.id.0,law.name,law.available_from,
                            state.enacted.get(&law.id).map(|entry|format!("enacted {} by bill #{}",date(entry.day),entry.bill.0)).unwrap_or_else(||"not enacted".into()));
                    }
                    for bill in state.bills.iter().rev() {
                        println!("Bill #{} · {} · {} · paid {}m / {}m · reserved {}m · consent {} · enactment {} · {}",bill.id.0,bill.law.0,bill.status.label(),bill.paid.0,bill.total_cost.0,bill.reserved.0,date(bill.review_day),date(bill.enactment_day),bill.explanation);
                    }
                }
                "plan-laws" => {
                    let policy: PolicyId = (*words.get(1).ok_or("Missing policy ID")?).into();
                    let value = number(words.get(2))?;
                    let plans = world.policy_law_plan(content, Viewer::Government(world.player.clone()), &policy, value)?;
                    println!("Statutory prerequisites for your government's {policy} target {value}. This inspection approves no order or funding.");
                    if plans.is_empty() { println!("No statutory prerequisite is defined for this target in the loaded scenario."); }
                    for plan in plans {
                        println!("{} · {}", plan.law.0, plan.name);
                        match plan.progress {
                            LawPermissionProgress::Enacted {bill, day} => println!("Enacted {} · bill #{}",date(day),bill.0),
                            LawPermissionProgress::Pending {bill,status,paid,reserved,review_day,enactment_day} => println!("Bill #{} · {} · paid {}m · protected {}m · conditional consent {} · earliest enactment {}. Pending work grants no permission.",bill.0,status.label(),paid.0,reserved.0,date(review_day),date(enactment_day)),
                            LawPermissionProgress::Proposable(p) => println!("Not enacted. A proposal today costs {}m: {}m drafting now, {}m protected implementation cash; conditional consent {}; earliest enactment {}. {}",p.total_cost.0,p.upfront.0,p.reserved.0,date(p.review_day),date(p.enactment_day),p.authority.label()),
                            LawPermissionProgress::Blocked(reason) => println!("Not enacted. Proposal condition: {reason}"),
                        }
                    }
                    println!("Law enactment grants permission; programme funding, consent and delivery remain separate. Dates depend on backing and paid implementation. Inspect laws or preview-law ID before proposing.");
                }
                "preview-law" => {
                    let law = LawId(words.get(1).ok_or("Missing law ID")?.to_string());
                    let p = world.preview_law(content,&world.player,&law)?;
                    println!("{} · total {}m: {}m drafting now, {}m protected implementation cash; consent {}; earliest enactment {}. {}. Programme funding and delivery remain separate.",law.0,p.total_cost.0,p.upfront.0,p.reserved.0,date(p.review_day),date(p.enactment_day),p.authority.label());
                }
                "propose-law" => { let law=LawId(words.get(1).ok_or("Missing law ID")?.to_string());issue(world,content,Action::ProposeLaw{law})?; }
                "withdraw-law" => { issue(world,content,Action::WithdrawLaw{request:LawBillId(number(words.get(1))?)})?; }
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
                "dev-ai-log" => {
                    if !developer {
                        return Err("AI decision inspection requires the explicit --developer host".into());
                    }
                    if !(2..=3).contains(&words.len()) {
                        return Err("Usage: dev-ai-log COUNTRY [LIMIT] (1..500, default 100)".into());
                    }
                    let country = CountryId(words[1].into());
                    let limit = words.get(2).map_or(Ok(100), |value| value.parse::<usize>())
                        .map_err(|_| "Invalid AI log limit")?;
                    let log = world.developer_ai_log(&country, limit)?;
                    println!("LIVE HOST TRUTH · recorded automated commands and retained AI review messages. Newest first; reviewed budget dispatch is identified by budget_bill. Unrecorded deliberation is unavailable.");
                    println!("{}", serde_json::to_string_pretty(&log).map_err(|e| e.to_string())?);
                }
                "dev-trace-replay" => {
                    if !developer {
                        return Err("Replay traces require the explicit --developer host".into());
                    }
                    if words.len() != 2 {
                        return Err("Usage: dev-trace-replay NEWFILE".into());
                    }
                    let file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(words[1])
                        .map_err(|error| format!("Cannot create new trace: {error}"))?;
                    let mut writer = io::BufWriter::new(file);
                    let summary = sim_core::replay_trace::write_replay_trace(world, content, &mut writer)?;
                    println!("Replay trace: {} boundaries; {:?}", summary.footer.points, summary.footer.outcome);
                }
                "dev-compare-traces" => {
                    if !developer {
                        return Err("Replay traces require the explicit --developer host".into());
                    }
                    if words.len() != 3 {
                        return Err("Usage: dev-compare-traces REFERENCE ACTUAL".into());
                    }
                    let reference = std::fs::File::open(words[1]).map_err(|e| e.to_string())?;
                    let actual = std::fs::File::open(words[2]).map_err(|e| e.to_string())?;
                    let comparison = sim_core::replay_trace::compare_replay_traces(
                        io::BufReader::new(reference), io::BufReader::new(actual))?;
                    if let Some(difference) = comparison.first_difference {
                        println!("First replay difference at boundary {}: reference {:?}; actual {:?}; changed components {:?}",
                            difference.sequence, difference.expected, difference.actual, difference.components);
                    } else {
                        println!("Replay executions match across {} boundaries.", comparison.compared_points);
                    }
                    println!("Reference outcome: {:?}; actual outcome: {:?}", comparison.expected_outcome, comparison.actual_outcome);
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
                "plan-policy" => {
                    let policy:PolicyId=(*words.get(1).ok_or("Missing policy ID")?).into();
                    let value=number(words.get(2))?;
                    let plan=world.policy_funding_plan(content,&policy,value)?;
                    println!("Funding comparison for {} at proposed value {value}: implementation cost {}m; spendable cash {}m; actual retained cash {}m; spendable shortfall {}m.",content.policy(&policy).expect("validated policy").name,plan.preview.cost.0,plan.available_cash.0,plan.retained_cash.0,plan.shortfall.0);
                    println!("Authority: {}. Conditional consent {} days; delivery lag {} days; upfront {}m and remaining reservation {}m.",plan.preview.route,plan.preview.consent_delay.units(),plan.preview.delay.units(),plan.preview.upfront_payment.0,plan.preview.cost.0-plan.preview.upfront_payment.0);
                    println!("This comparison approves no order or funding. Future receipts and retention targets are not cash. Release retained cash before proposing; actual funding and authority are rechecked.");
                }
                "preview" | "enact" => {
                    let policy: PolicyId = (*words.get(1).ok_or("Missing policy ID")?).into();
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
                        issue(world, content, Action::Enact { policy, value })?;
                        println!("Commitment accepted.");
                    }
                }
                "commitment" => {
                    let request = CommitmentId(number(words.get(1))?);
                    let record = world.player_country().commitment(&request)
                        .ok_or("No programme with that ID in your government's view")?;
                    println!("Programme #{request} · current stage {:?}", record.stage());
                    println!("{}", serde_json::to_string_pretty(record).map_err(|e| e.to_string())?);
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
                        issue(world, content, Action::AmendCommitment { request, value })?;
                    }
                }
                "withdraw" | "preview-withdraw" => {
                    let request = CommitmentId(number(words.get(1))?);
                    let preview = world.preview_cancellation(content, &world.player, request)?;
                    if words[0] == "preview-withdraw" {
                        println!("{preview:?}");
                    } else {
                        issue(world, content, Action::CancelCommitment { request })?;
                    }
                }
                "advance" => {
                    world.advance(content, sim_core::DaySpan::new(number(words.get(1))?))?;
                    status(world, content);
                }
                "until" => {
                    let target = parse_date(words.get(1).ok_or("Missing date YYYY-MM-DD")?)?;
                    if target < world.day {
                        return Err("Cannot rewind; load an earlier snapshot".into());
                    }
                    while world.day.units() < (target).units() {
                        world.advance(content, sim_core::DaySpan::new(((target).units() - world.day.units()).min(3660)))?;
                    }
                    status(world, content);
                }
                "next" => {
                    println!("Advanced {} days", world.advance_to_event(content)?.units());
                    status(world, content);
                }
                "borrow" => issue(world, content,
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
                        issue(world, content, Action::ProposeBudget { year, targets })?;
                    }
                }
                "withdraw-budget" => issue(world, content,
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
                        issue(world, content, Action::DeclareUndertaking { template, project })?;
                    }
                }
                "withdraw-undertaking" => issue(world, content,
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
                        issue(world, content, Action::ReformTerritory { target, contested })?;
                    }
                }
                "withdraw-territory" => issue(world, content,
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
                        issue(world, content, Action::RequisitionRegionalFunds { region, amount })?;
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
                        days: sim_core::DaySpan::new(number(words.get(3))?),
                    };
                    if words[0] == "preview-region-grant" {
                        println!(
                            "{:?}",
                            world.preview_regional_order(content, &world.player, &region, &kind)?
                        );
                    } else {
                        issue(world, content, Action::RegionalOrder { region, kind })?;
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
                        issue(world, content, Action::RegionalOrder { region, kind })?;
                    }
                }
                "cancel-region" => issue(world, content,
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
                "request-election" => issue(world, content, Action::RequestElection)?,
                "withdraw-election" => issue(world, content,
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
                        issue(world, content, Action::Appoint { office, person })?;
                    } else {
                        println!("{preview:?}");
                    }
                }
                "dismiss" | "preview-dismissal" => {
                    let office = OfficeId(words.get(1).ok_or("Missing portfolio ID")?.to_string());
                    let fee = world.preview_dismissal(content, &world.player, &office)?;
                    if words[0] == "dismiss" {
                        issue(world, content, Action::DismissMinister { office })?;
                    } else {
                        println!(
                            "Dismissal administration {}m; civil-service preparation and a new appointment cooldown apply.",
                            fee.0
                        );
                    }
                }
                "cancel-appointment" => issue(world, content,
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
                        issue(world, content, Action::Reconstitute { target, contested })?;
                    }
                }
                "withdraw-constitution" => issue(world, content,
                    Action::CancelConstitution {
                        request: TransitionId(number(words.get(1))?),
                    },
                )?,
                "outlook" => {
                    let horizon = if words.len() > 1 {
                        number(words.get(1))?
                    } else {
                        90_u32.min(MAX_DAY - world.day.units())
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
                    if let Some(interest) = &plan.interest_accrual {
                        println!("Available cash {}m · mean balance {:+.4}m/day · interest within {horizon} days {}m without borrowing / {}m if approved, including carried fraction.",plan.available_cash.0,interest.national_daily_numerator as f64 / f64::from(INTEREST_DENOMINATOR),interest.without_loan_due.0,interest.if_approved_due.0);
                        println!("{}",plan.covered_days.map_or_else(||format!("No projected funding gap within {horizon} days; later funding is not guaranteed."),|days|format!("First funding gap after {days} fully covered days.")));
                    } else { println!(
                        "Available cash {}m · balance {:+}m/day · deficit runway {}",
                        plan.available_cash.0,
                        plan.daily_balance.0,
                        plan.covered_days.map_or_else(
                            || "balanced or positive run rate".into(),
                            |days| format!("{days} fully covered days")
                        )
                    );
                    }
                    if world.player_country().cash_management.as_ref().is_some_and(|s|s.active().is_some()) { println!("Retained cash {}m excluded. Projection holds retention fixed; future capture and automatic release are not simulated.",world.player_country().retained_cash().0); }
                    if content.finance.as_ref().is_some_and(|rules|rules.maturity_days.is_some()) {
                        println!("Dated principal within horizon: {}m; conditional total {}m. Subtracts full due principal (existing overdue principal from tomorrow); recurring interest is held fixed after repayment, conservatively overstating it. No automatic refinancing.",plan.principal_due.0,plan.conditional_principal_due.0);
                    }
                    for point in plan.points.iter().filter(|point| {
                        point.day == world.day
                            || point.day == sim_core::CampaignDay::new(world.day.units() + horizon)
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
                        let rate = plan.interest_accrual.as_ref().map_or_else(|| format!("{}m/day",plan.additional_daily_interest.0), |i| format!("{:.4}m/day mean",i.additional_daily_numerator as f64 / f64::from(INTEREST_DENOMINATOR)));
                        println!("Pending {}m on {} IF approved · additional interest {rate}",loan.amount.0,date(loan.decision_day));
                    }
                }
                "cash-retention" => {
                    let c = world.player_country();
                    let state = c.cash_management.as_ref().ok_or("This campaign has no temporary cash retention")?;
                    println!("Actual retained cash {}m; spendable {}m; target is not money. Deferring operating payments creates arrears; matured principal remains due. Release before funding a normal decision.", c.retained_cash().0, c.available_cash().0);
                    println!("{}", serde_json::to_string_pretty(state).map_err(|e|e.to_string())?);
                }
                "preview-retention" => {
                    let preview = world.preview_cash_hold(content, &world.player, Money(number(words.get(1))?), DaySpan::new(number(words.get(2))?))?;
                    println!("Target {}m; actual protected now {}m; approval -{}; expiry {}; reuse {}. Unfilled target is not cash. Wages, welfare and interest may become arrears; matured principal may remain unpaid. Local claims and project reservations remain protected.", preview.target.0, preview.retained_now.0, preview.approval_cost, date(preview.expiry_day), date(preview.next_allowed_day));
                }
                "retain-cash" => issue(world, content, Action::HoldCash { target: Money(number(words.get(1))?), days: sim_core::DaySpan::new(number(words.get(2))?)})?,
                "release-cash" => issue(world, content, Action::ReleaseCash { request: CashHoldId(number(words.get(1))?) })?,
                "sovereignty" => {
                    let country=words.get(1).map_or_else(||world.player.clone(),|id|CountryId((*id).into()));
                    if let Some(report)=content.bookmark_sovereignty(&country)? {
                        println!("Sovereignty/recognition reference for {} at bookmark {}. Live government and regime are separate; future references do not force campaign changes.",country.0,report.bookmark);
                        for row in report.territories {
                            if let Some(r)=row.reference {println!("{} [{}]: {} · {} · {:?} · {}@{} · {} until {} (end excluded)",row.territory.name,row.territory.id.0,r.sovereignty.label(),r.control.label(),r.quality,r.source,r.source_version,r.validity.valid_from,r.validity.valid_to.map_or_else(||"open-ended".into(),|d|d.to_string()));}
                        }
                        for r in report.recognition {println!("{} about {}: {} · {:?} · {}@{} · {} until {} (end excluded)",r.observer.0,r.subject.0,r.position.label(),r.quality,r.source,r.source_version,r.validity.valid_from,r.validity.valid_to.map_or_else(||"open-ended".into(),|d|d.to_string()));}
                        println!("Unlisted recognition is unknown. Map outlines establish no sovereignty/control. Records grant no revenue, military powers or territorial orders.");
                        if let Some(catalogue)=&content.sovereignty {
                            for row in catalogue.territories_at(report.bookmark) {if row.reference.as_ref().is_some_and(|r|matches!(r.sovereignty,LegalSovereignty::Unknown)||matches!(r.control,TerritorialControl::Unknown)) {println!("Unknown reference territory: {} [{}] · {}",row.territory.name,row.territory.id.0,row.reference.as_ref().map_or_else(||"No record".into(),|r|format!("{} · {} · {:?} · {}@{}",r.sovereignty.label(),r.control.label(),r.quality,r.source,r.source_version)));}}
                        }
                    } else {println!("This pack has no sovereignty/recognition reference catalogue; absence is unknown.");}
                }
                "technologies" => {
                    let selected=words.get(1).map(|id|PolicyId::from(*id));
                    if let Some(policy)=&selected { content.policy(policy).ok_or("Unknown policy")?; }
                    if content.technologies.is_none() { println!("This campaign has no technology prerequisites."); }
                    else {
                        for policy in content.policies.iter().filter(|p|selected.as_ref().is_none_or(|id|id==&p.id)) {
                            println!("{}:",policy.name);
                            let requirements=world.policy_technology_eligibility(content,&world.player,&policy.id)?;
                            if requirements.is_empty() {println!("  No technology prerequisite in this scenario.");}
                            for r in requirements {println!("  {} · earliest {} · country eligible {} · available today {}",r.name,r.available_from,r.country_eligible,r.available());}
                        }
                        println!("Scenario eligibility is not research or adoption. Normal authority, cash and implementation remain necessary; availability grants no capacity.");
                    }
                }
                "finance" => {
                    if let Some(financing) = &world.player_country().financing {
                        println!("Borrowing authority · pending funds are not treasury cash");
                        if let Some(interest) = &financing.interest_accrual {
                            println!("Interest: fixed 4% annual actual/365, no compounding. Whole assessed {}m; actually paid {}m; unpaid whole interest {}m; unsettled fraction {}/{} of 1m. Fraction is a creditor claim, never available cash, and survives principal retirement.",interest.whole_assessed().0,interest.paid.0,world.player_country().arrears_external.0,interest.remainder,INTEREST_DENOMINATOR);
                            if let Some(last) = &interest.last { println!("Latest assessment {}: principal basis {}m, whole interest due {}m. Later manual finance commands affect tomorrow.",date(last.day),last.principal.0,last.amount_due.0); }
                        }
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
                        println!("Principal due/overdue now {}m; including next 30 days {}m. Outstanding matured principal remains debt, not duplicated operating arrears.",financing.principal_due_through(world.day).0,financing.principal_due_through(sim_core::CampaignDay::new(world.day.units().saturating_add(30))).0);
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
                            preview.consent_days.units()
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
                            rule.cooldown_days.units()
                        ),
                        Err(reason) => println!("Emergency route unavailable: {reason}"),
                    }
                }
                "emergency-borrow" => issue(world, content,
                    Action::EmergencyBorrow {
                        amount: Money(number(words.get(1))?),
                    },
                )?,
                "cancel-loan" => issue(world, content,
                    Action::CancelBorrow {
                        request: LoanId(
                            number::<i64>(words.get(1))?
                                .try_into()
                                .map_err(|_| "Loan ID must be positive")?,
                        ),
                    },
                )?,
                "repay" => issue(world, content,
                    Action::Repay {
                        amount: Money(number(words.get(1))?),
                    },
                )?,
                "public-report" => {
                    let id=CountryId((*words.get(1).ok_or("Usage: public-report COUNTRY")?).into());
                    let report=world.public_report(content,&id)?;
                    println!("{}",serde_json::to_string_pretty(&report).map_err(|error|error.to_string())?);
                }
                "event" => {
                    let id=words.get(1).ok_or("Usage: event COUNTRY#NUMBER")?.parse::<EventId>()?;
                    match world.event_by_reference(content,Viewer::Government(world.player.clone()),&id)? {
                        Some(event)=>print_event(event),
                        None=>println!("No retained event with that reference in your government's view."),
                    }
                }
                "events" => {
                    let id = words.get(1).map_or(world.player.0.as_str(), |s| *s);
                    if id == world.player.0 {
                        for e in world.events.iter().filter(|e| e.country == world.player) {
                            print_event(e);
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
                "social-groups" => {
                    let id = words.get(1).map_or_else(|| world.player.clone(), |id| CountryId((*id).into()));
                    let selected = words.iter().skip(2).map(|id| SocialGroupId((*id).into())).collect();
                    match world.social_membership_report(content, Viewer::Government(world.player.clone()), &id, &selected)? {
                        Some(report) => {
                            println!("{} · scenario joint memberships · {} / {}", id.0, report.source, report.source_version);
                            for group in &report.groups { println!("{} · {}", group.id, group.name); }
                            if let Some(counts) = report.owned {
                                println!("Unique modeled residents on {}: {}. Membership counts overlap and are not additional residents.", date(report.day), counts.population.0);
                                for (id, count) in counts.members { println!("{}: {} projected members", id, count.0); }
                                if !selected.is_empty() { println!("Any selected: {} · every selected: {} · none selected: {}", counts.any_selected.0, counts.all_selected.0, counts.none_selected.0); }
                            } else { println!("No current membership counts are published. Scenario definitions are not a zero-membership observation."); }
                            println!("Authored cross-sectional joint profiles, not historical prevalence or individual identity histories. Membership grants no cash, votes or policy exemptions.");
                        }
                        None => println!("This preserved pack has no joint social membership calibration; unknown is not zero."),
                    }
                }
                "population" => {
                    let id = words.get(1).map_or_else(|| world.player.clone(), |id| CountryId((*id).into()));
                    match world.demographic_report(content, Viewer::Government(world.player.clone()), &id)? {
                        Some(report) => {
                            println!("{} · authored population change · {} / {}", id.0, report.source, report.source_version);
                            for release in report.published { println!("Published #{} {}: {} residents; rounding interval {}–{}, resolution {}, statistical capacity {}%. Dated rounding, not statistical confidence or current truth.", release.id.0, date(release.day), release.population.0, release.lower.0, release.upper.0, release.resolution.0, release.statistical_capacity); }
                            if let Some(groups) = report.owned_groups {
                                for group in groups { println!("Owned {} as of {}: young {} / working age {} / older {} · households {} · cumulative births {} · deaths {} · arrivals {} / departures {} · formed {} / dissolved {}", group.group, date(report.as_of), group.ages.young.0, group.ages.working_age.0, group.ages.older.0, group.households.0, group.flows.births.0, group.flows.deaths.total()?.0, group.flows.arrivals.total()?.0, group.flows.departures.total()?.0, group.flows.households_formed.0, group.flows.households_dissolved.0); }
                                if let Some(next) = report.next_attempt { println!("Next census attempt {}. Actual administration determines measurement precision and subsequent frequency.", date(next)); }
                            }
                            println!("Fixed authored scenario rates, not historical national vital statistics. Working age is not employment. Household units create no dwellings or assets. Production capacity, cash and programme authority require their separate normal processes.");
                        }
                        None => println!("This preserved pack has fixed opening population and no demographic transition model."),
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
                                |gini| format!("{:.3}", f64::from(gini.units()) / 10000.0)
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
                    for (field, value) in [("population", d.population.0 as i64), ("gdp", d.gdp.units())] {
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
                    if let Some(archive) = &world.player_country().statistical_archive {
                        println!("Older archived review-sample means (floor rounded, not annual totals or daily time averages):");
                        for summary in &archive.years {
                            if let Some(mean) = summary.mean() {
                                println!("{} · {} samples, {} through {} · mean cash {}m · debt {}m · approval {}% · needs {}% · daily output {}m", summary.year, summary.samples, date(summary.first_day), date(summary.last_day), mean.treasury.0, mean.debt.0, mean.approval, mean.needs, mean.output.0);
                            }
                        }
                        println!("Recent review point observations:");
                    }
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
                "reserves" => {
                    for good in [GoodKind::Energy,GoodKind::Food] {
                        match world.reserve_supply(content,&world.player,good)? {
                            Some(s)=>println!("{good:?} observed {}: civilian supply {}/{}; imports {}/{}; spare {} units/day. {} Last settlement only; future deliveries are conditional.",date(s.day),s.civilian_supply,s.physical_need,s.civilian_imports,s.import_capacity,s.spare_import_capacity,s.explanation()),
                            None=>println!("{good:?}: no settled supply observation yet; advance one day."),
                        }
                    }
                    println!("{}",serde_json::to_string_pretty(&world.player_country().reserves).map_err(|e|e.to_string())?);
                    println!("National buffers are separate from firms' working stocks. Procurement uses spare import capacity after civilian physical needs. Releases cover post-import physical shortfalls; household affordability still applies.");
                }
                "preview-reserve" | "procure-reserve" => {
                    let good=match words.get(1).copied() { Some("energy")=>GoodKind::Energy,Some("food")=>GoodKind::Food,_=>return Err("Choose energy or food".into()) };
                    let days=NeedDays::new(number(words.get(2))?);
                    if words[0]=="preview-reserve" {println!("{:?}",world.preview_reserve(content,&world.player,good,days)?);} else {issue(world, content,Action::ProcureReserve{good,days})?;}
                }
                "cancel-reserve" => { issue(world, content,Action::CancelReserve{request:ReserveOrderId(number(words.get(1))?)})?; }
                "party-preferences" => {
                    let country = CountryId(words.get(1).map_or(world.player.0.as_str(), |value| *value).to_string());
                    let definition = content.country(&country).ok_or("Unknown country")?;
                    if let Some(rules) = &content.party_preferences {
                        println!("Party preference model {}@{}; authored/reference positions, independent of ideology labels.", rules.source, rules.source_version);
                        for profile in rules.profiles.iter().filter(|p| p.country == country) {
                            let party = definition.party(&profile.party).ok_or("Missing party")?;
                            println!("{} [{}] · {:?} · {}@{}", party.name, party.id, profile.quality, profile.source, profile.source_version);
                            for position in &profile.positions { println!("{} · preferred {} · relative importance {}", position.policy, position.preferred.as_ref().map_or_else(|| "Unknown".into(), |range| format!("{}..{}", range.min, range.max)), position.importance); }
                        }
                    } else { println!("This pack defines no party policy-preference model."); }
                }
                "party-alignment" => {
                    if let Some(report) = world.party_alignment_for(content, &world.player, &FiscalTargets::current(world.player_country()))? {
                        println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
                        println!("Only operating policy levels affect the backing target; held backing follows monthly political reviews and other opinion/funding effects.");
                    } else { println!("This campaign has no party policy-preference model."); }
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
        world.advance(&content, sim_core::DaySpan::new(days))?;
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
