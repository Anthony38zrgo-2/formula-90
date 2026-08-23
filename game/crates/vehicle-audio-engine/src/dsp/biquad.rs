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
}
