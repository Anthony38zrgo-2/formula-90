//! Acoustic exhaust-runner waveguide.
//!
//! The single header pipe that runs from a cylinder's exhaust valve to the
//! collector is modelled as a fixed-length delay line with reflection and loss.
//! This preserves the behaviour captured by the original [`crate::acoustics`]
//! `Header` while living behind the exhaust naming so it can evolve into the
//! stateful runner of PHY-062: temperature-aware propagation and a pressure
//! state added without disturbing the header length, wave delay, reflection
//! and loss that define the existing character.

/// A one-dimensional wave delay line representing a single exhaust runner
/// (primary header pipe).
///
/// - **length**   → number of delay samples (derived from `length_m` and wave speed),
/// - **wave delay** → the physical round-trip time through the pipe,
/// - **reflection** → mixing of the arriving travelling wave back into the outgoing
///   wave (boundary reflection coefficient),
/// - **loss** → one-pole low-pass on the reflected feedback, a frequency-dependent
///   attenuation that symbolises viscous/thermal wall loss.
pub struct RunnerWaveguide {
    delay: Vec<f32>,
    cursor: usize,
    reflection: f32,
    feedback_lowpass: f32,
}

impl RunnerWaveguide {
    pub fn new(length_m: f32, wave_speed_mps: f32, reflection: f32, sample_rate: f32) -> Self {
        let delay_samples = (length_m / wave_speed_mps * sample_rate).round().max(2.0) as usize;
        Self {
            delay: vec![0.0; delay_samples],
            cursor: 0,
            reflection,
            feedback_lowpass: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let arrived = self.delay[self.cursor];
        self.feedback_lowpass += 0.32 * (arrived - self.feedback_lowpass);
        self.delay[self.cursor] = input + self.reflection * self.feedback_lowpass;
        self.cursor += 1;
        if self.cursor == self.delay.len() {
            self.cursor = 0;
        }
        arrived
    }

    pub fn delay_samples(&self) -> usize {
        self.delay.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrival_matches_runner_geometry() {
        let mut runner = RunnerWaveguide::new(0.545, 545.0, -0.3, 48_000.0);
        let expected = runner.delay_samples();
        let mut first = None;
        for sample in 0..expected + 4 {
            let y = runner.process(if sample == 0 { 1.0 } else { 0.0 });
            if y != 0.0 && first.is_none() {
                first = Some(sample);
            }
        }
        assert_eq!(first, Some(expected));
    }
}
