use super::driver_tuning::DriverTuning;
use crate::track::vehicle_performance::PerformanceSample;

#[derive(Default)]
pub struct LongitudinalController {
    pub signed_pedal_fraction: f64,
    pub regularized_acceleration_metres_per_second_squared: f64,
    pub braking_engaged: bool,
    integrated_speed_error: f64,
}

impl LongitudinalController {
    pub fn advance(
        &mut self,
        tuning: &DriverTuning,
        speed_error_metres_per_second: f64,
        planned_acceleration_metres_per_second_squared: f64,
        performance: PerformanceSample,
        duration_seconds: f64,
    ) -> (f64, f64) {
        let settings = &tuning.longitudinal_control;
        let tracking_error = speed_error_metres_per_second * speed_error_metres_per_second.abs()
            / speed_error_metres_per_second.hypot(tuning.speed_tracking_deadband_metres_per_second);
        let acceleration_change =
            settings.maximum_profile_jerk_metres_per_second_cubed * duration_seconds;
        self.regularized_acceleration_metres_per_second_squared =
            planned_acceleration_metres_per_second_squared.clamp(
                self.regularized_acceleration_metres_per_second_squared - acceleration_change,
                self.regularized_acceleration_metres_per_second_squared + acceleration_change,
            );
        let integration_allowed = (speed_error_metres_per_second < 0.0
            && self.signed_pedal_fraction > -1.0)
            || (speed_error_metres_per_second >= 0.0 && self.signed_pedal_fraction < 1.0);
        if speed_error_metres_per_second.abs()
            < tuning.integrator_active_speed_error_metres_per_second
            && integration_allowed
        {
            self.integrated_speed_error =
                (self.integrated_speed_error + tracking_error * duration_seconds).clamp(
                    -tuning.maximum_integrated_speed_error,
                    tuning.maximum_integrated_speed_error,
                );
        } else if speed_error_metres_per_second.abs()
            >= tuning.integrator_active_speed_error_metres_per_second
        {
            self.integrated_speed_error = 0.0;
        }
        let acceleration_demand = self.regularized_acceleration_metres_per_second_squared
            + tuning.speed_proportional_gain * tracking_error
            + tuning.speed_integral_gain * self.integrated_speed_error;
        let coasting_deceleration = performance
            .coasting_deceleration_metres_per_second_squared
            .max(0.1);
        let brake_capacity = (performance.braking_deceleration_metres_per_second_squared
            - performance.coasting_deceleration_metres_per_second_squared)
            .max(0.1);
        let requested_brake =
            (-(acceleration_demand + coasting_deceleration) / brake_capacity).clamp(0.0, 1.0);
        if self.braking_engaged {
            if requested_brake <= settings.brake_release_fraction {
                self.braking_engaged = false;
            }
        } else if requested_brake >= settings.brake_engagement_fraction {
            self.braking_engaged = true;
        }
        let requested_signed_pedal = if self.braking_engaged {
            -requested_brake
        } else if acceleration_demand < -coasting_deceleration {
            0.0
        } else if acceleration_demand < 0.0 {
            tuning.maintenance_throttle_fraction
                * (1.0 + acceleration_demand / coasting_deceleration)
        } else {
            (tuning.maintenance_throttle_fraction
                + acceleration_demand / settings.acceleration_reference_metres_per_second_squared)
                .clamp(0.0, 1.0)
        };
        let urgent_braking = speed_error_metres_per_second
            < -settings.urgent_overspeed_metres_per_second
            && requested_brake >= settings.brake_engagement_fraction;
        let mut change_per_second = if requested_signed_pedal > self.signed_pedal_fraction {
            if self.signed_pedal_fraction < 0.0 {
                tuning.maximum_brake_decrease_per_second
            } else {
                tuning.maximum_throttle_increase_per_second
            }
        } else if self.signed_pedal_fraction > 0.0 {
            tuning.maximum_throttle_decrease_per_second
        } else {
            tuning.maximum_brake_increase_per_second
        };
        if urgent_braking && requested_signed_pedal < self.signed_pedal_fraction {
            change_per_second = change_per_second.max(settings.urgent_pedal_change_per_second);
        }
        let change = change_per_second * duration_seconds;
        self.signed_pedal_fraction = requested_signed_pedal.clamp(
            self.signed_pedal_fraction - change,
            self.signed_pedal_fraction + change,
        );
        (
            self.signed_pedal_fraction.max(0.0),
            (-self.signed_pedal_fraction).max(0.0),
        )
    }
}
