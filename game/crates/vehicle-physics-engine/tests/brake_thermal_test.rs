// BRAKE-1000 acceptance: compact two-node brake thermal model exercised
// through the public API and the JSON config layer.
use vehicle_physics_engine::*;

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
