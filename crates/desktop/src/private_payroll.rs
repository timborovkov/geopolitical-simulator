use eframe::egui::{self, Color32};
use sim_core::*;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    let Some(definition) = &content.private_payroll else {
        return;
    };
    egui::CollapsingHeader::new("Private employment & payroll").default_open(true).show(ui, |ui| {
        let state = match world.private_payroll_report(content, Viewer::Government(world.player.clone()), country) {
            Ok(Some(s)) => s,
            Ok(None) => {ui.label("Current employment contracts and wage claims are private to this government."); return;},
            Err(error) => {ui.colored_label(Color32::LIGHT_YELLOW,error); return;},
        };
        let unpaid = match state.unpaid() {Ok(m)=>m,Err(error)=>{ui.label(error);return;}};
        ui.strong(format!("Unpaid private wage claims {}m",unpaid.0));
        ui.label("Wage claims are household receivables and business liabilities. They cannot fund purchases until paid. Private contracts and funded public positions partition working-age residents.");
        for contract in &state.contracts {
            let name=content.country(country).and_then(|c| c.households.iter().find(|h|h.id==contract.group)).map_or(contract.group.0.as_str(),|h|h.name.as_str());
            ui.label(format!("{name}: {} private positions · accrued {}m · paid {}m · unpaid {}m",contract.jobs.0,contract.assessed.0,contract.paid.0,contract.unpaid.0));
        }
        if let Some(day)=&state.last {
            ui.separator();
            ui.strong(format!("{} payroll",date(day.day)));
            for r in &day.groups {
                ui.label(format!("{}: opening claims {}m + today's wages {}m − payment {}m = closing claims {}m",r.group.0,r.opening_unpaid.0,r.due.0,r.paid.0,r.closing_unpaid.0));
                ui.small(format!("Paid toward older claims {}m; current wages paid {}m.",r.arrears_paid.0,r.current_paid.0));
            }
            ui.label(format!("Other private factor income paid {}m. This is separate from wages and owner distributions.",day.nonwage_paid.0));
        } else {ui.label("First payroll is assessed after the first completed day.");}
        ui.small("Available cash is apportioned across eligible claims; older wages are paid first within each cohort. Unpaid wages reduce business earnings. Paying old claims creates no second production funding or inventory cost. Dividends and equipment investment protect unpaid wage claims alongside public contract advances.");
        ui.small("Private positions are authored contracts bounded by opening positions and current working-age residents. Public positions remain a service-funding estimate. Recruitment, dismissals, individual contracts, wage bargaining and formal insolvency remain unfinished.");
        ui.small(format!("Assumptions: {} · {}",definition.source,definition.source_version));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payroll_and_foreign_privacy_render_at_both_scales_without_mutation() {
        let c = Content::private_payroll_bundled().unwrap();
        let mut w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
        w.advance(&c, DaySpan::new(1)).unwrap();
        let before = persistence::encode(&w, &c).unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            for id in [&w.player, &CountryId("USA".into())] {
                let mut text = String::new();
                for _ in 0..3 {
                    let o = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1280., 1600.),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| show(ui, &c, &w, id));
                        },
                    );
                    text = o
                        .shapes
                        .iter()
                        .filter_map(|s| match &s.shape {
                            egui::Shape::Text(t) => Some(t.galley.text()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                }
                if id == &w.player {
                    for label in [
                        "Unpaid private wage claims",
                        "private positions",
                        "today's wages",
                        "Other private factor income",
                    ] {
                        assert!(text.contains(label), "{label}: {text}");
                    }
                } else {
                    assert!(text.contains("private to this government"));
                    assert!(!text.contains("Unpaid private wage claims"));
                    assert!(!text.contains("today's wages"));
                }
            }
        }
        assert_eq!(persistence::encode(&w, &c).unwrap(), before);
    }
}
