pub struct EventJitter {
    state: u64,
    amplitude_deg: f64,
}

impl EventJitter {
    pub fn new(seed: u64, amplitude_deg: f64) -> Self {
        Self {
            state: seed,
            amplitude_deg: amplitude_deg.abs().min(720.0),
        }
    }

    #[inline]
    fn next_uniform(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.state >> 11) as f64) * (1.0 / 9_007_199_254_740_992.0)
    }

    pub fn next_offset(&mut self) -> f64 {
        (self.next_uniform() * 2.0 - 1.0) * self.amplitude_deg
    }

    /// Deterministic bipolar value used for slow cycle-to-cycle combustion
    /// variation. Keeping this in the event RNG avoids a second noise generator
    /// in the sample loop.
    pub fn next_signed(&mut self) -> f32 {
        (self.next_uniform() * 2.0 - 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_are_bounded_by_amplitude() {
        let mut jitter = EventJitter::new(42, 7.2);
        for _ in 0..10_000 {
            let offset = jitter.next_offset();
            assert!(offset.abs() <= 7.2);
        }
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = EventJitter::new(0xF0_9019_94D1_5CA1, 7.2);
        let mut b = EventJitter::new(0xF0_9019_94D1_5CA1, 7.2);
        for _ in 0..1000 {
            assert_eq!(a.next_offset(), b.next_offset());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = EventJitter::new(1, 7.2);
        let mut b = EventJitter::new(2, 7.2);
        let mut diverged = false;
        for _ in 0..64 {
            if a.next_offset() != b.next_offset() {
                diverged = true;
                break;
            }
        }
        assert!(diverged);
    }

    #[test]
    fn zero_amplitude_is_deterministic_zero() {
        let mut jitter = EventJitter::new(7, 0.0);
        for _ in 0..100 {
            assert_eq!(jitter.next_offset(), 0.0);
        }
    }
}
