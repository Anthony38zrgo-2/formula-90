#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn from_coefficients(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inverse = 1.0 / a0;
        Self {
            b0: b0 * inverse,
            b1: b1 * inverse,
            b2: b2 * inverse,
            a1: a1 * inverse,
            a2: a2 * inverse,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub(crate) fn highpass(cutoff_hz: f32, q: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);
        Self::from_coefficients(
            (1.0 + cos_w0) * 0.5,
            -(1.0 + cos_w0),
            (1.0 + cos_w0) * 0.5,
            1.0 + alpha,
            -2.0 * cos_w0,
            1.0 - alpha,
        )
    }

    fn lowpass(cutoff_hz: f32, q: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);
        Self::from_coefficients(
            (1.0 - cos_w0) * 0.5,
            1.0 - cos_w0,
            (1.0 - cos_w0) * 0.5,
            1.0 + alpha,
            -2.0 * cos_w0,
            1.0 - alpha,
        )
    }

    /// RBJ high shelf: flat below `cutoff_hz`, `gain_db` above it.
    pub(crate) fn high_shelf(cutoff_hz: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        Self::from_coefficients(
            a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
            a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
            (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
            2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
            (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
        )
    }

    #[inline]
    pub(crate) fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[cfg(test)]
mod biquad_tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44_100.0;

    fn sine_amplitude_db(biquad: &mut Biquad, frequency: f32) -> f64 {
        let total = SAMPLE_RATE as usize;
        let skip = total / 2;
        let mut sum = 0.0f64;
        for index in 0..total {
            let phase =
                2.0 * std::f64::consts::PI * frequency as f64 * index as f64 / SAMPLE_RATE as f64;
            let output = biquad.process(phase.sin() as f32);
            if index >= skip {
                sum += (output as f64) * (output as f64);
            }
        }
        let rms = (sum / (total - skip) as f64).sqrt();
        20.0 * (rms * 2.0_f64.sqrt()).log10()
    }

    #[test]
    fn high_shelf_boosts_only_above_cutoff() {
        let mut shelf = Biquad::high_shelf(2_500.0, 0.707, 4.0, SAMPLE_RATE);
        let low = sine_amplitude_db(&mut shelf, 500.0);
        let mut shelf = Biquad::high_shelf(2_500.0, 0.707, 4.0, SAMPLE_RATE);
        let high = sine_amplitude_db(&mut shelf, 6_000.0);
        assert!(low.abs() < 0.4, "500 Hz must stay flat, got {low} dB");
        assert!((high - 4.0).abs() < 1.0, "6 kHz must lift ~4 dB, got {high} dB");
    }
}

pub struct UpperMidShelf {
    highpass: Biquad,
    lowpass: Biquad,
    gain: f32,
}

impl UpperMidShelf {
    pub const MIN_GAIN: f32 = 0.0;
    pub const MAX_GAIN: f32 = 1.0;
    pub const LOW_EDGE_HZ: f32 = 1800.0;
    pub const HIGH_EDGE_HZ: f32 = 5600.0;

    pub fn new(sample_rate: f32, gain: f32) -> Result<Self, String> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(format!("invalid sample rate: {sample_rate}"));
        }
        if !gain.is_finite() || !(Self::MIN_GAIN..=Self::MAX_GAIN).contains(&gain) {
            return Err(format!("upper-mid shelf gain out of range: {gain}"));
        }
        let q = std::f32::consts::FRAC_1_SQRT_2;
        Ok(Self {
            highpass: Biquad::highpass(Self::LOW_EDGE_HZ, q, sample_rate),
            lowpass: Biquad::lowpass(Self::HIGH_EDGE_HZ, q, sample_rate),
            gain,
        })
    }

    pub fn gain(&self) -> f32 {
        self.gain
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let band = self.lowpass.process(self.highpass.process(input));
        input + self.gain * band
    }

    pub fn reset(&mut self) {
        self.highpass.reset();
        self.lowpass.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44_100.0;

    fn sine_amplitude_db(gain: f32, frequency: f32) -> f64 {
        let mut shelf = UpperMidShelf::new(SAMPLE_RATE, gain).unwrap();
        let total = SAMPLE_RATE as usize;
        let skip = total / 2;
        let mut sum = 0.0f64;
        for index in 0..total {
            let phase = 2.0 * std::f64::consts::PI * frequency as f64 * index as f64 / SAMPLE_RATE as f64;
            let output = shelf.process(phase.sin() as f32);
            if index >= skip {
                sum += (output as f64) * (output as f64);
            }
        }
        let rms = (sum / (total - skip) as f64).sqrt();
        20.0 * (rms * 2.0_f64.sqrt()).log10()
    }

    #[test]
    fn zero_gain_is_bit_transparent() {
        let mut shelf = UpperMidShelf::new(SAMPLE_RATE, 0.0).unwrap();
        let mut state = 0x1234_5678u32;
        for _ in 0..4096 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let input = (state as f32 / u32::MAX as f32) * 2.0 - 1.0;
            assert_eq!(shelf.process(input), input);
        }
    }

    #[test]
    fn boosts_band_without_heavy_top_octave_lift() {
        let center = sine_amplitude_db(0.25, 3200.0);
        let top = sine_amplitude_db(0.25, 8000.0);
        let low = sine_amplitude_db(0.25, 500.0);
        assert!(center > 1.0 && center < 2.2, "center gain {center} dB");
        assert!(top < 0.7, "8 kHz gain {top} dB");
        assert!(low.abs() < 0.3, "500 Hz gain {low} dB");
    }

    #[test]
    fn rejects_out_of_range_gain() {
        assert!(UpperMidShelf::new(SAMPLE_RATE, 1.1).is_err());
        assert!(UpperMidShelf::new(SAMPLE_RATE, -0.1).is_err());
    }
}
