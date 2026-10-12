use super::*;
use super::document::JsonAutomaticShift;

/// Automatic transmission shift logic parameters, sourced from `f1_94_physics.json`
/// (`automatic_shift`). JSON is authoritative; `Default` reproduces the F1-94 tuned values so
/// the canonical fallback behaves identically to the JSON-driven path. All 15 keys are consumed
/// by the gear-selection logic in `powertrain.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomaticShift {
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

impl Default for AutomaticShift {
    fn default() -> Self {
        Self {
            upshift_normalized_rpm_low_throttle: 0.72,
            upshift_normalized_rpm_full_throttle: 0.965,
            downshift_normalized_rpm_low_throttle: 0.42,
            downshift_normalized_rpm_full_throttle: 0.58,
            downshift_throttle_aggression_factor: 0.35,
            upshift_coast_normalized_rpm: 0.42,
            downshift_coast_normalized_rpm: 0.30,
            reverse_max_speed_ms: 6.0,
            parked_resume_speed_ms: 1.0,
            kickdown_delay_factor: 0.75,
            wheel_road_spin_blend_threshold_rads: 10.0,
            wheel_spin_weight: 0.60,
            road_spin_weight: 0.40,
            upshift_target_rpm_margin_high: 0.94,
            downshift_target_rpm_margin_high: 0.82,
        }
    }
}

impl AutomaticShift {
    pub(crate) fn from_json(j: &JsonAutomaticShift) -> Self {
        Self {
            upshift_normalized_rpm_low_throttle: j.upshift_normalized_rpm_low_throttle,
            upshift_normalized_rpm_full_throttle: j.upshift_normalized_rpm_full_throttle,
            downshift_normalized_rpm_low_throttle: j.downshift_normalized_rpm_low_throttle,
            downshift_normalized_rpm_full_throttle: j.downshift_normalized_rpm_full_throttle,
            downshift_throttle_aggression_factor: j.downshift_throttle_aggression_factor,
            upshift_coast_normalized_rpm: j.upshift_coast_normalized_rpm,
            downshift_coast_normalized_rpm: j.downshift_coast_normalized_rpm,
            reverse_max_speed_ms: j.reverse_max_speed_ms,
            parked_resume_speed_ms: j.parked_resume_speed_ms,
            kickdown_delay_factor: j.kickdown_delay_factor,
            wheel_road_spin_blend_threshold_rads: j.wheel_road_spin_blend_threshold_rads,
            wheel_spin_weight: j.wheel_spin_weight,
            road_spin_weight: j.road_spin_weight,
            upshift_target_rpm_margin_high: j.upshift_target_rpm_margin_high,
            downshift_target_rpm_margin_high: j.downshift_target_rpm_margin_high,
        }
    }
}
