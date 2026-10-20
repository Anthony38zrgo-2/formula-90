use crate::coupled_vehicle_dynamics::{factor_generalized_vehicle_matrix, generalized_projection, generalized_vehicle_energy,
    solve_factored_vehicle_matrix, GeneralizedVehicleMatrix, GeneralizedVehicleVelocity, VEHICLE_GENERALIZED_VELOCITY_COUNT};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const STATIC_FRICTION_SPEED_THRESHOLD_METRES_PER_SECOND: f64 = 1e-3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneralizedContactSettings {
    pub maximum_iterations: usize,
    pub velocity_tolerance_metres_per_second: f64,
    pub restitution: f64,
    pub restitution_minimum_speed_metres_per_second: f64,
    pub impact_friction_coefficient: f64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub impact_friction_by_surface_code: BTreeMap<u32, f64>,
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
            || self.impact_friction_by_surface_code.iter().any(|(surface_code, coefficient)|
                *surface_code > 7 || !coefficient.is_finite() || *coefficient < 0.0)
        { return Err("Invalid generalized contact settings".into()); }
        Ok(())
    }

    pub fn impact_friction_for_surface(&self, surface_code: u32) -> f64 {
        self.impact_friction_by_surface_code.get(&surface_code).copied()
            .unwrap_or(self.impact_friction_coefficient)
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
    pub surface_code: u32,
    pub normal_impulse_newton_seconds: f64,
    pub tangential_impulse_newton_seconds: [f64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneralizedContactResult {
    pub iterations: usize,
    pub warm_started_constraint_count: usize,
    pub maximum_normal_velocity_residual_metres_per_second: f64,
    pub maximum_tangential_velocity_residual_metres_per_second: f64,
    pub kinetic_energy_change_joules: f64,
    pub prescribed_actuator_work_joules_by_entity: Vec<f64>,
    pub normal_impulses_newton_seconds: Vec<f64>,
    pub tangential_impulses_newton_seconds: Vec<[f64; 2]>,
    pub static_friction_constraint_count: usize,
    pub sliding_constraint_count: usize,
}

fn relative_contact_velocity(constraint: &GeneralizedContactConstraint, axis: usize, velocities: &[GeneralizedVehicleVelocity]) -> f64 {
    generalized_projection(constraint.first_jacobians[axis], velocities[constraint.first_entity])
        + constraint.second_entity.map_or(0.0, |entity| generalized_projection(constraint.second_jacobians[axis], velocities[entity]))
        + constraint.first_prescribed_velocity_metres_per_second[axis] + constraint.second_prescribed_velocity_metres_per_second[axis]
}

pub fn generalized_contact_relative_velocity(constraint: &GeneralizedContactConstraint, axis: usize,
    velocities: &[GeneralizedVehicleVelocity]) -> f64 {
    relative_contact_velocity(constraint, axis, velocities)
}

fn apply_contact_impulse(constraint: &GeneralizedContactConstraint, axis: usize, impulse: f64,
    responses: &[([GeneralizedVehicleVelocity; 3], [GeneralizedVehicleVelocity; 3])], index: usize,
    velocities: &mut [GeneralizedVehicleVelocity], fixed_entities: &[bool]) {
    for coordinate in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
        if !fixed_entities[constraint.first_entity] {
            velocities[constraint.first_entity][coordinate] += responses[index].0[axis][coordinate] * impulse;
        }
        if let Some(entity) = constraint.second_entity {
            if !fixed_entities[entity] {
                velocities[entity][coordinate] += responses[index].1[axis][coordinate] * impulse;
            }
        }
    }
}

pub fn solve_generalized_vehicle_contacts(matrices: &[GeneralizedVehicleMatrix], velocities: &mut [GeneralizedVehicleVelocity],
    constraints: &mut [GeneralizedContactConstraint], settings: &GeneralizedContactSettings) -> Result<GeneralizedContactResult, String> {
    let fixed_entities = vec![false; velocities.len()];
    solve_generalized_vehicle_contacts_with_fixed_entities(matrices, velocities, constraints, settings, &fixed_entities)
}

pub fn solve_generalized_vehicle_contacts_with_fixed_entities(matrices: &[GeneralizedVehicleMatrix],
    velocities: &mut [GeneralizedVehicleVelocity], constraints: &mut [GeneralizedContactConstraint],
    settings: &GeneralizedContactSettings, fixed_entities: &[bool]) -> Result<GeneralizedContactResult, String> {
    let mut candidate_velocities = velocities.to_vec();
    let mut candidate_constraints = constraints.to_vec();
    let result = solve_generalized_vehicle_contact_candidate(matrices, &mut candidate_velocities,
        &mut candidate_constraints, settings, fixed_entities)?;
    velocities.copy_from_slice(&candidate_velocities);
    constraints.clone_from_slice(&candidate_constraints);
    Ok(result)
}

fn solve_generalized_vehicle_contact_candidate(matrices: &[GeneralizedVehicleMatrix], velocities: &mut [GeneralizedVehicleVelocity],
    constraints: &mut [GeneralizedContactConstraint], settings: &GeneralizedContactSettings,
    fixed_entities: &[bool]) -> Result<GeneralizedContactResult, String> {
    settings.validate()?;
    if matrices.len() != velocities.len() || velocities.iter().flatten().any(|value| !value.is_finite())
        || fixed_entities.len() != velocities.len() {
        return Err("Invalid generalized contact state dimensions or velocities".into());
    }
    let factors = matrices.iter().map(factor_generalized_vehicle_matrix).collect::<Result<Vec<_>, _>>()?;
    let initial_energy = matrices.iter().zip(velocities.iter()).map(|(matrix, velocity)| generalized_vehicle_energy(matrix, *velocity)).sum::<f64>();
    let mut responses = Vec::new();
    let mut restitution_targets = Vec::new();
    let mut tangential_effective_masses = Vec::new();
    let mut previous_identity: Option<&str> = None;
    for constraint in constraints.iter() {
        if constraint.first_entity >= velocities.len() || constraint.second_entity.is_some_and(|entity| entity >= velocities.len() || entity == constraint.first_entity)
            || (constraint.second_entity.is_none() && constraint.second_prescribed_velocity_metres_per_second != [0.0; 3])
            || previous_identity.is_some_and(|previous| previous >= constraint.identifier.as_str())
            || constraint.first_jacobians.iter().flatten().chain(constraint.second_jacobians.iter().flatten()).any(|value| !value.is_finite())
            || constraint.first_prescribed_velocity_metres_per_second.iter().chain(constraint.second_prescribed_velocity_metres_per_second.iter()).any(|value| !value.is_finite())
            || !constraint.normal_impulse_newton_seconds.is_finite() || constraint.normal_impulse_newton_seconds < 0.0
            || constraint.tangential_impulse_newton_seconds.iter().any(|value| !value.is_finite())
        { return Err("Contact constraints require valid entities, finite Jacobians, unique sorted identities and finite non-negative impulses".into()); }
        previous_identity = Some(&constraint.identifier);
        let mut first_response = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; 3];
        let mut second_response = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; 3];
        let mut inverse_mass = [0.0; 3];
        for axis in 0..3 {
            if !fixed_entities[constraint.first_entity] {
                first_response[axis] = solve_factored_vehicle_matrix(&factors[constraint.first_entity], constraint.first_jacobians[axis])?;
                inverse_mass[axis] = generalized_projection(constraint.first_jacobians[axis], first_response[axis]);
            }
            if let Some(entity) = constraint.second_entity {
                if !fixed_entities[entity] {
                    second_response[axis] = solve_factored_vehicle_matrix(&factors[entity], constraint.second_jacobians[axis])?;
                    inverse_mass[axis] += generalized_projection(constraint.second_jacobians[axis], second_response[axis]);
                }
            }
            if inverse_mass[axis] <= 0.0 || !inverse_mass[axis].is_finite() { return Err("Contact has singular generalized effective mass".into()); }
        }
        let initial_normal_velocity = relative_contact_velocity(constraint, 0, velocities);
        restitution_targets.push(if initial_normal_velocity < -settings.restitution_minimum_speed_metres_per_second { -settings.restitution * initial_normal_velocity } else { 0.0 });
        let tangential_cross_response = generalized_projection(constraint.first_jacobians[1], first_response[2])
            + constraint.second_entity.map_or(0.0, |_| generalized_projection(constraint.second_jacobians[1], second_response[2]));
        let tangential_matrix = [[inverse_mass[1], tangential_cross_response],
            [tangential_cross_response, inverse_mass[2]]];
        let determinant = tangential_matrix[0][0] * tangential_matrix[1][1]
            - tangential_matrix[0][1] * tangential_matrix[1][0];
        if determinant <= 0.0 || !determinant.is_finite() {
            return Err("Contact has a singular tangential effective mass".into());
        }
        tangential_effective_masses.push(tangential_matrix);
        responses.push((first_response, second_response));
    }
    let warm_started_constraint_count = constraints.iter().filter(|constraint| constraint.normal_impulse_newton_seconds > 0.0
        || constraint.tangential_impulse_newton_seconds != [0.0; 2]).count();
    for (index, constraint) in constraints.iter().enumerate() {
        let normal_impulse = constraint.normal_impulse_newton_seconds;
        if normal_impulse != 0.0 {
            apply_contact_impulse(constraint, 0, normal_impulse, &responses, index, velocities, fixed_entities);
        }
        for tangent in 0..2 {
            let impulse = constraint.tangential_impulse_newton_seconds[tangent];
            if impulse != 0.0 {
                apply_contact_impulse(constraint, tangent + 1, impulse, &responses, index, velocities, fixed_entities);
            }
        }
    }
    let mut normal_matrix = vec![vec![0.0; constraints.len()]; constraints.len()];
    for (row, constraint) in constraints.iter().enumerate() {
        for (column, other) in constraints.iter().enumerate() {
            let mut value = 0.0;
            if constraint.first_entity == other.first_entity {
                value += generalized_projection(constraint.first_jacobians[0], responses[column].0[0]);
            } else if Some(constraint.first_entity) == other.second_entity {
                value += generalized_projection(constraint.first_jacobians[0], responses[column].1[0]);
            }
            if let Some(second_entity) = constraint.second_entity {
                if second_entity == other.first_entity {
                    value += generalized_projection(constraint.second_jacobians[0], responses[column].0[0]);
                } else if Some(second_entity) == other.second_entity {
                    value += generalized_projection(constraint.second_jacobians[0], responses[column].1[0]);
                }
            }
            normal_matrix[row][column] = value;
        }
    }
    let initial_normal_impulses: Vec<f64> = constraints.iter()
        .map(|constraint| constraint.normal_impulse_newton_seconds).collect();
    let friction_coefficients: Vec<f64> = constraints.iter()
        .map(|constraint| settings.impact_friction_for_surface(constraint.surface_code)).collect();
    let zero_impulses = vec![0.0; constraints.len()];
    let mut used_iterations = 0_usize;
    let mut maximum_residual: f64;
    let mut maximum_tangential_residual: f64 = 0.0;
    let mut outer_iterations = 0;
    loop {
        for (index, constraint) in constraints.iter_mut().enumerate() {
            let impulse = constraint.normal_impulse_newton_seconds;
            if impulse != 0.0 { apply_contact_impulse(constraint, 0, -impulse, &responses, index, velocities, fixed_entities); }
            constraint.normal_impulse_newton_seconds = 0.0;
        }
        let current_normal_velocities: Vec<f64> = constraints.iter()
            .map(|constraint| relative_contact_velocity(constraint, 0, velocities)).collect();
        let start_impulses = if outer_iterations == 0 { &initial_normal_impulses } else { &zero_impulses };
        let (resolved_normal_impulses, normal_solve_iterations) = solve_active_set_normal_impulses(
            &normal_matrix, &current_normal_velocities, &restitution_targets, start_impulses,
            settings.velocity_tolerance_metres_per_second)?;
        used_iterations += normal_solve_iterations;
        for (index, constraint) in constraints.iter_mut().enumerate() {
            let impulse = resolved_normal_impulses[index];
            constraint.normal_impulse_newton_seconds = impulse;
            apply_contact_impulse(constraint, 0, impulse, &responses, index, velocities, fixed_entities);
        }
        for _ in 0..settings.maximum_iterations {
            for (index, constraint) in constraints.iter_mut().enumerate() {
                let previous = constraint.tangential_impulse_newton_seconds;
                let matrix = tangential_effective_masses[index];
                let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
                let first_velocity = relative_contact_velocity(constraint, 1, velocities);
                let second_velocity = relative_contact_velocity(constraint, 2, velocities);
                let mut next = [
                    previous[0] + (-first_velocity * matrix[1][1] + second_velocity * matrix[0][1]) / determinant,
                    previous[1] + (-second_velocity * matrix[0][0] + first_velocity * matrix[1][0]) / determinant,
                ];
                let magnitude = next[0].hypot(next[1]);
                let bound = friction_coefficients[index] * constraint.normal_impulse_newton_seconds;
                if magnitude > bound && magnitude > 0.0 { next.iter_mut().for_each(|value| *value *= bound / magnitude); }
                constraint.tangential_impulse_newton_seconds = next;
                for tangent in 0..2 {
                    apply_contact_impulse(constraint, tangent + 1, next[tangent] - previous[tangent],
                        &responses, index, velocities, fixed_entities);
                }
            }
            used_iterations += 1;
            maximum_tangential_residual = constraints.iter().enumerate().map(|(index, constraint)| {
                let previous = constraint.tangential_impulse_newton_seconds;
                let matrix = tangential_effective_masses[index];
                let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
                let first_velocity = relative_contact_velocity(constraint, 1, velocities);
                let second_velocity = relative_contact_velocity(constraint, 2, velocities);
                let mut projected = [
                    previous[0] + (-first_velocity * matrix[1][1] + second_velocity * matrix[0][1]) / determinant,
                    previous[1] + (-second_velocity * matrix[0][0] + first_velocity * matrix[1][0]) / determinant,
                ];
                let magnitude = projected[0].hypot(projected[1]);
                let bound = friction_coefficients[index] * constraint.normal_impulse_newton_seconds;
                if magnitude > bound && magnitude > 0.0 { projected.iter_mut().for_each(|value| *value *= bound / magnitude); }
                let first_change = projected[0] - previous[0];
                let second_change = projected[1] - previous[1];
                let first_residual = matrix[0][0] * first_change + matrix[0][1] * second_change;
                let second_residual = matrix[1][0] * first_change + matrix[1][1] * second_change;
                first_residual.hypot(second_residual)
            }).fold(0.0, f64::max);
            if maximum_tangential_residual <= settings.velocity_tolerance_metres_per_second { break; }
        }
        maximum_residual = constraints.iter().enumerate().map(|(index, constraint)| {
            let residual = relative_contact_velocity(constraint, 0, velocities) - restitution_targets[index];
            if constraint.normal_impulse_newton_seconds > 0.0 { residual.abs() } else { (-residual).max(0.0) }
        }).fold(0.0, f64::max);
        outer_iterations += 1;
        if maximum_residual.max(maximum_tangential_residual) <= settings.velocity_tolerance_metres_per_second { break; }
        if outer_iterations >= 40 { break; }
    }
    if maximum_residual.max(maximum_tangential_residual) > settings.velocity_tolerance_metres_per_second {
        return Err(format!("Generalized contact iteration budget exceeded; normal residual {maximum_residual}, tangential residual {maximum_tangential_residual} metres per second"));
    }
    let final_energy = matrices.iter().zip(velocities.iter()).map(|(matrix, velocity)| generalized_vehicle_energy(matrix, *velocity)).sum::<f64>();
    let mut prescribed_actuator_work = vec![0.0; velocities.len()];
    for constraint in constraints.iter() {
        let impulses = [constraint.normal_impulse_newton_seconds, constraint.tangential_impulse_newton_seconds[0], constraint.tangential_impulse_newton_seconds[1]];
        for axis in 0..3 {
            if !fixed_entities[constraint.first_entity] {
                prescribed_actuator_work[constraint.first_entity] -= impulses[axis] * constraint.first_prescribed_velocity_metres_per_second[axis];
            }
            if let Some(entity) = constraint.second_entity {
                if !fixed_entities[entity] {
                    prescribed_actuator_work[entity] -= impulses[axis] * constraint.second_prescribed_velocity_metres_per_second[axis];
                }
            }
        }
    }
    let prescribed_work = prescribed_actuator_work.iter().sum::<f64>();
    if final_energy > initial_energy + prescribed_work + 1e-8 * (initial_energy.abs() + prescribed_work.abs()).max(1.0) {
        return Err("Passive generalized contact solve added unexplained kinetic energy".into());
    }
    let static_friction_constraint_count = constraints.iter().filter(|constraint| {
        constraint.normal_impulse_newton_seconds > 0.0
            && relative_contact_velocity(constraint, 1, velocities).hypot(
                relative_contact_velocity(constraint, 2, velocities))
                <= STATIC_FRICTION_SPEED_THRESHOLD_METRES_PER_SECOND
    }).count();
    let sliding_constraint_count = constraints.iter().filter(|constraint| {
        constraint.normal_impulse_newton_seconds > 0.0
            && relative_contact_velocity(constraint, 1, velocities).hypot(
                relative_contact_velocity(constraint, 2, velocities))
                > STATIC_FRICTION_SPEED_THRESHOLD_METRES_PER_SECOND
    }).count();
    Ok(GeneralizedContactResult { iterations: used_iterations,
        warm_started_constraint_count,
        maximum_normal_velocity_residual_metres_per_second: maximum_residual,
        maximum_tangential_velocity_residual_metres_per_second: maximum_tangential_residual,
        kinetic_energy_change_joules: final_energy - initial_energy,
        prescribed_actuator_work_joules_by_entity: prescribed_actuator_work,
        normal_impulses_newton_seconds: constraints.iter().map(|constraint| constraint.normal_impulse_newton_seconds).collect(),
        tangential_impulses_newton_seconds: constraints.iter().map(|constraint| constraint.tangential_impulse_newton_seconds).collect(),
        static_friction_constraint_count,
        sliding_constraint_count })
}

fn solve_active_set_normal_impulses(
    matrix: &[Vec<f64>],
    initial_velocities: &[f64],
    targets: &[f64],
    initial_impulses: &[f64],
    tolerance: f64,
) -> Result<(Vec<f64>, usize), String> {
    let count = matrix.len();
    if count == 0 {
        return Ok((Vec::new(), 0));
    }
    if matrix.iter().any(|row| row.len() != count)
        || initial_velocities.len() != count
        || targets.len() != count
        || initial_impulses.len() != count
    {
        return Err("Invalid simultaneous normal contact system dimensions".into());
    }
    let mut impulses = initial_impulses.to_vec();
    for value in &mut impulses {
        if !value.is_finite() || *value < 0.0 { *value = 0.0; }
    }
    let mut active: Vec<usize> = (0..count)
        .filter(|index| initial_velocities[*index] < targets[*index] - tolerance || impulses[*index] > 0.0)
        .collect();
    let mut iterations = 0;
    for _ in 0..(2 * count + 20) {
        iterations += 1;
        if !active.is_empty() {
            let dimension = active.len();
            let mut system = vec![vec![0.0; dimension]; dimension];
            let mut right_hand_side = vec![0.0; dimension];
            for (row, first) in active.iter().enumerate() {
                for (column, second) in active.iter().enumerate() {
                    system[row][column] = matrix[*first][*second];
                }
                let current_velocity = initial_velocities[*first]
                    + (0..count).map(|index| matrix[*first][index] * impulses[index]).sum::<f64>();
                right_hand_side[row] = targets[*first] - current_velocity;
            }
            let delta = match solve_dense_system(&mut system, &mut right_hand_side) {
                Some(delta) => delta,
                None => {
                    let scale = (0..count).map(|index| matrix[index][index].abs()).fold(1.0, f64::max);
                    let regularization = scale * 1e-9;
                    let mut regularized_system = vec![vec![0.0; dimension]; dimension];
                    for (row, first) in active.iter().enumerate() {
                        for (column, second) in active.iter().enumerate() {
                            regularized_system[row][column] = matrix[*first][*second];
                        }
                        let current_velocity = initial_velocities[*first]
                            + (0..count).map(|index| matrix[*first][index] * impulses[index]).sum::<f64>();
                        right_hand_side[row] = targets[*first] - current_velocity;
                    }
                    for (index, row) in regularized_system.iter_mut().enumerate() { row[index] += regularization; }
                    solve_dense_system(&mut regularized_system, &mut right_hand_side)
                        .ok_or("Singular simultaneous normal contact system")?
                }
            };
            for (position, index) in active.iter().enumerate() {
                impulses[*index] += delta[position];
            }
            let mut minimum_position = 0_usize;
            for position in 1..active.len() {
                if impulses[active[position]] < impulses[active[minimum_position]] {
                    minimum_position = position;
                }
            }
            if impulses[active[minimum_position]] < -tolerance {
                impulses[active[minimum_position]] = 0.0;
                active.remove(minimum_position);
                continue;
            }
            for index in &active {
                if impulses[*index] < 0.0 { impulses[*index] = 0.0; }
            }
        }
        let velocities: Vec<f64> = (0..count)
            .map(|row| {
                initial_velocities[row]
                    + (0..count).map(|column| matrix[row][column] * impulses[column]).sum::<f64>()
            })
            .collect();
        let mut worst_inactive: Option<usize> = None;
        let mut worst_violation = tolerance;
        for index in 0..count {
            if active.contains(&index) { continue; }
            let violation = targets[index] - velocities[index];
            if violation > worst_violation {
                worst_violation = violation;
                worst_inactive = Some(index);
            }
        }
        if let Some(index) = worst_inactive {
            active.push(index);
            continue;
        }
        let active_error = active.iter()
            .map(|index| (velocities[*index] - targets[*index]).abs())
            .fold(0.0, f64::max);
        let inactive_error = (0..count)
            .filter(|index| !active.contains(index))
            .map(|index| (targets[index] - velocities[index]).max(0.0))
            .fold(0.0, f64::max);
        if active_error <= tolerance * 10.0 && inactive_error <= tolerance {
            return Ok((impulses, iterations));
        }
    }
    Err("Simultaneous normal contact active set did not converge".into())
}

fn solve_dense_system(matrix: &mut [Vec<f64>], right_hand_side: &mut [f64]) -> Option<Vec<f64>> {
    let dimension = right_hand_side.len();
    if matrix.len() != dimension || matrix.iter().any(|row| row.len() != dimension) {
        return None;
    }
    for column in 0..dimension {
        let mut pivot = column;
        for row in column + 1..dimension {
            if matrix[row][column].abs() > matrix[pivot][column].abs() { pivot = row; }
        }
        if matrix[pivot][column].abs() < 1e-14 { return None; }
        if pivot != column {
            matrix.swap(pivot, column);
            right_hand_side.swap(pivot, column);
        }
        for row in column + 1..dimension {
            let factor = matrix[row][column] / matrix[column][column];
            if factor == 0.0 { continue; }
            for entry in column..dimension {
                matrix[row][entry] -= factor * matrix[column][entry];
            }
            right_hand_side[row] -= factor * right_hand_side[column];
        }
    }
    let mut solution = vec![0.0; dimension];
    for row in (0..dimension).rev() {
        let mut value = right_hand_side[row];
        for column in row + 1..dimension {
            value -= matrix[row][column] * solution[column];
        }
        solution[row] = value / matrix[row][row];
    }
    if solution.iter().any(|value| !value.is_finite()) { return None; }
    Some(solution)
}
