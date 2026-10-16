use race_intelligence_engine::local_driver::autonomous_driver::{
    AutonomousDriver, DriverObservation,
};
use race_intelligence_engine::local_driver::driver_tuning::load_driver_tuning;
use race_intelligence_engine::local_driver::transmission_supervision::supervise_transmission;
use race_intelligence_engine::local_driver::transmission_supervision::TransmissionSupervision;
use race_intelligence_engine::local_driver::transmission_tuning::load_transmission_tuning_from_document;
use race_intelligence_engine::scheduling::control_scheduler::{
    consume_host_interval, create_scheduler,
};
use race_intelligence_engine::track::path_projection::project_onto_path;
use race_intelligence_engine::track::path_reader::load_racing_line_from_document;
use race_intelligence_engine::track::speed_profile::{build_speed_profile, SpeedProfileLimits};

const FUJI_RACING_LINE_DOCUMENT: &str =
    include_str!("../../../tracks/fuji76_77/metadata/racing_line.json");
const TRANSMISSION_TUNING_DOCUMENT: &str =
    include_str!("../../../data/race_intelligence/automatic_transmission_tuning.json");

#[test]
fn fuji_racing_line_loads_with_expected_waypoint_count() {
    let racing_line = load_racing_line_from_document(FUJI_RACING_LINE_DOCUMENT).unwrap();
    assert_eq!(racing_line.track_identifier, "fuji76_77");
    assert_eq!(racing_line.waypoints.len(), 819);
    assert_eq!(racing_line.discarded_tail_waypoint_count, 170);
    assert!((racing_line.lap_length_metres - 4310.293945).abs() < 1e-6);
}

#[test]
fn artificial_driver_transmission_selects_automatic() {
    let tuning = load_transmission_tuning_from_document(TRANSMISSION_TUNING_DOCUMENT).unwrap();
    assert!(tuning.automatic_transmission);
    let supervision = TransmissionSupervision {
        launch_throttle_threshold_fraction: tuning.supervision.launch_throttle_threshold_fraction,
        launch_maximum_speed_metres_per_second: tuning
            .supervision
            .launch_maximum_speed_metres_per_second,
        reverse_maximum_speed_metres_per_second: tuning
            .supervision
            .reverse_maximum_speed_metres_per_second,
    };
    let launch_request = supervise_transmission(supervision, 0, 0.0, 0.8).unwrap();
    assert_eq!(launch_request, Some(1));
    let cruise_request = supervise_transmission(supervision, 3, 60.0, 0.8).unwrap();
    assert_eq!(cruise_request, None);
}

#[test]
fn fuji_speed_profile_stays_within_physical_limits() {
    let racing_line = load_racing_line_from_document(FUJI_RACING_LINE_DOCUMENT).unwrap();
    let limits = SpeedProfileLimits {
        maximum_speed_metres_per_second: 95.0,
        lateral_acceleration_limit_metres_per_second_squared: 28.0,
        longitudinal_deceleration_limit_metres_per_second_squared: 14.0,
        longitudinal_acceleration_limit_metres_per_second_squared: 7.0,
        safety_margin_fraction: 0.15,
    };
    let profile = build_speed_profile(
        &racing_line.waypoints,
        racing_line.lap_length_metres,
        limits,
    )
    .unwrap();
    assert_eq!(profile.len(), 819);
    for sample in &profile {
        assert!(sample.target_speed_metres_per_second.is_finite());
        assert!(sample.target_speed_metres_per_second >= 2.0);
        assert!(
            sample.target_speed_metres_per_second
                <= limits.maximum_speed_metres_per_second * (1.0 - limits.safety_margin_fraction)
                    + 1e-9
        );
    }
}

#[test]
fn projection_and_control_produce_finite_commands_on_fuji_start() {
    let racing_line = load_racing_line_from_document(FUJI_RACING_LINE_DOCUMENT).unwrap();
    let first_waypoint = &racing_line.waypoints[0];
    let projection = project_onto_path(
        &racing_line.waypoints,
        racing_line.lap_length_metres,
        first_waypoint.position_x_metres,
        first_waypoint.position_z_metres,
        0.0,
        0,
    )
    .unwrap();
    assert!(projection.arc_length_metres.is_finite());
    let first_x = first_waypoint.position_x_metres;
    let first_z = first_waypoint.position_z_metres;
    let mut tuning = load_driver_tuning(include_str!(
        "../../../data/race_intelligence/solitary_driver_tuning.json"
    ))
    .unwrap();
    tuning.live_grip_control.enabled = false;
    let mut driver = AutonomousDriver::new(racing_line, tuning, &vehicle_physics_engine::VehicleConfig::from_json_str(include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json")).unwrap()).unwrap();
    let command = driver
        .command(DriverObservation { vehicle_dynamics: None,
            position_x_metres: first_x,
            position_z_metres: first_z,
            heading_radians: 0.0,
            speed_metres_per_second: 0.0,
            duration_seconds: 1.0 / 60.0,
            current_gear: 0,
        })
        .unwrap();
    assert!(command.steering_fraction.is_finite());
    assert!(command.throttle_fraction > 0.0);
    assert_eq!(command.brake_fraction, 0.0);
    assert_eq!(command.gear_request, Some(1));
}

#[test]
fn scheduler_runs_control_faster_than_tactics() {
    let mut scheduler = create_scheduler(1.0 / 60.0, 0.2).unwrap();
    let mut control_runs = 0;
    let mut tactical_runs = 0;
    for _ in 0..60 {
        let (run_control, run_tactical) =
            consume_host_interval(&mut scheduler, 1.0 / 60.0).unwrap();
        if run_control {
            control_runs += 1;
        }
        if run_tactical {
            tactical_runs += 1;
        }
    }
    assert_eq!(control_runs, 60);
    assert_eq!(tactical_runs, 5);
}
