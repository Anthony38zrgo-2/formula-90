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

use crate::audio_worker::{surface_from_kind, AudioStepPacket};
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

/// Translate one `audio.gf509.v10_filter` / `gearbox_filter` object into a
/// spectrum chain config. Absent keys keep the runtime bypass defaults.
fn spectrum_chain_from_value(
    value: &serde_json::Value,
    label: &str,
) -> Result<v10_engine_synth::SpectrumChainConfig, String> {
    let scalar = |name: &str| -> Result<Option<f32>, String> {
        match value.get(name) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(raw) => {
                let number = raw
                    .as_f64()
                    .ok_or_else(|| format!("{label}.{name} must be a number"))?;
                if !number.is_finite() {
                    return Err(format!("{label}.{name} is not finite"));
                }
                Ok(Some(number as f32))
            }
        }
    };
    let mut config = v10_engine_synth::SpectrumChainConfig::default();
    if let Some(number) = scalar("highpass_hz")? {
        config.highpass_hz = number;
    }
    if let Some(number) = scalar("highpass_slope_db_per_oct")? {
        config.highpass_slope_db_per_oct = number;
    }
    if let Some(number) = scalar("lowpass_hz")? {
        config.lowpass_hz = number;
    }
    if let Some(number) = scalar("lowpass_slope_db_per_oct")? {
        config.lowpass_slope_db_per_oct = number;
    }
    if let Some(number) = scalar("peak_hz")? {
        config.peak_hz = number;
    }
    if let Some(number) = scalar("peak_gain_db")? {
        config.peak_gain_db = number;
    }
    if let Some(number) = scalar("peak_q")? {
        config.peak_q = number;
    }
    if let Some(number) = scalar("peak_drive")? {
        config.peak_drive = number;
    }
    if let Some(number) = scalar("peak_drive_mix")? {
        config.peak_drive_mix = number;
    }
    if config.highpass_hz < 0.0
        || config.lowpass_hz < 0.0
        || config.highpass_slope_db_per_oct < 0.0
        || config.lowpass_slope_db_per_oct < 0.0
        || config.peak_hz < 0.0
        || config.peak_q < 0.0
        || config.peak_drive < 0.0
        || !(0.0..=1.0).contains(&config.peak_drive_mix)
    {
        return Err(format!("{label} out of range"));
    }
    Ok(config)
}

/// Translate the profile `audio.gf509` section into explicit layer tuning.
/// Absent keys reproduce shipped GF509 behavior exactly; geometry keys select
/// an exhaust candidate only when the profile declares them.
fn v10_layer_tuning_from_section(
    section: Option<&serde_json::Value>,
) -> Result<vehicle_audio_engine::V10LayerTuning, String> {
    let mut tuning = vehicle_audio_engine::V10LayerTuning::default();
    let Some(section) = section else {
        return Ok(tuning);
    };
    tuning.disable_sample_rasp = section
        .get("disable_sample_rasp")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    tuning.residual_gain_scale = section
        .get("residual_gain_scale")
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32);
    if let Some(value) = section
        .get("upper_mid_shelf_gain")
        .and_then(serde_json::Value::as_f64)
    {
        let value = value as f32;
        if !value.is_finite() {
            return Err("upper_mid_shelf_gain is not finite".into());
        }
        tuning.upper_mid_shelf_gain = value;
    }
    if let Some(gains) = section.get("scene_gains").and_then(|value| value.as_object()) {
        for (branch, gain) in gains {
            if let Some(gain) = gain.as_f64() {
                tuning.scene_gains.push((branch.clone(), gain as f32));
            }
        }
    }
    if let Some(scale) = section.get("header_length_scale").and_then(serde_json::Value::as_f64) {
        tuned_scalar(&mut tuning.header_length_scale, "header_length_scale", scale)?;
    }
    if let Some(hz) = section
        .get("cover_radiation_lowpass_hz")
        .and_then(serde_json::Value::as_f64)
    {
        tuned_scalar(
            &mut tuning.cover_radiation_lowpass_hz,
            "cover_radiation_lowpass_hz",
            hz,
        )?;
    }
    if let Some(weight) = section
        .get("sample_blend_weight")
        .and_then(serde_json::Value::as_f64)
    {
        tuned_scalar(&mut tuning.sample_blend_weight, "sample_blend_weight", weight)?;
    }
    if let Some(weight) = section
        .get("physical_blend_weight")
        .and_then(serde_json::Value::as_f64)
    {
        tuned_scalar(
            &mut tuning.physical_blend_weight,
            "physical_blend_weight",
            weight,
        )?;
    }
    if let Some(trims) = section
        .get("sample_zone_trim_db")
        .and_then(serde_json::Value::as_array)
    {
        let mut values = Vec::with_capacity(trims.len());
        for value in trims {
            let trim = value
                .as_f64()
                .ok_or_else(|| "sample_zone_trim_db entries must be numbers".to_string())?;
            if !trim.is_finite() {
                return Err("sample_zone_trim_db entry is not finite".into());
            }
            values.push(trim as f32);
        }
        tuning.zone_trim_db = Some(values);
    }
    if let Some(geometry) = section.get("collector_geometry").filter(|value| !value.is_null()) {
        let field = |name: &str| -> Result<f32, String> {
            geometry
                .get(name)
                .and_then(serde_json::Value::as_f64)
                .map(|value| value as f32)
                .ok_or_else(|| format!("collector_geometry.{name} missing or not a number"))
        };
        tuning.collector_geometry = Some(v10_engine_synth::CollectorGeometry {
            volume_l: field("volume_l")?,
            outlet_length_m: field("outlet_length_m")?,
            outlet_diameter_m: field("outlet_diameter_m")?,
            gas_temperature_k: field("gas_temperature_k")?,
            loss_fraction_per_cycle: field("loss_fraction_per_cycle")?,
            mode_coupling: field("mode_coupling")?,
        });
    }
    if let Some(value) = section
        .get("gear_shift_gain")
        .and_then(serde_json::Value::as_f64)
    {
        let value = value as f32;
        if !value.is_finite() {
            return Err("gear_shift_gain is not finite".into());
        }
        tuning.gear_shift_gain = Some(value);
    }
    if let Some(value) = section
        .get("gear_shift_reference_rpm")
        .and_then(serde_json::Value::as_f64)
    {
        let value = value as f32;
        if !value.is_finite() || value <= 0.0 {
            return Err("gear_shift_reference_rpm out of range".into());
        }
        tuning.gear_shift_reference_rpm = Some(value);
    }
    if let Some(value) = section
        .get("engine_mode")
        .and_then(serde_json::Value::as_str)
    {
        tuning.engine_mode = match value {
            "hybrid" => v10_engine_synth::EngineMode::Hybrid,
            "sample_only" => v10_engine_synth::EngineMode::SampleOnly,
            other => return Err(format!("unknown engine_mode: {other}")),
        };
    }
    if let Some(value) = section
        .get("sampled_gearbox_gain")
        .and_then(serde_json::Value::as_f64)
    {
        let value = value as f32;
        if !value.is_finite() || value < 0.0 {
            return Err("sampled_gearbox_gain out of range".into());
        }
        tuning.sampled_gearbox_gain = Some(value);
    }
    if let Some(value) = section
        .get("sampled_engine_gain")
        .and_then(serde_json::Value::as_f64)
    {
        let value = value as f32;
        if !value.is_finite() || value < 0.0 {
            return Err("sampled_engine_gain out of range".into());
        }
        tuning.sampled_engine_gain = Some(value);
    }
    if let Some(value) = section.get("v10_filter").filter(|value| !value.is_null()) {
        tuning.v10_filter = spectrum_chain_from_value(value, "v10_filter")?;
    }
    if let Some(value) = section
        .get("gearbox_filter")
        .filter(|value| !value.is_null())
    {
        tuning.gearbox_filter = spectrum_chain_from_value(value, "gearbox_filter")?;
    }
    if let Some(transmission) = section.get("transmission").filter(|value| !value.is_null()) {
        if let Some(gain) = transmission.get("gain").and_then(serde_json::Value::as_f64) {
            let gain = gain as f32;
            if !gain.is_finite() {
                return Err("transmission.gain is not finite".into());
            }
            tuning.transmission_gain = gain;
        }
        if let Some(value) = transmission.get("whine").and_then(serde_json::Value::as_f64) {
            tuned_scalar(&mut tuning.transmission_whine_gain, "transmission.whine", value)?;
        }
        if let Some(value) = transmission.get("clack").and_then(serde_json::Value::as_f64) {
            tuned_scalar(&mut tuning.transmission_clack_gain, "transmission.clack", value)?;
        }
        if let Some(value) = transmission.get("rattle").and_then(serde_json::Value::as_f64) {
            tuned_scalar(&mut tuning.transmission_rattle_gain, "transmission.rattle", value)?;
        }
        if let Some(value) = transmission.get("clutch").and_then(serde_json::Value::as_f64) {
            tuned_scalar(&mut tuning.transmission_clutch_gain, "transmission.clutch", value)?;
        }
        if let Some(value) = transmission
            .get("final_drive")
            .and_then(serde_json::Value::as_f64)
        {
            tuned_scalar(&mut tuning.transmission_final_drive, "transmission.final_drive", value)?;
        }
        if let Some(value) = transmission
            .get("reverse_ratio")
            .and_then(serde_json::Value::as_f64)
        {
            tuned_scalar(
                &mut tuning.transmission_reverse_ratio,
                "transmission.reverse_ratio",
                value,
            )?;
        }
        if let Some(value) = transmission
            .get("gear_teeth")
            .and_then(serde_json::Value::as_f64)
        {
            tuned_scalar(&mut tuning.transmission_gear_teeth, "transmission.gear_teeth", value)?;
        }
        if let Some(value) = transmission
            .get("final_teeth")
            .and_then(serde_json::Value::as_f64)
        {
            tuned_scalar(
                &mut tuning.transmission_final_teeth,
                "transmission.final_teeth",
                value,
            )?;
        }
        if let Some(ratios) = transmission
            .get("gear_ratios")
            .and_then(serde_json::Value::as_array)
        {
            let mut values = Vec::with_capacity(ratios.len());
            for value in ratios {
                let ratio = value.as_f64().ok_or_else(|| {
                    "transmission.gear_ratios entries must be numbers".to_string()
                })?;
                if !ratio.is_finite() || ratio <= 0.0 {
                    return Err("transmission.gear_ratios entry out of range".into());
                }
                values.push(ratio as f32);
            }
            tuning.transmission_gear_ratios = Some(values);
        }
    }
    Ok(tuning)
}

/// Mirror the profile powertrain ratios into the transmission tuning when the
/// audio section does not declare its own table, so the mesh frequency always
/// follows the physically simulated gearbox.
fn apply_powertrain_transmission(
    tuning: &mut vehicle_audio_engine::V10LayerTuning,
    config: &vehicle_physics_engine::VehicleConfig,
) {
    if tuning.transmission_gear_ratios.is_none() {
        tuning.transmission_gear_ratios = Some(
            config
                .gear_ratios
                .iter()
                .map(|value| *value as f32)
                .collect(),
        );
    }
    if tuning.transmission_final_drive.is_none() {
        tuning.transmission_final_drive = Some(config.final_drive as f32);
    }
    if tuning.transmission_reverse_ratio.is_none() {
        tuning.transmission_reverse_ratio = Some(config.reverse_ratio as f32);
    }
}

fn tuned_scalar(slot: &mut Option<f32>, name: &str, value: f64) -> Result<(), String> {
    let value = value as f32;
    if !value.is_finite() {
        return Err(format!("{name} is not finite"));
    }
    *slot = Some(value);
    Ok(())
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
    /// Game directory root, used to resolve the V10 bank manifest declared
    /// by the vehicle profile (`audio.gf509.manifest`).
    game_root: Option<PathBuf>,
    telemetry_adapter: crate::audio_telemetry::AudioTelemetryAdapter,
}

impl AudioModule {
    /// Build the audio subsystem. If `enabled` and the bank fails to load, the
    /// module degrades to telemetry-only (health flag) — it never fails the facade.
    pub fn new(bank_dir: Option<&Path>, enabled: bool) -> Self {
        let game_root = bank_dir.and_then(|dir| {
            dir.parent()?
                .parent()?
                .parent()
                .map(|game| game.to_path_buf())
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
            game_root,
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
    pub fn enable_synth_from_profile(
        &mut self,
        audio: Option<&serde_json::Value>,
        powertrain: Option<&vehicle_physics_engine::VehicleConfig>,
    ) -> bool {
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
                let gf509_section = audio.get("gf509");
                let gf509_gain = gf509_section
                    .and_then(|value| value.get("gain"))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(1.0) as f32;
                eng.set_synth_volume(gf509_gain);
                let diagnostic_mode = gf509_section
                    .and_then(|value| value.get("diagnostic_mode"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("mix");
                eng.set_diagnostic_mode(match diagnostic_mode {
                    "v10_only" => vehicle_audio_engine::DiagnosticMode::V10Only,
                    "events_only" => vehicle_audio_engine::DiagnosticMode::EventsOnly,
                    _ => vehicle_audio_engine::DiagnosticMode::Mix,
                });
                // Bank directory comes from the profile manifest (schema 1 GF509
                // and schema 2 experimental banks both validate inside the
                // runtime). Missing manifest falls back to the packaged GF509.
                let manifest_rel = gf509_section
                    .and_then(|value| value.get("manifest"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("audio/v10_gf509/manifest.json");
                let bank_directory = self.game_root.as_deref().and_then(|game| {
                    game.join(manifest_rel)
                        .parent()
                        .map(|parent| parent.to_path_buf())
                });
                // Optional per-profile layer tuning. Absent keys reproduce
                // shipped GF509 behavior exactly.
                let mut tuning = match v10_layer_tuning_from_section(gf509_section) {
                    Ok(tuning) => tuning,
                    Err(error) => {
                        eprintln!(
                            "[formula90_core] GF509 tuning invalid; using baseline geometry: {error}"
                        );
                        vehicle_audio_engine::V10LayerTuning::default()
                    }
                };
                if let Some(config) = powertrain {
                    apply_powertrain_transmission(&mut tuning, config);
                }
                match bank_directory.as_deref() {
                    Some(directory) => {
                        if let Err(error) = eng.enable_v10_layer(directory, &tuning) {
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
                normalized_engine_load: eng.last_normalized_engine_load(),
                normalized_engine_torque: eng.last_normalized_engine_torque(),
                torque_sign: eng.last_torque_sign(),
                rpm_derivative: eng.last_rpm_derivative(),
                throttle_derivative: eng.last_throttle_derivative(),
                shift_phase: eng.last_shift_phase(),
                clutch_engagement: eng.last_clutch_engagement(),
                tc_cut_ratio: eng.last_tc_cut_ratio(),
                rev_limiter_active: eng.last_rev_limiter_active(),
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

    /// Apply one physics-tick [`AudioStepPacket`] in the same order the
    /// render-thread fallback uses (scrape -> tire scrub -> physical), so the
    /// worker route and the inline route stay sample-identical.
    pub(crate) fn apply_step_packet(&mut self, packet: &AudioStepPacket) {
        if packet.scrape.present {
            self.set_scrape_state(
                packet.scrape.active,
                packet.scrape.intensity,
                packet.scrape.speed_m_s,
                packet.scrape.onset_strength,
            );
        }
        let surface = surface_from_kind(packet.surface);
        self.set_tire_scrub_state(
            packet.scrub_slip_ratio,
            packet.scrub_slip_angle,
            packet.scrub_contact_fraction,
            packet.scrub_normal_force,
            packet.scrub_speed_kph,
            surface,
        );
        self.set_physical_state(
            packet.rpm,
            packet.idle_rpm,
            packet.max_rpm,
            packet.throttle,
            packet.speed_kph,
            packet.gear,
            packet.mechanical,
            packet.dt,
            packet.slip,
            surface,
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_gf509_section_keeps_baseline_geometry() {
        let tuning = v10_layer_tuning_from_section(None).unwrap();
        assert!(tuning.header_length_scale.is_none());
        assert!(tuning.collector_geometry.is_none());
        assert!(tuning.scene_gains.is_empty());
    }

    #[test]
    fn geometry_keys_are_transported_explicitly() {
        let section = serde_json::json!({
            "scene_gains": { "metal": 0.855 },
            "header_length_scale": 1.5,
            "collector_geometry": {
                "volume_l": 2.5,
                "outlet_length_m": 0.35,
                "outlet_diameter_m": 0.09,
                "gas_temperature_k": 1000.0,
                "loss_fraction_per_cycle": 0.5,
                "mode_coupling": 0.6
            }
        });
        let tuning = v10_layer_tuning_from_section(Some(&section)).unwrap();
        assert_eq!(tuning.header_length_scale, Some(1.5));
        let geometry = tuning.collector_geometry.unwrap();
        assert_eq!(geometry.volume_l, 2.5);
        assert_eq!(geometry.outlet_diameter_m, 0.09);
    }

    #[test]
    fn incomplete_collector_geometry_is_rejected() {
        let section = serde_json::json!({
            "collector_geometry": { "volume_l": 2.5 }
        });
        assert!(v10_layer_tuning_from_section(Some(&section)).is_err());
    }

    #[test]
    fn scene_filter_keys_are_transported_explicitly() {
        let section = serde_json::json!({
            "scene_gains": { "engine_air": 0.841, "engine_cover": 0.9156 },
            "cover_radiation_lowpass_hz": 1_000_000.0,
            "sample_blend_weight": 1.3,
            "physical_blend_weight": 0.77,
            "sample_zone_trim_db": [2.0, 0.0, 0.0, 1.5, 1.5, 1.5],
            "upper_mid_shelf_gain": 0.25,
            "transmission": {
                "gain": 0.6,
                "whine": 1.2,
                "clack": 1.1,
                "rattle": 0.4,
                "clutch": 0.5,
                "gear_teeth": 22.0,
                "final_teeth": 41.0,
                "reverse_ratio": 4.1
            }
        });
        let tuning = v10_layer_tuning_from_section(Some(&section)).unwrap();
        assert_eq!(tuning.cover_radiation_lowpass_hz, Some(1_000_000.0));
        assert_eq!(tuning.sample_blend_weight, Some(1.3));
        assert_eq!(tuning.physical_blend_weight, Some(0.77));
        assert_eq!(tuning.upper_mid_shelf_gain, 0.25);
        assert_eq!(tuning.transmission_gain, 0.6);
        assert_eq!(tuning.transmission_whine_gain, Some(1.2));
        assert_eq!(tuning.transmission_gear_teeth, Some(22.0));
        assert_eq!(tuning.transmission_final_teeth, Some(41.0));
        assert_eq!(tuning.transmission_reverse_ratio, Some(4.1));
        assert_eq!(
            tuning.zone_trim_db,
            Some(vec![2.0, 0.0, 0.0, 1.5, 1.5, 1.5])
        );
        assert!(tuning.scene_gains.contains(&("engine_air".to_string(), 0.841)));
    }

    #[test]
    fn engine_mode_and_sampled_gearbox_gain_are_transported() {
        let section = serde_json::json!({
            "engine_mode": "sample_only",
            "sampled_gearbox_gain": 0.4
        });
        let tuning = v10_layer_tuning_from_section(Some(&section)).unwrap();
        assert_eq!(
            tuning.engine_mode,
            v10_engine_synth::EngineMode::SampleOnly
        );
        assert_eq!(tuning.sampled_gearbox_gain, Some(0.4));

        let hybrid = serde_json::json!({ "engine_mode": "hybrid" });
        assert_eq!(
            v10_layer_tuning_from_section(Some(&hybrid))
                .unwrap()
                .engine_mode,
            v10_engine_synth::EngineMode::Hybrid
        );
        let unknown = serde_json::json!({ "engine_mode": "warp_drive" });
        assert!(v10_layer_tuning_from_section(Some(&unknown)).is_err());
    }

    #[test]
    fn spectrum_filter_blocks_are_transported() {
        let section = serde_json::json!({
            "v10_filter": {
                "highpass_hz": 97.0,
                "highpass_slope_db_per_oct": 24.0,
                "lowpass_hz": 13100.0,
                "lowpass_slope_db_per_oct": 72.0,
                "peak_hz": 10000.0,
                "peak_gain_db": 6.0,
                "peak_q": 3.0,
                "peak_drive": 2.0,
                "peak_drive_mix": 0.3
            },
            "gearbox_filter": {
                "highpass_hz": 515.0,
                "highpass_slope_db_per_oct": 72.0,
                "lowpass_hz": 3100.0,
                "lowpass_slope_db_per_oct": 72.0
            }
        });
        let tuning = v10_layer_tuning_from_section(Some(&section)).unwrap();
        assert_eq!(tuning.v10_filter.highpass_hz, 97.0);
        assert_eq!(tuning.v10_filter.highpass_slope_db_per_oct, 24.0);
        assert_eq!(tuning.v10_filter.lowpass_hz, 13_100.0);
        assert_eq!(tuning.v10_filter.lowpass_slope_db_per_oct, 72.0);
        assert_eq!(tuning.v10_filter.peak_hz, 10_000.0);
        assert_eq!(tuning.v10_filter.peak_gain_db, 6.0);
        assert_eq!(tuning.v10_filter.peak_q, 3.0);
        assert_eq!(tuning.v10_filter.peak_drive, 2.0);
        assert_eq!(tuning.v10_filter.peak_drive_mix, 0.3);
        assert_eq!(tuning.gearbox_filter.highpass_hz, 515.0);
        assert_eq!(tuning.gearbox_filter.highpass_slope_db_per_oct, 72.0);
        assert_eq!(tuning.gearbox_filter.lowpass_hz, 3_100.0);
        assert_eq!(tuning.gearbox_filter.lowpass_slope_db_per_oct, 72.0);
        assert_eq!(tuning.gearbox_filter.peak_gain_db, 0.0);

        let absent = v10_layer_tuning_from_section(Some(&serde_json::json!({}))).unwrap();
        assert!(absent.v10_filter.is_bypass());
        assert!(absent.gearbox_filter.is_bypass());

        let bad = serde_json::json!({ "v10_filter": { "lowpass_hz": -5.0 } });
        assert!(v10_layer_tuning_from_section(Some(&bad)).is_err());
    }

    #[test]
    fn powertrain_ratios_fill_transmission_tuning_unless_overridden() {
        let config = vehicle_physics_engine::VehicleConfig::default();
        let mut tuning = vehicle_audio_engine::V10LayerTuning::default();
        apply_powertrain_transmission(&mut tuning, &config);
        assert_eq!(
            tuning.transmission_gear_ratios.as_ref().map(Vec::len),
            Some(config.gear_ratios.len())
        );
        assert_eq!(
            tuning.transmission_final_drive,
            Some(config.final_drive as f32)
        );
        assert_eq!(
            tuning.transmission_reverse_ratio,
            Some(config.reverse_ratio as f32)
        );

        let mut declared = vehicle_audio_engine::V10LayerTuning {
            transmission_gear_ratios: Some(vec![9.0, 8.0]),
            ..vehicle_audio_engine::V10LayerTuning::default()
        };
        apply_powertrain_transmission(&mut declared, &config);
        assert_eq!(declared.transmission_gear_ratios, Some(vec![9.0, 8.0]));
    }
}
