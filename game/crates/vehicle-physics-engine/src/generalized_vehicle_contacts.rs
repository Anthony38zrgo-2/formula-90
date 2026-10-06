use crate::coupled_vehicle_dynamics::{factor_generalized_vehicle_matrix, generalized_projection, generalized_vehicle_energy,
    solve_factored_vehicle_matrix, GeneralizedVehicleMatrix, GeneralizedVehicleVelocity, VEHICLE_GENERALIZED_VELOCITY_COUNT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneralizedContactSettings {
    pub maximum_iterations: usize,
    pub velocity_tolerance_metres_per_second: f64,
    pub restitution: f64,
    pub restitution_minimum_speed_metres_per_second: f64,
    pub impact_friction_coefficient: f64,
    pub prediction_distance_metres: f64,
    pub maximum_position_correction_metres: f64,
    pub maximum_events_per_step: usize,
    pub maximum_supported_speed_metres_per_second: f64,
    pub maximum_supported_angular_speed_radians_per_second: f64,
}

impl GeneralizedContactSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.maximum_iterations == 0 || self.maximum_iterations > 1000 || self.maximum_events_per_step == 0
            || !self.restitution.is_finite() || !(0.0..=1.0).contains(&self.restitution)
            || [self.velocity_tolerance_metres_per_second, self.restitution_minimum_speed_metres_per_second,
                self.impact_friction_coefficient, self.prediction_distance_metres, self.maximum_position_correction_metres,
                self.maximum_supported_speed_metres_per_second, self.maximum_supported_angular_speed_radians_per_second]
                .iter().any(|value| !value.is_finite() || *value <= 0.0)
        { return Err("Invalid generalized contact settings".into()); }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GeneralizedContactConstraint {
    pub identifier: String,
    pub first_entity: usize,
    pub second_entity: Option<usize>,
    pub first_jacobians: [GeneralizedVehicleVelocity; 3],
    pub second_jacobians: [GeneralizedVehicleVelocity; 3],
    pub first_prescribed_velocity_metres_per_second: [f64; 3],
    pub second_prescribed_velocity_metres_per_second: [f64; 3],
    pub normal_impulse_newton_seconds: f64,
    pub tangential_impulse_newton_seconds: [f64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneralizedContactResult {
    pub iterations: usize,
    pub maximum_normal_velocity_residual_metres_per_second: f64,
    pub maximum_tangential_velocity_residual_metres_per_second: f64,
    pub kinetic_energy_change_joules: f64,
    pub prescribed_actuator_work_joules_by_entity: Vec<f64>,
    pub normal_impulses_newton_seconds: Vec<f64>,
    pub tangential_impulses_newton_seconds: Vec<[f64; 2]>,
}

fn relative_contact_velocity(constraint: &GeneralizedContactConstraint, axis: usize, velocities: &[GeneralizedVehicleVelocity]) -> f64 {
    generalized_projection(constraint.first_jacobians[axis], velocities[constraint.first_entity])
        + constraint.second_entity.map_or(0.0, |entity| generalized_projection(constraint.second_jacobians[axis], velocities[entity]))
        + constraint.first_prescribed_velocity_metres_per_second[axis] + constraint.second_prescribed_velocity_metres_per_second[axis]
}

fn apply_contact_impulse(constraint: &GeneralizedContactConstraint, axis: usize, impulse: f64,
    responses: &[([GeneralizedVehicleVelocity; 3], [GeneralizedVehicleVelocity; 3])], index: usize,
    velocities: &mut [GeneralizedVehicleVelocity]) {
    for coordinate in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
        velocities[constraint.first_entity][coordinate] += responses[index].0[axis][coordinate] * impulse;
        if let Some(entity) = constraint.second_entity { velocities[entity][coordinate] += responses[index].1[axis][coordinate] * impulse; }
    }
}

pub fn solve_generalized_vehicle_contacts(matrices: &[GeneralizedVehicleMatrix], velocities: &mut [GeneralizedVehicleVelocity],
    constraints: &mut [GeneralizedContactConstraint], settings: &GeneralizedContactSettings) -> Result<GeneralizedContactResult, String> {
    let mut candidate_velocities = velocities.to_vec();
    let mut candidate_constraints = constraints.to_vec();
    let result = solve_generalized_vehicle_contact_candidate(matrices, &mut candidate_velocities, &mut candidate_constraints, settings)?;
    velocities.copy_from_slice(&candidate_velocities);
    constraints.clone_from_slice(&candidate_constraints);
    Ok(result)
}

fn solve_generalized_vehicle_contact_candidate(matrices: &[GeneralizedVehicleMatrix], velocities: &mut [GeneralizedVehicleVelocity],
    constraints: &mut [GeneralizedContactConstraint], settings: &GeneralizedContactSettings) -> Result<GeneralizedContactResult, String> {
    settings.validate()?;
    if matrices.len() != velocities.len() || velocities.iter().flatten().any(|value| !value.is_finite()) {
        return Err("Invalid generalized contact state dimensions or velocities".into());
    }
    let factors = matrices.iter().map(factor_generalized_vehicle_matrix).collect::<Result<Vec<_>, _>>()?;
    let initial_energy = matrices.iter().zip(velocities.iter()).map(|(matrix, velocity)| generalized_vehicle_energy(matrix, *velocity)).sum::<f64>();
    let mut responses = Vec::new();
    let mut inverse_effective_masses = Vec::new();
    let mut restitution_targets = Vec::new();
    let mut tangential_step_denominators = Vec::new();
    let mut previous_identity: Option<&str> = None;
    for constraint in constraints.iter() {
        if constraint.first_entity >= velocities.len() || constraint.second_entity.is_some_and(|entity| entity >= velocities.len() || entity == constraint.first_entity)
            || (constraint.second_entity.is_none() && constraint.second_prescribed_velocity_metres_per_second != [0.0; 3])
            || previous_identity.is_some_and(|previous| previous >= constraint.identifier.as_str())
            || constraint.first_jacobians.iter().flatten().chain(constraint.second_jacobians.iter().flatten()).any(|value| !value.is_finite())
            || constraint.first_prescribed_velocity_metres_per_second.iter().chain(constraint.second_prescribed_velocity_metres_per_second.iter()).any(|value| !value.is_finite())
            || constraint.normal_impulse_newton_seconds != 0.0 || constraint.tangential_impulse_newton_seconds != [0.0; 2]
        { return Err("Contact constraints require valid entities, finite Jacobians, unique sorted identities and cold impulses".into()); }
        previous_identity = Some(&constraint.identifier);
        let mut first_response = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; 3];
        let mut second_response = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; 3];
        let mut inverse_mass = [0.0; 3];
        for axis in 0..3 {
            first_response[axis] = solve_factored_vehicle_matrix(&factors[constraint.first_entity], constraint.first_jacobians[axis])?;
            inverse_mass[axis] = generalized_projection(constraint.first_jacobians[axis], first_response[axis]);
            if let Some(entity) = constraint.second_entity {
                second_response[axis] = solve_factored_vehicle_matrix(&factors[entity], constraint.second_jacobians[axis])?;
                inverse_mass[axis] += generalized_projection(constraint.second_jacobians[axis], second_response[axis]);
            }
            if inverse_mass[axis] <= 0.0 || !inverse_mass[axis].is_finite() { return Err("Contact has singular generalized effective mass".into()); }
        }
        let initial_normal_velocity = relative_contact_velocity(constraint, 0, velocities);
        restitution_targets.push(if initial_normal_velocity < -settings.restitution_minimum_speed_metres_per_second { -settings.restitution * initial_normal_velocity } else { 0.0 });
        let tangential_cross_response = generalized_projection(constraint.first_jacobians[1], first_response[2])
            + constraint.second_entity.map_or(0.0, |_| generalized_projection(constraint.second_jacobians[1], second_response[2]));
        let tangential_step_denominator = 0.5 * (inverse_mass[1] + inverse_mass[2]
            + ((inverse_mass[1] - inverse_mass[2]).powi(2) + 4.0 * tangential_cross_response.powi(2)).sqrt());
        tangential_step_denominators.push(tangential_step_denominator);
        inverse_effective_masses.push(inverse_mass);
        responses.push((first_response, second_response));
    }
    let mut used_iterations = 0;
    let mut maximum_residual = 0.0;
    let mut maximum_tangential_residual = 0.0;
    for iteration in 0..settings.maximum_iterations {
        for (index, constraint) in constraints.iter_mut().enumerate() {
            let normal_velocity = relative_contact_velocity(constraint, 0, velocities);
            let previous = constraint.normal_impulse_newton_seconds;
            let next = (previous + (restitution_targets[index] - normal_velocity) / inverse_effective_masses[index][0]).max(0.0);
            constraint.normal_impulse_newton_seconds = next;
            apply_contact_impulse(constraint, 0, next - previous, &responses, index, velocities);
            let previous = constraint.tangential_impulse_newton_seconds;
            let mut next = previous;
            for tangent in 0..2 {
                next[tangent] -= relative_contact_velocity(constraint, tangent + 1, velocities) / tangential_step_denominators[index];
            }
            let magnitude = (next[0].powi(2) + next[1].powi(2)).sqrt();
            let bound = settings.impact_friction_coefficient * constraint.normal_impulse_newton_seconds;
            if magnitude > bound && magnitude > 0.0 { next.iter_mut().for_each(|value| *value *= bound / magnitude); }
            constraint.tangential_impulse_newton_seconds = next;
            for tangent in 0..2 { apply_contact_impulse(constraint, tangent + 1, next[tangent] - previous[tangent], &responses, index, velocities); }
        }
        used_iterations = iteration + 1;
        maximum_residual = constraints.iter().enumerate().map(|(index, constraint)| {
            let residual = relative_contact_velocity(constraint, 0, velocities) - restitution_targets[index];
            if constraint.normal_impulse_newton_seconds > 0.0 { residual.abs() } else { (-residual).max(0.0) }
        }).fold(0.0, f64::max);
        maximum_tangential_residual = constraints.iter().enumerate().map(|(index, constraint)| {
            let previous = constraint.tangential_impulse_newton_seconds;
            let mut projected = std::array::from_fn::<_, 2, _>(|tangent| previous[tangent]
                - relative_contact_velocity(constraint, tangent + 1, velocities) / tangential_step_denominators[index]);
            let magnitude = projected[0].hypot(projected[1]);
            let bound = settings.impact_friction_coefficient * constraint.normal_impulse_newton_seconds;
            if magnitude > bound && magnitude > 0.0 { projected.iter_mut().for_each(|value| *value *= bound / magnitude); }
            (projected[0] - previous[0]).hypot(projected[1] - previous[1]) * tangential_step_denominators[index]
        }).fold(0.0, f64::max);
        if maximum_residual.max(maximum_tangential_residual) <= settings.velocity_tolerance_metres_per_second { break; }
    }
    if maximum_residual.max(maximum_tangential_residual) > settings.velocity_tolerance_metres_per_second {
        return Err(format!("Generalized contact iteration budget exceeded; normal residual {maximum_residual}, tangential residual {maximum_tangential_residual} metres per second"));
    }
    let final_energy = matrices.iter().zip(velocities.iter()).map(|(matrix, velocity)| generalized_vehicle_energy(matrix, *velocity)).sum::<f64>();
    let mut prescribed_actuator_work = vec![0.0; velocities.len()];
    for constraint in constraints.iter() {
        let impulses = [constraint.normal_impulse_newton_seconds, constraint.tangential_impulse_newton_seconds[0], constraint.tangential_impulse_newton_seconds[1]];
        for axis in 0..3 {
            prescribed_actuator_work[constraint.first_entity] -= impulses[axis] * constraint.first_prescribed_velocity_metres_per_second[axis];
            if let Some(entity) = constraint.second_entity {
                prescribed_actuator_work[entity] -= impulses[axis] * constraint.second_prescribed_velocity_metres_per_second[axis];
            }
        }
    }
    let prescribed_work = prescribed_actuator_work.iter().sum::<f64>();
    if final_energy > initial_energy + prescribed_work + 1e-8 * (initial_energy.abs() + prescribed_work.abs()).max(1.0) {
        return Err("Passive generalized contact solve added unexplained kinetic energy".into());
    }
    Ok(GeneralizedContactResult { iterations: used_iterations, maximum_normal_velocity_residual_metres_per_second: maximum_residual,
        maximum_tangential_velocity_residual_metres_per_second: maximum_tangential_residual,
        kinetic_energy_change_joules: final_energy - initial_energy,
        prescribed_actuator_work_joules_by_entity: prescribed_actuator_work,
        normal_impulses_newton_seconds: constraints.iter().map(|constraint| constraint.normal_impulse_newton_seconds).collect(),
        tangential_impulses_newton_seconds: constraints.iter().map(|constraint| constraint.tangential_impulse_newton_seconds).collect() })
}
