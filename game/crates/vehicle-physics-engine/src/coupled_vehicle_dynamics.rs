use crate::suspension_component_kinematics::{component_pose, wheel_contact_component, ComponentKinematicInput};
use crate::suspension_coupled_solver::{CoupledSuspensionDiagnostics, CoupledSuspensionInput, CoupledSuspensionModel, CoupledSuspensionSettings, CoupledSuspensionState};
use crate::suspension_kinematics::{solve_corner, travel_envelope};
use crate::types::{Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use crate::vehicle_mass_inventory::VehicleMassInventory;
use serde::{Deserialize, Serialize};

pub const VEHICLE_GENERALIZED_VELOCITY_COUNT: usize = 14;
pub type GeneralizedVehicleVelocity = [f64; VEHICLE_GENERALIZED_VELOCITY_COUNT];
pub type GeneralizedVehicleMatrix = [[f64; VEHICLE_GENERALIZED_VELOCITY_COUNT]; VEHICLE_GENERALIZED_VELOCITY_COUNT];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoupledVehicleState {
    pub suspension: CoupledSuspensionState,
    pub wheel_angular_velocity_radians_per_second: [f64; 4],
    pub wheel_spin_angle_radians: [f64; 4],
    pub fuel_exchange_energy_joules: f64,
    pub actuator_work_joules: f64,
    pub brake_dissipated_energy_joules: f64,
    pub position_correction_energy_joules: f64,
}

impl CoupledVehicleState {
    pub fn at_rest(position_world_metres: Vec3) -> Self {
        Self { suspension: CoupledSuspensionState::at_rest(position_world_metres),
            wheel_angular_velocity_radians_per_second: [0.0; 4], wheel_spin_angle_radians: [0.0; 4],
            fuel_exchange_energy_joules: 0.0, actuator_work_joules: 0.0, brake_dissipated_energy_joules: 0.0, position_correction_energy_joules: 0.0 }
    }

    pub fn velocity(&self) -> GeneralizedVehicleVelocity {
        let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        velocity[..10].copy_from_slice(&self.suspension.generalized_velocity);
        velocity[10..].copy_from_slice(&self.wheel_angular_velocity_radians_per_second);
        velocity
    }

    pub fn set_velocity(&mut self, velocity: GeneralizedVehicleVelocity) {
        self.suspension.generalized_velocity.copy_from_slice(&velocity[..10]);
        self.wheel_angular_velocity_radians_per_second.copy_from_slice(&velocity[10..]);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneralizedWheelBrakingResult {
    pub impulses_newton_metre_seconds: [f64; 4],
    pub dissipated_energy_joules: f64,
    pub dissipated_energy_joules_by_wheel: [f64; 4],
    pub iterations: usize,
}

pub fn apply_generalized_wheel_braking(matrix: &GeneralizedVehicleMatrix, velocity: &mut GeneralizedVehicleVelocity,
    torque_limits_newton_metres: [f64; 4], duration_seconds: f64, maximum_iterations: usize,
    angular_velocity_tolerance_radians_per_second: f64) -> Result<GeneralizedWheelBrakingResult, String> {
    if torque_limits_newton_metres.iter().any(|value| !value.is_finite() || *value < 0.0)
        || velocity.iter().any(|value| !value.is_finite()) || !duration_seconds.is_finite() || duration_seconds <= 0.0
        || maximum_iterations == 0 || !angular_velocity_tolerance_radians_per_second.is_finite()
        || angular_velocity_tolerance_radians_per_second <= 0.0
    { return Err("Invalid generalized wheel braking state or solver budget".into()); }
    let factor = factor_generalized_vehicle_matrix(matrix)?;
    let mut responses = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; 4];
    for wheel in 0..4 {
        let mut force = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        force[10 + wheel] = 1.0;
        responses[wheel] = solve_factored_vehicle_matrix(&factor, force)?;
    }
    let mut candidate = *velocity;
    let initial_velocity = *velocity;
    let initial_energy = generalized_vehicle_energy(matrix, candidate);
    let mut impulses = [0.0; 4];
    for iteration in 0..maximum_iterations {
        for wheel in 0..4 {
            let bound = torque_limits_newton_metres[wheel] * duration_seconds;
            let next = (impulses[wheel] - candidate[10 + wheel] / responses[wheel][10 + wheel]).clamp(-bound, bound);
            let change = next - impulses[wheel];
            impulses[wheel] = next;
            for coordinate in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT { candidate[coordinate] += responses[wheel][coordinate] * change; }
        }
        let residual = (0..4).map(|wheel| {
            let bound = torque_limits_newton_metres[wheel] * duration_seconds;
            let projected = (impulses[wheel] - candidate[10 + wheel] / responses[wheel][10 + wheel]).clamp(-bound, bound);
            (projected - impulses[wheel]).abs() * responses[wheel][10 + wheel]
        }).fold(0.0, f64::max);
        if residual <= angular_velocity_tolerance_radians_per_second {
            let energy_loss = initial_energy - generalized_vehicle_energy(matrix, candidate);
            if energy_loss < -1e-8 * initial_energy.abs().max(1.0) { return Err("Generalized wheel braking added unexplained energy".into()); }
            let dissipated_energy_joules_by_wheel = std::array::from_fn(|wheel|
                -impulses[wheel] * (candidate[10 + wheel] + initial_velocity[10 + wheel]) * 0.5);
            if dissipated_energy_joules_by_wheel.iter().any(|energy| *energy < -1e-8 * initial_energy.abs().max(1.0)) {
                return Err("Brake interval crosses a spin reversal and requires further event subdivision".into());
            }
            *velocity = candidate;
            return Ok(GeneralizedWheelBrakingResult { impulses_newton_metre_seconds: impulses,
                dissipated_energy_joules: energy_loss.max(0.0), dissipated_energy_joules_by_wheel, iterations: iteration + 1 });
        }
    }
    Err("Generalized wheel braking exceeded its convergence budget; state was not committed".into())
}

pub struct CoupledVehicleEvaluation {
    pub mass_matrix: GeneralizedVehicleMatrix,
    pub generalized_force: GeneralizedVehicleVelocity,
    pub suspension_diagnostics: CoupledSuspensionDiagnostics,
    pub kinetic_energy_joules: f64,
    pub prescribed_generalized_momentum: GeneralizedVehicleVelocity,
}

#[derive(Debug, Clone)]
pub struct CoupledVehicleModel {
    pub configuration: VehicleConfig,
    pub inventory: VehicleMassInventory,
    pub settings: CoupledSuspensionSettings,
    suspension_model: CoupledSuspensionModel,
    pub travel_limits_metres: [(f64, f64); 4],
}

pub fn factor_generalized_vehicle_matrix(matrix: &GeneralizedVehicleMatrix) -> Result<GeneralizedVehicleMatrix, String> {
    let mut factor = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    let mut scale = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    for row in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
        if !matrix[row][row].is_finite() || matrix[row][row] <= 0.0 { return Err("Generalized vehicle matrix requires positive finite diagonal entries".into()); }
        scale[row] = matrix[row][row].sqrt();
    }
    for row in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
        for column in 0..=row {
            if !matrix[row][column].is_finite() || !matrix[column][row].is_finite()
                || (matrix[row][column] - matrix[column][row]).abs() > 1e-10 * scale[row] * scale[column]
            { return Err("Generalized vehicle matrix must be finite and symmetric".into()); }
            let remainder = matrix[row][column] / (scale[row] * scale[column])
                - (0..column).map(|previous| factor[row][previous] * factor[column][previous]).sum::<f64>();
            if row == column {
                if remainder <= 1e-12 || !remainder.is_finite() { return Err("Generalized vehicle matrix is singular or indefinite".into()); }
                factor[row][column] = remainder.sqrt();
            } else { factor[row][column] = remainder / factor[column][column]; }
        }
    }
    for row in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT { for column in 0..=row { factor[row][column] *= scale[row]; } }
    Ok(factor)
}

pub fn solve_factored_vehicle_matrix(factor: &GeneralizedVehicleMatrix, force: GeneralizedVehicleVelocity) -> Result<GeneralizedVehicleVelocity, String> {
    if force.iter().any(|value| !value.is_finite()) { return Err("Nonfinite generalized vehicle force".into()); }
    let mut solution = force;
    for row in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
        solution[row] = (solution[row] - (0..row).map(|column| factor[row][column] * solution[column]).sum::<f64>()) / factor[row][row];
    }
    for row in (0..VEHICLE_GENERALIZED_VELOCITY_COUNT).rev() {
        solution[row] = (solution[row] - (row + 1..VEHICLE_GENERALIZED_VELOCITY_COUNT).map(|column| factor[column][row] * solution[column]).sum::<f64>()) / factor[row][row];
    }
    if solution.iter().any(|value| !value.is_finite()) { return Err("Nonfinite generalized vehicle response".into()); }
    Ok(solution)
}

pub fn generalized_vehicle_energy(matrix: &GeneralizedVehicleMatrix, velocity: GeneralizedVehicleVelocity) -> f64 {
    0.5 * (0..VEHICLE_GENERALIZED_VELOCITY_COUNT).map(|row|
        velocity[row] * (0..VEHICLE_GENERALIZED_VELOCITY_COUNT).map(|column| matrix[row][column] * velocity[column]).sum::<f64>()).sum::<f64>()
}

pub fn generalized_projection(first: GeneralizedVehicleVelocity, second: GeneralizedVehicleVelocity) -> f64 {
    first.iter().zip(second).map(|(first, second)| first * second).sum()
}

impl CoupledVehicleModel {
    pub fn new(configuration: VehicleConfig, settings: CoupledSuspensionSettings) -> Result<Self, String> {
        let inventory = configuration.physical_mass_inventory.clone().ok_or("Coupled vehicle requires a complete physical mass inventory")?;
        inventory.validate(&configuration)?;
        let suspension_model = CoupledSuspensionModel::new(configuration.clone(), inventory.reference_inventory(), settings.clone())?;
        let geometry = configuration.geometric_suspension.as_ref().ok_or("Missing coupled vehicle hardpoints")?;
        let mut travel_limits_metres = [(0.0, 0.0); 4];
        for wheel in WheelIndex::ALL {
            let axle = geometry.axle(wheel);
            travel_limits_metres[wheel as usize] = travel_envelope(geometry.corners.get(wheel), axle.wheel_droop_m, axle.wheel_bump_m, 0.0, wheel.is_front());
        }
        Ok(Self { configuration, inventory, settings, suspension_model, travel_limits_metres })
    }

    pub fn kinematic_input(&self, state: &CoupledVehicleState, input: &CoupledSuspensionInput) -> ComponentKinematicInput {
        ComponentKinematicInput { body_origin_world_metres: state.suspension.body_origin_world_metres,
            body_orientation_world: state.suspension.body_orientation_world.to_mat3(),
            generalized_velocity: state.suspension.generalized_velocity,
            wheel_travel_metres: state.suspension.wheel_travel_metres,
            steering_rack_metres: input.steering_rack_metres,
            steering_rack_velocity_metres_per_second: input.steering_rack_velocity_metres_per_second,
            steering_rack_acceleration_metres_per_second_squared: input.steering_rack_acceleration_metres_per_second_squared,
            differentiation_step_metres: self.settings.differentiation_step_metres }
    }

    pub fn contact_jacobian(&self, state: &CoupledVehicleState, input: &CoupledSuspensionInput,
        wheel: Option<WheelIndex>, point_world_metres: Vec3, direction_world: Vec3) -> Result<GeneralizedVehicleVelocity, String> {
        let mut jacobian = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        if let Some(wheel) = wheel {
            let hub = wheel_contact_component(&self.configuration, wheel, self.kinematic_input(state, input))?;
            let arm = point_world_metres - hub.center_of_mass_world_metres;
            for coordinate in 0..10 {
                jacobian[coordinate] = direction_world.dot(hub.linear_velocity_jacobian[coordinate]
                    + hub.angular_velocity_jacobian[coordinate].cross(arm));
            }
            let properties = self.inventory.components.iter().find(|component| component.rotating_wheel == Some(wheel)).ok_or("Missing rotating attachment")?;
            let pose = component_pose(&self.configuration, &properties.properties, state.suspension.wheel_travel_metres, input.steering_rack_metres)?;
            let spin_axis = state.suspension.body_orientation_world.to_mat3().transform_vector(pose.orientation_local.x);
            jacobian[10 + wheel as usize] = direction_world.dot(spin_axis.cross(arm));
        } else {
            let arm = point_world_metres - state.suspension.body_origin_world_metres;
            jacobian[..3].copy_from_slice(&[direction_world.x, direction_world.y, direction_world.z]);
            let moment = arm.cross(direction_world);
            jacobian[3..6].copy_from_slice(&[moment.x, moment.y, moment.z]);
        }
        Ok(jacobian)
    }

    pub fn evaluate(&self, state: &CoupledVehicleState, input: &CoupledSuspensionInput,
        relative_actuator_torque_newton_metres: [f64; 4]) -> Result<CoupledVehicleEvaluation, String> {
        let reference = self.suspension_model.evaluate(&state.suspension, input)?;
        let mut diagnostics = reference.diagnostics.clone();
        let velocity = state.velocity();
        if velocity.iter().any(|value| !value.is_finite()) || relative_actuator_torque_newton_metres.iter().any(|value| !value.is_finite()) {
            return Err("Nonfinite coupled vehicle velocity or actuator torque".into());
        }
        let mut mass_matrix = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        let mut force = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        let mut prescribed_generalized_momentum = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
        for component in &reference.components {
            let prescribed_angular_momentum = component.inertia_world.multiply(component.prescribed_angular_velocity_radians_per_second);
            for coordinate in 0..10 {
                prescribed_generalized_momentum[coordinate] += component.mass_kilograms
                    * component.linear_velocity_jacobian[coordinate].dot(component.prescribed_linear_velocity_metres_per_second)
                    + component.angular_velocity_jacobian[coordinate].dot(prescribed_angular_momentum);
            }
        }
        let mut rotating_inertial_changes = vec![Vec3::ZERO; reference.components.len()];
        let mut rotating_axes = vec![None; reference.components.len()];
        let mut rotating_kinetic_energy_joules = 0.0;
        let mut wheel_axes_world = [Vec3::ZERO; 4];
        let geometry = self.configuration.geometric_suspension.as_ref().ok_or("Coupled spin requires physical hardpoints")?;
        for wheel in WheelIndex::ALL {
            let solution = solve_corner(geometry.corners.get(wheel), state.suspension.wheel_travel_metres[wheel as usize],
                input.steering_rack_metres, wheel.is_front(),
                if wheel.is_front() { self.configuration.front_camber } else { self.configuration.rear_camber },
                if wheel.is_front() { self.configuration.front_toe } else { self.configuration.rear_toe },
                if wheel.is_left() { 1.0 } else { -1.0 }).ok_or("Wheel spin frame is unreachable")?;
            if !solution.converged || solution.rocker_clamped || solution.steering_clamped { return Err("Wheel spin frame is outside the supported domain".into()); }
            wheel_axes_world[wheel as usize] = state.suspension.body_orientation_world.to_mat3().transform_vector(solution.wheel_basis.x);
        }
        for row in 0..10 {
            mass_matrix[row][..10].copy_from_slice(&reference.equations.mass_matrix[row]);
            force[row] = (0..10).map(|column| reference.equations.mass_matrix[row][column]
                * reference.diagnostics.generalized_acceleration[column]).sum();
        }
        for (index, component) in self.inventory.components.iter().enumerate() {
            if let Some(wheel) = component.rotating_wheel {
                let moving = &reference.components[index];
                let axis = wheel_axes_world[wheel as usize];
                let spin_coordinate = 10 + wheel as usize;
                let axis_momentum = moving.inertia_world.multiply(axis);
                prescribed_generalized_momentum[spin_coordinate] += axis_momentum.dot(moving.prescribed_angular_velocity_radians_per_second);
                diagnostics.angular_momentum_about_world_origin_kilogram_square_metres_per_second += axis_momentum * velocity[spin_coordinate];
                mass_matrix[spin_coordinate][spin_coordinate] += axis.dot(axis_momentum);
                for coordinate in 0..10 {
                    let coupling = moving.angular_velocity_jacobian[coordinate].dot(axis_momentum);
                    mass_matrix[coordinate][spin_coordinate] += coupling;
                    mass_matrix[spin_coordinate][coordinate] += coupling;
                }
                let parent_velocity = moving.angular_velocity(state.suspension.generalized_velocity);
                rotating_kinetic_energy_joules += velocity[spin_coordinate] * axis_momentum.dot(parent_velocity)
                    + 0.5 * axis.dot(axis_momentum) * velocity[spin_coordinate].powi(2);
                let full_velocity = parent_velocity + axis * velocity[spin_coordinate];
                let full_bias = moving.angular_acceleration_bias_radians_per_second_squared
                    + parent_velocity.cross(axis) * velocity[spin_coordinate];
                let previous_inertial_torque = moving.inertia_world.multiply(moving.angular_acceleration_bias_radians_per_second_squared)
                    + parent_velocity.cross(moving.inertia_world.multiply(parent_velocity));
                let full_inertial_torque = moving.inertia_world.multiply(full_bias)
                    + full_velocity.cross(moving.inertia_world.multiply(full_velocity));
                rotating_inertial_changes[index] = full_inertial_torque - previous_inertial_torque;
                rotating_axes[index] = Some((axis, spin_coordinate));
                for coordinate in 0..10 { force[coordinate] -= moving.angular_velocity_jacobian[coordinate].dot(full_inertial_torque - previous_inertial_torque); }
                force[spin_coordinate] -= axis.dot(full_inertial_torque);
            }
        }
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            force[10 + index] += relative_actuator_torque_newton_metres[index];
            if let Some(contact) = input.contacts[index] {
                if reference.diagnostics.normal_force_newtons[index] > 0.0 {
                    let radius = if wheel.is_front() { self.configuration.front_tire_radius } else { self.configuration.rear_tire_radius };
                    let tangent = contact.tangential_force_world_newtons
                        - contact.surface_normal_world * contact.tangential_force_world_newtons.dot(contact.surface_normal_world);
                    let spin_torque = (-contact.surface_normal_world * radius).cross(tangent).dot(wheel_axes_world[index]);
                    force[10 + index] += spin_torque;
                    diagnostics.external_power_watts += spin_torque * velocity[10 + index];
                }
            }
        }
        let acceleration = solve_factored_vehicle_matrix(&factor_generalized_vehicle_matrix(&mass_matrix)?, force)?;
        for (index, component) in reference.components.iter().enumerate() {
            let mut linear_acceleration_change = Vec3::ZERO;
            let mut angular_acceleration_change = Vec3::ZERO;
            for coordinate in 0..10 {
                let change = acceleration[coordinate] - reference.diagnostics.generalized_acceleration[coordinate];
                linear_acceleration_change += component.linear_velocity_jacobian[coordinate] * change;
                angular_acceleration_change += component.angular_velocity_jacobian[coordinate] * change;
            }
            if let Some((axis, spin_coordinate)) = rotating_axes[index] {
                angular_acceleration_change += axis * acceleration[spin_coordinate];
            }
            diagnostics.steering_actuator_power_watts += (linear_acceleration_change * component.mass_kilograms)
                .dot(component.prescribed_linear_velocity_metres_per_second)
                + (component.inertia_world.multiply(angular_acceleration_change) + rotating_inertial_changes[index])
                    .dot(component.prescribed_angular_velocity_radians_per_second);
        }
        diagnostics.generalized_acceleration.copy_from_slice(&acceleration[..10]);
        let kinetic_energy_joules = reference.diagnostics.kinetic_energy_joules + rotating_kinetic_energy_joules;
        diagnostics.kinetic_energy_joules = kinetic_energy_joules;
        diagnostics.total_energy_joules = kinetic_energy_joules + diagnostics.gravitational_potential_energy_joules
            + diagnostics.elastic_potential_energy_joules;
        Ok(CoupledVehicleEvaluation { mass_matrix, generalized_force: force, suspension_diagnostics: diagnostics, kinetic_energy_joules,
            prescribed_generalized_momentum })
    }

    pub fn prescribed_contact_velocity(&self, state: &CoupledVehicleState, input: &CoupledSuspensionInput,
        wheel: Option<WheelIndex>, point_world_metres: Vec3) -> Result<Vec3, String> {
        if let Some(wheel) = wheel {
            let hub = wheel_contact_component(&self.configuration, wheel, self.kinematic_input(state, input))?;
            Ok(hub.prescribed_linear_velocity_metres_per_second + hub.prescribed_angular_velocity_radians_per_second
                .cross(point_world_metres - hub.center_of_mass_world_metres))
        } else { Ok(Vec3::ZERO) }
    }

    pub fn update_fuel_mass(&mut self, state: &mut CoupledVehicleState, input: &CoupledSuspensionInput, kilograms: f64) -> Result<(), String> {
        let exchange = self.suspension_model.update_fuel_mass(&state.suspension, input, kilograms)?;
        state.fuel_exchange_energy_joules += exchange.system_energy_change_joules;
        self.configuration.fuel.set_current_kg(kilograms);
        Ok(())
    }

    pub fn advance(&self, state: &mut CoupledVehicleState, input: &CoupledSuspensionInput,
        actuator_torque: [f64; 4], duration_seconds: f64, maximum_events: usize) -> Result<(), String> {
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 || duration_seconds > 1.0 || maximum_events == 0 {
            return Err("Invalid coupled vehicle integration interval or event budget".into());
        }
        let mut candidate = state.clone();
        let mut remaining = duration_seconds;
        let mut events = 0;
        let mut elapsed = 0.0;
        while remaining > duration_seconds * 1e-12 {
            let mut step = remaining.min(self.settings.maximum_substep_seconds);
            let mut current_input = input.clone();
            current_input.steering_rack_metres += input.steering_rack_velocity_metres_per_second * elapsed
                + 0.5 * input.steering_rack_acceleration_metres_per_second_squared * elapsed * elapsed;
            current_input.steering_rack_velocity_metres_per_second += input.steering_rack_acceleration_metres_per_second_squared * elapsed;
            for contact in current_input.contacts.iter_mut().flatten() {
                contact.surface_point_world_metres += contact.surface_velocity_world_metres_per_second * elapsed;
            }
            let evaluation = self.evaluate(&candidate, &current_input, actuator_torque)?;
            let factor = factor_generalized_vehicle_matrix(&evaluation.mass_matrix)?;
            let acceleration = solve_factored_vehicle_matrix(&factor, evaluation.generalized_force)?;
            let previous_velocity = candidate.velocity();
            let mut at_boundary = [false; 4];
            for wheel in 0..4 {
                let travel = candidate.suspension.wheel_travel_metres[wheel];
                let velocity = previous_velocity[6 + wheel];
                let predicted_velocity = velocity + acceleration[6 + wheel] * step;
                let (lower, upper) = self.travel_limits_metres[wheel];
                at_boundary[wheel] = (travel <= lower + 1e-10 && predicted_velocity < 0.0)
                    || (travel >= upper - 1e-10 && predicted_velocity > 0.0);
                if at_boundary[wheel] { continue; }
                let predicted = travel + predicted_velocity * step;
                let target = if predicted < lower { Some(lower) } else if predicted > upper { Some(upper) } else { None };
                if let Some(target) = target {
                    let distance = target - travel;
                    let coefficient = acceleration[6 + wheel];
                    let mut crossing = step;
                    if coefficient.abs() < 1e-12 {
                        if velocity.abs() > 1e-12 { crossing = distance / velocity; }
                    } else {
                        let discriminant = velocity * velocity + 4.0 * coefficient * distance;
                        if discriminant < 0.0 { return Err("Travel event root has a negative discriminant".into()); }
                        for root in [(-velocity - discriminant.sqrt()) / (2.0 * coefficient), (-velocity + discriminant.sqrt()) / (2.0 * coefficient)] {
                            if root > 1e-12 && root < crossing { crossing = root; }
                        }
                    }
                    step = step.min(crossing.max(1e-12));
                }
            }
            let mut velocity = previous_velocity;
            for coordinate in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT { velocity[coordinate] += acceleration[coordinate] * step; }
            if at_boundary.iter().any(|active| *active) {
                let unconstrained_velocity = velocity;
                candidate.suspension.dissipated_energy_joules += project_vehicle_travel_constraints(&evaluation.mass_matrix, &mut velocity, at_boundary)?;
                let velocity_change = std::array::from_fn(|coordinate| velocity[coordinate] - unconstrained_velocity[coordinate]);
                candidate.suspension.steering_actuator_work_joules += generalized_projection(evaluation.prescribed_generalized_momentum, velocity_change);
            }
            candidate.set_velocity(velocity);
            candidate.suspension.body_origin_world_metres += Vec3::new(velocity[0], velocity[1], velocity[2]) * step;
            let angular_world = Vec3::new(velocity[3], velocity[4], velocity[5]);
            let angular_local = candidate.suspension.body_orientation_world.to_mat3().inverse_transform_vector(angular_world);
            candidate.suspension.body_orientation_world = candidate.suspension.body_orientation_world.integrate_angular_velocity(angular_local, step).normalized();
            let mut impacted = [false; 4];
            for wheel in 0..4 {
                let (lower, upper) = self.travel_limits_metres[wheel];
                let previous = candidate.suspension.wheel_travel_metres[wheel];
                let advanced = previous + velocity[6 + wheel] * step;
                if advanced < lower - 1e-8 || advanced > upper + 1e-8 { return Err("Travel localization exceeded the supported geometry domain".into()); }
                impacted[wheel] = (advanced <= lower + 1e-10 && velocity[6 + wheel] < 0.0)
                    || (advanced >= upper - 1e-10 && velocity[6 + wheel] > 0.0);
                candidate.suspension.wheel_travel_metres[wheel] = advanced.clamp(lower, upper);
                candidate.wheel_spin_angle_radians[wheel] = (candidate.wheel_spin_angle_radians[wheel] + velocity[10 + wheel] * step).rem_euclid(std::f64::consts::TAU);
                candidate.actuator_work_joules += actuator_torque[wheel] * 0.5 * (previous_velocity[10 + wheel] + velocity[10 + wheel]) * step;
            }
            if impacted.iter().any(|active| *active) {
                events += 1;
                if events > maximum_events { return Err("Travel event budget exceeded; interval was not committed".into()); }
                let mut event_input = current_input.clone();
                event_input.steering_rack_metres += current_input.steering_rack_velocity_metres_per_second * step
                    + 0.5 * current_input.steering_rack_acceleration_metres_per_second_squared * step * step;
                event_input.steering_rack_velocity_metres_per_second += current_input.steering_rack_acceleration_metres_per_second_squared * step;
                for contact in event_input.contacts.iter_mut().flatten() {
                    contact.surface_point_world_metres += contact.surface_velocity_world_metres_per_second * step;
                }
                let event_evaluation = self.evaluate(&candidate, &event_input, actuator_torque)?;
                let unconstrained_velocity = velocity;
                candidate.suspension.dissipated_energy_joules += project_vehicle_travel_constraints(&event_evaluation.mass_matrix, &mut velocity, impacted)?;
                let velocity_change = std::array::from_fn(|coordinate| velocity[coordinate] - unconstrained_velocity[coordinate]);
                candidate.suspension.steering_actuator_work_joules += generalized_projection(event_evaluation.prescribed_generalized_momentum, velocity_change);
                candidate.set_velocity(velocity);
            }
            candidate.suspension.dissipated_energy_joules += evaluation.suspension_diagnostics.dissipated_power_watts * step;
            candidate.suspension.external_work_joules += evaluation.suspension_diagnostics.external_power_watts * step;
            candidate.suspension.steering_actuator_work_joules += evaluation.suspension_diagnostics.steering_actuator_power_watts * step;
            candidate.suspension.simulated_time_seconds += step;
            elapsed += step;
            remaining = (duration_seconds - elapsed).max(0.0);
        }
        let mut final_input = input.clone();
        final_input.steering_rack_metres += input.steering_rack_velocity_metres_per_second * elapsed
            + 0.5 * input.steering_rack_acceleration_metres_per_second_squared * elapsed * elapsed;
        final_input.steering_rack_velocity_metres_per_second += input.steering_rack_acceleration_metres_per_second_squared * elapsed;
        for contact in final_input.contacts.iter_mut().flatten() {
            contact.surface_point_world_metres += contact.surface_velocity_world_metres_per_second * elapsed;
        }
        self.evaluate(&candidate, &final_input, actuator_torque)?;
        *state = candidate;
        Ok(())
    }
}

pub fn project_vehicle_travel_constraints(matrix: &GeneralizedVehicleMatrix, velocity: &mut GeneralizedVehicleVelocity,
    active: [bool; 4]) -> Result<f64, String> {
    let factor = factor_generalized_vehicle_matrix(matrix)?;
    let mut constraint_matrix = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    let mut force = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    let mut responses = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; 4];
    for coordinate in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT { constraint_matrix[coordinate][coordinate] = 1.0; }
    for wheel in 0..4 {
        if active[wheel] {
            let mut jacobian = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
            jacobian[6 + wheel] = 1.0;
            responses[wheel] = solve_factored_vehicle_matrix(&factor, jacobian)?;
            force[6 + wheel] = -velocity[6 + wheel];
        }
    }
    for first in 0..4 { for second in 0..4 {
        if active[first] && active[second] { constraint_matrix[6 + first][6 + second] = responses[second][6 + first]; }
    } }
    let impulses = solve_factored_vehicle_matrix(&factor_generalized_vehicle_matrix(&constraint_matrix)?, force)?;
    let previous_energy = generalized_vehicle_energy(matrix, *velocity);
    for wheel in 0..4 { for coordinate in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT { velocity[coordinate] += responses[wheel][coordinate] * impulses[6 + wheel]; } }
    let energy_loss = previous_energy - generalized_vehicle_energy(matrix, *velocity);
    if energy_loss < -1e-8 * previous_energy.abs().max(1.0) { return Err("Travel projection added unexplained kinetic energy".into()); }
    Ok(energy_loss.max(0.0))
}
