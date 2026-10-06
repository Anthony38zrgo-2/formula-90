use crate::suspension_mass_properties::{vector_is_finite, InertiaTensor};
use crate::types::Vec3;

pub const SUSPENSION_GENERALIZED_VELOCITY_COUNT: usize = 10;
pub type GeneralizedSuspensionVector = [f64; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
pub type GeneralizedSuspensionMatrix =
    [[f64; SUSPENSION_GENERALIZED_VELOCITY_COUNT]; SUSPENSION_GENERALIZED_VELOCITY_COUNT];

#[derive(Debug, Clone)]
pub struct MovingSuspensionComponent {
    pub mass_kilograms: f64,
    pub inertia_world: InertiaTensor,
    pub center_of_mass_world_metres: Vec3,
    pub linear_velocity_jacobian: [Vec3; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
    pub angular_velocity_jacobian: [Vec3; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
    pub prescribed_linear_velocity_metres_per_second: Vec3,
    pub prescribed_angular_velocity_radians_per_second: Vec3,
    pub linear_acceleration_bias_metres_per_second_squared: Vec3,
    pub angular_acceleration_bias_radians_per_second_squared: Vec3,
}

#[derive(Debug, Clone)]
pub struct CoupledSuspensionEquations {
    pub mass_matrix: GeneralizedSuspensionMatrix,
    pub generalized_gravity_force: GeneralizedSuspensionVector,
    pub generalized_inertial_bias_force: GeneralizedSuspensionVector,
}

impl CoupledSuspensionEquations {
    pub fn effective_body_mass_matrix(&self) -> Result<[[f64; 6]; 6], String> {
        let mut wheel_block =
            [[0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT]; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        for coordinate in 0..6 {
            wheel_block[coordinate][coordinate] = 1.0;
        }
        for row in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            for column in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
                wheel_block[row][column] = self.mass_matrix[row][column];
            }
        }
        let mut effective_body_mass = [[0.0; 6]; 6];
        for column in 0..6 {
            let mut right_hand_side = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
            for wheel_coordinate in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
                right_hand_side[wheel_coordinate] = self.mass_matrix[wheel_coordinate][column];
            }
            let response = solve_positive_definite_matrix(&wheel_block, right_hand_side)?;
            for row in 0..6 {
                effective_body_mass[row][column] = self.mass_matrix[row][column]
                    - (6..SUSPENSION_GENERALIZED_VELOCITY_COUNT)
                        .map(|wheel_coordinate| {
                            self.mass_matrix[row][wheel_coordinate] * response[wheel_coordinate]
                        })
                        .sum::<f64>();
            }
        }
        Ok(effective_body_mass)
    }

    pub fn project_travel_stop_velocities(
        &self,
        velocity: &mut GeneralizedSuspensionVector,
        active_stops: [bool; 4],
    ) -> Result<f64, String> {
        let mut constraint_matrix =
            [[0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT]; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        let mut right_hand_side = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        let mut responses = [[0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT]; 4];
        for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            constraint_matrix[coordinate][coordinate] = 1.0;
        }
        for wheel in 0..4 {
            if active_stops[wheel] {
                let mut constraint = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
                constraint[6 + wheel] = 1.0;
                responses[wheel] = solve_positive_definite_matrix(&self.mass_matrix, constraint)?;
                right_hand_side[6 + wheel] = -velocity[6 + wheel];
            }
        }
        for row in 0..4 {
            for column in 0..4 {
                if active_stops[row] && active_stops[column] {
                    constraint_matrix[6 + row][6 + column] = responses[column][6 + row];
                }
            }
        }
        let impulses = solve_positive_definite_matrix(&constraint_matrix, right_hand_side)?;
        let energy_before = self.kinetic_energy_joules(*velocity);
        for wheel in 0..4 {
            for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
                velocity[coordinate] += responses[wheel][coordinate] * impulses[6 + wheel];
            }
        }
        Ok((energy_before - self.kinetic_energy_joules(*velocity)).max(0.0))
    }
    pub fn assemble(
        components: &[MovingSuspensionComponent],
        velocity: GeneralizedSuspensionVector,
        gravity_world: Vec3,
    ) -> Result<Self, String> {
        if velocity.iter().any(|entry| !entry.is_finite()) || !vector_is_finite(gravity_world) {
            return Err("Nonfinite generalized velocity or gravity".into());
        }
        let mut equations = Self {
            mass_matrix: [[0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
                SUSPENSION_GENERALIZED_VELOCITY_COUNT],
            generalized_gravity_force: [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
            generalized_inertial_bias_force: [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
        };
        for component in components {
            component.validate()?;
            let angular_velocity = component.angular_velocity(velocity);
            let inertial_torque = component
                .inertia_world
                .multiply(component.angular_acceleration_bias_radians_per_second_squared)
                + angular_velocity.cross(component.inertia_world.multiply(angular_velocity));
            for row in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
                equations.generalized_gravity_force[row] += component.linear_velocity_jacobian[row]
                    .dot(gravity_world * component.mass_kilograms);
                equations.generalized_inertial_bias_force[row] +=
                    component.linear_velocity_jacobian[row].dot(
                        component.linear_acceleration_bias_metres_per_second_squared
                            * component.mass_kilograms,
                    ) + component.angular_velocity_jacobian[row].dot(inertial_torque);
                for column in 0..=row {
                    let contribution = component.mass_kilograms
                        * component.linear_velocity_jacobian[row]
                            .dot(component.linear_velocity_jacobian[column])
                        + component.angular_velocity_jacobian[row].dot(
                            component
                                .inertia_world
                                .multiply(component.angular_velocity_jacobian[column]),
                        );
                    equations.mass_matrix[row][column] += contribution;
                    if row != column {
                        equations.mass_matrix[column][row] += contribution;
                    }
                }
            }
        }
        factor_positive_definite_matrix(&equations.mass_matrix)?;
        Ok(equations)
    }

    pub fn acceleration(
        &self,
        generalized_applied_force: GeneralizedSuspensionVector,
    ) -> Result<GeneralizedSuspensionVector, String> {
        let mut right_hand_side = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            right_hand_side[coordinate] = generalized_applied_force[coordinate]
                + self.generalized_gravity_force[coordinate]
                - self.generalized_inertial_bias_force[coordinate];
        }
        solve_positive_definite_matrix(&self.mass_matrix, right_hand_side)
    }

    pub fn kinetic_energy_joules(&self, velocity: GeneralizedSuspensionVector) -> f64 {
        let mut energy = 0.0;
        for row in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            for column in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
                energy += 0.5 * velocity[row] * self.mass_matrix[row][column] * velocity[column];
            }
        }
        energy
    }

    pub fn apply_body_velocity_change(
        &self,
        velocity: &mut GeneralizedSuspensionVector,
        body_velocity_change: [f64; 6],
    ) -> Result<f64, String> {
        if body_velocity_change.iter().any(|entry| !entry.is_finite()) {
            return Err("Nonfinite body collision velocity change".into());
        }
        let mut wheel_block =
            [[0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT]; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        let mut right_hand_side = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        for body_coordinate in 0..6 {
            wheel_block[body_coordinate][body_coordinate] = 1.0;
        }
        for wheel_coordinate in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            for other_wheel_coordinate in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
                wheel_block[wheel_coordinate][other_wheel_coordinate] =
                    self.mass_matrix[wheel_coordinate][other_wheel_coordinate];
            }
            for body_coordinate in 0..6 {
                right_hand_side[wheel_coordinate] -= self.mass_matrix[wheel_coordinate]
                    [body_coordinate]
                    * body_velocity_change[body_coordinate];
            }
        }
        let correction = solve_positive_definite_matrix(&wheel_block, right_hand_side)?;
        let energy_before = self.kinetic_energy_joules(*velocity);
        for coordinate in 0..6 {
            velocity[coordinate] += body_velocity_change[coordinate];
        }
        for coordinate in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            velocity[coordinate] += correction[coordinate];
        }
        Ok(self.kinetic_energy_joules(*velocity) - energy_before)
    }
}

impl MovingSuspensionComponent {
    pub fn validate(&self) -> Result<(), String> {
        if !self.mass_kilograms.is_finite() || self.mass_kilograms <= 0.0 {
            return Err("Moving component requires finite positive mass".into());
        }
        if !self.inertia_world.is_point_mass() {
            self.inertia_world.validate()?;
        }
        if self
            .linear_velocity_jacobian
            .iter()
            .chain(self.angular_velocity_jacobian.iter())
            .any(|vector| !vector_is_finite(*vector))
            || !vector_is_finite(self.center_of_mass_world_metres)
            || !vector_is_finite(self.prescribed_linear_velocity_metres_per_second)
            || !vector_is_finite(self.prescribed_angular_velocity_radians_per_second)
            || !vector_is_finite(self.linear_acceleration_bias_metres_per_second_squared)
            || !vector_is_finite(self.angular_acceleration_bias_radians_per_second_squared)
        {
            return Err("Moving component contains nonfinite kinematics".into());
        }
        Ok(())
    }

    pub fn linear_velocity(&self, velocity: GeneralizedSuspensionVector) -> Vec3 {
        self.linear_velocity_jacobian.iter().zip(velocity).fold(
            self.prescribed_linear_velocity_metres_per_second,
            |sum, (derivative, speed)| sum + *derivative * speed,
        )
    }

    pub fn angular_velocity(&self, velocity: GeneralizedSuspensionVector) -> Vec3 {
        self.angular_velocity_jacobian.iter().zip(velocity).fold(
            self.prescribed_angular_velocity_radians_per_second,
            |sum, (derivative, speed)| sum + *derivative * speed,
        )
    }

    pub fn kinetic_energy_joules(&self, velocity: GeneralizedSuspensionVector) -> f64 {
        let angular_velocity = self.angular_velocity(velocity);
        0.5 * self.mass_kilograms * self.linear_velocity(velocity).length_squared()
            + 0.5 * angular_velocity.dot(self.inertia_world.multiply(angular_velocity))
    }

    pub fn generalized_force(
        &self,
        force_world: Vec3,
        torque_about_center_world: Vec3,
    ) -> GeneralizedSuspensionVector {
        let mut generalized_force = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            generalized_force[coordinate] = self.linear_velocity_jacobian[coordinate]
                .dot(force_world)
                + self.angular_velocity_jacobian[coordinate].dot(torque_about_center_world);
        }
        generalized_force
    }
}

pub fn factor_positive_definite_matrix(
    matrix: &GeneralizedSuspensionMatrix,
) -> Result<GeneralizedSuspensionMatrix, String> {
    let mut scaling = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    for row in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
        if !matrix[row][row].is_finite() || matrix[row][row] <= 0.0 {
            return Err("Mass matrix must have finite positive diagonal entries".into());
        }
        scaling[row] = matrix[row][row].sqrt();
    }
    let mut factor =
        [[0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT]; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    for row in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
        for column in 0..=row {
            if !matrix[row][column].is_finite()
                || !matrix[column][row].is_finite()
                || (matrix[row][column] - matrix[column][row]).abs()
                    > 1e-10 * scaling[row] * scaling[column]
            {
                return Err("Mass matrix must be finite and symmetric".into());
            }
            let remainder = matrix[row][column] / (scaling[row] * scaling[column])
                - (0..column)
                    .map(|previous| factor[row][previous] * factor[column][previous])
                    .sum::<f64>();
            if row == column {
                if !remainder.is_finite() || remainder <= 1e-12 {
                    return Err("Mass matrix is singular or not positive definite".into());
                }
                factor[row][column] = remainder.sqrt();
            } else {
                factor[row][column] = remainder / factor[column][column];
            }
        }
    }
    for row in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
        for column in 0..=row {
            factor[row][column] *= scaling[row];
        }
    }
    Ok(factor)
}

pub fn solve_positive_definite_matrix(
    matrix: &GeneralizedSuspensionMatrix,
    right_hand_side: GeneralizedSuspensionVector,
) -> Result<GeneralizedSuspensionVector, String> {
    if right_hand_side.iter().any(|entry| !entry.is_finite()) {
        return Err("Nonfinite generalized force".into());
    }
    let factor = factor_positive_definite_matrix(matrix)?;
    let mut solution = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    for row in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
        solution[row] = (right_hand_side[row]
            - (0..row)
                .map(|column| factor[row][column] * solution[column])
                .sum::<f64>())
            / factor[row][row];
    }
    for row in (0..SUSPENSION_GENERALIZED_VELOCITY_COUNT).rev() {
        solution[row] = (solution[row]
            - (row + 1..SUSPENSION_GENERALIZED_VELOCITY_COUNT)
                .map(|column| factor[column][row] * solution[column])
                .sum::<f64>())
            / factor[row][row];
    }
    if solution.iter().any(|entry| !entry.is_finite()) {
        return Err("Nonfinite generalized acceleration".into());
    }
    Ok(solution)
}
