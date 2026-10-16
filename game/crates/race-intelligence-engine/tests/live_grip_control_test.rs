use race_intelligence_engine::local_driver::driver_tuning::load_driver_tuning;
use race_intelligence_engine::local_driver::live_grip_control::*;
use race_intelligence_engine::track::vehicle_performance::VehiclePerformanceEnvelope;
use vehicle_physics_engine::VehicleConfig;

fn configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json")).unwrap()
}

fn observation() -> VehicleDynamicsObservation {
    VehicleDynamicsObservation {
        fast_state: FastVehicleState {
            wheel_normal_load_newtons: [3000.0; 4], wheel_combined_utilization: [0.6; 4],
            wheel_combined_demand: [0.6; 4], wheel_slip_angle_radians: [0.02; 4],
            wheel_peak_slip_angle_radians: [0.1; 4], front_steering_angle_radians: 0.02,
            yaw_rate_radians_per_second: 0.2, lateral_speed_metres_per_second: 0.0,
        },
        capacity_state: Some(CapacityVehicleState {
            wheel_lateral_capacity_newtons: [6000.0; 4], wheel_longitudinal_capacity_newtons: [6000.0; 4],
            wheel_lateral_force_newtons: [2000.0; 4], wheel_longitudinal_force_newtons: [0.0; 4],
        }),
        slow_state: Some(SlowVehicleState { wheel_grip_scale: [1.0; 4], wheel_brake_efficiency: [1.0; 4],
            total_mass_kilograms: 700.0, front_weight_fraction: 0.5 }),
    }
}

#[test]
fn multirate_updates_preserve_cadence_and_reuse_slow_packets() {
    let tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    let performance = VehiclePerformanceEnvelope::from_configuration(&configuration(), tuning.grip_utilization_fraction, tuning.nominal_tire_grip_fraction).unwrap();
    let mut controller = LiveGripController::default();
    for tick in 0..300 {
        let mut state = observation();
        if tick % 3 != 0 { state.capacity_state = None; }
        if tick % 12 != 0 { state.slow_state = None; }
        controller.observe(state, &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    }
    assert_eq!(controller.diagnostics.capacity_updates, 100);
    assert_eq!(controller.diagnostics.planning_updates, 50);
    assert_eq!(controller.diagnostics.slow_state_updates, 25);
}

#[test]
fn contact_loss_reacts_without_waiting_for_capacity_or_slow_state() {
    let tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    let performance = VehiclePerformanceEnvelope::from_configuration(&configuration(), tuning.grip_utilization_fraction, tuning.nominal_tire_grip_fraction).unwrap();
    let mut controller = LiveGripController::default();
    controller.observe(observation(), &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    let mut airborne = observation();
    airborne.capacity_state = None;
    airborne.slow_state = None;
    airborne.fast_state.wheel_normal_load_newtons[0] = 0.0;
    controller.observe(airborne, &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    assert!(controller.diagnostics.critical_grip_state);
    assert!(controller.transient_speed_fraction < 1.0);
    assert_eq!(controller.diagnostics.capacity_updates, 1);
}

#[test]
fn stale_capacity_is_rejected_instead_of_driving_with_unbounded_old_grip() {
    let tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    let performance = VehiclePerformanceEnvelope::from_configuration(&configuration(), tuning.grip_utilization_fraction, tuning.nominal_tire_grip_fraction).unwrap();
    let mut controller = LiveGripController::default();
    controller.observe(observation(), &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    let mut state = observation(); state.capacity_state = None; state.slow_state = None;
    let mut rejected = false;
    for _ in 0..20 { rejected |= controller.observe(state, &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).is_err(); }
    assert!(rejected);
}

#[test]
fn combined_slip_leaves_less_capacity_for_braking() {
    let tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    let performance = VehiclePerformanceEnvelope::from_configuration(&configuration(), tuning.grip_utilization_fraction, tuning.nominal_tire_grip_fraction).unwrap();
    let mut straight = LiveGripController::default();
    let mut turning = LiveGripController::default();
    let state = observation();
    straight.observe(state, &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    let mut saturated = state;
    saturated.capacity_state.as_mut().unwrap().wheel_lateral_force_newtons = [5900.0; 4];
    turning.observe(saturated, &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    assert!(turning.available_braking_deceleration < straight.available_braking_deceleration * 0.3);
}

#[test]
fn front_saturation_requires_both_slip_and_insufficient_yaw_response() {
    let tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    let performance = VehiclePerformanceEnvelope::from_configuration(&configuration(), tuning.grip_utilization_fraction, tuning.nominal_tire_grip_fraction).unwrap();
    let mut controller = LiveGripController::default();
    let mut state = observation();
    state.fast_state.front_steering_angle_radians = 0.1;
    state.fast_state.wheel_slip_angle_radians = [0.11; 4];
    state.fast_state.yaw_rate_radians_per_second = 0.1;
    controller.observe(state, &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    assert!(controller.diagnostics.understeer_detected);
    assert!(controller.coordinate_steering(0.2, &tuning) < 0.2);
    state.fast_state.yaw_rate_radians_per_second = 2.0;
    controller.observe(state, &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    assert!(!controller.diagnostics.understeer_detected);
}

#[test]
fn invalid_frequency_combinations_are_rejected_from_json() {
    let document = include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json");
    let mut value: serde_json::Value = serde_json::from_str(document).unwrap();
    value["live_grip_control"]["capacity_estimation_hertz"] = serde_json::json!(120.0);
    assert!(load_driver_tuning(&value.to_string()).is_err());
}

#[test]
fn measured_capacity_replaces_the_nominal_grip_reserve_once() {
    let tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    let performance = VehiclePerformanceEnvelope::from_configuration(&configuration(), tuning.grip_utilization_fraction, tuning.nominal_tire_grip_fraction).unwrap();
    let baseline = performance.axle_accelerations_at_speed(50.0);
    let mut state = observation();
    for wheel in 0..4 {
        state.capacity_state.as_mut().unwrap().wheel_lateral_capacity_newtons[wheel] = baseline[wheel / 2] * 350.0
            / (tuning.grip_utilization_fraction * tuning.nominal_tire_grip_fraction) * 0.5;
    }
    let mut controller = LiveGripController::default();
    for _ in 0..360 {
        controller.observe(state, &tuning, 50.0, 3.0, &performance, 1.0 / 60.0).unwrap();
    }
    for scale in controller.axle_scales {
        assert!((scale - 1.0 / tuning.nominal_tire_grip_fraction).abs() < 1e-10);
    }
}

#[test]
fn observation_time_limit_is_taken_from_json_tuning() {
    use race_intelligence_engine::local_driver::autonomous_driver::{AutonomousDriver, DriverObservation};
    use race_intelligence_engine::track::path_reader::load_racing_line_from_document;
    let line = load_racing_line_from_document(include_str!("../../../tracks/fuji76_77/metadata/racing_line.json")).unwrap();
    let mut tuning = load_driver_tuning(include_str!("../../../data/race_intelligence/unrestricted_driver_tuning.json")).unwrap();
    tuning.live_grip_control.enabled = false;
    tuning.live_grip_control.maximum_fast_state_age_seconds = 0.04;
    let observation = DriverObservation { position_x_metres: line.waypoints[0].position_x_metres,
        position_z_metres: line.waypoints[0].position_z_metres, heading_radians: 0.0,
        speed_metres_per_second: 0.0, duration_seconds: 0.05, current_gear: 1, vehicle_dynamics: None };
    let mut strict = AutonomousDriver::new(line.clone(), tuning, &configuration()).unwrap();
    assert!(strict.command(observation.clone()).is_err());
    tuning.live_grip_control.maximum_fast_state_age_seconds = 0.08;
    let mut relaxed = AutonomousDriver::new(line, tuning, &configuration()).unwrap();
    assert!(relaxed.command(observation).is_ok());
}
