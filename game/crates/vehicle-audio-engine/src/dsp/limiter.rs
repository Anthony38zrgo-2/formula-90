#[derive(Debug, Clone, Copy)]
pub struct StereoLimiter {
    ceiling: f32,
}
impl StereoLimiter {
    pub fn new(ceiling: f32) -> Self {
        Self {
            ceiling: ceiling.clamp(0.0, 1.0),
        }
    }
    #[inline]
    pub fn process(&self, left: f32, right: f32) -> (f32, f32) {
        if !left.is_finite() || !right.is_finite() {
            return (0.0, 0.0);
        }
        let peak = left.abs().max(right.abs());
        let gain = if peak > self.ceiling && peak > 0.0 {
            self.ceiling / peak
        } else {
            1.0
        };
        (left * gain, right * gain)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linked_gain_preserves_ratio_and_ceiling() {
        let limiter = StereoLimiter::new(0.9);
        let (l, r) = limiter.process(1.8, 0.9);
        assert!((l - 0.9).abs() < 1e-6 && (r - 0.45).abs() < 1e-6);
    }
}
