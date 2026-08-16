use crate::types::Vec3;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// Aerodynamic load distribution across 3 points (front wing, floor/diffuser, rear wing)
/// with progressive 4-pillar model: smoothstep blend, yaw decay, flex, and lag.
fn default_yaw_decay() -> f64 { 1.0 }
fn default_flex() -> f64 { 1.0 }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AeroForces {
    pub total_downforce: f64,    // N (downward) — lag-filtered
    pub front_downforce: f64,    // N (front wing, 15% canon)
    pub diffuser_downforce: f64, // N (floor/diffuser, 60%)
    pub rear_downforce: f64,     // N (rear wing, 25%)
    pub drag_force: f64,         // N (opposing motion) — lag-filtered
    #[serde(default)]
    pub effective_cl: f64,       // diagnostic
    #[serde(default = "default_yaw_decay")]
    pub yaw_decay_factor: f64,   // diagnostic f_yaw
    #[serde(default)]
    pub blend_factor: f64,       // diagnostic S(v)
    #[serde(default = "default_flex")]
    pub flex_factor: f64,        // diagnostic f_flex
}

impl AeroForces {
    pub fn zero() -> Self {
        Self {
            total_downforce: 0.0,
            front_downforce: 0.0,
            diffuser_downforce: 0.0,
            rear_downforce: 0.0,
            drag_force: 0.0,
            effective_cl: 0.0,
            yaw_decay_factor: 1.0,
            blend_factor: 0.0,
            flex_factor: 1.0,
        }
    }

    /// Stateful progressive aero step. Preserves first-order lag across ticks.
    /// local_velocity: +X = right, +Y = up, -Z = forward (Godot convention).
    pub fn step(&mut self, config: &VehicleConfig, local_velocity: Vec3, dt: f64) {
        let v_fwd = (-local_velocity.z).max(0.0);
        let v_lat = local_velocity.x.abs();

        // Pillar B: Smoothstep Speed Blending
        let blend_min = config.aero_blend_min_speed;
        let blend_full = config.aero_blend_full_speed;
        let denom = (blend_full - blend_min).max(1e-6);
        let x = ((v_fwd - blend_min) / denom).clamp(0.0, 1.0);
        let s = 3.0 * x * x - 2.0 * x * x * x;

        // Pillar C: Sideslip / Yaw Angle Attenuation
        let cos_beta = v_fwd / (v_fwd * v_fwd + v_lat * v_lat + 1e-6).sqrt();
        let cos_clamped = cos_beta.clamp(0.0, 1.0);
        let f_yaw = cos_clamped.powf(config.aero_yaw_decay_exponent);

        // Pillar D: High-Speed Aeroelastic Wing Flex
        let f_flex = 1.0 / (1.0 + config.aero_flex_coefficient * v_fwd);

        // Stage 2: Effective Coefficients & Instantaneous Target Loads
        let cl_eff = config.coefficient_of_downforce * s * f_yaw * f_flex;
        let v_sq = v_fwd * v_fwd;
        let q = 0.5 * config.air_density * v_sq;
        let downforce_target = q * cl_eff * config.frontal_area;
        let drag_target = q * config.coefficient_of_drag * config.frontal_area;

        // Stage 3: Dynamic Aerodynamic Lag / Flow Relaxation (Pillar A) — first-order low-pass
        let tau = config.aero_lag_tau.max(1e-4);
        let alpha = (dt / tau).clamp(0.0, 1.0);
        self.total_downforce += alpha * (downforce_target - self.total_downforce);
        self.drag_force += alpha * (drag_target - self.drag_force);

        // Stage 4: 3-Point Distribution (clamped filtered total)
        self.front_downforce = self.total_downforce * config.aero_split_front;
        self.diffuser_downforce = self.total_downforce * config.aero_split_diffuser;
        self.rear_downforce = self.total_downforce * config.aero_split_rear;

        // Diagnostics (rust-internal only, not FFI-exposed per gate decision)
        self.effective_cl = cl_eff;
        self.yaw_decay_factor = f_yaw;
        self.blend_factor = s;
        self.flex_factor = f_flex;
    }
}
