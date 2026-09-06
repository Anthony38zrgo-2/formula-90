//! Audio subsystem inside the facade.
//!
//! Wraps `vehicle_audio_engine` (the pure-Rust sample-accurate mixer) and drives it
//! from the SAME telemetry the physics step just produced — no Godot property
//! round-trip. Note the mixer already fires gear-shift one-shots internally on
//! `set_state` (see `mixer.rs`), so the facade must NOT re-trigger on gear change
//! (that was a double-trigger in the legacy C++ controller).

use std::path::{Path, PathBuf};

use vehicle_audio_engine::Trigger;
use vehicle_audio_engine::VehicleAudioEngine;
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
    gf509_asset_dir: Option<PathBuf>,
    telemetry_adapter: crate::audio_telemetry::AudioTelemetryAdapter,
}

impl AudioModule {
    /// Build the audio subsystem. If `enabled` and the bank fails to load, the
    /// module degrades to telemetry-only (health flag) — it never fails the facade.
    pub fn new(bank_dir: Option<&Path>, enabled: bool) -> Self {
        let gf509_asset_dir = bank_dir.and_then(|dir| {
            dir.parent()?
                .parent()?
                .parent()
                .map(|game| game.join("audio/v10_gf509"))
        });
        let engine = if enabled {
            match bank_dir {
                Some(dir) => VehicleAudioEngine::new(dir).ok(),
                None => None,
            }
        } else {
            None
        };
        if enabled && engine.is_none() {
            eprintln!("[formula90_core] audio bank load failed (or no bank_dir): telemetry-only");
        }
        Self {
            engine,
            enabled,
            last_surface: SurfaceType::Road,
            last_slip: 0.0,
            last_trigger_code: -1,
            gf509_asset_dir,
            telemetry_adapter: Default::default(),
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

    /// Apply the listener/ambient downlink (camera-to-vehicle distance, TC cut
    /// ratio, RPM-limiter enabled flag). Returns false when the mixer is absent.
    pub fn set_ambient(
        &mut self,
        distance_m: f32,
        tc_cut_ratio: f32,
        limiter_active: bool,
    ) -> bool {
        match self.engine.as_mut() {
            Some(eng) => {
                eng.set_listener_distance(distance_m);
                eng.set_tc_cut(tc_cut_ratio);
                eng.set_limiter_flag(limiter_active);
                true
            }
            None => false,
        }
    }

    /// Record the surface/slip the physics step observed this tick (used even when
    /// the mixer is missing so readouts stay meaningful).
    pub fn observe(&mut self, surface: SurfaceType, slip: f32) {
        self.last_surface = surface;
        self.last_slip = slip;
    }

    /// Enable the procedural synth from the profile `audio` section (a JSON object
    /// whose `powertrain_synthesis` child is parsed by
    /// `vehicle_audio_engine::powertrain`). Returns false when the mixer is absent
    /// or the profile has no (valid) enabled `powertrain_synthesis` section. The
    /// fallback is a no-op: the engine keeps its sampled-band path (silent engine).
    pub fn enable_synth_from_profile(&mut self, audio: Option<&serde_json::Value>) -> bool {
        let Some(audio) = audio else {
            return false;
        };
        // `from_profile_json` expects the full profile JSON with an `audio` root;
        // re-wrap the passthrough section to satisfy that contract.
        let wrapped = serde_json::json!({ "audio": audio });
        let Ok(Some(config)) =
            vehicle_audio_engine::powertrain::from_profile_json(&wrapped.to_string())
        else {
            return false;
        };
        if !config.enabled {
            return false;
        }
        if let Some(eng) = self.engine.as_mut() {
            eng.enable_synth(&config);
            // Optional C++/Faust post-combustion layer. Loading, symbol
            // resolution, BUILD validation and instance creation all happen
            // here during profile setup, never in `render()`.
            if let Some(cpp) = audio.get("cpp_dsp") {
                let enabled = cpp
                    .get("enabled")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let gain = cpp
                    .get("gain")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0) as f32;
                if enabled {
                    match vehicle_audio_engine::dsp_runtime::DspRuntime::load_default() {
                        Ok(runtime) => {
                            eng.attach_cpp_dsp(runtime);
                            eng.set_cpp_layer_gain(gain);
                            eng.set_cpp_layer_enabled(true);
                        }
                        Err(error) => {
                            eprintln!(
                                "[formula90_core] C++/Faust audio layer unavailable: {error:?}"
                            );
                            eng.set_cpp_layer_enabled(false);
                            eng.set_cpp_layer_gain(0.0);
                        }
                    }
                } else {
                    eng.set_cpp_layer_enabled(false);
                    eng.set_cpp_layer_gain(0.0);
                }
            }
            let source = audio
                .get("continuous_source")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("legacy");
            if source == "v10_gf509" {
                let gf509_gain = audio
                    .get("gf509")
                    .and_then(|value| value.get("gain"))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(1.0) as f32;
                eng.set_synth_volume(gf509_gain);
                let diagnostic_mode = audio
                    .get("gf509")
                    .and_then(|value| value.get("diagnostic_mode"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("mix");
                eng.set_diagnostic_mode(match diagnostic_mode {
                    "v10_only" => vehicle_audio_engine::DiagnosticMode::V10Only,
                    "events_only" => vehicle_audio_engine::DiagnosticMode::EventsOnly,
                    _ => vehicle_audio_engine::DiagnosticMode::Mix,
                });
                match self.gf509_asset_dir.as_deref() {
                    Some(directory) => {
                        if let Err(error) = eng.enable_v10_gf509(directory) {
                            eprintln!(
                                "[formula90_core] GF509 initialization failed; using legacy: {error}"
                            );
                        }
                    }
                    None => {
                        eprintln!("[formula90_core] GF509 asset root unavailable; using legacy");
                        eng.use_legacy_continuous_source();
                    }
                }
            } else {
                eng.use_legacy_continuous_source();
            }
            // `enable_synth` returns the *previous* enabled state; the real result
            // is observable via `synth_enabled`.
            eng.synth_enabled()
                || eng.continuous_source() == vehicle_audio_engine::ContinuousSourceKind::V10Gf509
        } else {
            false
        }
    }

    /// Whether the procedural synth is currently driving the engine path.
    pub fn synth_enabled(&self) -> bool {
        self.engine
            .as_ref()
            .map_or(false, VehicleAudioEngine::synth_enabled)
    }

    pub fn gf509_enabled(&self) -> bool {
        self.engine.as_ref().is_some_and(|engine| {
            engine.continuous_source() == vehicle_audio_engine::ContinuousSourceKind::V10Gf509
        })
    }

    /// Push the current telemetry into the mixer. Gear-change one-shots fire inside
    /// the mixer; the facade must not re-trigger them.
    pub(crate) fn set_physical_state(
        &mut self,
        rpm: f64,
        idle_rpm: f64,
        max_rpm: f64,
        throttle: f32,
        speed_kph: f64,
        gear: i32,
        mechanical: crate::audio_telemetry::MechanicalAudioState,
        dt_seconds: f32,
        slip: f32,
        surface: SurfaceType,
    ) {
        self.observe(surface, slip);
        let packet = self.telemetry_adapter.update(rpm, idle_rpm, max_rpm, throttle,
            speed_kph, gear, slip, dt_seconds, mechanical);
        if let Some(eng) = self.engine.as_mut() {
            let prev_code = code_from_bank_key(eng.last_trigger());
            eng.set_telemetry_timed(&packet, surface_token(surface), dt_seconds);
            let now_tag = eng.last_trigger();
            self.last_trigger_code = if !now_tag.is_empty() && code_from_bank_key(now_tag) != prev_code {
                code_from_bank_key(now_tag)
            } else {
                self.last_trigger_code
            };
        }
    }

    pub fn reset(&mut self) {
        self.telemetry_adapter = Default::default();
        if let Some(engine) = self.engine.as_mut() {
            let _ = engine.reset_audio_state();
        }
        self.last_surface = SurfaceType::Road;
        self.last_slip = 0.0;
        self.last_trigger_code = -1;
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
                tyre_scrub_gain: eng.tyre_scrub_gain(),
                tyre_scrub_pitch: eng.tyre_scrub_pitch(),
                tyre_scrub_mode: eng.tyre_scrub_mode(),
                tyre_scrub_cursor: eng.tyre_scrub_cursor(),
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
        if let Some(eng) = self.engine.as_mut() {
            eng.set_scrape_state(active, intensity, speed_m_s, onset_strength);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_tire_scrub_state(
        &mut self,
        slip_ratio: [f32; 4],
        slip_angle_rad: [f32; 4],
        contact_fraction: [f32; 4],
        normal_force_n: [f32; 4],
        speed_kph: f32,
        surface: SurfaceType,
    ) {
        if let Some(eng) = self.engine.as_mut() {
            eng.set_tire_scrub_state(
                slip_ratio,
                slip_angle_rad,
                contact_fraction,
                normal_force_n,
                speed_kph,
                surface_token(surface),
            );
        }
    }
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
