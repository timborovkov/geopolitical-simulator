use eframe::egui;
use sim_core::*;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World) {
    match world.governing_institutions_view(
        content,
        Viewer::Government(world.player.clone()),
        &world.player,
    ) {
        Ok(Some(view)) => {
            egui::CollapsingHeader::new("Governing institutions").default_open(true).show(ui, |ui| {
                ui.strong(&view.primary.name);
                ui.label(format!("Government #{} · {:?} · {}", view.context.government, view.context.route, view.primary.id.0));
                ui.label(format!("Current modeled backing {} / required {}", view.backing, view.threshold));
                if let Some(council) = view.council {
                    ui.label(format!("{} · backing {} / base threshold {}", council.name, view.council_backing, view.council_threshold));
                    ui.small(council.id.0);
                }
                ui.small("Backing is a support score. Special procedures may require different consent; large royal spending increases require council backing of 65.");
                ui.small(format!("Authored gameplay institutions · {} / {}. Historical membership and votes are unspecified.", view.primary.source, view.primary.source_version));
            });
        }
        Ok(None) => {}
        Err(reason) => {
            ui.colored_label(egui::Color32::LIGHT_YELLOW, reason);
        }
    }
}
