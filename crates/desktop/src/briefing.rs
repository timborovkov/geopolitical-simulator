//! Acknowledgement is client state; notices never issue simulation commands.
use eframe::egui;
use sim_core::{CountryId, World, date};
#[derive(PartialEq, Eq)]
struct NoticeKey {
    content: u64,
    seed: u64,
    player: CountryId,
    day: u32,
    events: Vec<String>,
}
#[derive(Default)]
pub struct Briefing {
    dismissed: Option<NoticeKey>,
}
impl Briefing {
    /// Returns true only when the player explicitly opens their chronicle.
    pub fn show(&mut self, ctx: &egui::Context, world: &World) -> bool {
        let events: Vec<_> = world
            .events
            .iter()
            .rev()
            .filter(|event| {
                event.important && event.country == world.player && event.day == world.day
            })
            .collect();
        if events.is_empty() {
            return false;
        }
        let key = NoticeKey {
            content: world.content_hash,
            seed: world.seed,
            player: world.player.clone(),
            day: world.day.units(),
            events: events.iter().map(|event| event.text.clone()).collect(),
        };
        if self.dismissed.as_ref() == Some(&key) {
            return false;
        }
        let mut open = false;
        let mut dismiss = false;
        egui::TopBottomPanel::top("government_event_brief").min_height(90.0).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Your government · {} · {}", world.player.0, date(world.day.units())));
                ui.label(format!("{} important event{} today", events.len(), if events.len() == 1 { "" } else { "s" }));
                open = ui.button("Open chronicle").clicked();
                dismiss = ui.button("Dismiss brief").on_hover_text("Hide this brief until a new important event. The chronicle keeps the original records.").clicked();
            });
            egui::ScrollArea::vertical().id_salt("government_event_brief_text").max_height(80.0).show(ui, |ui| {
                for event in events.iter().take(2) {
                    ui.label(&event.text);
                }
                if events.len() > 2 {
                    ui.label(format!("{} more important events today in your chronicle.", events.len() - 2));
                }
            });
        });
        if dismiss || open {
            self.dismissed = Some(key);
        }
        open
    }
}
