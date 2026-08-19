//! C-ABI surface of the orchestrator — the SINGLE handshake Godot uses.
//!
//! Replaces the three per-module DLLs (vehicle_physics_engine, game_sim,
//! vehicle_audio_engine) with one artifact and one ABI version. Structs here are
//! mirrored EXACTLY in `native/include/formula90s/core/f90_core.h`.
//!
//! Extension rule: adding a module only adds additive `f90_core_module_<name>_*`
//! symbols — the core loop and this ABI are stable.

use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;

use serde::Deserialize;
use vehicle_physics_engine::{
    BodyKinematics, Quat, Transform3D, TriRaycastSample, Vec3, VehicleInput,
};

use crate::frame::AudioReadouts;
use crate::{CoreConfig, CoreFacade};

/// ABI v2: `f90_core_step` now carries the FULL aids mask (`aids_mask: u32`) instead
/// of a single `toggle_tc` pulse, and `f90_core_apply_runtime_config` was added.
pub const F90_CORE_ABI_VERSION: u32 = 2;

/// Reuses the mirrored `game_sim` tri-ray sample struct (already mirrored as
/// `F90SimTriRaycastSample` in `f90_sim_bridge.h`); here it is `F90TriRaycastSample`
/// in `f90_core.h`. One family of structs for the whole facade.
pub use game_sim::c_abi::CSimTriRaycastSample as F90TriRaycastSample;

/// Single output block of `f90_core_step`. Mirrored in `f90_core.h`.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct F90CoreFrameOut {
    // physics
    pub force_x: f64,
    pub force_y: f64,
    pub force_z: f64,
    pub torque_x: f64,
    pub torque_y: f64,
    pub torque_z: f64,
    // telemetry (compact)
    pub speed_kmh: f64,
    pub rpm: f64,
    pub gear: i32,
    pub steer: f64,
    pub throttle: f64,
    pub lat_g: f64,
    pub long_g: f64,
    pub vert_g: f64,
    pub fl_comp_mm: f64,
    pub fr_comp_mm: f64,
    pub rl_comp_mm: f64,
    pub rr_comp_mm: f64,
    pub front_slip: f64,
    pub rear_slip: f64,
    pub tc_active: f64,
    pub drive_torque: f64,
    // pose / velocity
    pub px: f64,
    pub py: f64,
    pub pz: f64,
    pub yaw: f64,
    pub lvx: f64,
    pub lvy: f64,
    pub lvz: f64,
    pub avx: f64,
    pub avy: f64,
    pub avz: f64,
    // audio presentation readouts
    pub surface_code: i32,
    pub active_bed_code: i32,
    pub trigger_code: i32,
    pub last_norm: f32,
    pub last_rpm: f64,
    pub last_throttle: f32,
    pub last_speed_kph: f64,
    pub last_slip: f32,
    pub last_engine_gain: f32,
    pub weights: [f32; 5],
    pub pitches: [f32; 5],
}

fn write_error(buf: *mut u8, len: u32, msg: &str) {
    if buf.is_null() || len == 0 {
        return;
    }
    let bytes = msg.as_bytes();
    let copy = bytes.len().min(len as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, copy);
        *buf.add(copy) = 0;
    }
}

#[no_mangle]
pub extern "C" fn f90_core_abi_version() -> u32 {
    F90_CORE_ABI_VERSION
}

static SHA: &[u8] = b"unknown\0";
#[no_mangle]
pub extern "C" fn f90_core_build_sha() -> *const c_char {
    SHA.as_ptr() as *const c_char
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Opts {
    bank_dir: Option<String>,
    config_json_path: Option<String>,
    use_canonical: bool,
    fixed_dt: Option<f64>,
    enable_audio: bool,
    idle_rpm: Option<f64>,
    max_rpm: Option<f64>,
    vehicle_scene: Option<String>,
    track_scene: Option<String>,
    modules: Vec<String>,
}

fn parse_opts(s: &str) -> Result<CoreConfig, String> {
    let o: Opts = serde_json::from_str(s).map_err(|e| format!("invalid opts JSON: {e}"))?;
    let mut cfg = CoreConfig::default();
    cfg.bank_dir = o.bank_dir.map(PathBuf::from);
    cfg.config_json_path = o.config_json_path.map(PathBuf::from);
    cfg.use_canonical = o.use_canonical || cfg.config_json_path.is_none();
    if let Some(d) = o.fixed_dt {
        cfg.fixed_dt = d;
    }
    cfg.enable_audio = o.enable_audio;
    if let Some(v) = o.idle_rpm {
        cfg.idle_rpm = v;
    }
    if let Some(v) = o.max_rpm {
        cfg.max_rpm = v;
    }
    if let Some(v) = o.vehicle_scene {
        cfg.vehicle_scene = v;
    }
    if let Some(v) = o.track_scene {
        cfg.track_scene = v;
    }
    cfg.modules = o.modules;
    Ok(cfg)
}

/// Create the facade. `opts_json` is a JSON object (see `Opts`). Returns an opaque
/// handle, or null + error string on failure.
///
/// # Safety
/// `opts_json` must be a NUL-terminated string; `err_buf` (when non-null) must be
/// `err_len` bytes. The returned handle must be released with `f90_core_destroy`.
#[no_mangle]
pub unsafe extern "C" fn f90_core_create(
    opts_json: *const c_char,
    err_buf: *mut u8,
    err_len: u32,
) -> *mut c_void {
    if opts_json.is_null() {
        write_error(err_buf, err_len, "opts_json is null");
        return std::ptr::null_mut();
    }
    let s = match CStr::from_ptr(opts_json).to_str() {
        Ok(s) => s,
        Err(_) => {
            write_error(err_buf, err_len, "opts_json is not valid UTF-8");
            return std::ptr::null_mut();
        }
    };
    let cfg = match parse_opts(s) {
        Ok(c) => c,
        Err(e) => {
            write_error(err_buf, err_len, &e);
            return std::ptr::null_mut();
        }
    };
    match CoreFacade::new(cfg) {
        Ok(f) => Box::into_raw(Box::new(f)) as *mut c_void,
        Err(e) => {
            write_error(err_buf, err_len, &e.to_string());
            std::ptr::null_mut()
        }
    }
}

fn facade_mut(h: *mut c_void) -> &'static mut CoreFacade {
    debug_assert!(!h.is_null());
    unsafe { &mut *(h as *mut CoreFacade) }
}

fn facade_ref(h: *mut c_void) -> &'static CoreFacade {
    debug_assert!(!h.is_null());
    unsafe { &*(h as *const CoreFacade) }
}

/// Destroy a handle created by `f90_core_create`.
///
/// # Safety
/// `h` must be null or an unreleased handle from `f90_core_create`.
#[no_mangle]
pub unsafe extern "C" fn f90_core_destroy(h: *mut c_void) {
    if h.is_null() {
        return;
    }
    drop(Box::from_raw(h as *mut CoreFacade));
}

/// Spawn the primary entity (if not spawned yet) and return its id (0 on error).
#[no_mangle]
pub extern "C" fn f90_core_spawn(h: *mut c_void) -> u32 {
    match facade_mut(h).ensure_spawned() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("[f90_core] spawn failed: {e}");
            0
        }
    }
}

/// Reset every entity to a pose/yaw and reset all modules.
#[no_mangle]
pub extern "C" fn f90_core_reset(h: *mut c_void, x: f64, y: f64, z: f64, yaw: f64) {
    facade_mut(h).reset(x, y, z, yaw);
}

/// Apply a runtime-tunable config to an entity (mirror of `F90RuntimeConfig` /
/// `FfiRuntimeConfig`). Returns true on success. Allows the GDScript tuning panel
/// and the vehicle setters to write every tunable JSON parameter into the facade
/// even in `bridge_controlled` mode.
///
/// # Safety
/// `h` must be a valid facade handle; `config` must point to a valid config.
#[no_mangle]
pub unsafe extern "C" fn f90_core_apply_runtime_config(
    h: *mut c_void,
    id: u32,
    config: *const vehicle_physics_engine::FfiRuntimeConfig,
) -> bool {
    if config.is_null() {
        return false;
    }
    facade_mut(h).apply_runtime_config(id, &*config)
}

/// Orchestrated step: physics solve + module ticks + audio state update, writing
/// ONE frame block out. Gravity and rigid-body integration stay in Godot.
///
/// # Safety
/// `h` must be a valid facade handle; `samples` must point to 4 `F90TriRaycastSample`;
/// `out` (when non-null) must point to a writable `F90CoreFrameOut`.
#[no_mangle]
pub unsafe extern "C" fn f90_core_step(
    h: *mut c_void,
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
    aids_mask: u32,
    dt: f64,
    samples: *const F90TriRaycastSample,
    out: *mut F90CoreFrameOut,
) {
    if samples.is_null() {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts(samples, 4) };
    let mut rust_samples: [TriRaycastSample; 4] = [TriRaycastSample::default(); 4];
    for (i, s) in slice.iter().enumerate() {
        rust_samples[i] = TriRaycastSample {
            inner: from_c_hit(&s.inner),
            center: from_c_hit(&s.center),
            outer: from_c_hit(&s.outer),
        };
    }
    let body = BodyKinematics {
        transform: Transform3D::new(
            Vec3::new(x, y, z),
            Quat::new(qx, qy, qz, qw).normalized().to_mat3(),
        ),
        orientation: Quat::new(qx, qy, qz, qw).normalized(),
        linear_velocity: Vec3::new(lx, ly, lz),
        angular_velocity: Vec3::new(ax, ay, az),
    };
    let input = VehicleInput {
        throttle,
        steering: steer,
        brake,
        handbrake,
        clutch,
        gear_request: if gear_request == 0 {
            None
        } else {
            Some(gear_request)
        },
    };
    let frame = facade_mut(h).step(id, body, &input, aids_mask, &rust_samples, dt);
    if !out.is_null() {
        let a = frame.audio;
        unsafe {
            *out = F90CoreFrameOut {
                force_x: frame.force[0],
                force_y: frame.force[1],
                force_z: frame.force[2],
                torque_x: frame.torque[0],
                torque_y: frame.torque[1],
                torque_z: frame.torque[2],
                speed_kmh: frame.speed_kmh,
                rpm: frame.rpm,
                gear: frame.gear,
                steer: frame.steer,
                throttle: frame.throttle,
                lat_g: frame.lat_g,
                long_g: frame.long_g,
                vert_g: frame.vert_g,
                fl_comp_mm: frame.fl_comp_mm,
                fr_comp_mm: frame.fr_comp_mm,
                rl_comp_mm: frame.rl_comp_mm,
                rr_comp_mm: frame.rr_comp_mm,
                front_slip: frame.front_slip,
                rear_slip: frame.rear_slip,
                tc_active: if frame.tc_active { 1.0 } else { 0.0 },
                drive_torque: frame.drive_torque,
                px: frame.px,
                py: frame.py,
                pz: frame.pz,
                yaw: frame.yaw,
                lvx: frame.lvx,
                lvy: frame.lvy,
                lvz: frame.lvz,
                avx: frame.avx,
                avy: frame.avy,
                avz: frame.avz,
                surface_code: a.surface_code as i32,
                active_bed_code: a.active_bed_code as i32,
                trigger_code: a.trigger_code,
                last_norm: a.last_norm,
                last_rpm: a.last_rpm,
                last_throttle: a.last_throttle,
                last_speed_kph: a.last_speed_kph,
                last_slip: a.last_slip,
                last_engine_gain: a.last_engine_gain,
                weights: a.weights,
                pitches: a.pitches,
            };
        }
    }
}

/// Render `n` stereo audio frames (the mixer advances its state). Returns frames
/// written (0 when the mixer is unavailable).
///
/// # Safety
/// `h` must be a valid facade handle; `out_l`/`out_r` must each point to at least
/// `n` writable `f32`s.
#[no_mangle]
pub unsafe extern "C" fn f90_core_audio_render(
    h: *mut c_void,
    out_l: *mut f32,
    out_r: *mut f32,
    n: u32,
) -> u32 {
    if h.is_null() || out_l.is_null() || out_r.is_null() {
        return 0;
    }
    let n = n as usize;
    let l = std::slice::from_raw_parts_mut(out_l, n);
    let r = std::slice::from_raw_parts_mut(out_r, n);
    facade_mut(h).audio_render(l, r, n) as u32
}

/// Fire a named one-shot by legacy code (0..10). Returns true if consumed.
/// # Safety
/// `h` must be a valid facade handle.
#[no_mangle]
pub extern "C" fn f90_core_audio_trigger(h: *mut c_void, code: i32) -> bool {
    facade_mut(h).audio_trigger(code)
}

/// Refresh audio presentation readouts into `out` (only the audio tail fields of
/// `F90CoreFrameOut` are meaningful; e.g. after a render pass the weights update).
/// # Safety
/// `h` must be a valid facade handle; `out` must point to a writable struct.
#[no_mangle]
pub extern "C" fn f90_core_audio_readouts(h: *mut c_void, out: *mut F90CoreFrameOut) {
    if out.is_null() {
        return;
    }
    let a: AudioReadouts = facade_mut(h).audio_readouts();
    unsafe {
        let o = &mut *out;
        o.surface_code = a.surface_code as i32;
        o.active_bed_code = a.active_bed_code as i32;
        o.trigger_code = a.trigger_code;
        o.last_norm = a.last_norm;
        o.last_rpm = a.last_rpm;
        o.last_throttle = a.last_throttle;
        o.last_speed_kph = a.last_speed_kph;
        o.last_slip = a.last_slip;
        o.last_engine_gain = a.last_engine_gain;
        o.weights = a.weights;
        o.pitches = a.pitches;
    }
}

/// Serialize the orchestrated snapshot (bincode) into `out`. Returns bytes needed;
/// 0 on success with `out_len` set. If the buffer is too small, returns the needed
/// size and leaves the buffer untouched.
/// # Safety
/// `h` must be a valid facade handle; `out` (when `cap` > 0) must point to `cap`
/// writable bytes; `out_len` must be non-null.
#[no_mangle]
pub extern "C" fn f90_core_snapshot(
    h: *mut c_void,
    out: *mut u8,
    cap: u32,
    out_len: *mut u32,
) -> u32 {
    if out_len.is_null() {
        return 0;
    }
    let bytes = match facade_ref(h).facade_snapshot().to_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[f90_core] snapshot serialize failed: {e}");
            unsafe { *out_len = 0 };
            return 0;
        }
    };
    let needed = bytes.len() as u32;
    unsafe {
        *out_len = needed;
    }
    if cap >= needed && !out.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
        }
        return 0;
    }
    needed
}

fn surface_from_u32(v: u32) -> vehicle_physics_engine::SurfaceType {
    use vehicle_physics_engine::SurfaceType::*;
    match v {
        1 => Curb,
        2 => Dirt,
        3 => Grass,
        4 => Gravel,
        5 => Sand,
        6 => Wall,
        7 => Metal,
        _ => Road,
    }
}

fn from_c_hit(h: &game_sim::c_abi::CSimRaycastHit) -> vehicle_physics_engine::RaycastHit {
    vehicle_physics_engine::RaycastHit {
        is_colliding: h.is_colliding,
        distance: h.distance,
        point: Vec3::new(h.px, h.py, h.pz),
        normal: Vec3::new(h.nx, h.ny, h.nz),
        surface: surface_from_u32(h.surface),
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    /// Locks the `F90CoreFrameOut` padding so the C mirror in
    /// `native/include/formula90s/core/f90_core.h` (which static_asserts the same
    /// offsets) can never drift silently. If you change the struct, update BOTH
    /// this test and the C header.
    #[test]
    fn f90_core_frame_out_layout_locked() {
        assert_eq!(offset_of!(F90CoreFrameOut, force_x), 0);
        assert_eq!(offset_of!(F90CoreFrameOut, force_z), 16);
        assert_eq!(offset_of!(F90CoreFrameOut, torque_z), 40);
        assert_eq!(offset_of!(F90CoreFrameOut, speed_kmh), 48);
        assert_eq!(offset_of!(F90CoreFrameOut, rpm), 56);
        assert_eq!(offset_of!(F90CoreFrameOut, gear), 64);
        assert_eq!(offset_of!(F90CoreFrameOut, steer), 72);
        assert_eq!(offset_of!(F90CoreFrameOut, throttle), 80);
        assert_eq!(offset_of!(F90CoreFrameOut, lat_g), 88);
        assert_eq!(offset_of!(F90CoreFrameOut, drive_torque), 168);
        assert_eq!(offset_of!(F90CoreFrameOut, px), 176);
        assert_eq!(offset_of!(F90CoreFrameOut, yaw), 200);
        assert_eq!(offset_of!(F90CoreFrameOut, surface_code), 256);
        assert_eq!(offset_of!(F90CoreFrameOut, active_bed_code), 260);
        assert_eq!(offset_of!(F90CoreFrameOut, trigger_code), 264);
        assert_eq!(offset_of!(F90CoreFrameOut, last_norm), 268);
        assert_eq!(offset_of!(F90CoreFrameOut, last_rpm), 272);
        assert_eq!(offset_of!(F90CoreFrameOut, last_speed_kph), 288);
        assert_eq!(offset_of!(F90CoreFrameOut, weights), 304);
        assert_eq!(offset_of!(F90CoreFrameOut, pitches), 324);
        assert_eq!(size_of::<F90CoreFrameOut>(), 344);
    }

    /// A `F90TriRaycastSample` reuses the mirrored game_sim struct; its per-hit
    /// layout must match the C mirror too.
    #[test]
    fn tri_ray_sample_hit_layout_locked() {
        assert_eq!(size_of::<game_sim::c_abi::CSimRaycastHit>(), 72);
        assert_eq!(offset_of!(game_sim::c_abi::CSimRaycastHit, is_colliding), 0);
        assert_eq!(offset_of!(game_sim::c_abi::CSimRaycastHit, distance), 8);
        assert_eq!(size_of::<F90TriRaycastSample>(), 216);
    }
}
