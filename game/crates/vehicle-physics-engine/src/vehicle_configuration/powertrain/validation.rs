use super::document::JsonPowertrain;

pub(crate) fn validate_powertrain(powertrain: &JsonPowertrain) -> Result<(), String> {
        if powertrain.gear_ratios.len() < 1 {
            return Err("At least one gear ratio required".to_string());
        }
        if !powertrain.downshift_maximum_target_revolutions_per_minute_fraction.is_finite()
            || !(0.0..=1.0).contains(&powertrain.downshift_maximum_target_revolutions_per_minute_fraction)
            || !powertrain.downshift_blip_torque_fraction.is_finite()
            || !(0.0..=1.0).contains(&powertrain.downshift_blip_torque_fraction)
            || !powertrain.downshift_minimum_interval_seconds.is_finite()
            || powertrain.downshift_minimum_interval_seconds < 0.0
            || !powertrain.downshift_clutch_reengagement_seconds.is_finite()
            || powertrain.downshift_clutch_reengagement_seconds < 0.0
        {
            return Err("Invalid downshift rev matching configuration".to_string());
        }
        if powertrain.torque_curve.len() < 2 {
            return Err("At least 2 torque curve points required".to_string());
        }
    Ok(())
}
