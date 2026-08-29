//! Intake layer: broadband noise through the intake resonances, gated by
//! throttle aperture and load.
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
    throttle_follow: f32,
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
            throttle_follow: config.throttle_follow.clamp(0.0, 1.0),
        }
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

    pub fn process(&mut self, throttle: f32, load: f32) -> f32 {
        if !self.enabled || self.filters.is_empty() {
            return 0.0;
        }
        let aperture = Self::aperture(throttle, load);
        if !aperture.is_finite() || aperture <= 0.0 {
            return 0.0;
        }
        let gate = aperture
            * (1.0 - self.throttle_follow + self.throttle_follow * throttle.clamp(0.0, 1.0));
        let mut out = self.next_noise();
        for filter in &mut self.filters {
            out = filter.process(out);
        }
        out * self.noise_gain * gate
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
            assert_eq!(a.process(throttle, load), b.process(throttle, load));
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
            assert_eq!(intake.process(0.0, 1.0), 0.0);
        }
    }

    #[test]
    fn zero_frequency_resonance_is_safe() {
        let mut cfg = config();
        cfg.resonances[0].frequency_hz = 0.0;
        let mut intake = IntakeSynth::new(&cfg, 44100.0, 9);
        for _ in 0..4096 {
            let out = intake.process(1.0, 1.0);
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
            assert_eq!(a.process(1.0, 1.0), 0.0);
            assert_eq!(b.process(1.0, 1.0), 0.0);
        }
    }
}
