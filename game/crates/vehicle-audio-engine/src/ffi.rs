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
pub const VEHICLE_AUDIO_ABI_VERSION: u32 = 3;

/// Plain POD telemetry packet shared with C++.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VehicleAudioTelemetryV3 {
    pub schema_version: u32,
    pub struct_size: u32,

    pub rpm: f64,
    pub idle_rpm: f64,
    pub max_rpm: f64,

    pub throttle: f32,
    pub normalized_engine_load: f32,
    pub normalized_engine_torque: f32,
    pub rpm_derivative: f32,
    pub throttle_derivative: f32,

    pub speed_kph: f64,
    pub slip: f32,

    pub gear: i32,
    pub torque_sign: i32,
    pub shift_phase: i32,

    pub clutch_engagement: f32,
    pub tc_cut_ratio: f32,
    pub rev_limiter_active: u32,
}

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

/// Feed telemetry packet. See `VehicleAudioEngine::set_telemetry`.
///
/// # Safety
/// `handle` must be a valid engine handle. `telemetry` must point to a valid
/// `VehicleAudioTelemetryV3` matching `VEHICLE_AUDIO_ABI_VERSION`. `surface` must be a
/// valid NUL-terminated C string (or null).
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_set_telemetry(
    handle: *mut c_void,
    telemetry: *const VehicleAudioTelemetryV3,
    surface: *const c_char,
) -> bool {
    if handle.is_null() || telemetry.is_null() {
        return false;
    }
    let telem = &*telemetry;
    if telem.schema_version != VEHICLE_AUDIO_ABI_VERSION {
        return false;
    }
    if telem.struct_size as usize != std::mem::size_of::<VehicleAudioTelemetryV3>() {
        return false;
    }
    let surface = if surface.is_null() {
        "asphalt"
    } else {
        CStr::from_ptr(surface).to_str().unwrap_or("asphalt")
    };
    let engine = &mut *(handle as *mut VehicleAudioEngine);
    engine.set_telemetry(telem, surface);
    true
}

/// Feed telemetry (legacy shim). See `VehicleAudioEngine::set_state`.
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

/// Set the camera-to-vehicle listener distance in metres.
///
/// # Safety
/// `handle` must be a valid engine handle.
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_set_listener_distance(handle: *mut c_void, distance_m: f32) {
    if handle.is_null() {
        return;
    }
    let engine = &mut *(handle as *mut VehicleAudioEngine);
    engine.set_listener_distance(distance_m);
}

/// Get the camera-to-vehicle listener distance in metres.
///
/// # Safety
/// `handle` must be a valid engine handle.
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_listener_distance(handle: *mut c_void) -> f32 {
    if handle.is_null() {
        return 0.0;
    }
    let engine = &*(handle as *mut VehicleAudioEngine);
    engine.listener_distance()
}

/// Set the traction-control cut ratio [0.0, 1.0] on the active synth.
///
/// # Safety
/// `handle` must be a valid engine handle.
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_set_tc_cut(handle: *mut c_void, cut_ratio: f32) {
    if handle.is_null() {
        return;
    }
    let engine = &mut *(handle as *mut VehicleAudioEngine);
    engine.set_tc_cut(cut_ratio);
}

/// Set the RPM-limiter hard gate enabled flag on the active synth.
///
/// # Safety
/// `handle` must be a valid engine handle.
#[no_mangle]
pub unsafe extern "C" fn vehicle_audio_set_limiter_flag(handle: *mut c_void, active: bool) {
    if handle.is_null() {
        return;
    }
    let engine = &mut *(handle as *mut VehicleAudioEngine);
    engine.set_limiter_flag(active);
}

/// Build SHA (or "unknown"). NUL-terminated.
#[no_mangle]
pub extern "C" fn vehicle_audio_build_sha() -> *const c_char {
    static SHA: &[u8] = concat!(env!("FORMULA90_BUILD_SHA"), "\0").as_bytes();
    SHA.as_ptr() as *const c_char
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn vehicle_audio_telemetry_v3_layout_locked() {
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, schema_version), 0);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, struct_size), 4);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, rpm), 8);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, idle_rpm), 16);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, max_rpm), 24);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, throttle), 32);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, normalized_engine_load), 36);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, normalized_engine_torque), 40);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, rpm_derivative), 44);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, throttle_derivative), 48);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, speed_kph), 56);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, slip), 64);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, gear), 68);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, torque_sign), 72);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, shift_phase), 76);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, clutch_engagement), 80);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, tc_cut_ratio), 84);
        assert_eq!(offset_of!(VehicleAudioTelemetryV3, rev_limiter_active), 88);
        assert_eq!(size_of::<VehicleAudioTelemetryV3>(), 96);
    }

    #[test]
    fn telemetry_v3_validation_and_forwarding() {
        let bank_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sounds/banks/v10_vehicle");
        let c_bank = std::ffi::CString::new(bank_path.to_str().unwrap()).unwrap();
        let handle = unsafe { vehicle_audio_create(c_bank.as_ptr()) };
        assert!(!handle.is_null());

        let mut telem = VehicleAudioTelemetryV3 {
            schema_version: VEHICLE_AUDIO_ABI_VERSION,
            struct_size: size_of::<VehicleAudioTelemetryV3>() as u32,
            rpm: 12000.0,
            idle_rpm: 4500.0,
            max_rpm: 15000.0,
            throttle: 0.85,
            normalized_engine_load: 0.75,
            normalized_engine_torque: 0.65,
            rpm_derivative: 1500.0,
            throttle_derivative: 2.0,
            speed_kph: 240.0,
            slip: 0.05,
            gear: 5,
            torque_sign: 1,
            shift_phase: 0,
            clutch_engagement: 1.0,
            tc_cut_ratio: 0.0,
            rev_limiter_active: 0,
        };

        // Rejects mismatched schema_version
        telem.schema_version = 999;
        assert!(!unsafe { vehicle_audio_set_telemetry(handle, &telem, std::ptr::null()) });
        telem.schema_version = VEHICLE_AUDIO_ABI_VERSION;

        // Rejects mismatched struct_size
        telem.struct_size = 48;
        assert!(!unsafe { vehicle_audio_set_telemetry(handle, &telem, std::ptr::null()) });
        telem.struct_size = size_of::<VehicleAudioTelemetryV3>() as u32;

        // Valid packet is accepted
        assert!(unsafe { vehicle_audio_set_telemetry(handle, &telem, std::ptr::null()) });

        let engine = unsafe { &*(handle as *const VehicleAudioEngine) };
        assert_eq!(engine.last_normalized_engine_load(), 0.75);
        assert_eq!(engine.last_normalized_engine_torque(), 0.65);
        assert_eq!(engine.last_torque_sign(), 1);
        assert_eq!(engine.last_rpm_derivative(), 1500.0);
        assert_eq!(engine.last_throttle_derivative(), 2.0);
        assert_eq!(engine.last_shift_phase(), 0);

        unsafe { vehicle_audio_destroy(handle) };
    }
}
