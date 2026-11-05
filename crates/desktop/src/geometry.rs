//! Reference display polygons are separate from simulation regions and authority.
use eframe::egui::{Pos2, Rect};
use sim_core::{Content, CountryId};
use std::collections::BTreeMap;

type Rings = Vec<Vec<[f32; 2]>>;
pub struct Polygon {
    pub rings: Rings,
    pub points: Vec<[f32; 2]>,
    pub triangles: Vec<usize>,
    bounds: Rect,
}
impl Polygon {
    pub(crate) fn new(rings: Rings) -> Self {
        assert!(!rings.is_empty(), "bundled polygon exterior");
        let mut points = Vec::new();
        let mut holes = Vec::new();
        for (index, ring) in rings.iter().enumerate() {
            assert!(
                ring.len() >= 4 && ring.first() == ring.last(),
                "closed source ring"
            );
            if index > 0 {
                holes.push(points.len());
            }
            points.extend_from_slice(&ring[..ring.len() - 1]);
        }
        let flat: Vec<_> = points
            .iter()
            .flat_map(|p| [f64::from(p[0]), f64::from(p[1])])
            .collect();
        let triangles = earcutr::earcut(&flat, &holes, 2).expect("bundled polygon triangulation");
        let mut bounds = Rect::NOTHING;
        for point in &points {
            bounds.extend_with(Pos2::new(point[0], point[1]));
        }
        Self {
            rings,
            points,
            triangles,
            bounds,
        }
    }
    pub(crate) fn contains(&self, point: [f32; 2]) -> bool {
        self.bounds.contains(Pos2::new(point[0], point[1]))
            && inside_ring(&self.rings[0], point)
            && !self.rings[1..].iter().any(|hole| inside_ring(hole, point))
    }
}
pub(crate) fn inside_ring(ring: &[[f32; 2]], point: [f32; 2]) -> bool {
    let [x, y] = point;
    let mut inside = false;
    for pair in ring.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}
pub struct Atlas {
    pub countries: BTreeMap<CountryId, Vec<Polygon>>,
}
impl Atlas {
    pub fn bundled() -> Self {
        let data: BTreeMap<CountryId, Vec<Rings>> =
            serde_json::from_str(include_str!("../../../data/country-outlines-110m.json"))
                .expect("bundled reference country outlines");
        Self {
            countries: data
                .into_iter()
                .map(|(id, polygons)| (id, polygons.into_iter().map(Polygon::new).collect()))
                .collect(),
        }
    }
    pub fn country_at(&self, content: &Content, point: [f32; 2]) -> Option<CountryId> {
        self.countries
            .iter()
            .find(|(id, polygons)| {
                content.country(id).is_some() && polygons.iter().any(|p| p.contains(point))
            })
            .map(|(id, _)| id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_outlines_pick_mainlands_islands_and_overseas_parts() {
        let atlas = Atlas::bundled();
        let content = Content::bundled().unwrap();
        assert_eq!(atlas.countries.len(), content.countries.len());
        for (id, point) in [
            ("GBR", [-4.0, 56.0]),
            ("GBR", [-6.7, 54.6]),
            ("FRA", [2.5, 47.0]),
            ("FRA", [-53.0, 4.0]),
            ("FRA", [9.0, 42.0]),
            ("DEU", [10.0, 51.0]),
            ("CUB", [-79.5, 22.0]),
            ("JPN", [138.0, 36.0]),
            ("USA", [-150.0, 64.0]),
            ("RUS", [179.0, 66.0]),
            ("RUS", [-179.0, 67.0]),
        ] {
            assert_eq!(
                atlas.country_at(&content, point),
                Some(CountryId(id.into())),
                "{point:?}"
            );
        }
        for point in [
            [28.2, -29.5],
            [-8.0, 53.0],
            [34.1, 45.1],
            [121.0, 24.0],
            [0.0, 0.0],
        ] {
            assert_eq!(
                atlas.country_at(&content, point),
                None,
                "hole/unrepresented land/ocean {point:?}"
            );
        }
    }
    #[test]
    fn triangles_preserve_interior_holes_and_never_cross_the_antimeridian() {
        let atlas = Atlas::bundled();
        for polygons in atlas.countries.values() {
            for polygon in polygons {
                assert!(!polygon.triangles.is_empty());
                let ring_area = |ring: &Vec<[f32; 2]>| {
                    ring.windows(2)
                        .map(|pair| {
                            f64::from(pair[0][0]) * f64::from(pair[1][1])
                                - f64::from(pair[1][0]) * f64::from(pair[0][1])
                        })
                        .sum::<f64>()
                        .abs()
                        / 2.0
                };
                let expected = ring_area(&polygon.rings[0])
                    - polygon.rings[1..].iter().map(ring_area).sum::<f64>();
                let mut actual = 0.0;
                let (triangles, remainder) = polygon.triangles.as_chunks::<3>();
                assert!(remainder.is_empty());
                for triangle in triangles {
                    let a = polygon.points[triangle[0]];
                    let b = polygon.points[triangle[1]];
                    let c = polygon.points[triangle[2]];
                    actual += ((f64::from(b[0]) - f64::from(a[0]))
                        * (f64::from(c[1]) - f64::from(a[1]))
                        - (f64::from(c[0]) - f64::from(a[0]))
                            * (f64::from(b[1]) - f64::from(a[1])))
                    .abs()
                        / 2.0;
                }
                assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
                for ring in &polygon.rings {
                    assert!(
                        ring.windows(2)
                            .all(|p| (p[0][0] - p[1][0]).abs() <= 180.001)
                    );
                }
            }
        }
    }
    #[test]
    fn custom_fictional_rosters_do_not_inherit_real_country_shapes() {
        let content =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        assert_eq!(Atlas::bundled().country_at(&content, [10.0, 51.0]), None);
    }
}
