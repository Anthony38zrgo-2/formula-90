use vehicle_physics_engine::coupled_vehicle_world::{CoupledVehicleWorld, CoupledVehicleWorldConfiguration};
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldShape};
use vehicle_physics_engine::{Vec3, VehicleConfig};
use parry3d_f64::na::Point3;
use parry3d_f64::shape::{SharedShape, TriMesh};

const ISOLATED_IMPACT_FRICTION: f64 = 1e-6;

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn flat_floor_world() -> PhysicalCollisionWorld {
    let shape = SharedShape::new(TriMesh::new(
        vec![Point3::new(-50.0, 0.0, -50.0), Point3::new(50.0, 0.0, -50.0),
            Point3::new(50.0, 0.0, 50.0), Point3::new(-50.0, 0.0, 50.0)],
        vec![[0, 1, 2], [0, 2, 3]]).unwrap());
    PhysicalCollisionWorld { source_digest: "analytical_flat_floor".into(), shapes: vec![PhysicalWorldShape {
        identifier: "flat_floor".into(), surface_code: 0, driveable: true, shape, wheel_impact_shape: None }] }
}

fn two_vehicle_configuration() -> (VehicleConfig, CoupledVehicleWorldConfiguration) {
    let mut configuration = actual_configuration();
    let mut world_configuration = configuration.coupled_world.clone().unwrap();
    let floor_box = world_configuration.body_collision_boxes.iter()
        .find(|shape| shape.identifier == "floor").expect("floor collision box").clone();
    world_configuration.body_collision_boxes = vec![floor_box];
    configuration.coupled_world = Some(world_configuration.clone());
    (configuration, world_configuration)
}

fn step_isolated_two_vehicle_impact(first_x: f64, first_velocity_x: f64, second_x: f64, second_velocity_x: f64) -> ([f64; 2], CoupledVehicleWorld, u64, u64) {
    let (configuration, world_configuration) = two_vehicle_configuration();
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), world_configuration).unwrap();
    let first = world.register_vehicle(configuration.clone(), Vec3::new(first_x, 0.35, 0.0), 0.0).unwrap();
    let second = world.register_vehicle(configuration, Vec3::new(second_x, 0.35, 0.0), 0.0).unwrap();
    world.vehicles.get_mut(&first).unwrap().state.suspension.generalized_velocity[0] = first_velocity_x;
    world.vehicles.get_mut(&second).unwrap().state.suspension.generalized_velocity[0] = second_velocity_x;
    world.configuration.contacts.restitution = 0.0;
    world.configuration.contacts.impact_friction_coefficient = ISOLATED_IMPACT_FRICTION;
    world.advance_host_interval(1.0 / 120.0).unwrap();
    let velocities = [
        world.vehicles[&first].state.suspension.generalized_velocity[0],
        world.vehicles[&second].state.suspension.generalized_velocity[0],
    ];
    (velocities, world, first, second)
}

#[test]
fn two_vehicle_contact_is_solved_in_one_world_step_and_reports_the_island() {
    let (velocities, world, first, second) = step_isolated_two_vehicle_impact(-0.64, 2.0, 0.64, -2.0);
    assert!((velocities[0] + velocities[1]).abs() < 0.05, "equal masses must conserve x momentum apart from ground/tire external forces");
    assert!(velocities[0] < 2.0);
    assert!(velocities[1] > -2.0);
    let pair_event = world.collision_events.iter()
        .find(|event| event.second_entity_identifier.is_some())
        .expect("the two-vehicle impact must publish a pair event");
    assert_eq!(pair_event.first_entity_identifier, first);
    assert_eq!(pair_event.second_entity_identifier, Some(second));
    let diagnostics = &world.contact_island_diagnostics;
    assert_eq!(diagnostics.last_largest_island_entity_count, 2);
    assert!(diagnostics.last_largest_island_constraint_count > 0);
    assert!(diagnostics.last_constraint_count > 0);
    assert!(diagnostics.last_solver_iterations > 0);
}

#[test]
fn entity_registration_mirror_is_deterministic() {
    let (left_first, _, _, _) = step_isolated_two_vehicle_impact(-0.64, 2.0, 0.64, -2.0);
    let (right_first, _, _, _) = step_isolated_two_vehicle_impact(0.64, -2.0, -0.64, 2.0);
    assert!((left_first[0] - right_first[1]).abs() < 1e-5, "mirrored ids must receive mirrored velocities");
    assert!((left_first[1] - right_first[0]).abs() < 1e-5, "mirrored ids must receive mirrored velocities");
}
