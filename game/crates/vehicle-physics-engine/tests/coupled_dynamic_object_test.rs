use parry3d_f64::na::Point3;
use parry3d_f64::shape::{SharedShape, TriMesh};
use vehicle_physics_engine::coupled_vehicle_world::*;
use vehicle_physics_engine::dynamic_world_objects::*;
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldShape};
use vehicle_physics_engine::types::{Quat, Vec3};
use vehicle_physics_engine::VehicleConfig;

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn empty_world() -> PhysicalCollisionWorld {
    PhysicalCollisionWorld {
        source_digest: "analytical_empty_world".into(),
        shapes: Vec::new(),
    }
}

fn flat_floor_world() -> PhysicalCollisionWorld {
    let shape = SharedShape::new(
        TriMesh::new(
            vec![
                Point3::new(-50.0, 0.0, -50.0),
                Point3::new(50.0, 0.0, -50.0),
                Point3::new(50.0, 0.0, 50.0),
                Point3::new(-50.0, 0.0, 50.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
        .unwrap(),
    );
    PhysicalCollisionWorld {
        source_digest: "analytical_flat_floor".into(),
        shapes: vec![PhysicalWorldShape {
            identifier: "flat_floor".into(),
            surface_code: 0,
            driveable: true,
            shape,
            wheel_impact_shape: None,
        }],
    }
}

fn track_marker_definition() -> DynamicWorldObjectDefinition {
    DynamicWorldObjectDefinition {
        schema_version: 1,
        identifier: "team_orange_track_marker".into(),
        object_class: DynamicWorldObjectClass::TrackMarkerCone,
        shape: DynamicWorldObjectShapeDefinition::Cylinder {
            half_height_metres: 0.25,
            radius_metres: 0.15,
        },
        mass_kilograms: 4.0,
        surface_code: 1,
        linear_damping_per_second: 0.0,
        angular_damping_per_second: 0.0,
    }
}

fn portable_barrier_definition() -> DynamicWorldObjectDefinition {
    DynamicWorldObjectDefinition {
        schema_version: 1,
        identifier: "pit_lane_portable_barrier".into(),
        object_class: DynamicWorldObjectClass::PortableBarrier,
        shape: DynamicWorldObjectShapeDefinition::Cuboid {
            half_extents_metres: Vec3::new(0.4, 0.5, 0.15),
        },
        mass_kilograms: 12.0,
        surface_code: 6,
        linear_damping_per_second: 0.05,
        angular_damping_per_second: 0.05,
    }
}

#[test]
fn dynamic_object_lifecycle_uses_the_shared_world_identity_space() {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(empty_world(), settings).unwrap();
    let object_identifier = world
        .register_dynamic_object(
            track_marker_definition(),
            Vec3::new(1.0, 2.0, 3.0),
            Quat::IDENTITY,
        )
        .unwrap();
    let vehicle_identifier = world
        .register_vehicle(configuration, Vec3::new(0.0, 1.0, 0.0), 0.0)
        .unwrap();
    assert_ne!(object_identifier, vehicle_identifier);
    let snapshots = world.dynamic_object_snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].entity_identifier, object_identifier);
    assert_eq!(snapshots[0].definition_identifier, "team_orange_track_marker");
    assert_eq!(
        world.dynamic_objects[&object_identifier].state.position_world_metres,
        Vec3::new(1.0, 2.0, 3.0)
    );
    {
        let object = world.dynamic_objects.get_mut(&object_identifier).unwrap();
        object.state.position_world_metres = Vec3::new(9.0, 9.0, 9.0);
        object.state.linear_velocity_world_metres_per_second = Vec3::new(4.0, 5.0, 6.0);
        object.state.angular_velocity_world_radians_per_second = Vec3::new(1.0, 2.0, 3.0);
    }
    world.reset_dynamic_object(object_identifier).unwrap();
    let reset = world.dynamic_objects[&object_identifier].state.clone();
    assert_eq!(reset.position_world_metres, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(reset.linear_velocity_world_metres_per_second, Vec3::ZERO);
    assert_eq!(reset.angular_velocity_world_radians_per_second, Vec3::ZERO);
    assert_eq!(world.dynamic_object_diagnostics.reset_total, 1);
    world.remove_dynamic_object(object_identifier).unwrap();
    assert!(world.reset_dynamic_object(object_identifier).is_err());
    assert!(world.remove_dynamic_object(object_identifier).is_err());
    assert_eq!(world.dynamic_object_diagnostics.removed_total, 1);
    let replacement = world
        .register_dynamic_object(
            track_marker_definition(),
            Vec3::new(0.0, 2.0, 0.0),
            Quat::IDENTITY,
        )
        .unwrap();
    assert_ne!(replacement, object_identifier);
    assert_ne!(replacement, vehicle_identifier);
}

#[test]
fn legacy_godot_owned_vehicle_interaction_is_explicitly_disabled() {
    assert!(DynamicWorldObjectClass::TrackMarkerCone.uses_shared_rust_dynamics());
    assert!(DynamicWorldObjectClass::PortableBarrier.uses_shared_rust_dynamics());
    assert!(!DynamicWorldObjectClass::LegacyGodotOwnedVehicle.uses_shared_rust_dynamics());
    let mut legacy = portable_barrier_definition();
    legacy.object_class = DynamicWorldObjectClass::LegacyGodotOwnedVehicle;
    assert!(legacy.validate().is_err());
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.clone().unwrap();
    let mut world = CoupledVehicleWorld::new(empty_world(), settings).unwrap();
    assert!(world
        .register_dynamic_object(legacy, Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY)
        .is_err());
    assert_eq!(
        world.dynamic_object_diagnostics.rejected_unsupported_class_total,
        1
    );
    let unsupported_document = r#"{
        "schema_version": 1,
        "identifier": "animated_godot_body",
        "object_class": "animated_godot_rigid_body",
        "shape": {"kind": "sphere", "radius_metres": 0.2},
        "mass_kilograms": 2.0,
        "surface_code": 0
    }"#;
    assert!(serde_json::from_str::<DynamicWorldObjectDefinition>(unsupported_document).is_err());
    let supported_document = serde_json::to_string(&track_marker_definition()).unwrap();
    let supported = serde_json::from_str::<DynamicWorldObjectDefinition>(&supported_document).unwrap();
    supported.validate().unwrap();
}

#[test]
fn falling_dynamic_object_receives_ground_impulse_without_tunneling() {
    let configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 0.8;
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let identifier = world
        .register_dynamic_object(
            track_marker_definition(),
            Vec3::new(0.0, 1.0, 0.0),
            Quat::IDENTITY,
        )
        .unwrap();
    let mut minimum_height = f64::INFINITY;
    let mut observed_ground_event = false;
    for _ in 0..240 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        minimum_height = minimum_height.min(
            world.dynamic_objects[&identifier]
                .state
                .position_world_metres
                .y,
        );
        observed_ground_event |= world
            .collision_events
            .iter()
            .any(|event| event.first_entity_identifier == identifier);
    }
    let state = &world.dynamic_objects[&identifier].state;
    assert!(minimum_height > 0.15, "object tunneled through the floor: {minimum_height}");
    assert!(
        state.position_world_metres.y < 0.45,
        "object did not settle near the floor: {}",
        state.position_world_metres.y
    );
    assert!(
        state.linear_velocity_world_metres_per_second.length() < 0.5,
        "object retained excessive speed: {:?}",
        state.linear_velocity_world_metres_per_second
    );
    assert!(state.collision_energy_change_joules <= 1e-9);
    assert!(observed_ground_event, "ground contact must publish an event");
    assert!(state.sleeping, "settled object must enter the sleep state");
    assert_eq!(world.dynamic_object_diagnostics.sleeping_object_count, 1);
    assert!(world.dynamic_object_diagnostics.sleep_transition_total >= 1);
    let tracked_manifolds = world
        .contact_manifolds
        .keys()
        .filter(|key| key.first_entity_identifier == identifier || key.second_entity_identifier == Some(identifier))
        .count();
    assert!(tracked_manifolds > 0, "ground contact must leave a persistent manifold");
    world.reset_dynamic_object(identifier).unwrap();
    assert!(world.contact_manifolds.keys().all(|key| {
        key.first_entity_identifier != identifier && key.second_entity_identifier != Some(identifier)
    }));
    assert!(world.contact_manifold_diagnostics.removed_reset_total > 0);
}

#[test]
fn vehicle_impulse_pushes_dynamic_object_through_the_single_world_solver() {
    let mut configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.body_collision_boxes = settings
        .body_collision_boxes
        .iter()
        .filter(|shape| shape.identifier == "front_wing")
        .cloned()
        .collect();
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 1e-6;
    configuration.coupled_world = Some(settings.clone());
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let vehicle_identifier = world
        .register_vehicle(configuration, Vec3::new(0.0, 0.35, 0.0), 0.0)
        .unwrap();
    let object_identifier = world
        .register_dynamic_object(
            track_marker_definition(),
            Vec3::new(0.0, 0.25, -2.6),
            Quat::IDENTITY,
        )
        .unwrap();
    world
        .vehicles
        .get_mut(&vehicle_identifier)
        .unwrap()
        .state
        .suspension
        .generalized_velocity[2] = -10.0;
    let mut observed_object_velocity: f64 = 0.0;
    let mut observed_pair_event = false;
    let mut observed_island_entity_count = 0;
    for _ in 0..24 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        observed_object_velocity = observed_object_velocity.min(
            world.dynamic_objects[&object_identifier]
                .state
                .linear_velocity_world_metres_per_second
                .z,
        );
        observed_pair_event |= world.collision_events.iter().any(|event| {
            event.first_entity_identifier == object_identifier
                || event.second_entity_identifier == Some(object_identifier)
        });
        observed_island_entity_count = observed_island_entity_count
            .max(world.contact_island_diagnostics.last_largest_island_entity_count);
    }
    let object_state = &world.dynamic_objects[&object_identifier].state;
    assert!(
        object_state.position_world_metres.z < -2.6,
        "object was not pushed forward: {}",
        object_state.position_world_metres.z
    );
    assert!(
        observed_object_velocity < -0.5,
        "object never received a forward impulse: {observed_object_velocity}"
    );
    assert!(observed_pair_event, "vehicle-object contact must publish an event");
    assert!(
        observed_island_entity_count >= 2,
        "vehicle-object contact must form a shared island: {observed_island_entity_count}"
    );
    let vehicle_velocity = world.vehicles[&vehicle_identifier]
        .state
        .suspension
        .generalized_velocity[2];
    assert!(
        vehicle_velocity > -10.0,
        "vehicle must lose forward velocity to the object: {vehicle_velocity}"
    );
}

#[test]
fn two_dynamic_objects_exchange_momentum_inside_the_same_world_solver() {
    let configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 1e-6;
    let mut world = CoupledVehicleWorld::new(empty_world(), settings).unwrap();
    let first = world
        .register_dynamic_object(
            track_marker_definition(),
            Vec3::new(0.0, 2.0, -1.0),
            Quat::IDENTITY,
        )
        .unwrap();
    let second = world
        .register_dynamic_object(
            portable_barrier_definition(),
            Vec3::new(0.0, 2.0, 0.2),
            Quat::IDENTITY,
        )
        .unwrap();
    world.dynamic_objects.get_mut(&first).unwrap().state.linear_velocity_world_metres_per_second =
        Vec3::new(0.0, 0.0, 6.0);
    let mut observed_pair_contact = false;
    let mut observed_island_entity_count = 0;
    for _ in 0..48 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        observed_pair_contact |= world.collision_events.iter().any(|event| {
            (event.first_entity_identifier == first && event.second_entity_identifier == Some(second))
                || (event.first_entity_identifier == second && event.second_entity_identifier == Some(first))
        });
        observed_island_entity_count = observed_island_entity_count
            .max(world.contact_island_diagnostics.last_largest_island_entity_count);
    }
    let first_velocity = world.dynamic_objects[&first]
        .state
        .linear_velocity_world_metres_per_second
        .z;
    let second_velocity = world.dynamic_objects[&second]
        .state
        .linear_velocity_world_metres_per_second
        .z;
    assert!(observed_pair_contact, "object-object contact must publish a pair event");
    assert!(
        observed_island_entity_count >= 2,
        "object-object contact must form a shared island: {observed_island_entity_count}"
    );
    assert!(second_velocity > 0.5, "second object must receive forward momentum: {second_velocity}");
    assert!(
        (4.0 * first_velocity + 12.0 * second_velocity - 24.0).abs() < 0.5,
        "object-object normal momentum must close without an external z force"
    );
}

#[test]
fn native_interface_accepts_a_dynamic_object_registration_document() {
    use vehicle_physics_engine::coupled_vehicle_world_interface::CoupledVehicleWorldRequest;
    let document = r#"{
        "operation": "register_dynamic_object",
        "world_identifier": 1,
        "definition": {
            "schema_version": 1,
            "identifier": "team_orange_track_marker",
            "object_class": "track_marker_cone",
            "shape": {"kind": "cylinder", "half_height_metres": 0.25, "radius_metres": 0.15},
            "mass_kilograms": 4.0,
            "surface_code": 1
        },
        "position_world_metres": {"x": 0.0, "y": 0.25, "z": -2.0},
        "orientation": {"x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0}
    }"#;
    let request: CoupledVehicleWorldRequest = serde_json::from_str(document).unwrap();
    assert!(matches!(
        request,
        CoupledVehicleWorldRequest::RegisterDynamicObject { .. }
    ));
}

#[test]
fn dynamic_object_mass_matrix_is_a_free_rigid_body_with_positive_contact_mass() {
    let object = DynamicWorldObject::new(
        track_marker_definition(),
        Vec3::new(0.0, 2.0, 0.0),
        Quat::IDENTITY,
    )
    .unwrap();
    let matrix = object.mass_matrix().unwrap();
    assert!((matrix[0][0] - 4.0).abs() < 1e-12);
    assert!((matrix[1][1] - 4.0).abs() < 1e-12);
    assert!((matrix[2][2] - 4.0).abs() < 1e-12);
    let transverse = 4.0 * (3.0 * 0.15_f64.powi(2) + 4.0 * 0.25_f64.powi(2)) / 12.0;
    assert!((matrix[3][3] - transverse).abs() < 1e-12);
    assert!((matrix[4][4] - 0.5 * 4.0 * 0.15_f64.powi(2)).abs() < 1e-12);
    assert!((matrix[5][5] - transverse).abs() < 1e-12);
    for coordinate in 6..14 {
        assert!((matrix[coordinate][coordinate] - 1.0).abs() < 1e-12);
    }
    let jacobian = object.contact_jacobian(Vec3::new(0.0, 0.0, 0.1), Vec3::UP);
    let response = vehicle_physics_engine::coupled_vehicle_dynamics::solve_factored_vehicle_matrix(
        &vehicle_physics_engine::coupled_vehicle_dynamics::factor_generalized_vehicle_matrix(&matrix)
            .unwrap(),
        jacobian,
    )
    .unwrap();
    let effective_mass =
        vehicle_physics_engine::coupled_vehicle_dynamics::generalized_projection(jacobian, response);
    assert!(effective_mass > 0.0);
    assert!(effective_mass < 1.0);
}
