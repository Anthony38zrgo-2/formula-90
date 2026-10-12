use serde::{Deserialize, Serialize};

use super::super::*;

/// Schema v3 steady tire force curve (parser-only until TIRE-100/TIRE-200).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TireForceProfile {
    pub lateral_peak_slip_angle_rad: f64,
    pub longitudinal_peak_slip_ratio: f64,
    pub lateral_slide_mu_ratio: f64,
    pub longitudinal_slide_mu_ratio: f64,
    /// Half-width of the C1 blend band around the combined-slip budget knee
    /// (GRIP-01). `0.0` reproduces the legacy hard clip exactly; a positive value
    /// spreads the transition over `utilization in [1 - w, 1 + w]` and keeps the
    /// applied force inside the shared budget at all times.
    pub budget_blend_width: f64,
    /// Sharpness of the post-peak decay `d = sharpness * (u - 1)` (GRIP-02).
    /// `2.0` matches the legacy internal constant.
    pub falloff_sharpness: f64,
    /// Time constant (s) for the sliding-memory state to follow a rising
    /// post-peak demand. `0.0` disables the loss-side lag (legacy instant decay).
    pub slip_loss_tau_s: f64,
    /// Time constant (s) for the sliding-memory state to release once the tire
    /// is back inside the envelope. Larger than `slip_loss_tau_s` models the
    /// slower, progressive grip recovery (GRIP-02). `0.0` disables recovery lag.
    pub slip_recovery_tau_s: f64,
    /// Fraction of the lateral peak slip angle consumed by full longitudinal
    /// demand (combined-slip peak migration, GRIP-04). `0.0` keeps independent
    /// axes.
    pub combined_lateral_peak_migration: f64,
    /// Fraction of the longitudinal peak slip ratio consumed by full lateral
    /// demand. `0.0` keeps independent axes.
    pub combined_longitudinal_peak_migration: f64,
    /// Lower bound (s) for the carcass relaxation time constant (GRIP-05), so
    /// high-speed transients stay readable. `0.0` keeps pure distance-based tau.
    pub min_relaxation_tau_s: f64,
    /// Multiplier applied to the relaxation tau while an axis releases slip
    /// (effective slip above target), modelling slower grip recovery. `1.0`
    /// keeps the legacy symmetric relaxation.
    pub relaxation_recovery_scale: f64,
    /// Time constant (s) for the contact blend that ramps tire forces in and out
    /// when a wheel loses or regains load (GRIP-06). `0.0` keeps the legacy hard
    /// contact switch.
    pub contact_ramp_tau_s: f64,
}
impl Default for TireForceProfile {
    fn default() -> Self {
        Self {
            lateral_peak_slip_angle_rad: 0.10,
            longitudinal_peak_slip_ratio: 0.12,
            // 1.0 = no post-peak drop (matches current saturating tanh baseline).
            // TIRE-200 introduces an explicit post-peak decay via profile values.
            lateral_slide_mu_ratio: 1.0,
            longitudinal_slide_mu_ratio: 1.0,
            // Legacy parity: profiles that never opt in keep the exact hard clip.
            budget_blend_width: 0.0,
            falloff_sharpness: 2.0,
            // Legacy parity: zero taus make the sliding memory instant.
            slip_loss_tau_s: 0.0,
            slip_recovery_tau_s: 0.0,
            // Legacy parity: no combined-slip peak migration by default.
            combined_lateral_peak_migration: 0.0,
            combined_longitudinal_peak_migration: 0.0,
            // Legacy parity: pure distance-based, symmetric relaxation.
            min_relaxation_tau_s: 0.0,
            relaxation_recovery_scale: 1.0,
            // Legacy parity: hard contact switch.
            contact_ramp_tau_s: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiresConfiguration {
    pub front_tire_radius: f64,
    pub rear_tire_radius: f64,
    pub front_tire_width: f64, // meters
    pub rear_tire_width: f64,  // meters
    pub front_wheel_mass: f64,
    pub rear_wheel_mass: f64,
    // Global fallback (used when a per-axle value is omitted in JSON).
    pub contact_patch: f64,
    // Per-axle contact patch (front/rear).
    pub front_contact_patch: f64,
    pub rear_contact_patch: f64,
    pub front_airborne_decay: f64,
    pub rear_airborne_decay: f64,

    // Tire pressure + thermal (pressure-aware carcass + 5-node thermal model)
    pub tire_pressure: TirePressureConfig,
    pub tire_thermal: TireThermalAxleConfig,

    #[serde(default)]
    pub tire_wear: TireWearAxleConfig,

    // Schema v3: per-axle steady tire force envelope (consumed by tire.rs).
    #[serde(default)]
    pub front_tire_force: TireForceProfile,
    #[serde(default)]
    pub rear_tire_force: TireForceProfile,
    #[serde(default)]
    pub pressure_mechanics: PressureMechanicsSensitivity,

    // Surface properties
    pub surface_friction: HashMap<SurfaceType, f64>,
    pub surface_stiffness: HashMap<SurfaceType, f64>,
    pub surface_rolling_resistance: HashMap<SurfaceType, f64>,
    pub surface_longitudinal_grip_ratio: HashMap<SurfaceType, f64>,
    /// GRIP-06: first-order filter tau (s) for the per-wheel effective surface
    /// friction/stiffness/rolling values, so kerb and grass crossings are
    /// gradual. `0.0` keeps instantaneous surface steps (legacy).
    #[serde(default)]
    pub surface_transition_tau_s: f64,
}
