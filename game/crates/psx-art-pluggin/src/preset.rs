//! JSON serialization and parsing for retro art presets.

use crate::types::PsxArtPreset;
use crate::validation::{validate_preset, ValidationError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PresetError {
    #[error("Failed to parse JSON preset: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),
}

/// Parses a retro art preset from JSON string and runs strict validation.
pub fn parse_preset_json(json_str: &str) -> Result<PsxArtPreset, PresetError> {
    let preset: PsxArtPreset = serde_json::from_str(json_str)?;
    validate_preset(&preset)?;
    Ok(preset)
}

/// Serializes a retro art preset to a pretty-printed JSON string.
pub fn to_preset_json(preset: &PsxArtPreset) -> Result<String, PresetError> {
    validate_preset(preset)?;
    let json = serde_json::to_string_pretty(preset)?;
    Ok(json)
}
