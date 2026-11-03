use eframe::egui::{self, Color32, Ui};
use sim_core::*;

pub struct Panel {
    target: Regime,
    contested: bool,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            target: Regime::Parliamentary,
            contested: false,
        }
    }
}
impl Panel {
    /// Institutions workspace calls this only for the controlled government.
    pub fn show(&mut self, ui: &mut Ui, content: &Content, world: &World) -> Option<Action> {
        let rules = content.constitutions.as_ref()?;
        if rules.contested.is_none() {
            self.contested = false;
        }
        let country = world.player_country();
        let state = country.constitution.as_ref()?;
        let mut action = None;
        ui.separator();
        ui.heading("Constitutional change");
        ui.label(format!("Current form {:?} · recorded in this campaign since {}. Country identity, accounts and existing debt continue across a change of government.", state.regime, date(state.since)));
        ui.small("These routes and templates are authored scenario rules. Council/elite backing is a simplified effective-power model, not a complete military, court or faction simulation.");
        if let Some(request) = &state.pending {
            ui.label(format!(
                "Pending #{} · {:?} to {:?} · {} · decision {} · sponsor cabinet #{}",
                request.id.0,
                request.from,
                request.target,
                if request.contested {
                    "CONTESTED"
                } else {
                    "lawful"
                },
                date(request.decision_day),
                request.sponsor_government
            ));
            ui.label(format!("Administration paid {}m; reserved {}m. Backing and cabinet sponsorship will be checked again. Withdrawal/refusal keeps paid costs and the cooldown.", request.upfront.0, request.reserved.0));
            if ui.button("Withdraw constitutional request").clicked() {
                action = Some(Action::CancelConstitution {
                    request: request.id,
                });
            }
        }
        ui.label(format!(
            "Next request permitted after {} · backing, funding and time limits also apply.",
            date(state.next_request_day)
        ));
        egui::ComboBox::from_label("Proposed governing form")
            .selected_text(format!("{:?}", self.target))
            .show_ui(ui, |ui| {
                for template in &rules.templates {
                    ui.selectable_value(
                        &mut self.target,
                        template.regime.clone(),
                        format!("{:?} · {}", template.regime, template.title),
                    );
                }
            });
        if rules.contested.is_some() {
            ui.checkbox(&mut self.contested, "Use contested effective-power route");
        }
        if let Ok(route) = rules.route(self.contested) {
            let legislature = !self.contested
                && world.governing_rules(content, &world.player).ok()?.route
                    == DecisionRoute::Legislature;
            ui.label(format!(
                "Required {} backing {} · council {} · approval {} · decision in {} days.",
                if legislature { "legislative" } else { "elite" },
                if legislature {
                    route.legislative_threshold
                } else {
                    route.elite_threshold
                },
                route.council_threshold,
                route.approval_threshold,
                route.days
            ));
            ui.label(format!("If enacted: approval -{}, legislature -{}, elite -{}, council -{} (floor zero). New competitive governments schedule an election in one year; other forms use their configured leadership reviews.", route.approval_cost, route.legislative_cost, route.elite_cost, route.council_cost));
        }
        if self.contested {
            ui.colored_label(Color32::LIGHT_YELLOW, "This explicitly bypasses the ordinary constitutional route. Effective backing, cash and the decision delay still constrain the attempt; political costs apply if enacted.");
        }
        if let Some(template) = rules.template(&self.target) {
            ui.label(format!(
                "Proposed ordinary decision rights: {}",
                template.government.label()
            ));
        }
        ui.label("Existing funded programs continue. Unapproved programs face the new government's consent rules. Pending borrowing is cancelled on constitutional succession; the successor seeks its own loan mandate.");
        match world.preview_constitution(content, &world.player, &self.target, self.contested) {
            Ok(preview) => {
                ui.label(format!("Administration {}m total · pay {}m now · reserve {}m · decision {}. Cancellation or refusal does not refund paid administration or remove the cooldown.", preview.cost.0, preview.upfront.0, preview.reserved.0, date(preview.decision_day)));
                if ui
                    .button(if self.contested {
                        "Initiate contested change with these costs"
                    } else {
                        "Propose lawful constitutional change"
                    })
                    .clicked()
                {
                    action = Some(Action::Reconstitute {
                        target: self.target.clone(),
                        contested: self.contested,
                    });
                }
            }
            Err(reason) => {
                ui.label(format!("Cannot propose: {reason}"));
            }
        }
        egui::CollapsingHeader::new("Enacted constitutional history").show(ui, |ui| {
            if state.history.is_empty() {
                ui.label("No governing-form changes in this campaign.");
            }
            for change in state.history.iter().rev() {
                ui.label(format!(
                    "{} · {:?} to {:?} · {} · cabinet #{}",
                    date(change.day),
                    change.from,
                    change.to,
                    if change.contested {
                        "CONTESTED"
                    } else {
                        "lawful"
                    },
                    change.government_number
                ));
            }
        });
        action
    }
}
