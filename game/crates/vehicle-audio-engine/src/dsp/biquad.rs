#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn peaking(sample_rate: f32, frequency: f32, q: f32, gain_db: f32) -> Self {
        let frequency = frequency.clamp(1.0, sample_rate * 0.49);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = std::f32::consts::TAU * frequency / sample_rate;
        let alpha = omega.sin() / (2.0 * q.clamp(0.25, 4.0));
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * omega.cos()) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * omega.cos()) / a0,
            a2: (1.0 - alpha / a) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }
    pub fn lowpass(sample_rate: f32, cutoff_hz: f32) -> Self {
        let cutoff = cutoff_hz.clamp(1.0, sample_rate * 0.49);
        let omega = std::f32::consts::TAU * cutoff / sample_rate;
        let cos_omega = omega.cos();
        let alpha = omega.sin() * std::f32::consts::FRAC_1_SQRT_2;
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 - cos_omega) * 0.5 / a0,
            b1: (1.0 - cos_omega) / a0,
            b2: (1.0 - cos_omega) * 0.5 / a0,
            a1: (-2.0 * cos_omega) / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }
    pub fn highpass(sample_rate: f32, cutoff_hz: f32) -> Self {
        let cutoff = cutoff_hz.clamp(1.0, sample_rate * 0.49);
        let omega = std::f32::consts::TAU * cutoff / sample_rate;
        let cos_omega = omega.cos();
        let alpha = omega.sin() * std::f32::consts::FRAC_1_SQRT_2;
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 + cos_omega) * 0.5 / a0,
            b1: -(1.0 + cos_omega) / a0,
            b2: (1.0 + cos_omega) * 0.5 / a0,
            a1: (-2.0 * cos_omega) / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        if self.z1.abs() < 1e-30 {
            self.z1 = 0.0;
        }
        if self.z2.abs() < 1e-30 {
            self.z2 = 0.0;
        }
        if output.is_finite() {
            output
        } else {
            self.reset();
            0.0
        }
    }
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_db_is_transparent() {
        let mut f = Biquad::peaking(44100.0, 1000.0, 1.0, 0.0);
        for x in [-1.0, 0.0, 0.3, 1.0] {
            assert!((f.process(x) - x).abs() < 1e-6);
        }
    }
    #[test]
    fn lowpass_has_unity_dc_gain_and_suppresses_high_freq() {
        let mut f = Biquad::lowpass(44100.0, 150.0);
        let mut dc = 0.0f32;
        for _ in 0..4096 {
            dc = f.process(1.0);
        }
        assert!((dc - 1.0).abs() < 1e-3, "dc gain {dc}");
        let mut f = Biquad::lowpass(44100.0, 150.0);
        let mut steady = 0.0f32;
        for i in 4000..4096 {
            let phase = 2.0 * std::f32::consts::PI * 5000.0 * i as f32 / 44100.0;
            steady = f.process(phase.sin() * 0.5);
        }
        assert!(steady.abs() < 0.05, "5000 Hz not suppressed: {steady}");
    }
    #[test]
    fn highpass_rejects_dc_and_preserves_engine_band() {
        let mut f = Biquad::highpass(44100.0, 20.0);
        let mut dc = 0.0f32;
        for _ in 0..200_000 {
            dc = f.process(1.0);
        }
        assert!(dc.abs() < 3e-4, "dc residue {dc}");

        let mut f = Biquad::highpass(44100.0, 20.0);
        let mut peak = 0.0f32;
        for i in 0..100_000 {
            let phase = std::f32::consts::TAU * 750.0 * i as f32 / 44100.0;
            let y = f.process(phase.sin());
            if i >= 50_000 {
                peak = peak.max(y.abs());
            }
        }
        assert!(peak > 0.98, "engine band attenuated: {peak}");
    }
    #[test]
    fn peaking_boosts_center_frequency_by_gain_db() {
        let sr = 44100.0;
        let center = 1000.0;
        let gain_db = 6.0;
        let mut f = Biquad::peaking(sr, center, 2.0, gain_db);
        let mut peak = 0.0f32;
        for i in 0..200_000 {
            let phase = 2.0 * std::f32::consts::PI * center * i as f32 / sr;
            let y = f.process(phase.sin());
            if i >= 100_000 {
                peak = peak.max(y.abs());
            }
        }
        let expected = 10.0_f32.powf(gain_db / 20.0);
        assert!(
            (peak - expected).abs() / expected < 0.03,
            "center gain {peak} vs expected {expected}"
        );
    }
    #[test]
    fn peaking_is_unity_far_from_center() {
        let sr = 44100.0;
        let mut f = Biquad::peaking(sr, 1000.0, 2.0, 6.0);
        let mut peak = 0.0f32;
        for i in 0..200_000 {
            let phase = 2.0 * std::f32::consts::PI * 40.0 * i as f32 / sr;
            let y = f.process(phase.sin());
            if i >= 100_000 {
                peak = peak.max(y.abs());
            }
        }
        assert!(
            (peak - 1.0).abs() < 0.05,
            "far band gain should be unity: {peak}"
        );
    }
}
