//! Second-bank reconstruction for the half-block V10 synthesis model.
//!
//! The second cylinder bank is never simulated directly: it is derived from
//! the first bank's firing excitation by (1) delaying the excitation train
//! by the configured firing phase offset (the bank is physically rotated),
//! (2) running it through a slightly different body timbre (the second bank
//! lowpass is shifted by `timbre_diff`), and (3) crossfeed delay lines that
//! reproduce the mechanical decorrelation between banks.
//!
//! All reads are linear-interpolated from fixed-size ring buffers (allocation
//! free in the render path).

use crate::dsp::biquad::Biquad;
use crate::powertrain::HalfBlockConfig;

use super::impulse::BODY_LOWPASS_HZ;

/// History depth of every reconstruction ring. At 44.1 kHz this is ~93 ms,
/// comfortably above the maximum configurable delay (50 ms).
const RECON_CAP: usize = 4096;

/// Fixed-size delay ring with linear-interpolated reads.
struct FracRing {
    data: [f32; RECON_CAP],
    head: usize,
    filled: usize,
}

impl FracRing {
    fn new() -> Self {
        Self {
            data: [0.0; RECON_CAP],
            head: 0,
            filled: 0,
        }
    }

    #[inline]
    fn push(&mut self, value: f32) {
        self.data[self.head] = value;
        self.head = (self.head + 1) % RECON_CAP;
        self.filled = (self.filled + 1).min(RECON_CAP);
    }

    /// Read `offset` samples behind the most recent sample (offset 0 = newest).
    /// Positions that predate the ring history return 0.0; negative offsets
    /// (read-ahead) clamp to the most recent sample.
    #[inline]
    fn read(&self, offset: f32) -> f32 {
        if offset < 0.0 {
            return self.data[(self.head + RECON_CAP - 1) % RECON_CAP];
        }
        if offset >= self.filled as f32 {
            return 0.0;
        }
        let i0 = offset.floor() as usize;
        let frac = offset - i0 as f32;
        let i1 = (i0 + 1).min(RECON_CAP - 1);
        let back = |idx: usize| self.data[(self.head + RECON_CAP - 1 - idx) % RECON_CAP];
        let s0 = back(i0);
        if frac == 0.0 {
            return s0;
        }
        let s1 = back(i1);
        s0 + (s1 - s0) * frac
    }
}

/// Derives the second bank from the simulated first bank (see module docs).
pub struct HalfBlockReconstruct {
    phase_ring: FracRing,
    bank1_ring: FracRing,
    bank2_ring: FracRing,
    bank2_filter: Biquad,
    offset_deg: f32,
    offset_samples: f32,
    delay_samples: f32,
    crossfeed_gain: f32,
    second_gain: f32,
}

impl HalfBlockReconstruct {
    pub fn new(sample_rate: u32, config: &HalfBlockConfig) -> Self {
        let sr = sample_rate as f32;
        let cutoff =
            (BODY_LOWPASS_HZ * (1.0 + config.timbre_diff.clamp(0.0, 1.0))).clamp(1.0, sr * 0.49);
        let delay = (config.delay_s.clamp(0.0, 0.05) * sample_rate as f32).round() as f32;
        Self {
            phase_ring: FracRing::new(),
            bank1_ring: FracRing::new(),
            bank2_ring: FracRing::new(),
            bank2_filter: Biquad::lowpass(sr, cutoff),
            offset_deg: config.phase_offset_deg.clamp(0.0, 720.0),
            offset_samples: 0.0,
            delay_samples: delay.min((RECON_CAP - 1) as f32),
            crossfeed_gain: config.decorrelation.clamp(0.0, 1.0),
            second_gain: config.gain.clamp(0.0, 2.0),
        }
    }

    /// Recompute the sample-domain offset from the current cycle increment.
    /// Called whenever RPM is updated (never in the sample loop).
    pub fn update(&mut self, increment_deg_per_sample: f64) {
        self.offset_samples = if increment_deg_per_sample > 1e-9 {
            (self.offset_deg as f64 / increment_deg_per_sample).clamp(0.0, (RECON_CAP - 8) as f64)
                as f32
        } else {
            0.0
        };
    }

    /// Current offset (samples) used to derive the second bank's excitation.
    pub fn offset_samples(&self) -> f32 {
        self.offset_samples
    }

    /// Render one stereo frame from the first bank.
    ///
    /// - Bank 1: the simulated bank's body-filtered excitation (`bank1_filter`).
    /// - Bank 2: the excitation train delayed by the firing-phase offset, shaped
    ///   by `bank2_filter` (timbre-shifted body) and scaled by `second_gain`.
    /// - Stereo: each bank feeds the opposite channel through a short crossfeed
    ///   delay (`decorrelation`), producing the mechanical decorrelation.
    #[inline]
    pub fn process(
        &mut self,
        excitation: f32,
        bank1_filter: &mut Biquad,
        level: f32,
    ) -> (f32, f32) {
        self.phase_ring.push(excitation);
        let second_excitation = self.phase_ring.read(self.offset_samples) * level;
        let bank1 = bank1_filter.process(excitation * level);
        let bank2 = self.bank2_filter.process(second_excitation) * self.second_gain;
        self.bank1_ring.push(bank1);
        self.bank2_ring.push(bank2);
        let bank2_delayed = self.bank2_ring.read(self.delay_samples);
        let bank1_delayed = self.bank1_ring.read(self.delay_samples);
        (
            bank1 + self.crossfeed_gain * bank2_delayed,
            bank2 + self.crossfeed_gain * bank1_delayed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_reads_back_most_recent_sample() {
        let mut ring = FracRing::new();
        ring.push(0.25);
        ring.push(-0.5);
        ring.push(1.0);
        assert!((ring.read(0.0) - 1.0).abs() < 1e-7);
        assert!((ring.read(1.0) - -0.5).abs() < 1e-7);
        assert!((ring.read(2.0) - 0.25).abs() < 1e-7);
        assert_eq!(ring.read(0.5), 0.25);
        assert_eq!(ring.read(3.0), 0.0, "past history must read zero");
        assert!(
            (ring.read(-1.0) - 1.0).abs() < 1e-7,
            "read-ahead clamps to newest"
        );
    }

    #[test]
    fn offset_samples_follows_rpm_increment() {
        let mut r = HalfBlockReconstruct::new(44100, &HalfBlockConfig::default());
        let increment = (9000.0 / 120.0 * 720.0) / 44100.0;
        r.update(increment);
        let expected = 36.0f32 / increment as f32;
        assert!(
            (r.offset_samples() - expected).abs() < 1e-3,
            "offset {} vs {expected}",
            r.offset_samples()
        );
        r.update(0.0);
        assert_eq!(r.offset_samples(), 0.0);
    }
}
