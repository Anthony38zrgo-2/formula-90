//! Orchestrated frame: the atomic outcome of one `CoreFacade::step`.
//!
//! Every tick the facade produces a single `CoreFrame` carrying physics force/torque,
//! the compact telemetry (`EntityTelemetry`), audio readouts and per-module outputs.
//! Mirrors (C++ `F90Core` node, headless `core_cli`) consume this frame; the audio
//! render pulls from the same frame so audio never sees stale phyics.

use serde::{Deserialize, Serialize};

/// Surface codes mirrored in `native/include/formula90s/core/f90_core.h`.
pub const SURFACE_ASPHALT: u8 = 0;
pub const SURFACE_RUMBLE: u8 = 1;
pub const SURFACE_GRASS: u8 = 2;
pub const SURFACE_SAND: u8 = 3;

/// Active surface-bed codes mirrored in `f90_core.h`.
pub const BED_NONE: u8 = 0;
pub const BED_RUMBLE: u8 = 1;
pub const BED_GRASS: u8 = 2;
pub const BED_SAND: u8 = 3;

/// Audio presentation readouts a HUD/telemetry mirror needs (mirrors the legacy
/// `VehicleAudioControllerNative` getter surface).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioReadouts {
    pub surface_code: u8,
    pub active_bed_code: u8,
    /// One-shot trigger code (0..10, legacy mapping) or -1 if none this tick.
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

impl Default for AudioReadouts {
    fn default() -> Self {
        Self {
            surface_code: SURFACE_ASPHALT,
            active_bed_code: BED_NONE,
            trigger_code: -1,
            last_norm: 0.0,
            last_rpm: 0.0,
            last_throttle: 0.0,
            last_speed_kph: 0.0,
            last_slip: 0.0,
            last_engine_gain: 0.0,
            weights: [0.0; 5],
            pitches: [1.0; 5],
        }
    }
}

/// Serializable per-module contribution merged into the facade snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleOutput {
    pub name: String,
    pub payload: Vec<u8>,
}

/// The complete orchestrated outcome of one fixed step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreFrame {
    pub time_ms: i64,
    pub force: [f64; 3],
    pub torque: [f64; 3],
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
    pub tc_active: bool,
    pub drive_torque: f64,
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
    // Tire pressure + thermal (per wheel, WheelIndex order FL/FR/RL/RR).
    pub tire_pressure_kpa: [f64; 4],
    pub tire_tread_inner_c: [f64; 4],
    pub tire_tread_center_c: [f64; 4],
    pub tire_tread_outer_c: [f64; 4],
    pub tire_carcass_c: [f64; 4],
    pub tire_gas_c: [f64; 4],
    pub audio: AudioReadouts,
    pub modules: Vec<ModuleOutput>,
}

impl Default for CoreFrame {
    fn default() -> Self {
        Self {
            time_ms: 0,
            force: [0.0; 3],
            torque: [0.0; 3],
            speed_kmh: 0.0,
            rpm: 0.0,
            gear: 0,
            steer: 0.0,
            throttle: 0.0,
            lat_g: 0.0,
            long_g: 0.0,
            vert_g: 0.0,
            fl_comp_mm: 0.0,
            fr_comp_mm: 0.0,
            rl_comp_mm: 0.0,
            rr_comp_mm: 0.0,
            front_slip: 0.0,
            rear_slip: 0.0,
            tc_active: false,
            drive_torque: 0.0,
            px: 0.0,
            py: 0.0,
            pz: 0.0,
            yaw: 0.0,
            lvx: 0.0,
            lvy: 0.0,
            lvz: 0.0,
            avx: 0.0,
            avy: 0.0,
            avz: 0.0,
            tire_pressure_kpa: [0.0; 4],
            tire_tread_inner_c: [0.0; 4],
            tire_tread_center_c: [0.0; 4],
            tire_tread_outer_c: [0.0; 4],
            tire_carcass_c: [0.0; 4],
            tire_gas_c: [0.0; 4],
            audio: AudioReadouts::default(),
            modules: Vec::new(),
        }
    }
}
