use crate::types::{Mat3, Vec3};

#[derive(Debug, Clone, Copy, Default)]
pub struct StaticLateralContact {
    pub arm_from_center_world_metres: Vec3,
    pub lateral_direction_world: Vec3,
    pub lateral_velocity_metres_per_second: f64,
    pub force_capacity_newtons: f64,
    pub blend_weight: f64,
}

pub fn solve_static_lateral_contact_forces(
    total_mass_kilograms: f64,
    principal_inertia_kilogram_square_metres: Vec3,
    body_basis: Mat3,
    contacts: &[StaticLateralContact; 4],
    duration_seconds: f64,
) -> [f64; 4] {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return [0.0; 4];
    }
    let inverse_mass = 1.0 / total_mass_kilograms.max(1e-6);
    let inverse_inertia = Vec3::new(
        1.0 / principal_inertia_kilogram_square_metres.x.max(1e-6),
        1.0 / principal_inertia_kilogram_square_metres.y.max(1e-6),
        1.0 / principal_inertia_kilogram_square_metres.z.max(1e-6),
    );
    let angular_directions_local: [Vec3; 4] = std::array::from_fn(|index| {
        body_basis.inverse_transform_vector(
            contacts[index]
                .arm_from_center_world_metres
                .cross(contacts[index].lateral_direction_world),
        )
    });
    let contact_response: [[f64; 4]; 4] = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let angular_response = Vec3::new(
                angular_directions_local[column].x * inverse_inertia.x,
                angular_directions_local[column].y * inverse_inertia.y,
                angular_directions_local[column].z * inverse_inertia.z,
            );
            contacts[row]
                .lateral_direction_world
                .dot(contacts[column].lateral_direction_world)
                * inverse_mass
                + angular_directions_local[row].dot(angular_response)
        })
    });
    let mut impulses_newton_seconds = [0.0; 4];
    for _ in 0..64 {
        let mut maximum_impulse_change = 0.0_f64;
        for index in 0..4 {
            let contact = &contacts[index];
            let weight = contact.blend_weight.clamp(0.0, 1.0);
            let diagonal_response = contact_response[index][index];
            if weight <= 0.0
                || !diagonal_response.is_finite()
                || diagonal_response <= 1e-12
                || !contact.lateral_velocity_metres_per_second.is_finite()
            {
                continue;
            }
            let compliance_response = diagonal_response * (1.0 - weight) / weight;
            let residual_velocity = contact.lateral_velocity_metres_per_second
                + contact_response[index]
                    .iter()
                    .zip(impulses_newton_seconds)
                    .map(|(response, impulse)| response * impulse)
                    .sum::<f64>()
                + compliance_response * impulses_newton_seconds[index];
            let impulse_capacity = contact.force_capacity_newtons.max(0.0) * duration_seconds;
            let updated_impulse = (impulses_newton_seconds[index]
                - residual_velocity / (diagonal_response + compliance_response))
                .clamp(-impulse_capacity, impulse_capacity);
            maximum_impulse_change = maximum_impulse_change
                .max((updated_impulse - impulses_newton_seconds[index]).abs());
            impulses_newton_seconds[index] = updated_impulse;
        }
        if maximum_impulse_change < 1e-10 {
            break;
        }
    }
    impulses_newton_seconds.map(|impulse| impulse / duration_seconds)
}
