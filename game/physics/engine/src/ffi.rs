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
use std::ffi::c_void;

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
            gear_request: if f.gear_request >= -1 { Some(f.gear_request as i8) } else { None },
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
}

#[no_mangle]
pub extern "C" fn f1_94_physics_create_default() -> *mut c_void {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn_height = default_spawn_height(&cfg);
    let sim = Box::new(VehicleSimulator::new(cfg, Vec3::new(0.0, spawn_height, 0.0), 0.0));
    Box::into_raw(sim) as *mut c_void
}

#[no_mangle]
pub extern "C" fn f1_94_physics_create_with_pos(
    pos_x: f64,
    pos_y: f64,
    pos_z: f64,
    yaw_rad: f64,
) -> *mut c_void {
    let cfg = VehicleConfig::f1_94_canonical();
    let sim = Box::new(VehicleSimulator::new(cfg, Vec3::new(pos_x, pos_y, pos_z), yaw_rad));
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
    if sim_ptr.is_null() { return; }
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

    let sim = unsafe { &mut *(sim_ptr as *mut VehicleSimulator) };
    let body = BodyKinematics::from(unsafe { *body_ptr });
    let input = VehicleInput::from(unsafe { *input_ptr });
    let samples = read_samples(samples_ptr);
    let (forces, telem) = sim.solve_external(body, &input, &samples, dt);

    if !out_forces.is_null() {
        unsafe { *out_forces = forces.into(); }
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
    if sim_ptr.is_null() || input_ptr.is_null() || samples_ptr.is_null() { return; }
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
    if sim_ptr.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() { return; }
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let wheel = WheelIndex::ALL[(wheel_idx as usize).min(3)];
    let anchor = sim.config.wheel_anchor_local(wheel);
    unsafe {
        *out_x = anchor.x;
        *out_y = anchor.y;
        *out_z = anchor.z;
    }
}

/// Offset from center ray to inner/outer ray. Total sampled width is 2*span.
#[no_mangle]
pub extern "C" fn f1_94_physics_get_tri_ray_span(sim_ptr: *const c_void, wheel_idx: u32) -> f64 {
    if sim_ptr.is_null() { return 0.12; }
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let wheel = WheelIndex::ALL[(wheel_idx as usize).min(3)];
    let tire_w = if wheel.is_front() { sim.config.front_tire_width } else { sim.config.rear_tire_width };
    tire_w * sim.config.tri_ray_spacing_ratio
}

#[no_mangle]
pub extern "C" fn f1_94_physics_get_default_spawn_height(sim_ptr: *const c_void) -> f64 {
    if sim_ptr.is_null() {
        return default_spawn_height(&VehicleConfig::f1_94_canonical());
    }
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
    if sim_ptr.is_null() || out_x.is_null() || out_y.is_null() || out_z.is_null() { return; }
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let cg = center_of_mass_local(&sim.config);
    unsafe {
        *out_x = cg.x;
        *out_y = cg.y;
        *out_z = cg.z;
    }
}

#[no_mangle]
pub extern "C" fn f1_94_physics_get_ray_length(sim_ptr: *const c_void, wheel_idx: u32) -> f64 {
    if sim_ptr.is_null() { return 0.6; }
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
    if sim_ptr.is_null() { return VehicleConfig::f1_94_canonical().vehicle_mass; }
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    sim.config.vehicle_mass
}

#[no_mangle]
pub extern "C" fn f1_94_physics_destroy(sim_ptr: *mut c_void) {
    if !sim_ptr.is_null() {
        unsafe { drop(Box::from_raw(sim_ptr as *mut VehicleSimulator)); }
    }
}

fn read_samples(samples_ptr: *const FfiTriRaycastSample) -> [TriRaycastSample; 4] {
    let ffi_samples = unsafe { std::slice::from_raw_parts(samples_ptr, 4) };
    [
        ffi_samples[0].into(),
        ffi_samples[1].into(),
        ffi_samples[2].into(),
        ffi_samples[3].into(),
    ]
}

fn write_telemetry(sim: &VehicleSimulator, telem: &crate::telemetry::TelemetryFrame, out: *mut FfiTelemetryOutput) {
    if out.is_null() { return; }
    let q = sim.state.orientation;
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
        };
    }
}
