//! Audio subsystem inside the facade.
//!
//! Wraps `vehicle_audio_engine` (the pure-Rust sample-accurate mixer) and drives it
//! from the SAME telemetry the physics step just produced — no Godot property
//! round-trip. Note the mixer already fires gear-shift one-shots internally on
//! `set_state` (see `mixer.rs`), so the facade must NOT re-trigger on gear change
//! (that was a double-trigger in the legacy C++ controller).

use std::path::Path;

use vehicle_audio_engine::{
    AudioBackend, AudioCommandFrame, AudioTelemetryFrame, CollisionAudioInput,
    CommonV10BankAdapter, FamilyMutes, Trigger, VehicleAudioEngine,
};
use vehicle_physics_engine::SurfaceType;

use crate::frame::{
    AudioReadouts, BED_GRASS, BED_NONE, BED_RUMBLE, BED_SAND, SURFACE_ASPHALT, SURFACE_GRASS,
    SURFACE_RUMBLE, SURFACE_SAND,
};

/// Primary surface token derived from a physics `SurfaceType`, matching the legacy
/// C++ `surface_token_from_wheel_type` semantics (rumble > grass > sand > road).
pub fn surface_token(st: SurfaceType) -> &'static str {
    match st {
        SurfaceType::Curb => "rumble",
        SurfaceType::Grass => "grass",
        SurfaceType::Gravel | SurfaceType::Dirt | SurfaceType::Sand => "sand",
        _ => "asphalt",
    }
}

pub fn surface_code(st: SurfaceType) -> u8 {
    match st {
        SurfaceType::Curb => SURFACE_RUMBLE,
        SurfaceType::Grass => SURFACE_GRASS,
        SurfaceType::Gravel | SurfaceType::Dirt | SurfaceType::Sand => SURFACE_SAND,
        _ => SURFACE_ASPHALT,
    }
}

fn bed_code(token: &str) -> u8 {
    match token {
        "rumble" => BED_RUMBLE,
        "grass" => BED_GRASS,
        "sand" => BED_SAND,
        _ => BED_NONE,
    }
}

/// Legacy one-shot code table (mirrors `VehicleAudioControllerNative::trigger_code`).
fn code_from_bank_key(key: &str) -> i32 {
    match key {
        "shift_up" => 0,
        "shift_down" => 1,
        "engine_backfire" => 2,
        "impact_hit_1" => 3,
        "impact_hit_2" => 4,
        "impact_hit_3" => 5,
        "impact_hit_4" => 6,
        "impact_barrier" => 7,
        "impact_cone" => 8,
        "impact_fire" => 9,
        "impact_scrape" => 10,
        _ => -1,
    }
}

/// The facade-owned audio subsystem.
pub struct AudioModule {
    engine: Option<VehicleAudioEngine>,
    enabled: bool,
    last_surface: SurfaceType,
    last_slip: f32,
    last_trigger_code: i32,
    backend: AudioBackend,
    adapter: CommonV10BankAdapter,
    commands: AudioCommandFrame,
    tick: u64,
    scrape_active: bool,
    scrape_intensity: f32,
    scrape_speed_m_s: f32,
    listener_distance_m: f32,
    /// Optional per-tick telemetry recorder (Fase 6 A/B): appends one CSV line
    /// per tick; replayed offline through both backends by
    /// `vehicle-audio-engine/examples/live_replay.rs`.
    record_path: Option<std::path::PathBuf>,
}

impl AudioModule {
    /// Build the audio subsystem. If `enabled` and the bank fails to load, the
    /// module degrades to telemetry-only (health flag) — it never fails the facade.
    pub fn new(bank_dir: Option<&Path>, enabled: bool, backend: AudioBackend) -> Self {
        let engine = if enabled && backend == AudioBackend::LegacyV10Pcm {
            match bank_dir {
                Some(dir) => VehicleAudioEngine::new(dir).ok(),
                None => None,
            }
        } else {
            None
        };
        if enabled && backend == AudioBackend::LegacyV10Pcm && engine.is_none() {
            eprintln!("[formula90_core] audio bank load failed (or no bank_dir): telemetry-only");
        }
        Self {
            engine,
            enabled,
            last_surface: SurfaceType::Road,
            last_slip: 0.0,
            last_trigger_code: -1,
            backend,
            adapter: CommonV10BankAdapter::default(),
            commands: AudioCommandFrame::default(),
            tick: 0,
            scrape_active: false,
            scrape_intensity: 0.0,
            scrape_speed_m_s: 0.0,
            listener_distance_m: 0.0,
            record_path: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// True when the mixer is actually usable (bank loaded).
    pub fn healthy(&self) -> bool {
        self.enabled && self.engine.is_some()
    }

    pub fn mut_engine(&mut self) -> Option<&mut VehicleAudioEngine> {
        self.engine.as_mut()
    }

    /// Record the surface/slip the physics step observed this tick (used even when
    /// the mixer is missing so readouts stay meaningful).
    pub fn observe(&mut self, surface: SurfaceType, slip: f32) {
        self.last_surface = surface;
        self.last_slip = slip;
    }

    /// Push the current telemetry into the mixer. Gear-change one-shots fire inside
    /// the mixer; the facade must not re-trigger them.
    pub fn set_state(
        &mut self,
        rpm: f64,
        idle_rpm: f64,
        max_rpm: f64,
        throttle: f32,
        speed_kph: f64,
        drivetrain_speed_kph: f32,
        gear: i32,
        slip: f32,
        surface: SurfaceType,
        surfaces: [u8; 4],
        dt_s: f32,
        brake: f32,
        slip_ratio: [f32; 4],
        wheel_load_n: [f32; 4],
        suspension_velocity_m_s: [f32; 4],
        tire_pressure_kpa: [f32; 4],
    ) {
        self.observe(surface, slip);
        self.tick += 1;
        if let Some(path) = &self.record_path {
            record_tick(
                path,
                self.tick,
                rpm,
                throttle,
                speed_kph,
                drivetrain_speed_kph,
                gear,
                slip,
                surfaces,
                brake,
                slip_ratio,
                wheel_load_n,
                suspension_velocity_m_s,
                tire_pressure_kpa,
                self.listener_distance_m,
            );
        }
        self.commands = self.adapter.adapt(
            AudioTelemetryFrame {
                tick: self.tick,
                dt_s,
                rpm: rpm as f32,
                throttle,
                speed_kph: speed_kph as f32,
                drivetrain_speed_kph,
                gear,
                slip,
                slip_ratio,
                surfaces,
                brake,
                wheel_load_n,
                suspension_velocity_m_s,
                tire_pressure_kpa,
                listener_distance_m: self.listener_distance_m,
                underfloor_scrape_active: self.scrape_active,
                underfloor_scrape_intensity: self.scrape_intensity,
                underfloor_scrape_speed_m_s: self.scrape_speed_m_s,
                ..Default::default()
            },
            self.backend,
        );
        if let Some(eng) = self.engine.as_mut() {
            let prev_tag = eng.last_trigger().to_string();
            eng.set_state(
                rpm,
                idle_rpm,
                max_rpm,
                throttle,
                speed_kph,
                gear,
                slip,
                surface_token(surface),
            );
            let now_tag = eng.last_trigger();
            self.last_trigger_code = if !now_tag.is_empty() && now_tag != prev_tag {
                code_from_bank_key(now_tag)
            } else {
                self.last_trigger_code
            };
        }
    }

    /// Fire a named one-shot via its legacy code (0..10). Returns true if the mixer
    /// accepted it.
    pub fn trigger_code(&mut self, code: i32) -> bool {
        let Some(trigger) = trigger_from_code(code) else {
            return false;
        };
        if let Some(eng) = self.engine.as_mut() {
            eng.trigger(trigger);
            self.last_trigger_code = code;
            true
        } else {
            // Telemetry-only: still record the trigger so HUD shows it.
            self.last_trigger_code = code;
            true
        }
    }

    /// Render `n` stereo frames from the mixer into `out_l`/`out_r`. Returns the
    /// number of frames written (0 when the mixer is unavailable).
    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32], n: usize) -> usize {
        match self.engine.as_mut() {
            Some(eng) => {
                let n = n.min(out_l.len()).min(out_r.len());
                let (l, r) = (&mut out_l[..n], &mut out_r[..n]);
                eng.render(l, r, n);
                n
            }
            None => 0,
        }
    }

    /// Presentation readouts for HUD/telemetry mirrors.
    pub fn readouts(&mut self) -> AudioReadouts {
        let surface = self.last_surface;
        let base = AudioReadouts {
            surface_code: surface_code(surface),
            active_bed_code: bed_code(surface_token(surface)),
            trigger_code: self.last_trigger_code,
            last_slip: self.last_slip,
            ..Default::default()
        };
        match self.engine.as_mut() {
            Some(eng) => AudioReadouts {
                surface_code: base.surface_code,
                active_bed_code: base.active_bed_code,
                trigger_code: self.last_trigger_code,
                last_norm: eng.last_norm(),
                last_rpm: eng.last_rpm(),
                last_throttle: eng.last_throttle(),
                last_speed_kph: eng.last_speed_kph(),
                last_slip: eng.last_slip(),
                last_engine_gain: eng.last_engine_gain(),
                weights: eng.last_weights(),
                pitches: eng.last_pitches(),
                scrape_gain: eng.scrape_gain(),
                scrape_pitch: eng.scrape_pitch(),
                scrape_cursor: eng.scrape_cursor(),
            },
            None => base,
        }
    }

    pub fn set_scrape_state(
        &mut self,
        active: bool,
        intensity: f32,
        speed_m_s: f32,
        onset_strength: f32,
    ) {
        self.scrape_active = active;
        self.scrape_intensity = intensity;
        self.scrape_speed_m_s = speed_m_s;
        if let Some(eng) = self.engine.as_mut() {
            eng.set_scrape_state(active, intensity, speed_m_s, onset_strength);
        }
    }

    pub fn command_frame_json(&self) -> String {
        serde_json::to_string(&self.commands).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn push_collision(&mut self, collision: CollisionAudioInput) {
        if self.backend == AudioBackend::CommonV10Commands {
            self.adapter.push_collision(collision);
        }
    }

    /// Diagnostic A/B mute mask (handoff section 7 paso 2): bit0=engine_int,
    /// bit1=engine_ext, bit2=transmission, bit3=wind, bit4=wheel, bit5=surfaces,
    /// bit6=collisions, bit7=ambience. Muted families keep their voices at -80 dB.
    pub fn set_mute_mask(&mut self, mask: u8) {
        self.adapter.set_mutes(FamilyMutes {
            engine_int: mask & 1 != 0,
            engine_ext: mask & 2 != 0,
            transmission: mask & 4 != 0,
            wind: mask & 8 != 0,
            wheel: mask & 16 != 0,
            surfaces: mask & 32 != 0,
            collisions: mask & 64 != 0,
            ambience: mask & 128 != 0,
        });
    }

    pub fn backend(&self) -> AudioBackend {
        self.backend
    }

    pub fn set_listener_distance(&mut self, distance_m: f32) {
        self.listener_distance_m = distance_m.max(0.0);
    }

    /// Start appending per-tick telemetry to `path` (CSV, for offline A/B replay).
    pub fn set_record_path(&mut self, path: Option<std::path::PathBuf>) {
        self.record_path = path;
    }
}

/// One CSV line per audio tick — the exact inputs the adapter consumed. Layout:
/// tick,rpm,throttle,speed_kph,drivetrain_speed_kph,gear,slip,
/// slip_ratio[4],surfaces[4],brake,wheel_load_n[4],suspension_velocity_m_s[4],
/// tire_pressure_kpa[4],listener_distance_m
fn record_tick(
    path: &std::path::Path,
    tick: u64,
    rpm: f64,
    throttle: f32,
    speed_kph: f64,
    drivetrain_speed_kph: f32,
    gear: i32,
    slip: f32,
    surfaces: [u8; 4],
    brake: f32,
    slip_ratio: [f32; 4],
    wheel_load_n: [f32; 4],
    suspension_velocity_m_s: [f32; 4],
    tire_pressure_kpa: [f32; 4],
    listener_distance_m: f32,
) {
    use std::io::Write;
    let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = writeln!(
        f,
        "{tick},{rpm:.3},{throttle:.5},{speed_kph:.3},{drivetrain_speed_kph:.3},{gear},{slip:.5},\
         {},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        slip_ratio[0], slip_ratio[1], slip_ratio[2], slip_ratio[3],
        surfaces[0], surfaces[1], surfaces[2], surfaces[3],
        brake,
        wheel_load_n[0], wheel_load_n[1], wheel_load_n[2], wheel_load_n[3],
        suspension_velocity_m_s[0], suspension_velocity_m_s[1],
        suspension_velocity_m_s[2], suspension_velocity_m_s[3],
        tire_pressure_kpa[0], tire_pressure_kpa[1], tire_pressure_kpa[2],
        tire_pressure_kpa[3],
        listener_distance_m,
    );
}

fn trigger_from_code(code: i32) -> Option<Trigger> {
    match code {
        0 => Some(Trigger::ShiftUp),
        1 => Some(Trigger::ShiftDown),
        2 => Some(Trigger::Backfire),
        3 => Some(Trigger::Hit1),
        4 => Some(Trigger::Hit2),
        5 => Some(Trigger::Hit3),
        6 => Some(Trigger::Hit4),
        7 => Some(Trigger::Barrier),
        8 => Some(Trigger::Cone),
        9 => Some(Trigger::Fire),
        10 => Some(Trigger::Scrape),
        _ => None,
    }
}
