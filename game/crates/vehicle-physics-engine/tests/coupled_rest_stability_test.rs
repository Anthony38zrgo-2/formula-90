use parry3d_f64::na::Point3;
use parry3d_f64::shape::{SharedShape, TriMesh};
use vehicle_physics_engine::coupled_vehicle_world::*;
use vehicle_physics_engine::dynamic_world_objects::*;
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldShape};
use vehicle_physics_engine::types::{Mat3, Quat, Vec3};
use vehicle_physics_engine::VehicleConfig;

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn subdivided_plane_world(identifier: &str, half_extent_metres: f64, divisions: usize,
    slope_angle_radians: f64, role: &str, surface_code: u32) -> PhysicalCollisionWorld {
    subdivided_plane_world_at_height(identifier, half_extent_metres, divisions, slope_angle_radians,
        0.0, role, surface_code)
}

fn subdivided_plane_world_at_height(identifier: &str, half_extent_metres: f64, divisions: usize,
    slope_angle_radians: f64, height_offset_metres: f64, role: &str,
    surface_code: u32) -> PhysicalCollisionWorld {
    let columns = divisions + 1;
    let slope = slope_angle_radians.tan();
    let mut vertices = Vec::new();
    for row in 0..=divisions {
        for column in 0..=divisions {
            let x = -half_extent_metres + 2.0 * half_extent_metres * column as f64 / divisions as f64;
            let z = -half_extent_metres + 2.0 * half_extent_metres * row as f64 / divisions as f64;
            vertices.push(Point3::new(x, height_offset_metres + x * slope, z));
        }
    }
    let mut triangles = Vec::new();
    for row in 0..divisions {
        for column in 0..divisions {
            let first = row * columns + column;
            let second = first + 1;
            let third = first + columns + 1;
            let fourth = first + columns;
            triangles.push([first as u32, third as u32, second as u32]);
            triangles.push([first as u32, fourth as u32, third as u32]);
        }
    }
    let shape = SharedShape::new(TriMesh::new(vertices, triangles).unwrap());
    PhysicalCollisionWorld {
        source_digest: format!("analytical_subdivided_plane_{identifier}"),
        shapes: vec![PhysicalWorldShape {
            identifier: identifier.into(),
            surface_code,
            driveable: role == "driveable",
            shape,
            wheel_impact_shape: None,
        }],
    }
}

fn flat_floor_world() -> PhysicalCollisionWorld {
    subdivided_plane_world("flat_floor", 50.0, 1, 0.0, "driveable", 0)
}

fn barrier_definition() -> DynamicWorldObjectDefinition {
    DynamicWorldObjectDefinition {
        schema_version: 1,
        identifier: "pit_lane_portable_barrier".into(),
        object_class: DynamicWorldObjectClass::PortableBarrier,
        shape: DynamicWorldObjectShapeDefinition::Cuboid {
            half_extents_metres: Vec3::new(0.4, 0.5, 0.15),
        },
        mass_kilograms: 12.0,
        surface_code: 6,
        linear_damping_per_second: 0.0,
        angular_damping_per_second: 0.0,
    }
}

fn marker_definition() -> DynamicWorldObjectDefinition {
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

fn raised_solid_support_world() -> PhysicalCollisionWorld {
    let mut world = subdivided_plane_world("flat_floor", 50.0, 1, 0.0, "driveable", 0);
    world.shapes.push(PhysicalWorldShape {
        identifier: "raised_support".into(),
        surface_code: 6,
        driveable: false,
        shape: SharedShape::cuboid(0.5, 0.1, 1.5),
        wheel_impact_shape: None,
    });
    world.source_digest = "analytical_raised_solid_support".into();
    world
}

#[test]
fn dynamic_barrier_holds_static_friction_on_a_slope() {
    let angle = 15.0_f64.to_radians();
    let normal = Vec3::new(-angle.sin(), angle.cos(), 0.0);
    let spawn = normal * 0.51;
    let orientation = Quat::from_mat3(&Mat3::from_axis_angle(Vec3::BACK, angle));
    let run = |friction: f64| {
        let mut configuration = actual_configuration();
        let mut settings = configuration.coupled_world.clone().unwrap();
        settings.contacts.restitution = 0.0;
        settings.contacts.impact_friction_coefficient = friction;
        configuration.coupled_world = Some(settings.clone());
        let mut world = CoupledVehicleWorld::new(
            subdivided_plane_world("inclined_plane", 5.0, 8, angle, "solid", 0),
            settings,
        )
        .unwrap();
        let identifier = world
            .register_dynamic_object(barrier_definition(), spawn, orientation)
            .unwrap();
        let mut maximum_static_friction_contacts = 0;
        for _ in 0..60 {
            world.advance_host_interval(1.0 / 120.0).unwrap();
            maximum_static_friction_contacts = maximum_static_friction_contacts.max(
                world.contact_island_diagnostics.last_static_friction_constraint_count);
        }
        let settled = world.dynamic_objects[&identifier].state.position_world_metres;
        for _ in 0..180 {
            world.advance_host_interval(1.0 / 120.0).unwrap();
            world.dynamic_objects.get_mut(&identifier).unwrap().wake();
            maximum_static_friction_contacts = maximum_static_friction_contacts.max(
                world.contact_island_diagnostics.last_static_friction_constraint_count);
        }
        let finished = world.dynamic_objects[&identifier].state.position_world_metres;
        ((finished - settled).length(), world, identifier, maximum_static_friction_contacts)
    };
    let (held_displacement, held_world, _held_identifier, held_static_friction_contacts) = run(1.0);
    assert!(
        held_displacement < 0.01,
        "static friction did not hold the barrier: {held_displacement}"
    );
    assert!(held_world.contact_manifold_diagnostics.resting_contact_count > 0);
    assert!(held_world.contact_manifold_diagnostics.full_warm_started_total > 0);
    assert!(
        held_static_friction_contacts > 0,
        "static friction must be reported while the barrier is held"
    );
    assert!(
        held_world.contact_manifold_diagnostics.tangential_position_correction_total > 0,
        "static friction must correct the pre-solve tangential drift"
    );
    let (sliding_displacement, sliding_world, _, _) = run(0.05);
    assert!(
        sliding_displacement > held_displacement + 0.05,
        "low friction must slide farther than high friction: {sliding_displacement}"
    );
    assert!(sliding_world.contact_island_diagnostics.last_sliding_constraint_count > 0);
}

#[test]
fn dynamic_object_sleeps_at_rest_and_wakes_on_impact() {
    let mut configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 1e-6;
    configuration.coupled_world = Some(settings.clone());
    let mut world = CoupledVehicleWorld::new(flat_floor_world(), settings).unwrap();
    let sleeper = world
        .register_dynamic_object(marker_definition(), Vec3::new(0.0, 0.25, 0.0), Quat::IDENTITY)
        .unwrap();
    for _ in 0..120 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
    }
    assert!(world.dynamic_objects[&sleeper].state.sleeping);
    assert_eq!(world.dynamic_object_diagnostics.sleeping_object_count, 1);
    let sleeping_position = world.dynamic_objects[&sleeper].state.position_world_metres;
    let intruder = world
        .register_dynamic_object(marker_definition(), Vec3::new(0.0, 0.25, -1.5), Quat::IDENTITY)
        .unwrap();
    world.dynamic_objects.get_mut(&intruder).unwrap().state.linear_velocity_world_metres_per_second =
        Vec3::new(0.0, 0.0, 5.0);
    let wake_transitions_before = world.dynamic_object_diagnostics.wake_transition_total;
    for _ in 0..120 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
    }
    assert!(
        world.dynamic_object_diagnostics.wake_transition_total > wake_transitions_before,
        "moving contact must wake the sleeping object"
    );
    assert!(!world.dynamic_objects[&sleeper].state.sleeping);
    assert!(
        world.dynamic_objects[&sleeper].state.position_world_metres.z
            > sleeping_position.z + 0.1,
        "woken object was not pushed forward"
    );
}

#[test]
fn mesh_seam_rest_keeps_feature_identity_and_bounded_penetration() {
    let mut configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 1e-6;
    configuration.coupled_world = Some(settings.clone());
    let mut world = CoupledVehicleWorld::new(
        subdivided_plane_world("seam_floor", 4.0, 16, 0.0, "solid", 0),
        settings,
    )
    .unwrap();
    let identifier = world
        .register_dynamic_object(barrier_definition(), Vec3::new(0.0, 0.51, 0.0), Quat::IDENTITY)
        .unwrap();
    for _ in 0..240 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
    }
    let state = &world.dynamic_objects[&identifier].state;
    assert!(state.sleeping);
    assert!((state.position_world_metres.y - 0.5).abs() < 0.01,
        "seam rest penetrated or floated: {}", state.position_world_metres.y);
    assert!(state.position_world_metres.x.abs() < 0.01);
    assert!(state.position_world_metres.z.abs() < 0.01);
    assert!(world.contact_manifold_diagnostics.resting_contact_count > 0);
    assert!(world.contact_manifold_diagnostics.full_warm_started_total > 0);
    assert_eq!(world.contact_manifold_diagnostics.removed_normal_rotation_total, 0);
    let active_features = world
        .contact_manifolds
        .iter()
        .filter(|(_, entry)| entry.last_seen_step == world.physical_step_index)
        .filter(|(key, _)| key.first_shape_identifier == "seam_floor")
        .map(|(key, _)| key.first_feature_identifier)
        .collect::<std::collections::BTreeSet<u32>>();
    assert!(!active_features.is_empty(), "seam rest must retain active features");
    assert!(
        state.position_correction_energy_joules.abs() < 10.0,
        "seam rest correction energy is unbounded: {}",
        state.position_correction_energy_joules
    );
}

#[test]
fn seam_contact_manifolds_retain_features_from_multiple_coplanar_triangles() {
    let floor = subdivided_plane_world("seam_floor", 2.0, 8, 0.0, "solid", 0);
    let floor_shape = floor.shapes[0].shape.clone();
    let box_shape = SharedShape::cuboid(0.4, 0.5, 0.15);
    let box_pose = parry3d_f64::na::Isometry3::translation(0.0, 0.49, 0.0);
    let manifolds = vehicle_physics_engine::physical_collision_world::shape_pair_contact_manifolds(
        &parry3d_f64::na::Isometry3::identity(),
        &floor_shape,
        &box_pose,
        &box_shape,
        0.01,
    )
    .unwrap();
    let features = manifolds
        .iter()
        .flat_map(|manifold| manifold.points.iter())
        .map(|point| point.first_feature_identifier)
        .collect::<std::collections::BTreeSet<u32>>();
    assert!(features.len() >= 2, "seam contact must span more than one triangle: {features:?}");
}

#[test]
fn vehicle_rest_keeps_persistent_contacts_without_freezing_suspension() {
    let mut configuration = actual_configuration();
    let mut settings = configuration.coupled_world.clone().unwrap();
    settings.contacts.restitution = 0.0;
    settings.contacts.impact_friction_coefficient = 0.8;
    let floor_box = settings.body_collision_boxes.iter()
        .find(|shape| shape.identifier == "floor")
        .expect("floor collision box")
        .clone();
    settings.body_collision_boxes = vec![floor_box];
    configuration.coupled_world = Some(settings.clone());
    let mut world = CoupledVehicleWorld::new(raised_solid_support_world(), settings).unwrap();
    let identifier = world
        .register_vehicle(configuration, Vec3::new(0.0, 0.35, 0.0), 0.0)
        .unwrap();
    let mut maximum_resting_contacts = 0;
    let mut maximum_full_warm_starts = 0;
    for _ in 0..240 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
        maximum_resting_contacts = maximum_resting_contacts
            .max(world.contact_manifold_diagnostics.resting_contact_count);
        maximum_full_warm_starts = maximum_full_warm_starts
            .max(world.contact_manifold_diagnostics.full_warm_started_total);
    }
    let settled = world.vehicles[&identifier].state.suspension.body_origin_world_metres;
    for _ in 0..120 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
    }
    let finished = world.vehicles[&identifier].state.suspension.body_origin_world_metres;
    assert!(settled.y.is_finite() && finished.y.is_finite());
    assert!((finished.y - settled.y).abs() < 0.005,
        "resting vehicle drifted vertically: {} to {}", settled.y, finished.y);
    assert!((finished.x - settled.x).abs() < 0.005,
        "resting vehicle drifted laterally in x: {} to {}", settled.x, finished.x);
    assert!((finished.z - settled.z).abs() < 0.005,
        "resting vehicle drifted longitudinally in z: {} to {}", settled.z, finished.z);
    assert!(maximum_resting_contacts > 0);
    assert!(maximum_full_warm_starts > 0);
    assert!(world.contact_manifold_diagnostics.tangential_position_correction_total > 0);
    assert!(world.contact_island_diagnostics.last_static_friction_constraint_count > 0);
    let before = finished;
    world.vehicles.get_mut(&identifier).unwrap().state.suspension.generalized_velocity[1] = 0.2;
    world.advance_host_interval(1.0 / 120.0).unwrap();
    let after = world.vehicles[&identifier].state.suspension.body_origin_world_metres;
    assert!(after.y > before.y, "resting vehicle must not be frozen");
}
