//! Offline display geography. These identities never grant simulation authority.
use eframe::egui::{Pos2, Rect};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct FeatureId(pub String);
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SettlementId(pub String);
#[derive(Deserialize)]
pub struct Mesh {
    pub bounds: [f32; 4],
    pub rings: Vec<Vec<[f32; 2]>>,
    pub points: Vec<[f32; 2]>,
    pub triangles: Vec<u32>,
}
impl Mesh {
    pub fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.bounds[0]
            && p[0] <= self.bounds[2]
            && p[1] >= self.bounds[1]
            && p[1] <= self.bounds[3]
            && crate::geometry::inside_ring(&self.rings[0], p)
            && !self.rings[1..]
                .iter()
                .any(|r| crate::geometry::inside_ring(r, p))
    }
}
#[derive(Deserialize)]
pub struct Country {
    pub id: FeatureId,
    pub code: String,
    pub name: String,
    pub anchor: [f32; 2],
    pub area: f32,
    pub rank: u8,
    pub lods: Vec<Vec<Mesh>>,
}
#[derive(Deserialize)]
pub struct City {
    pub id: SettlementId,
    pub name: String,
    pub country: FeatureId,
    pub position: [f32; 2],
    pub rank: u8,
    pub capital: bool,
}
#[derive(Deserialize)]
pub struct Line {
    pub kind: String,
    pub points: Vec<[f32; 2]>,
    pub bounds: [f32; 4],
}
impl Line {
    pub fn visible(&self, viewport: Rect) -> bool {
        Rect::from_min_max(
            Pos2::new(self.bounds[0], self.bounds[1]),
            Pos2::new(self.bounds[2], self.bounds[3]),
        )
        .intersects(viewport)
    }
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TileKey {
    pub level: u8,
    pub x: u32,
    pub y: u32,
}
impl TileKey {
    pub fn file(self) -> String {
        format!("{}-{}-{}.jpg", self.level, self.x, self.y)
    }
    pub fn bounds(self) -> [f32; 4] {
        let span = 180.0 / (1u32 << self.level) as f32;
        [
            -180.0 + self.x as f32 * span,
            90.0 - (self.y + 1) as f32 * span,
            -180.0 + (self.x + 1) as f32 * span,
            90.0 - self.y as f32 * span,
        ]
    }
}
#[derive(Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub files: BTreeMap<String, String>,
    pub tiles: Vec<TileKey>,
}
impl Manifest {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported map asset version".into());
        }
        let mut keys = BTreeSet::new();
        for tile in &self.tiles {
            if tile.level > 4
                || tile.x >= (2 << tile.level)
                || tile.y >= (1 << tile.level)
                || !keys.insert(*tile)
                || !self.files.contains_key(&tile.file())
            {
                return Err("Invalid terrain tile manifest".into());
            }
        }
        if keys.len() != 682 {
            return Err("Incomplete terrain tile pyramid".into());
        }
        Ok(())
    }
}
#[derive(Deserialize)]
pub struct Atlas {
    pub countries: Vec<Country>,
    pub cities: Vec<City>,
    pub lines: Vec<Line>,
}
impl Atlas {
    pub fn load() -> Result<(Arc<Self>, Arc<Manifest>, PathBuf), String> {
        let root = std::env::var_os("STATECRAFT_MAP_ASSETS")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let beside = std::env::current_exe()
                    .unwrap_or_default()
                    .with_file_name("map-atlas");
                if beside.is_dir() {
                    beside
                } else {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/map-atlas")
                }
            });
        Self::load_from(root)
    }
    fn load_from(root: PathBuf) -> Result<(Arc<Self>, Arc<Manifest>, PathBuf), String> {
        let manifest: Manifest = serde_json::from_slice(
            &std::fs::read(root.join("manifest.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        manifest.validate()?;
        // These two images guarantee a complete coarse fallback while detail streams.
        // Without them, use the established simplified map instead of an empty terrain.
        for x in 0..2 {
            checked_read(&root, &TileKey { level: 0, x, y: 0 }.file(), &manifest)?;
        }
        let bytes = checked_read(&root, "atlas.json", &manifest)?;
        let atlas: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if atlas.countries.is_empty() || atlas.countries.len() > 512 {
            return Err("Unsupported atlas country count".into());
        }
        for c in &atlas.countries {
            if c.lods.len() != 3 {
                return Err("Invalid map detail levels".into());
            }
            for m in c.lods.iter().flatten() {
                if m.rings.is_empty()
                    || m.rings.iter().any(|r| r.len() < 4 || r.first() != r.last())
                    || m.triangles.len() % 3 != 0
                    || m.triangles.iter().any(|i| *i as usize >= m.points.len())
                    || m.points.iter().any(|p| {
                        !p[0].is_finite()
                            || !p[1].is_finite()
                            || p[0].abs() > 180.001
                            || p[1].abs() > 90.001
                    })
                {
                    return Err("Invalid country mesh".into());
                }
            }
        }
        Ok((Arc::new(atlas), Arc::new(manifest), root))
    }
    pub fn country_at(&self, p: [f32; 2], lod: usize) -> Option<&Country> {
        self.countries
            .iter()
            .find(|c| c.lods[lod].iter().any(|m| m.contains(p)))
    }
}
pub fn checked_read(root: &Path, name: &str, manifest: &Manifest) -> Result<Vec<u8>, String> {
    if Path::new(name).components().count() != 1 {
        return Err("Invalid map asset path".into());
    }
    let expected = manifest
        .files
        .get(name)
        .ok_or("Map asset missing from manifest")?;
    let bytes = std::fs::read(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
    if format!("{:x}", Sha256::digest(&bytes)) != *expected {
        return Err(format!("Map checksum mismatch: {name}"));
    }
    Ok(bytes)
}
pub fn lod(scale: f32) -> usize {
    if scale < 8.0 {
        0
    } else if scale < 24.0 {
        1
    } else {
        2
    }
}
pub fn political_color(index: usize) -> [f32; 4] {
    const COLORS: [[f32; 3]; 8] = [
        [0.54, 0.64, 0.47],
        [0.60, 0.54, 0.64],
        [0.72, 0.59, 0.44],
        [0.46, 0.60, 0.67],
        [0.67, 0.48, 0.45],
        [0.62, 0.65, 0.53],
        [0.51, 0.56, 0.67],
        [0.65, 0.57, 0.52],
    ];
    let c = COLORS[(index * 5) % COLORS.len()];
    [c[0], c[1], c[2], 0.22]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn global_reference_picking_preserves_scenario_boundary_and_islands() {
        let (atlas, manifest, _) = Atlas::load().unwrap();
        assert_eq!(atlas.countries.len(), 258);
        assert_eq!(atlas.cities.len(), 7325);
        assert_eq!(manifest.tiles.len(), 682);
        manifest.validate().unwrap();
        for level in 0..3 {
            for (point, code) in [
                ([2.35, 48.85], "FRA"),
                ([139.7, 35.7], "JPN"),
                ([-100.0, 40.0], "USA"),
                ([12.5, 41.9], "ITA"),
                ([178.4, -18.1], "FJI"),
            ] {
                assert_eq!(
                    atlas
                        .country_at(point, level)
                        .unwrap_or_else(|| panic!("missing {point:?} at LOD {level}"))
                        .code,
                    code
                );
            }
            assert!(atlas.country_at([-30.0, 30.0], level).is_none());
        }
        let content = sim_core::Content::bundled().unwrap();
        let italy = atlas.country_at([12.5, 41.9], 2).unwrap();
        assert!(
            content
                .country(&sim_core::CountryId(italy.code.clone()))
                .is_none()
        );
        let london = atlas.cities.iter().find(|c| c.name == "London").unwrap();
        assert!(london.capital);
        assert_eq!(
            atlas
                .countries
                .iter()
                .find(|c| c.id == london.country)
                .unwrap()
                .code,
            "GBR"
        );
    }
    #[test]
    fn missing_corrupt_assets_and_path_traversal_are_errors() {
        let root = std::env::temp_dir().join(format!("atlas-integrity-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut manifest = Manifest {
            version: 1,
            files: BTreeMap::new(),
            tiles: vec![],
        };
        manifest
            .files
            .insert("test".into(), format!("{:x}", Sha256::digest(b"expected")));
        assert!(checked_read(&root, "test", &manifest).is_err());
        std::fs::write(root.join("test"), b"corrupt").unwrap();
        assert!(
            checked_read(&root, "test", &manifest)
                .unwrap_err()
                .contains("checksum")
        );
        assert!(checked_read(&root, "../test", &manifest).is_err());
        std::fs::write(root.join("test"), b"expected").unwrap();
        assert_eq!(checked_read(&root, "test", &manifest).unwrap(), b"expected");
        assert!(manifest.validate().is_err());
        manifest.tiles.push(TileKey {
            level: 255,
            x: 0,
            y: 0,
        });
        assert!(manifest.validate().is_err());
        let atlas_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/map-atlas");
        std::fs::copy(atlas_root.join("manifest.json"), root.join("manifest.json")).unwrap();
        assert!(
            Atlas::load_from(root.clone())
                .err()
                .unwrap()
                .contains("0-0-0.jpg")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn tile_pyramid_covers_globe_with_exact_shared_edges() {
        for level in 0..5 {
            let n = 1 << level;
            assert_eq!(TileKey { level, x: 0, y: 0 }.bounds()[0], -180.0);
            assert_eq!(
                TileKey {
                    level,
                    x: 2 * n - 1,
                    y: n - 1
                }
                .bounds()[2..],
                [180.0, -90.0 + 180.0 / n as f32]
            );
            for x in 0..2 * n - 1 {
                assert_eq!(
                    TileKey { level, x, y: 0 }.bounds()[2],
                    TileKey {
                        level,
                        x: x + 1,
                        y: 0
                    }
                    .bounds()[0]
                );
            }
        }
    }
}
