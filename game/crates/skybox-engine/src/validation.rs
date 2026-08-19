//! Validation rules for background presets, layers, and skybox configs.
//!
//! Mirrors `BackgroundValidator.validate_preset()` from GDScript.
//! All validation rules are pure functions; no I/O or Godot dependencies.

use crate::types::{BackgroundPreset, LayerConfig, SkyboxConfig, SkyboxMode};
use std::collections::HashMap;

/// Result of validating a preset or component.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, message: String) {
        self.is_valid = false;
        self.errors.push(message);
    }

    pub fn add_warning(&mut self, message: String) {
        self.warnings.push(message);
    }

    pub fn get_error_summary(&self) -> String {
        if self.is_valid {
            "OK".to_string()
        } else {
            self.errors.join("\n")
        }
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate a complete background preset.
///
/// Mirrors `BackgroundValidator.validate_preset()` from GDScript.
pub fn validate_preset(preset: &BackgroundPreset) -> ValidationResult {
    let mut result = ValidationResult::new();

    if preset.id.is_empty() {
        result.add_error("El preset no tiene un 'id' valido o esta vacio.".to_string());
    }

    if let Some(ref skybox) = preset.skybox {
        validate_skybox(skybox, &preset.id, &mut result);
    }

    if preset.layers.is_empty() {
        result.add_error(format!(
            "El preset '{}' no contiene ninguna capa ('layers' esta vacio).",
            preset.id
        ));
        return result;
    }

    let mut seen_ids: HashMap<String, usize> = HashMap::new();
    let mut seen_depths: HashMap<i32, String> = HashMap::new();

    for (i, layer) in preset.layers.iter().enumerate() {
        validate_layer(layer, i, &preset.id, &mut seen_ids, &mut seen_depths, &mut result);
    }

    result
}

/// Validate a skybox configuration.
///
/// Mirrors `BackgroundValidator._validate_skybox()` from GDScript.
pub fn validate_skybox(
    skybox: &SkyboxConfig,
    preset_id: &str,
    result: &mut ValidationResult,
) {
    match skybox.mode {
        SkyboxMode::Gradient => {
            match &skybox.gradient {
                None => {
                    result.add_error(format!(
                        "El skybox del preset '{}' tiene mode=gradient pero no declara 'gradient'.",
                        preset_id
                    ));
                }
                Some(g) => {
                    // Mirrors GDScript: validates PRESENCE of keys, not values.
                    // GDScript: `not sky.gradient.has("zenith_color") or not sky.gradient.has("horizon_color")`
                    if g.zenith_color.is_none() || g.horizon_color.is_none() {
                        result.add_error(format!(
                            "El skybox del preset '{}' requiere al menos 'zenith_color' y 'horizon_color' en gradient.",
                            preset_id
                        ));
                    }
                }
            }
        }
        SkyboxMode::Texture => {
            if skybox.texture_path.is_empty() {
                result.add_error(format!(
                    "El skybox del preset '{}' tiene mode=texture pero no especifica 'texture'.",
                    preset_id
                ));
            }
            // Note: path existence check (ResourceLoader.exists) is Godot-only;
            // the GDScript layer performs that check as a complement.
        }
    }

    if skybox.distance <= 0.0 {
        result.add_error(format!(
            "El skybox del preset '{}' tiene distance invalida ({:.1}). Debe ser > 0.0.",
            preset_id, skybox.distance
        ));
    }

    if skybox.pixel_size < 0.0 {
        result.add_error(format!(
            "El skybox del preset '{}' tiene pixel_size negativo ({:.4}). Debe ser >= 0 (0 = auto).",
            preset_id, skybox.pixel_size
        ));
    }
}

/// Validate a single layer configuration.
///
/// Mirrors the per-layer validation loop in `BackgroundValidator.validate_preset()`.
pub fn validate_layer(
    layer: &LayerConfig,
    index: usize,
    _preset_id: &str,
    seen_ids: &mut HashMap<String, usize>,
    seen_depths: &mut HashMap<i32, String>,
    result: &mut ValidationResult,
) {
    let layer_context: String;

    if layer.id.is_empty() {
        layer_context = format!("Capa #{}", index);
        result.add_error(format!(
            "{} no declara un 'id' obligatorio.",
            layer_context
        ));
        return;
    } else {
        layer_context = format!("Capa '{}' (index {})", layer.id, index);
        if let Some(&dup_index) = seen_ids.get(&layer.id) {
            result.add_error(format!(
                "{} duplica el identificador de capa '{}' (ya usado en index {}).",
                layer_context, layer.id, dup_index
            ));
        } else {
            seen_ids.insert(layer.id.clone(), index);
        }
    }

    // Validate depth
    if layer.depth < 0 {
        result.add_error(format!(
            "{} tiene un depth negativo ({}). Los depths deben ser >= 0.",
            layer_context, layer.depth
        ));
    } else if let Some(dup_id) = seen_depths.get(&layer.depth) {
        result.add_error(format!(
            "{} tiene depth duplicado ({}) ya usado por '{}'.",
            layer_context, layer.depth, dup_id
        ));
    } else {
        seen_depths.insert(layer.depth, layer.id.clone());
    }

    // Validate texture or shader path
    if layer.procedural {
        if layer.shader_path.is_empty() {
            result.add_error(format!(
                "{} es procedural pero no especifica 'shader_path'.",
                layer_context
            ));
        } else if !layer.shader_path.ends_with(".gdshader") {
            result.add_error(format!(
                "{} especifica un archivo que no es .gdshader: '{}'.",
                layer_context, layer.shader_path
            ));
        }
        // Note: path existence check is Godot-only (ResourceLoader.exists).
    } else if layer.texture_path.is_empty() {
        result.add_error(format!(
            "{} no especifica 'texture_path' o 'texture'.",
            layer_context
        ));
        // Note: path existence check is Godot-only.
    }

    // Validate positive scale
    if layer.scale.x <= 0.0 || layer.scale.y <= 0.0 {
        result.add_error(format!(
            "{} tiene escala invalida o no positiva ({}, {}). La escala debe ser > 0.0.",
            layer_context, layer.scale.x, layer.scale.y
        ));
    }

    // Validate pixel_size positive (procedural layers use 0 = auto in runtime)
    if !layer.procedural && layer.pixel_size <= 0.0 {
        result.add_error(format!(
            "{} tiene pixel_size invalido o no positivo ({:.4}). Debe ser > 0.0.",
            layer_context, layer.pixel_size
        ));
    }

    // Validate finite numeric values
    if layer.parallax_x.is_nan()
        || layer.parallax_x.is_infinite()
        || layer.parallax_y.is_nan()
        || layer.parallax_y.is_infinite()
    {
        result.add_error(format!(
            "{} tiene valores no finitos de parallax (x={:.2}, y={:.2}).",
            layer_context, layer.parallax_x, layer.parallax_y
        ));
    }

    if layer.offset_x.is_nan()
        || layer.offset_x.is_infinite()
        || layer.offset_y.is_nan()
        || layer.offset_y.is_infinite()
    {
        result.add_error(format!(
            "{} tiene valores no finitos de offset (x={:.2}, y={:.2}).",
            layer_context, layer.offset_x, layer.offset_y
        ));
    }

    // Validate distance behind camera (Z < 0)
    let dz = layer.distance_z.unwrap_or(-800.0);
    if dz >= 0.0 {
        result.add_error(format!(
            "{} tiene distance_z >= 0 ({:.1}). Las capas de background deben estar detras de la camara (Z < 0).",
            layer_context, dz
        ));
    }

    // Validate non-negative parallax
    if layer.parallax_x < 0.0 || layer.parallax_y < 0.0 {
        result.add_error(format!(
            "{} tiene parallax negativo (x={:.2}, y={:.2}). Los valores deben ser >= 0.",
            layer_context, layer.parallax_x, layer.parallax_y
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GradientConfig, Scale2, SkyboxConfig};

    fn make_layer(id: &str, depth: i32) -> LayerConfig {
        LayerConfig {
            id: id.to_string(),
            texture_path: "res://test.png".to_string(),
            depth,
            parallax_x: 0.08,
            parallax_y: 0.01,
            scale: Scale2 { x: 1.0, y: 1.0 },
            offset_x: 0.0,
            offset_y: 0.0,
            repeat_x: false,
            repeat_y: false,
            pixel_snap: true,
            distance_z: Some(-700.0),
            pixel_size: 1.0,
            procedural: false,
            shader_path: String::new(),
            uniforms: HashMap::new(),
        }
    }

    fn make_preset(layers: Vec<LayerConfig>) -> BackgroundPreset {
        BackgroundPreset {
            id: "test".to_string(),
            display_name: "Test".to_string(),
            skybox: None,
            layers,
        }
    }

    #[test]
    fn valid_preset_passes() {
        let preset = make_preset(vec![
            make_layer("sky", 0),
            make_layer("far", 1),
            make_layer("near", 2),
        ]);
        let result = validate_preset(&preset);
        assert!(result.is_valid, "Expected valid, got: {}", result.get_error_summary());
    }

    #[test]
    fn empty_id_fails() {
        let mut preset = make_preset(vec![make_layer("sky", 0)]);
        preset.id = String::new();
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("id")));
    }

    #[test]
    fn empty_layers_fails() {
        let preset = make_preset(vec![]);
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("vacio")));
    }

    #[test]
    fn duplicate_layer_id_fails() {
        let preset = make_preset(vec![make_layer("sky", 0), make_layer("sky", 1)]);
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("duplica")));
    }

    #[test]
    fn duplicate_depth_fails() {
        let preset = make_preset(vec![make_layer("sky", 0), make_layer("far", 0)]);
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("depth duplicado")));
    }

    #[test]
    fn missing_texture_path_fails() {
        let mut layer = make_layer("sky", 0);
        layer.texture_path = String::new();
        let preset = make_preset(vec![layer]);
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
    }

    #[test]
    fn non_positive_scale_fails() {
        let mut layer = make_layer("sky", 0);
        layer.scale = Scale2 { x: -1.0, y: 1.0 };
        let preset = make_preset(vec![layer]);
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("escala")));
    }

    #[test]
    fn positive_distance_z_fails() {
        let mut layer = make_layer("sky", 0);
        layer.distance_z = Some(100.0);
        let preset = make_preset(vec![layer]);
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("distance_z >= 0")));
    }

    #[test]
    fn negative_parallax_fails() {
        let mut layer = make_layer("sky", 0);
        layer.parallax_x = -0.1;
        let preset = make_preset(vec![layer]);
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("parallax negativo")));
    }

    #[test]
    fn skybox_gradient_requires_colors() {
        let mut preset = make_preset(vec![make_layer("sky", 0)]);
        preset.skybox = Some(SkyboxConfig {
            mode: SkyboxMode::Gradient,
            distance: 800.0,
            gradient: None,
            ..Default::default()
        });
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("gradient")));
    }

    #[test]
    fn skybox_gradient_missing_horizon_color_fails() {
        let mut preset = make_preset(vec![make_layer("sky", 0)]);
        preset.skybox = Some(SkyboxConfig {
            mode: SkyboxMode::Gradient,
            distance: 800.0,
            gradient: Some(GradientConfig {
                zenith_color: Some([0.18, 0.42, 0.82]),
                horizon_color: None, // missing!
                ground_color: [0.72, 0.68, 0.55],
                horizon_sharpness: 0.65,
                ground_start: 0.48,
            }),
            ..Default::default()
        });
        let result = validate_preset(&preset);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("zenith_color") && e.contains("horizon_color")));
    }

    #[test]
    fn skybox_gradient_both_colors_present_passes() {
        let mut preset = make_preset(vec![make_layer("sky", 0)]);
        preset.skybox = Some(SkyboxConfig {
            mode: SkyboxMode::Gradient,
            distance: 800.0,
            gradient: Some(GradientConfig {
                zenith_color: Some([0.18, 0.42, 0.82]),
                horizon_color: Some([0.95, 0.92, 0.82]),
                ground_color: [0.72, 0.68, 0.55],
                horizon_sharpness: 0.65,
                ground_start: 0.48,
            }),
            ..Default::default()
        });
        let result = validate_preset(&preset);
        assert!(result.is_valid, "Expected valid, got: {}", result.get_error_summary());
    }

    #[test]
    fn procedural_layer_accepted() {
        let mut layer = make_layer("clouds", 1);
        layer.procedural = true;
        layer.shader_path = "res://addons/formula90s/shaders/cloud_layer.gdshader".to_string();
        layer.pixel_size = 0.0;
        let preset = make_preset(vec![make_layer("far", 0), layer]);
        let result = validate_preset(&preset);
        assert!(result.is_valid, "Expected valid, got: {}", result.get_error_summary());
    }
}
