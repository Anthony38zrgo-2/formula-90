use crate::suspension_multibody::GeneralizedSuspensionVector;

#[derive(Clone, Debug)]
pub enum SuspensionDampingLaw {
    Digressive { compression: f64, rebound: f64, knee: f64, fast_fraction: f64 },
    UnilateralContact { coefficient: f64, elastic_force: f64 },
    TravelStop { coefficient: f64, outward_sign: f64 },
}

#[derive(Clone, Debug)]
pub struct SuspensionDampingElement {
    pub velocity_derivative: GeneralizedSuspensionVector,
    pub prescribed_velocity: f64,
    pub law: SuspensionDampingLaw,
}

impl SuspensionDampingElement {
    pub fn relative_velocity(&self, velocity: &[f64]) -> f64 {
        self.prescribed_velocity + self.velocity_derivative.iter().zip(velocity).map(|(derivative, value)| derivative * value).sum::<f64>()
    }

    pub fn response(&self, velocity: f64) -> (f64, f64) {
        match self.law {
            SuspensionDampingLaw::Digressive { compression, rebound, knee, fast_fraction } => {
                let coefficient = if velocity >= 0.0 { compression } else { rebound };
                (velocity.signum() * coefficient * (velocity.abs().min(knee) + (velocity.abs() - knee).max(0.0) * fast_fraction),
                    coefficient * if velocity.abs() <= knee { 1.0 } else { fast_fraction })
            }
            SuspensionDampingLaw::UnilateralContact { coefficient, elastic_force } => {
                if coefficient * velocity <= elastic_force { (coefficient * velocity, coefficient) } else { (elastic_force, 0.0) }
            }
            SuspensionDampingLaw::TravelStop { coefficient, outward_sign } => {
                if velocity * outward_sign > 0.0 { (coefficient * velocity, coefficient) } else { (0.0, 0.0) }
            }
        }
    }
}

pub fn integrate_suspension_damping<const COORDINATE_COUNT: usize>(
    mass_matrix: &[[f64; COORDINATE_COUNT]; COORDINATE_COUNT], generalized_force: [f64; COORDINATE_COUNT],
    damping_elements: &[SuspensionDampingElement], previous_velocity: [f64; COORDINATE_COUNT], duration_seconds: f64,
    solve_matrix: impl Fn(&[[f64; COORDINATE_COUNT]; COORDINATE_COUNT], [f64; COORDINATE_COUNT]) -> Result<[f64; COORDINATE_COUNT], String>,
) -> Result<([f64; COORDINATE_COUNT], f64, f64), String> {
    let mut conservative_force = generalized_force;
    for element in damping_elements {
        let force = element.response(element.relative_velocity(&previous_velocity)).0;
        for coordinate in 0..10 { conservative_force[coordinate] += element.velocity_derivative[coordinate] * force; }
    }
    let conservative_acceleration = solve_matrix(mass_matrix, conservative_force)?;
    let free_velocity: [f64; COORDINATE_COUNT] = std::array::from_fn(|coordinate| previous_velocity[coordinate] + conservative_acceleration[coordinate] * duration_seconds);
    let momentum: [f64; COORDINATE_COUNT] = std::array::from_fn(|row|
        mass_matrix[row].iter().zip(free_velocity).map(|(mass, velocity)| mass * velocity).sum());
    let mut velocity = previous_velocity;
    for iteration in 0..32 {
        let mut matrix = *mass_matrix;
        let mut right_hand_side = momentum;
        let midpoint_velocity: [f64; COORDINATE_COUNT] = std::array::from_fn(|coordinate| 0.5 * (previous_velocity[coordinate] + velocity[coordinate]));
        for element in damping_elements {
            let (force, tangent) = element.response(element.relative_velocity(&midpoint_velocity));
            let dynamic_velocity: f64 = element.velocity_derivative.iter().zip(velocity).map(|(derivative, value)| derivative * value).sum();
            for row in 0..10 {
                right_hand_side[row] -= duration_seconds * element.velocity_derivative[row] * (force - 0.5 * tangent * dynamic_velocity);
                for column in 0..10 {
                    matrix[row][column] += 0.5 * duration_seconds * tangent * element.velocity_derivative[row] * element.velocity_derivative[column];
                }
            }
        }
        let next_velocity = solve_matrix(&matrix, right_hand_side)?;
        let maximum_change = next_velocity.iter().zip(velocity).map(|(next, previous)| (next - previous).abs()).fold(0.0, f64::max);
        velocity = next_velocity;
        if maximum_change <= 1e-10 * velocity.iter().map(|value| value.abs()).fold(1.0, f64::max) {
            let midpoint_velocity: [f64; COORDINATE_COUNT] = std::array::from_fn(|coordinate| 0.5 * (previous_velocity[coordinate] + velocity[coordinate]));
            let mut dissipated_energy = 0.0;
            let mut steering_work_correction = 0.0;
            for element in damping_elements {
                let force = element.response(element.relative_velocity(&midpoint_velocity)).0;
                let previous_force = element.response(element.relative_velocity(&previous_velocity)).0;
                dissipated_energy += force * element.relative_velocity(&midpoint_velocity) * duration_seconds;
                steering_work_correction += (force - previous_force) * element.prescribed_velocity * duration_seconds;
            }
            if !dissipated_energy.is_finite() || dissipated_energy < -1e-8 {
                return Err("Implicit suspension damping violated discrete passivity".into());
            }
            return Ok((velocity, dissipated_energy.max(0.0), steering_work_correction));
        }
        if iteration == 31 { return Err("Implicit suspension damping exceeded its convergence budget".into()); }
    }
    unreachable!()
}


