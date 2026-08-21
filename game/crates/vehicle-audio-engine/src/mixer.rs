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
use crate::state::*;

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
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            pitch_min: PITCH_MIN,
            pitch_max: PITCH_MAX,
            coast_gain: 0.45,
            throttle_gain: 0.50,
            saturation: 0.12,
            limiter_threshold: 0.90,
            engine_headroom: 0.62,
            attack_seconds: 0.035,
            release_seconds: 0.12,
            shift_gain: 0.80,
            bed_base: 0.25,
            bed_slip: 0.55,
            bed_speed: 0.12,
            scrape_gain: 0.62,
        }
    }
}

/// A one-shot voice (gear shifts, impacts) drawn from the bank.
struct OneShot {
    trigger: Trigger,
    key: String,
    cursor: usize,
    active: bool,
}

/// Short fade-in/out (samples) applied to one-shots to avoid click on start/end.
const ONE_SHOT_ENV_SAMPLES: usize = 256;

/// RPM glide time constant (seconds). Smooths the playback rate + band weights so
/// RPM changes glide continuously instead of stepping each frame (which produced
/// zipper/warble artifacts at transitions). One-pole; 25 ms is inaudible lag but
/// removes the per-frame discontinuity.
const RPM_SMOOTH_TAU: f64 = 0.025;

/// Stateful engine audio mixer.
pub struct VehicleAudioEngine {
    bank: VehicleSoundBank,
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
    backfire_cooldown_samples: usize,
    variant_rng_state: u64,
}

impl VehicleAudioEngine {
    /// Load the bank and build the mixer. `bank_dir` must contain `bank_manifest.json`.
    pub fn new(bank_dir: &Path) -> Result<Self, BankError> {
        let bank = VehicleSoundBank::load(bank_dir)?;
        let layer_keys: Vec<String> = ENGINE_BAND_KEYS.iter().map(|s| s.to_string()).collect();
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
                active: false,
            });
        }

        let sample_rate = bank
            .samples
            .values()
            .next()
            .map(|s| s.sample_rate)
            .unwrap_or(44100);

        Ok(Self {
            bank,
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
            cur_weights: [0.0; 5],
            cur_pitches: [1.0; 5],
            cur_engine_gain: 0.0,
            one_shots,
            sample_rate,
            cfg: AudioConfig::default(),
            last_norm: 0.0,
            last_rpm: 0.0,
            last_throttle: 0.0,
            last_speed_kph: 0.0,
            last_slip: 0.0,
            last_gear: 0,
            last_trigger: String::new(),
            smoothed_rpm: 0.0,
            target_rpm: 0.0,
            idle_rpm: 0.0,
            max_rpm: 0.0,
            backfire_cooldown_samples: 0,
            variant_rng_state: 0xF090_1994_D15C_A11D,
        })
    }

    pub fn config(&self) -> &AudioConfig {
        &self.cfg
    }

    pub fn set_config(&mut self, cfg: AudioConfig) {
        self.cfg = cfg;
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

    /// Feed the current vehicle telemetry. Computes the mix targets and fires
    /// gear-shift one-shots on gear changes.
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
        let norm = if max_rpm > idle_rpm {
            (((rpm - idle_rpm) / (max_rpm - idle_rpm)) as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let egain = engine_gain(throttle);
        let skey = surface_key(surface);
        let sgain = match skey {
            Some(_) => {
                self.cfg.bed_base
                    + self.cfg.bed_slip * slip.clamp(0.0, 1.0)
                    + self.cfg.bed_speed * (speed_kph.min(120.0) / 120.0) as f32
            }
            None => 0.0,
        };

        // RPM is stored as a target; `render` glides `smoothed_rpm` toward it per
        // sample so pitch + band weights move continuously (no per-frame step).
        self.target_rpm = rpm;
        self.idle_rpm = idle_rpm;
        self.max_rpm = max_rpm;
        self.cur_engine_gain = egain;

        self.target_engine_gain = egain;
        self.target_bed_gain = sgain;
        self.bed_key = skey.map(|s| s.to_string());

        if gear != self.last_gear {
            if self.last_gear != 0 {
                let t = if gear > self.last_gear {
                    Trigger::ShiftUp
                } else {
                    Trigger::ShiftDown
                };
                self.trigger(t);
            }
            self.last_gear = gear;
        }

        // Over-run backfire: sudden lift-off from high throttle at high RPM (> 12,000 RPM).
        if self.last_throttle >= 0.80
            && throttle <= 0.15
            && rpm >= 12000.0
            && self.backfire_cooldown_samples == 0
        {
            self.trigger(Trigger::Backfire);
            self.backfire_cooldown_samples = (self.sample_rate as f64 * 0.35) as usize;
        }

        self.last_norm = norm;
        self.last_rpm = rpm;
        self.last_throttle = throttle;
        self.last_speed_kph = speed_kph;
        self.last_slip = slip;
    }

    /// Fire one deterministic pseudo-random variant for the requested one-shot.
    pub fn trigger(&mut self, t: Trigger) {
        self.last_trigger = t.bank_key().to_string();
        let variant_count = self.one_shots.iter().filter(|o| o.trigger == t).count();
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
                o.active = ordinal == selected;
                if o.active {
                    o.cursor = 0;
                }
                ordinal += 1;
            }
        }
    }

    /// Render `n` stereo frames into `out_l`/`out_r`. Sample-accurate: engine
    /// layers and bed are read with a fractional cursor (no Godot resampler),
    /// gains are smoothed per-sample, and a tanh soft-clip limiter catches peaks.
    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32], n: usize) {
        self.backfire_cooldown_samples = self.backfire_cooldown_samples.saturating_sub(n);
        let sr = self.sample_rate as f64;
        let rpm_alpha = 1.0 - (-1.0 / (sr * RPM_SMOOTH_TAU)).exp();
        for i in 0..n {
            // Smooth RPM so pitch + band weights glide continuously (no per-frame
            // step -> no zipper/warble at RPM changes).
            self.smoothed_rpm += (self.target_rpm - self.smoothed_rpm) * rpm_alpha;

            // Per-sample gain smoothing (attack/release time constants).
            let (eg_tgt, eg_cur) = (self.target_engine_gain, self.smoothed_engine_gain);
            let eg_tau = if eg_tgt > eg_cur {
                self.cfg.attack_seconds
            } else {
                self.cfg.release_seconds
            };
            let eg_alpha = 1.0 - (-1.0 / (sr * eg_tau as f64)).exp();
            let eg = eg_cur + (eg_tgt - eg_cur) * eg_alpha as f32;
            self.smoothed_engine_gain = eg;

            let (bg_tgt, bg_cur) = (self.target_bed_gain, self.smoothed_bed_gain);
            let bg_tau = if bg_tgt > bg_cur {
                self.cfg.attack_seconds
            } else {
                self.cfg.release_seconds
            };
            let bg_alpha = 1.0 - (-1.0 / (sr * bg_tau as f64)).exp();
            let bg = bg_cur + (bg_tgt - bg_cur) * bg_alpha as f32;
            self.smoothed_bed_gain = bg;

            let scrape_tau = if self.target_scrape_gain > self.smoothed_scrape_gain {
                0.020
            } else {
                0.150
            };
            let scrape_alpha = 1.0 - (-1.0 / (sr * scrape_tau)).exp();
            self.smoothed_scrape_gain +=
                (self.target_scrape_gain - self.smoothed_scrape_gain) * scrape_alpha as f32;

            // Engine bands: weights + pitch derived continuously from smoothed_rpm.
            let norm = if self.max_rpm > self.idle_rpm {
                (((self.smoothed_rpm - self.idle_rpm) / (self.max_rpm - self.idle_rpm)) as f32)
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            let weights = engine_weights(norm);
            let mut acc = 0.0f32;
            for b in 0..self.layer_keys.len().min(5) {
                if let Some(sample) = self.bank.get(&self.layer_keys[b]) {
                    let ratio = engine_pitch_scale(self.smoothed_rpm, b) as f64;
                    let s = read_looped(&sample.pcm, &mut self.layer_cursors[b], ratio);
                    acc += weights[b] * s;
                }
            }
            let engine = acc * eg * self.cfg.engine_headroom;
            for (wi, w) in weights.iter().enumerate() {
                self.cur_weights[wi] = *w;
            }
            for b in 0..5 {
                self.cur_pitches[b] = engine_pitch_scale(self.smoothed_rpm, b);
            }

            // Surface bed (loops at native rate).
            let mut bed = 0.0f32;
            if let Some(bk) = &self.bed_key {
                if let Some(sample) = self.bank.get(bk) {
                    let s = read_looped(&sample.pcm, &mut self.bed_cursor, 1.0);
                    bed = s * bg;
                }
            }

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
                    ) * self.smoothed_scrape_gain;
                }
            }

            // One-shots (non-looping, short envelope).
            let mut os = 0.0f32;
            for o in self.one_shots.iter_mut() {
                if !o.active {
                    continue;
                }
                if let Some(sample) = self.bank.get(&o.key) {
                    let len = sample.pcm.len();
                    if len == 0 {
                        o.active = false;
                        continue;
                    }
                    let idx = o.cursor.min(len - 1);
                    let s = sample.pcm[idx] as f32 / 32768.0;
                    os += s * self.cfg.shift_gain * one_shot_env(o.cursor, len);
                    o.cursor += 1;
                    if o.cursor >= len {
                        o.active = false;
                    }
                } else {
                    o.active = false;
                }
            }

            let mixed = engine + bed + scrape + os;
            let out = self.limiter(mixed);
            if i < out_l.len() {
                out_l[i] = out;
            }
            if i < out_r.len() {
                out_r[i] = out;
            }
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
        }
    }

    fn engine_with_bank(bank: VehicleSoundBank) -> VehicleAudioEngine {
        let mut e = VehicleAudioEngine {
            bank,
            layer_keys: ENGINE_BAND_KEYS.iter().map(|s| s.to_string()).collect(),
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
            smoothed_rpm: 0.0,
            target_rpm: 0.0,
            idle_rpm: 0.0,
            max_rpm: 0.0,
            backfire_cooldown_samples: 0,
            variant_rng_state: 0xF090_1994_D15C_A11D,
        };
        e.one_shots.push(OneShot {
            trigger: Trigger::ShiftUp,
            key: "shift_up".to_string(),
            cursor: 0,
            active: false,
        });
        e.one_shots.push(OneShot {
            trigger: Trigger::Backfire,
            key: "int_backfire".to_string(),
            cursor: 0,
            active: false,
        });
        e.one_shots.push(OneShot {
            trigger: Trigger::Backfire,
            key: "int_backfire_2".to_string(),
            cursor: 0,
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
    fn pitch_glides_instead_of_stepping() {
        let mut e = engine_with_bank(dummy_bank());
        e.set_state(2000.0, 1000.0, 15000.0, 1.0, 0.0, 3, 0.0, "asphalt");
        e.render(&mut vec![0.0; 128], &mut vec![0.0; 128], 128);
        let p0 = e.last_pitches()[0];
        // Hard RPM jump; render only a few samples so the smoothed pitch must still
        // be near the old value (gliding), not snapped to the new target.
        e.set_state(14000.0, 1000.0, 15000.0, 1.0, 0.0, 3, 0.0, "asphalt");
        e.render(&mut vec![0.0; 4], &mut vec![0.0; 4], 4);
        let p1 = e.last_pitches()[0];
        let target = engine_pitch_scale(14000.0, 0);
        // With smoothing the pitch barely moved; without it p1 would equal target.
        assert!(
            (p1 - p0).abs() < (target - p0).abs() * 0.5,
            "pitch snapped instead of gliding: p0={p0} p1={p1} target={target}"
        );
    }

    #[test]
    fn backfire_fires_on_overrun_above_12k_rpm() {
        let mut e = engine_with_bank(dummy_bank());
        // High throttle at high RPM (>12000)
        e.set_state(13500.0, 1000.0, 15000.0, 1.0, 200.0, 4, 0.0, "asphalt");
        assert_eq!(e.last_trigger(), "");

        // Sudden lift-off
        e.set_state(13200.0, 1000.0, 15000.0, 0.0, 200.0, 4, 0.0, "asphalt");
        assert_eq!(e.last_trigger(), "engine_backfire");

        // Cooldown prevents a second lift-off from restarting another variant.
        let active_before = e
            .one_shots
            .iter()
            .position(|o| o.trigger == Trigger::Backfire && o.active);
        e.render(&mut vec![0.0; 256], &mut vec![0.0; 256], 256);
        let cooldown_before = e.backfire_cooldown_samples;
        e.set_state(13000.0, 1000.0, 15000.0, 1.0, 200.0, 4, 0.0, "asphalt");
        e.set_state(12800.0, 1000.0, 15000.0, 0.0, 200.0, 4, 0.0, "asphalt");
        let active_after = e
            .one_shots
            .iter()
            .position(|o| o.trigger == Trigger::Backfire && o.active);
        assert_eq!(e.backfire_cooldown_samples, cooldown_before);
        assert_eq!(active_after, active_before);

        // Advance render past the remaining cooldown.
        let mut l = vec![0.0f32; 15200];
        let mut r = vec![0.0f32; 15200];
        e.render(&mut l, &mut r, 15200);
        assert_eq!(e.backfire_cooldown_samples, 0);

        // Now next lift-off triggers backfire again
        e.set_state(13000.0, 1000.0, 15000.0, 1.0, 200.0, 4, 0.0, "asphalt");
        e.set_state(12700.0, 1000.0, 15000.0, 0.0, 200.0, 4, 0.0, "asphalt");
        assert_eq!(e.last_trigger(), "engine_backfire");
    }

    #[test]
    fn backfire_does_not_fire_at_low_rpm() {
        let mut e = engine_with_bank(dummy_bank());
        // High throttle at low RPM (<12000)
        e.set_state(8000.0, 1000.0, 15000.0, 1.0, 100.0, 3, 0.0, "asphalt");
        // Sudden lift-off
        e.set_state(7800.0, 1000.0, 15000.0, 0.0, 100.0, 3, 0.0, "asphalt");
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
}
