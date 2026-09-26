use vehicle_physics_engine::*;

const FUELED_PROFILE_JSON: &str = r#"{
  "schema_version": 3,
  "chassis": {"vehicle_mass": 600.0, "front_weight_distribution": 0.45},
  "geometry": {"wheelbase": 2.95},
  "fuel": {
    "capacity_kg": 110.0,
    "initial_kg": 7.6,
    "tank_position_local_m": {"x": 0.0, "y": 0.05, "z": 0.60},
    "brake_specific_consumption_kg_per_kwh": 0.30,
    "idle_consumption_kg_per_hour": 2.0,
    "estimated_lap_consumption_kg": 2.53,
    "reference_lap_time_s": 90.0
  }
}"#;

fn fueled_config() -> VehicleConfig {
    VehicleConfig::from_json_str(FUELED_PROFILE_JSON).expect("fueled profile must parse")
}

fn parked_simulator() -> VehicleSimulator {
    let config = fueled_config();
    let spawn_height = default_spawn_height(&config);
    VehicleSimulator::new(config, Vec3::new(0.0, spawn_height, 0.0), 0.0)
}

#[test]
fn set_fuel_kg_clamps_to_tank_limits_and_restarts_consumption_counting() {
    let mut sim = parked_simulator();
    sim.set_fuel_kg(37.95);
    assert!((sim.config.fuel.effective_current_kg() - 37.95).abs() < 1e-12);
    assert_eq!(sim.config.fuel.consumed_kg, 0.0);
    assert!((sim.config.total_vehicle_mass() - 637.95).abs() < 1e-9);

    sim.config.fuel.consumed_kg = 12.0;
    sim.set_fuel_kg(20.0);
    assert!((sim.config.fuel.effective_current_kg() - 20.0).abs() < 1e-12);
    assert_eq!(sim.config.fuel.consumed_kg, 0.0);

    sim.set_fuel_kg(999.0);
    assert_eq!(
        sim.config.fuel.effective_current_kg(),
        sim.config.fuel.capacity_kg
    );
    sim.set_fuel_kg(-4.0);
    assert_eq!(sim.config.fuel.effective_current_kg(), 0.0);
    sim.set_fuel_kg(50.0);
    sim.set_fuel_kg(f64::NAN);
    assert!(
        (sim.config.fuel.effective_current_kg() - 50.0).abs() < 1e-12,
        "a non-finite target must keep the current load"
    );
}

#[test]
fn replace_tire_set_restores_fresh_cold_tires_without_touching_the_brakes() {
    let mut sim = parked_simulator();
    for wheel in sim.state.tire_wear.wheels.iter_mut() {
        wheel.inner_wear_fraction = 0.40;
        wheel.center_wear_fraction = 0.55;
        wheel.outer_wear_fraction = 0.20;
        wheel.wear_grip_scale = 0.62;
    }
    for wheel in sim.state.tire_thermal.wheels.iter_mut() {
        wheel.tread_inner_c = 130.0;
        wheel.tread_center_c = 145.0;
        wheel.tread_outer_c = 120.0;
        wheel.carcass_c = 120.0;
        wheel.gas_c = 125.0;
        wheel.pressure_kpa_gauge += 40.0;
    }
    let brake_before = sim.state.brake_thermal.wheels[0].disc_c;

    sim.replace_tire_set();

    for (index, wheel) in sim.state.tire_wear.wheels.iter().enumerate() {
        assert_eq!(wheel.inner_wear_fraction, 0.0, "wheel {index} inner wear");
        assert_eq!(wheel.center_wear_fraction, 0.0, "wheel {index} center wear");
        assert_eq!(wheel.outer_wear_fraction, 0.0, "wheel {index} outer wear");
        assert_eq!(wheel.remaining_tread_fraction(), 1.0, "wheel {index} tread");
        assert_eq!(wheel.wear_grip_scale, 1.0, "wheel {index} grip scale");
        assert_eq!(wheel.accumulated_friction_work_joules, 0.0);
    }
    for (index, wheel) in sim.state.tire_thermal.wheels.iter().enumerate() {
        let axle_initial = if index < 2 {
            sim.config.tire_thermal.front.initial_temperature_c
        } else {
            sim.config.tire_thermal.rear.initial_temperature_c
        };
        assert_eq!(wheel.tread_inner_c, axle_initial, "wheel {index} inner temp");
        assert_eq!(wheel.tread_center_c, axle_initial, "wheel {index} center temp");
        assert_eq!(wheel.tread_outer_c, axle_initial, "wheel {index} outer temp");
        assert_eq!(wheel.carcass_c, axle_initial, "wheel {index} carcass temp");
        assert_eq!(wheel.gas_c, axle_initial, "wheel {index} gas temp");
        assert_eq!(
            wheel.pressure_kpa_gauge,
            sim.config.tire_pressure.cold_kpa_gauge[index],
            "wheel {index} cold pressure"
        );
        assert_eq!(wheel.deflection_ratio, 0.0);
    }
    assert_eq!(
        sim.state.brake_thermal.wheels[0].disc_c, brake_before,
        "a tire change must not cool the brakes"
    );
}

#[test]
fn refueled_tank_burn_resumes_from_the_service_target() {
    let mut sim = parked_simulator();
    sim.config.fuel.current_kg = 0.5;
    sim.set_fuel_kg(37.95);
    let mut samples = [TriRaycastSample::default(); 4];
    for wheel in WheelIndex::ALL {
        let anchor = sim
            .state
            .transform
            .transform_point(sim.config.wheel_anchor_local(wheel));
        let hit = RaycastHit {
            is_colliding: true,
            distance: anchor.y.max(0.0),
            point: Vec3::new(anchor.x, 0.0, anchor.z),
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        samples[wheel as usize] = TriRaycastSample {
            inner: hit,
            center: hit,
            outer: hit,
        };
    }
    for _ in 0..120 {
        let body = sim.state.body_kinematics();
        sim.solve_external(body, &VehicleInput::default(), &samples, 1.0 / 120.0);
    }
    let remaining = sim.config.fuel.effective_current_kg();
    assert!(
        (remaining - 37.95).abs() < 0.01,
        "one idle second must burn only the idle flow after the refill, got {remaining}"
    );
    assert!(remaining < 37.95);
}
