use vehicle_physics_engine::aero::{AeroEnvironment, AeroForces};
use vehicle_physics_engine::tire::TireSystem;
use vehicle_physics_engine::{SurfaceType, Vec3, VehicleConfig, WheelIndex};
use super::path_reader::RacingLineWaypoint;
use super::speed_profile::SpeedProfileSample;

#[derive(Clone, Copy)]
pub struct PerformanceSample {
    pub speed_metres_per_second: f64,
    pub lateral_acceleration_metres_per_second_squared: f64,
    pub braking_deceleration_metres_per_second_squared: f64,
}

pub struct VehiclePerformanceEnvelope {
    samples: Vec<PerformanceSample>,
    pub mechanical_maximum_speed_metres_per_second: f64,
    grip_utilization: f64,
}

impl VehiclePerformanceEnvelope {
    pub fn from_configuration(configuration: &VehicleConfig, grip_utilization: f64, nominal_tire_grip: f64) -> Result<Self, String> {
        if !(0.0..=1.0).contains(&grip_utilization) || grip_utilization == 0.0
            || !(0.0..=1.0).contains(&nominal_tire_grip) || nominal_tire_grip == 0.0 {
            return Err("Vehicle performance grip fractions are invalid".into());
        }
        let mass = configuration.chassis.total_vehicle_mass(&configuration.tires, &configuration.fuel);
        let gravity = configuration.coupled_world.as_ref().ok_or("Physical performance requires the coupled vehicle configuration")?
            .suspension.gravity_world_metres_per_second_squared.y.abs();
        let friction = *configuration.tires.surface_friction.get(&SurfaceType::Road).ok_or("Road friction is unavailable")?;
        let longitudinal_ratio = *configuration.tires.surface_longitudinal_grip_ratio.get(&SurfaceType::Road).ok_or("Road longitudinal grip is unavailable")?;
        let front_distribution = configuration.chassis.effective_front_weight_distribution(&configuration.tires, &configuration.fuel);
        let highest_ratio = configuration.powertrain.gear_ratios.last().ok_or("Vehicle has no forward gears")?.abs()
            * configuration.powertrain.final_drive.abs();
        let maximum_speed = configuration.powertrain.max_rpm * std::f64::consts::TAU / 60.0
            * configuration.tires.rear_tire_radius / highest_ratio;
        if !maximum_speed.is_finite() || maximum_speed <= 0.0 || !mass.is_finite() || mass <= 0.0 {
            return Err("Vehicle mechanical speed or mass is invalid".into());
        }
        let mut samples = Vec::new();
        let sample_count = 100;
        for sample_index in 0..=sample_count {
            let speed = maximum_speed * sample_index as f64 / sample_count as f64;
            let mut aerodynamics = AeroForces::zero();
            for _ in 0..120 {
                aerodynamics.step_with_environment(configuration, Vec3::new(0.0, 0.0, -speed), &AeroEnvironment::default(), 0.0, 1.0 / 120.0);
            }
            let front_load = mass * gravity * front_distribution + aerodynamics.front_axle_downforce.max(0.0);
            let rear_load = mass * gravity * (1.0 - front_distribution) + aerodynamics.rear_axle_downforce.max(0.0);
            let mut tires = TireSystem::new(configuration);
            let mut axle_capacities = [0.0; 2];
            for (wheel_index, wheel) in [WheelIndex::FrontLeft, WheelIndex::FrontRight, WheelIndex::RearLeft, WheelIndex::RearRight].into_iter().enumerate() {
                let axle_index = if wheel_index < 2 { 0 } else { 1 };
                let load = if axle_index == 0 { front_load * 0.5 } else { rear_load * 0.5 };
                tires.set_mechanical_state(configuration, wheel, load, 0.0, 0.0, 1.0);
                axle_capacities[axle_index] += load * friction * tires.wheels[wheel_index].load_sensitivity_scale * nominal_tire_grip;
            }
            let lateral_capacity = (axle_capacities[0] / (mass * front_distribution))
                .min(axle_capacities[1] / (mass * (1.0 - front_distribution))) * grip_utilization;
            let front_brake_force = configuration.brakes.max_brake_torque * configuration.brakes.front_brake_bias / configuration.tires.front_tire_radius;
            let rear_brake_force = configuration.brakes.max_brake_torque * (1.0 - configuration.brakes.front_brake_bias) / configuration.tires.rear_tire_radius;
            let braking_force = front_brake_force.min(axle_capacities[0] * longitudinal_ratio)
                + rear_brake_force.min(axle_capacities[1] * longitudinal_ratio);
            let braking_capacity = (braking_force + aerodynamics.drag_force.max(0.0)) / mass;
            if !lateral_capacity.is_finite() || !braking_capacity.is_finite() || lateral_capacity <= 0.0 || braking_capacity <= 0.0 {
                return Err("Vehicle performance model produced invalid force capacity".into());
            }
            samples.push(PerformanceSample { speed_metres_per_second: speed,
                lateral_acceleration_metres_per_second_squared: lateral_capacity,
                braking_deceleration_metres_per_second_squared: braking_capacity });
        }
        Ok(Self { samples, mechanical_maximum_speed_metres_per_second: maximum_speed, grip_utilization })
    }

    pub fn at_speed(&self, speed: f64) -> PerformanceSample {
        let coordinate = (speed / self.mechanical_maximum_speed_metres_per_second).clamp(0.0, 1.0) * (self.samples.len() - 1) as f64;
        let index = (coordinate.floor() as usize).min(self.samples.len() - 2);
        let fraction = coordinate - index as f64;
        let first = self.samples[index];
        let second = self.samples[index + 1];
        PerformanceSample { speed_metres_per_second: speed,
            lateral_acceleration_metres_per_second_squared: first.lateral_acceleration_metres_per_second_squared
                + fraction * (second.lateral_acceleration_metres_per_second_squared - first.lateral_acceleration_metres_per_second_squared),
            braking_deceleration_metres_per_second_squared: first.braking_deceleration_metres_per_second_squared
                + fraction * (second.braking_deceleration_metres_per_second_squared - first.braking_deceleration_metres_per_second_squared) }
    }

    pub fn speed_profile(&self, waypoints: &[RacingLineWaypoint], lap_length: f64, speed_cap: Option<f64>) -> Result<Vec<SpeedProfileSample>, String> {
        super::speed_profile::validate_profile_geometry(waypoints, lap_length)?;
        let maximum_speed = speed_cap.unwrap_or(self.mechanical_maximum_speed_metres_per_second).min(self.mechanical_maximum_speed_metres_per_second);
        let mut profile = Vec::with_capacity(waypoints.len());
        for index in 0..waypoints.len() {
            let curvature = super::speed_profile::curvature_from_triplet(
                &waypoints[(index + waypoints.len() - 2) % waypoints.len()], &waypoints[index], &waypoints[(index + 2) % waypoints.len()]);
            let mut lower = 0.0;
            let mut upper = maximum_speed;
            for _ in 0..40 {
                let speed = (lower + upper) * 0.5;
                if speed * speed * curvature.abs() <= self.at_speed(speed).lateral_acceleration_metres_per_second_squared { lower = speed; } else { upper = speed; }
            }
            profile.push(SpeedProfileSample { arc_length_metres: waypoints[index].lap_distance_metres,
                target_speed_metres_per_second: lower, curvature_per_metre: curvature });
        }
        for _ in 0..=waypoints.len() {
            let mut changed = false;
            for index in (0..waypoints.len()).rev() {
                let next = (index + 1) % waypoints.len();
                let distance = (profile[next].arc_length_metres - profile[index].arc_length_metres).rem_euclid(lap_length);
                let speed = profile[index].target_speed_metres_per_second;
                let capacity = self.at_speed(speed);
                let lateral_fraction = (speed.powi(2) * profile[index].curvature_per_metre.abs()
                    / capacity.lateral_acceleration_metres_per_second_squared * self.grip_utilization).clamp(0.0, 1.0);
                let slope = (waypoints[next].position_y_metres - waypoints[index].position_y_metres) / distance;
                let deceleration = (capacity.braking_deceleration_metres_per_second_squared * self.grip_utilization
                    * (1.0 - lateral_fraction.powi(2)).max(0.0).sqrt() + 9.80665 * slope).max(0.5);
                let reachable = (profile[next].target_speed_metres_per_second.powi(2) + 2.0 * deceleration * distance).sqrt();
                if reachable + 1e-8 < speed { profile[index].target_speed_metres_per_second = reachable; changed = true; }
            }
            if !changed { return Ok(profile); }
        }
        Err("Vehicle-derived speed profile did not converge".into())
    }
}
