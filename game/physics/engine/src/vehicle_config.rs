use crate::types::{SurfaceType, Vec3, WheelIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Formula-90 vehicle configuration with GEVP-compatible suspension, tire and drivetrain semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleConfig {
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
    pub contact_patch: f64,
    pub braking_grip_multiplier: f64,
    pub front_brake_bias: f64,
    pub max_brake_torque: f64,

    // ABS
    pub enable_abs: bool,
    pub abs_pulse_time: f64,
    pub abs_spin_diff_threshold: f64,

    // Surface properties
    pub surface_friction: HashMap<SurfaceType, f64>,
    pub surface_stiffness: HashMap<SurfaceType, f64>,
    pub surface_rolling_resistance: HashMap<SurfaceType, f64>,

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

        Self {
            vehicle_name: "F1 1994 (V10)".to_string(),

            // Mass & Geometry
            vehicle_mass: 505.0,
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
            max_torque: 340.0,
            max_rpm: 17000.0,
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
            automatic_transmission: true,
            front_torque_split: 0.0,

            // Differential (Salisbury Clutch-Pack LSD, AMS2/Reiza aligned)
            diff_preload: 90.0,
            diff_power_ramp_angle_deg: 45.0,
            diff_coast_ramp_angle_deg: 60.0,
            diff_clutches: 4.0,
            diff_clutch_friction_coeff: 0.25,

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
            front_brake_bias: 0.57,
            max_brake_torque: 2800.0,

            // ABS
            enable_abs: false,
            abs_pulse_time: 0.03,
            abs_spin_diff_threshold: 12.0,

            // Surface Maps
            surface_friction,
            surface_stiffness,
            surface_rolling_resistance,

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
            front_brake_bias: 0.58,
            max_brake_torque: 2800.0,

            // ABS
            enable_abs: false,
            abs_pulse_time: 0.03,
            abs_spin_diff_threshold: 12.0,

            // Surface Maps
            surface_friction,
            surface_stiffness,
            surface_rolling_resistance,

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
}
