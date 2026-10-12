use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonSteering {
    #[serde(default = "default_max_steer")]
    pub(crate) max_steering_angle: f64,
    #[serde(default)]
    pub(crate) front_steering_ratio: f64,
    #[serde(default)]
    pub(crate) rear_steering_ratio: f64,
    #[serde(default = "default_steer_speed")]
    pub(crate) steering_speed: f64,
    #[serde(default = "default_steer_decay")]
    pub(crate) steering_speed_decay: f64,
    #[serde(default = "default_slip_assist")]
    pub(crate) steering_slip_assist: f64,
    #[serde(default = "default_counter_assist")]
    pub(crate) countersteer_assist: f64,
    #[serde(default = "default_steer_exp")]
    pub(crate) steering_exponent: f64,
    #[serde(default = "default_ackermann")]
    pub(crate) ackermann: f64,
}
impl Default for JsonSteering {
    fn default() -> Self {
        Self {
            max_steering_angle: default_max_steer(),
            front_steering_ratio: 1.0,
            rear_steering_ratio: 0.0,
            steering_speed: default_steer_speed(),
            steering_speed_decay: default_steer_decay(),
            steering_slip_assist: default_slip_assist(),
            countersteer_assist: default_counter_assist(),
            steering_exponent: default_steer_exp(),
            ackermann: default_ackermann(),
        }
    }
}
pub(crate) fn default_max_steer() -> f64 {
    0.436332
}
pub(crate) fn default_steer_speed() -> f64 {
    4.25
}
pub(crate) fn default_counter_speed() -> f64 {
    11.0
}
pub(crate) fn default_steer_decay() -> f64 {
    0.20
}
pub(crate) fn default_slip_assist() -> f64 {
    0.54
}
pub(crate) fn default_counter_assist() -> f64 {
    0.89
}
pub(crate) fn default_steer_exp() -> f64 {
    1.50
}
pub(crate) fn default_ackermann() -> f64 {
    0.15
}
