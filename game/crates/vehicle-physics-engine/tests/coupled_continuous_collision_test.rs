use vehicle_physics_engine::coupled_vehicle_world::{
    ContactResponseOwner, CoupledVehicleWorld, CoupledVehicleWorldConfiguration,
};
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldShape};
use vehicle_physics_engine::{Mat3, Quat, Vec3, VehicleConfig};
use parry3d_f64::na::Point3;
use parry3d_f64::shape::{SharedShape, TriMesh};

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn world_configuration() -> CoupledVehicleWorldConfiguration {
    actual_configuration().coupled_world.unwrap()
}

fn box_shape(identifier: &str, role: &str, surface_code: u32, center: Vec3, half_extents: Vec3) -> PhysicalWorldShape {
    let mut vertices = Vec::new();
    for z_sign in [-1.0, 1.0] {
        for y_sign in [-1.0, 1.0] {
            for x_sign in [-1.0, 1.0] {
                vertices.push([center.x + x_sign * half_extents.x, center.y + y_sign * half_extents.y,
                    center.z + z_sign * half_extents.z]);
            }
        }
    }
    let triangles = vec![
        [0, 2, 1], [0, 3, 2], [4, 5, 6], [4, 6, 7],
        [0, 1, 5], [0, 5, 4], [1, 2, 6], [1, 6, 5],
        [2, 3, 7], [2, 7, 6], [3, 0, 4], [3, 4, 7],
    ];
    let points = vertices.iter().map(|position| Point3::new(position[0], position[1], position[2])).collect();
    PhysicalWorldShape { identifier: identifier.to_string(), surface_code, driveable: role == "driveable",
        shape: SharedShape::new(TriMesh::new(points, triangles).unwrap()), wheel_impact_shape: None }
}

fn floor_and_thin_wall_world() -> PhysicalCollisionWorld {
    let floor = PhysicalWorldShape {
        identifier: "high_speed_floor".into(), surface_code: 0, driveable: true,
        shape: SharedShape::new(TriMesh::new(
            vec![Point3::new(-50.0, 0.0, -50.0), Point3::new(50.0, 0.0, -50.0),
                Point3::new(50.0, 0.0, 50.0), Point3::new(-50.0, 0.0, 50.0)],
            vec![[0, 1, 2], [0, 2, 3]]).unwrap()),
        wheel_impact_shape: None };
    let wall = box_shape("thin_wall", "solid", 6, Vec3::new(0.0, 1.5, -3.0), Vec3::new(5.0, 1.5, 0.025));
    PhysicalCollisionWorld { source_digest: "analytical_thin_wall".into(), shapes: vec![floor, wall] }
}

#[test]
fn high_speed_body_does_not_tunnel_through_a_thin_wall() {
    let mut world = CoupledVehicleWorld::new(floor_and_thin_wall_world(), world_configuration()).unwrap();
    let identifier = world.register_vehicle(actual_configuration(), Vec3::new(0.0, 0.35, 0.0), 0.0).unwrap();
    {
        let vehicle = world.vehicles.get_mut(&identifier).unwrap();
        vehicle.state.suspension.generalized_velocity[2] = -50.0;
        vehicle.state.suspension.wheel_travel_metres[0] = 0.02;
        vehicle.state.suspension.generalized_velocity[6] = 0.5;
    }
    for _ in 0..8 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
    }
    let diagnostics = &world.continuous_collision_diagnostics;
    assert!(diagnostics.total_impact_events > 0, "the high-speed wall impact was not localized");
    assert!(diagnostics.last_impact_time_seconds.is_some());
    assert!(diagnostics.maximum_event_subdivisions >= 1);
    let body_origin = world.vehicles[&identifier].state.suspension.body_origin_world_metres;
    assert!(body_origin.z - 2.0 > -3.05, "body tunneled through the thin wall to {}", body_origin.z);
    assert!(world.contact_classification_diagnostics.tire_tread_supports > 0);
    assert!(world.contact_classification_diagnostics.rigid_impact_contacts > 0);
    assert!(world.collision_events.iter().all(|event| event.response_owner == ContactResponseOwner::RigidImpact));
}

#[test]
fn rotating_body_near_a_wall_is_localized_without_tunneling() {
    let mut world = CoupledVehicleWorld::new(floor_and_thin_wall_world(), world_configuration()).unwrap();
    let identifier = world.register_vehicle(actual_configuration(), Vec3::new(0.0, 0.35, 0.0), 0.0).unwrap();
    {
        let vehicle = world.vehicles.get_mut(&identifier).unwrap();
        vehicle.state.suspension.generalized_velocity[2] = -20.0;
        vehicle.state.suspension.generalized_velocity[5] = 5.0;
        vehicle.state.suspension.body_orientation_world =
            Quat::from_mat3(&Mat3::from_axis_angle(Vec3::UP, 0.35));
    }
    for _ in 0..8 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
    }
    assert!(world.continuous_collision_diagnostics.total_impact_events > 0);
    assert!(world.continuous_collision_diagnostics.last_impact_shape_identifier.is_some());
    let body_origin = world.vehicles[&identifier].state.suspension.body_origin_world_metres;
    assert!(body_origin.z > -3.0, "rotating body crossed the wall plane to {}", body_origin.z);
}
