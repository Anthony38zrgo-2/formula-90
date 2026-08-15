//! Versioned `TrackDocument -> BuildIR` compiler.
//!
//! BuildIR is the explicit, sampled, destructive, compiler-facing
//! representation that Blender materializes without reinterpreting the
//! semantic document. It is byte-deterministic for the same input and is never
//! edited by hand.

pub mod build_graph;

use serde::Serialize;
use sha2::{Digest, Sha256};
use track_assets::AssetRegistry;
use track_domain::{ProfilePoint, TrackDocument};
use track_geometry::{
    closed_polyline_length, resample_closed_polyline, sample_closed_centerline, Point2,
};

pub const BUILD_IR_VERSION: u32 = 1;
pub const COMPILER_VERSION: &str = "track-build-0.1.0";
pub const ROAD_SAMPLE_SPACING_M: f64 = 2.0;
pub const TERRAIN_CELL_M: f64 = 6.0;
const TERRAIN_MARGIN_M: f64 = 20.0;
const DEFAULT_VEGETATION_SPACING_M: f64 = 4.0;
const DEFAULT_HALF_WIDTH_M: f64 = 6.0;

#[derive(Debug)]
pub enum BuildError {
    Serialization(serde_json::Error),
    Geometry(track_geometry::GeometryError),
    NoCenterline,
    Process(String),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialization(error) => write!(formatter, "serialization: {error}"),
            Self::Geometry(error) => write!(formatter, "geometry: {error}"),
            Self::NoCenterline => write!(formatter, "document has no valid centerline"),
            Self::Process(message) => write!(formatter, "process: {message}"),
        }
    }
}

impl std::error::Error for BuildError {}

impl From<serde_json::Error> for BuildError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

impl From<track_geometry::GeometryError> for BuildError {
    fn from(error: track_geometry::GeometryError) -> Self {
        Self::Geometry(error)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RoadSample {
    pub station: f64,
    pub position: [f64; 2],
    pub tangent: [f64; 2],
    pub normal: [f64; 2],
    pub width_left: f64,
    pub width_right: f64,
    pub elevation: f64,
    pub bank: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExplicitAssetInstance {
    pub instance_id: String,
    pub asset_id: String,
    pub position: [f64; 2],
    pub yaw_rad: f64,
    pub scale: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// One deterministic terrain grid cell. `height` is the semantic elevation at
/// that cell center; the Blender backend creates the heightfield mesh from this.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TerrainCell {
    pub x: f64,
    pub z: f64,
    pub height: f64,
}

/// An explicit vegetation placement produced deterministically from a region.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VegetationInstance {
    pub instance_id: String,
    pub asset_id: String,
    pub position: [f64; 2],
    pub yaw_rad: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BuildIR {
    pub build_ir_version: u32,
    pub compiler_version: String,
    pub source_project_hash: String,
    pub geometry_hash: String,
    pub terrain_hash: String,
    pub vegetation_hash: String,
    pub objects_hash: String,
    pub gameplay_hash: String,
    pub road_samples: Vec<RoadSample>,
    pub asset_instances: Vec<ExplicitAssetInstance>,
    pub terrain_cell_m: f64,
    pub terrain_heightfield: Vec<TerrainCell>,
    pub vegetation_instances: Vec<VegetationInstance>,
}

impl BuildIR {
    pub fn to_bytes(&self) -> Result<Vec<u8>, BuildError> {
        Ok(serde_json::to_vec(self)?)
    }
}

/// List of missing materialization prerequisites. A Blender backend must be
/// able to build the track from this IR alone, without reinterpreting SVG or
/// re-evaluating the spline. Road samples must exist and be finite with
/// positive half widths; asset instances may be empty when the track has none.
pub fn materialization_prerequisites(build_ir: &BuildIR) -> Vec<String> {
    let mut missing = Vec::new();
    if build_ir.road_samples.is_empty() {
        missing.push("road_samples are empty".to_owned());
    }
    for (index, sample) in build_ir.road_samples.iter().enumerate() {
        if !sample.position.iter().all(|v| v.is_finite())
            || !sample.tangent.iter().all(|v| v.is_finite())
            || !sample.normal.iter().all(|v| v.is_finite())
        {
            missing.push(format!("road sample {index} has non-finite geometry"));
        }
        if sample.width_left <= 0.0 || sample.width_right <= 0.0 {
            missing.push(format!("road sample {index} has non-positive half width"));
        }
    }
    missing
}

pub fn materialization_ready(build_ir: &BuildIR) -> bool {
    materialization_prerequisites(build_ir).is_empty()
}

/// Compile and write `track.build.json` to disk. The file is the explicit,
/// self-contained contract consumed by the Blender backend.
pub fn write_track_build_json(
    document: &TrackDocument,
    registry: Option<&AssetRegistry>,
    path: impl AsRef<std::path::Path>,
) -> Result<(), BuildError> {
    let build_ir = compile(document, registry)?;
    std::fs::write(path, build_ir.to_bytes()?)
        .map_err(|error| BuildError::Serialization(serde_json::Error::io(error)))
}

/// Compile a `TrackDocument` into a deterministic BuildIR.
pub fn compile(
    document: &TrackDocument,
    registry: Option<&AssetRegistry>,
) -> Result<BuildIR, BuildError> {
    if document.centerline.len() < 3 {
        return Err(BuildError::NoCenterline);
    }

    let source_project_hash = sha256_bytes(&document.canonical_json()?);
    let points: Vec<Point2> = document
        .centerline
        .iter()
        .map(|point| Point2 {
            x: point.x,
            z: point.z,
        })
        .collect();
    let total_length = closed_polyline_length(&points)?;
    let resampled = resample_closed_polyline(&points, ROAD_SAMPLE_SPACING_M)?;
    let controls = document.centerline.clone();

    let mut road_samples = Vec::with_capacity(resampled.len());
    for (index, position) in resampled.iter().enumerate() {
        let station = total_length * index as f64 / resampled.len() as f64;
        let sample = sample_closed_centerline(&controls, station)?;
        let tangent = sample.tangent;
        let normal = Point2 {
            x: -tangent.z,
            z: tangent.x,
        };
        road_samples.push(RoadSample {
            station: round6(station),
            position: [round6(position.x), round6(position.z)],
            tangent: [round6(tangent.x), round6(tangent.z)],
            normal: [round6(normal.x), round6(normal.z)],
            width_left: round6(eval_profile(
                &document.profiles.width_left,
                station,
                DEFAULT_HALF_WIDTH_M,
            )),
            width_right: round6(eval_profile(
                &document.profiles.width_right,
                station,
                DEFAULT_HALF_WIDTH_M,
            )),
            elevation: round6(eval_profile(&document.profiles.elevation, station, 0.0)),
            bank: round6(eval_profile(&document.profiles.banking, station, 0.0)),
        });
    }

    let asset_instances = document
        .asset_instances
        .iter()
        .map(|instance| {
            let spec = registry.and_then(|registry| registry.lookup(&instance.asset_id));
            ExplicitAssetInstance {
                instance_id: instance.id.clone(),
                asset_id: instance.asset_id.clone(),
                position: [round6(instance.x), round6(instance.z)],
                yaw_rad: round6(instance.rotation[1]),
                scale: round6(instance.scale[0]),
                kind: spec.map(|spec| spec.kind.clone()),
                source: spec.and_then(|spec| spec.source.clone()),
            }
        })
        .collect();

    let terrain_heightfield = sample_terrain_heightfield(&road_samples, TERRAIN_CELL_M);
    let vegetation_instances = expand_vegetation(&document.vegetation_regions, &road_samples);

    Ok(BuildIR {
        build_ir_version: BUILD_IR_VERSION,
        compiler_version: COMPILER_VERSION.into(),
        source_project_hash,
        geometry_hash: subsystem_hash(&document.centerline)?,
        terrain_hash: subsystem_hash(&terrain_heightfield)?,
        vegetation_hash: subsystem_hash(&vegetation_instances)?,
        objects_hash: subsystem_hash(&document.asset_instances)?,
        gameplay_hash: subsystem_hash(&document.gameplay.markers)?,
        road_samples,
        asset_instances,
        terrain_cell_m: round6(TERRAIN_CELL_M),
        terrain_heightfield,
        vegetation_instances,
    })
}

/// Deterministic regular heightfield over the road bounds. Each cell's height
/// is the elevation of the nearest road sample, so the terrain matches the road
/// surface near the track and falls back to the sampled elevation elsewhere.
fn sample_terrain_heightfield(road_samples: &[RoadSample], cell_m: f64) -> Vec<TerrainCell> {
    if road_samples.is_empty() {
        return Vec::new();
    }
    let mut min_x = f64::INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for sample in road_samples {
        min_x = min_x.min(sample.position[0]);
        min_z = min_z.min(sample.position[1]);
        max_x = max_x.max(sample.position[0]);
        max_z = max_z.max(sample.position[1]);
    }
    min_x -= TERRAIN_MARGIN_M;
    min_z -= TERRAIN_MARGIN_M;
    max_x += TERRAIN_MARGIN_M;
    max_z += TERRAIN_MARGIN_M;

    let mut cells = Vec::new();
    let mut z = min_z;
    while z < max_z {
        let mut x = min_x;
        while x < max_x {
            let height = nearest_road_elevation(road_samples, x, z);
            cells.push(TerrainCell {
                x: round6(x),
                z: round6(z),
                height: round6(height),
            });
            x += cell_m;
        }
        z += cell_m;
    }
    cells
}

fn nearest_road_elevation(road_samples: &[RoadSample], x: f64, z: f64) -> f64 {
    let mut best = f64::INFINITY;
    let mut height = 0.0;
    for sample in road_samples {
        let dx = sample.position[0] - x;
        let dz = sample.position[1] - z;
        let distance = dx.hypot(dz);
        if distance < best {
            best = distance;
            height = sample.elevation;
        }
    }
    height
}

/// Expand vegetation regions into explicit deterministic instances using a
/// seeded jittered grid. Points outside the region polygon or inside the road
/// exclusion band are discarded.
fn expand_vegetation(
    regions: &[track_domain::VegetationRegion],
    road_samples: &[RoadSample],
) -> Vec<VegetationInstance> {
    let mut instances = Vec::new();
    for region in regions {
        let mut random = SeededRandom::new(region.seed);
        let polygon = &region.polygon;
        let spacing = if region.min_spacing > 0.0 {
            region.min_spacing
        } else {
            DEFAULT_VEGETATION_SPACING_M
        };
        let (min_x, min_z, max_x, max_z) = polygon_bounds(polygon);
        let mut index = 0usize;
        let mut z = min_z;
        while z < max_z {
            let mut x = min_x;
            while x < max_x {
                let jitter_x = (random.next_f64() - 0.5) * spacing;
                let jitter_z = (random.next_f64() - 0.5) * spacing;
                let px = x + jitter_x;
                let pz = z + jitter_z;
                if point_in_polygon(px, pz, polygon)
                    && nearest_road_distance(road_samples, px, pz) >= region.track_exclusion
                {
                    let asset_id = region
                        .asset_set
                        .get((random.next_u64() as usize) % region.asset_set.len())
                        .cloned()
                        .unwrap_or_default();
                    let scale = lerp_range(region.scale_range, random.next_f64());
                    let yaw = lerp_range(region.rotation_range, random.next_f64());
                    instances.push(VegetationInstance {
                        instance_id: format!("{}_{index}", region.id),
                        asset_id,
                        position: [round6(px), round6(pz)],
                        yaw_rad: round6(yaw),
                        scale: round6(scale),
                    });
                    index += 1;
                }
                x += spacing;
            }
            z += spacing;
        }
    }
    instances
}

fn polygon_bounds(polygon: &[track_domain::Point2]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for point in polygon {
        min_x = min_x.min(point.x);
        min_z = min_z.min(point.z);
        max_x = max_x.max(point.x);
        max_z = max_z.max(point.z);
    }
    (min_x, min_z, max_x, max_z)
}

fn point_in_polygon(x: f64, z: f64, polygon: &[track_domain::Point2]) -> bool {
    let mut inside = false;
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let a = &polygon[i];
        let b = &polygon[(i + 1) % n];
        if (a.z > z) != (b.z > z) && x < (b.x - a.x) * (z - a.z) / (b.z - a.z) + a.x {
            inside = !inside;
        }
    }
    inside
}

fn nearest_road_distance(road_samples: &[RoadSample], x: f64, z: f64) -> f64 {
    road_samples
        .iter()
        .map(|sample| {
            let dx = sample.position[0] - x;
            let dz = sample.position[1] - z;
            dx.hypot(dz)
        })
        .fold(f64::INFINITY, f64::min)
}

fn lerp_range(range: [f64; 2], t: f64) -> f64 {
    range[0] + (range[1] - range[0]) * t
}

/// Deterministic xorshift64* generator, seeded per region.
struct SeededRandom {
    state: u64,
}

impl SeededRandom {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn eval_profile(profile: &[ProfilePoint], station: f64, default: f64) -> f64 {
    if profile.is_empty() {
        return default;
    }
    if profile.len() == 1 {
        return profile[0].value;
    }
    if station <= profile[0].station {
        return profile[0].value;
    }
    if station >= profile[profile.len() - 1].station {
        return profile[profile.len() - 1].value;
    }
    for pair in profile.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if station >= a.station && station <= b.station {
            if b.station - a.station <= 1e-12 {
                return b.value;
            }
            let ratio = (station - a.station) / (b.station - a.station);
            return a.value + (b.value - a.value) * ratio;
        }
    }
    default
}

fn subsystem_hash<T: Serialize>(value: &T) -> Result<String, BuildError> {
    let bytes = serde_json::to_vec(value)?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn round6(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use track_assets::{AssetRegistry, AssetSpec, Budget};
    use track_domain::{AssetInstance, ControlPoint, Point2, ProfilePoint, VegetationRegion};

    fn square_document() -> TrackDocument {
        let mut document = TrackDocument::new("square");
        document.centerline = vec![
            ControlPoint {
                id: "p0".into(),
                x: 0.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "p1".into(),
                x: 10.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "p2".into(),
                x: 10.0,
                z: 10.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "p3".into(),
                x: 0.0,
                z: 10.0,
                handle_in: None,
                handle_out: None,
            },
        ];
        document.profiles.banking.push(ProfilePoint {
            station: 0.0,
            value: 3.0,
        });
        document.profiles.elevation.push(ProfilePoint {
            station: 0.0,
            value: 1.0,
        });
        document.profiles.width_left.push(ProfilePoint {
            station: 0.0,
            value: 5.0,
        });
        document.profiles.width_right.push(ProfilePoint {
            station: 0.0,
            value: 6.0,
        });
        document
    }

    fn registry_with_tree() -> AssetRegistry {
        AssetRegistry::from_specs(vec![AssetSpec {
            id: "tree_v2_01".into(),
            kind: "vegetation".into(),
            category: "trees".into(),
            source: Some("assets/tree.glb".into()),
            source_sha256: None,
            dimensions_m: BTreeMap::new(),
            preview: None,
            collision_class: "none".into(),
            budget: Budget::default(),
            metadata: BTreeMap::new(),
        }])
    }

    #[test]
    fn compile_produces_deterministic_road_samples() {
        let document = square_document();
        let ir = compile(&document, None).unwrap();

        assert_eq!(ir.build_ir_version, 1);
        assert!(!ir.road_samples.is_empty());
        assert_eq!(ir.source_project_hash.len(), 64);
        assert_eq!(ir.road_samples[0].width_left, 5.0);
        assert_eq!(ir.road_samples[0].width_right, 6.0);
        assert_eq!(ir.road_samples[0].elevation, 1.0);
        assert_eq!(ir.road_samples[0].bank, 3.0);

        assert_eq!(
            compile(&document, None).unwrap().to_bytes().unwrap(),
            ir.to_bytes().unwrap()
        );
    }

    #[test]
    fn asset_instances_resolve_kind_and_source_from_registry() {
        let mut document = square_document();
        document.asset_instances.push(AssetInstance {
            id: "tree_a".into(),
            asset_id: "tree_v2_01".into(),
            x: 2.0,
            z: 3.0,
            rotation: [0.0, 0.5, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: BTreeMap::new(),
        });
        let ir = compile(&document, Some(&registry_with_tree())).unwrap();
        assert_eq!(ir.asset_instances.len(), 1);
        let asset = &ir.asset_instances[0];
        assert_eq!(asset.kind.as_deref(), Some("vegetation"));
        assert_eq!(asset.source.as_deref(), Some("assets/tree.glb"));
        assert_eq!(asset.yaw_rad, 0.5);
    }

    #[test]
    fn subsystem_hashes_change_independently() {
        let document = square_document();
        let base = compile(&document, None).unwrap();

        let mut changed = square_document();
        changed.asset_instances.push(AssetInstance {
            id: "a".into(),
            asset_id: "tree_v2_01".into(),
            x: 1.0,
            z: 1.0,
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: BTreeMap::new(),
        });
        let ir = compile(&changed, None).unwrap();

        assert_eq!(base.geometry_hash, ir.geometry_hash);
        assert_ne!(base.objects_hash, ir.objects_hash);
    }

    #[test]
    fn missing_centerline_is_an_error() {
        let document = TrackDocument::new("empty");
        assert!(matches!(
            compile(&document, None),
            Err(BuildError::NoCenterline)
        ));
    }

    #[test]
    fn materialization_ready_flags_non_positive_width() {
        let document = square_document();
        let ir = compile(&document, None).unwrap();
        assert!(materialization_ready(&ir));

        let mut broken = square_document();
        broken.profiles.width_left.clear();
        broken.profiles.width_left.push(ProfilePoint {
            station: 0.0,
            value: -1.0,
        });
        let ir = compile(&broken, None).unwrap();
        assert!(!materialization_ready(&ir));
        assert!(materialization_prerequisites(&ir)
            .iter()
            .any(|m| m.contains("half width")));
    }

    #[test]
    fn write_track_build_json_is_deterministic_and_readable() {
        let document = square_document();
        let path =
            std::env::temp_dir().join(format!("formula90_track_build_{}.json", std::process::id()));
        write_track_build_json(&document, None, &path).unwrap();
        let first = std::fs::read(&path).unwrap();
        write_track_build_json(&document, None, &path).unwrap();
        let second = std::fs::read(&path).unwrap();
        assert_eq!(first, second);
        serde_json::from_slice::<serde_json::Value>(&first)
            .expect("track.build.json is valid JSON");
        std::fs::remove_file(path).unwrap();
    }

    fn vegetation_document() -> TrackDocument {
        let mut document = square_document();
        document.vegetation_regions.push(VegetationRegion {
            id: "trees_000".into(),
            polygon: vec![
                Point2 { x: 0.0, z: 0.0 },
                Point2 { x: 10.0, z: 0.0 },
                Point2 { x: 10.0, z: 10.0 },
                Point2 { x: 0.0, z: 10.0 },
            ],
            seed: 1995,
            density: 1.0,
            asset_set: vec!["tree_v2_01".into(), "tree_v2_02".into()],
            min_spacing: 2.0,
            scale_range: [0.8, 1.4],
            rotation_range: [0.0, std::f64::consts::PI],
            track_exclusion: 1.0,
        });
        document
    }

    #[test]
    fn terrain_heightfield_is_deterministic_and_positive_sized() {
        let document = square_document();
        let ir = compile(&document, None).unwrap();
        assert!(!ir.terrain_heightfield.is_empty());
        assert!(ir
            .terrain_heightfield
            .iter()
            .all(|cell| cell.height.is_finite()));
        assert_eq!(
            compile(&document, None).unwrap().terrain_heightfield,
            ir.terrain_heightfield
        );
        assert!(ir.terrain_cell_m >= 0.0);
    }

    #[test]
    fn vegetation_generation_is_deterministic_per_seed() {
        let document = vegetation_document();
        let first = compile(&document, None).unwrap();
        let second = compile(&document, None).unwrap();
        assert_eq!(first.vegetation_instances, second.vegetation_instances);
        assert!(!first.vegetation_instances.is_empty());
        // Instances must sit inside the region polygon.
        for instance in &first.vegetation_instances {
            let (x, z) = (instance.position[0], instance.position[1]);
            assert!((-1e-6..=10.0 + 1e-6).contains(&x));
            assert!((-1e-6..=10.0 + 1e-6).contains(&z));
        }
    }

    #[test]
    fn vegetation_seed_change_changes_instances() {
        let mut document = vegetation_document();
        let base = compile(&document, None).unwrap();
        document.vegetation_regions[0].seed = 42;
        let changed = compile(&document, None).unwrap();
        assert_ne!(base.vegetation_instances, changed.vegetation_instances);
        assert_ne!(base.vegetation_hash, changed.vegetation_hash);
    }

    #[test]
    fn terrain_and_vegetation_hashes_are_part_of_build_ir() {
        let document = vegetation_document();
        let ir = compile(&document, None).unwrap();
        assert_eq!(ir.terrain_hash.len(), 64);
        assert_eq!(ir.vegetation_hash.len(), 64);
    }
}
