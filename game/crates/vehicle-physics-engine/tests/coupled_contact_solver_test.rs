use vehicle_physics_engine::coupled_vehicle_dynamics::{
    factor_generalized_vehicle_matrix, generalized_vehicle_energy,
    solve_factored_vehicle_matrix, CoupledVehicleModel, CoupledVehicleState,
    GeneralizedVehicleMatrix,
};
use vehicle_physics_engine::generalized_vehicle_contacts::{
    solve_generalized_vehicle_contacts, GeneralizedContactConstraint, GeneralizedContactSettings,
};
use vehicle_physics_engine::suspension_coupled_solver::CoupledSuspensionInput;
use vehicle_physics_engine::{Vec3, VehicleConfig, WheelIndex};

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn actual_model() -> CoupledVehicleModel {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.as_ref().unwrap().suspension.clone();
    CoupledVehicleModel::new(configuration, settings).unwrap()
}

fn contact_settings() -> GeneralizedContactSettings {
    GeneralizedContactSettings { maximum_iterations: 100, velocity_tolerance_metres_per_second: 1e-8,
        restitution: 0.0, restitution_minimum_speed_metres_per_second: 1.0, impact_friction_coefficient: 0.5,
        impact_friction_by_surface_code: std::collections::BTreeMap::new(),
        prediction_distance_metres: 0.001, maximum_position_correction_metres: 0.001, maximum_events_per_step: 8,
        maximum_supported_speed_metres_per_second: 120.0, maximum_supported_angular_speed_radians_per_second: 10.0 }
}

fn redundant_support_constraint(x: f64, z: f64) -> GeneralizedContactConstraint {
    let mut jacobians = [[0.0; 14]; 3];
    jacobians[0][1] = 1.0;
    jacobians[0][3] = -z;
    jacobians[0][5] = x;
    jacobians[1][0] = 1.0;
    jacobians[1][4] = z;
    jacobians[2][2] = 1.0;
    jacobians[2][4] = -x;
    GeneralizedContactConstraint { identifier: format!("support_{x}_{z}"), first_entity: 0, second_entity: None,
        first_jacobians: jacobians, second_jacobians: [[0.0; 14]; 3],
        first_prescribed_velocity_metres_per_second: [0.0; 3],
        second_prescribed_velocity_metres_per_second: [0.0; 3],
        surface_code: 0,
        normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2] }
}

fn pair_constraint(identifier: &str, first: usize, second: usize) -> GeneralizedContactConstraint {
    let mut first_jacobians = [[0.0; 14]; 3];
    let mut second_jacobians = [[0.0; 14]; 3];
    for axis in 0..3 {
        first_jacobians[axis][axis] = 1.0;
        second_jacobians[axis][axis] = -1.0;
    }
    GeneralizedContactConstraint { identifier: identifier.into(), first_entity: first, second_entity: Some(second),
        first_jacobians, second_jacobians,
        first_prescribed_velocity_metres_per_second: [0.0; 3],
        second_prescribed_velocity_metres_per_second: [0.0; 3],
        surface_code: 6, normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2] }
}

#[test]
fn three_vehicle_chain_closes_momentum_and_transmits_through_the_middle_car() {
    let masses = [100.0_f64, 300.0, 100.0];
    let matrices = masses.iter().map(|mass| {
        let mut matrix: GeneralizedVehicleMatrix = [[0.0; 14]; 14];
        for coordinate in 0..3 { matrix[coordinate][coordinate] = *mass; }
        for coordinate in 3..14 { matrix[coordinate][coordinate] = 1.0; }
        matrix
    }).collect::<Vec<_>>();
    let mut velocities = [[0.0; 14]; 3];
    velocities[0][0] = -5.0;
    velocities[1][0] = 0.0;
    velocities[2][0] = 15.0;
    let mut constraints = [pair_constraint("a_b", 0, 1), pair_constraint("b_c", 1, 2)];
    let initial_momentum = masses.iter().zip(velocities.iter()).map(|(mass, velocity)| mass * velocity[0]).sum::<f64>();
    let result = solve_generalized_vehicle_contacts(&matrices, &mut velocities, &mut constraints, &contact_settings()).unwrap();
    let final_momentum = masses.iter().zip(velocities.iter()).map(|(mass, velocity)| mass * velocity[0]).sum::<f64>();
    assert!((final_momentum - initial_momentum).abs() < 1e-6);
    assert!(velocities[1][0].abs() > 1e-6, "the middle car must receive a transmitted impulse");
    assert!(result.normal_impulses_newton_seconds.iter().all(|impulse| *impulse >= 0.0));
    assert!(result.maximum_normal_velocity_residual_metres_per_second <= contact_settings().velocity_tolerance_metres_per_second);
}

#[test]
fn four_redundant_coplanar_supports_converge_without_attraction() {
    let mut matrix: GeneralizedVehicleMatrix = [[0.0; 14]; 14];
    matrix[0][0] = 600.0; matrix[1][1] = 600.0; matrix[2][2] = 600.0;
    matrix[3][3] = 300.0; matrix[4][4] = 450.0; matrix[5][5] = 250.0;
    for coordinate in 6..14 { matrix[coordinate][coordinate] = 1.0; }
    let mut constraints = vec![
        redundant_support_constraint(-0.5, -1.0),
        redundant_support_constraint(0.5, -1.0),
        redundant_support_constraint(0.5, 1.0),
        redundant_support_constraint(-0.5, 1.0),
    ];
    constraints.sort_by(|first, second| first.identifier.cmp(&second.identifier));
    let mut velocities = [[0.0; 14]];
    velocities[0][1] = -2.0;
    let initial_energy = generalized_vehicle_energy(&matrix, velocities[0]);
    let result = solve_generalized_vehicle_contacts(&[matrix], &mut velocities, &mut constraints, &contact_settings()).unwrap();
    assert!(result.maximum_normal_velocity_residual_metres_per_second <= contact_settings().velocity_tolerance_metres_per_second);
    assert!(result.maximum_tangential_velocity_residual_metres_per_second <= contact_settings().velocity_tolerance_metres_per_second);
    assert!(result.normal_impulses_newton_seconds.iter().all(|impulse| *impulse >= 0.0));
    assert!(result.normal_impulses_newton_seconds.iter().any(|impulse| *impulse > 0.0));
    assert!(velocities[0][1] >= -contact_settings().velocity_tolerance_metres_per_second);
    assert!(result.kinetic_energy_change_joules <= 1e-9);
    assert!((result.kinetic_energy_change_joules + initial_energy).abs() <= initial_energy * 1e-6);
    assert!(result.iterations > 0);
}

fn contact_settings_with_friction(global: f64, pairs: &[(u32, f64)]) -> GeneralizedContactSettings {
    let mut settings = contact_settings();
    settings.impact_friction_coefficient = global;
    settings.impact_friction_by_surface_code = pairs.iter().copied().collect();
    settings
}

fn identity_matrix() -> GeneralizedVehicleMatrix {
    let mut matrix = [[0.0; 14]; 14];
    for coordinate in 0..14 { matrix[coordinate][coordinate] = 1.0; }
    matrix
}

fn single_tangential_contact(surface_code: u32, tangential_velocity: f64) -> ([[f64; 14]; 1], [GeneralizedContactConstraint; 1]) {
    let mut jacobians = [[0.0; 14]; 3];
    jacobians[0][1] = 1.0;
    jacobians[1][0] = 1.0;
    jacobians[2][2] = 1.0;
    let mut velocities = [[0.0; 14]];
    velocities[0][1] = -1.0;
    velocities[0][0] = tangential_velocity;
    let constraints = [GeneralizedContactConstraint { identifier: "material_probe".into(), first_entity: 0, second_entity: None,
        first_jacobians: jacobians, second_jacobians: [[0.0; 14]; 3],
        first_prescribed_velocity_metres_per_second: [0.0; 3],
        second_prescribed_velocity_metres_per_second: [0.0; 3],
        surface_code, normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2] }];
    (velocities, constraints)
}

#[test]
fn surface_material_pairing_bounds_tangential_impulse_per_surface() {
    let settings = contact_settings_with_friction(0.5, &[(0, 0.1), (1, 1.0)]);
    let (mut low_velocity, mut low_constraint) = single_tangential_contact(0, 2.0);
    let (mut high_velocity, mut high_constraint) = single_tangential_contact(1, 2.0);
    let low_result = solve_generalized_vehicle_contacts(&[identity_matrix()],
        &mut low_velocity, &mut low_constraint, &settings).unwrap();
    let high_result = solve_generalized_vehicle_contacts(&[identity_matrix()],
        &mut high_velocity, &mut high_constraint, &settings).unwrap();
    assert!(low_result.tangential_impulses_newton_seconds[0][0] < 0.0);
    assert!(high_result.tangential_impulses_newton_seconds[0][0] < 0.0);
    assert!(low_result.tangential_impulses_newton_seconds[0][0].abs() <= 0.1 + 1e-9);
    assert!(high_result.tangential_impulses_newton_seconds[0][0].abs() <= 1.0 + 1e-9);
    assert!(high_velocity[0][0] < low_velocity[0][0]);
    assert!(low_result.kinetic_energy_change_joules <= 1e-9);
    assert!(high_result.kinetic_energy_change_joules <= 1e-9);
}

#[test]
fn stick_transition_stops_small_tangential_slip_within_the_cone() {
    let settings = contact_settings_with_friction(0.5, &[(0, 1.0)]);
    let (mut velocities, mut constraints) = single_tangential_contact(0, 0.05);
    let initial_energy = generalized_vehicle_energy(&identity_matrix(), velocities[0]);
    let result = solve_generalized_vehicle_contacts(&[identity_matrix()],
        &mut velocities, &mut constraints, &settings).unwrap();
    assert!(velocities[0][0].abs() < 1e-8);
    assert!(result.tangential_impulses_newton_seconds[0][0].abs() <= result.normal_impulses_newton_seconds[0] + 1e-9);
    assert!(result.kinetic_energy_change_joules <= 1e-9);
    assert!(initial_energy > 0.0);
}

#[test]
fn spinning_wheel_against_a_barrier_receives_friction_spin_torque() {
    let model = actual_model();
    let state = CoupledVehicleState::at_rest(Vec3::new(0.0, 1.0, 0.0));
    let input = CoupledSuspensionInput::default();
    let wheel = WheelIndex::FrontLeft;
    let evaluation = model.evaluate(&state, &input, [0.0; 4]).unwrap();
    let hub = evaluation.components.iter().zip(model.inventory.components.iter())
        .find(|(_, definition)| definition.rotating_wheel == Some(wheel))
        .map(|(moving, _)| moving.center_of_mass_world_metres).unwrap();
    let radius = model.configuration.tires.front_tire_radius;
    let contact_point = hub + Vec3::FORWARD * radius;
    let normal = Vec3::BACK;
    let first_tangent = Vec3::UP;
    let second_tangent = normal.cross(first_tangent).normalized();
    let axes = [normal, first_tangent, second_tangent];
    let mut jacobians = [[0.0; 14]; 3];
    for (axis, direction) in axes.iter().enumerate() {
        jacobians[axis] = model.contact_jacobian(&state, &input, Some(wheel), contact_point, *direction).unwrap();
    }
    let constraint = GeneralizedContactConstraint { identifier: "spinning_wheel_barrier".into(), first_entity: 0, second_entity: None,
        first_jacobians: jacobians, second_jacobians: [[0.0; 14]; 3],
        first_prescribed_velocity_metres_per_second: [0.0; 3],
        second_prescribed_velocity_metres_per_second: [0.0; 3],
        surface_code: 6, normal_impulse_newton_seconds: 0.0, tangential_impulse_newton_seconds: [0.0; 2] };
    let mut velocities = [state.velocity()];
    velocities[0][2] = -5.0;
    velocities[0][10 + wheel as usize] = 50.0;
    let initial_spin = velocities[0][10 + wheel as usize];
    let result = solve_generalized_vehicle_contacts(&[evaluation.mass_matrix], &mut velocities,
        &mut [constraint], &contact_settings_with_friction(0.5, &[(6, 0.7)])).unwrap();
    assert!(result.normal_impulses_newton_seconds[0] > 0.0);
    assert!((velocities[0][10 + wheel as usize] - initial_spin).abs() > 0.1);
    let tangential_magnitude = result.tangential_impulses_newton_seconds[0][0].hypot(result.tangential_impulses_newton_seconds[0][1]);
    assert!(tangential_magnitude <= 0.7 * result.normal_impulses_newton_seconds[0] + 1e-9);
    assert!(result.kinetic_energy_change_joules <= 1e-9);
}

#[test]
fn off_center_wheel_impulse_reaches_body_and_internal_coordinates() {
    let model = actual_model();
    let state = CoupledVehicleState::at_rest(Vec3::new(0.0, 1.0, 0.0));
    let input = CoupledSuspensionInput::default();
    let wheel = WheelIndex::FrontLeft;
    let evaluation = model.evaluate(&state, &input, [0.0; 4]).unwrap();
    let hub = evaluation.components.iter().zip(model.inventory.components.iter())
        .find(|(_, definition)| definition.rotating_wheel == Some(wheel))
        .map(|(moving, _)| moving.center_of_mass_world_metres).unwrap();
    let contact_point = hub + Vec3::DOWN * model.configuration.tires.front_tire_radius;
    let wheel_jacobian = model.contact_jacobian(&state, &input, Some(wheel), contact_point, Vec3::FORWARD).unwrap();
    let body_jacobian = model.contact_jacobian(&state, &input, None, contact_point, Vec3::FORWARD).unwrap();
    let factor = factor_generalized_vehicle_matrix(&model.evaluate(&state, &input, [0.0; 4]).unwrap().mass_matrix).unwrap();
    let wheel_response = solve_factored_vehicle_matrix(&factor, wheel_jacobian).unwrap();
    let body_response = solve_factored_vehicle_matrix(&factor, body_jacobian).unwrap();
    assert!(wheel_response[2].abs() > 0.0);
    assert!(wheel_response[3].abs() + wheel_response[4].abs() + wheel_response[5].abs() > 0.0);
    assert!(wheel_response[6..10].iter().any(|velocity| velocity.abs() > 0.0));
    assert!(wheel_response[10..14].iter().any(|velocity| velocity.abs() > 0.0));
    assert!(wheel_jacobian[6..10].iter().any(|value| value.abs() > 0.0));
    assert!(wheel_jacobian[10..14].iter().any(|value| value.abs() > 0.0));
    assert!(body_jacobian[6..10].iter().all(|value| value.abs() < 1e-12));
    assert!(body_jacobian[10..14].iter().all(|value| value.abs() < 1e-12));
    assert_ne!(wheel_response, body_response);
}
