use std::ffi::c_void;

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
    "idle_consumption_kg_per_hour": 2.0
  }
}"#;

fn fueled_config() -> VehicleConfig {
    VehicleConfig::from_json_str(FUELED_PROFILE_JSON).expect("fueled profile must parse")
}

fn flat_ground_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut out = [TriRaycastSample::default(); 4];
    for wheel in WheelIndex::ALL {
        let i = wheel as usize;
        let anchor_local = sim.config.wheel_anchor_local(wheel);
        let anchor_world = sim.state.transform.transform_point(anchor_local);
        let distance = anchor_world.y.max(0.0);
        let span = if wheel.is_front() {
            sim.config.front_tire_width
        } else {
            sim.config.rear_tire_width
        } * sim.config.tri_ray_spacing_ratio;
        let side = if wheel.is_left() { 1.0 } else { -1.0 };
        let hit = |x: f64| RaycastHit {
            is_colliding: true,
            distance,
            point: Vec3::new(anchor_world.x + x, 0.0, anchor_world.z),
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        out[i] = TriRaycastSample {
            inner: hit(side * span),
            center: hit(0.0),
            outer: hit(-side * span),
        };
    }
    out
}

#[test]
fn profile_without_fuel_keeps_dry_mass_and_distribution() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert_eq!(cfg.fuel.effective_current_kg(), 0.0);
    assert_eq!(cfg.total_vehicle_mass(), cfg.vehicle_mass);
    assert_eq!(
        cfg.effective_front_weight_distribution(),
        cfg.front_weight_distribution
    );
}

#[test]
fn tank_load_raises_mass_and_moves_distribution_rearward() {
    let mut cfg = fueled_config();
    assert!((cfg.fuel.current_kg - 7.6).abs() < 1e-12);

    let loaded_mass = cfg.total_vehicle_mass();
    assert!((loaded_mass - 607.6).abs() < 1e-9);
    assert!(cfg.effective_front_weight_distribution() < cfg.front_weight_distribution);

    cfg.fuel.current_kg = cfg.fuel.capacity_kg;
    let total_mass = cfg.total_vehicle_mass();
    let dry_center_of_mass_z = (0.5 - cfg.front_weight_distribution) * cfg.wheelbase;
    let combined_center_of_mass_z =
        (cfg.vehicle_mass * dry_center_of_mass_z + 110.0 * 0.60) / total_mass;
    let expected = 0.5 - combined_center_of_mass_z / cfg.wheelbase;
    assert!(
        (cfg.effective_front_weight_distribution() - expected).abs() < 1e-12,
        "effective distribution must follow the blended center of mass"
    );
    assert!(
        (cfg.effective_front_weight_distribution() - 0.4262).abs() < 0.001,
        "a full 110 kg tank must drop the front share near 42.6%, got {}",
        cfg.effective_front_weight_distribution()
    );
    let summed_wheel_mass: f64 = WheelIndex::ALL
        .iter()
        .map(|wheel| cfg.mass_over_wheel(*wheel))
        .sum();
    assert!((summed_wheel_mass - total_mass).abs() < 1e-6);
}

#[test]
fn solver_burns_fuel_proportional_to_engine_power() {
    let cfg = fueled_config();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
    let samples = flat_ground_samples(&sim);
    let input = VehicleInput {
        throttle: 1.0,
        ..VehicleInput::default()
    };
    for _ in 0..240 {
        let body = sim.state.body_kinematics();
        sim.solve_external(body, &input, &samples, 1.0 / 120.0);
    }
    let remaining_kg = sim.config.fuel.effective_current_kg();
    assert!(
        remaining_kg < 7.6,
        "fuel must burn while the engine drives, remaining={remaining_kg}"
    );
    assert!(
        remaining_kg > 7.0,
        "two seconds must not empty a three-lap load, remaining={remaining_kg}"
    );
    assert!(sim.config.fuel.consumed_kg > 0.0);
}

#[test]
fn burn_matches_the_telemetry_mechanical_power() {
    let cfg = fueled_config();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
    let samples = flat_ground_samples(&sim);
    let input = VehicleInput {
        throttle: 1.0,
        ..VehicleInput::default()
    };
    let mut previous_remaining = sim.config.fuel.effective_current_kg();
    for _ in 0..240 {
        let body = sim.state.body_kinematics();
        let (_, telem) = sim.solve_external(body, &input, &samples, 1.0 / 120.0);
        let remaining = sim.config.fuel.effective_current_kg();
        let burned = previous_remaining - remaining;
        let expected = (telem.engine_mechanical_power_watts / 1000.0
            * sim.config.fuel.brake_specific_consumption_kg_per_kwh
            / 3600.0
            + sim.config.fuel.idle_consumption_kg_per_hour / 3600.0)
            * (1.0 / 120.0);
        assert!(
            (burned - expected).abs() < 1e-9,
            "burn {burned} must match the telemetry power flow {expected}"
        );
        previous_remaining = remaining;
    }
}

#[test]
fn idle_burn_applies_without_throttle() {
    let cfg = fueled_config();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
    let samples = flat_ground_samples(&sim);
    for _ in 0..120 {
        let body = sim.state.body_kinematics();
        sim.solve_external(body, &VehicleInput::default(), &samples, 1.0 / 120.0);
    }
    let remaining_kg = sim.config.fuel.effective_current_kg();
    let expected_idle_burn = 2.0 / 3600.0;
    assert!(
        (7.6 - remaining_kg - expected_idle_burn).abs() < expected_idle_burn * 0.25,
        "one second of idle must burn the idle flow, remaining={remaining_kg}"
    );
}

#[test]
fn empty_tank_cuts_positive_engine_torque() {
    let cfg = fueled_config();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
    sim.config.fuel.current_kg = 0.0;
    let samples = flat_ground_samples(&sim);
    let input = VehicleInput {
        throttle: 1.0,
        ..VehicleInput::default()
    };
    for _ in 0..120 {
        let body = sim.state.body_kinematics();
        let (_, telem) = sim.solve_external(body, &input, &samples, 1.0 / 120.0);
        assert!(
            telem.engine_output_torque_newton_meters <= 1e-9,
            "empty tank must cut positive torque, got {}",
            telem.engine_output_torque_newton_meters
        );
        assert_eq!(telem.fuel_remaining_kg, 0.0);
    }
    assert!(sim.config.fuel.is_empty());
}

#[test]
fn shipped_f1_2030_profile_declares_the_three_lap_load() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");
    let cfg = VehicleConfig::from_json_path(&path).expect("shipped profile must load");
    assert_eq!(cfg.vehicle_mass, 600.0);
    assert_eq!(cfg.fuel.capacity_kg, 110.0);
    assert!((cfg.fuel.initial_kg - 7.6).abs() < 1e-9);
    assert!((cfg.total_vehicle_mass() - 607.6).abs() < 1e-9);
    assert!(cfg.effective_front_weight_distribution() < 0.45);
}

#[test]
fn reset_refills_fuel_to_initial_load() {
    let cfg = fueled_config();
    let spawn = default_spawn_height(&cfg);
    let mut sim = Box::new(VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0));
    sim.config.fuel.current_kg = 0.0;
    sim.config.fuel.consumed_kg = 100.0;
    let ptr = sim.as_mut() as *mut VehicleSimulator as *mut c_void;
    f1_94_physics_reset(ptr, 0.0, 0.0, 0.0, 0.0);
    assert!((sim.config.fuel.current_kg - 7.6).abs() < 1e-12);
    assert_eq!(sim.config.fuel.consumed_kg, 0.0);
}
