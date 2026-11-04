use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use sim_core::{Content, CountryId, PublicCountryReport, Viewer, World};
#[derive(Clone, Copy, Debug, PartialEq)]
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
    fn focus_bounds(&mut self, bounds: Rect, viewport: Rect) {
        let span = bounds.size().max(Vec2::splat(0.25));
        let scale = Camera::default().scale(viewport);
        self.longitude = bounds.center().x;
        self.latitude = bounds.center().y;
        self.zoom = ((viewport.width() * 0.8 / (span.x * scale))
            .min(viewport.height() * 0.8 / (span.y * scale)))
        .clamp(1.0, 12.0);
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
fn layer_value(layer: Layer, report: Option<&PublicCountryReport>) -> Option<i32> {
    let report = report?;
    let release = report.latest.as_ref()?;
    match layer {
        Layer::Countries => None,
        Layer::Approval => Some(release.approval),
        Layer::Needs => Some(release.needs),
        Layer::Debt => Some((release.debt.0 * 100 / report.opening_gdp.0) as i32),
    }
}
fn layer_color(layer: Layer, active: bool, value: Option<i32>) -> Color32 {
    if layer == Layer::Countries {
        if active {
            Color32::from_rgb(104, 224, 186)
        } else {
            Color32::from_rgb(194, 183, 147)
        }
    } else {
        match value {
            None => Color32::GRAY,
            Some(v) => {
                let healthy = if layer == Layer::Debt {
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
    }
}
fn nearest_marker(
    content: &Content,
    camera: Camera,
    rect: Rect,
    position: Pos2,
    eligible: impl Fn(&CountryId) -> bool,
) -> Option<CountryId> {
    content
        .countries
        .iter()
        .filter(|c| eligible(&c.id))
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
    atlas: crate::geometry::Atlas,
    hydrology: crate::hydrology::Hydrology,
    lakes: bool,
    rivers: bool,
    water_query: String,
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
            atlas: crate::geometry::Atlas::bundled(),
            hydrology: crate::hydrology::Hydrology::bundled(),
            lakes: true,
            rivers: true,
            water_query: String::new(),
            camera: Camera::default(),
            layer: Layer::Countries,
        }
    }
    fn pick(&self, content: &Content, rect: Rect, position: Pos2) -> Option<CountryId> {
        self.atlas
            .country_at(content, self.camera.unproject(rect, position))
            .or_else(|| {
                nearest_marker(content, self.camera, rect, position, |id| {
                    !self.atlas.countries.contains_key(id)
                })
            })
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        world: Option<&World>,
        selected: &mut CountryId,
        height: f32,
    ) {
        // One checked, public-only projection supplies shapes, markers and tooltips.
        let projection = world.map(|world| world.project(content, Viewer::Observer));
        let reports = projection.as_ref().and_then(|result| result.as_ref().ok());
        let report_error = projection.as_ref().and_then(|result| result.as_ref().err());
        if let Some(error) = report_error {
            ui.colored_label(
                Color32::LIGHT_RED,
                format!("Public reports unavailable: {error}"),
            );
        }
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(ui.available_width(), height));
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
            ui.checkbox(&mut self.lakes, "Lakes");
            ui.checkbox(&mut self.rivers, "Rivers");
            if let Some(reference) = self.hydrology.picker(ui, &mut self.water_query) {
                self.camera.focus_bounds(reference.bounds, viewport);
                match reference.kind {
                    crate::hydrology::WaterKind::Lake => self.lakes = true,
                    crate::hydrology::WaterKind::River => self.rivers = true,
                }
            }
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
                .and_then(|p| self.pick(content, rect, p))
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
        for (id, polygons) in &self.atlas.countries {
            if content.country(id).is_none() {
                continue;
            }
            let active = id == selected;
            let color = layer_color(
                self.layer,
                active,
                layer_value(self.layer, reports.and_then(|view| view.reports.get(id))),
            );
            let fill = Color32::from_rgba_unmultiplied(
                color.r(),
                color.g(),
                color.b(),
                if active { 100 } else { 35 },
            );
            let mut mesh = egui::Mesh::default();
            for polygon in polygons {
                let base = mesh.vertices.len() as u32;
                for point in &polygon.points {
                    mesh.colored_vertex(project(*point), fill);
                }
                for index in &polygon.triangles {
                    mesh.indices.push(base + *index as u32);
                }
            }
            painter.add(egui::Shape::mesh(mesh));
            for polygon in polygons {
                for ring in &polygon.rings {
                    painter.add(egui::Shape::line(
                        ring.iter().map(|point| project(*point)).collect(),
                        Stroke::new(
                            if active { 1.4_f32 } else { 0.7_f32 },
                            color.gamma_multiply(if active { 1.0 } else { 0.6 }),
                        ),
                    ));
                }
            }
        }
        self.hydrology
            .paint(&painter, project, self.lakes, self.rivers);
        let hovered = response
            .hover_pos()
            .and_then(|p| self.pick(content, rect, p));
        let mut occupied: Vec<Rect> = Vec::new();
        let mut countries: Vec<_> = content.countries.iter().collect();
        countries.sort_by_key(|c| c.id != *selected);
        for country in countries {
            let position = project([country.longitude as f32, country.latitude as f32]);
            if !rect.contains(position) {
                continue;
            }
            let active = country.id == *selected;
            let value = layer_value(
                self.layer,
                reports.and_then(|view| view.reports.get(&country.id)),
            );
            let color = layer_color(self.layer, active, value);
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
        }
        let water = response.hover_pos().and_then(|cursor| {
            self.hydrology.hover(
                self.camera.unproject(rect, cursor),
                cursor,
                project,
                self.lakes,
                self.rivers,
            )
        });
        if let Some((feature, kind)) = water {
            response
                .clone()
                .on_hover_text(self.hydrology.hover_text(feature, kind));
        } else if let Some(id) = hovered
            && let Some(country) = content.country(&id)
        {
            let report = reports.and_then(|view| view.reports.get(&id));
            let executive = report
                .as_ref()
                .map_or(country.executive.as_str(), |r| r.executive.as_str());
            let stamp = report.as_ref().and_then(|r| r.latest.as_ref()).map_or_else(
                || {
                    if report_error.is_some() {
                        "Public reports unavailable".into()
                    } else {
                        "No statistical release yet".into()
                    }
                },
                |o| {
                    format!(
                        "Published {} · needs {}% · approval {}%",
                        sim_core::date(o.day),
                        o.needs,
                        o.approval
                    )
                },
            );
            response.clone().on_hover_text(format!("{} · {:?}\n{executive}\n{stamp}\nClick a country outline or its marker to inspect. Search covers islands omitted at this scale.",country.name,report.as_ref().map_or(&country.regime, |report| &report.regime)));
        }
        painter.text(
            rect.left_bottom() + Vec2::new(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag · Zoom at pointer · Focused map: arrows / Home",
            egui::FontId::proportional(11.0),
            Color32::GRAY,
        );
        ui.small("Natural Earth reference outlines · historical border review incomplete · display shapes do not define sovereignty or simulation regions · overlays use dated reports; gray means unknown");
        self.hydrology.provenance(ui);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlays_share_published_values_and_withhold_mismatched_campaign_reports() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let id = world.player.clone();
        let opening = world.project(&content, Viewer::Observer).unwrap();
        assert_eq!(layer_value(Layer::Approval, opening.reports.get(&id)), None);
        world.advance(&content, 31).unwrap();
        let before = world.hash();
        let view = world.project(&content, Viewer::Observer).unwrap();
        let report = &view.reports[&id];
        let release = report.latest.as_ref().unwrap();
        assert_eq!(release.day, 30);
        assert_eq!(
            layer_value(Layer::Approval, Some(report)),
            Some(release.approval)
        );
        assert_eq!(layer_value(Layer::Needs, Some(report)), Some(release.needs));
        assert_eq!(
            layer_value(Layer::Debt, Some(report)),
            Some((release.debt.0 * 100 / report.opening_gdp.0) as i32)
        );
        let mut other = content.clone();
        other.countries[0].name.push_str(" changed");
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut map = WorldMap::new();
            map.layer = Layer::Debt;
            let mut selected = id.clone();
            let output = frame(&ctx, &mut map, &other, &world, &mut selected, scale, vec![]);
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().contains("Public reports unavailable:"))));
            assert_eq!(selected, id);
        }
        assert_eq!(world.hash(), before);
    }
    fn frame(
        ctx: &egui::Context,
        map: &mut WorldMap,
        content: &Content,
        world: &World,
        selected: &mut CountryId,
        scale: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1200.0 / scale, 900.0 / scale),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    map.show(ui, content, Some(world), selected, 350.0)
                });
            },
        )
    }
    #[test]
    fn physical_layer_mouse_controls_preserve_camera_selection_and_campaign_at_two_scales() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, 30).unwrap();
        let before = world.hash();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut map = WorldMap::new();
            map.camera = Camera {
                longitude: -75.0,
                latitude: 43.0,
                zoom: 4.0,
            };
            let camera = map.camera;
            let mut selected = world.player.clone();
            let initial = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            let label_position = |output: &egui::FullOutput, name: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|s| {
                        if let egui::Shape::Text(t) = &s.shape {
                            (t.galley.text() == name)
                                .then_some(t.pos + Vec2::new(3.0, t.galley.size().y / 2.0))
                        } else {
                            None
                        }
                    })
                    .unwrap()
            };
            for label in ["Lakes", "Rivers"] {
                let p = label_position(&initial, label);
                assert!(p.x < 1200.0 / scale && p.y < 900.0 / scale);
                for pressed in [true, false] {
                    frame(
                        &ctx,
                        &mut map,
                        &content,
                        &world,
                        &mut selected,
                        scale,
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
            }
            assert!(!map.lakes && !map.rivers);
            assert_eq!(map.camera, camera);
            assert_eq!(selected, world.player);
            assert!(map.layer == Layer::Countries);
            // Re-enabling the physical display must not alter country-outline picking.
            let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 500.0));
            let point = map.camera.project(rect, [-80.0, 40.0]);
            assert!(rect.contains(point));
            let prior = map.pick(&content, rect, point);
            assert_eq!(prior, Some(CountryId("USA".into())));
            map.lakes = true;
            map.rivers = true;
            assert_eq!(map.pick(&content, rect, point), prior);
            assert_eq!(world.hash(), before);
        }
    }

    #[test]
    fn reference_picker_handles_mouse_aliases_and_keyboard_ambiguity_without_country_orders() {
        fn text(output: &egui::FullOutput, label: &str) -> Pos2 {
            output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        (t.galley.text() == label)
                            .then_some(t.pos + Vec2::new(3.0, t.galley.size().y / 2.0))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| panic!("missing {label}"))
        }
        fn click(
            ctx: &egui::Context,
            map: &mut WorldMap,
            content: &Content,
            world: &World,
            selected: &mut CountryId,
            scale: f32,
            p: Pos2,
        ) -> egui::FullOutput {
            let pointer = |pressed| {
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]
            };
            frame(ctx, map, content, world, selected, scale, pointer(true));
            frame(ctx, map, content, world, selected, scale, pointer(false))
        }
        let key = |key| {
            [true, false]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .collect()
        };
        let content = Content::bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let before = world.hash();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut map = WorldMap::new();
            map.lakes = false;
            map.rivers = false;
            map.layer = Layer::Debt;
            let camera = map.camera;
            let mut selected = world.player.clone();
            let output = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            let p = text(&output, "Find lake / river");
            click(&ctx, &mut map, &content, &world, &mut selected, scale, p);
            let output = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![egui::Event::Text("lake balkhash".into())],
            );
            assert_eq!(map.water_query, "lake balkhash");
            assert_eq!(map.camera, camera);
            assert_eq!(selected, world.player);
            let p = text(&output, "Balqash Köli · lake-0009");
            assert!(
                Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0 / scale, 900.0 / scale))
                    .contains(p)
            );
            click(&ctx, &mut map, &content, &world, &mut selected, scale, p);
            let target = map.hydrology.matches("lake balkhash").remove(0);
            assert_eq!(map.camera.longitude, target.bounds.center().x);
            assert_eq!(map.camera.latitude, target.bounds.center().y);
            assert!(map.lakes && !map.rivers);
            assert_eq!(selected, world.player);
            assert!(map.layer == Layer::Debt);
            let focused = map.camera;
            let output = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            let p = text(&output, "Find lake / river");
            click(&ctx, &mut map, &content, &world, &mut selected, scale, p);
            frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![egui::Event::Text("river peace".into())],
            );
            let output = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                key(egui::Key::Enter),
            );
            assert_eq!(map.camera, focused);
            text(&output, "Peace · river-0004");
            text(&output, "Peace · river-0007");
            frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                key(egui::Key::Escape),
            );
            let output = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            let p = text(&output, "Find lake / river");
            click(&ctx, &mut map, &content, &world, &mut selected, scale, p);
            frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![egui::Event::Text("river-0012".into())],
            );
            assert_eq!(map.camera, focused);
            frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                key(egui::Key::Enter),
            );
            let target = map.hydrology.matches("river-0012").remove(0);
            assert_eq!(map.camera.longitude, target.bounds.center().x);
            assert_eq!(map.camera.latitude, target.bounds.center().y);
            assert!(map.lakes && map.rivers);
            assert_eq!(selected, world.player);
            assert!(map.layer == Layer::Debt);
            assert_eq!(world.hash(), before);
        }
    }

    #[test]
    fn reference_focus_fits_all_source_bounds_and_does_not_depend_on_the_previous_view() {
        let h = crate::hydrology::Hydrology::bundled();
        for rect in [
            Rect::from_min_size(Pos2::new(80.0, 140.0), Vec2::new(1000.0, 400.0)),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(420.0, 300.0)),
        ] {
            for reference in h.matches("") {
                let mut a = Camera::default();
                let mut b = Camera {
                    longitude: 150.0,
                    latitude: -60.0,
                    zoom: 12.0,
                };
                a.focus_bounds(reference.bounds, rect);
                b.focus_bounds(reference.bounds, rect);
                assert_eq!(a, b);
                assert!((1.0..=12.0).contains(&a.zoom));
                let center = a.project(
                    rect,
                    [reference.bounds.center().x, reference.bounds.center().y],
                );
                assert!(center.distance(rect.center()) < 0.001);
                for point in [reference.bounds.min, reference.bounds.max] {
                    assert!(
                        rect.shrink(2.0)
                            .contains(a.project(rect, [point.x, point.y])),
                        "{} must fit {rect:?}",
                        reference.id
                    );
                }
            }
        }
    }
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
            assert_eq!(
                nearest_marker(&content, c, rect, point, |_| true),
                Some(d.id.clone())
            );
        }
    }
    #[test]
    fn polygon_picking_preserves_holes_at_every_zoom_and_markers_for_fictional_states() {
        let content = Content::bundled().unwrap();
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 500.0));
        let mut map = WorldMap::new();
        for zoom in [1.0, 4.0, 12.0] {
            map.camera = Camera {
                longitude: 28.2,
                latitude: -29.5,
                zoom,
            };
            let hole = map.camera.project(rect, [28.2, -29.5]);
            assert_eq!(map.pick(&content, rect, hole), None);
        }
        map.camera = Camera::default();
        for d in &content.countries {
            let point = map
                .camera
                .project(rect, [d.longitude as f32, d.latitude as f32]);
            assert_eq!(
                map.pick(&content, rect, point),
                Some(d.id.clone()),
                "{} marker",
                d.id.0
            );
        }
        let fictional =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        let point = map.camera.project(rect, [0.0, 30.0]);
        assert_eq!(
            map.pick(&fictional, rect, point),
            Some(CountryId("AUR".into()))
        );
    }
}
