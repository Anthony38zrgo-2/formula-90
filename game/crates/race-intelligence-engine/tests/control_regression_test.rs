use race_intelligence_engine::local_driver::autonomous_driver::{
    AutonomousDriver, DriverObservation,
};
use race_intelligence_engine::local_driver::driver_tuning::load_driver_tuning;
use race_intelligence_engine::scheduling::control_scheduler::{
    consume_host_interval_ticks, create_scheduler,
};
use race_intelligence_engine::track::path_reader::load_racing_line_from_document;

const FUJI_DOCUMENT: &str = include_str!("../../../tracks/fuji76_77/metadata/racing_line.json");
const TUNING_DOCUMENT: &str =
    include_str!("../../../data/race_intelligence/solitary_driver_tuning.json");

#[test]
fn unrestricted_profile_has_finite_braking_targets_above_the_previous_speed_cap() {
    let tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    assert!(tuning.maximum_speed_metres_per_second.is_none());
    let line = load_racing_line_from_document(FUJI_DOCUMENT).unwrap();
    let configuration = vehicle_physics_engine::VehicleConfig::from_json_str(include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json")).unwrap();
    let performance = race_intelligence_engine::track::vehicle_performance::VehiclePerformanceEnvelope::from_configuration(&configuration, tuning.grip_utilization_fraction, tuning.nominal_tire_grip_fraction).unwrap();
    let profile = performance.speed_profile(&line.waypoints, line.lap_length_metres, None).unwrap();
    assert!(profile.iter().all(|sample| sample.target_speed_metres_per_second.is_finite()));
    assert!(profile.iter().any(|sample| sample.target_speed_metres_per_second > 60.0));
}

#[test]
fn scheduler_preserves_fractional_time_and_recovers_missed_ticks() {
    let mut scheduler = create_scheduler(1.0 / 60.0, 0.2).unwrap();
    let mut control_ticks = 0;
    let mut tactical_ticks = 0;
    for _ in 0..100 {
        let counts = consume_host_interval_ticks(&mut scheduler, 0.01).unwrap();
        control_ticks += counts.0;
        tactical_ticks += counts.1;
    }
    assert_eq!((control_ticks, tactical_ticks), (60, 5));
    let counts = consume_host_interval_ticks(&mut scheduler, 0.1).unwrap();
    assert_eq!(counts.0, 6);
    assert!(scheduler.accumulated_control_seconds < 1e-10);
}

#[test]
fn malformed_loop_is_rejected_instead_of_silently_truncated() {
    let mut document: serde_json::Value = serde_json::from_str(FUJI_DOCUMENT).unwrap();
    document["waypoints"][2]["lap_distance_m"] = serde_json::json!(1000.0);
    assert!(load_racing_line_from_document(&document.to_string()).is_err());
    document = serde_json::from_str(FUJI_DOCUMENT).unwrap();
    document["track_id"] = serde_json::json!("unrecognized_track");
    assert!(load_racing_line_from_document(&document.to_string()).is_err());
}

#[test]
fn pursuit_anticipates_curve_and_limits_actuator_changes() {
    let mut document: serde_json::Value = serde_json::from_str(FUJI_DOCUMENT).unwrap();
    document["track_id"] = serde_json::json!("analytical_circle");
    document["lap_length_m"] = serde_json::json!(200.0 * std::f64::consts::PI);
    let waypoints: Vec<serde_json::Value> = (0..100).map(|index| {
        let angle = index as f64 * std::f64::consts::TAU / 100.0;
        serde_json::json!({"index": index, "pos": [100.0 * angle.cos(), 0.0, 100.0 * angle.sin()],
            "width_left": 10.0, "width_right": 10.0, "lap_distance_m": angle * 100.0, "branch_id": 0})
    }).collect();
    document["waypoint_count"] = serde_json::json!(100);
    document["waypoints"] = serde_json::json!(waypoints);
    let line = load_racing_line_from_document(&document.to_string()).unwrap();
    let tuning = load_driver_tuning(TUNING_DOCUMENT).unwrap();
    let mut driver = AutonomousDriver::new(line, tuning, &vehicle_physics_engine::VehicleConfig::from_json_str(include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json")).unwrap()).unwrap();
    let command = driver
        .command(DriverObservation {
            position_x_metres: 100.0,
            position_z_metres: 0.0,
            heading_radians: std::f64::consts::PI,
            speed_metres_per_second: 20.0,
            duration_seconds: 1.0 / 60.0,
            current_gear: 3,
        })
        .unwrap();
    assert!(command.steering_fraction.abs() > 0.001);
    assert!(
        command.steering_fraction.abs() <= tuning.maximum_steering_change_per_second / 60.0 + 1e-10
    );
    assert!(command.throttle_fraction == 0.0 || command.brake_fraction == 0.0);
}

#[test]
fn transmission_rejects_invalid_supervision_speed_at_load_time() {
    let mut document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/race_intelligence/automatic_transmission_tuning.json"
    ))
    .unwrap();
    document["artificial_driver_transmission"]["supervision"]
        ["reverse_maximum_speed_metres_per_second"] = serde_json::json!(-1.0);
    assert!(race_intelligence_engine::local_driver::transmission_tuning::load_transmission_tuning_from_document(&document.to_string()).is_err());
}

#[test]
fn closed_speed_profile_obeys_both_longitudinal_constraints() {
    use race_intelligence_engine::track::speed_profile::{build_speed_profile, SpeedProfileLimits};
    let line = load_racing_line_from_document(FUJI_DOCUMENT).unwrap();
    for maximum_speed in [0.5, 45.0, 95.0] {
        let profile = build_speed_profile(
            &line.waypoints,
            line.lap_length_metres,
            SpeedProfileLimits {
                maximum_speed_metres_per_second: maximum_speed,
                lateral_acceleration_limit_metres_per_second_squared: 8.0,
                longitudinal_deceleration_limit_metres_per_second_squared: 5.0,
                longitudinal_acceleration_limit_metres_per_second_squared: 3.0,
                safety_margin_fraction: 0.2,
            },
        )
        .unwrap();
        for index in 0..profile.len() {
            let current = profile[index];
            let next = profile[(index + 1) % profile.len()];
            let distance = (next.arc_length_metres - current.arc_length_metres)
                .rem_euclid(line.lap_length_metres);
            assert!(current.target_speed_metres_per_second <= maximum_speed * 0.8 + 1e-9);
            assert!(
                current.target_speed_metres_per_second.powi(2)
                    <= next.target_speed_metres_per_second.powi(2) + 10.0 * distance + 1e-8
            );
            assert!(
                next.target_speed_metres_per_second.powi(2)
                    <= current.target_speed_metres_per_second.powi(2) + 6.0 * distance + 1e-8
            );
        }
    }
}
