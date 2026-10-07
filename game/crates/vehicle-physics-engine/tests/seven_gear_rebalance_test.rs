use vehicle_physics_engine::{PowertrainState, VehicleConfig, VehicleInput};

const PHYSICS_STEP_SECONDS: f64 = 1.0 / 120.0;
const EXPECTED_GEAR_RATIOS: [f64; 7] = [3.45, 2.85, 2.40, 2.05, 1.80, 1.64, 1.50];
const EXPECTED_TRACTION_AUTHORITY: [f64; 7] = [1.0, 0.9, 0.65, 0.45, 0.2, 0.1, 0.1];
const EXPECTED_TRACTION_MAX_CUT: [f64; 7] = [0.78, 0.65, 0.55, 0.4, 0.14, 0.07, 0.07];
const EXPECTED_TRACTION_SLIP_TARGET: [f64; 7] = [0.055, 0.065, 0.085, 0.105, 0.14, 0.18, 0.18];

fn canonical_f1_2030_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .expect("canonical F1 2030 profile must load with seven gears")
}

fn legacy_six_speed_configuration() -> VehicleConfig {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../game/data/vehicles/f1_2030/f1_2030_v10_physics.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    VehicleConfig::from_json_str(&json).expect("legacy six-speed profile must stay valid")
}

fn road_speed_for_engine_revolutions_per_minute(
    configuration: &VehicleConfig,
    gear: usize,
    engine_revolutions_per_minute: f64,
) -> f64 {
    let engine_radians_per_second =
        engine_revolutions_per_minute * 2.0 * std::f64::consts::PI / 60.0;
    engine_radians_per_second * configuration.rear_tire_radius
        / (configuration.gear_ratios[gear - 1] * configuration.final_drive)
}

fn advance_powertrain(
    powertrain: &mut PowertrainState,
    configuration: &VehicleConfig,
    requested_gear: Option<i8>,
    road_speed_meters_per_second: f64,
) {
    let wheel_angular_velocity = road_speed_meters_per_second / configuration.rear_tire_radius;
    let input = VehicleInput {
        gear_request: requested_gear,
        throttle: 1.0,
        ..VehicleInput::default()
    };
    powertrain.step_with_reaction(
        configuration,
        &input,
        &[0.0, 0.0, wheel_angular_velocity, wheel_angular_velocity],
        &[0.0; 4],
        road_speed_meters_per_second,
        false,
        false,
        false,
        PHYSICS_STEP_SECONDS,
    );
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn canonical_f1_2030_profile_exposes_seven_gears_and_complete_maps() {
    let configuration = canonical_f1_2030_configuration();
    assert_eq!(configuration.gear_ratios.len(), 7);
    for (actual, expected) in configuration
        .gear_ratios
        .iter()
        .zip(EXPECTED_GEAR_RATIOS)
    {
        assert_close(*actual, expected);
    }
    assert_close(configuration.final_drive, 4.25);
    assert_close(configuration.max_rpm, 18000.0);
    assert_close(configuration.max_torque, 432.0);
    assert!(!configuration.automatic_transmission);
    assert_eq!(
        configuration.aids.traction_control_gear_authority.len(),
        7
    );
    assert_eq!(configuration.aids.traction_control_gear_max_cut.len(), 7);
    assert_eq!(
        configuration.aids.traction_control_gear_slip_target.len(),
        7
    );
    for (actual, expected) in configuration
        .aids
        .traction_control_gear_authority
        .iter()
        .zip(EXPECTED_TRACTION_AUTHORITY)
    {
        assert_close(*actual, expected);
    }
    for (actual, expected) in configuration
        .aids
        .traction_control_gear_max_cut
        .iter()
        .zip(EXPECTED_TRACTION_MAX_CUT)
    {
        assert_close(*actual, expected);
    }
    for (actual, expected) in configuration
        .aids
        .traction_control_gear_slip_target
        .iter()
        .zip(EXPECTED_TRACTION_SLIP_TARGET)
    {
        assert_close(*actual, expected);
    }
}

#[test]
fn traction_maps_must_match_the_seven_ratio_count() {
    let short_authority = r#"{
        "schema_version": 2,
        "powertrain": {
            "gear_ratios": [3.45, 2.85, 2.40, 2.05, 1.80, 1.64, 1.50]
        },
        "aids": {
            "traction_control_gear_authority": [1.0, 0.9, 0.65, 0.45, 0.2, 0.1],
            "traction_control_gear_slip_target": [0.055, 0.065, 0.085, 0.105, 0.14, 0.18, 0.18],
            "traction_control_gear_max_cut": [0.78, 0.65, 0.55, 0.4, 0.14, 0.07, 0.07]
        }
    }"#;
    let error = VehicleConfig::from_json_str(short_authority)
        .expect_err("authority shorter than the gear count must be rejected");
    assert!(error.contains("exactly 7"), "unexpected error: {error}");

    let short_ratios = r#"{
        "schema_version": 2,
        "powertrain": {
            "gear_ratios": [3.45, 2.85, 2.40, 2.05, 1.80, 1.64]
        },
        "aids": {
            "traction_control_gear_authority": [1.0, 0.9, 0.65, 0.45, 0.2, 0.1, 0.1],
            "traction_control_gear_slip_target": [0.055, 0.065, 0.085, 0.105, 0.14, 0.18, 0.18],
            "traction_control_gear_max_cut": [0.78, 0.65, 0.55, 0.4, 0.14, 0.07, 0.07]
        }
    }"#;
    let error = VehicleConfig::from_json_str(short_ratios)
        .expect_err("maps longer than the gear count must be rejected");
    assert!(error.contains("exactly 6"), "unexpected error: {error}");
}

#[test]
fn legacy_six_speed_profile_still_rejects_a_seventh_gear() {
    let configuration = legacy_six_speed_configuration();
    assert_eq!(configuration.gear_ratios.len(), 6);
    assert_eq!(configuration.aids.traction_control_gear_authority.len(), 6);

    let mut powertrain = PowertrainState::new(&configuration);
    powertrain.current_gear = 6;
    powertrain.target_gear = 6;
    powertrain.rpm = 17000.0;
    let road_speed = road_speed_for_engine_revolutions_per_minute(&configuration, 6, 17000.0);

    advance_powertrain(&mut powertrain, &configuration, Some(7), road_speed);

    assert_eq!(powertrain.current_gear, 6);
    assert_eq!(powertrain.target_gear, 6);
    assert_eq!(powertrain.shift_timer, 0.0);
}

#[test]
fn seventh_gear_engages_and_an_eighth_request_is_ignored() {
    let configuration = canonical_f1_2030_configuration();
    let mut powertrain = PowertrainState::new(&configuration);
    powertrain.current_gear = 6;
    powertrain.target_gear = 6;
    powertrain.rpm = 17000.0;
    let road_speed = road_speed_for_engine_revolutions_per_minute(&configuration, 6, 17000.0);

    advance_powertrain(&mut powertrain, &configuration, Some(7), road_speed);
    assert_eq!(powertrain.target_gear, 7);

    for _ in 0..20 {
        advance_powertrain(&mut powertrain, &configuration, Some(7), road_speed);
        if powertrain.current_gear == 7 {
            break;
        }
    }
    assert_eq!(powertrain.current_gear, 7);
    assert!(powertrain.rpm < configuration.max_rpm);

    advance_powertrain(&mut powertrain, &configuration, Some(8), road_speed);
    assert_eq!(powertrain.current_gear, 7);
    assert_eq!(powertrain.target_gear, 7);

    advance_powertrain(&mut powertrain, &configuration, Some(7), road_speed);
    assert_eq!(powertrain.current_gear, 7);
    assert_eq!(powertrain.target_gear, 7);
}

#[test]
fn downshift_from_seventh_requires_a_safe_revolution_target() {
    let configuration = canonical_f1_2030_configuration();
    let mut powertrain = PowertrainState::new(&configuration);
    powertrain.current_gear = 7;
    powertrain.target_gear = 7;
    powertrain.rpm = 17000.0;
    let road_speed = road_speed_for_engine_revolutions_per_minute(&configuration, 7, 17000.0);

    advance_powertrain(&mut powertrain, &configuration, Some(6), road_speed);
    assert_eq!(powertrain.current_gear, 7);
    assert_eq!(powertrain.target_gear, 7);
    assert_eq!(powertrain.shift_timer, 0.0);

    powertrain.rpm = 15000.0;
    let road_speed = road_speed_for_engine_revolutions_per_minute(&configuration, 7, 15000.0);
    advance_powertrain(&mut powertrain, &configuration, Some(6), road_speed);
    assert_eq!(powertrain.target_gear, 6);

    let mut previous_revolutions_per_minute = powertrain.rpm;
    for _ in 0..20 {
        advance_powertrain(&mut powertrain, &configuration, None, road_speed);
        assert!(
            (powertrain.rpm - previous_revolutions_per_minute).abs() < 400.0,
            "seventh-to-sixth downshift must not snap engine revolutions"
        );
        previous_revolutions_per_minute = powertrain.rpm;
        if powertrain.current_gear == 6 {
            break;
        }
    }
    assert_eq!(powertrain.current_gear, 6);
    assert!(powertrain.rpm < configuration.max_rpm);
}

#[test]
fn neutral_and_reverse_requests_keep_working_with_seven_gears() {
    let configuration = canonical_f1_2030_configuration();
    let mut powertrain = PowertrainState::new(&configuration);
    powertrain.current_gear = 7;
    powertrain.target_gear = 7;
    powertrain.rpm = 15000.0;
    let road_speed = road_speed_for_engine_revolutions_per_minute(&configuration, 7, 15000.0);

    advance_powertrain(&mut powertrain, &configuration, Some(0), road_speed);
    assert_eq!(powertrain.target_gear, 0);
    for _ in 0..20 {
        advance_powertrain(&mut powertrain, &configuration, None, 0.0);
        if powertrain.current_gear == 0 {
            break;
        }
    }
    assert_eq!(powertrain.current_gear, 0);

    advance_powertrain(&mut powertrain, &configuration, Some(-1), 0.0);
    assert_eq!(powertrain.target_gear, -1);
    for _ in 0..20 {
        advance_powertrain(&mut powertrain, &configuration, None, 0.0);
        if powertrain.current_gear == -1 {
            break;
        }
    }
    assert_eq!(powertrain.current_gear, -1);
}
