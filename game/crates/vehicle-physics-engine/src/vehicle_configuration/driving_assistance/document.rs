use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonAids {
    // Traction control (CT / TCS)
    #[serde(default = "default_tc_available")]
    pub(crate) traction_control_available: bool,
    #[serde(default = "default_tc_default")]
    pub(crate) traction_control_default_enabled: bool,
    #[serde(default = "default_tc_selectable")]
    pub(crate) traction_control_player_selectable: bool,
    #[serde(default = "default_tc_slip")]
    pub(crate) traction_control_slip_threshold: f64,
    #[serde(default = "default_tc_cut_gain")]
    pub(crate) traction_control_cut_gain: f64,
    #[serde(default = "default_tc_actuator")]
    pub(crate) traction_control_actuator: String,
    #[serde(default = "default_tc_gear_authority")]
    pub(crate) traction_control_gear_authority: Vec<f64>,
    #[serde(default = "default_tc_gear_slip_target")]
    pub(crate) traction_control_gear_slip_target: Vec<f64>,
    #[serde(default = "default_tc_gear_max_cut")]
    pub(crate) traction_control_gear_max_cut: Vec<f64>,
    #[serde(default = "default_tc_attack_rate")]
    pub(crate) traction_control_attack_rate: f64,
    #[serde(default = "default_tc_release_rate")]
    pub(crate) traction_control_release_rate: f64,

    // ABS policy
    #[serde(default = "default_abs_available")]
    pub(crate) abs_available: bool,
    #[serde(default = "default_abs_default")]
    pub(crate) abs_default_enabled: bool,
    #[serde(default = "default_abs_selectable")]
    pub(crate) abs_player_selectable: bool,

    // Stability / yaw
    #[serde(default = "default_stab_available")]
    pub(crate) stability_available: bool,
    #[serde(default = "default_stab_default")]
    pub(crate) stability_default_enabled: bool,
    #[serde(default = "default_stab_selectable")]
    pub(crate) stability_player_selectable: bool,
    #[serde(default = "default_stab_engage")]
    pub(crate) stability_yaw_engage_angle_rad: f64,
    #[serde(default = "default_stab_yaw_strength")]
    pub(crate) stability_yaw_strength: f64,
    #[serde(default = "default_stab_ref_ay")]
    pub(crate) stability_reference_lateral_accel_m_s2: f64,
    #[serde(default = "default_stab_ground_mult")]
    pub(crate) stability_grounded_multiplier: f64,
    #[serde(default = "default_stab_upright_spring")]
    pub(crate) stability_upright_spring: f64,
    #[serde(default = "default_stab_upright_damping")]
    pub(crate) stability_upright_damping: f64,

    // Steering assists
    #[serde(default = "default_steer_slip_on")]
    pub(crate) steering_slip_assist_default_enabled: bool,
    #[serde(default = "default_counter_on")]
    pub(crate) countersteer_default_enabled: bool,
    #[serde(default = "default_steer_selectable")]
    pub(crate) steering_assist_player_selectable: bool,

    // Auto-clutch
    #[serde(default = "default_auto_clutch_on")]
    pub(crate) auto_clutch_default_enabled: bool,
    #[serde(default = "default_auto_clutch_selectable")]
    pub(crate) auto_clutch_player_selectable: bool,

    // Launch control
    #[serde(default = "default_launch_available")]
    pub(crate) launch_control_available: bool,
    #[serde(default = "default_launch_default")]
    pub(crate) launch_control_default_enabled: bool,
    #[serde(default = "default_launch_selectable")]
    pub(crate) launch_control_player_selectable: bool,
    #[serde(default = "default_launch_target_rpm")]
    pub(crate) launch_control_target_rpm: f64,
    #[serde(default = "default_launch_throttle_limit")]
    pub(crate) launch_control_throttle_limit: f64,
    #[serde(default = "default_launch_release_rate")]
    pub(crate) launch_control_clutch_release_rate_per_s: f64,

    // Brake assist
    #[serde(default = "default_brake_assist_available")]
    pub(crate) brake_assist_available: bool,
    #[serde(default = "default_brake_assist_default")]
    pub(crate) brake_assist_default_enabled: bool,
    #[serde(default = "default_brake_assist_selectable")]
    pub(crate) brake_assist_player_selectable: bool,
    #[serde(default = "default_brake_assist_mult")]
    pub(crate) brake_assist_force_multiplier: f64,

    // Handbrake
    #[serde(default = "default_true")]
    pub(crate) handbrake_rear_only: bool,
    #[serde(default = "default_true")]
    pub(crate) handbrake_abs_interlock: bool,
    #[serde(default = "default_true")]
    pub(crate) handbrake_clutch_coupling: bool,

    // Input smoothing
    #[serde(default = "default_true")]
    pub(crate) input_smoothing_enabled: bool,
    #[serde(default = "default_steer_rate")]
    pub(crate) input_smoothing_steering_rate: f64,
    #[serde(default = "default_throttle_rate")]
    pub(crate) input_smoothing_throttle_rate: f64,
    #[serde(default = "default_brake_rate")]
    pub(crate) input_smoothing_brake_rate: f64,
}
impl Default for JsonAids {
    fn default() -> Self {
        Self {
            traction_control_available: default_tc_available(),
            traction_control_default_enabled: default_tc_default(),
            traction_control_player_selectable: default_tc_selectable(),
            traction_control_slip_threshold: default_tc_slip(),
            traction_control_cut_gain: default_tc_cut_gain(),
            traction_control_actuator: default_tc_actuator(),
            traction_control_gear_authority: default_tc_gear_authority(),
            traction_control_gear_slip_target: default_tc_gear_slip_target(),
            traction_control_gear_max_cut: default_tc_gear_max_cut(),
            traction_control_attack_rate: default_tc_attack_rate(),
            traction_control_release_rate: default_tc_release_rate(),
            abs_available: default_abs_available(),
            abs_default_enabled: default_abs_default(),
            abs_player_selectable: default_abs_selectable(),
            stability_available: default_stab_available(),
            stability_default_enabled: default_stab_default(),
            stability_player_selectable: default_stab_selectable(),
            stability_yaw_engage_angle_rad: default_stab_engage(),
            stability_yaw_strength: default_stab_yaw_strength(),
            stability_reference_lateral_accel_m_s2: default_stab_ref_ay(),
            stability_grounded_multiplier: default_stab_ground_mult(),
            stability_upright_spring: default_stab_upright_spring(),
            stability_upright_damping: default_stab_upright_damping(),
            steering_slip_assist_default_enabled: default_steer_slip_on(),
            countersteer_default_enabled: default_counter_on(),
            steering_assist_player_selectable: default_steer_selectable(),
            auto_clutch_default_enabled: default_auto_clutch_on(),
            auto_clutch_player_selectable: default_auto_clutch_selectable(),
            launch_control_available: default_launch_available(),
            launch_control_default_enabled: default_launch_default(),
            launch_control_player_selectable: default_launch_selectable(),
            launch_control_target_rpm: default_launch_target_rpm(),
            launch_control_throttle_limit: default_launch_throttle_limit(),
            launch_control_clutch_release_rate_per_s: default_launch_release_rate(),
            brake_assist_available: default_brake_assist_available(),
            brake_assist_default_enabled: default_brake_assist_default(),
            brake_assist_player_selectable: default_brake_assist_selectable(),
            brake_assist_force_multiplier: default_brake_assist_mult(),
            handbrake_rear_only: true,
            handbrake_abs_interlock: true,
            handbrake_clutch_coupling: true,
            input_smoothing_enabled: true,
            input_smoothing_steering_rate: default_steer_rate(),
            input_smoothing_throttle_rate: default_throttle_rate(),
            input_smoothing_brake_rate: default_brake_rate(),
        }
    }
}
pub(crate) fn default_tc_available() -> bool {
    true
}
pub(crate) fn default_tc_default() -> bool {
    true
}
pub(crate) fn default_tc_selectable() -> bool {
    true
}
pub(crate) fn default_tc_slip() -> f64 {
    0.08
}
pub(crate) fn default_tc_cut_gain() -> f64 {
    1.4
}
pub(crate) fn default_tc_actuator() -> String {
    "engine_torque".to_string()
}
pub(crate) fn default_tc_gear_authority() -> Vec<f64> {
    vec![1.0, 0.90, 0.65, 0.45, 0.20, 0.10]
}
pub(crate) fn default_tc_gear_slip_target() -> Vec<f64> {
    vec![0.055, 0.065, 0.085, 0.105, 0.140, 0.180]
}
pub(crate) fn default_tc_gear_max_cut() -> Vec<f64> {
    vec![0.78, 0.65, 0.45, 0.30, 0.14, 0.07]
}
pub(crate) fn default_tc_attack_rate() -> f64 {
    34.0
}
pub(crate) fn default_tc_release_rate() -> f64 {
    18.0
}
pub(crate) fn default_abs_available() -> bool {
    true
}
pub(crate) fn default_abs_default() -> bool {
    false
}
pub(crate) fn default_abs_selectable() -> bool {
    true
}
pub(crate) fn default_stab_available() -> bool {
    true
}
pub(crate) fn default_stab_default() -> bool {
    true
}
pub(crate) fn default_stab_selectable() -> bool {
    true
}
pub(crate) fn default_stab_engage() -> f64 {
    0.045
}
pub(crate) fn default_stab_yaw_strength() -> f64 {
    15.0
}
pub(crate) fn default_stab_ref_ay() -> f64 {
    21.0
}
pub(crate) fn default_stab_ground_mult() -> f64 {
    2.6
}
pub(crate) fn default_stab_upright_spring() -> f64 {
    0.0
}
pub(crate) fn default_stab_upright_damping() -> f64 {
    0.0
}
pub(crate) fn default_steer_slip_on() -> bool {
    true
}
pub(crate) fn default_counter_on() -> bool {
    true
}
pub(crate) fn default_steer_selectable() -> bool {
    true
}
pub(crate) fn default_auto_clutch_on() -> bool {
    true
}
pub(crate) fn default_auto_clutch_selectable() -> bool {
    false
}
pub(crate) fn default_launch_available() -> bool {
    false
}
pub(crate) fn default_launch_default() -> bool {
    false
}
pub(crate) fn default_launch_selectable() -> bool {
    true
}
pub(crate) fn default_launch_target_rpm() -> f64 {
    9000.0
}
pub(crate) fn default_launch_throttle_limit() -> f64 {
    1.0
}
pub(crate) fn default_launch_release_rate() -> f64 {
    2.0
}
pub(crate) fn default_brake_assist_available() -> bool {
    false
}
pub(crate) fn default_brake_assist_default() -> bool {
    false
}
pub(crate) fn default_brake_assist_selectable() -> bool {
    true
}
pub(crate) fn default_brake_assist_mult() -> f64 {
    1.0
}
pub(crate) fn default_steer_rate() -> f64 {
    4.25
}
pub(crate) fn default_throttle_rate() -> f64 {
    20.0
}
pub(crate) fn default_brake_rate() -> f64 {
    10.0
}
pub(crate) fn default_true() -> bool {
    true
}
