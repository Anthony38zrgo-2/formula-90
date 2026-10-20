use parry3d_f64::na::Point3;
use parry3d_f64::shape::{SharedShape, TriMesh};
use vehicle_physics_engine::coupled_vehicle_world::*;
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldShape};
use vehicle_physics_engine::types::Vec3;
use vehicle_physics_engine::VehicleConfig;

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn flat_floor_world() -> PhysicalCollisionWorld {
    let shape = SharedShape::new(TriMesh::new(
        vec![
            Point3::new(-50.0, 0.0, -50.0),
            Point3::new(50.0, 0.0, -50.0),
            Point3::new(50.0, 0.0, 50.0),
            Point3::new(-50.0, 0.0, 50.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    ).unwrap());
    PhysicalCollisionWorld {
        source_digest: "analytical_mixed_floor".into(),
        shapes: vec![PhysicalWorldShape {
            identifier: "flat_floor".into(),
            surface_code: 0,
            driveable: true,
            shape,
            wheel_impact_shape: None,
        }],
    }
}

fn contact_configuration() -> VehicleConfig {
    let mut configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.body_collision_boxes.retain(|shape| shape.identifier == "monocoque");
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 1e-6;
    configuration.coupled_world = Some(settings);
    configuration
}

#[test]
fn mixed_world_advances_player_every_tick_and_ai_every_second_tick() {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(PhysicalCollisionWorld {
        source_digest: "analytical_empty_mixed_world".into(),
        shapes: Vec::new(),
    }, settings).unwrap();
    let player = world.register_vehicle_with_frequency(configuration.clone(),
        Vec3::new(0.0, 10.0, 0.0), 0.0, 120).unwrap();
    let ai = world.register_vehicle_with_frequency(configuration,
        Vec3::new(10.0, 10.0, 0.0), 0.0, 60).unwrap();
    assert_eq!(world.vehicles[&player].physical_frequency_hertz, 120);
    assert_eq!(world.vehicles[&ai].physical_frequency_hertz, 60);
    world.advance_host_interval(1.0 / 120.0).unwrap();
    let player_velocity_after_first = world.vehicles[&player].state.suspension.generalized_velocity[1];
    let ai_velocity_after_first = world.vehicles[&ai].state.suspension.generalized_velocity[1];
    assert!(player_velocity_after_first < 0.0);
    assert!(ai_velocity_after_first < 0.0, "AI must advance on its 60 Hz update tick");
    world.advance_host_interval(1.0 / 120.0).unwrap();
    let ai_velocity_after_second = world.vehicles[&ai].state.suspension.generalized_velocity[1];
    assert_eq!(ai_velocity_after_second, ai_velocity_after_first,
        "AI must be held on the non-update tick");
    let snapshots = world.snapshots().unwrap();
    assert_eq!(snapshots.iter().find(|snapshot| snapshot.entity_identifier == player)
        .unwrap().physical_frequency_hertz, 120);
    assert_eq!(snapshots.iter().find(|snapshot| snapshot.entity_identifier == ai)
        .unwrap().physical_frequency_hertz, 60);
}

#[test]
fn player_ai_contact_does_not_change_held_ai_velocity() {
    let configuration = contact_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let player = world.register_vehicle_with_frequency(configuration.clone(),
        Vec3::new(0.0, 0.35, 0.0), 0.0, 120).unwrap();
    let ai = world.register_vehicle_with_frequency(configuration,
        Vec3::new(0.0, 0.35, -1.61), 0.0, 60).unwrap();
    world.advance_host_interval(1.0 / 120.0).unwrap();
    let ai_state_after_update = world.vehicles[&ai].state.clone();
    let player_velocity_after_update = world.vehicles[&player].state.suspension.generalized_velocity;
    world.advance_host_interval(1.0 / 120.0).unwrap();
    let ai_state_after_held_tick = world.vehicles[&ai].state.clone();
    assert_eq!(ai_state_after_held_tick.suspension.body_origin_world_metres,
        ai_state_after_update.suspension.body_origin_world_metres);
    assert_eq!(ai_state_after_held_tick.suspension.generalized_velocity,
        ai_state_after_update.suspension.generalized_velocity);
    assert_ne!(world.vehicles[&player].state.suspension.generalized_velocity,
        player_velocity_after_update, "player contact must still respond at 120 Hz");
}

#[test]
fn ai_ai_contact_is_solved_only_on_the_ai_phase() {
    let configuration = contact_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let first = world.register_vehicle_with_frequency(configuration.clone(),
        Vec3::new(0.0, 0.35, 0.0), 0.0, 60).unwrap();
    let second = world.register_vehicle_with_frequency(configuration,
        Vec3::new(0.0, 0.35, -1.605), 0.0, 60).unwrap();
    world.vehicles.get_mut(&first).unwrap().state.suspension.generalized_velocity[2] = -1.0;
    world.vehicles.get_mut(&second).unwrap().state.suspension.generalized_velocity[2] = 1.0;
    world.advance_host_interval(1.0 / 120.0).unwrap();
    assert!(world.collision_events.iter().any(|event| {
        (event.first_entity_identifier == first && event.second_entity_identifier == Some(second))
            || (event.first_entity_identifier == second && event.second_entity_identifier == Some(first))
    }), "AI-AI contact must be solved on the AI phase");
    let first_after_ai_phase = world.vehicles[&first].state.clone();
    let second_after_ai_phase = world.vehicles[&second].state.clone();
    world.advance_host_interval(1.0 / 120.0).unwrap();
    assert!(world.collision_events.is_empty(), "AI-AI contact must not rerun on the held tick");
    assert_eq!(world.vehicles[&first].state.suspension.generalized_velocity,
        first_after_ai_phase.suspension.generalized_velocity);
    assert_eq!(world.vehicles[&second].state.suspension.generalized_velocity,
        second_after_ai_phase.suspension.generalized_velocity);
}

#[test]
fn mixed_frequency_registration_rejects_unsupported_rates() {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    assert!(world.register_vehicle_with_frequency(configuration.clone(),
        Vec3::new(0.0, 0.35, 0.0), 0.0, 90).is_err());
    let ai = world.register_vehicle_with_frequency(configuration,
        Vec3::new(0.0, 0.35, 0.0), 0.0, 60).unwrap();
    world.reset_vehicle(ai).unwrap();
    assert!((world.vehicles[&ai].model.settings.maximum_substep_seconds - 1.0 / 60.0).abs() < 1e-12,
        "reset must preserve the AI integration rate");
}

#[test]
fn native_interface_accepts_an_ai_frequency() {
    use vehicle_physics_engine::coupled_vehicle_world_interface::CoupledVehicleWorldRequest;
    let document = r#"{
        "operation": "register_vehicle",
        "world_identifier": 1,
        "vehicle_profile": "{}",
        "position_world_metres": {"x": 0.0, "y": 0.35, "z": 0.0},
        "yaw_radians": 0.0,
        "physical_frequency_hertz": 60
    }"#;
    let request: CoupledVehicleWorldRequest = serde_json::from_str(document).unwrap();
    assert!(matches!(
        request,
        CoupledVehicleWorldRequest::RegisterVehicle { physical_frequency_hertz: Some(60), .. }
    ));
}

#[test]
fn fixed_contact_entity_keeps_its_velocity_while_relative_motion_closes() {
    use vehicle_physics_engine::coupled_vehicle_dynamics::{
        GeneralizedVehicleMatrix, GeneralizedVehicleVelocity,
    };
    use vehicle_physics_engine::generalized_vehicle_contacts::{
        solve_generalized_vehicle_contacts_with_fixed_entities, GeneralizedContactConstraint,
        GeneralizedContactSettings,
    };
    let mut matrix: GeneralizedVehicleMatrix = [[0.0; 14]; 14];
    for coordinate in 0..14 { matrix[coordinate][coordinate] = if coordinate < 3 { 100.0 } else { 1.0 }; }
    let mut first_jacobians = [[0.0; 14]; 3];
    let mut second_jacobians = [[0.0; 14]; 3];
    for axis in 0..3 {
        first_jacobians[axis][axis] = 1.0;
        second_jacobians[axis][axis] = -1.0;
    }
    let constraint = GeneralizedContactConstraint {
        identifier: "mixed_fixed_contact".into(), first_entity: 0, second_entity: Some(1),
        first_jacobians, second_jacobians,
        first_prescribed_velocity_metres_per_second: [0.0; 3],
        second_prescribed_velocity_metres_per_second: [0.0; 3],
        surface_code: 0, normal_impulse_newton_seconds: 0.0,
        tangential_impulse_newton_seconds: [0.0; 2],
    };
    let settings = GeneralizedContactSettings {
        maximum_iterations: 100, velocity_tolerance_metres_per_second: 1e-9,
        restitution: 0.0, restitution_minimum_speed_metres_per_second: 1.0,
        impact_friction_coefficient: 0.5, impact_friction_by_surface_code: std::collections::BTreeMap::new(),
        prediction_distance_metres: 0.001, maximum_position_correction_metres: 0.001,
        maximum_events_per_step: 8, maximum_supported_speed_metres_per_second: 120.0,
        maximum_supported_angular_speed_radians_per_second: 10.0,
    };
    let mut velocities: [GeneralizedVehicleVelocity; 2] = [[0.0; 14]; 2];
    velocities[0][0] = -10.0;
    velocities[1][0] = 2.0;
    let fixed = [false, true];
    let result = solve_generalized_vehicle_contacts_with_fixed_entities(
        &[matrix, matrix], &mut velocities, &mut [constraint], &settings, &fixed).unwrap();
    assert!(velocities[1][0] == 2.0, "fixed entity velocity changed");
    assert!((velocities[0][0] - 2.0).abs() < 1e-9, "relative contact velocity did not close");
    assert!(result.normal_impulses_newton_seconds[0] > 0.0);
}
