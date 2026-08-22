//! Validation rules for PSX/Retro art presets.

use crate::types::PsxArtPreset;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("Profile name cannot be empty")]
    EmptyProfileName,

    #[error("Internal width must be between 1 and 7680 (got {0})")]
    InvalidWidth(u32),

    #[error("Internal height must be between 1 and 4320 (got {0})")]
    InvalidHeight(u32),

    #[error("Bit depth must be between 1 and 8 (got {0})")]
    InvalidBitDepth(u32),

    #[error("Dither strength must be between 0.0 and 2.0 (got {0})")]
    InvalidDitherStrength(f32),

    #[error("Vertex snap distance must be >= 0.0 (got {0})")]
    NegativeVertexSnap(f32),

    #[error("Affine texture strength must be between 0.0 and 1.0 (got {0})")]
    InvalidAffineStrength(f32),

    #[error("Fog near distance must be >= 0.0 (got {0})")]
    NegativeFogNear(f32),

    #[error("Fog far distance ({far}) must be greater than near distance ({near})")]
    InvalidFogRange { near: f32, far: f32 },

    #[error("Fog color component {index} out of range [0.0, 1.0]: {val}")]
    InvalidFogColor { index: usize, val: f32 },

    #[error("Scanline opacity must be between 0.0 and 1.0 (got {0})")]
    InvalidScanlineOpacity(f32),

    #[error("Composite bleed must be between 0.0 and 1.0 (got {0})")]
    InvalidCompositeBleed(f32),
}

/// Validates all fields of a preset according to hardware domain rules.
pub fn validate_preset(preset: &PsxArtPreset) -> Result<(), ValidationError> {
    if preset.profile_name.trim().is_empty() {
        return Err(ValidationError::EmptyProfileName);
    }

    if preset.display.internal_width == 0 || preset.display.internal_width > 7680 {
        return Err(ValidationError::InvalidWidth(preset.display.internal_width));
    }
    if preset.display.internal_height == 0 || preset.display.internal_height > 4320 {
        return Err(ValidationError::InvalidHeight(preset.display.internal_height));
    }

    if preset.color.bit_depth < 1 || preset.color.bit_depth > 8 {
        return Err(ValidationError::InvalidBitDepth(preset.color.bit_depth));
    }
    if preset.color.dither_strength < 0.0 || preset.color.dither_strength > 2.0 {
        return Err(ValidationError::InvalidDitherStrength(preset.color.dither_strength));
    }

    if preset.geometry.vertex_snap_distance < 0.0 {
        return Err(ValidationError::NegativeVertexSnap(preset.geometry.vertex_snap_distance));
    }
    if !(0.0..=1.0).contains(&preset.geometry.affine_texture_strength) {
        return Err(ValidationError::InvalidAffineStrength(preset.geometry.affine_texture_strength));
    }

    if preset.fog.enabled {
        if preset.fog.near < 0.0 {
            return Err(ValidationError::NegativeFogNear(preset.fog.near));
        }
        if preset.fog.far <= preset.fog.near {
            return Err(ValidationError::InvalidFogRange {
                near: preset.fog.near,
                far: preset.fog.far,
            });
        }
        for (i, &c) in preset.fog.color.iter().enumerate() {
            if !(0.0..=1.0).contains(&c) {
                return Err(ValidationError::InvalidFogColor { index: i, val: c });
            }
        }
    }

    if !(0.0..=1.0).contains(&preset.crt_effects.scanline_opacity) {
        return Err(ValidationError::InvalidScanlineOpacity(preset.crt_effects.scanline_opacity));
    }
    if !(0.0..=1.0).contains(&preset.crt_effects.composite_bleed) {
        return Err(ValidationError::InvalidCompositeBleed(preset.crt_effects.composite_bleed));
    }

    Ok(())
}
