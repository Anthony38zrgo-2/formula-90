use std::path::Path;
use vehicle_physics_engine::simulation::{CoupledThermalAdvanceContext, VehicleState};
use vehicle_physics_engine::slow_states::{
    advance_after_mechanics, SlowStateBackend, SlowStateSystem,
};
use vehicle_physics_engine::tire_thermals::TireMechanicalModifiers;
use vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root;
use vehicle_physics_engine::{Vec3, VehicleConfig, VehicleSimulator, WheelIndex};

fn configuration() -> VehicleConfig {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    load_vehicle_configuration_from_authorized_root(
        &repository.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"),
        &repository,
    )
    .unwrap()
}

fn prepared_state(configuration: &VehicleConfig) -> VehicleState {
    let mut state = VehicleState::new(configuration, Vec3::ZERO, 0.0);
    state.slow_states = SlowStateSystem::new(SlowStateBackend::Simplified);
    for wheel in WheelIndex::ALL {
        let index = wheel as usize;
        state.suspension.wheels[index].total_normal_force = 1800.0;
        state.suspension.wheels[index].ray_grounded = [true; 3];
        state.suspension.wheels[index].effective_friction = 2.7;
        state.suspension.wheels[index].tire_deflection_m = 0.008;
        state.tires.wheels[index].longitudinal_force = 1200.0;
        state.tires.wheels[index].lateral_force = 600.0;
        state.tires.wheels[index].spin_velocity_diff = 2.0;
        state.tires.wheels[index].spin = 60.0;
    }
    state.powertrain.rpm = 9000.0;
    state.powertrain.engine_torque = 300.0;
    state.brake_thermal.update_cooling_ducts(
        &configuration.brakes.brake_thermal,
        configuration.aerodynamics.air_density,
        30.0,
    );
    state.powertrain_thermal.evaluate_cooling_ducts(
        &configuration.powertrain.powertrain_thermal,
        configuration.aerodynamics.air_density,
        30.0,
    );
    state
}

fn context(duration: f64) -> CoupledThermalAdvanceContext {
    CoupledThermalAdvanceContext {
        applied_brake_torque_newton_metres: [100.0; 4],
        wheel_spin_before_radians_per_second: [60.0; 4],
        wheel_lateral_slip_metres_per_second: [1.0; 4],
        thermal_modifiers: [TireMechanicalModifiers::identity(); 4],
        vehicle_speed_metres_per_second: 30.0,
        driver_throttle_fraction: 0.7,
        duration_seconds: duration,
    }
}

fn close(first: f64, second: f64) {
    assert!(
        (first - second).abs() <= 1e-8 * first.abs().max(second.abs()).max(1.0),
        "{first} != {second}"
    );
}

#[test]
fn requested_frequencies_preserve_exact_mechanical_energy_and_update_counts() {
    let configuration = configuration();
    let mut state = prepared_state(&configuration);
    for _ in 0..1200 {
        advance_after_mechanics(&configuration, &mut state, &context(1.0 / 120.0), None).unwrap();
    }
    let accounting = state.slow_states.accounting;
    assert_eq!(accounting.mechanical_intervals, 1200);
    assert_eq!(accounting.tire_updates, 200);
    assert_eq!(accounting.brake_updates, 100);
    assert_eq!(accounting.powertrain_updates, 100);
    close(accounting.elapsed_physical_seconds, 10.0);
    for index in 0..4 {
        close(accounting.friction_work_joules[index], 3000.0 * 10.0);
        close(accounting.brake_work_joules[index], 100.0 * 60.0 * 10.0);
        close(
            accounting.applied_brake_work_joules[index],
            accounting.brake_work_joules[index],
        );
        close(
            state.brake_thermal.wheels[index].brake_energy_j,
            accounting.brake_work_joules[index],
        );
        close(
            state.tire_wear.wheels[index].accumulated_friction_work_joules,
            accounting.friction_work_joules[index],
        );
        close(
            accounting.applied_tire_generated_heat_joules[index],
            accounting.tire_generated_heat_joules[index],
        );
    }
    close(
        accounting.applied_engine_generated_heat_joules,
        accounting.engine_generated_heat_joules,
    );
}

#[test]
fn heat_and_wear_are_accounted_before_first_thermal_boundary() {
    let configuration = configuration();
    let mut state = prepared_state(&configuration);
    let initial_tread = state.tire_thermal.wheels[0].tread_center_c;
    for _ in 0..5 {
        advance_after_mechanics(&configuration, &mut state, &context(1.0 / 120.0), None).unwrap();
    }
    assert_eq!(state.slow_states.accounting.tire_updates, 0);
    assert_eq!(state.slow_states.accounting.brake_updates, 0);
    assert_eq!(state.tire_thermal.wheels[0].tread_center_c, initial_tread);
    assert!(state.tire_wear.wheels[0].accumulated_friction_work_joules > 0.0);
    close(state.brake_thermal.wheels[0].brake_energy_j, 250.0);
    advance_after_mechanics(&configuration, &mut state, &context(1.0 / 120.0), None).unwrap();
    assert_eq!(state.slow_states.accounting.tire_updates, 1);
    assert_eq!(state.slow_states.accounting.brake_updates, 0);
    for _ in 0..6 {
        advance_after_mechanics(&configuration, &mut state, &context(1.0 / 120.0), None).unwrap();
    }
    assert_eq!(state.slow_states.accounting.brake_updates, 1);
    assert_eq!(state.slow_states.accounting.powertrain_updates, 1);
}

#[test]
fn variable_intervals_and_large_catchup_do_not_drop_time_or_work() {
    let configuration = configuration();
    for intervals in [
        vec![1.0 / 120.0; 120],
        vec![0.007, 0.013, 0.021, 0.009, 0.05, 0.037, 0.063, 0.3, 0.5],
        vec![1.0],
    ] {
        let mut state = prepared_state(&configuration);
        for duration in intervals {
            advance_after_mechanics(&configuration, &mut state, &context(duration), None).unwrap();
        }
        assert_eq!(state.slow_states.accounting.tire_updates, 20);
        assert_eq!(state.slow_states.accounting.brake_updates, 10);
        assert_eq!(state.slow_states.accounting.powertrain_updates, 10);
        close(state.slow_states.accounting.elapsed_physical_seconds, 1.0);
        close(state.slow_states.accounting.brake_work_joules[0], 6000.0);
        close(
            state.tire_wear.wheels[0].accumulated_friction_work_joules,
            3000.0,
        );
    }
}

#[test]
fn anti_correlated_sources_are_integrated_before_averaging() {
    let configuration = configuration();
    let mut state = prepared_state(&configuration);
    for tick in 0..120 {
        let mut sample = context(1.0 / 120.0);
        if tick % 2 == 0 {
            sample.applied_brake_torque_newton_metres = [0.0; 4];
            sample.wheel_spin_before_radians_per_second = [100.0; 4];
            state
                .tires
                .wheels
                .iter_mut()
                .for_each(|wheel| wheel.spin = 100.0);
            state.powertrain.rpm = 10000.0;
            state.powertrain.engine_torque = 0.0;
        } else {
            sample.applied_brake_torque_newton_metres = [1000.0; 4];
            sample.wheel_spin_before_radians_per_second = [0.0; 4];
            state
                .tires
                .wheels
                .iter_mut()
                .for_each(|wheel| wheel.spin = 0.0);
            state.powertrain.rpm = 0.0;
            state.powertrain.engine_torque = 400.0;
        }
        advance_after_mechanics(&configuration, &mut state, &sample, None).unwrap();
    }
    close(state.slow_states.accounting.brake_work_joules[0], 0.0);
    close(
        state.slow_states.accounting.engine_generated_heat_joules,
        configuration
            .powertrain
            .powertrain_thermal
            .idle_heat_generation_watts,
    );
}

#[test]
fn partial_windows_survive_serialization_without_lost_energy() {
    let configuration = configuration();
    let mut first = prepared_state(&configuration);
    for _ in 0..5 {
        advance_after_mechanics(&configuration, &mut first, &context(1.0 / 120.0), None).unwrap();
    }
    let encoded = serde_json::to_string(&first).unwrap();
    let mut second: VehicleState = serde_json::from_str(&encoded).unwrap();
    for _ in 0..115 {
        advance_after_mechanics(&configuration, &mut first, &context(1.0 / 120.0), None).unwrap();
        advance_after_mechanics(&configuration, &mut second, &context(1.0 / 120.0), None).unwrap();
    }
    close(
        first.tire_thermal.wheels[0].pressure_kpa_gauge,
        second.tire_thermal.wheels[0].pressure_kpa_gauge,
    );
    close(
        first.brake_thermal.wheels[0].disc_c,
        second.brake_thermal.wheels[0].disc_c,
    );
    close(
        first.powertrain_thermal.water_temperature_celsius,
        second.powertrain_thermal.water_temperature_celsius,
    );
    close(
        first.slow_states.accounting.engine_generated_heat_joules,
        second.slow_states.accounting.engine_generated_heat_joules,
    );
    assert_eq!(
        first.slow_states.accounting.tire_updates,
        second.slow_states.accounting.tire_updates
    );
}

#[test]
fn backend_switch_flushes_partial_windows_before_returning_to_reference() {
    let configuration = configuration();
    let mut simulator = VehicleSimulator::new(configuration.clone(), Vec3::ZERO, 0.0);
    simulator.state = prepared_state(&configuration);
    advance_after_mechanics(&configuration, &mut simulator.state, &context(0.025), None).unwrap();
    simulator.select_slow_state_backend(SlowStateBackend::Standard);
    assert_eq!(
        simulator.state.slow_states.backend,
        SlowStateBackend::Standard
    );
    close(simulator.state.slow_states.brake_elapsed_seconds, 0.0);
    close(
        simulator
            .state
            .slow_states
            .accounting
            .applied_brake_work_joules[0],
        150.0,
    );
    close(simulator.state.slow_states.pending_engine_heat_joules, 0.0);
    assert!(simulator.state.slow_states.cached_tire_modifiers.is_none());
}

#[test]
fn tire_service_keeps_brake_and_engine_pending_energy_but_discards_no_old_tire_heat() {
    let configuration = configuration();
    let mut simulator = VehicleSimulator::new(configuration.clone(), Vec3::ZERO, 0.0);
    simulator.state = prepared_state(&configuration);
    advance_after_mechanics(&configuration, &mut simulator.state, &context(0.025), None).unwrap();
    let engine_pending = simulator.state.slow_states.pending_engine_heat_joules;
    simulator.replace_tire_set();
    close(simulator.state.slow_states.tire_elapsed_seconds, 0.0);
    close(simulator.state.slow_states.brake_elapsed_seconds, 0.025);
    close(
        simulator.state.slow_states.pending_engine_heat_joules,
        engine_pending,
    );
    close(
        simulator.state.tire_wear.wheels[0].accumulated_friction_work_joules,
        0.0,
    );
    close(
        simulator.state.tire_thermal.wheels[0].pressure_kpa_gauge,
        configuration.tires.tire_pressure.cold_kpa_gauge[0],
    );
    assert!(simulator
        .state
        .slow_states
        .pending_tire_work
        .iter()
        .all(|work| work.tread_slip_heat_joules == [0.0; 3]));
}

#[test]
fn coupled_brake_work_uses_authoritative_energy_without_reconstructing_torque_work() {
    let configuration = configuration();
    let mut state = prepared_state(&configuration);
    for _ in 0..12 {
        advance_after_mechanics(
            &configuration,
            &mut state,
            &context(1.0 / 120.0),
            Some([2.0; 4]),
        )
        .unwrap();
    }
    close(state.slow_states.accounting.brake_work_joules[0], 24.0);
    close(state.brake_thermal.wheels[0].brake_energy_j, 24.0);
}

#[test]
fn invalid_durations_and_energy_do_not_advance_accumulators() {
    let configuration = configuration();
    let mut state = prepared_state(&configuration);
    for duration in [0.0, -0.1, f64::NAN, f64::INFINITY] {
        assert!(
            advance_after_mechanics(&configuration, &mut state, &context(duration), None).is_err()
        );
    }
    assert!(advance_after_mechanics(
        &configuration,
        &mut state,
        &context(1.0 / 120.0),
        Some([-1.0; 4])
    )
    .is_err());
    assert_eq!(state.slow_states.accounting.mechanical_intervals, 0);
}

#[test]
fn legacy_serialized_state_defaults_to_reference_backend() {
    let configuration = configuration();
    let state = prepared_state(&configuration);
    let mut encoded = serde_json::to_value(&state).unwrap();
    encoded.as_object_mut().unwrap().remove("slow_states");
    let restored: VehicleState = serde_json::from_value(encoded).unwrap();
    assert_eq!(restored.slow_states.backend, SlowStateBackend::Standard);
}

fn total_thermal_energy(configuration: &VehicleConfig, state: &VehicleState) -> f64 {
    let mut total = 0.0;
    for wheel in WheelIndex::ALL {
        let index = wheel as usize;
        let thermal = configuration.tires.tire_thermal.for_wheel(wheel);
        let tire = state.tire_thermal.wheels[index];
        total += (tire.tread_inner_c + tire.tread_center_c + tire.tread_outer_c)
            * thermal.tread_zone_heat_capacity_j_k
            + tire.carcass_c * thermal.carcass_heat_capacity_j_k
            + tire.gas_c * thermal.gas_heat_capacity_j_k;
        let brake = if wheel.is_front() {
            &configuration.brakes.brake_thermal.front
        } else {
            &configuration.brakes.brake_thermal.rear
        };
        total += state.brake_thermal.wheels[index].disc_c * brake.resolve().rotor_capacity_j_k
            + state.brake_thermal.wheels[index].rim_c * brake.rim_heat_capacity_j_k;
    }
    let thermal = &configuration.powertrain.powertrain_thermal;
    let powertrain = &state.powertrain_thermal;
    total += powertrain.engine_block_temperature_celsius
        * thermal.engine_block_heat_capacity_joules_per_kelvin
        + powertrain.water_temperature_celsius * thermal.water_heat_capacity_joules_per_kelvin
        + powertrain.oil_temperature_celsius * thermal.oil_heat_capacity_joules_per_kelvin;
    total
}

#[test]
fn closed_thermal_network_conserves_energy_including_brake_to_tire_exchange() {
    let mut configuration = configuration();
    for thermal in [
        &mut configuration.tires.tire_thermal.front,
        &mut configuration.tires.tire_thermal.rear,
    ] {
        thermal.tread_to_air_w_k = 0.0;
        thermal.carcass_to_air_w_k = 0.0;
        thermal.road_conductance_w_k = 0.0;
    }
    for brake in [
        &mut configuration.brakes.brake_thermal.front,
        &mut configuration.brakes.brake_thermal.rear,
    ] {
        brake.installation_airflow_scale = 0.0;
        brake.rim_base_air_w_k = 0.0;
        brake.rotor_radiation_emissivity = 0.0;
    }
    configuration.brakes.brake_thermal.front_duct.opening = 0.0;
    configuration.brakes.brake_thermal.rear_duct.opening = 0.0;
    for duct in [
        &mut configuration
            .powertrain
            .powertrain_thermal
            .water_cooling_duct,
        &mut configuration.powertrain.powertrain_thermal.oil_cooling_duct,
    ] {
        duct.opening = 0.0;
        duct.base_cooling_conductance_watts_per_kelvin = 0.0;
    }
    let mut state = prepared_state(&configuration);
    let before = total_thermal_energy(&configuration, &state);
    for _ in 0..120 {
        advance_after_mechanics(&configuration, &mut state, &context(1.0 / 120.0), None).unwrap();
    }
    let accounting = state.slow_states.accounting;
    let engine = &configuration.powertrain.powertrain_thermal;
    let expected = accounting.tire_generated_heat_joules.iter().sum::<f64>()
        + accounting.brake_work_joules.iter().sum::<f64>()
            * configuration.brakes.brake_thermal.braking_heat_fraction
        + accounting.engine_generated_heat_joules
            * (engine.engine_block_heat_fraction
                + engine.water_heat_fraction
                + engine.oil_heat_fraction);
    close(
        total_thermal_energy(&configuration, &state) - before,
        expected,
    );
}

#[test]
fn pressure_wear_brake_fade_and_engine_derating_remain_observable() {
    let configuration = configuration();
    let mut state = prepared_state(&configuration);
    for wheel in &mut state.tire_thermal.wheels {
        wheel.carcass_c = 100.0;
    }
    state.powertrain_thermal.water_temperature_celsius = configuration
        .powertrain
        .powertrain_thermal
        .water_critical_temperature_celsius
        + 5.0;
    state.powertrain_thermal.oil_temperature_celsius = configuration
        .powertrain
        .powertrain_thermal
        .oil_critical_temperature_celsius
        + 5.0;
    for wheel in &mut state.brake_thermal.wheels {
        wheel.disc_c = 1400.0;
    }
    let initial_pressure = state.tire_thermal.wheels[0].pressure_kpa_gauge;
    for _ in 0..120 {
        advance_after_mechanics(&configuration, &mut state, &context(1.0 / 120.0), None).unwrap();
    }
    assert!(state.tire_thermal.wheels[0].pressure_kpa_gauge > initial_pressure);
    assert!(state.tire_wear.wheels[0].inner_wear_fraction > 0.0);
    assert!(state.brake_thermal.wheels[0].efficiency < 1.0);
    assert!(state.powertrain_thermal.available_engine_torque_fraction < 1.0);
    assert!(state.brake_thermal.wheels[0].disc_c < 1400.0);
}

#[test]
fn prescribed_two_minute_work_trace_stays_close_to_reference_temperatures() {
    let configuration = configuration();
    let mut standard = prepared_state(&configuration);
    standard.slow_states = SlowStateSystem::new(SlowStateBackend::Standard);
    let mut simplified = standard.clone();
    simplified.slow_states = SlowStateSystem::new(SlowStateBackend::Simplified);
    let mut maximum = [0.0_f64; 8];
    for tick in 0..14400 {
        let time = tick as f64 / 120.0;
        let mut sample = context(1.0 / 120.0);
        sample.applied_brake_torque_newton_metres = [if time.rem_euclid(5.0) < 1.0 {
            configuration.brakes.max_brake_torque * 0.12
        } else {
            0.0
        }; 4];
        sample.vehicle_speed_metres_per_second = 30.0 + 10.0 * (time * 0.5).sin();
        for state in [&mut standard, &mut simplified] {
            state.powertrain.rpm =
                configuration.powertrain.max_rpm * (0.65 + 0.2 * (time * 0.4).sin());
            state.powertrain.engine_torque =
                configuration.powertrain.max_torque * (0.6 + 0.3 * (time * 0.7).cos());
            state.brake_thermal.update_cooling_ducts(
                &configuration.brakes.brake_thermal,
                configuration.aerodynamics.air_density,
                sample.vehicle_speed_metres_per_second,
            );
            state.powertrain_thermal.evaluate_cooling_ducts(
                &configuration.powertrain.powertrain_thermal,
                configuration.aerodynamics.air_density,
                sample.vehicle_speed_metres_per_second,
            );
            VehicleSimulator::advance_thermal_systems(&configuration, state, &sample, None)
                .unwrap();
        }
        for wheel in 0..4 {
            let first = standard.tire_thermal.wheels[wheel];
            let second = simplified.tire_thermal.wheels[wheel];
            let differences = [
                (first.average_tread_c() - second.average_tread_c()).abs(),
                (first.carcass_c - second.carcass_c).abs(),
                (first.gas_c - second.gas_c).abs(),
                (first.pressure_kpa_gauge - second.pressure_kpa_gauge).abs(),
                (standard.brake_thermal.wheels[wheel].disc_c
                    - simplified.brake_thermal.wheels[wheel].disc_c)
                    .abs(),
                (standard.brake_thermal.wheels[wheel].rim_c
                    - simplified.brake_thermal.wheels[wheel].rim_c)
                    .abs(),
                (standard.powertrain_thermal.water_temperature_celsius
                    - simplified.powertrain_thermal.water_temperature_celsius)
                    .abs(),
                (standard.powertrain_thermal.oil_temperature_celsius
                    - simplified.powertrain_thermal.oil_temperature_celsius)
                    .abs(),
            ];
            for index in 0..8 {
                maximum[index] = maximum[index].max(differences[index]);
            }
        }
    }
    println!(
        "SLOW_STATE_REFERENCE_MAXIMUM_DIFFERENCES {}",
        serde_json::to_string(&maximum).unwrap()
    );
    for index in [0, 1, 2, 3, 6, 7] {
        assert!(
            maximum[index] < 2.0,
            "Unexpected thermal difference at channel {index}: {}",
            maximum[index]
        );
    }
    assert!(maximum[4] < 10.0, "Rotor difference {}", maximum[4]);
    assert!(maximum[5] < 5.0, "Rim difference {}", maximum[5]);
    for wheel in 0..4 {
        close(
            standard.brake_thermal.wheels[wheel].brake_energy_j,
            simplified.brake_thermal.wheels[wheel].brake_energy_j,
        );
        close(
            standard.tire_wear.wheels[wheel].accumulated_friction_work_joules,
            simplified.tire_wear.wheels[wheel].accumulated_friction_work_joules,
        );
        let first = standard.tire_wear.wheels[wheel].inner_wear_fraction;
        let second = simplified.tire_wear.wheels[wheel].inner_wear_fraction;
        assert!((first - second).abs() < first.max(1e-6) * 0.05);
    }
}

#[test]
fn first_mechanical_step_preserves_instantaneous_cooling_drag_and_pressure_coupling() {
    use vehicle_physics_engine::{RaycastHit, SurfaceType, TriRaycastSample, VehicleInput};
    let configuration = configuration();
    let height = vehicle_physics_engine::simulation::default_spawn_height(&configuration);
    let mut standard =
        VehicleSimulator::new(configuration.clone(), Vec3::new(0.0, height, 0.0), 0.0);
    standard.select_slow_state_backend(SlowStateBackend::Standard);
    standard.state.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
    let mut simplified = standard.clone();
    simplified.select_slow_state_backend(SlowStateBackend::Simplified);
    let samples = std::array::from_fn(|index| {
        let wheel = WheelIndex::ALL[index];
        let radius = if wheel.is_front() {
            configuration.tires.front_tire_radius
        } else {
            configuration.tires.rear_tire_radius
        };
        let length = if wheel.is_front() {
            configuration.suspension.front_spring_length
        } else {
            configuration.suspension.rear_spring_length
        };
        let hit = RaycastHit {
            is_colliding: true,
            distance: radius + length - 0.02,
            point: Vec3::ZERO,
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        TriRaycastSample {
            inner: hit,
            center: hit,
            outer: hit,
        }
    });
    let body = standard.state.body_kinematics();
    let (first, _) = standard.solve_external(body, &VehicleInput::default(), &samples, 1.0 / 120.0);
    let (second, _) =
        simplified.solve_external(body, &VehicleInput::default(), &samples, 1.0 / 120.0);
    assert!((first.force_world - second.force_world).length() < 1e-8);
    assert!((first.torque_world - second.torque_world).length() < 1e-8);
    assert!(standard.state.slow_states.cached_tire_modifiers.is_none());
    assert!(
        (simplified.state.tire_thermal.wheels[0].pressure_kpa_gauge
            - standard.state.tire_thermal.wheels[0].pressure_kpa_gauge)
            .abs()
            < 2.0
    );
    assert!(
        simplified
            .state
            .powertrain_thermal
            .total_powertrain_cooling_drag_force_newtons
            > 0.0
    );
}

#[test]
fn invalid_update_frequency_is_rejected_before_work_is_recorded() {
    let configuration = configuration();
    let mut state = prepared_state(&configuration);
    state.slow_states.frequencies.brake_frequency_hertz = 0.0;
    assert!(
        advance_after_mechanics(&configuration, &mut state, &context(1.0 / 120.0), None).is_err()
    );
    assert_eq!(state.slow_states.accounting.mechanical_intervals, 0);
}

#[test]
fn coupled_carcass_heat_comes_from_damping_loss_and_is_zero_when_unloaded() {
    let configuration = configuration();
    for loaded in [false, true] {
        let mut state = prepared_state(&configuration);
        for wheel in 0..4 {
            state.tires.wheels[wheel].longitudinal_force = 0.0;
            state.tires.wheels[wheel].lateral_force = 0.0;
            state.suspension.wheels[wheel].total_normal_force = if loaded { 1800.0 } else { 0.0 };
            state.suspension.wheels[wheel].tire_deflection_velocity_m_s = 0.1;
        }
        for _ in 0..12 {
            advance_after_mechanics(
                &configuration,
                &mut state,
                &context(1.0 / 120.0),
                Some([0.0; 4]),
            )
            .unwrap();
        }
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            let expected = if loaded {
                configuration
                    .coupled_world
                    .as_ref()
                    .unwrap()
                    .suspension
                    .tire_vertical_damping_newton_seconds_per_metre[index]
                    * 0.1_f64.powi(2)
                    * configuration
                        .tires
                        .tire_thermal
                        .for_wheel(wheel)
                        .carcass_hysteresis_efficiency
                    * 0.1
            } else {
                0.0
            };
            close(
                state.slow_states.accounting.tire_generated_heat_joules[index],
                expected,
            );
        }
    }
}
