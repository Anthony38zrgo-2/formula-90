//! C-ABI surface for the authoritative simulation core. This is the contract the
//! thin Godot (`godot-cpp`) bridge uses to drive the core without `gdext`. Keep it
//! minimal and `#[repr(C)]`-stable; the C side mirrors these structs in
//! `f90_sim_bridge.h`.
//!
//! Design notes:
//! - The world is owned as a `Box<World>` behind a void pointer.
//! - The core integrates the vehicle itself (standalone `step`); the bridge only
//!   reads the resulting pose/telemetry and reflects it in Godot. This is the
//!   snapshot-server model: ONE simulator, no divergence with the headless path.
//! - Flat-ground samples are used here for the bridge's vehicle; feeding real
//!   tri-ray samples is a later, in-engine-specific extension (same `World::step`).
#![allow(clippy::not_unsafe_ptr_arg_deref)] // Limite ABI C: los punteros llegan del bridge Godot y se validan en la entrada.
use std::boxed::Box;
use std::ffi::{c_char, CStr};
use std::os::raw::c_void;
use std::path::Path;

use vehicle_physics_engine::{
    default_spawn_height, BodyKinematics, Mat3, Quat, RaycastHit, SurfaceType, Transform3D,
    TriRaycastSample, Vec3, VehicleConfig,
};

use crate::input::DriverInput;
use crate::world::{yaw_from_transform, World};

/// Mirrored in C as `F90SimRaycastHit`. Must match `f90_sim_bridge.h`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CSimRaycastHit {
    pub is_colliding: bool,
    pub distance: f64,
    pub px: f64,
    pub py: f64,
    pub pz: f64,
    pub nx: f64,
    pub ny: f64,
    pub nz: f64,
    pub surface: u32,
}

/// Mirrored in C as `F90SimTriRaycastSample` (Inner, Center, Outer).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CSimTriRaycastSample {
    pub inner: CSimRaycastHit,
    pub center: CSimRaycastHit,
    pub outer: CSimRaycastHit,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CSimPose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
}

/// Telemetry block mirrored in C (`F90SimTelemetry`). Order must match.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CSimTelemetry {
    pub speed_kmh: f64,
    pub rpm: f64,
    pub gear: f64, // i8 expressed as f64 for C ergonomics
    pub steer: f64, // true steering amount
    pub lat_g: f64,
    pub long_g: f64,
    pub vert_g: f64,
    pub fl_comp_mm: f64,
    pub fr_comp_mm: f64,
    pub rl_comp_mm: f64,
    pub rr_comp_mm: f64,
    pub front_slip: f64,
    pub rear_slip: f64,
    pub tc_active: f64, // 0.0 / 1.0
    pub drive_torque: f64,
}

fn world_mut(ptr: *mut c_void) -> &'static mut World {
    debug_assert!(!ptr.is_null());
    // SAFETY: callers guarantee `ptr` is a live `Box<World>` handle created by `sim_world_create`.
    unsafe { &mut *(ptr as *mut World) }
}

fn world_ref(ptr: *mut c_void) -> &'static World {
    debug_assert!(!ptr.is_null());
    // SAFETY: callers guarantee `ptr` is a live `Box<World>` handle created by `sim_world_create`.
    unsafe { &*(ptr as *const World) }
}

#[no_mangle]
pub extern "C" fn sim_world_create(dt: f64) -> *mut c_void {
    let boxed: Box<World> = Box::new(World::new(dt));
    Box::into_raw(boxed) as *mut c_void
}

#[no_mangle]
pub extern "C" fn sim_world_destroy(world: *mut c_void) {
    if world.is_null() {
        return;
    }
    // SAFETY: `world` is non-null (checked above) and was returned by `sim_world_create`.
    unsafe {
        let _ = Box::from_raw(world as *mut World);
    }
}

/// Spawn a vehicle from a JSON config path. Returns the entity id (0 on error).
#[no_mangle]
pub extern "C" fn sim_world_spawn_from_json(world: *mut c_void, json_path: *const c_char) -> u32 {
    let w = world_mut(world);
    if json_path.is_null() {
        return 0;
    }
    // SAFETY: `json_path` is non-null (checked above) and, per the C ABI contract, points to a valid NUL-terminated C string.
    let cstr = unsafe { CStr::from_ptr(json_path) };
    let Ok(path_str) = cstr.to_str() else {
        return 0;
    };
    let cfg = match VehicleConfig::from_json_path(Path::new(path_str)) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let spawn = Transform3D {
        origin: Vec3::new(0.0, default_spawn_height(&cfg), 0.0),
        basis: Mat3::IDENTITY,
    };
    w.spawn_vehicle(
        cfg,
        spawn,
		"res://scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn".to_string(),
        "res://scenes/tracks/test_field/la_chutana_track.tscn".to_string(),
    )
}

/// Spawn using the built-in canonical F1-94 config (no file needed). For tests.
#[no_mangle]
pub extern "C" fn sim_world_spawn_canonical(world: *mut c_void) -> u32 {
	let cfg = VehicleConfig::f1_2026_2008_canonical();
    let spawn = Transform3D {
        origin: Vec3::new(0.0, default_spawn_height(&cfg), 0.0),
        basis: Mat3::IDENTITY,
    };
    world_mut(world).spawn_vehicle(
        cfg,
        spawn,
        "vs".to_string(),
        "ts".to_string(),
    )
}

#[no_mangle]
pub extern "C" fn sim_world_set_input(
    world: *mut c_void,
    id: u32,
    throttle: f64,
    brake: f64,
    steer: f64,
    handbrake: f64,
    clutch: f64,
    gear_request: i8,
    toggle_tc: bool,
) {
    let w = world_mut(world);
    if let Some(ent) = w.entities.iter_mut().find(|e| e.id == id) {
        let inp = DriverInput {
            throttle,
            brake,
            steer,
            handbrake,
            clutch,
            gear_request: if gear_request < -1 {
                None
            } else {
                Some(gear_request)
            },
            toggle_traction_control: toggle_tc,
            shift_up: false,
            shift_down: false,
            toggle_transmission: false,
        };
        ent.pending_input = Some(inp);
    }
}

/// Advance the world by one fixed step using each entity's pending input. The core
/// integrates the vehicle and produces the pose/telemetry the bridge reflects.
#[no_mangle]
pub extern "C" fn sim_world_step(world: *mut c_void) {
    world_mut(world).step_pending();
}

#[no_mangle]
pub extern "C" fn sim_world_pose(world: *mut c_void, id: u32, out: *mut CSimPose) {
    let w = world_ref(world);
    if out.is_null() {
        return;
    }
    if let Some(ent) = w.entities.iter().find(|e| e.id == id) {
        let tf = &ent.sim.state.transform;
        let pose = CSimPose {
            x: tf.origin.x,
            y: tf.origin.y,
            z: tf.origin.z,
            yaw: yaw_from_transform(tf),
        };
        // SAFETY: `out` is non-null (checked above) and points to a writable `CSimPose`.
        unsafe {
            *out = pose;
        }
    }
}

#[no_mangle]
pub extern "C" fn sim_world_telemetry(world: *mut c_void, id: u32, out: *mut CSimTelemetry) {
    let w = world_ref(world);
    if out.is_null() {
        return;
    }
    if let Some(t) = w.entity_telemetry(id) {
        let telem = CSimTelemetry {
            speed_kmh: t.speed_kmh,
            rpm: t.rpm,
            gear: t.gear as f64,
            steer: t.steering,
            lat_g: t.lat_g,
            long_g: t.long_g,
            vert_g: t.vert_g,
            fl_comp_mm: t.fl_comp_mm,
            fr_comp_mm: t.fr_comp_mm,
            rl_comp_mm: t.rl_comp_mm,
            rr_comp_mm: t.rr_comp_mm,
            front_slip: t.front_slip,
            rear_slip: t.rear_slip,
            tc_active: if t.tc_active { 1.0 } else { 0.0 },
            drive_torque: t.drive_torque,
        };
        // SAFETY: `out` is non-null (checked above) and points to a writable `CSimTelemetry`.
        unsafe {
            *out = telem;
        }
    }
}

/// Build tri-ray samples from the simulator's own transform (used by the bridge for
/// the in-engine vehicle once real raycasts are wired). Exposed for completeness;
/// the bridge may override with scene raycasts.
#[no_mangle]
pub extern "C" fn sim_world_flat_samples(
    world: *mut c_void,
    id: u32,
    out: *mut CSimTriRaycastSample,
) {
    let w = world_ref(world);
    if out.is_null() {
        return;
    }
    if let Some(ent) = w.entities.iter().find(|e| e.id == id) {
        let samples = crate::world::flat_ground_samples(&ent.sim);
        let mut buf = Vec::new();
        for s in &samples {
            buf.push(CSimTriRaycastSample {
                inner: to_c_hit(&s.inner),
                center: to_c_hit(&s.center),
                outer: to_c_hit(&s.outer),
            });
        }
        // SAFETY: `out` is non-null (checked above) and points to at least `buf.len()` writable
        // `CSimTriRaycastSample` slots (the caller passes an array of 4).
        unsafe {
            std::ptr::copy_nonoverlapping(buf.as_ptr(), out, buf.len());
        }
    }
}

fn to_c_hit(h: &RaycastHit) -> CSimRaycastHit {
    CSimRaycastHit {
        is_colliding: h.is_colliding,
        distance: h.distance,
        px: h.point.x,
        py: h.point.y,
        pz: h.point.z,
        nx: h.normal.x,
        ny: h.normal.y,
        nz: h.normal.z,
        surface: h.surface as u32,
    }
}

fn surface_from_u32(v: u32) -> SurfaceType {
    match v {
        1 => SurfaceType::Curb,
        2 => SurfaceType::Dirt,
        3 => SurfaceType::Grass,
        4 => SurfaceType::Gravel,
        5 => SurfaceType::Sand,
        6 => SurfaceType::Wall,
        7 => SurfaceType::Metal,
        _ => SurfaceType::Road,
    }
}

fn from_c_hit(h: &CSimRaycastHit) -> RaycastHit {
    RaycastHit {
        is_colliding: h.is_colliding,
        distance: h.distance,
        point: Vec3::new(h.px, h.py, h.pz),
        normal: Vec3::new(h.nx, h.ny, h.nz),
        surface: surface_from_u32(h.surface),
    }
}

/// Seed an entity's transform from the engine's resolved pose (velocity-drive loop).
#[no_mangle]
pub extern "C" fn sim_world_set_pose(
    world: *mut c_void,
    id: u32,
    x: f64,
    y: f64,
    z: f64,
    yaw: f64,
) {
    world_mut(world).set_pose(id, x, y, z, yaw);
}

/// Seed an entity's transform AND velocity from the engine's resolved body (stable
/// velocity-drive loop: the core's internal state matches the body each frame).
#[no_mangle]
pub extern "C" fn sim_world_set_pose_and_velocity(
    world: *mut c_void,
    id: u32,
    x: f64,
    y: f64,
    z: f64,
    yaw: f64,
    lx: f64,
    ly: f64,
    lz: f64,
    ax: f64,
    ay: f64,
    az: f64,
) {
    world_mut(world).set_pose_and_velocity(
        id,
        x,
        y,
        z,
        yaw,
        Vec3::new(lx, ly, lz),
        Vec3::new(ax, ay, az),
    );
}

/// Step a single entity with caller-supplied tri-ray samples (real Godot raycasts).
#[no_mangle]
pub extern "C" fn sim_world_step_with_samples(
    world: *mut c_void,
    id: u32,
    throttle: f64,
    brake: f64,
    steer: f64,
    handbrake: f64,
    clutch: f64,
    gear_request: i8,
    toggle_tc: bool,
    dt: f64,
    samples: *const CSimTriRaycastSample,
) {
    let w = world_mut(world);
    if samples.is_null() {
        return;
    }
    // SAFETY: `samples` is non-null (checked above) and points to 4 valid `CSimTriRaycastSample`.
    let slice = unsafe { std::slice::from_raw_parts(samples, 4) };
    let mut rust_samples: [TriRaycastSample; 4] = [TriRaycastSample::default(); 4];
    for (i, s) in slice.iter().enumerate() {
        rust_samples[i] = TriRaycastSample {
            inner: from_c_hit(&s.inner),
            center: from_c_hit(&s.center),
            outer: from_c_hit(&s.outer),
        };
    }
    let inp = DriverInput {
        throttle,
        brake,
        steer,
        handbrake,
        clutch,
        gear_request: if gear_request == 0 {
            None
        } else {
            Some(gear_request)
        },
        toggle_traction_control: toggle_tc,
        shift_up: false,
        shift_down: false,
        toggle_transmission: false,
    };
    w.step_with_samples(id, &inp, &rust_samples, dt);
}

/// Solve forces for one entity with caller-supplied body kinematics + tri-ray samples,
/// and return the world-space force/torque for Godot to integrate (the same proven
/// path the legacy `vehicle_physics_engine` DLL uses: the core computes forces, Godot
/// owns gravity + rigid-body integration + collisions). This avoids the core's internal
/// standalone pose integrator, which is unstable when seeded every frame.
#[no_mangle]
pub extern "C" fn sim_world_solve_external(
    world: *mut c_void,
    id: u32,
    x: f64,
    y: f64,
    z: f64,
    qx: f64,
    qy: f64,
    qz: f64,
    qw: f64,
    lx: f64,
    ly: f64,
    lz: f64,
    ax: f64,
    ay: f64,
    az: f64,
    throttle: f64,
    brake: f64,
    steer: f64,
    handbrake: f64,
    clutch: f64,
    gear_request: i8,
    toggle_tc: bool,
    dt: f64,
    samples: *const CSimTriRaycastSample,
    out_force: *mut f64,
    out_torque: *mut f64,
) {
    let w = world_mut(world);
    if samples.is_null() || out_force.is_null() || out_torque.is_null() {
        return;
    }
    // SAFETY: `samples` is non-null (checked above) and points to 4 valid `CSimTriRaycastSample`.
    let slice = unsafe { std::slice::from_raw_parts(samples, 4) };
    let mut rust_samples: [TriRaycastSample; 4] = [TriRaycastSample::default(); 4];
    for (i, s) in slice.iter().enumerate() {
        rust_samples[i] = TriRaycastSample {
            inner: from_c_hit(&s.inner),
            center: from_c_hit(&s.center),
            outer: from_c_hit(&s.outer),
        };
    }
    let inp = DriverInput {
        throttle,
        brake,
        steer,
        handbrake,
        clutch,
        gear_request: if gear_request == 0 {
            None
        } else {
            Some(gear_request)
        },
        toggle_traction_control: toggle_tc,
        shift_up: false,
        shift_down: false,
        toggle_transmission: false,
    };
    if let Some(ent) = w.entities.iter_mut().find(|e| e.id == id) {
        let orientation = Quat::new(qx, qy, qz, qw).normalized();
        let basis = orientation.to_mat3();
        let body = BodyKinematics {
            transform: Transform3D {
                origin: Vec3::new(x, y, z),
                basis,
            },
            orientation,
            linear_velocity: Vec3::new(lx, ly, lz),
            angular_velocity: Vec3::new(ax, ay, az),
        };
        let (forces, telem) = ent.sim.solve_external(body, &inp.to_vehicle_input(), &rust_samples, dt);
        ent.last = Some(telem);
        // SAFETY: `out_force`/`out_torque` are non-null (checked above) and point to at least
        // 3 writable `f64`s each.
        unsafe {
            let f = std::slice::from_raw_parts_mut(out_force, 3);
            let t = std::slice::from_raw_parts_mut(out_torque, 3);
            f[0] = forces.force_world.x;
            f[1] = forces.force_world.y;
            f[2] = forces.force_world.z;
            t[0] = forces.torque_world.x;
            t[1] = forces.torque_world.y;
            t[2] = forces.torque_world.z;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn c_abi_roundtrip() {
        let w = sim_world_create(1.0 / 120.0);
        assert!(!w.is_null());
        let path = CString::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
			"/../../data/vehicles/f1_2026_2008/f1_2026_2008_physics.json"
        ))
        .unwrap();
    let id = sim_world_spawn_from_json(w, path.as_ptr());
    assert!(id != 0, "spawn must succeed");
    // The f1_2026_2008 profile ships with TC disabled by default
    // (`traction_control_default_enabled: false`). Enable it once via the
    // edge-triggered toggle so the TC telemetry path is exercised end-to-end.
    let mut tc_seen = false;
    for step in 0..120 {
        sim_world_set_input(w, id, 1.0, 0.0, 0.0, 0.0, 0.0, 1, step == 0);
        sim_world_step(w);
        let mut t = CSimTelemetry::default();
        sim_world_telemetry(w, id, &mut t);
        tc_seen |= t.tc_active == 1.0;
    }
    let mut pose = CSimPose::default();
    sim_world_pose(w, id, &mut pose);
    // Flat ground, straight launch: must have moved forward (z), no large lateral.
    assert!(pose.z < -1.0, "vehicle should accelerate forward");
    assert!(pose.x.abs() < 0.5, "no significant lateral drift on straight launch");
    let mut tel = CSimTelemetry::default();
    sim_world_telemetry(w, id, &mut tel);
    assert!(tel.speed_kmh > 0.0, "telemetry speed > 0 after launch");
    assert!(tc_seen, "TC must engage at least once when enabled under full-throttle launch");
    sim_world_destroy(w);
    }
}
