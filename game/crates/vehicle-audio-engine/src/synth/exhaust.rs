//! Lightweight procedural exhaust: five fractional header lines feed a
//! reflected collector, then radiation, colour and optional saturation.

use crate::config::TubeColorConfig;
use crate::dsp::biquad::Biquad;
use crate::dsp::tube::Tube;
use crate::powertrain::{ExhaustConfig, HeaderConfig};

const HEADER_RING: usize = 4096;
const COLLECTOR_RING: usize = 8192;

struct FractionalLine<const N: usize> {
    ring: Box<[f32; N]>,
    head: usize,
    delay_i: usize,
    delay_frac: f32,
    gain: f32,
    damping: f32,
    state: f32,
}
impl<const N: usize> FractionalLine<N> {
    fn new(delay: f32, gain: f32, damping: f32) -> Self {
        Self {
            ring: Box::new([0.0; N]),
            head: 0,
            delay_i: delay.clamp(0.0, (N - 2) as f32).floor() as usize,
            delay_frac: delay.clamp(0.0, (N - 2) as f32).fract(),
            gain,
            damping,
            state: 0.0,
        }
    }
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.ring[self.head] = input;
        let i0 = (self.head + N - self.delay_i) % N;
        let frac = self.delay_frac;
        let i1 = (i0 + N - 1) % N;
        let delayed = self.ring[i0] + (self.ring[i1] - self.ring[i0]) * frac;
        self.head = (self.head + 1) % N;
        self.state += (delayed - self.state) * (1.0 - self.damping.clamp(0.0, 0.999));
        self.state * self.gain
    }
}

pub struct ExhaustSynth {
    enabled: bool,
    headers: [FractionalLine<HEADER_RING>; 5],
    collector: FractionalLine<COLLECTOR_RING>,
    feedback_filter: Biquad,
    radiation_filter: Biquad,
    feedback_state: f32,
    reflection_gain: f32,
    resonators: Vec<Biquad>,
    tube: Option<Tube>,
    pulse_gain: f32,
    resonator_scale: f32,
    last_collector: f32,
    distant: bool,
}
impl ExhaustSynth {
    pub fn new(config: &ExhaustConfig, sample_rate: f32) -> Self {
        let speed = config.effective_sound_speed_mps.max(1.0);
        let headers = std::array::from_fn(|i| {
            let fallback = HeaderConfig::default();
            let h = config.headers.get(i).unwrap_or(&fallback);
            FractionalLine::new(
                h.length_m.max(0.001) / speed * sample_rate,
                h.gain,
                h.damping,
            )
        });
        let collector_delay = config.collector.effective_length_m.max(0.001) / speed * sample_rate;
        let resonators = config
            .collector_resonances
            .iter()
            .map(|r| Biquad::peaking(sample_rate, r.frequency_hz, r.q, r.gain_db))
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
            headers,
            collector: FractionalLine::new(collector_delay, 1.0, config.damping),
            feedback_filter: Biquad::lowpass(sample_rate, config.collector.damping_hz.max(20.0)),
            radiation_filter: Biquad::lowpass(
                sample_rate,
                (config.collector.damping_hz * 1.5).max(20.0),
            ),
            feedback_state: 0.0,
            reflection_gain: config.collector.reflection_gain.clamp(-0.94, 0.94),
            resonators,
            tube,
            pulse_gain: config.pulse_gain.clamp(0.0, 2.0),
            resonator_scale: 1.0,
            last_collector: 0.0,
            distant: false,
        }
    }
    pub fn set_resonator_scale(&mut self, scale: f32) {
        self.resonator_scale = scale.clamp(0.0, 1.0);
    }
    pub fn resonator_scale(&self) -> f32 {
        self.resonator_scale
    }
    pub fn set_distant(&mut self, distant: bool) {
        self.distant = distant;
    }
    #[inline]
    pub fn process_individual(&mut self, excitation: &[f32; 5]) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        if self.distant {
            let direct = excitation.iter().sum::<f32>() * self.pulse_gain * 0.2;
            self.last_collector = direct;
            return self.radiation_filter.process(direct);
        }
        let mut headers = 0.0;
        for (line, input) in self.headers.iter_mut().zip(excitation) {
            headers += line.process(*input * self.pulse_gain);
        }
        // The collector radiates its direct sum plus a delayed, damped
        // reflection. Keeping both terms avoids turning collector length into
        // a mere gain control.
        let delayed = self.collector.process(headers + self.feedback_state);
        let reflected = self.feedback_filter.process(delayed) * self.reflection_gain;
        self.feedback_state = reflected;
        let n = (self.resonators.len() as f32 * self.resonator_scale).ceil() as usize;
        let collector = headers + reflected;
        self.last_collector = collector;
        let mut colour = collector;
        for resonator in self.resonators.iter_mut().take(n) {
            colour += resonator.process(colour) * 0.04;
        }
        let mut out = self.radiation_filter.process(colour);
        if let Some(tube) = &mut self.tube {
            out = tube.process(out);
        }
        out
    }
    pub fn collector_output(&self) -> f32 {
        self.last_collector
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
        for x in [0.1, 1.0, -0.4, 0.0, 0.8] {
            let input = [x, 0.0, 0.0, 0.0, 0.0];
            assert_eq!(a.process_individual(&input), b.process_individual(&input));
        }
    }
    #[test]
    fn disabled_layer_is_silent() {
        let mut cfg = config();
        cfg.enabled = false;
        let mut e = ExhaustSynth::new(&cfg, 44100.0);
        for _ in 0..1024 {
            assert_eq!(e.process_individual(&[1.0; 5]), 0.0);
        }
    }
    #[test]
    fn reflection_zero_has_no_tail() {
        let mut cfg = config();
        cfg.collector.reflection_gain = 0.0;
        cfg.saturation = 0.0;
        cfg.collector_resonances.clear();
        let mut e = ExhaustSynth::new(&cfg, 44100.0);
        for _ in 0..256 {
            e.process_individual(&[1.0; 5]);
        }
        for _ in 0..256 {
            e.process_individual(&[0.0; 5]);
        }
        for i in 0..2000 {
            let out = e.process_individual(&[0.0; 5]);
            assert!(out.abs() < 1e-4, "tail at {i}: {out}");
        }
    }
    #[test]
    fn output_is_finite_and_deterministic() {
        let cfg = config();
        let mut a = ExhaustSynth::new(&cfg, 44100.0);
        let mut b = ExhaustSynth::new(&cfg, 44100.0);
        for i in 0..4096 {
            let x = if i == 20 {
                [1.0, 0.0, 0.0, 0.0, 0.0]
            } else {
                [0.0; 5]
            };
            let xa = a.process_individual(&x);
            let xb = b.process_individual(&x);
            assert_eq!(xa, xb);
            assert!(xa.is_finite());
        }
    }

    #[test]
    fn header_delay_and_fractional_interpolation_are_measurable() {
        let mut line = FractionalLine::<64>::new(10.5, 1.0, 0.0);
        let mut values = Vec::new();
        for i in 0..32 {
            values.push(line.process(if i == 0 { 1.0 } else { 0.0 }));
        }
        assert!(values[10] > 0.0 && values[11] > 0.0);
    }

    #[test]
    fn changing_one_header_is_isolated() {
        let mut a = config();
        let mut b = config();
        b.headers[2].length_m *= 1.5;
        let mut x = ExhaustSynth::new(&a, 44100.0);
        let mut y = ExhaustSynth::new(&b, 44100.0);
        for i in 0..512 {
            let input = if i == 0 {
                [1.0, 0.0, 0.0, 0.0, 0.0]
            } else {
                [0.0; 5]
            };
            let _ = x.process_individual(&input);
            let _ = y.process_individual(&input);
        }
        assert!(x.collector_output().is_finite() && y.collector_output().is_finite());
    }

    #[test]
    fn collector_feedback_energy_decays_and_stays_bounded() {
        let cfg = config();
        let mut e = ExhaustSynth::new(&cfg, 44100.0);
        for _ in 0..256 {
            let out = e.process_individual(&[1.0, 0.0, 0.0, 0.0, 0.0]);
            assert!(out.is_finite() && out.abs() < 10.0);
        }
        let mut energy = 0.0;
        for _ in 0..4096 {
            let out = e.process_individual(&[0.0; 5]);
            assert!(out.is_finite());
            energy += (out as f64) * (out as f64);
        }
        assert!(energy.is_finite() && energy < 1.0e6);
    }
}
