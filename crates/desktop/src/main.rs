mod advance;
mod analysis;
mod cashflows;
mod charts;
mod geometry;
mod help;
mod inspector;
mod map;
mod markets;
mod settings;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use sim_core::{persistence, *};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const MINT: Color32 = Color32::from_rgb(104, 224, 186);
const MUTED: Color32 = Color32::from_rgb(148, 170, 181);
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Overview,
    Policies,
    Treasury,
    Society,
    Analysis,
    Settings,
    Politics,
    Chronicle,
    Guide,
    Inspector,
}
struct GameApp {
    content: Content,
    world: Option<World>,
    selected: CountryId,
    tab: Tab,
    map: map::WorldMap,
    search: String,
    seed: u64,
    scenario: Scenario,
    policy: String,
    value: i32,
    status: String,
    speed: u32,
    last_tick: Instant,
    pause_alerts: bool,
    save_path: String,
    money: i64,
    sort_gdp: bool,
    show_ledger: bool,
    recovery: Vec<persistence::RecoveryCandidate>,
    analysis: analysis::Analysis,
    pending: Option<advance::AdvanceJob>,
    target_date: String,
    preferences: settings::Preferences,
    developer_mode: bool,
    inspector: inspector::Inspector,
    help: help::Help,
    cashflows: cashflows::CashFlows,
    bundled_catalog: bool,
}
impl GameApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        content: Content,
        developer_mode: bool,
        bundled_catalog: bool,
    ) -> Self {
        let mut app = Self::from_context(&cc.egui_ctx, content, settings::Preferences::load());
        app.developer_mode = developer_mode;
        app.bundled_catalog = bundled_catalog;
        app
    }
    fn from_context(
        ctx: &egui::Context,
        content: Content,
        preferences: settings::Preferences,
    ) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(18, 29, 39);
        visuals.window_fill = Color32::from_rgb(23, 37, 48);
        visuals.selection.bg_fill = Color32::from_rgb(37, 91, 83);
        visuals.selection.stroke = Stroke::new(1.0_f32, MINT);
        ctx.set_visuals(visuals);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        ctx.set_style(style);
        let selected = content.countries[0].id.clone();
        let target_date =
            date((content.start_day().expect("validated bookmark") + 30).min(MAX_DAY));
        ctx.set_zoom_factor(preferences.active.ui_scale);
        let save_path = format!(
            "saves/campaign-v{}.json",
            if content.rules == 3 { 4 } else { 3 }
        );
        Self {
            content,
            world: None,
            selected,
            tab: Tab::Overview,
            map: map::WorldMap::new(),
            search: String::new(),
            seed: 2000,
            scenario: Scenario::Sandbox,
            policy: "energy".into(),
            value: 1,
            status: "Choose a country and begin your mandate, or load a saved campaign.".into(),
            speed: 0,
            last_tick: Instant::now(),
            pause_alerts: true,
            save_path,
            money: 1000,
            sort_gdp: false,
            show_ledger: false,
            recovery: Vec::new(),
            analysis: analysis::Analysis::default(),
            pending: None,
            target_date,
            preferences,
            developer_mode: false,
            inspector: inspector::Inspector::default(),
            help: help::Help::default(),
            cashflows: cashflows::CashFlows::default(),
            bundled_catalog: false,
        }
    }
    fn advance(&mut self, days: u32) {
        self.start_advance(days, self.pause_alerts);
    }
    fn start_advance(&mut self, days: u32, important: bool) {
        if !self.recovery.is_empty() {
            self.speed = 0;
            self.status = "Close save recovery before advancing.".into();
            return;
        }
        if self.pending.is_some() {
            self.status =
                "Already advancing; stop at a day boundary to issue another order.".into();
            return;
        }
        let Some(world) = self.world.as_ref() else {
            return;
        };
        match advance::AdvanceJob::start(world.clone(), self.content.clone(), days, important) {
            Ok(job) => {
                self.pending = Some(job);
                self.status =
                    format!("Advancing up to {days} days. You can stop at the next day boundary.");
            }
            Err(error) => {
                self.status = error;
                self.speed = 0;
            }
        }
    }
    fn pause(&mut self) {
        self.speed = 0;
        if let Some(job) = &self.pending {
            job.stop();
        }
    }
    fn poll_advance(&mut self) {
        let Some(job) = self.pending.as_ref() else {
            return;
        };
        let result = job.poll();
        let start = job.start_day;
        let requested = job.requested;
        match result {
            Ok(None) => {}
            Err(error) => {
                self.pending = None;
                self.speed = 0;
                self.status = error;
            }
            Ok(Some(outcome)) => {
                let days = outcome.world.day - start;
                let interrupted = days < requested;
                self.world = Some(outcome.world);
                self.pending = None;
                if interrupted || outcome.stopped_for_event || outcome.error.is_some() {
                    self.speed = 0;
                }
                self.status = outcome.error.unwrap_or_else(|| {
                    format!(
                        "Advanced {days} days to {}{}",
                        date(self.world.as_ref().expect("world").day),
                        if interrupted {
                            " · stopped at an event or requested boundary"
                        } else {
                            ""
                        }
                    )
                });
                let world = self.world.as_ref().expect("world");
                if world.day / 30 != start / 30 {
                    let path = persistence::autosave_path(world);
                    let result = std::fs::create_dir_all("saves")
                        .map_err(|e| e.to_string())
                        .and_then(|()| persistence::save(&path, world, &self.content));
                    if let Err(error) = result {
                        self.speed = 0;
                        self.status = format!("Autosave failed: {error}");
                    }
                }
            }
        }
    }
    fn issue(&mut self, action: Action) {
        if self.pending.is_some() {
            self.status = "Stop advancing before issuing a command.".into();
            return;
        }
        self.speed = 0;
        if let Some(w) = &mut self.world {
            self.status = match w.issue(&self.content, action) {
                Ok(()) => "Command accepted. Inspect the Chronicle and pending commitments.".into(),
                Err(e) => format!("Rejected: {e}"),
            };
        }
    }
    fn save(&mut self) {
        if self.pending.is_some() {
            self.pause();
            self.status = "Stopping at a day boundary; save once advancement finishes.".into();
            return;
        }
        self.speed = 0;
        if let Some(w) = &self.world {
            let path = PathBuf::from(&self.save_path);
            let result = if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())
            } else {
                Ok(())
            };
            self.status = match result.and_then(|()| persistence::save(&path, w, &self.content)) {
                Ok(()) => format!(
                    "Saved {} in format {}. Previous snapshot retained as .json.bak; converted older originals are archived.",
                    path.display(),
                    persistence::snapshot_schema(w)
                ),
                Err(e) => e,
            };
        }
    }
    fn load(&mut self) {
        if self.pending.is_some() {
            self.pause();
            self.status = "Stopping at a day boundary; load once advancement finishes.".into();
            return;
        }
        self.speed = 0;
        let path = PathBuf::from(&self.save_path);
        let loaded = if self.bundled_catalog {
            persistence::load_bundled(&path)
        } else {
            persistence::load(&path, &self.content).map(|world| (self.content.clone(), world))
        };
        match loaded {
            Ok((content, w)) => {
                self.content = content;
                self.selected = w.player.clone();
                self.target_date = date((w.day + 30).min(MAX_DAY));
                self.world = Some(w);
                self.status = if self.content.rules == 2 {
                    "Classic fixed-price campaign loaded. Simulation paused."
                } else {
                    "Relative-price campaign loaded. Simulation paused."
                }
                .into();
                self.recovery.clear();
            }
            Err(e) => {
                self.status = e;
                self.inspect_recovery();
            }
        }
    }
    fn inspect_recovery(&mut self) {
        self.pause();
        let path = PathBuf::from(&self.save_path);
        self.recovery = if self.bundled_catalog {
            persistence::recovery_candidates_bundled(&path)
        } else {
            persistence::recovery_candidates(&path, &self.content)
        };
    }
    fn playback_label(&self) -> String {
        if let Some(job) = &self.pending {
            if job.is_stopping() {
                "Stopping at next day boundary".into()
            } else {
                format!(
                    "Advancing {} / {} days",
                    job.completed.load(std::sync::atomic::Ordering::Relaxed),
                    job.requested
                )
            }
        } else if self.speed == 0 {
            "Paused".into()
        } else {
            format!("Running · {} days per step", self.speed)
        }
    }
    fn top(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("command_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("STATECRAFT").size(23.0).strong().color(MINT));
                if let Some(w) = &self.world {
                    ui.strong(date(w.day));
                    ui.label(format!("Governing {} · {}", w.player.0, w.scenario.id()));
                    ui.strong(self.playback_label());
                    if ui.button(format!("{} [{}]", if self.speed == 0 && self.pending.is_none() { "Play" } else { "Pause" }, self.preferences.active.pause.name())).clicked() {
                        if self.speed > 0 || self.pending.is_some() { self.pause(); } else { self.speed = 1; }
                        self.last_tick = Instant::now();
                    }
                } else { ui.label(format!("{} · Offline single player", self.content.start)); }
                if self.developer_mode { ui.colored_label(Color32::LIGHT_YELLOW, "Developer mode"); }
                if self.world.as_ref().is_some_and(World::is_developer_modified) { ui.colored_label(Color32::LIGHT_YELLOW, "DEVELOPER-MODIFIED"); }
                if ui.button("Settings [F1]").clicked() { self.pause(); self.tab = Tab::Settings; }
            });
            if self.world.is_some() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Automatic step:");
                    for (days, label) in [(1, "1 day"), (7, "7 days"), (30, "30 days")] {
                        if ui.selectable_label(self.speed == days, label).on_hover_text("Start automatic progression at this many days per step. Important events can stop it earlier.").clicked() { self.speed = days; }
                    }
                    for (days, label) in [(1, "+1 day"), (30, "+30 days")] {
                        if ui.add_enabled(self.pending.is_none(), egui::Button::new(label)).clicked() { self.speed = 0; self.advance(days); }
                    }
                    if ui.add_enabled(self.pending.is_none(), egui::Button::new("Next event")).clicked() {
                        self.speed = 0;
                        let remaining = MAX_DAY - self.world.as_ref().expect("world").day;
                        self.start_advance(365.min(remaining), true);
                    }
                    ui.add(egui::TextEdit::singleline(&mut self.target_date).desired_width(95.0).hint_text("YYYY-MM-DD"));
                    if ui.add_enabled(self.pending.is_none(), egui::Button::new("Advance to date")).clicked() {
                        match parse_date(&self.target_date) {
                            Ok(day) => {
                                let current = self.world.as_ref().expect("world").day;
                                if day < current { self.status = "Choose a future date; load a save to return to an earlier state.".into(); }
                                else { self.speed = 0; self.advance(day - current); }
                            }
                            Err(error) => self.status = error,
                        }
                    }
                });
            }
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&self.status).color(MINT));
            });
        });
    }
    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .resizable(true)
            .default_width(230.0)
            .width_range(190.0..=320.0)
            .show(ctx, |ui| {
                let panel_height=ui.available_height();
                egui::ScrollArea::vertical().id_salt("sidebar_scroll").show(ui,|ui| {
                ui.add_space(10.0);
                ui.label(RichText::new("GOVERNMENT DESK").color(MUTED).small());
                for (tab, label) in [
                    (Tab::Overview, "01  World & overview"),
                    (Tab::Policies, "02  Policies & delivery"),
                    (Tab::Society, "03  Households & needs"),
                    (Tab::Treasury, "04  Treasury & accounts"),
                    (Tab::Analysis, "05  Trends & comparisons"),
                    (Tab::Politics, "06  Institutions & parties"),
                    (Tab::Chronicle, "07  Chronicle & AI"),
                    (Tab::Guide, "08  How to play"),
                    (Tab::Settings,"09  Controls & accessibility"),
                ] {
                    if ui.selectable_value(&mut self.tab, tab, label).changed()&&tab==Tab::Settings {self.pause();}
                }
                if self.developer_mode && ui.selectable_value(&mut self.tab, Tab::Inspector, "Developer inspector").changed() {self.pause();}
                ui.separator();
                ui.label(RichText::new("COUNTRY DIRECTORY").color(MUTED).small());
                ui.text_edit_singleline(&mut self.search);
                ui.checkbox(&mut self.sort_gdp, "Sort by opening GDP");
                egui::ScrollArea::vertical()
                    .id_salt("country_list")
                    .max_height((panel_height-650.0).clamp(80.0,200.0))
                    .show(ui, |ui| {
                        let mut countries: Vec<_> = self.content.countries.iter().collect();
                        if self.sort_gdp {
                            countries.sort_by_key(|c| std::cmp::Reverse(c.gdp));
                        }
                        let query = self.search.to_lowercase();
                        for d in countries {
                            if !format!("{} {} {}", d.name, d.id.0, d.executive)
                                .to_lowercase()
                                .contains(&query)
                            {
                                continue;
                            }
                            ui.selectable_value(
                                &mut self.selected,
                                d.id.clone(),
                                format!("{}  {}", d.id.0, d.name),
                            );
                        }
                    });
                ui.separator();
                if let Some(world) = &self.world {
                    if ui.button("Return to my country").clicked() {
                        self.selected = world.player.clone();
                    }
                    ui.checkbox(&mut self.pause_alerts, "Pause on important events");
                }
                ui.label("Save file / recovery .bak path");
                ui.add(egui::TextEdit::singleline(&mut self.save_path).id(egui::Id::new("campaign_save_path")));
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.world.is_some(), egui::Button::new("Save"))
                        .clicked()
                    {
                        self.save();
                    }
                    if ui.button("Load").clicked() {
                        self.load();
                    }
                });
                if ui.button("Inspect recovery snapshots").clicked(){self.inspect_recovery();}
                ui.small(format!("{} save · {} load · {} next event · Tab to navigate",self.preferences.active.save.name(),self.preferences.active.load.name(),self.preferences.active.next_event.name()));
                ui.small(
                    "Sourced reference GDP/population are identified in the overview; remaining values are authored.",
                );
                });
            });
    }
    fn developer_inspector(&mut self, ui: &mut egui::Ui) {
        if !self.developer_mode {
            ui.label("Developer inspection is disabled. Launch with --developer to enable it.");
            return;
        }
        if let Some(edit) = self.inspector.show(
            ui,
            &self.content,
            self.world.as_ref(),
            &self.selected,
            self.pending.is_none(),
        ) {
            self.speed = 0;
            if self.pending.is_none()
                && let Some(world) = &mut self.world
            {
                self.status = match world.issue_developer(
                    &self.content,
                    self.selected.clone(),
                    edit,
                ) {
                    Ok(()) => {
                        "Developer edit recorded. Campaign permanently marked developer-modified."
                            .into()
                    }
                    Err(error) => error,
                };
            }
        }
    }
    fn overview(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("THE WORLD AT YOUR DESK").color(MUTED).small());
        let d = self
            .content
            .country(&self.selected)
            .expect("selection")
            .clone();
        ui.heading(&d.name);
        ui.label(format!(
            "{} · {:?} · {} {}",
            d.capital,
            d.regime,
            d.office,
            self.world
                .as_ref()
                .map_or(d.executive.as_str(), |w| w.countries[&self.selected]
                    .executive
                    .as_str())
        ));
        self.map.show(
            ui,
            &self.content,
            self.world.as_ref(),
            &mut self.selected,
            (ui.available_height() * 0.48).clamp(200.0, 420.0),
        );
        if self.selected != d.id {
            return;
        }
        egui::CollapsingHeader::new("Opening data & provenance").show(ui,|ui|{
            let facts:Vec<_>=self.content.facts.iter().filter(|f|f.country==d.id).collect();
            if facts.is_empty(){ui.label("Population and GDP are authored approximations for this country.");}
            for fact in facts {ui.label(format!("{}: {} {} ({} observation; source {}). Applied {}. {}",fact.field,fact.original_value,fact.original_units,fact.observation_period,fact.source,fact.applied_value,fact.transformation));ui.small(format!("Pinned mirror revision {}. Primary API not independently reconciled.",fact.source_version));}
            ui.small("Debt, capacities, household profiles, seats, preferences and political rules are authored. Identity names remain a starter curation; all simulated outcomes branch from the bookmark.");
        });
        let offices = self.content.bookmark_offices(&self.selected);
        if !offices.is_empty() {
            ui.collapsing(format!("Reference offices at {}", self.content.start), |ui| {
                ui.label("Reference terms initialize the bookmark. Campaign succession follows simulated institutions; future reference records do not force a change.");
                ui.label(format!("Primary currency identity: {}. Monetary values use game reporting credits; exchange rates are not modeled.", d.currency));
                for office in &offices {
                    ui.strong(format!("{} · {}: {}", office.role.label(), office.title, office.holder));
                    if let Some(term) = &office.term {
                        ui.small(format!("Reference from {} until {} (end excluded) · {:?} · source {}@{}", term.validity.valid_from,
                            term.validity.valid_to.map_or_else(|| "open-ended".into(), |date| date.to_string()), term.quality, term.source, term.source_version));
                    } else { ui.small("No incumbent record at the bookmark; this is unknown, not a recorded vacancy."); }
                }
            });
        }
        if self.foreign_report(ui) {
            return;
        }
        if self.world.is_none() {
            ui.add_space(10.0);
            ui.heading("Your first mandate");
            ui.label(format!("Keep food and energy needs at 90%, approval at 50%, clear arrears and debt below opening GDP by {}. Continue as the country after elections or leadership changes.", date(self.content.review_day().expect("validated bookmark"))));
            ui.horizontal_wrapped(|ui| {
                metric(
                    ui,
                    "Population",
                    format!("{:.1} million", d.population as f64 / 1_000_000.0),
                );
                metric(ui, "Opening GDP", format!("{}m credits/year", d.gdp));
                metric(
                    ui,
                    "Energy / food",
                    format!("{}% / {}% domestic", d.energy, d.food),
                );
                metric(ui, "Collection capacity", format!("{}%", d.capacity));
            });
            ui.label(format!("Governing route: {}. Policy construction and fiscal constraints apply in every regime.",d.government.label()));
            egui::ComboBox::from_label("Scenario")
                .selected_text(self.scenario.id())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.scenario, Scenario::Sandbox, "Sandbox");
                    ui.selectable_value(
                        &mut self.scenario,
                        Scenario::FiscalSupplyCrisis,
                        "Fiscal and supply crisis (authored)",
                    );
                });
            if self.scenario == Scenario::FiscalSupplyCrisis {
                ui.colored_label(Color32::LIGHT_YELLOW,"Authored crisis overrides: domestic food/energy 40%, tax 10%, services 18%, welfare 12%, administration 60%, approval 20%. GDP and population source observations are retained.");
            }
            ui.horizontal(|ui|{
                ui.label("World seed");ui.add(egui::DragValue::new(&mut self.seed));
                if ui.button(RichText::new(format!("Begin as {} >",d.name)).color(MINT)).clicked(){
                    match World::new_scenario(&self.content,d.id.clone(),self.seed,self.scenario){Ok(w)=>{self.world=Some(w);self.money=(d.gdp/100).max(1);self.status="Mandate started, paused. Advance a day to collect the first fiscal and supply report.".into();},Err(e)=>self.status=e}
                }
            });
        } else if let Some(w) = &self.world {
            let c = &w.countries[&self.selected];
            ui.horizontal_wrapped(|ui| {
                metric(ui, "Treasury", format!("{}m", c.accounts.treasury.0));
                metric(
                    ui,
                    "Debt / opening GDP",
                    format!("{}%", c.debt.0 * 100 / d.gdp),
                );
                metric(ui, "Approval", format!("{}%", c.approval));
                metric(
                    ui,
                    "Energy / food delivered",
                    if w.day == self.content.start_day().expect("validated bookmark") {
                        "Awaiting first settlement".into()
                    } else {
                        format!(
                            "{}% / {}%",
                            c.report.energy_coverage, c.report.food_coverage
                        )
                    },
                );
            });
            if w.day > self.content.start_day().expect("validated bookmark") {
                ui.label(format!(
                    "Last day: output {}m · tax receipts {}m · payments {}m · unpaid {}m",
                    c.report.output.0,
                    c.report.revenue.0,
                    c.report.spending.0 + c.report.interest.0,
                    c.report.unpaid.0
                ));
                if c.report.food_coverage < 90 || c.report.energy_coverage < 90 {
                    ui.colored_label(Color32::LIGHT_YELLOW,"Supply gap: compare a capacity program with fiscal headroom under Policies.");
                }
                if c.arrears.0 > 0 {
                    ui.colored_label(
                        Color32::LIGHT_RED,
                        "Unpaid obligations: borrow or lower spending to restore funded delivery.",
                    );
                }
            }
            if !c.history.is_empty() {
                sparkline(ui, &c.history);
            }
            ui.label(format!(
                "{} pending commitments · {} · next election {}",
                c.projects.len(),
                c.executive,
                c.next_election
                    .map(date)
                    .unwrap_or_else(|| "Elite backing reviewed monthly".into())
            ));
            if let Some(event) = w.events.iter().rev().find(|e| e.country == self.selected) {
                ui.label(format!("{}  {}", date(event.day), event.text));
            }
            self.situation(ui);
        }
    }
    fn situation(&mut self, ui: &mut egui::Ui) {
        let Some(world) = &self.world else {
            return;
        };
        if self.selected != world.player
            || world.day == self.content.start_day().expect("validated bookmark")
        {
            return;
        }
        let c = world.player_country();
        let (energy, food, tax, available, gap) = (
            c.report.energy_coverage,
            c.report.food_coverage,
            c.tax,
            c.accounts.treasury.0 - c.committed_cash.0,
            c.report.spending.0 + c.report.interest.0 + c.report.unpaid.0 - c.report.revenue.0,
        );
        let rules = &self
            .content
            .country(&world.player)
            .expect("country")
            .government;
        let backing = if rules.route == DecisionRoute::Legislature {
            c.legislative_support
        } else {
            c.elite_support
        };
        let authority_blocked =
            backing < rules.threshold || c.council_support < rules.council_threshold;
        ui.separator();
        ui.heading("Cabinet situation brief");
        if authority_blocked {
            ui.label("Governing support is below the threshold for ordinary policy decisions. Negotiate support before committing to a new program.");
            if ui.button("Draft governing support talks").clicked() {
                self.policy = "coalition".into();
                self.value = 1;
                self.tab = Tab::Policies;
            }
        }
        if gap > 0 {
            ui.label(format!("Funding pressure: last-day obligations exceeded receipts by {gap}m. Available cash {available}m covers about {} days at that rate; this is a run-rate estimate, not a guarantee.",available/gap));
            ui.horizontal(|ui| {
                if ui.button("Compare wage-tax reform").clicked() {
                    self.policy = "income_tax".into();
                    self.value = (tax + 3).min(45);
                    self.tab = Tab::Policies;
                }
                if ui.button("Review financing").clicked() {
                    self.tab = Tab::Treasury;
                }
            });
        }
        if energy < 90 {
            ui.label(format!("Energy delivery {energy}%: supply is limiting feasible production. Capacity takes time and capital; imports have a quantity limit."));
            if ui.button("Draft energy capacity").clicked() {
                self.policy = "energy".into();
                self.value = 1;
                self.tab = Tab::Policies;
            }
        }
        if food < 90 {
            ui.label(format!("Food delivery {food}%: inspect both physical supply and each household group's purchasing power."));
            ui.horizontal(|ui| {
                if ui.button("Draft food resilience").clicked() {
                    self.policy = "agriculture".into();
                    self.value = 1;
                    self.tab = Tab::Policies;
                }
                if ui.button("Inspect household needs").clicked() {
                    self.tab = Tab::Society;
                }
            });
        }
        if gap <= 0 && energy >= 90 && food >= 90 {
            ui.label("Current essential needs and cash flow are stable. Review debt, compare future commitments and keep capacity for supply shocks.");
        }
        ui.small("Drafts still require the normal feasibility preview, authority and funding checks. A brief describes current pressures; it does not promise an optimal strategy.");
    }
    fn foreign_report(&self, ui: &mut egui::Ui) -> bool {
        let Some(w) = &self.world else {
            return false;
        };
        if self.selected == w.player {
            return false;
        }
        let report = w
            .public_report(&self.content, &self.selected)
            .expect("selected country");
        ui.heading(format!("{} · published country report", report.name));
        ui.label(format!(
            "Executive: {} · population {:.1}m · opening GDP {}m/year",
            report.executive,
            report.population.0 as f64 / 1_000_000.0,
            report.opening_gdp.0
        ));
        if let Some(latest) = report.latest {
            ui.label(format!("Statistical release {} ({} days old): approval {}%, needs {}%, debt {}m, daily output {}m.",date(latest.day),w.day-latest.day,latest.approval,latest.needs,latest.debt.0,latest.daily_output.0));
        } else {
            ui.label("No statistical release yet. The first report is due at a political/statistical review; missing observations are not zero.");
        }
        ui.label("Foreign treasury accounts, household finances, council deliberations and AI decision logs are outside your government's information view.");
        ui.label("Current public legislative seats");
        let definition = self.content.country(&self.selected).expect("country");
        for party in &definition.parties {
            ui.label(format!("{}: {} seats", party.name, report.seats[&party.id]));
        }
        true
    }
    fn policies(&mut self, ui: &mut egui::Ui) {
        if self.foreign_report(ui) {
            return;
        }

        ui.heading("Policies & implementation");
        ui.label("Compare a commitment, check its route and cash cost, then approve. The effect starts after implementation completes.");
        egui::ComboBox::from_label("Program")
            .selected_text(&self.content.policy(&self.policy).expect("policy").name)
            .show_ui(ui, |ui| {
                for p in &self.content.policies {
                    if ui
                        .selectable_value(&mut self.policy, p.id.clone(), &p.name)
                        .changed()
                    {
                        self.value = p.min;
                    }
                }
            });
        let p = self.content.policy(&self.policy).expect("policy").clone();
        self.value = self.value.clamp(p.min, p.max);
        if p.min != p.max {
            ui.add(egui::Slider::new(&mut self.value, p.min..=p.max).text(
                if self.policy == "income_tax" {
                    "% of wages"
                } else {
                    "% of annual GDP"
                },
            ));
        }
        ui.label(&p.description);
        ui.small(format!("Affected groups: {}", p.groups));
        let Some(w) = &self.world else {
            ui.label("Begin a mandate to preview policy feasibility.");
            return;
        };
        let c = &w.countries[&self.selected];
        ui.label(format!(
            "Operating levels: income tax {}% · services {}% GDP · welfare {}% GDP",
            c.tax, c.services, c.welfare
        ));
        ui.label(format!(
            "Domestic capacity: energy {}% · food {}% · administration {}%",
            c.energy_capacity, c.food_capacity, c.administration
        ));
        if self.selected != w.player {
            ui.label("Foreign government: select your own country to issue commands.");
            return;
        }
        match w.preview(&self.content, &self.selected, &self.policy, self.value) {
            Ok(preview) => {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.strong("Decision preview · forecast");
                    ui.label(format!(
                        "Cash now: {}m · delivery: {} days · annual net expenditure change: {:+}m",
                        preview.cost.0, preview.delay, preview.annual_budget_change.0
                    ));
                    ui.label(format!("Authority: {}", preview.route));
                    ui.label(format!("Paid upfront {}m; reserve {}m until consent in {} days. Reserved money cannot fund other obligations.",preview.upfront_payment.0,preview.cost.0-preview.upfront_payment.0,preview.consent_delay));
                    ui.strong("Direct household income change after delivery · forecast m/day");
                    for impact in &preview.household_impacts {ui.label(format!("{}: {:+}m",impact.group,impact.daily_disposable_change.0));}
                    ui.label(preview.explanation);
                });
                if ui
                    .button(
                        RichText::new(if preview.consent_delay == 0 {
                            "Authorize and fund commitment"
                        } else {
                            "Propose and reserve funds"
                        })
                        .color(MINT),
                    )
                    .clicked()
                {
                    self.issue(Action::Enact {
                        policy: self.policy.clone(),
                        value: self.value,
                    });
                }
            }
            Err(e) => {
                ui.colored_label(Color32::LIGHT_YELLOW, format!("Cannot approve: {e}"));
            }
        }
        ui.separator();
        ui.heading("Committed programs");
        if let Some(w) = &self.world {
            let c = &w.countries[&self.selected];
            if c.projects.is_empty() {
                ui.label("No programs in implementation.");
            }
            for project in &c.projects {
                let progress = w.day.saturating_sub(project.start_day) as f32
                    / (project.due_day - project.start_day) as f32;
                ui.label(format!(
                    "{} · level {} · committed {}m · due {}",
                    project.policy,
                    project.value,
                    project.cost.0,
                    date(project.due_day)
                ));
                ui.label(format!(
                    "Stage {:?} · reserved {}m · consent {}",
                    project.stage,
                    project.reserved.0,
                    date(project.consent_day)
                ));
                ui.add(egui::ProgressBar::new(progress).show_percentage());
            }
        }
    }
    fn society(&self, ui: &mut egui::Ui) {
        if self.foreign_report(ui) {
            return;
        }

        ui.heading("Households, employment & needs");
        ui.label("Groups partition population and household cash without duplication. Profiles, employment responses and baskets are authored gameplay assumptions; group totals reconcile to national accounts.");
        let Some(w) = &self.world else {
            ui.label("Begin a mandate and advance a day to inspect household outcomes.");
            return;
        };
        let c = &w.countries[&self.selected];
        let d = self.content.country(&self.selected).expect("country");
        if w.day == self.content.start_day().expect("validated bookmark") {
            ui.label("Opening state: advance one day for realized income, baskets and delivery.");
        } else {
            match living_standards(&c.households) {
                Ok(indicators) => {
                    ui.strong(format!(
                        "Realized household indicators · {} · daily flows",
                        date(w.day)
                    ));
                    let total = indicators.population.0.max(1) as f64;
                    ui.horizontal_wrapped(|ui| {
                        metric(
                            ui,
                            "Population in groups with income below basket",
                            format!(
                                "{:.1}%",
                                indicators.income_below_basket.0 as f64 * 100.0 / total
                            ),
                        );
                        metric(
                            ui,
                            "Population in groups with food below 90%",
                            format!(
                                "{:.1}%",
                                indicators.food_below_target.0 as f64 * 100.0 / total
                            ),
                        );
                        metric(
                            ui,
                            "Group-mean income Gini · 0–1",
                            indicators.group_mean_gini_bps.map_or_else(
                                || "Undefined".into(),
                                |gini| format!("{:.3}", f64::from(gini) / 10000.0),
                            ),
                        );
                    });
                    ui.label(format!("Disposable income {}m/day · essential basket {}m/day · remaining household cash {}m.", indicators.disposable_income.0, indicators.essential_basket.0, indicators.cash.0));
                    ui.small("These are disjoint group means, not individual poverty estimates. Savings can fund a basket when daily income is low; sufficient income does not guarantee food supply. The Gini excludes inequality within groups and is undefined when total income is zero or group means cannot be represented.");
                }
                Err(error) => {
                    ui.colored_label(Color32::LIGHT_YELLOW, error);
                }
            }
        }
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("households").striped(true).show(ui, |ui| {
                for label in [
                    "Group",
                    "Population",
                    "Employed / working age",
                    "Cash m",
                    "Disposable m/day",
                    "Basket m/day",
                    "Income / basket %",
                    "Cash / basket days",
                    "Food %",
                    "Approval %",
                    "Arrears m",
                ] {
                    ui.strong(label);
                }
                ui.end_row();
                for (h, definition) in c.households.iter().zip(&d.households) {
                    for value in [
                        definition.name.clone(),
                        h.population.0.to_string(),
                        format!("{} / {}", h.employed.0, h.working_age.0),
                        h.cash.0.to_string(),
                        (h.income.0 - h.tax.0).to_string(),
                        h.basket_cost.0.to_string(),
                        if h.basket_cost.0 > 0 {
                            format!(
                                "{:.0}",
                                (h.income.0 - h.tax.0) as f64 * 100.0 / h.basket_cost.0 as f64
                            )
                        } else {
                            "Unavailable".into()
                        },
                        if h.basket_cost.0 > 0 {
                            format!("{:.1}", h.cash.0 as f64 / h.basket_cost.0 as f64)
                        } else {
                            "Unavailable".into()
                        },
                        h.food_coverage.to_string(),
                        h.approval.to_string(),
                        h.arrears.0.to_string(),
                    ] {
                        ui.label(value);
                    }
                    ui.end_row();
                }
            });
        });
        ui.small("Income/basket compares realized daily income after tax to the modeled essential basket. Cash/basket is the remaining cash buffer at today's basket cost, ignoring future income and price changes.");
        ui.separator();
        ui.label(format!(
            "Cash reconciliation: groups {}m = household sector {}m. Population: {} = national {}.",
            c.households.iter().map(|h| h.cash.0).sum::<i64>(),
            c.accounts.households.0,
            c.households.iter().map(|h| h.population.0).sum::<u64>(),
            c.population.0
        ));
        ui.label("Welfare targets transfer-dependent households (80%) and working households (20%). Service payments go to public-service households. Private income follows the country's authored profile. Taxes are paid from each group's cash; consumption is limited by affordability and food supply.");
        for (h, definition) in c.households.iter().zip(&d.households) {
            if h.food_coverage < 90 {
                ui.colored_label(Color32::LIGHT_YELLOW,format!("{}: food delivery {}%. Compare welfare, wage-tax changes and domestic food capacity; inspect cash and arrears.",definition.name,h.food_coverage));
            }
        }
    }
    fn treasury(&mut self, ui: &mut egui::Ui) {
        if self.foreign_report(ui) {
            return;
        }

        ui.heading("Treasury & accounts");
        ui.label("Money: millions of constant-2000 USD-equivalent game credits. Annual GDP is an opening calibration; receipts and payments below are realized daily flows.");
        let Some(w) = &self.world else {
            ui.label("Begin a mandate to inspect accounts.");
            return;
        };
        let c = &w.countries[&self.selected];
        self.cashflows.show(ui, &self.content, w, &self.selected);
        markets::show(ui, &self.content, w, &self.selected);
        let d = self
            .content
            .country(&self.selected)
            .expect("country")
            .clone();
        egui::Grid::new("accounts")
            .striped(true)
            .min_col_width(180.0)
            .show(ui, |ui| {
                for (name, value) in [
                    ("Treasury cash", c.accounts.treasury.0),
                    ("Reserved policy capital", c.committed_cash.0),
                    (
                        "Available treasury",
                        c.accounts.treasury.0 - c.committed_cash.0,
                    ),
                    ("Household cash", c.accounts.households.0),
                    ("Business cash", c.accounts.firms.0),
                    ("External counterpart cash", c.accounts.external.0),
                    ("Public debt principal", c.debt.0),
                    ("Accrued unpaid obligations", c.arrears.0),
                    ("Last-day tax receipts", c.report.revenue.0),
                    ("Last-day public payments", c.report.spending.0),
                    ("Last-day interest paid", c.report.interest.0),
                    (
                        "Daily fiscal balance",
                        c.report.revenue.0 - c.report.spending.0 - c.report.interest.0,
                    ),
                ] {
                    ui.label(name);
                    ui.monospace(format!("{value}m"));
                    ui.end_row();
                }
            });
        ui.separator();
        let plan = &c.budget;
        ui.strong(format!(
            "Fiscal period {} to {} · opening forecast vs execution",
            date(plan.start_day),
            date(plan.end_day)
        ));
        ui.label(format!("Revenue realized {}m / opening forecast {}m to date ({:+}m variance). Payments {}m · capital {}m · net financing {}m.",plan.revenue.0,plan.expected_revenue(w.day),plan.revenue.0-plan.expected_revenue(w.day),plan.payments.0,plan.capital.0,plan.borrowing.0-plan.principal_repaid.0));
        ui.label(format!(
            "Opening forecast closing cash {}m. Cash reconciliation now {}m = treasury {}m.",
            plan.forecast_closing_cash(),
            plan.expected_cash(),
            c.accounts.treasury.0
        ));
        let latest = c.forecast(&d);
        ui.label(format!("Current no-shock run rate: revenue {}m/day, spending {}m/day, interest {}m/day. It excludes pending programs and new financing; forecasts are not spendable money.",latest.daily_revenue.0,latest.daily_spending.0,latest.daily_interest.0));
        ui.small("Forecast differences reflect supply shocks, rounded tax collection, policy changes and cash-constrained private income. Arrears and capital payments are shown separately.");
        egui::CollapsingHeader::new("Closed fiscal periods").show(ui, |ui| {
            for review in c.budget_reviews.iter().rev() {
                ui.label(format!(
                    "{}: revenue {}m, payments {}m, capital {}m, closing cash {}m, arrears {}m",
                    date(review.period.end_day),
                    review.period.revenue.0,
                    review.period.payments.0,
                    review.period.capital.0,
                    review.closing_cash.0,
                    review.arrears.0
                ));
            }
        });
        ui.small(format!("Reconciliation: sector cash {}m = opening {}m. External debt uses explicit principal records and 4% annual interest. Borrowing ceiling: {}m.",c.accounts.total(),c.opening_money.0,d.gdp*3/2));
        if self.selected == w.player {
            ui.horizontal_wrapped(|ui| {
                ui.label("Principal (m)");
                ui.add(
                    egui::DragValue::new(&mut self.money)
                        .range(1..=d.gdp / 10)
                        .speed((d.gdp / 10000).max(1) as f64),
                );
                if ui.button("Issue loan").clicked() {
                    self.issue(Action::Borrow {
                        amount: Money(self.money),
                    });
                }
                if ui.button("Repay debt").clicked() {
                    self.issue(Action::Repay {
                        amount: Money(self.money),
                    });
                }
            });
        }
        ui.separator();
        ui.checkbox(
            &mut self.show_ledger,
            "Show recent ledger (30-day retention)",
        );
        let w = self.world.as_ref().expect("world");
        let c = &w.countries[&self.selected];
        ui.strong("Physical accounts · last day · modeled basket/energy units");
        ui.label(format!(
            "Energy: produced {} + imported {} → used {} + inventory {}",
            c.report.energy_produced,
            c.report.energy_imported,
            c.report.energy_used,
            c.energy_stock
        ));
        ui.label(format!(
            "Food: produced {} + imported {} → consumed {} + spoiled {} + inventory {}",
            c.report.food_produced,
            c.report.food_imported,
            c.report.food_consumed,
            c.report.food_lost,
            c.food_stock
        ));
        ui.small("Daily stock reconciliation also includes the previous day's inventory. Imports are limited to 25% of energy need and 35% of food need, and require business cash.");
        if self.show_ledger {
            for e in w
                .ledger
                .iter()
                .rev()
                .filter(|e| e.country == self.selected)
                .take(150)
            {
                ui.label(format!(
                    "{} · {:?} → {:?} · {}m · {}",
                    date(e.day),
                    e.from,
                    e.to,
                    e.amount.0,
                    e.reason
                ));
            }
        }
    }
    fn politics(&mut self, ui: &mut egui::Ui) {
        if self.foreign_report(ui) {
            return;
        }

        let d = self
            .content
            .country(&self.selected)
            .expect("country")
            .clone();
        ui.heading("Institutions & political survival");
        ui.label(format!(
            "{} · {:?}\n{}",
            d.name,
            d.regime,
            d.government.label()
        ));
        ui.label("All seats, political preferences and election cycles are scenario approximations. The 100-seat assembly is a game abstraction; it is not a historical parliament.");
        let c = self.world.as_ref().map(|w| &w.countries[&self.selected]);
        if let Some(c) = c {
            ui.label(format!(
                "Executive: {}\nApproval {}% · legislative support {} · elite backing {}",
                c.executive, c.approval, c.legislative_support, c.elite_support
            ));
            ui.label(format!(
                "Council/cadre backing {} · confidence warnings {} / 3 · leadership review {}",
                c.council_support,
                c.confidence_crises,
                c.next_leadership_review
                    .map(date)
                    .unwrap_or_else(|| "None scheduled".into())
            ));
            ui.label(format!(
                "Next election: {}",
                c.next_election.map(date).unwrap_or_else(|| {
                    "No competitive election; elite coalition reviewed monthly".into()
                })
            ));
        } else {
            ui.label(format!(
                "Bookmark executive: {} ({})",
                d.executive, d.office
            ));
        }
        ui.separator();
        egui::Grid::new("parties").striped(true).show(ui, |ui| {
            for label in [
                "Party / governing body",
                "Bookmark leader",
                "Authored ideology",
                "Seats",
            ] {
                ui.strong(label);
            }
            ui.end_row();
            for p in &d.parties {
                let ruling = c.map_or(p.id == d.ruling_party, |c| p.id == c.ruling_party);
                ui.label(format!(
                    "{}{}",
                    if ruling { "Governing: " } else { "" },
                    p.name
                ));
                ui.label(&p.leader);
                ui.label(&p.ideology);
                ui.monospace(c.map_or(p.seats, |c| c.seats[&p.id]).to_string());
                ui.end_row();
            }
        });
        ui.separator();
        ui.label("Authority thresholds are content-defined. Parliamentary support can trigger an early confidence election after three weak monthly reviews. Party councils review cadre backing; royal councils can block a spending expansion above 4 GDP points without 65 backing. Consent is rechecked before construction. Country play continues after leadership changes.");
        ui.label("Cohort needs, disposable income, taxes, employment and funded services affect opinion. Parliamentary support, elite backing and council backing are distinct. Seat allocation, succession and preferences remain scenario approximations.");
    }
    fn chronicle(&self, ui: &mut egui::Ui) {
        if self.foreign_report(ui) {
            return;
        }

        ui.heading("Chronicle & decisions");
        ui.label("Accepted outcomes, delivery dates and AI explanations. Most recent first; filtered to the selected country.");
        let Some(w) = &self.world else {
            ui.label("Your chronicle begins with a mandate.");
            return;
        };
        for e in w.events.iter().rev().filter(|e| e.country == self.selected) {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.colored_label(
                    if e.important { MINT } else { MUTED },
                    format!(
                        "{} · {}",
                        date(e.day),
                        if e.important {
                            "DECISION / ALERT"
                        } else {
                            "REPORT"
                        }
                    ),
                );
                ui.label(&e.text);
            });
        }
    }
    fn guide(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.help.reference, false, "First mandate walkthrough");
            ui.selectable_value(&mut self.help.reference, true, "Search game reference");
        });
        ui.separator();
        if self.help.reference {
            if let Some(destination) = self.help.show(ui, &self.content) {
                if let Some(country) = destination.country {
                    self.selected = country;
                }
                if let Some(policy) = destination.policy {
                    self.value = self.content.policy(&policy).expect("reference policy").min;
                    self.policy = policy;
                }
                self.tab = destination.tab;
                if self.tab == Tab::Settings {
                    self.pause();
                }
            }
            return;
        }
        ui.heading("Learn your first mandate");
        for (title, text) in [
            (
                "1. Read the situation",
                "Start paused. Advance one day, then inspect energy/food delivery and the daily fiscal balance. Countries with shortages need supply investment; cash-poor governments need a financing plan.",
            ),
            (
                "2. Fund a feasible decision",
                "Policies show the legal route, upfront payment and implementation delay. Energy and food programs expand supply; modernization improves collection and service delivery. Budget levels change annual spending obligations, with payment every day.",
            ),
            (
                "3. Watch delivery",
                "Advance time. A funded commitment takes 30–180 base days plus consent and administrative delays. Completion appears in the Chronicle. Supply shocks may still disrupt delivery. Imports are bounded and paid from business cash.",
            ),
            (
                "4. Balance competing needs",
                "Higher wage tax improves revenue but reduces household disposable income. Services and welfare support living standards but require cash. Loans supply real external funds at 4% interest; an exhausted treasury creates arrears and reduces delivery.",
            ),
            (
                "5. Maintain authority",
                "Competitive systems require legislative support; single-party, military, royal, personalist and religious governments require elite backing. Negotiate support during gridlock. Elections and leadership changes continue your country campaign.",
            ),
            (
                "6. Complete the review",
                "At the five-year review, meet food and energy needs at 90%, approval at 50%, zero arrears and debt no more than opening GDP. The sandbox continues whether the objective is met or missed.",
            ),
            (
                "Controls & recovery",
                "Default controls: Space pauses/plays, F5 saves, F9 loads, N advances to an important event and F1 opens rebindable controls/scaling. A monthly autosave is written inside saves with a country, scenario and content-specific filename. Use Inspect recovery snapshots to review and restore validated primary, pending or backup files while preserving originals. Map markers and the country directory select an inspection context. Only your own government's commands are authorized.",
            ),
            (
                "Content scope",
                "Inspect the loaded pack description and source records below. Historical facts and authored approximations are labeled separately. Land outlines show geography only. This initial game does not yet implement warfare, detailed diplomacy, full historical coverage or regional service networks.",
            ),
        ] {
            ui.strong(title);
            ui.label(text);
            ui.add_space(8.0);
        }
        ui.strong(format!(
            "{} · {} countries · bookmark {}",
            self.content.name,
            self.content.countries.len(),
            self.content.start
        ));
        ui.label(&self.content.description);
        ui.label(format!(
            "Content {} · hash {:016x} · rules {}",
            self.content.id,
            self.content.hash(),
            self.content.rules
        ));
    }
}
impl GameApp {
    fn process_shortcuts(&mut self, ctx: &egui::Context) {
        if settings::shortcut(ctx, self.preferences.active.pause) && self.world.is_some() {
            if self.speed > 0 || self.pending.is_some() {
                self.pause();
            } else {
                self.speed = 1;
            }
            self.last_tick = Instant::now();
        }
        if settings::shortcut(ctx, self.preferences.active.save) {
            self.save();
        }
        if settings::shortcut(ctx, self.preferences.active.load) {
            self.load();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1)) {
            self.pause();
            self.tab = Tab::Settings;
        }
        if settings::shortcut(ctx, self.preferences.active.next_event)
            && let Some(world) = &self.world
        {
            let remaining = MAX_DAY - world.day;
            self.speed = 0;
            self.start_advance(remaining.min(365), true);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.pause();
            if self.recovery.is_empty() {
                self.tab = Tab::Overview;
            } else {
                self.recovery.clear();
            }
        }
    }
    fn render(&mut self, ctx: &egui::Context) {
        self.poll_advance();
        if self.preferences.active.pause_on_focus_loss
            && !ctx.input(|i| i.focused)
            && (self.speed > 0 || self.pending.is_some())
        {
            self.pause();
            self.status = "Paused because the game window lost focus.".into();
        }
        if self.speed > 0
            && self.pending.is_none()
            && self.last_tick.elapsed() >= Duration::from_millis(350)
        {
            self.advance(self.speed);
            self.last_tick = Instant::now();
        }
        self.top(ctx);
        self.sidebar(ctx);
        if !self.recovery.is_empty() {
            let mut chosen = None;
            let mut close = false;
            egui::Window::new("Save recovery · review before restoring").collapsible(false).show(ctx,|ui|{
                ui.label("Restoration preserves the current file and interrupted writes before replacing the primary. The active campaign changes only after successful recovery.");
                for candidate in &self.recovery {
                    ui.separator();ui.label(format!("{:?}: {}",candidate.kind,candidate.path.display()));
                    if let Some(error)=&candidate.error{ui.colored_label(Color32::LIGHT_YELLOW,error);}else if let Some(day)=candidate.day {ui.label(format!("Valid campaign at {} · snapshot format {} · {} · rules {}",date(day),candidate.schema.unwrap_or_default(),candidate.content_id.as_deref().unwrap_or("unknown pack"),candidate.rules.unwrap_or_default()));if ui.add_enabled(self.pending.is_none(),egui::Button::new(format!("Restore {:?} from {}",candidate.kind,date(day)))).clicked(){chosen=Some(candidate.path.clone());}}
                }
                close=ui.button("Close without changes").clicked();
            });
            if let Some(candidate) = chosen {
                let path = PathBuf::from(&self.save_path);
                let restored = if self.bundled_catalog {
                    persistence::restore_bundled(&path, &candidate)
                } else {
                    persistence::restore(&path, &candidate, &self.content)
                        .map(|world| (self.content.clone(), world))
                };
                match restored {
                    Ok((content, world)) => {
                        self.content = content;
                        self.selected = world.player.clone();
                        self.target_date = date((world.day + 30).min(MAX_DAY));
                        self.world = Some(world);
                        self.recovery.clear();
                        self.status =
                            "Recovered snapshot. Original files preserved; game paused.".into();
                    }
                    Err(error) => self.status = error,
                }
            } else if close {
                self.recovery.clear();
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("main_scroll")
                .show(ui, |ui| match self.tab {
                    Tab::Overview => self.overview(ui),
                    Tab::Policies => self.policies(ui),
                    Tab::Treasury => self.treasury(ui),
                    Tab::Society => self.society(ui),
                    Tab::Analysis => self.analysis.show(
                        ui,
                        &self.content,
                        self.world.as_ref(),
                        &mut self.selected,
                    ),
                    Tab::Politics => self.politics(ui),
                    Tab::Chronicle => self.chronicle(ui),
                    Tab::Guide => self.guide(ui),
                    Tab::Settings => self.preferences.show(ui),
                    Tab::Inspector => self.developer_inspector(ui),
                });
        });
        self.process_shortcuts(ctx);
        if self.speed > 0 || self.pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
}
impl eframe::App for GameApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(ctx);
    }
}
fn metric(ui: &mut egui::Ui, label: &str, value: String) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(140.0);
        ui.label(RichText::new(label).color(MUTED).small());
        ui.label(RichText::new(value).size(19.0).strong());
    });
}
fn sparkline(ui: &mut egui::Ui, history: &[Observation]) {
    ui.small("Monthly approval trend · 0–100%");
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(650.0), 60.0),
        egui::Sense::hover(),
    );
    let points: Vec<_> = history
        .iter()
        .enumerate()
        .map(|(i, o)| {
            egui::pos2(
                rect.left() + i as f32 / (history.len().max(2) - 1) as f32 * rect.width(),
                rect.bottom() - o.approval as f32 / 100.0 * rect.height(),
            )
        })
        .collect();
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0_f32, MUTED),
    );
    if points.len() > 1 {
        ui.painter()
            .add(egui::Shape::line(points, Stroke::new(2.0_f32, MINT)));
    }
}
fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let mut content_path = None;
    let mut developer_mode = false;
    let usage = "Usage: desktop [--content pack.json] [--developer]";
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--developer" => developer_mode = true,
            "--content" if content_path.is_none() => {
                content_path = args.next();
                if content_path.is_none() {
                    eprintln!("{usage}");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("{usage}");
                std::process::exit(1);
            }
        }
    }
    let bundled_catalog = content_path.is_none();
    let content = if let Some(path) = content_path {
        std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|s| Content::from_json(&s))
    } else {
        Content::bundled()
    };
    let content = content.unwrap_or_else(|error| {
        eprintln!("Cannot start: {error}");
        std::process::exit(1)
    });
    eframe::run_native(
        "Statecraft · A New Century",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1440.0, 940.0])
                .with_min_inner_size([1000.0, 700.0]),
            renderer: eframe::Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(GameApp::new(
                cc,
                content,
                developer_mode,
                bundled_catalog,
            )))
        }),
    )
}

#[cfg(test)]
mod interface_tests {
    use super::*;
    #[test]
    fn bundled_loading_keeps_classic_rules_and_explicit_pack_selection_stays_pinned() {
        let current = Content::bundled().unwrap();
        let classic = Content::classic_bundled().unwrap();
        let mut old_world = World::new(&classic, CountryId("GBR".into()), 17).unwrap();
        old_world.advance(&classic, 31).unwrap();
        let mut header: serde_json::Value =
            serde_json::from_slice(&persistence::encode(&old_world, &classic).unwrap()).unwrap();
        header["schema"] = 2.into();
        let original = serde_json::to_vec_pretty(&header).unwrap();
        let dir = std::env::temp_dir().join(format!("geosim-ui-classic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("classic.json");
        std::fs::write(&path, &original).unwrap();
        let ctx = egui::Context::default();
        let new_world = World::new(&current, CountryId("GBR".into()), 17).unwrap();
        let expected = new_world.hash();
        let mut app =
            GameApp::from_context(&ctx, current.clone(), settings::Preferences::default());
        app.world = Some(new_world);
        app.save_path = path.display().to_string();
        assert!(!app.bundled_catalog);
        app.load();
        assert_eq!(app.content.hash(), current.hash());
        assert_eq!(app.world.as_ref().unwrap().hash(), expected);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        app.bundled_catalog = true;
        app.load();
        assert_eq!(app.content.hash(), classic.hash());
        assert_eq!(app.world.as_ref().unwrap().hash(), old_world.hash());
        assert_eq!(app.world.as_ref().unwrap().rules, 2);
        assert!(app.status.contains("Classic fixed-price"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        app.save();
        assert_eq!(
            std::fs::read(dir.join("classic.json.format-v2-1")).unwrap(),
            original
        );
        assert_eq!(
            persistence::load_bundled(&path).unwrap().1.hash(),
            old_world.hash()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn focus_loss_and_escape_pause_while_preserving_country_and_map_context() {
        let content =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let ctx = egui::Context::default();
        let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
        app.world = Some(world);
        app.selected = CountryId("BOR".into());
        app.map.camera.zoom = 3.0;
        app.map.camera.longitude = 40.0;
        app.tab = Tab::Policies;
        app.speed = 7;
        let _ = ctx.run(
            egui::RawInput {
                focused: false,
                ..Default::default()
            },
            |ctx| app.render(ctx),
        );
        assert_eq!(app.speed, 0);
        assert_eq!(app.playback_label(), "Paused");
        app.start_advance(0, false);
        let _ = ctx.run(
            egui::RawInput {
                focused: true,
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| app.process_shortcuts(ctx),
        );
        assert!(app.pending.as_ref().unwrap().is_stopping());
        assert_eq!(app.playback_label(), "Stopping at next day boundary");
        assert!(app.tab == Tab::Overview);
        assert_eq!(app.selected.0, "BOR");
        assert_eq!(app.map.camera.zoom, 3.0);
        assert_eq!(app.map.camera.longitude, 40.0);
    }
    #[test]
    fn inspector_requires_explicit_host_capability_even_for_a_modified_save() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world
            .issue_developer(
                &content,
                CountryId("CHN".into()),
                DeveloperEdit {
                    field: DeveloperField::Tax,
                    value: 35,
                },
            )
            .unwrap();
        let expected = world.hash();
        let ctx = egui::Context::default();
        let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
        assert!(!app.developer_mode);
        app.world = Some(world);
        app.selected = CountryId("CHN".into());
        app.tab = Tab::Inspector;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0, 940.0),
                )),
                ..Default::default()
            },
            |ctx| app.render(ctx),
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Developer inspection is disabled"));
        assert!(text.contains("DEVELOPER-MODIFIED"));
        assert!(!text.contains("live world truth"));
        assert!(!text.contains("Apply developer edit"));
        assert_eq!(app.world.unwrap().hash(), expected);
    }
    #[test]
    fn every_screen_renders_at_compact_and_scaled_sizes_without_mutating_campaign() {
        for (content, own, foreign) in [
            (Content::bundled().unwrap(), "GBR", "CHN"),
            (
                Content::from_json(include_str!("../../../data/synthetic-dated-test.json"))
                    .unwrap(),
                "AUR",
                "BOR",
            ),
            (
                Content::from_json(include_str!("../../../data/synthetic-market-test.json"))
                    .unwrap(),
                "AUR",
                "BOR",
            ),
        ] {
            let mut world = World::new(&content, CountryId(own.into()), 17).unwrap();
            world.advance(&content, 360).unwrap();
            let expected = world.hash();
            for scale in [1.0, 1.4] {
                let ctx = egui::Context::default();
                let mut app =
                    GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
                app.developer_mode = true;
                ctx.set_zoom_factor(scale);
                for selected in [None, Some(own), Some(foreign)] {
                    app.world = selected.map(|_| world.clone());
                    app.selected = CountryId(selected.unwrap_or(own).into());
                    for tab in [
                        Tab::Overview,
                        Tab::Policies,
                        Tab::Society,
                        Tab::Treasury,
                        Tab::Analysis,
                        Tab::Politics,
                        Tab::Chronicle,
                        Tab::Guide,
                        Tab::Guide,
                        Tab::Settings,
                        Tab::Inspector,
                    ] {
                        app.tab = tab;
                        if tab == Tab::Guide {
                            app.help.reference = !app.help.reference;
                        }
                        let input = egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1280.0 / scale, 800.0 / scale),
                            )),
                            focused: true,
                            ..Default::default()
                        };
                        let output = ctx.run(input, |ctx| app.render(ctx));
                        assert!(!output.shapes.is_empty());
                        assert!(
                            output
                                .shapes
                                .iter()
                                .all(|shape| shape.clip_rect.is_finite())
                        );
                        if let Some(w) = &app.world {
                            assert_eq!(w.hash(), expected);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn load_shortcut_uses_text_edited_in_the_same_frame() {
        let content = Content::bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let dir = std::env::temp_dir().join(format!(
            "geosim-ui-key-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wanted.json");
        let mut wanted = world.clone();
        wanted.advance(&content, 2).unwrap();
        persistence::save(&path, &wanted, &content).unwrap();
        let ctx = egui::Context::default();
        let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
        app.world = Some(world);
        app.save_path = dir.join("old.json").display().to_string();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 940.0));
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                focused: true,
                ..Default::default()
            },
            |ctx| app.render(ctx),
        );
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("campaign_save_path")));
        let control = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let input = egui::RawInput {
            screen_rect: Some(screen),
            focused: true,
            events: vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: control,
                },
                egui::Event::Text(path.display().to_string()),
                egui::Event::Key {
                    key: egui::Key::F9,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| app.render(ctx));
        assert_eq!(app.save_path, path.display().to_string());
        assert_eq!(app.world.as_ref().unwrap().hash(), wanted.hash());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
