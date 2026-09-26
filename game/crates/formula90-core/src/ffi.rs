//! C-ABI surface of the orchestrator — the SINGLE handshake Godot uses.
//!
//! Replaces the three per-module DLLs (vehicle_physics_engine, game_sim,
//! vehicle_audio_engine) with one artifact and one ABI version. Structs here are
//! mirrored EXACTLY in `native/include/formula90s/core/f90_core.h`.
//!
//! Extension rule: adding a module only adds additive `f90_core_module_<name>_*`
//! symbols — the core loop and this ABI are stable.
#![allow(clippy::not_unsafe_ptr_arg_deref)] // Limite ABI C: los punteros llegan del bridge Godot y se validan en la entrada.

use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;

use serde::Deserialize;
use vehicle_physics_engine::{
    BodyKinematics, Quat, Transform3D, TriRaycastSample, Vec3, VehicleInput,
};

use crate::audio_worker::{AudioWorker, AudioWorkerStats};
use crate::frame::AudioReadouts;
use crate::underfloor::{UnderfloorRayHit, UnderfloorRigidContact, UnderfloorSample};
use crate::{CoreConfig, CoreFacade};

/// ABI v5: brake energy diagnostics were appended after the brake thermal tail.
/// v13: added `f90_core_audio_set_ambient` (listener distance + TC/limiter downlink).
pub const F90_CORE_ABI_VERSION: u32 = 16;

/// Reuses the mirrored `game_sim` tri-ray sample struct (already mirrored as
/// `F90SimTriRaycastSample` in `f90_sim_bridge.h`); here it is `F90TriRaycastSample`
/// in `f90_core.h`. One family of structs for the whole facade.
pub use game_sim::c_abi::CSimTriRaycastSample as F90TriRaycastSample;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct F90UnderfloorRayHit {
    pub valid: f64,
    pub clearance_m: f64,
    pub point_x: f64,
    pub point_y: f64,
    pub point_z: f64,
    pub normal_x: f64,
    pub normal_y: f64,
    pub normal_z: f64,
    pub surface_code: f64,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct F90UnderfloorSample {
    pub rays: [F90UnderfloorRayHit; 5],
    pub rigid_confirmed: f64,
    pub rigid_local_x: f64,
    pub rigid_local_y: f64,
    pub rigid_local_z: f64,
    pub rigid_normal_impulse_ns: f64,
    pub rigid_tangential_speed_m_s: f64,
}

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
    // tire pressure + thermal (per wheel, WheelIndex order FL/FR/RL/RR)
    pub tire_pressure_kpa: [f64; 4],
    pub tire_tread_inner_c: [f64; 4],
    pub tire_tread_center_c: [f64; 4],
    pub tire_tread_outer_c: [f64; 4],
    pub tire_carcass_c: [f64; 4],
    pub tire_gas_c: [f64; 4],
    pub brake_disc_c: [f64; 4],
    pub brake_rim_c: [f64; 4],
    pub brake_efficiency: [f64; 4],
    pub duct_mass_flow_kg_s: [f64; 4],
    pub duct_drag_n: [f64; 4],
    pub total_brake_duct_drag_n: f64,
    pub brake_optimal_min_c: f64,
    pub brake_optimal_max_c: f64,
    pub brake_fade_start_c: f64,
    pub brake_critical_c: f64,
    pub brake_torque_nm: [f64; 4],
    pub brake_spin_pre_rad_s: [f64; 4],
    pub brake_spin_post_rad_s: [f64; 4],
    pub brake_power_w: [f64; 4],
    pub brake_energy_j: [f64; 4],
    // Lumped rotor cooling diagnostics (compact brake model).
    pub brake_natural_cooling_w_k: [f64; 4],
    pub brake_speed_cooling_w_k: [f64; 4],
    pub underfloor_clearance_m: [f64; 5],
    pub underfloor_valid_mask: u32,
    pub underfloor_scrape_phase: i32,
    pub underfloor_min_clearance_m: f64,
    pub underfloor_rake_rad: f64,
    pub underfloor_roll_rad: f64,
    pub underfloor_contact_confidence: f64,
    pub underfloor_scrape_intensity: f64,
    pub audio_scrape_gain: f32,
    pub audio_scrape_pitch: f32,
    pub audio_scrape_cursor: f64,
    pub underfloor_compression_m: [f64; 5],
    pub underfloor_closing_speed_m_s: [f64; 5],
    pub underfloor_normal_force_n: [f64; 5],
    pub underfloor_bottoming_phase: [i32; 5],
    pub underfloor_active_probe_mask: u32,
    pub underfloor_total_normal_force_n: f64,
    pub underfloor_max_probe_force_n: f64,
    pub underfloor_force_center_local: [f64; 3],
    pub underfloor_bottoming_torque: [f64; 3],
    pub underfloor_dissipated_energy_j: f64,
    pub underfloor_rigid_contact_blend: f64,
    pub aero_total_downforce_n: f64,
    pub aero_raw_downforce_n: f64,
    pub aero_front_downforce_n: f64,
    pub aero_floor_downforce_n: f64,
    pub aero_rear_downforce_n: f64,
    pub aero_drag_n: f64,
    pub aero_front_wing_angle_deg: f64,
    pub aero_rear_wing_angle_deg: f64,
    pub aero_front_wing_cl: f64,
    pub aero_rear_wing_cl: f64,
    pub aero_floor_height_factor: f64,
    pub aero_floor_rake_factor: f64,
    pub aero_floor_seal_factor: f64,
    pub aero_diffuser_stall_factor: f64,
    pub aero_global_limit_factor: f64,
    pub aero_load_ratio: f64,
    pub aero_balance_front: f64,
    // Append-only ABI 11 powertrain diagnostics.
    pub wheel_drive_torque_nm: [f64; 4],
    pub tc_cut_ratio: f64,
    pub net_drive_power_w: f64,
    // Append-only ABI 12 expanded traction-control diagnostics.
    pub tc_enabled: f64,
    pub tc_eligible: f64,
    pub tc_gear_authority: f64,
    pub tc_slip_target: f64,
    pub tc_raw_cut_ratio: f64,
    pub tc_slip_ratio: [f64; 4],
    pub wheel_drive_torque_pre_tc_nm: [f64; 4],
    pub pre_tc_drive_power_w: f64,
    // Append-only ABI 13 underfloor rigid-contact diagnostics.
    pub underfloor_rigid_local_y: f64,
    pub underfloor_rigid_normal_impulse_ns: f64,
    pub engine_block_temperature_celsius: f64,
    pub water_temperature_celsius: f64,
    pub oil_temperature_celsius: f64,
    pub engine_output_torque_newton_meters: f64,
    pub engine_mechanical_power_watts: f64,
    pub water_cooling_duct_opening: f64,
    pub oil_cooling_duct_opening: f64,
    pub water_cooling_mass_flow_kilograms_per_second: f64,
    pub oil_cooling_mass_flow_kilograms_per_second: f64,
    pub water_cooling_drag_force_newtons: f64,
    pub oil_cooling_drag_force_newtons: f64,
    pub total_powertrain_cooling_drag_force_newtons: f64,
    pub generated_engine_heat_watts: f64,
    pub engine_to_water_heat_transfer_watts: f64,
    pub engine_to_oil_heat_transfer_watts: f64,
    pub water_rejected_heat_watts: f64,
    pub oil_rejected_heat_watts: f64,
    pub available_engine_torque_fraction: f64,
    pub water_optimal_minimum_temperature_celsius: f64,
    pub water_optimal_maximum_temperature_celsius: f64,
    pub water_hot_derating_temperature_celsius: f64,
    pub water_critical_temperature_celsius: f64,
    pub oil_optimal_minimum_temperature_celsius: f64,
    pub oil_optimal_maximum_temperature_celsius: f64,
    pub oil_hot_derating_temperature_celsius: f64,
    pub oil_critical_temperature_celsius: f64,
    // Append-only ABI 15 onboard fuel state.
    pub fuel_remaining_kg: f64,
    pub fuel_capacity_kg: f64,
    pub total_vehicle_mass_kg: f64,
    pub effective_front_weight_distribution: f64,
    pub tire_wear_inner_fraction: [f64; 4],
    pub tire_wear_center_fraction: [f64; 4],
    pub tire_wear_outer_fraction: [f64; 4],
    pub tire_wear_remaining_fraction: [f64; 4],
    pub tire_wear_grip_scale: [f64; 4],
}

fn write_error(buf: *mut u8, len: u32, msg: &str) {
    if buf.is_null() || len == 0 {
        return;
    }
    let bytes = msg.as_bytes();
    let copy = bytes.len().min(len as usize - 1);
    // SAFETY: `buf` is non-null and `len > 0` (checked above); `copy` is clamped to `len - 1`,
    // so the copy and the NUL terminator stay in bounds.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, copy);
        *buf.add(copy) = 0;
    }
}

#[no_mangle]
pub extern "C" fn f90_core_abi_version() -> u32 {
    F90_CORE_ABI_VERSION
}

#[no_mangle]
pub extern "C" fn f90_core_build_sha() -> *const c_char {
    static SHA: &[u8] = concat!(env!("FORMULA90_BUILD_SHA"), "\0").as_bytes();
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
    underfloor_contact: Option<crate::underfloor::UnderfloorContactConfig>,
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
    if let Some(v) = o.underfloor_contact {
        cfg.underfloor_contact = v;
    }
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
    // SAFETY: `opts_json` is non-null (checked above) and, per `# Safety`, points to a valid NUL-terminated string.
    let s = match unsafe { CStr::from_ptr(opts_json) }.to_str() {
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
    // SAFETY: callers guarantee `h` is a live `Box<CoreFacade>` handle from `f90_core_create`.
    unsafe { &mut *(h as *mut CoreFacade) }
}

fn facade_ref(h: *mut c_void) -> &'static CoreFacade {
    debug_assert!(!h.is_null());
    // SAFETY: callers guarantee `h` is a live `Box<CoreFacade>` handle from `f90_core_create`.
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
    // SAFETY: `h` is non-null (checked above) and, per `# Safety`, is an unreleased
    // handle from `f90_core_create`.
    drop(unsafe { Box::from_raw(h as *mut CoreFacade) });
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
    // SAFETY: `config` is non-null (checked above) and, per `# Safety`, points to a valid config.
    facade_mut(h).apply_runtime_config(id, unsafe { &*config })
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
    underfloor: *const F90UnderfloorSample,
    out: *mut F90CoreFrameOut,
) {
    if samples.is_null() {
        return;
    }
    // SAFETY: `samples` is non-null (checked above) and points to 4 valid `F90TriRaycastSample`.
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
        gear_request: gear_request_from_abi(gear_request),
    };
    let rust_underfloor = if underfloor.is_null() {
        UnderfloorSample::default()
    } else {
        // SAFETY: `underfloor` is non-null (checked above) and points to a valid `F90UnderfloorSample`.
        let src = unsafe { &*underfloor };
        let mut result = UnderfloorSample::default();
        for (i, ray) in src.rays.iter().enumerate() {
            result.rays[i] = UnderfloorRayHit {
                valid: ray.valid > 0.5,
                clearance_m: ray.clearance_m,
                point_world: [ray.point_x, ray.point_y, ray.point_z],
                normal_world: [ray.normal_x, ray.normal_y, ray.normal_z],
                surface_code: ray.surface_code.clamp(0.0, 255.0) as u8,
            };
        }
        result.rigid_contact = UnderfloorRigidContact {
            confirmed: src.rigid_confirmed > 0.5,
            local_position: [src.rigid_local_x, src.rigid_local_y, src.rigid_local_z],
            normal_impulse_ns: src.rigid_normal_impulse_ns,
            tangential_speed_m_s: src.rigid_tangential_speed_m_s,
        };
        result
    };
    let frame = facade_mut(h).step_with_underfloor(
        id,
        body,
        &input,
        aids_mask,
        &rust_samples,
        &rust_underfloor,
        dt,
    );
    if !out.is_null() {
        let a = frame.audio;
        // SAFETY: `out` is non-null (checked above) and points to a writable `F90CoreFrameOut`.
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
                tire_pressure_kpa: frame.tire_pressure_kpa,
                tire_tread_inner_c: frame.tire_tread_inner_c,
                tire_tread_center_c: frame.tire_tread_center_c,
                tire_tread_outer_c: frame.tire_tread_outer_c,
                tire_carcass_c: frame.tire_carcass_c,
                tire_gas_c: frame.tire_gas_c,
                brake_disc_c: frame.brake_disc_c,
                brake_rim_c: frame.brake_rim_c,
                brake_efficiency: frame.brake_efficiency,
                duct_mass_flow_kg_s: frame.duct_mass_flow_kg_s,
                duct_drag_n: frame.duct_drag_n,
                total_brake_duct_drag_n: frame.total_brake_duct_drag_n,
                brake_optimal_min_c: frame.brake_optimal_min_c,
                brake_optimal_max_c: frame.brake_optimal_max_c,
                brake_fade_start_c: frame.brake_fade_start_c,
                brake_critical_c: frame.brake_critical_c,
                brake_torque_nm: frame.brake_torque_nm,
                brake_spin_pre_rad_s: frame.brake_spin_pre_rad_s,
                brake_spin_post_rad_s: frame.brake_spin_post_rad_s,
                brake_power_w: frame.brake_power_w,
                brake_energy_j: frame.brake_energy_j,
                brake_natural_cooling_w_k: frame.brake_natural_cooling_w_k,
                brake_speed_cooling_w_k: frame.brake_speed_cooling_w_k,
                underfloor_clearance_m: frame.underfloor_clearance_m,
                underfloor_valid_mask: frame.underfloor_valid_mask,
                underfloor_scrape_phase: frame.underfloor_scrape_phase,
                underfloor_min_clearance_m: frame.underfloor_min_clearance_m,
                underfloor_rake_rad: frame.underfloor_rake_rad,
                underfloor_roll_rad: frame.underfloor_roll_rad,
                underfloor_contact_confidence: frame.underfloor_contact_confidence,
                underfloor_scrape_intensity: frame.underfloor_scrape_intensity,
                audio_scrape_gain: a.scrape_gain,
                audio_scrape_pitch: a.scrape_pitch,
                audio_scrape_cursor: a.scrape_cursor,
                underfloor_compression_m: frame.underfloor_compression_m,
                underfloor_closing_speed_m_s: frame.underfloor_closing_speed_m_s,
                underfloor_normal_force_n: frame.underfloor_normal_force_n,
                underfloor_bottoming_phase: frame.underfloor_bottoming_phase,
                underfloor_active_probe_mask: frame.underfloor_active_probe_mask,
                underfloor_total_normal_force_n: frame.underfloor_total_normal_force_n,
                underfloor_max_probe_force_n: frame.underfloor_max_probe_force_n,
                underfloor_force_center_local: frame.underfloor_force_center_local,
                underfloor_bottoming_torque: frame.underfloor_bottoming_torque,
                underfloor_dissipated_energy_j: frame.underfloor_dissipated_energy_j,
                underfloor_rigid_contact_blend: frame.underfloor_rigid_contact_blend,
                aero_total_downforce_n: frame.aero_total_downforce_n,
                aero_raw_downforce_n: frame.aero_raw_downforce_n,
                aero_front_downforce_n: frame.aero_front_downforce_n,
                aero_floor_downforce_n: frame.aero_floor_downforce_n,
                aero_rear_downforce_n: frame.aero_rear_downforce_n,
                aero_drag_n: frame.aero_drag_n,
                aero_front_wing_angle_deg: frame.aero_front_wing_angle_deg,
                aero_rear_wing_angle_deg: frame.aero_rear_wing_angle_deg,
                aero_front_wing_cl: frame.aero_front_wing_cl,
                aero_rear_wing_cl: frame.aero_rear_wing_cl,
                aero_floor_height_factor: frame.aero_floor_height_factor,
                aero_floor_rake_factor: frame.aero_floor_rake_factor,
                aero_floor_seal_factor: frame.aero_floor_seal_factor,
                aero_diffuser_stall_factor: frame.aero_diffuser_stall_factor,
                aero_global_limit_factor: frame.aero_global_limit_factor,
                aero_load_ratio: frame.aero_load_ratio,
                aero_balance_front: frame.aero_balance_front,
                wheel_drive_torque_nm: frame.wheel_drive_torque_nm,
                tc_cut_ratio: frame.tc_cut_ratio,
                net_drive_power_w: frame.net_drive_power_w,
                tc_enabled: if frame.tc_enabled { 1.0 } else { 0.0 },
                tc_eligible: if frame.tc_eligible { 1.0 } else { 0.0 },
                tc_gear_authority: frame.tc_gear_authority,
                tc_slip_target: frame.tc_slip_target,
                tc_raw_cut_ratio: frame.tc_raw_cut_ratio,
                tc_slip_ratio: frame.tc_slip_ratio,
                wheel_drive_torque_pre_tc_nm: frame.wheel_drive_torque_pre_tc_nm,
                pre_tc_drive_power_w: frame.pre_tc_drive_power_w,
                underfloor_rigid_local_y: frame.underfloor_rigid_local_y,
                underfloor_rigid_normal_impulse_ns: frame.underfloor_rigid_normal_impulse_ns,
                engine_block_temperature_celsius: frame.engine_block_temperature_celsius,
                water_temperature_celsius: frame.water_temperature_celsius,
                oil_temperature_celsius: frame.oil_temperature_celsius,
                engine_output_torque_newton_meters: frame.engine_output_torque_newton_meters,
                engine_mechanical_power_watts: frame.engine_mechanical_power_watts,
                water_cooling_duct_opening: frame.water_cooling_duct_opening,
                oil_cooling_duct_opening: frame.oil_cooling_duct_opening,
                water_cooling_mass_flow_kilograms_per_second: frame
                    .water_cooling_mass_flow_kilograms_per_second,
                oil_cooling_mass_flow_kilograms_per_second: frame
                    .oil_cooling_mass_flow_kilograms_per_second,
                water_cooling_drag_force_newtons: frame.water_cooling_drag_force_newtons,
                oil_cooling_drag_force_newtons: frame.oil_cooling_drag_force_newtons,
                total_powertrain_cooling_drag_force_newtons: frame
                    .total_powertrain_cooling_drag_force_newtons,
                generated_engine_heat_watts: frame.generated_engine_heat_watts,
                engine_to_water_heat_transfer_watts: frame.engine_to_water_heat_transfer_watts,
                engine_to_oil_heat_transfer_watts: frame.engine_to_oil_heat_transfer_watts,
                water_rejected_heat_watts: frame.water_rejected_heat_watts,
                oil_rejected_heat_watts: frame.oil_rejected_heat_watts,
                available_engine_torque_fraction: frame.available_engine_torque_fraction,
                water_optimal_minimum_temperature_celsius: frame
                    .water_optimal_minimum_temperature_celsius,
                water_optimal_maximum_temperature_celsius: frame
                    .water_optimal_maximum_temperature_celsius,
                water_hot_derating_temperature_celsius: frame
                    .water_hot_derating_temperature_celsius,
                water_critical_temperature_celsius: frame.water_critical_temperature_celsius,
                oil_optimal_minimum_temperature_celsius: frame
                    .oil_optimal_minimum_temperature_celsius,
                oil_optimal_maximum_temperature_celsius: frame
                    .oil_optimal_maximum_temperature_celsius,
                oil_hot_derating_temperature_celsius: frame
                    .oil_hot_derating_temperature_celsius,
                oil_critical_temperature_celsius: frame.oil_critical_temperature_celsius,
                fuel_remaining_kg: frame.fuel_remaining_kg,
                fuel_capacity_kg: frame.fuel_capacity_kg,
                total_vehicle_mass_kg: frame.total_vehicle_mass_kg,
                effective_front_weight_distribution: frame.effective_front_weight_distribution,
                tire_wear_inner_fraction: frame.tire_wear_inner_fraction,
                tire_wear_center_fraction: frame.tire_wear_center_fraction,
                tire_wear_outer_fraction: frame.tire_wear_outer_fraction,
                tire_wear_remaining_fraction: frame.tire_wear_remaining_fraction,
                tire_wear_grip_scale: frame.tire_wear_grip_scale,
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
    // SAFETY: `out_l`/`out_r` are non-null (checked above) and, per `# Safety`, each
    // points to at least `n` writable `f32`s.
    let (l, r) = unsafe {
        (
            std::slice::from_raw_parts_mut(out_l, n),
            std::slice::from_raw_parts_mut(out_r, n),
        )
    };
    facade_mut(h).audio_render(l, r, n) as u32
}

/// Set the listener/ambient downlink: camera-to-vehicle distance (m), TC cut
/// ratio, and RPM-limiter enabled flag. Returns true when the audio mixer is
/// present and the telemetry was applied.
/// # Safety
/// `h` must be null or a valid facade handle.
#[no_mangle]
pub extern "C" fn f90_core_audio_set_ambient(
    h: *mut c_void,
    distance_m: f32,
    tc_cut_ratio: f32,
    limiter_active: bool,
) -> bool {
    if h.is_null() {
        return false;
    }
    facade_mut(h).audio_set_ambient(distance_m, tc_cut_ratio, limiter_active)
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
    // SAFETY: `out` is non-null (checked above) and points to a writable `F90CoreFrameOut`.
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

/// Active continuous audio source: 0 legacy, 1 GF509, 2 Grand Prix sampler,
/// -1 when the handle is invalid. Additive ABI 13 extension; layouts unchanged.
/// # Safety
/// `h` must be null or a valid facade handle.
#[no_mangle]
pub extern "C" fn f90_core_audio_source_code(h: *mut c_void) -> i32 {
    if h.is_null() {
        return -1;
    }
    facade_mut(h).audio_source_code()
}

/// Start the dedicated audio worker: the mixer moves off the render thread. The
/// host creates and pins the OS thread that calls `f90_core_audio_worker_run`.
/// Returns false when the mixer is unavailable or a worker already owns it (the
/// inline pump remains the fallback).
/// # Safety
/// `h` must be a valid facade handle.
#[no_mangle]
pub extern "C" fn f90_core_audio_worker_start(h: *mut c_void) -> bool {
    if h.is_null() {
        return false;
    }
    facade_mut(h).start_audio_worker()
}

/// Stable pointer to the started worker (null when the inline pump is in charge).
/// Valid while the facade lives; the host must join its thread before destroy.
/// # Safety
/// `h` must be a valid facade handle.
#[no_mangle]
pub extern "C" fn f90_core_audio_worker_handle(h: *mut c_void) -> *const c_void {
    if h.is_null() {
        return std::ptr::null();
    }
    facade_mut(h).audio_worker_ptr() as *const c_void
}

/// Blocking worker loop for the dedicated thread. `stop` is polled (non-zero
/// exits); the host owns its storage (a `std::atomic<uint32_t>`). Returns after
/// flushing pending control data.
/// # Safety
/// `handle` must come from `f90_core_audio_worker_handle` and stay valid for the
/// whole call; `stop` must point to a readable, host-written `u32`.
#[no_mangle]
pub unsafe extern "C" fn f90_core_audio_worker_run(handle: *const c_void, stop: *const u32) {
    if handle.is_null() || stop.is_null() {
        return;
    }
    // SAFETY: the host guarantees the handle outlives this blocking call (it joins
    // the thread before destroying the facade) and that `stop` is readable.
    let worker = unsafe { &*(handle as *const AudioWorker) };
    worker.run_until(|| unsafe { std::ptr::read_volatile(stop) } != 0);
}

/// Drain up to `n` stereo frames produced by the worker. Returns frames written
/// (0 when no worker is active or the ring is empty).
/// # Safety
/// `h` must be a valid facade handle; `out_l`/`out_r` must each point to at least
/// `n` writable `f32`s.
#[no_mangle]
pub unsafe extern "C" fn f90_core_audio_worker_pull(
    h: *mut c_void,
    out_l: *mut f32,
    out_r: *mut f32,
    n: u32,
) -> u32 {
    if h.is_null() || out_l.is_null() || out_r.is_null() {
        return 0;
    }
    let n = n as usize;
    // SAFETY: per `# Safety`, each pointer has room for `n` writable `f32`s.
    let (l, r) = unsafe {
        (
            std::slice::from_raw_parts_mut(out_l, n),
            std::slice::from_raw_parts_mut(out_r, n),
        )
    };
    facade_mut(h).audio_render(l, r, n) as u32
}

/// Set the worker's steady-state ring occupancy target (frames). The host calls
/// this every rendered frame with `ceil(rate * delta)` so the added latency
/// tracks the frame demand instead of a fixed 0.1 s backlog.
/// # Safety
/// `h` must be a valid facade handle.
#[no_mangle]
pub extern "C" fn f90_core_audio_worker_set_target(h: *mut c_void, frames: u32) {
    if h.is_null() {
        return;
    }
    if let Some(worker) = facade_mut(h).audio_worker() {
        worker.set_target(frames as usize);
    }
}

/// Copy the worker counters into `out`. Returns false when no worker is active.
/// # Safety
/// `h` must be a valid facade handle; `out` must point to a writable
/// `AudioWorkerStats`.
#[no_mangle]
pub unsafe extern "C" fn f90_core_audio_worker_stats(
    h: *mut c_void,
    out: *mut AudioWorkerStats,
) -> bool {
    if h.is_null() || out.is_null() {
        return false;
    }
    let facade = facade_mut(h);
    match facade.audio_worker() {
        Some(worker) => {
            // SAFETY: `out` is non-null (checked above) and writable per `# Safety`.
            unsafe { *out = worker.stats() };
            true
        }
        None => false,
    }
}

/// Serialize the orchestrated snapshot (postcard) into `out`. Returns bytes needed;
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
            // SAFETY: `out_len` is non-null (checked above) and points to a writable `u32`.
            unsafe { *out_len = 0 };
            return 0;
        }
    };
    let needed = bytes.len() as u32;
    // SAFETY: `out_len` is non-null (checked above) and points to a writable `u32`.
    unsafe {
        *out_len = needed;
    }
    if cap >= needed && !out.is_null() {
        // SAFETY: `out` is non-null and `cap >= needed == bytes.len()` (checked above), so the
        // destination has room for the whole snapshot.
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

/// Maps the `f90_core_step` gear sentinel to the solver contract documented in
/// `vehicle_physics_engine::ffi`: `-2` (or below) = no change, `-1` = Reverse,
/// `0` = Neutral, `1..=max` = forward gear. Treating `0` as "no change" made
/// Neutral unreachable, which blocked the 1 -> N -> R manual shift sequence.
fn gear_request_from_abi(v: i8) -> Option<i8> {
    if v < -1 {
        None
    } else {
        Some(v)
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
        // Tire pressure/thermal block appended after the audio tail (six [f64; 4]).
        assert_eq!(offset_of!(F90CoreFrameOut, tire_pressure_kpa), 344);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_tread_inner_c), 376);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_tread_center_c), 408);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_tread_outer_c), 440);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_carcass_c), 472);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_gas_c), 504);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_disc_c), 536);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_rim_c), 568);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_efficiency), 600);
        assert_eq!(offset_of!(F90CoreFrameOut, duct_mass_flow_kg_s), 632);
        assert_eq!(offset_of!(F90CoreFrameOut, duct_drag_n), 664);
        assert_eq!(offset_of!(F90CoreFrameOut, total_brake_duct_drag_n), 696);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_optimal_min_c), 704);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_optimal_max_c), 712);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_fade_start_c), 720);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_critical_c), 728);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_torque_nm), 736);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_spin_pre_rad_s), 768);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_spin_post_rad_s), 800);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_power_w), 832);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_energy_j), 864);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_natural_cooling_w_k), 896);
        assert_eq!(offset_of!(F90CoreFrameOut, brake_speed_cooling_w_k), 928);
        assert_eq!(offset_of!(F90CoreFrameOut, underfloor_clearance_m), 960);
        assert_eq!(offset_of!(F90CoreFrameOut, underfloor_valid_mask), 1000);
        assert_eq!(offset_of!(F90CoreFrameOut, underfloor_scrape_phase), 1004);
        assert_eq!(offset_of!(F90CoreFrameOut, audio_scrape_cursor), 1056);
        assert_eq!(offset_of!(F90CoreFrameOut, underfloor_compression_m), 1064);
        assert_eq!(
            offset_of!(F90CoreFrameOut, underfloor_bottoming_phase),
            1184
        );
        assert_eq!(
            offset_of!(F90CoreFrameOut, underfloor_active_probe_mask),
            1204
        );
        assert_eq!(
            offset_of!(F90CoreFrameOut, underfloor_total_normal_force_n),
            1208
        );
        assert_eq!(
            offset_of!(F90CoreFrameOut, underfloor_bottoming_torque),
            1248
        );
        assert_eq!(
            offset_of!(F90CoreFrameOut, underfloor_rigid_contact_blend),
            1280
        );
        assert_eq!(offset_of!(F90CoreFrameOut, aero_total_downforce_n), 1288);
        assert_eq!(offset_of!(F90CoreFrameOut, aero_balance_front), 1416);
        assert_eq!(offset_of!(F90CoreFrameOut, wheel_drive_torque_nm), 1424);
        assert_eq!(offset_of!(F90CoreFrameOut, tc_cut_ratio), 1456);
        assert_eq!(offset_of!(F90CoreFrameOut, net_drive_power_w), 1464);
        assert_eq!(offset_of!(F90CoreFrameOut, tc_enabled), 1472);
        assert_eq!(offset_of!(F90CoreFrameOut, tc_eligible), 1480);
        assert_eq!(offset_of!(F90CoreFrameOut, tc_gear_authority), 1488);
        assert_eq!(offset_of!(F90CoreFrameOut, tc_slip_ratio), 1512);
        assert_eq!(
            offset_of!(F90CoreFrameOut, wheel_drive_torque_pre_tc_nm),
            1544
        );
        assert_eq!(offset_of!(F90CoreFrameOut, pre_tc_drive_power_w), 1576);
        assert_eq!(offset_of!(F90CoreFrameOut, underfloor_rigid_local_y), 1584);
        assert_eq!(
            offset_of!(F90CoreFrameOut, underfloor_rigid_normal_impulse_ns),
            1592
        );
        assert_eq!(offset_of!(F90CoreFrameOut, engine_block_temperature_celsius), 1600);
        assert_eq!(offset_of!(F90CoreFrameOut, oil_critical_temperature_celsius), 1800);
        // FUEL-100 append-only ABI 15 block.
        assert_eq!(offset_of!(F90CoreFrameOut, fuel_remaining_kg), 1808);
        assert_eq!(offset_of!(F90CoreFrameOut, fuel_capacity_kg), 1816);
        assert_eq!(offset_of!(F90CoreFrameOut, total_vehicle_mass_kg), 1824);
        assert_eq!(
            offset_of!(F90CoreFrameOut, effective_front_weight_distribution),
            1832
        );
        assert_eq!(offset_of!(F90CoreFrameOut, tire_wear_inner_fraction), 1840);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_wear_center_fraction), 1872);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_wear_outer_fraction), 1904);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_wear_remaining_fraction), 1936);
        assert_eq!(offset_of!(F90CoreFrameOut, tire_wear_grip_scale), 1968);
        assert_eq!(size_of::<F90CoreFrameOut>(), 2000);
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

#[cfg(test)]
mod abi_tests {
    use super::*;
    use std::ffi::CString;

    fn flat_samples() -> [F90TriRaycastSample; 4] {
        let hit = game_sim::c_abi::CSimRaycastHit {
            is_colliding: true,
            distance: 0.3,
            nx: 0.0,
            ny: 1.0,
            nz: 0.0,
            surface: 0,
            ..Default::default()
        };
        [F90TriRaycastSample {
            inner: hit,
            center: hit,
            outer: hit,
        }; 4]
    }

    #[test]
    fn surface_from_u32_maps_every_code() {
        use vehicle_physics_engine::SurfaceType::*;
        assert_eq!(surface_from_u32(0), Road);
        assert_eq!(surface_from_u32(1), Curb);
        assert_eq!(surface_from_u32(2), Dirt);
        assert_eq!(surface_from_u32(3), Grass);
        assert_eq!(surface_from_u32(4), Gravel);
        assert_eq!(surface_from_u32(5), Sand);
        assert_eq!(surface_from_u32(6), Wall);
        assert_eq!(surface_from_u32(7), Metal);
        assert_eq!(surface_from_u32(99), Road);
    }

    #[test]
    fn parse_opts_applies_overrides_and_rejects_bad_json() {
        let defaults = parse_opts("{}").expect("empty opts");
        assert!(defaults.use_canonical);

        let cfg = parse_opts(
            r#"{"fixed_dt":0.01,"enable_audio":false,"idle_rpm":2000.0,"max_rpm":9000.0,"modules":["weather"],"vehicle_scene":"v","track_scene":"t"}"#,
        )
        .expect("valid opts");
        assert_eq!(cfg.fixed_dt, 0.01);
        assert_eq!(cfg.idle_rpm, 2000.0);
        assert_eq!(cfg.max_rpm, 9000.0);
        assert_eq!(cfg.modules, vec!["weather".to_string()]);
        assert_eq!(cfg.vehicle_scene, "v");
        assert_eq!(cfg.track_scene, "t");
        assert!(parse_opts("{ not json").is_err());
    }

    #[test]
    fn abi_lifecycle_covers_core_entrypoints() {
        let opts = CString::new(r#"{"use_canonical":true}"#).unwrap();
        let mut err = [0u8; 256];
        let h = unsafe { f90_core_create(opts.as_ptr(), err.as_mut_ptr(), err.len() as u32) };
        assert!(!h.is_null(), "create failed: {}", String::from_utf8_lossy(&err));

        // Null/bad inputs hit the early-return guards.
        assert!(unsafe { f90_core_create(std::ptr::null(), err.as_mut_ptr(), err.len() as u32) }.is_null());
        let h2 = unsafe { f90_core_create(opts.as_ptr(), std::ptr::null_mut(), 0) };
        assert!(!h2.is_null());
        unsafe { f90_core_destroy(h2) };

        let id = f90_core_spawn(h);
        assert_ne!(id, 0, "spawn must succeed");

        let samples = flat_samples();
        let mut out = F90CoreFrameOut::default();
        for _ in 0..4 {
            unsafe {
                f90_core_step(
                    h, id, 0.0, 0.3, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                    0.0, 0.0, 0.0, 0.0, 1, 0, 1.0 / 120.0, samples.as_ptr(), std::ptr::null_mut(),
                    &mut out,
                )
            };
        }
        assert!(out.speed_kmh.is_finite());

        // Snapshot: size query, too-small buffer, and full write.
        let mut len = 0u32;
        let needed = f90_core_snapshot(h, std::ptr::null_mut(), 0, &mut len);
        assert!(needed > 0);
        assert_eq!(needed, len);
        let mut buf = vec![0u8; needed as usize];
        assert_eq!(f90_core_snapshot(h, buf.as_mut_ptr(), needed, &mut len), 0);
        assert_eq!(f90_core_snapshot(h, std::ptr::null_mut(), 0, std::ptr::null_mut()), 0);

        // Audio downlink/readouts (audio disabled -> no-op but exercised).
        let mut aout = F90CoreFrameOut::default();
        f90_core_audio_readouts(h, &mut aout);
        let _ = f90_core_audio_set_ambient(h, 12.0, 0.0, false);
        let _ = f90_core_audio_trigger(h, 0);
        let mut l = vec![0f32; 64];
        let mut r = vec![0f32; 64];
        assert_eq!(
            unsafe { f90_core_audio_render(h, l.as_mut_ptr(), r.as_mut_ptr(), 64) },
            0
        );

        // Runtime config path (all-zero config is rejected, but the entrypoint runs).
        let cfg: vehicle_physics_engine::FfiRuntimeConfig = unsafe { std::mem::zeroed() };
        let _ = unsafe { f90_core_apply_runtime_config(h, id, &cfg) };
        assert!(!unsafe { f90_core_apply_runtime_config(h, id, std::ptr::null()) });

        f90_core_reset(h, 0.0, 0.3, 0.0, 0.0);
        unsafe { f90_core_destroy(h) };
        unsafe { f90_core_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn audio_worker_ffi_lifecycle_produces_pcm() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let bank = crate::audio_worker::packaged_bank_dir();
        if !bank.exists() {
            eprintln!("[ffi] bank missing; skipping worker lifecycle test");
            return;
        }
        let profile = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");
        let opts_json = serde_json::json!({
            "bank_dir": bank.to_string_lossy().replace('\\', "/"),
            "config_json_path": profile.to_string_lossy().replace('\\', "/"),
            "enable_audio": true,
        })
        .to_string();
        let opts = CString::new(opts_json).unwrap();
        let mut err = [0u8; 256];
        let h = unsafe { f90_core_create(opts.as_ptr(), err.as_mut_ptr(), err.len() as u32) };
        assert!(!h.is_null(), "create failed: {}", String::from_utf8_lossy(&err));
        let id = f90_core_spawn(h);
        assert_ne!(id, 0, "spawn must succeed");

        if !f90_core_audio_worker_start(h) {
            eprintln!("[ffi] worker start refused (mixer unavailable); skipping");
            unsafe { f90_core_destroy(h) };
            return;
        }
        let handle = f90_core_audio_worker_handle(h);
        assert!(!handle.is_null(), "started worker must expose a handle");
        let handle_addr = handle as usize;

        let stop = AtomicU32::new(0);
        let stop_addr = &stop as *const AtomicU32 as usize;
        let worker_thread = std::thread::spawn(move || unsafe {
            f90_core_audio_worker_run(
                handle_addr as *const c_void,
                stop_addr as *const u32,
            );
        });

        let samples = flat_samples();
        let mut out = F90CoreFrameOut::default();
        for _ in 0..60 {
            unsafe {
                f90_core_step(
                    h, id, 0.0, 0.3, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                    0.0, 0.0, 0.0, 0.0, 1, 0, 1.0 / 120.0, samples.as_ptr(), std::ptr::null_mut(),
                    &mut out,
                )
            };
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        std::thread::sleep(std::time::Duration::from_millis(250));

        let mut l = vec![0f32; 4_096];
        let mut r = vec![0f32; 4_096];
        let pulled = unsafe { f90_core_audio_worker_pull(h, l.as_mut_ptr(), r.as_mut_ptr(), 4_096) };
        assert!(pulled > 0, "worker must have produced PCM for the pull");

        let mut stats = AudioWorkerStats::default();
        assert!(unsafe { f90_core_audio_worker_stats(h, &mut stats) });
        assert_eq!(stats.healthy, 1);
        assert!(stats.produced_frames > 0);
        assert!(stats.packets_applied > 0, "steps must reach the worker");
        assert_eq!(stats.packets_dropped, 0);

        stop.store(1, Ordering::Release);
        worker_thread.join().expect("worker thread must join");
        unsafe { f90_core_destroy(h) };
    }

    #[test]
    fn gear_request_sentinel_maps_neutral_and_reverse() {
        // Contract (vehicle_physics_engine::ffi): -2/below = no change,
        // -1 = Reverse, 0 = Neutral, 1.. = forward gear.
        assert_eq!(gear_request_from_abi(-2), None, "-2 must mean no change");
        assert_eq!(gear_request_from_abi(-3), None, "below -2 must mean no change");
        assert_eq!(gear_request_from_abi(-1), Some(-1), "-1 must select Reverse");
        assert_eq!(
            gear_request_from_abi(0),
            Some(0),
            "0 must select Neutral (was broken: 0 mapped to no-change)"
        );
        for gear in 1..=6 {
            assert_eq!(gear_request_from_abi(gear), Some(gear));
        }
    }

    #[test]
    fn f90_core_step_gear_sentinel_reaches_neutral_then_reverse() {
        let opts = CString::new(r#"{"use_canonical":true,"enable_audio":false}"#).unwrap();
        let mut err = [0u8; 256];
        let h = unsafe { f90_core_create(opts.as_ptr(), err.as_mut_ptr(), err.len() as u32) };
        assert!(!h.is_null(), "create failed: {}", String::from_utf8_lossy(&err));
        let id = f90_core_spawn(h);
        assert_ne!(id, 0, "spawn must succeed");

        let samples = flat_samples();
        let mut out = F90CoreFrameOut::default();
        let mut step_with_gear = |gear_request: i8| {
            for _ in 0..120 {
                unsafe {
                    f90_core_step(
                        h, id, 0.0, 0.3, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                        0.0, 0.0, 0.0, 0.0, 0.0, gear_request, 0, 1.0 / 120.0, samples.as_ptr(),
                        std::ptr::null_mut(), &mut out,
                    )
                };
            }
            out.gear
        };

        assert_eq!(step_with_gear(0), 0, "gear_request=0 must reach Neutral");
        assert_eq!(step_with_gear(-1), -1, "gear_request=-1 must reach Reverse");

        unsafe { f90_core_destroy(h) };
    }
}
