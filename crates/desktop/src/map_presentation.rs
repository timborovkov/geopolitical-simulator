use super::*;
use crate::reference_atlas::{self, TileKey};
impl WorldMap {
    pub fn reset_campaign_view(&mut self) {
        let reference = self.reference.take();
        let tiles = std::mem::take(&mut self.tile_keys);
        let diagnostic = self.diagnostic.clone();
        let gpu = self.gpu;
        *self = Self::new();
        self.reference = reference;
        self.tile_keys = tiles;
        self.diagnostic = diagnostic;
        self.gpu = gpu;
        if self.reference.is_some() {
            self.layer = Layer::Terrain;
        }
    }
    pub fn select_country(&mut self, id: &CountryId) {
        self.reference_selection = self
            .reference
            .as_ref()
            .and_then(|a| a.countries.iter().find(|c| c.code == id.0))
            .map(|c| ReferenceSelection::Country(c.id.clone()));
    }

    pub fn install_renderer(&mut self, cc: &eframe::CreationContext<'_>) {
        match reference_atlas::Atlas::load() {
            Ok((atlas, manifest, root)) => {
                self.tile_keys = manifest.tiles.clone();
                if let Some(state) = &cc.wgpu_render_state {
                    self.diagnostic = crate::map_gpu::Renderer::install(
                        state,
                        cc.egui_ctx.clone(),
                        &atlas,
                        manifest,
                        root,
                    );
                    self.gpu = true;
                }
                self.reference = Some(atlas);
                self.layer = Layer::Terrain;
                if std::env::var_os("STATECRAFT_MAP_BENCHMARK").is_some() {
                    self.benchmark = Some(Benchmark::default());
                }
            }
            Err(error) => {
                *self.diagnostic.lock().unwrap() = Some(format!("Simplified map: {error}"))
            }
        }
    }
    /// Returns a request for the existing full country analytical workspace.
    pub fn world_screen(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        world: Option<&World>,
        selected: &mut CountryId,
    ) -> bool {
        if let Some(error) = self.diagnostic.lock().unwrap().as_ref() {
            ui.colored_label(Color32::LIGHT_YELLOW, error);
        }
        let Some(atlas) = self.reference.clone() else {
            self.show(
                ui,
                content,
                world,
                selected,
                (ui.available_height() - 120.0).max(100.0),
            );
            return false;
        };
        if let Some(benchmark) = &mut self.benchmark {
            benchmark.advance(ui.ctx(), &mut self.camera);
        }
        let projection = world.map(|w| w.project(content, Viewer::Observer));
        let reports = projection.as_ref().and_then(|p| p.as_ref().ok());
        if let Some(Err(error)) = &projection {
            ui.colored_label(
                Color32::LIGHT_RED,
                format!("Public reports unavailable: {error}"),
            );
        }
        let statistical = !matches!(self.layer, Layer::Countries | Layer::Terrain);
        let dates = ReportTime::dates(reports);
        self.report_time.normalize(&dates);
        let mut details = false;
        egui::TopBottomPanel::bottom("atlas_modes").show_inside(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                for layer in [Layer::Countries,Layer::Terrain,Layer::Approval,Layer::Needs,Layer::Debt] {
                    ui.selectable_value(&mut self.layer,layer,layer.title());
                }
                ui.separator();ui.checkbox(&mut self.cities,"Cities");
                ui.checkbox(&mut self.infrastructure,"Reference infrastructure").on_hover_text("Modern generalized roads, railways and urban areas. Not verified for 2000; no simulation effects.");
                crate::persistent_menu(ui, "Map",|ui|{
                    if ui.button("Fit world").clicked(){self.camera=Camera::default();ui.close();}
                    if ui.button(format!("Focus {}",selected.0)).clicked() && let Some(c)=content.country(selected){self.camera=Camera{longitude:c.longitude as f32,latitude:c.latitude as f32,zoom:6.0};ui.close();}
                    if ui.button("Zoom −").clicked(){self.camera.zoom=(self.camera.zoom/1.4).max(1.0);}
                    if ui.button("Zoom +").clicked(){self.camera.zoom=(self.camera.zoom*1.4).min(12.0);}
                    ui.checkbox(&mut self.lakes,"Lakes");ui.checkbox(&mut self.rivers,"Rivers");
                    ui.checkbox(&mut self.divisions,"Reference divisions");
                    let viewport=Rect::from_min_size(Pos2::ZERO,Vec2::new(1000.0,700.0));
                    if let Some(id)=self.administrative.picker(ui,content,&mut self.division_query) && let Some(d)=self.administrative.division(&id) {*selected=d.country.clone();self.camera.focus_bounds(d.focus_bounds,viewport);self.selected_division=Some(id);self.divisions=true;}
                    if let Some(water)=self.hydrology.picker(ui,&mut self.water_query){self.camera.focus_bounds(water.bounds,viewport);}

                    ui.separator();ui.label("Made with Natural Earth · public domain");
                    ui.small("Modern reference geography; historical borders and infrastructure are not verified for 2000. Gray statistical fill means unknown. Search includes countries absent at this scale.");
                });
                crate::persistent_menu(ui,"Find place",|ui|{
                    ui.add(egui::TextEdit::singleline(&mut self.reference_query).hint_text("Country or city"));
                    let query=self.reference_query.to_lowercase();
                    egui::ScrollArea::vertical().max_height(240.0).show(ui,|ui|{
                        for c in atlas.countries.iter().filter(|c|c.name.to_lowercase().contains(&query)||c.code.to_lowercase()==query).take(20){
                            if ui.button(&c.name).clicked(){self.reference_selection=Some(ReferenceSelection::Country(c.id.clone()));self.camera=Camera{longitude:c.anchor[0],latitude:c.anchor[1],zoom:6.0};let id=CountryId(c.code.clone());if content.country(&id).is_some(){*selected=id;}ui.close();}
                        }
                        if query.len()>=2{for city in atlas.cities.iter().filter(|c|c.name.to_lowercase().contains(&query)).take(30){
                            let country=atlas.countries.iter().find(|c|c.id==city.country).map_or("",|c|c.name.as_str());
                            if ui.button(format!("{} · {}",city.name,country)).clicked(){self.reference_selection=Some(ReferenceSelection::City(city.id.clone()));self.camera=Camera{longitude:city.position[0],latitude:city.position[1],zoom:12.0};self.cities=true;ui.close();}
                        }}
                    });
                });
                if statistical{self.report_time.picker(ui,&dates);}
            });
            if statistical {let (good,bad)=self.layer.bands();ui.horizontal_wrapped(|ui|{ui.colored_label(layer_color(self.layer,false,Some(if self.layer==Layer::Debt{0}else{100})),good);ui.colored_label(layer_color(self.layer,false,Some(if self.layer==Layer::Debt{100}else{0})),bad);ui.colored_label(Color32::GRAY,"Unknown / outside scenario");});}
        });
        let statistical = !matches!(self.layer, Layer::Countries | Layer::Terrain);
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(1.0)),
            Sense::click_and_drag(),
        );
        if response.dragged() {
            let delta = response.drag_delta();
            let scale = self.camera.scale(rect);
            self.camera.longitude -= delta.x / scale;
            self.camera.latitude += delta.y / scale;
            self.camera.longitude = wrap_longitude(self.camera.longitude);
            self.camera.clamp();
        }
        if response.hovered() {
            let scroll = ui.ctx().input_mut(|i| {
                let s = i.smooth_scroll_delta.y;
                i.smooth_scroll_delta = Vec2::ZERO;
                i.raw_scroll_delta = Vec2::ZERO;
                s
            });
            if scroll != 0.0 {
                self.camera.zoom_wrapped(
                    rect,
                    response.hover_pos().unwrap_or(rect.center()),
                    (scroll * 0.002).exp(),
                );
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
            self.camera.longitude = wrap_longitude(self.camera.longitude);
            self.camera.clamp();
        }
        let scale = self.camera.scale(rect);
        let lod = reference_atlas::lod(scale);
        let camera = self.camera;
        let project = |p: [f32; 2]| {
            camera.project(rect, [p[0] + longitude_shift(p[0], camera.longitude), p[1]])
        };
        let a = self.camera.unproject(rect, rect.left_bottom());
        let b = self.camera.unproject(rect, rect.right_top());
        let geographic = Rect::from_min_max(Pos2::new(a[0], a[1]), Pos2::new(b[0], b[1]));
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(24, 47, 62));
        let country_selected = match &self.reference_selection {
            Some(ReferenceSelection::Country(id)) => Some(id.clone()),
            Some(ReferenceSelection::City(id)) => atlas
                .cities
                .iter()
                .find(|c| c.id == *id)
                .map(|c| c.country.clone()),
            None => None,
        };
        let time = self.report_time.for_layer(self.layer);
        let colors = atlas
            .countries
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if statistical {
                    let col = layer_color(
                        self.layer,
                        false,
                        layer_value(
                            self.layer,
                            reports.and_then(|r| r.reports.get(&CountryId(c.code.clone()))),
                            time,
                        ),
                    );
                    [
                        col.r() as f32 / 255.0,
                        col.g() as f32 / 255.0,
                        col.b() as f32 / 255.0,
                        0.88,
                    ]
                } else {
                    let mut color = reference_atlas::political_color(i);
                    color[3] = if self.layer == Layer::Countries {
                        0.74
                    } else {
                        0.18
                    };
                    color
                }
            })
            .collect::<Vec<_>>();
        if self.gpu {
            let raster_scale = scale * ui.ctx().pixels_per_point();
            let level = ((raster_scale * 360.0 / 1024.0).log2().ceil() as i32).clamp(0, 4) as u8;
            let tiles = self
                .tile_keys
                .iter()
                .filter(|k| k.level <= level && tile_visible(**k, geographic))
                .copied()
                .collect();
            let transform = [
                2.0 * scale / rect.width(),
                2.0 * scale / rect.height(),
                -2.0 * scale * self.camera.longitude / rect.width(),
                -2.0 * scale * self.camera.latitude / rect.height(),
            ];
            painter.add(egui::Shape::Callback(
                eframe::egui_wgpu::Callback::new_paint_callback(
                    rect,
                    crate::map_gpu::Draw {
                        transform,
                        colors,
                        lod,
                        tiles,
                    },
                ),
            ));
        } else {
            for (i, c) in atlas.countries.iter().enumerate() {
                let col = colors[i];
                let color = Color32::from_rgb(
                    (col[0] * 255.0) as u8,
                    (col[1] * 255.0) as u8,
                    (col[2] * 255.0) as u8,
                );
                for m in &c.lods[lod] {
                    let mut mesh = egui::Mesh::default();
                    let shift =
                        longitude_shift((m.bounds[0] + m.bounds[2]) * 0.5, camera.longitude);
                    for p in &m.points {
                        mesh.colored_vertex(camera.project(rect, [p[0] + shift, p[1]]), color);
                    }
                    mesh.indices = m.triangles.clone();
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
        }
        for infrastructure in [false, true] {
            if infrastructure && !self.infrastructure {
                continue;
            }
            for line in &atlas.lines {
                let is_infrastructure = matches!(line.kind.as_str(), "urban" | "roads" | "rail");
                if is_infrastructure != infrastructure {
                    continue;
                }
                let visible = match line.kind.as_str() {
                    "rivers" => self.rivers && scale >= 8.0,
                    "lakes" => self.lakes,
                    "urban" | "roads" | "rail" => self.infrastructure && scale >= 24.0,
                    _ => false,
                };
                let shift =
                    longitude_shift((line.bounds[0] + line.bounds[2]) * 0.5, camera.longitude);
                if !visible || !line.visible(geographic.translate(Vec2::new(-shift, 0.0))) {
                    continue;
                }
                let (width, color): (f32, Color32) = match line.kind.as_str() {
                    "rivers" | "lakes" => (0.8, Color32::from_rgb(75, 129, 155)),
                    "rail" => (0.8, Color32::from_rgb(70, 66, 61)),
                    "urban" => (1.4, Color32::from_rgb(124, 115, 101)),
                    _ => (0.65, Color32::from_rgb(193, 174, 133)),
                };
                painter.add(egui::Shape::line(
                    line.points
                        .iter()
                        .map(|p| camera.project(rect, [p[0] + shift, p[1]]))
                        .collect(),
                    Stroke::new(width, color),
                ));
            }
        }
        // Country edges remain vectors and selected outlines are painted last.
        for c in &atlas.countries {
            if country_selected.as_ref() == Some(&c.id) {
                continue;
            }
            paint_border(
                &painter,
                c,
                lod,
                camera,
                rect,
                Stroke::new(
                    0.65_f32,
                    Color32::from_rgba_unmultiplied(216, 220, 207, 150),
                ),
            );
        }
        if self.divisions {
            self.administrative.paint(
                &painter,
                selected,
                self.selected_division.as_ref(),
                geographic,
                project,
            );
        }
        let mut occupied = Vec::<Rect>::new();
        // A selected settlement owns its label space before country labels compete.
        let selected_city = self.reference_selection.as_ref().and_then(|selection| {
            if let ReferenceSelection::City(id) = selection {
                atlas.cities.iter().find(|city| city.id == *id)
            } else {
                None
            }
        });
        let reserved_city = selected_city.filter(|_| self.cities).and_then(|city| {
            let galley = painter.layout_no_wrap(
                city.name.clone(),
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
            let bounds =
                Rect::from_min_size(project(city.position) + Vec2::new(5.0, -5.0), galley.size())
                    .expand(2.0);
            rect.contains_rect(bounds).then_some(bounds)
        });
        let mut label_countries: Vec<_> = atlas.countries.iter().collect();
        label_countries.sort_by(|a, b| {
            (country_selected.as_ref() != Some(&a.id))
                .cmp(&(country_selected.as_ref() != Some(&b.id)))
                .then_with(|| a.rank.cmp(&b.rank))
                .then_with(|| b.area.total_cmp(&a.area))
        });
        for c in label_countries {
            if c.area * scale * scale < 1800.0 && country_selected.as_ref() != Some(&c.id) {
                continue;
            }
            let p = project(c.anchor);
            if !rect.shrink(12.0).contains(p) {
                continue;
            }
            let active = country_selected.as_ref() == Some(&c.id);
            if scale > 28.0 && !active {
                continue;
            }
            let label = if statistical {
                let v = layer_value(
                    self.layer,
                    reports.and_then(|r| r.reports.get(&CountryId(c.code.clone()))),
                    time,
                );
                format!(
                    "{} · {}",
                    c.name,
                    v.map_or("Unknown".into(), |v| format!("{v}%"))
                )
            } else {
                c.name.to_uppercase()
            };
            let galley = painter.layout_no_wrap(
                label,
                egui::FontId::proportional(if active { 15.0 } else { 12.0 }),
                Color32::from_rgb(235, 232, 216),
            );
            let bounds = Rect::from_center_size(p, galley.size());
            if rect.contains_rect(bounds)
                && !reserved_city.is_some_and(|r| r.intersects(bounds))
                && !occupied.iter().any(|r| r.intersects(bounds))
            {
                painter.rect_filled(bounds.expand(3.0), 2.0, Color32::from_black_alpha(35));
                painter.galley(bounds.min, galley, Color32::WHITE);
                occupied.push(bounds.expand(4.0));
            }
        }
        let mut visible_cities = Vec::new();
        if self.cities {
            let mut cities: Vec<_> = atlas.cities.iter().collect();
            cities.sort_by_key(|city| {
                (
                    self.reference_selection.as_ref()
                        != Some(&ReferenceSelection::City(city.id.clone())),
                    !city.capital,
                    city.rank,
                )
            });
            for city in cities {
                let threshold = if selected_city.is_some_and(|selected| selected.id == city.id) {
                    0.0
                } else if city.capital {
                    5.0
                } else {
                    8.0 + f32::from(city.rank) * 3.0
                };
                if scale < threshold {
                    continue;
                }
                let p = project(city.position);
                if !rect.shrink(6.0).contains(p) {
                    continue;
                }
                let alpha = ((scale - threshold) / 3.0).clamp(0.0, 1.0);
                let color = Color32::from_rgba_unmultiplied(240, 237, 220, (alpha * 255.0) as u8);
                let galley = painter.layout_no_wrap(
                    city.name.clone(),
                    egui::FontId::proportional(11.0),
                    color,
                );
                let bounds = Rect::from_min_size(p + Vec2::new(5.0, -5.0), galley.size());
                if !rect.contains_rect(bounds)
                    || occupied.iter().any(|r| r.intersects(bounds.expand(2.0)))
                {
                    continue;
                }
                painter.circle_filled(p, if city.capital { 2.6 } else { 1.8 }, color);
                painter.galley(bounds.min, galley, color);
                occupied.push(bounds.expand(2.0));
                visible_cities.push((city, p));
            }
        }
        if let Some(id) = &country_selected
            && let Some(c) = atlas.countries.iter().find(|c| c.id == *id)
        {
            paint_border(
                &painter,
                c,
                lod,
                camera,
                rect,
                Stroke::new(1.5_f32, Color32::from_rgb(236, 212, 143)),
            );
        }
        if let Some(cursor) = response.hover_pos()
            && let Some(c) = atlas.country_at(canonical(self.camera.unproject(rect, cursor)), lod)
        {
            response.clone().on_hover_text(format!("{} · reference geography\nClick to inspect. Modern borders are not verified for 2000.",c.name));
        }
        if response.clicked() {
            response.request_focus();
            if let Some(cursor) = response.interact_pointer_pos() {
                if let Some((city, _)) = visible_cities
                    .iter()
                    .filter(|(_, p)| p.distance(cursor) < 10.0)
                    .min_by(|(_, a), (_, b)| {
                        a.distance_sq(cursor).total_cmp(&b.distance_sq(cursor))
                    })
                {
                    self.reference_selection = Some(ReferenceSelection::City(city.id.clone()));
                } else if let Some(c) =
                    atlas.country_at(canonical(self.camera.unproject(rect, cursor)), lod)
                {
                    self.reference_selection = Some(ReferenceSelection::Country(c.id.clone()));
                    let id = CountryId(c.code.clone());
                    if content.country(&id).is_some() {
                        *selected = id;
                    }
                } else {
                    self.reference_selection = None;
                }
            }
        }
        painter.text(
            rect.left_bottom() + Vec2::new(10.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            "Natural Earth reference atlas · Drag to pan · Scroll to zoom",
            egui::FontId::proportional(10.0),
            Color32::from_rgb(190, 202, 204),
        );
        if let Some(selection) = &self.reference_selection {
            let city = if let ReferenceSelection::City(id) = selection {
                atlas.cities.iter().find(|c| c.id == *id)
            } else {
                None
            };
            let country = match selection {
                ReferenceSelection::Country(id) => atlas.countries.iter().find(|c| c.id == *id),
                ReferenceSelection::City(_) => {
                    city.and_then(|city| atlas.countries.iter().find(|c| c.id == city.country))
                }
            };
            let mut open = true;
            egui::Window::new("Map inspection").id(egui::Id::new("atlas_inspector")).open(&mut open).default_pos(rect.left_bottom()+Vec2::new(12.0,-260.0)).default_width(280.0_f32.min(rect.width()-24.0)).resizable(false).collapsible(false).constrain_to(rect).show(ui.ctx(),|ui|{
                egui::ScrollArea::vertical().max_height((rect.height()-80.0).max(60.0)).show(ui,|ui|{
                    if let Some(city)=city{ui.strong(&city.name);ui.small(if city.capital{"Reference capital"}else{"Reference settlement"});}
                    if let Some(c)=country{
                        ui.strong(&c.name);let id=CountryId(c.code.clone());
                        if let Some(d)=content.country(&id){ui.label(&d.capital);if let Some(report)=reports.and_then(|r|r.reports.get(&id)){ui.label(format!("{:?}",report.regime));if let Some(release)=time.release(report){ui.label(format!("Approval {}% · needs {}%",release.approval,release.needs));ui.small(format!("Published {}",sim_core::date(release.day)));}else{ui.small("No statistical release yet");}}
                            if ui.button("Country details →").clicked(){*selected=id;details=true;}
                        } else {ui.label("Reference only · outside this scenario");}
                    }
                    ui.separator();ui.small("Natural Earth · modern reference geography. Not verified for 2000; settlements and infrastructure have no simulation effects.");
                });
            });
            if !open {
                self.reference_selection = None;
            }
        }
        details
    }
}
impl Camera {
    fn zoom_wrapped(&mut self, rect: Rect, cursor: Pos2, factor: f32) {
        let anchor = self.unproject(rect, cursor);
        self.zoom = (self.zoom * factor).clamp(1.0, 12.0);
        let relative = cursor - rect.center();
        let scale = self.scale(rect);
        self.longitude = wrap_longitude(anchor[0] - relative.x / scale);
        self.latitude = (anchor[1] + relative.y / scale).clamp(-90.0, 90.0);
    }
}
fn wrap_longitude(lon: f32) -> f32 {
    (lon + 180.0).rem_euclid(360.0) - 180.0
}
fn canonical(p: [f32; 2]) -> [f32; 2] {
    [wrap_longitude(p[0]), p[1]]
}
fn longitude_shift(lon: f32, center: f32) -> f32 {
    ((center - lon) / 360.0).round() * 360.0
}
fn tile_visible(key: TileKey, view: Rect) -> bool {
    let [x0, y0, x1, y1] = key.bounds();
    let bounds = Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1));
    [-360.0, 0.0, 360.0]
        .into_iter()
        .any(|shift| bounds.translate(Vec2::new(shift, 0.0)).intersects(view))
}
fn paint_border(
    painter: &egui::Painter,
    c: &reference_atlas::Country,
    lod: usize,
    camera: Camera,
    rect: Rect,
    stroke: Stroke,
) {
    for mesh in &c.lods[lod] {
        let shift = longitude_shift((mesh.bounds[0] + mesh.bounds[2]) * 0.5, camera.longitude);
        let project = |p: [f32; 2]| camera.project(rect, [p[0] + shift, p[1]]);
        let bounds = Rect::from_two_pos(
            project([mesh.bounds[0], mesh.bounds[1]]),
            project([mesh.bounds[2], mesh.bounds[3]]),
        );
        if !painter.clip_rect().intersects(bounds) {
            continue;
        }
        for ring in &mesh.rings {
            painter.add(egui::Shape::line(
                ring.iter().map(|p| project(*p)).collect(),
                stroke,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pacific_wrap_keeps_cursor_anchor_and_neighbour_tiles_visible() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0));
        let mut camera = Camera {
            longitude: 179.0,
            latitude: -18.0,
            zoom: 6.0,
        };
        let cursor = Pos2::new(900.0, 400.0);
        let before = canonical(camera.unproject(rect, cursor));
        camera.zoom_wrapped(rect, cursor, 1.4);
        let after = canonical(camera.unproject(rect, cursor));
        assert!((before[0] - after[0]).abs() < 0.0001 && (before[1] - after[1]).abs() < 0.0001);
        let view = Rect::from_min_max(Pos2::new(170.0, -30.0), Pos2::new(190.0, 0.0));
        assert!(tile_visible(
            TileKey {
                level: 4,
                x: 0,
                y: 9
            },
            view
        ));
        assert!(tile_visible(
            TileKey {
                level: 4,
                x: 31,
                y: 9
            },
            view
        ));
        assert_eq!(wrap_longitude(181.0), -179.0);
    }

    #[test]
    fn compact_map_controls_accept_clicks_and_reference_visits_navigate() {
        let (atlas, _, _) = reference_atlas::Atlas::load().unwrap();
        let content = Content::bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let before = sim_core::persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut map = WorldMap::new();
            map.reference = Some(atlas.clone());
            map.layer = Layer::Terrain;
            let mut selected = world.player.clone();
            let mut render = |map: &mut WorldMap, events| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(1280.0 / scale, 800.0 / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            map.world_screen(ui, &content, Some(&world), &mut selected);
                        });
                    },
                )
            };
            for label in ["Approval", "Terrain", "Cities", "Reference infrastructure"] {
                render(&mut map, vec![]);
                let output = render(&mut map, vec![]);
                let point = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(t) if t.galley.text() == label => {
                            Some(t.pos + t.galley.size() * 0.5)
                        }
                        _ => None,
                    })
                    .expect("Map control is visible");
                for pressed in [true, false] {
                    render(
                        &mut map,
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
                match label {
                    "Approval" => assert!(map.layer == Layer::Approval),
                    "Terrain" => assert!(map.layer == Layer::Terrain),
                    "Cities" => assert!(!map.cities),
                    _ => assert!(map.infrastructure),
                }
            }
            let mut history = crate::navigation::History::default();
            let visit = |map: &WorldMap| crate::navigation::View {
                tab: crate::Tab::Overview,
                country: world.player.clone(),
                policy: "energy".into(),
                value: 90,
                reference: false,
                map: map.view(),
            };
            map.reference_selection =
                Some(ReferenceSelection::Country(atlas.countries[0].id.clone()));
            let first = visit(&map);
            history.observe(first.clone());
            map.reference_selection = Some(ReferenceSelection::City(atlas.cities[0].id.clone()));
            let second = visit(&map);
            history.observe(second.clone());
            assert!(history.travel(second, false).unwrap().map == first.map);
            assert!(history.can_forward());
        }
        assert_eq!(
            sim_core::persistence::encode(&world, &content).unwrap(),
            before
        );
    }

    #[test]
    fn reference_inspection_layers_and_history_preserve_campaign_bytes() {
        let (atlas, _, _) = reference_atlas::Atlas::load().unwrap();
        let content = Content::bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let before = sim_core::persistence::encode(&world, &content).unwrap();
        let city = atlas
            .cities
            .iter()
            .find(|c| c.name == "London")
            .unwrap()
            .id
            .clone();
        let italy = atlas
            .countries
            .iter()
            .find(|c| c.code == "ITA")
            .unwrap()
            .id
            .clone();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut map = WorldMap::new();
            map.reference = Some(atlas.clone());
            map.camera = Camera {
                longitude: 8.0,
                latitude: 48.0,
                zoom: 6.0,
            };
            let mut selected = world.player.clone();
            for selection in [
                ReferenceSelection::Country(italy.clone()),
                ReferenceSelection::City(city.clone()),
            ] {
                map.reference_selection = Some(selection.clone());
                map.infrastructure = true;
                map.cities = true;
                for layer in [
                    Layer::Terrain,
                    Layer::Countries,
                    Layer::Approval,
                    Layer::Needs,
                    Layer::Debt,
                ] {
                    map.layer = layer;
                    let view = map.view();
                    let output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                Pos2::ZERO,
                                Vec2::new(1280.0 / scale, 800.0 / scale),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                assert!(!map.world_screen(
                                    ui,
                                    &content,
                                    Some(&world),
                                    &mut selected
                                ));
                            });
                        },
                    );
                    assert!(!output.shapes.is_empty());
                    map.cities = false;
                    map.infrastructure = false;
                    map.reference_selection = None;
                    map.restore_view(view.clone());
                    assert!(map.view() == view);
                    assert_eq!(selected, world.player);
                }
            }
        }
        let mut map = WorldMap::new();
        map.reference = Some(atlas.clone());
        map.infrastructure = true;
        map.reset_campaign_view();
        assert!(std::sync::Arc::ptr_eq(
            map.reference.as_ref().unwrap(),
            &atlas
        ));
        assert!(!map.infrastructure);
        assert_eq!(
            sim_core::persistence::encode(&world, &content).unwrap(),
            before
        );
    }
}

/// Opt-in repeatable presentation-only route. Timings include frame scheduling;
/// software Vulkan measurements are not a hardware 60 FPS certification.
#[derive(Default)]
pub(super) struct Benchmark {
    frame: u32,
    last: Option<std::time::Instant>,
    samples: Vec<f64>,
}
impl Benchmark {
    fn advance(&mut self, ctx: &egui::Context, camera: &mut Camera) {
        if self.frame >= 420 {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(last) = self.last
            && self.frame >= 60
        {
            self.samples
                .push(now.duration_since(last).as_secs_f64() * 1000.0);
        }
        self.last = Some(now);
        let route = self.frame.saturating_sub(60);
        let scene = (route / 90).min(3) as usize;
        let fraction = (route % 90) as f32 / 90.0;
        let (lon, lat) = [(8.0, 48.0), (116.0, 34.0), (-98.0, 38.0), (178.0, -18.0)][scene];
        *camera = Camera {
            longitude: (lon + 2.0 * fraction).min(180.0),
            latitude: lat,
            zoom: 3.0 + fraction * 3.0,
        };
        self.frame += 1;
        if self.frame == 420 {
            self.samples.sort_by(f64::total_cmp);
            eprintln!(
                "MAP_BENCHMARK frames={} p50_ms={:.2} p95_ms={:.2} max_ms={:.2}",
                self.samples.len(),
                self.samples[self.samples.len() / 2],
                self.samples[self.samples.len() * 95 / 100],
                self.samples.last().unwrap()
            );
        } else {
            ctx.request_repaint();
        }
    }
}
