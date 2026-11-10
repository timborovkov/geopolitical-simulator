use eframe::egui;
use sim_core::*;

/// Keep the expanded roster's manual files separate from the preserved rules-8 baseline.
pub fn default_save_path(content: &Content) -> String {
    let suffix = if content.id == "initial-2000-bookmark-identities-v24" {
        "-bookmark-identities"
    } else if content.id == "initial-2000-observed-v8" {
        "-observed"
    } else {
        ""
    };
    format!(
        "saves/campaign{suffix}-v{}.json",
        sim_core::persistence::snapshot_schema_for_rules(content.rules)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_setup_preserves_governing_and_history_starters_with_separate_files_at_two_scales() {
        let choices = [
            (
                "Optional · private employment & payroll",
                Content::private_payroll_bundled as fn() -> Result<Content, String>,
                41,
                true,
            ),
            (
                "Optional · private equipment investment",
                Content::private_investment_bundled as fn() -> Result<Content, String>,
                40,
                true,
            ),
            (
                "Earlier · population & production",
                Content::production_bundled as fn() -> Result<Content, String>,
                32,
                true,
            ),
            (
                "Optional · business production costs",
                Content::business_costs_bundled as fn() -> Result<Content, String>,
                38,
                true,
            ),
            (
                "Optional · delivered household consumption",
                Content::consumption_bundled as fn() -> Result<Content, String>,
                37,
                true,
            ),
            (
                "Optional · household-owned businesses",
                Content::businesses_bundled as fn() -> Result<Content, String>,
                36,
                true,
            ),
            (
                "Optional · industrial imports & exports",
                Content::industrial_trade_bundled as fn() -> Result<Content, String>,
                35,
                true,
            ),
            (
                "Optional · public industry lifecycle",
                Content::industry_lifecycle_bundled as fn() -> Result<Content, String>,
                34,
                true,
            ),
            (
                "Earlier · integrated government",
                Content::governing_institutions_bundled as fn() -> Result<Content, String>,
                29,
                true,
            ),
            (
                "Earlier · history and opening references",
                Content::statistical_history_bundled,
                26,
                false,
            ),
        ];
        for (label, factory, schema, live_institutions) in choices {
            for scale in [1., 1.4] {
                let content = Content::recommended_bundled().unwrap();
                let ctx = egui::Context::default();
                ctx.set_zoom_factor(scale);
                let mut setup = Setup::new(&content, &CountryId("GBR".into()), 17, true).unwrap();
                assert_eq!(setup.content.rules, 38);
                assert_eq!(setup.save_path, "saves/campaign-v39.json");
                if scale == 1.4 {
                    setup.save_path = "saves/my-government.json".into();
                }
                let mut frame = |events| {
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
                        |ctx| {
                            assert!(matches!(
                                setup.show(ctx, true, true, "saves/unchanged.json"),
                                Choice::Waiting
                            ));
                        },
                    )
                };
                let locate = |output: &egui::FullOutput, label: &str| {
                    output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.text() == label => {
                                let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                                shape.clip_rect.contains_rect(rect).then_some(rect.center())
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| panic!("Visible choice missing: {label}"))
                };
                frame(vec![]);
                frame(vec![]);
                let point = locate(&frame(vec![]), "Other campaign systems");
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
                for _ in 0..20 {
                    frame(vec![]);
                }
                let point = locate(&frame(vec![]), "Recommended · integrated campaign");
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
                let mut output = frame(vec![]);
                let mut found = None;
                for _ in 0..15 {
                    found = output.shapes.iter().find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => {
                            let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                            shape.clip_rect.contains_rect(rect).then_some(rect.center())
                        }
                        _ => None,
                    });
                    if found.is_some() {
                        break;
                    }
                    output = frame(vec![
                        egui::Event::PointerMoved(point + egui::vec2(50., 65.)),
                        egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0., -70.),
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]);
                }
                let point =
                    found.unwrap_or_else(|| panic!("Visible archived choice missing: {label}"));
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
                frame(vec![]);
                assert_eq!(setup.content.hash(), factory().unwrap().hash());
                assert_eq!(
                    setup.save_path,
                    if scale == 1.4 {
                        "saves/my-government.json".into()
                    } else {
                        format!("saves/campaign-v{schema}.json")
                    }
                );
                let world = setup.prepare().unwrap();
                world.validate(&setup.content).unwrap();
                assert_eq!(persistence::snapshot_schema(&world), schema);
                assert!(world.player_country().statistical_archive.is_some());
                assert_eq!(
                    world.player_country().executive_identity.is_some(),
                    live_institutions
                );
                assert_eq!(
                    world.player_country().legislation.is_some(),
                    live_institutions
                );
                assert_eq!(world.production.is_some(), schema >= 32);
                assert_eq!(world.demographics.is_some(), schema >= 30);
                assert_eq!(
                    world.player_country().industrial_trade.is_some(),
                    schema >= 35
                );
                assert_eq!(world.player_country().business.is_some(), schema >= 36);
                assert_eq!(world.seed, 17);
            }
        }
    }
    #[test]
    fn optional_identity_pack_prepares_without_reusing_the_default_manual_save_path() {
        let current = Content::recommended_bundled().unwrap();
        let pack = Content::bookmark_identities_bundled().unwrap();
        let setup = Setup::new(&pack, &CountryId("GBR".into()), 17, false).unwrap();
        assert_ne!(setup.save_path, default_save_path(&current));
        let world = setup.prepare().unwrap();
        assert_eq!(world.content_hash, pack.hash());
        assert_eq!(world.player.0, "GBR");
        assert_eq!(world.player_country().executive, "Tony Blair");
        assert_eq!(persistence::snapshot_schema(&world), 25);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Pack {
    PrivatePayroll,
    PrivateInvestment,
    Current,
    Demographics,
    SocialMemberships,
    GoverningInstitutions,
    IndustryInvestment,
    IndustryLifecycle,
    IndustrialTrade,
    Businesses,
    Consumption,
    Production,
    BusinessCosts,
    StatisticalHistory,
    ExecutiveIdentities,
    Legislation,
    BookmarkIdentities,
    StableElections,
    ProgrammeEvents,
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
            Content::recommended_bundled()?
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
                    ui.collapsing("Other campaign systems", |ui| {
                    let previous = self.pack;
                    let pack_label = match self.pack { Pack::PrivatePayroll => "Optional · private employment & payroll", Pack::PrivateInvestment => "Optional · private equipment investment", Pack::Production => "Earlier · population & production", Pack::BusinessCosts => "Optional · business production costs", Pack::Consumption => "Optional · delivered household consumption", Pack::Businesses => "Optional · household-owned businesses", Pack::IndustrialTrade => "Optional · industrial imports & exports", Pack::IndustryLifecycle => "Optional · public industry lifecycle", Pack::IndustryInvestment => "Optional · public industry investment", Pack::GoverningInstitutions => "Earlier · integrated government", Pack::SocialMemberships => "Earlier · overlapping social groups", Pack::Demographics => "Earlier · authored population change", Pack::StatisticalHistory => "Earlier · history and opening references", Pack::ExecutiveIdentities => "Live executive identities", Pack::Current => "Recommended · integrated campaign", Pack::Legislation => "Authored statutory authority", Pack::BookmarkIdentities => "Optional · dated opening references", Pack::StableElections => "Earlier · stable political selection", Pack::ProgrammeEvents => "Earlier · linked programme outcomes", Pack::EventReferences => "Earlier · stable chronicle references", Pack::Reserve => "Earlier · national commodity reserves", Pack::Party => "Earlier · party policy preferences", Pack::Technology => "Earlier · technology prerequisites", Pack::Interest => "Earlier · carried debt interest", Pack::Cash => "Earlier · temporary cash retention", Pack::DatedDebt => "Earlier · dated loan principal", Pack::NationalFiscal => "Earlier · national fiscal reviews", Pack::Income => "Earlier · progressive private-income bands", Pack::Fiscal => "Earlier · annual and supplementary budgets", Pack::Undertaking => "Earlier · delivery undertakings", Pack::Territorial => "Earlier · territorial government", Pack::Observed => "Earlier · expanded observations", Pack::Electoral => "Earlier · electoral mandates", Pack::Cabinet => "Earlier · ministries and policies", Pack::Commitment => "Earlier · policy commitments", Pack::Constitution => "Earlier · political transitions", Pack::Finance => "Earlier · borrowing and prices", Pack::Market => "Earlier · prices only", _ => "Classic · fixed prices" };
                    egui::ComboBox::from_label("World pack").height(320.0).selected_text(pack_label).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.pack, Pack::Current, "Recommended · integrated campaign");
                        ui.selectable_value(&mut self.pack, Pack::PrivatePayroll, "Optional · private employment & payroll");
                        ui.selectable_value(&mut self.pack, Pack::PrivateInvestment, "Optional · private equipment investment");
                        ui.selectable_value(&mut self.pack, Pack::BusinessCosts, "Optional · business production costs");
                        ui.selectable_value(&mut self.pack, Pack::Consumption, "Optional · delivered household consumption");
                        ui.selectable_value(&mut self.pack, Pack::Businesses, "Optional · household-owned businesses");
                        ui.selectable_value(&mut self.pack, Pack::IndustrialTrade, "Optional · industrial imports & exports");
                        ui.selectable_value(&mut self.pack, Pack::IndustryLifecycle, "Optional · public industry lifecycle");
                        ui.selectable_value(&mut self.pack, Pack::IndustryInvestment, "Optional · public industry investment");
                        ui.selectable_value(&mut self.pack, Pack::Production, "Earlier · population & production");
                        ui.selectable_value(&mut self.pack, Pack::GoverningInstitutions, "Earlier · integrated government");
                        ui.selectable_value(&mut self.pack, Pack::Demographics, "Earlier · authored population change");
                        ui.selectable_value(&mut self.pack, Pack::SocialMemberships, "Earlier · overlapping social groups");
                        ui.selectable_value(&mut self.pack, Pack::StatisticalHistory, "Earlier · history and opening references");
                        ui.selectable_value(&mut self.pack, Pack::ExecutiveIdentities, "Live executive identities");
                        ui.selectable_value(&mut self.pack, Pack::Legislation, "Authored statutory authority");
                        ui.selectable_value(&mut self.pack, Pack::BookmarkIdentities, "Optional · dated opening references");
                        ui.selectable_value(&mut self.pack, Pack::StableElections, "Earlier · stable political selection");
                        ui.selectable_value(&mut self.pack, Pack::ProgrammeEvents, "Earlier · linked programme outcomes");
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
                        match match self.pack { Pack::PrivatePayroll => Content::private_payroll_bundled(), Pack::PrivateInvestment => Content::private_investment_bundled(), Pack::Production => Content::production_bundled(), Pack::BusinessCosts => Content::business_costs_bundled(), Pack::Consumption => Content::consumption_bundled(), Pack::Businesses => Content::businesses_bundled(), Pack::IndustrialTrade => Content::industrial_trade_bundled(), Pack::IndustryLifecycle => Content::industry_lifecycle_bundled(), Pack::IndustryInvestment => Content::industry_bundled(), Pack::GoverningInstitutions => Content::governing_institutions_bundled(), Pack::SocialMemberships => Content::social_memberships_bundled(), Pack::Demographics => Content::demographic_bundled(), Pack::StatisticalHistory => Content::statistical_history_bundled(), Pack::ExecutiveIdentities => Content::executive_identities_bundled(), Pack::Legislation => Content::legislation_bundled(), Pack::BookmarkIdentities => Content::bookmark_identities_bundled(), Pack::StableElections => Content::stable_elections_bundled(), Pack::ProgrammeEvents => Content::programme_events_bundled(), Pack::EventReferences => Content::event_reference_bundled(), Pack::Reserve => Content::reserve_bundled(), Pack::Party => Content::party_bundled(), Pack::Technology => Content::technology_bundled(), Pack::Interest => Content::interest_bundled(), Pack::Cash => Content::cash_bundled(), Pack::DatedDebt => Content::debt_bundled(), Pack::NationalFiscal => Content::national_fiscal_bundled(), Pack::Income => Content::income_bundled(), Pack::Fiscal => Content::fiscal_bundled(), Pack::Undertaking => Content::undertaking_bundled(), Pack::Territorial => Content::territorial_bundled(), Pack::Observed => Content::observed_bundled(), Pack::Electoral => Content::electoral_bundled(), Pack::Cabinet => Content::cabinet_bundled(), Pack::Classic => Content::classic_bundled(), Pack::Commitment => Content::commitment_bundled(), Pack::Constitution => Content::constitution_bundled(), Pack::Finance => Content::finance_bundled(), Pack::Market => Content::market_bundled(), _ => Content::recommended_bundled() } {
                            Ok(content) => {
                                if content.country(&self.player).is_none() { self.player = content.countries[0].id.clone(); }
                                if self.save_path == old_default { self.save_path = default_save_path(&content); }
                                self.content = content;
                                self.error = None;
                            }
                            Err(error) => { self.pack = previous; self.error = Some(error); }
                        }
                    }
                    });
                } else { ui.label(format!("Pinned world pack: {}", self.content.name)); }
                if self.pack == Pack::Current {
                    ui.strong("A New Century · integrated campaign");
                    ui.label("Govern institutions, build industry, meet household needs and manage public finances. Businesses earn income and retain cash for their commitments.");
                } else { ui.strong(&self.content.name); }
                ui.label(format!("{} playable countries · starts {}", self.content.countries.len(), self.content.start));
                ui.collapsing("Campaign systems & assumptions", |ui| {
                if self.content.production.is_some() { ui.label("Infrastructure sectors use paid worker-days, equipment and stored inputs. Funded construction needs actual assembly packages; service inputs and maintenance can constrain delivery. These auxiliary quantities are authored assumptions, separate from civilian food and energy."); }
                if self.content.social_memberships.is_some() { ui.label("Social groups can overlap while each resident stays in one population account. Household inspection compares any/every selected membership through authored joint profiles; these are not observed national memberships or extra votes, cash or powers."); }
                if self.content.demographics.is_some() { ui.label("Authored population change: residents and family units evolve; opening physical capacity stays fixed until funded construction. Statistical releases are dated estimates. These rates and migration corridors are scenario assumptions, not historical national vital statistics."); }
                if self.content.legislation.is_some() { ui.label("Higher service targets may need an enacted law. Laws, programme funding and delivery are separate decisions."); }
                    ui.label(format!("Loaded scenario: {}", self.content.name));
                    if self.content.private_payroll.is_some() { ui.label("Private employment contracts, wage claims, earnings and equipment are enabled. Only actual wages paid enter household cash; hiring, dismissals and formal insolvency remain unfinished."); } else if self.content.private_investment.is_some() { ui.label("Private businesses can purchase, install and depreciate equipment against observed import costs and protected cash. Formal insolvency remains unfinished."); } else if self.content.business_finance.is_some() { ui.label("Industry, delivered household consumption and earnings-based business payouts are enabled. Businesses protect unfinished client advances; private investment and formal insolvency remain unfinished."); }
                });
                egui::ComboBox::from_label("Country").selected_text(&self.content.country(&self.player).expect("selected country").name).show_ui(ui, |ui| {
                    for country in &self.content.countries { ui.selectable_value(&mut self.player, country.id.clone(), format!("{} · {}", country.id.0, country.name)); }
                });
                let country = self.content.country(&self.player).expect("selected country");
                ui.label(format!("{:?} · {}", country.regime, country.government.label()));
                ui.label(format!("Bookmark baseline: {:.1}m people · GDP {}m/year · domestic energy/food {}%/{}% · administration {}%", country.population.0 as f64 / 1_000_000.0, country.gdp.units(), country.energy, country.food, country.capacity));
                ui.horizontal(|ui| { ui.label("World seed"); ui.add(egui::TextEdit::singleline(&mut self.seed).desired_width(180.0)); });
                egui::ComboBox::from_label("Scenario").selected_text(self.scenario.id()).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.scenario, Scenario::Sandbox, "Sandbox");
                    ui.selectable_value(&mut self.scenario, Scenario::FiscalSupplyCrisis, "Fiscal and supply crisis (authored)");
                });
                if self.scenario == Scenario::FiscalSupplyCrisis { ui.label("Crisis starts with reduced supply/administration, low tax, high spending and weak approval. GDP and population observations are retained."); }
                ui.label("Five-year objective: energy and food at least 90%, approval at least 50%, no arrears or overdue loan principal, and debt no greater than opening GDP.");
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
