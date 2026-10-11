use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnderfloorContactConfig {
    pub enabled: bool,
    pub approach_clearance_m: f64,
    pub activation_clearance_m: f64,
    pub release_clearance_m: f64,
    pub linear_rate_n_m: f64,
    pub progressive_rate_n_m2: f64,
    pub damping_n_s_m: f64,
    pub max_force_per_probe_n: f64,
    pub rigid_contact_spring_scale: f64,
    pub normal_min_y: f64,
    pub rigid_maximum_local_height_metres: f64,
    pub rigid_minimum_impulse_newton_seconds: f64,
    pub contact_hold_seconds: f64,
    pub rigid_hold_seconds: f64,
    pub clearance_filter_seconds: f64,
}

impl Default for UnderfloorContactConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            approach_clearance_m: 0.035,
            activation_clearance_m: 0.025,
            release_clearance_m: 0.050,
            linear_rate_n_m: 180_000.0,
            progressive_rate_n_m2: 10_000_000.0,
            damping_n_s_m: 8_000.0,
            max_force_per_probe_n: 12_000.0,
            rigid_contact_spring_scale: 0.25,
            normal_min_y: 0.55,
            rigid_maximum_local_height_metres: -0.12,
            rigid_minimum_impulse_newton_seconds: 8.0,
            contact_hold_seconds: 0.060,
            rigid_hold_seconds: 0.080,
            clearance_filter_seconds: 0.015,
        }
    }
}


impl UnderfloorContactConfig {
    pub fn validate(&self) -> Result<(), String> {
        let nonnegative_values = [self.approach_clearance_m, self.activation_clearance_m,
            self.release_clearance_m, self.linear_rate_n_m, self.progressive_rate_n_m2,
            self.damping_n_s_m, self.max_force_per_probe_n,
            self.rigid_minimum_impulse_newton_seconds, self.contact_hold_seconds,
            self.rigid_hold_seconds];
        if nonnegative_values.iter().any(|value| !value.is_finite() || *value < 0.0)
            || !self.clearance_filter_seconds.is_finite() || self.clearance_filter_seconds <= 0.0
            || !self.rigid_maximum_local_height_metres.is_finite()
            || !self.normal_min_y.is_finite() || !(0.0..=1.0).contains(&self.normal_min_y)
            || !self.rigid_contact_spring_scale.is_finite()
            || !(0.0..=1.0).contains(&self.rigid_contact_spring_scale)
            || self.activation_clearance_m > self.approach_clearance_m
            || self.approach_clearance_m >= self.release_clearance_m {
            return Err("underfloor_contact requires finite rates, activation <= approach < release, and valid normal and blend limits".into());
        }
        Ok(())
    }
}
