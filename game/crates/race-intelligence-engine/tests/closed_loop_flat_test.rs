use parry3d_f64::na::Point3;
use parry3d_f64::shape::{SharedShape, TriMesh};
use race_intelligence_engine::local_driver::autonomous_driver::{
    AutonomousDriver, DriverObservation,
};
use race_intelligence_engine::local_driver::driver_tuning::load_driver_tuning;
use race_intelligence_engine::track::path_projection::project_onto_path;
use race_intelligence_engine::track::path_reader::load_racing_line_from_document;
use vehicle_physics_engine::coupled_vehicle_world::{CoupledVehicleWorld, TimestampedVehicleInput};
use vehicle_physics_engine::physical_collision_world::{
    PhysicalCollisionWorld, PhysicalWorldShape,
};
use vehicle_physics_engine::{Vec3, VehicleConfig, VehicleInput};

const FUJI_RACING_LINE_DOCUMENT: &str =
    include_str!("../../../tracks/fuji76_77/metadata/racing_line.json");
const GEOMETRIC_PROFILE_DOCUMENT: &str =
    include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");

fn analytical_flat_ground() -> PhysicalCollisionWorld {
    let half_extent_metres = 2000.0;
    let vertices = vec![
        Point3::new(-half_extent_metres, 0.0, -half_extent_metres),
        Point3::new(half_extent_metres, 0.0, -half_extent_metres),
        Point3::new(half_extent_metres, 0.0, half_extent_metres),
        Point3::new(-half_extent_metres, 0.0, half_extent_metres),
    ];
    let triangles = vec![[0, 2, 1], [0, 3, 2]];
    let mesh = TriMesh::new(vertices, triangles).expect("flat ground mesh must build");
    let shape = SharedShape::new(mesh);
    PhysicalCollisionWorld {
        source_digest: "analytical_flat_ground".to_string(),
        shapes: vec![PhysicalWorldShape {
            identifier: "analytical_flat_ground".to_string(),
            surface_code: 0,
            driveable: true,
            shape,
            wheel_impact_shape: None,
        }],
    }
}

fn artificial_driver_configuration() -> VehicleConfig {
    let mut configuration = VehicleConfig::from_json_str(GEOMETRIC_PROFILE_DOCUMENT)
        .expect("geometric profile must load");
    configuration.powertrain.automatic_transmission = true;
    let transmission_document: serde_json::Value = serde_json::from_str(include_str!("../../../data/race_intelligence/automatic_transmission_tuning.json")).unwrap();
    configuration.powertrain.automatic_shift = serde_json::from_value(transmission_document["artificial_driver_transmission"]["automatic_shift"].clone()).unwrap();
    configuration
}

#[test]
fn artificial_driver_completes_opening_stint_on_flat_ground() {
    let racing_line = load_racing_line_from_document(FUJI_RACING_LINE_DOCUMENT).unwrap();
    let mut tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/solitary_driver_tuning.json"
    ))
    .unwrap();
    tuning.maximum_speed_metres_per_second = Some(60.0);
    let configuration = artificial_driver_configuration();
    let mut driver = AutonomousDriver::new(racing_line.clone(), tuning, &configuration).unwrap();
    let world_settings = configuration
        .coupled_world
        .clone()
        .expect("profile selects coupled world");
    let mut world = CoupledVehicleWorld::new(analytical_flat_ground(), world_settings)
        .expect("world must build");
    let start_waypoint = &racing_line.waypoints[0];
    let start_index = 0;
    let next_waypoint = &racing_line.waypoints[1];
    let tangent_x = next_waypoint.position_x_metres - start_waypoint.position_x_metres;
    let tangent_z = next_waypoint.position_z_metres - start_waypoint.position_z_metres;
    let spawn_yaw_radians = (-tangent_x).atan2(-tangent_z);
    let spawn_position = Vec3::new(
        start_waypoint.position_x_metres,
        0.35,
        start_waypoint.position_z_metres,
    );
    let identifier = world
        .register_vehicle(configuration, spawn_position, spawn_yaw_radians)
        .expect("vehicle registers");
    let control_interval_seconds = 1.0 / 60.0;
    let settle_ticks = 60;
    for _ in 0..settle_ticks {
        world
            .advance_host_interval(control_interval_seconds)
            .expect("settle advance succeeds");
    }
    let control_ticks = 25 * 60;
    let mut segment_index = start_index;
    let mut previous_arc_metres = start_waypoint.lap_distance_metres;
    let mut travelled_metres: f64 = 0.0;
    let mut maximum_lateral_metres: f64 = 0.0;
    let mut maximum_speed_metres_per_second: f64 = 0.0;
    let mut maximum_gear = i8::MIN;
    let mut minimum_height_metres: f64 = f64::MAX;
    let mut tick_counter = 0;
    for _ in 0..control_ticks {
        let snapshot = world.snapshots().expect("snapshot succeeds");
        let vehicle_snapshot = &snapshot[0];
        let body_origin = vehicle_snapshot.state.suspension.body_origin_world_metres;
        assert!(
            body_origin.x.is_finite() && body_origin.y.is_finite() && body_origin.z.is_finite()
        );
        minimum_height_metres = minimum_height_metres.min(body_origin.y);
        let forward_world = vehicle_snapshot
            .state
            .suspension
            .body_orientation_world
            .rotate_vec3(Vec3::FORWARD);
        let vehicle_yaw_radians = (-forward_world.x).atan2(-forward_world.z);
        let projection = project_onto_path(
            &racing_line.waypoints,
            racing_line.lap_length_metres,
            body_origin.x,
            body_origin.z,
            vehicle_yaw_radians,
            segment_index,
        )
        .expect("projection succeeds");
        segment_index = projection.segment_start_index;
        assert!(projection.lateral_offset_metres.is_finite());
        maximum_lateral_metres = maximum_lateral_metres.max(projection.lateral_offset_metres.abs());
        let mut arc_advance = projection.arc_length_metres - previous_arc_metres;
        if arc_advance < -racing_line.lap_length_metres * 0.5 {
            arc_advance += racing_line.lap_length_metres;
        }
        if arc_advance > 0.0 {
            travelled_metres += arc_advance;
        }
        previous_arc_metres = projection.arc_length_metres;
        let current_speed = (vehicle_snapshot.telemetry.speed_kmh / 3.6).max(0.0);
        maximum_speed_metres_per_second = maximum_speed_metres_per_second.max(current_speed);
        maximum_gear = maximum_gear.max(vehicle_snapshot.telemetry.gear);
        let command = driver
            .command(DriverObservation {
                position_x_metres: body_origin.x,
                position_z_metres: body_origin.z,
                heading_radians: vehicle_yaw_radians,
                speed_metres_per_second: current_speed,
                duration_seconds: control_interval_seconds,
                current_gear: vehicle_snapshot.telemetry.gear,
            })
            .expect("autonomous control succeeds");
        world
            .enqueue_input(
                identifier,
                TimestampedVehicleInput {
                    time_seconds: world.time_seconds,
                    input: VehicleInput {
                        steering: command.steering_fraction,
                        throttle: command.throttle_fraction,
                        brake: command.brake_fraction,
                        handbrake: 0.0,
                        clutch: 0.0,
                        gear_request: command.gear_request,
                    },
                    driving_aids_mask: vehicle_snapshot.driving_aids_mask,
                },
            )
            .expect("input enqueues");
        world
            .advance_host_interval(control_interval_seconds)
            .expect("control advance succeeds");
        tick_counter += 1;
        if tick_counter % 60 == 0 {
            println!(
                "TRACE second={} speed_kmh={:.1} gear={} rpm={:.0} throttle={:.2} brake={:.2} steering={:.3} lateral={:.2} arc={:.1} target={:.1} pos=({:.1},{:.1},{:.1})",
                tick_counter / 60,
                vehicle_snapshot.telemetry.speed_kmh,
                vehicle_snapshot.telemetry.gear,
                vehicle_snapshot.telemetry.rpm,
                command.throttle_fraction,
                command.brake_fraction,
                command.steering_fraction,
                projection.lateral_offset_metres,
                projection.arc_length_metres,
                command.target_speed_metres_per_second,
                body_origin.x,
                body_origin.y,
                body_origin.z,
            );
        }
    }
    println!(
        "{{\"travelled_metres\":{:.1},\"maximum_lateral_metres\":{:.2},\"maximum_speed_metres_per_second\":{:.1},\"maximum_gear\":{},\"minimum_height_metres\":{:.2},\"simulated_seconds\":{:.1}}}",
        travelled_metres,
        maximum_lateral_metres,
        maximum_speed_metres_per_second,
        maximum_gear,
        minimum_height_metres,
        world.time_seconds,
    );
    assert!(
        travelled_metres > 500.0,
        "vehicle travelled {travelled_metres}"
    );
    assert!(
        maximum_lateral_metres < 3.0,
        "lateral {maximum_lateral_metres}"
    );
    assert!(
        minimum_height_metres > -1.0,
        "height {minimum_height_metres}"
    );
    assert!(maximum_gear >= 2, "gear {maximum_gear}");
}
