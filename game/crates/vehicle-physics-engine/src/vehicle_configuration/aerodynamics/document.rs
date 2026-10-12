use super::*;
use super::configuration::{default_aero_blend_full_speed, default_aero_blend_min_speed, default_aero_lag_tau};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonAero {
    #[serde(default)]
    pub(crate) model: AeroModelConfig,
    #[serde(default = "default_cd")]
    pub(crate) drag_coefficient: f64,
    #[serde(default = "default_area")]
    pub(crate) frontal_area: f64,
    #[serde(default = "default_air_density")]
    pub(crate) air_density: f64,
    #[serde(default = "default_lag_tau")]
    pub(crate) lag_tau_s: f64,
    #[serde(default = "default_blend_min")]
    pub(crate) blend_min_speed_mps: f64,
    #[serde(default = "default_blend_full")]
    pub(crate) blend_full_speed_mps: f64,
}
impl Default for JsonAero {
    fn default() -> Self {
        Self {
            model: AeroModelConfig::default(),
            drag_coefficient: 0.78,
            frontal_area: 1.25,
            air_density: 1.225,
            lag_tau_s: default_aero_lag_tau(),
            blend_min_speed_mps: default_aero_blend_min_speed(),
            blend_full_speed_mps: default_aero_blend_full_speed(),
        }
    }
}
pub(crate) fn default_cd() -> f64 {
    0.78
}
pub(crate) fn default_area() -> f64 {
    1.25
}
pub(crate) fn default_air_density() -> f64 {
    1.225
}
pub(crate) fn default_lag_tau() -> f64 {
    default_aero_lag_tau()
}
pub(crate) fn default_blend_min() -> f64 {
    default_aero_blend_min_speed()
}
pub(crate) fn default_blend_full() -> f64 {
    default_aero_blend_full_speed()
}

// ── Driving aids (schema v2) ────────────────────────────────────────────────────
