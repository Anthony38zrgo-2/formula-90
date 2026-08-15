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

/// Generation parameters from the Python pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParams {
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
    if gen.segments % gen.variants_per_ring != 0 {
        result.add_error(format!(
            "segments ({}) must be divisible by variants_per_ring ({})",
            gen.segments, gen.variants_per_ring
        ));
    }
    if gen.height_scale_far <= 0.0 {
        result.add_error("height_scale_far must be > 0".to_string());
    }
    if gen.height_scale_near <= 0.0 {
        result.add_error("height_scale_near must be > 0".to_string());
    }
    if gen.depth_far <= 0.0 {
        result.add_error("depth_far must be > 0".to_string());
    }
    if gen.depth_near <= 0.0 {
        result.add_error("depth_near must be > 0".to_string());
    }
    if gen.terrain_rows < 2 {
        result.add_error("terrain_rows must be >= 2".to_string());
    }
    if gen.radius_far <= 0.0 {
        result.add_error("radius_far must be > 0".to_string());
    }
    if gen.radius_near <= 0.0 {
        result.add_error("radius_near must be > 0".to_string());
    }
    if gen.radius_near >= gen.radius_far {
        result.add_warning("radius_near should be < radius_far for proper layering".to_string());
    }
    if gen.sky_dome_radius <= 0.0 {
        result.add_error("sky_dome_radius must be > 0".to_string());
    }
    if gen.sky_dome_radius <= gen.radius_far {
        result.add_warning("sky_dome_radius should be > radius_far to enclose mountains".to_string());
    }
    if gen.palette_colors_far == 0 {
        result.add_warning("palette_colors_far is 0".to_string());
    }
    if gen.palette_colors_near == 0 {
        result.add_warning("palette_colors_near is 0".to_string());
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
}
