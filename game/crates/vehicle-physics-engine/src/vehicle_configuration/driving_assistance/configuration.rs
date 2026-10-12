use super::*;
use super::document::{
    default_tc_attack_rate, default_tc_gear_authority, default_tc_gear_max_cut,
    default_tc_gear_slip_target, default_tc_release_rate,
};

/// Canonical driving-aid model and policy for a vehicle profile.
///
/// `available` declares that the hardware/software package exposes the aid.
/// `default_enabled` is the value used when no runtime mask overrides it.
/// `player_selectable` gates whether the UI may toggle it at runtime.
/// Physical tuning parameters (thresholds, gains) are model data; they do
/// not by themselves enable the aid. Runtime activation is governed by
/// `VehicleSimulator.aids_enabled_mask` (see ffi.rs `aids_enabled_mask`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AidsConfig {
    // Traction control (CT / TCS) — reduces drive torque above a slip threshold.
    pub traction_control_available: bool,
    pub traction_control_default_enabled: bool,
    pub traction_control_player_selectable: bool,
    pub traction_control_slip_threshold: f64,
    pub traction_control_cut_gain: f64,
    pub traction_control_actuator: String,
    pub traction_control_gear_authority: Vec<f64>,
    pub traction_control_gear_slip_target: Vec<f64>,
    pub traction_control_gear_max_cut: Vec<f64>,
    pub traction_control_attack_rate: f64,
    pub traction_control_release_rate: f64,

    // ABS policy
    pub abs_available: bool,
    pub abs_default_enabled: bool,
    pub abs_player_selectable: bool,

    // Stability / yaw correction policy
    pub stability_available: bool,
    pub stability_default_enabled: bool,
    pub stability_player_selectable: bool,
    pub stability_yaw_engage_angle_rad: f64,
    pub stability_yaw_strength: f64,
    /// Lateral acceleration envelope (m/s^2) that bounds the reference yaw rate
    /// `v*tan(delta)/L`. Without it a full-lock command at speed implies a
    /// kinematic yaw no tire can hold and the aid stays idle while the rear
    /// slides (power-on oversteer).
    pub stability_reference_lateral_accel_m_s2: f64,
    pub stability_grounded_multiplier: f64,
    pub stability_upright_spring: f64,
    pub stability_upright_damping: f64,

    // Steering assists (already applied in simulation.rs; policy gates them)
    pub steering_slip_assist_default_enabled: bool,
    pub countersteer_default_enabled: bool,
    pub steering_assist_player_selectable: bool,

    // Automatic transmission / auto-clutch (always on for F1-94)
    pub auto_clutch_default_enabled: bool,
    pub auto_clutch_player_selectable: bool,

    // Launch control
    pub launch_control_available: bool,
    pub launch_control_default_enabled: bool,
    pub launch_control_player_selectable: bool,
    pub launch_control_target_rpm: f64,
    pub launch_control_throttle_limit: f64,
    pub launch_control_clutch_release_rate_per_s: f64,

    // Brake assist
    pub brake_assist_available: bool,
    pub brake_assist_default_enabled: bool,
    pub brake_assist_player_selectable: bool,
    pub brake_assist_force_multiplier: f64,

    // Handbrake behavior
    pub handbrake_rear_only: bool,
    pub handbrake_abs_interlock: bool,
    pub handbrake_clutch_coupling: bool,

    // Input smoothing
    pub input_smoothing_enabled: bool,
    pub input_smoothing_steering_rate: f64,
    pub input_smoothing_throttle_rate: f64,
    pub input_smoothing_brake_rate: f64,
}


pub(crate) fn default_aids() -> AidsConfig {
    AidsConfig {
        traction_control_available: true,
        traction_control_default_enabled: true,
        traction_control_player_selectable: true,
        traction_control_slip_threshold: 0.08,
        traction_control_cut_gain: 1.4,
        traction_control_actuator: "engine_torque".to_string(),
        traction_control_gear_authority: default_tc_gear_authority(),
        traction_control_gear_slip_target: default_tc_gear_slip_target(),
        traction_control_gear_max_cut: default_tc_gear_max_cut(),
        traction_control_attack_rate: default_tc_attack_rate(),
        traction_control_release_rate: default_tc_release_rate(),

        abs_available: true,
        abs_default_enabled: false,
        abs_player_selectable: true,

        stability_available: true,
        stability_default_enabled: true,
        stability_player_selectable: true,
        stability_yaw_engage_angle_rad: 0.045,
        stability_yaw_strength: 15.0,
        stability_reference_lateral_accel_m_s2: 21.0,
        stability_grounded_multiplier: 2.6,
        stability_upright_spring: 0.0,
        stability_upright_damping: 0.0,

        steering_slip_assist_default_enabled: true,
        countersteer_default_enabled: true,
        steering_assist_player_selectable: true,

        auto_clutch_default_enabled: true,
        auto_clutch_player_selectable: false,

        launch_control_available: false,
        launch_control_default_enabled: false,
        launch_control_player_selectable: true,
        launch_control_target_rpm: 9000.0,
        launch_control_throttle_limit: 1.0,
        launch_control_clutch_release_rate_per_s: 2.0,

        brake_assist_available: false,
        brake_assist_default_enabled: false,
        brake_assist_player_selectable: true,
        brake_assist_force_multiplier: 1.0,

        handbrake_rear_only: true,
        handbrake_abs_interlock: true,
        handbrake_clutch_coupling: true,

        input_smoothing_enabled: true,
        input_smoothing_steering_rate: 4.25,
        input_smoothing_throttle_rate: 20.0,
        input_smoothing_brake_rate: 10.0,
    }
}
