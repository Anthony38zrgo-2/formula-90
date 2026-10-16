use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveGripControlTuning {
    pub enabled: bool,
    pub record_observations: bool,
    pub control_hertz: f64,
    pub capacity_estimation_hertz: f64,
    pub local_planning_hertz: f64,
    pub slow_state_hertz: f64,
    pub maximum_fast_state_age_seconds: f64,
    pub maximum_capacity_state_age_seconds: f64,
    pub maximum_slow_state_age_seconds: f64,
    pub planning_horizon_seconds: f64,
    pub minimum_planning_distance_metres: f64,
    pub capacity_increase_per_second: f64,
    pub capacity_decrease_per_second: f64,
    pub minimum_capacity_scale: f64,
    pub maximum_capacity_scale: f64,
    pub minimum_estimation_speed_metres_per_second: f64,
    pub minimum_grounded_load_newtons: f64,
    pub minimum_reference_force_newtons: f64,
    pub understeer_slip_fraction: f64,
    pub understeer_yaw_response_fraction: f64,
    pub minimum_turning_rate_radians_per_second: f64,
    pub critical_utilization_fraction: f64,
    pub critical_demand_fraction: f64,
    pub critical_throttle_fraction: f64,
    pub critical_state_hold_seconds: f64,
    pub critical_speed_recovery_per_second: f64,
    pub critical_speed_reduction_per_second: f64,
    pub maximum_transient_speed_reduction_fraction: f64,
    pub maximum_additional_steering_radians: f64,
    pub minimum_wheel_peak_angle_radians: f64,
    pub longitudinal_capacity_reserve_fraction: f64,
}


#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LongitudinalControlTuning {
    pub braking_anticipation_seconds: f64,
    pub acceleration_regularization_seconds: f64,
    pub minimum_regularization_distance_metres: f64,
    pub maximum_profile_jerk_metres_per_second_cubed: f64,
    pub acceleration_reference_metres_per_second_squared: f64,
    pub brake_engagement_fraction: f64,
    pub brake_release_fraction: f64,
    pub urgent_overspeed_metres_per_second: f64,
    pub urgent_pedal_change_per_second: f64,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriverTuning {
    pub live_grip_control: LiveGripControlTuning,
    pub longitudinal_control: LongitudinalControlTuning,
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
    let settings = &tuning.live_grip_control;
    let live_values = [
        settings.control_hertz,
        settings.capacity_estimation_hertz,
        settings.local_planning_hertz,
        settings.slow_state_hertz,
        settings.maximum_fast_state_age_seconds,
        settings.maximum_capacity_state_age_seconds,
        settings.maximum_slow_state_age_seconds,
        settings.planning_horizon_seconds,
        settings.minimum_planning_distance_metres,
        settings.capacity_increase_per_second,
        settings.capacity_decrease_per_second,
        settings.minimum_capacity_scale,
        settings.maximum_capacity_scale,
        settings.minimum_estimation_speed_metres_per_second,
        settings.minimum_grounded_load_newtons,
        settings.critical_state_hold_seconds,
        settings.critical_speed_recovery_per_second,
        settings.critical_speed_reduction_per_second,
        settings.minimum_reference_force_newtons,
        settings.understeer_slip_fraction,
        settings.understeer_yaw_response_fraction,
        settings.minimum_turning_rate_radians_per_second,
        settings.critical_utilization_fraction,
        settings.critical_demand_fraction,
        settings.maximum_transient_speed_reduction_fraction,
        settings.maximum_additional_steering_radians,
        settings.minimum_wheel_peak_angle_radians,
        settings.longitudinal_capacity_reserve_fraction,
    ];
    if !settings.critical_throttle_fraction.is_finite()
        || !(0.0..=1.0).contains(&settings.critical_throttle_fraction)
        || live_values.iter().any(|value| !value.is_finite() || *value <= 0.0)
        || 1.0 / settings.control_hertz > settings.maximum_fast_state_age_seconds
        || settings.capacity_estimation_hertz > settings.control_hertz
        || settings.local_planning_hertz > settings.capacity_estimation_hertz
        || settings.slow_state_hertz > settings.capacity_estimation_hertz
        || settings.minimum_capacity_scale > settings.maximum_capacity_scale
        || settings.minimum_capacity_scale > 1.0
        || settings.maximum_capacity_scale < 1.0
        || settings.critical_utilization_fraction > 1.0
        || settings.maximum_transient_speed_reduction_fraction >= 1.0
        || settings.longitudinal_capacity_reserve_fraction >= 1.0
        || settings.maximum_fast_state_age_seconds < 1.0 / settings.control_hertz
        || settings.maximum_capacity_state_age_seconds < 1.0 / settings.capacity_estimation_hertz
        || settings.maximum_slow_state_age_seconds < 1.0 / settings.slow_state_hertz
    { return Err("Live grip control tuning contains invalid limits".into()); }
    let positive_values = [
        tuning.longitudinal_control.braking_anticipation_seconds,
        tuning.longitudinal_control.acceleration_regularization_seconds,
        tuning.longitudinal_control.minimum_regularization_distance_metres,
        tuning.longitudinal_control.maximum_profile_jerk_metres_per_second_cubed,
        tuning.longitudinal_control.acceleration_reference_metres_per_second_squared,
        tuning.longitudinal_control.urgent_overspeed_metres_per_second,
        tuning.longitudinal_control.urgent_pedal_change_per_second,
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
        || !tuning.longitudinal_control.brake_release_fraction.is_finite()
        || !tuning.longitudinal_control.brake_engagement_fraction.is_finite()
        || tuning.longitudinal_control.brake_release_fraction <= 0.0
        || tuning.longitudinal_control.brake_engagement_fraction >= 1.0
        || tuning.longitudinal_control.brake_release_fraction
            >= tuning.longitudinal_control.brake_engagement_fraction
    {
        return Err("Artificial driver tuning contains invalid limits".into());
    }
    Ok(())
}
