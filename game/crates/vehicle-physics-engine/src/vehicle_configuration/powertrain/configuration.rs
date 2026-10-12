use serde::{Deserialize, Serialize};

use super::super::*;
use super::document::JsonAutomaticShift;

/// Automatic transmission shift logic parameters, sourced from `f1_94_physics.json`
/// (`automatic_shift`). JSON is authoritative; `Default` reproduces the F1-94 tuned values so
/// the canonical fallback behaves identically to the JSON-driven path. All 15 keys are consumed
/// by the gear-selection logic in `powertrain.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomaticShift {
    pub upshift_normalized_rpm_low_throttle: f64,
    pub upshift_normalized_rpm_full_throttle: f64,
    pub downshift_normalized_rpm_low_throttle: f64,
    pub downshift_normalized_rpm_full_throttle: f64,
    pub downshift_throttle_aggression_factor: f64,
    pub upshift_coast_normalized_rpm: f64,
    pub downshift_coast_normalized_rpm: f64,
    pub reverse_max_speed_ms: f64,
    pub parked_resume_speed_ms: f64,
    pub kickdown_delay_factor: f64,
    pub wheel_road_spin_blend_threshold_rads: f64,
    pub wheel_spin_weight: f64,
    pub road_spin_weight: f64,
    pub upshift_target_rpm_margin_high: f64,
    pub downshift_target_rpm_margin_high: f64,
}

impl Default for AutomaticShift {
    fn default() -> Self {
        Self {
            upshift_normalized_rpm_low_throttle: 0.72,
            upshift_normalized_rpm_full_throttle: 0.965,
            downshift_normalized_rpm_low_throttle: 0.42,
            downshift_normalized_rpm_full_throttle: 0.58,
            downshift_throttle_aggression_factor: 0.35,
            upshift_coast_normalized_rpm: 0.42,
            downshift_coast_normalized_rpm: 0.30,
            reverse_max_speed_ms: 6.0,
            parked_resume_speed_ms: 1.0,
            kickdown_delay_factor: 0.75,
            wheel_road_spin_blend_threshold_rads: 10.0,
            wheel_spin_weight: 0.60,
            road_spin_weight: 0.40,
            upshift_target_rpm_margin_high: 0.94,
            downshift_target_rpm_margin_high: 0.82,
        }
    }
}

impl AutomaticShift {
    pub(crate) fn from_json(j: &JsonAutomaticShift) -> Self {
        Self {
            upshift_normalized_rpm_low_throttle: j.upshift_normalized_rpm_low_throttle,
            upshift_normalized_rpm_full_throttle: j.upshift_normalized_rpm_full_throttle,
            downshift_normalized_rpm_low_throttle: j.downshift_normalized_rpm_low_throttle,
            downshift_normalized_rpm_full_throttle: j.downshift_normalized_rpm_full_throttle,
            downshift_throttle_aggression_factor: j.downshift_throttle_aggression_factor,
            upshift_coast_normalized_rpm: j.upshift_coast_normalized_rpm,
            downshift_coast_normalized_rpm: j.downshift_coast_normalized_rpm,
            reverse_max_speed_ms: j.reverse_max_speed_ms,
            parked_resume_speed_ms: j.parked_resume_speed_ms,
            kickdown_delay_factor: j.kickdown_delay_factor,
            wheel_road_spin_blend_threshold_rads: j.wheel_road_spin_blend_threshold_rads,
            wheel_spin_weight: j.wheel_spin_weight,
            road_spin_weight: j.road_spin_weight,
            upshift_target_rpm_margin_high: j.upshift_target_rpm_margin_high,
            downshift_target_rpm_margin_high: j.downshift_target_rpm_margin_high,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowertrainConfiguration {
    pub max_torque: f64,               // N·m
    pub max_rpm: f64,                  // RPM
    pub idle_rpm: f64,                 // RPM
    pub motor_moment: f64,             // kg·m²
    pub torque_curve: Vec<(f64, f64)>, // (normalized_rpm, normalized_torque)
    pub gear_ratios: Vec<f64>,
    pub final_drive: f64,
    pub reverse_ratio: f64,
    pub shift_time: f64, // seconds
    pub automatic_transmission: bool,
    pub downshift_rev_matching_enabled: bool,
    pub downshift_maximum_target_revolutions_per_minute_fraction: f64,
    pub downshift_blip_torque_fraction: f64,
    pub downshift_minimum_interval_seconds: f64,
    pub downshift_clutch_reengagement_seconds: f64,
    pub front_torque_split: f64, // 0.0 = pure RWD, 1.0 = pure FWD

    // Differential (Salisbury Clutch-Pack LSD, AMS2/Reiza aligned)
    pub diff_preload: f64,               // Static preload torque (N·m)
    pub diff_power_ramp_angle_deg: f64,  // Power lock ramp angle (degrees)
    pub diff_coast_ramp_angle_deg: f64,  // Coast lock ramp angle (degrees)
    pub diff_clutches: f64,              // Number of friction plate surfaces
    pub diff_clutch_friction_coeff: f64, // Clutch plate friction coefficient

    // Auto-clutch / launch (previously hardcoded in powertrain)
    pub max_clutch_torque_ratio: f64,
    pub clutch_out_rpm_offset: f64,
    pub idle_disengagement_hysteresis_rpm: f64,
    pub variable_drag_ratio: f64,
    pub constant_brake_ratio: f64,
    pub handbrake_torque_fraction: f64,

    // Engine torque attack limiter: caps the RISE of positive engine torque in Nm/s
    // so throttle no longer slams the drivetrain. 0.0 = disabled (legacy behavior).
    pub torque_attack_rate_nm_s: f64,

    // Differential slip transition threshold (was hardcoded 0.50)
    pub diff_slip_transition_threshold_rad_s: f64,

    // Automatic transmission shift logic (from f1_94_physics.json `automatic_shift`)
    pub automatic_shift: AutomaticShift,
    pub gear_inertia: f64,
    pub powertrain_thermal: PowertrainThermalConfig,
}

impl PowertrainConfiguration {
    /// Evaluates normalized engine torque from normalized RPM (0.0 to 1.0) using piecewise linear interpolation.
    pub fn evaluate_torque_curve(&self, normalized_rpm: f64) -> f64 {
        let rpm_clamped = normalized_rpm.clamp(0.0, 1.0);
        if self.torque_curve.is_empty() {
            return 1.0;
        }
        if rpm_clamped <= self.torque_curve[0].0 {
            return self.torque_curve[0].1;
        }
        let last_idx = self.torque_curve.len() - 1;
        if rpm_clamped >= self.torque_curve[last_idx].0 {
            return self.torque_curve[last_idx].1;
        }

        for i in 0..last_idx {
            let (r0, t0) = self.torque_curve[i];
            let (r1, t1) = self.torque_curve[i + 1];
            if rpm_clamped >= r0 && rpm_clamped <= r1 {
                let frac = (rpm_clamped - r0) / (r1 - r0).max(1e-6);
                return t0 + frac * (t1 - t0);
            }
        }
        self.torque_curve[last_idx].1
    }
}
