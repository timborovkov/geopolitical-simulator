use eframe::egui;
use sim_core::*;

/// Keep the expanded roster's manual files separate from the preserved rules-8 baseline.
pub fn default_save_path(content: &Content) -> String {
    let suffix = if content.id == "initial-2000-observed-v8" {
        "-observed"
    } else {
        ""
    };
    format!(
        "saves/campaign{suffix}-v{}.json",
        sim_core::persistence::snapshot_schema_for_rules(content.rules)
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Pack {
    Current,
    EventReferences,
    Reserve,
    Party,
    Technology,
    Interest,
    Cash,
    DatedDebt,
    NationalFiscal,
    Income,
    Fiscal,
    Undertaking,
    Territorial,
    Observed,
    Electoral,
    Cabinet,
    Commitment,
    Constitution,
    Finance,
    Market,
    Classic,
    Pinned,
}
pub enum Choice {
    Waiting,
    Cancel,
    Start { save_current: bool },
}
pub struct Setup {
    pub content: Content,
    pub player: CountryId,
    pub seed: String,
    pub scenario: Scenario,
    pub save_path: String,
    pub error: Option<String>,
    pack: Pack,
    bundled_catalog: bool,
}
impl Setup {
    pub fn new(
        current: &Content,
        player: &CountryId,
        seed: u64,
        bundled_catalog: bool,
    ) -> Result<Self, String> {
        let content = if bundled_catalog {
            Content::bundled()?
        } else {
            current.clone()
        };
        let selected = if content.country(player).is_some() {
            player.clone()
        } else {
            content.countries[0].id.clone()
        };
        let save_path = default_save_path(&content);
        Ok(Self {
            content,
            player: selected,
            seed: seed.to_string(),
            scenario: Scenario::Sandbox,
            save_path,
            error: None,
            pack: if bundled_catalog {
                Pack::Current
            } else {
                Pack::Pinned
            },
            bundled_catalog,
        })
    }
    pub fn prepare(&self) -> Result<World, String> {
        if self.save_path.trim().is_empty() {
            return Err("Choose a save path for the new campaign.".into());
        }
        let seed = self
            .seed
            .trim()
            .parse::<u64>()
            .map_err(|_| "Seed must be an unsigned 64-bit integer.")?;
        World::new_scenario(&self.content, self.player.clone(), seed, self.scenario)
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        active: bool,
        can_start: bool,
        current_save_path: &str,
    ) -> Choice {
        let mut choice = Choice::Waiting;
        let width = (ctx.content_rect().width() - 60.0).clamp(280.0, 600.0);
        let height = (ctx.content_rect().height() - 100.0).clamp(280.0, 650.0);
        let id = egui::Id::new("new_campaign_setup");
        let response = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).default_size(egui::vec2(width + 16.0, height + 16.0)))
            .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(width, height));
            ui.set_max_width(width);
            ui.heading("New campaign");
            egui::ScrollArea::vertical().max_height(height - 80.0).auto_shrink([false, false]).show(ui, |ui| {
                if active { ui.label("Starting replaces the active campaign. Save and start keeps a permanent archive of its current state. Starting without saving discards unsaved progress."); }
                if active { ui.label(format!("Save active campaign to: {current_save_path}. A separate archived snapshot will also be retained.")); }
                if self.bundled_catalog {
                    let previous = self.pack;
                    let pack_label = match self.pack { Pack::Current => "Default · linked programme outcomes", Pack::EventReferences => "Earlier · stable chronicle references", Pack::Reserve => "Earlier · national commodity reserves", Pack::Party => "Earlier · party policy preferences", Pack::Technology => "Earlier · technology prerequisites", Pack::Interest => "Earlier · carried debt interest", Pack::Cash => "Earlier · temporary cash retention", Pack::DatedDebt => "Earlier · dated loan principal", Pack::NationalFiscal => "Earlier · national fiscal reviews", Pack::Income => "Earlier · progressive private-income bands", Pack::Fiscal => "Earlier · annual and supplementary budgets", Pack::Undertaking => "Earlier · delivery undertakings", Pack::Territorial => "Earlier · territorial government", Pack::Observed => "Earlier · expanded observations", Pack::Electoral => "Earlier · electoral mandates", Pack::Cabinet => "Earlier · ministries and policies", Pack::Commitment => "Earlier · policy commitments", Pack::Constitution => "Earlier · political transitions", Pack::Finance => "Earlier · borrowing and prices", Pack::Market => "Earlier · prices only", _ => "Classic · fixed prices" };
                    egui::ComboBox::from_label("World pack").height(320.0).selected_text(pack_label).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.pack, Pack::Current, "Default · linked programme outcomes");
                        ui.selectable_value(&mut self.pack, Pack::EventReferences, "Earlier · stable chronicle references");
                        ui.selectable_value(&mut self.pack, Pack::Reserve, "Earlier · national commodity reserves");
                        ui.selectable_value(&mut self.pack, Pack::Party, "Earlier · party policy preferences");
                        ui.selectable_value(&mut self.pack, Pack::Technology, "Earlier · technology prerequisites");
                        ui.selectable_value(&mut self.pack, Pack::Interest, "Earlier · carried debt interest");
                        ui.selectable_value(&mut self.pack, Pack::Cash, "Earlier · temporary cash retention");
                        ui.selectable_value(&mut self.pack, Pack::DatedDebt, "Earlier · dated loan principal");
                        ui.selectable_value(&mut self.pack, Pack::NationalFiscal, "Earlier · national fiscal reviews");
                        ui.selectable_value(&mut self.pack, Pack::Income, "Earlier · progressive private-income bands");
                        ui.selectable_value(&mut self.pack, Pack::Fiscal, "Earlier · annual and supplementary budgets");
                        ui.selectable_value(&mut self.pack, Pack::Undertaking, "Earlier · delivery undertakings");
                        ui.selectable_value(&mut self.pack, Pack::Territorial, "Earlier · territorial government");
                        ui.selectable_value(&mut self.pack, Pack::Observed, "Earlier · expanded observations");
                        ui.selectable_value(&mut self.pack, Pack::Electoral, "Earlier · electoral mandates");
                        ui.selectable_value(&mut self.pack, Pack::Cabinet, "Earlier · ministries and policies");
                        ui.selectable_value(&mut self.pack, Pack::Commitment, "Earlier · policy commitments");
                        ui.selectable_value(&mut self.pack, Pack::Constitution, "Earlier · political transitions");
                        ui.selectable_value(&mut self.pack, Pack::Finance, "Earlier · borrowing and prices");
                        ui.selectable_value(&mut self.pack, Pack::Market, "Earlier · prices only");
                        ui.selectable_value(&mut self.pack, Pack::Classic, "Classic · fixed prices");
                    });
                    if self.pack != previous {
                        let old_default = default_save_path(&self.content);
                        match match self.pack { Pack::EventReferences => Content::event_reference_bundled(), Pack::Reserve => Content::reserve_bundled(), Pack::Party => Content::party_bundled(), Pack::Technology => Content::technology_bundled(), Pack::Interest => Content::interest_bundled(), Pack::Cash => Content::cash_bundled(), Pack::DatedDebt => Content::debt_bundled(), Pack::NationalFiscal => Content::national_fiscal_bundled(), Pack::Income => Content::income_bundled(), Pack::Fiscal => Content::fiscal_bundled(), Pack::Undertaking => Content::undertaking_bundled(), Pack::Territorial => Content::territorial_bundled(), Pack::Observed => Content::observed_bundled(), Pack::Electoral => Content::electoral_bundled(), Pack::Cabinet => Content::cabinet_bundled(), Pack::Classic => Content::classic_bundled(), Pack::Commitment => Content::commitment_bundled(), Pack::Constitution => Content::constitution_bundled(), Pack::Finance => Content::finance_bundled(), Pack::Market => Content::market_bundled(), _ => Content::bundled() } {
                            Ok(content) => {
                                if content.country(&self.player).is_none() { self.player = content.countries[0].id.clone(); }
                                if self.save_path == old_default { self.save_path = default_save_path(&content); }
                                self.content = content;
                                self.error = None;
                            }
                            Err(error) => { self.pack = previous; self.error = Some(error); }
                        }
                    }
                } else { ui.label(format!("Pinned world pack: {}", self.content.name)); }
                ui.label(format!("{} · bookmark {}", self.content.name, self.content.start));
                egui::ComboBox::from_label("Country").selected_text(&self.content.country(&self.player).expect("selected country").name).show_ui(ui, |ui| {
                    for country in &self.content.countries { ui.selectable_value(&mut self.player, country.id.clone(), format!("{} · {}", country.id.0, country.name)); }
                });
                let country = self.content.country(&self.player).expect("selected country");
                ui.label(format!("{:?} · {}", country.regime, country.government.label()));
                ui.label(format!("Bookmark baseline: {:.1}m people · GDP {}m/year · domestic energy/food {}%/{}% · administration {}%", country.population as f64 / 1_000_000.0, country.gdp, country.energy, country.food, country.capacity));
                ui.horizontal(|ui| { ui.label("World seed"); ui.add(egui::TextEdit::singleline(&mut self.seed).desired_width(180.0)); });
                egui::ComboBox::from_label("Scenario").selected_text(self.scenario.id()).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.scenario, Scenario::Sandbox, "Sandbox");
                    ui.selectable_value(&mut self.scenario, Scenario::FiscalSupplyCrisis, "Fiscal and supply crisis (authored)");
                });
                if self.scenario == Scenario::FiscalSupplyCrisis { ui.label("Crisis starts with reduced supply/administration, low tax, high spending and weak approval. GDP and population observations are retained."); }
                ui.label(format!("Five-year review: {}. Country play continues afterward.", date(self.content.review_day().expect("validated content"))));
                ui.label("New campaign save path");
                ui.text_edit_singleline(&mut self.save_path);
                if let Some(error) = &self.error { ui.colored_label(egui::Color32::LIGHT_YELLOW, error); }
                if !can_start { ui.label("Stopping the current advance at a day boundary. Starting is available once it finishes."); }
            });
            ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if active && ui.add_enabled(can_start, egui::Button::new("Save and start")).clicked() { choice = Choice::Start { save_current: true }; }
                    if ui.add_enabled(can_start, egui::Button::new(if active { "Start without saving" } else { "Start campaign" })).clicked() { choice = Choice::Start { save_current: false }; }
                    if ui.button("Cancel").clicked() { choice = Choice::Cancel; }
                });
        });
        if response.should_close() {
            Choice::Cancel
        } else {
            choice
        }
    }
}
