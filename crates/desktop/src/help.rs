//! Searchable rules/content reference. This view has no access to private World state.
use crate::Tab;
use eframe::egui;
use sim_core::{Content, CountryId, OfficeHolder, PolicyId};

#[derive(Default)]
pub struct Help {
    pub reference: bool,
    query: String,
}
pub struct Destination {
    pub tab: Tab,
    pub country: Option<CountryId>,
    pub policy: Option<PolicyId>,
}
struct Entry {
    title: String,
    body: String,
    destination: Destination,
}
fn entries(content: &Content) -> Vec<Entry> {
    let mut entries: Vec<_> = [
        ("Food, affordability and shortages", "Households need both affordable baskets and delivered food. Domestic capacity, shocks, bounded imports and inventory constrain supply. Food imports require business cash. Welfare changes recipients' purchasing power; agriculture expands capacity after implementation. The household screen identifies groups with poor delivery.", Tab::Society),
        ("Household income, savings and group-mean inequality", "The household screen compares income after tax with each group's modeled essential basket and remaining cash buffer. Income below basket cost is financial pressure, not an individual poverty headcount: savings can fund consumption. Delivered food is a separate physical measure. The population-weighted Gini uses group-mean disposable income and excludes within-group inequality; zero total income or unsupported group means make it undefined. Expand Income distribution for the cohort-mean Lorenz curve. Hover over the chart for the nearest cohort boundary; all cumulative population and income shares are also listed below it.", Tab::Society),
        ("Energy and production", "Delivered energy constrains output. Domestic capacity, supply shocks, paid imports and stored energy determine availability. A capacity program takes time to implement. Compare domestic capacity with realized delivery; they are different measures.", Tab::Treasury),
        ("Treasury, reservations and arrears", "Treasury is a stock of cash. Revenue and payments are flows over a stated period. Reserved policy capital remains inside treasury but cannot fund another commitment or debt repayment. Funding outlook shows whole days of the current deficit covered by available cash; its conditional chart includes additional loan interest. Negative forecasts mean a funding gap, not an overdraft. The forecast holds policies and receipts constant and excludes shocks, price changes, arrears repayment and pending program delivery. Unpaid obligations become arrears owed to their original recipients; a forecast is not available cash. Expand Trace realized cash transfers to compare payer/recipient flows, opening and closing balances, transaction reasons and dates.", Tab::Treasury),
        ("Borrowing, repayment and interest", "Loans transfer finite external cash to treasury and add debt principal. Principal repayment reduces debt and pays the external counterpart. Interest is charged at the authored 4% annual rate. Inspect financing, interest, arrears and uncommitted cash before borrowing or repaying. Where the campaign defines borrowing authority, requests await consent and funds stay unavailable until approval. Backing, sponsor and credit are rechecked. The explicit contested emergency route shows political costs and cooldown before acceptance; cancellation releases no cash and creates no debt.", Tab::Treasury),
        ("Consent, authority and implementation", "Government definitions choose the legal route, consent delay and required political backing. A proposal pays upfront and reserves the remaining capital. Authority is checked again at consent. Accepted policy changes take effect after implementation; a pending commitment is not an operating program.", Tab::Policies),
        ("Relative prices and market settlement", "Rules-3 packs configure food and energy price bounds, daily adjustment limits, import caps, storage and spoilage. Today's settlement uses the previously announced price; supply versus affordable demand determines the next day's index. Food reprices a configured share of essential baskets; food and energy prices change import bills. Expand the market report in Treasury to distinguish physical need, affordable demand, supply, delivery and inventories. The ordinary rules-2 starter retains fixed prices.", Tab::Treasury),
        ("Parties, elections and leadership", "Elections and leadership reviews follow the selected country's institutional configuration. Legislative, elite and council support are separate constraints. Seat counts, profiles and political responses in this initial pack are authored game abstractions. Country play continues after a change of government.", Tab::Politics),
        ("Reports, charts and missing observations", "Charts and map overlays use dated observations. A missing release is unknown, not zero. Foreign countries expose published aggregates; private treasury, household accounts, pending policies and AI plans are not government knowledge. Comparisons show observation dates and units.", Tab::Analysis),
        ("Five-year mandate and continuing play", "At the five-calendar-year review, achieve food and energy delivery of at least 90%, approval of at least 50%, zero arrears, and debt no greater than opening annual GDP. The campaign continues after success or failure. Developer-edited campaigns are marked and cannot receive ordinary mandate certification.", Tab::Overview),
        ("Events, alerts and delivery dates", "The chronicle records accepted decisions, political reviews, implementation and important alerts. Pause on important events or use Next event to advance to a decision boundary. A rejected order explains its failed condition and does not enter the accepted-command journal.", Tab::Chronicle),
        ("Map, country selection and borders", "Select a country by its map shape or directory entry. Pan by dragging, zoom at the pointer and use Focus to locate the selected country. Reference geometry is separate from simulation identity; it is not certified historical sovereignty or territorial control. Fictional packs use markers. Find lake / river filters pinned physical reference names, alternate labels and local source IDs. Choosing a row or confirming one unique match frames its bounds and shows its layer without changing country selection, the statistical overlay or campaign state.", Tab::Overview),
        ("Back, Forward and inspection history", "Back and Forward retrace up to 64 inspection visits: screen, country, policy draft and map camera/layers/reference selection/publication cutoff. Alt+Left and Alt+Right work outside text editors and open popups. Panning or editing a draft updates the current visit; taking a new route replaces forward history. Search queries and individual programme focus are retained by their panels rather than versioned by this history. These controls issue no order and do not rewind time, undo decisions or change the country you govern. Loading, recovering or starting a campaign clears history. Escape still pauses and returns to the map.", Tab::Overview),
        ("Save, autosave, recovery and replay", "Save and Load use the sidebar path. Monthly autosaves have country/scenario/content-specific names inside saves. Inspect recovery snapshots before restoring a primary, backup or interrupted write. Compatible originals are preserved before conversion. Rules-2 campaigns write format 3; relative-price rules-3 campaigns require format 4. Replay verifies recorded commands and final state; unsupported files leave the active campaign intact.", Tab::Settings),
        ("Pause, speed and keyboard controls", "Pause and time controls stay at the top of every page. Automatic progression advances the chosen number of days per step; rendering does not change simulation rules. Space toggles play by default. F1 opens settings; Escape pauses and returns to the map, or closes recovery. Focus loss pauses by default. Rebind controls and change interface scale in settings.", Tab::Settings),
    ].into_iter().map(|(title, body, tab)| Entry {
        title: title.into(), body: body.into(), destination: Destination { tab, country: None, policy: None }
    }).collect();
    if content
        .finance
        .as_ref()
        .is_some_and(|f| f.interest_accrual.is_some())
    {
        entries.push(Entry {title:"Carried interest and whole cash settlement".into(),body:"This campaign accrues fixed 4% annual interest with an actual/365 denominator on outstanding principal after scheduled redemption. Fractions carry without compounding until a whole million-credit unit becomes due. Treasury distinguishes cumulative assessment, actual payments, unpaid whole-unit arrears and a noncash fractional creditor claim. The fraction survives retiring all principal and never funds a decision. Manual loans and repayments after settlement affect the next day; delayed approval before settlement affects that day's principal basis. Funding charts use the same carried fraction and interval dues; mean rates are display values. In this ruleset, covered days means the interval before the first projected negative balance within the selected horizon, including dated principal. Taxes, wages and purchases still round to whole reporting units. Earlier campaign rules remain unchanged.".into(),destination:Destination {tab:Tab::Treasury,country:None,policy:None}});
    }
    if content.sovereignty.is_some() {
        entries.push(Entry {title:"Sovereignty, occupation and recognition references".into(),body:"The world overview has a dated public bookmark inspector. Legal sovereignty, disputed claims, civil administration, explicit occupation and directional recognition are independent records with provenance. Missing recognition is unknown, not refusal. Map geometry establishes no legal status. Future reference records select later bookmarks, never force campaign transfers. Government and regime changes do not rewrite these reference records. No revenue, combat, diplomatic recognition actions or border simulation is granted.".into(),destination:Destination {tab:Tab::Overview,country:None,policy:None}});
    }
    if let Some(technologies) = &content.technologies {
        entries.push(Entry {title:"Technology eligibility for programme commissioning".into(),body:"A scenario can require dated technology prerequisites before a programme may be proposed. Every prerequisite needs both the configured country eligibility and its earliest commissioning date. Public availability grants no cash, capacity, faster construction, research or adopted knowledge. Policies, fiscal packages, funding plans and nonplayer orders use the same conditions. Future-date programmes cannot be commissioned in advance through a budget. On a relevant eligibility date, the Chronicle announces that the date restriction has ended; other prerequisites still apply. These are authored scenario restrictions, not historical innovation predictions. Full research and adoption dynamics remain later work.".into(),destination:Destination {tab:Tab::Policies,country:None,policy:None}});
        for technology in &technologies.definitions {
            let policy = technologies
                .policy_requirements
                .iter()
                .find(|r| r.technologies.contains(&technology.id))
                .map(|r| r.policy.clone());
            entries.push(Entry {title:technology.name.clone(),body:format!("Authored technology prerequisite. Earliest commissioning date {}. Eligible countries: {}. Source {} / {}. This restriction grants no cash, capacity or researched knowledge.",technology.available_from,technology.eligible_countries.iter().map(|c|c.0.as_str()).collect::<Vec<_>>().join(", "),technology.source,technology.source_version),destination:Destination {tab:Tab::Policies,country:None,policy}});
        }
    }
    if content.cash_management.is_some() {
        entries.push(Entry {title:"Temporary retention of actual national cash".into(),body:"Treasury can protect a bounded amount of real national cash for a later decision. A cash-blocked policy offers Plan cash retention for this decision to prefill its full implementation cost in Treasury. Preparing a draft spends nothing and leaves existing retention unchanged. Compare planned decision returns to the chosen policy and value for normal approval after actual funding; authority and pending-programme restrictions still apply. Unfunded targets create no money. Existing capital reservations and protected local claims retain first claim. Actual tax receipts can fill the target before maturity, interest, wages and welfare payments; deferring operating payments creates arrears and unpaid dated principal remains outstanding. Creation has an approval cost and reuse cooldown. Release while paused before proposing a decision through normal funding, backing and delay rules. Expiry or a new government automatically releases actual cash, without cancelling any obligation. Funding scenarios exclude current retained cash and hold protection constant rather than predicting future capture or expiry.".into(),destination:Destination {tab:Tab::Treasury,country:None,policy:None}});
    }
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
    if content.rules >= 24 {
        entries.push(Entry {title:"Seat allocation and coalition leadership".into(),body:"Where governing-party elections are configured, this scenario uses an authored 100-seat model. The incumbent party receives an approval-based share bounded to 15–80 percent. Remaining seats are divided evenly among other registered parties, with leftover seats assigned by stable party reference. Below 50 percent, those parties form the successor coalition. Its largest party leads; equal seat totals choose the lowest stable party reference. Display names and definition-row order do not settle these choices. This is a game abstraction, not a national electoral-law model. Earlier campaigns retain their original rules.".into(),destination:Destination{tab:Tab::Politics,country:None,policy:None}});
    }
    if content.rules >= 23 {
        entries.push(Entry {title:"Inspect a programme from its event".into(),body:"Programme events in Chronicle identify the exact proposal, amendment, consent/refusal, delivery, withdrawal or observed review. Choose Inspect programme to open that record's pending controls or historical evidence, then Show all programmes to restore the list. Inspection issues no order and reveals no foreign private records. An event records a past stage; the programme view shows its current state. Observed outcome reviews also reflect other policies, prices and shocks.".into(),destination:Destination{tab:Tab::Chronicle,country:None,policy:None}});
    }
    if content.rules >= 22 {
        entries.push(Entry {title:"Stable chronicle references".into(),body:"Your retained chronicle entries have campaign-scoped references such as AUR#12. Accepted command receipts identify their immediate outcome entries. Copy a reference or find it in Chronicle. Command acceptance records an order; consent and delivery may still be pending. Lookup never grants access to foreign private events. The history retains up to 400 entries across the world; expired references are not reused or reconstructed as known records. Existing campaigns retain their original rules.".into(),destination:Destination{tab:Tab::Chronicle,country:None,policy:None}});
    }
    if content.reserves.is_some() {
        entries.push(Entry { title:"National reserves and supply resilience".into(), body:"Costed food and energy import contracts reserve real national purchase cash. Institutional consent and spare import capacity are required; civilian physical needs go first. Fixed quantities and prices are explicit scenario conventions. National stocks automatically cover post-import physical shortfalls, with food storage loss and continued household-affordability limits. Cancel or expire unfilled contracts to release unspent money; this does not refund delivered goods or administration.".into(), destination:Destination{tab:Tab::Treasury,country:None,policy:None}});
    }
    if content.party_preferences.is_some() {
        entries.push(Entry { title: "Party preferences and operating policy".into(), body: "Explicit party fiscal ranges and relative importance are separate from ideology labels. Once tax, public-service or welfare settings operate, policy distance can lower the backing target at monthly political reviews. Legislatures weight governing coalition seats; party leadership uses the ruling party. Council and executive routes keep their independent backing rules. Unknown positions remain unknown and create no modeled pressure. Review positions and operating alignment in Institutions, compare programme/budget previews, or pay for an ordinary governing-support agreement.".into(), destination: Destination { tab: Tab::Politics, country: None, policy: None } });
    }
    entries.extend(content.policies.iter().map(|policy| Entry {
        title: format!("Policy · {}", policy.name),
        body: format!("{} {} Affected groups: {}. Allowed values {}–{}; base implementation {} days; capital cost {} basis points of opening GDP. Authority and administration may add delay: use the live preview before committing.", policy.id, policy.description, policy.groups, policy.min, policy.max, policy.days, policy.cost_bps.units_i64()),
        destination: Destination { tab: Tab::Policies, country: None, policy: Some(policy.id.clone()) },
    }));
    entries.extend(content.countries.iter().map(|country| Entry {
        title: format!("Country · {} ({})", country.name, country.id.0),
        body: format!("Opening reference at {}: capital {}; executive {} ({}); currency {}; {:?}. {} Data quality: {}. These are bookmark definitions; the overview shows campaign developments and source records.", content.start, country.capital, country.executive, country.office, country.currency, country.regime, country.government.label(), country.quality),
        destination: Destination { tab: Tab::Overview, country: Some(country.id.clone()), policy: None },
    }));
    entries.extend(bookmark_entries(content));
    entries
}
fn bookmark_entries(content: &Content) -> Vec<Entry> {
    let mut entries = Vec::new();
    for country in &content.countries {
        for party in &country.parties {
            entries.push(Entry {
                title:format!("Party · {} ({})",party.name,party.id),
                body:format!("Opening reference at {} · {} ({}). Leader label: {}. Ideology: {}. Opening seats: {}. Party reference {}. Seats and profiles are authored scenario definitions, not current support or polling. Identity source {}; country data quality {}.",content.start,country.name,country.id.0,party.leader,party.ideology,party.seats,party.id,country.identity_source,country.quality),
                destination:Destination {tab:Tab::Politics,country:Some(country.id.clone()),policy:None}
            });
        }
    }
    let Some(catalogue) = &content.identities else {
        return entries;
    };
    let date = content.start.parse().expect("validated bookmark");
    let active_offices: Vec<_> = catalogue
        .offices
        .iter()
        .filter(|office| office.validity.contains(date))
        .collect();
    for person in &catalogue.people {
        let mut roles: Vec<_> = active_offices
            .iter()
            .filter(|office| {
                catalogue.terms.iter().any(|term| {
                    term.office == office.id
                        && term.validity.contains(date)
                        && match &term.holder {
                            OfficeHolder::Person { person: id } => id == &person.id,
                            OfficeHolder::Collective { members, .. } => {
                                members.contains(&person.id)
                            }
                            OfficeHolder::Vacant => false,
                        }
                })
            })
            .copied()
            .collect();
        roles.sort_by(|a, b| a.country.cmp(&b.country).then(a.id.cmp(&b.id)));
        let Some(first) = roles.first() else {
            continue;
        };
        let labels = roles
            .iter()
            .map(|office| {
                format!(
                    "{} ({}): {} ({})",
                    content
                        .country(&office.country)
                        .expect("validated office country")
                        .name,
                    office.country.0,
                    office.title,
                    office.id.0
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        entries.push(Entry {
            title:format!("Person · {} ({})",person.name,person.id.0),
            body:format!("Person reference {} · active reference roles or listed collective memberships at {}: {}. Quality {:?}; source {} / {}. This is bookmark identity data, not a live appointment, biography or a scheduled handover.",person.id.0,content.start,labels,person.quality,person.source,person.source_version),
            destination:Destination {tab:Tab::Overview,country:Some(first.country.clone()),policy:None}
        });
    }
    for country in &content.countries {
        for record in content.bookmark_offices(&country.id) {
            let office = active_offices
                .iter()
                .find(|office| office.id == record.id)
                .expect("bookmark office");
            let detail=record.term.as_ref().map_or_else(||"Incumbency unspecified at this bookmark; this is not an explicit vacancy.".into(),|term| {
                let members=match &term.holder {
                    OfficeHolder::Collective{members,..} if members.is_empty()=>" Collective membership unspecified.".to_string(),
                    OfficeHolder::Collective{members,..}=>format!(" Listed collective members: {}.",members.iter().map(|id|catalogue.people.iter().find(|person|&person.id==id).expect("validated person").name.as_str()).collect::<Vec<_>>().join(", ")),
                    _=>String::new()
                };
                format!("Holder: {}.{} Term {} from {} until {} (end excluded); quality {:?}; source {} / {}.",record.holder,members,term.id,term.validity.valid_from,term.validity.valid_to.map_or_else(||"unspecified".into(),|date|date.to_string()),term.quality,term.source,term.source_version)
            });
            entries.push(Entry {
                title:format!("Office · {} · {} ({})",record.title,country.name,record.id.0),
                body:format!("Office reference {} · {} · {} ({}) at {}. {} Office coverage from {} until {} (end excluded); quality {:?}; source {} / {}. Reference dates do not schedule campaign outcomes or grant governing authority.",record.id.0,record.role.label(),country.name,country.id.0,content.start,detail,office.validity.valid_from,office.validity.valid_to.map_or_else(||"unspecified".into(),|date|date.to_string()),office.quality,office.source,office.source_version),
                destination:Destination{tab:Tab::Overview,country:Some(country.id.clone()),policy:None}
            });
        }
    }
    for term in catalogue
        .currencies
        .iter()
        .filter(|term| term.validity.contains(date))
    {
        let country = content
            .country(&term.country)
            .expect("validated currency country");
        entries.push(Entry {
            title:format!("Currency · {} · {}",term.code,country.name),
            body:format!("Primary-currency reference {} · {} ({}) at {}. Record {} from {} until {} (end excluded); quality {:?}; source {} / {}. Monetary values remain game reporting credits; no exchange rate or future currency transition is inferred.",term.code,country.name,country.id.0,content.start,term.id,term.validity.valid_from,term.validity.valid_to.map_or_else(||"unspecified".into(),|date|date.to_string()),term.quality,term.source,term.source_version),
            destination:Destination{tab:Tab::Overview,country:Some(country.id.clone()),policy:None}
        });
    }
    entries
}
fn matches(entry: &Entry, query: &str) -> bool {
    let haystack = crate::hydrology::search_key(&format!("{} {}", entry.title, entry.body));
    query
        .split_whitespace()
        .all(|word| haystack.contains(&crate::hydrology::search_key(word)))
}
impl Help {
    pub fn show(&mut self, ui: &mut egui::Ui, content: &Content) -> Option<Destination> {
        ui.heading("Game reference");
        ui.label("Search rules, policies, parties and the loaded bookmark's active person/office/currency references. Reference records stay distinct from live campaign state. Opening a screen changes inspection context; it does not issue an order.");
        ui.horizontal_wrapped(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .return_key(None)
                    .id_salt("game_reference_query")
                    .hint_text("Search rules, names, parties or stable references")
                    .desired_width(540.0),
            );
            if ui.button("Clear search").clicked() {
                self.query.clear();
                edit.request_focus();
            }
        });
        let results: Vec<_> = entries(content)
            .into_iter()
            .filter(|entry| matches(entry, &self.query))
            .collect();
        ui.label(format!(
            "{} matching {}",
            results.len(),
            if results.len() == 1 {
                "entry"
            } else {
                "entries"
            }
        ));
        if results.is_empty() {
            ui.label("No matching entries. Try a shorter term, an unaccented name or a country/record reference.");
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
    fn party_and_identity_search_preserves_loaded_reference_dates_labels_and_missingness() {
        let preserved = Content::stable_elections_bundled().unwrap();
        let parties = bookmark_entries(&preserved);
        assert_eq!(parties.len(), 34);
        let french = parties
            .iter()
            .find(|entry| matches(entry, "francois hollande"))
            .unwrap();
        assert_eq!(french.destination.country.as_ref().unwrap().0, "FRA");
        assert!(french.destination.tab == Tab::Politics);
        assert!(french.body.contains("François Hollande"));
        assert!(french.body.contains("Opening seats"));
        assert!(parties.iter().any(|entry| matches(entry, "KEIZO OBUCHI")));
        let content = Content::bundled().unwrap();
        let records = bookmark_entries(&content);
        for (prefix, count) in [
            ("Party ·", 34),
            ("Person ·", 37),
            ("Office ·", 49),
            ("Currency ·", 15),
        ] {
            assert_eq!(
                records
                    .iter()
                    .filter(|entry| entry.title.starts_with(prefix))
                    .count(),
                count
            );
        }
        let collective = records
            .iter()
            .find(|entry| {
                entry.title.starts_with("Office ·") && matches(entry, "DEU:greens:leadership")
            })
            .unwrap();
        assert!(
            collective
                .body
                .contains("Collective membership unspecified")
        );
        assert!(collective.body.contains("2000-01-02 (end excluded)"));
        assert!(
            !records
                .iter()
                .any(|entry| entry.title.starts_with("Person · Collective"))
        );
        assert!(
            records
                .iter()
                .any(|entry| entry.title.starts_with("Currency ·")
                    && matches(entry, "FRA EUR")
                    && entry.body.contains("no exchange rate"))
        );

        let dated =
            Content::from_json(include_str!("../../../data/synthetic-dated-test.json")).unwrap();
        let records = bookmark_entries(&dated);
        let person = records
            .iter()
            .find(|entry| entry.title.starts_with("Person · Fictional Steward C"))
            .unwrap();
        assert!(person.body.contains("Aurora Council"));
        assert!(!person.body.contains("First Minister"));
        assert!(
            records
                .iter()
                .any(|entry| entry.title.starts_with("Office · Assembly Speaker")
                    && entry.body.contains("Vacant"))
        );
        assert!(records.iter().any(
            |entry| entry.title.starts_with("Office · Independent Auditor")
                && entry.body.contains("Incumbency unspecified")
                && entry.body.contains("not an explicit vacancy")
        ));
    }
    #[test]
    fn reference_destinations_follow_stable_keys_after_definition_reordering_and_label_changes() {
        let mut content = Content::bookmark_identities_bundled().unwrap();
        content.countries.reverse();
        let catalogue = content.identities.as_mut().unwrap();
        catalogue.people.reverse();
        catalogue.offices.reverse();
        catalogue.terms.reverse();
        catalogue
            .people
            .iter_mut()
            .find(|person| person.id.0 == "FRA:ps:opening-person")
            .unwrap()
            .name = "Revised person display".into();
        let records = bookmark_entries(&content);
        let person = records
            .iter()
            .find(|entry| {
                entry.title.starts_with("Person ·") && matches(entry, "FRA:ps:opening-person")
            })
            .unwrap();
        assert!(person.title.contains("Revised person display"));
        assert_eq!(person.destination.country.as_ref().unwrap().0, "FRA");
        let office = records
            .iter()
            .find(|entry| {
                entry.title.starts_with("Office ·") && matches(entry, "FRA:ps:leadership")
            })
            .unwrap();
        assert_eq!(office.destination.country.as_ref().unwrap().0, "FRA");
        assert!(office.body.contains("Revised person display"));
    }
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
            .find(|entry| {
                entry.destination.policy.as_ref().map(PolicyId::as_str) == Some("agriculture")
            })
            .unwrap();
        assert!(policy.destination.tab == Tab::Policies);
        assert!(policy.destination.country.is_none());
    }
}
