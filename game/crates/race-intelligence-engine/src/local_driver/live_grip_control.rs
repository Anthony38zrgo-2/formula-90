use serde::{Deserialize, Serialize};
use super::driver_tuning::DriverTuning;
use crate::track::vehicle_performance::VehiclePerformanceEnvelope;

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FastVehicleState {
    pub wheel_normal_load_newtons: [f64; 4],
    pub wheel_combined_utilization: [f64; 4],
    pub wheel_combined_demand: [f64; 4],
    pub wheel_slip_angle_radians: [f64; 4],
    pub wheel_peak_slip_angle_radians: [f64; 4],
    pub front_steering_angle_radians: f64,
    pub yaw_rate_radians_per_second: f64,
    pub lateral_speed_metres_per_second: f64,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityVehicleState {
    pub wheel_lateral_capacity_newtons: [f64; 4],
    pub wheel_longitudinal_capacity_newtons: [f64; 4],
    pub wheel_lateral_force_newtons: [f64; 4],
    pub wheel_longitudinal_force_newtons: [f64; 4],
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlowVehicleState {
    pub wheel_grip_scale: [f64; 4],
    pub wheel_brake_efficiency: [f64; 4],
    pub total_mass_kilograms: f64,
    pub front_weight_fraction: f64,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleDynamicsObservation {
    pub fast_state: FastVehicleState,
    pub capacity_state: Option<CapacityVehicleState>,
    pub slow_state: Option<SlowVehicleState>,
}

#[derive(Default, Clone, Copy, Serialize)]
pub struct LiveGripDiagnostics {
    pub capacity_updates: u64,
    pub planning_updates: u64,
    pub slow_state_updates: u64,
    pub front_capacity_scale: f64,
    pub rear_capacity_scale: f64,
    pub front_peak_utilization: f64,
    pub rear_peak_utilization: f64,
    pub understeer_detected: bool,
    pub critical_grip_state: bool,
}

pub struct LiveGripController {
    pub axle_scales: [f64; 2],
    pub brake_efficiency: f64,
    pub available_braking_deceleration: f64,
    pub diagnostics: LiveGripDiagnostics,
    pub fast_state: Option<FastVehicleState>,
    pub transient_speed_fraction: f64,
    critical_hold_remaining_seconds: f64,
    capacity_state: Option<CapacityVehicleState>,
    slow_state: Option<SlowVehicleState>,
    capacity_age_seconds: f64,
    slow_state_age_seconds: f64,
    estimation_elapsed_seconds: f64,
    planning_elapsed_seconds: f64,
}

impl Default for LiveGripController {
    fn default() -> Self {
        Self { axle_scales: [1.0; 2], brake_efficiency: 1.0,
            available_braking_deceleration: f64::INFINITY, diagnostics: LiveGripDiagnostics::default(),
            transient_speed_fraction: 1.0, critical_hold_remaining_seconds: 0.0, fast_state: None, capacity_state: None, slow_state: None,
            capacity_age_seconds: 0.0, slow_state_age_seconds: 0.0,
            estimation_elapsed_seconds: 0.0, planning_elapsed_seconds: 0.0 }
    }
}

fn finite(values: &[f64]) -> bool { values.iter().all(|value| value.is_finite()) }
fn nonnegative(values: &[f64]) -> bool { values.iter().all(|value| value.is_finite() && *value >= 0.0) }

impl LiveGripController {
    pub fn observe(&mut self, observation: VehicleDynamicsObservation, tuning: &DriverTuning,
        speed: f64, wheelbase: f64, performance: &VehiclePerformanceEnvelope, duration: f64) -> Result<bool, String> {
        let settings = &tuning.live_grip_control;
        let fast = observation.fast_state;
        if duration > settings.maximum_fast_state_age_seconds
            || !nonnegative(&fast.wheel_normal_load_newtons)
            || !nonnegative(&fast.wheel_combined_utilization)
            || !nonnegative(&fast.wheel_combined_demand)
            || !nonnegative(&fast.wheel_peak_slip_angle_radians)
            || !finite(&fast.wheel_slip_angle_radians)
            || !finite(&[fast.front_steering_angle_radians, fast.yaw_rate_radians_per_second, fast.lateral_speed_metres_per_second]) {
            return Err("Invalid or stale fast vehicle state".into());
        }
        self.capacity_age_seconds += duration;
        self.slow_state_age_seconds += duration;
        self.estimation_elapsed_seconds += duration;
        self.planning_elapsed_seconds += duration;
        if let Some(capacity) = observation.capacity_state {
            if !nonnegative(&capacity.wheel_lateral_capacity_newtons)
                || !nonnegative(&capacity.wheel_longitudinal_capacity_newtons)
                || !finite(&capacity.wheel_lateral_force_newtons)
                || !finite(&capacity.wheel_longitudinal_force_newtons) {
                return Err("Invalid capacity vehicle state".into());
            }
            self.capacity_state = Some(capacity);
            self.capacity_age_seconds = 0.0;
        }
        if let Some(slow) = observation.slow_state {
            if !nonnegative(&slow.wheel_grip_scale) || !nonnegative(&slow.wheel_brake_efficiency)
                || !slow.total_mass_kilograms.is_finite() || slow.total_mass_kilograms <= 0.0
                || !slow.front_weight_fraction.is_finite() || slow.front_weight_fraction <= 0.0 || slow.front_weight_fraction >= 1.0 {
                return Err("Invalid slow vehicle state".into());
            }
            self.slow_state = Some(slow);
            self.slow_state_age_seconds = 0.0;
            self.diagnostics.slow_state_updates += 1;
        }
        let capacity = self.capacity_state.ok_or("Missing initial capacity vehicle state")?;
        let slow = self.slow_state.ok_or("Missing initial slow vehicle state")?;
        if self.capacity_age_seconds > settings.maximum_capacity_state_age_seconds
            || self.slow_state_age_seconds > settings.maximum_slow_state_age_seconds {
            return Err("Live grip observation exceeded its configured maximum age".into());
        }
        self.fast_state = Some(fast);
        self.diagnostics.front_peak_utilization = fast.wheel_combined_utilization[0].max(fast.wheel_combined_utilization[1]);
        self.diagnostics.rear_peak_utilization = fast.wheel_combined_utilization[2].max(fast.wheel_combined_utilization[3]);
        let front_slip_fraction = (0..2).map(|wheel| fast.wheel_slip_angle_radians[wheel].abs()
            / fast.wheel_peak_slip_angle_radians[wheel].max(settings.minimum_wheel_peak_angle_radians)).fold(0.0, f64::max);
        let expected_yaw_rate = speed * fast.front_steering_angle_radians.tan() / wheelbase;
        self.diagnostics.understeer_detected = expected_yaw_rate.abs() > settings.minimum_turning_rate_radians_per_second
            && fast.yaw_rate_radians_per_second * expected_yaw_rate.signum() < expected_yaw_rate.abs() * settings.understeer_yaw_response_fraction
            && front_slip_fraction > settings.understeer_slip_fraction;
        self.diagnostics.critical_grip_state = speed >= settings.minimum_estimation_speed_metres_per_second
            && (fast.wheel_normal_load_newtons.iter().any(|load| *load < settings.minimum_grounded_load_newtons)
                || (0..4).any(|wheel| fast.wheel_combined_demand[wheel] > settings.critical_demand_fraction
                    && fast.wheel_combined_utilization[wheel] > settings.critical_utilization_fraction)
                || self.diagnostics.understeer_detected);
        if self.diagnostics.critical_grip_state {
            self.critical_hold_remaining_seconds = settings.critical_state_hold_seconds;
        } else {
            self.critical_hold_remaining_seconds = (self.critical_hold_remaining_seconds - duration).max(0.0);
            self.diagnostics.critical_grip_state = self.critical_hold_remaining_seconds > 0.0;
        }
        let requested_speed_fraction = if self.diagnostics.critical_grip_state {
            1.0 - settings.maximum_transient_speed_reduction_fraction
        } else { 1.0 };
        self.transient_speed_fraction = requested_speed_fraction.clamp(
            self.transient_speed_fraction - settings.critical_speed_reduction_per_second * duration,
            self.transient_speed_fraction + settings.critical_speed_recovery_per_second * duration);
        if self.diagnostics.capacity_updates == 0 || self.estimation_elapsed_seconds + f64::EPSILON >= 1.0 / settings.capacity_estimation_hertz {
            let elapsed = self.estimation_elapsed_seconds;
            self.estimation_elapsed_seconds = (self.estimation_elapsed_seconds - 1.0 / settings.capacity_estimation_hertz).max(0.0);
            self.diagnostics.capacity_updates += 1;
            if speed >= settings.minimum_estimation_speed_metres_per_second
                && fast.wheel_normal_load_newtons.iter().all(|load| *load >= settings.minimum_grounded_load_newtons) {
                let baseline = performance.axle_accelerations_at_speed(speed);
                for axle in 0..2 {
                    let mass = slow.total_mass_kilograms * if axle == 0 { slow.front_weight_fraction } else { 1.0 - slow.front_weight_fraction };
                    let lateral_capacity = capacity.wheel_lateral_capacity_newtons[axle * 2] + capacity.wheel_lateral_capacity_newtons[axle * 2 + 1];
                    let measured_scale = (lateral_capacity * tuning.grip_utilization_fraction
                        / (mass * baseline[axle]).max(settings.minimum_reference_force_newtons))
                        .clamp(settings.minimum_capacity_scale, settings.maximum_capacity_scale);
                    self.axle_scales[axle] = measured_scale.clamp(
                        self.axle_scales[axle] - settings.capacity_decrease_per_second * elapsed,
                        self.axle_scales[axle] + settings.capacity_increase_per_second * elapsed);
                }
            }
            self.brake_efficiency = slow.wheel_brake_efficiency.into_iter().fold(1.0, f64::min);
            self.available_braking_deceleration = (0..4).map(|wheel| {
                let lateral_fraction = (capacity.wheel_lateral_force_newtons[wheel].abs()
                    / capacity.wheel_lateral_capacity_newtons[wheel].max(settings.minimum_reference_force_newtons)).clamp(0.0, 1.0);
                capacity.wheel_longitudinal_capacity_newtons[wheel] * (1.0 - lateral_fraction.powi(2)).sqrt()
                    * slow.wheel_brake_efficiency[wheel]
            }).sum::<f64>() / slow.total_mass_kilograms * (1.0 - settings.longitudinal_capacity_reserve_fraction);
        }
        self.diagnostics.front_capacity_scale = self.axle_scales[0];
        self.diagnostics.rear_capacity_scale = self.axle_scales[1];
        let planning_due = self.diagnostics.planning_updates == 0
            || self.planning_elapsed_seconds + f64::EPSILON >= 1.0 / settings.local_planning_hertz;
        if planning_due {
            self.planning_elapsed_seconds = (self.planning_elapsed_seconds - 1.0 / settings.local_planning_hertz).max(0.0);
            self.diagnostics.planning_updates += 1;
        }
        Ok(planning_due)
    }

    pub fn coordinate_steering(&self, desired_angle: f64, tuning: &DriverTuning) -> f64 {
        if !self.diagnostics.understeer_detected { return desired_angle; }
        let actual_angle = self.fast_state.unwrap().front_steering_angle_radians;
        desired_angle.clamp(-actual_angle.abs() - tuning.live_grip_control.maximum_additional_steering_radians,
            actual_angle.abs() + tuning.live_grip_control.maximum_additional_steering_radians)
    }
}
