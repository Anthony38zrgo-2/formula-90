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
//! tri-ray samples is a later, in-engine-specific extension (same `World::step`).
use std::boxed::Box;
use std::ffi::{c_char, CStr};
use std::os::raw::c_void;
use std::path::Path;

use vehicle_physics_engine::{
    default_spawn_height, Mat3, RaycastHit, Transform3D, Vec3, VehicleConfig,
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
    unsafe { &mut *(ptr as *mut World) }
}

fn world_ref(ptr: *mut c_void) -> &'static World {
    debug_assert!(!ptr.is_null());
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
        "res://scenes/vehicles/f1_94/f1_94_rust.tscn".to_string(),
        "res://scenes/tracks/test_field/la_chutana_track.tscn".to_string(),
    )
}

/// Spawn using the built-in canonical F1-94 config (no file needed). For tests.
#[no_mangle]
pub extern "C" fn sim_world_spawn_canonical(world: *mut c_void) -> u32 {
    let cfg = VehicleConfig::f1_94_canonical();
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
            "/../data/vehicles/f1_94/f1_94_physics.json"
        ))
        .unwrap();
        let id = sim_world_spawn_from_json(w, path.as_ptr());
        assert!(id != 0, "spawn must succeed");
        for _ in 0..120 {
            sim_world_set_input(w, id, 1.0, 0.0, 0.0, 0.0, 0.0, 0, false);
            sim_world_step(w);
        }
        let mut pose = CSimPose::default();
        sim_world_pose(w, id, &mut pose);
        // Flat ground, straight launch: must have moved forward (z), no large lateral.
        assert!(pose.z < -1.0, "vehicle should accelerate forward");
        assert!(pose.x.abs() < 0.5, "no significant lateral drift on straight launch");
        let mut tel = CSimTelemetry::default();
        sim_world_telemetry(w, id, &mut tel);
        assert!(tel.speed_kmh > 0.0, "telemetry speed > 0 after launch");
        assert!(tel.tc_active == 1.0, "TC active during launch");
        sim_world_destroy(w);
    }
}
