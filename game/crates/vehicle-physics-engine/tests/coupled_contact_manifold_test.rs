use vehicle_physics_engine::coupled_vehicle_dynamics::{
    GeneralizedVehicleMatrix, GeneralizedVehicleVelocity,
};
use vehicle_physics_engine::coupled_vehicle_world::{
    CoupledVehicleWorld, CoupledVehicleWorldConfiguration, CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_NORMAL,
    CONTACT_MANIFOLD_RETENTION_STEPS,
};
use vehicle_physics_engine::generalized_vehicle_contacts::{
    solve_generalized_vehicle_contacts, GeneralizedContactConstraint, GeneralizedContactSettings,
};
use vehicle_physics_engine::physical_collision_world::{
    shape_pair_contact_manifolds, PhysicalCollisionWorld, PhysicalWorldShape,
};
use vehicle_physics_engine::{Mat3, Quat, Vec3, VehicleConfig};
use parry3d_f64::na::{Isometry3, Point3};
use parry3d_f64::shape::{SharedShape, TriMesh};
use std::collections::BTreeMap;

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn world_configuration() -> CoupledVehicleWorldConfiguration {
    actual_configuration().coupled_world.unwrap()
}

fn trimesh_shape(identifier: &str, role: &str, surface_code: u32, vertices: &[[f64; 3]], triangles: &[[u32; 3]]) -> PhysicalWorldShape {
    let points = vertices.iter().map(|position| Point3::new(position[0], position[1], position[2])).collect();
    let shape = SharedShape::new(TriMesh::new(points, triangles.to_vec()).unwrap());
    PhysicalWorldShape { identifier: identifier.to_string(), surface_code, driveable: role == "driveable", shape, wheel_impact_shape: None }
}

fn subdivided_pad_shape(extent_x: f64, extent_z: f64, top: f64, bottom: f64, divisions_x: usize, divisions_z: usize) -> PhysicalWorldShape {
    let columns = divisions_x + 1;
    let rows = divisions_z + 1;
    let mut vertices = Vec::new();
    for bottom_layer in [false, true] {
        let height = if bottom_layer { bottom } else { top };
        for row in 0..rows {
            for column in 0..columns {
                let x = -extent_x + 2.0 * extent_x * column as f64 / divisions_x as f64;
                let z = -extent_z + 2.0 * extent_z * row as f64 / divisions_z as f64;
                vertices.push([x, height, z]);
            }
        }
    }
    let mut triangles = Vec::new();
    let layer_size = columns * rows;
    for layer in 0..2 {
        let offset = layer * layer_size;
        for row in 0..divisions_z {
            for column in 0..divisions_x {
                let first = offset + row * columns + column;
                let second = first + 1;
                let third = first + columns + 1;
                let fourth = first + columns;
                triangles.push([first as u32, third as u32, second as u32]);
                triangles.push([first as u32, fourth as u32, third as u32]);
            }
        }
    }
    let top_offset = layer_size;
    let bottom_offset = 0;
    let boundary = |layer_offset: usize| -> Vec<usize> {
        let mut indices = Vec::new();
        for column in 0..columns { indices.push(layer_offset + column); }
        for row in 1..rows { indices.push(layer_offset + row * columns + columns - 1); }
        for column in (0..columns - 1).rev() { indices.push(layer_offset + (rows - 1) * columns + column); }
        for row in (1..rows - 1).rev() { indices.push(layer_offset + row * columns); }
        indices
    };
    let top_boundary = boundary(top_offset);
    let bottom_boundary = boundary(bottom_offset);
    for index in 0..top_boundary.len() {
        let next = (index + 1) % top_boundary.len();
        triangles.push([top_boundary[index] as u32, bottom_boundary[index] as u32, top_boundary[next] as u32]);
        triangles.push([top_boundary[next] as u32, bottom_boundary[index] as u32, bottom_boundary[next] as u32]);
    }
    trimesh_shape("raising_pad", "solid", 6, &vertices, &triangles)
}

fn floor_and_pad_world() -> PhysicalCollisionWorld {
    let floor = trimesh_shape("flat_floor", "driveable", 0,
        &[[-50.0, 0.0, -50.0], [50.0, 0.0, -50.0], [50.0, 0.0, 50.0], [-50.0, 0.0, 50.0]],
        &[[0, 1, 2], [0, 2, 3]]);
    let pad = subdivided_pad_shape(1.5, 0.8, 0.10, -0.10, 8, 8);
    PhysicalCollisionWorld { source_digest: "analytical_manifold_pad".into(), shapes: vec![floor, pad] }
}

fn register_on_pad(world: &mut CoupledVehicleWorld, body_origin_height_metres: f64) -> u64 {
    world.register_vehicle(actual_configuration(), Vec3::new(0.0, body_origin_height_metres, 0.0), 0.0).unwrap()
}

#[test]
fn resting_contact_manifolds_grow_age_and_warm_start() {
    let mut world = CoupledVehicleWorld::new(floor_and_pad_world(), world_configuration()).unwrap();
    register_on_pad(&mut world, 0.35);
    let mut warm_started = 0_u64;
    for _ in 0..6 {
        let summary = world.refresh_contact_manifolds().unwrap();
        warm_started += summary.warm_started_contact_count;
    }
    let diagnostics = &world.contact_manifold_diagnostics;
    assert!(diagnostics.active_count > 0, "resting pad contact must keep manifolds alive");
    assert!(diagnostics.created_total > 0);
    assert!(warm_started > 0, "persistent contacts must warm start");
    assert!(diagnostics.maximum_age_steps >= 5, "manifold age did not grow: {}", diagnostics.maximum_age_steps);
    assert_eq!(diagnostics.removed_normal_rotation_total, 0);
    assert!(diagnostics.retained_count == 0);
}

fn contacts_per_shape_pair_and_normal(world: &CoupledVehicleWorld) -> BTreeMap<(u64, Option<usize>, String, String, [i32; 3]), usize> {
    let mut counts: BTreeMap<(u64, Option<usize>, String, String, [i32; 3]), usize> = BTreeMap::new();
    for (key, entry) in world.contact_manifolds.iter() {
        if entry.last_seen_step != world.physical_step_index { continue; }
        let normal = entry.normal_world;
        let bucket = [(normal.x * 5.0).round() as i32, (normal.y * 5.0).round() as i32, (normal.z * 5.0).round() as i32];
        let group = (key.first_entity_identifier, key.world_shape_index, key.first_shape_identifier.clone(), key.second_shape_identifier.clone(), bucket);
        *counts.entry(group).or_insert(0) += 1;
    }
    counts
}

#[test]
fn redundant_contacts_are_reduced_per_shape_pair() {
    let mut world = CoupledVehicleWorld::new(floor_and_pad_world(), world_configuration()).unwrap();
    register_on_pad(&mut world, 0.35);
    let summary = world.refresh_contact_manifolds().unwrap();
    assert!(summary.generated_contact_count > summary.reduced_contact_count);
    assert!(summary.reduced_contact_count > 0, "a large pad must trigger contact reduction");
    for count in contacts_per_shape_pair_and_normal(&world).values() {
        assert!(*count <= CONTACT_MANIFOLD_MAXIMUM_CONTACTS_PER_NORMAL, "contact normal kept {count} contacts");
    }
}

#[test]
fn reset_and_removal_invalidate_manifolds() {
    let mut world = CoupledVehicleWorld::new(floor_and_pad_world(), world_configuration()).unwrap();
    let identifier = register_on_pad(&mut world, 0.35);
    world.refresh_contact_manifolds().unwrap();
    assert!(world.contact_manifold_diagnostics.active_count > 0);
    world.reset_vehicle(identifier).unwrap();
    assert!(world.contact_manifolds.keys().all(|key| key.first_entity_identifier != identifier && key.second_entity_identifier != Some(identifier)));
    assert!(world.contact_manifold_diagnostics.removed_reset_total > 0);
    assert_eq!(world.contact_manifold_diagnostics.active_count, 0);
    world.refresh_contact_manifolds().unwrap();
    world.remove_vehicle(identifier).unwrap();
    assert!(world.contact_manifolds.is_empty());
    assert!(world.contact_manifold_diagnostics.removed_entity_total > 0);
}

#[test]
fn leaving_contact_retires_manifolds_and_reentry_recreates_them() {
    let mut world = CoupledVehicleWorld::new(floor_and_pad_world(), world_configuration()).unwrap();
    let identifier = register_on_pad(&mut world, 0.35);
    world.refresh_contact_manifolds().unwrap();
    let created_before = world.contact_manifold_diagnostics.created_total;
    assert!(created_before > 0);
    world.vehicles.get_mut(&identifier).unwrap().state.suspension.body_origin_world_metres.y = 1.5;
    for _ in 0..(CONTACT_MANIFOLD_RETENTION_STEPS + 2) {
        world.refresh_contact_manifolds().unwrap();
    }
    assert_eq!(world.contact_manifold_diagnostics.active_count, 0);
    assert!(world.contact_manifold_diagnostics.removed_unseen_total > 0);
    world.vehicles.get_mut(&identifier).unwrap().state.suspension.body_origin_world_metres.y = 0.35;
    world.refresh_contact_manifolds().unwrap();
    assert!(world.contact_manifold_diagnostics.active_count > 0);
    assert!(world.contact_manifold_diagnostics.created_total > created_before);
}

#[test]
fn slowly_rotating_contact_does_not_drop_the_manifold() {
    let mut world = CoupledVehicleWorld::new(floor_and_pad_world(), world_configuration()).unwrap();
    let identifier = register_on_pad(&mut world, 0.35);
    let mut warm_started = 0_u64;
    for step in 0..6 {
        world.vehicles.get_mut(&identifier).unwrap().state.suspension.body_orientation_world =
            Quat::from_mat3(&Mat3::from_axis_angle(Vec3::UP, step as f64 * 0.0005));
        let summary = world.refresh_contact_manifolds().unwrap();
        warm_started += summary.warm_started_contact_count;
    }
    assert!(world.contact_manifold_diagnostics.active_count > 0);
    assert!(warm_started > 0);
    assert_eq!(world.contact_manifold_diagnostics.removed_normal_rotation_total, 0);
}

#[test]
fn feature_identity_and_local_anchors_survive_small_motion() {
    let first_shape = SharedShape::cuboid(0.5, 0.5, 0.5);
    let second_shape = SharedShape::cuboid(0.5, 0.5, 0.5);
    let first_pose = Isometry3::identity();
    let second_pose = Isometry3::translation(0.0, 1.02, 0.0);
    let first = shape_pair_contact_manifolds(&first_pose, &first_shape, &second_pose, &second_shape, 0.05).unwrap();
    assert!(!first.is_empty());
    let first_features: Vec<u32> = first.iter().flat_map(|manifold| manifold.points.iter().map(|point| point.first_feature_identifier)).collect();
    assert!(first_features.iter().any(|feature| *feature != 0));
    let moved_pose = Isometry3::translation(0.002, 1.02, 0.0);
    let moved = shape_pair_contact_manifolds(&first_pose, &first_shape, &moved_pose, &second_shape, 0.05).unwrap();
    let moved_features: Vec<u32> = moved.iter().flat_map(|manifold| manifold.points.iter().map(|point| point.first_feature_identifier)).collect();
    let persistent_features = first_features.iter().filter(|feature| moved_features.contains(feature)).count();
    assert!(persistent_features > 0, "nearby poses must keep the persistent feature identities");
    for manifold in &moved {
        for point in &manifold.points {
            assert!(point.separation_metres <= 0.05);
            assert!((point.point_on_first_world_metres - point.point_on_second_world_metres).length() <= 0.06);
        }
    }
}

#[test]
fn contact_feature_keys_stay_stable_across_warm_steps() {
    let mut world = CoupledVehicleWorld::new(floor_and_pad_world(), world_configuration()).unwrap();
    register_on_pad(&mut world, 0.35);
    world.refresh_contact_manifolds().unwrap();
    let first_step_keys: Vec<_> = world.contact_manifolds.keys().cloned().collect();
    assert!(!first_step_keys.is_empty());
    world.refresh_contact_manifolds().unwrap();
    let second_step_keys: Vec<_> = world.contact_manifolds.keys().cloned().collect();
    let persistent = first_step_keys.iter().filter(|key| second_step_keys.contains(key)).count();
    assert!(persistent > 0, "a resting contact must keep the same feature keys");
    for key in &second_step_keys {
        assert!(key.first_entity_identifier >= 1);
        assert!(key.world_shape_index.is_some());
        assert!(!key.first_shape_identifier.is_empty());
    }
}

#[test]
fn warm_start_impulses_converge_to_the_cold_solution() {
    let matrix: GeneralizedVehicleMatrix = {
        let mut matrix = [[0.0; 14]; 14];
        for coordinate in 0..14 { matrix[coordinate][coordinate] = 1.0; }
        matrix
    };
    let mut first_jacobians = [[0.0; 14]; 3];
    first_jacobians[0][0] = 1.0;
    first_jacobians[1][1] = 1.0;
    first_jacobians[2][2] = 1.0;
    let constraint = |normal_impulse: f64| GeneralizedContactConstraint {
        identifier: "warm_start_probe".into(), first_entity: 0, second_entity: None,
        first_jacobians, second_jacobians: [[0.0; 14]; 3],
        first_prescribed_velocity_metres_per_second: [0.0; 3],
        second_prescribed_velocity_metres_per_second: [0.0; 3],
        surface_code: 0,
        normal_impulse_newton_seconds: normal_impulse, tangential_impulse_newton_seconds: [0.0; 2],
    };
    let settings = GeneralizedContactSettings { maximum_iterations: 100, velocity_tolerance_metres_per_second: 1e-9,
        restitution: 0.0, restitution_minimum_speed_metres_per_second: 1.0, impact_friction_coefficient: 0.5,
        impact_friction_by_surface_code: std::collections::BTreeMap::new(),
        prediction_distance_metres: 0.01, maximum_position_correction_metres: 0.01, maximum_events_per_step: 8,
        maximum_supported_speed_metres_per_second: 120.0, maximum_supported_angular_speed_radians_per_second: 10.0 };
    let initial_velocity: GeneralizedVehicleVelocity = [0.0; 14];
    let mut cold_velocities = [initial_velocity];
    cold_velocities[0][0] = -10.0;
    let mut cold_constraints = [constraint(0.0)];
    let cold = solve_generalized_vehicle_contacts(&[matrix], &mut cold_velocities, &mut cold_constraints, &settings).unwrap();
    let mut warm_velocities = [initial_velocity];
    warm_velocities[0][0] = -10.0;
    let mut warm_constraints = [constraint(5.0)];
    let warm = solve_generalized_vehicle_contacts(&[matrix], &mut warm_velocities, &mut warm_constraints, &settings).unwrap();
    assert_eq!(warm.warm_started_constraint_count, 1);
    assert!((cold.normal_impulses_newton_seconds[0] - warm.normal_impulses_newton_seconds[0]).abs() < 1e-9);
    assert!((warm_velocities[0][0] - 0.0).abs() < 1e-9);
    assert!((warm_velocities[0][0] - cold_velocities[0][0]).abs() < 1e-9);
    assert!(warm.iterations <= cold.iterations);
}

#[test]
fn full_world_step_commits_persistent_pad_contacts() {
    let mut world = CoupledVehicleWorld::new(floor_and_pad_world(), world_configuration()).unwrap();
    register_on_pad(&mut world, 0.35);
    for _ in 0..6 {
        world.advance_host_interval(1.0 / 120.0).unwrap();
    }
    assert!(world.contact_manifold_diagnostics.active_count > 0);
    assert!(world.contact_manifold_diagnostics.warm_started_total > 0);
    assert!(world.contact_manifold_diagnostics.maximum_age_steps >= 4);
    for entry in world.contact_manifolds.values().filter(|entry| entry.last_seen_step == world.physical_step_index) {
        let bound = world.configuration.contacts.impact_friction_for_surface(entry.surface_code)
            * entry.normal_impulse_newton_seconds + 1e-9;
        assert!(entry.tangential_impulse_newton_seconds[0].hypot(entry.tangential_impulse_newton_seconds[1]) <= bound);
    }
}

#[test]
fn coplanar_mesh_seams_keep_consistent_normals_and_features() {
    let mut world = CoupledVehicleWorld::new(floor_and_pad_world(), world_configuration()).unwrap();
    register_on_pad(&mut world, 0.35);
    world.refresh_contact_manifolds().unwrap();
    assert!(world.contact_manifold_diagnostics.active_count > 0);
    let active: Vec<_> = world.contact_manifolds.iter()
        .filter(|(_, entry)| entry.last_seen_step == world.physical_step_index).collect();
    for (_, entry) in &active {
        let dominant_axis = entry.normal_world.x.abs().max(entry.normal_world.y.abs()).max(entry.normal_world.z.abs());
        assert!(dominant_axis > 0.9, "seam normal must stay axis aligned: {:?}", entry.normal_world);
    }
    let features: std::collections::BTreeSet<u32> = active.iter()
        .filter(|(key, _)| key.first_shape_identifier == "raising_pad")
        .map(|(key, _)| key.first_feature_identifier).collect();
    assert!(features.len() >= 2, "the seam test must retain features from more than one triangle");
}
