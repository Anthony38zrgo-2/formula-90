//! Intake layer: RPM-synchronous pressure pulses plus a small turbulent-air
//! component through the configured intake resonances.
//!
//! The noise source is a deterministic xorshift64 generator. The thin
//! throttle gate makes the layer silent at closed throttle (no intake flow,
//! no noise) — a mechanical threshold, not a smoothed fade.

use crate::dsp::biquad::Biquad;
use crate::powertrain::IntakeConfig;

const LFSR_SEED_MIX: u64 = 0x9E37_79B9_7F4A_7C15;

pub struct IntakeSynth {
    lfsr: u64,
    filters: Vec<Biquad>,
    enabled: bool,
    noise_gain: f32,
    pulse_gain: f32,
    throttle_follow: f32,
    turbulence_fast: f32,
    turbulence_slow: f32,
    turbulence_fast_alpha: f32,
    turbulence_slow_alpha: f32,
    /// Fraction of the configured resonances actually processed (per LOD). Only
    /// limits how many Biquads run; it never rebuilds the filter bank, so the
    /// configured timbre is preserved (higher qualities just process more of it).
    resonator_scale: f32,
}

impl IntakeSynth {
    pub fn new(config: &IntakeConfig, sample_rate: f32, seed: u64) -> Self {
        let filters = config
            .resonances
            .iter()
            .map(|resonance| {
                Biquad::peaking(
                    sample_rate,
                    resonance.frequency_hz,
                    resonance.q,
                    resonance.gain_db,
                )
            })
            .collect();
        Self {
            lfsr: seed ^ LFSR_SEED_MIX,
            filters,
            enabled: config.enabled,
            noise_gain: config.noise_gain.clamp(0.0, 1.0),
            pulse_gain: config.pulse_gain.clamp(0.0, 1.0),
            throttle_follow: config.throttle_follow.clamp(0.0, 1.0),
            turbulence_fast: 0.0,
            turbulence_slow: 0.0,
            turbulence_fast_alpha: 1.0 - (-1.0 / (sample_rate * 0.00035)).exp(),
            turbulence_slow_alpha: 1.0 - (-1.0 / (sample_rate * 0.0025)).exp(),
            resonator_scale: 1.0,
        }
    }

    /// Set the fraction of configured resonances to process, clamped to (0, 1].
    pub fn set_resonator_scale(&mut self, scale: f32) {
        self.resonator_scale = scale.clamp(0.0, 1.0);
    }

    pub fn resonator_scale(&self) -> f32 {
        self.resonator_scale
    }

    #[inline]
    fn next_noise(&mut self) -> f32 {
        let mut x = self.lfsr;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.lfsr = x;
        ((x >> 40) as f32 / 16_777_216.0) * 2.0 - 1.0
    }

    /// Effective air aperture: squared throttle (flow is roughly quadratic)
    /// times current load. Zero at closed throttle.
    pub fn aperture(throttle: f32, load: f32) -> f32 {
        throttle.clamp(0.0, 1.0).powi(2) * load.clamp(0.0, 1.0)
    }

    pub fn process(&mut self, excitation: f32, throttle: f32, load: f32) -> f32 {
        if !self.enabled || self.filters.is_empty() {
            return 0.0;
        }
        let aperture = Self::aperture(throttle, load);
        if !aperture.is_finite() || aperture <= 0.0 {
            return 0.0;
        }
        let gate = aperture
            * (1.0 - self.throttle_follow + self.throttle_follow * throttle.clamp(0.0, 1.0));
        let n = (self.filters.len() as f32 * self.resonator_scale).ceil() as usize;
        // Turbulence is coloured before mixing and, critically, is multiplied
        // by the valve-event envelope. It cannot become a free-running white
        // hiss between events.
        let noise = self.next_noise();
        self.turbulence_fast += (noise - self.turbulence_fast) * self.turbulence_fast_alpha;
        self.turbulence_slow += (noise - self.turbulence_slow) * self.turbulence_slow_alpha;
        let event_gate = (excitation.abs() * 2.5).clamp(0.0, 1.0);
        let turbulence = (self.turbulence_fast - self.turbulence_slow) * event_gate;
        let source = excitation * self.pulse_gain + turbulence * self.noise_gain;
        let mut resonant = 0.0f32;
        for filter in self.filters.iter_mut().take(n) {
            resonant += filter.process(source);
        }
        let colored = if n > 0 { resonant / n as f32 } else { source };
        (source * 0.35 + colored * 0.65) * gate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> IntakeConfig {
        IntakeConfig::default()
    }

    #[test]
    fn same_seed_and_inputs_render_identical_output() {
        let cfg = config();
        let mut a = IntakeSynth::new(&cfg, 44100.0, 7);
        let mut b = IntakeSynth::new(&cfg, 44100.0, 7);
        for (throttle, load) in [(0.0, 1.0), (0.5, 0.7), (1.0, 1.0), (0.2, 0.3)] {
            assert_eq!(
                a.process(0.4, throttle, load),
                b.process(0.4, throttle, load)
            );
        }
    }

    #[test]
    fn aperture_is_zero_at_closed_throttle() {
        assert_eq!(IntakeSynth::aperture(0.0, 1.0), 0.0);
        assert_eq!(IntakeSynth::aperture(0.0, 0.0), 0.0);
        assert!((IntakeSynth::aperture(1.0, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn closed_throttle_is_silent_even_when_enabled() {
        let mut intake = IntakeSynth::new(&config(), 44100.0, 7);
        for _ in 0..4096 {
            assert_eq!(intake.process(0.5, 0.0, 1.0), 0.0);
        }
    }

    #[test]
    fn zero_frequency_resonance_is_safe() {
        let mut cfg = config();
        cfg.resonances[0].frequency_hz = 0.0;
        let mut intake = IntakeSynth::new(&cfg, 44100.0, 9);
        for _ in 0..4096 {
            let out = intake.process(0.5, 1.0, 1.0);
            assert!(out.is_finite());
            assert!(out.abs() <= 4.0, "resonance run-away: {out}");
        }
    }

    #[test]
    fn disabled_or_unresonated_layer_is_silent() {
        let mut disabled = config();
        disabled.enabled = false;
        let mut without = config();
        without.resonances.clear();
        let mut a = IntakeSynth::new(&disabled, 44100.0, 7);
        let mut b = IntakeSynth::new(&without, 44100.0, 7);
        for _ in 0..1024 {
            assert_eq!(a.process(0.5, 1.0, 1.0), 0.0);
            assert_eq!(b.process(0.5, 1.0, 1.0), 0.0);
        }
    }

    #[test]
    fn pressure_pulse_drives_intake_without_noise_dominance() {
        let mut cfg = config();
        cfg.noise_gain = 0.0;
        cfg.pulse_gain = 0.5;
        let mut intake = IntakeSynth::new(&cfg, 44100.0, 7);
        let mut peak = 0.0f32;
        for _ in 0..256 {
            peak = peak.max(intake.process(0.5, 1.0, 1.0).abs());
        }
        assert!(peak > 0.05, "synchronous intake pulse missing: {peak}");
    }
}
