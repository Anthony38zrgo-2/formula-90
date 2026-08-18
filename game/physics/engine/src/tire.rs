// GEVP-aligned behavior; see THIRD_PARTY_NOTICES.md for upstream MIT attribution.
use crate::types::{SurfaceType, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// Runtime wheel state. The brush model follows GEVP's wheel.gd semantics;
/// +X is right and -Z is vehicle-forward in this Rust port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelTireState {
    pub spin: f64,
    pub steer_angle_rad: f64,
    pub slip_angle_rad: f64,
    pub slip_ratio: f64,
    pub lateral_force: f64,
    pub longitudinal_force: f64,
    pub rolling_resistance: f64,
    pub aligning_torque: f64,
    pub spin_velocity_diff: f64,
    pub wheel_moment: f64,
    pub applied_torque: f64,
    pub limit_spin: bool,
    pub reaction_torque: f64,
}

impl WheelTireState {
    pub fn new(config: &VehicleConfig, wheel: WheelIndex) -> Self {
        let radius = tire_radius(config, wheel);
        let mass = wheel_mass(config, wheel);
        Self {
            spin: 0.0,
            steer_angle_rad: 0.0,
            slip_angle_rad: 0.0,
            slip_ratio: 0.0,
            lateral_force: 0.0,
            longitudinal_force: 0.0,
            rolling_resistance: 0.0,
            aligning_torque: 0.0,
            spin_velocity_diff: 0.0,
            wheel_moment: 0.5 * mass * radius * radius,
            applied_torque: 0.0,
            limit_spin: false,
            reaction_torque: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireSystem {
    pub wheels: [WheelTireState; 4],
}

impl TireSystem {
    pub fn new(config: &VehicleConfig) -> Self {
        Self {
            wheels: [
                WheelTireState::new(config, WheelIndex::FrontLeft),
                WheelTireState::new(config, WheelIndex::FrontRight),
                WheelTireState::new(config, WheelIndex::RearLeft),
                WheelTireState::new(config, WheelIndex::RearRight),
            ],
        }
    }

    pub fn reaction_torques(&self) -> [f64; 4] {
        [
            self.wheels[0].reaction_torque,
            self.wheels[1].reaction_torque,
            self.wheels[2].reaction_torque,
            self.wheels[3].reaction_torque,
        ]
    }

    /// Update wheel angular speed from drivetrain/brake torque using the previous
    /// frame's tire reaction torque, matching GEVP's feedback order.
    pub fn process_wheel_torque(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        drive_torque_nm: f64,
        drive_inertia: f64,
        brake_torque_nm: f64,
        dt: f64,
    ) {
        let state = &mut self.wheels[wheel as usize];
        let dt = dt.max(1e-5);
        let is_driven = is_driven(config, wheel);
        let inertia = state.wheel_moment + if is_driven { drive_inertia.max(0.0) } else { 0.0 };
        let previous_spin = state.spin;

        state.applied_torque = if state.spin.abs() < 1e-6 {
            (drive_torque_nm - brake_torque_nm).abs()
        } else {
            (drive_torque_nm - brake_torque_nm * state.spin.signum()).abs()
        };

        // Tire reaction always participates. reaction_torque is the torque exerted
        // BY THE ROAD ON THE WHEEL (opposing drive torque).
        let mut net_torque = drive_torque_nm + state.reaction_torque;
        if state.spin.abs() > 1e-5 {
            net_torque -= brake_torque_nm * state.spin.signum();
        } else if brake_torque_nm >= net_torque.abs() {
            net_torque = 0.0;
        }

        let mut new_spin = state.spin + (net_torque / inertia.max(1e-6)) * dt;
        if previous_spin.abs() > 1e-6
            && previous_spin.signum() != new_spin.signum()
            && brake_torque_nm > drive_torque_nm.abs()
        {
            new_spin = 0.0;
        }
        state.spin = if new_spin.is_finite() { new_spin } else { 0.0 };
    }

    /// GEVP brush tire force calculation, adapted from +Z-forward to Godot's
    /// conventional -Z-forward used by Formula-90.
    #[allow(clippy::too_many_arguments)]
    pub fn process_wheel_forces(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        normal_force_n: f64,
        surface: SurfaceType,
        effective_friction: f64,
        effective_stiffness: f64,
        effective_rolling_resistance: f64,
        braking: bool,
        local_wheel_velocity: Vec3,
        dt: f64,
    ) {
        let state = &mut self.wheels[wheel as usize];
        let dt = dt.max(1e-5);
        let radius = tire_radius(config, wheel);
        let width_mm = tire_width(config, wheel) * 1000.0;

        if normal_force_n <= 1e-4 {
            state.slip_angle_rad = 0.0;
            state.slip_ratio = 0.0;
            state.lateral_force = 0.0;
            state.longitudinal_force = 0.0;
            state.rolling_resistance = 0.0;
            state.aligning_torque = 0.0;
            state.spin_velocity_diff = 0.0;
            state.reaction_torque = 0.0;
            state.limit_spin = false;
            // GEVP lets an airborne wheel decay slowly instead of snapping to road speed.
            state.spin -= state.spin.signum() * (tire_airborne_decay(config, wheel) / state.wheel_moment.max(1e-6)) * dt;
            if state.spin.abs() < 1e-4 { state.spin = 0.0; }
            return;
        }

        let v_forward = -local_wheel_velocity.z;
        let v_lateral = local_wheel_velocity.x;
        let wheel_velocity = state.spin * radius;

        // GEVP's lateral slip uses a normalized planar velocity capped to one.
        let planar_len = (v_lateral * v_lateral + v_forward * v_forward).sqrt();
        let planar_scale = if planar_len > 1e-9 { local_wheel_velocity.length().min(1.0) / planar_len } else { 0.0 };
        let planar_x = v_lateral * planar_scale;
        state.slip_angle_rad = (-planar_x.clamp(-1.0, 1.0)).asin();

        // Internally reproduce GEVP's longitudinal slip sign. Its wheel script uses
        // +Z-forward; this port uses -Z-forward, so force sign is inverted on output.
        let forward_sign = if v_forward.abs() < 1e-9 { 1.0 } else { v_forward.signum() };
        let gevp_slip_y = (v_forward.abs() - wheel_velocity * forward_sign) / (1.0 + v_forward.abs());
        state.slip_ratio = -gevp_slip_y;
        state.spin_velocity_diff = wheel_velocity - v_forward;

        let stiffness = 1_000_000.0 + 8_000_000.0 * effective_stiffness.max(0.0);
        let cornering_stiffness = 0.5 * stiffness * tire_contact_patch(config, wheel).powi(2);
        let friction = (effective_friction * normal_force_n
            - normal_force_n / (width_mm * tire_contact_patch(config, wheel) * 0.2).max(1e-6))
            .max(0.0);

        let sx = state.slip_angle_rad;
        let sy = gevp_slip_y;
        let slip_norm = ((cornering_stiffness * sy).powi(2) + (cornering_stiffness * sx).powi(2)).sqrt();
        let deflect = if slip_norm > 1e-9 { 1.0 / slip_norm } else { 1.0e9 };

        let needed_rolling_force = ((state.spin_velocity_diff * state.wheel_moment) / radius.max(1e-6)) / dt;
        let max_long_force = if state.applied_torque.abs() > needed_rolling_force.abs() {
            (state.applied_torque / radius.max(1e-6)).abs()
        } else {
            (needed_rolling_force / radius.max(1e-6)).abs()
        };
        let max_lateral_force = (config.mass_over_wheel(wheel) * v_lateral).abs() / dt;

        let braking_help = if sy > 0.3 && braking {
            1.0 + tire_braking_grip(config, wheel) * sy.abs().clamp(0.0, 1.0)
        } else {
            1.0
        };
        let longitudinal_grip_ratio = longitudinal_grip_ratio(surface);
        let lateral_assist = lateral_grip_assist(surface);

        let brush = (1.0 - friction * (1.0 - sy) * 0.25 * deflect) * deflect;
        let mut force_y_gevp = friction * longitudinal_grip_ratio * cornering_stiffness * sy * brush * braking_help * forward_sign;
        let mut force_x = friction * cornering_stiffness * sx * brush * (sx.abs() * lateral_assist + 1.0);

        if force_y_gevp.abs() > max_long_force {
            force_y_gevp = max_long_force * force_y_gevp.signum();
            state.limit_spin = true;
        } else {
            state.limit_spin = false;
        }
        if force_x.abs() > max_lateral_force {
            force_x = max_lateral_force * force_x.signum();
        }

        state.rolling_resistance = rolling_resistance_force(v_forward, normal_force_n) * effective_rolling_resistance;
        let mut longitudinal_force = -force_y_gevp;
        if v_forward.abs() > 0.05 {
            longitudinal_force -= state.rolling_resistance * v_forward.signum();
        }

        state.lateral_force = finite_or_zero(force_x);
        state.longitudinal_force = finite_or_zero(longitudinal_force);
        state.aligning_torque = 0.0; // GEVP produces yaw through contact forces, not a hidden Mz.
        // Invariant: reaction_torque is the torque exerted BY THE ROAD ON THE WHEEL.
        // For forward traction (longitudinal_force > 0), road reaction opposes forward spin (< 0).
        state.reaction_torque = finite_or_zero(-state.longitudinal_force * radius);
    }

    /// Compatibility entry point retained for existing Rust callers. New simulation
    /// code uses process_wheel_torque + process_wheel_forces explicitly.
    #[allow(clippy::too_many_arguments)]
    pub fn step_wheel(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        normal_force_n: f64,
        effective_friction_coef: f64,
        drive_torque_nm: f64,
        brake_torque_nm: f64,
        local_wheel_velocity: Vec3,
        dt: f64,
        effective_gear_ratio: f64,
        clutch_engagement: f64,
    ) {
        let total_ratio = effective_gear_ratio * config.final_drive;
        let drive_inertia = if is_driven(config, wheel) {
            (config.motor_moment * total_ratio * total_ratio * clutch_engagement.clamp(0.0, 1.0)).max(0.0)
        } else {
            0.0
        };
        self.process_wheel_torque(config, wheel, drive_torque_nm, drive_inertia, brake_torque_nm, dt);
        let surface = SurfaceType::Road;
        let stiffness = *config.surface_stiffness.get(&surface).unwrap_or(&5.0);
        let rolling = *config.surface_rolling_resistance.get(&surface).unwrap_or(&1.0);
        self.process_wheel_forces(
            config,
            wheel,
            normal_force_n,
            surface,
            effective_friction_coef,
            stiffness,
            rolling,
            brake_torque_nm > 0.0,
            local_wheel_velocity,
            dt,
        );
    }
}

pub fn is_driven(config: &VehicleConfig, wheel: WheelIndex) -> bool {
    if wheel.is_front() { config.front_torque_split > 0.0 } else { config.front_torque_split < 1.0 }
}

fn tire_radius(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_tire_radius } else { config.rear_tire_radius }
}
fn tire_width(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_tire_width } else { config.rear_tire_width }
}
fn wheel_mass(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_wheel_mass } else { config.rear_wheel_mass }
}
fn tire_contact_patch(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_contact_patch } else { config.rear_contact_patch }
}
fn tire_braking_grip(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_braking_grip } else { config.rear_braking_grip }
}
fn tire_airborne_decay(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_airborne_decay } else { config.rear_airborne_decay }
}
fn lateral_grip_assist(surface: SurfaceType) -> f64 {
    match surface {
        SurfaceType::Road | SurfaceType::Curb | SurfaceType::Metal => 0.05,
        _ => 0.0,
    }
}
fn longitudinal_grip_ratio(_surface: SurfaceType) -> f64 { 0.5 }
fn rolling_resistance_force(v_forward: f64, normal_force: f64) -> f64 {
    let c = 0.005 + 0.5 * (0.01 + 0.0095 * (v_forward * 0.036).powi(2));
    c * normal_force
}
fn finite_or_zero(v: f64) -> f64 { if v.is_finite() { v } else { 0.0 } }
