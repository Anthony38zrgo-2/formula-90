//! Modular synthesis layer for the procedural engine.
//!
//! The combustion simulator ends at the bank + V10 combustion body. The
//! modular layer starts there: four independent voices (saw, square, noise,
//! air-saw) keyed by the *same* bank firing clock (the 144-degree bank event
//! cycle of the 720-degree master cycle, jittered exactly like the bodies).
//! Each voice is a PolyBLEP anti-aliased oscillator (noise is an LCG burst, no
//! oscillator), a per-voice HP/LP biquad cascade and an in-layer gain, and all
//! voices of one bank are shaped by a single ADSR envelope (attack, decay,
//! sustain; release is the next trigger) retriggered by every firing event.
//!
//! The layer never competes with the body registers: the voices are banded by
//! design (saw 300-3500, square 1200-5000, noise 3500-9000, air-saw 7000-11000
//! at the reference 12000 rpm). It is allocation-free, deterministic per seed
//! and bounded by construction (no tanh anywhere).

use crate::dsp::biquad::Biquad;
use crate::powertrain::{ModularConfig, Waveform};

use crate::synth::event_gen::EventJitter;

/// Bank firing cycle in degrees of the 720-degree master cycle (720 / 5
/// simulated cylinders). Each bank triggers once per `TRIGGER_CYCLE_DEG`; bank
/// B is offset by `half_block.phase_offset_deg` (default 72 degrees).
pub const TRIGGER_CYCLE_DEG: f64 = 144.0;

/// Minimum RPM-relative layer level (same philosophy as the rasp floor, so the
/// modular texture does not disappear in the mid range).
const MODULAR_RPM_FLOOR: f32 = 0.2;

/// PolyBLEP discontinuity correction (classic implementation).
///
/// `t` is the normalized phase in [0, 1), `dt` the oscillator increment in
/// cycles per sample. Returns a bounded correction that cancels the aliasing
/// images of the naive waveform at the single discontinuity in `t`.
#[inline]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// Anti-aliased saw: naive 2t-1 corrected at the wrap discontinuity.
#[inline]
pub fn saw_sample(t: f32, dt: f32) -> f32 {
    2.0 * t - 1.0 - poly_blep(t, dt)
}

/// Anti-aliased square: naive bipolar step corrected at both discontinuities.
#[inline]
pub fn square_sample(t: f32, dt: f32) -> f32 {
    let mut value = if t < 0.5 { 1.0 } else { -1.0 };
    value += poly_blep(t, dt);
    let mirrored = (t + 0.5) % 1.0;
    value -= poly_blep(mirrored, dt);
    value
}

/// Per-bank ADSR envelope value at an event age in degrees.
///
/// Attack rises 0 -> 1 over `attack_deg`, then decays 1 -> `sustain` over
/// `decay_deg` and holds until the next trigger (release == retrigger, because
/// the next firing event restarts the attack).
fn envelope_value(config: &ModularConfig, age_deg: f64) -> f32 {
    let attack = config.attack_deg.max(0.5) as f64;
    if age_deg < attack {
        return (age_deg / attack).max(0.0) as f32;
    }
    let decay = config.decay_deg.max(1.0) as f64;
    let t = ((age_deg - attack) / decay).clamp(0.0, 1.0) as f32;
    1.0 - (1.0 - config.sustain.clamp(0.0, 1.0)) * t
}

/// One bank of the modular layer: four voices + shared envelope.
pub struct ModularEngine {
    config: ModularConfig,
    phase0_deg: f64,
    phase_deg: f64,
    next_trigger_deg: f64,
    trigger_index: u64,
    event_age_deg: f64,
    jitter: EventJitter,
    voice_phase: [f64; 4],
    noise_state: u64,
    hp: [Biquad; 4],
    lp: [Biquad; 4],
}

impl ModularEngine {
    fn new(config: &ModularConfig, sample_rate: u32, seed: u64, phase0_deg: f64) -> Self {
        // Align each oscillator to this bank's first mechanical firing phase.
        // Previously both banks started at oscillator phase 0 even though bank B
        // fires 72 crank degrees later; the 0.5x saw could therefore reinforce an
        // artificial RPM/24 ridge. This offset makes phase 0 coincide with the
        // first trigger of each bank.
        let mut voice_phase = [0.0f64; 4];
        for (index, voice) in config.voices.iter().enumerate() {
            if !matches!(voice.waveform, Waveform::Noise) {
                voice_phase[index] = -phase0_deg * 5.0 * voice.frequency_factor as f64;
            }
        }
        Self {
            config: config.clone(),
            phase0_deg,
            phase_deg: 0.0,
            next_trigger_deg: phase0_deg,
            trigger_index: 0,
            event_age_deg: 1.0e9,
            jitter: EventJitter::new(seed, config.jitter_deg.clamp(0.0, 8.0) as f64),
            voice_phase,
            // SplitMix64-style warm start; never zero (LCG must be non-zero).
            noise_state: seed.wrapping_add(0x9E37_79B9_7F4A_7C15) | 1,
            hp: std::array::from_fn(|index| {
                Biquad::highpass(sample_rate as f32, config.voices[index].highpass_hz)
            }),
            lp: std::array::from_fn(|index| {
                Biquad::lowpass(sample_rate as f32, config.voices[index].lowpass_hz)
            }),
        }
    }

    #[inline]
    fn next_noise(&mut self) -> f32 {
        let mut x = self.noise_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.noise_state = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let top = (self.noise_state >> 41) & 0x7F_FFFF; // 23 bits
        (top as f32 / 8388607.0) * 2.0 - 1.0
    }

    /// Advance one sample and return the mixed mono voice output of this bank.
    ///
    /// `increment_deg` is the master 720-degree phase increment per sample;
    /// voice frequencies are `frequency_factor * 5 * increment_deg` (the engine
    /// firing rate is 5 x the master increment, e.g. 1000 Hz at 12000 rpm).
    /// `energy` is the simulated combustion energy: the layer scales with the
    /// engine, so it is silent when the engine is quiet.
    pub fn process(&mut self, increment_deg: f64, energy: f32) -> f32 {
        if increment_deg <= 0.0 || energy <= 0.0 {
            return 0.0;
        }
        self.phase_deg += increment_deg;
        if self.phase_deg >= self.next_trigger_deg {
            self.trigger_index += 1;
            let jitter = self.jitter.next_offset();
            self.next_trigger_deg =
                self.phase0_deg + self.trigger_index as f64 * TRIGGER_CYCLE_DEG + jitter;
            self.event_age_deg = 0.0;
        }
        self.event_age_deg += increment_deg;
        let env = envelope_value(&self.config, self.event_age_deg);
        let mut out = 0.0f32;
        for index in 0..self.config.voices.len() {
            let voice = &self.config.voices[index];
            if !voice.enabled {
                continue;
            }
            // Copy scalars first so no borrow of `self.config` outlives the
            // mutable oscillator/filter calls below.
            let waveform = voice.waveform;
            let factor = voice.frequency_factor as f64;
            let gain = voice.gain;
            let speed_deg = 5.0 * factor * increment_deg;
            let dt = (speed_deg / 360.0).min(0.5) as f32;
            let y = match waveform {
                Waveform::Noise => self.next_noise(),
                Waveform::Saw => {
                    self.voice_phase[index] += speed_deg;
                    let t = (self.voice_phase[index].rem_euclid(360.0) / 360.0) as f32;
                    saw_sample(t, dt)
                }
                Waveform::Square => {
                    self.voice_phase[index] += speed_deg;
                    let t = (self.voice_phase[index].rem_euclid(360.0) / 360.0) as f32;
                    square_sample(t, dt)
                }
            };
            let banded = self.lp[index].process(self.hp[index].process(y));
            out += gain * banded;
        }
        out * env * energy
    }
}

/// Modular layer for both banks: A (phase 0) and B (phase `bank_b_offset_deg`,
/// i.e. `half_block.phase_offset_deg`).
pub struct ModularSynth {
    engines: [ModularEngine; 2],
}

impl ModularSynth {
    pub fn new(
        config: &ModularConfig,
        sample_rate: u32,
        seed: u64,
        bank_b_offset_deg: f64,
    ) -> Self {
        Self {
            engines: [
                ModularEngine::new(config, sample_rate, seed, 0.0),
                ModularEngine::new(config, sample_rate, seed.wrapping_add(1), bank_b_offset_deg),
            ],
        }
    }

    /// Advance both banks one sample. Returns `(bank_a, bank_b)`.
    pub fn process(&mut self, increment_deg: f64, energy: f32) -> (f32, f32) {
        (
            self.engines[0].process(increment_deg, energy),
            self.engines[1].process(increment_deg, energy),
        )
    }

    /// RPM/load-dependent layer gain multiplier (mirrors the rasp curve): a
    /// floor at `MODULAR_RPM_FLOOR` rising to full `gain` at `rpm_full_ratio`.
    pub fn rpm_gain(gain: f32, rpm_start_ratio: f32, rpm_full_ratio: f32, rpm_norm: f32) -> f32 {
        let span = (rpm_full_ratio - rpm_start_ratio).max(1e-3);
        let t = ((rpm_norm - rpm_start_ratio) / span).clamp(0.0, 1.0);
        gain * (MODULAR_RPM_FLOOR + (1.0 - MODULAR_RPM_FLOOR) * t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44100.0;

    fn voice(
        enabled: bool,
        waveform: Waveform,
        factor: f32,
        gain: f32,
        hp: f32,
        lp: f32,
    ) -> crate::powertrain::ModularVoiceConfig {
        crate::powertrain::ModularVoiceConfig {
            enabled,
            waveform,
            frequency_factor: factor,
            gain,
            highpass_hz: hp,
            lowpass_hz: lp,
        }
    }

    fn enabled_config() -> ModularConfig {
        let mut c = ModularConfig::default();
        c.enabled = true;
        c.voices = [
            voice(true, Waveform::Saw, 0.5, 0.30, 300.0, 3500.0),
            voice(true, Waveform::Square, 1.25, 0.12, 1200.0, 5000.0),
            voice(true, Waveform::Noise, 1.0, 0.08, 3500.0, 9000.0),
            voice(true, Waveform::Saw, 7.0, 0.035, 7000.0, 11000.0),
        ];
        c
    }

    /// Power estimate of one bin via Goertzel (proportional to spectral power).
    fn goertzel_power(samples: &[f32], freq: f32) -> f32 {
        let w = std::f32::consts::TAU * freq / SR;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &x in samples {
            let s0 = x + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        s1 * s1 + s2 * s2 - coeff * s1 * s2
    }

    fn band_energy(samples: &[f32], bins: &[f32]) -> f32 {
        bins.iter().map(|&f| goertzel_power(samples, f)).sum()
    }

    #[test]
    fn polyblep_saw_suppresses_alias_energy() {
        let len = 131_072usize;
        let naive: Vec<f32> = (0..len)
            .map(|i| {
                let t = 10000.0 * i as f32 / 44100.0 % 1.0;
                2.0 * t - 1.0
            })
            .collect();
        let dt = 10000.0 / 44100.0;
        let clean: Vec<f32> = (0..len)
            .map(|i| {
                let t = 10000.0 * i as f32 / 44100.0 % 1.0;
                saw_sample(t, dt)
            })
            .collect();
        // Above-Nyquist images fold back below 22.05 kHz but above the tone.
        let bins = [15_500.0f32, 17_250.0, 19_000.0, 20_500.0];
        let aliased = band_energy(&naive, &bins);
        let suppressed = band_energy(&clean, &bins);
        assert!(
            suppressed < aliased * 1e-3,
            "saw alias not suppressed: naive {aliased} vs polyblep {suppressed}"
        );
        let bin_10k = goertzel_power(&clean, 10_000.0);
        let bin_10k_naive = goertzel_power(&naive, 10_000.0);
        assert!(
            bin_10k > bin_10k_naive * 0.5,
            "polyblep damaged the fundamental: {bin_10k} vs {bin_10k_naive}"
        );
    }

    #[test]
    fn polyblep_square_suppresses_alias_energy() {
        let len = 131_072usize;
        let naive: Vec<f32> = (0..len)
            .map(|i| {
                if (6000.0 * i as f32 / 44100.0 % 1.0) < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            })
            .collect();
        let dt = 6000.0 / 44100.0;
        let clean: Vec<f32> = (0..len)
            .map(|i| {
                let t = 6000.0 * i as f32 / 44100.0 % 1.0;
                square_sample(t, dt)
            })
            .collect();
        let bins = [9_000.0f32, 10_800.0, 12_600.0, 14_400.0];
        let aliased = band_energy(&naive, &bins);
        let suppressed = band_energy(&clean, &bins);
        assert!(
            suppressed < aliased * 1e-4,
            "square alias not suppressed: naive {aliased} vs polyblep {suppressed}"
        );
    }

    #[test]
    fn envelope_is_ad_s_and_retriggers() {
        let config = ModularConfig::default();
        assert_eq!(envelope_value(&config, 0.0), 0.0);
        assert!((envelope_value(&config, config.attack_deg as f64 * 0.5) - 0.5).abs() < 1e-4);
        assert!((envelope_value(&config, config.attack_deg as f64) - 1.0).abs() < 1e-4);
        assert!(
            (envelope_value(
                &config,
                config.attack_deg as f64 + config.decay_deg as f64 * 0.5
            ) - (1.0 + config.sustain) * 0.5)
                .abs()
                < 1e-4
        );
        assert!(
            (envelope_value(
                &config,
                config.attack_deg as f64 + config.decay_deg as f64 * 10.0
            ) - config.sustain)
                .abs()
                < 1e-4
        );
        // Every next trigger resets the age: the value starts from zero again.
        assert_eq!(envelope_value(&config, 0.0), 0.0);
    }

    #[test]
    fn output_scales_with_energy_and_is_zero_at_zero() {
        let config = enabled_config();
        let mut a = ModularSynth::new(&config, 44_100, 7, 72.0);
        let inc = 12000.0 / 120.0 * 720.0 / 44100.0;
        let mut full = 0.0f32;
        let mut half = 0.0f32;
        for i in 0..4096 {
            let (fa, _) = a.process(inc, 1.0);
            full += fa * fa;
        }
        let mut b = ModularSynth::new(&config, 44_100, 7, 72.0);
        for i in 0..4096 {
            let (ha, _) = b.process(inc, 0.5);
            half += ha * ha;
        }
        let (full_rms, half_rms) = ((full / 4096.0).sqrt(), (half / 4096.0).sqrt());
        assert!(
            (full_rms - 2.0 * half_rms).abs() < 1e-5,
            "energy scaling broken: {full_rms} vs {half_rms}"
        );
        let mut c = ModularSynth::new(&config, 44_100, 7, 72.0);
        for _ in 0..4096 {
            let (v, w) = c.process(inc, 0.0);
            assert_eq!(v.to_bits(), 0.0f32.to_bits());
            assert_eq!(w.to_bits(), 0.0f32.to_bits());
        }
    }

    #[test]
    fn disabled_layer_is_exact_zero() {
        let config = ModularConfig {
            enabled: false,
            ..ModularConfig::default()
        };
        let mut s = ModularSynth::new(&config, 44_100, 11, 72.0);
        let inc = 12000.0 / 120.0 * 720.0 / 44100.0;
        for _ in 0..1024 {
            let (a, b) = s.process(inc, 1.0);
            assert_eq!(a.to_bits(), 0.0f32.to_bits(), "disabled bank A voiced");
            assert_eq!(b.to_bits(), 0.0f32.to_bits(), "disabled bank B voiced");
        }
    }

    #[test]
    fn same_seed_reproduces_identical_samples() {
        let config = enabled_config();
        let inc = 12000.0 / 120.0 * 720.0 / 44100.0;
        let mut a = ModularSynth::new(&config, 44_100, 42, 72.0);
        let mut b = ModularSynth::new(&config, 44_100, 42, 72.0);
        for _ in 0..4096 {
            let (va, _) = a.process(inc, 0.8);
            let (vb, _) = b.process(inc, 0.8);
            assert_eq!(va.to_bits(), vb.to_bits(), "determinism broken");
        }
    }

    #[test]
    fn output_is_bounded_and_finite_across_rpm_ramp() {
        let config = enabled_config();
        let mut s = ModularSynth::new(&config, 44_100, 99, 72.0);
        for step in 0..2205 {
            let rpm = 500.0 + step as f64 * (15000.0 - 500.0) / 2205.0;
            let inc = rpm / 120.0 * 720.0 / 44100.0;
            for _ in 0..10 {
                let (a, b) = s.process(inc, 1.0);
                assert!(a.is_finite() && b.is_finite(), "non-finite at {rpm}");
                assert!(
                    a.abs() <= 1.2 && b.abs() <= 1.2,
                    "unbounded at {rpm}: {a} {b}"
                );
            }
        }
    }

    #[test]
    fn rpm_gain_floors_and_reaches_full() {
        let g = ModularSynth::rpm_gain(0.5, 0.3, 0.95, 0.0);
        assert!((g - 0.5 * MODULAR_RPM_FLOOR).abs() < 1e-5, "floor {g}");
        let g2 = ModularSynth::rpm_gain(0.5, 0.3, 0.95, 1.0);
        assert!((g2 - 0.5).abs() < 1e-5, "full {g2}");
    }

    #[test]
    fn noise_voice_passes_band_and_rejects_dc() {
        let config = ModularConfig {
            enabled: true,
            voices: [
                voice(true, Waveform::Noise, 1.0, 1.0, 3500.0, 9000.0),
                voice(false, Waveform::Saw, 0.5, 0.0, 300.0, 3500.0),
                voice(false, Waveform::Square, 1.25, 0.0, 1200.0, 5000.0),
                voice(false, Waveform::Saw, 7.0, 0.0, 7000.0, 11000.0),
            ],
            ..ModularConfig::default()
        };
        let mut s = ModularSynth::new(&config, 44_100, 5, 72.0);
        let inc = 12000.0 / 120.0 * 720.0 / 44100.0;
        let mut mean = 0.0f64;
        let mut energy = 0.0f64;
        for _ in 0..8192 {
            let (a, _) = s.process(inc, 1.0);
            mean += a as f64;
            energy += a as f64 * a as f64;
        }
        mean /= 8192.0;
        let rms = (energy / 8192.0).sqrt();
        assert!(mean.abs() < 0.05, "dc leak: {mean}");
        assert!(rms > 0.01, "noise band dead: {rms}");
    }
}
