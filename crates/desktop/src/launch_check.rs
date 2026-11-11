//! Explicit developer launch proof from the native renderer's screenshot readback.
use crate::GameApp;
use eframe::egui;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Outcome {
    pub report: Option<Report>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct Report {
    schema: u32,
    platform: &'static str,
    renderer: &'static str,
    backend: String,
    adapter: String,
    frames: u32,
    width: usize,
    height: usize,
    nonuniform_pixels: bool,
    campaign_started: bool,
}

pub struct LaunchCheck {
    app: GameApp,
    outcome: Arc<Mutex<Outcome>>,
    backend: String,
    adapter: String,
    frames: u32,
    request: egui::UserData,
}

impl LaunchCheck {
    pub fn new(
        app: GameApp,
        cc: &eframe::CreationContext<'_>,
        outcome: Arc<Mutex<Outcome>>,
    ) -> Result<Self, std::io::Error> {
        let state = cc.wgpu_render_state.as_ref().ok_or_else(|| {
            std::io::Error::other("Native launch check requires the real wgpu renderer")
        })?;
        let info = state.adapter.get_info();
        Ok(Self {
            app,
            outcome,
            backend: format!("{:?}", info.backend),
            adapter: info.name,
            frames: 0,
            request: egui::UserData::new("native-launch-check"),
        })
    }
}

impl eframe::App for LaunchCheck {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.render(ctx);
        self.frames += 1;
        let capture = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot {
                    viewport_id,
                    user_data,
                    image,
                } if *viewport_id == egui::ViewportId::ROOT && *user_data == self.request => {
                    Some(image.clone())
                }
                _ => None,
            })
        });
        if let Some(image) = capture {
            let [width, height] = image.size;
            let nonuniform = image
                .pixels
                .first()
                .is_some_and(|first| image.pixels.iter().any(|pixel| pixel != first));
            let mut outcome = self.outcome.lock().expect("launch outcome");
            if width < 320 || height < 240 || !nonuniform || self.app.world.is_some() {
                outcome.error = Some(
                    "Native readback is empty, uniform, too small or changed the campaign".into(),
                );
            } else {
                outcome.report = Some(Report {
                    schema: 1,
                    platform: std::env::consts::OS,
                    renderer: "wgpu",
                    backend: self.backend.clone(),
                    adapter: self.adapter.clone(),
                    frames: self.frames,
                    width,
                    height,
                    nonuniform_pixels: nonuniform,
                    campaign_started: false,
                });
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.frames == 2 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(self.request.clone()));
        }
        ctx.request_repaint();
    }
}
