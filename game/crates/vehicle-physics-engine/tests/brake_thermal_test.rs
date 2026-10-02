// BRAKE-1000 acceptance: compact two-node brake thermal model exercised
// through the public API and the JSON config layer.
use vehicle_physics_engine::*;

fn current_vehicle_configuration() -> VehicleConfig {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");
    VehicleConfig::from_json_path(&path).unwrap()
}

#[test]
fn radiation_rejects_heat_without_creating_mechanical_braking_energy() {
    let mut configuration = current_vehicle_configuration();
    configuration.brake_thermal.front.installation_airflow_scale = 0.0;
    configuration
        .brake_thermal
        .front
        .rotor_radiation_ambient_view_factor = 1.0;
    configuration.brake_thermal.front.rotor_to_rim_w_k = 0.0;
    configuration.brake_thermal.front_duct.opening = 0.0;
    let mut system = BrakeThermalSystem::new(&configuration.brake_thermal);
    let mut cooling_rates = Vec::new();
    for temperature in [25.0, 350.0, 900.0] {
        system.wheels[0].disc_c = temperature;
        system.step_after_braking(
            WheelIndex::FrontLeft,
            &configuration.brake_thermal,
            BrakeThermalInput {
                ambient_temperature_c: 25.0,
                tire_carcass_temperature_c: 25.0,
                tire_gas_temperature_c: 25.0,
                ..Default::default()
            },
            1.0 / 120.0,
        );
        cooling_rates.push((temperature - system.wheels[0].disc_c) * 120.0);
        assert_eq!(system.wheels[0].brake_power_w, 0.0);
        assert_eq!(system.wheels[0].brake_energy_j, 0.0);
    }
    assert_eq!(cooling_rates[0], 0.0);
    assert!(cooling_rates[1] > 0.0);
    assert!(cooling_rates[2] > cooling_rates[1] * 10.0);
    assert!((cooling_rates[2] - 5.91).abs() < 0.02);
}

#[test]
fn radiation_is_optional_validated_and_preserved_by_serialization() {
    let configuration = current_vehicle_configuration();
    let serialized = configuration.to_json_value();
    let restored = VehicleConfig::from_json_str(&serialized.to_string()).unwrap();
    assert_eq!(restored.brake_thermal.front.rotor_radiation_emissivity, 0.8);
    assert_eq!(restored.brake_thermal.rear.rotor_radiation_emissivity, 0.8);
    assert_eq!(
        restored
            .brake_thermal
            .front
            .rotor_radiation_ambient_view_factor,
        0.5
    );
    assert_eq!(
        restored
            .brake_thermal
            .rear
            .rotor_radiation_ambient_view_factor,
        0.5
    );
    let legacy = VehicleConfig::from_json_str(r#"{"schema_version":3}"#).unwrap();
    assert_eq!(legacy.brake_thermal.front.rotor_radiation_emissivity, 0.0);
    assert_eq!(
        legacy
            .brake_thermal
            .front
            .rotor_radiation_ambient_view_factor,
        1.0
    );
    for invalid_emissivity in [-0.01, 1.01] {
        let mut invalid = serialized.clone();
        invalid["brakes"]["thermal"]["rear"]["rotor_radiation_emissivity"] =
            invalid_emissivity.into();
        assert!(VehicleConfig::from_json_str(&invalid.to_string()).is_err());
        invalid["brakes"]["thermal"]["rear"]["rotor_radiation_emissivity"] = 0.8.into();
        invalid["brakes"]["thermal"]["rear"]["rotor_radiation_ambient_view_factor"] =
            invalid_emissivity.into();
        assert!(VehicleConfig::from_json_str(&invalid.to_string()).is_err());
    }
}

#[test]
fn internal_ventilation_does_not_multiply_external_face_convection() {
    let mut axle = current_vehicle_configuration().brake_thermal.front;
    axle.rotor_ventilation = BrakeRotorVentilation::Solid;
    let solid_external = axle.resolve();
    axle.rotor_ventilation = BrakeRotorVentilation::Vented;
    let vented_external = axle.resolve();
    assert_eq!(
        solid_external.rotor_natural_w_k,
        vented_external.rotor_natural_w_k
    );
    assert_eq!(
        solid_external.rotor_forced_at_reference_w_k,
        vented_external.rotor_forced_at_reference_w_k
    );
    axle.cooling_profile = BrakeCoolingProfile::OpenWheelDucted;
    let legacy_vented = axle.resolve();
    assert!(
        (legacy_vented.rotor_forced_at_reference_w_k
            / vented_external.rotor_forced_at_reference_w_k
            - 1.8)
            .abs()
            < 1e-12
    );
}

#[test]
fn ambient_view_factor_scales_only_radiative_heat_exchange() {
    let mut configuration = current_vehicle_configuration();
    configuration.brake_thermal.front.installation_airflow_scale = 0.0;
    configuration.brake_thermal.front.rotor_to_rim_w_k = 0.0;
    configuration.brake_thermal.front_duct.opening = 0.0;
    let mut extracted_energies = Vec::new();
    for ambient_view_factor in [0.0, 0.5, 1.0] {
        configuration
            .brake_thermal
            .front
            .rotor_radiation_ambient_view_factor = ambient_view_factor;
        let mut system = BrakeThermalSystem::new(&configuration.brake_thermal);
        system.wheels[0].disc_c = 450.0;
        system.step_after_braking(
            WheelIndex::FrontLeft,
            &configuration.brake_thermal,
            BrakeThermalInput {
                ambient_temperature_c: 25.0,
                tire_carcass_temperature_c: 25.0,
                tire_gas_temperature_c: 25.0,
                ..Default::default()
            },
            1.0 / 120.0,
        );
        extracted_energies.push(
            (450.0 - system.wheels[0].disc_c)
                * configuration
                    .brake_thermal
                    .front
                    .resolve()
                    .rotor_capacity_j_k,
        );
        assert_eq!(system.wheels[0].duct.mass_flow_kg_s, 0.0);
        assert_eq!(system.wheels[0].rim_c, 25.0);
    }
    assert_eq!(extracted_energies[0], 0.0);
    assert!((extracted_energies[1] * 2.0 - extracted_energies[2]).abs() < 1e-8);
    let expected_radiated_energy = 0.8
        * configuration
            .brake_thermal
            .front
            .resolve()
            .rotor_radiating_area_square_meters
        * 5.670374419e-8
        * (723.15_f64.powi(4) - 298.15_f64.powi(4))
        / 120.0;
    assert!((extracted_energies[2] - expected_radiated_energy).abs() < 1e-8);
}

fn simulate_current_brakes(opening: Option<f64>, pedal: f64, seconds: usize) -> BrakeThermalSystem {
    let mut configuration = current_vehicle_configuration();
    if let Some(duct_opening) = opening {
        configuration.brake_thermal.front_duct.opening = duct_opening;
        configuration.brake_thermal.rear_duct.opening = duct_opening;
    }
    let mut system = BrakeThermalSystem::new(&configuration.brake_thermal);
    for tick in 0..seconds * 120 {
        let braking = tick % (20 * 120) < 3 * 120;
        let speed = if braking {
            70.0 - 45.0 * (tick % (20 * 120)) as f64 / 360.0
        } else {
            60.0
        };
        let efficiencies = system.efficiency_scales();
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            let axle_share = if wheel.is_front() {
                configuration.front_brake_bias
            } else {
                1.0 - configuration.front_brake_bias
            };
            let radius = if wheel.is_front() {
                configuration.front_tire_radius
            } else {
                configuration.rear_tire_radius
            };
            system.step_after_braking(
                wheel,
                &configuration.brake_thermal,
                BrakeThermalInput {
                    applied_brake_torque_nm: if braking {
                        configuration.max_brake_torque
                            * axle_share
                            * 0.5
                            * pedal
                            * efficiencies[index]
                    } else {
                        0.0
                    },
                    wheel_spin_pre_rad_s: speed / radius,
                    wheel_spin_post_rad_s: speed / radius,
                    vehicle_speed_ms: speed,
                    air_density_kg_m3: configuration.air_density,
                    ambient_temperature_c: configuration.brake_thermal.initial_temperature_c,
                    tire_carcass_temperature_c: 70.0,
                    tire_gas_temperature_c: 70.0,
                },
                1.0 / 120.0,
            );
        }
    }
    system
}

#[test]
fn current_brakes_respond_to_duct_opening_and_actual_braking_demand() {
    let closed = simulate_current_brakes(Some(0.0), 1.0, 600);
    let configured = simulate_current_brakes(None, 1.0, 600);
    let open = simulate_current_brakes(Some(1.0), 1.0, 600);
    let light_braking = simulate_current_brakes(None, 0.15, 600);
    for index in 0..4 {
        assert!(closed.wheels[index].disc_c > configured.wheels[index].disc_c);
        assert!(configured.wheels[index].disc_c > open.wheels[index].disc_c);
        assert!(configured.wheels[index].disc_c > light_braking.wheels[index].disc_c);
        assert!(
            configured.wheels[index].brake_energy_j > light_braking.wheels[index].brake_energy_j
        );
        assert!(
            configured.wheels[index].disc_c >= 350.0,
            "wheel {index}: {}",
            configured.wheels[index].disc_c
        );
        assert!(open.wheels[index].disc_c < 350.0);
        assert!(light_braking.wheels[index].disc_c < 350.0);
    }
}

fn simulate_vehicle_braking_at_temperature(temperature_celsius: f64) -> TelemetryFrame {
    let configuration = current_vehicle_configuration();
    let height = default_spawn_height(&configuration);
    let mut simulator =
        VehicleSimulator::new(configuration.clone(), Vec3::new(0.0, height, 0.0), 0.0);
    simulator.state.linear_velocity = Vec3::new(0.0, 0.0, -70.0);
    simulator.state.brake_input_smoothed = 1.0;
    let mut contact_samples = [TriRaycastSample::default(); 4];
    for wheel in WheelIndex::ALL {
        let index = wheel as usize;
        let radius = if wheel.is_front() {
            configuration.front_tire_radius
        } else {
            configuration.rear_tire_radius
        };
        simulator.state.tires.wheels[index].spin = 70.0 / radius;
        simulator.state.brake_thermal.wheels[index].disc_c = temperature_celsius;
        simulator.state.brake_thermal.wheels[index].efficiency =
            brake_efficiency(temperature_celsius, &configuration.brake_thermal);
        let anchor = simulator
            .state
            .transform
            .transform_point(configuration.wheel_anchor_local(wheel));
        let hit = RaycastHit {
            is_colliding: true,
            distance: anchor.y.max(0.0),
            point: Vec3::new(anchor.x, 0.0, anchor.z),
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        contact_samples[index] = TriRaycastSample {
            inner: hit,
            center: hit,
            outer: hit,
        };
    }
    simulator.step(
        &VehicleInput {
            brake: 1.0,
            ..Default::default()
        },
        &contact_samples,
        1.0 / 120.0,
    )
}

#[test]
fn vehicle_solver_uses_temperature_efficiency_for_torque_power_and_heat() {
    let configuration = current_vehicle_configuration();
    for temperature in [25.0, 450.0, 1050.0] {
        let telemetry = simulate_vehicle_braking_at_temperature(temperature);
        let efficiency = brake_efficiency(temperature, &configuration.brake_thermal);
        for wheel in WheelIndex::ALL {
            let index = wheel as usize;
            let axle_share = if wheel.is_front() {
                configuration.front_brake_bias
            } else {
                1.0 - configuration.front_brake_bias
            };
            let expected_torque = configuration.max_brake_torque * axle_share * 0.5 * efficiency;
            assert!((telemetry.brake_torque_nm[index] - expected_torque).abs() < 1e-6);
            let average_spin = (telemetry.brake_spin_pre_rad_s[index]
                + telemetry.brake_spin_post_rad_s[index])
                .abs()
                * 0.5;
            assert!((telemetry.brake_power_w[index] - expected_torque * average_spin).abs() < 1e-6);
            assert!(
                (telemetry.brake_energy_j[index] - telemetry.brake_power_w[index] / 120.0).abs()
                    < 1e-6
            );
            assert!(telemetry.brake_power_w[index] > 0.0);
        }
    }
}

fn stop_input(torque_nm: f64, spin_rad_s: f64, speed_ms: f64) -> BrakeThermalInput {
    BrakeThermalInput {
        applied_brake_torque_nm: torque_nm,
        wheel_spin_pre_rad_s: spin_rad_s,
        wheel_spin_post_rad_s: spin_rad_s,
        vehicle_speed_ms: speed_ms,
        air_density_kg_m3: 1.225,
        ambient_temperature_c: 25.0,
        tire_carcass_temperature_c: 25.0,
        tire_gas_temperature_c: 25.0,
    }
}

#[test]
fn legacy_v2_canonical_brakes_map_to_compact_two_node() {
    // Canonical v2-shaped brakes block: v3 keys absent so physical lumping of
    // the rotor->hub->rim conduction path plus radiation must apply.
    let json = r#"{
        "schema_version": 2,
        "brakes": {"thermal": {
            "initial_temperature_c": 25.0,
            "braking_heat_fraction": 0.97,
            "front": {
                "model": "scaled_two_node_v1",
                "rotor_material": "carbon_carbon",
                "rotor_mass_kg": 1.35,
                "rotor_outer_diameter_m": 0.278,
                "rotor_inner_diameter_m": 0.105,
                "rotor_ventilation": "vented",
                "cooling_profile": "open_wheel_ducted",
                "installation_airflow_scale": 0.7,
                "thermal_mass_scale": 1.0,
                "rim_heat_capacity_j_k": 4200.0,
                "hub_to_rim_w_k": 170.0,
                "rim_to_tire_carcass_w_k": 55.0,
                "rim_to_tire_gas_w_k": 38.0,
                "rim_base_air_w_k": 7.0
            },
            "rear": {
                "rotor_mass_kg": 1.8,
                "rim_heat_capacity_j_k": 6000.0,
                "hub_to_rim_w_k": 70.0,
                "rim_to_tire_carcass_w_k": 18.0,
                "rim_to_tire_gas_w_k": 12.0
            },
            "front_duct": {
                "opening": 0.02,
                "cooling_reference_speed_ms": 50.0,
                "air_specific_heat_j_kg_k": 1005.0,
                "heat_exchanger_ua_w_k": 400.0,
                "disc_cooling_weight": 0.556,
                "caliper_cooling_weight": 0.222,
                "hub_cooling_weight": 0.095,
                "rim_cooling_weight": 0.127
            }
        }}
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    let front_series = 1.0 / (1.0 / 38.0 + 1.0 / 170.0);
    let rear_series = 1.0 / (1.0 / 38.0 + 1.0 / 70.0);
    assert!((cfg.brake_thermal.front.rotor_to_rim_w_k - (front_series + 9.0)).abs() < 1e-9);
    assert!((cfg.brake_thermal.rear.rotor_to_rim_w_k - (rear_series + 9.0)).abs() < 1e-9);
    assert!((cfg.brake_thermal.front.rim_heat_capacity_j_k - 4200.0).abs() < 1e-9);
    assert!((cfg.brake_thermal.rear.rim_heat_capacity_j_k - 6000.0).abs() < 1e-9);
    // Duct weights lump into the single rotor_cooling_fraction (sums to 1.0).
    assert!((cfg.brake_thermal.front_duct.rotor_cooling_fraction - 0.873).abs() < 1e-9);
    assert_eq!(
        cfg.brake_thermal.front.model,
        BrakeThermalModelKind::ScaledTwoNodeV1
    );
    assert!((cfg.brake_thermal.front.resolve().rotor_capacity_j_k - 1500.0).abs() < 0.1);
    assert!((cfg.brake_thermal.rear.resolve().rotor_capacity_j_k - 2000.0).abs() < 0.1);
}

#[test]
fn v3_compact_axle_overrides_load_directly() {
    let json = r#"{
        "schema_version": 3,
        "brakes": {"thermal": {
            "front": {
                "rotor_mass_kg": 2.0,
                "rotor_to_rim_w_k": 42.0,
                "rim_heat_capacity_j_k": 7000.0,
                "rim_base_air_w_k": 12.0
            },
            "front_duct": {
                "rotor_cooling_fraction": 0.9
            }
        }}
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    assert!((cfg.brake_thermal.front.rotor_to_rim_w_k - 42.0).abs() < 1e-9);
    assert!((cfg.brake_thermal.front.rim_heat_capacity_j_k - 7000.0).abs() < 1e-9);
    assert!((cfg.brake_thermal.front.rim_base_air_w_k - 12.0).abs() < 1e-9);
    assert!((cfg.brake_thermal.front_duct.rotor_cooling_fraction - 0.9).abs() < 1e-9);
    let capacity = cfg.brake_thermal.front.resolve().rotor_capacity_j_k;
    assert!((capacity - 2222.222).abs() < 0.5);
    assert!((cfg.brake_thermal.rear.rotor_to_rim_w_k - 34.0).abs() < 1e-9);
}

#[test]
fn single_stop_energy_is_conserved_through_rotor_and_rim() {
    let json = r#"{
        "schema_version": 3,
        "brakes": {"thermal": {
            "braking_heat_fraction": 0.94,
            "front": {
                "rotor_mass_kg": 1.35,
                "installation_airflow_scale": 0.0,
                "rim_base_air_w_k": 0.0,
                "rim_to_tire_carcass_w_k": 0.0,
                "rim_to_tire_gas_w_k": 0.0
            },
            "front_duct": {"opening": 0.0}
        }}
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    let mut system = BrakeThermalSystem::new(&cfg.brake_thermal);
    let ticks = 180;
    let dt = 1.0 / 120.0;
    for _ in 0..ticks {
        system.step_after_braking(
            WheelIndex::FrontLeft,
            &cfg.brake_thermal,
            stop_input(1_000.0, 80.0, 0.0),
            dt,
        );
    }
    let w = system.wheels[0];
    let rotor_capacity = cfg.brake_thermal.front.resolve().rotor_capacity_j_k;
    let stored = (w.disc_c - 25.0) * rotor_capacity
        + (w.rim_c - 25.0) * cfg.brake_thermal.front.rim_heat_capacity_j_k;
    let generated = 1_000.0 * 80.0 * cfg.brake_thermal.braking_heat_fraction * (ticks as f64) * dt;
    assert!(
        (stored / generated - 1.0).abs() < 0.05,
        "stored={stored} generated={generated}"
    );
    assert!(w.disc_c > w.rim_c);
}

#[test]
fn repeated_braking_enters_window_then_fades_with_insufficient_cooling() {
    let json = r#"{
        "schema_version": 3,
        "brakes": {"thermal": {
            "initial_temperature_c": 25.0,
            "optimal_min_temperature_c": 400.0,
            "optimal_max_temperature_c": 800.0,
            "fade_start_temperature_c": 900.0,
            "critical_temperature_c": 1100.0,
            "front": {"installation_airflow_scale": 1.0},
            "front_duct": {"opening": 0.0}
        }}
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    let mut system = BrakeThermalSystem::new(&cfg.brake_thermal);
    let dt = 1.0 / 120.0;
    let mut saw_optimal_window = false;
    for _ in 0..2_400 {
        let w = system.wheels[0];
        if w.disc_c >= cfg.brake_thermal.optimal_min_temperature_c
            && w.disc_c <= cfg.brake_thermal.optimal_max_temperature_c
        {
            saw_optimal_window = true;
        }
        system.step_after_braking(
            WheelIndex::FrontLeft,
            &cfg.brake_thermal,
            stop_input(1_200.0, 100.0, 40.0),
            dt,
        );
    }
    let w = system.wheels[0];
    assert!(saw_optimal_window);
    assert!(w.disc_c > cfg.brake_thermal.fade_start_temperature_c);
    assert!(w.efficiency < 1.0);
    assert!(w.efficiency >= cfg.brake_thermal.minimum_fade_efficiency);
    assert!(w.disc_c > w.rim_c);
    assert!(system.efficiency_scales()[0] == w.efficiency);
    assert_eq!(system.efficiency_scales().len(), 4);
}

#[test]
fn duct_opening_increases_cooling_and_drag() {
    let mut cfg = BrakeThermalConfig::default();
    cfg.front_duct.opening = 0.10;
    let closed_flow = evaluate_duct_flow(&cfg.front_duct, 1.225, 70.0);
    cfg.front_duct.opening = 0.90;
    let open_flow = evaluate_duct_flow(&cfg.front_duct, 1.225, 70.0);
    assert!(open_flow.mass_flow_kg_s > closed_flow.mass_flow_kg_s);
    assert!(open_flow.cooling_conductance_w_k > closed_flow.cooling_conductance_w_k);
    assert!(open_flow.drag_force_n > closed_flow.drag_force_n);

    let mut system = BrakeThermalSystem::new(&cfg);
    let slow_drag = system.total_duct_drag_force_n(&cfg, 1.225, 30.0);
    let fast_drag = system.total_duct_drag_force_n(&cfg, 1.225, 70.0);
    assert!(fast_drag > slow_drag * 3.5);
    assert!(fast_drag > 0.0);

    let mut closed = BrakeThermalConfig::default();
    closed.front_duct.opening = 0.0;
    closed.rear_duct.opening = 0.0;
    let mut closed_system = BrakeThermalSystem::new(&closed);
    assert!((closed_system.total_duct_drag_force_n(&closed, 1.225, 70.0)).abs() < 1e-9);
}

#[test]
fn brake_rim_soak_raises_tire_carcass_and_gas() {
    let json = r#"{
        "schema_version": 3,
        "brakes": {"thermal": {
            "front": {
                "rim_heat_capacity_j_k": 4200.0,
                "rim_to_tire_carcass_w_k": 55.0,
                "rim_to_tire_gas_w_k": 38.0
            }
        }}
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    let mut brakes = BrakeThermalSystem::new(&cfg.brake_thermal);
    brakes.wheels[0].rim_c = 200.0;
    let tire_pressure = TirePressureConfig::default();
    let tire_cfg = TireThermalConfig::default();
    let environment = TireEnvironment::fallback(&tire_cfg);
    let mut tires = TireThermalSystem::new(&tire_pressure, &tire_cfg);

    for _ in 0..120 {
        let heat = brakes.step_after_braking(
            WheelIndex::FrontLeft,
            &cfg.brake_thermal,
            BrakeThermalInput {
                ambient_temperature_c: 25.0,
                tire_carcass_temperature_c: tires.wheels[0].carcass_c,
                tire_gas_temperature_c: tires.wheels[0].gas_c,
                ..Default::default()
            },
            1.0 / 120.0,
        );
        tires.step_after_forces(
            WheelIndex::FrontLeft,
            &tire_pressure,
            &tire_cfg,
            environment,
            TireThermalInput {
                external_carcass_heat_w: heat.carcass_heat_w,
                external_gas_heat_w: heat.gas_heat_w,
                ..Default::default()
            },
            1.0 / 120.0,
        );
    }
    assert!(tires.wheels[0].carcass_c > 25.0);
    assert!(tires.wheels[0].gas_c > 25.0);
    // Preheated rim drains heat back into the colder rotor/air and the tires.
    assert!(brakes.wheels[0].rim_c < 200.0);
    assert!(brakes.wheels[0].to_rim_heat_w < 0.0);
}

#[test]
fn standstill_state_has_no_fabricated_temperatures() {
    let cfg = BrakeThermalConfig::default();
    let mut system = BrakeThermalSystem::new(&cfg);
    system.step_after_braking(WheelIndex::FrontLeft, &cfg, BrakeThermalInput::default(), 1.0 / 120.0);
    let w = system.wheels[0];
    // Natural convection still exists at rest; speed-derived cooling is zero.
    assert!(w.natural_cooling_w_k > 0.0);
    assert_eq!(w.speed_cooling_w_k, 0.0);
    assert_eq!(w.duct.mass_flow_kg_s, 0.0);
    assert_eq!(w.duct.drag_force_n, 0.0);
    assert_eq!(w.to_rim_heat_w, 0.0);
    assert_eq!(w.brake_power_w, 0.0);
    assert_eq!(w.brake_energy_j, 0.0);
    for field in [w.disc_c, w.rim_c, w.efficiency, w.resolved_rotor_capacity_j_k] {
        assert!(field.is_finite());
        assert!(field > 0.0);
    }
    // Axle profile capacities land on the correct wheels.
    assert!((system.wheels[0].resolved_rotor_capacity_j_k - 1500.0).abs() < 0.1);
    assert!((system.wheels[1].resolved_rotor_capacity_j_k - 1500.0).abs() < 0.1);
    assert!((system.wheels[2].resolved_rotor_capacity_j_k - 2000.0).abs() < 0.1);
    assert!((system.wheels[3].resolved_rotor_capacity_j_k - 2000.0).abs() < 0.1);
}

#[test]
fn legacy_v2_deprecated_fields_are_accepted_and_ignored() {
    let json = r#"{
        "schema_version": 2,
        "brakes": {"thermal": {
            "direct_caliper_heat_fraction": 0.04,
            "front": {
                "surface_bulk_response_scale": 40.0,
                "disc_heat_capacity_j_k": 2400.0,
                "disc_surface_heat_capacity_j_k": 300.0,
                "disc_bulk_heat_capacity_j_k": 2100.0,
                "disc_surface_to_bulk_w_k": 450.0,
                "disc_surface_base_air_w_k": 4.0,
                "caliper_heat_capacity_j_k": 300.0,
                "hub_heat_capacity_j_k": 200.0,
                "disc_to_caliper_w_k": 150.0,
                "disc_base_air_w_k": 2.0,
                "caliper_base_air_w_k": 8.0,
                "hub_base_air_w_k": 6.0,
                "disc_flow_cooling_gain_w_k": 0.5,
                "caliper_flow_cooling_gain_w_k": 0.2,
                "hub_flow_cooling_gain_w_k": 0.1,
                "rim_flow_cooling_gain_w_k": 0.4
            }
        }}
    }"#;
    let cfg = VehicleConfig::from_json_str(json).unwrap();
    let mut system = BrakeThermalSystem::new(&cfg.brake_thermal);
    assert!((cfg.brake_thermal.front.rotor_mass_kg - 1.35).abs() < 1e-9);
    assert!((cfg.brake_thermal.front.rotor_to_rim_w_k - 26.0).abs() < 1e-9);
    assert_eq!(system.wheels[0].disc_c, cfg.brake_thermal.initial_temperature_c);
    assert_eq!(system.wheels[0].rim_c, cfg.brake_thermal.initial_temperature_c);
}
