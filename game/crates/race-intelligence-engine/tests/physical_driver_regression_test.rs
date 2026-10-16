use race_intelligence_engine::local_driver::autonomous_driver::{AutonomousDriver, DriverObservation, lateral_recovery_speed_fraction};
use race_intelligence_engine::local_driver::driver_tuning::load_driver_tuning;
use race_intelligence_engine::track::path_reader::load_racing_line_from_document;
use race_intelligence_engine::track::vehicle_performance::VehiclePerformanceEnvelope;
use vehicle_physics_engine::{SurfaceType, VehicleConfig};

fn configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json")).unwrap()
}

#[test]
fn steering_command_matches_the_physical_rack_geometry() {
    use race_intelligence_engine::local_driver::steering_rack_calibration::SteeringRackCalibration;
    use vehicle_physics_engine::{WheelIndex, Vec3};
    use vehicle_physics_engine::suspension_kinematics::solve_corner;
    let configuration = configuration();
    let calibration = SteeringRackCalibration::from_configuration(&configuration).unwrap();
    let geometry = configuration.suspension.geometric_suspension.as_ref().unwrap();
    for desired_angle in [-0.05, 0.0, 0.05] {
        let command = calibration.command_for_wheel_angle(desired_angle);
        let mut actual_angle = 0.0;
        for wheel in [WheelIndex::FrontLeft, WheelIndex::FrontRight] {
            let solution = solve_corner(geometry.corners.get(wheel), 0.0,
                command * configuration.coupled_world.as_ref().unwrap().steering_rack_travel_metres, true,
                configuration.suspension.front_camber, configuration.suspension.front_toe,
                if wheel.is_left() { 1.0 } else { -1.0 }).unwrap();
            let forward = solution.wheel_basis.transform_vector(Vec3::FORWARD);
            actual_angle += forward.x.atan2(-forward.z) * 0.5;
        }
        assert!((actual_angle - desired_angle).abs() < 0.0005);
    }
}

#[test]
fn vehicle_performance_responds_to_real_tire_grip_and_brake_torque() {
    let configuration = configuration();
    let performance = VehiclePerformanceEnvelope::from_configuration(&configuration, 0.8, 0.85).unwrap();
    let capacity = performance.at_speed(40.0);
    assert!(capacity.lateral_acceleration_metres_per_second_squared > 8.0);
    assert!(capacity.braking_deceleration_metres_per_second_squared > 5.0);
    let mut low_grip = configuration.clone();
    low_grip.tires.surface_friction.insert(SurfaceType::Road, 0.7);
    let reduced = VehiclePerformanceEnvelope::from_configuration(&low_grip, 0.8, 0.85).unwrap().at_speed(40.0);
    assert!(reduced.lateral_acceleration_metres_per_second_squared < capacity.lateral_acceleration_metres_per_second_squared);
    assert!(reduced.braking_deceleration_metres_per_second_squared < capacity.braking_deceleration_metres_per_second_squared);
    let mut weak_brakes = configuration.clone();
    weak_brakes.brakes.max_brake_torque *= 0.2;
    let reduced_brakes = VehiclePerformanceEnvelope::from_configuration(&weak_brakes, 0.8, 0.85).unwrap().at_speed(40.0);
    assert!(reduced_brakes.braking_deceleration_metres_per_second_squared < capacity.braking_deceleration_metres_per_second_squared);
    println!("PERFORMANCE lateral={} braking={}", capacity.lateral_acceleration_metres_per_second_squared, capacity.braking_deceleration_metres_per_second_squared);
}

#[test]
fn recovery_uses_the_relevant_edge_and_remains_continuous() {
    assert_eq!(lateral_recovery_speed_fraction(1.7, 3.0, 10.0, 1.0, 1.5, 0.8), 1.0);
    assert!(lateral_recovery_speed_fraction(-1.7, 3.0, 10.0, 1.0, 1.5, 0.8) < 1.0);
    let before = lateral_recovery_speed_fraction(-0.5, 3.0, 10.0, 1.0, 1.5, 0.8);
    let after = lateral_recovery_speed_fraction(-0.5001, 3.0, 10.0, 1.0, 1.5, 0.8);
    assert!((before - after).abs() < 0.0001);
    assert!(lateral_recovery_speed_fraction(-2.0, 3.0, 10.0, 1.0, 1.5, 0.8) >= 0.8);
}

#[test]
fn steering_preview_does_not_select_a_future_braking_speed() {
    let line = load_racing_line_from_document(include_str!("../../../tracks/fuji76_77/metadata/racing_line.json")).unwrap();
    let mut tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    tuning.live_grip_control.enabled = false;
    let configuration = configuration();
    let waypoint = line.waypoints[30].clone();
    let next = &line.waypoints[31];
    let heading = (waypoint.position_x_metres - next.position_x_metres).atan2(waypoint.position_z_metres - next.position_z_metres);
    tuning.minimum_lookahead_metres = 5.0;
    tuning.maximum_lookahead_metres = 5.0;
    let mut near_driver = AutonomousDriver::new(line.clone(), tuning, &configuration).unwrap();
    tuning.minimum_lookahead_metres = 28.0;
    tuning.maximum_lookahead_metres = 28.0;
    let mut far_driver = AutonomousDriver::new(line, tuning, &configuration).unwrap();
    let observation = || DriverObservation { vehicle_dynamics: None,position_x_metres: waypoint.position_x_metres,
        position_z_metres: waypoint.position_z_metres, heading_radians: heading,
        speed_metres_per_second: 40.0, duration_seconds: 1.0 / 60.0, current_gear: 2};
    let near = near_driver.command(observation()).unwrap();
    let far = far_driver.command(observation()).unwrap();
    assert!((near.target_speed_metres_per_second - far.target_speed_metres_per_second).abs() < 1e-9);
    assert!((near.planned_acceleration_metres_per_second_squared - far.planned_acceleration_metres_per_second_squared).abs() < 1e-9);
}

#[test]
fn acceleration_request_reaches_full_throttle_without_simultaneous_braking() {
    let line = load_racing_line_from_document(include_str!("../../../tracks/fuji76_77/metadata/racing_line.json")).unwrap();
    let mut tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    tuning.live_grip_control.enabled = false;
    let waypoint = line.waypoints[0].clone();
    let next_waypoint = &line.waypoints[1];
    let heading = (waypoint.position_x_metres - next_waypoint.position_x_metres)
        .atan2(waypoint.position_z_metres - next_waypoint.position_z_metres);
    let mut driver = AutonomousDriver::new(line, tuning, &configuration()).unwrap();
    for tick in 0..120 {
        let command = driver.command(DriverObservation { vehicle_dynamics: None,
            position_x_metres: waypoint.position_x_metres,
            position_z_metres: waypoint.position_z_metres,
            heading_radians: heading,
            speed_metres_per_second: 0.0,
            duration_seconds: 1.0 / 60.0,
            current_gear: 1,
        }).unwrap();
        assert_eq!(command.brake_fraction, 0.0);
        if tick >= (60.0 / tuning.maximum_throttle_increase_per_second).ceil() as usize {
            assert_eq!(command.throttle_fraction, 1.0);
        }
    }
}

