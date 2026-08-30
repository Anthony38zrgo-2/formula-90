//! Event-driven rasp (grit) layer for the V10 combustion texture.
//!
//! The rasp is a structured, band-limited "grit" derived from the *event*
//! excitation already produced by the cylinder model. It never contains a
//! free-running oscillator or an independent white-noise source: its timing,
//! jitter, cylinder variation and density are inherited directly from the
//! approved organic body. Each bank's reconstructed rasp excitation (a
//! smoothed combustion-pressure derivative) is band-passed into the 2.5-7 kHz
//! region and soft-clipped, then gain-modulated by RPM/load so it grows
//! toward high/redline without vanishing at mid RPM.
//!
//! The synth is allocation-free and deterministic in the render path.

use crate::dsp::biquad::Biquad;

/// Minimum rasp gain floor (relative to the configured `gain`) applied even at
/// low RPM, so the texture never disappears completely in the mid range.
const RASP_RPM_FLOOR: f32 = 0.25;

pub struct RaspSynth {
    hp: Biquad,
    lp: Biquad,
    saturation: f32,
}

impl RaspSynth {
    pub fn new(
        sample_rate: u32,
        highpass_hz: f32,
        lowpass_hz: f32,
        saturation: f32,
    ) -> Self {
        Self {
            hp: Biquad::highpass(sample_rate as f32, highpass_hz),
            lp: Biquad::lowpass(sample_rate as f32, lowpass_hz),
            saturation: saturation.clamp(0.0, 1.0),
        }
    }

    /// Band-limit and soft-clip a single rasp excitation sample.
    #[inline]
    pub fn process(&mut self, excitation: f32) -> f32 {
        let band = self.lp.process(self.hp.process(excitation));
        // Soft, bounded saturation: tanh keeps the grit from clipping while
        // adding a non-linear edge. `saturation` scales how much drive is
        // applied; the result always stays within [-1, 1].
        let drive = 1.0 + self.saturation * 4.0;
        (band * drive).tanh()
    }
}

impl RaspSynth {
    /// Compute the RPM/load-dependent rasp gain multiplier for one control
    /// update. Returns `gain` scaled by a curve that floors at `RASP_RPM_FLOOR`
    /// and reaches full `gain` at `rpm_full_ratio`.
    pub fn rpm_gain(
        gain: f32,
        rpm_start_ratio: f32,
        rpm_full_ratio: f32,
        rpm_norm: f32,
    ) -> f32 {
        let span = (rpm_full_ratio - rpm_start_ratio).max(1e-3);
        let t = ((rpm_norm - rpm_start_ratio) / span).clamp(0.0, 1.0);
        gain * (RASP_RPM_FLOOR + (1.0 - RASP_RPM_FLOOR) * t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandpass_passes_midrange_and_rejects_dc_and_ultrasonic() {
        let mut s = RaspSynth::new(44100, 2500.0, 7000.0, 0.5);
        // 4 kHz tone should pass.
        let mut peak = 0.0f32;
        for i in 0..20_000 {
            let p = std::f32::consts::TAU * 4000.0 * i as f32 / 44100.0;
            let y = s.process(p.sin() * 0.5);
            if i >= 10_000 {
                peak = peak.max(y.abs());
            }
        }
        assert!(peak > 0.3, "4 kHz should pass, got {peak}");
        // DC should be rejected.
        let mut dc = 0.0f32;
        let mut s2 = RaspSynth::new(44100, 2500.0, 7000.0, 0.5);
        for _ in 0..10_000 {
            dc = s2.process(1.0);
        }
        assert!(dc.abs() < 0.05, "dc not rejected: {dc}");
    }

    #[test]
    fn output_is_bounded_and_finite() {
        let mut s = RaspSynth::new(44100, 2500.0, 7000.0, 1.0);
        for i in 0..4096 {
            let v = s.process((i as f32 * 0.123).sin() * 5.0);
            assert!(v.is_finite() && v.abs() <= 1.0, "unbounded: {v}");
        }
    }

    #[test]
    fn rpm_gain_floors_at_low_rpm_and_saturates_at_full() {
        let g = RaspSynth::rpm_gain(0.6, 0.25, 0.9, 0.0);
        assert!(g > 0.6 * 0.2 && g < 0.6 * 0.3, "low rpm floor {g}");
        let g2 = RaspSynth::rpm_gain(0.6, 0.25, 0.9, 1.0);
        assert!((g2 - 0.6).abs() < 1e-4, "full rpm gain {g2}");
        let g3 = RaspSynth::rpm_gain(0.6, 0.25, 0.9, 0.5);
        assert!(g3 > g && g3 < g2, "monotonic {g3}");
    }
}
