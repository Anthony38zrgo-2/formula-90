use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriverTuning {
    pub grip_utilization_fraction: f64,
    pub nominal_tire_grip_fraction: f64,
    pub recovery_speed_change_per_second: f64,
    pub speed_tracking_deadband_metres_per_second: f64,
    pub maximum_speed_metres_per_second: Option<f64>,
    pub lookahead_time_seconds: f64,
    pub minimum_lookahead_metres: f64,
    pub maximum_lookahead_metres: f64,
    pub maximum_steering_change_per_second: f64,
    pub track_edge_margin_metres: f64,
    pub recovery_speed_fraction: f64,
    pub maintenance_throttle_fraction: f64,
    pub speed_proportional_gain: f64,
    pub speed_integral_gain: f64,
    pub maximum_integrated_speed_error: f64,
    pub integrator_active_speed_error_metres_per_second: f64,
    pub maximum_throttle_increase_per_second: f64,
    pub maximum_throttle_decrease_per_second: f64,
    pub maximum_brake_increase_per_second: f64,
    pub maximum_brake_decrease_per_second: f64,
}

pub fn load_driver_tuning(document: &str) -> Result<DriverTuning, String> {
    let tuning: DriverTuning = serde_json::from_str(document).map_err(|error| error.to_string())?;
    validate_driver_tuning(&tuning)?;
    Ok(tuning)
}

pub fn validate_driver_tuning(tuning: &DriverTuning) -> Result<(), String> {
    let positive_values = [
        tuning.lookahead_time_seconds,
        tuning.minimum_lookahead_metres,
        tuning.maximum_lookahead_metres,
        tuning.maximum_steering_change_per_second,
        tuning.track_edge_margin_metres,
        tuning.speed_proportional_gain,
        tuning.speed_integral_gain,
        tuning.maximum_integrated_speed_error,
        tuning.integrator_active_speed_error_metres_per_second,
        tuning.maximum_throttle_increase_per_second,
        tuning.maximum_throttle_decrease_per_second,
        tuning.maximum_brake_increase_per_second,
        tuning.maximum_brake_decrease_per_second,
        tuning.recovery_speed_change_per_second,
        tuning.speed_tracking_deadband_metres_per_second,
    ];
    if tuning.maximum_speed_metres_per_second.is_some_and(|speed| !speed.is_finite() || speed <= 0.0)
        || positive_values
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
        || !(0.0..=1.0).contains(&tuning.maintenance_throttle_fraction)
        || !(0.0..=1.0).contains(&tuning.recovery_speed_fraction)
        || tuning.recovery_speed_fraction == 0.0
        || !(0.0..=1.0).contains(&tuning.grip_utilization_fraction)
        || tuning.grip_utilization_fraction == 0.0
        || !(0.0..=1.0).contains(&tuning.nominal_tire_grip_fraction)
        || tuning.nominal_tire_grip_fraction == 0.0
        || tuning.maximum_lookahead_metres < tuning.minimum_lookahead_metres
    {
        return Err("Artificial driver tuning contains invalid limits".into());
    }
    Ok(())
}
