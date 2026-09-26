use vehicle_physics_engine::*;

const GEOMETRIC_PROFILE_JSON: &str =
    include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");

const STEP_SECONDS: f64 = 1.0 / 120.0;
const CORNER_SECONDS: f64 = 25.0;
const STRAIGHT_SECONDS: f64 = 15.0;
const PREHEAT_TEMPERATURE_C: f64 = 70.0;

fn geometric_config() -> VehicleConfig {
    VehicleConfig::from_json_str(GEOMETRIC_PROFILE_JSON)
        .expect("checked-in f1_2030_v10_geometric.json must be valid")
}

fn corner_input() -> TireThermalInput {
    TireThermalInput {
        normal_force_n: 3300.0,
        longitudinal_force_n: 300.0,
        lateral_force_n: 4500.0,
        slip_velocity_long_ms: 0.4,
        slip_velocity_lat_ms: 3.0,
        tire_deflection_m: 0.008,
        tire_deflection_velocity_m_s: 0.02,
        max_tire_deflection_m: 0.04,
        dynamic_camber_rad: -0.02,
        vehicle_speed_ms: 58.0,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [1.0, 2.0, 1.0],
    }
}

fn straight_input() -> TireThermalInput {
    TireThermalInput {
        normal_force_n: 3200.0,
        longitudinal_force_n: 1200.0,
        lateral_force_n: 300.0,
        slip_velocity_long_ms: 0.02,
        slip_velocity_lat_ms: 0.05,
        tire_deflection_m: 0.006,
        tire_deflection_velocity_m_s: 0.004,
        max_tire_deflection_m: 0.04,
        dynamic_camber_rad: -0.005,
        vehicle_speed_ms: 78.0,
        external_carcass_heat_w: 0.0,
        external_gas_heat_w: 0.0,
        zone_contact_weights: [1.0, 2.0, 1.0],
    }
}

struct AxleCycleTemperatures {
    first_corner_tread_c: f64,
    first_straight_tread_c: f64,
    second_corner_tread_c: f64,
    second_straight_tread_c: f64,
    second_corner_carcass_c: f64,
}

fn advance_seconds(
    system: &mut TireThermalSystem,
    wheel: WheelIndex,
    pressure: &TirePressureConfig,
    thermal: &TireThermalConfig,
    environment: TireEnvironment,
    input: TireThermalInput,
    seconds: f64,
) {
    let steps = (seconds / STEP_SECONDS).round() as usize;
    for _ in 0..steps {
        system.step_after_forces(wheel, pressure, thermal, environment, input, STEP_SECONDS);
    }
}

fn run_single_wheel_cycle(
    pressure: &TirePressureConfig,
    thermal: &TireThermalConfig,
    wheel: WheelIndex,
) -> AxleCycleTemperatures {
    let environment = TireEnvironment::fallback(thermal);
    let mut system = TireThermalSystem::new(pressure, thermal);

    advance_seconds(
        &mut system,
        wheel,
        pressure,
        thermal,
        environment,
        corner_input(),
        CORNER_SECONDS,
    );
    let first_corner_tread_c = system.wheels[wheel as usize].average_tread_c();

    advance_seconds(
        &mut system,
        wheel,
        pressure,
        thermal,
        environment,
        straight_input(),
        STRAIGHT_SECONDS,
    );
    let first_straight_tread_c = system.wheels[wheel as usize].average_tread_c();

    advance_seconds(
        &mut system,
        wheel,
        pressure,
        thermal,
        environment,
        corner_input(),
        CORNER_SECONDS,
    );
    let second_corner_tread_c = system.wheels[wheel as usize].average_tread_c();
    let second_corner_carcass_c = system.wheels[wheel as usize].carcass_c;

    advance_seconds(
        &mut system,
        wheel,
        pressure,
        thermal,
        environment,
        straight_input(),
        STRAIGHT_SECONDS,
    );
    let second_straight_tread_c = system.wheels[wheel as usize].average_tread_c();

    AxleCycleTemperatures {
        first_corner_tread_c,
        first_straight_tread_c,
        second_corner_tread_c,
        second_straight_tread_c,
        second_corner_carcass_c,
    }
}

#[test]
fn rear_profile_warms_into_window_without_one_lap_cooling() {
    let config = geometric_config();
    let rear = run_single_wheel_cycle(
        &config.tire_pressure,
        &config.tire_thermal.rear,
        WheelIndex::RearLeft,
    );

    assert!(
        rear.first_corner_tread_c > PREHEAT_TEMPERATURE_C + 15.0,
        "rear tread must warm well above preheat in the first corner: {}",
        rear.first_corner_tread_c
    );
    assert!(
        rear.second_corner_tread_c > rear.first_corner_tread_c + 2.0,
        "rear tread must keep climbing between loaded phases: {} -> {}",
        rear.first_corner_tread_c,
        rear.second_corner_tread_c
    );
    assert!(
        (90.0..=102.0).contains(&rear.second_corner_tread_c),
        "rear corner equilibrium must sit in the operating window: {}",
        rear.second_corner_tread_c
    );
    assert!(
        rear.second_straight_tread_c >= 78.0,
        "rear tread must stay near the window after a straight: {}",
        rear.second_straight_tread_c
    );

    let per_lap_gain_c = rear.second_straight_tread_c - rear.first_straight_tread_c;
    assert!(
        (2.0..=8.0).contains(&per_lap_gain_c),
        "rear tread must gain per lap instead of cooling: {per_lap_gain_c}"
    );

    let straight_cooling_rate_c_per_s =
        (rear.second_corner_tread_c - rear.second_straight_tread_c) / STRAIGHT_SECONDS;
    assert!(
        (0.5..=1.1).contains(&straight_cooling_rate_c_per_s),
        "rear straight cooling must be gradual: {straight_cooling_rate_c_per_s}"
    );
    assert!(
        (72.0..=88.0).contains(&rear.second_corner_carcass_c),
        "rear carcass must settle near its optimum: {}",
        rear.second_corner_carcass_c
    );
}

#[test]
fn front_profile_stays_on_its_checked_in_calibration() {
    let config = geometric_config();
    let front = &config.tire_thermal.front;

    assert_eq!(front.tread_zone_heat_capacity_j_k, 2100.0);
    assert_eq!(front.carcass_heat_capacity_j_k, 5300.0);
    assert_eq!(front.gas_heat_capacity_j_k, 750.0);
    assert_eq!(front.tread_to_air_w_k, 4.5);
    assert_eq!(front.carcass_to_air_w_k, 3.5);
    assert_eq!(front.tread_to_carcass_w_k, 75.0);
    assert_eq!(front.carcass_to_gas_w_k, 50.0);
    assert_eq!(front.road_conductance_w_k, 60.0);
    assert_eq!(front.lateral_tread_conductance_w_k, 24.0);
    assert_eq!(front.slip_heat_efficiency, 1.0);

    let run = run_single_wheel_cycle(&config.tire_pressure, front, WheelIndex::FrontLeft);
    assert!(
        run.second_corner_tread_c >= 98.0,
        "front corner equilibrium must stay hot: {}",
        run.second_corner_tread_c
    );
    assert!(
        run.second_straight_tread_c - run.first_straight_tread_c > 4.0,
        "front tread must keep its per-lap warm-up trend"
    );
}

#[test]
fn rear_tracks_front_within_construction_offset() {
    let config = geometric_config();
    let front = run_single_wheel_cycle(
        &config.tire_pressure,
        &config.tire_thermal.front,
        WheelIndex::FrontLeft,
    );
    let rear = run_single_wheel_cycle(
        &config.tire_pressure,
        &config.tire_thermal.rear,
        WheelIndex::RearLeft,
    );

    let corner_gap_c = front.second_corner_tread_c - rear.second_corner_tread_c;
    let straight_gap_c = front.second_straight_tread_c - rear.second_straight_tread_c;
    assert!(
        (1.0..=16.0).contains(&corner_gap_c),
        "rear corner temperature must track the front without cloning it: {corner_gap_c}"
    );
    assert!(
        (0.0..=14.0).contains(&straight_gap_c),
        "rear straight temperature must track the front: {straight_gap_c}"
    );
}

#[test]
fn legacy_rear_calibration_still_demonstrates_the_cooling_failure() {
    let config = geometric_config();
    let mut legacy_rear = config.tire_thermal.rear;
    legacy_rear.tread_zone_heat_capacity_j_k = 5200.0;
    legacy_rear.carcass_heat_capacity_j_k = 12500.0;
    legacy_rear.gas_heat_capacity_j_k = 1800.0;
    legacy_rear.tread_to_air_w_k = 24.0;
    legacy_rear.carcass_to_air_w_k = 12.0;
    legacy_rear.tread_to_carcass_w_k = 58.0;
    legacy_rear.carcass_to_gas_w_k = 22.0;
    legacy_rear.slip_heat_efficiency = 0.72;
    legacy_rear.road_conductance_w_k = 62.0;

    let run = run_single_wheel_cycle(
        &config.tire_pressure,
        &legacy_rear,
        WheelIndex::RearLeft,
    );

    assert!(
        run.second_straight_tread_c < PREHEAT_TEMPERATURE_C,
        "legacy rear calibration must still fall below the preheat within a stint: {}",
        run.second_straight_tread_c
    );
}
