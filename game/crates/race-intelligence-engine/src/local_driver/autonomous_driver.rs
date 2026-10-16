use super::driver_tuning::DriverTuning;
use crate::track::path_projection::project_onto_path;
use crate::track::path_reader::RacingLine;
use super::longitudinal_control::LongitudinalController;
use super::longitudinal_profile::LongitudinalProfile;
use serde::{Deserialize, Serialize};
use crate::track::vehicle_performance::VehiclePerformanceEnvelope;
use vehicle_physics_engine::VehicleConfig;

#[derive(Clone, Deserialize)]
pub struct DriverObservation {
    pub position_x_metres: f64,
    pub position_z_metres: f64,
    pub heading_radians: f64,
    pub speed_metres_per_second: f64,
    pub duration_seconds: f64,
    pub current_gear: i8,
    #[serde(default)]
    pub vehicle_dynamics: Option<super::live_grip_control::VehicleDynamicsObservation>,
}

#[derive(Serialize)]
pub struct DriverCommand {
    pub steering_fraction: f64,
    pub throttle_fraction: f64,
    pub brake_fraction: f64,
    pub gear_request: Option<i8>,
    pub lateral_offset_metres: f64,
    pub arc_length_metres: f64,
    pub target_speed_metres_per_second: f64,
    pub available_half_width_metres: f64,
    pub vehicle_half_width_metres: f64,
    pub planned_acceleration_metres_per_second_squared: f64,
    pub recovery_speed_fraction: f64,
    pub longitudinal_demand_fraction: f64,
    pub regularized_acceleration_metres_per_second_squared: f64,
    pub braking_engaged: bool,
    pub live_grip: super::live_grip_control::LiveGripDiagnostics,
}

pub struct AutonomousDriver {
    pub racing_line: RacingLine,
    profile: LongitudinalProfile,
    previous_segment: usize,
    previous_steering: f64,
    longitudinal_control: LongitudinalController,
    tuning: DriverTuning,
    performance: VehiclePerformanceEnvelope,
    live_grip: super::live_grip_control::LiveGripController,
    recovery_speed_fraction: f64,
    vehicle_half_width_metres: f64,
    wheelbase_metres: f64,
    steering_calibration: super::steering_rack_calibration::SteeringRackCalibration,
}

impl AutonomousDriver {
    pub fn new(racing_line: RacingLine, tuning: DriverTuning, configuration: &VehicleConfig) -> Result<Self, String> {
        super::driver_tuning::validate_driver_tuning(&tuning)?;
        if tuning.maximum_lookahead_metres >= racing_line.lap_length_metres {
            return Err("Artificial driver lookahead must be shorter than the lap".into());
        }
        let performance = VehiclePerformanceEnvelope::from_configuration(configuration, tuning.grip_utilization_fraction, tuning.nominal_tire_grip_fraction)?;
        let profile = performance.speed_profile(&racing_line.waypoints, racing_line.lap_length_metres, tuning.maximum_speed_metres_per_second)?;
        let steering_calibration = super::steering_rack_calibration::SteeringRackCalibration::from_configuration(configuration)?;
        let vehicle_half_width_metres = (configuration.chassis.front_track * 0.5 + configuration.tires.front_tire_width * 0.5)
            .max(configuration.chassis.rear_track * 0.5 + configuration.tires.rear_tire_width * 0.5);
        let profile = LongitudinalProfile::new(profile, racing_line.lap_length_metres);
        Ok(Self { live_grip: super::live_grip_control::LiveGripController::default(), racing_line, profile, previous_segment: usize::MAX, previous_steering: 0.0,
            longitudinal_control: LongitudinalController::default(), tuning, performance,
            recovery_speed_fraction: 1.0, vehicle_half_width_metres, wheelbase_metres: configuration.chassis.wheelbase, steering_calibration })
    }

    pub fn command(&mut self, observation: DriverObservation) -> Result<DriverCommand, String> {
        if !observation.duration_seconds.is_finite()
            || observation.duration_seconds <= 0.0
            || observation.duration_seconds > self.tuning.live_grip_control.maximum_fast_state_age_seconds
            || !observation.speed_metres_per_second.is_finite()
            || observation.speed_metres_per_second < 0.0
        {
            return Err("Artificial driver observation time or speed is invalid".into());
        }
        let projection = project_onto_path(
            &self.racing_line.waypoints,
            self.racing_line.lap_length_metres,
            observation.position_x_metres,
            observation.position_z_metres,
            observation.heading_radians,
            self.previous_segment,
        )?;
        self.previous_segment = projection.segment_start_index;
        if self.tuning.live_grip_control.enabled {
            let planning_due = self.live_grip.observe(observation.vehicle_dynamics.ok_or("Live grip control requires vehicle dynamics")?,
                &self.tuning, observation.speed_metres_per_second, self.wheelbase_metres, &self.performance, observation.duration_seconds)?;
            if planning_due {
                self.profile.update_local_capacity(&self.racing_line.waypoints, projection.segment_start_index,
                    observation.speed_metres_per_second, &self.performance, self.live_grip.axle_scales,
                    self.live_grip.brake_efficiency, &self.tuning);
            }
        }
        let lookahead_metres =
            (observation.speed_metres_per_second * self.tuning.lookahead_time_seconds).clamp(
                self.tuning.minimum_lookahead_metres,
                self.tuning.maximum_lookahead_metres,
            );
        let mut target_index = projection.segment_start_index;
        let segment_distance_metres = (projection.arc_length_metres
            - self.racing_line.waypoints[target_index].lap_distance_metres)
            .rem_euclid(self.racing_line.lap_length_metres);
        let mut remaining_metres = lookahead_metres + segment_distance_metres;
        let (target_x_metres, target_z_metres) = loop {
            let start = &self.racing_line.waypoints[target_index];
            let next_index = (target_index + 1) % self.racing_line.waypoints.len();
            let end = &self.racing_line.waypoints[next_index];
            let distance_metres = (end.position_x_metres - start.position_x_metres)
                .hypot(end.position_z_metres - start.position_z_metres);
            if remaining_metres <= distance_metres {
                let fraction = remaining_metres / distance_metres;
                break (
                    start.position_x_metres
                        + fraction * (end.position_x_metres - start.position_x_metres),
                    start.position_z_metres
                        + fraction * (end.position_z_metres - start.position_z_metres),
                );
            }
            remaining_metres -= distance_metres;
            target_index = next_index;
        };
        let relative_x_metres = target_x_metres - observation.position_x_metres;
        let relative_z_metres = target_z_metres - observation.position_z_metres;
        let target_heading = (-relative_x_metres).atan2(-relative_z_metres);
        let heading_error = observation.heading_radians - target_heading;
        let distance_metres = relative_x_metres.hypot(relative_z_metres).max(1.0);
        let desired_wheel_angle = (2.0 * self.wheelbase_metres * heading_error.sin() / distance_metres).atan();
        let coordinated_wheel_angle = if self.tuning.live_grip_control.enabled {
            self.live_grip.coordinate_steering(desired_wheel_angle, &self.tuning)
        } else { desired_wheel_angle };
        let requested_steering = self.steering_calibration.command_for_wheel_angle(coordinated_wheel_angle);
        let maximum_change =
            self.tuning.maximum_steering_change_per_second * observation.duration_seconds;
        self.previous_steering = requested_steering.clamp(-1.0, 1.0).clamp(
            self.previous_steering - maximum_change,
            self.previous_steering + maximum_change,
        );
        let waypoint = &self.racing_line.waypoints[projection.segment_start_index];
        let next_profile_index = (projection.segment_start_index + 1) % self.racing_line.waypoints.len();
        let segment_length = (self.racing_line.waypoints[next_profile_index].lap_distance_metres
            - waypoint.lap_distance_metres).rem_euclid(self.racing_line.lap_length_metres);
        let segment_fraction = (segment_distance_metres / segment_length).clamp(0.0, 1.0);
        let next_waypoint = &self.racing_line.waypoints[next_profile_index];
        let left_width = waypoint.width_left_metres + segment_fraction * (next_waypoint.width_left_metres - waypoint.width_left_metres);
        let right_width = waypoint.width_right_metres + segment_fraction * (next_waypoint.width_right_metres - waypoint.width_right_metres);
        let half_width = if projection.lateral_offset_metres >= 0.0 { right_width } else { left_width };
        let requested_recovery = lateral_recovery_speed_fraction(projection.lateral_offset_metres, left_width, right_width,
            self.vehicle_half_width_metres, self.tuning.track_edge_margin_metres, self.tuning.recovery_speed_fraction);
        let recovery_change = self.tuning.recovery_speed_change_per_second * observation.duration_seconds;
        self.recovery_speed_fraction = requested_recovery.clamp(self.recovery_speed_fraction - recovery_change,
            self.recovery_speed_fraction + recovery_change);
        let (profile_target_speed, profile_acceleration) = self.profile.demand_at(
            projection.arc_length_metres,
            observation.speed_metres_per_second,
            &self.tuning.longitudinal_control,
        );
        let transient_speed_fraction = if self.tuning.live_grip_control.enabled {
            self.live_grip.transient_speed_fraction
        } else { 1.0 };
        let target_speed = profile_target_speed * self.recovery_speed_fraction * transient_speed_fraction;
        let planned_acceleration = profile_acceleration * self.recovery_speed_fraction.powi(2);
        let current_performance = if self.tuning.live_grip_control.enabled {
            self.performance.corrected_at_speed(observation.speed_metres_per_second, self.live_grip.axle_scales, self.live_grip.brake_efficiency)
        } else { self.performance.at_speed(observation.speed_metres_per_second) };
        let previous_pedal = self.longitudinal_control.signed_pedal_fraction;
        let (mut throttle_fraction, mut brake_fraction) = self.longitudinal_control.advance(
            &self.tuning,
            target_speed - observation.speed_metres_per_second,
            planned_acceleration,
            current_performance,
            observation.duration_seconds,
        );
        if self.tuning.live_grip_control.enabled {
            if self.live_grip.diagnostics.critical_grip_state && throttle_fraction > self.tuning.live_grip_control.critical_throttle_fraction {
                let allowed_throttle = (previous_pedal.max(0.0) - self.tuning.maximum_throttle_decrease_per_second * observation.duration_seconds)
                    .max(self.tuning.live_grip_control.critical_throttle_fraction);
                throttle_fraction = throttle_fraction.min(allowed_throttle);
                self.longitudinal_control.signed_pedal_fraction = throttle_fraction;
            }
            let brake_capacity = (current_performance.braking_deceleration_metres_per_second_squared
                - current_performance.coasting_deceleration_metres_per_second_squared).max(f64::EPSILON);
            brake_fraction = brake_fraction.min((self.live_grip.available_braking_deceleration / brake_capacity).clamp(0.0, 1.0));
        }
        Ok(DriverCommand {
            live_grip: self.live_grip.diagnostics,
            steering_fraction: self.previous_steering,
            throttle_fraction,
            brake_fraction,
            gear_request: if observation.current_gear == 0
                && observation.speed_metres_per_second < 1.0
            {
                Some(1)
            } else {
                None
            },
            lateral_offset_metres: projection.lateral_offset_metres,
            arc_length_metres: projection.arc_length_metres,
            target_speed_metres_per_second: target_speed,
            available_half_width_metres: half_width,
            vehicle_half_width_metres: self.vehicle_half_width_metres,
            planned_acceleration_metres_per_second_squared: planned_acceleration,
            recovery_speed_fraction: self.recovery_speed_fraction,
            longitudinal_demand_fraction: self.longitudinal_control.signed_pedal_fraction,
            regularized_acceleration_metres_per_second_squared: self.longitudinal_control.regularized_acceleration_metres_per_second_squared,
            braking_engaged: self.longitudinal_control.braking_engaged,
        })
    }
}

pub fn lateral_recovery_speed_fraction(offset: f64, left_width: f64, right_width: f64,
    half_vehicle_width: f64, edge_margin: f64, minimum_speed_fraction: f64) -> f64 {
    let available_width = if offset >= 0.0 { right_width } else { left_width };
    let clearance = available_width - offset.abs() - half_vehicle_width;
    let proximity = ((edge_margin - clearance) / edge_margin).clamp(0.0, 1.0);
    let smooth_proximity = proximity * proximity * (3.0 - 2.0 * proximity);
    1.0 - (1.0 - minimum_speed_fraction) * smooth_proximity
}
