use serde::{Deserialize, Serialize};

use super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspensionConfiguration {
    #[serde(default)]
    pub use_authored_wheel_centers: bool,
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
    /// Simplified-backend virtual unsprung mass scale over the resolved corner
    /// mass. Absent or 1.0 keeps the resolved value.
    #[serde(default = "default_virtual_unsprung_mass_multiplier")]
    pub virtual_unsprung_mass_multiplier: f64,
}

fn default_virtual_unsprung_mass_multiplier() -> f64 {
    1.0
}

impl SuspensionConfiguration {
    /// Helper to get suspension spring rate in N/m for a wheel based on rest target.
    pub fn calculate_spring_rate(
        &self,
        chassis: &ChassisConfiguration,
        tires: &TiresConfiguration,
        fuel: &FuelConfig,
        wheel: WheelIndex,
    ) -> f64 {
        let mass = chassis.mass_over_wheel(tires, fuel, wheel);
        let g = 9.80665;
        let spring_length = if wheel.is_front() {
            self.front_spring_length
        } else {
            self.rear_spring_length
        };
        let resting_ratio = if wheel.is_front() {
            self.front_resting_ratio
        } else {
            self.rear_resting_ratio
        };
        let rest_travel = spring_length * resting_ratio;
        if rest_travel > 1e-4 {
            (mass * g) / rest_travel
        } else {
            50000.0
        }
    }
}
