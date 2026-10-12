use serde::{Deserialize, Serialize};

use super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AerodynamicsConfiguration {
    pub coefficient_of_drag: f64,
    pub frontal_area: f64,
    #[serde(default)]
    pub aero_model: AeroModelConfig,
    pub air_density: f64,
    #[serde(default = "default_aero_lag_tau")]
    pub aero_lag_tau: f64,
    #[serde(default = "default_aero_blend_min_speed")]
    pub aero_blend_min_speed: f64,
    #[serde(default = "default_aero_blend_full_speed")]
    pub aero_blend_full_speed: f64,
}

pub(crate) fn default_aero_lag_tau() -> f64 {
    0.030
}

pub(crate) fn default_aero_blend_min_speed() -> f64 {
    5.556
}

pub(crate) fn default_aero_blend_full_speed() -> f64 {
    41.667
}
