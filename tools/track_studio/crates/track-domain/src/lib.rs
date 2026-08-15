use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

pub const TRACK_DOCUMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackDocument {
    pub schema_version: u32,
    pub project_version: String,
    pub compiler_version: String,
    pub track_id: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
    pub centerline: Vec<ControlPoint>,
    #[serde(default)]
    pub profiles: Profiles,
    #[serde(default)]
    pub track_sections: Vec<TrackSection>,
    #[serde(default)]
    pub terrain_regions: Vec<Region>,
    #[serde(default)]
    pub vegetation_regions: Vec<VegetationRegion>,
    #[serde(default)]
    pub asset_instances: Vec<AssetInstance>,
    #[serde(default)]
    pub reference_layers: Vec<ReferenceLayer>,
    #[serde(default)]
    pub gameplay: Gameplay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Profiles {
    #[serde(default)]
    pub width_left: Vec<ProfilePoint>,
    #[serde(default)]
    pub width_right: Vec<ProfilePoint>,
    #[serde(default)]
    pub elevation: Vec<ProfilePoint>,
    #[serde(default)]
    pub banking: Vec<ProfilePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ControlPoint {
    pub id: String,
    pub x: f64,
    pub z: f64,
    #[serde(default)]
    pub handle_in: Option<Point2>,
    #[serde(default)]
    pub handle_out: Option<Point2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfilePoint {
    pub station: f64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackSection {
    pub id: String,
    pub section_type: String,
    pub station_start: f64,
    pub station_end: f64,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Region {
    pub id: String,
    pub region_type: String,
    pub polygon: Vec<Point2>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VegetationRegion {
    pub id: String,
    pub polygon: Vec<Point2>,
    pub seed: u64,
    pub density: f64,
    pub asset_set: Vec<String>,
    pub min_spacing: f64,
    pub scale_range: [f64; 2],
    pub rotation_range: [f64; 2],
    pub track_exclusion: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetInstance {
    pub id: String,
    pub asset_id: String,
    pub x: f64,
    pub z: f64,
    pub rotation: [f64; 3],
    pub scale: [f64; 3],
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Gameplay {
    #[serde(default)]
    pub markers: Vec<GameplayMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameplayMarker {
    pub id: String,
    pub marker_type: String,
    pub station: f64,
    pub x: f64,
    pub z: f64,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

/// A reference image anchored in the world for manual authoring. It is a
/// first-class entity: position, rotation, scale, opacity, lock and visibility
/// plus a deterministic pixel-to-meter calibration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceLayer {
    pub id: String,
    pub image_path: String,
    pub position: Point2,
    pub rotation_rad: f64,
    pub scale: f64,
    pub opacity: f64,
    pub locked: bool,
    pub visible: bool,
    #[serde(default)]
    pub calibration: Option<Calibration>,
}

/// Deterministic pixel-to-meter calibration for a reference image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Calibration {
    pub pixels_per_meter: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    #[serde(default)]
    pub object_id: Option<String>,
    #[serde(default)]
    pub station: Option<f64>,
    pub message: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

impl TrackDocument {
    pub fn new(track_id: impl Into<String>) -> Self {
        Self {
            schema_version: TRACK_DOCUMENT_SCHEMA_VERSION,
            project_version: "0.1.0".into(),
            compiler_version: "track-domain-0.1.0".into(),
            track_id: track_id.into(),
            metadata: BTreeMap::new(),
            centerline: Vec::new(),
            profiles: Profiles::default(),
            track_sections: Vec::new(),
            terrain_regions: Vec::new(),
            vegetation_regions: Vec::new(),
            asset_instances: Vec::new(),
            reference_layers: Vec::new(),
            gameplay: Gameplay::default(),
        }
    }

    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.schema_version != TRACK_DOCUMENT_SCHEMA_VERSION {
            diagnostics.push(Diagnostic::error(
                "domain.schema_version",
                format!("unsupported TrackDocument schema {}", self.schema_version),
            ));
        }
        if self.track_id.trim().is_empty() {
            diagnostics.push(Diagnostic::error("domain.track_id", "track_id is required"));
        }
        if self.centerline.len() < 3 {
            diagnostics.push(Diagnostic::error(
                "centerline.minimum_points",
                "centerline needs at least three control points",
            ));
        }

        let mut ids = BTreeSet::new();
        for point in &self.centerline {
            check_id(&mut diagnostics, &mut ids, &point.id, "centerline");
            check_point(&mut diagnostics, &point.id, point.x, point.z);
        }
        for section in &self.track_sections {
            check_id(&mut diagnostics, &mut ids, &section.id, "track_section");
            check_range(
                &mut diagnostics,
                &section.id,
                section.station_start,
                section.station_end,
            );
        }
        for region in &self.terrain_regions {
            check_id(&mut diagnostics, &mut ids, &region.id, "terrain_region");
            check_polygon(&mut diagnostics, &region.id, &region.polygon);
        }
        for region in &self.vegetation_regions {
            check_id(&mut diagnostics, &mut ids, &region.id, "vegetation_region");
            check_polygon(&mut diagnostics, &region.id, &region.polygon);
            check_finite(&mut diagnostics, &region.id, region.density, "density");
            check_finite(
                &mut diagnostics,
                &region.id,
                region.min_spacing,
                "min_spacing",
            );
            if region.asset_set.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "vegetation.asset_set",
                    format!("vegetation region {} has no assets", region.id),
                ));
            }
        }
        for asset in &self.asset_instances {
            check_id(&mut diagnostics, &mut ids, &asset.id, "asset_instance");
            check_id_value(&mut diagnostics, &asset.id, &asset.asset_id, "asset_id");
            check_point(&mut diagnostics, &asset.id, asset.x, asset.z);
            for value in asset.rotation.iter().chain(asset.scale.iter()) {
                check_finite(&mut diagnostics, &asset.id, *value, "transform");
            }
        }
        for layer in &self.reference_layers {
            check_id(&mut diagnostics, &mut ids, &layer.id, "reference_layer");
            check_point(
                &mut diagnostics,
                &layer.id,
                layer.position.x,
                layer.position.z,
            );
            check_finite(
                &mut diagnostics,
                &layer.id,
                layer.rotation_rad,
                "rotation_rad",
            );
            check_finite(&mut diagnostics, &layer.id, layer.scale, "scale");
            check_finite(&mut diagnostics, &layer.id, layer.opacity, "opacity");
            if !(0.0..=1.0).contains(&layer.opacity) {
                diagnostics.push(Diagnostic::error(
                    "reference_layer.opacity",
                    format!("{}: opacity must be in [0, 1]", layer.id),
                ));
            }
            if layer.scale <= 0.0 {
                diagnostics.push(Diagnostic::error(
                    "reference_layer.scale",
                    format!("{}: scale must be positive", layer.id),
                ));
            }
            if let Some(calibration) = &layer.calibration {
                if !calibration.pixels_per_meter.is_finite() || calibration.pixels_per_meter <= 0.0
                {
                    diagnostics.push(Diagnostic::error(
                        "reference_layer.calibration",
                        format!("{}: pixels_per_meter must be positive", layer.id),
                    ));
                }
            }
        }
        for marker in &self.gameplay.markers {
            check_id(&mut diagnostics, &mut ids, &marker.id, "gameplay_marker");
            check_finite(&mut diagnostics, &marker.id, marker.station, "station");
            check_point(&mut diagnostics, &marker.id, marker.x, marker.z);
        }
        diagnostics.extend(validate_profiles(&self.profiles));
        diagnostics
    }

    pub fn canonicalize(&mut self) {
        self.track_sections.sort_by(|a, b| a.id.cmp(&b.id));
        self.terrain_regions.sort_by(|a, b| a.id.cmp(&b.id));
        self.vegetation_regions.sort_by(|a, b| a.id.cmp(&b.id));
        self.asset_instances.sort_by(|a, b| a.id.cmp(&b.id));
        self.gameplay.markers.sort_by(|a, b| a.id.cmp(&b.id));
        for profile in [
            &mut self.profiles.width_left,
            &mut self.profiles.width_right,
            &mut self.profiles.elevation,
            &mut self.profiles.banking,
        ] {
            profile.sort_by(|a, b| a.station.total_cmp(&b.station));
        }
    }

    pub fn canonical_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut document = self.clone();
        document.canonicalize();
        let mut output = Vec::new();
        let formatter = FixedFloatFormatter;
        let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
        document.serialize(&mut serializer)?;
        Ok(output)
    }
}

struct FixedFloatFormatter;

impl serde_json::ser::Formatter for FixedFloatFormatter {
    fn write_f64<W>(&mut self, writer: &mut W, value: f64) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        write!(writer, "{value:.6}")
    }
}

impl Diagnostic {
    fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            object_id: None,
            station: None,
            message: message.into(),
            metadata: BTreeMap::new(),
        }
    }
}

fn check_id(diagnostics: &mut Vec<Diagnostic>, ids: &mut BTreeSet<String>, id: &str, kind: &str) {
    if id.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            format!("{kind}.id"),
            "persistent id is required",
        ));
    } else if !ids.insert(id.to_owned()) {
        diagnostics.push(Diagnostic::error(
            "domain.duplicate_id",
            format!("duplicate id {id}"),
        ));
    }
}

fn check_id_value(diagnostics: &mut Vec<Diagnostic>, owner: &str, value: &str, field: &str) {
    if value.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            format!("asset.{field}"),
            format!("{owner}: {field} is required"),
        ));
    }
}

fn check_point(diagnostics: &mut Vec<Diagnostic>, owner: &str, x: f64, z: f64) {
    check_finite(diagnostics, owner, x, "x");
    check_finite(diagnostics, owner, z, "z");
}

fn check_finite(diagnostics: &mut Vec<Diagnostic>, owner: &str, value: f64, field: &str) {
    if !value.is_finite() {
        diagnostics.push(Diagnostic::error(
            "domain.non_finite",
            format!("{owner}: {field} must be finite"),
        ));
    }
}

fn check_range(diagnostics: &mut Vec<Diagnostic>, owner: &str, start: f64, end: f64) {
    check_finite(diagnostics, owner, start, "station_start");
    check_finite(diagnostics, owner, end, "station_end");
    if start < 0.0 || end < start {
        diagnostics.push(Diagnostic::error(
            "section.station_range",
            format!("{owner}: invalid station range"),
        ));
    }
}

fn check_polygon(diagnostics: &mut Vec<Diagnostic>, owner: &str, polygon: &[Point2]) {
    if polygon.len() < 3 {
        diagnostics.push(Diagnostic::error(
            "region.polygon",
            format!("{owner}: polygon needs at least three points"),
        ));
    }
    for point in polygon {
        check_point(diagnostics, owner, point.x, point.z);
    }
}

fn validate_profiles(profiles: &Profiles) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (name, points) in [
        ("width_left", &profiles.width_left),
        ("width_right", &profiles.width_right),
        ("elevation", &profiles.elevation),
        ("banking", &profiles.banking),
    ] {
        let mut previous = None;
        for point in points {
            check_finite(&mut diagnostics, name, point.station, "station");
            check_finite(&mut diagnostics, name, point.value, "value");
            if point.station < 0.0 || previous.is_some_and(|station| point.station < station) {
                diagnostics.push(Diagnostic::error(
                    "profile.station_order",
                    format!("{name}: station values must be non-negative and ordered"),
                ));
            }
            previous = Some(point.station);
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_reports_missing_centerline() {
        let document = TrackDocument::new("la_chutana");
        assert!(document
            .validate()
            .iter()
            .any(|diagnostic| diagnostic.code == "centerline.minimum_points"));
    }

    #[test]
    fn duplicate_ids_and_non_finite_values_are_diagnostics() {
        let mut document = TrackDocument::new("test");
        document.centerline = vec![
            ControlPoint {
                id: "p".into(),
                x: 0.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "p".into(),
                x: f64::NAN,
                z: 1.0,
                handle_in: None,
                handle_out: None,
            },
        ];
        let diagnostics = document.validate();
        assert!(diagnostics.iter().any(|d| d.code == "domain.duplicate_id"));
        assert!(diagnostics.iter().any(|d| d.code == "domain.non_finite"));
    }

    #[test]
    fn canonical_json_is_independent_of_collection_insertion_order() {
        let mut first = TrackDocument::new("test");
        first.asset_instances.push(asset("b"));
        first.asset_instances.push(asset("a"));

        let mut second = TrackDocument::new("test");
        second.asset_instances.push(asset("a"));
        second.asset_instances.push(asset("b"));

        assert_eq!(
            first.canonical_json().unwrap(),
            second.canonical_json().unwrap()
        );
        let json = String::from_utf8(first.canonical_json().unwrap()).unwrap();
        assert!(json.contains("1.000000"));
        assert!(json.contains("2.000000"));
    }

    fn asset(id: &str) -> AssetInstance {
        AssetInstance {
            id: id.into(),
            asset_id: "tree_v2_01".into(),
            x: 1.0,
            z: 2.0,
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: BTreeMap::new(),
        }
    }
}
