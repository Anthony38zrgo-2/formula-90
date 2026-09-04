//! Real-time vehicle audio mixer — pure Rust, sample-accurate.
//!
//! This is the runtime core that the GDExtension C++ node drives each frame via
//! the `ffi` layer. It mirrors the offline oracle (`tools/audio/render_audio_scenario.py`)
//! and the GDScript `VehicleAudioController` semantics, but produces samples
//! directly instead of pushing per-stream `pitch_scale`/`volume_db` into Godot
//! players. Working in the sample domain (fractional cursor playback + a per-sample
//! limiter) removes the resampler discontinuities that caused "buffer glitches"
//! when `pitch_scale` was changed every frame on looping `AudioStreamPlayer`s.

use crate::bank::{BankError, VehicleSoundBank};
use crate::config::{ConfigLoadResult, ExhaustConfig, SoundConfig, SoundMixerConfig};
use crate::dsp::{
    adsr::Adsr, eq::GraphicEq, limiter::StereoLimiter, pan::equal_power, reverb::StereoReverb,
    tube::Tube,
};
use crate::dsp_contract::EventBuilder;
use crate::dsp_runtime::{DspControls, DspRuntime};
use crate::powertrain::{AudioPowertrainSynthesis, DistanceLevels};
use crate::state::*;
use crate::synth::LodLevel;
use crate::tire_scrub::{compute_tire_scrub_target, TireScrubMode};

use std::collections::BTreeMap;
use std::path::Path;

/// Mixing/voice configuration. Defaults mirror the F1-94 presentation layer
/// (`vehicle_audio_controller.gd`) so the Rust core is the source of truth.
#[derive(Clone, Copy)]
pub struct AudioConfig {
    pub pitch_min: f32,
    pub pitch_max: f32,
    pub coast_gain: f32,
    pub throttle_gain: f32,
    pub saturation: f32,
    pub limiter_threshold: f32,
    /// Engine mix headroom applied before the limiter. Mirrors the offline oracle's
    /// `engine * 0.62` factor so steady-state peaks stay off the limiter knee and
    /// band crossfades at RPM changes don't distort into perceived clipping.
    pub engine_headroom: f32,
    /// Gain smoothing time constants (seconds).
    pub attack_seconds: f32,
    pub release_seconds: f32,
    /// Per-one-shot output gain.
    pub shift_gain: f32,
    /// Surface bed gain coefficients (base + slip + speed).
    pub bed_base: f32,
    pub bed_slip: f32,
    pub bed_speed: f32,
    /// Continuous underfloor scrape voice gain.
    pub scrape_gain: f32,
    /// Exhaust microphone layer (behaviour-driven sample playback).
    pub exhaust: ExhaustConfig,
    /// Crossfade duration (ms) applied on a distance-level (LOD) change. Reuses
    /// the same `transition_*` mechanism as config hot-reload so switches between
    /// Near/Mid/Far/Virtual are click-free.
    pub lod_transition_ms: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            pitch_min: PITCH_MIN,
            pitch_max: PITCH_MAX,
            coast_gain: 0.45,
            throttle_gain: 0.50,
            saturation: 0.06,
            limiter_threshold: 0.90,
            engine_headroom: 0.62,
            attack_seconds: 0.035,
            release_seconds: 0.12,
            shift_gain: 0.80,
            bed_base: 0.25,
            bed_slip: 0.55,
            bed_speed: 0.12,
            scrape_gain: 0.62,
            exhaust: ExhaustConfig::default_runtime(),
            lod_transition_ms: LOD_TRANSITION_MS,
        }
    }
}

/// A one-shot voice (gear shifts, impacts) drawn from the bank.
struct OneShot {
    trigger: Trigger,
    key: String,
    cursor: usize,
    delay_remaining: usize,
    active: bool,
}

const SHIFT_COMPANION_DELAY_MS: usize = 20;

struct VoiceStrip {
    config: SoundConfig,
    envelope: Option<Adsr>,
    eq: GraphicEq,
    tube: Tube,
    bus_index: Option<usize>,
    // Constants per strip: pan-law gains and linear reverb send gain. Computed
    // once at build time so `process` avoids `cos/sin` (equal_power) and a
    // `10^powf` on every sample — the always-on strip cost that pushed the CPU
    // gate over budget on active layers.
    pan_l: f32,
    pan_r: f32,
    reverb_gain: f32,
}

impl VoiceStrip {
    fn new(config: SoundConfig, sample_rate: u32, bus_index: Option<usize>) -> Self {
        let positive_boosts = config
            .eq
            .bands_db
            .values()
            .copied()
            .filter(|gain| *gain > 0.0)
            .sum();
        let envelope = config.adsr.enabled.then(|| {
            let mut env = Adsr::new(config.adsr, sample_rate);
            env.note_on();
            env
        });
        let eq = GraphicEq::new(&config.eq, sample_rate);
        let tube = Tube::new(config.eq.tube_color, positive_boosts);
        let (pan_l, pan_r) = equal_power(config.pan);
        let reverb_gain = if config.reverb.enabled {
            10.0_f32.powf(config.reverb.send_db / 20.0)
        } else {
            0.0
        };
        Self {
            config,
            envelope,
            eq,
            tube,
            bus_index,
            pan_l,
            pan_r,
            reverb_gain,
        }
    }

    fn note_on(&mut self) {
        if let Some(envelope) = &mut self.envelope {
            envelope.note_on();
        }
    }
    fn note_off(&mut self) {
        if let Some(envelope) = &mut self.envelope {
            envelope.note_off();
        }
    }
    #[inline]
    fn process(&mut self, source: f32) -> (f32, f32, Option<(usize, f32, f32)>) {
        let envelope = self.envelope.as_mut().map_or(1.0, Adsr::next_sample);
        let mono = self.tube.process(self.eq.process(source * envelope)) * self.config.volume;
        let left = mono * self.pan_l;
        let right = mono * self.pan_r;
        let send = self
            .bus_index
            .filter(|_| self.config.reverb.enabled)
            .map(|index| (index, left * self.reverb_gain, right * self.reverb_gain));
        (left, right, send)
    }
}

struct ReverbBus {
    processor: StereoReverb,
    input_l: f32,
    input_r: f32,
}

#[inline]
fn mix_through_strip(
    strips: &mut BTreeMap<String, VoiceStrip>,
    buses: &mut [ReverbBus],
    key: &str,
    source: f32,
    left: &mut f32,
    right: &mut f32,
) {
    // A zero source is bit-identical through the strip pipeline (eq(0)=0,
    // tube(0)=0, send=(0,0)) so skip the BTreeMap lookup and the full
    // ADSR->EQ->Tube pipeline entirely. Silent non-looping layers are the
    // norm (bed/scrape/tyre are often muted), and this was the always-on
    // single-core cost that pushed the CPU gate over budget.
    if source == 0.0 {
        return;
    }
    if let Some(strip) = strips.get_mut(key) {
        let (out_l, out_r, send) = strip.process(source);
        *left += out_l;
        *right += out_r;
        if let Some((index, send_l, send_r)) = send {
            if let Some(bus) = buses.get_mut(index) {
                bus.input_l += send_l;
                bus.input_r += send_r;
            }
        }
    } else {
        *left += source;
        *right += source;
    }
}

/// Short fade-in/out (samples) applied to one-shots to avoid click on start/end.
const ONE_SHOT_ENV_SAMPLES: usize = 256;

/// RPM glide time constant (seconds). Smooths the playback rate + band weights so
/// RPM changes glide continuously instead of stepping each frame (which produced
/// zipper/warble artifacts at transitions). One-pole; 25 ms is inaudible lag but
/// removes the per-frame discontinuity.
const RPM_SMOOTH_TAU: f64 = 0.025;

/// Crossfade duration (ms) on a distance-level (LOD) change. Defaults to the
/// same 30 ms hot-reload crossfade so LOD switches are inaudible.
const LOD_TRANSITION_MS: u32 = 30;

/// Hard cap on the per-render-block size the C++ DSP can process in one call
/// (`F90_DSP_MAX_BLOCK_SAMPLES`). The runtime render callback chunks at this
/// bound; the Fase-4 harness uses 512, comfortably within it.
const MAX_CPP_BLOCK: usize = 4096;

#[inline]
fn scale_cpp_layer(sample: f32, gain: f32) -> f32 {
    sample * gain.max(0.0)
}

#[inline]
fn continuous_output_gain(
    source: ContinuousSourceKind,
    smoothed_engine_gain: f32,
    engine_headroom: f32,
    synth_volume: f32,
) -> f32 {
    let source_gain = match source {
        // GF509 already models throttle and authoritative engine load internally.
        // Applying the legacy pedal gain here attenuates coast a second time.
        ContinuousSourceKind::V10Gf509 => 1.0,
        ContinuousSourceKind::Legacy => smoothed_engine_gain,
    };
    source_gain * engine_headroom * synth_volume
}

/// Stateful engine audio mixer.
/// Real-time audio DSP must never stall on subnormal (denormal) floats. On x86
/// a subnormal operand or result costs ~100+ cycles; after a loud passage the
/// reverb/feedback filters decay into subnormal territory and the mixer slows
/// catastrophically. Set the MXCSR FTZ (flush-to-zero, bit 15) and DAZ
/// (denormals-are-zero, bit 6) flags once per render thread so subnormals are
/// treated as zero (inaudible below ~1e-38). Other platforms are no-ops.
#[cfg(target_arch = "x86_64")]
fn enable_fast_floats() {
    use std::cell::Cell;
    thread_local! {
        static SET: Cell<bool> = const { Cell::new(false) };
    }
    SET.with(|s| {
        if s.get() {
            return;
        }
        let mut mxcsr: u32 = 0;
        unsafe {
            core::arch::asm!(
                "stmxcsr [{0}]",
                in(reg) &mut mxcsr,
                options(nostack, preserves_flags),
            );
        }
        mxcsr |= 0x8000 | 0x0040;
        unsafe {
            core::arch::asm!(
                "ldmxcsr [{0}]",
                in(reg) &mxcsr,
                options(nostack, preserves_flags),
            );
        }
        s.set(true);
    });
}

#[cfg(not(target_arch = "x86_64"))]
fn enable_fast_floats() {}

pub struct VehicleAudioEngine {
    bank: VehicleSoundBank,
    engine_bands: Vec<EngineBandProfile>,
    layer_keys: Vec<String>,
    layer_cursors: Vec<f64>,
    bed_cursor: f64,
    bed_key: Option<String>,

    // Smoothed gains (per-sample attack/release toward target).
    smoothed_engine_gain: f32,
    target_engine_gain: f32,
    smoothed_bed_gain: f32,
    target_bed_gain: f32,
    smoothed_scrape_gain: f32,
    target_scrape_gain: f32,
    scrape_pitch: f64,
    scrape_cursor: f64,
    smoothed_tyre_scrub_gain: f32,
    target_tyre_scrub_gain: f32,
    tyre_scrub_pitch: f64,
    tyre_scrub_cursor: f64,
    tyre_scrub_mode: TireScrubMode,

    // Smoothed RPM (one-pole glide) so pitch + band weights move continuously
    // instead of stepping each frame (which caused zipper/click at RPM changes).
    smoothed_rpm: f64,
    target_rpm: f64,
    idle_rpm: f64,
    max_rpm: f64,

    // Current mix (for telemetry / FFI snapshot).
    cur_weights: [f32; 5],
    cur_pitches: [f32; 5],
    cur_engine_gain: f32,

    one_shots: Vec<OneShot>,
    sample_rate: u32,
    cfg: AudioConfig,

    // Telemetry snapshot (raw inputs last seen).
    last_norm: f32,
    last_rpm: f64,
    last_throttle: f32,
    last_speed_kph: f64,
    last_slip: f32,
    last_gear: i32,
    last_trigger: String,
    last_normalized_engine_load: f32,
    last_normalized_engine_torque: f32,
    last_torque_sign: i32,
    last_rpm_derivative: f32,
    last_throttle_derivative: f32,
    last_shift_phase: i32,
    last_clutch_engagement: f32,
    last_tc_cut_ratio: f32,
    last_rev_limiter_active: bool,
    backfire_cooldown_samples: usize,
    variant_rng_state: u64,

    /// Per-sample linear gain ceiling (0..1) from `sound_mixer_config.json`,
    /// keyed by bank key. Absent keys play at full level (1.0).
    per_sample_gain: BTreeMap<String, f32>,
    strips: BTreeMap<String, VoiceStrip>,
    reverb_buses: Vec<ReverbBus>,
    stereo_limiter: StereoLimiter,
    master_gain: f32,
    config_generation: u64,
    config_source_hash: String,
    transition_remaining: usize,
    transition_total: usize,
    transition_start_l: f32,
    transition_start_r: f32,
    last_output_l: f32,
    last_output_r: f32,

    // Exhaust microphone layer (behaviour-driven sample playback).
    exhaust_key: String,
    exhaust_cursor: f64,
    exhaust_crackle_samples: usize,

    // Half-block V10 synthesizer (default OFF; replaces the sampled engine
    // bands + exhaust microphone layer when enabled).
    synth: Option<crate::synth::HalfBlock>,
    synth_enabled: bool,
    synth_volume: f32,
    last_synth_energy: f32,
    // Used only when the C++ layer is active: the Rust body is rendered first
    // so its real CylState events can be packetized before the DLL processes
    // the same block. Allocated once during construction.
    synth_block_l: Vec<f32>,
    synth_block_r: Vec<f32>,
    gf509: Option<v10_engine_synth::Gf509Runtime>,
    continuous_source: ContinuousSourceKind,
    gf509_block_l: Vec<f32>,
    gf509_block_r: Vec<f32>,
    gf509_render_failed: bool,
    diagnostics: ContinuousDiagnostics,
    diagnostic_mode: DiagnosticMode,

    // Camera-to-vehicle listener distance (metres), forwarded to the controller
    // and smoothed upstream. Not baked into the synth (see Commit 6 spec).
    listener_distance: f32,
    // Current distance level (LOD) of the procedural engine, and the distance
    // thresholds/hysteresis that drive it (derived from the powertrain contract).
    lod: LodLevel,
    lod_levels: DistanceLevels,

    // Fase 4 — optional C++ post-combustion DSP layer. It is an *added
    // enhancement* to the approved `combustion_body`, never a replacement.
    // `cpp_layer_gain` defaults to 0.0 and the layer is disabled by default, so
    // the existing path is byte-invariant until explicitly enabled (4.3).
    cpp_dsp: Option<DspRuntime>,
    cpp_event_builder: Option<EventBuilder>,
    cpp_layer_enabled: bool,
    cpp_layer_gain: f32,
    // Preallocated per-block C++ output buffers (no allocation in the callback).
    cpp_out_l: Vec<f32>,
    cpp_out_r: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContinuousSourceKind {
    Legacy,
    V10Gf509,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticMode {
    Mix,
    V10Only,
    EventsOnly,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ContinuousDiagnostics {
    pub source: u8,
    pub received_rpm: f32,
    pub rendered_rpm: f32,
    pub peak_pre_protection: f32,
    pub protection_reduction_db: f32,
    pub last_render_ns: u64,
    pub worst_render_ns: u64,
    pub blocks: u64,
    pub underruns: u64,
    pub asset_or_render_errors: u64,
}

impl VehicleAudioEngine {
    /// Load the bank and build the mixer. `bank_dir` must contain `bank_manifest.json`.
    pub fn new(bank_dir: &Path) -> Result<Self, BankError> {
        let bank = VehicleSoundBank::load(bank_dir)?;
        let engine_bands = bank.engine_bands.clone();
        let layer_keys: Vec<String> = engine_bands.iter().map(|band| band.key.clone()).collect();
        let layer_cursors = vec![0.0f64; layer_keys.len()];

        let mut one_shots: Vec<OneShot> = Vec::new();
        for t in [
            Trigger::ShiftUp,
            Trigger::ShiftDown,
            Trigger::Hit1,
            Trigger::Hit2,
            Trigger::Hit3,
            Trigger::Hit4,
            Trigger::Barrier,
            Trigger::Cone,
            Trigger::Fire,
            Trigger::Scrape,
        ] {
            let key = t.bank_key().to_string();
            if bank.get(&key).is_some() {
                one_shots.push(OneShot {
                    trigger: t,
                    key,
                    cursor: 0,
                    delay_remaining: 0,
                    active: false,
                });
            }
        }
        // Exterior gear samples layer with, rather than replace, the existing
        // shift voices. Their delay is applied in the sample domain below.
        for (trigger, key) in [
            (Trigger::ShiftUp, "shift_up_delayed"),
            (Trigger::ShiftDown, "shift_down_delayed"),
        ] {
            if bank.get(key).is_some() {
                one_shots.push(OneShot {
                    trigger,
                    key: key.to_string(),
                    cursor: 0,
                    delay_remaining: 0,
                    active: false,
                });
            }
        }
        // A role may expose multiple samples. Backfire currently has two internal
        // variants; keep their BTreeMap order stable for deterministic replays.
        for sample in bank
            .samples
            .values()
            .filter(|s| s.role == "engine_backfire")
        {
            one_shots.push(OneShot {
                trigger: Trigger::Backfire,
                key: sample.key.clone(),
                cursor: 0,
                delay_remaining: 0,
                active: false,
            });
        }

        let sample_rate = bank
            .samples
            .values()
            .next()
            .map(|s| s.sample_rate)
            .unwrap_or(44100);

        // Per-sample gain ceiling + exhaust behaviour config from
        // <bank_dir>/../../sound_mixer_config.json (see `SoundMixerConfig`).
        // Missing/invalid file degrades to defaults — never an error.
        let bank_keys = bank.keys();
        let load_result =
            crate::config::SoundMixerConfig::load_result_from_bank_dir(bank_dir, &bank_keys);
        let config_source_hash = load_result.source_hash.clone();
        let mixer_cfg = load_result.resolved;
        let mut per_sample_gain = match &mixer_cfg {
            SoundMixerConfig::V1(_) => mixer_cfg.gains_map(),
            SoundMixerConfig::V2(_) => BTreeMap::new(),
        };
        let cfg = AudioConfig {
            exhaust: mixer_cfg.exhaust_config(),
            ..AudioConfig::default()
        };
        let (resolved_sounds, bus_configs, master) = match &mixer_cfg {
            SoundMixerConfig::V1(_) => (
                bank_keys
                    .iter()
                    .map(|key| {
                        let config = SoundConfig {
                            volume: std::f32::consts::SQRT_2,
                            ..SoundConfig::default()
                        };
                        (key.clone(), config)
                    })
                    .collect(),
                BTreeMap::new(),
                crate::config::MasterConfig::default(),
            ),
            SoundMixerConfig::V2(config) => {
                let mut sounds = config.sounds.clone();
                if let Some(exhaust) = sounds.get_mut("exhaust-mic") {
                    per_sample_gain.insert("exhaust-mic".into(), exhaust.volume);
                    exhaust.volume = 1.0;
                }
                (sounds, config.reverb_buses.clone(), config.master)
            }
        };
        let bus_indices: BTreeMap<String, usize> = bus_configs
            .keys()
            .enumerate()
            .map(|(index, name)| (name.clone(), index))
            .collect();
        let reverb_buses = bus_configs
            .values()
            .map(|config| ReverbBus {
                processor: StereoReverb::new(*config, sample_rate),
                input_l: 0.0,
                input_r: 0.0,
            })
            .collect();
        let strips = resolved_sounds
            .into_iter()
            .map(|(key, config)| {
                let bus_index = bus_indices.get(&config.reverb.bus).copied();
                (key, VoiceStrip::new(config, sample_rate, bus_index))
            })
            .collect();
        let stereo_limiter = StereoLimiter::new(master.limiter_threshold);
        let master_gain = 10.0_f32.powf(master.output_db / 20.0);

        let exhaust_key = if bank.get("exhaust-mic").is_some() {
            "exhaust-mic".to_string()
        } else {
            String::new()
        };

        Ok(Self {
            bank,
            engine_bands,
            layer_keys,
            layer_cursors,
            bed_cursor: 0.0,
            bed_key: None,
            smoothed_engine_gain: 0.0,
            target_engine_gain: 0.0,
            smoothed_bed_gain: 0.0,
            target_bed_gain: 0.0,
            smoothed_scrape_gain: 0.0,
            target_scrape_gain: 0.0,
            scrape_pitch: 1.0,
            scrape_cursor: 0.0,
            smoothed_tyre_scrub_gain: 0.0,
            target_tyre_scrub_gain: 0.0,
            tyre_scrub_pitch: 1.0,
            tyre_scrub_cursor: 0.0,
            tyre_scrub_mode: TireScrubMode::None,
            cur_weights: [0.0; 5],
            cur_pitches: [1.0; 5],
            cur_engine_gain: 0.0,
            one_shots,
            sample_rate,
            cfg,
            last_norm: 0.0,
            last_rpm: 0.0,
            last_throttle: 0.0,
            last_speed_kph: 0.0,
            last_slip: 0.0,
            last_gear: 0,
            last_trigger: String::new(),
            last_normalized_engine_load: 0.0,
            last_normalized_engine_torque: 0.0,
            last_torque_sign: 0,
            last_rpm_derivative: 0.0,
            last_throttle_derivative: 0.0,
            last_shift_phase: 0,
            last_clutch_engagement: 0.0,
            last_tc_cut_ratio: 0.0,
            last_rev_limiter_active: false,
            smoothed_rpm: 0.0,
            target_rpm: 0.0,
            idle_rpm: 0.0,
            max_rpm: 0.0,
            backfire_cooldown_samples: 0,
            variant_rng_state: 0xF090_1994_D15C_A11D,
            per_sample_gain,
            strips,
            reverb_buses,
            stereo_limiter,
            master_gain,
            config_generation: 1,
            config_source_hash,
            transition_remaining: 0,
            transition_total: 0,
            transition_start_l: 0.0,
            transition_start_r: 0.0,
            last_output_l: 0.0,
            last_output_r: 0.0,
            exhaust_key,
            exhaust_cursor: 0.0,
            exhaust_crackle_samples: 0,
            synth: None,
            synth_enabled: false,
            synth_volume: 1.0,
            last_synth_energy: 0.0,
            synth_block_l: vec![0.0; MAX_CPP_BLOCK],
            synth_block_r: vec![0.0; MAX_CPP_BLOCK],
            gf509: None,
            continuous_source: ContinuousSourceKind::Legacy,
            gf509_block_l: vec![0.0; MAX_CPP_BLOCK],
            gf509_block_r: vec![0.0; MAX_CPP_BLOCK],
            gf509_render_failed: false,
            diagnostics: ContinuousDiagnostics::default(),
            diagnostic_mode: DiagnosticMode::Mix,
            listener_distance: 0.0,
            lod: LodLevel::Near,
            lod_levels: DistanceLevels::default(),
            cpp_dsp: None,
            cpp_event_builder: None,
            cpp_layer_enabled: false,
            cpp_layer_gain: 0.0,
            cpp_out_l: vec![0.0f32; MAX_CPP_BLOCK],
            cpp_out_r: vec![0.0f32; MAX_CPP_BLOCK],
        })
    }

    /// Per-sample linear gain ceiling; 1.0 (full level) when not configured.
    #[inline]
    fn sample_gain(&self, key: &str) -> f32 {
        self.per_sample_gain.get(key).copied().unwrap_or(1.0)
    }

    pub fn config(&self) -> &AudioConfig {
        &self.cfg
    }

    pub fn set_config(&mut self, cfg: AudioConfig) {
        self.cfg = cfg;
    }

    /// Parse and prepare a complete configuration on the owner thread. Call
    /// only between render blocks; invalid JSON leaves the active snapshot intact.
    pub fn apply_config_json(&mut self, raw: &str) -> ConfigLoadResult {
        let bank_keys = self.bank.keys();
        let result = crate::config::resolve_config_json(raw, &bank_keys);
        if !result.is_valid() || result.source_hash == self.config_source_hash {
            return result;
        }
        let SoundMixerConfig::V2(config) = &result.resolved else {
            self.per_sample_gain = result.resolved.gains_map();
            self.config_source_hash = result.source_hash.clone();
            self.config_generation = self.config_generation.saturating_add(1);
            return result;
        };
        let bus_indices: BTreeMap<String, usize> = config
            .reverb_buses
            .keys()
            .enumerate()
            .map(|(index, name)| (name.clone(), index))
            .collect();
        self.reverb_buses = config
            .reverb_buses
            .values()
            .map(|bus| ReverbBus {
                processor: StereoReverb::new(*bus, self.sample_rate),
                input_l: 0.0,
                input_r: 0.0,
            })
            .collect();
        let mut sounds = config.sounds.clone();
        self.per_sample_gain.clear();
        if let Some(exhaust) = sounds.get_mut("exhaust-mic") {
            self.per_sample_gain
                .insert("exhaust-mic".into(), exhaust.volume);
            exhaust.volume = 1.0;
        }
        self.strips = sounds
            .iter()
            .map(|(key, sound)| {
                let bus = bus_indices.get(&sound.reverb.bus).copied();
                (
                    key.clone(),
                    VoiceStrip::new(sound.clone(), self.sample_rate, bus),
                )
            })
            .collect();
        self.master_gain = 10.0_f32.powf(config.master.output_db / 20.0);
        self.stereo_limiter = StereoLimiter::new(config.master.limiter_threshold);
        self.cfg.exhaust = config.exhaust.to_runtime();
        self.transition_total =
            ((config.hot_reload.transition_ms as u64 * self.sample_rate as u64) / 1000) as usize;
        self.transition_remaining = self.transition_total;
        self.transition_start_l = self.last_output_l;
        self.transition_start_r = self.last_output_r;
        self.config_source_hash = result.source_hash.clone();
        self.config_generation = self.config_generation.saturating_add(1);
        result
    }

    pub fn config_generation(&self) -> u64 {
        self.config_generation
    }
    pub fn config_source_hash(&self) -> &str {
        &self.config_source_hash
    }

    /// Drive the sustained underfloor scrape voice. `onset_strength > 0` fires the
    /// transient once; the continuous voice keeps its cursor until a new contact.
    pub fn set_scrape_state(
        &mut self,
        active: bool,
        intensity: f32,
        speed_m_s: f32,
        onset_strength: f32,
    ) {
        let was_inactive = self.target_scrape_gain <= 1e-4 && self.smoothed_scrape_gain <= 1e-3;
        self.target_scrape_gain = if active {
            intensity.clamp(0.0, 1.0) * self.cfg.scrape_gain
        } else {
            0.0
        };
        self.scrape_pitch = (0.85 + (speed_m_s.abs() / 70.0) as f64 * 0.30).clamp(0.85, 1.15);
        if active && onset_strength > 0.0 {
            self.trigger(Trigger::Scrape);
        }
        if active && was_inactive {
            self.scrape_cursor = 0.0;
        }
    }

    /// Feed full versioned vehicle audio telemetry.
    pub fn set_telemetry(&mut self, telem: &crate::ffi::VehicleAudioTelemetryV3, surface: &str) {
        let norm = if telem.max_rpm > telem.idle_rpm {
            (((telem.rpm - telem.idle_rpm) / (telem.max_rpm - telem.idle_rpm)) as f32)
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        let egain = engine_gain(telem.throttle);
        let skey = surface_key(surface);
        let sgain = match skey {
            Some(_) => {
                self.cfg.bed_base
                    + self.cfg.bed_slip * telem.slip.clamp(0.0, 1.0)
                    + self.cfg.bed_speed * (telem.speed_kph.min(120.0) / 120.0) as f32
            }
            None => 0.0,
        };

        // RPM is stored as a target; `render` glides `smoothed_rpm` toward it per
        // sample so pitch + band weights move continuously (no per-frame step).
        self.target_rpm = telem.rpm;
        self.idle_rpm = telem.idle_rpm;
        self.max_rpm = telem.max_rpm;
        self.cur_engine_gain = egain;

        self.target_engine_gain = egain;
        self.target_bed_gain = sgain;
        self.bed_key = skey.map(|s| s.to_string());

        if telem.gear != self.last_gear {
            if self.last_gear != 0 {
                let t = if telem.gear > self.last_gear {
                    Trigger::ShiftUp
                } else {
                    Trigger::ShiftDown
                };
                self.trigger(t);
            }
            self.last_gear = telem.gear;
        }

        // Over-run backfire: sudden lift-off from high throttle at high RPM
        // (> 13,500 RPM). The 1 s cooldown stops throttle pumps / rev-limiter
        // bouncing from chaining full backfire samples back-to-back.
        if self.last_throttle >= 0.80
            && telem.throttle <= 0.15
            && telem.rpm >= 13500.0
            && self.backfire_cooldown_samples == 0
        {
            self.trigger(Trigger::Backfire);
            self.backfire_cooldown_samples = (self.sample_rate as f64 * 1.0) as usize;
            // Exhaust pop: short gain boost on the exhaust layer when a backfire
            // fires. Duration scales with sample rate (~120 ms).
            self.exhaust_crackle_samples = (self.sample_rate as f64 * 0.12) as usize;
        }

        if self.synth_enabled {
            if let Some(synth) = &mut self.synth {
                synth.update_controls(telem.rpm, telem.idle_rpm, telem.max_rpm, telem.throttle);
            }
        }
        if let Some(gf509) = &mut self.gf509 {
            let torque_sign = v10_engine_synth::TorqueSign::from_i32(telem.torque_sign)
                .unwrap_or(v10_engine_synth::TorqueSign::Neutral);
            let shift_phase = v10_engine_synth::ShiftPhase::from_i32(telem.shift_phase)
                .unwrap_or(v10_engine_synth::ShiftPhase::None);
            let _ = gf509.update_telemetry(v10_engine_synth::RuntimeTelemetry {
                rpm: telem.rpm.clamp(0.0, 25_000.0) as f32,
                throttle: telem.throttle.clamp(0.0, 1.0),
                normalized_engine_load: telem.normalized_engine_load.clamp(0.0, 1.0),
                normalized_engine_torque: telem.normalized_engine_torque.clamp(-1.0, 1.0),
                torque_sign,
                rpm_derivative: telem.rpm_derivative,
                throttle_derivative: telem.throttle_derivative,
                gear: telem.gear.clamp(-1, 12) as i8,
                shift_phase,
                dt_seconds: 0.0,
            });
        }

        self.last_norm = norm;
        self.last_rpm = telem.rpm;
        self.last_throttle = telem.throttle;
        self.last_speed_kph = telem.speed_kph;
        self.last_slip = telem.slip;
        self.last_normalized_engine_load = telem.normalized_engine_load;
        self.last_normalized_engine_torque = telem.normalized_engine_torque;
        self.last_torque_sign = telem.torque_sign;
        self.last_rpm_derivative = telem.rpm_derivative;
        self.last_throttle_derivative = telem.throttle_derivative;
        self.last_shift_phase = telem.shift_phase;
        self.last_clutch_engagement = telem.clutch_engagement;
        self.last_tc_cut_ratio = telem.tc_cut_ratio;
        self.last_rev_limiter_active = telem.rev_limiter_active != 0;
    }

    /// Feed the current vehicle telemetry (legacy shim).
    #[allow(clippy::too_many_arguments)]
    pub fn set_state(
        &mut self,
        rpm: f64,
        idle_rpm: f64,
        max_rpm: f64,
        throttle: f32,
        speed_kph: f64,
        gear: i32,
        slip: f32,
        surface: &str,
    ) {
        let telem = crate::ffi::VehicleAudioTelemetryV3 {
            schema_version: crate::ffi::VEHICLE_AUDIO_ABI_VERSION,
            struct_size: std::mem::size_of::<crate::ffi::VehicleAudioTelemetryV3>() as u32,
            rpm,
            idle_rpm,
            max_rpm,
            throttle,
            normalized_engine_load: throttle.clamp(0.0, 1.0),
            normalized_engine_torque: 0.0,
            rpm_derivative: 0.0,
            throttle_derivative: 0.0,
            speed_kph,
            slip,
            gear,
            torque_sign: 0,
            shift_phase: 0,
            clutch_engagement: 1.0,
            tc_cut_ratio: 0.0,
            rev_limiter_active: 0,
        };
        self.set_telemetry(&telem, surface);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_tire_scrub_state(
        &mut self,
        slip_ratio: [f32; 4],
        slip_angle_rad: [f32; 4],
        contact_fraction: [f32; 4],
        normal_force_n: [f32; 4],
        speed_kph: f32,
        surface: &str,
    ) {
        let was_inactive = self.target_tyre_scrub_gain <= 1e-4;
        let target = compute_tire_scrub_target(
            slip_ratio,
            slip_angle_rad,
            contact_fraction,
            normal_force_n,
            speed_kph,
            surface,
        );
        self.target_tyre_scrub_gain = target.gain;
        self.tyre_scrub_pitch = target.pitch as f64;
        self.tyre_scrub_mode = target.mode;
        if was_inactive && target.gain > 1e-4 {
            self.tyre_scrub_cursor = 0.0;
        }
    }

    /// Fire one deterministic pseudo-random variant for the requested one-shot.
    pub fn trigger(&mut self, t: Trigger) {
        self.last_trigger = t.bank_key().to_string();
        let variant_count = self
            .one_shots
            .iter()
            .filter(|o| o.trigger == t && !o.key.ends_with("_delayed"))
            .count();
        if variant_count == 0 {
            return;
        }
        self.variant_rng_state = self
            .variant_rng_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let selected = ((self.variant_rng_state >> 32) as usize) % variant_count;
        let mut ordinal = 0usize;
        for o in self.one_shots.iter_mut() {
            if o.trigger == t {
                let is_companion = o.key.ends_with("_delayed");
                o.active = is_companion || ordinal == selected;
                if o.active {
                    o.cursor = 0;
                    o.delay_remaining = if is_companion {
                        self.sample_rate as usize * SHIFT_COMPANION_DELAY_MS / 1000
                    } else {
                        0
                    };
                    if let Some(strip) = self.strips.get_mut(&o.key) {
                        strip.note_on();
                    }
                }
                if !is_companion {
                    ordinal += 1;
                }
            }
        }
    }

    /// Render `n` stereo frames into `out_l`/`out_r`. Sample-accurate: engine
    /// layers and bed are read with a fractional cursor (no Godot resampler),
    /// gains are smoothed per-sample, and a tanh soft-clip limiter catches peaks.
    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32], n: usize) {
        let render_started = std::time::Instant::now();
        let mut peak_pre_protection = 0.0f32;
        enable_fast_floats();
        self.backfire_cooldown_samples = self.backfire_cooldown_samples.saturating_sub(n);
        let sr = self.sample_rate as f64;
        let rpm_alpha = 1.0 - (-1.0 / (sr * RPM_SMOOTH_TAU)).exp();
        // All smoothing time constants are invariant for the whole render
        // block. Derive their coefficients once, outside the sample hot path.
        let eg_attack_alpha = 1.0 - (-1.0 / (sr * self.cfg.attack_seconds as f64)).exp();
        let eg_release_alpha = 1.0 - (-1.0 / (sr * self.cfg.release_seconds as f64)).exp();
        let bg_attack_alpha = eg_attack_alpha;
        let bg_release_alpha = eg_release_alpha;
        let scrape_attack_alpha = 1.0 - (-1.0 / (sr * 0.020)).exp();
        let scrape_release_alpha = 1.0 - (-1.0 / (sr * 0.150)).exp();
        let tyre_attack_alpha = 1.0 - (-1.0 / (sr * 0.025)).exp();
        let tyre_release_alpha = 1.0 - (-1.0 / (sr * 0.160)).exp();

        // Fase 4 — C++ post-combustion enhancement layer. Disabled by default
        // (gain 0); it never alters `combustion_body`. The Rust `EventBuilder` is
        // the temporal authority: it produces the same `F90DspEventBlock` the C++
        // consumes, so the cpp impulse coincides with `rust_event_impulse` timing.
        let cpp_active = self.cpp_layer_enabled
            && self.cpp_dsp.is_some()
            && self.cpp_event_builder.is_some()
            && n <= MAX_CPP_BLOCK;
        let cpp_gain = if cpp_active { self.cpp_layer_gain } else { 0.0 };
        let mut synth_prerendered = false;
        let gf509_active = self.continuous_source == ContinuousSourceKind::V10Gf509
            && self.gf509.is_some()
            && n <= MAX_CPP_BLOCK;
        self.gf509_render_failed = false;
        if gf509_active {
            if let Some(gf509) = &mut self.gf509 {
                if gf509
                    .render_block(&mut self.gf509_block_l[..n], &mut self.gf509_block_r[..n])
                    .is_err()
                {
                    self.gf509_render_failed = true;
                    self.gf509_block_l[..n].fill(0.0);
                    self.gf509_block_r[..n].fill(0.0);
                }
            }
        }
        if cpp_active {
            let block_rpm = self.smoothed_rpm as f32;
            let mut ok = false;
            if let (Some(dsp), Some(builder), Some(synth)) = (
                self.cpp_dsp.as_mut(),
                self.cpp_event_builder.as_mut(),
                self.synth.as_mut(),
            ) {
                builder.begin_block(n as u32);
                if self.synth_enabled && self.lod != LodLevel::Virtual {
                    for i in 0..n {
                        let (l, r) = synth.render_stereo_with_events(builder, i as u32);
                        self.synth_block_l[i] = l;
                        self.synth_block_r[i] = r;
                    }
                    synth_prerendered = true;
                }
                // The event derivative already carries physical event amplitude.
                // Feed the C++ layer the synth's smoothed mechanical energy rather
                // than raw pedal position so coast/idle retain a continuous timbre
                // and load is not squared by a second throttle multiplier.
                let acoustic_load = synth.energy().clamp(0.0, 1.0);
                let block = builder.finish_block();
                let ctl = DspControls {
                    rpm: block_rpm,
                    throttle: self.last_throttle,
                    load: acoustic_load,
                    tc_cut: 0.0,
                    // Rust owns the layer gain at the final mix point. C++ runs
                    // at unity so the response is gain, never gain squared.
                    master_gain: 1.0,
                    lod: 0,
                    bypass: false,
                };
                if dsp.process_block(&block, &ctl).is_ok() {
                    let m = dsp.left().len().min(n);
                    self.cpp_out_l[..m].copy_from_slice(&dsp.left()[..m]);
                    self.cpp_out_r[..m].copy_from_slice(&dsp.right()[..m]);
                    ok = true;
                }
            }
            if !ok {
                for v in &mut self.cpp_out_l[..n] {
                    *v = 0.0;
                }
                for v in &mut self.cpp_out_r[..n] {
                    *v = 0.0;
                }
            }
        }

        for i in 0..n {
            let mut mixed_l = 0.0f32;
            let mut mixed_r = 0.0f32;
            for bus in &mut self.reverb_buses {
                bus.input_l = 0.0;
                bus.input_r = 0.0;
            }
            // Smooth RPM so pitch + band weights glide continuously (no per-frame
            // step -> no zipper/warble at RPM changes).
            self.smoothed_rpm += (self.target_rpm - self.smoothed_rpm) * rpm_alpha;

            // Per-sample gain smoothing (attack/release time constants). The
            // one-pole alpha depends only on the constant sample rate and the
            // fixed attack/release taus, so both directions are precomputed once
            // per block instead of calling `exp` four times per sample. The
            // four-per-sample `exp` (2048/block) dominated the always-on single-core
            // budget, so hoisting is bit-identical but removes it from the loop.
            let eg_alpha = if self.target_engine_gain > self.smoothed_engine_gain {
                eg_attack_alpha
            } else {
                eg_release_alpha
            };
            let eg = self.smoothed_engine_gain
                + (self.target_engine_gain - self.smoothed_engine_gain) * eg_alpha as f32;
            self.smoothed_engine_gain = eg;

            let bg_alpha = if self.target_bed_gain > self.smoothed_bed_gain {
                bg_attack_alpha
            } else {
                bg_release_alpha
            };
            let bg = self.smoothed_bed_gain
                + (self.target_bed_gain - self.smoothed_bed_gain) * bg_alpha as f32;
            self.smoothed_bed_gain = bg;

            let scrape_alpha = if self.target_scrape_gain > self.smoothed_scrape_gain {
                scrape_attack_alpha
            } else {
                scrape_release_alpha
            };
            self.smoothed_scrape_gain +=
                (self.target_scrape_gain - self.smoothed_scrape_gain) * scrape_alpha as f32;

            let tyre_alpha = if self.target_tyre_scrub_gain > self.smoothed_tyre_scrub_gain {
                tyre_attack_alpha
            } else {
                tyre_release_alpha
            };
            self.smoothed_tyre_scrub_gain +=
                (self.target_tyre_scrub_gain - self.smoothed_tyre_scrub_gain) * tyre_alpha as f32;

            if self.diagnostic_mode != DiagnosticMode::EventsOnly
                && self.continuous_source == ContinuousSourceKind::V10Gf509
            {
                let gain = continuous_output_gain(
                    self.continuous_source,
                    eg,
                    self.cfg.engine_headroom,
                    self.synth_volume,
                );
                mixed_l += self.gf509_block_l.get(i).copied().unwrap_or(0.0) * gain;
                mixed_r += self.gf509_block_r.get(i).copied().unwrap_or(0.0) * gain;
                self.cur_weights.iter_mut().for_each(|weight| *weight = 0.0);
                self.cur_weights[0] = 1.0;
                self.cur_pitches[0] = (self.smoothed_rpm * 5.0 / 120.0) as f32;
            } else if self.diagnostic_mode != DiagnosticMode::EventsOnly && self.synth_enabled {
                let (synth_l, synth_r) = if self.lod == LodLevel::Virtual {
                    // Virtual: no audible frames. Only advance the mechanical phase
                    // accumulator ring (cheap, keeps re-entry phase-coherent).
                    if let Some(synth) = &mut self.synth {
                        synth.update_phase();
                    }
                    (0.0, 0.0)
                } else {
                    if synth_prerendered {
                        (self.synth_block_l[i], self.synth_block_r[i])
                    } else {
                        self.synth
                            .as_mut()
                            .map_or((0.0, 0.0), |s| s.render_stereo())
                    }
                };
                let gain = continuous_output_gain(
                    self.continuous_source,
                    eg,
                    self.cfg.engine_headroom,
                    self.synth_volume,
                );
                // The procedural engine is strictly dual-mono (synth_l ==
                // synth_r bit for bit). No pan law is applied here: an
                // equal-power centre would shave ~3.01 dB off each channel and
                // reintroduce a channel difference, so the mono signal is summed
                // at unity.
                mixed_l += synth_l * gain;
                mixed_r += synth_r * gain;
                self.last_synth_energy = self.synth.as_ref().map_or(0.0, |s| s.energy());
                self.cur_weights.iter_mut().for_each(|w| *w = 0.0);
                self.cur_weights[0] = 1.0;
                self.cur_pitches[0] = (self.smoothed_rpm * 5.0 / 120.0) as f32;
            } else {
                // The sampled engine bands and exhaust microphone layer are retired
                // (see bank::RETIRED_KEYS); the procedural synth (above) is the engine
                // source. Keep the band telemetry (weights/pitches) so consumers still
                // see a glide, but emit no sampled engine/exhaust audio.
                let norm = if self.max_rpm > self.idle_rpm {
                    (((self.smoothed_rpm - self.idle_rpm) / (self.max_rpm - self.idle_rpm)) as f32)
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let weights = engine_weights(norm, &self.engine_bands);
                for (wi, w) in weights.iter().enumerate() {
                    self.cur_weights[wi] = *w;
                }
                for (band, pitch) in self.engine_bands.iter().zip(self.cur_pitches.iter_mut()) {
                    *pitch = engine_pitch_scale(self.smoothed_rpm, band);
                }
            }

            let continuous_l = mixed_l;
            let continuous_r = mixed_r;

            // Surface bed (loops at native rate).
            let mut bed = 0.0f32;
            if let Some(bk) = &self.bed_key {
                if let Some(sample) = self.bank.get(bk) {
                    let s = read_looped(&sample.pcm, &mut self.bed_cursor, 1.0);
                    bed = s * bg * self.sample_gain(bk);
                }
            }
            if let Some(key) = self.bed_key.as_deref() {
                mix_through_strip(
                    &mut self.strips,
                    &mut self.reverb_buses,
                    key,
                    bed,
                    &mut mixed_l,
                    &mut mixed_r,
                );
            }

            // Continuous tyre scrub from lateral slide, wheelspin or wheel lock.
            let mut tyre_scrub = 0.0f32;
            if self.smoothed_tyre_scrub_gain > 1e-5 {
                if let Some(sample) = self.bank.get("tyre_scrub") {
                    tyre_scrub = read_looped(
                        &sample.pcm,
                        &mut self.tyre_scrub_cursor,
                        self.tyre_scrub_pitch,
                    ) * self.smoothed_tyre_scrub_gain
                        * self.sample_gain("tyre_scrub");
                }
            }
            mix_through_strip(
                &mut self.strips,
                &mut self.reverb_buses,
                "tyre_scrub",
                tyre_scrub,
                &mut mixed_l,
                &mut mixed_r,
            );

            // Sustained underfloor voice. The middle 50% of the existing scrape
            // sample is used as its stable body; a short seam crossfade prevents
            // the procedural attack/tail from repeating at every wrap.
            let mut scrape = 0.0f32;
            if self.smoothed_scrape_gain > 1e-5 {
                if let Some(sample) = self.bank.get("impact_scrape") {
                    scrape = read_region_looped(
                        &sample.pcm,
                        &mut self.scrape_cursor,
                        self.scrape_pitch,
                        0.25,
                        0.75,
                        0.035,
                    ) * self.smoothed_scrape_gain
                        * self.sample_gain("impact_scrape");
                }
            }
            mix_through_strip(
                &mut self.strips,
                &mut self.reverb_buses,
                "impact_scrape",
                scrape,
                &mut mixed_l,
                &mut mixed_r,
            );

            // One-shots (non-looping, short envelope).
            for o in self.one_shots.iter_mut() {
                if !o.active {
                    continue;
                }
                if o.delay_remaining > 0 {
                    o.delay_remaining -= 1;
                    continue;
                }
                if let Some(sample) = self.bank.get(&o.key) {
                    let len = sample.pcm.len();
                    if len == 0 {
                        o.active = false;
                        continue;
                    }
                    let idx = o.cursor.min(len - 1);
                    if len.saturating_sub(o.cursor) == ONE_SHOT_ENV_SAMPLES {
                        if let Some(strip) = self.strips.get_mut(&o.key) {
                            strip.note_off();
                        }
                    }
                    let s = sample.pcm[idx] as f32 / 32768.0;
                    // Field-level lookup: `one_shots` is mutably borrowed by the
                    // loop, so `self.sample_gain(..)` (a whole-&self borrow) is
                    // rejected; per_sample_gain is a disjoint field.
                    let os_gain = self
                        .per_sample_gain
                        .get(o.key.as_str())
                        .copied()
                        .unwrap_or(1.0);
                    let source = s * os_gain * self.cfg.shift_gain * one_shot_env(o.cursor, len);
                    mix_through_strip(
                        &mut self.strips,
                        &mut self.reverb_buses,
                        &o.key,
                        source,
                        &mut mixed_l,
                        &mut mixed_r,
                    );
                    o.cursor += 1;
                    if o.cursor >= len {
                        o.active = false;
                    }
                } else {
                    o.active = false;
                }
            }

            for bus in &mut self.reverb_buses {
                let (wet_l, wet_r) = bus.processor.process(bus.input_l, bus.input_r);
                mixed_l += wet_l;
                mixed_r += wet_r;
            }
            if self.diagnostic_mode == DiagnosticMode::V10Only {
                mixed_l = continuous_l;
                mixed_r = continuous_r;
            }
            // Fase 4 C++ enhancement, added to the master (pre-limiter). Inert when
            // disabled or gain 0, leaving `combustion_body` untouched.
            if cpp_gain > 0.0 {
                let cl = self.cpp_out_l.get(i).copied().unwrap_or(0.0);
                let cr = self.cpp_out_r.get(i).copied().unwrap_or(0.0);
                mixed_l += scale_cpp_layer(cl, cpp_gain);
                mixed_r += scale_cpp_layer(cr, cpp_gain);
            }
            let saturated_l = self.limiter(mixed_l * self.master_gain);
            let saturated_r = self.limiter(mixed_r * self.master_gain);
            peak_pre_protection = peak_pre_protection
                .max((mixed_l * self.master_gain).abs())
                .max((mixed_r * self.master_gain).abs());
            let (mut out_left, mut out_right) =
                self.stereo_limiter.process(saturated_l, saturated_r);
            if self.transition_remaining > 0 && self.transition_total > 0 {
                let progress =
                    1.0 - self.transition_remaining as f32 / self.transition_total as f32;
                out_left =
                    self.transition_start_l + (out_left - self.transition_start_l) * progress;
                out_right =
                    self.transition_start_r + (out_right - self.transition_start_r) * progress;
                self.transition_remaining -= 1;
            }
            self.last_output_l = out_left;
            self.last_output_r = out_right;
            if i < out_l.len() {
                out_l[i] = out_left;
            }
            if i < out_r.len() {
                out_r[i] = out_right;
            }
        }
        let elapsed_ns = render_started.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        let deadline_ns = n as u64 * 1_000_000_000u64 / self.sample_rate.max(1) as u64;
        self.diagnostics.source = u8::from(self.continuous_source == ContinuousSourceKind::V10Gf509);
        self.diagnostics.received_rpm = self.target_rpm as f32;
        self.diagnostics.rendered_rpm = self.gf509.as_ref().map_or(
            self.smoothed_rpm as f32,
            |runtime| runtime.rendered_telemetry().rpm,
        );
        self.diagnostics.peak_pre_protection = peak_pre_protection;
        self.diagnostics.protection_reduction_db = if peak_pre_protection > 1.0 {
            -20.0 * peak_pre_protection.log10()
        } else {
            0.0
        };
        self.diagnostics.last_render_ns = elapsed_ns;
        self.diagnostics.worst_render_ns = self.diagnostics.worst_render_ns.max(elapsed_ns);
        self.diagnostics.blocks = self.diagnostics.blocks.saturating_add(1);
        if elapsed_ns > deadline_ns {
            self.diagnostics.underruns = self.diagnostics.underruns.saturating_add(1);
        }
        if self.gf509_render_failed {
            self.diagnostics.asset_or_render_errors =
                self.diagnostics.asset_or_render_errors.saturating_add(1);
        }
    }

    /// Smooth soft-saturation limiter. The signal is driven through `tanh` (which
    /// asymptotes gracefully, never flat-topping) and scaled to `limiter_threshold`,
    /// so the peak can never reach ±1.0 (the DAC clip point) and transients are
    /// compressed rather than hard-clipped. This replaces the previous `tanh` +
    /// `clamp(±threshold)` which flat-topped loud transients (audible clipping).
    fn limiter(&self, x: f32) -> f32 {
        let drive = 1.0 + (self.cfg.saturation.max(0.0)) * 3.0;
        (x * drive).tanh() * self.cfg.limiter_threshold
    }

    // --- Telemetry accessors ---
    pub fn last_norm(&self) -> f32 {
        self.last_norm
    }
    pub fn last_rpm(&self) -> f64 {
        self.last_rpm
    }
    pub fn last_throttle(&self) -> f32 {
        self.last_throttle
    }
    pub fn last_speed_kph(&self) -> f64 {
        self.last_speed_kph
    }
    pub fn last_slip(&self) -> f32 {
        self.last_slip
    }
    pub fn last_weights(&self) -> [f32; 5] {
        self.cur_weights
    }
    pub fn last_pitches(&self) -> [f32; 5] {
        self.cur_pitches
    }
    pub fn last_engine_gain(&self) -> f32 {
        self.cur_engine_gain
    }
    pub fn last_trigger(&self) -> &str {
        &self.last_trigger
    }
    pub fn scrape_gain(&self) -> f32 {
        self.smoothed_scrape_gain
    }
    pub fn scrape_pitch(&self) -> f32 {
        self.scrape_pitch as f32
    }
    pub fn scrape_cursor(&self) -> f64 {
        self.scrape_cursor
    }
    pub fn tyre_scrub_gain(&self) -> f32 {
        self.smoothed_tyre_scrub_gain
    }
    pub fn tyre_scrub_pitch(&self) -> f32 {
        self.tyre_scrub_pitch as f32
    }
    pub fn tyre_scrub_mode(&self) -> u8 {
        self.tyre_scrub_mode as u8
    }
    pub fn tyre_scrub_cursor(&self) -> f64 {
        self.tyre_scrub_cursor
    }

    // --- Half-block V10 synthesizer ---

    /// Build the half-block synthesizer from the powertrain contract and switch
    /// the engine/exhaust path to it. Off by default.
    pub fn enable_synth(&mut self, config: &AudioPowertrainSynthesis) -> bool {
        let was_enabled = self.synth_enabled;
        let powertrain = crate::synth::PowertrainConfig::from(config);
        self.lod_levels = config.distance_levels.clone();
        self.synth = Some(crate::synth::HalfBlock::new(&powertrain, self.sample_rate));
        self.synth_enabled = true;
        // Derive the initial LOD from the current listener distance and apply its
        // quality so the first render already runs the correct DSP budget.
        self.lod = crate::synth::select_lod(
            self.listener_distance,
            LodLevel::Near,
            self.lod_levels.near_max_m,
            self.lod_levels.mid_max_m,
            self.lod_levels.far_max_m,
            self.lod_levels.hysteresis_ratio,
        );
        if let Some(synth) = &mut self.synth {
            synth.set_lod(self.lod);
        }
        was_enabled
    }

    pub fn disable_synth(&mut self) {
        self.synth_enabled = false;
        self.synth = None;
    }

    pub fn set_synth_enabled(&mut self, enabled: bool) {
        if enabled && self.synth.is_none() {
            return;
        }
        self.synth_enabled = enabled;
    }

    pub fn set_synth_volume(&mut self, volume: f32) {
        self.synth_volume = volume.clamp(0.0, 2.0);
    }

    pub fn synth_enabled(&self) -> bool {
        self.synth_enabled
    }

    /// Select the packaged GF509 continuous source. Initialization failures
    /// explicitly retain the legacy source; render failures never switch source.
    pub fn enable_v10_gf509(&mut self, asset_directory: &Path) -> Result<(), String> {
        let mut config = v10_engine_synth::Gf509RuntimeConfig::default();
        config.engine.sample_rate = self.sample_rate;
        config.sample_layer_directory = Some(asset_directory.to_path_buf());
        config.max_block_frames = MAX_CPP_BLOCK;
        match v10_engine_synth::Gf509Runtime::new(config) {
            Ok(runtime) => {
                self.gf509 = Some(runtime);
                self.continuous_source = ContinuousSourceKind::V10Gf509;
                self.gf509_render_failed = false;
                Ok(())
            }
            Err(error) => {
                self.continuous_source = ContinuousSourceKind::Legacy;
                Err(error)
            }
        }
    }

    pub fn use_legacy_continuous_source(&mut self) {
        self.continuous_source = ContinuousSourceKind::Legacy;
    }

    pub fn continuous_source(&self) -> ContinuousSourceKind {
        self.continuous_source
    }

    pub fn gf509_render_failed(&self) -> bool {
        self.gf509_render_failed
    }

    pub fn continuous_diagnostics(&self) -> ContinuousDiagnostics {
        self.diagnostics
    }

    pub fn set_diagnostic_mode(&mut self, mode: DiagnosticMode) {
        self.diagnostic_mode = mode;
    }

    /// Replace the provisional control-only load with the authoritative physics
    /// load and tick duration before the next audio block.
    pub fn set_gf509_physics(&mut self, load: f32, dt_seconds: f32) -> Result<(), String> {
        if let Some(gf509) = &mut self.gf509 {
            let torque_sign = v10_engine_synth::TorqueSign::from_i32(self.last_torque_sign)
                .unwrap_or(v10_engine_synth::TorqueSign::Neutral);
            let shift_phase = v10_engine_synth::ShiftPhase::from_i32(self.last_shift_phase)
                .unwrap_or(v10_engine_synth::ShiftPhase::None);
            gf509.update_telemetry(v10_engine_synth::RuntimeTelemetry {
                rpm: self.target_rpm.clamp(0.0, 25_000.0) as f32,
                throttle: self.last_throttle.clamp(0.0, 1.0),
                normalized_engine_load: load.clamp(0.0, 1.0),
                normalized_engine_torque: self.last_normalized_engine_torque.clamp(-1.0, 1.0),
                torque_sign,
                rpm_derivative: self.last_rpm_derivative,
                throttle_derivative: self.last_throttle_derivative,
                gear: self.last_gear.clamp(-1, 12) as i8,
                shift_phase,
                dt_seconds: dt_seconds.clamp(0.0, 1.0),
            })?;
        }
        self.last_normalized_engine_load = load;
        Ok(())
    }

    /// Reset continuous-engine state and stop all active transient voices.
    /// Asset reload performed by GF509 is a control-thread operation.
    pub fn reset_audio_state(&mut self) -> Result<(), String> {
        if let Some(gf509) = &mut self.gf509 {
            gf509.reset()?;
        }
        for one_shot in &mut self.one_shots {
            one_shot.active = false;
            one_shot.cursor = 0;
        }
        self.bed_cursor = 0.0;
        self.scrape_cursor = 0.0;
        self.tyre_scrub_cursor = 0.0;
        self.smoothed_engine_gain = 0.0;
        self.target_engine_gain = 0.0;
        self.smoothed_bed_gain = 0.0;
        self.target_bed_gain = 0.0;
        self.smoothed_scrape_gain = 0.0;
        self.target_scrape_gain = 0.0;
        self.smoothed_tyre_scrub_gain = 0.0;
        self.target_tyre_scrub_gain = 0.0;
        self.smoothed_rpm = 0.0;
        self.target_rpm = 0.0;
        self.exhaust_crackle_samples = 0;
        self.backfire_cooldown_samples = 0;
        self.last_throttle = 0.0;
        self.last_gear = 0;
        self.last_trigger.clear();
        self.last_normalized_engine_load = 0.0;
        self.last_normalized_engine_torque = 0.0;
        self.last_torque_sign = 0;
        self.last_rpm_derivative = 0.0;
        self.last_throttle_derivative = 0.0;
        self.last_shift_phase = 0;
        self.last_clutch_engagement = 0.0;
        self.last_tc_cut_ratio = 0.0;
        self.last_rev_limiter_active = false;
        self.gf509_render_failed = false;
        Ok(())
    }

    pub fn last_normalized_engine_load(&self) -> f32 {
        self.last_normalized_engine_load
    }

    pub fn last_normalized_engine_torque(&self) -> f32 {
        self.last_normalized_engine_torque
    }

    pub fn last_torque_sign(&self) -> i32 {
        self.last_torque_sign
    }

    pub fn last_rpm_derivative(&self) -> f32 {
        self.last_rpm_derivative
    }

    pub fn last_throttle_derivative(&self) -> f32 {
        self.last_throttle_derivative
    }

    pub fn last_shift_phase(&self) -> i32 {
        self.last_shift_phase
    }

    pub fn last_clutch_engagement(&self) -> f32 {
        self.last_clutch_engagement
    }

    pub fn last_tc_cut_ratio(&self) -> f32 {
        self.last_tc_cut_ratio
    }

    pub fn last_rev_limiter_active(&self) -> bool {
        self.last_rev_limiter_active
    }

    pub fn last_synth_energy(&self) -> f32 {
        self.last_synth_energy
    }

    /// Camera-to-vehicle listener distance in metres. Stored (not folded into
    /// the synth) so the controller can expose it as telemetry.
    pub fn set_listener_distance(&mut self, distance: f32) {
        self.listener_distance = if distance.is_finite() {
            distance.max(0.0)
        } else {
            0.0
        };
        self.refine_lod();
    }

    pub fn listener_distance(&self) -> f32 {
        self.listener_distance
    }

    /// Current distance level (LOD) of the procedural engine.
    pub fn synth_lod(&self) -> LodLevel {
        self.lod
    }

    // --- P7.6 synth telemetry accessors (proxy the procedural half-block) ----
    // These expose the synth state so replay/baseline/bench tools can emit the
    // per-frame synth contract without reaching into private fields. When the
    // synth is disabled (or not built yet) they return a safe zero/default.

    /// Total firing events fired since the synth was built (cumulative).
    pub fn synth_events_fired(&self) -> u64 {
        self.synth.as_ref().map_or(0, |s| s.events_fired())
    }

    /// Current smoothed energy [0.0, 1.0].
    pub fn synth_energy(&self) -> f32 {
        self.synth
            .as_ref()
            .map_or(self.last_synth_energy, |s| s.energy())
    }

    /// Current smoothed load [0.0, 1.0] (intake gating/burble indicator).
    pub fn synth_load(&self) -> f32 {
        self.synth.as_ref().map_or(0.0, |s| s.load())
    }

    /// Current intake/exhaust resonator scale (0.0 at Virtual, 1.0 at Near).
    pub fn synth_resonator_scale(&self) -> f32 {
        self.synth.as_ref().map_or(0.0, |s| s.resonator_scale())
    }

    /// Whether the RPM limiter hard gate is currently cutting.
    pub fn synth_limiter_active(&self) -> bool {
        self.synth.as_ref().map_or(false, |s| s.limiter_active())
    }

    /// Current smoothed limiter cut depth [0.0, 1.0].
    pub fn synth_limiter_cut(&self) -> f32 {
        self.synth.as_ref().map_or(0.0, |s| s.limiter_cut())
    }

    /// Current smoothed traction-control suppression [0.0, 1.0].
    pub fn synth_tc_suppression(&self) -> f32 {
        self.synth.as_ref().map_or(0.0, |s| s.tc_suppression())
    }

    /// Current mechanical phase (deg) of the procedural engine. Diagnostic hook
    /// used to verify Virtual only advances the phase ring, never resets it.
    pub fn synth_phase_deg(&self) -> f64 {
        self.synth.as_ref().map_or(0.0, |s| s.phase_deg())
    }

    /// Re-derive the LOD from `listener_distance` with direction-aware hysteresis
    /// and, on any change, switch the synth quality and start a click-free
    /// crossfade using the existing `transition_*` mechanism.
    fn refine_lod(&mut self) {
        let new = crate::synth::select_lod(
            self.listener_distance,
            self.lod,
            self.lod_levels.near_max_m,
            self.lod_levels.mid_max_m,
            self.lod_levels.far_max_m,
            self.lod_levels.hysteresis_ratio,
        );
        if new != self.lod {
            self.lod = new;
            if let Some(synth) = &mut self.synth {
                synth.set_lod(new);
            }
            if self.synth_enabled {
                self.lod_transition_start();
            }
        }
    }

    /// Arm a crossfade from the last rendered output toward the new mix. Reuses
    /// the exact `transition_*` fields the config hot-reload path already uses.
    fn lod_transition_start(&mut self) {
        let ms = self.cfg.lod_transition_ms;
        self.transition_total = ((ms as u64 * self.sample_rate as u64) / 1000) as usize;
        self.transition_remaining = self.transition_total;
        self.transition_start_l = self.last_output_l;
        self.transition_start_r = self.last_output_r;
    }

    /// Set the traction-control cut ratio [0.0, 1.0] on the active synth.
    /// No-op when the synth is disabled or missing.
    pub fn set_tc_cut(&mut self, cut_ratio: f32) {
        if self.synth_enabled {
            if let Some(synth) = &mut self.synth {
                synth.set_tc_cut_ratio(cut_ratio);
            }
        }
    }

    /// Set the RPM-limiter hard gate enabled flag on the active synth.
    /// No-op when the synth is disabled or missing.
    pub fn set_limiter_flag(&mut self, active: bool) {
        if self.synth_enabled {
            if let Some(synth) = &mut self.synth {
                synth.set_limiter_enabled(active);
            }
        }
    }

    // --- Fase 4: C++ post-combustion DSP layer (enhancement, default OFF) ---

    /// Attach a loaded C++ DSP runtime. The layer starts disabled with gain 0, so
    /// the `combustion_body` path is unaffected until explicitly enabled.
    pub fn attach_cpp_dsp(&mut self, runtime: DspRuntime) {
        self.cpp_dsp = Some(runtime);
        self.cpp_event_builder = Some(DspRuntime::make_event_builder(
            self.sample_rate as f64,
            0xF090_1994_5CA1,
        ));
        self.cpp_layer_enabled = false;
        self.cpp_layer_gain = 0.0;
    }

    /// Explicit profile flag: enable/disable the C++ enhancement layer.
    pub fn set_cpp_layer_enabled(&mut self, on: bool) {
        self.cpp_layer_enabled = on;
    }

    /// Set the C++ layer linear gain. Clamped to >= 0. Default is 0 (inert).
    pub fn set_cpp_layer_gain(&mut self, g: f32) {
        self.cpp_layer_gain = g.max(0.0);
    }

    pub fn cpp_layer_enabled(&self) -> bool {
        self.cpp_layer_enabled
    }

    pub fn cpp_layer_gain(&self) -> f32 {
        self.cpp_layer_gain
    }

    pub fn has_cpp_dsp(&self) -> bool {
        self.cpp_dsp.is_some()
    }

    /// Returns a copy of the C++ DSP layer output for the most recently rendered
    /// block (up to `n` frames), or `None` when the layer is inactive. Used by the
    /// offline renderer to emit the `cpp_event_impulse` diagnostic stem.
    pub fn cpp_block_output(&self, n: usize) -> Option<(Vec<f32>, Vec<f32>)> {
        let active =
            self.cpp_layer_enabled && self.cpp_dsp.is_some() && self.cpp_event_builder.is_some();
        if !active {
            return None;
        }
        let n = n.min(self.cpp_out_l.len()).min(self.cpp_out_r.len());
        Some((self.cpp_out_l[..n].to_vec(), self.cpp_out_r[..n].to_vec()))
    }
}

/// Fractional-cursor looped read with linear interpolation (no resampler). The
/// loop seam is expected to be seamless (the bank generator crossfades it), so a
/// plain modulo wrap is click-free.
fn read_looped(pcm: &[i16], cursor: &mut f64, ratio: f64) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    let n = pcm.len() as f64;
    let a = (*cursor as usize) % pcm.len();
    let b = (a + 1) % pcm.len();
    let frac = (*cursor - cursor.floor()) as f32;
    let sa = pcm[a] as f32 / 32768.0;
    let sb = pcm[b] as f32 / 32768.0;
    let s = sa + (sb - sa) * frac;
    *cursor += ratio;
    if *cursor >= n {
        *cursor = cursor.rem_euclid(n);
    }
    s
}

fn read_region_looped(
    pcm: &[i16],
    cursor: &mut f64,
    ratio: f64,
    start_ratio: f64,
    end_ratio: f64,
    crossfade_s: f64,
) -> f32 {
    if pcm.len() < 4 {
        return 0.0;
    }
    let start = (pcm.len() as f64 * start_ratio)
        .floor()
        .clamp(0.0, (pcm.len() - 2) as f64);
    let end = (pcm.len() as f64 * end_ratio)
        .ceil()
        .clamp(start + 2.0, pcm.len() as f64);
    if *cursor < start || *cursor >= end {
        *cursor = start;
    }
    let read = |pos: f64| {
        let p = pos.clamp(0.0, (pcm.len() - 1) as f64);
        let i0 = p.floor() as usize;
        let i1 = (i0 + 1).min(pcm.len() - 1);
        let f = (p - i0 as f64) as f32;
        ((pcm[i0] as f32) * (1.0 - f) + (pcm[i1] as f32) * f) / 32768.0
    };
    let fade = (crossfade_s * 44_100.0).clamp(1.0, (end - start) * 0.25);
    let mut value = read(*cursor);
    if *cursor > end - fade {
        let t = ((*cursor - (end - fade)) / fade).clamp(0.0, 1.0) as f32;
        value = value * (1.0 - t) + read(start + (*cursor - (end - fade))) * t;
    }
    *cursor += ratio.max(0.01);
    if *cursor >= end {
        *cursor = start + (*cursor - end);
    }
    value
}

/// Linear fade-in/out envelope (0..1) for one-shots.
fn one_shot_env(cursor: usize, len: usize) -> f32 {
    let attack = (cursor as f32 / ONE_SHOT_ENV_SAMPLES as f32).min(1.0);
    let tail = ((len - cursor) as f32 / ONE_SHOT_ENV_SAMPLES as f32).min(1.0);
    attack * tail
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_bank() -> VehicleSoundBank {
        use crate::bank::Sample;
        let mut samples = std::collections::BTreeMap::new();
        // Engine layers use a full-scale seamless sine so crossfade overlap peaks
        // are exercised (and the loop wrap is click-free) in headroom/click tests.
        let engine_pcm: Vec<i16> = (0..2048)
            .map(|i| {
                (30000.0 * (2.0 * std::f32::consts::PI * 8.0 * i as f32 / 2048.0).sin()) as i16
            })
            .collect();
        let silent: Vec<i16> = vec![0i16; 2048];
        let pcm: Vec<i16> = (0..2048)
            .map(|i| ((i as f32 / 2048.0 * 2.0 - 1.0) * 1000.0) as i16)
            .collect();
        for key in [
            "engine_idle",
            "engine_low",
            "engine_mid",
            "engine_high",
            "engine_redline",
            "surf_grass",
            "shift_up",
            "int_backfire",
            "int_backfire_2",
            "impact_scrape",
        ] {
            let (data, is_loop) =
                if key == "shift_up" || key.starts_with("int_backfire") || key == "impact_scrape" {
                    (pcm.clone(), false)
                } else if key == "surf_grass" {
                    (pcm.clone(), true)
                } else if key.starts_with("engine") {
                    (engine_pcm.clone(), true)
                } else {
                    (silent.clone(), false)
                };
            samples.insert(
                key.to_string(),
                Sample {
                    key: key.to_string(),
                    role: key.to_string(),
                    sample_rate: 44100,
                    pcm: data,
                    is_loop,
                },
            );
        }
        VehicleSoundBank {
            samples,
            bank_name: "unit".to_string(),
            engine_bands: Vec::new(),
            retired_names: Vec::new(),
            native_rpm: BTreeMap::from([("exhaust-mic".to_string(), 14400.0)]),
        }
    }

    /// Like `dummy_bank` but with silent engine layers, for tests that must isolate
    /// one-shots / beds from the engine signal.
    fn silent_engine_bank() -> VehicleSoundBank {
        use crate::bank::Sample;
        let mut samples = std::collections::BTreeMap::new();
        let silent: Vec<i16> = vec![0i16; 2048];
        let pcm: Vec<i16> = (0..2048)
            .map(|i| ((i as f32 / 2048.0 * 2.0 - 1.0) * 1000.0) as i16)
            .collect();
        for key in [
            "engine_idle",
            "engine_low",
            "engine_mid",
            "engine_high",
            "engine_redline",
            "surf_grass",
            "shift_up",
        ] {
            let (data, is_loop) = if key == "shift_up" {
                (pcm.clone(), false)
            } else if key == "surf_grass" {
                (pcm.clone(), true)
            } else {
                (silent.clone(), key.starts_with("engine"))
            };
            samples.insert(
                key.to_string(),
                Sample {
                    key: key.to_string(),
                    role: key.to_string(),
                    sample_rate: 44100,
                    pcm: data,
                    is_loop,
                },
            );
        }
        VehicleSoundBank {
            samples,
            bank_name: "unit".to_string(),
            engine_bands: Vec::new(),
            retired_names: Vec::new(),
            native_rpm: BTreeMap::from([("exhaust-mic".to_string(), 14400.0)]),
        }
    }

    /// Like `dummy_bank` but with low-level engine sine layers, so the render
    /// stays in the limiter's linear region and per-sample gains scale exactly.
    fn low_level_engine_bank() -> VehicleSoundBank {
        use crate::bank::Sample;
        let mut samples = std::collections::BTreeMap::new();
        let engine_pcm: Vec<i16> = (0..2048)
            .map(|i| (2000.0 * (2.0 * std::f32::consts::PI * 8.0 * i as f32 / 2048.0).sin()) as i16)
            .collect();
        for key in test_engine_bands().iter().map(|band| band.key.as_str()) {
            samples.insert(
                key.to_string(),
                Sample {
                    key: key.to_string(),
                    role: key.to_string(),
                    sample_rate: 44100,
                    pcm: engine_pcm.clone(),
                    is_loop: true,
                },
            );
        }
        VehicleSoundBank {
            samples,
            bank_name: "unit".to_string(),
            engine_bands: Vec::new(),
            retired_names: Vec::new(),
            native_rpm: BTreeMap::from([("exhaust-mic".to_string(), 14400.0)]),
        }
    }

    /// Bank with silent engine layers plus the two backfire one-shot variants
    /// (deterministic ramp), for exact per-sample gain scaling checks.
    fn silent_backfire_bank() -> VehicleSoundBank {
        use crate::bank::Sample;
        let mut samples = std::collections::BTreeMap::new();
        let silent: Vec<i16> = vec![0i16; 2048];
        let ramp: Vec<i16> = (0..2048)
            .map(|i| ((i as f32 / 2048.0 * 2.0 - 1.0) * 800.0) as i16)
            .collect();
        for key in test_engine_bands().iter().map(|band| band.key.as_str()) {
            samples.insert(
                key.to_string(),
                Sample {
                    key: key.to_string(),
                    role: key.to_string(),
                    sample_rate: 44100,
                    pcm: silent.clone(),
                    is_loop: true,
                },
            );
        }
        for key in ["int_backfire", "int_backfire_2"] {
            samples.insert(
                key.to_string(),
                Sample {
                    key: key.to_string(),
                    role: "engine_backfire".to_string(),
                    sample_rate: 44100,
                    pcm: ramp.clone(),
                    is_loop: false,
                },
            );
        }
        VehicleSoundBank {
            samples,
            bank_name: "unit".to_string(),
            engine_bands: Vec::new(),
            retired_names: Vec::new(),
            native_rpm: BTreeMap::from([("exhaust-mic".to_string(), 14400.0)]),
        }
    }

    fn engine_with_bank(bank: VehicleSoundBank) -> VehicleAudioEngine {
        let mut e = VehicleAudioEngine {
            bank,
            engine_bands: test_engine_bands(),
            layer_keys: test_engine_bands()
                .iter()
                .map(|band| band.key.clone())
                .collect(),
            layer_cursors: vec![0.0; 5],
            bed_cursor: 0.0,
            bed_key: None,
            smoothed_engine_gain: 0.0,
            target_engine_gain: 0.0,
            smoothed_bed_gain: 0.0,
            target_bed_gain: 0.0,
            smoothed_scrape_gain: 0.0,
            target_scrape_gain: 0.0,
            scrape_pitch: 1.0,
            scrape_cursor: 0.0,
            smoothed_tyre_scrub_gain: 0.0,
            target_tyre_scrub_gain: 0.0,
            tyre_scrub_pitch: 1.0,
            tyre_scrub_cursor: 0.0,
            tyre_scrub_mode: TireScrubMode::None,
            cur_weights: [0.0; 5],
            cur_pitches: [1.0; 5],
            cur_engine_gain: 0.0,
            one_shots: Vec::new(),
            sample_rate: 44100,
            cfg: AudioConfig::default(),
            last_norm: 0.0,
            last_rpm: 0.0,
            last_throttle: 0.0,
            last_speed_kph: 0.0,
            last_slip: 0.0,
            last_gear: 0,
            last_trigger: String::new(),
            last_normalized_engine_load: 0.0,
            last_normalized_engine_torque: 0.0,
            last_torque_sign: 0,
            last_rpm_derivative: 0.0,
            last_throttle_derivative: 0.0,
            last_shift_phase: 0,
            last_clutch_engagement: 0.0,
            last_tc_cut_ratio: 0.0,
            last_rev_limiter_active: false,
            smoothed_rpm: 0.0,
            target_rpm: 0.0,
            idle_rpm: 0.0,
            max_rpm: 0.0,
            backfire_cooldown_samples: 0,
            variant_rng_state: 0xF090_1994_D15C_A11D,
            per_sample_gain: BTreeMap::new(),
            strips: BTreeMap::new(),
            reverb_buses: Vec::new(),
            stereo_limiter: StereoLimiter::new(0.9),
            master_gain: 1.0,
            config_generation: 1,
            config_source_hash: String::new(),
            transition_remaining: 0,
            transition_total: 0,
            transition_start_l: 0.0,
            transition_start_r: 0.0,
            last_output_l: 0.0,
            last_output_r: 0.0,
            exhaust_key: String::new(),
            exhaust_cursor: 0.0,
            exhaust_crackle_samples: 0,
            synth: None,
            synth_enabled: false,
            synth_volume: 1.0,
            last_synth_energy: 0.0,
            synth_block_l: vec![0.0; MAX_CPP_BLOCK],
            synth_block_r: vec![0.0; MAX_CPP_BLOCK],
            gf509: None,
            continuous_source: ContinuousSourceKind::Legacy,
            gf509_block_l: vec![0.0; MAX_CPP_BLOCK],
            gf509_block_r: vec![0.0; MAX_CPP_BLOCK],
            gf509_render_failed: false,
            diagnostics: ContinuousDiagnostics::default(),
            diagnostic_mode: DiagnosticMode::Mix,
            listener_distance: 0.0,
            lod: LodLevel::Near,
            lod_levels: DistanceLevels::default(),
            cpp_dsp: None,
            cpp_event_builder: None,
            cpp_layer_enabled: false,
            cpp_layer_gain: 0.0,
            cpp_out_l: vec![0.0f32; MAX_CPP_BLOCK],
            cpp_out_r: vec![0.0f32; MAX_CPP_BLOCK],
        };
        e.one_shots.push(OneShot {
            trigger: Trigger::ShiftUp,
            key: "shift_up".to_string(),
            cursor: 0,
            delay_remaining: 0,
            active: false,
        });
        e.one_shots.push(OneShot {
            trigger: Trigger::Backfire,
            key: "int_backfire".to_string(),
            cursor: 0,
            delay_remaining: 0,
            active: false,
        });
        e.one_shots.push(OneShot {
            trigger: Trigger::Backfire,
            key: "int_backfire_2".to_string(),
            cursor: 0,
            delay_remaining: 0,
            active: false,
        });
        e
    }

    #[test]
    fn render_is_finite_and_limited() {
        let mut e = engine_with_bank(dummy_bank());
        e.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l = vec![0.0f32; 4096];
        let mut r = vec![0.0f32; 4096];
        e.render(&mut l, &mut r, 4096);
        for (a, b) in l.iter().zip(r.iter()) {
            assert!(a.is_finite() && b.is_finite());
            assert!(*a <= e.config().limiter_threshold + 1e-3);
            assert!(*a >= -e.config().limiter_threshold - 1e-3);
        }
    }

    #[test]
    fn voice_strip_applies_pan_and_tube_bypass_contract() {
        let mut left_config = SoundConfig {
            pan: -1.0,
            ..SoundConfig::default()
        };
        left_config.eq.tube_color.enabled = false;
        let mut strip = VoiceStrip::new(left_config, 44100, None);
        let (left, right, _) = strip.process(0.5);
        assert!((left - 0.5).abs() < 1e-6 && right.abs() < 1e-6);

        let mut colored = SoundConfig::default();
        colored.eq.tube_color.enabled = true;
        colored.eq.tube_color.amount = 0.8;
        colored.eq.tube_color.mix = 1.0;
        let mut strip = VoiceStrip::new(colored, 44100, None);
        let (left, right, _) = strip.process(0.5);
        assert!(left.is_finite() && right.is_finite());
        assert!((left - 0.5 * std::f32::consts::FRAC_1_SQRT_2).abs() > 1e-3);
    }

    #[test]
    fn two_strips_share_one_reverb_processor() {
        let mut strips = BTreeMap::new();
        let mut config = SoundConfig::default();
        config.reverb.enabled = true;
        config.reverb.send_db = 0.0;
        strips.insert("a".into(), VoiceStrip::new(config.clone(), 44100, Some(0)));
        strips.insert("b".into(), VoiceStrip::new(config, 44100, Some(0)));
        let mut buses = vec![ReverbBus {
            processor: StereoReverb::new(crate::config::ReverbBusConfig::default(), 44100),
            input_l: 0.0,
            input_r: 0.0,
        }];
        let (mut left, mut right) = (0.0, 0.0);
        mix_through_strip(&mut strips, &mut buses, "a", 1.0, &mut left, &mut right);
        mix_through_strip(&mut strips, &mut buses, "b", 1.0, &mut left, &mut right);
        assert_eq!(buses.len(), 1);
        assert!(buses[0].input_l > 1.0 && buses[0].input_r > 1.0);
    }

    #[test]
    fn config_update_is_block_boundary_and_rejects_invalid_candidate() {
        let mut engine = engine_with_bank(dummy_bank());
        let valid = engine.apply_config_json(r#"{"schema_version":2,"defaults":{"pan":-1.0}}"#);
        assert!(valid.is_valid());
        let generation = engine.config_generation();
        let hash = engine.config_source_hash().to_string();
        let invalid = engine.apply_config_json("{partial");
        assert!(!invalid.is_valid());
        assert_eq!(engine.config_generation(), generation);
        assert_eq!(engine.config_source_hash(), hash);
    }

    #[test]
    fn sustained_scrape_keeps_cursor_and_releases_smoothly() {
        let mut e = engine_with_bank(dummy_bank());
        e.set_scrape_state(true, 0.8, 30.0, 0.0);
        let mut l = vec![0.0f32; 512];
        let mut r = vec![0.0f32; 512];
        e.render(&mut l, &mut r, 512);
        let first_cursor = e.scrape_cursor();
        let first_gain = e.scrape_gain();
        assert!(first_cursor > 0.0);
        assert!(first_gain > 0.0);

        e.set_scrape_state(true, 0.8, 30.0, 0.0);
        e.render(&mut l, &mut r, 512);
        assert_ne!(
            e.scrape_cursor(),
            first_cursor,
            "sustained state must not restart the cursor"
        );
        let sustained_gain = e.scrape_gain();

        e.set_scrape_state(false, 0.0, 0.0, 0.0);
        e.render(&mut l, &mut r, 512);
        assert!(e.scrape_gain() < sustained_gain);
        assert!(l.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn limiter_soft_clips_without_harsh_clip() {
        let e = engine_with_bank(dummy_bank());
        let ceiling = e.config().limiter_threshold;
        let drive = 1.0 + e.config().saturation * 3.0;

        // 1) Bounded: a deep overload must never exceed the ceiling (no DAC clip).
        for x in [0.0f32, 0.5, 1.0, 2.0, 5.0, 20.0, 1000.0] {
            let y = e.limiter(x);
            assert!(y.abs() <= ceiling + 1e-4, "exceeded ceiling at x={x}: {y}");
            assert!(y.is_finite(), "non-finite at x={x}");
        }

        // 2) Monotonic: a proper limiter never inverts the signal.
        let mut prev = -1e9f32;
        for i in 0..400usize {
            let x = (i as f32) * 0.1 - 20.0;
            let y = e.limiter(x);
            assert!(y >= prev - 1e-5, "limiter not monotonic at x={x}");
            prev = y;
        }

        // 3) Low-level transparency: quiet signals pass through with ~unity drive
        //    gain (not squashed), proving the limiter only acts on peaks.
        let quiet = e.limiter(0.05);
        let expected = (0.05 * drive).tanh() * ceiling;
        assert!(
            (quiet - expected).abs() < 1e-4,
            "quiet signal not transparent: {quiet} vs {expected}"
        );
        assert!(quiet.abs() > 0.04, "quiet signal over-attenuated: {quiet}");
    }

    #[test]
    fn one_shot_fires_and_stops() {
        let mut e = engine_with_bank(silent_engine_bank());
        e.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        e.trigger(Trigger::ShiftUp);
        let mut l = vec![0.0f32; 4096];
        let mut r = vec![0.0f32; 4096];
        e.render(&mut l, &mut r, 4096);
        let tail = l[3000..]
            .iter()
            .copied()
            .fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(tail < 1e-3, "one-shot should have ended, tail={tail}");
    }

    #[test]
    fn shift_companion_starts_exactly_twenty_ms_late() {
        let mut e = engine_with_bank(silent_engine_bank());
        let mut delayed_sample = e.bank.samples["shift_up"].clone();
        delayed_sample.key = "shift_up_delayed".to_string();
        e.bank
            .samples
            .insert("shift_up_delayed".to_string(), delayed_sample);
        e.one_shots.push(OneShot {
            trigger: Trigger::ShiftUp,
            key: "shift_up_delayed".to_string(),
            cursor: 0,
            delay_remaining: 0,
            active: false,
        });

        e.trigger(Trigger::ShiftUp);
        let delay_frames = e.sample_rate as usize * SHIFT_COMPANION_DELAY_MS / 1000;
        let mut left = vec![0.0; delay_frames];
        let mut right = vec![0.0; delay_frames];
        e.render(&mut left, &mut right, delay_frames);

        let delayed = e
            .one_shots
            .iter()
            .find(|voice| voice.key == "shift_up_delayed")
            .unwrap();
        assert!(delayed.active);
        assert_eq!(delayed.delay_remaining, 0);
        assert_eq!(delayed.cursor, 0, "companion must not advance before 20 ms");

        e.render(&mut [0.0], &mut [0.0], 1);
        let delayed = e
            .one_shots
            .iter()
            .find(|voice| voice.key == "shift_up_delayed")
            .unwrap();
        assert_eq!(delayed.cursor, 1, "companion must start on frame 882");
    }

    #[test]
    fn bed_activates_off_asphalt() {
        let mut e = engine_with_bank(dummy_bank());
        e.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.2, "grass");
        let mut l = vec![0.0f32; 1024];
        let mut r = vec![0.0f32; 1024];
        e.render(&mut l, &mut r, 1024);
        let energy = l.iter().copied().map(|v| v * v).sum::<f32>();
        assert!(energy > 0.0);
    }

    #[test]
    fn rpm_ramp_stays_below_headroom_ceiling() {
        let mut e = engine_with_bank(dummy_bank());
        let mut l = vec![0.0f32; 2048];
        let mut r = vec![0.0f32; 2048];
        let mut peak_post = 0.0f32;
        // Ramp RPM across the whole band range, re-calling set_state per block like
        // the physics tick does. On asphalt (no bed/one-shots) the engine signal is
        // the only thing present, directly exercising band crossfade overlap.
        for step in 0..40 {
            let rpm = 2000.0 + step as f64 * 350.0;
            e.set_state(rpm, 1000.0, 15000.0, 1.0, 0.0, 3, 0.0, "asphalt");
            e.render(&mut l, &mut r, 2048);
            for v in l.iter() {
                let a = v.abs();
                if a > peak_post {
                    peak_post = a;
                }
            }
        }
        // With engine_headroom=0.62 the steady engine peak (~0.58) sits well below
        // the limiter ceiling; without it the crossfade would slam to ~0.76, which
        // is the clipping-at-RPM-change regression. Use 0.70 as a tight guard.
        let ceiling = e.config().limiter_threshold;
        assert!(
            peak_post <= ceiling + 1e-3,
            "exceeded limiter ceiling: {peak_post}"
        );
        assert!(
            peak_post < 0.70,
            "engine headroom lost: peak_post={peak_post} (expected < 0.70)"
        );
    }

    #[test]
    fn synth_rpm_ramp_stays_below_headroom_ceiling() {
        let mut e = engine_with_bank(dummy_bank());
        e.enable_synth(&AudioPowertrainSynthesis::default());
        let mut l = vec![0.0f32; 2048];
        let mut r = vec![0.0f32; 2048];
        let mut peak_post = 0.0f32;
        let mut peak_limited = 0.0f32;
        // Same ramp as the banked test: the procedural synth runs the full band
        // range. Step 36 is the last below the limiter threshold (14925 f32);
        // steps 37..39 are cut by the procedural limiter. With headroom 0.62
        // and the 0.90 limiter ceiling the peak must stay under the ceiling
        // everywhere; the last step (cut settled long ago) must come out
        // clearly suppressed.
        for step in 0..40 {
            let rpm = 2000.0 + step as f64 * 350.0;
            e.set_state(rpm, 1000.0, 15000.0, 1.0, 0.0, 3, 0.0, "asphalt");
            e.render(&mut l, &mut r, 2048);
            for v in l.iter() {
                let a = v.abs();
                if a > peak_post {
                    peak_post = a;
                }
                if step == 39 && a > peak_limited {
                    peak_limited = a;
                }
            }
        }
        let ceiling = e.config().limiter_threshold;
        assert!(
            peak_post <= ceiling + 1e-3,
            "synth exceeded limiter ceiling: {peak_post}"
        );
        assert!(
            peak_post < 0.90,
            "synth peak too close to ceiling: peak_post={peak_post}"
        );
        assert!(
            peak_limited < 0.50,
            "procedural limiter did not suppress the top steps: {peak_limited}"
        );
        assert!(
            e.last_synth_energy() > 0.0,
            "synth energy missing during ramp"
        );
    }

    #[test]
    fn pitch_glides_instead_of_stepping() {
        let mut e = engine_with_bank(dummy_bank());
        e.set_state(2000.0, 1000.0, 15000.0, 1.0, 0.0, 3, 0.0, "asphalt");
        e.render(&mut vec![0.0; 128], &mut vec![0.0; 128], 128);
        let p0 = e.last_pitches()[0];
        // Hard RPM jump; render only a few samples so the smoothed pitch must still
        // be near the old value (gliding), not snapped to the new target.
        e.set_state(14000.0, 1000.0, 15000.0, 1.0, 0.0, 3, 0.0, "asphalt");
        e.render(&mut [0.0; 4], &mut [0.0; 4], 4);
        let p1 = e.last_pitches()[0];
        let target = engine_pitch_scale(14000.0, &test_engine_bands()[0]);
        // With smoothing the pitch barely moved; without it p1 would equal target.
        assert!(
            (p1 - p0).abs() < (target - p0).abs() * 0.5,
            "pitch snapped instead of gliding: p0={p0} p1={p1} target={target}"
        );
    }

    #[test]
    fn backfire_fires_on_overrun_above_13_5k_rpm() {
        let mut e = engine_with_bank(dummy_bank());
        // High throttle at high RPM (>13500)
        e.set_state(14500.0, 1000.0, 15000.0, 1.0, 200.0, 4, 0.0, "asphalt");
        assert_eq!(e.last_trigger(), "");

        // Sudden lift-off
        e.set_state(14200.0, 1000.0, 15000.0, 0.0, 200.0, 4, 0.0, "asphalt");
        assert_eq!(e.last_trigger(), "engine_backfire");

        // Cooldown prevents a second lift-off from restarting another variant.
        let active_before = e
            .one_shots
            .iter()
            .position(|o| o.trigger == Trigger::Backfire && o.active);
        e.render(&mut vec![0.0; 256], &mut vec![0.0; 256], 256);
        let cooldown_before = e.backfire_cooldown_samples;
        e.set_state(14000.0, 1000.0, 15000.0, 1.0, 200.0, 4, 0.0, "asphalt");
        e.set_state(13800.0, 1000.0, 15000.0, 0.0, 200.0, 4, 0.0, "asphalt");
        let active_after = e
            .one_shots
            .iter()
            .position(|o| o.trigger == Trigger::Backfire && o.active);
        assert_eq!(e.backfire_cooldown_samples, cooldown_before);
        assert_eq!(active_after, active_before);

        // Advance render past the remaining cooldown (1 s @ 44.1 kHz).
        let mut l = vec![0.0f32; 44200];
        let mut r = vec![0.0f32; 44200];
        e.render(&mut l, &mut r, 44200);
        assert_eq!(e.backfire_cooldown_samples, 0);

        // Now next lift-off triggers backfire again
        e.set_state(14000.0, 1000.0, 15000.0, 1.0, 200.0, 4, 0.0, "asphalt");
        e.set_state(13900.0, 1000.0, 15000.0, 0.0, 200.0, 4, 0.0, "asphalt");
        assert_eq!(e.last_trigger(), "engine_backfire");
    }

    #[test]
    fn backfire_does_not_fire_at_low_rpm() {
        let mut e = engine_with_bank(dummy_bank());
        // High throttle at low RPM (<13500)
        e.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        // Sudden lift-off
        e.set_state(7800.0, 1000.0, 15000.0, 0.0, 100.0, 3, 0.0, "asphalt");
        assert_eq!(e.last_trigger(), "");

        // Just below the 13.5k threshold: lift-off must not fire.
        e.set_state(13400.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        e.set_state(13200.0, 1000.0, 15000.0, 0.0, 100.0, 3, 0.0, "asphalt");
        assert_eq!(e.last_trigger(), "");
    }

    #[test]
    fn backfire_selects_one_of_two_variants_deterministically() {
        let mut e = engine_with_bank(dummy_bank());
        let mut sequence = Vec::new();
        for _ in 0..8 {
            e.trigger(Trigger::Backfire);
            let active: Vec<&str> = e
                .one_shots
                .iter()
                .filter(|o| o.trigger == Trigger::Backfire && o.active)
                .map(|o| o.key.as_str())
                .collect();
            assert_eq!(active.len(), 1);
            sequence.push(active[0].to_string());
        }
        assert!(sequence.iter().any(|k| k == "int_backfire"));
        assert!(sequence.iter().any(|k| k == "int_backfire_2"));

        let mut replay = engine_with_bank(dummy_bank());
        let replay_sequence: Vec<String> = (0..8)
            .map(|_| {
                replay.trigger(Trigger::Backfire);
                replay
                    .one_shots
                    .iter()
                    .find(|o| o.trigger == Trigger::Backfire && o.active)
                    .unwrap()
                    .key
                    .clone()
            })
            .collect();
        assert_eq!(sequence, replay_sequence);
    }

    #[test]
    fn configured_gain_scales_engine_bands() {
        let mut e1 = engine_with_bank(low_level_engine_bank());
        let mut e2 = engine_with_bank(low_level_engine_bank());
        for key in test_engine_bands().iter().map(|band| band.key.as_str()) {
            e2.per_sample_gain.insert(key.to_string(), 0.5);
        }
        e1.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        e2.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l1 = vec![0.0f32; 512];
        let mut r1 = vec![0.0f32; 512];
        let mut l2 = vec![0.0f32; 512];
        let mut r2 = vec![0.0f32; 512];
        e1.render(&mut l1, &mut r1, 512);
        e2.render(&mut l2, &mut r2, 512);
        for i in 0..512 {
            // Absolute comparison: the sine crosses zero, so ratios are NaN there.
            let want = 0.5 * l1[i];
            assert!(
                (l2[i] - want).abs() < 1e-3,
                "sample {i}: got {} want {want}",
                l2[i]
            );
        }
    }

    #[test]
    fn configured_gain_scales_backfire_one_shot() {
        let mut e1 = engine_with_bank(silent_backfire_bank());
        let mut e2 = engine_with_bank(silent_backfire_bank());
        e2.per_sample_gain.insert("int_backfire".to_string(), 0.4);
        e2.per_sample_gain.insert("int_backfire_2".to_string(), 0.4);
        e1.set_state(14000.0, 1000.0, 15000.0, 0.0, 200.0, 4, 0.0, "asphalt");
        e2.set_state(14000.0, 1000.0, 15000.0, 0.0, 200.0, 4, 0.0, "asphalt");
        e1.trigger(Trigger::Backfire);
        e2.trigger(Trigger::Backfire);
        let mut l1 = vec![0.0f32; 2048];
        let mut r1 = vec![0.0f32; 2048];
        let mut l2 = vec![0.0f32; 2048];
        let mut r2 = vec![0.0f32; 2048];
        e1.render(&mut l1, &mut r1, 2048);
        e2.render(&mut l2, &mut r2, 2048);
        let mut checked = 0;
        for i in 100..1700 {
            if l1[i] == 0.0 {
                continue;
            }
            let ratio = l2[i] / l1[i];
            assert!(
                (ratio - 0.4).abs() < 0.02,
                "sample {i}: ratio {ratio} (want 0.4)"
            );
            checked += 1;
        }
        assert!(checked > 0, "one-shot produced no audible output");
    }

    #[test]
    fn unconfigured_keys_play_at_full_level() {
        let mut e1 = engine_with_bank(dummy_bank());
        let mut e2 = engine_with_bank(dummy_bank());
        e2.per_sample_gain.insert("does_not_exist".to_string(), 0.0);
        e1.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        e2.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l1 = vec![0.0f32; 256];
        let mut r1 = vec![0.0f32; 256];
        let mut l2 = vec![0.0f32; 256];
        let mut r2 = vec![0.0f32; 256];
        e1.render(&mut l1, &mut r1, 256);
        e2.render(&mut l2, &mut r2, 256);
        assert_eq!(l1, l2, "unconfigured keys must not affect the mix");
    }

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|v| v * v).sum::<f32>() / buf.len().max(1) as f32).sqrt()
    }

    fn pearson(a: &[f32], b: &[f32]) -> f32 {
        let n = a.len().min(b.len()).max(1);
        let mut mean_a = 0.0f64;
        let mut mean_b = 0.0f64;
        for i in 0..n {
            mean_a += a[i] as f64;
            mean_b += b[i] as f64;
        }
        mean_a /= n as f64;
        mean_b /= n as f64;
        let (mut cov, mut var_a, mut var_b) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..n {
            let da = a[i] as f64 - mean_a;
            let db = b[i] as f64 - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }
        (cov / (var_a * var_b).sqrt()) as f32
    }

    #[test]
    fn synth_defaults_disabled() {
        let mut e = engine_with_bank(silent_engine_bank());
        assert!(!e.synth_enabled());
        assert_eq!(e.last_synth_energy(), 0.0);
        e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l = vec![0.0f32; 4096];
        e.render(&mut l, &mut vec![0.0f32; 4096], 4096);
        assert!(
            l.iter().all(|v| v.abs() < 1e-6),
            "silence expected while disabled"
        );
    }

    #[test]
    fn synth_render_produces_finite_bounded_output() {
        let mut e = engine_with_bank(dummy_bank());
        let contract = AudioPowertrainSynthesis::default();
        e.enable_synth(&contract);
        assert!(e.synth_enabled());
        e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l = vec![0.0f32; 4096];
        let mut r = vec![0.0f32; 4096];
        e.render(&mut l, &mut r, 4096);
        assert!(l
            .iter()
            .zip(r.iter())
            .all(|(a, b)| a.is_finite() && b.is_finite()));
        let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.0, "synth should be audible when enabled");
        assert!(
            peak <= e.config().limiter_threshold + 1e-3,
            "synth must stay under limiter ({peak})"
        );
        assert!(e.last_synth_energy() > 0.0, "energy should track state");
    }

    #[test]
    fn toggle_synth_switches_engine_feed() {
        let mut e = engine_with_bank(silent_engine_bank());
        let contract = AudioPowertrainSynthesis::default();
        e.enable_synth(&contract);
        e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l_on = vec![0.0f32; 2048];
        e.render(&mut l_on, &mut vec![0.0f32; 2048], 2048);
        let rms_on = rms(&l_on);

        e.disable_synth();
        e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l_off = vec![0.0f32; 2048];
        e.render(&mut l_off, &mut vec![0.0f32; 2048], 2048);
        let rms_off = rms(&l_off);

        assert!(rms_on > 1e-5, "synth on should be audible");
        assert!(rms_off < 1e-6, "synth off should be silent");
    }

    fn packaged_gf509_assets() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../audio/v10_gf509")
    }

    #[test]
    fn gf509_initialization_is_fail_fast_and_retains_legacy() {
        let mut engine = engine_with_bank(silent_engine_bank());
        assert!(engine
            .enable_v10_gf509(std::path::Path::new("definitely-missing-gf509-assets"))
            .is_err());
        assert_eq!(engine.continuous_source(), ContinuousSourceKind::Legacy);
    }

    #[test]
    fn gf509_replaces_only_continuous_source_and_one_shots_remain() {
        let mut engine = engine_with_bank(silent_engine_bank());
        engine.enable_v10_gf509(&packaged_gf509_assets()).unwrap();
        engine.set_synth_volume(0.0);
        engine.set_state(7_499.0, 1_000.0, 15_000.0, 0.92, 100.0, 3, 0.0, "asphalt");
        let mut silent_l = vec![0.0; 512];
        let mut silent_r = vec![0.0; 512];
        engine.render(&mut silent_l, &mut silent_r, 512);
        assert!(silent_l.iter().all(|sample| sample.abs() < 1e-7));
        assert_eq!(silent_l, silent_r);

        engine.trigger(Trigger::ShiftUp);
        let mut event_l = vec![0.0; 512];
        let mut event_r = vec![0.0; 512];
        engine.render(&mut event_l, &mut event_r, 512);
        assert!(event_l.iter().any(|sample| sample.abs() > 1e-6));
        assert_eq!(engine.continuous_source(), ContinuousSourceKind::V10Gf509);
        assert!(!engine.gf509_render_failed());
    }

    #[test]
    fn gf509_render_has_zero_rust_allocations() {
        let mut engine = engine_with_bank(silent_engine_bank());
        engine.enable_v10_gf509(&packaged_gf509_assets()).unwrap();
        engine.set_state(9_000.0, 1_000.0, 15_000.0, 0.8, 120.0, 4, 0.0, "asphalt");
        let mut left = vec![0.0; 512];
        let mut right = vec![0.0; 512];
        engine.render(&mut left, &mut right, 512);
        crate::allocation_probe::start();
        engine.render(&mut left, &mut right, 512);
        let (allocations, frees) = crate::allocation_probe::stop();
        assert_eq!((allocations, frees), (0, 0));
    }

    #[test]
    fn reset_stops_one_shots_and_clears_gf509_state() {
        let mut engine = engine_with_bank(silent_engine_bank());
        engine.enable_v10_gf509(&packaged_gf509_assets()).unwrap();
        engine.set_synth_volume(0.0);
        engine.set_state(9_000.0, 1_000.0, 15_000.0, 0.8, 120.0, 4, 0.0, "asphalt");
        engine.trigger(Trigger::ShiftUp);
        engine.reset_audio_state().unwrap();
        let mut left = vec![0.0; 512];
        let mut right = vec![0.0; 512];
        engine.render(&mut left, &mut right, 512);
        assert!(left.iter().all(|sample| sample.abs() < 1e-7));
        assert_eq!(left, right);
        assert!(!engine.gf509_render_failed());
    }

    #[test]
    fn synth_stereo_output_is_dual_mono_and_finite() {
        // The continuous engine carries no width: both mixer channels must be
        // bit-identical while still being audible and bounded.
        let mut e = engine_with_bank(silent_engine_bank());
        let contract = AudioPowertrainSynthesis::default();
        e.enable_synth(&contract);
        e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l = vec![0.0f32; 8192];
        let mut r = vec![0.0f32; 8192];
        e.render(&mut l, &mut r, 8192);
        assert!(l
            .iter()
            .zip(r.iter())
            .all(|(a, b)| a.is_finite() && b.is_finite()));
        assert!(
            l.iter()
                .zip(r.iter())
                .all(|(a, b)| a.to_bits() == b.to_bits()),
            "dual-mono engine channels must not differ"
        );
        assert!(rms(&l[1024..]) > 1e-5, "engine must be audible");
        let peak = l.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(
            peak <= e.config().limiter_threshold + 1e-3,
            "stereo synth must stay under limiter ({peak})"
        );
    }

    #[test]
    fn synth_engine_is_dual_mono_in_the_mixer() {
        // The continuous procedural engine carries no pan law: both mixer
        // channels must receive the identical sample, so no width, crossfeed or
        // equal-power attenuation can reappear downstream of the synth.
        let mut e = engine_with_bank(silent_engine_bank());
        e.enable_synth(&AudioPowertrainSynthesis::default());
        e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut l = vec![0.0f32; 8192];
        let mut r = vec![0.0f32; 8192];
        e.render(&mut l, &mut r, 8192);
        assert!(rms(&l[1024..]) > 1e-5, "engine must be audible");
        for (index, (left, right)) in l.iter().zip(r.iter()).enumerate() {
            assert_eq!(
                left.to_bits(),
                right.to_bits(),
                "channels must be bit-identical (sample {index}: {left} vs {right})"
            );
        }
    }

    #[test]
    fn synth_stereo_render_is_deterministic() {
        let build = || {
            let mut e = engine_with_bank(silent_engine_bank());
            e.enable_synth(&AudioPowertrainSynthesis::default());
            e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
            e
        };
        let mut a = build();
        let mut b = build();
        let (mut la, mut ra) = (vec![0.0f32; 4096], vec![0.0f32; 4096]);
        let (mut lb, mut rb) = (vec![0.0f32; 4096], vec![0.0f32; 4096]);
        a.render(&mut la, &mut ra, 4096);
        b.render(&mut lb, &mut rb, 4096);
        assert_eq!(la, lb);
        assert_eq!(ra, rb);
    }

    #[test]
    fn synth_lod_follows_listener_distance_with_hysteresis() {
        let mut e = engine_with_bank(silent_engine_bank());
        e.enable_synth(&AudioPowertrainSynthesis::default());
        // Default contract distance levels: near 25 / mid 80 / far 200, hyst 0.10.
        assert_eq!(e.synth_lod(), LodLevel::Near);

        e.set_listener_distance(28.0); // >= 25 * 1.10 -> Mid
        assert_eq!(e.synth_lod(), LodLevel::Mid);
        e.set_listener_distance(25.0); // inside 22.5..27.5 band -> stays Mid
        assert_eq!(e.synth_lod(), LodLevel::Mid);
        e.set_listener_distance(22.0); // < 25 * 0.90 -> Near
        assert_eq!(e.synth_lod(), LodLevel::Near);

        e.set_listener_distance(88.0); // climb Near -> Far
        assert_eq!(e.synth_lod(), LodLevel::Far);
        e.set_listener_distance(200.0); // inside 180..220 band -> stays Far
        assert_eq!(e.synth_lod(), LodLevel::Far);
        e.set_listener_distance(220.0); // >= 200 * 1.10 -> Virtual
        assert_eq!(e.synth_lod(), LodLevel::Virtual);
        e.set_listener_distance(179.0); // < 200 * 0.90 -> Far
        assert_eq!(e.synth_lod(), LodLevel::Far);
    }

    #[test]
    fn nonfinite_listener_distance_normalizes_to_near() {
        let mut e = engine_with_bank(silent_engine_bank());
        e.enable_synth(&AudioPowertrainSynthesis::default());
        e.set_listener_distance(f32::NAN);
        assert_eq!(e.listener_distance(), 0.0);
        assert_eq!(e.synth_lod(), LodLevel::Near);
        e.set_listener_distance(f32::INFINITY);
        assert_eq!(e.listener_distance(), 0.0);
    }

    #[test]
    fn virtual_lod_is_silent_and_reentry_is_deterministic() {
        let render_lod = || {
            let mut e = engine_with_bank(silent_engine_bank());
            e.enable_synth(&AudioPowertrainSynthesis::default());
            e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
            let (mut l, mut r) = (vec![0.0f32; 2048], vec![0.0f32; 2048]);
            // Approach/depart: Near -> Virtual (silent) -> Near (audible, phase kept).
            e.set_listener_distance(5.0);
            e.render(&mut l, &mut r, 2048);
            assert!(rms(&l) > 1e-5, "Near should be audible");
            let near_phase = e.synth_phase_deg();

            e.set_listener_distance(250.0);
            assert_eq!(e.synth_lod(), LodLevel::Virtual);
            // Drain the armed LOD crossfade, then measure the settled silence.
            e.render(&mut l, &mut r, 2048);
            assert!(l.iter().all(|v| v.is_finite()), "Virtual must stay finite");
            e.render(&mut l, &mut r, 2048);
            assert!(rms(&l) < 1e-6, "Virtual must be silent");

            e.set_listener_distance(5.0);
            assert_eq!(e.synth_lod(), LodLevel::Near);
            let mut near = vec![0.0f32; 2048];
            e.render(&mut near, &mut vec![0.0f32; 2048], 2048);
            assert!(rms(&near) > 1e-5, "re-entering Near must be audible");
            // Phase must have advanced through Virtual, not reset.
            assert_ne!(e.synth_phase_deg(), near_phase);
            near
        };
        let a = render_lod();
        let b = render_lod();
        assert_eq!(a, b, "Virtual re-entry must be deterministic");
    }

    #[test]
    fn lod_ramp_is_click_free() {
        // Baseline peak sample-to-sample step in steady Near.
        let mut base = engine_with_bank(silent_engine_bank());
        base.enable_synth(&AudioPowertrainSynthesis::default());
        base.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut prev = 0.0f32;
        let mut base_step = 0.0f32;
        for _ in 0..8192 {
            let (mut l, mut r) = (vec![0.0f32; 128], vec![0.0f32; 128]);
            base.render(&mut l, &mut r, 128);
            for &v in l.iter() {
                base_step = base_step.max((v - prev).abs());
                prev = v;
            }
        }

        // Sweep the whole LOD range; every boundary must crossfade, not click.
        let mut e = engine_with_bank(silent_engine_bank());
        e.enable_synth(&AudioPowertrainSynthesis::default());
        e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let mut prev = 0.0f32;
        let mut peak_step = 0.0f32;
        for dist in [0.0, 30.0, 90.0, 250.0, 5.0] {
            e.set_listener_distance(dist);
            let (mut l, mut r) = (vec![0.0f32; 2048], vec![0.0f32; 2048]);
            e.render(&mut l, &mut r, 2048);
            for &v in l.iter() {
                assert!(v.is_finite(), "LOD ramp must stay finite");
                peak_step = peak_step.max((v - prev).abs());
                prev = v;
            }
        }
        assert!(
            peak_step < base_step * 2.0 + 0.05,
            "LOD switch produced a click: step {peak_step} vs baseline {base_step}"
        );
    }

    // --- Fase 4: C++ DSP layer ON/OFF comparison (4.3 conservative mix) ---

    #[cfg(windows)]
    fn cpp_scenario(attach: bool, gain: f32) -> (Vec<f32>, Vec<f32>) {
        let mut e = engine_with_bank(dummy_bank());
        // The C++ layer consumes real CylState combustion events; there is no
        // fallback event generator when the physical synth is absent.
        e.enable_synth(&AudioPowertrainSynthesis::default());
        if attach {
            let dsp = DspRuntime::load_default().expect("f90_audio_dsp.dll must be available");
            e.attach_cpp_dsp(dsp);
            e.set_cpp_layer_enabled(true);
            e.set_cpp_layer_gain(gain);
        }
        let mut l = Vec::new();
        let mut r = Vec::new();
        for _ in 0..3 {
            e.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
            let mut bl = vec![0.0f32; 512];
            let mut br = vec![0.0f32; 512];
            e.render(&mut bl, &mut br, 512);
            l.extend_from_slice(&bl);
            r.extend_from_slice(&br);
        }
        (l, r)
    }

    #[test]
    #[cfg(windows)]
    fn cpp_layer_gain_zero_leaves_body_invariant() {
        let (a_l, a_r) = cpp_scenario(false, 0.0);
        let (b_l, b_r) = cpp_scenario(true, 0.0);
        assert_eq!(a_l.len(), b_l.len());
        for (x, y) in a_l.iter().zip(&b_l) {
            assert_eq!(
                x.to_bits(),
                y.to_bits(),
                "left body changed with cpp gain 0"
            );
        }
        for (x, y) in a_r.iter().zip(&b_r) {
            assert_eq!(
                x.to_bits(),
                y.to_bits(),
                "right body changed with cpp gain 0"
            );
        }
    }

    #[test]
    #[cfg(windows)]
    fn cpp_layer_enabled_augments_master() {
        let (a_l, a_r) = cpp_scenario(false, 0.0);
        let (c_l, c_r) = cpp_scenario(true, 1.0);
        let energy = |v: &[f32]| v.iter().map(|s| (s * s) as f64).sum::<f64>();
        let ea = energy(&a_l) + energy(&a_r);
        let ec = energy(&c_l) + energy(&c_r);
        assert!(
            ec > ea + 1e-6,
            "enabled C++ layer must increase master energy"
        );
        let diff = a_l
            .iter()
            .zip(&c_l)
            .filter(|(x, y)| x.to_bits() != y.to_bits())
            .count();
        assert!(diff > 0, "enabled C++ layer must alter at least one sample");
    }

    #[test]
    fn gf509_output_gain_does_not_apply_legacy_throttle_attenuation() {
        let headroom = 0.62;
        let volume = 0.8;
        let coast_legacy_gain = engine_gain(0.0);
        let gf509 = continuous_output_gain(
            ContinuousSourceKind::V10Gf509,
            coast_legacy_gain,
            headroom,
            volume,
        );
        let legacy = continuous_output_gain(
            ContinuousSourceKind::Legacy,
            coast_legacy_gain,
            headroom,
            volume,
        );

        assert_eq!(gf509.to_bits(), (headroom * volume).to_bits());
        assert_eq!(
            legacy.to_bits(),
            (coast_legacy_gain * headroom * volume).to_bits()
        );
        assert!(gf509 > legacy);
    }

    #[test]
    fn cpp_layer_gain_is_linear_not_squared() {
        let unity = scale_cpp_layer(0.8, 1.0);
        let half = scale_cpp_layer(0.8, 0.5);
        let zero = scale_cpp_layer(0.8, 0.0);
        assert_eq!(zero.to_bits(), 0.0f32.to_bits());
        assert!((half / unity - 0.5).abs() < f32::EPSILON);
        let db = 20.0 * (half / unity).log10();
        assert!((db + 6.0206).abs() < 0.001, "half gain was {db} dB");
    }

    #[test]
    #[cfg(windows)]
    fn cpp_callback_has_zero_rust_allocations_and_frees() {
        let mut engine = engine_with_bank(dummy_bank());
        engine.enable_synth(&AudioPowertrainSynthesis::default());
        engine.set_state(9000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        let runtime = DspRuntime::load_default().expect("f90_audio_dsp.dll must be available");
        engine.attach_cpp_dsp(runtime);
        engine.set_cpp_layer_enabled(true);
        engine.set_cpp_layer_gain(0.5);
        let mut left = [0.0f32; 512];
        let mut right = [0.0f32; 512];
        engine.render(&mut left, &mut right, 512); // warm caches/state
        crate::allocation_probe::start();
        engine.render(&mut left, &mut right, 512);
        let (allocs, frees) = crate::allocation_probe::stop();
        assert_eq!(
            (allocs, frees),
            (0, 0),
            "callback allocated/freed Rust memory"
        );
    }
}
