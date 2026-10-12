use serde::{Deserialize, Serialize};

pub const GRAND_PRIX_SAMPLER_DEFAULT_MANIFEST: &str = "sounds/banks/v10-v2-bank/manifest.json";
pub const V10_GF509_DEFAULT_MANIFEST: &str = "audio/v10_gf509/manifest.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CppDspDocument {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub gain: f64,
}

impl Default for CppDspDocument {
    fn default() -> Self {
        Self {
            enabled: false,
            gain: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GrandPrixSamplerDocument {
    pub manifest: String,
    pub schema_version: u32,
    pub engine_gain: f64,
    pub gearbox_gain: f64,
    pub gearbox_whine_gain: f64,
    pub gearbox_whine_tone_gain: f64,
    pub gearbox_whine_noise_gain: f64,
    pub backfire_gain: f64,
    pub limiter_gain: f64,
    pub coast_gain: Option<f64>,
    pub fallback_to_legacy_on_initialization_error: bool,
}

impl Default for GrandPrixSamplerDocument {
    fn default() -> Self {
        Self {
            manifest: GRAND_PRIX_SAMPLER_DEFAULT_MANIFEST.to_string(),
            schema_version: 1,
            engine_gain: 1.0,
            gearbox_gain: 1.0,
            gearbox_whine_gain: 0.0,
            gearbox_whine_tone_gain: 0.12,
            gearbox_whine_noise_gain: 0.025,
            backfire_gain: 1.0,
            limiter_gain: 1.0,
            coast_gain: None,
            fallback_to_legacy_on_initialization_error: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleAudioDocument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuous_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpp_dsp: Option<CppDspDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gf509: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grand_prix_sampler: Option<GrandPrixSamplerDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub powertrain_synthesis: Option<serde_json::Value>,
}

impl Default for VehicleAudioDocument {
    fn default() -> Self {
        Self {
            continuous_source: None,
            cpp_dsp: None,
            gf509: None,
            grand_prix_sampler: None,
            powertrain_synthesis: None,
        }
    }
}
