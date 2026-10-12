use serde::{Deserialize, Serialize};

use super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspensionConfiguration {
    pub front_spring_length: f64,
    pub front_resting_ratio: f64,
    pub front_damping_ratio: f64,
    pub front_bump_damp_multiplier: f64,
    pub front_rebound_damp_multiplier: f64,
    pub front_arb_ratio: f64,
    pub front_toe: f64,
    pub front_camber: f64,
    pub front_bump_stop_multiplier: f64,
    pub rear_spring_length: f64,
    pub rear_resting_ratio: f64,
    pub rear_damping_ratio: f64,
    pub rear_bump_damp_multiplier: f64,
    pub rear_rebound_damp_multiplier: f64,
    pub rear_arb_ratio: f64,
    pub rear_toe: f64,
    pub rear_camber: f64,
    pub rear_bump_stop_multiplier: f64,
    pub tri_ray_spacing_ratio: f64,
    /// Wheel-kinematic camber change (rad) per meter of suspension travel relative
    /// to static/rest compression (see `rest_compression_m`). Positive gain means a
    /// bump (compression > rest, e.g. +0.02 m travel) shifts camber toward positive
    /// by the gain times the travel; a rebound (compression < rest, e.g. -0.01 m)
    /// shifts it negative. Expressed in the same sign convention as axle `camber`
    /// (the tire model mirrors the right-hand wheels). Default 0.0 (legacy parity).
    #[serde(default)]
    pub front_camber_gain_rad_per_m: f64,
    /// Same sign convention and bump/rebound behaviour as `front_camber_gain_rad_per_m`.
    #[serde(default)]
    pub rear_camber_gain_rad_per_m: f64,
    /// Effective toe change (rad) per meter of suspension travel relative to static/rest
    /// compression (bump steer). Positive gain means a bump (e.g. +0.02 m) increases the
    /// effective toe (and the mirrored per-wheel steer angle) by the gain times the
    /// travel; rebound inverts it. Left wheels negate the signed result, so symmetric
    /// travel keeps the axle neutral. Applies even when the axle steering ratio is 0.
    /// Default 0.0 (legacy parity).
    #[serde(default)]
    pub front_toe_gain_rad_per_m: f64,
    /// Same sign convention and bump/rebound behaviour as `front_toe_gain_rad_per_m`.
    #[serde(default)]
    pub rear_toe_gain_rad_per_m: f64,
    /// SUS-GEO-02 model selector. Absent means legacy 1-DOF.
    #[serde(default)]
    pub suspension_model: SuspensionModelKind,
    /// SUS-GEO-02 model version. Must be 1.
    #[serde(default = "default_suspension_model_version")]
    pub suspension_model_version: u32,
    /// SUS-GEO-02 physical geometry. Required for Geometric, None for Legacy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometric_suspension: Option<GeometricSuspensionConfig>,
}
