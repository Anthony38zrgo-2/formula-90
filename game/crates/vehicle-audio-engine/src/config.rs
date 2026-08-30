//! Runtime mixer configuration loaded from `sound_mixer_config.json`.
//!
//! Supports both schema v1 (legacy gains-only) and v2 (full sound design).
//! The file lives in the sounds root (`game/sounds/`), two levels above the bank
//! directory (`game/sounds/banks/v10_vehicle` -> `../../sound_mixer_config.json`).
//!
//! Missing, invalid or malformed JSON degrades to safe defaults: the mixer must
//! never fail to start because of a tuning file.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Filename of the runtime mixer config, resolved relative to the bank parent.
pub const MIXER_CONFIG_FILENAME: &str = "sound_mixer_config.json";

/// EQ band center frequencies (ISO standard, Hz).
pub const EQ_BAND_FREQS: [u32; 10] = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

/// ADSR envelope curve types.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AdsrCurve {
    #[default]
    Linear,
    Exponential,
    Logarithmic,
}

/// ADSR envelope configuration (Attack-Decay-Sustain-Release).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdsrConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Attack time in milliseconds [0, 10000].
    #[serde(default)]
    pub attack_ms: f32,
    /// Decay time in milliseconds [0, 10000].
    #[serde(default)]
    pub decay_ms: f32,
    /// Sustain level [0, 1].
    #[serde(default = "default_sustain")]
    pub sustain: f32,
    /// Release time in milliseconds [0, 10000].
    #[serde(default = "default_release_ms")]
    pub release_ms: f32,
    /// Envelope curve shape.
    #[serde(default)]
    pub curve: AdsrCurve,
}

fn default_sustain() -> f32 {
    1.0
}

fn default_release_ms() -> f32 {
    5.8
}

impl Default for AdsrConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            attack_ms: 0.0,
            decay_ms: 0.0,
            sustain: 1.0,
            release_ms: 5.8,
            curve: AdsrCurve::Linear,
        }
    }
}

impl AdsrConfig {
    /// Validate and clamp to safe ranges.
    pub fn sanitized(mut self) -> Self {
        self.attack_ms = self.attack_ms.clamp(0.0, 10000.0);
        self.decay_ms = self.decay_ms.clamp(0.0, 10000.0);
        self.sustain = self.sustain.clamp(0.0, 1.0);
        self.release_ms = self.release_ms.clamp(0.0, 10000.0);
        // Replace NaN with safe defaults
        if !self.attack_ms.is_finite() {
            self.attack_ms = 0.0;
        }
        if !self.decay_ms.is_finite() {
            self.decay_ms = 0.0;
        }
        if !self.sustain.is_finite() {
            self.sustain = 1.0;
        }
        if !self.release_ms.is_finite() {
            self.release_ms = 5.8;
        }
        self
    }
}

/// Tube coloration (valve warmth) configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TubeColorConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Drive amount [0, 1].
    #[serde(default)]
    pub amount: f32,
    /// Link to EQ boosts [0, 1].
    #[serde(default = "default_boost_link")]
    pub boost_link: f32,
    /// DC bias offset [0, 0.5].
    #[serde(default = "default_bias")]
    pub bias: f32,
    /// Dry/wet mix [0, 1].
    #[serde(default = "default_tube_mix")]
    pub mix: f32,
    /// Apply automatic gain compensation.
    #[serde(default = "default_true")]
    pub auto_gain: bool,
}

fn default_boost_link() -> f32 {
    0.35
}

fn default_bias() -> f32 {
    0.05
}

fn default_tube_mix() -> f32 {
    0.15
}

fn default_true() -> bool {
    true
}

impl Default for TubeColorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 0.0,
            boost_link: 0.35,
            bias: 0.05,
            mix: 0.15,
            auto_gain: true,
        }
    }
}

impl TubeColorConfig {
    pub fn sanitized(mut self) -> Self {
        self.amount = self.amount.clamp(0.0, 1.0);
        self.boost_link = self.boost_link.clamp(0.0, 1.0);
        self.bias = self.bias.clamp(0.0, 0.5);
        self.mix = self.mix.clamp(0.0, 1.0);
        if !self.amount.is_finite() {
            self.amount = 0.0;
        }
        if !self.boost_link.is_finite() {
            self.boost_link = 0.35;
        }
        if !self.bias.is_finite() {
            self.bias = 0.05;
        }
        if !self.mix.is_finite() {
            self.mix = 0.15;
        }
        self
    }
}

/// Graphic EQ configuration with 10 ISO bands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Per-band gain in dB [-18, +18]. Keys are frequency strings: "31", "62", ..., "16000".
    #[serde(default)]
    pub bands_db: BTreeMap<String, f32>,
    /// Filter Q factor [0.25, 4].
    #[serde(default = "default_q")]
    pub q: f32,
    /// Output trim in dB [-24, +12].
    #[serde(default)]
    pub output_db: f32,
    /// Tube coloration applied after EQ.
    #[serde(default)]
    pub tube_color: TubeColorConfig,
}

fn default_q() -> f32 {
    1.0
}

impl Default for EqConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bands_db: BTreeMap::new(),
            q: 1.0,
            output_db: 0.0,
            tube_color: TubeColorConfig::default(),
        }
    }
}

impl EqConfig {
    pub fn sanitized(mut self) -> Self {
        // Clamp and validate bands
        self.bands_db.retain(|k, v| {
            // Only allow valid ISO frequencies
            let valid = EQ_BAND_FREQS.iter().any(|&f| f.to_string() == *k);
            valid && v.is_finite()
        });
        for v in self.bands_db.values_mut() {
            *v = v.clamp(-18.0, 18.0);
        }
        // Handle non-finite values BEFORE clamping to ensure safe defaults
        if !self.q.is_finite() {
            self.q = 1.0;
        } else {
            self.q = self.q.clamp(0.25, 4.0);
        }
        if !self.output_db.is_finite() {
            self.output_db = 0.0;
        } else {
            self.output_db = self.output_db.clamp(-24.0, 12.0);
        }
        self.tube_color = self.tube_color.sanitized();
        self
    }
}

/// Reverb send configuration (per sound).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReverbSendConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Target reverb bus name.
    #[serde(default)]
    pub bus: String,
    /// Send level in dB [-120, +6]. -120 dB = silence/bypass.
    #[serde(default = "default_send_db")]
    pub send_db: f32,
}

fn default_send_db() -> f32 {
    -120.0
}

impl Default for ReverbSendConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bus: String::new(),
            send_db: -120.0,
        }
    }
}

impl ReverbSendConfig {
    pub fn sanitized(mut self) -> Self {
        self.send_db = self.send_db.clamp(-120.0, 6.0);
        if !self.send_db.is_finite() {
            self.send_db = -120.0;
        }
        self
    }
}

/// Reverb bus configuration (shared across multiple sounds).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReverbBusConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Pre-delay in milliseconds [0, 250].
    #[serde(default)]
    pub pre_delay_ms: f32,
    /// Decay time (RT60) in seconds [0.05, 10].
    #[serde(default = "default_decay_s")]
    pub decay_s: f32,
    /// High-frequency damping [0, 1].
    #[serde(default = "default_damping")]
    pub damping: f32,
    /// Low-cut filter frequency in Hz [20, 2000].
    #[serde(default = "default_low_cut")]
    pub low_cut_hz: f32,
    /// High-cut filter frequency in Hz [1000, 20000].
    #[serde(default = "default_high_cut")]
    pub high_cut_hz: f32,
    /// Stereo width [0, 1]. 0 = mono, 1 = full stereo.
    #[serde(default = "default_width")]
    pub stereo_width: f32,
    /// Wet output level in dB [-120, +6].
    #[serde(default = "default_wet_db")]
    pub wet_db: f32,
}

fn default_decay_s() -> f32 {
    0.55
}

fn default_damping() -> f32 {
    0.58
}

fn default_low_cut() -> f32 {
    180.0
}

fn default_high_cut() -> f32 {
    6800.0
}

fn default_width() -> f32 {
    0.75
}

fn default_wet_db() -> f32 {
    -3.0
}

impl Default for ReverbBusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pre_delay_ms: 8.0,
            decay_s: 0.55,
            damping: 0.58,
            low_cut_hz: 180.0,
            high_cut_hz: 6800.0,
            stereo_width: 0.75,
            wet_db: -3.0,
        }
    }
}

impl ReverbBusConfig {
    pub fn sanitized(mut self) -> Self {
        self.pre_delay_ms = self.pre_delay_ms.clamp(0.0, 250.0);
        self.decay_s = self.decay_s.clamp(0.05, 10.0);
        self.damping = self.damping.clamp(0.0, 1.0);
        self.low_cut_hz = self.low_cut_hz.clamp(20.0, 2000.0);
        self.high_cut_hz = self.high_cut_hz.clamp(1000.0, 20000.0);
        // Ensure high_cut > low_cut
        if self.high_cut_hz <= self.low_cut_hz {
            self.high_cut_hz = self.low_cut_hz + 100.0;
        }
        self.stereo_width = self.stereo_width.clamp(0.0, 1.0);
        self.wet_db = self.wet_db.clamp(-120.0, 6.0);
        // Finite checks
        if !self.pre_delay_ms.is_finite() {
            self.pre_delay_ms = 0.0;
        }
        if !self.decay_s.is_finite() {
            self.decay_s = 0.55;
        }
        if !self.damping.is_finite() {
            self.damping = 0.58;
        }
        if !self.low_cut_hz.is_finite() {
            self.low_cut_hz = 180.0;
        }
        if !self.high_cut_hz.is_finite() {
            self.high_cut_hz = 6800.0;
        }
        if !self.stereo_width.is_finite() {
            self.stereo_width = 0.75;
        }
        if !self.wet_db.is_finite() {
            self.wet_db = -3.0;
        }
        self
    }
}

/// Master output configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MasterConfig {
    /// Output trim in dB [-24, +12].
    #[serde(default)]
    pub output_db: f32,
    /// Limiter threshold [0, 1].
    #[serde(default = "default_limiter_threshold")]
    pub limiter_threshold: f32,
    /// Saturation amount [0, 1].
    #[serde(default = "default_saturation")]
    pub saturation: f32,
}

fn default_limiter_threshold() -> f32 {
    0.90
}

fn default_saturation() -> f32 {
    0.06
}

impl Default for MasterConfig {
    fn default() -> Self {
        Self {
            output_db: 0.0,
            limiter_threshold: 0.90,
            saturation: 0.06,
        }
    }
}

impl MasterConfig {
    pub fn sanitized(mut self) -> Self {
        self.output_db = self.output_db.clamp(-24.0, 12.0);
        self.limiter_threshold = self.limiter_threshold.clamp(0.0, 1.0);
        self.saturation = self.saturation.clamp(0.0, 1.0);
        if !self.output_db.is_finite() {
            self.output_db = 0.0;
        }
        if !self.limiter_threshold.is_finite() {
            self.limiter_threshold = 0.90;
        }
        if !self.saturation.is_finite() {
            self.saturation = 0.12;
        }
        self
    }
}

/// Hot reload configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HotReloadConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Polling interval in milliseconds [50, 5000].
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u32,
    /// Transition/crossfade time in milliseconds [0, 500].
    #[serde(default = "default_transition_ms")]
    pub transition_ms: u32,
}

fn default_poll_interval() -> u32 {
    250
}

fn default_transition_ms() -> u32 {
    30
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            poll_interval_ms: 250,
            transition_ms: 30,
        }
    }
}

impl HotReloadConfig {
    pub fn sanitized(mut self) -> Self {
        self.poll_interval_ms = self.poll_interval_ms.clamp(50, 5000);
        self.transition_ms = self.transition_ms.clamp(0, 500);
        self
    }
}

/// Per-sound DSP configuration (volume, pan, ADSR, EQ, reverb send).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundConfig {
    /// Linear volume [0, 2]. Values > 1 allow boost; master/headroom protects.
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Stereo pan [-1, 1]. Equal-power law: L=cos((pan+1)PI/4), R=sin((pan+1)PI/4).
    #[serde(default)]
    pub pan: f32,
    /// ADSR envelope.
    #[serde(default)]
    pub adsr: AdsrConfig,
    /// Graphic EQ with tube color.
    #[serde(default)]
    pub eq: EqConfig,
    /// Reverb send to a shared bus.
    #[serde(default)]
    pub reverb: ReverbSendConfig,
}

fn default_volume() -> f32 {
    1.0
}

/// JSON-facing per-sound configuration. `None` means "inherit"; explicit
/// values such as `0.0`, `1.0`, or `false` remain real overrides.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawSoundConfig {
    pub volume: Option<f32>,
    pub pan: Option<f32>,
    pub adsr: Option<RawAdsrConfig>,
    pub eq: Option<RawEqConfig>,
    pub reverb: Option<RawReverbSendConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawAdsrConfig {
    pub enabled: Option<bool>,
    pub attack_ms: Option<f32>,
    pub decay_ms: Option<f32>,
    pub sustain: Option<f32>,
    pub release_ms: Option<f32>,
    pub curve: Option<AdsrCurve>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawTubeColorConfig {
    pub enabled: Option<bool>,
    pub amount: Option<f32>,
    pub boost_link: Option<f32>,
    pub bias: Option<f32>,
    pub mix: Option<f32>,
    pub auto_gain: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawEqConfig {
    pub enabled: Option<bool>,
    pub bands_db: Option<BTreeMap<String, f32>>,
    pub q: Option<f32>,
    pub output_db: Option<f32>,
    pub tube_color: Option<RawTubeColorConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RawReverbSendConfig {
    pub enabled: Option<bool>,
    pub bus: Option<String>,
    pub send_db: Option<f32>,
}

impl RawSoundConfig {
    fn resolve_over(&self, base: &SoundConfig) -> SoundConfig {
        let mut out = base.clone();
        if let Some(value) = self.volume {
            out.volume = value;
        }
        if let Some(value) = self.pan {
            out.pan = value;
        }
        if let Some(raw) = &self.adsr {
            if let Some(value) = raw.enabled {
                out.adsr.enabled = value;
            }
            if let Some(value) = raw.attack_ms {
                out.adsr.attack_ms = value;
            }
            if let Some(value) = raw.decay_ms {
                out.adsr.decay_ms = value;
            }
            if let Some(value) = raw.sustain {
                out.adsr.sustain = value;
            }
            if let Some(value) = raw.release_ms {
                out.adsr.release_ms = value;
            }
            if let Some(value) = raw.curve {
                out.adsr.curve = value;
            }
        }
        if let Some(raw) = &self.eq {
            if let Some(value) = raw.enabled {
                out.eq.enabled = value;
            }
            if let Some(value) = &raw.bands_db {
                out.eq.bands_db = value.clone();
            }
            if let Some(value) = raw.q {
                out.eq.q = value;
            }
            if let Some(value) = raw.output_db {
                out.eq.output_db = value;
            }
            if let Some(tube) = &raw.tube_color {
                if let Some(value) = tube.enabled {
                    out.eq.tube_color.enabled = value;
                }
                if let Some(value) = tube.amount {
                    out.eq.tube_color.amount = value;
                }
                if let Some(value) = tube.boost_link {
                    out.eq.tube_color.boost_link = value;
                }
                if let Some(value) = tube.bias {
                    out.eq.tube_color.bias = value;
                }
                if let Some(value) = tube.mix {
                    out.eq.tube_color.mix = value;
                }
                if let Some(value) = tube.auto_gain {
                    out.eq.tube_color.auto_gain = value;
                }
            }
        }
        if let Some(raw) = &self.reverb {
            if let Some(value) = raw.enabled {
                out.reverb.enabled = value;
            }
            if let Some(value) = &raw.bus {
                out.reverb.bus = value.clone();
            }
            if let Some(value) = raw.send_db {
                out.reverb.send_db = value;
            }
        }
        out.sanitized()
    }
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: 0.0,
            adsr: AdsrConfig::default(),
            eq: EqConfig::default(),
            reverb: ReverbSendConfig::default(),
        }
    }
}

impl SoundConfig {
    /// Merge with defaults (inheritance). Missing fields take default values.
    pub fn merge_with_defaults(&self, defaults: &SoundConfig) -> Self {
        // For v2, we do field-level inheritance. For now, simple override.
        // In a full implementation, each field would be Option<T> and merged.
        Self {
            volume: if self.volume != 1.0 {
                self.volume
            } else {
                defaults.volume
            },
            pan: if self.pan != 0.0 {
                self.pan
            } else {
                defaults.pan
            },
            adsr: if self.adsr.enabled {
                self.adsr
            } else {
                defaults.adsr
            },
            eq: if self.eq.enabled {
                self.eq.clone()
            } else {
                defaults.eq.clone()
            },
            reverb: if self.reverb.enabled {
                self.reverb.clone()
            } else {
                defaults.reverb.clone()
            },
        }
    }

    pub fn sanitized(mut self) -> Self {
        self.volume = self.volume.clamp(0.0, 2.0);
        self.pan = self.pan.clamp(-1.0, 1.0);
        if !self.volume.is_finite() {
            self.volume = 1.0;
        }
        if !self.pan.is_finite() {
            self.pan = 0.0;
        }
        self.adsr = self.adsr.sanitized();
        self.eq = self.eq.sanitized();
        self.reverb = self.reverb.sanitized();
        self
    }
}

/// Runtime parameters for the exhaust microphone layer.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ExhaustConfig {
    /// If false, the exhaust layer is silent.
    pub enabled: bool,
    /// Base linear gain of the exhaust sample (0..1).
    pub base_gain: f32,
    /// How much throttle modulates the exhaust amplitude (0..1).
    pub throttle_sensitivity: f32,
    /// Reserved for additive gas-flow noise.
    pub gas_flow_gain: f32,
    /// Extra gain added during backfire events.
    pub crackle_gain: f32,
}

impl ExhaustConfig {
    pub fn default_runtime() -> Self {
        Self {
            enabled: true,
            base_gain: 0.7,
            throttle_sensitivity: 0.8,
            gas_flow_gain: 0.0,
            crackle_gain: 0.6,
        }
    }
}

/// Raw ``"exhaust"`` section as it appears in JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsonExhaustConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub base_gain: Option<f32>,
    #[serde(default)]
    pub throttle_sensitivity: Option<f32>,
    #[serde(default)]
    pub gas_flow_gain: Option<f32>,
    #[serde(default)]
    pub crackle_gain: Option<f32>,
}

impl JsonExhaustConfig {
    pub fn to_runtime(&self) -> ExhaustConfig {
        let mut cfg = ExhaustConfig::default_runtime();
        if let Some(v) = self.enabled {
            cfg.enabled = v;
        }
        if let Some(v) = self.base_gain {
            cfg.base_gain = v.clamp(0.0, 1.0);
        }
        if let Some(v) = self.throttle_sensitivity {
            cfg.throttle_sensitivity = v.clamp(0.0, 1.0);
        }
        if let Some(v) = self.gas_flow_gain {
            cfg.gas_flow_gain = v.clamp(0.0, 1.0);
        }
        if let Some(v) = self.crackle_gain {
            cfg.crackle_gain = v.clamp(0.0, 1.0);
        }
        cfg
    }
}

/// Schema v1 (legacy): simple gains map + exhaust.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SoundMixerConfigV1 {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub gains: BTreeMap<String, f32>,
    #[serde(default)]
    pub exhaust: JsonExhaustConfig,
}

/// Schema v2 (full sound design): defaults, per-sound config, reverb buses, master.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SoundMixerConfigV2 {
    pub schema_version: u32,
    /// Default configuration inherited by all sounds.
    #[serde(default)]
    pub defaults: SoundConfig,
    /// Per-sound configuration (bank key -> config).
    #[serde(default)]
    pub sounds: BTreeMap<String, SoundConfig>,
    /// Shared reverb buses (bus name -> config).
    #[serde(default)]
    pub reverb_buses: BTreeMap<String, ReverbBusConfig>,
    /// Exhaust microphone layer (behaviour-driven).
    #[serde(default)]
    pub exhaust: JsonExhaustConfig,
    /// Master output section.
    #[serde(default)]
    pub master: MasterConfig,
    /// Hot reload settings.
    #[serde(default)]
    pub hot_reload: HotReloadConfig,
}

/// Deserialized schema-v2 document before inheritance and cross-reference
/// validation. Runtime code must consume `SoundMixerConfigV2`, never this type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawSoundMixerConfigV2 {
    pub schema_version: u32,
    #[serde(default)]
    pub defaults: RawSoundConfig,
    #[serde(default)]
    pub sounds: BTreeMap<String, RawSoundConfig>,
    #[serde(default)]
    pub reverb_buses: BTreeMap<String, ReverbBusConfig>,
    #[serde(default)]
    pub exhaust: JsonExhaustConfig,
    #[serde(default)]
    pub master: MasterConfig,
    #[serde(default)]
    pub hot_reload: HotReloadConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ConfigLoadResult {
    pub resolved: SoundMixerConfig,
    pub warnings: Vec<ConfigDiagnostic>,
    pub errors: Vec<ConfigDiagnostic>,
    pub source_hash: String,
    pub schema_version: u32,
}

impl ConfigLoadResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Owner-thread storage for startup and hot reload. An invalid candidate is
/// reported but never replaces the last valid snapshot.
#[derive(Debug, Clone, Default)]
pub struct ConfigSnapshotStore {
    current: Option<ConfigLoadResult>,
    generation: u64,
}

impl ConfigSnapshotStore {
    pub fn current(&self) -> Option<&ConfigLoadResult> {
        self.current.as_ref()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn try_update(&mut self, raw: &str, bank_keys: &[String]) -> ConfigLoadResult {
        let candidate = resolve_config_json(raw, bank_keys);
        if candidate.is_valid() {
            let changed = self
                .current
                .as_ref()
                .is_none_or(|current| current.source_hash != candidate.source_hash);
            if changed {
                self.generation = self.generation.saturating_add(1);
                self.current = Some(candidate.clone());
            }
        }
        candidate
    }
}

/// Unified config supporting both v1 and v2 schemas.
#[derive(Debug, Clone)]
pub enum SoundMixerConfig {
    V1(SoundMixerConfigV1),
    V2(SoundMixerConfigV2),
}

impl Default for SoundMixerConfig {
    fn default() -> Self {
        Self::V1(SoundMixerConfigV1::default())
    }
}

/// Parse, resolve inheritance for every bank key, sanitize, and validate a
/// configuration in one shared path used by runtime and tooling.
pub fn resolve_config_json(raw: &str, bank_keys: &[String]) -> ConfigLoadResult {
    use sha2::{Digest, Sha256};
    let source_hash = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(error) => {
            return ConfigLoadResult {
                resolved: SoundMixerConfig::default(),
                warnings,
                errors: vec![ConfigDiagnostic {
                    path: "$".into(),
                    message: format!("invalid JSON: {error}"),
                }],
                source_hash,
                schema_version: 1,
            }
        }
    };
    collect_unknown_keys(&value, &mut warnings);
    collect_range_diagnostics(&value, &mut warnings, &mut errors);
    let version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    let resolved = if version == 2 {
        match serde_json::from_value::<RawSoundMixerConfigV2>(value) {
            Ok(raw_v2) => {
                let defaults = raw_v2.defaults.resolve_over(&SoundConfig::default());
                let mut sounds = BTreeMap::new();
                for key in bank_keys.iter().chain(raw_v2.sounds.keys()) {
                    if sounds.contains_key(key) {
                        continue;
                    }
                    let mut sound = raw_v2.sounds.get(key).map_or_else(
                        || defaults.clone(),
                        |raw_sound| raw_sound.resolve_over(&defaults),
                    );
                    if sound.reverb.enabled && !raw_v2.reverb_buses.contains_key(&sound.reverb.bus)
                    {
                        warnings.push(ConfigDiagnostic {
                            path: format!("$.sounds.{key}.reverb.bus"),
                            message: format!(
                                "bus '{}' does not exist; send disabled",
                                sound.reverb.bus
                            ),
                        });
                        sound.reverb.enabled = false;
                        sound.reverb.send_db = -120.0;
                    }
                    sounds.insert(key.clone(), sound);
                }
                SoundMixerConfig::V2(SoundMixerConfigV2 {
                    schema_version: 2,
                    defaults,
                    sounds,
                    reverb_buses: raw_v2
                        .reverb_buses
                        .into_iter()
                        .map(|(k, v)| (k, v.sanitized()))
                        .collect(),
                    exhaust: raw_v2.exhaust,
                    master: raw_v2.master.sanitized(),
                    hot_reload: raw_v2.hot_reload.sanitized(),
                })
            }
            Err(error) => {
                errors.push(ConfigDiagnostic {
                    path: "$".into(),
                    message: error.to_string(),
                });
                SoundMixerConfig::default()
            }
        }
    } else {
        match serde_json::from_value::<SoundMixerConfigV1>(value) {
            Ok(cfg) => SoundMixerConfig::V1(cfg).sanitized(),
            Err(error) => {
                errors.push(ConfigDiagnostic {
                    path: "$".into(),
                    message: error.to_string(),
                });
                SoundMixerConfig::default()
            }
        }
    };
    ConfigLoadResult {
        resolved,
        warnings,
        errors,
        source_hash,
        schema_version: version,
    }
}

fn collect_unknown_keys(value: &serde_json::Value, warnings: &mut Vec<ConfigDiagnostic>) {
    fn check(
        object: Option<&serde_json::Map<String, serde_json::Value>>,
        allowed: &[&str],
        path: &str,
        out: &mut Vec<ConfigDiagnostic>,
    ) {
        if let Some(object) = object {
            for key in object.keys().filter(|key| !allowed.contains(&key.as_str())) {
                out.push(ConfigDiagnostic {
                    path: format!("{path}.{key}"),
                    message: "unknown key ignored".into(),
                });
            }
        }
    }
    let root = value.as_object();
    check(
        root,
        &[
            "schema_version",
            "gains",
            "defaults",
            "sounds",
            "reverb_buses",
            "exhaust",
            "master",
            "hot_reload",
        ],
        "$",
        warnings,
    );
    if value.get("schema_version").and_then(|v| v.as_u64()) != Some(2) {
        return;
    }
    check(
        value.get("defaults").and_then(|v| v.as_object()),
        &["volume", "pan", "adsr", "eq", "reverb"],
        "$.defaults",
        warnings,
    );
    check_sound_children(value.get("defaults"), "$.defaults", warnings, &check);
    if let Some(sounds) = value.get("sounds").and_then(|v| v.as_object()) {
        for (key, sound) in sounds {
            let path = format!("$.sounds.{key}");
            check(
                sound.as_object(),
                &["volume", "pan", "adsr", "eq", "reverb"],
                &path,
                warnings,
            );
            check_sound_children(Some(sound), &path, warnings, &check);
        }
    }
    if let Some(buses) = value.get("reverb_buses").and_then(|v| v.as_object()) {
        for (name, bus) in buses {
            check(
                bus.as_object(),
                &[
                    "enabled",
                    "pre_delay_ms",
                    "decay_s",
                    "damping",
                    "low_cut_hz",
                    "high_cut_hz",
                    "stereo_width",
                    "wet_db",
                ],
                &format!("$.reverb_buses.{name}"),
                warnings,
            );
        }
    }
    check(
        value.get("master").and_then(|v| v.as_object()),
        &["output_db", "limiter_threshold", "saturation"],
        "$.master",
        warnings,
    );
    check(
        value.get("hot_reload").and_then(|v| v.as_object()),
        &["enabled", "poll_interval_ms", "transition_ms"],
        "$.hot_reload",
        warnings,
    );
    check(
        value.get("exhaust").and_then(|v| v.as_object()),
        &[
            "enabled",
            "base_gain",
            "throttle_sensitivity",
            "gas_flow_gain",
            "crackle_gain",
        ],
        "$.exhaust",
        warnings,
    );
}

fn check_sound_children<F>(
    value: Option<&serde_json::Value>,
    path: &str,
    warnings: &mut Vec<ConfigDiagnostic>,
    check: &F,
) where
    F: Fn(
        Option<&serde_json::Map<String, serde_json::Value>>,
        &[&str],
        &str,
        &mut Vec<ConfigDiagnostic>,
    ),
{
    let Some(sound) = value else { return };
    check(
        sound.get("adsr").and_then(|v| v.as_object()),
        &[
            "enabled",
            "attack_ms",
            "decay_ms",
            "sustain",
            "release_ms",
            "curve",
        ],
        &format!("{path}.adsr"),
        warnings,
    );
    check(
        sound.get("eq").and_then(|v| v.as_object()),
        &["enabled", "bands_db", "q", "output_db", "tube_color"],
        &format!("{path}.eq"),
        warnings,
    );
    check(
        sound
            .get("eq")
            .and_then(|v| v.get("tube_color"))
            .and_then(|v| v.as_object()),
        &[
            "enabled",
            "amount",
            "boost_link",
            "bias",
            "mix",
            "auto_gain",
        ],
        &format!("{path}.eq.tube_color"),
        warnings,
    );
    check(
        sound.get("reverb").and_then(|v| v.as_object()),
        &["enabled", "bus", "send_db"],
        &format!("{path}.reverb"),
        warnings,
    );
}

fn collect_range_diagnostics(
    value: &serde_json::Value,
    warnings: &mut Vec<ConfigDiagnostic>,
    errors: &mut Vec<ConfigDiagnostic>,
) {
    fn number(
        value: Option<&serde_json::Value>,
        path: &str,
        min: f64,
        max: f64,
        warnings: &mut Vec<ConfigDiagnostic>,
        errors: &mut Vec<ConfigDiagnostic>,
    ) {
        let Some(value) = value else { return };
        let Some(number) = value.as_f64() else {
            errors.push(ConfigDiagnostic {
                path: path.into(),
                message: "expected finite number".into(),
            });
            return;
        };
        if !(min..=max).contains(&number) {
            warnings.push(ConfigDiagnostic {
                path: path.into(),
                message: format!("{number} outside [{min},{max}]; clamped"),
            });
        }
    }
    fn sound(
        value: Option<&serde_json::Value>,
        path: &str,
        warnings: &mut Vec<ConfigDiagnostic>,
        errors: &mut Vec<ConfigDiagnostic>,
    ) {
        let Some(value) = value else { return };
        number(
            value.get("volume"),
            &format!("{path}.volume"),
            0.0,
            2.0,
            warnings,
            errors,
        );
        number(
            value.get("pan"),
            &format!("{path}.pan"),
            -1.0,
            1.0,
            warnings,
            errors,
        );
        if let Some(tube) = value.get("eq").and_then(|eq| eq.get("tube_color")) {
            for field in ["amount", "boost_link", "mix"] {
                number(
                    tube.get(field),
                    &format!("{path}.eq.tube_color.{field}"),
                    0.0,
                    1.0,
                    warnings,
                    errors,
                );
            }
            number(
                tube.get("bias"),
                &format!("{path}.eq.tube_color.bias"),
                0.0,
                0.5,
                warnings,
                errors,
            );
        }
    }
    sound(value.get("defaults"), "$.defaults", warnings, errors);
    if let Some(sounds) = value.get("sounds").and_then(|v| v.as_object()) {
        for (key, entry) in sounds {
            sound(Some(entry), &format!("$.sounds.{key}"), warnings, errors);
        }
    }
    if let Some(master) = value.get("master") {
        number(
            master.get("output_db"),
            "$.master.output_db",
            -24.0,
            12.0,
            warnings,
            errors,
        );
        number(
            master.get("limiter_threshold"),
            "$.master.limiter_threshold",
            0.0,
            1.0,
            warnings,
            errors,
        );
        number(
            master.get("saturation"),
            "$.master.saturation",
            0.0,
            1.0,
            warnings,
            errors,
        );
    }
}

impl SoundMixerConfig {
    /// Load from `<bank_dir>/../../sound_mixer_config.json`.
    pub fn load_from_bank_dir(bank_dir: &Path) -> Self {
        Self::load_result_from_bank_dir(bank_dir, &[]).resolved
    }

    pub fn load_result_from_bank_dir(bank_dir: &Path, bank_keys: &[String]) -> ConfigLoadResult {
        let Some(sounds_dir) = bank_dir.parent().and_then(|p| p.parent()) else {
            return ConfigLoadResult {
                resolved: Self::default(),
                warnings: vec![ConfigDiagnostic {
                    path: "$".into(),
                    message: "cannot resolve sounds directory; using safe defaults".into(),
                }],
                errors: Vec::new(),
                source_hash: String::new(),
                schema_version: 1,
            };
        };
        let path = sounds_dir.join(MIXER_CONFIG_FILENAME);
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(error) => {
                return ConfigLoadResult {
                    resolved: Self::default(),
                    warnings: vec![ConfigDiagnostic {
                        path: path.display().to_string(),
                        message: format!("{error}; using safe defaults"),
                    }],
                    errors: Vec::new(),
                    source_hash: String::new(),
                    schema_version: 1,
                }
            }
        };
        resolve_config_json(&raw, bank_keys)
    }

    /// Sanitize and validate all values.
    pub fn sanitized(self) -> Self {
        match self {
            Self::V1(mut cfg) => {
                cfg.gains.retain(|_, g| g.is_finite());
                for g in cfg.gains.values_mut() {
                    *g = g.clamp(0.0, 1.0);
                }
                Self::V1(cfg)
            }
            Self::V2(mut cfg) => {
                cfg.defaults = cfg.defaults.sanitized();
                for sound in cfg.sounds.values_mut() {
                    *sound = sound.clone().sanitized();
                }
                for bus in cfg.reverb_buses.values_mut() {
                    *bus = bus.sanitized();
                }
                cfg.master = cfg.master.sanitized();
                cfg.hot_reload = cfg.hot_reload.sanitized();
                Self::V2(cfg)
            }
        }
    }

    /// Get gain for a bank key (v1 compatibility).
    pub fn gain(&self, key: &str) -> f32 {
        match self {
            Self::V1(cfg) => cfg.gains.get(key).copied().unwrap_or(1.0),
            Self::V2(cfg) => cfg
                .sounds
                .get(key)
                .map(|s| s.volume)
                .unwrap_or(cfg.defaults.volume),
        }
    }

    /// Extract gains as a BTreeMap for uniform access.
    /// For V1: returns the gains field directly.
    /// For V2: builds a map from sounds where each key maps to its volume.
    pub fn gains_map(&self) -> std::collections::BTreeMap<String, f32> {
        match self {
            Self::V1(cfg) => cfg.gains.clone(),
            Self::V2(cfg) => {
                let mut map = std::collections::BTreeMap::new();
                for (key, sound) in &cfg.sounds {
                    map.insert(key.clone(), sound.volume);
                }
                map
            }
        }
    }

    /// Exhaust runtime config.
    pub fn exhaust_config(&self) -> ExhaustConfig {
        match self {
            Self::V1(cfg) => cfg.exhaust.to_runtime(),
            Self::V2(cfg) => cfg.exhaust.to_runtime(),
        }
    }

    /// Schema version.
    pub fn schema_version(&self) -> u32 {
        match self {
            Self::V1(_) => 1,
            Self::V2(_) => 2,
        }
    }

    /// Convert v1 to v2 (migration).
    pub fn to_v2(&self) -> SoundMixerConfigV2 {
        match self {
            Self::V1(cfg) => {
                let mut sounds = BTreeMap::new();
                for (key, gain) in &cfg.gains {
                    sounds.insert(
                        key.clone(),
                        SoundConfig {
                            volume: *gain,
                            ..SoundConfig::default()
                        },
                    );
                }
                SoundMixerConfigV2 {
                    schema_version: 2,
                    defaults: SoundConfig::default(),
                    sounds,
                    reverb_buses: BTreeMap::new(),
                    exhaust: cfg.exhaust.clone(),
                    // Equal-power center pan is -3.0103 dB per channel. Restore
                    // legacy mono-to-both-channel level for bypass migrations.
                    master: MasterConfig {
                        output_db: 3.010_3,
                        ..MasterConfig::default()
                    },
                    hot_reload: HotReloadConfig::default(),
                }
            }
            Self::V2(cfg) => cfg.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DirGuard(std::path::PathBuf);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn make_bank_dir() -> (DirGuard, std::path::PathBuf) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("f90_audio_cfg_{unique}"));
        let bank = root.join("banks").join("v10_vehicle");
        std::fs::create_dir_all(&bank).expect("create temp bank dir");
        (DirGuard(root), bank)
    }

    fn write_config(parent: &std::path::Path, body: &str) {
        std::fs::write(parent.join(MIXER_CONFIG_FILENAME), body).expect("write test config");
    }

    #[test]
    fn v1_missing_file_yields_empty_config() {
        let (_guard, bank) = make_bank_dir();
        let cfg = SoundMixerConfig::load_from_bank_dir(&bank);
        assert_eq!(cfg.schema_version(), 1);
        assert_eq!(cfg.gain("int_backfire"), 1.0);
    }

    #[test]
    fn v1_parses_gains() {
        let (_guard, bank) = make_bank_dir();
        let root = bank.parent().unwrap().parent().unwrap();
        write_config(
            root,
            r#"{"schema_version": 1, "gains": {"int_backfire": 0.5}}"#,
        );
        let cfg = SoundMixerConfig::load_from_bank_dir(&bank).sanitized();
        assert_eq!(cfg.schema_version(), 1);
        assert_eq!(cfg.gain("int_backfire"), 0.5);
        assert_eq!(cfg.gain("unknown"), 1.0);
    }

    #[test]
    fn v2_parses_full_config() {
        let (_guard, bank) = make_bank_dir();
        let root = bank.parent().unwrap().parent().unwrap();
        write_config(
            root,
            r#"{
                "schema_version": 2,
                "defaults": {"volume": 0.9, "pan": 0.0},
                "sounds": {
                    "engine_mid": {"volume": 0.8, "pan": -0.1}
                },
                "reverb_buses": {
                    "sfx_short": {"pre_delay_ms": 10, "decay_s": 0.6}
                },
                "master": {"output_db": -1.0}
            }"#,
        );
        let cfg = SoundMixerConfig::load_from_bank_dir(&bank).sanitized();
        assert_eq!(cfg.schema_version(), 2);
        assert_eq!(cfg.gain("engine_mid"), 0.8);
        if let SoundMixerConfig::V2(v2) = &cfg {
            assert_eq!(v2.defaults.volume, 0.9);
            assert!(v2.reverb_buses.contains_key("sfx_short"));
            assert_eq!(v2.master.output_db, -1.0);
        } else {
            panic!("Expected V2");
        }
    }

    #[test]
    fn v2_sanitizes_out_of_range() {
        let cfg = SoundMixerConfigV2 {
            schema_version: 2,
            defaults: SoundConfig {
                volume: 3.0,
                pan: 2.0,
                ..SoundConfig::default()
            },
            sounds: BTreeMap::new(),
            reverb_buses: BTreeMap::new(),
            exhaust: JsonExhaustConfig::default(),
            master: MasterConfig {
                output_db: 20.0,
                ..MasterConfig::default()
            },
            hot_reload: HotReloadConfig::default(),
        };
        let sanitized = SoundMixerConfig::V2(cfg).sanitized();
        if let SoundMixerConfig::V2(v2) = sanitized {
            assert_eq!(v2.defaults.volume, 2.0);
            assert_eq!(v2.defaults.pan, 1.0);
            assert_eq!(v2.master.output_db, 12.0);
        } else {
            panic!("Expected V2");
        }
    }

    #[test]
    fn v1_to_v2_migration() {
        let v1 = SoundMixerConfig::V1(SoundMixerConfigV1 {
            schema_version: 1,
            gains: BTreeMap::from([
                ("engine_idle".to_string(), 0.8),
                ("shift_up".to_string(), 0.6),
            ]),
            exhaust: JsonExhaustConfig::default(),
        });
        let v2 = v1.to_v2();
        assert_eq!(v2.schema_version, 2);
        assert_eq!(v2.sounds.get("engine_idle").unwrap().volume, 0.8);
        assert_eq!(v2.sounds.get("shift_up").unwrap().volume, 0.6);
    }

    #[test]
    fn adsr_sanitized() {
        let adsr = AdsrConfig {
            enabled: true,
            attack_ms: 15000.0,
            decay_ms: -5.0,
            sustain: 1.5,
            release_ms: f32::NAN,
            curve: AdsrCurve::Exponential,
        }
        .sanitized();
        assert_eq!(adsr.attack_ms, 10000.0);
        assert_eq!(adsr.decay_ms, 0.0);
        assert_eq!(adsr.sustain, 1.0);
        assert_eq!(adsr.release_ms, 5.8);
    }

    #[test]
    fn eq_validates_bands() {
        let mut bands = BTreeMap::new();
        bands.insert("1000".to_string(), 20.0); // Out of range, clamped to 18.0
        bands.insert("999".to_string(), 5.0); // Invalid freq, removed
        bands.insert("250".to_string(), -10.0); // Valid
        let eq = EqConfig {
            enabled: true,
            bands_db: bands,
            q: 5.0, // Out of range
            output_db: f32::INFINITY,
            tube_color: TubeColorConfig::default(),
        }
        .sanitized();
        assert_eq!(eq.bands_db.len(), 2); // "1000" and "250" retained
        assert_eq!(*eq.bands_db.get("250").unwrap(), -10.0);
        assert_eq!(*eq.bands_db.get("1000").unwrap(), 18.0); // Clamped from 20.0
        assert_eq!(eq.q, 4.0);
        assert_eq!(eq.output_db, 0.0);
    }

    #[test]
    fn reverb_bus_validated() {
        let bus = ReverbBusConfig {
            pre_delay_ms: 500.0,
            decay_s: 0.01,
            low_cut_hz: 5000.0,
            high_cut_hz: 3000.0, // Invalid: < low_cut
            ..ReverbBusConfig::default()
        }
        .sanitized();
        assert_eq!(bus.pre_delay_ms, 250.0);
        assert_eq!(bus.decay_s, 0.05);
        assert!(bus.high_cut_hz > bus.low_cut_hz);
    }

    #[test]
    fn raw_resolution_distinguishes_omitted_from_explicit_defaults() {
        let keys = vec!["omitted".to_string(), "explicit".to_string()];
        let result = resolve_config_json(
            r#"{
            "schema_version": 2,
            "defaults": {
                "volume": 0.4,
                "pan": 0.7,
                "eq": {"tube_color": {"enabled": true, "amount": 0.8, "mix": 0.5}}
            },
            "sounds": {
                "explicit": {
                    "volume": 1.0,
                    "pan": 0.0,
                    "eq": {"tube_color": {"enabled": false, "amount": 0.0, "mix": 0.0}}
                }
            }
        }"#,
            &keys,
        );
        assert!(result.is_valid());
        let SoundMixerConfig::V2(config) = result.resolved else {
            panic!("expected v2")
        };
        let omitted = &config.sounds["omitted"];
        assert_eq!(omitted.volume, 0.4);
        assert_eq!(omitted.pan, 0.7);
        assert!(omitted.eq.tube_color.enabled);
        let explicit = &config.sounds["explicit"];
        assert_eq!(explicit.volume, 1.0);
        assert_eq!(explicit.pan, 0.0);
        assert!(!explicit.eq.tube_color.enabled);
        assert_eq!(explicit.eq.tube_color.amount, 0.0);
        assert_eq!(explicit.eq.tube_color.mix, 0.0);
    }

    #[test]
    fn missing_bus_is_diagnosed_and_disabled_in_resolved_snapshot() {
        let result = resolve_config_json(
            r#"{
            "schema_version": 2,
            "sounds": {"shot": {"reverb": {"enabled": true, "bus": "missing", "send_db": -3}}}
        }"#,
            &["shot".into()],
        );
        assert!(result
            .warnings
            .iter()
            .any(|d| d.path == "$.sounds.shot.reverb.bus"));
        let SoundMixerConfig::V2(config) = result.resolved else {
            panic!("expected v2")
        };
        assert!(!config.sounds["shot"].reverb.enabled);
        assert_eq!(config.sounds["shot"].reverb.send_db, -120.0);
    }

    #[test]
    fn unknown_sound_key_reports_json_path() {
        let result = resolve_config_json(
            r#"{
            "schema_version": 2,
            "sounds": {"shot": {"volumee": 0.5}}
        }"#,
            &[],
        );
        assert!(result
            .warnings
            .iter()
            .any(|d| d.path == "$.sounds.shot.volumee"));
    }

    #[test]
    fn invalid_candidate_preserves_last_valid_snapshot() {
        let mut store = ConfigSnapshotStore::default();
        let first = store.try_update(
            r#"{"schema_version":2,"defaults":{"volume":0.25}}"#,
            &["engine".into()],
        );
        assert!(first.is_valid());
        assert_eq!(store.generation(), 1);
        let rejected = store.try_update("{partial", &["engine".into()]);
        assert!(!rejected.is_valid());
        assert_eq!(store.generation(), 1);
        let Some(ConfigLoadResult {
            resolved: SoundMixerConfig::V2(current),
            ..
        }) = store.current()
        else {
            panic!("valid v2 snapshot expected")
        };
        assert_eq!(current.sounds["engine"].volume, 0.25);
    }

    #[test]
    fn clamps_and_unknown_sections_have_structured_diagnostics() {
        let result = resolve_config_json(
            r#"{
            "schema_version": 2,
            "defaults": {"volume": 9, "eq": {"tube_color": {"amount": 2, "mystery": 1}}},
            "master": {"limiter_threshold": -1, "unknown": true},
            "hot_reload": {"enabled": false, "surprise": 4}
        }"#,
            &["engine".into()],
        );
        for path in [
            "$.defaults.volume",
            "$.defaults.eq.tube_color.amount",
            "$.defaults.eq.tube_color.mystery",
            "$.master.limiter_threshold",
            "$.master.unknown",
            "$.hot_reload.surprise",
        ] {
            assert!(
                result
                    .warnings
                    .iter()
                    .any(|diagnostic| diagnostic.path == path),
                "missing diagnostic {path}"
            );
        }
    }

    #[test]
    fn contract_fixtures_cover_supported_and_invalid_documents() {
        let fixtures = [
            include_str!("../tests/fixtures/config_v1.json"),
            include_str!("../tests/fixtures/config_v2_minimal.json"),
            include_str!("../tests/fixtures/config_v2_full.json"),
        ];
        for fixture in fixtures {
            assert!(resolve_config_json(fixture, &["engine_idle".into()]).is_valid());
        }
        assert!(!resolve_config_json(
            include_str!("../tests/fixtures/config_v2_invalid.json"),
            &[]
        )
        .is_valid());
    }
}
