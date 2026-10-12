use serde::{Deserialize, Serialize};

use super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrakesConfiguration {
    pub front_brake_bias: f64,
    pub max_brake_torque: f64,
    pub brake_thermal: BrakeThermalConfig,
    pub enable_abs: bool,
    pub abs_pulse_time: f64,
    pub abs_spin_diff_threshold: f64,
    /// GRIP-03: wheel-spin band (rad/s) where brake torque fades linearly to
    /// zero and sticking is regularized. `0.0` keeps the legacy hard stick.
    #[serde(default)]
    pub wheel_lock_blend_spin_rad_s: f64,
    /// GRIP-03: time constant (s) of the extra stiction pull that parks the wheel
    /// once the brake can hold the unbraked torque. `0.0` means the implicit
    /// band damping alone parks it.
    #[serde(default)]
    pub wheel_lock_stick_tau_s: f64,
}
