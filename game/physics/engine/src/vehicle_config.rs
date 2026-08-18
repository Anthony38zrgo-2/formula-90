use crate::types::{SurfaceType, Vec3, WheelIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Formula-90 vehicle configuration with GEVP-compatible suspension, tire and drivetrain semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleConfig {
    pub schema_version: u32,
    pub vehicle_name: String,

    // Mass & Geometry
    pub vehicle_mass: f64,
    pub front_weight_distribution: f64,
    pub center_of_gravity_height_offset: f64,
    pub inertia_multipliers: Vec3,
    pub wheelbase: f64,
    pub front_track: f64,
    pub rear_track: f64,

    // Steering
    pub max_steering_angle: f64, // radians
    pub front_steering_ratio: f64,
    pub rear_steering_ratio: f64,
    pub steering_speed: f64,
    pub countersteer_speed: f64,
    pub steering_speed_decay: f64,
    pub steering_slip_assist: f64,
    pub countersteer_assist: f64,
    pub steering_exponent: f64,
    pub ackermann: f64,

    // Powertrain & Gearing
    pub max_torque: f64,       // N·m
    pub max_rpm: f64,          // RPM
    pub idle_rpm: f64,         // RPM
    pub motor_moment: f64,     // kg·m²
    pub torque_curve: Vec<(f64, f64)>, // (normalized_rpm, normalized_torque)
    pub gear_ratios: Vec<f64>,
    pub final_drive: f64,
    pub reverse_ratio: f64,
    pub shift_time: f64,       // seconds
    pub automatic_transmission: bool,
    pub front_torque_split: f64, // 0.0 = pure RWD, 1.0 = pure FWD

    // Differential (Salisbury Clutch-Pack LSD, AMS2/Reiza aligned)
    pub diff_preload: f64,               // Static preload torque (N·m)
    pub diff_power_ramp_angle_deg: f64,  // Power lock ramp angle (degrees)
    pub diff_coast_ramp_angle_deg: f64,  // Coast lock ramp angle (degrees)
    pub diff_clutches: f64,              // Number of friction plate surfaces
    pub diff_clutch_friction_coeff: f64, // Clutch plate friction coefficient

    // Suspension Parameters (Front)
    pub front_spring_length: f64,
    pub front_resting_ratio: f64,
    pub front_damping_ratio: f64,
    pub front_bump_damp_multiplier: f64,
    pub front_rebound_damp_multiplier: f64,
    pub front_arb_ratio: f64,
    pub front_toe: f64,
    pub front_camber: f64,
    pub front_bump_stop_multiplier: f64,

    // Suspension Parameters (Rear)
    pub rear_spring_length: f64,
    pub rear_resting_ratio: f64,
    pub rear_damping_ratio: f64,
    pub rear_bump_damp_multiplier: f64,
    pub rear_rebound_damp_multiplier: f64,
    pub rear_arb_ratio: f64,
    pub rear_toe: f64,
    pub rear_camber: f64,
    pub rear_bump_stop_multiplier: f64,

    // Tri-Raycast Configuration
    pub tri_ray_spacing_ratio: f64, // 0.40 = +/- 40% of tire width

    // Tires & Contact
    pub front_tire_radius: f64,
    pub rear_tire_radius: f64,
    pub front_tire_width: f64, // meters
    pub rear_tire_width: f64,  // meters
    pub front_wheel_mass: f64,
    pub rear_wheel_mass: f64,
    // Global fallback (used when a per-axle value is omitted in JSON).
    pub contact_patch: f64,
    pub braking_grip_multiplier: f64,
    // Per-axle tire grip (front/rear) — restores GEVP per-wheel fidelity.
    pub front_contact_patch: f64,
    pub rear_contact_patch: f64,
    pub front_braking_grip: f64,
    pub rear_braking_grip: f64,
    pub front_airborne_decay: f64,
    pub rear_airborne_decay: f64,
    pub front_brake_bias: f64,
    pub max_brake_torque: f64,

    // ABS
    pub enable_abs: bool,
    pub abs_pulse_time: f64,
    pub abs_spin_diff_threshold: f64,

    // Auto-clutch / launch (previously hardcoded in powertrain)
    pub max_clutch_torque_ratio: f64,
    pub clutch_out_rpm_offset: f64,
    pub idle_disengagement_hysteresis_rpm: f64,
    pub variable_drag_ratio: f64,
    pub constant_brake_ratio: f64,
    pub handbrake_torque_fraction: f64,

    // Differential slip transition threshold (was hardcoded 0.50)
    pub diff_slip_transition_threshold_rad_s: f64,

    // Surface properties
    pub surface_friction: HashMap<SurfaceType, f64>,
    pub surface_stiffness: HashMap<SurfaceType, f64>,
    pub surface_rolling_resistance: HashMap<SurfaceType, f64>,
    pub surface_lateral_grip_assist: HashMap<SurfaceType, f64>,
    pub surface_longitudinal_grip_ratio: HashMap<SurfaceType, f64>,

    // Driving aids (canonical model + policy). Runtime enablement is held
    // separately in the simulator's aids_enabled_mask, not here.
    pub aids: AidsConfig,

    // Aerodynamics
    pub coefficient_of_drag: f64,
    pub frontal_area: f64,
    pub coefficient_of_downforce: f64,
    pub aero_split_front: f64,
    pub aero_split_diffuser: f64,
    pub aero_split_rear: f64,
    pub air_density: f64,

    // Progressive Aerodynamic Extensions (F1-94 canon, Jordan legacy stub uses same defaults)
    #[serde(default = "default_aero_lag_tau")]
    pub aero_lag_tau: f64,
    #[serde(default = "default_aero_blend_min_speed")]
    pub aero_blend_min_speed: f64,
    #[serde(default = "default_aero_blend_full_speed")]
    pub aero_blend_full_speed: f64,
    #[serde(default = "default_aero_yaw_decay_exponent")]
    pub aero_yaw_decay_exponent: f64,
    #[serde(default = "default_aero_flex_coefficient")]
    pub aero_flex_coefficient: f64,
}

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

fn default_aids() -> AidsConfig {
    AidsConfig {
        traction_control_available: true,
        traction_control_default_enabled: false,
        traction_control_player_selectable: true,
        traction_control_slip_threshold: 0.08,
        traction_control_cut_gain: 1.0,
        traction_control_actuator: "engine_torque".to_string(),

        abs_available: true,
        abs_default_enabled: false,
        abs_player_selectable: true,

        stability_available: false,
        stability_default_enabled: false,
        stability_player_selectable: true,
        stability_yaw_engage_angle_rad: 0.01,
        stability_yaw_strength: 5.25,
        stability_grounded_multiplier: 2.0,
        stability_upright_spring: 1.0,
        stability_upright_damping: 1000.0,

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

fn default_aero_lag_tau() -> f64 {
    0.048
}
fn default_aero_blend_min_speed() -> f64 {
    4.167
}
fn default_aero_blend_full_speed() -> f64 {
    27.778
}
fn default_aero_yaw_decay_exponent() -> f64 {
    1.30
}
fn default_aero_flex_coefficient() -> f64 {
    0.0012
}

impl Default for VehicleConfig {
    fn default() -> Self {
        Self::f1_94_canonical()
    }
}

impl VehicleConfig {
    /// Canonical F1-94 specification matching data/vehicles/f1_94/ resources.
    pub fn f1_94_canonical() -> Self {
        let mut surface_friction = HashMap::new();
        surface_friction.insert(SurfaceType::Road, 2.9);
        surface_friction.insert(SurfaceType::Curb, 2.2);
        surface_friction.insert(SurfaceType::Dirt, 1.4);
        surface_friction.insert(SurfaceType::Grass, 0.9);
        surface_friction.insert(SurfaceType::Gravel, 1.1);

        let mut surface_stiffness = HashMap::new();
        surface_stiffness.insert(SurfaceType::Road, 8.75);
        surface_stiffness.insert(SurfaceType::Curb, 7.0);
        surface_stiffness.insert(SurfaceType::Dirt, 0.5);
        surface_stiffness.insert(SurfaceType::Grass, 0.5);
        surface_stiffness.insert(SurfaceType::Gravel, 0.5);

        let mut surface_rolling_resistance = HashMap::new();
        surface_rolling_resistance.insert(SurfaceType::Road, 1.0);
        surface_rolling_resistance.insert(SurfaceType::Curb, 1.5);
        surface_rolling_resistance.insert(SurfaceType::Dirt, 2.0);
        surface_rolling_resistance.insert(SurfaceType::Grass, 4.0);
        surface_rolling_resistance.insert(SurfaceType::Gravel, 2.0);

        let mut surface_lateral_grip_assist = HashMap::new();
        for st in [SurfaceType::Road, SurfaceType::Curb, SurfaceType::Dirt, SurfaceType::Grass, SurfaceType::Gravel] {
            surface_lateral_grip_assist.insert(st, 0.05);
        }
        let mut surface_longitudinal_grip_ratio = HashMap::new();
        for st in [SurfaceType::Road, SurfaceType::Curb, SurfaceType::Dirt, SurfaceType::Grass, SurfaceType::Gravel] {
            surface_longitudinal_grip_ratio.insert(st, 0.5);
        }

        Self {
            schema_version: 2,
            vehicle_name: "F1 1994 (V10)".to_string(),

            // Mass & Geometry
            vehicle_mass: 575.0,
            front_weight_distribution: 0.45,
            center_of_gravity_height_offset: -0.12,
            inertia_multipliers: Vec3::new(1.10, 1.10, 1.10),
            wheelbase: 2.92065,
            front_track: 1.5925,
            rear_track: 1.5246,

            // Steering
            max_steering_angle: 0.436332,
            front_steering_ratio: 1.0,
            rear_steering_ratio: 0.0,
            steering_speed: 4.25,
            countersteer_speed: 11.0,
            steering_speed_decay: 0.20,
            steering_slip_assist: 0.54,
            countersteer_assist: 0.89,
            steering_exponent: 1.50,
            ackermann: 0.15,

            // Powertrain & Gearing
            max_torque: 455.0,
            max_rpm: 15000.0,
            idle_rpm: 4500.0,
            motor_moment: 0.12,
            torque_curve: vec![
                (0.00, 0.20),
                (0.265, 0.28),
                (0.44, 0.58),
                (0.62, 0.85),
                (0.79, 0.98),
                (0.88, 1.00),
                (0.97, 0.95),
                (1.00, 0.88),
            ],
            gear_ratios: vec![2.65, 2.10, 1.75, 1.50, 1.32, 1.18],
            final_drive: 6.30,
            reverse_ratio: 3.00,
            shift_time: 0.12,
            automatic_transmission: false,
            front_torque_split: 0.0,

            // Differential (Salisbury Clutch-Pack LSD, AMS2/Reiza aligned) — CORR-03 candidate B: 170/65/75/mu0.0
            diff_preload: 170.0,
            diff_power_ramp_angle_deg: 65.0,
            diff_coast_ramp_angle_deg: 75.0,
            diff_clutches: 4.0,
            diff_clutch_friction_coeff: 0.0,

            // Suspension (Front)
            front_spring_length: 0.250,
            front_resting_ratio: 0.280,
            front_damping_ratio: 0.80,
            front_bump_damp_multiplier: 1.3,
            front_rebound_damp_multiplier: 1.1,
            front_arb_ratio: 0.20,
            front_toe: 0.0017453,
            front_camber: -0.0174533,
            front_bump_stop_multiplier: 2.2,

            // Suspension (Rear)
            rear_spring_length: 0.180,
            rear_resting_ratio: 0.350,
            rear_damping_ratio: 0.80,
            rear_bump_damp_multiplier: 1.3,
            rear_rebound_damp_multiplier: 1.1,
            rear_arb_ratio: 0.05,
            rear_toe: 0.0017453,
            rear_camber: -0.0174533,
            rear_bump_stop_multiplier: 3.5,

            // Tri-Raycast
            tri_ray_spacing_ratio: 0.40,

            // Tires & Contact
            front_tire_radius: 0.31695,
            rear_tire_radius: 0.32901,
            front_tire_width: 0.30030,
            rear_tire_width: 0.36832,
            front_wheel_mass: 12.0,
            rear_wheel_mass: 16.0,
            contact_patch: 0.21,
            braking_grip_multiplier: 1.08,
            front_contact_patch: 0.21,
            rear_contact_patch: 0.21,
            front_braking_grip: 1.08,
            rear_braking_grip: 1.08,
            front_airborne_decay: 2.0,
            rear_airborne_decay: 2.0,
            front_brake_bias: 0.57,
            max_brake_torque: 2800.0,

            // ABS
            enable_abs: false,
            abs_pulse_time: 0.03,
            abs_spin_diff_threshold: 12.0,

            // Auto-clutch / launch (previously hardcoded in powertrain)
            max_clutch_torque_ratio: 1.6,
            clutch_out_rpm_offset: 1000.0,
            idle_disengagement_hysteresis_rpm: 50.0,
            variable_drag_ratio: 0.10,
            constant_brake_ratio: 0.02,
            handbrake_torque_fraction: 0.4,

            // Differential slip transition threshold (was hardcoded 0.50)
            diff_slip_transition_threshold_rad_s: 0.5,

            // Surface Maps
            surface_friction: surface_friction.clone(),
            surface_stiffness: surface_stiffness.clone(),
            surface_rolling_resistance: surface_rolling_resistance.clone(),
            surface_lateral_grip_assist: surface_lateral_grip_assist.clone(),
            surface_longitudinal_grip_ratio: surface_longitudinal_grip_ratio.clone(),

            // Driving aids
            aids: default_aids(),

            // Aerodynamics (3-point distribution: 15% front, 60% diffuser, 25% rear = 45% front / 55% rear axle load)
            coefficient_of_drag: 0.78,
            frontal_area: 1.25,
            coefficient_of_downforce: 1.995,
            aero_split_front: 0.15,
            aero_split_diffuser: 0.60,
            aero_split_rear: 0.25,
            air_density: 1.225,

            // Progressive Aero (Pillars A-D)
            aero_lag_tau: default_aero_lag_tau(),
            aero_blend_min_speed: default_aero_blend_min_speed(),
            aero_blend_full_speed: default_aero_blend_full_speed(),
            aero_yaw_decay_exponent: default_aero_yaw_decay_exponent(),
            aero_flex_coefficient: default_aero_flex_coefficient(),
        }
    }

    /// Canonical Jordan 197 specification (legacy — aero extensions use same defaults, not validated).
    pub fn jordan_197_canonical() -> Self {
        let mut surface_friction = HashMap::new();
        surface_friction.insert(SurfaceType::Road, 2.0);
        surface_friction.insert(SurfaceType::Curb, 1.85);
        surface_friction.insert(SurfaceType::Dirt, 1.4);
        surface_friction.insert(SurfaceType::Grass, 1.0);

        let mut surface_stiffness = HashMap::new();
        surface_stiffness.insert(SurfaceType::Road, 5.0);
        surface_stiffness.insert(SurfaceType::Curb, 4.5);
        surface_stiffness.insert(SurfaceType::Dirt, 0.5);
        surface_stiffness.insert(SurfaceType::Grass, 0.5);

        let mut surface_rolling_resistance = HashMap::new();
        surface_rolling_resistance.insert(SurfaceType::Road, 1.0);
        surface_rolling_resistance.insert(SurfaceType::Curb, 1.0);
        surface_rolling_resistance.insert(SurfaceType::Dirt, 1.5);
        surface_rolling_resistance.insert(SurfaceType::Grass, 2.0);

        Self {
            schema_version: 2,
            vehicle_name: "Jordan 197 (V10)".to_string(),

            // Mass & Geometry
            vehicle_mass: 505.0,
            front_weight_distribution: 0.45,
            center_of_gravity_height_offset: -0.12,
            inertia_multipliers: Vec3::new(1.10, 1.10, 1.10),
            wheelbase: 2.950,
            front_track: 1.762,
            rear_track: 1.748,

            // Steering
            max_steering_angle: 0.436332, // ~25.0 degrees
            front_steering_ratio: 1.0,
            rear_steering_ratio: 0.0,
            steering_speed: 3.7,
            countersteer_speed: 9.0,
            steering_speed_decay: 0.26,
            steering_slip_assist: 0.11,
            countersteer_assist: 0.70,
            steering_exponent: 1.70,
            ackermann: 0.15,

            // Powertrain & Gearing
            max_torque: 430.0,
            max_rpm: 13000.0,
            idle_rpm: 3500.0,
            motor_moment: 0.22,
            torque_curve: vec![
                (0.00, 0.38),
                (0.30, 0.65),
                (0.55, 0.88),
                (0.72, 1.00),
                (0.90, 0.92),
                (1.00, 0.76),
            ],
            gear_ratios: vec![3.00, 2.25, 1.75, 1.42, 1.20, 1.03],
            final_drive: 3.10,
            reverse_ratio: 3.00,
            shift_time: 0.16,
            automatic_transmission: true,
            front_torque_split: 0.0, // RWD

            // Differential (Salisbury Clutch-Pack LSD)
            diff_preload: 90.0,
            diff_power_ramp_angle_deg: 45.0,
            diff_coast_ramp_angle_deg: 60.0,
            diff_clutches: 4.0,
            diff_clutch_friction_coeff: 0.25,

            // Suspension (Front)
            front_spring_length: 0.100,
            front_resting_ratio: 0.500,
            front_damping_ratio: 0.80,
            front_bump_damp_multiplier: 1.3,
            front_rebound_damp_multiplier: 1.1,
            front_arb_ratio: 0.25,
            front_toe: 0.002,
            front_camber: -0.052, // -3.0 deg
            front_bump_stop_multiplier: 2.2,

            // Suspension (Rear)
            rear_spring_length: 0.100,
            rear_resting_ratio: 0.500,
            rear_damping_ratio: 0.85,
            rear_bump_damp_multiplier: 1.3,
            rear_rebound_damp_multiplier: 1.1,
            rear_arb_ratio: 0.10,
            rear_toe: 0.001,
            rear_camber: -0.026, // -1.5 deg
            rear_bump_stop_multiplier: 2.2,

            // Tri-Raycast
            tri_ray_spacing_ratio: 0.40,

            // Tires & Brakes
            front_tire_radius: 0.324,
            rear_tire_radius: 0.324,
            front_tire_width: 0.308, // 308 mm
            rear_tire_width: 0.358,  // 358 mm
            front_wheel_mass: 12.0,
            rear_wheel_mass: 16.0,
            contact_patch: 0.20,
            braking_grip_multiplier: 1.4,
            front_contact_patch: 0.20,
            rear_contact_patch: 0.20,
            front_braking_grip: 1.4,
            rear_braking_grip: 1.4,
            front_airborne_decay: 2.0,
            rear_airborne_decay: 2.0,
            front_brake_bias: 0.58,
            max_brake_torque: 2800.0,

            // ABS
            enable_abs: false,
            abs_pulse_time: 0.03,
            abs_spin_diff_threshold: 12.0,

            // Auto-clutch / launch (previously hardcoded in powertrain)
            max_clutch_torque_ratio: 1.6,
            clutch_out_rpm_offset: 1000.0,
            idle_disengagement_hysteresis_rpm: 50.0,
            variable_drag_ratio: 0.10,
            constant_brake_ratio: 0.02,
            handbrake_torque_fraction: 0.4,

            // Differential slip transition threshold (was hardcoded 0.50)
            diff_slip_transition_threshold_rad_s: 0.5,

            // Surface Maps
            surface_friction,
            surface_stiffness,
            surface_rolling_resistance,
            surface_lateral_grip_assist: HashMap::new(),
            surface_longitudinal_grip_ratio: HashMap::new(),

            // Driving aids
            aids: default_aids(),

            // Aerodynamics (legacy splits preserved)
            coefficient_of_drag: 0.75,
            frontal_area: 1.20,
            coefficient_of_downforce: 1.995,
            aero_split_front: 0.20,
            aero_split_diffuser: 0.60,
            aero_split_rear: 0.20,
            air_density: 1.225,

            // Progressive Aero (stub defaults, not validated for Jordan)
            aero_lag_tau: default_aero_lag_tau(),
            aero_blend_min_speed: default_aero_blend_min_speed(),
            aero_blend_full_speed: default_aero_blend_full_speed(),
            aero_yaw_decay_exponent: default_aero_yaw_decay_exponent(),
            aero_flex_coefficient: default_aero_flex_coefficient(),
        }
    }

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

    /// Helper to get mass supported by a single wheel at static rest.
    pub fn mass_over_wheel(&self, wheel: WheelIndex) -> f64 {
        if wheel.is_front() {
            self.vehicle_mass * self.front_weight_distribution * 0.5
        } else {
            self.vehicle_mass * (1.0 - self.front_weight_distribution) * 0.5
        }
    }

    /// Helper to get suspension spring rate in N/m for a wheel based on rest target.
    pub fn calculate_spring_rate(&self, wheel: WheelIndex) -> f64 {
        let mass = self.mass_over_wheel(wheel);
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

    /// Nominal suspension-ray origin in vehicle-local geometric space (-Z forward, +Y up, +X right),
    /// where origin (0, 0, 0) is centered between axles at mean wheel-center height.
    pub fn wheel_anchor_local(&self, wheel: WheelIndex) -> Vec3 {
        let z = if wheel.is_front() {
            -self.wheelbase * 0.5
        } else {
            self.wheelbase * 0.5
        };
        let x = if wheel.is_front() {
            if wheel.is_right() { self.front_track * 0.5 } else { -self.front_track * 0.5 }
        } else if wheel.is_right() {
            self.rear_track * 0.5
        } else {
            -self.rear_track * 0.5
        };

        let mean_radius = (self.front_tire_radius + self.rear_tire_radius) * 0.5;
        let hub_y = if wheel.is_front() {
            self.front_tire_radius - mean_radius
        } else {
            self.rear_tire_radius - mean_radius
        };
        let spring_length = if wheel.is_front() { self.front_spring_length } else { self.rear_spring_length };
        let resting_ratio = if wheel.is_front() { self.front_resting_ratio } else { self.rear_resting_ratio };
        let anchor_y = hub_y + spring_length * (1.0 - resting_ratio);

        Vec3::new(x, anchor_y, z)
    }

    /// Load vehicle configuration from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, String> {
        let spec: JsonVehicleSpec = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        Self::from_json_spec(spec)
    }

    /// Load vehicle configuration from a JSON file path.
    pub fn from_json_path(path: &Path) -> Result<Self, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        Self::from_json_str(&json)
    }

    /// Serialize this config to a serde_json::Value matching the JSON schema.
    pub fn to_json_value(&self) -> serde_json::Value {
        let spec = JsonVehicleSpec::from_config(self);
        serde_json::to_value(spec).unwrap_or_default()
    }

    fn from_json_spec(spec: JsonVehicleSpec) -> Result<Self, String> {
        spec.validate()?;
        Ok(spec.to_config())
    }
}

// ── JSON schema types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonVehicleSpec {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    vehicle_id: String,
    #[serde(default)]
    profile_name: String,
    #[serde(default)]
    units: String,
    #[serde(default)]
    chassis: JsonChassis,
    #[serde(default)]
    geometry: JsonGeometry,
    #[serde(default)]
    steering: JsonSteering,
    #[serde(default)]
    powertrain: JsonPowertrain,
    #[serde(default)]
    suspension: JsonSuspension,
    #[serde(default)]
    tires: JsonTires,
    #[serde(default)]
    brakes: JsonBrakes,
    #[serde(default)]
    aero: JsonAero,
    #[serde(default)]
    aids: JsonAids,
    #[serde(default)]
    coordinate_contract: Option<HashMap<String, String>>,
}

fn default_schema_version() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonChassis {
    #[serde(default = "default_vehicle_name")]
    vehicle_name: String,
    #[serde(default = "default_mass")]
    vehicle_mass: f64,
    #[serde(default = "default_front_weight_dist")]
    front_weight_distribution: f64,
    #[serde(default = "default_cog_height")]
    center_of_gravity_height_offset: f64,
    #[serde(default)]
    inertia_multipliers: JsonVec3,
}
impl Default for JsonChassis {
    fn default() -> Self {
        Self {
            vehicle_name: default_vehicle_name(),
            vehicle_mass: default_mass(),
            front_weight_distribution: default_front_weight_dist(),
            center_of_gravity_height_offset: default_cog_height(),
            inertia_multipliers: JsonVec3::default(),
        }
    }
}
fn default_vehicle_name() -> String { "F1 1994 (V10)".to_string() }
fn default_mass() -> f64 { 575.0 }
fn default_front_weight_dist() -> f64 { 0.45 }
fn default_cog_height() -> f64 { -0.12 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonVec3 { x: f64, y: f64, z: f64 }
impl Default for JsonVec3 {
    fn default() -> Self { Self { x: 1.0, y: 1.0, z: 1.0 } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonGeometry {
    #[serde(default = "default_wheelbase")]
    wheelbase: f64,
    #[serde(default = "default_front_track")]
    front_track: f64,
    #[serde(default = "default_rear_track")]
    rear_track: f64,
    #[serde(default)]
    wheel_hubs: Option<HashMap<String, JsonVec3>>,
}
impl Default for JsonGeometry {
    fn default() -> Self {
        Self {
            wheelbase: default_wheelbase(),
            front_track: default_front_track(),
            rear_track: default_rear_track(),
            wheel_hubs: None,
        }
    }
}
fn default_wheelbase() -> f64 { 2.92065 }
fn default_front_track() -> f64 { 1.5925 }
fn default_rear_track() -> f64 { 1.5246 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonSteering {
    #[serde(default = "default_max_steer")]
    max_steering_angle: f64,
    #[serde(default)]
    front_steering_ratio: f64,
    #[serde(default)]
    rear_steering_ratio: f64,
    #[serde(default = "default_steer_speed")]
    steering_speed: f64,
    #[serde(default = "default_counter_speed")]
    countersteer_speed: f64,
    #[serde(default = "default_steer_decay")]
    steering_speed_decay: f64,
    #[serde(default = "default_slip_assist")]
    steering_slip_assist: f64,
    #[serde(default = "default_counter_assist")]
    countersteer_assist: f64,
    #[serde(default = "default_steer_exp")]
    steering_exponent: f64,
    #[serde(default = "default_ackermann")]
    ackermann: f64,
}
impl Default for JsonSteering {
    fn default() -> Self {
        Self {
            max_steering_angle: default_max_steer(), front_steering_ratio: 1.0,
            rear_steering_ratio: 0.0, steering_speed: default_steer_speed(),
            countersteer_speed: default_counter_speed(), steering_speed_decay: default_steer_decay(),
            steering_slip_assist: default_slip_assist(), countersteer_assist: default_counter_assist(),
            steering_exponent: default_steer_exp(), ackermann: default_ackermann(),
        }
    }
}
fn default_max_steer() -> f64 { 0.436332 }
fn default_steer_speed() -> f64 { 4.25 }
fn default_counter_speed() -> f64 { 11.0 }
fn default_steer_decay() -> f64 { 0.20 }
fn default_slip_assist() -> f64 { 0.54 }
fn default_counter_assist() -> f64 { 0.89 }
fn default_steer_exp() -> f64 { 1.50 }
fn default_ackermann() -> f64 { 0.15 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonPowertrain {
    #[serde(default = "default_max_torque")]
    max_torque: f64,
    #[serde(default = "default_max_rpm")]
    max_rpm: f64,
    #[serde(default = "default_idle_rpm")]
    idle_rpm: f64,
    #[serde(default = "default_motor_moment")]
    motor_moment: f64,
    #[serde(default = "default_torque_curve")]
    torque_curve: Vec<(f64, f64)>,
    #[serde(default = "default_gear_ratios")]
    gear_ratios: Vec<f64>,
    #[serde(default = "default_final_drive")]
    final_drive: f64,
    #[serde(default = "default_reverse_ratio")]
    reverse_ratio: f64,
    #[serde(default = "default_shift_time")]
    shift_time: f64,
    #[serde(default = "default_auto_trans")]
    automatic_transmission: bool,
    #[serde(default)]
    front_torque_split: f64,
    #[serde(default = "default_gear_inertia")]
    gear_inertia: f64,
    #[serde(default = "default_max_clutch_ratio")]
    max_clutch_torque_ratio: f64,
    #[serde(default = "default_clutch_out_offset")]
    clutch_out_rpm_offset: f64,
    #[serde(default = "default_idle_hysteresis")]
    idle_disengagement_hysteresis_rpm: f64,
    #[serde(default = "default_var_drag")]
    variable_drag_ratio: f64,
    #[serde(default = "default_const_brake")]
    constant_brake_ratio: f64,
    #[serde(default = "default_handbrake_frac")]
    handbrake_torque_fraction: f64,
    #[serde(default)]
    automatic_shift: Option<JsonAutomaticShift>,
    #[serde(default)]
    differential: JsonDifferential,
}
impl Default for JsonPowertrain {
    fn default() -> Self {
        Self {
            max_torque: default_max_torque(), max_rpm: default_max_rpm(),
            idle_rpm: default_idle_rpm(), motor_moment: default_motor_moment(),
            torque_curve: default_torque_curve(), gear_ratios: default_gear_ratios(),
            final_drive: default_final_drive(), reverse_ratio: default_reverse_ratio(),
            shift_time: default_shift_time(), automatic_transmission: default_auto_trans(),
            front_torque_split: 0.0, gear_inertia: default_gear_inertia(),
            max_clutch_torque_ratio: default_max_clutch_ratio(),
            clutch_out_rpm_offset: default_clutch_out_offset(),
            idle_disengagement_hysteresis_rpm: default_idle_hysteresis(),
            variable_drag_ratio: default_var_drag(), constant_brake_ratio: default_const_brake(),
            handbrake_torque_fraction: default_handbrake_frac(),
            automatic_shift: None,
            differential: JsonDifferential::default(),
        }
    }
}
fn default_max_torque() -> f64 { 455.0 }
fn default_max_rpm() -> f64 { 15000.0 }
fn default_idle_rpm() -> f64 { 4500.0 }
fn default_motor_moment() -> f64 { 0.12 }
fn default_torque_curve() -> Vec<(f64, f64)> {
    vec![(0.00,0.20),(0.265,0.28),(0.44,0.58),(0.62,0.85),(0.79,0.98),(0.88,1.00),(0.97,0.95),(1.00,0.88)]
}
fn default_gear_ratios() -> Vec<f64> { vec![2.65, 2.10, 1.75, 1.50, 1.32, 1.18] }
fn default_final_drive() -> f64 { 6.30 }
fn default_reverse_ratio() -> f64 { 3.00 }
fn default_shift_time() -> f64 { 0.12 }
fn default_auto_trans() -> bool { false }
fn default_gear_inertia() -> f64 { 0.02 }
fn default_max_clutch_ratio() -> f64 { 1.6 }
fn default_clutch_out_offset() -> f64 { 1000.0 }
fn default_idle_hysteresis() -> f64 { 50.0 }
fn default_var_drag() -> f64 { 0.10 }
fn default_const_brake() -> f64 { 0.02 }
fn default_handbrake_frac() -> f64 { 0.4 }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonAutomaticShift {
    #[serde(default)]
    upshift_normalized_rpm_low_throttle: f64,
    #[serde(default)]
    upshift_normalized_rpm_full_throttle: f64,
    #[serde(default)]
    downshift_normalized_rpm_low_throttle: f64,
    #[serde(default)]
    downshift_normalized_rpm_full_throttle: f64,
    #[serde(default)]
    downshift_throttle_aggression_factor: f64,
    #[serde(default)]
    upshift_coast_normalized_rpm: f64,
    #[serde(default)]
    downshift_coast_normalized_rpm: f64,
    #[serde(default)]
    reverse_max_speed_ms: f64,
    #[serde(default)]
    parked_resume_speed_ms: f64,
    #[serde(default)]
    kickdown_delay_factor: f64,
    #[serde(default)]
    wheel_road_spin_blend_threshold_rads: f64,
    #[serde(default)]
    wheel_spin_weight: f64,
    #[serde(default)]
    road_spin_weight: f64,
    #[serde(default)]
    upshift_target_rpm_margin_high: f64,
    #[serde(default)]
    downshift_target_rpm_margin_high: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonDifferential {
    #[serde(default = "default_preload")]
    preload_nm: f64,
    #[serde(default = "default_power_ramp")]
    power_ramp_angle_deg: f64,
    #[serde(default = "default_coast_ramp")]
    coast_ramp_angle_deg: f64,
    #[serde(default = "default_clutches")]
    clutches: f64,
    #[serde(default)]
    clutch_friction_coefficient: f64,
    #[serde(default = "default_slip_threshold")]
    slip_transition_threshold_rad_s: f64,
}
impl Default for JsonDifferential {
    fn default() -> Self {
        Self {
            preload_nm: 170.0, power_ramp_angle_deg: 65.0, coast_ramp_angle_deg: 75.0,
            clutches: 4.0, clutch_friction_coefficient: 0.0, slip_transition_threshold_rad_s: 0.5,
        }
    }
}
fn default_preload() -> f64 { 170.0 }
fn default_power_ramp() -> f64 { 65.0 }
fn default_coast_ramp() -> f64 { 75.0 }
fn default_clutches() -> f64 { 4.0 }
fn default_slip_threshold() -> f64 { 0.5 }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonSuspension {
    #[serde(default)]
    front: JsonSuspensionAxle,
    #[serde(default)]
    rear: JsonSuspensionAxle,
    #[serde(default = "default_tri_ray")]
    tri_ray_spacing_ratio: f64,
}
fn default_tri_ray() -> f64 { 0.40 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonSuspensionAxle {
    #[serde(default)]
    spring_length: f64,
    #[serde(default)]
    resting_ratio: f64,
    #[serde(default = "default_damping")]
    damping_ratio: f64,
    #[serde(default = "default_bump_mult")]
    bump_damp_multiplier: f64,
    #[serde(default = "default_rebound_mult")]
    rebound_damp_multiplier: f64,
    #[serde(default)]
    arb_ratio: f64,
    #[serde(default)]
    toe: f64,
    #[serde(default)]
    camber: f64,
    #[serde(default = "default_bump_stop")]
    bump_stop_multiplier: f64,
}
impl Default for JsonSuspensionAxle {
    fn default() -> Self {
        Self {
            spring_length: 0.25, resting_ratio: 0.28, damping_ratio: 0.80,
            bump_damp_multiplier: 1.3, rebound_damp_multiplier: 1.1, arb_ratio: 0.20,
            toe: 0.0017453, camber: -0.0174533, bump_stop_multiplier: 2.2,
        }
    }
}
fn default_damping() -> f64 { 0.80 }
fn default_bump_mult() -> f64 { 1.3 }
fn default_rebound_mult() -> f64 { 1.1 }
fn default_bump_stop() -> f64 { 2.2 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonTires {
    #[serde(default)]
    front: JsonTireAxle,
    #[serde(default)]
    rear: JsonTireAxle,
    #[serde(default = "default_contact_patch")]
    contact_patch: f64,
    #[serde(default = "default_braking_grip")]
    braking_grip_multiplier: f64,
    #[serde(default = "default_airborne_decay")]
    airborne_spin_decay_torque: f64,
    #[serde(default)]
    surfaces: HashMap<String, JsonSurfaceEntry>,
}
impl Default for JsonTires {
    fn default() -> Self {
        Self {
            front: JsonTireAxle::default(), rear: JsonTireAxle::default(),
            contact_patch: default_contact_patch(), braking_grip_multiplier: default_braking_grip(),
            airborne_spin_decay_torque: default_airborne_decay(), surfaces: HashMap::new(),
        }
    }
}
fn default_contact_patch() -> f64 { 0.21 }
fn default_braking_grip() -> f64 { 1.08 }
fn default_airborne_decay() -> f64 { 2.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonTireAxle {
    #[serde(default = "default_tire_radius_front")]
    radius: f64,
    #[serde(default = "default_tire_width_front")]
    width: f64,
    #[serde(default = "default_wheel_mass_front")]
    wheel_mass: f64,
    #[serde(default)]
    contact_patch: Option<f64>,
    #[serde(default)]
    braking_grip_multiplier: Option<f64>,
    #[serde(default)]
    airborne_spin_decay_torque: Option<f64>,
}
impl Default for JsonTireAxle {
    fn default() -> Self {
        Self {
            radius: default_tire_radius_front(),
            width: default_tire_width_front(),
            wheel_mass: default_wheel_mass_front(),
            contact_patch: None,
            braking_grip_multiplier: None,
            airborne_spin_decay_torque: None,
        }
    }
}
fn default_tire_radius_front() -> f64 { 0.31695 }
fn default_tire_width_front() -> f64 { 0.30030 }
fn default_wheel_mass_front() -> f64 { 12.0 }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonSurfaceEntry {
    friction: f64,
    stiffness: f64,
    rolling_resistance: f64,
    #[serde(default = "default_lat_assist")]
    lateral_grip_assist: f64,
    #[serde(default = "default_long_ratio")]
    longitudinal_grip_ratio: f64,
}
fn default_lat_assist() -> f64 { 0.05 }
fn default_long_ratio() -> f64 { 0.5 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonBrakes {
    #[serde(default = "default_brake_bias")]
    front_brake_bias: f64,
    #[serde(default = "default_max_brake")]
    max_brake_torque: f64,
    #[serde(default)]
    enable_abs: bool,
    #[serde(default = "default_abs_pulse")]
    abs_pulse_time: f64,
    #[serde(default = "default_abs_thresh")]
    abs_spin_diff_threshold: f64,
}
impl Default for JsonBrakes {
    fn default() -> Self {
        Self {
            front_brake_bias: default_brake_bias(), max_brake_torque: default_max_brake(),
            enable_abs: false, abs_pulse_time: default_abs_pulse(),
            abs_spin_diff_threshold: default_abs_thresh(),
        }
    }
}
fn default_brake_bias() -> f64 { 0.57 }
fn default_max_brake() -> f64 { 2800.0 }
fn default_abs_pulse() -> f64 { 0.03 }
fn default_abs_thresh() -> f64 { 12.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonAero {
    #[serde(default = "default_cd")]
    drag_coefficient: f64,
    #[serde(default = "default_area")]
    frontal_area: f64,
    #[serde(default = "default_cl")]
    downforce_coefficient: f64,
    #[serde(default)]
    split: JsonAeroSplit,
    #[serde(default = "default_air_density")]
    air_density: f64,
    #[serde(default = "default_lag_tau")]
    lag_tau_s: f64,
    #[serde(default = "default_blend_min")]
    blend_min_speed_mps: f64,
    #[serde(default = "default_blend_full")]
    blend_full_speed_mps: f64,
    #[serde(default = "default_yaw_exp")]
    yaw_decay_exponent: f64,
    #[serde(default = "default_flex")]
    flex_coefficient: f64,
}
impl Default for JsonAero {
    fn default() -> Self {
        Self {
            drag_coefficient: 0.78, frontal_area: 1.25, downforce_coefficient: 1.995,
            split: JsonAeroSplit::default(), air_density: 1.225, lag_tau_s: 0.048,
            blend_min_speed_mps: 4.167, blend_full_speed_mps: 27.778,
            yaw_decay_exponent: 1.30, flex_coefficient: 0.0012,
        }
    }
}
fn default_cd() -> f64 { 0.78 }
fn default_area() -> f64 { 1.25 }
fn default_cl() -> f64 { 1.995 }
fn default_air_density() -> f64 { 1.225 }
fn default_lag_tau() -> f64 { 0.048 }
fn default_blend_min() -> f64 { 4.167 }
fn default_blend_full() -> f64 { 27.778 }
fn default_yaw_exp() -> f64 { 1.30 }
fn default_flex() -> f64 { 0.0012 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonAeroSplit {
    #[serde(default = "default_split_front")]
    front: f64,
    #[serde(default = "default_split_diff")]
    diffuser: f64,
    #[serde(default = "default_split_rear")]
    rear: f64,
}
impl Default for JsonAeroSplit {
    fn default() -> Self { Self { front: 0.15, diffuser: 0.60, rear: 0.25 } }
}
fn default_split_front() -> f64 { 0.15 }
fn default_split_diff() -> f64 { 0.60 }
fn default_split_rear() -> f64 { 0.25 }

// ── Driving aids (schema v2) ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct JsonAids {
    // Traction control (CT / TCS)
    #[serde(default = "default_tc_available")]
    traction_control_available: bool,
    #[serde(default = "default_tc_default")]
    traction_control_default_enabled: bool,
    #[serde(default = "default_tc_selectable")]
    traction_control_player_selectable: bool,
    #[serde(default = "default_tc_slip")]
    traction_control_slip_threshold: f64,
    #[serde(default = "default_tc_cut_gain")]
    traction_control_cut_gain: f64,
    #[serde(default = "default_tc_actuator")]
    traction_control_actuator: String,

    // ABS policy
    #[serde(default = "default_abs_available")]
    abs_available: bool,
    #[serde(default = "default_abs_default")]
    abs_default_enabled: bool,
    #[serde(default = "default_abs_selectable")]
    abs_player_selectable: bool,

    // Stability / yaw
    #[serde(default = "default_stab_available")]
    stability_available: bool,
    #[serde(default = "default_stab_default")]
    stability_default_enabled: bool,
    #[serde(default = "default_stab_selectable")]
    stability_player_selectable: bool,
    #[serde(default = "default_stab_engage")]
    stability_yaw_engage_angle_rad: f64,
    #[serde(default = "default_stab_yaw_strength")]
    stability_yaw_strength: f64,
    #[serde(default = "default_stab_ground_mult")]
    stability_grounded_multiplier: f64,
    #[serde(default = "default_stab_upright_spring")]
    stability_upright_spring: f64,
    #[serde(default = "default_stab_upright_damping")]
    stability_upright_damping: f64,

    // Steering assists
    #[serde(default = "default_steer_slip_on")]
    steering_slip_assist_default_enabled: bool,
    #[serde(default = "default_counter_on")]
    countersteer_default_enabled: bool,
    #[serde(default = "default_steer_selectable")]
    steering_assist_player_selectable: bool,

    // Auto-clutch
    #[serde(default = "default_auto_clutch_on")]
    auto_clutch_default_enabled: bool,
    #[serde(default = "default_auto_clutch_selectable")]
    auto_clutch_player_selectable: bool,

    // Launch control
    #[serde(default = "default_launch_available")]
    launch_control_available: bool,
    #[serde(default = "default_launch_default")]
    launch_control_default_enabled: bool,
    #[serde(default = "default_launch_selectable")]
    launch_control_player_selectable: bool,
    #[serde(default = "default_launch_target_rpm")]
    launch_control_target_rpm: f64,
    #[serde(default = "default_launch_throttle_limit")]
    launch_control_throttle_limit: f64,
    #[serde(default = "default_launch_release_rate")]
    launch_control_clutch_release_rate_per_s: f64,

    // Brake assist
    #[serde(default = "default_brake_assist_available")]
    brake_assist_available: bool,
    #[serde(default = "default_brake_assist_default")]
    brake_assist_default_enabled: bool,
    #[serde(default = "default_brake_assist_selectable")]
    brake_assist_player_selectable: bool,
    #[serde(default = "default_brake_assist_mult")]
    brake_assist_force_multiplier: f64,

    // Handbrake
    #[serde(default = "default_true")]
    handbrake_rear_only: bool,
    #[serde(default = "default_true")]
    handbrake_abs_interlock: bool,
    #[serde(default = "default_true")]
    handbrake_clutch_coupling: bool,

    // Input smoothing
    #[serde(default = "default_true")]
    input_smoothing_enabled: bool,
    #[serde(default = "default_steer_rate")]
    input_smoothing_steering_rate: f64,
    #[serde(default = "default_throttle_rate")]
    input_smoothing_throttle_rate: f64,
    #[serde(default = "default_brake_rate")]
    input_smoothing_brake_rate: f64,
}
impl Default for JsonAids {
    fn default() -> Self {
        Self {
            traction_control_available: default_tc_available(),
            traction_control_default_enabled: default_tc_default(),
            traction_control_player_selectable: default_tc_selectable(),
            traction_control_slip_threshold: default_tc_slip(),
            traction_control_cut_gain: default_tc_cut_gain(),
            traction_control_actuator: default_tc_actuator(),
            abs_available: default_abs_available(),
            abs_default_enabled: default_abs_default(),
            abs_player_selectable: default_abs_selectable(),
            stability_available: default_stab_available(),
            stability_default_enabled: default_stab_default(),
            stability_player_selectable: default_stab_selectable(),
            stability_yaw_engage_angle_rad: default_stab_engage(),
            stability_yaw_strength: default_stab_yaw_strength(),
            stability_grounded_multiplier: default_stab_ground_mult(),
            stability_upright_spring: default_stab_upright_spring(),
            stability_upright_damping: default_stab_upright_damping(),
            steering_slip_assist_default_enabled: default_steer_slip_on(),
            countersteer_default_enabled: default_counter_on(),
            steering_assist_player_selectable: default_steer_selectable(),
            auto_clutch_default_enabled: default_auto_clutch_on(),
            auto_clutch_player_selectable: default_auto_clutch_selectable(),
            launch_control_available: default_launch_available(),
            launch_control_default_enabled: default_launch_default(),
            launch_control_player_selectable: default_launch_selectable(),
            launch_control_target_rpm: default_launch_target_rpm(),
            launch_control_throttle_limit: default_launch_throttle_limit(),
            launch_control_clutch_release_rate_per_s: default_launch_release_rate(),
            brake_assist_available: default_brake_assist_available(),
            brake_assist_default_enabled: default_brake_assist_default(),
            brake_assist_player_selectable: default_brake_assist_selectable(),
            brake_assist_force_multiplier: default_brake_assist_mult(),
            handbrake_rear_only: true,
            handbrake_abs_interlock: true,
            handbrake_clutch_coupling: true,
            input_smoothing_enabled: true,
            input_smoothing_steering_rate: default_steer_rate(),
            input_smoothing_throttle_rate: default_throttle_rate(),
            input_smoothing_brake_rate: default_brake_rate(),
        }
    }
}
fn default_tc_available() -> bool { true }
fn default_tc_default() -> bool { false }
fn default_tc_selectable() -> bool { true }
fn default_tc_slip() -> f64 { 0.08 }
fn default_tc_cut_gain() -> f64 { 1.0 }
fn default_tc_actuator() -> String { "engine_torque".to_string() }
fn default_abs_available() -> bool { true }
fn default_abs_default() -> bool { false }
fn default_abs_selectable() -> bool { true }
fn default_stab_available() -> bool { false }
fn default_stab_default() -> bool { false }
fn default_stab_selectable() -> bool { true }
fn default_stab_engage() -> f64 { 0.01 }
fn default_stab_yaw_strength() -> f64 { 5.25 }
fn default_stab_ground_mult() -> f64 { 2.0 }
fn default_stab_upright_spring() -> f64 { 1.0 }
fn default_stab_upright_damping() -> f64 { 1000.0 }
fn default_steer_slip_on() -> bool { true }
fn default_counter_on() -> bool { true }
fn default_steer_selectable() -> bool { true }
fn default_auto_clutch_on() -> bool { true }
fn default_auto_clutch_selectable() -> bool { false }
fn default_launch_available() -> bool { false }
fn default_launch_default() -> bool { false }
fn default_launch_selectable() -> bool { true }
fn default_launch_target_rpm() -> f64 { 9000.0 }
fn default_launch_throttle_limit() -> f64 { 1.0 }
fn default_launch_release_rate() -> f64 { 2.0 }
fn default_brake_assist_available() -> bool { false }
fn default_brake_assist_default() -> bool { false }
fn default_brake_assist_selectable() -> bool { true }
fn default_brake_assist_mult() -> f64 { 1.0 }
fn default_steer_rate() -> f64 { 4.25 }
fn default_throttle_rate() -> f64 { 20.0 }
fn default_brake_rate() -> f64 { 10.0 }
fn default_true() -> bool { true }

// ── Conversion: JSON spec → VehicleConfig ────────────────────────────────────────

impl JsonVehicleSpec {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 && self.schema_version != 2 {
            return Err(format!("Unsupported schema_version: {}", self.schema_version));
        }
        if self.chassis.vehicle_mass <= 0.0 {
            return Err("vehicle_mass must be positive".to_string());
        }
        if self.geometry.wheelbase <= 0.0 {
            return Err("wheelbase must be positive".to_string());
        }
        if self.powertrain.gear_ratios.len() < 1 {
            return Err("At least one gear ratio required".to_string());
        }
        if self.powertrain.torque_curve.len() < 2 {
            return Err("At least 2 torque curve points required".to_string());
        }
        let split_sum = self.aero.split.front + self.aero.split.diffuser + self.aero.split.rear;
        if (split_sum - 1.0).abs() > 0.01 {
            return Err(format!("Aero split must sum to 1.0, got {split_sum}"));
        }
        if self.tires.front.radius <= 0.0 || self.tires.rear.radius <= 0.0 {
            return Err("Tire radii must be positive".to_string());
        }
        if self.suspension.front.spring_length <= 0.0 || self.suspension.rear.spring_length <= 0.0 {
            return Err("Spring lengths must be positive".to_string());
        }
        Ok(())
    }

    fn to_config(self) -> VehicleConfig {
        let mut surface_friction = HashMap::new();
        let mut surface_stiffness = HashMap::new();
        let mut surface_rolling_resistance = HashMap::new();

        for (name, entry) in &self.tires.surfaces {
            if let Ok(st) = parse_surface_type(name) {
                surface_friction.insert(st, entry.friction);
                surface_stiffness.insert(st, entry.stiffness);
                surface_rolling_resistance.insert(st, entry.rolling_resistance);
            }
        }

        VehicleConfig {
            schema_version: self.schema_version,
            vehicle_name: self.chassis.vehicle_name,
            vehicle_mass: self.chassis.vehicle_mass,
            front_weight_distribution: self.chassis.front_weight_distribution,
            center_of_gravity_height_offset: self.chassis.center_of_gravity_height_offset,
            inertia_multipliers: Vec3::new(
                self.chassis.inertia_multipliers.x,
                self.chassis.inertia_multipliers.y,
                self.chassis.inertia_multipliers.z,
            ),
            wheelbase: self.geometry.wheelbase,
            front_track: self.geometry.front_track,
            rear_track: self.geometry.rear_track,
            max_steering_angle: self.steering.max_steering_angle,
            front_steering_ratio: self.steering.front_steering_ratio,
            rear_steering_ratio: self.steering.rear_steering_ratio,
            steering_speed: self.steering.steering_speed,
            countersteer_speed: self.steering.countersteer_speed,
            steering_speed_decay: self.steering.steering_speed_decay,
            steering_slip_assist: self.steering.steering_slip_assist,
            countersteer_assist: self.steering.countersteer_assist,
            steering_exponent: self.steering.steering_exponent,
            ackermann: self.steering.ackermann,
            max_torque: self.powertrain.max_torque,
            max_rpm: self.powertrain.max_rpm,
            idle_rpm: self.powertrain.idle_rpm,
            motor_moment: self.powertrain.motor_moment,
            torque_curve: self.powertrain.torque_curve,
            gear_ratios: self.powertrain.gear_ratios,
            final_drive: self.powertrain.final_drive,
            reverse_ratio: self.powertrain.reverse_ratio,
            shift_time: self.powertrain.shift_time,
            automatic_transmission: self.powertrain.automatic_transmission,
            front_torque_split: self.powertrain.front_torque_split,
            diff_preload: self.powertrain.differential.preload_nm,
            diff_power_ramp_angle_deg: self.powertrain.differential.power_ramp_angle_deg,
            diff_coast_ramp_angle_deg: self.powertrain.differential.coast_ramp_angle_deg,
            diff_clutches: self.powertrain.differential.clutches,
            diff_clutch_friction_coeff: self.powertrain.differential.clutch_friction_coefficient,
            front_spring_length: self.suspension.front.spring_length,
            front_resting_ratio: self.suspension.front.resting_ratio,
            front_damping_ratio: self.suspension.front.damping_ratio,
            front_bump_damp_multiplier: self.suspension.front.bump_damp_multiplier,
            front_rebound_damp_multiplier: self.suspension.front.rebound_damp_multiplier,
            front_arb_ratio: self.suspension.front.arb_ratio,
            front_toe: self.suspension.front.toe,
            front_camber: self.suspension.front.camber,
            front_bump_stop_multiplier: self.suspension.front.bump_stop_multiplier,
            rear_spring_length: self.suspension.rear.spring_length,
            rear_resting_ratio: self.suspension.rear.resting_ratio,
            rear_damping_ratio: self.suspension.rear.damping_ratio,
            rear_bump_damp_multiplier: self.suspension.rear.bump_damp_multiplier,
            rear_rebound_damp_multiplier: self.suspension.rear.rebound_damp_multiplier,
            rear_arb_ratio: self.suspension.rear.arb_ratio,
            rear_toe: self.suspension.rear.toe,
            rear_camber: self.suspension.rear.camber,
            rear_bump_stop_multiplier: self.suspension.rear.bump_stop_multiplier,
            tri_ray_spacing_ratio: self.suspension.tri_ray_spacing_ratio,
            front_tire_radius: self.tires.front.radius,
            rear_tire_radius: self.tires.rear.radius,
            front_tire_width: self.tires.front.width,
            rear_tire_width: self.tires.rear.width,
            front_wheel_mass: self.tires.front.wheel_mass,
            rear_wheel_mass: self.tires.rear.wheel_mass,
            contact_patch: self.tires.contact_patch,
            braking_grip_multiplier: self.tires.braking_grip_multiplier,
            front_contact_patch: self.tires.front.contact_patch.unwrap_or(self.tires.contact_patch),
            rear_contact_patch: self.tires.rear.contact_patch.unwrap_or(self.tires.contact_patch),
            front_braking_grip: self.tires.front.braking_grip_multiplier.unwrap_or(self.tires.braking_grip_multiplier),
            rear_braking_grip: self.tires.rear.braking_grip_multiplier.unwrap_or(self.tires.braking_grip_multiplier),
            front_airborne_decay: self.tires.front.airborne_spin_decay_torque.unwrap_or(self.tires.airborne_spin_decay_torque),
            rear_airborne_decay: self.tires.rear.airborne_spin_decay_torque.unwrap_or(self.tires.airborne_spin_decay_torque),
            front_brake_bias: self.brakes.front_brake_bias,
            max_brake_torque: self.brakes.max_brake_torque,
            enable_abs: self.brakes.enable_abs,
            abs_pulse_time: self.brakes.abs_pulse_time,
            abs_spin_diff_threshold: self.brakes.abs_spin_diff_threshold,
            surface_friction,
            surface_stiffness,
            surface_rolling_resistance,
            coefficient_of_drag: self.aero.drag_coefficient,
            frontal_area: self.aero.frontal_area,
            coefficient_of_downforce: self.aero.downforce_coefficient,
            aero_split_front: self.aero.split.front,
            aero_split_diffuser: self.aero.split.diffuser,
            aero_split_rear: self.aero.split.rear,
            air_density: self.aero.air_density,
            aero_lag_tau: self.aero.lag_tau_s,
            aero_blend_min_speed: self.aero.blend_min_speed_mps,
            aero_blend_full_speed: self.aero.blend_full_speed_mps,
            aero_yaw_decay_exponent: self.aero.yaw_decay_exponent,
            aero_flex_coefficient: self.aero.flex_coefficient,

            // Auto-clutch / launch fidelity (previously hardcoded/ignored)
            max_clutch_torque_ratio: self.powertrain.max_clutch_torque_ratio,
            clutch_out_rpm_offset: self.powertrain.clutch_out_rpm_offset,
            idle_disengagement_hysteresis_rpm: self.powertrain.idle_disengagement_hysteresis_rpm,
            variable_drag_ratio: self.powertrain.variable_drag_ratio,
            constant_brake_ratio: self.powertrain.constant_brake_ratio,
            handbrake_torque_fraction: self.powertrain.handbrake_torque_fraction,
            diff_slip_transition_threshold_rad_s: self.powertrain.differential.slip_transition_threshold_rad_s,

            // Surface grip assists
            surface_lateral_grip_assist: build_surface_assist_map(&self.tires.surfaces, |e| e.lateral_grip_assist),
            surface_longitudinal_grip_ratio: build_surface_assist_map(&self.tires.surfaces, |e| e.longitudinal_grip_ratio),

            // Driving aids
            aids: AidsConfig {
                traction_control_available: self.aids.traction_control_available,
                traction_control_default_enabled: self.aids.traction_control_default_enabled,
                traction_control_player_selectable: self.aids.traction_control_player_selectable,
                traction_control_slip_threshold: self.aids.traction_control_slip_threshold,
                traction_control_cut_gain: self.aids.traction_control_cut_gain,
                traction_control_actuator: self.aids.traction_control_actuator.clone(),
                abs_available: self.aids.abs_available,
                abs_default_enabled: self.aids.abs_default_enabled,
                abs_player_selectable: self.aids.abs_player_selectable,
                stability_available: self.aids.stability_available,
                stability_default_enabled: self.aids.stability_default_enabled,
                stability_player_selectable: self.aids.stability_player_selectable,
                stability_yaw_engage_angle_rad: self.aids.stability_yaw_engage_angle_rad,
                stability_yaw_strength: self.aids.stability_yaw_strength,
                stability_grounded_multiplier: self.aids.stability_grounded_multiplier,
                stability_upright_spring: self.aids.stability_upright_spring,
                stability_upright_damping: self.aids.stability_upright_damping,
                steering_slip_assist_default_enabled: self.aids.steering_slip_assist_default_enabled,
                countersteer_default_enabled: self.aids.countersteer_default_enabled,
                steering_assist_player_selectable: self.aids.steering_assist_player_selectable,
                auto_clutch_default_enabled: self.aids.auto_clutch_default_enabled,
                auto_clutch_player_selectable: self.aids.auto_clutch_player_selectable,
                launch_control_available: self.aids.launch_control_available,
                launch_control_default_enabled: self.aids.launch_control_default_enabled,
                launch_control_player_selectable: self.aids.launch_control_player_selectable,
                launch_control_target_rpm: self.aids.launch_control_target_rpm,
                launch_control_throttle_limit: self.aids.launch_control_throttle_limit,
                launch_control_clutch_release_rate_per_s: self.aids.launch_control_clutch_release_rate_per_s,
                brake_assist_available: self.aids.brake_assist_available,
                brake_assist_default_enabled: self.aids.brake_assist_default_enabled,
                brake_assist_player_selectable: self.aids.brake_assist_player_selectable,
                brake_assist_force_multiplier: self.aids.brake_assist_force_multiplier,
                handbrake_rear_only: self.aids.handbrake_rear_only,
                handbrake_abs_interlock: self.aids.handbrake_abs_interlock,
                handbrake_clutch_coupling: self.aids.handbrake_clutch_coupling,
                input_smoothing_enabled: self.aids.input_smoothing_enabled,
                input_smoothing_steering_rate: self.aids.input_smoothing_steering_rate,
                input_smoothing_throttle_rate: self.aids.input_smoothing_throttle_rate,
                input_smoothing_brake_rate: self.aids.input_smoothing_brake_rate,
            },
        }
    }

    fn from_config(cfg: &VehicleConfig) -> Self {
        let mut surfaces = HashMap::new();
        for (st, friction) in &cfg.surface_friction {
            let name = surface_type_name(*st);
            let stiffness = cfg.surface_stiffness.get(st).copied().unwrap_or(1.0);
            let rolling = cfg.surface_rolling_resistance.get(st).copied().unwrap_or(1.0);
            let lateral = cfg.surface_lateral_grip_assist.get(st).copied().unwrap_or(0.05);
            let long = cfg.surface_longitudinal_grip_ratio.get(st).copied().unwrap_or(0.5);
            surfaces.insert(name, JsonSurfaceEntry {
                friction: *friction, stiffness, rolling_resistance: rolling,
                lateral_grip_assist: lateral, longitudinal_grip_ratio: long,
            });
        }
        Self {
            schema_version: 2,
            vehicle_id: "f1_94".to_string(),
            profile_name: cfg.vehicle_name.clone(),
            units: "SI".to_string(),
            chassis: JsonChassis {
                vehicle_name: cfg.vehicle_name.clone(),
                vehicle_mass: cfg.vehicle_mass,
                front_weight_distribution: cfg.front_weight_distribution,
                center_of_gravity_height_offset: cfg.center_of_gravity_height_offset,
                inertia_multipliers: JsonVec3 {
                    x: cfg.inertia_multipliers.x, y: cfg.inertia_multipliers.y, z: cfg.inertia_multipliers.z,
                },
            },
            geometry: JsonGeometry {
                wheelbase: cfg.wheelbase,
                front_track: cfg.front_track,
                rear_track: cfg.rear_track,
                wheel_hubs: None,
            },
            steering: JsonSteering {
                max_steering_angle: cfg.max_steering_angle,
                front_steering_ratio: cfg.front_steering_ratio,
                rear_steering_ratio: cfg.rear_steering_ratio,
                steering_speed: cfg.steering_speed,
                countersteer_speed: cfg.countersteer_speed,
                steering_speed_decay: cfg.steering_speed_decay,
                steering_slip_assist: cfg.steering_slip_assist,
                countersteer_assist: cfg.countersteer_assist,
                steering_exponent: cfg.steering_exponent,
                ackermann: cfg.ackermann,
            },
            powertrain: JsonPowertrain {
                max_torque: cfg.max_torque,
                max_rpm: cfg.max_rpm,
                idle_rpm: cfg.idle_rpm,
                motor_moment: cfg.motor_moment,
                torque_curve: cfg.torque_curve.clone(),
                gear_ratios: cfg.gear_ratios.clone(),
                final_drive: cfg.final_drive,
                reverse_ratio: cfg.reverse_ratio,
                shift_time: cfg.shift_time,
                automatic_transmission: cfg.automatic_transmission,
                front_torque_split: cfg.front_torque_split,
                gear_inertia: 0.02,
                max_clutch_torque_ratio: cfg.max_clutch_torque_ratio,
                clutch_out_rpm_offset: cfg.clutch_out_rpm_offset,
                idle_disengagement_hysteresis_rpm: cfg.idle_disengagement_hysteresis_rpm,
                variable_drag_ratio: cfg.variable_drag_ratio,
                constant_brake_ratio: cfg.constant_brake_ratio,
                handbrake_torque_fraction: cfg.handbrake_torque_fraction,
                automatic_shift: None,
                differential: JsonDifferential {
                    preload_nm: cfg.diff_preload,
                    power_ramp_angle_deg: cfg.diff_power_ramp_angle_deg,
                    coast_ramp_angle_deg: cfg.diff_coast_ramp_angle_deg,
                    clutches: cfg.diff_clutches,
                    clutch_friction_coefficient: cfg.diff_clutch_friction_coeff,
                    slip_transition_threshold_rad_s: cfg.diff_slip_transition_threshold_rad_s,
                },
            },
            suspension: JsonSuspension {
                front: JsonSuspensionAxle {
                    spring_length: cfg.front_spring_length,
                    resting_ratio: cfg.front_resting_ratio,
                    damping_ratio: cfg.front_damping_ratio,
                    bump_damp_multiplier: cfg.front_bump_damp_multiplier,
                    rebound_damp_multiplier: cfg.front_rebound_damp_multiplier,
                    arb_ratio: cfg.front_arb_ratio,
                    toe: cfg.front_toe,
                    camber: cfg.front_camber,
                    bump_stop_multiplier: cfg.front_bump_stop_multiplier,
                },
                rear: JsonSuspensionAxle {
                    spring_length: cfg.rear_spring_length,
                    resting_ratio: cfg.rear_resting_ratio,
                    damping_ratio: cfg.rear_damping_ratio,
                    bump_damp_multiplier: cfg.rear_bump_damp_multiplier,
                    rebound_damp_multiplier: cfg.rear_rebound_damp_multiplier,
                    arb_ratio: cfg.rear_arb_ratio,
                    toe: cfg.rear_toe,
                    camber: cfg.rear_camber,
                    bump_stop_multiplier: cfg.rear_bump_stop_multiplier,
                },
                tri_ray_spacing_ratio: cfg.tri_ray_spacing_ratio,
            },
            tires: JsonTires {
                front: JsonTireAxle {
                    radius: cfg.front_tire_radius,
                    width: cfg.front_tire_width,
                    wheel_mass: cfg.front_wheel_mass,
                    contact_patch: Some(cfg.front_contact_patch),
                    braking_grip_multiplier: Some(cfg.front_braking_grip),
                    airborne_spin_decay_torque: Some(cfg.front_airborne_decay),
                },
                rear: JsonTireAxle {
                    radius: cfg.rear_tire_radius,
                    width: cfg.rear_tire_width,
                    wheel_mass: cfg.rear_wheel_mass,
                    contact_patch: Some(cfg.rear_contact_patch),
                    braking_grip_multiplier: Some(cfg.rear_braking_grip),
                    airborne_spin_decay_torque: Some(cfg.rear_airborne_decay),
                },
                contact_patch: cfg.contact_patch,
                braking_grip_multiplier: cfg.braking_grip_multiplier,
                airborne_spin_decay_torque: cfg.front_airborne_decay,
                surfaces,
            },
            brakes: JsonBrakes {
                front_brake_bias: cfg.front_brake_bias,
                max_brake_torque: cfg.max_brake_torque,
                enable_abs: cfg.enable_abs,
                abs_pulse_time: cfg.abs_pulse_time,
                abs_spin_diff_threshold: cfg.abs_spin_diff_threshold,
            },
            aero: JsonAero {
                drag_coefficient: cfg.coefficient_of_drag,
                frontal_area: cfg.frontal_area,
                downforce_coefficient: cfg.coefficient_of_downforce,
                split: JsonAeroSplit {
                    front: cfg.aero_split_front, diffuser: cfg.aero_split_diffuser, rear: cfg.aero_split_rear,
                },
                air_density: cfg.air_density,
                lag_tau_s: cfg.aero_lag_tau,
                blend_min_speed_mps: cfg.aero_blend_min_speed,
                blend_full_speed_mps: cfg.aero_blend_full_speed,
                yaw_decay_exponent: cfg.aero_yaw_decay_exponent,
                flex_coefficient: cfg.aero_flex_coefficient,
            },
            coordinate_contract: Some({
                let mut m = HashMap::new();
                m.insert("forward".to_string(), "-Z".to_string());
                m.insert("right".to_string(), "+X".to_string());
                m.insert("up".to_string(), "+Y".to_string());
                m
            }),
            aids: JsonAids {
                traction_control_available: cfg.aids.traction_control_available,
                traction_control_default_enabled: cfg.aids.traction_control_default_enabled,
                traction_control_player_selectable: cfg.aids.traction_control_player_selectable,
                traction_control_slip_threshold: cfg.aids.traction_control_slip_threshold,
                traction_control_cut_gain: cfg.aids.traction_control_cut_gain,
                traction_control_actuator: cfg.aids.traction_control_actuator.clone(),
                abs_available: cfg.aids.abs_available,
                abs_default_enabled: cfg.aids.abs_default_enabled,
                abs_player_selectable: cfg.aids.abs_player_selectable,
                stability_available: cfg.aids.stability_available,
                stability_default_enabled: cfg.aids.stability_default_enabled,
                stability_player_selectable: cfg.aids.stability_player_selectable,
                stability_yaw_engage_angle_rad: cfg.aids.stability_yaw_engage_angle_rad,
                stability_yaw_strength: cfg.aids.stability_yaw_strength,
                stability_grounded_multiplier: cfg.aids.stability_grounded_multiplier,
                stability_upright_spring: cfg.aids.stability_upright_spring,
                stability_upright_damping: cfg.aids.stability_upright_damping,
                steering_slip_assist_default_enabled: cfg.aids.steering_slip_assist_default_enabled,
                countersteer_default_enabled: cfg.aids.countersteer_default_enabled,
                steering_assist_player_selectable: cfg.aids.steering_assist_player_selectable,
                auto_clutch_default_enabled: cfg.aids.auto_clutch_default_enabled,
                auto_clutch_player_selectable: cfg.aids.auto_clutch_player_selectable,
                launch_control_available: cfg.aids.launch_control_available,
                launch_control_default_enabled: cfg.aids.launch_control_default_enabled,
                launch_control_player_selectable: cfg.aids.launch_control_player_selectable,
                launch_control_target_rpm: cfg.aids.launch_control_target_rpm,
                launch_control_throttle_limit: cfg.aids.launch_control_throttle_limit,
                launch_control_clutch_release_rate_per_s: cfg.aids.launch_control_clutch_release_rate_per_s,
                brake_assist_available: cfg.aids.brake_assist_available,
                brake_assist_default_enabled: cfg.aids.brake_assist_default_enabled,
                brake_assist_player_selectable: cfg.aids.brake_assist_player_selectable,
                brake_assist_force_multiplier: cfg.aids.brake_assist_force_multiplier,
                handbrake_rear_only: cfg.aids.handbrake_rear_only,
                handbrake_abs_interlock: cfg.aids.handbrake_abs_interlock,
                handbrake_clutch_coupling: cfg.aids.handbrake_clutch_coupling,
                input_smoothing_enabled: cfg.aids.input_smoothing_enabled,
                input_smoothing_steering_rate: cfg.aids.input_smoothing_steering_rate,
                input_smoothing_throttle_rate: cfg.aids.input_smoothing_throttle_rate,
                input_smoothing_brake_rate: cfg.aids.input_smoothing_brake_rate,
            },
        }
    }
}

fn build_surface_assist_map<F>(surfaces: &HashMap<String, JsonSurfaceEntry>, f: F) -> HashMap<SurfaceType, f64>
where
    F: Fn(&JsonSurfaceEntry) -> f64,
{
    let mut map = HashMap::new();
    for (name, entry) in surfaces {
        if let Ok(st) = parse_surface_type(name) {
            map.insert(st, f(entry));
        }
    }
    map
}

fn parse_surface_type(name: &str) -> Result<SurfaceType, String> {
    match name {
        "Road" => Ok(SurfaceType::Road),
        "Curb" => Ok(SurfaceType::Curb),
        "Dirt" => Ok(SurfaceType::Dirt),
        "Grass" => Ok(SurfaceType::Grass),
        "Gravel" => Ok(SurfaceType::Gravel),
        "Sand" => Ok(SurfaceType::Sand),
        "Wall" => Ok(SurfaceType::Wall),
        "Metal" => Ok(SurfaceType::Metal),
        _ => Err(format!("Unknown surface type: {name}")),
    }
}

fn surface_type_name(st: SurfaceType) -> String {
    match st {
        SurfaceType::Road => "Road".to_string(),
        SurfaceType::Curb => "Curb".to_string(),
        SurfaceType::Dirt => "Dirt".to_string(),
        SurfaceType::Grass => "Grass".to_string(),
        SurfaceType::Gravel => "Gravel".to_string(),
        SurfaceType::Sand => "Sand".to_string(),
        SurfaceType::Wall => "Wall".to_string(),
        SurfaceType::Metal => "Metal".to_string(),
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;

    #[test]
    fn round_trip_preserves_all_fields() {
        let original = VehicleConfig::f1_94_canonical();
        let json_val = original.to_json_value();
        let json_str = serde_json::to_string_pretty(&json_val).unwrap();
        let loaded = VehicleConfig::from_json_str(&json_str).unwrap();
        assert_eq!(loaded.vehicle_name, original.vehicle_name);
        assert_eq!(loaded.vehicle_mass, original.vehicle_mass);
        assert_eq!(loaded.wheelbase, original.wheelbase);
        assert_eq!(loaded.diff_preload, original.diff_preload);
        assert_eq!(loaded.diff_clutch_friction_coeff, original.diff_clutch_friction_coeff);
        assert_eq!(loaded.gear_ratios, original.gear_ratios);
        assert_eq!(loaded.torque_curve, original.torque_curve);
        assert_eq!(loaded.coefficient_of_drag, original.coefficient_of_drag);
        assert_eq!(loaded.coefficient_of_downforce, original.coefficient_of_downforce);
        assert_eq!(loaded.aero_yaw_decay_exponent, original.aero_yaw_decay_exponent);
        assert!((loaded.front_weight_distribution - original.front_weight_distribution).abs() < 1e-10);
        assert_eq!(loaded.surface_friction.len(), original.surface_friction.len());
    }

    #[test]
    fn golden_json_matches_canonical() {
        let path = Path::new("data/vehicles/f1_94/f1_94_physics.json");
        if !path.exists() { return; }
        let loaded = VehicleConfig::from_json_path(path).unwrap();
        let canonical = VehicleConfig::f1_94_canonical();
        assert_eq!(loaded.vehicle_mass, canonical.vehicle_mass);
        assert_eq!(loaded.wheelbase, canonical.wheelbase);
        assert_eq!(loaded.diff_preload, canonical.diff_preload);
        assert_eq!(loaded.gear_ratios, canonical.gear_ratios);
    }

    #[test]
    fn rejects_invalid_schema_version() {
        let json = r#"{"schema_version": 99, "vehicle_id": "test"}"#;
        assert!(VehicleConfig::from_json_str(json).is_err());
    }

    #[test]
    fn rejects_non_positive_mass() {
        let json = r#"{"schema_version": 1, "chassis": {"vehicle_mass": -1.0}}"#;
        assert!(VehicleConfig::from_json_str(json).is_err());
    }

    #[test]
    fn minimal_json_uses_defaults() {
        let json = r#"{"schema_version": 1}"#;
        let cfg = VehicleConfig::from_json_str(json).unwrap();
        assert_eq!(cfg.vehicle_mass, 575.0);
        assert_eq!(cfg.diff_preload, 170.0);
    }

    #[test]
    fn schema_version_2_aids_round_trip() {
        let json = r#"{
            "schema_version": 2,
            "aids": {
                "traction_control_available": true,
                "traction_control_default_enabled": true,
                "traction_control_slip_threshold": 0.12,
                "abs_default_enabled": true,
                "stability_available": true,
                "stability_yaw_strength": 7.5,
                "brake_assist_force_multiplier": 1.4
            }
        }"#;
        let cfg = VehicleConfig::from_json_str(json).unwrap();
        assert!(cfg.aids.traction_control_available);
        assert!(cfg.aids.traction_control_default_enabled);
        assert!((cfg.aids.traction_control_slip_threshold - 0.12).abs() < 1e-9);
        assert!(cfg.aids.abs_default_enabled);
        assert!(cfg.aids.stability_available);
        assert!((cfg.aids.stability_yaw_strength - 7.5).abs() < 1e-9);
        assert!((cfg.aids.brake_assist_force_multiplier - 1.4).abs() < 1e-9);
    }

    #[test]
    fn deny_unknown_fields_rejects_typos() {
        let json_aids = r#"{"schema_version": 2, "aids": {"traction_controll": true}}"#;
        assert!(VehicleConfig::from_json_str(json_aids).is_err());

        let json_chassis = r#"{"schema_version": 2, "chassis": {"veh_mass": 575.0}}"#;
        assert!(VehicleConfig::from_json_str(json_chassis).is_err());

        let json_powertrain = r#"{"schema_version": 2, "powertrain": {"maximum_torque": 450.0}}"#;
        assert!(VehicleConfig::from_json_str(json_powertrain).is_err());

        let json_suspension = r#"{"schema_version": 2, "suspension": {"front": {"spring_len": 0.25}}}"#;
        assert!(VehicleConfig::from_json_str(json_suspension).is_err());
    }

    #[test]
    fn golden_json_schema_2_aids_present() {
        let path = Path::new("data/vehicles/f1_94/f1_94_physics.json");
        if !path.exists() { return; }
        let loaded = VehicleConfig::from_json_path(path).unwrap();
        assert!(loaded.aids.traction_control_available);
        assert!(loaded.aids.abs_available);
        assert_eq!(loaded.schema_version, 2);
    }

    #[test]
    fn fidelity_fields_consumed_from_json() {
        let path = Path::new("data/vehicles/f1_94/f1_94_physics.json");
        if !path.exists() { return; }
        let loaded = VehicleConfig::from_json_path(path).unwrap();
        assert!((loaded.max_clutch_torque_ratio - 1.6).abs() < 1e-9);
        assert!((loaded.clutch_out_rpm_offset - 1000.0).abs() < 1e-9);
        assert!((loaded.handbrake_torque_fraction - 0.4).abs() < 1e-9);
        assert!((loaded.diff_slip_transition_threshold_rad_s - 0.5).abs() < 1e-9);
        assert!((loaded.variable_drag_ratio - 0.10).abs() < 1e-9);
    }
}
