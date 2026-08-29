//! Exhaust layer: firing pulses shaped by collector resonances, phase
//! cancellation and soft saturation.
//!
//! The raw firing excitation (the physical bank) feeds the collector
//! resonances in series, then a one-pole damping stage (phase-cancellation
//! comb damping) and finally a generic saturation stage (`dsp::Tube`).

use crate::config::TubeColorConfig;
use crate::dsp::biquad::Biquad;
use crate::dsp::tube::Tube;
use crate::powertrain::ExhaustConfig;

pub struct ExhaustSynth {
    enabled: bool,
    resonators: Vec<Biquad>,
    damp_state: f32,
    damp_alpha: f32,
    tube: Option<Tube>,
    pulse_gain: f32,
}

impl ExhaustSynth {
    pub fn new(config: &ExhaustConfig, sample_rate: f32) -> Self {
        let resonators = config
            .collector_resonances
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
        let amount = config.saturation.clamp(0.0, 1.0);
        let tube = (amount > 0.0).then(|| {
            Tube::new(
                TubeColorConfig {
                    enabled: true,
                    amount,
                    boost_link: 0.0,
                    bias: 0.05,
                    mix: amount,
                    auto_gain: true,
                },
                0.0,
            )
        });
        Self {
            enabled: config.enabled,
            resonators,
            damp_state: 0.0,
            damp_alpha: 1.0 - config.damping.clamp(0.0, 0.99),
            tube,
            pulse_gain: config.pulse_gain.clamp(0.0, 2.0),
        }
    }

    #[inline]
    pub fn process(&mut self, excitation: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        let mut pulse = excitation * self.pulse_gain;
        for resonator in &mut self.resonators {
            pulse = resonator.process(pulse);
        }
        self.damp_state += (pulse - self.damp_state) * self.damp_alpha;
        let mut out = self.damp_state;
        if let Some(tube) = &mut self.tube {
            out = tube.process(out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ExhaustConfig {
        ExhaustConfig::default()
    }

    #[test]
    fn same_inputs_render_identical_output() {
        let cfg = config();
        let mut a = ExhaustSynth::new(&cfg, 44100.0);
        let mut b = ExhaustSynth::new(&cfg, 44100.0);
        for input in [0.1, 1.0, -0.4, 0.0, 0.8] {
            assert_eq!(a.process(input), b.process(input));
        }
    }

    #[test]
    fn disabled_layer_is_silent() {
        let mut cfg = config();
        cfg.enabled = false;
        let mut exhaust = ExhaustSynth::new(&cfg, 44100.0);
        for _ in 0..1024 {
            assert_eq!(exhaust.process(1.0), 0.0);
        }
    }

    #[test]
    fn saturation_bounds_strong_excitation() {
        let mut cfg = config();
        cfg.saturation = 1.0;
        cfg.damping = 0.5;
        let mut exhaust = ExhaustSynth::new(&cfg, 44100.0);
        // The tube's DC-removal stage may overshoot to ~±2.0 when the excitation
        // flips sign at full drive, but the output must stay bounded (no run-away).
        for _ in 0..4096 {
            let out = exhaust.process(5.0);
            assert!(out.is_finite());
            assert!(out.abs() < 2.1, "saturation failed: {out}");
        }
        for _ in 0..4096 {
            let out = exhaust.process(-5.0);
            assert!(out.is_finite());
            assert!(out.abs() < 2.1, "saturation failed: {out}");
        }
    }

    #[test]
    fn zero_saturation_zero_damping_is_transparent() {
        let mut cfg = config();
        cfg.saturation = 0.0;
        cfg.damping = 0.0;
        cfg.collector_resonances.clear();
        cfg.pulse_gain = 1.0;
        let mut exhaust = ExhaustSynth::new(&cfg, 44100.0);
        for input in [-0.8, 0.0, 0.25, 1.0] {
            assert_eq!(exhaust.process(input), input);
        }
    }
}
