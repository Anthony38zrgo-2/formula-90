//! Validation for procedural 3D mountain assets (manifest.json).
//!
//! Validates the manifest structure, generation parameters, and asset references
//! produced by `generate_mountains_3d.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::validation::ValidationResult;

/// Errors during manifest parsing.
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("JSON parse error: {0}")]
    JsonParse(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Coordinate convention for 3D assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateConvention {
    pub forward: String,
    pub up: String,
    pub right: String,
    pub unit: String,
}

/// Nested layer generation parameters (from topographic SVG pipeline).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerGenerationParams {
    #[serde(default)]
    pub layer: String,
    #[serde(default)]
    pub radius: f64,
    #[serde(default)]
    pub depth: f64,
    #[serde(default)]
    pub height_scale: f64,
    #[serde(default)]
    pub height_min: f64,
    #[serde(default)]
    pub height_max: f64,
    #[serde(default)]
    pub target_height_range: Vec<f64>,
}

/// Nested sky dome generation parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkyDomeGenerationParams {
    #[serde(default)]
    pub radius: f64,
    #[serde(default)]
    pub rings: usize,
    #[serde(default)]
    pub segments: usize,
}

/// Generation parameters from the Python pipeline (supports both flat and nested schema).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationParams {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub source_svg: String,
    #[serde(default)]
    pub source_sprites: Vec<String>,
    #[serde(default)]
    pub palette: String,
    #[serde(default)]
    pub palette_colors_far: usize,
    #[serde(default)]
    pub palette_colors_near: usize,
    #[serde(default)]
    pub segments: usize,
    #[serde(default)]
    pub variants_per_ring: usize,
    #[serde(default)]
    pub variant_noise_scale: f64,
    #[serde(default)]
    pub height_scale_far: f64,
    #[serde(default)]
    pub height_scale_near: f64,
    #[serde(default)]
    pub height_scale_factor_far: f64,
    #[serde(default)]
    pub depth_far: f64,
    #[serde(default)]
    pub depth_near: f64,
    #[serde(default)]
    pub terrain_rows: usize,
    #[serde(default)]
    pub radius_far: f64,
    #[serde(default)]
    pub radius_near: f64,
    #[serde(default)]
    pub sky_dome_radius: f64,
    #[serde(default)]
    pub sky_dome_rings: usize,
    #[serde(default)]
    pub sky_dome_segments: usize,
    #[serde(default)]
    pub near: Option<LayerGenerationParams>,
    #[serde(default)]
    pub far: Option<LayerGenerationParams>,
    #[serde(default)]
    pub sky: Option<SkyDomeGenerationParams>,
}

impl GenerationParams {
    pub fn get_radius_far(&self) -> f64 {
        if self.radius_far > 0.0 {
            self.radius_far
        } else {
            self.far.as_ref().map(|f| f.radius).unwrap_or(0.0)
        }
    }

    pub fn get_radius_near(&self) -> f64 {
        if self.radius_near > 0.0 {
            self.radius_near
        } else {
            self.near.as_ref().map(|n| n.radius).unwrap_or(0.0)
        }
    }

    pub fn get_depth_far(&self) -> f64 {
        if self.depth_far > 0.0 {
            self.depth_far
        } else {
            self.far.as_ref().map(|f| f.depth).unwrap_or(0.0)
        }
    }

    pub fn get_depth_near(&self) -> f64 {
        if self.depth_near > 0.0 {
            self.depth_near
        } else {
            self.near.as_ref().map(|n| n.depth).unwrap_or(0.0)
        }
    }

    pub fn get_height_scale_far(&self) -> f64 {
        if self.height_scale_far > 0.0 {
            self.height_scale_far
        } else {
            self.far.as_ref().map(|f| f.height_scale).unwrap_or(0.0)
        }
    }

    pub fn get_height_scale_near(&self) -> f64 {
        if self.height_scale_near > 0.0 {
            self.height_scale_near
        } else {
            self.near.as_ref().map(|n| n.height_scale).unwrap_or(0.0)
        }
    }

    pub fn get_sky_dome_radius(&self) -> f64 {
        if self.sky_dome_radius > 0.0 {
            self.sky_dome_radius
        } else {
            self.sky.as_ref().map(|s| s.radius).unwrap_or(0.0)
        }
    }
}

/// Waterfall placement data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallData {
    #[serde(default)]
    pub segment: usize,
    #[serde(default)]
    pub angle_rad: f64,
    #[serde(default)]
    pub position: Vec<f64>,
    #[serde(default)]
    pub scale_y: f64,
}

/// Validation hashes for output assets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationData {
    #[serde(default)]
    pub source_sha256: HashMap<String, String>,
}

/// Complete mountain asset manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountainAssetManifest {
    pub schema_version: u32,
    pub asset: String,
    #[serde(default)]
    pub coordinate_convention: Option<CoordinateConvention>,
    #[serde(default)]
    pub geometry_assets: HashMap<String, String>,
    #[serde(default)]
    pub generation: Option<GenerationParams>,
    #[serde(default)]
    pub waterfalls: Vec<WaterfallData>,
    #[serde(default)]
    pub validation: Option<ValidationData>,
}

/// Parse a mountain asset manifest from JSON.
pub fn parse_manifest(json_str: &str) -> Result<MountainAssetManifest, ManifestError> {
    serde_json::from_str(json_str).map_err(|e| ManifestError::JsonParse(e.to_string()))
}

/// Validate a mountain asset manifest.
pub fn validate_manifest(manifest: &MountainAssetManifest) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Schema version
    if manifest.schema_version == 0 {
        result.add_error("schema_version must be > 0".to_string());
    }

    // Asset ID
    if manifest.asset.is_empty() {
        result.add_error("asset ID is empty".to_string());
    }

    // Geometry assets: must have at least far_mountains and near_mountains
    if !manifest.geometry_assets.contains_key("far_mountains") {
        result.add_error("Missing geometry asset: far_mountains".to_string());
    }
    if !manifest.geometry_assets.contains_key("near_mountains") {
        result.add_error("Missing geometry asset: near_mountains".to_string());
    }

    // Validate generation params if present
    if let Some(ref gen) = manifest.generation {
        validate_generation_params(gen, &mut result);
    }

    // Validate waterfalls
    for (i, wf) in manifest.waterfalls.iter().enumerate() {
        if wf.position.len() != 3 {
            result.add_error(format!(
                "Waterfall {} position must have 3 components, got {}",
                i,
                wf.position.len()
            ));
        }
        if wf.scale_y <= 0.0 {
            result.add_error(format!("Waterfall {} scale_y must be > 0", i));
        }
    }

    // Coordinate convention
    if let Some(ref cc) = manifest.coordinate_convention {
        if cc.unit != "meter" {
            result.add_warning(format!(
                "Unexpected unit '{}', expected 'meter'",
                cc.unit
            ));
        }
    }

    result
}

/// Validate generation parameters.
fn validate_generation_params(gen: &GenerationParams, result: &mut ValidationResult) {
    if gen.segments == 0 {
        result.add_error("segments must be > 0".to_string());
    }
    if gen.variants_per_ring > 0 && gen.segments % gen.variants_per_ring != 0 {
        result.add_error(format!(
            "segments ({}) must be divisible by variants_per_ring ({})",
            gen.segments, gen.variants_per_ring
        ));
    }
    let h_far = gen.get_height_scale_far();
    if h_far <= 0.0 {
        result.add_error("height_scale_far must be > 0".to_string());
    }
    let h_near = gen.get_height_scale_near();
    if h_near <= 0.0 {
        result.add_error("height_scale_near must be > 0".to_string());
    }
    let d_far = gen.get_depth_far();
    if d_far <= 0.0 {
        result.add_error("depth_far must be > 0".to_string());
    }
    let d_near = gen.get_depth_near();
    if d_near <= 0.0 {
        result.add_error("depth_near must be > 0".to_string());
    }
    if gen.terrain_rows < 2 {
        result.add_error("terrain_rows must be >= 2".to_string());
    }
    let r_far = gen.get_radius_far();
    if r_far <= 0.0 {
        result.add_error("radius_far must be > 0".to_string());
    }
    let r_near = gen.get_radius_near();
    if r_near <= 0.0 {
        result.add_error("radius_near must be > 0".to_string());
    }
    if r_near > 0.0 && r_far > 0.0 && r_near >= r_far {
        result.add_warning("radius_near should be < radius_far for proper layering".to_string());
    }
    let sky_r = gen.get_sky_dome_radius();
    if sky_r <= 0.0 {
        result.add_error("sky_dome_radius must be > 0".to_string());
    }
    if sky_r > 0.0 && r_far > 0.0 && sky_r < r_far {
        result.add_warning("sky_dome_radius should be >= radius_far to enclose mountains".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST: &str = r#"{
        "schema_version": 1,
        "asset": "la_chutana_mountains_3d",
        "coordinate_convention": {
            "forward": "-Z", "up": "+Y", "right": "+X", "unit": "meter"
        },
        "geometry_assets": {
            "far_mountains": "far_mountains_ring.glb",
            "near_mountains": "near_mountains_ring.glb",
            "sky_dome": "sky_dome.glb"
        },
        "generation": {
            "segments": 640,
            "variants_per_ring": 8,
            "height_scale_far": 150.0,
            "height_scale_near": 100.0,
            "depth_far": 300.0,
            "depth_near": 200.0,
            "terrain_rows": 4,
            "radius_far": 700.0,
            "radius_near": 600.0,
            "sky_dome_radius": 1500.0,
            "palette_colors_far": 20,
            "palette_colors_near": 23
        },
        "waterfalls": [
            {"segment": 516, "angle_rad": 5.06, "position": [207.0, 50.0, -562.0], "scale_y": 1.15}
        ]
    }"#;

    #[test]
    fn parse_valid_manifest() {
        let manifest = parse_manifest(VALID_MANIFEST).unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.asset, "la_chutana_mountains_3d");
        assert_eq!(manifest.geometry_assets.len(), 3);
        assert_eq!(manifest.waterfalls.len(), 1);
    }

    #[test]
    fn validate_valid_manifest() {
        let manifest = parse_manifest(VALID_MANIFEST).unwrap();
        let result = validate_manifest(&manifest);
        assert!(result.is_valid, "Expected valid: {}", result.get_error_summary());
    }

    #[test]
    fn missing_far_mountains_fails() {
        let mut manifest = parse_manifest(VALID_MANIFEST).unwrap();
        manifest.geometry_assets.remove("far_mountains");
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("far_mountains")));
    }

    #[test]
    fn zero_segments_fails() {
        let mut manifest = parse_manifest(VALID_MANIFEST).unwrap();
        manifest.generation.as_mut().unwrap().segments = 0;
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("segments")));
    }

    #[test]
    fn negative_height_fails() {
        let mut manifest = parse_manifest(VALID_MANIFEST).unwrap();
        manifest.generation.as_mut().unwrap().height_scale_far = -10.0;
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid);
    }

    #[test]
    fn radius_near_gt_far_warns() {
        let mut manifest = parse_manifest(VALID_MANIFEST).unwrap();
        manifest.generation.as_mut().unwrap().radius_near = 800.0;
        let result = validate_manifest(&manifest);
        assert!(result.warnings.iter().any(|w| w.contains("radius_near")));
    }

    #[test]
    fn empty_asset_id_fails() {
        let mut manifest = parse_manifest(VALID_MANIFEST).unwrap();
        manifest.asset = String::new();
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid);
    }

    #[test]
    fn waterfall_bad_position_fails() {
        let mut manifest = parse_manifest(VALID_MANIFEST).unwrap();
        manifest.waterfalls[0].position = vec![1.0, 2.0]; // only 2 components
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid);
    }

    #[test]
    fn validate_production_topo_manifest() {
        let prod_manifest = r#"{
            "schema_version": 1,
            "asset": "la_chutana_mountains_3d",
            "coordinate_convention": {
                "forward": "-Z", "up": "+Y", "right": "+X", "unit": "meter"
            },
            "geometry_assets": {
                "far_mountains": "far_mountains_ring.glb",
                "near_mountains": "near_mountains_ring.glb",
                "sky_dome": "sky_dome.glb"
            },
            "generation": {
                "method": "svg_topographic_extrusion",
                "source_svg": "skybox_topography_semantic_v3.svg",
                "height_scale_base": 1.8,
                "height_scale_far": 1.5,
                "height_scale_near": 1.0,
                "depth_scale": 0.7,
                "distance": 4000.0,
                "near": {
                    "layer": "near",
                    "radius": 1150.0,
                    "depth": 154.0,
                    "height_scale": 1.0,
                    "height_min": 0.0,
                    "height_max": 100.0
                },
                "far": {
                    "layer": "far",
                    "radius": 1600.0,
                    "depth": 224.0,
                    "height_scale": 1.5,
                    "height_min": 0.0,
                    "height_max": 225.0
                },
                "sky": {
                    "radius": 1800.0,
                    "rings": 16,
                    "segments": 64
                },
                "terrain_rows": 6,
                "segments": 640
            },
            "waterfalls": [
                {"segment": 68, "angle_rad": 0.6676, "position": [903.11, 25.0, 711.96], "scale_y": 1.4},
                {"segment": 170, "angle_rad": 1.6690, "position": [-112.72, 25.0, 1144.46], "scale_y": 1.4},
                {"segment": 352, "angle_rad": 3.4558, "position": [-1093.71, 25.0, -355.37], "scale_y": 1.4}
            ]
        }"#;

        let manifest = parse_manifest(prod_manifest).unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.asset, "la_chutana_mountains_3d");
        assert_eq!(manifest.geometry_assets.len(), 3);
        assert_eq!(manifest.waterfalls.len(), 3);

        let result = validate_manifest(&manifest);
        assert!(result.is_valid, "Expected production manifest to be valid: {}", result.get_error_summary());
        assert!(result.warnings.is_empty(), "Expected zero warnings: {:?}", result.warnings);
    }
}
