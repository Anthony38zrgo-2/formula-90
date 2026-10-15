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
}
