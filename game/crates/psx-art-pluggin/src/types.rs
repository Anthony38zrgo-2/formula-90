//! Data types and configuration models for PSX/Retro art profiles.

use serde::{Deserialize, Serialize};

/// Mode of upscaling and presentation on modern display viewports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UpscaleMode {
    /// Raw un-interpolated nearest-neighbor pixel output.
    #[default]
    PixelPerfect,
    /// Integer-prescaled bilinear filtering (sharp pixels without shimmering).
    SharpBilinear,
    /// CRT scanlines and soft phosphor blending emulation.
    CrtSmooth,
    /// Renders at native display resolution while snapping vertices and quantizing
    /// color on a virtual 320x240 grid (HD emulator style).
    NativeHiresPsx,
}

/// Dither matrix pattern type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DitherMatrixType {
    /// No dithering applied.
    None,
    /// 2x2 Bayer Matrix (4 levels).
    Bayer2x2,
    /// 4x4 Bayer Matrix (16 levels, authentic PSX hardware).
    #[default]
    Bayer4x4,
    /// 8x8 Bayer Matrix (64 levels).
    Bayer8x8,
}

/// Display resolution and framerate settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayProfile {
    /// Internal framebuffer width (e.g. 320 or 640).
    pub internal_width: u32,
    /// Internal framebuffer height (e.g. 240 or 360).
    pub internal_height: u32,
    /// Upscaling algorithm for presenting to the window.
    #[serde(default)]
    pub upscale_mode: UpscaleMode,
    /// Engine FPS limiter (0 = uncapped / monitor vsync, 30, 60).
    #[serde(default)]
    pub fps_cap: u32,
}

impl Default for DisplayProfile {
    fn default() -> Self {
        Self {
            internal_width: 320,
            internal_height: 240,
            upscale_mode: UpscaleMode::SharpBilinear,
            fps_cap: 30,
        }
    }
}

/// Vertex and polygon rasterization parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeometryProfile {
    /// Grid snap distance in view space for vertex jitter (0.0 = disabled, typical PSX: 0.25 - 0.5).
    pub vertex_snap_distance: f32,
    /// Affine texture interpolation strength (0.0 = perspective-correct, 1.0 = full PSX affine warp).
    pub affine_texture_strength: f32,
}

impl Default for GeometryProfile {
    fn default() -> Self {
        Self {
            vertex_snap_distance: 0.35,
            affine_texture_strength: 1.0,
        }
    }
}

/// Color depth and dithering parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorProfile {
    /// Bits per RGB color channel (5 = 15-bit RGB555 PSX authentic, 8 = full 24-bit).
    pub bit_depth: u32,
    /// Type of dither matrix to apply.
    #[serde(default)]
    pub dither_matrix_type: DitherMatrixType,
    /// Intensity multiplier for the dithering effect (0.0 = off, 1.0 = standard).
    #[serde(default = "default_one_f32")]
    pub dither_strength: f32,
}

fn default_one_f32() -> f32 {
    1.0
}

impl Default for ColorProfile {
    fn default() -> Self {
        Self {
            bit_depth: 5,
            dither_matrix_type: DitherMatrixType::Bayer4x4,
            dither_strength: 1.0,
        }
    }
}

/// Distance fog parameters (mimics PSX fixed-function hardware fog).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FogProfile {
    /// Whether retro fog is active.
    pub enabled: bool,
    /// RGBA fog color (0.0 to 1.0).
    pub color: [f32; 4],
    /// Near distance in meters where fog starts blending.
    pub near: f32,
    /// Far distance in meters where fog reaches 100% opacity.
    pub far: f32,
}

impl Default for FogProfile {
    fn default() -> Self {
        Self {
            enabled: true,
            color: [0.5, 0.5, 0.5, 0.0],
            near: 10.0,
            far: 40.0,
        }
    }
}

/// CRT and analog signal post-processing emulation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrtProfile {
    /// Whether scanline simulation is enabled.
    #[serde(default)]
    pub scanlines_enabled: bool,
    /// Opacity of horizontal scanlines (0.0 = none, 1.0 = full black lines).
    #[serde(default)]
    pub scanline_opacity: f32,
    /// Composite video horizontal chroma bleeding (0.0 = clean, 0.1 = noticeable analog bleed).
    #[serde(default)]
    pub composite_bleed: f32,
}

impl Default for CrtProfile {
    fn default() -> Self {
        Self {
            scanlines_enabled: false,
            scanline_opacity: 0.0,
            composite_bleed: 0.0,
        }
    }
}

/// Complete Retro Art Preset definition (Single Source of Truth).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PsxArtPreset {
    /// Descriptive name of the artistic profile.
    pub profile_name: String,
    /// Display resolution and upscaling settings.
    pub display: DisplayProfile,
    /// Vertex and geometry styling.
    pub geometry: GeometryProfile,
    /// Color precision and dithering.
    pub color: ColorProfile,
    /// Fog behavior.
    pub fog: FogProfile,
    /// CRT and analog signal artifacts.
    #[serde(default)]
    pub crt_effects: CrtProfile,
}
