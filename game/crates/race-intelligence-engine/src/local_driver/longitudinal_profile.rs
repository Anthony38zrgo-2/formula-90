use super::driver_tuning::LongitudinalControlTuning;
use crate::track::speed_profile::SpeedProfileSample;

pub struct LongitudinalProfile {
    samples: Vec<SpeedProfileSample>,
    offsets_metres: Vec<f64>,
    lap_length_metres: f64,
}

impl LongitudinalProfile {
    pub fn new(samples: Vec<SpeedProfileSample>, lap_length_metres: f64) -> Self {
        let first_arc = samples[0].arc_length_metres;
        let offsets_metres = samples
            .iter()
            .map(|sample| (sample.arc_length_metres - first_arc).rem_euclid(lap_length_metres))
            .collect();
        Self {
            samples,
            offsets_metres,
            lap_length_metres,
        }
    }

    pub fn update_local_capacity(
        &mut self,
        waypoints: &[crate::track::path_reader::RacingLineWaypoint],
        start_index: usize,
        speed: f64,
        performance: &crate::track::vehicle_performance::VehiclePerformanceEnvelope,
        axle_scales: [f64; 2],
        brake_efficiency: f64,
        tuning: &super::driver_tuning::DriverTuning,
    ) {
        let settings = &tuning.live_grip_control;
        let horizon = (speed * settings.planning_horizon_seconds).max(settings.minimum_planning_distance_metres);
        let mut distance = 0.0;
        let mut count = 0;
        let rear_horizon = (speed * tuning.longitudinal_control.acceleration_regularization_seconds)
            .max(tuning.longitudinal_control.minimum_regularization_distance_metres) * 0.5;
        let mut first_index = start_index;
        let mut rear_distance = 0.0;
        while rear_distance < rear_horizon {
            let previous = (first_index + self.samples.len() - 1) % self.samples.len();
            rear_distance += (self.samples[first_index].arc_length_metres - self.samples[previous].arc_length_metres).rem_euclid(self.lap_length_metres);
            first_index = previous;
        }
        while count < self.samples.len() && (distance <= horizon || count < 3) {
            let index = (first_index + count) % self.samples.len();
            let next = (index + 1) % self.samples.len();
            let mut lower = 0.0;
            let mut upper = tuning.maximum_speed_metres_per_second.unwrap_or(performance.mechanical_maximum_speed_metres_per_second)
                .min(performance.mechanical_maximum_speed_metres_per_second);
            for _ in 0..40 {
                let middle = (lower + upper) * 0.5;
                if middle * middle * self.samples[index].curvature_per_metre.abs()
                    <= performance.corrected_at_speed(middle, axle_scales, brake_efficiency).lateral_acceleration_metres_per_second_squared {
                    lower = middle;
                } else { upper = middle; }
            }
            self.samples[index].target_speed_metres_per_second = lower;
            distance += (self.samples[next].arc_length_metres - self.samples[index].arc_length_metres).rem_euclid(self.lap_length_metres);
            count += 1;
        }
        for offset in (0..count.saturating_sub(1)).rev() {
            let index = (first_index + offset) % self.samples.len();
            let next = (index + 1) % self.samples.len();
            let distance = (self.samples[next].arc_length_metres - self.samples[index].arc_length_metres).rem_euclid(self.lap_length_metres);
            let speed = self.samples[index].target_speed_metres_per_second;
            let capacity = performance.corrected_at_speed(speed, axle_scales, brake_efficiency);
            let lateral_fraction = (speed.powi(2) * self.samples[index].curvature_per_metre.abs()
                / capacity.lateral_acceleration_metres_per_second_squared * tuning.grip_utilization_fraction).clamp(0.0, 1.0);
            let slope = (waypoints[next].position_y_metres - waypoints[index].position_y_metres) / distance;
            let deceleration = (capacity.braking_deceleration_metres_per_second_squared * tuning.grip_utilization_fraction
                * (1.0 - lateral_fraction.powi(2)).max(0.0).sqrt() + 9.80665 * slope).max(0.5);
            let reachable = (self.samples[next].target_speed_metres_per_second.powi(2) + 2.0 * deceleration * distance).sqrt();
            self.samples[index].target_speed_metres_per_second = speed.min(reachable);
        }
    }

    pub fn speed_squared_at(&self, arc_metres: f64) -> f64 {
        let offset =
            (arc_metres - self.samples[0].arc_length_metres).rem_euclid(self.lap_length_metres);
        let index = self
            .offsets_metres
            .partition_point(|value| *value <= offset)
            .saturating_sub(1);
        let next_index = (index + 1) % self.samples.len();
        let end = if next_index == 0 {
            self.lap_length_metres
        } else {
            self.offsets_metres[next_index]
        };
        let fraction = (offset - self.offsets_metres[index]) / (end - self.offsets_metres[index]);
        let first = self.samples[index].target_speed_metres_per_second.powi(2);
        let second = self.samples[next_index]
            .target_speed_metres_per_second
            .powi(2);
        first + (second - first) * fraction
    }

    pub fn demand_at(
        &self,
        arc_metres: f64,
        speed_metres_per_second: f64,
        settings: &LongitudinalControlTuning,
    ) -> (f64, f64) {
        let current_speed_squared = self.speed_squared_at(arc_metres);
        let anticipation_distance = speed_metres_per_second * settings.braking_anticipation_seconds;
        let regularization_distance = (speed_metres_per_second
            * settings.acceleration_regularization_seconds)
            .max(settings.minimum_regularization_distance_metres);
        let acceleration_at = |center: f64| {
            let entry_speed_squared = self.speed_squared_at(center - regularization_distance * 0.5);
            let exit_speed_squared = self.speed_squared_at(center + regularization_distance * 0.5);
            (exit_speed_squared - entry_speed_squared) / (2.0 * regularization_distance)
        };
        let acceleration =
            acceleration_at(arc_metres).min(acceleration_at(arc_metres + anticipation_distance));
        (current_speed_squared.max(0.0).sqrt(), acceleration)
    }
}
