use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonPowertrain {
    #[serde(default = "default_max_torque")]
    pub(crate) max_torque: f64,
    #[serde(default = "default_max_rpm")]
    pub(crate) max_rpm: f64,
    #[serde(default = "default_idle_rpm")]
    pub(crate) idle_rpm: f64,
    #[serde(default = "default_motor_moment")]
    pub(crate) motor_moment: f64,
    #[serde(default = "default_torque_curve")]
    pub(crate) torque_curve: Vec<(f64, f64)>,
    #[serde(default = "default_gear_ratios")]
    pub(crate) gear_ratios: Vec<f64>,
    #[serde(default = "default_final_drive")]
    pub(crate) final_drive: f64,
    #[serde(default = "default_reverse_ratio")]
    pub(crate) reverse_ratio: f64,
    #[serde(default = "default_shift_time")]
    pub(crate) shift_time: f64,
    #[serde(default = "default_auto_trans")]
    pub(crate) automatic_transmission: bool,
    #[serde(default)]
    pub(crate) downshift_rev_matching_enabled: bool,
    #[serde(default = "default_downshift_maximum_target_revolutions_per_minute_fraction")]
    pub(crate) downshift_maximum_target_revolutions_per_minute_fraction: f64,
    #[serde(default)]
    pub(crate) downshift_blip_torque_fraction: f64,
    #[serde(default)]
    pub(crate) downshift_minimum_interval_seconds: f64,
    #[serde(default)]
    pub(crate) downshift_clutch_reengagement_seconds: f64,
    #[serde(default)]
    pub(crate) front_torque_split: f64,
    #[serde(default = "default_gear_inertia")]
    pub(crate) gear_inertia: f64,
    #[serde(default = "default_max_clutch_ratio")]
    pub(crate) max_clutch_torque_ratio: f64,
    #[serde(default = "default_clutch_out_offset")]
    pub(crate) clutch_out_rpm_offset: f64,
    #[serde(default = "default_torque_attack_rate")]
    pub(crate) torque_attack_rate_nm_s: f64,
    #[serde(default = "default_idle_hysteresis")]
    pub(crate) idle_disengagement_hysteresis_rpm: f64,
    #[serde(default = "default_var_drag")]
    pub(crate) variable_drag_ratio: f64,
    #[serde(default = "default_const_brake")]
    pub(crate) constant_brake_ratio: f64,
    #[serde(default = "default_handbrake_frac")]
    pub(crate) handbrake_torque_fraction: f64,
    #[serde(default)]
    pub(crate) automatic_shift: Option<JsonAutomaticShift>,
    #[serde(default)]
    pub(crate) differential: JsonDifferential,
    #[serde(default)]
    pub(crate) thermal: Option<PowertrainThermalConfig>,
}
impl Default for JsonPowertrain {
    fn default() -> Self {
        Self {
            max_torque: default_max_torque(),
            max_rpm: default_max_rpm(),
            idle_rpm: default_idle_rpm(),
            motor_moment: default_motor_moment(),
            torque_curve: default_torque_curve(),
            gear_ratios: default_gear_ratios(),
            final_drive: default_final_drive(),
            reverse_ratio: default_reverse_ratio(),
            shift_time: default_shift_time(),
            automatic_transmission: default_auto_trans(),
            downshift_rev_matching_enabled: false,
            downshift_maximum_target_revolutions_per_minute_fraction: default_downshift_maximum_target_revolutions_per_minute_fraction(),
            downshift_blip_torque_fraction: 0.0,
            downshift_minimum_interval_seconds: 0.0,
            downshift_clutch_reengagement_seconds: 0.0,
            front_torque_split: 0.0,
            gear_inertia: default_gear_inertia(),
            max_clutch_torque_ratio: default_max_clutch_ratio(),
            clutch_out_rpm_offset: default_clutch_out_offset(),
            torque_attack_rate_nm_s: default_torque_attack_rate(),
            idle_disengagement_hysteresis_rpm: default_idle_hysteresis(),
            variable_drag_ratio: default_var_drag(),
            constant_brake_ratio: default_const_brake(),
            handbrake_torque_fraction: default_handbrake_frac(),
            automatic_shift: None,
            differential: JsonDifferential::default(),
            thermal: None,
        }
    }
}
pub(crate) fn default_max_torque() -> f64 {
    455.0
}
pub(crate) fn default_max_rpm() -> f64 {
    15000.0
}
pub(crate) fn default_idle_rpm() -> f64 {
    4500.0
}
pub(crate) fn default_motor_moment() -> f64 {
    0.12
}
pub(crate) fn default_torque_curve() -> Vec<(f64, f64)> {
    vec![
        (0.00, 0.20),
        (0.265, 0.28),
        (0.44, 0.58),
        (0.62, 0.85),
        (0.79, 0.98),
        (0.88, 1.00),
        (0.97, 0.95),
        (1.00, 0.88),
    ]
}
pub(crate) fn default_gear_ratios() -> Vec<f64> {
    vec![2.65, 2.10, 1.75, 1.50, 1.32, 1.18]
}
pub(crate) fn default_final_drive() -> f64 {
    6.30
}
pub(crate) fn default_reverse_ratio() -> f64 {
    3.00
}
pub(crate) fn default_shift_time() -> f64 {
    0.12
}
pub(crate) fn default_downshift_maximum_target_revolutions_per_minute_fraction() -> f64 {
    1.0
}
pub(crate) fn default_auto_trans() -> bool {
    false
}
pub(crate) fn default_gear_inertia() -> f64 {
    0.02
}
pub(crate) fn default_max_clutch_ratio() -> f64 {
    1.6
}
pub(crate) fn default_clutch_out_offset() -> f64 {
    1000.0
}
pub(crate) fn default_torque_attack_rate() -> f64 {
    0.0
}
pub(crate) fn default_idle_hysteresis() -> f64 {
    50.0
}
pub(crate) fn default_var_drag() -> f64 {
    0.10
}
pub(crate) fn default_const_brake() -> f64 {
    0.02
}
pub(crate) fn default_handbrake_frac() -> f64 {
    0.4
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonAutomaticShift {
    #[serde(default)]
    pub(crate) upshift_normalized_rpm_low_throttle: f64,
    #[serde(default)]
    pub(crate) upshift_normalized_rpm_full_throttle: f64,
    #[serde(default)]
    pub(crate) downshift_normalized_rpm_low_throttle: f64,
    #[serde(default)]
    pub(crate) downshift_normalized_rpm_full_throttle: f64,
    #[serde(default)]
    pub(crate) downshift_throttle_aggression_factor: f64,
    #[serde(default)]
    pub(crate) upshift_coast_normalized_rpm: f64,
    #[serde(default)]
    pub(crate) downshift_coast_normalized_rpm: f64,
    #[serde(default)]
    pub(crate) reverse_max_speed_ms: f64,
    #[serde(default)]
    pub(crate) parked_resume_speed_ms: f64,
    #[serde(default)]
    pub(crate) kickdown_delay_factor: f64,
    #[serde(default)]
    pub(crate) wheel_road_spin_blend_threshold_rads: f64,
    #[serde(default)]
    pub(crate) wheel_spin_weight: f64,
    #[serde(default)]
    pub(crate) road_spin_weight: f64,
    #[serde(default)]
    pub(crate) upshift_target_rpm_margin_high: f64,
    #[serde(default)]
    pub(crate) downshift_target_rpm_margin_high: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonDifferential {
    #[serde(default = "default_preload")]
    pub(crate) preload_nm: f64,
    #[serde(default = "default_power_ramp")]
    pub(crate) power_ramp_angle_deg: f64,
    #[serde(default = "default_coast_ramp")]
    pub(crate) coast_ramp_angle_deg: f64,
    #[serde(default = "default_clutches")]
    pub(crate) clutches: f64,
    #[serde(default)]
    pub(crate) clutch_friction_coefficient: f64,
    #[serde(default = "default_slip_threshold")]
    pub(crate) slip_transition_threshold_rad_s: f64,
}
impl Default for JsonDifferential {
    fn default() -> Self {
        Self {
            preload_nm: 170.0,
            power_ramp_angle_deg: 65.0,
            coast_ramp_angle_deg: 75.0,
            clutches: 4.0,
            clutch_friction_coefficient: 0.0,
            slip_transition_threshold_rad_s: 0.5,
        }
    }
}
pub(crate) fn default_preload() -> f64 {
    170.0
}
pub(crate) fn default_power_ramp() -> f64 {
    65.0
}
pub(crate) fn default_coast_ramp() -> f64 {
    75.0
}
pub(crate) fn default_clutches() -> f64 {
    4.0
}
pub(crate) fn default_slip_threshold() -> f64 {
    0.5
}
