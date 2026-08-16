//! C-ABI Foreign Function Interface (FFI) for Godot / GDExtension integration.
//!
//! Provides zero-allocation, thread-safe, deterministic C-callable symbols
//! for instancing, updating, stepping, and querying the 6-DOF F1-94 vehicle simulator.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::simulation::*;
use crate::types::*;
use crate::vehicle_config::*;
use std::ffi::c_void;

/// C-ABI representation of a single raycast hit.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiRaycastHit {
    pub is_colliding: bool,
    pub distance: f64,
    pub point_x: f64,
    pub point_y: f64,
    pub point_z: f64,
    pub normal_x: f64,
    pub normal_y: f64,
    pub normal_z: f64,
    pub surface_type: u32, // 0=Road, 1=Curb, 2=Dirt, 3=Grass, 4=Gravel, 5=Sand, 6=Wall, 7=Metal
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

/// C-ABI representation of a Tri-Raycast sample (Inner, Center, Outer).
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

/// C-ABI driver inputs for vehicle control.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FfiVehicleInput {
    pub throttle: f64,
    pub steering: f64,
    pub brake: f64,
    pub handbrake: f64,
    pub clutch: f64,
    pub gear_request: i32, // -2 = None, -1 = Reverse, 0 = Neutral, 1..6 = Forward gears
}

impl From<FfiVehicleInput> for VehicleInput {
    fn from(f: FfiVehicleInput) -> Self {
        let gear_request = if f.gear_request >= -1 {
            Some(f.gear_request as i8)
        } else {
            None
        };
        Self {
            throttle: f.throttle,
            steering: f.steering,
            brake: f.brake,
            handbrake: f.handbrake,
            clutch: f.clutch,
            gear_request,
        }
    }
}

/// C-ABI output snapshot containing full 6-DOF transform, telemetry, and wheel states.
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

    // Chassis 3D Pose
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub rot_quat_x: f64,
    pub rot_quat_y: f64,
    pub rot_quat_z: f64,
    pub rot_quat_w: f64,

    // Chassis 3D Motion
    pub lin_vel_x: f64,
    pub lin_vel_y: f64,
    pub lin_vel_z: f64,
    pub ang_vel_x: f64,
    pub ang_vel_y: f64,
    pub ang_vel_z: f64,
    pub lat_g: f64,
    pub long_g: f64,
    pub vert_g: f64,

    // Wheel Compressions (mm)
    pub fl_comp_mm: f64,
    pub fr_comp_mm: f64,
    pub rl_comp_mm: f64,
    pub rr_comp_mm: f64,

    // Wheel Spin Angular Velocities (rad/s)
    pub fl_spin: f64,
    pub fr_spin: f64,
    pub rl_spin: f64,
    pub rr_spin: f64,

    // Wheel Slip Ratios
    pub fl_slip: f64,
    pub fr_slip: f64,
    pub rl_slip: f64,
    pub rr_slip: f64,

    // Wheel Steer Angle (rad)
    pub steer_angle_rad: f64,
}

// ---------------------------------------------------------------------------
// C-ABI Exported Functions
// ---------------------------------------------------------------------------

/// Instantiates an F1-94 vehicle simulation instance with default configuration.
#[no_mangle]
pub extern "C" fn f1_94_physics_create_default() -> *mut c_void {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn_height = cfg.front_tire_radius + cfg.front_spring_length * (1.0 - cfg.front_resting_ratio);
    let sim = Box::new(VehicleSimulator::new(cfg, Vec3::new(0.0, spawn_height, 0.0), 0.0));
    Box::into_raw(sim) as *mut c_void
}

/// Instantiates an F1-94 vehicle simulation instance at an explicit world position and yaw.
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

/// Resets vehicle state to specified position and orientation.
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
    let sim = unsafe { &mut *(sim_ptr as *mut VehicleSimulator) };
    *sim = VehicleSimulator::new(sim.config.clone(), Vec3::new(pos_x, pos_y, pos_z), yaw_rad);
}

/// Advances the simulation by `dt` seconds given 4 tri-raycast wheel samples and driver inputs.
#[no_mangle]
pub extern "C" fn f1_94_physics_step(
    sim_ptr: *mut c_void,
    input_ptr: *const FfiVehicleInput,
    samples_ptr: *const FfiTriRaycastSample, // Array of 4 FfiTriRaycastSample [FL, FR, RL, RR]
    dt: f64,
    out_telem: *mut FfiTelemetryOutput,
) {
    if sim_ptr.is_null() || input_ptr.is_null() || samples_ptr.is_null() {
        return;
    }

    let sim = unsafe { &mut *(sim_ptr as *mut VehicleSimulator) };
    let ffi_input = unsafe { *input_ptr };
    let input = VehicleInput::from(ffi_input);

    let ffi_samples = unsafe { std::slice::from_raw_parts(samples_ptr, 4) };
    let samples = [
        TriRaycastSample::from(ffi_samples[0]),
        TriRaycastSample::from(ffi_samples[1]),
        TriRaycastSample::from(ffi_samples[2]),
        TriRaycastSample::from(ffi_samples[3]),
    ];

    let telem = sim.step(&input, &samples, dt);

    if !out_telem.is_null() {
        let q = sim.state.orientation;
        unsafe {
            *out_telem = FfiTelemetryOutput {
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
                vert_g: sim.state.linear_acceleration.y / 9.80665,

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

                steer_angle_rad: sim.state.steer_input_smoothed * sim.config.max_steering_angle,
            };
        }
    }
}

/// Retrieves the local anchor position for a wheel index (0=FL, 1=FR, 2=RL, 3=RR).
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
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let wheel = WheelIndex::ALL[(wheel_idx as usize).min(3)];
    let anchor = sim.config.wheel_anchor_local(wheel);
    unsafe {
        *out_x = anchor.x;
        *out_y = anchor.y;
        *out_z = anchor.z;
    }
}

/// Retrieves the transverse tri-raycast spacing offset for a wheel index.
#[no_mangle]
pub extern "C" fn f1_94_physics_get_tri_ray_span(sim_ptr: *const c_void, wheel_idx: u32) -> f64 {
    if sim_ptr.is_null() {
        return 0.12;
    }
    let sim = unsafe { &*(sim_ptr as *const VehicleSimulator) };
    let wheel = WheelIndex::ALL[(wheel_idx as usize).min(3)];
    let tire_w = if wheel.is_front() {
        sim.config.front_tire_width
    } else {
        sim.config.rear_tire_width
    };
    tire_w * sim.config.tri_ray_spacing_ratio
}

/// Destroys a vehicle simulator instance and deallocates memory.
#[no_mangle]
pub extern "C" fn f1_94_physics_destroy(sim_ptr: *mut c_void) {
    if !sim_ptr.is_null() {
        unsafe {
            drop(Box::from_raw(sim_ptr as *mut VehicleSimulator));
        }
    }
}
