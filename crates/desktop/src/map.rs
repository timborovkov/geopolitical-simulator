use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use sim_core::{Content, CountryId, World};
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub longitude: f32,
    pub latitude: f32,
    pub zoom: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            longitude: 0.0,
            latitude: 0.0,
            zoom: 1.0,
        }
    }
}
impl Camera {
    fn scale(self, rect: Rect) -> f32 {
        (rect.width() / 360.0).min(rect.height() / 180.0).max(0.001) * self.zoom
    }
    fn project(self, rect: Rect, p: [f32; 2]) -> Pos2 {
        let scale = self.scale(rect);
        rect.center()
            + Vec2::new(
                (p[0] - self.longitude) * scale,
                -(p[1] - self.latitude) * scale,
            )
    }
    fn unproject(self, rect: Rect, p: Pos2) -> [f32; 2] {
        let relative = p - rect.center();
        let scale = self.scale(rect);
        [
            self.longitude + relative.x / scale,
            self.latitude - relative.y / scale,
        ]
    }
    fn zoom_at(&mut self, rect: Rect, cursor: Pos2, factor: f32) {
        let anchor = self.unproject(rect, cursor);
        self.zoom = (self.zoom * factor).clamp(1.0, 12.0);
        let relative = cursor - rect.center();
        let scale = self.scale(rect);
        self.longitude = anchor[0] - relative.x / scale;
        self.latitude = anchor[1] + relative.y / scale;
        self.clamp();
    }
    fn clamp(&mut self) {
        self.longitude = self.longitude.clamp(-180.0, 180.0);
        self.latitude = self.latitude.clamp(-90.0, 90.0);
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Layer {
    Countries,
    Approval,
    Needs,
    Debt,
}
fn nearest_marker(
    content: &Content,
    camera: Camera,
    rect: Rect,
    position: Pos2,
) -> Option<CountryId> {
    content
        .countries
        .iter()
        .filter_map(|c| {
            let marker = camera.project(rect, [c.longitude as f32, c.latitude as f32]);
            let distance = position.distance_sq(marker);
            (rect.contains(marker) && distance <= 13.0 * 13.0).then_some((distance, c.id.clone()))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        .map(|(_, id)| id)
}
pub struct WorldMap {
    land: Vec<(Vec<[f32; 2]>, Vec<usize>)>,
    pub camera: Camera,
    layer: Layer,
}
impl WorldMap {
    pub fn new() -> Self {
        let polygons: Vec<Vec<[f32; 2]>> =
            serde_json::from_str(include_str!("../../../data/land-110m.json"))
                .expect("bundled land geometry");
        let land = polygons
            .into_iter()
            .map(|points| {
                let flat: Vec<_> = points
                    .iter()
                    .flat_map(|p| [f64::from(p[0]), f64::from(p[1])])
                    .collect();
                let triangles = earcutr::earcut(&flat, &[], 2).expect("land triangulation");
                (points, triangles)
            })
            .collect();
        Self {
            land,
            camera: Camera::default(),
            layer: Layer::Countries,
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        world: Option<&World>,
        selected: &mut CountryId,
        height: f32,
    ) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Fit world").clicked() {
                self.camera = Camera::default();
            }
            if ui.button(format!("Focus {}", selected.0)).clicked()
                && let Some(c) = content.country(selected)
            {
                self.camera = Camera {
                    longitude: c.longitude as f32,
                    latitude: c.latitude as f32,
                    zoom: 4.0,
                };
            }
            if ui.button("Zoom −").clicked() {
                self.camera.zoom = (self.camera.zoom / 1.4).max(1.0);
            }
            if ui.button("Zoom +").clicked() {
                self.camera.zoom = (self.camera.zoom * 1.4).min(12.0);
            }
            egui::ComboBox::from_label("Overlay")
                .selected_text(match self.layer {
                    Layer::Countries => "Countries",
                    Layer::Approval => "Approval",
                    Layer::Needs => "Needs met",
                    Layer::Debt => "Debt / opening GDP",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.layer, Layer::Countries, "Countries");
                    ui.selectable_value(&mut self.layer, Layer::Approval, "Approval");
                    ui.selectable_value(&mut self.layer, Layer::Needs, "Needs met");
                    ui.selectable_value(&mut self.layer, Layer::Debt, "Debt / opening GDP");
                });
        });
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height),
            Sense::click_and_drag(),
        );
        if response.dragged() {
            let delta = response.drag_delta();
            let scale = self.camera.scale(rect);
            self.camera.longitude -= delta.x / scale;
            self.camera.latitude += delta.y / scale;
            self.camera.clamp();
        }
        if response.hovered() {
            let scroll = ui.ctx().input_mut(|i| {
                let value = i.smooth_scroll_delta.y;
                i.smooth_scroll_delta = Vec2::ZERO;
                i.raw_scroll_delta = Vec2::ZERO;
                value
            });
            if scroll != 0.0 {
                self.camera.zoom_at(
                    rect,
                    response.hover_pos().unwrap_or(rect.center()),
                    (scroll * 0.002).exp(),
                );
            }
        }
        if response.clicked() {
            response.request_focus();
            if let Some(id) = response
                .interact_pointer_pos()
                .and_then(|p| nearest_marker(content, self.camera, rect, p))
            {
                *selected = id;
            }
        }
        if response.double_clicked() {
            self.camera = Camera::default();
        }
        if response.has_focus() {
            let delta = ui.input(|i| {
                Vec2::new(
                    f32::from(i.key_pressed(egui::Key::ArrowRight) as u8)
                        - f32::from(i.key_pressed(egui::Key::ArrowLeft) as u8),
                    f32::from(i.key_pressed(egui::Key::ArrowUp) as u8)
                        - f32::from(i.key_pressed(egui::Key::ArrowDown) as u8),
                )
            });
            self.camera.longitude += delta.x * 25.0 / self.camera.scale(rect);
            self.camera.latitude += delta.y * 25.0 / self.camera.scale(rect);
            if ui.input(|i| i.key_pressed(egui::Key::Home)) {
                self.camera = Camera::default();
            }
            self.camera.clamp();
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 8.0, Color32::from_rgb(13, 28, 41));
        let project = |p| self.camera.project(rect, p);
        for lon in (-180..=180).step_by(30) {
            painter.line_segment(
                [project([lon as f32, -90.0]), project([lon as f32, 90.0])],
                Stroke::new(0.5_f32, Color32::from_rgb(29, 46, 57)),
            );
        }
        for lat in (-60..=60).step_by(30) {
            painter.line_segment(
                [project([-180.0, lat as f32]), project([180.0, lat as f32])],
                Stroke::new(0.5_f32, Color32::from_rgb(29, 46, 57)),
            );
        }
        let mut mesh = egui::Mesh::default();
        for (points, triangles) in &self.land {
            let base = mesh.vertices.len() as u32;
            for p in points {
                mesh.colored_vertex(project(*p), Color32::from_rgb(38, 62, 68));
            }
            for i in triangles {
                mesh.indices.push(base + *i as u32);
            }
        }
        painter.add(egui::Shape::mesh(mesh));
        let hovered = response
            .hover_pos()
            .and_then(|p| nearest_marker(content, self.camera, rect, p));
        let mut occupied: Vec<Rect> = Vec::new();
        let mut countries: Vec<_> = content.countries.iter().collect();
        countries.sort_by_key(|c| c.id != *selected);
        for country in countries {
            let position = project([country.longitude as f32, country.latitude as f32]);
            if !rect.contains(position) {
                continue;
            }
            let active = country.id == *selected;
            let report = world.and_then(|w| w.public_report(content, &country.id).ok());
            let release = report.as_ref().and_then(|r| r.latest.clone());
            let value = release.as_ref().and_then(|o| match self.layer {
                Layer::Countries => None,
                Layer::Approval => Some(o.approval),
                Layer::Needs => Some(o.needs),
                Layer::Debt => Some((o.debt.0 * 100 / country.gdp) as i32),
            });
            let color = if self.layer == Layer::Countries {
                if active {
                    Color32::from_rgb(104, 224, 186)
                } else {
                    Color32::from_rgb(194, 183, 147)
                }
            } else {
                match value {
                    None => Color32::GRAY,
                    Some(v) => {
                        let healthy = if self.layer == Layer::Debt {
                            v < 80
                        } else {
                            v >= 70
                        };
                        if healthy {
                            Color32::from_rgb(104, 224, 186)
                        } else {
                            Color32::from_rgb(245, 181, 105)
                        }
                    }
                }
            };
            painter.circle_filled(position, if active { 6.0 } else { 4.0 }, color);
            if active {
                painter.circle_stroke(position, 10.0, Stroke::new(1.0_f32, color));
            }
            let label = format!(
                "{}{}",
                country.id.0,
                value.map_or(String::new(), |v| format!(" {v}%"))
            );
            let galley = painter.layout_no_wrap(label, egui::FontId::monospace(11.0), color);
            let label_rect = Rect::from_min_size(position + Vec2::new(10.0, -16.0), galley.size());
            if active || !occupied.iter().any(|r| r.intersects(label_rect)) {
                painter.galley(label_rect.min, galley, color);
                occupied.push(label_rect.expand(2.0));
            }
            if hovered.as_ref() == Some(&country.id) {
                let executive = report
                    .as_ref()
                    .map_or(country.executive.as_str(), |r| r.executive.as_str());
                let stamp = release.map_or_else(
                    || "No statistical release yet".into(),
                    |o| {
                        format!(
                            "Published {} · needs {}% · approval {}%",
                            sim_core::date(o.day),
                            o.needs,
                            o.approval
                        )
                    },
                );
                response.clone().on_hover_text(format!("{} · {:?}\n{executive}\n{stamp}\nClick marker to inspect. Search works for every seeded country.",country.name,country.regime));
            }
        }
        painter.text(
            rect.left_bottom() + Vec2::new(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag · Zoom at pointer · Focused map: arrows / Home",
            egui::FontId::proportional(11.0),
            Color32::GRAY,
        );
        ui.small("Natural Earth land outlines · markers are selection points, not borders or claims · overlays use dated public reports; gray means unknown");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_preserves_the_geographic_point_under_the_cursor() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 500.0));
        let cursor = Pos2::new(620.0, 185.0);
        let mut c = Camera::default();
        let before = c.unproject(rect, cursor);
        c.zoom_at(rect, cursor, 2.0);
        let after = c.unproject(rect, cursor);
        assert!((before[0] - after[0]).abs() < 0.0001 && (before[1] - after[1]).abs() < 0.0001);
    }
    #[test]
    fn close_markers_pick_the_nearest_country_instead_of_the_last_drawn() {
        let content = Content::bundled().unwrap();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 500.0));
        let c = Camera::default();
        for id in ["GBR", "FRA", "DEU", "CUB", "JPN"] {
            let d = content.country(&CountryId(id.into())).unwrap();
            let point = c.project(rect, [d.longitude as f32, d.latitude as f32]);
            assert_eq!(nearest_marker(&content, c, rect, point), Some(d.id.clone()));
        }
    }
}
