mod administrative;
mod advance;
mod analysis;
mod briefing;
mod cabinet;
mod cash_management;
mod cashflows;
mod charts;
mod chronicle;
mod commitments;
mod constitutions;
mod elections;
mod executive_identity;
mod financing;
mod fiscal_plans;
mod funding;
mod geometry;
mod governing_institutions;
mod help;
mod hydrology;
mod income_taxes;
mod inspector;
mod legislation;
mod map;
mod markets;
mod navigation;
mod new_game;
mod party_preferences;
mod reserves;
mod settings;
mod sovereignty;
mod subnational;
mod technologies;
mod undertakings;
mod walkthrough;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use sim_core::{persistence, *};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const MINT: Color32 = Color32::from_rgb(104, 224, 186);
const MUTED: Color32 = Color32::from_rgb(148, 170, 181);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
    navigation: navigation::History,
    search: String,
    seed: u64,
    scenario: Scenario,
    policy: PolicyId,
    value: i32,
    status: String,
    speed: u32,
    last_tick: Instant,
    pause_alerts: bool,
    save_path: String,
    money: i64,
    cash_management: cash_management::Panel,
    funding_chart: charts::Chart,
    constitution_panel: constitutions::Panel,
    commitment_panel: commitments::Panel,
    undertaking_panel: undertakings::Panel,
    fiscal_panel: fiscal_plans::Panel,
    cabinet_panel: cabinet::Panel,
    subnational_panel: subnational::Panel,
    sort_gdp: bool,
    show_ledger: bool,
    recovery: Vec<persistence::RecoveryCandidate>,
    analysis: analysis::Analysis,
    pending: Option<advance::AdvanceJob>,
    target_date: String,
    preferences: settings::Preferences,
    developer_mode: bool,
    inspector: inspector::Inspector,
    event_browser: chronicle::Panel,
    help: help::Help,
    cashflows: cashflows::CashFlows,
    bundled_catalog: bool,
    new_setup: Option<new_game::Setup>,
    briefing: briefing::Briefing,
    walkthrough: walkthrough::Panel,
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
        let save_path = new_game::default_save_path(&content);
        Self {
            content,
            world: None,
            selected,
            tab: Tab::Overview,
            map: map::WorldMap::new(),
            navigation: navigation::History::default(),
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
            cash_management: cash_management::Panel::default(),
            funding_chart: charts::Chart::default(),
            constitution_panel: constitutions::Panel::default(),
            commitment_panel: commitments::Panel::default(),
            undertaking_panel: undertakings::Panel::default(),
            fiscal_panel: fiscal_plans::Panel::default(),
            cabinet_panel: cabinet::Panel::default(),
            subnational_panel: subnational::Panel::default(),
            sort_gdp: false,
            show_ledger: false,
            recovery: Vec::new(),
            analysis: analysis::Analysis::default(),
            pending: None,
            target_date,
            preferences,
            developer_mode: false,
            inspector: inspector::Inspector::default(),
            event_browser: chronicle::Panel::default(),
            help: help::Help::default(),
            cashflows: cashflows::CashFlows::default(),
            bundled_catalog: false,
            new_setup: None,
            briefing: briefing::Briefing::default(),
            walkthrough: walkthrough::Panel::default(),
        }
    }
    fn advance(&mut self, elapsed: DaySpan) {
        self.start_advance(elapsed, self.pause_alerts);
    }
    fn next_event(&mut self) {
        self.speed = 0;
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let remaining = MAX_DAY - world.day.units();
        if remaining == 0 {
            self.status =
                "The campaign calendar is at its limit; there is no later event to search for."
                    .into();
            return;
        }
        let already_advancing = self.pending.is_some();
        self.start_advance(DaySpan::new(remaining), true);
        if !already_advancing && let Some(job) = &mut self.pending {
            job.seeking_event = true;
            self.status =
                "Searching for the next important event. You can stop at the next day boundary."
                    .into();
        }
    }
    fn start_advance(&mut self, elapsed: DaySpan, important: bool) {
        let days = elapsed.units();
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
        match advance::AdvanceJob::start(world.clone(), self.content.clone(), elapsed, important) {
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
        let requested = job.requested.units();
        match result {
            Ok(None) => {}
            Err(error) => {
                self.pending = None;
                self.speed = 0;
                self.status = error;
            }
            Ok(Some(outcome)) => {
                let days = outcome.world.day.units() - start;
                let interrupted = days < requested;
                self.world = Some(outcome.world);
                self.pending = None;
                if interrupted || outcome.stopped_for_event || outcome.error.is_some() {
                    self.speed = 0;
                }
                self.status = outcome.error.unwrap_or_else(|| {
                    format!(
                        "Advanced {days} days to {}{}",
                        date(self.world.as_ref().expect("world").day.units()),
                        if interrupted {
                            " · stopped at an event or requested boundary"
                        } else {
                            ""
                        }
                    )
                });
                let world = self.world.as_ref().expect("world");
                if world.day.units() / 30 != start / 30 {
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
            self.status = match w.issue_with_receipt(&self.content, action) {
                Ok(receipt) => match receipt.outcomes.first() {
                    Some(event) => format!(
                        "Command #{} accepted · outcome {event} · inspect Chronicle.",
                        receipt.command
                    ),
                    None => format!(
                        "Command #{} accepted. Inspect the Chronicle and pending commitments.",
                        receipt.command
                    ),
                },
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
                self.target_date = date((w.day.units() + 30).min(MAX_DAY));
                self.world = Some(w);
                self.navigation = navigation::History::default();
                self.undertaking_panel = undertakings::Panel::default();
                self.fiscal_panel = fiscal_plans::Panel::default();
                self.walkthrough = walkthrough::Panel::default();
                self.cash_management = cash_management::Panel::default();
                self.event_browser = chronicle::Panel::default();
                self.status = match self.content.rules {
                    2 => "Classic fixed-price campaign loaded. Simulation paused.",
                    3 => "Relative-price campaign loaded. Simulation paused.",
                    4 => "Institutional-financing campaign loaded. Simulation paused.",
                    5 => "Constitutional-transition campaign loaded. Simulation paused.",
                    6 => "Policy-lifecycle campaign loaded. Simulation paused.",
                    7 => "Ministerial-appointment campaign loaded. Simulation paused.",
                    8 => "Electoral-authority campaign loaded. Simulation paused.",
                    9 => "Subnational-mandate campaign loaded. Simulation paused.",
                    _ => "Campaign loaded. Simulation paused.",
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
    fn open_new_campaign(&mut self) {
        if !self.recovery.is_empty() {
            self.status = "Close save recovery before starting a new campaign.".into();
            return;
        }
        self.pause();
        match new_game::Setup::new(
            &self.content,
            &self.selected,
            self.seed,
            self.bundled_catalog,
        ) {
            Ok(setup) => self.new_setup = Some(setup),
            Err(error) => self.status = error,
        }
    }
    fn begin_new_campaign(
        &mut self,
        setup: &new_game::Setup,
        save_current: bool,
    ) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("Wait for the current advance to stop at a day boundary.".into());
        }
        let world = setup.prepare()?;
        let archive = if save_current {
            if let Some(current) = &self.world {
                let path = PathBuf::from(&self.save_path);
                if let Some(parent) = path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                Some(persistence::save_before_new_campaign(
                    &path,
                    current,
                    &self.content,
                )?)
            } else {
                None
            }
        } else {
            None
        };
        self.seed = world.seed;
        self.scenario = world.scenario;
        self.selected = world.player.clone();
        self.target_date = date((world.day.units() + 30).min(MAX_DAY));
        self.content = setup.content.clone();
        self.world = Some(world);
        self.navigation = navigation::History::default();
        self.save_path = setup.save_path.clone();
        self.tab = Tab::Overview;
        self.speed = 0;
        self.last_tick = Instant::now();
        self.recovery.clear();
        self.search.clear();
        self.policy = "energy".into();
        self.value = 1;
        self.money = (self
            .content
            .country(&self.selected)
            .expect("new country")
            .gdp
            .units()
            / 100)
            .max(1);
        self.show_ledger = false;
        self.map = map::WorldMap::new();
        self.analysis = analysis::Analysis::default();
        self.funding_chart = charts::Chart::default();
        self.constitution_panel = constitutions::Panel::default();
        self.commitment_panel = commitments::Panel::default();
        self.undertaking_panel = undertakings::Panel::default();
        self.fiscal_panel = fiscal_plans::Panel::default();
        self.cabinet_panel = cabinet::Panel::default();
        self.cashflows = cashflows::CashFlows::default();
        self.inspector = inspector::Inspector::default();
        self.help = help::Help::default();
        self.walkthrough = walkthrough::Panel::default();
        self.cash_management = cash_management::Panel::default();
        self.event_browser = chronicle::Panel::default();
        self.status = archive.map_or_else(
            || "New campaign started, paused.".into(),
            |path| {
                format!(
                    "New campaign started, paused. Previous campaign archived at {}",
                    path.display()
                )
            },
        );
        Ok(())
    }
    fn new_campaign_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut setup) = self.new_setup.take() else {
            return;
        };
        match setup.show(
            ctx,
            self.world.is_some(),
            self.pending.is_none(),
            &self.save_path,
        ) {
            new_game::Choice::Waiting => self.new_setup = Some(setup),
            new_game::Choice::Cancel => {}
            new_game::Choice::Start { save_current } => {
                if let Err(error) = self.begin_new_campaign(&setup, save_current) {
                    setup.error = Some(error);
                    self.new_setup = Some(setup);
                }
            }
        }
    }
    fn inspection(&self) -> navigation::View {
        navigation::View {
            tab: self.tab,
            country: self.selected.clone(),
            policy: self.policy.clone(),
            value: self.value,
            reference: self.help.reference,
            map: self.map.view(),
        }
    }
    fn navigate_history(&mut self, forward: bool) {
        let current = self.inspection();
        if let Some(view) = self.navigation.travel(current, forward) {
            self.tab = view.tab;
            self.selected = view.country;
            self.policy = view.policy;
            self.value = view.value;
            self.help.reference = view.reference;
            self.map.restore_view(view.map);
            if self.tab == Tab::Settings || self.tab == Tab::Inspector {
                self.pause();
            }
        }
    }
    fn playback_label(&self) -> String {
        if let Some(job) = &self.pending {
            if job.is_stopping() {
                "Stopping at next day boundary".into()
            } else if job.seeking_event {
                format!(
                    "Searching for next event · {} days advanced",
                    job.completed.load(std::sync::atomic::Ordering::Relaxed)
                )
            } else {
                format!(
                    "Advancing {} / {} days",
                    job.completed.load(std::sync::atomic::Ordering::Relaxed),
                    job.requested.units()
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
                    ui.strong(date(w.day.units()));
                    ui.label(format!("Governing {} · {}", w.player.0, w.scenario.id()));
                    ui.strong(self.playback_label());
                    if ui.button(format!("{} [{}]", if self.speed == 0 && self.pending.is_none() { "Play" } else { "Pause" }, self.preferences.active.pause.name())).clicked() {
                        if self.speed > 0 || self.pending.is_some() { self.pause(); } else { self.speed = 1; }
                        self.last_tick = Instant::now();
                    }
                } else { ui.label(format!("{} · Offline single player", self.content.start)); }
                if self.developer_mode { ui.colored_label(Color32::LIGHT_YELLOW, "Developer mode"); }
                if self.world.as_ref().is_some_and(World::is_developer_modified) { ui.colored_label(Color32::LIGHT_YELLOW, "DEVELOPER-MODIFIED"); }
                if ui.add_enabled(self.navigation.can_back() && self.new_setup.is_none() && self.recovery.is_empty(),egui::Button::new("Back")).on_hover_text("Previous inspection view [Alt+Left]. Restores the screen, country, policy draft and map view; issues no order.").clicked() { self.navigate_history(false); }
                if ui.add_enabled(self.navigation.can_forward() && self.new_setup.is_none() && self.recovery.is_empty(),egui::Button::new("Forward")).on_hover_text("Next inspection view [Alt+Right].").clicked() { self.navigate_history(true); }
                if ui.button("Settings [F1]").clicked() { self.pause(); self.tab = Tab::Settings; }
                if self.world.is_some() && ui.add_enabled(self.recovery.is_empty(), egui::Button::new("New campaign")).clicked() { self.open_new_campaign(); }
                if self.world.is_some() && ui.button("First session").clicked() {
                    self.pause();self.selected=self.world.as_ref().unwrap().player.clone();self.help.reference=false;self.tab=Tab::Guide;
                }
            });
            if self.world.is_some() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Automatic step:");
                    for (days, label) in [(1, "1 day"), (7, "7 days"), (30, "30 days")] {
                        if ui.selectable_label(self.speed == days, label).on_hover_text("Start automatic progression at this many days per step. Important events can stop it earlier.").clicked() { self.speed = days; }
                    }
                    for (days, label) in [(1, "+1 day"), (30, "+30 days")] {
                        if ui.add_enabled(self.pending.is_none(), egui::Button::new(label)).clicked() { self.speed = 0; self.advance(DaySpan::new(days)); }
                    }
                    if ui.add_enabled(self.pending.is_none(), egui::Button::new("Next event")).clicked() {
                        self.next_event();
                    }
                    ui.add(egui::TextEdit::singleline(&mut self.target_date).desired_width(95.0).hint_text("YYYY-MM-DD"));
                    if ui.add_enabled(self.pending.is_none(), egui::Button::new("Advance to date")).clicked() {
                        match parse_date(&self.target_date) {
                            Ok(day) => {
                                let current = self.world.as_ref().expect("world").day.units();
                                if day < current { self.status = "Choose a future date; load a save to return to an earlier state.".into(); }
                                else { self.speed = 0; self.advance(DaySpan::new(day - current)); }
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
                egui::TopBottomPanel::bottom("campaign_file_controls").show_inside(ui, |ui| {
                    self.campaign_file_controls(ui);
                });
                egui::ScrollArea::vertical().id_salt("sidebar_scroll").auto_shrink([false,false]).show(ui,|ui| {
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
                    .max_height((ui.clip_rect().bottom()-ui.cursor().top()-16.0).clamp(80.0,200.0))
                    .show(ui, |ui| {
                        let mut countries: Vec<_> = self.content.countries.iter().collect();
                        if self.sort_gdp {
                            countries.sort_by_key(|c| std::cmp::Reverse(c.gdp.units()));
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
                ui.small("Sourced reference GDP/population are identified in the overview; remaining values are authored.");
                });
            });
    }
    fn campaign_file_controls(&mut self, ui: &mut egui::Ui) {
        if let Some(world) = &self.world {
            if ui.button("Return to my country").clicked() {
                self.selected = world.player.clone();
            }
            ui.checkbox(&mut self.pause_alerts, "Pause on important events");
        }
        ui.label("Save file / recovery .bak path");
        ui.add(
            egui::TextEdit::singleline(&mut self.save_path).id(egui::Id::new("campaign_save_path")),
        );
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
        if ui.button("Inspect recovery snapshots").clicked() {
            self.inspect_recovery();
        }
        ui.small(format!(
            "{} save · {} load · {} next event · Tab to navigate",
            self.preferences.active.save.name(),
            self.preferences.active.load.name(),
            self.preferences.active.next_event.name()
        ));
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
        let report = self
            .world
            .as_ref()
            .and_then(|world| world.public_report(&self.content, &self.selected).ok());
        ui.label(format!(
            "{} · {:?} · {} {}",
            d.capital,
            report.as_ref().map_or(&d.regime, |report| &report.regime),
            report
                .as_ref()
                .map_or(d.office.as_str(), |report| report.government_title.as_str()),
            report
                .as_ref()
                .map_or(d.executive.as_str(), |report| report.executive.as_str())
        ));

        if self.world.is_none() {
            ui.add_space(10.0);
            ui.heading("Your first mandate");
            ui.label(format!("Keep food and energy needs at 90%, approval at 50%, clear arrears and keep debt no greater than opening GDP by {}. Continue as the country after elections or leadership changes.", date(self.content.review_day().expect("validated bookmark"))));
            if self
                .content
                .finance
                .as_ref()
                .is_some_and(|rules| rules.maturity_days.is_some())
            {
                ui.label("The five-year review also requires no overdue loan principal. Check debt dates before committing cash.");
            }
            ui.horizontal_wrapped(|ui| {
                metric(
                    ui,
                    "Population",
                    format!("{:.1} million", d.population.0 as f64 / 1_000_000.0),
                );
                metric(
                    ui,
                    "Opening GDP",
                    format!("{}m credits/year", d.gdp.units()),
                );
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
                    match World::new_scenario(&self.content,d.id.clone(),self.seed,self.scenario){Ok(w)=>{self.world=Some(w);self.money=(d.gdp.units()/100).max(1);self.status="Mandate started, paused. Advance a day to collect the first fiscal and supply report.".into();},Err(e)=>self.status=e}
                }
            });
        }

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
            for (field, value) in [("population", d.population.0 as i64), ("gdp", d.gdp.units())] {
                if !facts.iter().any(|fact| fact.field == field) { ui.label(format!("{field}: {value} · authored calibration; no observation record in this pack.")); }
            }
            for fact in facts {ui.label(format!("{}: {} {} ({} observation; source {}). Applied {}. {}",fact.field,fact.original_value,fact.original_units,fact.observation_period,fact.source,fact.applied_value,fact.transformation));ui.small(format!("Pinned mirror revision {}. Primary API not independently reconciled.",fact.source_version));}
            ui.small("Debt, capacities, household profiles, seats, preferences and political rules are authored. Identity names remain a starter curation; all simulated outcomes branch from the bookmark.");
        });
        sovereignty::show(ui, &self.content, &self.selected);
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
        if let Some(w) = &self.world {
            let c = &w.countries[&self.selected];
            ui.horizontal_wrapped(|ui| {
                metric(ui, "Treasury", format!("{}m", c.accounts.treasury.0));
                metric(
                    ui,
                    "Debt / opening GDP",
                    format!("{}%", c.debt.0 * 100 / d.gdp.units()),
                );
                metric(ui, "Approval", format!("{}%", c.approval));
                metric(
                    ui,
                    "Energy / food delivered",
                    if w.day.units() == self.content.start_day().expect("validated bookmark") {
                        "Awaiting first settlement".into()
                    } else {
                        format!(
                            "{}% / {}%",
                            c.report.energy_coverage, c.report.food_coverage
                        )
                    },
                );
            });
            if w.day.units() > self.content.start_day().expect("validated bookmark") {
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
                    .map(|day| date(day.units()))
                    .unwrap_or_else(|| "Elite backing reviewed monthly".into())
            ));
            if let Some(event) = w.events.iter().rev().find(|e| e.country == self.selected) {
                ui.label(format!("{}  {}", date(event.day.units()), event.text));
            }
            self.situation(ui);
        }
    }
    fn situation(&mut self, ui: &mut egui::Ui) {
        let Some(world) = &self.world else {
            return;
        };
        if self.selected != world.player
            || world.day.units() == self.content.start_day().expect("validated bookmark")
        {
            return;
        }
        let c = world.player_country();
        let (energy, food, tax, available, gap) = (
            c.report.energy_coverage,
            c.report.food_coverage,
            c.tax,
            c.available_cash().0,
            c.report.spending.0 + c.report.interest.0 + c.report.unpaid.0 - c.report.revenue.0
                + c.subnational.as_ref().map_or(0, |state| {
                    state
                        .regions
                        .iter()
                        .filter_map(|region| region.last.as_ref())
                        .map(|last| last.tax_receipts.0 - last.paid_base.0 - last.paid_supplement.0)
                        .sum::<i64>()
                }),
        );
        let rules = world
            .governing_rules(&self.content, &world.player)
            .expect("government");
        let backing = if rules.route == DecisionRoute::Legislature {
            c.legislative_support
        } else {
            c.elite_support
        };
        let authority_blocked =
            backing < rules.threshold || c.council_support < rules.council_threshold;
        ui.separator();
        ui.heading("Cabinet situation brief");
        if let Some(request) = c
            .constitution
            .as_ref()
            .and_then(|state| state.pending.as_ref())
        {
            ui.label(format!("Pending {} constitutional decision: {:?}, due {}. Reserved administration {}m; approval is not guaranteed.", if request.contested { "CONTESTED" } else { "lawful" }, request.target, date((request.decision_day).units()), request.reserved.0));
            if ui.button("Review constitutional decision").clicked() {
                self.tab = Tab::Politics;
            }
        }
        if let Some(request) = c
            .financing
            .as_ref()
            .and_then(|state| state.pending.as_ref())
        {
            ui.label(format!(
                "Borrowing request #{}: {}m, decision {}. Pending funds are not treasury cash.",
                request.id.0,
                request.amount.0,
                date((request.decision_day).units())
            ));
            if ui.button("Review borrowing decision").clicked() {
                self.tab = Tab::Treasury;
            }
        }

        if let Some(financing) = &c.financing {
            let due = financing.principal_due_through(world.day.units()).0;
            let upcoming = financing
                .principal_due_through(world.day.units().saturating_add(30))
                .0
                - due;
            if due > 0 || upcoming > 0 {
                ui.label(format!("Debt principal: {due}m overdue/due; {upcoming}m additionally due within 30 days. Refinancing is not automatic."));
                if ui.button("Review principal repayment calendar").clicked() {
                    self.tab = Tab::Treasury;
                }
            }
        }
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
        ui.label(format!(
            "Current governing form {:?} · {} · {}",
            report.regime,
            report.government_title,
            report.government.label()
        ));
        if let Some(authority) = &report.electoral_authority {
            ui.label(format!(
                "{} · next poll {}",
                authority.title,
                authority.next_poll.map_or_else(
                    || "None within the playable calendar".into(),
                    |day| date(day.units())
                )
            ));
            if let Some(poll) = &authority.last_poll {
                ui.label(format!(
                    "Last modeled poll: {}. Allocation uses the scenario's 100-seat rule.",
                    date(poll.day.units())
                ));
            }
        }
        if let Some(plans) = &report.fiscal_plans {
            for (index, p) in plans.iter().enumerate().rev() {
                ui.label(format!("Fiscal plan {} · approved {}: {} {}%, services {}% GDP, welfare {}% GDP; first scheduled setting change {}.",p.year,date((p.enacted_day).units()),if self.content.income_taxes.is_some(){"standard income-tax rate"}else{"tax"},p.targets.income_tax,p.targets.services,p.targets.welfare,date((p.first_scheduled_delivery).units())));
                fiscal_plans::published_delivery(
                    ui,
                    &self.content,
                    &report.country,
                    p,
                    index + 1 == plans.len(),
                );
            }
        }
        if let Some(undertakings) = &report.undertakings {
            ui.label(format!(
                "Public delivery credibility: {} / 100",
                undertakings.credibility
            ));
            for r in &undertakings.records {
                ui.label(format!(
                    "Undertaking #{} · {} by {} · {}",
                    r.id.0,
                    undertakings::goal_label(&self.content, &r.goal),
                    date((r.deadline_day).units()),
                    r.status.label()
                ));
            }
        }
        if let Some(territory) = &report.territorial {
            ui.label(format!(
                "Current territorial settlement: {:?}",
                territory.form
            ));
            if let Some(change) = &territory.last_change {
                ui.label(format!(
                    "Last charter change {} · contested {}",
                    date(change.day.units()),
                    change.contested
                ));
            }
            if let Some(claim) = &territory.last_requisition {
                ui.label(format!(
                    "Last contested local-fund requisition {} · {} · {}m",
                    date(claim.day.units()),
                    claim.region.0,
                    claim.amount.0
                ));
            }
        }
        if let Some(authorities) = &report.regional_authorities {
            for authority in authorities {
                ui.label(format!(
                    "{} · {} · base purchasing level {} · {}",
                    authority.name,
                    subnational::tier(&authority.tier),
                    authority.service_level,
                    if authority.autonomous_services {
                        "Autonomous service powers"
                    } else {
                        "Delegated service powers"
                    }
                ));
            }
        }
        if let Some(ministers) = &report.ministers {
            for minister in ministers {
                ui.label(format!(
                    "{}: {}",
                    minister.title,
                    minister
                        .holder
                        .as_deref()
                        .unwrap_or("Permanent civil service")
                ));
            }
        }
        if let Some(latest) = report.latest {
            ui.label(format!("Statistical release {} ({} days old): approval {}%, needs {}%, debt {}m, daily output {}m.",date(latest.day.units()),w.day.units()-latest.day.units(),latest.approval,latest.needs,latest.debt.0,latest.daily_output.0));
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
                    if self.content.income_taxes.is_some() {
                        "standard marginal %"
                    } else {
                        "% of wages"
                    }
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
            "Operating levels: {} {}% · services {}% GDP · welfare {}% GDP",
            if self.content.income_taxes.is_some() {
                "standard income-tax rate"
            } else {
                "income tax"
            },
            c.tax,
            c.services,
            c.welfare
        ));
        ui.label(format!(
            "Domestic capacity: energy {}% · food {}% · administration {}%",
            c.energy_capacity, c.food_capacity, c.administration
        ));
        if self.selected != w.player {
            ui.label("Foreign government: select your own country to issue commands.");
            return;
        }
        technologies::show(ui, &self.content, w, &self.policy);
        if self.policy == "income_tax" {
            income_taxes::show(ui, &self.content, w, Some(self.value));
        }
        let mut prepare_funding = false;
        match w.preview(&self.content, &self.selected, &self.policy, self.value) {
            Ok(preview) => {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.strong("Decision preview · forecast");
                    ui.label(format!(
                        "Cash now: {}m · delivery: {} days · annual consolidated net expenditure change: {:+}m",
                        preview.cost.0, preview.delay.units(), preview.annual_budget_change.0
                    ));
                    if self.policy == "coalition" {
                        ui.label("Authority: Direct governing negotiation; backing changes after delivery.");
                    } else { ui.label(format!("Authority: {}", preview.route)); }
                    ui.label(format!("Paid upfront {}m; reserve {}m until consent in {} days. Reserved money cannot fund other obligations.",preview.upfront_payment.0,preview.cost.0-preview.upfront_payment.0,preview.consent_delay.units()));
                    ui.strong("Direct household income change after delivery · forecast m/day");
                    for impact in &preview.household_impacts {ui.label(format!("{}: {:+}m",impact.group,impact.daily_disposable_change.0));}
                    ui.label(preview.explanation);
                });
                if ui
                    .button(
                        RichText::new(if preview.consent_delay.units() == 0 {
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
                let definition = self.content.country(&self.selected).expect("country");
                if w.governing_rules(&self.content, &self.selected)
                    .expect("government")
                    .check(c, self.policy.as_str(), self.value)
                    .is_err()
                    && self.content.policy(&"coalition".into()).is_some()
                    && ui.button("Review governing-support agreement").clicked()
                {
                    self.policy = "coalition".into();
                    self.value = self
                        .content
                        .policy(&"coalition".into())
                        .expect("policy")
                        .min;
                }
                if self.content.cash_management.is_some()
                    && w.policy_funding_plan(&self.content, &self.policy, self.value)
                        .is_ok_and(|plan| plan.shortfall.0 > 0)
                    && ui.button("Plan cash retention for this decision").clicked()
                {
                    prepare_funding = true;
                }
                if (definition.gdp.units() * p.cost_bps.units_i64() / 10000).max(1)
                    > c.available_cash().0
                    && ui.button("Review treasury and financing").clicked()
                {
                    self.tab = Tab::Treasury;
                }
            }
        }
        if prepare_funding {
            match self.cash_management.prepare(
                &self.content,
                self.world.as_ref().expect("world"),
                &self.policy,
                self.value,
            ) {
                Ok(()) => {
                    self.pause();
                    self.tab = Tab::Treasury;
                }
                Err(reason) => self.status = reason,
            }
        }
        ui.separator();
        if let Some(w) = &self.world
            && let Some(action) = self.undertaking_panel.show(ui, &self.content, w)
        {
            self.issue(action);
        }
        if let Some(w) = &self.world
            && self.content.commitments.is_some()
        {
            if let Some(action) = self.commitment_panel.show(ui, &self.content, w) {
                self.issue(action);
            }
            return;
        }
        ui.heading("Committed programs");
        if let Some(w) = &self.world {
            let c = &w.countries[&self.selected];
            if c.projects.is_empty() {
                ui.label("No programs in implementation.");
            }
            for project in &c.projects {
                let progress = w.day.units().saturating_sub((project.start_day).units()) as f32
                    / ((project.due_day).units() - (project.start_day).units()) as f32;
                ui.label(format!(
                    "{} · level {} · committed {}m · due {}",
                    project.policy,
                    project.value,
                    project.cost.0,
                    date((project.due_day).units())
                ));
                ui.label(format!(
                    "Stage {:?} · reserved {}m · consent {}",
                    project.stage,
                    project.reserved.0,
                    date((project.consent_day).units())
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
        if w.day.units() == self.content.start_day().expect("validated bookmark") {
            ui.label("Opening state: advance one day for realized income, baskets and delivery.");
        } else {
            match living_standards(&c.households) {
                Ok(indicators) => {
                    ui.strong(format!(
                        "Realized household indicators · {} · daily flows",
                        date(w.day.units())
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
                                |gini| format!("{:.3}", f64::from(gini.units()) / 10000.0),
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
        ui.scope(|ui| {
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::horizontal()
                .id_salt(("household_table_scroll", self.selected.0.as_str()))
                .show(ui, |ui| {
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
                                        (h.income.0 - h.tax.0) as f64 * 100.0
                                            / h.basket_cost.0 as f64
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
        if funding::show(ui, &self.content, w, self.money, &mut self.funding_chart) {
            self.policy = "coalition".into();
            self.value = self
                .content
                .policy(&"coalition".into())
                .expect("coalition policy")
                .min;
            self.tab = Tab::Policies;
        }
        if income_taxes::show(ui, &self.content, w, None) {
            self.policy = "income_tax".into();
            self.value = c.tax;
            self.tab = Tab::Policies;
        }
        let mut compare = None;
        let cash_action = match self.cash_management.show(ui, &self.content, w) {
            Some(cash_management::Choice::Command(action)) => Some(action),
            Some(cash_management::Choice::Compare { policy, value }) => {
                compare = Some((policy, value));
                None
            }
            None => None,
        };
        let reserve_action = reserves::show(ui, &self.content, w);
        let budget_action = self.fiscal_panel.show(ui, &self.content, w);
        let lending_action = financing::show(ui, &self.content, w, &mut self.money);
        let mut finance_action = cash_action
            .or(reserve_action)
            .or(budget_action)
            .or(lending_action);
        if self.content.finance.is_none() {
            ui.horizontal_wrapped(|ui| {
                ui.label("Principal (m)");
                ui.add(
                    egui::DragValue::new(&mut self.money)
                        .range(1..=d.gdp.units() / 10)
                        .speed((d.gdp.units() / 10000).max(1) as f64),
                );
                if ui.button("Issue loan").clicked() {
                    finance_action = Some(Action::Borrow {
                        amount: Money(self.money),
                    });
                }
                if ui.button("Repay debt").clicked() {
                    finance_action = Some(Action::Repay {
                        amount: Money(self.money),
                    });
                }
            });
        }
        ui.separator();
        if c.subnational.is_some() {
            ui.label("Public cash and execution consolidate national and local accounts. National availability excludes regional entitlements, accepted local grants and pending grant escrow.");
            ui.small("Frozen forecasts project current purchasing rates; regional grant term records show their finite end dates separately. The funding outlook uses national cash flow after local tax entitlements.");
        }
        egui::Grid::new("accounts")
            .striped(true)
            .min_col_width(180.0)
            .show(ui, |ui| {
                for (name, value) in [
                    (
                        if c.subnational.is_some() {
                            "Consolidated public cash"
                        } else {
                            "Treasury cash"
                        },
                        c.accounts.treasury.0,
                    ),
                    ("Reserved policy capital", c.committed_cash.0),
                    ("Available treasury", c.available_cash().0),
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
                if c.subnational.is_some() {
                    ui.label("Protected local funds / escrow");
                    ui.monospace(format!("{}m", c.regional_protected_cash().0));
                    ui.end_row();
                }
            });
        ui.separator();
        let plan = &c.budget;
        ui.strong(format!(
            "Fiscal period {} to {} · opening forecast vs execution",
            date((plan.start_day).units()),
            date((plan.end_day).units())
        ));
        ui.label(format!("Revenue realized {}m / opening forecast {}m to date ({:+}m variance). Payments {}m · capital {}m · net financing {}m.",plan.revenue.0,plan.expected_revenue(w.day.units()),plan.revenue.0-plan.expected_revenue(w.day.units()),plan.payments.0,plan.capital.0,plan.borrowing.0-plan.principal_repaid.0));
        ui.label(format!(
            "Opening forecast closing cash {}m. Cash reconciliation now {}m = treasury {}m.",
            plan.forecast_closing_cash(),
            plan.expected_cash(),
            c.accounts.treasury.0
        ));
        let latest = c.forecast(&d, self.content.income_taxes.as_ref());
        let rate = latest.interest_accrual.as_ref().map_or_else(
            || format!("{}m/day", latest.daily_interest.0),
            |i| {
                format!(
                    "{:.4}m/day mean; {}m due tomorrow including carry",
                    i.average_per_day(),
                    latest.daily_interest.0
                )
            },
        );
        ui.label(format!("Current no-shock run rate: revenue {}m/day, spending {}m/day, interest {rate}. It excludes pending programs and new financing; forecasts are not spendable money.",latest.daily_revenue.0,latest.daily_spending.0));
        ui.small("Forecast differences reflect supply shocks, rounded tax collection, policy changes and cash-constrained private income. Arrears and capital payments are shown separately.");
        egui::CollapsingHeader::new("Closed fiscal periods").show(ui, |ui| {
            for review in c.budget_reviews.iter().rev() {
                ui.label(format!(
                    "{}: revenue {}m, payments {}m, capital {}m, closing cash {}m, arrears {}m",
                    date((review.period.end_day).units()),
                    review.period.revenue.0,
                    review.period.payments.0,
                    review.period.capital.0,
                    review.closing_cash.0,
                    review.arrears.0
                ));
            }
        });
        ui.small(format!("Reconciliation: sector cash {}m = opening {}m. External debt uses explicit principal records and 4% annual interest. Borrowing ceiling: {}m.",c.accounts.total(),c.opening_money.0,d.gdp.units()*3/2));
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
                    date(e.day.units()),
                    e.from,
                    e.to,
                    e.amount.0,
                    e.reason
                ));
            }
        }
        if let Some(action) = finance_action {
            self.issue(action);
        }
        if let Some((policy, value)) = compare {
            self.pause();
            self.policy = policy;
            self.value = value;
            self.tab = Tab::Policies;
        }
    }
    fn politics(&mut self, ui: &mut egui::Ui) {
        party_preferences::show_reference(ui, &self.content, &self.selected);
        if self.foreign_report(ui) {
            return;
        }

        if let Some(world) = &self.world {
            governing_institutions::show(ui, &self.content, world);
            executive_identity::show(ui, &self.content, world);
        }
        if let Some(world) = &self.world
            && let Some(action) = legislation::show(ui, &self.content, world)
        {
            self.issue(action);
        }
        let d = self
            .content
            .country(&self.selected)
            .expect("country")
            .clone();
        ui.heading("Institutions & political survival");
        let regime = self
            .world
            .as_ref()
            .and_then(|world| world.governing_regime(&self.content, &self.selected).ok())
            .unwrap_or(&d.regime);
        let government = self
            .world
            .as_ref()
            .and_then(|world| world.governing_rules(&self.content, &self.selected).ok())
            .unwrap_or(&d.government);
        ui.label(format!("{} · {:?}\n{}", d.name, regime, government.label()));
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
                    .map(|day| date(day.units()))
                    .unwrap_or_else(|| "None scheduled".into())
            ));
            ui.label(format!(
                "Next election: {}",
                c.next_election
                    .map(|day| date(day.units()))
                    .unwrap_or_else(|| {
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
        if let Some(world) = &self.world {
            party_preferences::show_owned(ui, &self.content, world);
        }
        if c.is_some()
            && self.content.policy(&"coalition".into()).is_some()
            && ui.button("Review governing-support agreement").clicked()
        {
            self.policy = "coalition".into();
            self.value = self
                .content
                .policy(&"coalition".into())
                .expect("policy")
                .min;
            self.tab = Tab::Policies;
        }
        ui.separator();
        ui.label("Authority thresholds are content-defined. Parliamentary support can trigger an early confidence election after three weak monthly reviews. Party councils review cadre backing; royal councils can block a spending expansion above 4 GDP points without 65 backing. Consent is rechecked before construction. Country play continues after leadership changes.");
        ui.label("Cohort needs, disposable income, taxes, employment and funded services affect opinion. Parliamentary support, elite backing and council backing are distinct. Seat allocation, succession and preferences remain scenario approximations.");
        if let Some(world) = &self.world
            && let Some(action) = self.cabinet_panel.show(ui, &self.content, world)
        {
            self.issue(action);
        }
        if let Some(world) = &self.world
            && let Some(action) = self.subnational_panel.show(ui, &self.content, world)
        {
            self.issue(action);
        }
        if let Some(world) = &self.world
            && let Some(action) = elections::show(ui, &self.content, world)
        {
            self.issue(action);
        }
        if let Some(world) = &self.world
            && let Some(action) = self.constitution_panel.show(ui, &self.content, world)
        {
            self.issue(action);
        }
    }
    fn chronicle(&mut self, ui: &mut egui::Ui) {
        if self.foreign_report(ui) {
            return;
        }
        if let Some(request) = self
            .event_browser
            .show(ui, &self.content, self.world.as_ref())
        {
            self.commitment_panel.focus(request);
            self.tab = Tab::Policies;
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
        if let Some(world) = &self.world {
            let navigation =
                self.walkthrough
                    .show(ui, &self.content, world, self.selected == world.player);
            if let Some(navigation) = navigation {
                self.selected = self.world.as_ref().unwrap().player.clone();
                match navigation {
                    walkthrough::Navigation::Open(tab) => {
                        self.pause();
                        self.tab = tab;
                    }
                    walkthrough::Navigation::Draft { policy, value } => {
                        self.pause();
                        self.policy = policy;
                        self.value = value;
                        self.tab = Tab::Policies;
                    }
                    walkthrough::Navigation::Advance(days) => {
                        self.speed = 0;
                        self.start_advance(days, true);
                    }
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
    fn process_shortcuts(&mut self, ctx: &egui::Context, keyboard_owned_at_start: bool) {
        // Enter can surrender focus before shortcuts run. Include editors that
        // claimed and released input during this frame, as well as prior focus.
        let keyboard_owned = keyboard_owned_at_start
            || ctx.wants_keyboard_input()
            || ctx.output(|output| {
                output
                    .events
                    .iter()
                    .any(|event| event.widget_info().typ == egui::WidgetType::TextEdit)
            });
        if !keyboard_owned && !egui::Popup::is_any_open(ctx) && self.recovery.is_empty() {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft))
            {
                self.navigate_history(false);
            }
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowRight))
            {
                self.navigate_history(true);
            }
        }
        if settings::shortcut(ctx, self.preferences.active.pause, keyboard_owned)
            && self.world.is_some()
        {
            if self.speed > 0 || self.pending.is_some() {
                self.pause();
            } else {
                self.speed = 1;
            }
            self.last_tick = Instant::now();
        }
        if settings::shortcut(ctx, self.preferences.active.save, keyboard_owned) {
            self.save();
        }
        if settings::shortcut(ctx, self.preferences.active.load, keyboard_owned) {
            self.load();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1)) {
            self.pause();
            self.tab = Tab::Settings;
        }
        if settings::shortcut(ctx, self.preferences.active.next_event, keyboard_owned) {
            self.next_event();
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
        let keyboard_owned_at_start = ctx.wants_keyboard_input();
        self.navigation.observe(self.inspection());
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
            self.advance(DaySpan::new(self.speed));
            self.last_tick = Instant::now();
        }
        self.top(ctx);
        let mut opened_chronicle = false;
        if self.pending.is_none()
            && self.speed == 0
            && let Some(world) = &self.world
            && self.briefing.show(ctx, world)
        {
            self.selected = world.player.clone();
            self.tab = Tab::Chronicle;
            opened_chronicle = true;
        }
        self.sidebar(ctx);
        if !self.recovery.is_empty() {
            let mut chosen = None;
            let mut close = false;
            egui::Window::new("Save recovery · review before restoring").collapsible(false).show(ctx,|ui|{
                ui.label("Restoration preserves the current file and interrupted writes before replacing the primary. The active campaign changes only after successful recovery.");
                for candidate in &self.recovery {
                    ui.separator();ui.label(format!("{:?}: {}",candidate.kind,candidate.path.display()));
                    if let Some(error)=&candidate.error{ui.colored_label(Color32::LIGHT_YELLOW,error);}else if let Some(day)=candidate.day {ui.label(format!("Valid campaign at {} · snapshot format {} · {} · rules {}",date(day.units()),candidate.schema.unwrap_or_default(),candidate.content_id.as_deref().unwrap_or("unknown pack"),candidate.rules.unwrap_or_default()));if ui.add_enabled(self.pending.is_none(),egui::Button::new(format!("Restore {:?} from {}",candidate.kind,date(day.units())))).clicked(){chosen=Some(candidate.path.clone());}}
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
                        self.target_date = date((world.day.units() + 30).min(MAX_DAY));
                        self.world = Some(world);
                        self.navigation = navigation::History::default();
                        self.undertaking_panel = undertakings::Panel::default();
                        self.fiscal_panel = fiscal_plans::Panel::default();
                        self.walkthrough = walkthrough::Panel::default();
                        self.cash_management = cash_management::Panel::default();
                        self.event_browser = chronicle::Panel::default();
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
            let mut scroll = egui::ScrollArea::vertical().id_salt((
                "main_scroll",
                self.tab,
                self.selected.0.as_str(),
            ));
            if opened_chronicle {
                scroll = scroll.vertical_scroll_offset(0.0);
            }
            scroll.show(ui, |ui| match self.tab {
                Tab::Overview => self.overview(ui),
                Tab::Policies => self.policies(ui),
                Tab::Treasury => self.treasury(ui),
                Tab::Society => self.society(ui),
                Tab::Analysis => {
                    self.analysis
                        .show(ui, &self.content, self.world.as_ref(), &mut self.selected)
                }
                Tab::Politics => self.politics(ui),
                Tab::Chronicle => self.chronicle(ui),
                Tab::Guide => self.guide(ui),
                Tab::Settings => self.preferences.show(ui),
                Tab::Inspector => self.developer_inspector(ui),
            });
        });
        let setup_open = self.new_setup.is_some();
        self.new_campaign_dialog(ctx);
        if !setup_open {
            self.process_shortcuts(ctx, keyboard_owned_at_start);
        }
        self.navigation.observe(self.inspection());
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
    fn first_launch_start_is_visible_and_starts_the_combined_default_at_two_scales() {
        for scale in [1., 1.4] {
            let content = Content::bundled().unwrap();
            let ctx = egui::Context::default();
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            let mut frame = |events| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1280. / scale, 800. / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                )
            };
            frame(vec![]);
            frame(vec![]);
            let output = frame(vec![]);
            let point = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if text.galley.text() == "Begin as United Kingdom >" =>
                    {
                        let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                        shape.clip_rect.contains_rect(rect).then_some(rect.center())
                    }
                    _ => None,
                })
                .expect("The complete start action must be visible without scrolling");
            for pressed in [true, false] {
                frame(vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
            }
            let world = app
                .world
                .as_ref()
                .expect("Visible start button starts the country");
            world.validate(&content).unwrap();
            assert_eq!(world.player, CountryId("GBR".into()));
            assert_eq!(world.day.units(), 0);
            assert!(world.journal.is_empty());
            assert!(
                world
                    .player_country()
                    .statistical_archive
                    .as_ref()
                    .unwrap()
                    .years
                    .is_empty()
            );
            assert!(!content.bookmark_offices(&world.player).is_empty());
            assert_eq!(persistence::snapshot_schema(world), 26);
            assert_eq!(app.save_path, "saves/campaign-v26.json");
            assert_eq!(app.speed, 0);
            assert!(app.pending.is_none());
        }
    }

    fn editor_shortcut_frame(
        ctx: &egui::Context,
        app: &mut GameApp,
        scale: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440. / scale, 940. / scale),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.render(ctx),
        )
    }
    fn editor_shortcut_point(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    let point = text.pos + text.galley.size() / 2.;
                    shape.clip_rect.contains(point).then_some(point)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Visible editor text missing: {label}"))
    }
    fn editor_shortcut_focus(ctx: &egui::Context, app: &mut GameApp, scale: f32, label: &str) {
        let output = editor_shortcut_frame(ctx, app, scale, vec![]);
        let point = editor_shortcut_point(&output, label);
        for pressed in [true, false] {
            editor_shortcut_frame(
                ctx,
                app,
                scale,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(ctx.wants_keyboard_input());
    }
    fn editor_shortcut_keys(key: egui::Key, text: Option<&str>, enter: bool) -> Vec<egui::Event> {
        let key_event = |key, pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let mut events = vec![key_event(key, true)];
        if let Some(text) = text {
            events.push(egui::Event::Text(text.into()));
        }
        events.push(key_event(key, false));
        if enter {
            events.push(key_event(egui::Key::Enter, true));
            events.push(key_event(egui::Key::Enter, false));
        }
        events
    }
    #[test]
    fn ending_or_newly_claiming_an_editor_cannot_seek_pause_or_navigate_at_two_scales() {
        let content =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(61)).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.world = Some(world.clone());
            app.selected = world.player.clone();
            app.save_path = "n".into();
            app.target_date = "date".into();
            editor_shortcut_frame(&ctx, &mut app, scale, vec![]);
            editor_shortcut_frame(&ctx, &mut app, scale, vec![]);
            app.tab = Tab::Society;
            let output = editor_shortcut_frame(&ctx, &mut app, scale, vec![]);
            assert!(!ctx.wants_keyboard_input());
            let point = editor_shortcut_point(&output, "n");
            let mut events = vec![egui::Event::PointerMoved(point)];
            for pressed in [true, false] {
                events.push(egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            events.extend(editor_shortcut_keys(egui::Key::N, Some("n"), true));
            editor_shortcut_frame(&ctx, &mut app, scale, events);
            assert_eq!(app.save_path, "nn");
            assert!(
                !ctx.wants_keyboard_input(),
                "Enter may finish the edit without releasing its keys to the game"
            );
            assert!(
                app.pending.is_none(),
                "Click/type/Enter in one frame must not start event advancement"
            );
            assert_eq!(app.speed, 0);
            let path = app.save_path.clone();
            editor_shortcut_focus(&ctx, &mut app, scale, &path);
            editor_shortcut_frame(
                &ctx,
                &mut app,
                scale,
                editor_shortcut_keys(egui::Key::N, None, true),
            );
            assert!(
                app.pending.is_none(),
                "An editor focused at frame start still owns its confirmation frame"
            );
            editor_shortcut_focus(&ctx, &mut app, scale, "date");
            editor_shortcut_frame(
                &ctx,
                &mut app,
                scale,
                editor_shortcut_keys(egui::Key::Space, Some(" "), true),
            );
            assert_eq!(
                app.speed, 0,
                "Space in an ending editor must not start automatic time"
            );
            let target = app.target_date.clone();
            editor_shortcut_focus(&ctx, &mut app, scale, &target);
            let mut events = vec![egui::Event::Key {
                key: egui::Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::ALT,
            }];
            events.extend(editor_shortcut_keys(egui::Key::Enter, None, false));
            editor_shortcut_frame(&ctx, &mut app, scale, events);
            assert!(app.tab == Tab::Society, "Editing must not also invoke Back");
            // Ordinary shortcuts work on the following frame, even with the OS's text event.
            app.last_tick = Instant::now();
            editor_shortcut_frame(
                &ctx,
                &mut app,
                scale,
                editor_shortcut_keys(egui::Key::Space, Some(" "), false),
            );
            assert_eq!(app.speed, 1);
            app.last_tick = Instant::now();
            editor_shortcut_frame(
                &ctx,
                &mut app,
                scale,
                editor_shortcut_keys(egui::Key::Space, Some(" "), false),
            );
            assert_eq!(app.speed, 0);
            assert_eq!(
                persistence::encode(app.world.as_ref().unwrap(), &content).unwrap(),
                before
            );
        }
    }
    #[test]
    fn rebound_letters_do_not_save_or_load_from_editors_but_function_keys_still_work_at_two_scales()
    {
        let content =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(61)).unwrap();
        let other = World::new(&content, world.player.clone(), 18).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        let dir =
            std::env::temp_dir().join(format!("geosim-editor-shortcuts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let other_path = dir.join("other.json");
        std::fs::write(&other_path, persistence::encode(&other, &content).unwrap()).unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.world = Some(world.clone());
            app.selected = world.player.clone();
            app.tab = Tab::Society;
            app.target_date = "date".into();
            app.preferences.active.save = egui::Key::S;
            app.preferences.active.load = egui::Key::L;
            let path = dir.join(format!("ignored-{scale}.json"));
            app.save_path = path.to_string_lossy().into_owned();
            editor_shortcut_frame(&ctx, &mut app, scale, vec![]);
            editor_shortcut_frame(&ctx, &mut app, scale, vec![]);
            editor_shortcut_focus(&ctx, &mut app, scale, "date");
            editor_shortcut_frame(
                &ctx,
                &mut app,
                scale,
                editor_shortcut_keys(egui::Key::S, Some("s"), true),
            );
            assert!(
                !path.exists(),
                "A rebound letter inside text must not write a snapshot"
            );
            app.save_path = other_path.to_string_lossy().into_owned();
            app.target_date = "date".into();
            editor_shortcut_focus(&ctx, &mut app, scale, "date");
            editor_shortcut_frame(
                &ctx,
                &mut app,
                scale,
                editor_shortcut_keys(egui::Key::L, Some("l"), true),
            );
            assert_eq!(
                persistence::encode(app.world.as_ref().unwrap(), &content).unwrap(),
                before,
                "A rebound letter inside text must not replace the active campaign"
            );
            app.preferences.active.save = egui::Key::F5;
            app.preferences.active.load = egui::Key::F9;
            let explicit = dir.join(format!("explicit-{scale}.json"));
            app.save_path = explicit.to_string_lossy().into_owned();
            app.target_date = "date".into();
            editor_shortcut_focus(&ctx, &mut app, scale, "date");
            editor_shortcut_frame(
                &ctx,
                &mut app,
                scale,
                editor_shortcut_keys(egui::Key::F5, None, false),
            );
            assert_eq!(
                persistence::load(&explicit, &content).unwrap(),
                world,
                "Explicit function-key save stays available during editing"
            );
            assert!(ctx.wants_keyboard_input());
            app.save_path = other_path.to_string_lossy().into_owned();
            editor_shortcut_frame(
                &ctx,
                &mut app,
                scale,
                editor_shortcut_keys(egui::Key::F9, None, false),
            );
            assert_eq!(
                app.world.as_ref(),
                Some(&other),
                "Explicit function-key load stays available during editing"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn chronicle_links_open_exact_pending_and_historical_programmes_at_both_scales_without_orders()
    {
        fn labels(output: &egui::FullOutput) -> String {
            output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some(t.galley.text()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        fn click(ctx: &egui::Context, app: &mut GameApp, input: &egui::RawInput, label: &str) {
            let output = ctx.run(input.clone(), |ctx| app.render(ctx));
            let pos = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        let center = t.pos + t.galley.size() / 2.0;
                        s.clip_rect.contains(center).then_some(center)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing visible {label}: {}", labels(&output)));
            let mut pointer = input.clone();
            pointer.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ];
            let _ = ctx.run(pointer, |ctx| app.render(ctx));
        }
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-project-events-test.json"
        ))
        .unwrap();
        let mut pending = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        pending
            .issue(
                &content,
                Action::Enact {
                    policy: "income_tax".into(),
                    value: 30,
                },
            )
            .unwrap();
        let request = pending.player_country().projects[0].id;
        let mut withdrawn = pending.clone();
        withdrawn
            .issue(&content, Action::CancelCommitment { request })
            .unwrap();
        let mut reviewed = pending.clone();
        let due = reviewed.player_country().project(&request).unwrap().due_day;
        reviewed
            .advance(&content, sim_core::DaySpan::new((due).units()))
            .unwrap();
        let review = reviewed
            .player_country()
            .commitment(&request)
            .unwrap()
            .review_due
            .unwrap();
        reviewed
            .advance(
                &content,
                sim_core::DaySpan::new((review).units() - reviewed.day.units()),
            )
            .unwrap();
        for (world, expected) in [
            (pending, "Awaiting consent"),
            (withdrawn, "Withdrawn"),
            (reviewed, "Observed review"),
        ] {
            for scale in [1.0, 1.4] {
                let ctx = egui::Context::default();
                let mut app =
                    GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
                ctx.set_zoom_factor(scale);
                app.world = Some(world.clone());
                app.selected = world.player.clone();
                app.tab = Tab::Chronicle;
                let input = egui::RawInput {
                    focused: true,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440.0 / scale, 1000.0 / scale),
                    )),
                    ..Default::default()
                };
                let before = app.world.clone();
                click(&ctx, &mut app, &input, "Dismiss brief");
                click(
                    &ctx,
                    &mut app,
                    &input,
                    &format!("Inspect programme #{request}"),
                );
                assert!(app.tab == Tab::Policies);
                for _ in 0..3 {
                    let _ = ctx.run(input.clone(), |ctx| app.render(ctx));
                }
                let output = ctx.run(input.clone(), |ctx| app.render(ctx));
                let text = labels(&output);
                assert!(
                    text.contains(&format!("Inspecting programme #{request}")),
                    "{scale}: {text}"
                );
                assert!(text.contains(expected), "{scale}: {text}");
                click(&ctx, &mut app, &input, "Show all programmes");
                let output = ctx.run(input.clone(), |ctx| app.render(ctx));
                assert!(!labels(&output).contains("Inspecting programme"));
                // A prior user collapse must not hide the next event's history.
                click(&ctx, &mut app, &input, "Programme history · 1 record");
                app.tab = Tab::Chronicle;
                click(
                    &ctx,
                    &mut app,
                    &input,
                    &format!("Inspect programme #{request}"),
                );
                for _ in 0..3 {
                    let _ = ctx.run(input.clone(), |ctx| app.render(ctx));
                }
                let reopened = ctx.run(input.clone(), |ctx| app.render(ctx));
                assert!(
                    labels(&reopened).contains(expected),
                    "reopened {scale}: {}",
                    labels(&reopened)
                );
                app.selected = CountryId("BOR".into());
                app.tab = Tab::Chronicle;
                let output = ctx.run(input.clone(), |ctx| app.render(ctx));
                assert!(!labels(&output).contains("Inspect programme #"));
                assert_eq!(app.world, before);
            }
        }
    }
    #[test]
    fn event_briefs_are_owner_only_dismissible_and_open_the_owned_chronicle_without_commands() {
        for scale in [1.0, 1.4] {
            let content =
                Content::from_json(include_str!("../../../data/synthetic-election-test.json"))
                    .unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            world.events.push(Event {
                context: None,
                id: None,
                day: sim_core::CampaignDay::new(0),
                country: world.player.clone(),
                important: true,
                text: "OWN IMPORTANT DECISION".into(),
            });
            world.events.push(Event {
                context: None,
                id: None,
                day: sim_core::CampaignDay::new(0),
                country: CountryId("BOR".into()),
                important: true,
                text: "PRIVATE FOREIGN DECISION".into(),
            });
            let ctx = egui::Context::default();
            let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.selected = CountryId("BOR".into());
            app.tab = Tab::Politics;
            app.world = Some(world);
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0 / scale, 700.0 / scale),
                )),
                ..Default::default()
            };
            let text = |output: &egui::FullOutput| {
                output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(t) => Some(t.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let click = |app: &mut GameApp, output: &egui::FullOutput, label: &str| {
                let position = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(t) if t.galley.text() == label => {
                            let center = t.pos + t.galley.size() / 2.0;
                            assert!(shape.clip_rect.contains(center));
                            Some(center)
                        }
                        _ => None,
                    })
                    .unwrap();
                for pressed in [true, false] {
                    let mut frame = input.clone();
                    frame.events = vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ];
                    let _ = ctx.run(frame, |ctx| app.render(ctx));
                }
            };
            let original = app.world.as_ref().unwrap().hash();
            let output = ctx.run(input.clone(), |ctx| app.render(ctx));
            assert!(
                text(&output).contains("OWN IMPORTANT DECISION"),
                "{}",
                text(&output)
            );
            assert!(!text(&output).contains("PRIVATE FOREIGN DECISION"));
            click(&mut app, &output, "Dismiss brief");
            let output = ctx.run(input.clone(), |ctx| app.render(ctx));
            assert!(!text(&output).contains("OWN IMPORTANT DECISION"));
            assert_eq!(app.world.as_ref().unwrap().hash(), original);
            // A foreign notice cannot reopen an acknowledged owner brief.
            app.world.as_mut().unwrap().events.push(Event {
                context: None,
                id: None,
                day: sim_core::CampaignDay::new(0),
                country: CountryId("BOR".into()),
                important: true,
                text: "ANOTHER PRIVATE FOREIGN DECISION".into(),
            });
            let output = ctx.run(input.clone(), |ctx| app.render(ctx));
            assert!(!text(&output).contains("OWN IMPORTANT DECISION"));
            app.world.as_mut().unwrap().events.push(Event {
                context: None,
                id: None,
                day: sim_core::CampaignDay::new(0),
                country: CountryId("AUR".into()),
                important: true,
                text: "NEW OWN IMPORTANT DECISION".into(),
            });
            let expected = app.world.as_ref().unwrap().hash();
            let output = ctx.run(input.clone(), |ctx| app.render(ctx));
            assert!(text(&output).contains("NEW OWN IMPORTANT DECISION"));
            click(&mut app, &output, "Open chronicle");
            assert_eq!(app.selected.0, "AUR");
            assert!(app.tab == Tab::Chronicle);
            let output = ctx.run(input, |ctx| app.render(ctx));
            assert!(text(&output).contains("Chronicle & decisions"));
            assert!(text(&output).contains("NEW OWN IMPORTANT DECISION"));
            assert!(!text(&output).contains("PRIVATE FOREIGN DECISION"));
            assert_eq!(app.world.as_ref().unwrap().hash(), expected);
        }
    }
    #[test]
    fn blocked_policy_planning_prefills_treasury_and_returns_to_the_same_funded_comparison() {
        fn labels(output: &egui::FullOutput) -> String {
            output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some(t.galley.text()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        fn paint(
            ctx: &egui::Context,
            app: &mut GameApp,
            input: &egui::RawInput,
        ) -> egui::FullOutput {
            ctx.run(input.clone(), |ctx| app.render(ctx))
        }
        fn click(ctx: &egui::Context, app: &mut GameApp, input: &egui::RawInput, label: &str) {
            let output = paint(ctx, app, input);
            let position = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.text() == label
                            && s.clip_rect.contains(t.pos + t.galley.size() / 2.0) =>
                    {
                        Some(t.pos + t.galley.size() / 2.0)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("visible button {label} missing: {}", labels(&output)));
            let mut pressed = input.clone();
            pressed.events = vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ];
            let _ = ctx.run(pressed, |ctx| app.render(ctx));
        }
        for scale in [1.0, 1.4] {
            let content = Content::from_json(include_str!(
                "../../../data/synthetic-cash-management-test.json"
            ))
            .unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            world
                .issue(
                    &content,
                    Action::Repay {
                        amount: world.player_country().available_cash(),
                    },
                )
                .unwrap();
            let ctx = egui::Context::default();
            let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.world = Some(world);
            app.tab = Tab::Policies;
            app.policy = "income_tax".into();
            app.value = 30;
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                ..Default::default()
            };
            let before = app.world.clone();
            click(
                &ctx,
                &mut app,
                &input,
                "Plan cash retention for this decision",
            );
            assert!(app.tab == Tab::Treasury);
            assert_eq!(app.world, before);
            // Allow the requested scroll into the retention workspace to settle.
            for _ in 0..32 {
                let _ = paint(&ctx, &mut app, &input);
            }
            let output = paint(&ctx, &mut app, &input);
            let text = labels(&output);
            assert!(text.contains("proposed 30%"));
            assert!(text.contains("Implementation cost 20m"));
            assert!(text.contains("remaining 20m must come from future actual cash"));
            app.selected = CountryId("BOR".into());
            let output = paint(&ctx, &mut app, &input);
            assert!(!labels(&output).contains("Planned decision"));
            assert!(!labels(&output).contains("Plan cash retention"));
            app.selected = CountryId("AUR".into());
            click(
                &ctx,
                &mut app,
                &input,
                "Retain actual cash with these consequences",
            );
            assert_eq!(
                app.world
                    .as_ref()
                    .unwrap()
                    .player_country()
                    .cash_management
                    .as_ref()
                    .unwrap()
                    .active()
                    .unwrap()
                    .target,
                Money(20)
            );
            let content = app.content.clone();
            app.world
                .as_mut()
                .unwrap()
                .advance(&content, sim_core::DaySpan::new(2))
                .unwrap();
            for _ in 0..4 {
                let _ = paint(&ctx, &mut app, &input);
            }
            click(&ctx, &mut app, &input, "Release 20m of actual cash");
            let funded = app.world.clone();
            app.policy = "services".into();
            app.value = 12;
            click(&ctx, &mut app, &input, "Compare planned decision");
            assert!(app.tab == Tab::Policies);
            assert_eq!((app.policy.as_str(), app.value), ("income_tax", 30));
            assert_eq!(app.world, funded);
            // Read the full comparison before reaching its ordinary approval,
            // which is below the fold at the larger interface scale.
            let mut scroll = input.clone();
            scroll.events = vec![
                egui::Event::PointerMoved(egui::pos2(1000.0 / scale, 700.0 / scale)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -300.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ];
            let _ = ctx.run(scroll, |ctx| app.render(ctx));
            for _ in 0..32 {
                let _ = paint(&ctx, &mut app, &input);
            }
            click(&ctx, &mut app, &input, "Propose and reserve funds");
            let w = app.world.as_ref().unwrap();
            assert_eq!(w.player_country().projects[0].policy, "income_tax");
            assert_eq!(w.player_country().projects[0].value, 30);
            assert_eq!(w.player_country().committed_cash, Money(18));
            assert_eq!(persistence::replay(w, &app.content).unwrap(), *w);
        }
    }

    #[test]
    fn blocked_borrowing_links_to_a_support_preview_without_issuing_an_order_or_leaking_foreign_cash()
     {
        for scale in [1.0, 1.4] {
            let content =
                Content::from_json(include_str!("../../../data/synthetic-finance-test.json"))
                    .unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            world
                .issue(
                    &content,
                    Action::EmergencyBorrow {
                        amount: Money(1000),
                    },
                )
                .unwrap();
            let expected = world.hash();
            let ctx = egui::Context::default();
            let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.world = Some(world);
            app.tab = Tab::Treasury;
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                ..Default::default()
            };
            let output = ctx.run(input.clone(), |ctx| app.render(ctx));
            let position = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.text() == "Review governing-support agreement"
                    {
                        let center = text.pos + text.galley.size() / 2.0;
                        assert!(shape.clip_rect.contains(center));
                        Some(center)
                    } else {
                        None
                    }
                })
                .unwrap();
            let mut click = input.clone();
            click.events = vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ];
            let _ = ctx.run(click, |ctx| app.render(ctx));
            assert!(app.tab == Tab::Policies);
            assert_eq!(app.policy, "coalition");
            assert_eq!(app.world.as_ref().unwrap().hash(), expected);
            let output = ctx.run(input.clone(), |ctx| app.render(ctx));
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().contains("Authority: Direct governing negotiation"))));
            app.selected = CountryId("BOR".into());
            app.tab = Tab::Treasury;
            let output = ctx.run(input, |ctx| app.render(ctx));
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!text.contains("Funding outlook"));
            assert!(!text.contains("Uncommitted cash"));
            assert!(!text.contains("Review governing-support agreement"));
            assert_eq!(app.world.as_ref().unwrap().hash(), expected);
        }
    }
    #[test]
    fn next_event_shortcut_searches_until_the_actual_event_in_a_leap_year() {
        let mut content =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        for country in &mut content.countries {
            country.tax = sim_core::Percent::new(30);
            country.services = sim_core::Percent::new(6);
            country.welfare = sim_core::Percent::new(4);
        }
        let world = World::new(&content, CountryId("AUR".into()), 0).unwrap();
        let ctx = egui::Context::default();
        let mut app =
            GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
        app.world = Some(world);
        app.speed = 30;
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::N,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            app.process_shortcuts(ctx, ctx.wants_keyboard_input())
        });
        assert_eq!(app.speed, 0);
        assert!(app.playback_label().starts_with("Searching for next event"));
        let job = app
            .pending
            .take()
            .expect("Next-event shortcut starts a worker");
        assert!(job.seeking_event);
        assert_eq!(
            job.requested,
            DaySpan::new(MAX_DAY),
            "Search must not end at a fixed 365-day horizon"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let outcome = loop {
            if let Some(outcome) = job.poll().unwrap() {
                break outcome;
            }
            assert!(
                Instant::now() < deadline,
                "Next-event worker did not finish"
            );
            std::thread::sleep(Duration::from_millis(1));
        };
        assert!(outcome.error.is_none());
        assert!(outcome.stopped_for_event);
        assert_eq!(outcome.world.day.units(), 366);
        assert_eq!(
            outcome.world,
            persistence::replay(&outcome.world, &content).unwrap()
        );
        app.world.as_mut().unwrap().day = CampaignDay::new(MAX_DAY);
        app.next_event();
        assert!(app.pending.is_none());
        assert_eq!(app.playback_label(), "Paused");
        assert!(app.status.contains("calendar is at its limit"));
    }

    #[test]
    fn saving_before_a_new_campaign_archives_exact_active_state_outside_backup_rotation() {
        let classic = Content::classic_bundled().unwrap();
        let mut old = World::new(&classic, CountryId("GBR".into()), 17).unwrap();
        old.issue(
            &classic,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
        old.advance(&classic, sim_core::DaySpan::new(31)).unwrap();
        let expected_old = old.hash();
        let dir = std::env::temp_dir().join(format!("geosim-new-campaign-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let active_path = dir.join("current.json");
        let ctx = egui::Context::default();
        let mut app = GameApp::from_context(&ctx, classic, settings::Preferences::default());
        app.world = Some(old);
        app.save_path = active_path.display().to_string();
        app.bundled_catalog = true;
        let mut setup = new_game::Setup::new(&app.content, &app.selected, 42, true).unwrap();
        setup.player = CountryId("CHN".into());
        setup.scenario = Scenario::FiscalSupplyCrisis;
        setup.save_path = dir.join("new.json").display().to_string();
        let expected_new = setup.prepare().unwrap().hash();
        app.begin_new_campaign(&setup, true).unwrap();
        assert_eq!(app.world.as_ref().unwrap().hash(), expected_new);
        assert_eq!(app.content.rules, RULES_VERSION);
        assert_eq!(app.selected.0, "CHN");
        assert_eq!(app.speed, 0);
        let archive = dir.join("current.json.before-new-campaign-1");
        let archived_bytes = std::fs::read(&archive).unwrap();
        assert_eq!(
            persistence::load_bundled(&archive).unwrap().1.hash(),
            expected_old
        );
        assert_eq!(
            persistence::load_bundled(&active_path).unwrap().1.hash(),
            expected_old
        );
        for _ in 0..2 {
            app.world
                .as_mut()
                .unwrap()
                .advance(&app.content, sim_core::DaySpan::new(1))
                .unwrap();
            persistence::save(&active_path, app.world.as_ref().unwrap(), &app.content).unwrap();
        }
        assert_eq!(std::fs::read(&archive).unwrap(), archived_bytes);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn failed_save_and_invalid_new_setup_leave_the_active_campaign_unchanged() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, sim_core::DaySpan::new(10)).unwrap();
        let expected = world.hash();
        let dir = std::env::temp_dir().join(format!(
            "geosim-new-campaign-failure-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("current.json");
        persistence::save(&path, &world, &content).unwrap();
        let original = std::fs::read(&path).unwrap();
        std::fs::write(path.with_extension("json.tmp"), "interrupted").unwrap();
        let ctx = egui::Context::default();
        let mut app =
            GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
        app.world = Some(world);
        app.save_path = path.display().to_string();
        let mut setup = new_game::Setup::new(&content, &CountryId("CHN".into()), 42, true).unwrap();
        setup.save_path = dir.join("new.json").display().to_string();
        assert!(app.begin_new_campaign(&setup, true).is_err());
        assert_eq!(app.world.as_ref().unwrap().hash(), expected);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        setup.seed = "invalid seed".into();
        assert!(app.begin_new_campaign(&setup, false).is_err());
        assert_eq!(app.world.as_ref().unwrap().hash(), expected);
        assert_eq!(app.content.hash(), content.hash());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn campaign_file_controls_stay_visible_and_work_at_compact_and_large_interface_scales() {
        let directory = std::env::temp_dir().join(format!(
            "geosim-visible-file-controls-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        for (width, height) in [(1280.0, 800.0), (1440.0, 940.0)] {
            for scale in [1.0, 1.4] {
                let content = Content::bundled().unwrap();
                let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
                let expected = world.clone();
                let ctx = egui::Context::default();
                let mut app =
                    GameApp::from_context(&ctx, content, settings::Preferences::default());
                ctx.set_zoom_factor(scale);
                app.world = Some(world);
                app.tab = Tab::Chronicle;
                app.save_path = directory
                    .join(format!("campaign-{width}-{scale}.json"))
                    .display()
                    .to_string();
                let input = egui::RawInput {
                    focused: true,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width / scale, height / scale),
                    )),
                    ..Default::default()
                };
                let paint = |app: &mut GameApp| ctx.run(input.clone(), |ctx| app.render(ctx));
                let _ = paint(&mut app);
                let output = paint(&mut app);
                for label in [
                    "Save",
                    "Load",
                    "Inspect recovery snapshots",
                    "Return to my country",
                    "Save file / recovery .bak path",
                ] {
                    let shape = output.shapes.iter().find(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == label)).unwrap_or_else(|| panic!("Missing {label} at {width} and scale {scale}"));
                    if let egui::Shape::Text(text) = &shape.shape {
                        assert!(
                            shape
                                .clip_rect
                                .contains_rect(text.galley.rect.translate(text.pos.to_vec2())),
                            "{label} is clipped at {width} and scale {scale}"
                        );
                    }
                }
                let click = |app: &mut GameApp, output: &egui::FullOutput, label: &str| {
                    let position = output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.text() == label => {
                                Some(text.pos + egui::vec2(4.0, 4.0))
                            }
                            _ => None,
                        })
                        .unwrap();
                    let mut hover = input.clone();
                    hover.events = vec![egui::Event::PointerMoved(position)];
                    let _ = ctx.run(hover, |ctx| app.render(ctx));
                    for pressed in [true, false] {
                        let mut clicked = input.clone();
                        clicked.events = vec![egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        }];
                        let _ = ctx.run(clicked, |ctx| app.render(ctx));
                    }
                };
                click(&mut app, &output, "Save");
                assert_eq!(
                    persistence::load_bundled(std::path::Path::new(&app.save_path))
                        .unwrap_or_else(|reason| panic!("{reason}; UI status: {}", app.status))
                        .1,
                    expected
                );
                app.selected = CountryId("USA".into());
                let output = paint(&mut app);
                click(&mut app, &output, "Load");
                assert_eq!(app.selected, expected.player);
                assert_eq!(app.world.as_ref(), Some(&expected));
            }
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn new_campaign_modal_blocks_time_shortcuts_and_escape_cancels_without_losing_context() {
        for scale in [1.0, 1.4] {
            let content = Content::bundled().unwrap();
            let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
            let expected = world.hash();
            let ctx = egui::Context::default();
            let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.world = Some(world);
            app.tab = Tab::Treasury;
            let mut earlier = app.inspection();
            earlier.tab = Tab::Overview;
            app.navigation.observe(earlier);
            app.navigation.observe(app.inspection());
            assert!(app.navigation.can_back());
            app.open_new_campaign();
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0 / scale, 800.0 / scale),
                )),
                ..Default::default()
            };
            let output = ctx.run(input.clone(), |ctx| app.render(ctx));
            for label in ["Save and start", "Start without saving", "Cancel"] {
                let shape = output.shapes.iter().find(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == label)).expect("modal action is painted");
                if let egui::Shape::Text(text) = &shape.shape {
                    assert!(
                        shape
                            .clip_rect
                            .contains_rect(text.galley.rect.translate(text.pos.to_vec2())),
                        "{label} is clipped at scale {scale}"
                    );
                }
            }
            let key = |key| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![key(egui::Key::N)],
                    ..input.clone()
                },
                |ctx| app.render(ctx),
            );
            assert!(app.new_setup.is_some());
            assert!(app.pending.is_none());
            assert_eq!(app.speed, 0);
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: egui::Key::ArrowLeft,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::ALT,
                    }],
                    ..input.clone()
                },
                |ctx| app.render(ctx),
            );
            assert!(
                app.tab == Tab::Treasury && app.new_setup.is_some() && app.navigation.can_back()
            );
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![key(egui::Key::Escape)],
                    ..input
                },
                |ctx| app.render(ctx),
            );
            assert!(app.new_setup.is_none());
            assert!(app.tab == Tab::Treasury);
            assert_eq!(app.world.as_ref().unwrap().hash(), expected);
        }
    }
    #[test]
    fn bundled_loading_keeps_classic_rules_and_explicit_pack_selection_stays_pinned() {
        let current = Content::bundled().unwrap();
        let classic = Content::classic_bundled().unwrap();
        let mut old_world = World::new(&classic, CountryId("GBR".into()), 17).unwrap();
        old_world
            .advance(&classic, sim_core::DaySpan::new(31))
            .unwrap();
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
        app.start_advance(DaySpan::ZERO, false);
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
            |ctx| app.process_shortcuts(ctx, ctx.wants_keyboard_input()),
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
    fn page_scroll_positions_are_independent_and_return_to_the_same_context() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-income-bands-test.json"
        ))
        .unwrap();
        let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let hash = world.hash();
        let ctx = egui::Context::default();
        let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
        app.selected = world.player.clone();
        app.world = Some(world);
        app.tab = Tab::Treasury;
        let render = |app: &mut GameApp, time: f64, events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    focused: true,
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440.0, 940.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.render(ctx),
            )
        };
        let visible = |output: &egui::FullOutput, label: &str| {
            output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==label && s.clip_rect.contains(t.pos+t.galley.size()/2.0)))
        };
        render(&mut app, 0.0, vec![]);
        let initial = render(&mut app, 0.1, vec![]);
        assert!(visible(&initial, "Treasury & accounts"));
        render(
            &mut app,
            0.2,
            vec![
                egui::Event::PointerMoved(egui::pos2(1100.0, 450.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -600.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        render(&mut app, 0.4, vec![]);
        let scrolled = render(&mut app, 1.0, vec![]);
        assert!(!visible(&scrolled, "Treasury & accounts"));
        app.tab = Tab::Policies;
        let policy = render(&mut app, 2.0, vec![]);
        assert!(
            visible(&policy, "Policies & implementation"),
            "Treasury scrolling must not hide the new policy page's controls"
        );
        app.tab = Tab::Treasury;
        let returned = render(&mut app, 3.0, vec![]);
        assert!(
            !visible(&returned, "Treasury & accounts"),
            "Returning must preserve Treasury's own scroll position"
        );
        let foreign = CountryId("BOR".into());
        let heading = format!(
            "{} · published country report",
            app.content.country(&foreign).unwrap().name
        );
        app.selected = foreign;
        let country = render(&mut app, 4.0, vec![]);
        assert!(
            visible(&country, &heading),
            "A different country must open at its own report context"
        );
        app.selected = CountryId("AUR".into());
        let returned = render(&mut app, 5.0, vec![]);
        assert!(
            !visible(&returned, "Treasury & accounts"),
            "Returning from another country must preserve the original report position"
        );
        assert_eq!(app.world.unwrap().hash(), hash);
    }
    #[test]
    fn normal_foreign_institutions_do_not_expose_live_executive_profiles() {
        let content = Content::executive_identities_bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            app.world = Some(world.clone());
            app.tab = Tab::Politics;
            assert!(!app.developer_mode);
            for (country, own) in [("GBR", true), ("CHN", false)] {
                app.selected = CountryId(country.into());
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440.0 / scale, 940.0 / scale),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                );
                let labels = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                assert_eq!(labels.contains("Executive authority"), own, "{labels}");
                assert_eq!(
                    labels.contains("Executive history (1 retained mandates)"),
                    own,
                    "{labels}"
                );
                assert_eq!(app.world.as_ref().unwrap(), &world);
            }
        }
    }

    #[test]
    fn modeled_institution_views_are_owner_only_at_two_scales() {
        let content = Content::governing_institutions_bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            app.world = Some(world.clone());
            app.tab = Tab::Politics;
            assert!(!app.developer_mode);
            for (country, own) in [("GBR", true), ("CHN", false)] {
                app.selected = CountryId(country.into());
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440.0 / scale, 940.0 / scale),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                );
                let labels = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                assert_eq!(labels.contains("Governing institutions"), own, "{labels}");
                assert_eq!(
                    labels.contains("Modeled national legislature"),
                    own,
                    "{labels}"
                );
                assert_eq!(app.world.as_ref().unwrap(), &world);
            }
        }
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
            (
                Content::from_json(include_str!(
                    "../../../data/synthetic-constitution-test.json"
                ))
                .unwrap(),
                "AUR",
                "BOR",
            ),
            (
                Content::from_json(include_str!(
                    "../../../data/synthetic-undertakings-test.json"
                ))
                .unwrap(),
                "AUR",
                "BOR",
            ),
            (
                Content::from_json(include_str!(
                    "../../../data/synthetic-fiscal-plans-test.json"
                ))
                .unwrap(),
                "AUR",
                "BOR",
            ),
            (
                Content::from_json(include_str!(
                    "../../../data/synthetic-income-bands-test.json"
                ))
                .unwrap(),
                "AUR",
                "BOR",
            ),
        ] {
            let mut world = World::new(&content, CountryId(own.into()), 17).unwrap();
            world
                .advance(&content, sim_core::DaySpan::new(360))
                .unwrap();
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
        wanted.advance(&content, sim_core::DaySpan::new(2)).unwrap();
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
    #[test]
    fn reference_records_search_confirmation_clear_and_back_preserve_world_and_input_at_two_scales()
    {
        let content = Content::bookmark_identities_bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(61)).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.world = Some(world.clone());
            app.selected = world.player.clone();
            app.tab = Tab::Guide;
            app.help.reference = true;
            app.map.camera = map::Camera {
                longitude: 11.6,
                latitude: 48.2,
                zoom: 8.,
            };
            let camera = app.map.camera;
            let render = |app: &mut GameApp, events: Vec<egui::Event>| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440. / scale, 940. / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                )
            };
            let text = |output: &egui::FullOutput| {
                output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let click = |app: &mut GameApp, label: &str| {
                let output = render(app, vec![]);
                let point = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => {
                            let point = text.pos + text.galley.size() / 2.;
                            shape.clip_rect.contains(point).then_some(point)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        panic!("Visible {label} missing at {scale}: {}", text(&output))
                    });
                for pressed in [true, false] {
                    render(
                        app,
                        vec![
                            egui::Event::PointerMoved(point),
                            egui::Event::PointerButton {
                                pos: point,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                }
            };
            render(&mut app, vec![]);
            render(&mut app, vec![]);
            click(
                &mut app,
                "Search rules, names, parties or stable references",
            );
            let key = |key, pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let confirm = |value: &str| {
                vec![
                    key(egui::Key::N, true),
                    egui::Event::Text(value.into()),
                    key(egui::Key::N, false),
                    key(egui::Key::Enter, true),
                    key(egui::Key::Enter, false),
                ]
            };
            let output = render(&mut app, confirm("Francois Hollande"));
            assert!(
                app.pending.is_none(),
                "Reference input must not start time advancement"
            );
            assert_eq!(app.speed, 0);
            assert!(ctx.wants_keyboard_input());
            assert!(text(&output).contains("François Hollande"));
            assert!(text(&output).contains("3 matching entries"));
            click(&mut app, "Open relevant screen");
            assert_eq!(app.selected.0, "FRA");
            assert!(app.tab == Tab::Politics);
            click(&mut app, "Back");
            assert_eq!(app.selected.0, "GBR");
            assert!(app.tab == Tab::Guide && app.help.reference);
            let output = render(&mut app, vec![]);
            assert!(text(&output).contains("3 matching entries"));
            assert_eq!(app.map.camera, camera);
            click(&mut app, "Clear search");
            assert!(ctx.wants_keyboard_input());
            render(&mut app, confirm("N"));
            assert!(app.pending.is_none());
            assert_eq!(app.speed, 0);
            assert_eq!(app.selected, world.player);
            assert_eq!(
                persistence::encode(app.world.as_ref().unwrap(), &content).unwrap(),
                before
            );
        }
    }
    #[test]
    fn household_overflow_scrollbar_reaches_last_columns_and_returns_without_orders_at_two_scales()
    {
        let mut content =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        content.countries[0].population = Population(1_000_003);
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(61)).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.world = Some(world.clone());
            app.selected = world.player.clone();
            app.tab = Tab::Society;
            let render = |app: &mut GameApp, time, events: Vec<egui::Event>| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        time: Some(time),
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440. / scale, 940. / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                )
            };
            let visible = |output: &egui::FullOutput, label: &str| {
                output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()==label&&shape.clip_rect.contains_rect(text.galley.rect.translate(text.pos.to_vec2()))))
            };
            render(&mut app, 0., vec![]);
            render(&mut app, 0.25, vec![]);
            let initial = render(&mut app, 0.5, vec![]);
            assert!(visible(&initial, "Population"));
            if scale == 1.4 {
                assert!(!visible(&initial, "Arrears m"));
                let last_row = initial
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text)
                            if text.galley.text() == "Asset-owning households" =>
                        {
                            Some(text.pos.y + text.galley.size().y)
                        }
                        _ => None,
                    })
                    .unwrap();
                let notes = initial
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text)
                            if text.galley.text().starts_with("Income/basket compares") =>
                        {
                            Some(text.pos.y)
                        }
                        _ => None,
                    })
                    .unwrap();
                let track = initial
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if rect.rect.top() >= last_row
                                && rect.rect.bottom() <= notes
                                && rect.rect.height() >= 6.
                                && rect.rect.height() <= 18.
                                && rect.rect.width() > 200. =>
                        {
                            Some(rect.rect)
                        }
                        _ => None,
                    })
                    .max_by(|a, b| a.width().total_cmp(&b.width()))
                    .expect("Overflow has a visible mouse scrollbar even without hover");
                let drag = |app: &mut GameApp, start, end, time| {
                    render(
                        app,
                        time,
                        vec![
                            egui::Event::PointerMoved(start),
                            egui::Event::PointerButton {
                                pos: start,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                    render(app, time + 0.1, vec![egui::Event::PointerMoved(end)]);
                    render(
                        app,
                        time + 0.2,
                        vec![egui::Event::PointerButton {
                            pos: end,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        }],
                    );
                };
                let left = egui::pos2(track.left() + 12., track.center().y);
                let right = egui::pos2(track.right() - 12., track.center().y);
                drag(&mut app, left, right, 0.6);
                let end = render(&mut app, 1.0, vec![]);
                for label in ["Food %", "Approval %", "Arrears m"] {
                    assert!(visible(&end, label), "{label} stays reachable at {scale}");
                }
                drag(&mut app, right, left, 1.1);
            } else {
                assert!(visible(&initial, "Arrears m"));
            }
            let returned = render(&mut app, 1.5, vec![]);
            assert!(visible(&returned, "Population"));
            assert_eq!(app.selected, world.player);
            assert!(app.pending.is_none());
            assert_eq!(app.speed, 0);
            assert_eq!(
                persistence::encode(app.world.as_ref().unwrap(), &content).unwrap(),
                before
            );
        }
    }
    #[test]
    fn inspection_history_buttons_keys_branches_and_load_preserve_campaign_at_two_scales() {
        let content = Content::bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        let dir = std::env::temp_dir().join(format!("geosim-navigation-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("unchanged.json");
        std::fs::write(&path, &before).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            ctx.set_zoom_factor(scale);
            app.world = Some(world.clone());
            app.selected = world.player.clone();
            app.map.camera = map::Camera {
                longitude: 20.,
                latitude: 10.,
                zoom: 2.0,
            };
            let first_map = app.map.view();
            let render = |app: &mut GameApp, events: Vec<egui::Event>| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440. / scale, 940. / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                )
            };
            let click = |app: &mut GameApp, label: &str| {
                let output = render(app, vec![]);
                let point = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => {
                            let point = text.pos + text.galley.size() / 2.;
                            shape.clip_rect.contains(point).then_some(point)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("visible {label} missing at {scale}"));
                for pressed in [true, false] {
                    render(
                        app,
                        vec![
                            egui::Event::PointerMoved(point),
                            egui::Event::PointerButton {
                                pos: point,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                }
            };
            render(&mut app, vec![]);
            render(&mut app, vec![]);
            assert!(!app.navigation.can_back());
            click(&mut app, "Find division");
            let key = |key, modifiers| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            };
            render(
                &mut app,
                vec![
                    egui::Event::Text("Bavaria".into()),
                    key(egui::Key::Enter, egui::Modifiers::NONE),
                ],
            );
            render(&mut app, vec![]);
            assert_eq!(app.selected.0, "DEU");
            let division_map = app.map.view();
            click(&mut app, "02  Policies & delivery");
            let save_path = app.save_path.clone();
            click(&mut app, &save_path);
            render(
                &mut app,
                vec![key(egui::Key::ArrowLeft, egui::Modifiers::ALT)],
            );
            assert!(
                app.tab == Tab::Policies,
                "History shortcuts must leave focused text editors alone"
            );
            assert_eq!(app.selected.0, "DEU");
            click(&mut app, "Return to my country");
            assert_eq!(app.selected.0, "GBR");
            click(&mut app, "Back");
            assert_eq!(app.selected.0, "DEU");
            assert!(app.tab == Tab::Policies);
            click(&mut app, "Back");
            assert_eq!(app.selected.0, "DEU");
            assert!(app.tab == Tab::Overview);
            assert!(app.map.view() == division_map);
            click(&mut app, "Back");
            assert_eq!(app.selected.0, "GBR");
            assert!(app.map.view() == first_map);
            click(&mut app, "Forward");
            assert_eq!(app.selected.0, "DEU");
            assert!(app.map.view() == division_map);
            render(
                &mut app,
                vec![key(egui::Key::ArrowLeft, egui::Modifiers::ALT)],
            );
            assert_eq!(app.selected.0, "GBR");
            assert!(app.navigation.can_forward());
            click(&mut app, "04  Treasury & accounts");
            assert!(!app.navigation.can_forward());
            assert_eq!(
                persistence::encode(app.world.as_ref().unwrap(), &content).unwrap(),
                before
            );
            // Successful campaign replacement cannot expose a prior campaign's inspection route.
            app.save_path = path.to_string_lossy().into_owned();
            app.load();
            render(&mut app, vec![]);
            assert!(!app.navigation.can_back() && !app.navigation.can_forward());
            assert_eq!(
                persistence::encode(app.world.as_ref().unwrap(), &content).unwrap(),
                before
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn division_reference_confirmation_keeps_time_shortcuts_in_the_editor_at_two_scales() {
        let content = Content::bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut app =
                GameApp::from_context(&ctx, content.clone(), settings::Preferences::default());
            app.selected = world.player.clone();
            app.world = Some(world.clone());
            let render = |app: &mut GameApp, time: f64, events: Vec<egui::Event>| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        time: Some(time),
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440.0 / scale, 940.0 / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                )
            };
            let output = render(&mut app, 0.0, vec![]);
            let point = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape {
                        (text.galley.text() == "Find division")
                            .then_some(text.pos + text.galley.size() / 2.0)
                    } else {
                        None
                    }
                })
                .unwrap();
            for (time, pressed) in [(0.1, true), (0.2, false)] {
                render(
                    &mut app,
                    time,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            assert!(ctx.wants_keyboard_input());
            let key = |key, pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            render(
                &mut app,
                0.3,
                vec![
                    key(egui::Key::N, true),
                    egui::Event::Text("Bavaria".into()),
                    key(egui::Key::N, false),
                    key(egui::Key::Enter, true),
                    key(egui::Key::Enter, false),
                ],
            );
            assert!(
                app.pending.is_none(),
                "Reference confirmation must not advance the campaign"
            );
            assert_eq!(app.speed, 0);
            assert_eq!(app.selected, CountryId("DEU".into()));
            assert_eq!(app.world.as_ref().unwrap().player, world.player);
            assert_eq!(
                persistence::encode(app.world.as_ref().unwrap(), &content).unwrap(),
                before
            );
        }
    }
    #[test]
    fn physical_reference_confirmation_and_clear_cannot_trigger_campaign_shortcuts() {
        let content = Content::bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let before = world.hash();
        let ctx = egui::Context::default();
        let mut app = GameApp::from_context(&ctx, content, settings::Preferences::default());
        app.selected = world.player.clone();
        app.world = Some(world);
        let render = |app: &mut GameApp, time: f64, events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    focused: true,
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440.0, 940.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.render(ctx),
            )
        };
        let output = render(&mut app, 0.0, vec![]);
        let p = output
            .shapes
            .iter()
            .find_map(|s| {
                if let egui::Shape::Text(t) = &s.shape {
                    (t.galley.text() == "Find lake / river")
                        .then_some(t.pos + t.galley.size() / 2.0)
                } else {
                    None
                }
            })
            .unwrap();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            render(
                &mut app,
                time,
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert!(ctx.wants_keyboard_input());
        let key = |key, pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        render(
            &mut app,
            0.3,
            vec![
                key(egui::Key::N, true),
                egui::Event::Text("amazon".into()),
                key(egui::Key::N, false),
                key(egui::Key::Enter, true),
                key(egui::Key::Enter, false),
            ],
        );
        assert!(
            app.pending.is_none(),
            "confirming a text lookup must not request the next campaign event"
        );
        assert_eq!(app.speed, 0);
        assert_eq!(app.selected, CountryId("GBR".into()));
        assert_eq!(app.world.as_ref().unwrap().hash(), before);
        // Clearing/filtering is an editor action, not an instruction to dismiss the popup.
        let focused = app.map.camera;
        let output = render(&mut app, 0.4, vec![]);
        let point = |output: &egui::FullOutput, label: &str| {
            output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        (t.galley.text() == label).then_some(t.pos + t.galley.size() / 2.0)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| panic!("missing {label}"))
        };
        let p = point(&output, "Find lake / river");
        for (time, pressed) in [(0.5, true), (0.6, false)] {
            render(
                &mut app,
                time,
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        let output = render(&mut app, 0.7, vec![egui::Event::Text("river peace".into())]);
        let p = point(&output, "Clear");
        for (time, pressed) in [(0.8, true), (0.9, false)] {
            render(
                &mut app,
                time,
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        render(&mut app, 1.0, vec![]);
        let output = render(&mut app, 1.1, vec![]);
        point(&output, "Matches: 39");
        assert!(ctx.wants_keyboard_input());
        let output = render(
            &mut app,
            1.2,
            vec![
                key(egui::Key::N, true),
                key(egui::Key::Space, true),
                egui::Event::Text("not a listed feature".into()),
                key(egui::Key::N, false),
                key(egui::Key::Space, false),
                key(egui::Key::Enter, true),
                key(egui::Key::Enter, false),
            ],
        );
        point(&output, "No matching physical reference features.");
        assert!(app.pending.is_none());
        assert_eq!(app.speed, 0);
        assert_eq!(app.world.as_ref().unwrap().hash(), before);
        assert_eq!(app.map.camera, focused);
        assert_eq!(app.selected, CountryId("GBR".into()));
        let target = crate::hydrology::Hydrology::bundled()
            .matches("amazon")
            .remove(0);
        assert_eq!(app.map.camera.longitude, target.bounds.center().x);
        assert_eq!(app.map.camera.latitude, target.bounds.center().y);
    }
}
