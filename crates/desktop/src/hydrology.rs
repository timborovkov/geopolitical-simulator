//! Generalized physical reference features, independent of country picking and simulation.
use crate::geometry::Polygon;
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, Ui};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub alternate_name: String,
    pub source_row: u32,
    pub scalerank: u8,
}
#[derive(Deserialize)]
struct RawLake {
    #[serde(flatten)]
    feature: Feature,
    polygons: Vec<Vec<Vec<[f32; 2]>>>,
}
#[derive(Deserialize)]
struct River {
    #[serde(flatten)]
    feature: Feature,
    parts: Vec<Vec<[f32; 2]>>,
}
#[derive(Deserialize)]
struct RawAtlas {
    lakes: Vec<RawLake>,
    rivers: Vec<River>,
}
struct Lake {
    feature: Feature,
    polygons: Vec<Polygon>,
}
#[derive(Deserialize)]
struct Manifest {
    source: String,
    revision: String,
    retrieved_at: String,
    license: String,
    license_url: String,
    limitations: String,
    applicable_dates: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterKind {
    Lake,
    River,
}
impl WaterKind {
    fn label(self) -> &'static str {
        match self {
            Self::Lake => "lake",
            Self::River => "river",
        }
    }
}
#[derive(Clone, Debug)]
pub struct ReferenceMatch {
    pub id: String,
    pub name: String,
    pub alternate_name: String,
    pub kind: WaterKind,
    /// Geographic longitude/latitude bounds, unrelated to country authority.
    pub bounds: Rect,
}
pub struct Hydrology {
    lakes: Vec<Lake>,
    rivers: Vec<River>,
    manifest: Manifest,
}
impl Hydrology {
    pub fn bundled() -> Self {
        let raw: RawAtlas = serde_json::from_str(include_str!("../../../data/hydrology-110m.json"))
            .expect("bundled physical reference geometry");
        Self {
            lakes: raw
                .lakes
                .into_iter()
                .map(|lake| Lake {
                    feature: lake.feature,
                    polygons: lake.polygons.into_iter().map(Polygon::new).collect(),
                })
                .collect(),
            rivers: raw.rivers,
            manifest: serde_json::from_str(include_str!("../../../data/hydrology-manifest.json"))
                .expect("bundled physical reference provenance"),
        }
    }
    pub fn paint(
        &self,
        painter: &Painter,
        project: impl Fn([f32; 2]) -> Pos2,
        lakes: bool,
        rivers: bool,
    ) {
        if lakes {
            let mut mesh = egui::Mesh::default();
            for lake in &self.lakes {
                for polygon in &lake.polygons {
                    let base = mesh.vertices.len() as u32;
                    for point in &polygon.points {
                        mesh.colored_vertex(project(*point), Color32::from_rgb(20, 57, 77));
                    }
                    for index in &polygon.triangles {
                        mesh.indices.push(base + *index as u32);
                    }
                }
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        if rivers {
            for river in &self.rivers {
                for part in &river.parts {
                    painter.add(egui::Shape::line(
                        part.iter().map(|p| project(*p)).collect(),
                        Stroke::new(
                            if river.feature.scalerank <= 1 {
                                1.2_f32
                            } else {
                                0.9_f32
                            },
                            Color32::from_rgb(91, 151, 181),
                        ),
                    ));
                }
            }
        }
    }
    /// Lake surfaces have precedence; rivers use a fixed screen-distance tolerance.
    pub fn hover(
        &self,
        geographic: [f32; 2],
        cursor: Pos2,
        project: impl Fn([f32; 2]) -> Pos2,
        lakes: bool,
        rivers: bool,
    ) -> Option<(&Feature, &'static str)> {
        if lakes
            && let Some(lake) = self
                .lakes
                .iter()
                .find(|l| l.polygons.iter().any(|p| p.contains(geographic)))
        {
            return Some((&lake.feature, "lake"));
        }
        if !rivers {
            return None;
        }
        self.rivers
            .iter()
            .filter_map(|river| {
                let distance = river
                    .parts
                    .iter()
                    .flat_map(|part| part.windows(2))
                    .map(|pair| segment_distance_sq(cursor, project(pair[0]), project(pair[1])))
                    .min_by(f32::total_cmp)?;
                (distance <= 5.0 * 5.0).then_some((distance, &river.feature))
            })
            .min_by(|(a, fa), (b, fb)| a.total_cmp(b).then_with(|| fa.id.cmp(&fb.id)))
            .map(|(_, feature)| (feature, "river"))
    }
    pub fn hover_text(&self, feature: &Feature, kind: &str) -> String {
        let alias = if feature.alternate_name.is_empty() {
            String::new()
        } else {
            format!("\nAlternate source label: {}", feature.alternate_name)
        };
        format!(
            "{} · reference {kind}{alias}\nNatural Earth 1:110m · {} (source row {})\nGeneralized reference; not certified 2000 water extent, resources or territorial authority. Country selection follows the separate country outlines/markers.",
            feature.name, feature.id, feature.source_row
        )
    }
    pub fn matches(&self, query: &str) -> Vec<ReferenceMatch> {
        let query = search_key(query);
        let matches = |feature: &Feature, kind: WaterKind| {
            let haystack = search_key(&format!(
                "{} {} {} {}",
                feature.name,
                feature.alternate_name,
                feature.id,
                kind.label()
            ));
            query.split_whitespace().all(|term| haystack.contains(term))
        };
        let entry = |feature: &Feature, kind, bounds| ReferenceMatch {
            id: feature.id.clone(),
            name: feature.name.clone(),
            alternate_name: feature.alternate_name.clone(),
            kind,
            bounds,
        };
        let mut result = Vec::new();
        for lake in &self.lakes {
            if matches(&lake.feature, WaterKind::Lake) {
                let bounds = bounds(lake.polygons.iter().flat_map(|p| p.points.iter().copied()));
                result.push(entry(&lake.feature, WaterKind::Lake, bounds));
            }
        }
        for river in &self.rivers {
            if matches(&river.feature, WaterKind::River) {
                let bounds = bounds(river.parts.iter().flat_map(|part| part.iter().copied()));
                result.push(entry(&river.feature, WaterKind::River, bounds));
            }
        }
        result.sort_by(|a, b| {
            search_key(&a.name)
                .cmp(&search_key(&b.name))
                .then_with(|| a.id.cmp(&b.id))
        });
        result
    }
    pub fn picker(&self, ui: &mut Ui, query: &mut String) -> Option<ReferenceMatch> {
        let mut chosen = None;
        ui.push_id("physical_feature_picker", |ui| {
            let button = ui.button("Find lake / river");
            if button.clicked() {
                query.clear();
            }
            egui::Popup::menu(&button)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .width(300.0)
                .show(|ui| {
                let mut cleared = false;
                let edit = ui.horizontal(|ui| {
                    let width = (ui.available_width() - 55.0).max(100.0);
                    let edit = ui.add(
                        egui::TextEdit::singleline(query)
                            // The picker handles Enter; retain text ownership through this frame.
                            .return_key(None)
                            .hint_text("Name, alternative label, or source ID")
                            .desired_width(width),
                    );
                    if ui.button("Clear").clicked() {
                        query.clear();
                        cleared = true;
                        edit.request_focus();
                    }
                    edit
                }).inner;
                if button.clicked() {
                    edit.request_focus();
                }
                let matches = self.matches(query);
                ui.small(format!("Matches: {}", matches.len()));
                ui.small("Choose to show its layer and focus the map. Country selection stays unchanged. Enter focuses a single match.");
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && (edit.has_focus() || edit.lost_focus());
                if enter && matches.len() == 1 {
                    chosen = matches.first().cloned();
                    ui.close();
                } else {
                    if enter {
                        edit.request_focus();
                    }
                    let mut scroll = egui::ScrollArea::vertical()
                        .id_salt("physical_feature_results")
                        .max_height(230.0);
                    if edit.changed() || cleared || button.clicked() {
                        scroll = scroll.vertical_scroll_offset(0.0);
                    }
                    scroll.show(ui, |ui| {
                        if matches.is_empty() {
                            ui.label("No matching physical reference features.");
                        }
                        for item in matches {
                            let response = ui.selectable_label(
                                false,
                                format!("{} · {}", item.name, item.id),
                            );
                            if !item.alternate_name.is_empty() {
                                response.clone().on_hover_text(format!(
                                    "Alternate source label: {}", item.alternate_name,
                                ));
                            }
                            if response.clicked() {
                                chosen = Some(item);
                                ui.close();
                            }
                        }
                    });
                }
            });
        });
        chosen
    }
    pub fn provenance(&self, ui: &mut Ui) {
        ui.collapsing("Physical map sources",|ui| {
            ui.label(format!("{} · {} lake / {} river features",self.manifest.source,self.lakes.len(),self.rivers.len()));
            ui.label(&self.manifest.applicable_dates);
            ui.label(&self.manifest.limitations);
            ui.small(format!("Retrieved {} · revision {}. River stroke sizes serve readability, not discharge or channel width.",self.manifest.retrieved_at,self.manifest.revision));
            ui.hyperlink_to(format!("Natural Earth · {}",self.manifest.license),&self.manifest.license_url);
            ui.hyperlink_to("Pinned source files",format!("https://github.com/nvkelso/natural-earth-vector/tree/{}/110m_physical",self.manifest.revision));
        });
    }
}
// Match Latin-label accents without changing the attributed display/source names.
pub(crate) fn search_key(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'ē' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'ī' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ō' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'ū' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}
fn bounds(points: impl Iterator<Item = [f32; 2]>) -> Rect {
    let mut bounds = Rect::NOTHING;
    for p in points {
        bounds.extend_with(Pos2::new(p[0], p[1]));
    }
    bounds
}
fn segment_distance_sq(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let line = b - a;
    let length = line.length_sq();
    if length <= f32::EPSILON {
        return p.distance_sq(a);
    }
    let t = ((p - a).dot(line) / length).clamp(0.0, 1.0);
    p.distance_sq(a + line * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_search_matches_aliases_accents_ids_and_separate_named_segments() {
        let h = Hydrology::bundled();
        assert_eq!(h.matches("").len(), 39);
        for (query, expected) in [
            ("PARANA", "Paraná"),
            ("vanern", "Vänern"),
            ("lake balkhash", "Balqash Köli"),
            ("RIVER-0012", "Amazon"),
        ] {
            let found = h.matches(query);
            assert_eq!(found.len(), 1, "{query}");
            assert_eq!(found[0].name, expected);
            assert!(found[0].bounds.is_finite() && found[0].bounds.is_positive());
        }
        let peace = h.matches("  river   peace ");
        assert_eq!(peace.len(), 2);
        assert_ne!(peace[0].id, peace[1].id);
        assert_ne!(peace[0].bounds, peace[1].bounds);
        assert!(h.matches("not a recorded feature").is_empty());
        assert_eq!(
            h.matches("lake balkhash")[0].name,
            "Balqash Köli",
            "search normalization must preserve source labels"
        );
    }
    #[test]
    fn water_features_preserve_names_parts_and_surface_hover_without_country_authority() {
        let h = Hydrology::bundled();
        assert_eq!((h.lakes.len(), h.rivers.len()), (25, 14));
        let project = |p: [f32; 2]| Pos2::new(p[0] * 10.0, -p[1] * 10.0);
        let superior = [-87.5, 47.7];
        let (feature, kind) = h
            .hover(superior, project(superior), project, true, true)
            .unwrap();
        assert_eq!((feature.name.as_str(), kind), ("Lake Superior", "lake"));
        assert!(h.hover_text(feature, kind).contains("source row"));
        assert!(
            h.hover(superior, project(superior), project, false, false)
                .is_none()
        );
        assert!(
            h.hover([0.0, 0.0], project([0.0, 0.0]), project, true, true)
                .is_none()
        );
        let amazon = [-55.0, -2.0];
        let (feature, kind) = h
            .hover(amazon, project(amazon), project, true, true)
            .unwrap();
        assert_eq!((feature.name.as_str(), kind), ("Amazon", "river"));
        assert!(
            h.hover(amazon, project(amazon), project, true, false)
                .is_none()
        );
        assert_eq!(
            h.rivers
                .iter()
                .filter(|r| r.feature.name == "Peace")
                .count(),
            2,
            "separate source features must not join across gaps"
        );
        for lake in &h.lakes {
            assert!(!lake.feature.name.contains("\\r"));
            for polygon in &lake.polygons {
                assert!(!polygon.triangles.is_empty());
            }
        }
        for river in &h.rivers {
            for part in &river.parts {
                assert!(
                    part.windows(2)
                        .all(|p| (p[0][0] - p[1][0]).abs() <= 180.001 && p[0] != p[1])
                );
            }
        }
    }
    #[test]
    fn screen_tolerance_uses_segments_without_jumping_gaps_or_infinite_line_extensions() {
        let a = Pos2::new(10.0, 10.0);
        let b = Pos2::new(20.0, 10.0);
        assert_eq!(segment_distance_sq(Pos2::new(15.0, 13.0), a, b), 9.0);
        assert_eq!(segment_distance_sq(Pos2::new(25.0, 10.0), a, b), 25.0);
        assert_eq!(segment_distance_sq(Pos2::new(100.0, 10.0), a, b), 6400.0);
        assert_eq!(segment_distance_sq(Pos2::new(13.0, 14.0), a, a), 25.0);
    }
}
