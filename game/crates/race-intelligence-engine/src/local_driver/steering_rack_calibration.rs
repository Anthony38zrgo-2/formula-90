use vehicle_physics_engine::{VehicleConfig, WheelIndex, Vec3};
use vehicle_physics_engine::suspension_kinematics::solve_corner;

pub struct SteeringRackCalibration {
    samples: Vec<(f64, f64)>,
}

impl SteeringRackCalibration {
    pub fn from_configuration(configuration: &VehicleConfig) -> Result<Self, String> {
        let geometry = configuration.suspension.geometric_suspension.as_ref().ok_or("Steering rack calibration requires physical suspension geometry")?;
        let travel = configuration.coupled_world.as_ref().ok_or("Steering rack calibration requires the physical world")?.steering_rack_travel_metres;
        let mut samples = Vec::new();
        for index in 0..=100 {
            let command = -1.0 + index as f64 / 50.0;
            let mut angle = 0.0;
            let mut valid = true;
            for wheel in [WheelIndex::FrontLeft, WheelIndex::FrontRight] {
                let solution = solve_corner(geometry.corners.get(wheel), 0.0, command * travel, true,
                    configuration.suspension.front_camber, configuration.suspension.front_toe,
                    if wheel.is_left() { 1.0 } else { -1.0 }).ok_or("Physical steering calibration failed")?;
                if !solution.converged || solution.steering_clamped { valid = false; break; }
                let forward = solution.wheel_basis.transform_vector(Vec3::FORWARD);
                angle += forward.x.atan2(-forward.z) * 0.5;
            }
            if valid { samples.push((angle, command)); }
        }
        samples.sort_by(|first, second| first.0.total_cmp(&second.0));
        if samples.len() < 3 || samples[0].0 >= 0.0 || samples.last().unwrap().0 <= 0.0 {
            return Err("Physical steering calibration has no usable steering range".into());
        }
        Ok(Self { samples })
    }

    pub fn command_for_wheel_angle(&self, angle: f64) -> f64 {
        let angle = angle.clamp(self.samples[0].0, self.samples.last().unwrap().0);
        let upper = self.samples.partition_point(|sample| sample.0 < angle).clamp(1, self.samples.len() - 1);
        let first = self.samples[upper - 1];
        let second = self.samples[upper];
        let fraction = (angle - first.0) / (second.0 - first.0).max(1e-9);
        first.1 + fraction * (second.1 - first.1)
    }

    pub fn canonical_command_mapping(&self, configuration: &VehicleConfig) -> Vec<f64> {
        (0..=1000).map(|index| {
            let driver_command = -1.0 + 2.0 * index as f64 / 1000.0;
            let (mut lower_angle, mut upper_angle) = (-1.0, 1.0);
            for _ in 0..64 {
                let middle = (lower_angle + upper_angle) * 0.5;
                if self.command_for_wheel_angle(middle) < driver_command {
                    lower_angle = middle;
                } else {
                    upper_angle = middle;
                }
            }
            let target_angle = -(lower_angle + upper_angle) * 0.5;
            let (mut lower_command, mut upper_command) = (-1.0, 1.0);
            for _ in 0..64 {
                let middle = (lower_command + upper_command) * 0.5;
                let angle = [WheelIndex::FrontLeft, WheelIndex::FrontRight].into_iter()
                    .map(|wheel| vehicle_physics_engine::simulation::steering_angle_for_wheel_geo(
                        configuration, wheel, middle, 0.0))
                    .sum::<f64>() * 0.5;
                if angle < target_angle {
                    lower_command = middle;
                } else {
                    upper_command = middle;
                }
            }
            (lower_command + upper_command) * 0.5
        }).collect()
    }
}
