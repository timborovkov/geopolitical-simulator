use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use sim_core::{
    Content, CountryId, PlayerView, PublicCountryReport, PublishedObservation, Viewer, World,
};
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ReportTime {
    #[default]
    Latest,
    Published(u32),
}
impl ReportTime {
    fn for_layer(self, layer: Layer) -> Self {
        if layer == Layer::Countries {
            Self::Latest
        } else {
            self
        }
    }
    fn release(self, report: &PublicCountryReport) -> Option<&PublishedObservation> {
        match self {
            Self::Latest => report.latest.as_ref(),
            Self::Published(day) => report
                .history
                .iter()
                .filter(|release| release.day <= day)
                .max_by_key(|release| release.day),
        }
    }
    fn dates(view: Option<&PlayerView>) -> Vec<u32> {
        let Some(view) = view else {
            return Vec::new();
        };
        view.reports
            .values()
            .flat_map(|report| report.history.iter().map(|release| release.day))
            .filter(|day| *day <= view.day)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    fn normalize(&mut self, dates: &[u32]) {
        if let Self::Published(day) = *self
            && dates.binary_search(&day).is_err()
        {
            *self = Self::Latest;
        }
    }
    fn picker(&mut self, ui: &mut egui::Ui, dates: &[u32]) {
        ui.add_enabled_ui(!dates.is_empty(), |ui| {
            egui::ComboBox::from_label("Statistics")
                .selected_text(match *self { Self::Latest => "Latest releases".into(), Self::Published(day) => format!("As of {}", sim_core::date(day)) })
                .show_ui(ui, |ui| {
                    ui.selectable_value(self, Self::Latest, "Latest releases");
                    for day in dates.iter().rev() {
                        ui.selectable_value(self, Self::Published(*day), format!("Published {}", sim_core::date(*day)));
                    }
                }).response.on_hover_text("Changes the displayed statistical releases. Campaign time follows the time controls.");
        });
    }
}
impl Layer {
    fn title(self) -> &'static str {
        match self {
            Self::Countries => "Countries",
            Self::Approval => "Approval",
            Self::Needs => "Needs met",
            Self::Debt => "Debt / opening GDP",
        }
    }
    fn bands(self) -> (&'static str, &'static str) {
        match self {
            Self::Debt => ("Below 80%", "80% or above"),
            _ => ("70% or above", "Below 70%"),
        }
    }
    fn details(
        self,
        ui: &mut egui::Ui,
        view: Option<&PlayerView>,
        selected: &mut CountryId,
        time: ReportTime,
    ) {
        if self == Self::Countries {
            return;
        }
        let (green, amber) = self.bands();
        ui.horizontal_wrapped(|ui| {
            ui.strong(self.title()).on_hover_text("Colour bands are display thresholds. The five-year review uses its separate food, energy, approval and debt targets.");
            // Use the same colour function as both the polygons and markers.
            ui.colored_label(
                layer_color(self, false, Some(if self == Self::Debt { 0 } else { 100 })),
                green,
            );
            ui.colored_label(
                layer_color(self, false, Some(if self == Self::Debt { 100 } else { 0 })),
                amber,
            );
            ui.colored_label(layer_color(self, false, None), "Unknown");
        });
        let Some(view) = view else {
            ui.small("No campaign statistics available.");
            return;
        };
        let dates: std::collections::BTreeSet<_> = view
            .reports
            .values()
            .filter_map(|report| time.release(report).map(|release| release.day))
            .collect();
        let published = view
            .reports
            .values()
            .filter(|report| time.release(report).is_some())
            .count();
        let dates = match (dates.first(), dates.last()) {
            (Some(first), Some(last)) if first == last => {
                format!("Published {}", sim_core::date(*first))
            }
            (Some(first), Some(last)) => format!(
                "Latest release dates {} to {}",
                sim_core::date(*first),
                sim_core::date(*last)
            ),
            _ => "No statistical release yet".into(),
        };
        ui.small(format!(
            "{dates} · {published}/{} countries published · campaign {}",
            view.reports.len(),
            sim_core::date(view.day)
        ));
        if let ReportTime::Published(day) = time {
            ui.small(format!("Statistics cutoff {}. Age uses the campaign date; government labels stay current and reference geography stays fixed.", sim_core::date(day)));
        }
        if self == Self::Debt {
            ui.small("Published debt / fixed opening annual GDP. The denominator stays fixed throughout the campaign.");
        }
        ui.collapsing("Overlay reports — select a country", |ui| {
            egui::Grid::new("map-overlay-reports")
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("Country");
                    ui.strong(self.title());
                    ui.strong("Published");
                    ui.strong("Age");
                    ui.end_row();
                    for report in view.reports.values() {
                        if ui
                            .selectable_label(
                                report.country == *selected,
                                format!("{} {}", report.country.0, report.name),
                            )
                            .clicked()
                        {
                            *selected = report.country.clone();
                        }
                        ui.label(
                            layer_value(self, Some(report), time)
                                .map_or_else(|| "Unknown".into(), |value| format!("{value}%")),
                        );
                        ui.label(time.release(report).map_or_else(
                            || "No release".into(),
                            |release| sim_core::date(release.day),
                        ));
                        ui.label(time.release(report).map_or_else(
                            || "—".into(),
                            |release| {
                                let age = view.day.saturating_sub(release.day);
                                format!("{age} {}", if age == 1 { "day" } else { "days" })
                            },
                        ));
                        ui.end_row();
                    }
                });
        });
    }
}
fn layer_value(
    layer: Layer,
    report: Option<&PublicCountryReport>,
    time: ReportTime,
) -> Option<i32> {
    let report = report?;
    let release = time.release(report)?;
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
/// Small presentation snapshot; geometry and campaign data are never copied into history.
#[derive(Clone, PartialEq)]
pub struct View {
    camera: Camera,
    layer: Layer,
    report_time: ReportTime,
    divisions: bool,
    selected_division: Option<crate::administrative::ReferenceDivisionId>,
    lakes: bool,
    rivers: bool,
}
pub struct WorldMap {
    land: Vec<(Vec<[f32; 2]>, Vec<usize>)>,
    atlas: crate::geometry::Atlas,
    hydrology: crate::hydrology::Hydrology,
    administrative: std::sync::Arc<crate::administrative::AdministrativeAtlas>,
    divisions: bool,
    division_query: String,
    selected_division: Option<crate::administrative::ReferenceDivisionId>,
    lakes: bool,
    rivers: bool,
    water_query: String,
    report_time: ReportTime,
    pub camera: Camera,
    layer: Layer,
}
impl WorldMap {
    pub fn view(&self) -> View {
        View {
            camera: self.camera,
            layer: self.layer,
            report_time: self.report_time,
            divisions: self.divisions,
            selected_division: self.selected_division.clone(),
            lakes: self.lakes,
            rivers: self.rivers,
        }
    }
    pub fn restore_view(&mut self, view: View) {
        self.camera = view.camera;
        self.layer = view.layer;
        self.report_time = view.report_time;
        self.divisions = view.divisions;
        self.selected_division = view.selected_division;
        self.lakes = view.lakes;
        self.rivers = view.rivers;
    }
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
            administrative: crate::administrative::AdministrativeAtlas::bundled(),
            divisions: false,
            division_query: String::new(),
            selected_division: None,
            lakes: true,
            rivers: true,
            water_query: String::new(),
            report_time: ReportTime::Latest,
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
        if self
            .selected_division
            .as_ref()
            .and_then(|id| self.administrative.division(id))
            .is_none_or(|division| {
                division.country != *selected || content.country(&division.country).is_none()
            })
        {
            self.selected_division = None;
        }
        let dates = ReportTime::dates(reports);
        self.report_time.normalize(&dates);
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
                .selected_text(self.layer.title())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.layer, Layer::Countries, "Countries");
                    ui.selectable_value(&mut self.layer, Layer::Approval, "Approval");
                    ui.selectable_value(&mut self.layer, Layer::Needs, "Needs met");
                    ui.selectable_value(&mut self.layer, Layer::Debt, "Debt / opening GDP");
                });
            if self.layer != Layer::Countries {
                self.report_time.picker(ui, &dates);
            }
            ui.checkbox(&mut self.lakes, "Lakes");
            ui.checkbox(&mut self.rivers, "Rivers");
            let coverage = self.administrative.coverage(selected);
            if coverage == 0 {
                self.divisions = false;
            }
            ui.add_enabled(
                coverage > 0,
                egui::Checkbox::new(&mut self.divisions, "Divisions"),
            )
            .on_hover_text("Show reference administrative boundaries for the selected country.")
            .on_disabled_hover_text("No geographic subdivision coverage for this country.");
            if let Some(id) = self
                .administrative
                .picker(ui, content, &mut self.division_query)
                && let Some(division) = self.administrative.division(&id)
            {
                *selected = division.country.clone();
                self.camera.focus_bounds(division.focus_bounds, viewport);
                self.divisions = true;
                self.selected_division = Some(id);
            }

            if let Some(reference) = self.hydrology.picker(ui, &mut self.water_query) {
                self.camera.focus_bounds(reference.bounds, viewport);
                match reference.kind {
                    crate::hydrology::WaterKind::Lake => self.lakes = true,
                    crate::hydrology::WaterKind::River => self.rivers = true,
                }
            }
        });
        let report_time = self.report_time.for_layer(self.layer);
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
                if self
                    .selected_division
                    .as_ref()
                    .and_then(|id| self.administrative.division(id))
                    .is_some_and(|division| division.country != *selected)
                {
                    self.selected_division = None;
                }
            }
            if self.divisions
                && let Some(point) = response.interact_pointer_pos()
                && let Some(division) =
                    self.administrative
                        .at(content, selected, self.camera.unproject(rect, point))
            {
                self.selected_division = Some(division.id.clone());
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
                layer_value(
                    self.layer,
                    reports.and_then(|view| view.reports.get(id)),
                    report_time,
                ),
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
        if self.divisions {
            let a = self.camera.unproject(rect, rect.left_top());
            let b = self.camera.unproject(rect, rect.right_bottom());
            let geographic_viewport =
                Rect::from_two_pos(Pos2::new(a[0], a[1]), Pos2::new(b[0], b[1]));
            self.administrative.paint(
                &painter,
                selected,
                self.selected_division.as_ref(),
                geographic_viewport,
                project,
            );
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
                report_time,
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
        let division_hover = if self.divisions {
            response.hover_pos().and_then(|cursor| {
                self.administrative
                    .at(content, selected, self.camera.unproject(rect, cursor))
            })
        } else {
            None
        };
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
            let stamp = report.and_then(|r| report_time.release(r)).map_or_else(
                || {
                    if report_error.is_some() {
                        "Public reports unavailable".into()
                    } else if report_time == ReportTime::Latest {
                        "No statistical release yet".into()
                    } else {
                        "No statistical release by this date".into()
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
            let overlay = if self.layer == Layer::Countries {
                String::new()
            } else {
                format!(
                    "\n{}: {}",
                    self.layer.title(),
                    layer_value(self.layer, report, report_time)
                        .map_or_else(|| "Unknown".into(), |value| format!("{value}%"))
                )
            };
            let executive = if report_time == ReportTime::Latest {
                executive.to_string()
            } else {
                format!("Current executive: {executive}")
            };
            let division_label = division_hover.map_or(String::new(), |division| {
                format!("{}\n\n", division.description(content))
            });
            response.clone().on_hover_text(format!("{division_label}{} · {:?}\n{executive}\n{stamp}{overlay}\nClick a country outline or its marker to inspect. Search covers islands omitted at this scale.",country.name,report.as_ref().map_or(&country.regime, |report| &report.regime)));
        }
        painter.text(
            rect.left_bottom() + Vec2::new(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag · Zoom at pointer · Focused map: arrows / Home",
            egui::FontId::proportional(11.0),
            Color32::GRAY,
        );
        self.layer.details(ui, reports, selected, report_time);
        ui.small("Natural Earth reference outlines · historical border review incomplete · display shapes do not define sovereignty or simulation regions · overlays use dated reports; gray means unknown");
        self.hydrology.provenance(ui);
        if self.divisions {
            let count = self.administrative.coverage(selected);
            ui.small(format!("Reference divisions: {count} source features for {}. Scenario jurisdictions and their powers appear under Institutions & parties.",selected.0));
            if let Some(division) = self
                .selected_division
                .as_ref()
                .and_then(|id| self.administrative.division(id))
            {
                let label = format!(
                    "{} · {} · {}",
                    division.display_name(),
                    division.country.0,
                    division.id.0
                );
                let bounds = division.focus_bounds;
                let mut clear = false;
                ui.horizontal_wrapped(|ui| {
                    ui.strong(label)
                        .on_hover_text(division.description(content));
                    if ui.button("Focus division").clicked() {
                        self.camera.focus_bounds(bounds, rect);
                    }
                    clear = ui.button("Clear division").clicked();
                });
                if clear {
                    self.selected_division = None;
                }
            }
        }
        self.administrative.provenance(ui);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspection_view_restores_all_map_controls_without_copying_geometry() {
        let mut map = WorldMap::new();
        map.camera = Camera {
            longitude: 11.6,
            latitude: 48.2,
            zoom: 8.0,
        };
        map.layer = Layer::Needs;
        map.report_time = ReportTime::Published(30);
        map.divisions = true;
        map.selected_division = Some(crate::administrative::ReferenceDivisionId(
            "DEU-1591".into(),
        ));
        map.lakes = false;
        map.rivers = false;
        let saved = map.view();
        map.camera = Camera::default();
        map.layer = Layer::Debt;
        map.report_time = ReportTime::Latest;
        map.divisions = false;
        map.selected_division = None;
        map.lakes = true;
        map.rivers = true;
        map.restore_view(saved.clone());
        assert!(map.view() == saved);
    }
    #[test]
    fn overlays_share_published_values_and_withhold_mismatched_campaign_reports() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let id = world.player.clone();
        let opening = world.project(&content, Viewer::Observer).unwrap();
        assert_eq!(
            layer_value(
                Layer::Approval,
                opening.reports.get(&id),
                ReportTime::Latest
            ),
            None
        );
        world.advance(&content, sim_core::DaySpan::new(31)).unwrap();
        let before = world.hash();
        let view = world.project(&content, Viewer::Observer).unwrap();
        let report = &view.reports[&id];
        let release = report.latest.as_ref().unwrap();
        assert_eq!(release.day, 30);
        assert_eq!(
            layer_value(Layer::Approval, Some(report), ReportTime::Latest),
            Some(release.approval)
        );
        assert_eq!(
            layer_value(Layer::Needs, Some(report), ReportTime::Latest),
            Some(release.needs)
        );
        assert_eq!(
            layer_value(Layer::Debt, Some(report), ReportTime::Latest),
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
    #[test]
    fn overlay_legend_and_report_table_show_unknowns_dates_and_select_without_orders() {
        let content = Content::bundled().unwrap();
        let opening = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let mut released = opening.clone();
        released
            .advance(&content, sim_core::DaySpan::new(31))
            .unwrap();
        for scale in [1.0, 1.4] {
            for world in [&opening, &released] {
                let before = world.hash();
                let ctx = egui::Context::default();
                ctx.set_zoom_factor(scale);
                let mut map = WorldMap::new();
                map.layer = Layer::Debt;
                let camera = map.camera;
                let mut selected = world.player.clone();
                let initial = frame(
                    &ctx,
                    &mut map,
                    &content,
                    world,
                    &mut selected,
                    scale,
                    vec![],
                );
                let texts = |out: &egui::FullOutput| {
                    out.shapes
                        .iter()
                        .filter_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) => Some(text.galley.text().to_string()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                };
                let labels = texts(&initial);
                assert!(labels.iter().any(|text| text.contains("Below 80%")));
                assert!(labels.iter().any(|text| text.contains("80% or above")));
                assert!(
                    labels
                        .iter()
                        .any(|text| text.contains("fixed opening annual GDP"))
                );
                let expected = if world.day == 0 {
                    "No statistical release yet · 0/15"
                } else {
                    "Published 2000-01-31 · 15/15"
                };
                assert!(
                    labels.iter().any(|text| text.contains(expected)),
                    "{labels:?}"
                );
                let position = |out: &egui::FullOutput, label: &str| {
                    out.shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.text() == label => {
                                Some(text.pos + Vec2::new(5.0, text.galley.size().y / 2.0))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| panic!("missing {label}"))
                };
                let click = |pos| {
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]
                };
                let pos = position(&initial, "Overlay reports — select a country");
                let _ = frame(
                    &ctx,
                    &mut map,
                    &content,
                    world,
                    &mut selected,
                    scale,
                    click(pos),
                );
                let expanded = frame(
                    &ctx,
                    &mut map,
                    &content,
                    world,
                    &mut selected,
                    scale,
                    vec![],
                );
                let labels = texts(&expanded);
                let expected = if world.day == 0 {
                    "No release"
                } else {
                    "1 day"
                };
                assert!(labels.iter().any(|text| text == expected), "{labels:?}");
                if world.day == 0 {
                    assert!(labels.iter().any(|text| text == "Unknown"));
                }
                let pos = position(&expanded, "BRA Brazil");
                assert!(pos.y < 900.0 / scale);
                let _ = frame(
                    &ctx,
                    &mut map,
                    &content,
                    world,
                    &mut selected,
                    scale,
                    click(pos),
                );
                assert_eq!(selected, CountryId("BRA".into()));
                assert_eq!(map.camera, camera);
                assert_eq!(world.player, CountryId("GBR".into()));
                assert_eq!(world.hash(), before);
            }
        }
    }
    #[test]
    fn historical_statistics_choose_the_last_release_by_cutoff_and_keep_gaps_unknown() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, sim_core::DaySpan::new(91)).unwrap();
        let before = world.hash();
        let mut view = world.project(&content, Viewer::Observer).unwrap();
        assert!(view.own.is_none());
        assert_eq!(ReportTime::dates(Some(&view)), vec![30, 60, 90]);
        let mut report = view.reports[&world.player].clone();
        assert_eq!(
            ReportTime::Published(30)
                .for_layer(Layer::Countries)
                .release(&report)
                .unwrap()
                .day,
            90
        );
        assert_eq!(
            ReportTime::Published(30)
                .for_layer(Layer::Approval)
                .release(&report)
                .unwrap()
                .day,
            30
        );
        assert_eq!(ReportTime::Latest.release(&report).unwrap().day, 90);
        for (cutoff, expected) in [
            (29, None),
            (30, Some(30)),
            (59, Some(30)),
            (60, Some(60)),
            (91, Some(90)),
        ] {
            assert_eq!(
                ReportTime::Published(cutoff)
                    .release(&report)
                    .map(|release| release.day),
                expected
            );
        }
        report.history.reverse();
        assert_eq!(ReportTime::Published(59).release(&report).unwrap().day, 30);
        report.history.retain(|release| release.day >= 60);
        assert!(ReportTime::Published(30).release(&report).is_none());
        assert_eq!(
            layer_value(Layer::Approval, Some(&report), ReportTime::Published(30)),
            None
        );
        assert_eq!(layer_color(Layer::Approval, false, None), Color32::GRAY);
        let mut future = report.latest.clone().unwrap();
        future.day = 120;
        view.reports
            .get_mut(&world.player)
            .unwrap()
            .history
            .push(future);
        assert_eq!(ReportTime::dates(Some(&view)), vec![30, 60, 90]);
        assert_eq!(world.hash(), before);
    }
    fn history_text_position(out: &egui::FullOutput, label: &str) -> Pos2 {
        out.shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + Vec2::new(5.0, text.galley.size().y / 2.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {label}"))
    }
    fn history_click(pos: Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }
    fn history_labels(out: &egui::FullOutput) -> Vec<String> {
        out.shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_string()),
                _ => None,
            })
            .collect()
    }
    #[test]
    fn map_publication_picker_changes_only_the_display_and_can_return_to_latest_at_two_scales() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, sim_core::DaySpan::new(91)).unwrap();
        let before = world.hash();
        let report = world.public_report(&content, &world.player).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut map = WorldMap::new();
            map.layer = Layer::Approval;
            map.lakes = false;
            map.camera = Camera {
                longitude: -50.0,
                latitude: 30.0,
                zoom: 2.0,
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
            let pos = history_text_position(&initial, "Latest releases");
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                history_click(pos),
            );
            let menu = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            let pos = history_text_position(&menu, "Published 2000-01-31");
            assert!(pos.x < 1200.0 / scale && pos.y < 900.0 / scale);
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                history_click(pos),
            );
            let past = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            assert_eq!(map.report_time, ReportTime::Published(30));
            let labels = history_labels(&past);
            let approval = ReportTime::Published(30).release(&report).unwrap().approval;
            assert!(
                labels
                    .iter()
                    .any(|text| text == &format!("GBR {approval}%")),
                "{labels:?}"
            );
            assert!(
                labels
                    .iter()
                    .any(|text| text.contains("Published 2000-01-31 · 15/15"))
            );
            assert!(
                labels
                    .iter()
                    .any(|text| text.contains("government labels stay current"))
            );
            let pos = history_text_position(&past, "Overlay reports — select a country");
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                history_click(pos),
            );
            let table = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            assert!(history_labels(&table).iter().any(|text| text == "61 days"));
            let pos = history_text_position(&table, "As of 2000-01-31");
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                history_click(pos),
            );
            let menu = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            let pos = history_text_position(&menu, "Latest releases");
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                history_click(pos),
            );
            let latest = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            assert_eq!(map.report_time, ReportTime::Latest);
            assert!(
                history_labels(&latest)
                    .iter()
                    .any(|text| text.contains("Published 2000-03-31 · 15/15"))
            );
            assert_eq!(map.camera, camera);
            assert_eq!(selected, world.player);
            assert!(!map.lakes && map.rivers && map.layer == Layer::Approval);
            assert_eq!(world.hash(), before);
        }
    }
    #[test]
    fn map_publication_cutoff_survives_progression_and_resets_when_the_release_is_unavailable() {
        let content = Content::bundled().unwrap();
        let opening = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let mut world = opening.clone();
        world.advance(&content, sim_core::DaySpan::new(61)).unwrap();
        let ctx = egui::Context::default();
        let mut map = WorldMap::new();
        map.layer = Layer::Approval;
        map.report_time = ReportTime::Published(30);
        let mut selected = world.player.clone();
        let _ = frame(&ctx, &mut map, &content, &world, &mut selected, 1.0, vec![]);
        world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
        let before = world.hash();
        let _ = frame(&ctx, &mut map, &content, &world, &mut selected, 1.0, vec![]);
        assert_eq!(map.report_time, ReportTime::Published(30));
        assert_eq!(world.hash(), before);
        let _ = frame(
            &ctx,
            &mut map,
            &content,
            &opening,
            &mut selected,
            1.0,
            vec![],
        );
        assert_eq!(map.report_time, ReportTime::Latest);
        assert!(ReportTime::dates(None).is_empty());
        map.report_time = ReportTime::Published(30);
        let mut wrong = content.clone();
        wrong.countries[0].name.push_str(" changed");
        let output = frame(&ctx, &mut map, &wrong, &world, &mut selected, 1.0, vec![]);
        assert_eq!(map.report_time, ReportTime::Latest);
        assert!(
            history_labels(&output)
                .iter()
                .any(|text| text.contains("Public reports unavailable:"))
        );
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
    fn reference_layer_mouse_controls_preserve_camera_selection_and_campaign_at_two_scales() {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
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
            for label in ["Lakes", "Rivers", "Divisions"] {
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
            assert!(!map.lakes && !map.rivers && map.divisions);
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
    #[test]
    fn subdivision_lookup_changes_view_context_without_changing_reports_or_campaign_at_two_scales()
    {
        let content = Content::bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, sim_core::DaySpan::new(31)).unwrap();
        let before = sim_core::persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut map = WorldMap::new();
            map.layer = Layer::Needs;
            map.report_time = ReportTime::Published(30);
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
            let button = history_text_position(&initial, "Find division");
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                history_click(button),
            );
            let output = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![egui::Event::Text("Bavaria".into())],
            );
            assert_eq!(map.division_query, "Bavaria");
            let row = history_text_position(&output, "Bayern · Germany · DEU-1591");
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                history_click(row),
            );
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            assert_eq!(selected, CountryId("DEU".into()));
            assert!(map.divisions);
            assert_eq!(
                map.selected_division,
                Some(crate::administrative::ReferenceDivisionId(
                    "DEU-1591".into()
                ))
            );
            assert!(map.layer == Layer::Needs);
            assert_eq!(map.report_time, ReportTime::Published(30));
            let division = map
                .administrative
                .division(map.selected_division.as_ref().unwrap())
                .unwrap();
            assert_eq!(map.camera.longitude, division.focus_bounds.center().x);
            assert_eq!(map.camera.latitude, division.focus_bounds.center().y);
            let camera = map.camera;
            selected = CountryId("GBR".into());
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
            );
            assert!(map.selected_division.is_none());
            assert_eq!(map.camera, camera);
            assert_eq!(
                sim_core::persistence::encode(&world, &content).unwrap(),
                before
            );
        }
    }
    #[test]
    fn enabled_division_polygons_are_picked_by_map_pointer_at_two_scales() {
        let content = Content::bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let before = sim_core::persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut map = WorldMap::new();
            map.divisions = true;
            map.camera = Camera {
                longitude: 11.6,
                latitude: 48.2,
                zoom: 8.0,
            };
            let mut selected = CountryId("DEU".into());
            // Let the new DPI factor settle before sampling pixel coordinates.
            let _ = frame(
                &ctx,
                &mut map,
                &content,
                &world,
                &mut selected,
                scale,
                vec![],
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
            let rect = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Rect(rect) = &shape.shape {
                        (rect.fill == Color32::from_rgb(13, 28, 41)).then_some(rect.rect)
                    } else {
                        None
                    }
                })
                .unwrap();
            let point = map.camera.project(rect, [11.58, 48.14]);
            assert!(rect.contains(point));
            assert_eq!(
                map.pick(&content, rect, point),
                Some(CountryId("DEU".into()))
            );
            assert!(
                map.administrative
                    .at(&content, &selected, map.camera.unproject(rect, point))
                    .is_some()
            );
            for pressed in [true, false] {
                let _ = frame(
                    &ctx,
                    &mut map,
                    &content,
                    &world,
                    &mut selected,
                    scale,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            assert_eq!(
                map.selected_division,
                Some(crate::administrative::ReferenceDivisionId(
                    "DEU-1591".into()
                ))
            );
            assert_eq!(selected, CountryId("DEU".into()));
            assert_eq!(
                sim_core::persistence::encode(&world, &content).unwrap(),
                before
            );
        }
    }
}
