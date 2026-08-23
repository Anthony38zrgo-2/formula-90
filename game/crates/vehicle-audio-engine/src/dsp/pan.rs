use std::f32::consts::FRAC_PI_4;

#[inline]
pub fn equal_power(pan: f32) -> (f32, f32) {
    let angle = (pan.clamp(-1.0, 1.0) + 1.0) * FRAC_PI_4;
    (angle.cos(), angle.sin())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoints_center_and_power() {
        for pan in [-1.0, 0.0, 1.0] {
            let (left, right) = equal_power(pan);
            assert!((left * left + right * right - 1.0).abs() < 1e-5);
        }
        assert!(equal_power(-1.0).1.abs() < 1e-6);
        assert!(equal_power(1.0).0.abs() < 1e-6);
        let center = equal_power(0.0);
        assert!((center.0 - center.1).abs() < 1e-6);
    }
}
