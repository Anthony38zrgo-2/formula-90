//! C ABI boundary for the vehicle audio engine.
//!
//! Mirrors the physics crate FFI (`game/crates/vehicle-physics-engine/src/ffi.rs`): a versioned
//! symbol so the C++ GDExtension can reject ABI mismatches, plus thin owned-pointer
//! wrappers around `VehicleAudioEngine`. All pointers crossing the boundary are
//! plain `void*`/C arrays; safety is documented per function.

use crate::mixer::VehicleAudioEngine;
use crate::state::Trigger;
use std::ffi::{c_char, c_void, CStr};
use std::path::Path;

/// ABI version. Bump on any signature/semantic change to the symbols below.
pub const VEHICLE_AUDIO_ABI_VERSION: u32 = 1;

/// Opaque handle = `Box<VehicleAudioEngine>` leaked via `Box::into_raw`.
pub struct AudioHandle;

fn error_null<T>(_e: String) -> *mut T {
    std::ptr::null_mut()
}

/// Create a mixer from a bank directory (UTF-8 path). Returns null on failure.
///
/// # Safety
/// `bank_dir` must be a valid NUL-terminated C string. The returned handle must
/// be freed with `vehicle_audio_destroy`.
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_create(bank_dir: *const c_char) -> *mut c_void {
    if bank_dir.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = match CStr::from_ptr(bank_dir).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let engine = match VehicleAudioEngine::new(Path::new(cstr)) {
        Ok(e) => e,
        Err(e) => return error_null(format!("load bank failed: {e:?}")),
    };
    Box::into_raw(Box::new(engine)) as *mut c_void
}

/// Destroy a handle created by `vehicle_audio_create`.
///
/// # Safety
/// `handle` must be null or a pointer returned by `vehicle_audio_create` and not
/// freed already.
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_destroy(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    drop(Box::from_raw(handle as *mut VehicleAudioEngine));
}

/// Feed telemetry. See `VehicleAudioEngine::set_state`.
///
/// # Safety
/// `handle` must be a valid engine handle. `surface` must be a valid NUL-terminated
/// C string (or null). If null, treated as "asphalt".
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_set_state(
    handle: *mut c_void,
    rpm: f64,
    idle_rpm: f64,
    max_rpm: f64,
    throttle: f32,
    speed_kph: f64,
    gear: i32,
    slip: f32,
    surface: *const c_char,
) {
    if handle.is_null() {
        return;
    }
    let surface = if surface.is_null() {
        "asphalt"
    } else {
        CStr::from_ptr(surface).to_str().unwrap_or("asphalt")
    };
    let engine = &mut *(handle as *mut VehicleAudioEngine);
    engine.set_state(
        rpm, idle_rpm, max_rpm, throttle, speed_kph, gear, slip, surface,
    );
}

/// Fire a one-shot by integer trigger code. Mapping:
/// 0 ShiftUp, 1 ShiftDown, 2 Backfire, 3..6 Hit1..4, 7 Barrier, 8 Cone, 9 Fire, 10 Scrape.
///
/// # Safety
/// `handle` must be a valid engine handle.
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_trigger(handle: *mut c_void, code: i32) {
    if handle.is_null() {
        return;
    }
    let trigger = match code {
        0 => Trigger::ShiftUp,
        1 => Trigger::ShiftDown,
        2 => Trigger::Backfire,
        3 => Trigger::Hit1,
        4 => Trigger::Hit2,
        5 => Trigger::Hit3,
        6 => Trigger::Hit4,
        7 => Trigger::Barrier,
        8 => Trigger::Cone,
        9 => Trigger::Fire,
        10 => Trigger::Scrape,
        _ => return,
    };
    let engine = &mut *(handle as *mut VehicleAudioEngine);
    engine.trigger(trigger);
}

/// Render `n` stereo frames. Returns the number of frames written (0 if null).
///
/// # Safety
/// `handle` must be a valid engine handle. `out_l`/`out_r` must each point to at
/// least `n` writable `f32`s.
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_render(
    handle: *mut c_void,
    out_l: *mut f32,
    out_r: *mut f32,
    n: u32,
) -> u32 {
    if handle.is_null() || out_l.is_null() || out_r.is_null() {
        return 0;
    }
    let n = n as usize;
    let l = std::slice::from_raw_parts_mut(out_l, n);
    let r = std::slice::from_raw_parts_mut(out_r, n);
    let engine = &mut *(handle as *mut VehicleAudioEngine);
    engine.render(l, r, n);
    n as u32
}

/// ABI version this build exports.
#[no_mangle]
pub extern "C" fn vehicle_audio_abi_version() -> u32 {
    VEHICLE_AUDIO_ABI_VERSION
}

/// Build SHA (or "unknown"). NUL-terminated.
#[no_mangle]
pub extern "C" fn vehicle_audio_build_sha() -> *const c_char {
    static SHA: &[u8] = concat!(env!("FORMULA90_BUILD_SHA"), "\0").as_bytes();
    SHA.as_ptr() as *const c_char
}
