pub const IMPULSE_ATTACK_S: f64 = 0.00075;
pub const IMPULSE_DECAY_S: f64 = 0.006;
pub const IMPULSE_LEVEL: f32 = 0.35;
pub const SCAVENGE_DECAY_S: f64 = 0.012;

const DEFAULT_TORQUE_CURVE: &[(f64, f64)] = &[
    (0.00, 0.10),
    (0.30, 0.35),
    (0.45, 0.72),
    (0.60, 1.00),
    (0.90, 0.85),
    (1.00, 0.72),
];

pub fn default_torque_curve() -> Vec<(f64, f64)> {
    DEFAULT_TORQUE_CURVE.to_vec()
}

pub fn torque_curve_value(curve: &[(f64, f64)], norm_rpm: f64) -> f64 {
    if curve.is_empty() {
        return 1.0;
    }
    let u = norm_rpm.clamp(0.0, 1.0);
    if u <= curve[0].0 {
        return curve[0].1;
    }
    for pair in curve.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        if u <= x1 {
            let t = (u - x0) / (x1 - x0).max(1e-9);
            return y0 + (y1 - y0) * t;
        }
    }
    curve[curve.len() - 1].1
}

pub fn one_pole_alpha(sample_rate: f64, tau_s: f64) -> f32 {
    (-1.0 / (sample_rate * tau_s).max(1e-9)).exp() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_interpolates_linearly_between_points() {
        let curve = vec![(0.45, 0.72), (0.60, 1.00), (1.00, 0.75)];
        let mid = torque_curve_value(&curve, 0.525);
        assert!((mid - 0.86).abs() < 1e-6);
    }

    #[test]
    fn curve_clamps_outside_norm_range() {
        let curve = vec![(0.00, 0.10), (0.60, 1.00), (1.00, 0.75)];
        assert!((torque_curve_value(&curve, -0.5) - 0.10).abs() < 1e-6);
        assert!((torque_curve_value(&curve, 1.5) - 0.75).abs() < 1e-6);
        assert!((torque_curve_value(&curve, 0.5) - 0.85).abs() < 1e-6);
    }

    #[test]
    fn empty_curve_falls_back_to_unity() {
        assert_eq!(torque_curve_value(&[], 0.5), 1.0);
    }

    #[test]
    fn one_pole_alpha_decays_with_time_constant() {
        let alpha = one_pole_alpha(44100.0, 0.02);
        let expected = (-1.0f64 / (44100.0 * 0.02)).exp() as f32;
        assert!((alpha - expected).abs() < 1e-12);
    }
}
