//! Formula-90 background/skybox runtime core (pure Rust).
//!
//! Deterministic preset validation, parallax math, pixel-size calculation,
//! and gradient uniform computation. No Godot, no rendering, no I/O in this core.
//!
//! Modules:
//! - [`types`]: data models (`BackgroundPreset`, `LayerConfig`, `SkyboxConfig`, ...).
//! - [`preset`]: JSON parsing and serialization.
//! - [`validation`]: preset/layer/skybox validation rules.
//! - [`parallax`]: parallax offset, pixel snap, modular wrap, tiling.
//! - [`skybox`]: pixel-size-from-camera, gradient uniform computation.
//!
//! The thin Godot controllers (`background_controller.gd`, `background_skybox.gd`)
//! read camera state and drive Sprite3D nodes; all background rules live here.

pub mod asset_validation;
pub mod parallax;
pub mod preset;
pub mod skybox;
pub mod types;
pub mod validation;

pub use asset_validation::{
    parse_manifest, validate_manifest, GenerationParams, ManifestError, MountainAssetManifest,
    ValidationData, WaterfallData,
};

pub use parallax::{compute_parallax, modular_wrap, pixel_snap, sprite_width, tile_offsets};
pub use preset::{parse_preset_json, to_json, PresetError};
pub use skybox::{gradient_uniforms, pixel_size_from_camera, GradientUniforms, COVERAGE_FACTOR, PIXEL_SIZE_SNAP};
pub use types::{
    BackgroundPreset, CameraState, GradientConfig, KeepAspect, LayerConfig, SkyboxConfig,
    SkyboxMode, UniformValue, Vec3, Vec4,
};
pub use validation::{validate_layer, validate_preset, validate_skybox, ValidationResult};

/// The default relative path to the background presets from the game/ directory.
pub const DEFAULT_PRESETS_REL: &str = "assets/backgrounds";
