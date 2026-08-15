//! Data models for the Formula-90 background/skybox system.
//!
//! Mirrors `BackgroundPreset`, `BackgroundSkyboxConfig`, `BackgroundLayerConfig`,
//! and related types from the GDScript layer. All types are serializable via serde.

use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

/// 3-component float vector (mirrors GDScript Vector3).
pub type Vec3 = [f32; 3];

/// 4-component float vector (mirrors GDScript Vector4).
pub type Vec4 = [f32; 4];

/// Skybox rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkyboxMode {
    /// Static PNG texture.
    Texture,
    /// Procedural gradient via shader uniforms.
    Gradient,
}

impl Default for SkyboxMode {
    fn default() -> Self {
        Self::Texture
    }
}

/// Camera aspect-ratio keep mode (mirrors Godot Camera3D.KEEP_HEIGHT / KEEP_WIDTH).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAspect {
    /// `camera.fov` is vertical FOV (Godot default).
    Height,
    /// `camera.fov` is horizontal FOV.
    Width,
}

/// Uniform value for procedural shader layers.
///
/// GDScript `_apply_uniforms` accepts:
/// - Array size 3 → Vector3
/// - Array size 4 → Vector4
/// - float/int → float
/// - bool → bool
/// - Anything else → silently ignored
///
/// The `Ignored` variant captures non-3/4 arrays and other types that
/// GDScript would silently skip, preventing parse failures on harmless data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UniformValue {
    Vec3(Vec3),
    Vec4(Vec4),
    Float(f64),
    Bool(bool),
    /// Catch-all for values GDScript would silently ignore (e.g. [1.0, 2.0]).
    Ignored(serde_json::Value),
}

/// Gradient configuration for procedural skybox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientConfig {
    /// `None` = key absent in JSON. GDScript validates presence at validation time.
    #[serde(default)]
    pub zenith_color: Option<Vec3>,
    /// `None` = key absent in JSON. GDScript validates presence at validation time.
    #[serde(default)]
    pub horizon_color: Option<Vec3>,
    #[serde(default = "default_ground_color")]
    pub ground_color: Vec3,
    #[serde(default = "default_horizon_sharpness")]
    pub horizon_sharpness: f32,
    #[serde(default = "default_ground_start")]
    pub ground_start: f32,
}

fn default_ground_color() -> Vec3 {
    [0.72, 0.68, 0.55]
}
fn default_horizon_sharpness() -> f32 {
    0.65
}
fn default_ground_start() -> f32 {
    0.48
}

/// Skybox configuration (mirrors `BackgroundSkyboxConfig`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkyboxConfig {
    #[serde(default)]
    pub mode: SkyboxMode,
    #[serde(default, alias = "texture")]
    pub texture_path: String,
    #[serde(default = "default_skybox_distance")]
    pub distance: f64,
    #[serde(default)]
    pub pixel_size: f64,
    #[serde(default)]
    pub gradient: Option<GradientConfig>,
    #[serde(default = "default_time_of_day")]
    pub time_of_day: f64,
}

fn default_skybox_distance() -> f64 {
    800.0
}
fn default_time_of_day() -> f64 {
    11.0
}

impl Default for SkyboxConfig {
    fn default() -> Self {
        Self {
            mode: SkyboxMode::Texture,
            texture_path: String::new(),
            distance: 800.0,
            pixel_size: 0.0,
            gradient: None,
            time_of_day: 11.0,
        }
    }
}

/// Scale as a 2D tuple (x, y).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale2 {
    pub x: f64,
    pub y: f64,
}

impl Default for Scale2 {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}

impl Serialize for Scale2 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("x", &self.x)?;
        map.serialize_entry("y", &self.y)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for Scale2 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ScaleVisitor;

        impl<'de> serde::de::Visitor<'de> for ScaleVisitor {
            type Value = Scale2;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a number, [x, y] array, or {x, y} object")
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Scale2, E> {
                Ok(Scale2 { x: v, y: v })
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Scale2, E> {
                Ok(Scale2 {
                    x: v as f64,
                    y: v as f64,
                })
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Scale2, E> {
                Ok(Scale2 {
                    x: v as f64,
                    y: v as f64,
                })
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Scale2, A::Error> {
                let x: f64 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let y: f64 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(Scale2 { x, y })
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Scale2, M::Error> {
                let mut x = 1.0;
                let mut y = 1.0;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "x" => x = map.next_value()?,
                        "y" => y = map.next_value()?,
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(Scale2 { x, y })
            }
        }

        deserializer.deserialize_any(ScaleVisitor)
    }
}

/// Individual background layer configuration (mirrors `BackgroundLayerConfig`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerConfig {
    pub id: String,
    #[serde(default, alias = "texture")]
    pub texture_path: String,
    #[serde(default)]
    pub depth: i32,
    #[serde(default)]
    pub parallax_x: f64,
    #[serde(default)]
    pub parallax_y: f64,
    #[serde(default = "default_scale")]
    pub scale: Scale2,
    #[serde(default)]
    pub offset_x: f64,
    #[serde(default)]
    pub offset_y: f64,
    #[serde(default)]
    pub repeat_x: bool,
    #[serde(default)]
    pub repeat_y: bool,
    #[serde(default = "default_true")]
    pub pixel_snap: bool,
    /// Z distance from camera. Negative = behind camera.
    /// `None` = field absent in JSON → apply depth-based default in preset.rs.
    /// `Some(v)` = explicitly set in JSON → preserve exact value.
    /// After `parse_preset_json()`, this is always `Some(f64)`.
    /// Default: -800.0 + depth * 100.0 (applied in preset.rs when None).
    #[serde(default, deserialize_with = "deserialize_distance_z")]
    pub distance_z: Option<f64>,
    #[serde(default = "default_pixel_size")]
    pub pixel_size: f64,
    #[serde(default)]
    pub procedural: bool,
    #[serde(default)]
    pub shader_path: String,
    #[serde(default)]
    pub uniforms: HashMap<String, UniformValue>,
}

fn default_scale() -> Scale2 {
    Scale2::default()
}
fn default_true() -> bool {
    true
}
fn default_pixel_size() -> f64 {
    0.5
}

/// Custom deserializer for distance_z: missing field → None, present → Some(value).
/// This lets preset.rs distinguish "absent" from "explicit -800.0".
fn deserialize_distance_z<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<f64>, D::Error> {
    Option::<f64>::deserialize(deserializer)
}

/// Complete background preset (mirrors `BackgroundPreset`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundPreset {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skybox: Option<SkyboxConfig>,
    #[serde(default)]
    pub layers: Vec<LayerConfig>,
}

/// Camera state snapshot for parallax/pixel-size computation.
#[derive(Debug, Clone, Copy)]
pub struct CameraState {
    /// Vertical or horizontal FOV in degrees (depends on keep_aspect).
    pub fov_deg: f64,
    /// Whether fov_deg is vertical (Height) or horizontal (Width).
    pub keep_aspect: KeepAspect,
    /// Viewport dimensions in pixels (width, height).
    pub viewport_size: (f64, f64),
    /// Camera yaw in radians (Y-axis rotation).
    pub yaw_rad: f64,
    /// Camera pitch in radians (X-axis rotation).
    pub pitch_rad: f64,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            fov_deg: 70.0,
            keep_aspect: KeepAspect::Height,
            viewport_size: (1280.0, 720.0),
            yaw_rad: 0.0,
            pitch_rad: 0.0,
        }
    }
}
