//! Searchable rules/content reference. This view has no access to private World state.
use crate::Tab;
use eframe::egui;
use sim_core::{Content, CountryId};

#[derive(Default)]
pub struct Help {
    pub reference: bool,
    query: String,
}
pub struct Destination {
    pub tab: Tab,
    pub country: Option<CountryId>,
    pub policy: Option<String>,
}
struct Entry {
    title: String,
    body: String,
    destination: Destination,
}
fn entries(content: &Content) -> Vec<Entry> {
    let mut entries: Vec<_> = [
        ("Food, affordability and shortages", "Households need both affordable baskets and delivered food. Domestic capacity, shocks, bounded imports and inventory constrain supply. Food imports require business cash. Welfare changes recipients' purchasing power; agriculture expands capacity after implementation. The household screen identifies groups with poor delivery.", Tab::Society),
        ("Household income, savings and group-mean inequality", "The household screen compares income after tax with each group's modeled essential basket and remaining cash buffer. Income below basket cost is financial pressure, not an individual poverty headcount: savings can fund consumption. Delivered food is a separate physical measure. The population-weighted Gini uses group-mean disposable income and excludes within-group inequality; zero total income or unsupported group means make it undefined.", Tab::Society),
        ("Energy and production", "Delivered energy constrains output. Domestic capacity, supply shocks, paid imports and stored energy determine availability. A capacity program takes time to implement. Compare domestic capacity with realized delivery; they are different measures.", Tab::Treasury),
        ("Treasury, reservations and arrears", "Treasury is a stock of cash. Revenue and payments are flows over a stated period. Reserved policy capital remains inside treasury but cannot fund another commitment or debt repayment. Funding outlook shows whole days of the current deficit covered by available cash; its conditional chart includes additional loan interest. Negative forecasts mean a funding gap, not an overdraft. The forecast holds policies and receipts constant and excludes shocks, price changes, arrears repayment and pending program delivery. Unpaid obligations become arrears owed to their original recipients; a forecast is not available cash. Expand Trace realized cash transfers to compare payer/recipient flows, opening and closing balances, transaction reasons and dates.", Tab::Treasury),
        ("Borrowing, repayment and interest", "Loans transfer finite external cash to treasury and add debt principal. Principal repayment reduces debt and pays the external counterpart. Interest is charged at the authored 4% annual rate. Inspect financing, interest, arrears and uncommitted cash before borrowing or repaying. Where the campaign defines borrowing authority, requests await consent and funds stay unavailable until approval. Backing, sponsor and credit are rechecked. The explicit contested emergency route shows political costs and cooldown before acceptance; cancellation releases no cash and creates no debt.", Tab::Treasury),
        ("Consent, authority and implementation", "Government definitions choose the legal route, consent delay and required political backing. A proposal pays upfront and reserves the remaining capital. Authority is checked again at consent. Accepted policy changes take effect after implementation; a pending commitment is not an operating program.", Tab::Policies),
        ("Relative prices and market settlement", "Rules-3 packs configure food and energy price bounds, daily adjustment limits, import caps, storage and spoilage. Today's settlement uses the previously announced price; supply versus affordable demand determines the next day's index. Food reprices a configured share of essential baskets; food and energy prices change import bills. Expand the market report in Treasury to distinguish physical need, affordable demand, supply, delivery and inventories. The ordinary rules-2 starter retains fixed prices.", Tab::Treasury),
        ("Parties, elections and leadership", "Elections and leadership reviews follow the selected country's institutional configuration. Legislative, elite and council support are separate constraints. Seat counts, profiles and political responses in this initial pack are authored game abstractions. Country play continues after a change of government.", Tab::Politics),
        ("Reports, charts and missing observations", "Charts and map overlays use dated observations. A missing release is unknown, not zero. Foreign countries expose published aggregates; private treasury, household accounts, pending policies and AI plans are not government knowledge. Comparisons show observation dates and units.", Tab::Analysis),
        ("Five-year mandate and continuing play", "At the five-calendar-year review, achieve food and energy delivery of at least 90%, approval of at least 50%, zero arrears, and debt no greater than opening annual GDP. The campaign continues after success or failure. Developer-edited campaigns are marked and cannot receive ordinary mandate certification.", Tab::Overview),
        ("Events, alerts and delivery dates", "The chronicle records accepted decisions, political reviews, implementation and important alerts. Pause on important events or use Next event to advance to a decision boundary. A rejected order explains its failed condition and does not enter the accepted-command journal.", Tab::Chronicle),
        ("Map, country selection and borders", "Select a country by its map shape or directory entry. Pan by dragging, zoom at the pointer and use Focus to locate the selected country. Reference geometry is separate from simulation identity; it is not certified historical sovereignty or territorial control. Fictional packs use markers.", Tab::Overview),
        ("Save, autosave, recovery and replay", "Save and Load use the sidebar path. Monthly autosaves have country/scenario/content-specific names inside saves. Inspect recovery snapshots before restoring a primary, backup or interrupted write. Compatible originals are preserved before conversion. Rules-2 campaigns write format 3; relative-price rules-3 campaigns require format 4. Replay verifies recorded commands and final state; unsupported files leave the active campaign intact.", Tab::Settings),
        ("Pause, speed and keyboard controls", "Pause and time controls stay at the top of every page. Automatic progression advances the chosen number of days per step; rendering does not change simulation rules. Space toggles play by default. F1 opens settings; Escape pauses and returns to the map, or closes recovery. Focus loss pauses by default. Rebind controls and change interface scale in settings.", Tab::Settings),
    ].into_iter().map(|(title, body, tab)| Entry {
        title: title.into(), body: body.into(), destination: Destination { tab, country: None, policy: None }
    }).collect();
    if content.income_taxes.is_some() {
        entries.push(Entry {title:"Progressive private-income bands and withholding".into(),body:"The income-tax policy and fiscal plans set a standard marginal reference rate. Authored bands scale it and tax only the income slice in each band. Each cohort uses a representative resident, not an estimated within-cohort distribution or historical filing law. Private income actually received is the base; public/local service payments and welfare are excluded. Thresholds use opening GDP per resident and 365-day annualisation. Collection capacity reduces modeled withholding. Potential tax is neither treasury cash nor a new arrears claim. Treasury and the income-tax policy worksheet separate conditional forecasts from dated realized private gross, potential and paid tax. Standard-rate changes and administration programmes affect withholding only after delivery. Protected local shares use actual collected receipts.".into(),destination:Destination {tab:Tab::Treasury,country:None,policy:None}});
    }
    if content
        .finance
        .as_ref()
        .is_some_and(|rules| rules.maturity_days.is_some())
    {
        entries.push(Entry {title:"Dated loan principal and refinancing pressure".into(),body:"New loans mature a fixed authored number of days after actual disbursement, including contested loans. Opening debt remains a perpetual bookmark calibration. A 30-day notice and Treasury calendar identify obligations. After tax settlement and protected local claims, uncommitted national cash repays due principal before interest and national purchases. Reservations remain protected. Unpaid matured principal stays in outstanding debt and is retried daily; it is not duplicated as operating arrears. The five-year review also requires zero overdue principal. Voluntary repayment still clears oldest debt first. Refinancing needs a separate authorized loan and finite creditor cash; pending requests are not money. Funding charts subtract full dated principal while holding recurring interest fixed after repayment, conservatively overstating later interest. Default, restructuring, FX and historical maturity curves remain unfinished.".into(),destination:Destination {tab:Tab::Treasury,country:None,policy:None}});
    }
    if content.fiscal_plans.is_some() {
        entries.push(Entry {title:"Approved budgets, schedules and observed setting changes".into(),body:"An approved budget announces targets and funds implementation. Each linked policy still has its own schedule. Foreign-country fiscal reports show the scheduled dates and dated evidence when a linked setting actually changes. These records describe historical policy settings; daily funding and country outcomes need separate evidence, and later decisions can change operating settings again. A withdrawn programme is not credited when an unrelated replacement reaches the same target. Pending deliberations, cash and private project identifiers stay outside the public report.".into(),destination:Destination {tab:Tab::Treasury,country:None,policy:None}});
        entries.push(Entry {title:"Annual and supplementary fiscal plans".into(),body:"Treasury combines income tax, service and welfare settings into a current-year supplementary or next-year annual proposal. Annual preparation opens in the configured window. Administration is paid and all implementation costs reserved at submission; expected revenue is not spendable cash. Review rechecks backing and cabinet sponsorship and funds the whole package or refuses it without a partial enactment. Each funded programme retains its own preparation and delivery date; annual settings cannot operate before January. Pending proposals can be withdrawn, retaining administration and interval. Successor cabinets close pending bills; enacted programmes continue, subject to ordinary funded-project withdrawal rules. Conditional forecasts hold supply, collection, GDP and funding assumptions fixed. Actual daily execution and year-end reports show cash consequences. The plan does not impose a hard appropriation ceiling or model every tax base.".into(),destination:Destination {tab:Tab::Treasury,country:None,policy:None}});
    }
    if content.undertakings.is_some() {
        entries.push(Entry {title:"Delivery promises, governing support and project deadlines".into(),body:"Declare a costed undertaking under Policies & implementation, then obtain consent, fund and deliver its policy separately. Accepted undertakings publish a fixed target and deadline; shared commitment milestones and actual policy delivery records determine fulfilment. Backing and cabinet sponsorship are rechecked at review. Missing a deadline or withdrawing an accepted undertaking incurs explicit political costs and reduces credibility; pending withdrawal keeps administration/interval without a breach penalty. Successor cabinets close prior pending/active undertakings, while policy projects retain their own terms. Credibility is a bounded record-based eligibility score, not a production multiplier. These are authored public pledges, governing support arrangements and project deadlines; full election campaigns, coalition formation, commercial contracts and treaties remain unfinished.".into(),destination:Destination {tab:Tab::Policies,country:None,policy:None}});
    }
    if content.territorial.is_some() {
        entries.push(Entry {title:"Territorial reforms and contested requisitions".into(),body:"Territorial reforms change actual autonomy and collected-tax rights. Lawful loss of local powers requires an authored household consent proxy and reserved compensation. Contested changes require effective-power backing and incur political costs. Paid administration and reform intervals survive withdrawal/refusal; unused reservations are released. New charters supersede pending regional offers while accepted grants and existing local funds survive. A separate contested requisition can reclassify existing local base funds only, with administration, backing, political costs and an interval; accepted grants and escrow cannot be taken. Consolidated principal is conserved. Historical territorial law, local voting and coercive forces are not simulated.".into(),destination:Destination {tab:Tab::Politics,country:None,policy:None}});
    }
    if content.subnational.is_some() {
        entries.push(Entry { title:"Subnational powers, tax entitlements and funded grants".into(), body:"Territorial charters assign non-overlapping budget jurisdictions with protected shares of collected income tax. Public cash is consolidated, while national availability excludes local funds and grant escrow. Autonomous authorities control base service levels; delegated service orders use current national consent. Funded supplements pay administration and protect their full term in escrow. Review rechecks national backing, sponsorship and local cash absorption rules. Withdrawal/refusal releases escrow but retains fees/cooldown; accepted local grants survive national succession and cannot be reclaimed by national order. Purchases reach public-service households without creating physical capacity or certifying quality. Regional boundaries, local tax rates and separate regional households remain outside this fixture.".into(),destination:Destination {tab:Tab::Politics,country:None,policy:None}});
    }
    if content.elections.is_some() {
        entries.push(Entry {title:"Early elections and independent electoral authority".into(),body:"An early-election request needs the current form's explicit jurisdiction, ordinary governing backing, an elapsed interval, uncommitted administration cash and enough time for review and campaigning. Pending requests do not change the scheduled poll. Approval fixes an earlier date; government cannot withdraw it or postpone an earlier poll. Withdrawal/refusal keeps the fee and request interval. A mandatory confidence election takes precedence, while a costed constitutional change may supersede the mandate. Completed modeled polls publish a 100-seat allocation; turnout, districts and integrity certification are not modeled. Past historical polls are not invented at the bookmark.".into(),destination:Destination {tab:Tab::Politics,country:None,policy:None}});
    }
    if content.cabinet.is_some() {
        entries.push(Entry { title:"Ministers, appointment consent and preparation".into(), body:"Generated nominees have authored skill and political acceptability profiles. Appointments pay administration, use the current governing route and recheck cabinet sponsorship and backing at the decision. Higher skill shortens portfolio preparation for new proposals and amendments; construction already commissioned retains its date and no output or administrative capacity is created. Less acceptable nominees cost more elite and council backing. Withdrawal/refusal retains costs and cooldown. Dismissal costs administration and restores civil-service preparation; successor cabinets end prior terms and cancel pending appointments. Confirmed portfolio holders are public; pending deliberations remain private.".into(), destination: Destination {tab:Tab::Politics,country:None,policy:None} });
    }
    if content.commitments.is_some() {
        entries.push(Entry { title: "Amendment, withdrawal and outcome review".into(), body: "Pending policy levels can be amended before consent: pay administration and restart consent and delivery. The amendment limit applies to each proposal. Withdrawal releases only reserved cash; commissioned payments remain spent and no capacity is delivered. Funded withdrawal requires current authority. History retains proposals, amendments, funding, refusal, delivery, replacement and observed reviews. Direct setting changes are separated from observations affected by other programmes, prices and external shocks. Replacing a tax, service or welfare level repeals the previous level; existing physical capacity is not removed.".into(), destination: Destination { tab: Tab::Policies, country: None, policy: None } });
    }
    entries.extend(content.policies.iter().map(|policy| Entry {
        title: format!("Policy · {}", policy.name),
        body: format!("{} {} Affected groups: {}. Allowed values {}–{}; base implementation {} days; capital cost {} basis points of opening GDP. Authority and administration may add delay: use the live preview before committing.", policy.id, policy.description, policy.groups, policy.min, policy.max, policy.days, policy.cost_bps),
        destination: Destination { tab: Tab::Policies, country: None, policy: Some(policy.id.clone()) },
    }));
    entries.extend(content.countries.iter().map(|country| Entry {
        title: format!("Country · {} ({})", country.name, country.id.0),
        body: format!("Opening reference at {}: capital {}; executive {} ({}); currency {}; {:?}. {} Data quality: {}. These are bookmark definitions; the overview shows campaign developments and source records.", content.start, country.capital, country.executive, country.office, country.currency, country.regime, country.government.label(), country.quality),
        destination: Destination { tab: Tab::Overview, country: Some(country.id.clone()), policy: None },
    }));
    entries
}
fn matches(entry: &Entry, query: &str) -> bool {
    let haystack = format!("{} {}", entry.title, entry.body).to_lowercase();
    query
        .split_whitespace()
        .all(|word| haystack.contains(&word.to_lowercase()))
}
impl Help {
    pub fn show(&mut self, ui: &mut egui::Ui, content: &Content) -> Option<Destination> {
        ui.heading("Game reference");
        ui.label("Search rules, policies and the loaded bookmark's country records. Opening a screen changes inspection context; it does not issue an order.");
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search, for example: food imports, arrears, election, country name")
                    .desired_width(540.0),
            );
            if ui.button("Clear search").clicked() {
                self.query.clear();
            }
        });
        let results: Vec<_> = entries(content)
            .into_iter()
            .filter(|entry| matches(entry, &self.query))
            .collect();
        ui.label(format!("{} matching entries", results.len()));
        if results.is_empty() {
            ui.label("No matching entries. Try a shorter term, a policy name or a country ID.");
        }
        let mut destination = None;
        for entry in results {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(entry.title);
                ui.label(entry.body);
                if ui.button("Open relevant screen").clicked() {
                    destination = Some(entry.destination);
                }
            });
        }
        destination
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_search_uses_all_terms_and_only_the_loaded_bookmark() {
        let content =
            Content::from_json(include_str!("../../../data/synthetic-dated-test.json")).unwrap();
        let entries = entries(&content);
        let country = entries
            .iter()
            .find(|entry| matches(entry, "AUR steward b"))
            .unwrap();
        assert_eq!(country.destination.country.as_ref().unwrap().0, "AUR");
        assert!(matches(country, "  aur   STEWARD B  "));
        assert!(!country.body.contains("Steward C"));
        assert!(!matches(country, "nonexistent steward"));
        assert!(!entries.iter().any(|entry| matches(entry, "united kingdom")));
        assert!(entries.iter().any(|entry| matches(entry, "food imports")));
        let policy = entries
            .iter()
            .find(|entry| entry.destination.policy.as_deref() == Some("agriculture"))
            .unwrap();
        assert!(policy.destination.tab == Tab::Policies);
        assert!(policy.destination.country.is_none());
    }
}
