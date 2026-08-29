//! Continuous RPM limiter and traction-control suppression for the
//! procedural half-block synth.
//!
//! Both effects are one-pole smoothed envelopes driven by the contract's
//! attack/release times, applied to the rendered output only. The physical
//! combustion energy reported by [`super::HalfBlock::energy`] is untouched.
//! The render path is scalar only: peak/RMS envelope moves cost no
//! allocations and no coefficient recomputation.

use crate::powertrain::{CutShape, LimiterConfig, TcConfig};

use super::impulse::one_pole_alpha;

/// Suppression depth at full limiter cut (floor gain = 1.0 - depth).
/// Generic procedural default: the sampled `engine_limiter.wav` values are NOT
/// sourced here (CA-08), only contract shape/times plus this generic depth.
pub const LIMITER_DEPTH: f32 = 0.65;

/// How much of the raw (pre-body-filter) excitation is blended back over the
/// lowpassed body tone while the limiter opens the tone ("air"/brillo). The
/// body `Biquad` keeps its fixed `BODY_LOWPASS_HZ` cutoff, so no coefficients
/// are recomputed per sample.
pub const LIMITER_AIR_AMOUNT: f32 = 0.35;

/// Width of the gradual pre-cut ramp of [`CutShape::Soft`], as a fraction of
/// the limiter threshold (the ramp spans the 5% of the threshold below it).
const SOFT_BAND_RATIO: f64 = 0.05;

/// Smoothed RPM limiter state.
#[derive(Debug, Clone, Copy)]
pub struct LimiterState {
    /// Hard gate: true while `rpm >= threshold_rpm_ratio * max_rpm`.
    pub active: bool,
    /// Smoothed cut depth [0.0, 1.0]; 1.0 = full cut engaged.
    pub cut_smoothed: f32,
}

impl LimiterState {
    pub fn new() -> Self {
        Self {
            active: false,
            cut_smoothed: 0.0,
        }
    }

    fn threshold(max_rpm: f64, config: &LimiterConfig) -> f64 {
        max_rpm.max(0.0) * config.threshold_rpm_ratio as f64
    }

    fn active_at(rpm: f64, max_rpm: f64, config: &LimiterConfig) -> bool {
        max_rpm > 0.0 && rpm >= Self::threshold(max_rpm, config)
    }

    /// Cut target [0.0, 1.0] for the current RPM:
    /// - [`CutShape::Hard`]: steps at the threshold.
    /// - [`CutShape::Soft`]: gradual ramp to full cut at the threshold,
    ///   starting `SOFT_BAND_RATIO * threshold` below it (partial pre-cut).
    /// Both shapes are smoothed by [`LimiterState::step`], so the actual
    /// output transition is never a step (no click).
    fn target_cut(rpm: f64, max_rpm: f64, config: &LimiterConfig) -> f32 {
        let threshold = Self::threshold(max_rpm, config);
        match config.cut_shape {
            CutShape::Hard => Self::active_at(rpm, max_rpm, config) as u8 as f32,
            CutShape::Soft => {
                if threshold <= 0.0 {
                    0.0
                } else {
                    let band = threshold * SOFT_BAND_RATIO;
                    ((rpm - (threshold - band)) / band).clamp(0.0, 1.0) as f32
                }
            }
        }
    }

    /// Re-evaluate the hard gate and cut target from the current RPM. Called
    /// when controls update (never inside the sample loop). Returns the
    /// target for per-sample [`LimiterState::step`].
    pub fn evaluate(&mut self, rpm: f64, max_rpm: f64, config: &LimiterConfig) -> f32 {
        self.active = Self::active_at(rpm, max_rpm, config);
        Self::target_cut(rpm, max_rpm, config)
    }

    /// One-pole move of `cut_smoothed` toward the target on one sample. The
    /// `attack_alpha`/`release_alpha` arguments are the pole cofactor from
    /// [`one_pole_alpha`] (`exp(-1 / (sr * tau))`); the convergence step per
    /// sample is `1 - cofactor`, so the smoothing time constant equals the
    /// profile attack/release tau and the transition never steps (no click).
    #[inline]
    pub fn step(&mut self, target: f32, attack_alpha: f32, release_alpha: f32) {
        let convergence = if target > self.cut_smoothed {
            1.0 - attack_alpha
        } else {
            1.0 - release_alpha
        };
        self.cut_smoothed += (target - self.cut_smoothed) * convergence;
    }

    /// Output gain from the smoothed cut (suppression).
    #[inline]
    pub fn gain(&self) -> f32 {
        1.0 - self.cut_smoothed * LIMITER_DEPTH
    }

    /// Mix factor of the raw excitation blended over the body output while
    /// the limiter is cutting (aperture of the tone).
    #[inline]
    pub fn air_amount(&self) -> f32 {
        self.cut_smoothed * LIMITER_AIR_AMOUNT
    }
}

/// One-pole tau (as `alpha`) for the TC envelope, from the profile.
pub(crate) fn tc_alpha(sample_rate: f64, ms: f32) -> f32 {
    one_pole_alpha(sample_rate, ms as f64 / 1000.0)
}

/// TC envelope state: target ratio (clamped by `set_tc_cut_ratio`), smoothed
/// value advanced per sample, and the profile's suppression gain.
#[derive(Debug, Clone)]
pub(crate) struct TcEnvelope {
    target: f32,
    smoothed: f32,
    suppress_gain: f32,
    min_cut_threshold: f32,
}

impl TcEnvelope {
    pub fn new(config: &TcConfig) -> Self {
        Self {
            target: 0.0,
            smoothed: 0.0,
            suppress_gain: config.suppress_gain.clamp(0.0, 1.0),
            min_cut_threshold: config.min_cut_threshold.clamp(0.0, 1.0),
        }
    }

    pub fn set_target(&mut self, ratio: f32) {
        self.target = ratio.clamp(0.0, 1.0);
    }

    /// Effective target after the dead zone: ratios below `min_cut_threshold`
    /// are treated as fully open.
    fn effective_target(&self) -> f32 {
        if self.target < self.min_cut_threshold {
            0.0
        } else {
            self.target
        }
    }

    /// One-pole advance toward the effective target for one sample. Same
    /// cofactor convention as [`LimiterState::step`]: the arguments are pole
    /// cofactors from [`one_pole_alpha`] and the per-sample convergence step
    /// is `1 - cofactor`.
    #[inline]
    pub fn step(&mut self, attack_alpha: f32, release_alpha: f32) {
        let effective = self.effective_target();
        let convergence = if effective > self.smoothed {
            1.0 - attack_alpha
        } else {
            1.0 - release_alpha
        };
        self.smoothed += (effective - self.smoothed) * convergence;
    }

    /// Output gain: full cut scales the engine to `suppress_gain`.
    #[inline]
    pub fn gain(&self) -> f32 {
        1.0 - self.smoothed * (1.0 - self.suppress_gain)
    }

    /// Current smoothed suppression [0.0, 1.0] (diagnostics).
    pub fn suppression(&self) -> f32 {
        self.smoothed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter_config() -> LimiterConfig {
        LimiterConfig::default()
    }

    #[test]
    fn limiter_gate_uses_threshold_ratio_of_max_rpm() {
        let mut state = LimiterState::new();
        let mut config = limiter_config();
        // 0.995 * 15000 = 14925 (f32 threshold); the gate fires at/above it.
        // Margin away from the exact f32 boundary so the test is not
        // representation-sensitive.
        config.cut_shape = CutShape::Hard;
        assert_eq!(state.evaluate(14900.0, 15000.0, &config), 0.0);
        assert!(!state.active);
        assert_eq!(state.evaluate(14930.0, 15000.0, &config), 1.0);
        assert!(state.active);
    }

    #[test]
    fn limiter_guard_handles_zero_max_rpm() {
        let mut state = LimiterState::new();
        let config = limiter_config();
        let target = state.evaluate(9999.0, 0.0, &config);
        assert!(!state.active);
        assert_eq!(target, 0.0);
    }

    #[test]
    fn limiter_soft_shape_ramps_before_threshold() {
        let mut state = LimiterState::new();
        let config = limiter_config();
        // 14925 threshold; band = 746.25; ramp 14178.75..14925.
        assert_eq!(state.evaluate(13000.0, 15000.0, &config), 0.0);
        let mid = state.evaluate(14600.0, 15000.0, &config);
        assert!(mid > 0.0 && mid < 1.0, "partial pre-cut expected: {mid}");
        let top = state.evaluate(14925.0, 15000.0, &config);
        assert!(
            (top - 1.0).abs() < 1e-4,
            "full cut expected at threshold: {top}"
        );
        assert_eq!(state.evaluate(16000.0, 15000.0, &config), 1.0);
    }

    #[test]
    fn limiter_hard_shape_steps_at_threshold() {
        let mut state = LimiterState::new();
        let mut config = limiter_config();
        config.cut_shape = CutShape::Hard;
        assert_eq!(state.evaluate(14900.0, 15000.0, &config), 0.0);
        assert_eq!(state.evaluate(14950.0, 15000.0, &config), 1.0);
    }

    #[test]
    fn limiter_smoothed_cut_moves_with_taus() {
        let mut state = LimiterState::new();
        let config = limiter_config();
        let attack = tc_alpha(44100.0, config.attack_ms);
        let release = tc_alpha(44100.0, config.release_ms);
        // Target 1.0 with attack tau 2 ms: the cut must ramp, never step, and
        // be fully engaged after a couple of taus (2 ms = 88 samples).
        let target = state.evaluate(15000.0, 15000.0, &config);
        for i in 0..10 {
            state.step(target, attack, release);
            assert!(
                state.cut_smoothed < 0.5,
                "click: cut jumped at sample {i}: {}",
                state.cut_smoothed
            );
        }
        for _ in 0..441 {
            state.step(target, attack, release);
        }
        assert!(
            state.cut_smoothed > 0.95,
            "cut not engaged after attack time"
        );
        // Release back to 0 with release tau 60 ms: after 180 ms the cut is
        // below 5%.
        let relaxed = state.evaluate(14000.0, 15000.0, &config);
        for _ in 0..12000 {
            state.step(relaxed, attack, release);
        }
        assert!(
            state.cut_smoothed < 0.05,
            "cut not released: {}",
            state.cut_smoothed
        );
    }

    #[test]
    fn tc_dead_zone_treats_tiny_ratios_as_open() {
        let config = TcConfig::default();
        let mut env = TcEnvelope::new(&config);
        env.set_target(0.01); // below min_cut_threshold (0.02)
        for _ in 0..4410 {
            env.step(
                tc_alpha(44100.0, config.attack_ms),
                tc_alpha(44100.0, config.release_ms),
            );
        }
        assert_eq!(env.suppression(), 0.0);
        assert_eq!(env.gain(), 1.0);
    }

    #[test]
    fn tc_envelope_reaches_full_suppression_gain() {
        let config = TcConfig::default();
        let mut env = TcEnvelope::new(&config);
        env.set_target(1.0);
        for _ in 0..44100 {
            env.step(
                tc_alpha(44100.0, config.attack_ms),
                tc_alpha(44100.0, config.release_ms),
            );
        }
        assert!(
            (env.gain() - config.suppress_gain).abs() < 1e-3,
            "full TC cut must settle at suppress_gain: {}",
            env.gain()
        );
    }
}
