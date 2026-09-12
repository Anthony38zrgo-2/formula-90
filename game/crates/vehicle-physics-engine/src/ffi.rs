//! C-ABI FFI for Formula-90 / Godot integration.
//!
//! Recommended mode:
//!   Godot owns a RigidBody3D and all collision queries.
//!   Rust owns suspension, tire and drivetrain state and returns net force/torque.
//!   Call `f1_94_physics_solve_forces` once per physics tick.
//!
//! Legacy mode:
//!   `f1_94_physics_step` remains available for tools that let Rust integrate the body.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::simulation::*;
use crate::types::*;
use crate::vehicle_config::*;
use std::ffi::{c_char, c_void};

pub const F1_94_PHYSICS_ABI_VERSION: u32 = 13;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiRuntimeConfig {
    pub vehicle_mass: f64,
    pub front_brake_bias: f64,
    pub max_steering_angle: f64,
    pub max_torque: f64,
    pub coefficient_of_drag: f64,
    pub frontal_area: f64,
    pub air_density: f64,
    pub steering_exponent: f64,
    pub steering_speed: f64,
    pub countersteer_speed: f64,
    pub automatic_transmission: bool,

    // Differential (Salisbury Clutch-Pack LSD) — tunable at runtime from Godot
    // without recompiling Rust. GEVP's `rear_locking_differential_engage_torque`
    // maps to preload=engage_torque & friction_coeff=0 (flat capacity ceiling).
    pub diff_preload: f64,
    pub diff_power_ramp_angle_deg: f64,
    pub diff_coast_ramp_angle_deg: f64,
    pub diff_clutches: f64,
    pub diff_clutch_friction_coeff: f64,

    // Driving aids runtime enable mask (AidsMask::to_bits):
    // bit0=ABS, bit1=TC, bit2=stability, bit3=steering slip, bit4=countersteer,
    // bit5=auto-clutch, bit6=launch, bit7=brake-assist.
    pub aids_enabled_mask: u32,

    // Inertia multipliers from JSON (x, y, z)
    pub inertia_multiplier_x: f64,
    pub inertia_multiplier_y: f64,
    pub inertia_multiplier_z: f64,

    // Suspension geometry from JSON
    pub suspension_front_spring_length: f64,
    pub suspension_rear_spring_length: f64,
    pub suspension_front_resting_ratio: f64,
    pub suspension_rear_resting_ratio: f64,
}

#[no_mangle]
pub extern "C" fn f1_94_physics_abi_version() -> u32 {
    F1_94_PHYSICS_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn f1_94_physics_build_sha() -> *const c_char {
    concat!(env!("GIT_HASH"), "\0").as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn f1_94_physics_get_runtime_config(
    sim_ptr: *const c_void,
    out_config: *mut FfiRuntimeConfig,
) -> bool {
    if sim_ptr.is_null() || out_config.is_null() {
        return false;
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    // SAFETY: `out_config` is non-null (checked above) and points to a writable `FfiRuntimeConfig`.
    unsafe {
        *out_config = FfiRuntimeConfig {
            vehicle_mass: sim.config.vehicle_mass,
            front_brake_bias: sim.config.front_brake_bias,
            max_steering_angle: sim.config.max_steering_angle,
            max_torque: sim.config.max_torque,
            coefficient_of_drag: sim.config.coefficient_of_drag,
            frontal_area: sim.config.frontal_area,
            air_density: sim.config.air_density,
            steering_exponent: sim.config.steering_exponent,
            steering_speed: sim.config.steering_speed,
            countersteer_speed: sim.config.countersteer_speed,
            automatic_transmission: sim.config.automatic_transmission,
            diff_preload: sim.config.diff_preload,
            diff_power_ramp_angle_deg: sim.config.diff_power_ramp_angle_deg,
            diff_coast_ramp_angle_deg: sim.config.diff_coast_ramp_angle_deg,
            diff_clutches: sim.config.diff_clutches,
            diff_clutch_friction_coeff: sim.config.diff_clutch_friction_coeff,
            aids_enabled_mask: sim.aids.to_bits(),
            inertia_multiplier_x: sim.config.inertia_multipliers.x,
            inertia_multiplier_y: sim.config.inertia_multipliers.y,
            inertia_multiplier_z: sim.config.inertia_multipliers.z,
            suspension_front_spring_length: sim.config.front_spring_length,
            suspension_rear_spring_length: sim.config.rear_spring_length,
            suspension_front_resting_ratio: sim.config.front_resting_ratio,
            suspension_rear_resting_ratio: sim.config.rear_resting_ratio,
        };
    }
    true
}

/// Apply a runtime-tunable config to a simulator in place. Shared by the legacy
/// FFI (`f1_94_physics_apply_runtime_config`) and the orchestrator facade
/// (`formula90_core`), so every tunable JSON parameter stays usable at runtime on
/// BOTH integration paths. Sanitizes each field like the original FFI.
pub fn apply_runtime_config_to_sim(sim: &mut VehicleSimulator, cfg: &FfiRuntimeConfig) -> bool {
    if cfg.vehicle_mass.is_finite() && cfg.vehicle_mass > 0.0 {
        sim.config.vehicle_mass = cfg.vehicle_mass;
    }
    if cfg.front_brake_bias.is_finite()
        && cfg.front_brake_bias >= 0.0
        && cfg.front_brake_bias <= 1.0
    {
        sim.config.front_brake_bias = cfg.front_brake_bias;
    }
    if cfg.max_steering_angle.is_finite() && cfg.max_steering_angle > 0.0 {
        sim.config.max_steering_angle = cfg.max_steering_angle;
    }
    if cfg.max_torque.is_finite() && cfg.max_torque > 0.0 {
        sim.config.max_torque = cfg.max_torque;
    }
    if cfg.coefficient_of_drag.is_finite() && cfg.coefficient_of_drag >= 0.0 {
        sim.config.coefficient_of_drag = cfg.coefficient_of_drag;
    }
    if cfg.frontal_area.is_finite() && cfg.frontal_area > 0.0 {
        sim.config.frontal_area = cfg.frontal_area;
    }
    if cfg.air_density.is_finite() && cfg.air_density > 0.0 {
        sim.config.air_density = cfg.air_density;
    }
    if cfg.steering_exponent.is_finite() && cfg.steering_exponent > 0.0 {
        sim.config.steering_exponent = cfg.steering_exponent;
    }
    if cfg.steering_speed.is_finite() && cfg.steering_speed > 0.0 {
        sim.config.steering_speed = cfg.steering_speed;
    }
    if cfg.countersteer_speed.is_finite() && cfg.countersteer_speed > 0.0 {
        sim.config.countersteer_speed = cfg.countersteer_speed;
    }
    sim.config.automatic_transmission = cfg.automatic_transmission;

    if cfg.diff_preload.is_finite() && cfg.diff_preload > 0.0 {
        sim.config.diff_preload = cfg.diff_preload;
    }
    if cfg.diff_power_ramp_angle_deg.is_finite() && cfg.diff_power_ramp_angle_deg > 0.0 {
        sim.config.diff_power_ramp_angle_deg = cfg.diff_power_ramp_angle_deg;
    }
    if cfg.diff_coast_ramp_angle_deg.is_finite() && cfg.diff_coast_ramp_angle_deg > 0.0 {
        sim.config.diff_coast_ramp_angle_deg = cfg.diff_coast_ramp_angle_deg;
    }
    if cfg.diff_clutches.is_finite() && cfg.diff_clutches > 0.0 {
        sim.config.diff_clutches = cfg.diff_clutches;
    }
    if cfg.diff_clutch_friction_coeff.is_finite() && cfg.diff_clutch_friction_coeff >= 0.0 {
        sim.config.diff_clutch_friction_coeff = cfg.diff_clutch_friction_coeff;
    }
    if cfg.inertia_multiplier_x.is_finite() && cfg.inertia_multiplier_x > 0.0 {
        sim.config.inertia_multipliers.x = cfg.inertia_multiplier_x;
    }
    if cfg.inertia_multiplier_y.is_finite() && cfg.inertia_multiplier_y > 0.0 {
        sim.config.inertia_multipliers.y = cfg.inertia_multiplier_y;
    }
    if cfg.inertia_multiplier_z.is_finite() && cfg.inertia_multiplier_z > 0.0 {
        sim.config.inertia_multipliers.z = cfg.inertia_multiplier_z;
    }
    if cfg.suspension_front_spring_length.is_finite() && cfg.suspension_front_spring_length > 0.0 {
        sim.config.front_spring_length = cfg.suspension_front_spring_length;
    }
    if cfg.suspension_rear_spring_length.is_finite() && cfg.suspension_rear_spring_length > 0.0 {
        sim.config.rear_spring_length = cfg.suspension_rear_spring_length;
    }
    if cfg.suspension_front_resting_ratio.is_finite() && cfg.suspension_front_resting_ratio > 0.0 {
        sim.config.front_resting_ratio = cfg.suspension_front_resting_ratio;
    }
    if cfg.suspension_rear_resting_ratio.is_finite() && cfg.suspension_rear_resting_ratio > 0.0 {
        sim.config.rear_resting_ratio = cfg.suspension_rear_resting_ratio;
    }
    sim.aids = AidsMask::from_bits(cfg.aids_enabled_mask);
    true
}

#[no_mangle]
pub extern "C" fn f1_94_physics_apply_runtime_config(
    sim_ptr: *mut c_void,
    config_ptr: *const FfiRuntimeConfig,
) -> bool {
    if sim_ptr.is_null() || config_ptr.is_null() {
        return false;
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &mut *(sim_ptr as *mut VehicleSimulator) };
    // SAFETY: `config_ptr` is non-null (checked above) and points to a valid `FfiRuntimeConfig`.
    apply_runtime_config_to_sim(sim, unsafe { &*config_ptr })
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiRaycastHit {
    pub is_colliding: bool,
    /// Distance from the wheel suspension ray origin to the collision point, in metres.
    pub distance: f64,
    /// Collision point in world space.
    pub point_x: f64,
    pub point_y: f64,
    pub point_z: f64,
    /// Collision normal in world space.
    pub normal_x: f64,
    pub normal_y: f64,
    pub normal_z: f64,
    /// 0=Road, 1=Curb, 2=Dirt, 3=Grass, 4=Gravel, 5=Sand, 6=Wall, 7=Metal.
    pub surface_type: u32,
}

impl From<FfiRaycastHit> for RaycastHit {
    fn from(f: FfiRaycastHit) -> Self {
        let surface = match f.surface_type {
            1 => SurfaceType::Curb,
            2 => SurfaceType::Dirt,
            3 => SurfaceType::Grass,
            4 => SurfaceType::Gravel,
            5 => SurfaceType::Sand,
            6 => SurfaceType::Wall,
            7 => SurfaceType::Metal,
            _ => SurfaceType::Road,
        };
        Self {
            is_colliding: f.is_colliding,
            distance: f.distance,
            point: Vec3::new(f.point_x, f.point_y, f.point_z),
            normal: Vec3::new(f.normal_x, f.normal_y, f.normal_z),
            surface,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiTriRaycastSample {
    pub inner: FfiRaycastHit,
    pub center: FfiRaycastHit,
    pub outer: FfiRaycastHit,
}

impl From<FfiTriRaycastSample> for TriRaycastSample {
    fn from(f: FfiTriRaycastSample) -> Self {
        Self {
            inner: f.inner.into(),
            center: f.center.into(),
            outer: f.outer.into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiVehicleInput {
    pub throttle: f64,
    pub steering: f64,
    pub brake: f64,
    pub handbrake: f64,
    /// 0 = engaged, 1 = disengaged.
    pub clutch: f64,
    /// -2=None, -1=Reverse, 0=Neutral, 1..6=forward gear.
    pub gear_request: i32,
}

impl From<FfiVehicleInput> for VehicleInput {
    fn from(f: FfiVehicleInput) -> Self {
        Self {
            throttle: f.throttle,
            steering: f.steering,
            brake: f.brake,
            handbrake: f.handbrake,
            clutch: f.clutch,
            gear_request: if f.gear_request >= -1 {
                Some(f.gear_request as i8)
            } else {
                None
            },
        }
    }
}

/// Godot RigidBody3D state supplied to the Rust force solver.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiBodyKinematics {
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub rot_quat_x: f64,
    pub rot_quat_y: f64,
    pub rot_quat_z: f64,
    pub rot_quat_w: f64,
    pub lin_vel_x: f64,
    pub lin_vel_y: f64,
    pub lin_vel_z: f64,
    pub ang_vel_x: f64,
    pub ang_vel_y: f64,
    pub ang_vel_z: f64,
}

impl From<FfiBodyKinematics> for BodyKinematics {
    fn from(f: FfiBodyKinematics) -> Self {
        let q = Quat::new(f.rot_quat_x, f.rot_quat_y, f.rot_quat_z, f.rot_quat_w).normalized();
        Self {
            transform: Transform3D::new(Vec3::new(f.pos_x, f.pos_y, f.pos_z), q.to_mat3()),
            orientation: q,
            linear_velocity: Vec3::new(f.lin_vel_x, f.lin_vel_y, f.lin_vel_z),
            angular_velocity: Vec3::new(f.ang_vel_x, f.ang_vel_y, f.ang_vel_z),
        }
    }
}

/// Net non-gravity force/torque to apply to the Godot RigidBody3D this tick.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FfiForceTorqueOutput {
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub torque_x: f64,
    pub torque_y: f64,
    pub torque_z: f64,
}

impl From<ForceTorqueOutput> for FfiForceTorqueOutput {
    fn from(v: ForceTorqueOutput) -> Self {
        Self {
            force_x: v.force_world.x,
            force_y: v.force_world.y,
            force_z: v.force_world.z,
            torque_x: v.torque_world.x,
            torque_y: v.torque_world.y,
            torque_z: v.torque_world.z,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FfiTelemetryOutput {
    pub sim_time: f64,
    pub speed_kmh: f64,
    pub rpm: f64,
    pub gear: i32,
    pub engine_torque: f64,
    pub clutch_engagement: f64,
    pub throttle: f64,
    pub brake: f64,
    pub steer: f64,

    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub rot_quat_x: f64,
    pub rot_quat_y: f64,
    pub rot_quat_z: f64,
    pub rot_quat_w: f64,

    pub lin_vel_x: f64,
    pub lin_vel_y: f64,
    pub lin_vel_z: f64,
    pub ang_vel_x: f64,
    pub ang_vel_y: f64,
    pub ang_vel_z: f64,
    pub lat_g: f64,
    pub long_g: f64,
    pub vert_g: f64,

    pub fl_comp_mm: f64,
    pub fr_comp_mm: f64,
    pub rl_comp_mm: f64,
    pub rr_comp_mm: f64,

    pub fl_spin: f64,
    pub fr_spin: f64,
    pub rl_spin: f64,
    pub rr_spin: f64,

    pub fl_slip: f64,
    pub fr_slip: f64,
    pub rl_slip: f64,
    pub rr_slip: f64,

    pub steer_angle_rad: f64,

    // CORR-02 append-only diagnostics: actual clutch/wheel loads.
    pub clutch_torque: f64,
    pub fl_drive_torque: f64,
    pub fr_drive_torque: f64,
    pub rl_drive_torque: f64,
    pub rr_drive_torque: f64,
    pub fl_normal_force: f64,
    pub fr_normal_force: f64,
    pub rl_normal_force: f64,
    pub rr_normal_force: f64,

    // Append-only aids diagnostics
    pub abs_active: bool,
    pub tc_active: bool,
    pub tc_cut_ratio: f64,
    pub aids_enabled_mask: u32,

    // Tire pressure + thermal telemetry (per wheel, WheelIndex order FL/FR/RL/RR).
    // Gauge kPa and 5-node temperatures in degrees C.
    pub fl_pressure_kpa: f64,
    pub fr_pressure_kpa: f64,
    pub rl_pressure_kpa: f64,
    pub rr_pressure_kpa: f64,
    pub fl_tread_inner_c: f64,
    pub fr_tread_inner_c: f64,
    pub rl_tread_inner_c: f64,
    pub rr_tread_inner_c: f64,
    pub fl_tread_center_c: f64,
    pub fr_tread_center_c: f64,
    pub rl_tread_center_c: f64,
    pub rr_tread_center_c: f64,
    pub fl_tread_outer_c: f64,
    pub fr_tread_outer_c: f64,
    pub rl_tread_outer_c: f64,
    pub rr_tread_outer_c: f64,
    pub fl_carcass_c: f64,
    pub fr_carcass_c: f64,
    pub rl_carcass_c: f64,
    pub rr_carcass_c: f64,
    pub fl_gas_c: f64,
    pub fr_gas_c: f64,
    pub rl_gas_c: f64,
    pub rr_gas_c: f64,

    // Brake thermal + duct telemetry. Append-only ABI 9 block.
    pub fl_brake_disc_c: f64,
    pub fr_brake_disc_c: f64,
    pub rl_brake_disc_c: f64,
    pub rr_brake_disc_c: f64,
    pub fl_brake_rim_c: f64,
    pub fr_brake_rim_c: f64,
    pub rl_brake_rim_c: f64,
    pub rr_brake_rim_c: f64,
    pub fl_brake_efficiency: f64,
    pub fr_brake_efficiency: f64,
    pub rl_brake_efficiency: f64,
    pub rr_brake_efficiency: f64,
    pub fl_duct_mass_flow_kg_s: f64,
    pub fr_duct_mass_flow_kg_s: f64,
    pub rl_duct_mass_flow_kg_s: f64,
    pub rr_duct_mass_flow_kg_s: f64,
    pub fl_duct_drag_n: f64,
    pub fr_duct_drag_n: f64,
    pub rl_duct_drag_n: f64,
    pub rr_duct_drag_n: f64,
    pub brake_optimal_min_c: f64,
    pub brake_optimal_max_c: f64,
    pub brake_fade_start_c: f64,
    pub brake_critical_c: f64,

    // Brake energy diagnostics. Append-only ABI 10 block.
    pub fl_brake_torque_nm: f64,
    pub fr_brake_torque_nm: f64,
    pub rl_brake_torque_nm: f64,
    pub rr_brake_torque_nm: f64,
    pub fl_brake_spin_pre_rad_s: f64,
    pub fr_brake_spin_pre_rad_s: f64,
    pub rl_brake_spin_pre_rad_s: f64,
    pub rr_brake_spin_pre_rad_s: f64,
    pub fl_brake_spin_post_rad_s: f64,
    pub fr_brake_spin_post_rad_s: f64,
    pub rl_brake_spin_post_rad_s: f64,
    pub rr_brake_spin_post_rad_s: f64,
    pub fl_brake_power_w: f64,
    pub fr_brake_power_w: f64,
    pub rl_brake_power_w: f64,
    pub rr_brake_power_w: f64,
    pub fl_brake_energy_j: f64,
    pub fr_brake_energy_j: f64,
    pub rl_brake_energy_j: f64,
    pub rr_brake_energy_j: f64,
    // Lumped rotor cooling diagnostics (couples to the compact brake model).
    pub fl_brake_natural_cooling_w_k: f64,
    pub fr_brake_natural_cooling_w_k: f64,
    pub rl_brake_natural_cooling_w_k: f64,
    pub rr_brake_natural_cooling_w_k: f64,
    pub fl_brake_speed_cooling_w_k: f64,
    pub fr_brake_speed_cooling_w_k: f64,
    pub rl_brake_speed_cooling_w_k: f64,
    pub rr_brake_speed_cooling_w_k: f64,

    // Append-only ABI 13 traction-control diagnostics.
    pub tc_eligible: bool,
    pub tc_gear_authority: f64,
    pub tc_slip_target: f64,
    pub tc_raw_cut_ratio: f64,
    pub tc_slip_ratio: [f64; 4],
    pub drive_torque_pre_tc_nm: [f64; 4],
    pub pre_tc_drive_power_w: f64,
    pub net_drive_power_w: f64,
}

#[no_mangle]
pub extern "C" fn f1_94_physics_create_default() -> *mut c_void {
    let cfg = VehicleConfig::f1_2026_2008_canonical();
    let spawn_height = default_spawn_height(&cfg);
    let sim = Box::new(VehicleSimulator::new(
        cfg,
        Vec3::new(0.0, spawn_height, 0.0),
        0.0,
    ));
    Box::into_raw(sim) as *mut c_void
}

/// Create a VehicleSimulator from a JSON configuration string.
/// Returns null pointer on error. Error message written to error_buffer (null-terminated).
/// If error_buffer is null or error_buffer_len is 0, error is silently discarded.
#[no_mangle]
pub extern "C" fn f1_94_physics_create_from_json(
    json_ptr: *const u8,
    json_len: u32,
    error_buffer: *mut u8,
    error_buffer_len: u32,
) -> *mut c_void {
    if json_ptr.is_null() || json_len == 0 {
        write_error(error_buffer, error_buffer_len, "null or empty JSON input");
        return std::ptr::null_mut();
    }
    // SAFETY: `json_ptr` is non-null and `json_len > 0` (checked above); the caller guarantees
    // a readable buffer of `json_len` bytes.
    let json_bytes = unsafe { std::slice::from_raw_parts(json_ptr, json_len as usize) };
    let json_str = match std::str::from_utf8(json_bytes) {
        Ok(s) => s,
        Err(e) => {
            write_error(
                error_buffer,
                error_buffer_len,
                &format!("invalid UTF-8: {e}"),
            );
            return std::ptr::null_mut();
        }
    };
    let cfg = match VehicleConfig::from_json_str(json_str) {
        Ok(c) => c,
        Err(e) => {
            write_error(error_buffer, error_buffer_len, &e);
            return std::ptr::null_mut();
        }
    };
    let spawn_height = default_spawn_height(&cfg);
    let sim = Box::new(VehicleSimulator::new(
        cfg,
        Vec3::new(0.0, spawn_height, 0.0),
        0.0,
    ));
    Box::into_raw(sim) as *mut c_void
}

fn write_error(buf: *mut u8, buf_len: u32, msg: &str) {
    if buf.is_null() || buf_len == 0 {
        return;
    }
    let bytes = msg.as_bytes();
    let copy_len = bytes.len().min(buf_len as usize - 1);
    // SAFETY: `buf` is non-null and `buf_len > 0` (checked above); `copy_len` is clamped to
    // `buf_len - 1`, so the copy and the NUL terminator stay in bounds.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, copy_len);
        *buf.add(copy_len) = 0;
    }
}

#[no_mangle]
pub extern "C" fn f1_94_physics_create_with_pos(
    pos_x: f64,
    pos_y: f64,
    pos_z: f64,
    yaw_rad: f64,
) -> *mut c_void {
    let cfg = VehicleConfig::f1_2026_2008_canonical();
    let sim = Box::new(VehicleSimulator::new(
        cfg,
        Vec3::new(pos_x, pos_y, pos_z),
        yaw_rad,
    ));
    Box::into_raw(sim) as *mut c_void
}

#[no_mangle]
pub extern "C" fn f1_94_physics_reset(
    sim_ptr: *mut c_void,
    pos_x: f64,
    pos_y: f64,
    pos_z: f64,
    yaw_rad: f64,
) {
    if sim_ptr.is_null() {
        return;
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &mut *(sim_ptr as *mut VehicleSimulator) };
    *sim = VehicleSimulator::new(sim.config.clone(), Vec3::new(pos_x, pos_y, pos_z), yaw_rad);
}

/// Recommended API for Godot. Rust does NOT integrate the body and does NOT include gravity.
/// Godot should apply the returned force and torque to its RigidBody3D every physics tick.
#[no_mangle]
pub extern "C" fn f1_94_physics_solve_forces(
    sim_ptr: *mut c_void,
    body_ptr: *const FfiBodyKinematics,
    input_ptr: *const FfiVehicleInput,
    samples_ptr: *const FfiTriRaycastSample,
    dt: f64,
    out_forces: *mut FfiForceTorqueOutput,
    out_telem: *mut FfiTelemetryOutput,
) {
    if sim_ptr.is_null() || body_ptr.is_null() || input_ptr.is_null() || samples_ptr.is_null() {
        return;
    }

    // SAFETY: all four pointers are non-null (checked above); per the C ABI contract
    // `sim_ptr` is a live `VehicleSimulator` and `body_ptr`/`input_ptr` point to valid PODs.
    let sim = unsafe { &mut *(sim_ptr as *mut VehicleSimulator) };
    let body = BodyKinematics::from(unsafe { *body_ptr });
    let input = VehicleInput::from(unsafe { *input_ptr });
    let samples = read_samples(samples_ptr);
    // This ABI has no underfloor measurements. The explicit no-probe path keeps
    // free-air aerodynamics without manufacturing ground-effect geometry.
    let (forces, telem) = sim.solve_external(body, &input, &samples, dt);

    if !out_forces.is_null() {
        // SAFETY: `out_forces` is non-null (checked above) and points to a writable `FfiForceTorqueOutput`.
        unsafe {
            *out_forces = forces.into();
        }
    }
    write_telemetry(sim, &telem, out_telem);
}

/// Legacy standalone API. Rust integrates position/orientation internally.
#[no_mangle]
pub extern "C" fn f1_94_physics_step(
    sim_ptr: *mut c_void,
    input_ptr: *const FfiVehicleInput,
    samples_ptr: *const FfiTriRaycastSample,
    dt: f64,
    out_telem: *mut FfiTelemetryOutput,
) {
    if sim_ptr.is_null() || input_ptr.is_null() || samples_ptr.is_null() {
        return;
    }
    // SAFETY: `sim_ptr` and `input_ptr` are non-null (checked above); per the C ABI contract
    // `sim_ptr` is a live `VehicleSimulator` and `input_ptr` points to a valid POD.
    let sim = unsafe { &mut *(sim_ptr as *mut VehicleSimulator) };
    let input = VehicleInput::from(unsafe { *input_ptr });
    let samples = read_samples(samples_ptr);
    let telem = sim.step(&input, &samples, dt);
    write_telemetry(sim, &telem, out_telem);
}

#[no_mangle]
pub extern "C" fn f1_94_physics_get_wheel_anchor_local(
    sim_ptr: *const c_void,
    wheel_idx: u32,
    out_x: *mut f64,
    out_y: *mut f64,
    out_z: *mut f64,
) {
    if sim_ptr.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let wheel = WheelIndex::ALL[(wheel_idx as usize).min(3)];
    let anchor = sim.config.wheel_anchor_local(wheel);
    // SAFETY: `out_x`/`out_y`/`out_z` are non-null (checked above) and point to writable `f64`s.
    unsafe {
        *out_x = anchor.x;
        *out_y = anchor.y;
        *out_z = anchor.z;
    }
}

/// Offset from center ray to inner/outer ray. Total sampled width is 2*span.
#[no_mangle]
pub extern "C" fn f1_94_physics_get_tri_ray_span(sim_ptr: *const c_void, wheel_idx: u32) -> f64 {
    if sim_ptr.is_null() {
        return 0.12;
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let wheel = WheelIndex::ALL[(wheel_idx as usize).min(3)];
    let tire_w = if wheel.is_front() {
        sim.config.front_tire_width
    } else {
        sim.config.rear_tire_width
    };
    tire_w * sim.config.tri_ray_spacing_ratio
}

#[no_mangle]
pub extern "C" fn f1_94_physics_get_default_spawn_height(sim_ptr: *const c_void) -> f64 {
    if sim_ptr.is_null() {
        return default_spawn_height(&VehicleConfig::f1_94_canonical());
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    default_spawn_height(&sim.config)
}

#[no_mangle]
pub extern "C" fn f1_94_physics_get_center_of_mass_local(
    sim_ptr: *const c_void,
    out_x: *mut f64,
    out_y: *mut f64,
    out_z: *mut f64,
) {
    if sim_ptr.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() {
        return;
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let cg = center_of_mass_local(&sim.config);
    // SAFETY: `out_x`/`out_y`/`out_z` are non-null (checked above) and point to writable `f64`s.
    unsafe {
        *out_x = cg.x;
        *out_y = cg.y;
        *out_z = cg.z;
    }
}

#[no_mangle]
pub extern "C" fn f1_94_physics_get_ray_length(sim_ptr: *const c_void, wheel_idx: u32) -> f64 {
    if sim_ptr.is_null() {
        return 0.6;
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let wheel = WheelIndex::ALL[(wheel_idx as usize).min(3)];
    if wheel.is_front() {
        sim.config.front_spring_length + sim.config.front_tire_radius
    } else {
        sim.config.rear_spring_length + sim.config.rear_tire_radius
    }
}

#[no_mangle]
pub extern "C" fn f1_94_physics_get_vehicle_mass(sim_ptr: *const c_void) -> f64 {
    if sim_ptr.is_null() {
        return VehicleConfig::f1_94_canonical().vehicle_mass;
    }
    // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, points to a live `VehicleSimulator`.
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    sim.config.vehicle_mass
}

#[no_mangle]
pub extern "C" fn f1_94_physics_destroy(sim_ptr: *mut c_void) {
    if !sim_ptr.is_null() {
        // SAFETY: `sim_ptr` is non-null (checked above) and, per the C ABI contract, is an
        // unreleased pointer from `f1_94_physics_create_*`.
        unsafe {
            drop(Box::from_raw(sim_ptr as *mut VehicleSimulator));
        }
    }
}

fn read_samples(samples_ptr: *const FfiTriRaycastSample) -> [TriRaycastSample; 4] {
    // SAFETY: callers guarantee `samples_ptr` points to 4 valid `FfiTriRaycastSample` (Inner/Center/Outer x4).
    let ffi_samples = unsafe { std::slice::from_raw_parts(samples_ptr, 4) };
    [
        ffi_samples[0].into(),
        ffi_samples[1].into(),
        ffi_samples[2].into(),
        ffi_samples[3].into(),
    ]
}

fn write_telemetry(
    sim: &VehicleSimulator,
    telem: &crate::telemetry::TelemetryFrame,
    out: *mut FfiTelemetryOutput,
) {
    if out.is_null() {
        return;
    }
    let q = sim.state.orientation;
    // SAFETY: `out` is non-null (checked above) and points to a writable `FfiTelemetryOutput`.
    unsafe {
        *out = FfiTelemetryOutput {
            sim_time: sim.state.sim_time,
            speed_kmh: telem.speed_kmh,
            rpm: telem.rpm,
            gear: telem.gear as i32,
            engine_torque: sim.state.powertrain.engine_torque,
            clutch_engagement: sim.state.powertrain.clutch_engagement,
            throttle: telem.throttle,
            brake: telem.brake,
            steer: telem.steering,
            pos_x: sim.state.transform.origin.x,
            pos_y: sim.state.transform.origin.y,
            pos_z: sim.state.transform.origin.z,
            rot_quat_x: q.x,
            rot_quat_y: q.y,
            rot_quat_z: q.z,
            rot_quat_w: q.w,
            lin_vel_x: sim.state.linear_velocity.x,
            lin_vel_y: sim.state.linear_velocity.y,
            lin_vel_z: sim.state.linear_velocity.z,
            ang_vel_x: sim.state.angular_velocity.x,
            ang_vel_y: sim.state.angular_velocity.y,
            ang_vel_z: sim.state.angular_velocity.z,
            lat_g: telem.lat_g,
            long_g: telem.long_g,
            vert_g: telem.vert_g,
            fl_comp_mm: sim.state.suspension.wheels[0].compression_mm,
            fr_comp_mm: sim.state.suspension.wheels[1].compression_mm,
            rl_comp_mm: sim.state.suspension.wheels[2].compression_mm,
            rr_comp_mm: sim.state.suspension.wheels[3].compression_mm,
            fl_spin: sim.state.tires.wheels[0].spin,
            fr_spin: sim.state.tires.wheels[1].spin,
            rl_spin: sim.state.tires.wheels[2].spin,
            rr_spin: sim.state.tires.wheels[3].spin,
            fl_slip: sim.state.tires.wheels[0].slip_ratio,
            fr_slip: sim.state.tires.wheels[1].slip_ratio,
            rl_slip: sim.state.tires.wheels[2].slip_ratio,
            rr_slip: sim.state.tires.wheels[3].slip_ratio,
            steer_angle_rad: sim.state.tires.wheels[0].steer_angle_rad,
            clutch_torque: sim.state.powertrain.clutch_torque,
            fl_drive_torque: sim.state.powertrain.drive_torques[0],
            fr_drive_torque: sim.state.powertrain.drive_torques[1],
            rl_drive_torque: sim.state.powertrain.drive_torques[2],
            rr_drive_torque: sim.state.powertrain.drive_torques[3],
            fl_normal_force: sim.state.suspension.wheels[0].total_normal_force,
            fr_normal_force: sim.state.suspension.wheels[1].total_normal_force,
            rl_normal_force: sim.state.suspension.wheels[2].total_normal_force,
            rr_normal_force: sim.state.suspension.wheels[3].total_normal_force,
            abs_active: sim.state.powertrain.abs_active.iter().any(|a| *a),
            tc_active: sim.state.powertrain.tc_active,
            tc_cut_ratio: sim.state.powertrain.tc_cut_ratio,
            aids_enabled_mask: sim.aids.to_bits(),
            fl_pressure_kpa: sim.state.tire_thermal.wheels[0].pressure_kpa_gauge,
            fr_pressure_kpa: sim.state.tire_thermal.wheels[1].pressure_kpa_gauge,
            rl_pressure_kpa: sim.state.tire_thermal.wheels[2].pressure_kpa_gauge,
            rr_pressure_kpa: sim.state.tire_thermal.wheels[3].pressure_kpa_gauge,
            fl_tread_inner_c: sim.state.tire_thermal.wheels[0].tread_inner_c,
            fr_tread_inner_c: sim.state.tire_thermal.wheels[1].tread_inner_c,
            rl_tread_inner_c: sim.state.tire_thermal.wheels[2].tread_inner_c,
            rr_tread_inner_c: sim.state.tire_thermal.wheels[3].tread_inner_c,
            fl_tread_center_c: sim.state.tire_thermal.wheels[0].tread_center_c,
            fr_tread_center_c: sim.state.tire_thermal.wheels[1].tread_center_c,
            rl_tread_center_c: sim.state.tire_thermal.wheels[2].tread_center_c,
            rr_tread_center_c: sim.state.tire_thermal.wheels[3].tread_center_c,
            fl_tread_outer_c: sim.state.tire_thermal.wheels[0].tread_outer_c,
            fr_tread_outer_c: sim.state.tire_thermal.wheels[1].tread_outer_c,
            rl_tread_outer_c: sim.state.tire_thermal.wheels[2].tread_outer_c,
            rr_tread_outer_c: sim.state.tire_thermal.wheels[3].tread_outer_c,
            fl_carcass_c: sim.state.tire_thermal.wheels[0].carcass_c,
            fr_carcass_c: sim.state.tire_thermal.wheels[1].carcass_c,
            rl_carcass_c: sim.state.tire_thermal.wheels[2].carcass_c,
            rr_carcass_c: sim.state.tire_thermal.wheels[3].carcass_c,
            fl_gas_c: sim.state.tire_thermal.wheels[0].gas_c,
            fr_gas_c: sim.state.tire_thermal.wheels[1].gas_c,
            rl_gas_c: sim.state.tire_thermal.wheels[2].gas_c,
            rr_gas_c: sim.state.tire_thermal.wheels[3].gas_c,
            fl_brake_disc_c: sim.state.brake_thermal.wheels[0].disc_c,
            fr_brake_disc_c: sim.state.brake_thermal.wheels[1].disc_c,
            rl_brake_disc_c: sim.state.brake_thermal.wheels[2].disc_c,
            rr_brake_disc_c: sim.state.brake_thermal.wheels[3].disc_c,
            fl_brake_rim_c: sim.state.brake_thermal.wheels[0].rim_c,
            fr_brake_rim_c: sim.state.brake_thermal.wheels[1].rim_c,
            rl_brake_rim_c: sim.state.brake_thermal.wheels[2].rim_c,
            rr_brake_rim_c: sim.state.brake_thermal.wheels[3].rim_c,
            fl_brake_efficiency: sim.state.brake_thermal.wheels[0].efficiency,
            fr_brake_efficiency: sim.state.brake_thermal.wheels[1].efficiency,
            rl_brake_efficiency: sim.state.brake_thermal.wheels[2].efficiency,
            rr_brake_efficiency: sim.state.brake_thermal.wheels[3].efficiency,
            fl_duct_mass_flow_kg_s: sim.state.brake_thermal.wheels[0].duct.mass_flow_kg_s,
            fr_duct_mass_flow_kg_s: sim.state.brake_thermal.wheels[1].duct.mass_flow_kg_s,
            rl_duct_mass_flow_kg_s: sim.state.brake_thermal.wheels[2].duct.mass_flow_kg_s,
            rr_duct_mass_flow_kg_s: sim.state.brake_thermal.wheels[3].duct.mass_flow_kg_s,
            fl_duct_drag_n: sim.state.brake_thermal.wheels[0].duct.drag_force_n,
            fr_duct_drag_n: sim.state.brake_thermal.wheels[1].duct.drag_force_n,
            rl_duct_drag_n: sim.state.brake_thermal.wheels[2].duct.drag_force_n,
            rr_duct_drag_n: sim.state.brake_thermal.wheels[3].duct.drag_force_n,
            brake_optimal_min_c: sim.config.brake_thermal.optimal_min_temperature_c,
            brake_optimal_max_c: sim.config.brake_thermal.optimal_max_temperature_c,
            brake_fade_start_c: sim.config.brake_thermal.fade_start_temperature_c,
            brake_critical_c: sim.config.brake_thermal.critical_temperature_c,
            fl_brake_torque_nm: sim.state.brake_thermal.wheels[0].applied_brake_torque_nm,
            fr_brake_torque_nm: sim.state.brake_thermal.wheels[1].applied_brake_torque_nm,
            rl_brake_torque_nm: sim.state.brake_thermal.wheels[2].applied_brake_torque_nm,
            rr_brake_torque_nm: sim.state.brake_thermal.wheels[3].applied_brake_torque_nm,
            fl_brake_spin_pre_rad_s: sim.state.brake_thermal.wheels[0].wheel_spin_pre_rad_s,
            fr_brake_spin_pre_rad_s: sim.state.brake_thermal.wheels[1].wheel_spin_pre_rad_s,
            rl_brake_spin_pre_rad_s: sim.state.brake_thermal.wheels[2].wheel_spin_pre_rad_s,
            rr_brake_spin_pre_rad_s: sim.state.brake_thermal.wheels[3].wheel_spin_pre_rad_s,
            fl_brake_spin_post_rad_s: sim.state.brake_thermal.wheels[0].wheel_spin_post_rad_s,
            fr_brake_spin_post_rad_s: sim.state.brake_thermal.wheels[1].wheel_spin_post_rad_s,
            rl_brake_spin_post_rad_s: sim.state.brake_thermal.wheels[2].wheel_spin_post_rad_s,
            rr_brake_spin_post_rad_s: sim.state.brake_thermal.wheels[3].wheel_spin_post_rad_s,
            fl_brake_power_w: sim.state.brake_thermal.wheels[0].brake_power_w,
            fr_brake_power_w: sim.state.brake_thermal.wheels[1].brake_power_w,
            rl_brake_power_w: sim.state.brake_thermal.wheels[2].brake_power_w,
            rr_brake_power_w: sim.state.brake_thermal.wheels[3].brake_power_w,
            fl_brake_energy_j: sim.state.brake_thermal.wheels[0].brake_energy_j,
            fr_brake_energy_j: sim.state.brake_thermal.wheels[1].brake_energy_j,
            rl_brake_energy_j: sim.state.brake_thermal.wheels[2].brake_energy_j,
            rr_brake_energy_j: sim.state.brake_thermal.wheels[3].brake_energy_j,
            fl_brake_natural_cooling_w_k: sim.state.brake_thermal.wheels[0].natural_cooling_w_k,
            fr_brake_natural_cooling_w_k: sim.state.brake_thermal.wheels[1].natural_cooling_w_k,
            rl_brake_natural_cooling_w_k: sim.state.brake_thermal.wheels[2].natural_cooling_w_k,
            rr_brake_natural_cooling_w_k: sim.state.brake_thermal.wheels[3].natural_cooling_w_k,
            fl_brake_speed_cooling_w_k: sim.state.brake_thermal.wheels[0].speed_cooling_w_k,
            fr_brake_speed_cooling_w_k: sim.state.brake_thermal.wheels[1].speed_cooling_w_k,
            rl_brake_speed_cooling_w_k: sim.state.brake_thermal.wheels[2].speed_cooling_w_k,
            rr_brake_speed_cooling_w_k: sim.state.brake_thermal.wheels[3].speed_cooling_w_k,
            tc_eligible: sim.state.powertrain.tc_eligible,
            tc_gear_authority: sim.state.powertrain.tc_gear_authority,
            tc_slip_target: sim.state.powertrain.tc_slip_target,
            tc_raw_cut_ratio: sim.state.powertrain.tc_raw_cut_ratio,
            tc_slip_ratio: sim.state.powertrain.tc_slip_ratio,
            drive_torque_pre_tc_nm: sim.state.powertrain.drive_torques_pre_tc,
            pre_tc_drive_power_w: sim
                .state
                .powertrain
                .drive_torques_pre_tc
                .iter()
                .zip(sim.state.tires.wheels.iter())
                .map(|(torque, wheel)| torque * wheel.spin)
                .sum(),
            net_drive_power_w: sim
                .state
                .powertrain
                .drive_torques
                .iter()
                .zip(sim.state.tires.wheels.iter())
                .map(|(torque, wheel)| torque * wheel.spin)
                .sum(),
        };
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    /// Locks the FfiTelemetryOutput padding so the C mirror in
    /// native/include/formula90s/vehicle/formula90_physics.h can never drift
    /// silently. If you change the struct, update BOTH this test and the C header.
    #[test]
    fn ffi_telemetry_output_layout_locked() {
        assert_eq!(offset_of!(FfiTelemetryOutput, sim_time), 0);
        assert_eq!(offset_of!(FfiTelemetryOutput, gear), 24);
        assert_eq!(offset_of!(FfiTelemetryOutput, vert_g), 192);
        assert_eq!(offset_of!(FfiTelemetryOutput, steer_angle_rad), 296);
        assert_eq!(offset_of!(FfiTelemetryOutput, rr_normal_force), 368);
        assert_eq!(offset_of!(FfiTelemetryOutput, aids_enabled_mask), 392);
        // Tire pressure/thermal block is appended after the aids diagnostics.
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_pressure_kpa), 400);
        assert_eq!(offset_of!(FfiTelemetryOutput, rr_gas_c), 584);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_disc_c), 592);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_rim_c), 624);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_efficiency), 656);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_duct_mass_flow_kg_s), 688);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_duct_drag_n), 720);
        assert_eq!(offset_of!(FfiTelemetryOutput, brake_optimal_min_c), 752);
        assert_eq!(offset_of!(FfiTelemetryOutput, brake_optimal_max_c), 760);
        assert_eq!(offset_of!(FfiTelemetryOutput, brake_fade_start_c), 768);
        assert_eq!(offset_of!(FfiTelemetryOutput, brake_critical_c), 776);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_torque_nm), 784);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_spin_pre_rad_s), 816);
        assert_eq!(
            offset_of!(FfiTelemetryOutput, fl_brake_spin_post_rad_s),
            848
        );
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_power_w), 880);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_energy_j), 912);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_natural_cooling_w_k), 944);
        assert_eq!(offset_of!(FfiTelemetryOutput, fl_brake_speed_cooling_w_k), 976);
        assert_eq!(offset_of!(FfiTelemetryOutput, tc_eligible), 1008);
        assert_eq!(offset_of!(FfiTelemetryOutput, tc_gear_authority), 1016);
        assert_eq!(offset_of!(FfiTelemetryOutput, tc_slip_ratio), 1040);
        assert_eq!(offset_of!(FfiTelemetryOutput, drive_torque_pre_tc_nm), 1072);
        assert_eq!(offset_of!(FfiTelemetryOutput, pre_tc_drive_power_w), 1104);
        assert_eq!(offset_of!(FfiTelemetryOutput, net_drive_power_w), 1112);
        assert_eq!(size_of::<FfiTelemetryOutput>(), 1120);
    }
}
