#[derive(Debug, Clone, Copy)]
pub struct TransmissionSupervision {
    pub launch_throttle_threshold_fraction: f64,
    pub launch_maximum_speed_metres_per_second: f64,
    pub reverse_maximum_speed_metres_per_second: f64,
}

pub fn supervise_transmission(
    supervision: TransmissionSupervision,
    current_gear: i8,
    forward_speed_metres_per_second: f64,
    throttle_fraction: f64,
) -> Result<Option<i8>, String> {
    validate_transmission_supervision(&supervision)?;
    if !forward_speed_metres_per_second.is_finite() {
        return Err("Transmission forward speed is invalid".to_string());
    }
    if !throttle_fraction.is_finite() || throttle_fraction < 0.0 || throttle_fraction > 1.0 {
        return Err("Transmission throttle fraction is invalid".to_string());
    }
    if current_gear == 0
        && forward_speed_metres_per_second.abs()
            < supervision.launch_maximum_speed_metres_per_second
        && throttle_fraction >= supervision.launch_throttle_threshold_fraction
    {
        return Ok(Some(1));
    }
    if current_gear == -1
        && forward_speed_metres_per_second.abs()
            > supervision.reverse_maximum_speed_metres_per_second
    {
        return Ok(Some(0));
    }
    Ok(None)
}

fn validate_transmission_supervision(supervision: &TransmissionSupervision) -> Result<(), String> {
    if !supervision.launch_throttle_threshold_fraction.is_finite()
        || supervision.launch_throttle_threshold_fraction <= 0.0
        || supervision.launch_throttle_threshold_fraction > 1.0
    {
        return Err("Transmission launch throttle threshold is invalid".to_string());
    }
    if !supervision
        .launch_maximum_speed_metres_per_second
        .is_finite()
        || supervision.launch_maximum_speed_metres_per_second <= 0.0
    {
        return Err("Transmission launch speed is invalid".to_string());
    }
    if !supervision
        .reverse_maximum_speed_metres_per_second
        .is_finite()
        || supervision.reverse_maximum_speed_metres_per_second <= 0.0
    {
        return Err("Transmission reverse speed is invalid".to_string());
    }
    Ok(())
}
