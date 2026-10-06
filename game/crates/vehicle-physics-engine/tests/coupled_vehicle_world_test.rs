use vehicle_physics_engine::coupled_vehicle_dynamics::*;
use vehicle_physics_engine::coupled_vehicle_world::*;
use vehicle_physics_engine::generalized_vehicle_contacts::*;
use vehicle_physics_engine::physical_collision_world::*;
use vehicle_physics_engine::suspension_coupled_solver::CoupledSuspensionInput;
use vehicle_physics_engine::*;

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json")).unwrap()
}

fn actual_model() -> CoupledVehicleModel {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.as_ref().unwrap().suspension.clone();
    CoupledVehicleModel::new(configuration, settings).unwrap()
}

fn diagonal_mass(mass: f64) -> GeneralizedVehicleMatrix {
    let mut matrix = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    for coordinate in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT { matrix[coordinate][coordinate] = if coordinate < 3 { mass } else { 1.0 }; }
    matrix
}

fn normal_constraint(second: Option<usize>) -> GeneralizedContactConstraint {
    let mut first = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]; 3];
    let mut other = first;
    for axis in 0..3 { first[axis][axis] = 1.0; other[axis][axis] = -1.0; }
    GeneralizedContactConstraint { identifier: "analytical_contact".into(), first_entity: 0, second_entity: second,
        first_jacobians: first, second_jacobians: other, normal_impulse_newton_seconds: 0.0,
        first_prescribed_velocity_metres_per_second: [0.0; 3], second_prescribed_velocity_metres_per_second: [0.0; 3],
        tangential_impulse_newton_seconds: [0.0; 2] }
}

#[test]
fn actual_inventory_closes_every_budget_and_round_trips() {
    let configuration = actual_configuration();
    let inventory = configuration.physical_mass_inventory.as_ref().unwrap();
    inventory.validate(&configuration).unwrap();
    let audit = inventory.audit();
    assert_eq!(inventory.components.len(), 80);
    assert!((audit.base_mass_kilograms - 600.0).abs() < 1e-6);
    assert!((audit.complete_unfueled_mass_kilograms - 698.0).abs() < 1e-6);
    assert_eq!(audit.rim_and_tire_mass_kilograms, [21.0, 21.0, 28.0, 28.0]);
    let restored = VehicleConfig::from_json_str(&configuration.to_json_value().to_string()).unwrap();
    assert_eq!(serde_json::to_value(restored.physical_mass_inventory).unwrap(), serde_json::to_value(&configuration.physical_mass_inventory).unwrap());
}

#[test]
fn inventory_rejects_budget_reassignment_rotation_mismatch_and_invalid_uncertainty() {
    let configuration = actual_configuration();
    let mut inventory = configuration.physical_mass_inventory.clone().unwrap();
    inventory.components[0].budget = vehicle_physics_engine::vehicle_mass_inventory::VehicleMassBudget::RimAndTire;
    assert!(inventory.validate(&configuration).is_err());
    let mut inventory = configuration.physical_mass_inventory.clone().unwrap();
    inventory.components[0].rotating_wheel = Some(WheelIndex::FrontLeft);
    assert!(inventory.validate(&configuration).is_err());
    let mut inventory = configuration.physical_mass_inventory.clone().unwrap();
    inventory.components[0].uncertainty.lower_mass_kilograms = 100.0;
    assert!(inventory.validate(&configuration).is_err());
}

#[test]
fn production_mass_matrix_is_symmetric_and_matches_rotating_component_energy() {
    let model = actual_model();
    let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 10.0, 0.0));
    state.wheel_angular_velocity_radians_per_second = [10.0, -20.0, 30.0, -40.0];
    let evaluation = model.evaluate(&state, &CoupledSuspensionInput::default(), [0.0; 4]).unwrap();
    let audit = model.inventory.audit();
    let expected = (0..4).map(|wheel| 0.5 * audit.spin_inertia_kilogram_square_metres[wheel]
        * state.wheel_angular_velocity_radians_per_second[wheel].powi(2)).sum::<f64>();
    assert!((evaluation.kinetic_energy_joules - expected).abs() < expected * 1e-10);
    for row in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT { for column in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
        assert!((evaluation.mass_matrix[row][column] - evaluation.mass_matrix[column][row]).abs() < 1e-10);
    } }
}

#[test]
fn internal_wheel_torque_transfers_angular_momentum_to_the_body() {
    let model = actual_model();
    let state = CoupledVehicleState::at_rest(Vec3::new(0.0, 10.0, 0.0));
    let input = CoupledSuspensionInput::default();
    let passive = model.evaluate(&state, &input, [0.0; 4]).unwrap();
    let driven = model.evaluate(&state, &input, [100.0, 0.0, 0.0, 0.0]).unwrap();
    let force = std::array::from_fn(|coordinate| driven.generalized_force[coordinate] - passive.generalized_force[coordinate]);
    let acceleration = solve_factored_vehicle_matrix(&factor_generalized_vehicle_matrix(&driven.mass_matrix).unwrap(), force).unwrap();
    assert!(acceleration[10] > 0.0);
    assert!(acceleration[3] < 0.0);
    for body_coordinate in 0..6 {
        let momentum_rate = (0..VEHICLE_GENERALIZED_VELOCITY_COUNT).map(|coordinate| driven.mass_matrix[body_coordinate][coordinate] * acceleration[coordinate]).sum::<f64>();
        assert!(momentum_rate.abs() < 1e-8);
    }
}

#[test]
fn unilateral_static_contact_matches_analytical_restitution_and_dissipation() {
    let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    velocity[0] = -10.0;
    let mut velocities = [velocity];
    let mut constraints = [normal_constraint(None)];
    let mut settings = actual_configuration().coupled_world.unwrap().contacts;
    settings.restitution = 0.25;
    let result = solve_generalized_vehicle_contacts(&[diagonal_mass(100.0)], &mut velocities, &mut constraints, &settings).unwrap();
    assert!((velocities[0][0] - 2.5).abs() < 1e-10);
    assert!((result.normal_impulses_newton_seconds[0] - 1250.0).abs() < 1e-8);
    assert!((result.kinetic_energy_change_joules + 4687.5).abs() < 1e-8);
}

#[test]
fn separating_contacts_receive_no_attractive_normal_or_friction_impulse() {
    let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    velocity[0] = 10.0;
    velocity[1] = 5.0;
    let mut velocities = [velocity];
    let mut constraints = [normal_constraint(None)];
    let settings = actual_configuration().coupled_world.unwrap().contacts;
    let result = solve_generalized_vehicle_contacts(&[diagonal_mass(100.0)], &mut velocities, &mut constraints, &settings).unwrap();
    assert_eq!(velocities[0], velocity);
    assert_eq!(result.normal_impulses_newton_seconds[0], 0.0);
    assert_eq!(result.tangential_impulses_newton_seconds[0], [0.0; 2]);
}

#[test]
fn two_vehicle_impact_conserves_momentum_with_unequal_masses() {
    let mut first = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    let second = first;
    first[0] = -10.0;
    let mut velocities = [first, second];
    let mut constraints = [normal_constraint(Some(1))];
    let mut settings = actual_configuration().coupled_world.unwrap().contacts;
    settings.restitution = 0.0;
    solve_generalized_vehicle_contacts(&[diagonal_mass(100.0), diagonal_mass(300.0)], &mut velocities, &mut constraints, &settings).unwrap();
    assert!((velocities[0][0] + 2.5).abs() < 1e-10);
    assert!((velocities[1][0] + 2.5).abs() < 1e-10);
    assert!((100.0 * velocities[0][0] + 300.0 * velocities[1][0] + 1000.0).abs() < 1e-8);
}

#[test]
fn oblique_contact_friction_obeys_the_cone_and_loses_energy() {
    let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    velocity[..3].copy_from_slice(&[-10.0, 100.0, 100.0]);
    let mut velocities = [velocity];
    let mut constraints = [normal_constraint(None)];
    let settings = actual_configuration().coupled_world.unwrap().contacts;
    let result = solve_generalized_vehicle_contacts(&[diagonal_mass(100.0)], &mut velocities, &mut constraints, &settings).unwrap();
    let tangent = result.tangential_impulses_newton_seconds[0];
    assert!((tangent[0].powi(2) + tangent[1].powi(2)).sqrt() <= settings.impact_friction_coefficient * result.normal_impulses_newton_seconds[0] + 1e-10);
    assert!(result.kinetic_energy_change_joules < 0.0);
}

#[test]
fn coupled_world_preserves_fixed_time_pause_and_stale_entity_identity() {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let terrain = PhysicalCollisionWorld { source_digest: "analytical_empty_world".into(), shapes: Vec::new() };
    let mut world = CoupledVehicleWorld::new(terrain, settings).unwrap();
    let identifier = world.register_vehicle(configuration.clone(), Vec3::new(0.0, 10.0, 0.0), 0.0).unwrap();
    world.paused = true;
    world.advance_host_interval(0.1).unwrap();
    assert_eq!(world.time_seconds, 0.0);
    world.paused = false;
    world.advance_host_interval(0.002).unwrap();
    assert!((world.time_seconds + world.accumulated_host_time_seconds - 0.002).abs() < 1e-12);
    world.remove_vehicle(identifier).unwrap();
    assert!(world.reset_vehicle(identifier).is_err());
    let replacement = world.register_vehicle(configuration, Vec3::new(0.0, 10.0, 0.0), 0.0).unwrap();
    assert_ne!(identifier, replacement);
}

#[test]
fn fuel_exchange_changes_mass_without_changing_any_generalized_velocity() {
    let mut model = actual_model();
    let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 10.0, 0.0));
    state.suspension.generalized_velocity[0] = 10.0;
    state.wheel_angular_velocity_radians_per_second[0] = 30.0;
    let before = state.velocity();
    model.update_fuel_mass(&mut state, &CoupledSuspensionInput::default(), 0.0).unwrap();
    assert_eq!(state.velocity(), before);
    assert_eq!(model.configuration.total_vehicle_mass(), 698.0);
    assert!(state.fuel_exchange_energy_joules.is_finite());
}

#[test]
fn tangential_contact_converges_with_coupled_unequal_effective_masses() {
    let mut matrix = diagonal_mass(100.0);
    matrix[1][1] = 2.0;
    matrix[2][2] = 10.0;
    matrix[1][2] = 1.0;
    matrix[2][1] = 1.0;
    let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    velocity[..3].copy_from_slice(&[-10.0, 3.0, -4.0]);
    let mut velocities = [velocity];
    let mut constraints = [normal_constraint(None)];
    let settings = actual_configuration().coupled_world.unwrap().contacts;
    let result = solve_generalized_vehicle_contacts(&[matrix], &mut velocities, &mut constraints, &settings).unwrap();
    assert!(result.iterations > 1);
    assert!(velocities[0][1].hypot(velocities[0][2]) < 1e-5);
    assert!(result.maximum_tangential_velocity_residual_metres_per_second <= settings.velocity_tolerance_metres_per_second);
    assert!((result.tangential_impulses_newton_seconds[0][0] + 2.0).abs() < 1e-4);
    assert!((result.tangential_impulses_newton_seconds[0][1] - 37.0).abs() < 1e-4);
}

#[test]
fn wheel_continuous_detection_keeps_vertical_faces_of_driveable_meshes() {
    let vertices = vec![parry3d_f64::na::Point3::new(0.0, -10.0, -10.0),
        parry3d_f64::na::Point3::new(0.0, 10.0, -10.0), parry3d_f64::na::Point3::new(0.0, 0.0, 10.0)];
    let mesh = parry3d_f64::shape::TriMesh::new(vertices, vec![[0, 1, 2]]).unwrap();
    let shared_shape = parry3d_f64::shape::SharedShape::new(mesh);
    let world = PhysicalCollisionWorld { source_digest: "analytical_vertical_curb".into(),
        shapes: vec![PhysicalWorldShape { identifier: "curb_vertical_face".into(), surface_code: 1,
            driveable: true, shape: shared_shape.clone(), wheel_impact_shape: Some(shared_shape) }] };
    let shape = parry3d_f64::shape::SharedShape::ball(0.5);
    let pose = parry3d_f64::na::Isometry3::translation(2.0, 0.0, 0.0);
    let impact = world.earliest_translation_impact(&pose, &shape, Vec3::new(-100.0, 0.0, 0.0), 0.1, true).unwrap().unwrap();
    assert!((impact - 0.015).abs() < 1e-8);
}

#[test]
fn physical_interface_rejects_invalid_documents_versions_and_buffers() {
    use vehicle_physics_engine::coupled_vehicle_world_interface::*;
    for document in ["", "{}", "{\"operation\":\"snapshots\",\"world_identifier\":99999999}",
        "{\"operation\":\"snapshots\",\"world_identifier\":1,\"unexpected\":true}"] {
        let response: serde_json::Value = serde_json::from_str(&execute_coupled_vehicle_world_request(document)).unwrap();
        assert_eq!(response["success"], false);
        assert_eq!(response["interface_version"], COUPLED_VEHICLE_WORLD_INTERFACE_VERSION);
    }
    let request = serde_json::json!({"operation": "create_world", "interface_version": 999,
        "repository_root": ".", "physical_package_path": "missing", "configuration": actual_configuration().coupled_world.unwrap()});
    let response: serde_json::Value = serde_json::from_str(&execute_coupled_vehicle_world_request(&request.to_string())).unwrap();
    assert!(response["error"].as_str().unwrap().contains("version mismatch"));
    unsafe {
        let response = coupled_vehicle_world_execute_request(std::ptr::null(), 1);
        let document = std::ffi::CStr::from_ptr(response).to_str().unwrap();
        assert_eq!(serde_json::from_str::<serde_json::Value>(document).unwrap()["success"], false);
        coupled_vehicle_world_free_response(response);
        coupled_vehicle_world_free_response(std::ptr::null_mut());
        let invalid = [255_u8];
        let response = coupled_vehicle_world_execute_request(invalid.as_ptr(), invalid.len());
        assert!(std::ffi::CStr::from_ptr(response).to_str().unwrap().contains("UTF-8"));
        coupled_vehicle_world_free_response(response);
    }
}

#[test]
fn unconverged_contact_does_not_commit_partial_impulses_or_velocity() {
    let mut matrix = diagonal_mass(100.0);
    matrix[1][1] = 2.0;
    matrix[2][2] = 10.0;
    matrix[1][2] = 1.0;
    matrix[2][1] = 1.0;
    let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    velocity[..3].copy_from_slice(&[-10.0, 3.0, -4.0]);
    let mut velocities = [velocity];
    let mut constraints = [normal_constraint(None)];
    let mut settings = actual_configuration().coupled_world.unwrap().contacts;
    settings.maximum_iterations = 1;
    assert!(solve_generalized_vehicle_contacts(&[matrix], &mut velocities, &mut constraints, &settings).is_err());
    assert_eq!(velocities[0], velocity);
    assert_eq!(constraints[0].normal_impulse_newton_seconds, 0.0);
    assert_eq!(constraints[0].tangential_impulse_newton_seconds, [0.0; 2]);
}

#[test]
fn timestamped_input_changes_are_applied_inside_the_physical_interval() {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let duration = 1.0 / settings.physical_frequency_hertz as f64;
    let terrain = PhysicalCollisionWorld { source_digest: "analytical_empty_world".into(), shapes: Vec::new() };
    let mut world = CoupledVehicleWorld::new(terrain, settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 10.0, 0.0), 0.0).unwrap();
    world.enqueue_input(identifier, TimestampedVehicleInput { time_seconds: 0.0,
        input: VehicleInput::default(), driving_aids_mask: 32 }).unwrap();
    world.enqueue_input(identifier, TimestampedVehicleInput { time_seconds: duration * 0.5,
        input: VehicleInput::default(), driving_aids_mask: 0 }).unwrap();
    world.advance_host_interval(duration).unwrap();
    assert_eq!(world.vehicles[&identifier].driving_aids_mask, 0);
    assert!(world.vehicles[&identifier].pending_inputs.is_empty());
    assert!((world.vehicles[&identifier].state.suspension.simulated_time_seconds - duration).abs() < 1e-12);
}

#[test]
fn host_cadence_does_not_change_the_authoritative_trajectory() {
    let configuration = actual_configuration();
    let terrain = PhysicalCollisionWorld { source_digest: "analytical_empty_world".into(), shapes: Vec::new() };
    let mut first_world = CoupledVehicleWorld::new(terrain, configuration.coupled_world.clone().unwrap()).unwrap();
    let identifier = first_world.register_vehicle(configuration, Vec3::new(0.0, 10.0, 0.0), 0.0).unwrap();
    first_world.enqueue_input(identifier, TimestampedVehicleInput { time_seconds: 0.003,
        input: VehicleInput::default(), driving_aids_mask: 0 }).unwrap();
    let mut second_world = first_world.clone();
    first_world.advance_host_interval(0.01).unwrap();
    second_world.advance_host_interval(0.005).unwrap();
    second_world.advance_host_interval(0.005).unwrap();
    assert_eq!(serde_json::to_value(&first_world.vehicles[&identifier].state).unwrap(),
        serde_json::to_value(&second_world.vehicles[&identifier].state).unwrap());
    assert_eq!(serde_json::to_value(&first_world.vehicles[&identifier].systems.state).unwrap(),
        serde_json::to_value(&second_world.vehicles[&identifier].systems.state).unwrap());
}

#[test]
fn prescribed_rack_motion_contributes_physical_component_kinetic_energy() {
    let model = actual_model();
    let state = CoupledVehicleState::at_rest(Vec3::new(0.0, 10.0, 0.0));
    let mut input = CoupledSuspensionInput::default();
    input.steering_rack_velocity_metres_per_second = 0.02;
    let evaluation = model.evaluate(&state, &input, [0.0; 4]).unwrap();
    let components = vehicle_physics_engine::suspension_component_kinematics::moving_inventory(
        &model.configuration, &model.inventory.reference_inventory(), model.kinematic_input(&state, &input)).unwrap();
    let expected = components.iter().map(|component| component.kinetic_energy_joules(state.suspension.generalized_velocity)).sum::<f64>();
    assert!(expected > 0.0);
    assert!((evaluation.kinetic_energy_joules - expected).abs() < 1e-10);
    assert_eq!(generalized_vehicle_energy(&evaluation.mass_matrix, state.velocity()), 0.0);
}

#[test]
fn prescribed_contact_motion_has_an_explicit_energy_source() {
    let mut velocities = [[0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT]];
    let mut constraints = [normal_constraint(None)];
    constraints[0].first_prescribed_velocity_metres_per_second[0] = -10.0;
    let mut settings = actual_configuration().coupled_world.unwrap().contacts;
    settings.restitution = 0.0;
    let result = solve_generalized_vehicle_contacts(&[diagonal_mass(100.0)], &mut velocities, &mut constraints, &settings).unwrap();
    assert!((velocities[0][0] - 10.0).abs() < 1e-10);
    assert!((result.normal_impulses_newton_seconds[0] - 1000.0).abs() < 1e-8);
    assert!((result.prescribed_actuator_work_joules_by_entity[0] - 10000.0).abs() < 1e-8);
    assert!((result.kinetic_energy_change_joules - 5000.0).abs() < 1e-8);
}

#[test]
fn coupled_braking_stops_relative_spin_and_preserves_total_angular_momentum() {
    let mut matrix = diagonal_mass(100.0);
    matrix[3][3] = 10.0;
    matrix[10][10] = 2.0;
    matrix[3][10] = 2.0;
    matrix[10][3] = 2.0;
    let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    velocity[10] = 10.0;
    let result = apply_generalized_wheel_braking(&matrix, &mut velocity, [100.0, 0.0, 0.0, 0.0], 1.0, 100, 1e-10).unwrap();
    assert!(velocity[10].abs() < 1e-10);
    assert!((velocity[3] - 2.0).abs() < 1e-10);
    assert!((matrix[3][3] * velocity[3] + matrix[3][10] * velocity[10] - 20.0).abs() < 1e-10);
    assert!((result.dissipated_energy_joules - 80.0).abs() < 1e-10);
}

#[test]
fn bounded_braking_does_not_reverse_a_free_wheel() {
    let mut matrix = diagonal_mass(100.0);
    matrix[10][10] = 2.0;
    let mut velocity = [0.0; VEHICLE_GENERALIZED_VELOCITY_COUNT];
    velocity[10] = 10.0;
    let result = apply_generalized_wheel_braking(&matrix, &mut velocity, [1.0, 0.0, 0.0, 0.0], 0.1, 100, 1e-10).unwrap();
    assert!((velocity[10] - 9.95).abs() < 1e-10);
    assert!((result.impulses_newton_metre_seconds[0] + 0.1).abs() < 1e-10);
    assert!(result.dissipated_energy_joules > 0.0);
    assert!((result.dissipated_energy_joules_by_wheel.iter().sum::<f64>() - result.dissipated_energy_joules).abs() < 1e-10);
}

#[test]
fn brake_heat_uses_resolved_work_and_the_actual_event_duration() {
    use vehicle_physics_engine::brake_thermals::{BrakeThermalInput, BrakeThermalSystem};
    let configuration = actual_configuration();
    let mut system = BrakeThermalSystem::new(&configuration.brake_thermal);
    let duration_seconds = 0.00001;
    system.step_after_mechanical_work(WheelIndex::FrontLeft, &configuration.brake_thermal,
        BrakeThermalInput { wheel_spin_pre_rad_s: 50.0, wheel_spin_post_rad_s: 0.0,
            ambient_temperature_c: 25.0, tire_carcass_temperature_c: 25.0,
            tire_gas_temperature_c: 25.0, ..Default::default() }, 12.5, duration_seconds).unwrap();
    assert!((system.wheels[0].brake_energy_j - 12.5).abs() < 1e-12);
    assert!((system.wheels[0].brake_power_w * duration_seconds - 12.5).abs() < 1e-12);
    assert!(system.step_after_mechanical_work(WheelIndex::FrontLeft, &configuration.brake_thermal,
        BrakeThermalInput::default(), -1.0, duration_seconds).is_err());
    assert!((system.wheels[0].brake_energy_j - 12.5).abs() < 1e-12);
}

#[test]
fn input_event_subdivision_does_not_create_fuel_consumption_time() {
    let mut configuration = actual_configuration();
    configuration.fuel.brake_specific_consumption_kg_per_kwh = 0.0;
    configuration.fuel.idle_consumption_kg_per_hour = 3600.0;
    let settings = configuration.coupled_world.clone().unwrap();
    let duration_seconds = 1.0 / settings.physical_frequency_hertz as f64;
    let initial_fuel_kilograms = configuration.fuel.effective_current_kg();
    let terrain = PhysicalCollisionWorld { source_digest: "analytical_empty_world".into(), shapes: Vec::new() };
    let mut world = CoupledVehicleWorld::new(terrain, settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 10.0, 0.0), 0.0).unwrap();
    for fraction in [0.0, 0.01, 0.02, 0.5] {
        world.enqueue_input(identifier, TimestampedVehicleInput { time_seconds: duration_seconds * fraction,
            input: VehicleInput::default(), driving_aids_mask: 0 }).unwrap();
    }
    world.advance_host_interval(duration_seconds).unwrap();
    let fuel_consumed_kilograms = initial_fuel_kilograms - world.vehicles[&identifier].systems.config.fuel.effective_current_kg();
    assert!((fuel_consumed_kilograms - duration_seconds).abs() < 1e-12);
    assert!(world.vehicles[&identifier].systems.pending_coupled_thermal_interval.is_none());
}

#[test]
fn world_brake_thermal_energy_matches_coupled_mechanical_dissipation() {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let duration_seconds = 1.0 / settings.physical_frequency_hertz as f64;
    let terrain = PhysicalCollisionWorld { source_digest: "analytical_empty_world".into(), shapes: Vec::new() };
    let mut world = CoupledVehicleWorld::new(terrain, settings).unwrap();
    let identifier = world.register_vehicle(configuration, Vec3::new(0.0, 10.0, 0.0), 0.0).unwrap();
    world.vehicles.get_mut(&identifier).unwrap().state.wheel_angular_velocity_radians_per_second = [50.0; 4];
    world.enqueue_input(identifier, TimestampedVehicleInput { time_seconds: 0.0,
        input: VehicleInput { brake: 1.0, ..Default::default() }, driving_aids_mask: 0 }).unwrap();
    world.advance_host_interval(duration_seconds * 4.0).unwrap();
    let vehicle = &world.vehicles[&identifier];
    let thermal_energy_joules = vehicle.systems.state.brake_thermal.wheels.iter().map(|wheel| wheel.brake_energy_j).sum::<f64>();
    assert!(thermal_energy_joules > 0.0);
    assert!((thermal_energy_joules - vehicle.state.brake_dissipated_energy_joules).abs() < 1e-8);
}

#[test]
fn uncertainty_variants_preserve_every_budget_and_physical_rotor_inertia() {
    let configuration = actual_configuration();
    let inventory = configuration.physical_mass_inventory.as_ref().unwrap();
    let directions = (0..inventory.components.len()).map(|index| if index % 3 == 0 { 1.0 } else { -0.5 }).collect::<Vec<_>>();
    let variant = inventory.budget_constrained_mass_variant(&configuration, &directions).unwrap();
    let audit = variant.audit();
    assert!((audit.base_mass_kilograms - 600.0).abs() < 1e-10);
    assert!((audit.complete_unfueled_mass_kilograms - 698.0).abs() < 1e-10);
    assert!(variant.components.iter().zip(&inventory.components).any(|(variant, original)|
        (variant.properties.mass_kilograms - original.properties.mass_kilograms).abs() > 0.1));
    for (variant, original) in variant.components.iter().zip(&inventory.components) {
        let mass_ratio = variant.properties.mass_kilograms / original.properties.mass_kilograms;
        assert!(variant.properties.mass_kilograms >= original.uncertainty.lower_mass_kilograms);
        assert!(variant.properties.mass_kilograms <= original.uncertainty.upper_mass_kilograms);
        for (variant_entry, original_entry) in variant.properties.inertia_about_center_of_mass.entries_kilogram_square_metres.iter().flatten()
            .zip(original.properties.inertia_about_center_of_mass.entries_kilogram_square_metres.iter().flatten()) {
            assert!((variant_entry - original_entry * mass_ratio).abs() < 1e-10);
        }
    }
    let mut variant_configuration = configuration.clone();
    variant_configuration.physical_mass_inventory = Some(variant);
    let model = CoupledVehicleModel::new(variant_configuration, configuration.coupled_world.clone().unwrap().suspension).unwrap();
    let evaluation = model.evaluate(&CoupledVehicleState::at_rest(Vec3::ZERO), &CoupledSuspensionInput::default(), [0.0; 4]).unwrap();
    factor_generalized_vehicle_matrix(&evaluation.mass_matrix).unwrap();
    assert!(inventory.budget_constrained_mass_variant(&configuration.clone(), &[0.0]).is_err());
}

#[test]
fn shared_geometry_cache_preserves_exact_solutions_and_rejects_changed_hardpoints() {
    use vehicle_physics_engine::suspension_component_kinematics::{SharedSuspensionGeometrySolutions, SuspensionGeometryEvaluationCache};
    let configuration = actual_configuration();
    let shared = SharedSuspensionGeometrySolutions::new(&configuration).unwrap();
    for travel_metres in [-0.01, 0.0, 0.01] {
        for steering_rack_metres in [-0.005, 0.0, 0.005] {
            for wheel in WheelIndex::ALL {
                let expected = SuspensionGeometryEvaluationCache::new(&configuration).solution(wheel, travel_metres, steering_rack_metres).unwrap();
                let mut cached = SuspensionGeometryEvaluationCache::with_shared_solutions(&configuration, &shared).unwrap();
                assert_eq!(cached.solution(wheel, travel_metres, steering_rack_metres).unwrap(), expected);
                let mut second = SuspensionGeometryEvaluationCache::with_shared_solutions(&configuration, &shared).unwrap();
                assert_eq!(second.solution(wheel, travel_metres, steering_rack_metres).unwrap(), expected);
            }
        }
    }
    let mut changed = configuration.clone();
    changed.geometric_suspension.as_mut().unwrap().corners.fl.damper_chassis.y += 0.001;
    assert!(SuspensionGeometryEvaluationCache::with_shared_solutions(&changed, &shared).is_err());
}
