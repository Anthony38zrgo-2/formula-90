use vehicle_physics_engine::{PowertrainState, VehicleConfig, VehicleInput};

const PHYSICS_STEP_SECONDS: f64 = 1.0 / 120.0;

fn formula_one_2030_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .expect("F1 2030 vehicle profile must load")
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
        brake: 0.5,
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

#[test]
fn downshift_rejects_a_lower_gear_that_would_exceed_the_revolution_limit() {
    let configuration = formula_one_2030_configuration();
    let mut powertrain = PowertrainState::new(&configuration);
    powertrain.current_gear = 3;
    powertrain.target_gear = 3;
    powertrain.rpm = 16_000.0;
    let road_speed_meters_per_second =
        road_speed_for_engine_revolutions_per_minute(&configuration, 3, powertrain.rpm);

    advance_powertrain(&mut powertrain, &configuration, Some(2), road_speed_meters_per_second);

    assert_eq!(powertrain.current_gear, 3);
    assert_eq!(powertrain.target_gear, 3);
    assert_eq!(powertrain.shift_timer, 0.0);
}

#[test]
fn safe_downshift_blips_the_engine_before_progressive_clutch_reengagement() {
    let configuration = formula_one_2030_configuration();
    let mut powertrain = PowertrainState::new(&configuration);
    powertrain.current_gear = 3;
    powertrain.target_gear = 3;
    powertrain.rpm = 10_000.0;
    let road_speed_meters_per_second =
        road_speed_for_engine_revolutions_per_minute(&configuration, 3, powertrain.rpm);

    advance_powertrain(&mut powertrain, &configuration, Some(2), road_speed_meters_per_second);

    assert_eq!(powertrain.target_gear, 2);
    assert!(powertrain.engine_torque > 0.0);
    assert!(powertrain.rpm > 10_000.0);
    assert_eq!(powertrain.clutch_engagement, 0.0);
    assert_eq!(powertrain.drive_torques[2], 0.0);
    assert_eq!(powertrain.drive_torques[3], 0.0);

    let mut previous_revolutions_per_minute = powertrain.rpm;
    for _ in 0..20 {
        advance_powertrain(&mut powertrain, &configuration, None, road_speed_meters_per_second);
        assert!(
            (powertrain.rpm - previous_revolutions_per_minute).abs() < 400.0,
            "downshift must not snap engine revolutions when the gear engages"
        );
        previous_revolutions_per_minute = powertrain.rpm;
        if powertrain.current_gear == 2 {
            break;
        }
    }

    assert_eq!(powertrain.current_gear, 2);
    assert!(powertrain.downshift_clutch_reengagement_remaining_seconds > 0.0);
    assert!(powertrain.clutch_engagement < 1.0);
    assert!(powertrain.rpm < configuration.max_rpm);

    for _ in 0..20 {
        advance_powertrain(&mut powertrain, &configuration, None, road_speed_meters_per_second);
    }

    assert_eq!(powertrain.downshift_clutch_reengagement_remaining_seconds, 0.0);
    assert!(powertrain.clutch_engagement > 0.0);
}

#[test]
fn rapid_consecutive_downshifts_observe_the_configured_minimum_interval() {
    let configuration = formula_one_2030_configuration();
    let mut powertrain = PowertrainState::new(&configuration);
    powertrain.current_gear = 3;
    powertrain.target_gear = 3;
    powertrain.rpm = 10_000.0;
    let road_speed_meters_per_second =
        road_speed_for_engine_revolutions_per_minute(&configuration, 3, powertrain.rpm);

    advance_powertrain(&mut powertrain, &configuration, Some(2), road_speed_meters_per_second);
    for _ in 0..20 {
        advance_powertrain(&mut powertrain, &configuration, None, road_speed_meters_per_second);
        if powertrain.current_gear == 2 {
            break;
        }
    }

    advance_powertrain(&mut powertrain, &configuration, Some(1), road_speed_meters_per_second);
    assert_eq!(powertrain.current_gear, 2);
    assert_eq!(powertrain.target_gear, 2);

    for _ in 0..30 {
        advance_powertrain(&mut powertrain, &configuration, None, road_speed_meters_per_second);
    }
    advance_powertrain(&mut powertrain, &configuration, Some(1), road_speed_meters_per_second);
    assert_eq!(powertrain.target_gear, 1);
}

#[test]
fn selecting_neutral_during_clutch_reengagement_cancels_the_blip() {
    let configuration = formula_one_2030_configuration();
    let mut powertrain = PowertrainState::new(&configuration);
    powertrain.current_gear = 2;
    powertrain.target_gear = 2;
    powertrain.rpm = 10_000.0;
    powertrain.downshift_clutch_reengagement_remaining_seconds = 0.04;
    let road_speed_meters_per_second =
        road_speed_for_engine_revolutions_per_minute(&configuration, 2, powertrain.rpm);

    advance_powertrain(&mut powertrain, &configuration, Some(0), road_speed_meters_per_second);

    assert_eq!(powertrain.target_gear, 0);
    assert_eq!(powertrain.downshift_clutch_reengagement_remaining_seconds, 0.0);
    assert!(powertrain.engine_torque <= 0.0);
}
