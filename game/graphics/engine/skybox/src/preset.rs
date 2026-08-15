//! JSON parsing and serialization for background presets.
//!
//! Mirrors `BackgroundPreset.load_from_json_file()` and `to_dict()` from GDScript.
//! Handles the flexible scale deserialization (number, array, object) and
//! applies defaults consistent with the GDScript layer.

use crate::types::{BackgroundPreset, LayerConfig};
use serde_json;
use thiserror::Error;

/// Errors that can occur during preset JSON parsing.
#[derive(Debug, Error)]
pub enum PresetError {
    #[error("JSON parse error at line {line}: {message}")]
    JsonParse { line: usize, message: String },

    #[error("JSON root must be an object, got {0}")]
    RootNotObject(String),

    #[error("Invalid layer config at index {index}: {message}")]
    InvalidLayer { index: usize, message: String },
}

/// Parse a background preset from a JSON string.
///
/// Mirrors `BackgroundPreset.from_dict()` / `BackgroundPreset.load_from_json_file()`.
/// Applies defaults consistent with GDScript:
/// - `distance_z`: `-800.0 + depth * 100.0` if the field was omitted (serde default handles this)
/// - `pixel_size`: `0.5`
/// - `scale`: `(1.0, 1.0)`
pub fn parse_preset_json(json_str: &str) -> Result<BackgroundPreset, PresetError> {
    let mut preset: BackgroundPreset = serde_json::from_str(json_str).map_err(|e| {
        PresetError::JsonParse {
            line: e.line(),
            message: e.to_string(),
        }
    })?;

    // GDScript defaults display_name to id if empty
    if preset.display_name.is_empty() {
        preset.display_name = preset.id.clone();
    }

    // Apply distance_z default per layer: -800.0 + depth * 100.0
    // GDScript: if field ABSENT -> -800 + depth*100; if PRESENT -> use exact value.
    // Serde gives us Option<f64>: None = absent, Some(v) = present.
    for layer in &mut preset.layers {
        if layer.distance_z.is_none() {
            layer.distance_z = Some(-800.0 + (layer.depth as f64) * 100.0);
        }
    }

    Ok(preset)
}

/// Serialize a background preset to a JSON string.
///
/// Mirrors `BackgroundPreset.to_dict()`.
pub fn to_json(preset: &BackgroundPreset) -> String {
    serde_json::to_string_pretty(preset).unwrap_or_else(|_| "{}".to_string())
}

/// Get layers sorted by depth (ascending).
///
/// Mirrors `BackgroundPreset.get_layers_sorted_by_depth()`.
pub fn layers_sorted_by_depth(preset: &BackgroundPreset) -> Vec<&LayerConfig> {
    let mut sorted: Vec<&LayerConfig> = preset.layers.iter().collect();
    sorted.sort_by_key(|l| l.depth);
    sorted
}

/// Find a layer by ID.
///
/// Mirrors `BackgroundPreset.get_layer_by_id()`.
pub fn get_layer_by_id<'a>(
    preset: &'a BackgroundPreset,
    id: &str,
) -> Option<&'a LayerConfig> {
    preset.layers.iter().find(|l| l.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_JSON: &str = r#"{
        "id": "test_preset",
        "display_name": "Test Preset",
        "skybox": {
            "mode": "gradient",
            "distance": 800.0,
            "gradient": {
                "zenith_color": [0.18, 0.42, 0.82],
                "horizon_color": [0.95, 0.92, 0.82]
            }
        },
        "layers": [
            {
                "id": "far_mountains",
                "texture": "res://assets/test/far.png",
                "depth": 0,
                "parallax_x": 0.08,
                "distance_z": -700.0,
                "pixel_size": 1.5
            },
            {
                "id": "near_mountains",
                "texture": "res://assets/test/near.png",
                "depth": 1,
                "parallax_x": 0.18,
                "scale": 1.0,
                "pixel_size": 1.2
            }
        ]
    }"#;

    #[test]
    fn parse_valid_json() {
        let preset = parse_preset_json(VALID_JSON).unwrap();
        assert_eq!(preset.id, "test_preset");
        assert_eq!(preset.display_name, "Test Preset");
        assert!(preset.skybox.is_some());
        assert_eq!(preset.layers.len(), 2);
    }

    #[test]
    fn parse_skybox_gradient() {
        let preset = parse_preset_json(VALID_JSON).unwrap();
        let skybox = preset.skybox.unwrap();
        assert_eq!(skybox.mode, crate::types::SkyboxMode::Gradient);
        assert!(skybox.gradient.is_some());
        let g = skybox.gradient.unwrap();
        assert_eq!(g.zenith_color, Some([0.18, 0.42, 0.82]));
    }

    #[test]
    fn scale_as_number() {
        let preset = parse_preset_json(VALID_JSON).unwrap();
        // near_mountains has scale: 1.0 (number)
        let near = get_layer_by_id(&preset, "near_mountains").unwrap();
        assert_eq!(near.scale.x, 1.0);
        assert_eq!(near.scale.y, 1.0);
    }

    #[test]
    fn distance_z_default_applied() {
        let preset = parse_preset_json(VALID_JSON).unwrap();
        // near_mountains has depth=1, no explicit distance_z
        let near = get_layer_by_id(&preset, "near_mountains").unwrap();
        // Default should be -800.0 + 1*100.0 = -700.0
        assert!((near.distance_z.unwrap() - (-700.0)).abs() < 0.001);
    }

    #[test]
    fn display_name_defaults_to_id() {
        let json = r#"{"id": "foo", "layers": []}"#;
        let preset = parse_preset_json(json).unwrap();
        assert_eq!(preset.display_name, "foo");
    }

    #[test]
    fn json_roundtrip() {
        let preset = parse_preset_json(VALID_JSON).unwrap();
        let serialized = to_json(&preset);
        let reparsed = parse_preset_json(&serialized).unwrap();
        assert_eq!(preset.id, reparsed.id);
        assert_eq!(preset.layers.len(), reparsed.layers.len());
    }

    #[test]
    fn layers_sorted_by_depth_order() {
        let preset = parse_preset_json(VALID_JSON).unwrap();
        let sorted = layers_sorted_by_depth(&preset);
        assert_eq!(sorted[0].id, "far_mountains");
        assert_eq!(sorted[1].id, "near_mountains");
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = parse_preset_json("{bad json");
        assert!(result.is_err());
    }
}
