use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use sim_core::{Content, CountryId};

pub struct WorldMap {
    land: Vec<(Vec<[f32; 2]>, Vec<usize>)>,
    zoom: f32,
    offset: Vec2,
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
            zoom: 1.0,
            offset: Vec2::ZERO,
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        selected: &mut CountryId,
        height: f32,
    ) {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height),
            Sense::click_and_drag(),
        );
        if response.dragged() {
            self.offset += response.drag_delta();
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            self.zoom = (self.zoom * (1.0 + scroll * 0.002)).clamp(1.0, 8.0);
        }
        if response.double_clicked() {
            self.zoom = 1.0;
            self.offset = Vec2::ZERO;
        }
        self.offset.x = self.offset.x.clamp(
            -rect.width() * self.zoom / 2.0,
            rect.width() * self.zoom / 2.0,
        );
        self.offset.y = self.offset.y.clamp(
            -rect.height() * self.zoom / 2.0,
            rect.height() * self.zoom / 2.0,
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 8.0, Color32::from_rgb(13, 28, 41));
        let scale = (rect.width() / 360.0).min(rect.height() / 155.0) * self.zoom;
        let project = |p: [f32; 2]| {
            Pos2::new(
                rect.center().x + p[0] * scale + self.offset.x,
                rect.center().y - (p[1] - 10.0) * scale + self.offset.y,
            )
        };
        for lon in (-180..=180).step_by(30) {
            painter.line_segment(
                [project([lon as f32, -70.0]), project([lon as f32, 85.0])],
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
        for country in &content.countries {
            let position = project([country.longitude as f32, country.latitude as f32]);
            if !rect.contains(position) {
                continue;
            }
            let active = country.id == *selected;
            let hit = Rect::from_center_size(position, Vec2::splat(22.0)).intersect(rect);
            let marker = ui.interact(hit, ui.id().with(&country.id.0), Sense::click());
            let color = if active {
                Color32::from_rgb(104, 224, 186)
            } else {
                Color32::from_rgb(194, 183, 147)
            };
            painter.circle_filled(position, if active { 6.0 } else { 3.5 }, color);
            if active {
                painter.circle_stroke(position, 10.0, Stroke::new(1.0_f32, color));
            }
            painter.text(
                position + Vec2::new(9.0, -8.0),
                egui::Align2::LEFT_BOTTOM,
                &country.id.0,
                egui::FontId::monospace(11.0),
                color,
            );
            if marker.clicked() {
                *selected = country.id.clone();
            }
            marker.on_hover_text(format!(
                "{} · {:?}\n{}: {}\nClick to inspect",
                country.name, country.regime, country.office, country.executive
            ));
        }
        painter.text(
            rect.left_bottom() + Vec2::new(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag to pan · Scroll to zoom · Double-click to reset",
            egui::FontId::proportional(11.0),
            Color32::GRAY,
        );
        ui.small("Natural Earth land outlines · country markers · no political borders or territorial claims");
    }
}
