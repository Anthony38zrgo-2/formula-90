use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringConfiguration {
    pub max_steering_angle: f64,
    pub front_steering_ratio: f64,
    pub rear_steering_ratio: f64,
    pub steering_speed: f64,
    pub countersteer_speed: f64,
    pub steering_speed_decay: f64,
    pub steering_slip_assist: f64,
    pub countersteer_assist: f64,
    pub steering_exponent: f64,
    pub ackermann: f64,
}
