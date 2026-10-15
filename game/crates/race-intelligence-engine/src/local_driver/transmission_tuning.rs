use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct AutomaticShiftTuning {
    pub upshift_normalized_rpm_low_throttle: f64,
    pub upshift_normalized_rpm_full_throttle: f64,
    pub downshift_normalized_rpm_low_throttle: f64,
    pub downshift_normalized_rpm_full_throttle: f64,
    pub downshift_throttle_aggression_factor: f64,
    pub upshift_coast_normalized_rpm: f64,
    pub downshift_coast_normalized_rpm: f64,
    pub reverse_max_speed_ms: f64,
    pub parked_resume_speed_ms: f64,
    pub kickdown_delay_factor: f64,
    pub wheel_road_spin_blend_threshold_rads: f64,
    pub wheel_spin_weight: f64,
    pub road_spin_weight: f64,
    pub upshift_target_rpm_margin_high: f64,
    pub downshift_target_rpm_margin_high: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct LaunchSupervisionTuning {
    pub launch_throttle_threshold_fraction: f64,
    pub launch_maximum_speed_metres_per_second: f64,
    pub reverse_maximum_speed_metres_per_second: f64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ArtificialDriverTransmissionTuning {
    pub automatic_transmission: bool,
    pub automatic_shift: AutomaticShiftTuning,
    pub supervision: LaunchSupervisionTuning,
}

#[derive(Debug, Clone, Deserialize)]
struct TuningDocument {
    artificial_driver_transmission: ArtificialDriverTransmissionTuning,
}

pub fn load_transmission_tuning_from_document(
    document: &str,
) -> Result<ArtificialDriverTransmissionTuning, String> {
    let parsed: TuningDocument = serde_json::from_str(document)
        .map_err(|error| format!("Transmission tuning document is invalid: {error}"))?;
    let tuning = parsed.artificial_driver_transmission;
    validate_transmission_tuning(&tuning)?;
    super::transmission_supervision::supervise_transmission(
        super::transmission_supervision::TransmissionSupervision {
            launch_throttle_threshold_fraction: tuning
                .supervision
                .launch_throttle_threshold_fraction,
            launch_maximum_speed_metres_per_second: tuning
                .supervision
                .launch_maximum_speed_metres_per_second,
            reverse_maximum_speed_metres_per_second: tuning
                .supervision
                .reverse_maximum_speed_metres_per_second,
        },
        0,
        0.0,
        0.0,
    )?;
    Ok(tuning)
}

fn validate_transmission_tuning(tuning: &ArtificialDriverTransmissionTuning) -> Result<(), String> {
    if !tuning.automatic_transmission {
        return Err(
            "Artificial driver transmission must select automatic transmission".to_string(),
        );
    }
    let shift = &tuning.automatic_shift;
    if shift.downshift_normalized_rpm_low_throttle >= shift.upshift_normalized_rpm_low_throttle
        || shift.downshift_normalized_rpm_full_throttle
            >= shift.upshift_normalized_rpm_full_throttle
        || shift.downshift_coast_normalized_rpm >= shift.upshift_coast_normalized_rpm
    {
        return Err("Transmission downshift thresholds must lie below upshift thresholds".into());
    }
    let normalized_fields = [
        shift.upshift_normalized_rpm_low_throttle,
        shift.upshift_normalized_rpm_full_throttle,
        shift.downshift_normalized_rpm_low_throttle,
        shift.downshift_normalized_rpm_full_throttle,
        shift.upshift_coast_normalized_rpm,
        shift.downshift_coast_normalized_rpm,
        shift.upshift_target_rpm_margin_high,
        shift.downshift_target_rpm_margin_high,
    ];
    for value in normalized_fields {
        if !value.is_finite() || value <= 0.0 || value > 1.0 {
            return Err("Transmission normalized revolutions threshold is invalid".to_string());
        }
    }
    if shift.upshift_normalized_rpm_full_throttle < shift.upshift_normalized_rpm_low_throttle {
        return Err(
            "Transmission full-throttle upshift lies below part-throttle upshift".to_string(),
        );
    }
    if shift.downshift_normalized_rpm_full_throttle < shift.downshift_normalized_rpm_low_throttle {
        return Err(
            "Transmission full-throttle downshift lies below part-throttle downshift".to_string(),
        );
    }
    if !shift.downshift_throttle_aggression_factor.is_finite()
        || shift.downshift_throttle_aggression_factor < 0.0
        || shift.downshift_throttle_aggression_factor > 1.0
    {
        return Err("Transmission aggression factor is invalid".to_string());
    }
    if !shift.reverse_max_speed_ms.is_finite() || shift.reverse_max_speed_ms <= 0.0 {
        return Err("Transmission reverse speed is invalid".to_string());
    }
    if !shift.parked_resume_speed_ms.is_finite() || shift.parked_resume_speed_ms < 0.0 {
        return Err("Transmission resume speed is invalid".to_string());
    }
    if !shift.kickdown_delay_factor.is_finite()
        || shift.kickdown_delay_factor <= 0.0
        || shift.kickdown_delay_factor > 1.0
    {
        return Err("Transmission kickdown factor is invalid".to_string());
    }
    if !shift.wheel_road_spin_blend_threshold_rads.is_finite()
        || shift.wheel_road_spin_blend_threshold_rads <= 0.0
    {
        return Err("Transmission spin blend threshold is invalid".to_string());
    }
    if !shift.wheel_spin_weight.is_finite()
        || !shift.road_spin_weight.is_finite()
        || shift.wheel_spin_weight < 0.0
        || shift.road_spin_weight < 0.0
        || (shift.wheel_spin_weight + shift.road_spin_weight - 1.0).abs() > 1e-9
    {
        return Err("Transmission spin weights are invalid".to_string());
    }
    if (tuning.supervision.launch_throttle_threshold_fraction <= 0.0
        || tuning.supervision.launch_throttle_threshold_fraction > 1.0)
        || !tuning
            .supervision
            .launch_throttle_threshold_fraction
            .is_finite()
    {
        return Err("Transmission launch threshold is invalid".to_string());
    }
    Ok(())
}
