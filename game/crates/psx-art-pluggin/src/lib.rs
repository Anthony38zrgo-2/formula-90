//! Formula-90 PSX/Retro Art Engine Core (pure Rust).
//!
//! Deterministic visual preset parsing, validation, dithering LUT generation,
//! and C-ABI bridge for Godot 4 GDExtension.
//!
//! Modules:
//! - [`types`]: Data models (`PsxArtPreset`, `DisplayProfile`, `GeometryProfile`, ...).
//! - [`preset`]: JSON serialization, deserialization, and error handling.
//! - [`validation`]: Strict domain validation rules.
//! - [`dither`]: Bayer matrix and dither LUT calculations.
//! - [`c_abi`]: Stable `#[repr(C)]` FFI bridge.

pub mod c_abi;
pub mod dither;
pub mod preset;
pub mod types;
pub mod validation;

pub use c_abi::{psx_art_get_dither_matrix, psx_art_parse_preset, psx_art_version, CPsxArtConfig};
pub use dither::{generate_bayer_8x8, get_dither_matrix, BAYER_2X2, BAYER_4X4};
pub use preset::{parse_preset_json, to_preset_json, PresetError};
pub use types::{
    ColorProfile, CrtProfile, DisplayProfile, DitherMatrixType, FogProfile, GeometryProfile,
    PsxArtPreset, UpscaleMode,
};
pub use validation::{validate_preset, ValidationError};
