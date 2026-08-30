//! Second-bank reconstruction for the half-block V10 synthesis model.
//!
//! The second cylinder bank is never simulated directly: it is derived from
//! the first bank's firing excitation by delaying each event stream by the
//! configured firing phase offset and applying the configured gain. Acoustic
//! timbre and spatial transfer are applied by later layers.
//!
//! All reads are linear-interpolated from fixed-size ring buffers (allocation
//! free in the render path).

use crate::powertrain::HalfBlockConfig;

/// History depth of every reconstruction ring. At 44.1 kHz this is ~93 ms,
/// comfortably above the maximum configurable delay (50 ms).
const RECON_CAP: usize = 4096;

/// Fixed-size delay ring with linear-interpolated reads.
struct FracRing {
    data: Box<[f32; RECON_CAP]>,
    head: usize,
    filled: usize,
}

impl FracRing {
    fn new() -> Self {
        Self {
            data: Box::new([0.0; RECON_CAP]),
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
    // Indices: 0=body, 1=intake, 2=exhaust, 3=rasp, 4..=8=exhaust_headers[0..5].
    phase_rings: [FracRing; 9],
    offset_deg: f32,
    offset_samples: f32,
    second_gain: f32,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct BankExcitation {
    pub body: f32,
    pub intake: f32,
    pub exhaust: f32,
    /// Event-driven rasp (grit) excitation, reconstructed for bank B with the
    /// same 72-degree mechanical offset and second-gain as the other streams.
    pub rasp: f32,
    pub exhaust_headers: [f32; 5],
}

impl HalfBlockReconstruct {
    pub fn new(config: &HalfBlockConfig) -> Self {
        // Phase/gain belong to event reconstruction. Timbre, delay and
        // decorrelation are intentionally owned by the later acoustic/spatial
        // layers; they must not alter the event train here.
        Self {
            phase_rings: std::array::from_fn(|_| FracRing::new()),
            offset_deg: config.phase_offset_deg.clamp(0.0, 720.0),
            offset_samples: 0.0,
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

    /// Reconstruct all three event streams before acoustic DSP. No filtering,
    /// crossfeed or spatialization is performed here.
    #[inline]
    pub fn process_excitation(
        &mut self,
        excitation: BankExcitation,
    ) -> (BankExcitation, BankExcitation) {
        let values = [
            excitation.body,
            excitation.intake,
            excitation.exhaust,
            excitation.rasp,
        ];
        let mut bank2 = BankExcitation::default();
        for (index, value) in values.into_iter().enumerate() {
            self.phase_rings[index].push(value);
            let delayed = self.phase_rings[index].read(self.offset_samples) * self.second_gain;
            match index {
                0 => bank2.body = delayed,
                1 => bank2.intake = delayed,
                2 => bank2.exhaust = delayed,
                _ => bank2.rasp = delayed,
            }
        }
        for (index, value) in excitation.exhaust_headers.into_iter().enumerate() {
            self.phase_rings[index + 4].push(value);
            bank2.exhaust_headers[index] =
                self.phase_rings[index + 4].read(self.offset_samples) * self.second_gain;
        }
        (excitation, bank2)
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
        let mut r = HalfBlockReconstruct::new(&HalfBlockConfig::default());
        let increment = (9000.0 / 120.0 * 720.0) / 44100.0;
        r.update(increment);
        let expected = 72.0f32 / increment as f32;
        assert!(
            (r.offset_samples() - expected).abs() < 1e-3,
            "offset {} vs {expected}",
            r.offset_samples()
        );
        r.update(0.0);
        assert_eq!(r.offset_samples(), 0.0);
    }

    #[test]
    fn reconstruction_preserves_all_three_event_streams() {
        let mut r = HalfBlockReconstruct::new(&HalfBlockConfig::default());
        r.update((9000.0 / 120.0 * 720.0) / 44100.0);
        let input = BankExcitation {
            body: 1.0,
            intake: 2.0,
            exhaust: 3.0,
            rasp: 4.0,
            exhaust_headers: [3.0; 5],
        };
        let (a, _) = r.process_excitation(input);
        for _ in 0..512 {
            r.process_excitation(input);
        }
        let (_, b) = r.process_excitation(input);
        assert_eq!(a, input);
        assert!(b.body != 0.0 || b.intake != 0.0 || b.exhaust != 0.0);
    }

    /// A constant excitation cannot demonstrate a lag (every delayed read
    /// returns the same value). This test drives a single impulse through the
    /// rasp stream and checks that bank B reproduces it `offset_samples` later,
    /// scaled by `second_gain` — proving it is a reconstruction, not an
    /// independent simulation nor a simultaneous copy.
    #[test]
    fn rasp_b_is_the_delayed_impulse_of_rasp_a() {
        let mut r = HalfBlockReconstruct::new(&HalfBlockConfig::default());
        let increment = (9000.0 / 120.0 * 720.0) / 44100.0;
        r.update(increment);
        let offset = r.offset_samples();
        assert!(offset > 1.0, "offset must be a real multi-sample lag");
        let gain = HalfBlockConfig::default().gain;

        let impulse = BankExcitation {
            body: 0.0,
            intake: 0.0,
            exhaust: 0.0,
            rasp: 1.0,
            exhaust_headers: [0.0; 5],
        };
        let silence = BankExcitation::default();

        let mut b_series = Vec::new();
        let mut a_series = Vec::new();
        // Fire one impulse, then keep feeding silence and record both streams.
        for index in 0..512 {
            let frame = if index == 0 { impulse } else { silence };
            let (a, b) = r.process_excitation(frame);
            a_series.push(a.rasp);
            b_series.push(b.rasp);
        }

        // Bank A sees the impulse immediately.
        assert!(
            (a_series[0] - 1.0).abs() < 1e-6,
            "bank A impulse must be immediate: {}",
            a_series[0]
        );
        // Bank B must be silent until the mechanical offset has elapsed: a
        // simultaneous copy would already be non-zero at sample 0.
        let before = (offset.floor() as usize).saturating_sub(2);
        assert!(
            b_series[..before].iter().all(|v| v.abs() < 1e-6),
            "bank B must not anticipate the impulse (offset {offset})"
        );
        // At the offset, bank B reproduces the impulse scaled by second_gain.
        let peak = b_series
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
            .unwrap()
            .0;
        let expected_index = offset.round() as isize;
        assert!(
            (peak as isize - expected_index).abs() <= 1,
            "bank B impulse peak at {peak}, expected ~{expected_index}"
        );
        // The offset is fractional, so the impulse is spread across two adjacent
        // samples by linear interpolation; their sum is what must equal
        // second_gain. That also rules out an independent simulation.
        let total: f32 = b_series.iter().sum();
        assert!(
            (total - gain).abs() < 1e-2,
            "bank B impulse energy {} should equal second_gain {gain}",
            total
        );
    }
}
