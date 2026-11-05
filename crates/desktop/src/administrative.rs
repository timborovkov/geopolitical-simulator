//! Attributed geographic references, independent of governing jurisdictions and accounts.
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, Ui, Vec2};
use serde::Deserialize;
use sim_core::{Content, CountryId};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ReferenceDivisionId(pub String);
#[derive(Deserialize)]
struct RawDivision {
    id: ReferenceDivisionId,
    country: CountryId,
    name: String,
    alternate_name: String,
    kind: String,
    source_row: u32,
    polygons: Vec<Vec<Vec<[f32; 2]>>>,
}
#[derive(Deserialize)]
struct RawAtlas {
    subdivisions: Vec<RawDivision>,
}
#[derive(Deserialize)]
struct Manifest {
    source: String,
    revision: String,
    retrieved_at: String,
    license: String,
    license_url: String,
    applicable_dates: String,
    limitations: String,
}
struct Footprint {
    rings: Vec<Vec<[f32; 2]>>,
    bounds: Rect,
}
impl Footprint {
    fn contains(&self, point: [f32; 2]) -> bool {
        self.bounds.contains(Pos2::new(point[0], point[1]))
            && crate::geometry::inside_ring(&self.rings[0], point)
            && !self.rings[1..]
                .iter()
                .any(|ring| crate::geometry::inside_ring(ring, point))
    }
    fn area(&self) -> f64 {
        self.rings[0]
            .windows(2)
            .map(|pair| {
                f64::from(pair[0][0]) * f64::from(pair[1][1])
                    - f64::from(pair[1][0]) * f64::from(pair[0][1])
            })
            .sum::<f64>()
            .abs()
    }
}
pub struct Division {
    pub id: ReferenceDivisionId,
    pub country: CountryId,
    pub name: String,
    pub alternate_name: String,
    pub kind: String,
    source_row: u32,
    polygons: Vec<Footprint>,
    pub focus_bounds: Rect,
}
impl Division {
    fn new(raw: RawDivision) -> Self {
        let polygons: Vec<_> = raw
            .polygons
            .into_iter()
            .map(|rings| {
                let mut bounds = Rect::NOTHING;
                for point in rings.iter().flatten() {
                    bounds.extend_with(Pos2::new(point[0], point[1]));
                }
                Footprint { rings, bounds }
            })
            .collect();
        let mut focus_bounds = Rect::NOTHING;
        for polygon in &polygons {
            focus_bounds = focus_bounds.union(polygon.bounds);
        }
        if focus_bounds.width() > 180.0 {
            focus_bounds = polygons
                .iter()
                .max_by(|a, b| a.area().total_cmp(&b.area()))
                .expect("bundled subdivision parts")
                .bounds;
        }
        Self {
            id: raw.id,
            country: raw.country,
            name: raw.name,
            alternate_name: raw.alternate_name,
            kind: raw.kind,
            source_row: raw.source_row,
            polygons,
            focus_bounds,
        }
    }
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            "Unnamed reference area"
        } else {
            &self.name
        }
    }
    pub fn description(&self, content: &Content) -> String {
        let country = content
            .country(&self.country)
            .map_or(self.country.0.as_str(), |country| country.name.as_str());
        let kind = if self.kind.is_empty() {
            "type unspecified"
        } else {
            &self.kind
        };
        let alternate = if self.alternate_name.is_empty() {
            String::new()
        } else {
            format!("\nAlternate source label: {}", self.alternate_name)
        };
        format!(
            "{} · {} · geographic reference\n{country} · {}{alternate}\nScenario jurisdictions and their powers are shown under Institutions & parties.",
            self.display_name(),
            kind,
            self.id.0
        )
    }
}
pub struct AdministrativeAtlas {
    divisions: BTreeMap<ReferenceDivisionId, Division>,
    manifest: Manifest,
}
impl AdministrativeAtlas {
    pub fn bundled() -> Arc<Self> {
        static BUNDLED: OnceLock<Arc<AdministrativeAtlas>> = OnceLock::new();
        BUNDLED
            .get_or_init(|| {
                let raw: RawAtlas = serde_json::from_str(include_str!(
                    "../../../data/administrative-references-10m.json"
                ))
                .expect("bundled subdivision references");
                Arc::new(Self {
                    divisions: raw
                        .subdivisions
                        .into_iter()
                        .map(|raw| {
                            let division = Division::new(raw);
                            (division.id.clone(), division)
                        })
                        .collect(),
                    manifest: serde_json::from_str(include_str!(
                        "../../../data/administrative-references-manifest.json"
                    ))
                    .expect("bundled subdivision provenance"),
                })
            })
            .clone()
    }
    pub fn division(&self, id: &ReferenceDivisionId) -> Option<&Division> {
        self.divisions.get(id)
    }
    pub fn coverage(&self, country: &CountryId) -> usize {
        self.divisions
            .values()
            .filter(|division| &division.country == country)
            .count()
    }
    pub fn at(&self, content: &Content, country: &CountryId, point: [f32; 2]) -> Option<&Division> {
        content.country(country)?;
        self.divisions.values().find(|division| {
            &division.country == country
                && division
                    .polygons
                    .iter()
                    .any(|polygon| polygon.contains(point))
        })
    }
    fn matches<'a>(&'a self, content: &Content, query: &str) -> Vec<&'a Division> {
        let query = crate::hydrology::search_key(query.trim());
        let mut matches: Vec<_> = self
            .divisions
            .values()
            .filter(|division| {
                let Some(country) = content.country(&division.country) else {
                    return false;
                };
                query.is_empty()
                    || crate::hydrology::search_key(&format!(
                        "{} {} {} {} {} {}",
                        division.display_name(),
                        division.alternate_name,
                        division.id.0,
                        division.kind,
                        division.country.0,
                        country.name
                    ))
                    .contains(&query)
            })
            .collect();
        matches.sort_by(|a, b| {
            a.country
                .cmp(&b.country)
                .then(a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        matches
    }
    pub fn picker(
        &self,
        ui: &mut Ui,
        content: &Content,
        query: &mut String,
    ) -> Option<ReferenceDivisionId> {
        let mut chosen = None;
        let button=ui.button("Find division").on_hover_text("Find an attributed geographic subdivision by name, alternate label, country or source ID.");
        egui::Popup::menu(&button).id(egui::Id::new("administrative_lookup_popup"))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .width(420.0).show(|ui| {
            ui.set_min_width(400.0_f32.min(ui.available_width()));
            ui.set_max_width(560.0);
            ui.label("Reference subdivisions");
            let mut cleared=false;
            let edit=ui.horizontal(|ui| {
                let edit=ui.add(egui::TextEdit::singleline(query).return_key(None).id_salt("administrative_lookup_query")
                    .desired_width(280.0).hint_text("Name, country or reference ID"));
                if ui.button("Clear").clicked() { query.clear();cleared=true;edit.request_focus(); }
                edit
            }).inner;
            if button.clicked() { edit.request_focus(); }
            let matches=self.matches(content,query);
            ui.small(format!("Matches: {}",matches.len()));
            ui.small("Choose to inspect its country and focus the map. Enter focuses a single match.");
            let enter=edit.has_focus()&&ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE,egui::Key::Enter));
            if enter&&matches.len()==1 {
                chosen=Some(matches[0].id.clone());edit.request_focus();ui.close();
            } else {
                if enter { edit.request_focus(); }
                let mut scroll=egui::ScrollArea::vertical().id_salt("administrative_lookup_results").max_height(230.0);
                if edit.changed()||cleared||button.clicked() { scroll=scroll.vertical_scroll_offset(0.0); }
                let row_height=ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Body)+2.0*ui.spacing().button_padding.y);
                ui.scope(|ui| {
                    ui.style_mut().wrap_mode=Some(egui::TextWrapMode::Truncate);
                    scroll.show_rows(ui,row_height,matches.len(),|ui,range| {
                        for index in range {
                            let division=matches[index];
                            let country=&content.country(&division.country).expect("filtered country").name;
                            let response=ui.push_id(&division.id.0,|ui| ui.selectable_label(false,
                                format!("{} · {} · {}",division.display_name(),country,division.id.0))).inner;
                            response.clone().on_hover_text(division.description(content));
                            if response.clicked() { chosen=Some(division.id.clone());ui.close(); }
                        }
                    });
                });
                if matches.is_empty() { ui.label("No matching reference subdivisions in this world."); }
            }
        });
        chosen
    }
    pub fn paint(
        &self,
        painter: &Painter,
        country: &CountryId,
        selected: Option<&ReferenceDivisionId>,
        viewport: Rect,
        project: impl Fn([f32; 2]) -> Pos2,
    ) {
        for division in self
            .divisions
            .values()
            .filter(|division| &division.country == country)
        {
            let selected = selected == Some(&division.id);
            let stroke = if selected {
                Stroke::new(1.8_f32, Color32::from_rgb(239, 204, 124))
            } else {
                Stroke::new(
                    0.65_f32,
                    Color32::from_rgba_unmultiplied(162, 184, 188, 145),
                )
            };
            for polygon in &division.polygons {
                if !polygon.bounds.intersects(viewport) {
                    continue;
                }
                for ring in &polygon.rings {
                    painter.add(egui::Shape::line(
                        ring.iter().map(|point| project(*point)).collect(),
                        stroke,
                    ));
                }
            }
            if selected && viewport.contains(division.focus_bounds.center()) {
                let center = division.focus_bounds.center();
                let center = project([center.x, center.y]);
                painter.add(egui::Shape::closed_line(
                    vec![
                        center + Vec2::new(0., -7.),
                        center + Vec2::new(7., 0.),
                        center + Vec2::new(0., 7.),
                        center + Vec2::new(-7., 0.),
                    ],
                    stroke,
                ));
                painter.text(
                    center + Vec2::new(12., -10.),
                    egui::Align2::LEFT_BOTTOM,
                    division.display_name(),
                    egui::FontId::proportional(12.),
                    stroke.color,
                );
            }
        }
    }
    pub fn provenance(&self, ui: &mut Ui) {
        ui.collapsing("Administrative map sources", |ui| {
            ui.label(format!(
                "{} · {} reference subdivisions",
                self.manifest.source,
                self.divisions.len()
            ));
            ui.label(&self.manifest.applicable_dates);
            ui.label(&self.manifest.limitations);
            ui.small(format!(
                "Retrieved {} · revision {}",
                self.manifest.retrieved_at, self.manifest.revision
            ));
            ui.hyperlink_to(
                format!("Natural Earth · {}", self.manifest.license),
                &self.manifest.license_url,
            );
            ui.hyperlink_to(
                "Pinned source files",
                format!(
                    "https://github.com/nvkelso/natural-earth-vector/tree/{}/10m_cultural",
                    self.manifest.revision
                ),
            );
            if let Some(first) = self.divisions.values().next() {
                ui.small(format!(
                    "Source identities use ADM1_CODE, such as {}; DBF row {} is provenance only.",
                    first.id.0, first.source_row
                ));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_ids_pick_mainlands_islands_holes_and_both_dateline_parts() {
        let atlas = AdministrativeAtlas::bundled();
        let content = Content::bundled().unwrap();
        for (country, point, id) in [
            ("GBR", [-0.12, 51.5], "GBR-2080"),
            ("DEU", [11.58, 48.14], "DEU-1591"),
            ("BRA", [-46.63, -23.55], "BRA-1311"),
            ("USA", [-150., 64.], "USA-3563"),
            ("RUS", [179., 66.], "RUS-2321"),
            ("RUS", [-179., 67.], "RUS-2321"),
            ("FRA", [-53., 4.], "FRA-2000"),
            ("FRA", [9., 42.], "FRA-5281"),
        ] {
            assert_eq!(
                atlas
                    .at(&content, &CountryId(country.into()), point)
                    .unwrap()
                    .id,
                ReferenceDivisionId(id.into())
            );
        }
        for (country, point) in [
            ("ZAF", [28.2, -29.5]),
            ("GBR", [-8., 53.]),
            ("RUS", [34.1, 45.1]),
            ("CHN", [121., 24.]),
        ] {
            assert!(
                atlas
                    .at(&content, &CountryId(country.into()), point)
                    .is_none()
            );
        }
    }
    #[test]
    fn lookup_preserves_attributed_names_missingness_and_registered_country_scope() {
        let atlas = AdministrativeAtlas::bundled();
        let content = Content::bundled().unwrap();
        assert_eq!(atlas.divisions.len(), 726);
        assert_eq!(
            atlas.matches(&content, "bavaria")[0].id,
            ReferenceDivisionId("DEU-1591".into())
        );
        let sao = atlas.matches(&content, "SAO PAULO");
        assert_eq!(sao.len(), 1);
        assert_eq!(sao[0].name, "São Paulo");
        assert_eq!(atlas.matches(&content, "Germany").len(), 16);
        let unknown = atlas.matches(&content, "JPN+99?");
        assert_eq!(unknown.len(), 1);
        assert!(unknown[0].name.is_empty());
        assert_eq!(unknown[0].display_name(), "Unnamed reference area");
        assert_eq!(atlas.matches(&content, "unnamed").len(), 8);
        let fictional =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        assert!(atlas.matches(&fictional, "").is_empty());
        assert!(
            atlas
                .at(&fictional, &CountryId("USA".into()), [-150., 64.])
                .is_none()
        );
    }
    #[test]
    fn shared_cache_and_main_part_focus_do_not_span_the_dateline() {
        let first = AdministrativeAtlas::bundled();
        let second = AdministrativeAtlas::bundled();
        assert!(Arc::ptr_eq(&first, &second));
        for id in ["USA-3563", "RUS-2321"] {
            let division = first.division(&ReferenceDivisionId(id.into())).unwrap();
            assert!(division.focus_bounds.width() < 180.);
            assert!(division.focus_bounds.is_finite());
        }
    }
}
