use vehicle_physics_engine::*;

fn enabled_wear_config() -> TireWearConfig {
    TireWearConfig {
        tread_wear_per_megajoule_millimeters: 4.0,
        ..TireWearConfig::default()
    }
}

fn wear_inputs() -> ([f64; 3], [f64; 3], f64, f64, f64, f64) {
    ([0.7, 0.2, 0.1], [95.0, 95.0, 95.0], 5000.0, 4000.0, 1.5, 1.2)
}

#[test]
fn disabled_wear_config_keeps_legacy_grip_and_depths() {
    let config = TireWearConfig::default();
    assert!(!config.is_enabled());
    let mut system = TireWearSystem::new_with_axles(&TireWearAxleConfig::default());
    let (weights, temperatures, fz, fx, vx, vy) = wear_inputs();
    for _ in 0..600 {
        system.step_after_forces(
            WheelIndex::FrontLeft,
            &config,
            2.7,
            weights,
            temperatures,
            fz,
            fx,
            vx,
            vy,
            1.0 / 120.0,
        );
    }
    let state = &system.wheels[WheelIndex::FrontLeft as usize];
    assert_eq!(state.wear_grip_scale, 1.0);
    assert_eq!(state.inner_wear_fraction, 0.0);
    assert_eq!(
        state.tread_center_millimeters,
        config.initial_tread_depth_millimeters
    );
}

#[test]
fn deeper_fraction_work_depletes_the_loaded_zone_first() {
    let config = enabled_wear_config();
    let mut system = TireWearSystem::new_with_axles(&TireWearAxleConfig {
        front: config,
        rear: config,
    });
    let (weights, temperatures, fz, fx, vx, vy) = wear_inputs();
    for _ in 0..2400 {
        system.step_after_forces(
            WheelIndex::RearLeft,
            &config,
            2.7,
            weights,
            temperatures,
            fz,
            fx,
            vx,
            vy,
            1.0 / 120.0,
        );
    }
    let state = &system.wheels[WheelIndex::RearLeft as usize];
    assert!(state.inner_wear_fraction > state.center_wear_fraction);
    assert!(state.center_wear_fraction > state.outer_wear_fraction);
    assert!(state.accumulated_friction_work_joules > 0.0);
}

#[test]
fn hot_zones_wear_faster_than_cold_zones_for_equal_work() {
    let config = enabled_wear_config();
    let mut cold = TireWearSystem::new_with_axles(&TireWearAxleConfig {
        front: config,
        rear: config,
    });
    let mut hot = TireWearSystem::new_with_axles(&TireWearAxleConfig {
        front: config,
        rear: config,
    });
    let (weights, _, fz, fx, vx, vy) = wear_inputs();
    for _ in 0..1200 {
        cold.step_after_forces(
            WheelIndex::FrontLeft,
            &config,
            2.7,
            weights,
            [55.0, 55.0, 55.0],
            fz,
            fx,
            vx,
            vy,
            1.0 / 120.0,
        );
        hot.step_after_forces(
            WheelIndex::FrontLeft,
            &config,
            2.7,
            weights,
            [145.0, 145.0, 145.0],
            fz,
            fx,
            vx,
            vy,
            1.0 / 120.0,
        );
    }
    let cold_inner = cold.wheels[0].inner_wear_fraction;
    let hot_inner = hot.wheels[0].inner_wear_fraction;
    assert!(
        hot_inner > cold_inner * 1.5,
        "hot tread must wear substantially faster: cold={cold_inner} hot={hot_inner}"
    );
}

#[test]
fn low_friction_surface_wears_less_than_high_friction_surface() {
    let config = enabled_wear_config();
    let mut low_friction = TireWearSystem::new_with_axles(&TireWearAxleConfig {
        front: config,
        rear: config,
    });
    let mut high_friction = TireWearSystem::new_with_axles(&TireWearAxleConfig {
        front: config,
        rear: config,
    });
    let (weights, temperatures, fz, fx, vx, vy) = wear_inputs();
    for _ in 0..1200 {
        low_friction.step_after_forces(
            WheelIndex::RearLeft,
            &config,
            0.75,
            weights,
            temperatures,
            fz,
            fx,
            vx,
            vy,
            1.0 / 120.0,
        );
        high_friction.step_after_forces(
            WheelIndex::RearLeft,
            &config,
            2.7,
            weights,
            temperatures,
            fz,
            fx,
            vx,
            vy,
            1.0 / 120.0,
        );
    }
    assert!(
        low_friction.wheels[2].inner_wear_fraction
            < high_friction.wheels[2].inner_wear_fraction
    );
}

#[test]
fn zone_temperature_asymmetry_wears_inner_and_outer_differently() {
    let config = enabled_wear_config();
    let mut system = TireWearSystem::new_with_axles(&TireWearAxleConfig {
        front: config,
        rear: config,
    });
    for _ in 0..1200 {
        system.step_after_forces(
            WheelIndex::FrontRight,
            &config,
            2.7,
            [0.4, 0.2, 0.4],
            [150.0, 80.0, 80.0],
            4000.0,
            3000.0,
            1.0,
            1.0,
            1.0 / 120.0,
        );
    }
    let state = &system.wheels[WheelIndex::FrontRight as usize];
    assert!(
        state.inner_wear_fraction > state.outer_wear_fraction,
        "hot inner shoulder must wear faster: inner={} outer={}",
        state.inner_wear_fraction,
        state.outer_wear_fraction
    );
}

#[test]
fn wear_grip_scale_falls_with_accumulated_work() {
    let config = enabled_wear_config();
    let mut system = TireWearSystem::new_with_axles(&TireWearAxleConfig {
        front: config,
        rear: config,
    });
    let (weights, temperatures, fz, fx, vx, vy) = wear_inputs();
    let mut previous = 1.0;
    for _ in 0..6000 {
        system.step_after_forces(
            WheelIndex::RearRight,
            &config,
            2.7,
            weights,
            temperatures,
            fz,
            fx,
            vx,
            vy,
            1.0 / 120.0,
        );
        let grip = system.wheels[WheelIndex::RearRight as usize].wear_grip_scale;
        assert!(grip <= previous + 1e-12);
        assert!((0.0..=1.0).contains(&grip));
        previous = grip;
    }
    assert!(
        previous < 0.999,
        "wear must reduce the grip scale: {previous}"
    );
}

#[test]
fn worn_tread_reduces_delivered_longitudinal_force() {
    let config = VehicleConfig::f1_94_canonical();
    let wheel = WheelIndex::RearLeft;
    let normal = static_wheel_load(&config, wheel);
    let dt = 1.0 / 120.0;
    let velocity = Vec3::new(0.0, 0.0, -10.0);

    let run = |wear_grip_scale: f64| -> f64 {
        let mut tires = TireSystem::new(&config);
        tires.set_mechanical_modifiers(
            wheel,
            TireMechanicalModifiers {
                wear_grip_scale,
                ..TireMechanicalModifiers::identity()
            },
        );
        tires.set_mechanical_state(&config, wheel, normal, 0.0, 0.008, 1.0);
        tires.wheels[wheel as usize].spin = 60.0;
        for _ in 0..60 {
            tires.process_wheel_forces(
                &config,
                wheel,
                normal,
                SurfaceType::Road,
                *config.surface_friction.get(&SurfaceType::Road).unwrap(),
                *config.surface_stiffness.get(&SurfaceType::Road).unwrap(),
                *config
                    .surface_rolling_resistance
                    .get(&SurfaceType::Road)
                    .unwrap(),
                false,
                velocity,
                dt,
            );
        }
        tires.wheels[wheel as usize].longitudinal_force
    };

    let full = run(1.0);
    let worn = run(0.6);
    assert!(full > 0.0);
    assert!(
        worn < full * 0.75,
        "worn grip must reduce delivered force: full={full} worn={worn}"
    );
}

#[test]
fn wear_profile_parses_front_and_rear_independently() {
    let json = r#"{
        "tires": {
            "wear": {
                "tread_wear_per_megajoule_millimeters": 0.5,
                "cliff_start_wear_fraction": 0.7,
                "front": { "tread_wear_per_megajoule_millimeters": 0.8 },
                "rear": { "tread_wear_per_megajoule_millimeters": 0.4, "end_of_life_grip_scale": 0.45 }
            }
        }
    }"#;
    let config = VehicleConfig::from_json_str(json).expect("wear profile must parse");
    assert!((config.tire_wear.front.tread_wear_per_megajoule_millimeters - 0.8).abs() < 1e-9);
    assert!((config.tire_wear.rear.tread_wear_per_megajoule_millimeters - 0.4).abs() < 1e-9);
    assert!((config.tire_wear.rear.end_of_life_grip_scale - 0.45).abs() < 1e-9);
    assert!((config.tire_wear.front.end_of_life_grip_scale - 0.55).abs() < 1e-9);
    assert!((config.tire_wear.front.cliff_start_wear_fraction - 0.7).abs() < 1e-9);
}

fn flat_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut out = [TriRaycastSample::default(); 4];
    for wheel in WheelIndex::ALL {
        let index = wheel as usize;
        let anchor_world = sim.state.transform.transform_point(sim.config.wheel_anchor_local(wheel));
        let distance = anchor_world.y.max(0.0);
        let span = if wheel.is_front() {
            sim.config.front_tire_width
        } else {
            sim.config.rear_tire_width
        } * sim.config.tri_ray_spacing_ratio;
        let side = if wheel.is_left() { 1.0 } else { -1.0 };
        let hit = |offset_x: f64| RaycastHit {
            is_colliding: true,
            distance,
            point: Vec3::new(anchor_world.x + offset_x, 0.0, anchor_world.z),
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        };
        out[index] = TriRaycastSample {
            inner: hit(side * span),
            center: hit(0.0),
            outer: hit(-side * span),
        };
    }
    out
}

#[test]
fn canon_profile_wears_under_a_cornering_stint() {
    let config = VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .expect("checked-in f1_2030 geometric profile must be valid");
    assert!(config.tire_wear.front.is_enabled());
    assert!(config.tire_wear.rear.is_enabled());

    let mut sim = VehicleSimulator::new(config, Vec3::new(0.0, 0.40, 0.0), 0.0);
    let dt = 1.0 / 120.0;
    let steps = 90 * 120;
    for step in 0..steps {
        let time_s = step as f64 * dt;
        let speed = sim.state.linear_velocity.length();
        let input = VehicleInput {
            throttle: if speed > 42.0 { 0.0 } else { 1.0 },
            brake: if speed > 46.0 { 0.5 } else { 0.0 },
            steering: 0.22 * (time_s * std::f64::consts::PI).sin(),
            ..VehicleInput::default()
        };
        let samples = flat_samples(&sim);
        sim.step(&input, &samples, dt);
    }

    let front_left = &sim.state.tire_wear.wheels[WheelIndex::FrontLeft as usize];
    let rear_left = &sim.state.tire_wear.wheels[WheelIndex::RearLeft as usize];
    println!(
        "wear front_left inner={:.4} center={:.4} outer={:.4} grip={:.4}",
        front_left.inner_wear_fraction,
        front_left.center_wear_fraction,
        front_left.outer_wear_fraction,
        front_left.wear_grip_scale
    );
    println!(
        "wear rear_left inner={:.4} center={:.4} outer={:.4} grip={:.4}",
        rear_left.inner_wear_fraction,
        rear_left.center_wear_fraction,
        rear_left.outer_wear_fraction,
        rear_left.wear_grip_scale
    );
    for wheel in WheelIndex::ALL {
        let state = &sim.state.tire_wear.wheels[wheel as usize];
        println!(
            "wear work {wheel:?} joules={:.0} speed_kmh={:.1}",
            state.accumulated_friction_work_joules,
            sim.state.linear_velocity.length() * 3.6
        );
    }
    let front_left_wear_millimeters = front_left.inner_wear_fraction
        * sim.config.tire_wear.front.usable_tread_millimeters();
    println!(
        "wear front_left_mm_per_90s_lap={:.3}",
        front_left_wear_millimeters
    );
    assert!(
        front_left.inner_wear_fraction > 0.0,
        "canon stint must wear the tread"
    );
    assert!(
        rear_left.inner_wear_fraction > 0.0,
        "canon stint must wear the rear tread"
    );
    assert!(front_left.remaining_tread_fraction() < 1.0);
    assert!(
        front_left.wear_grip_scale < 1.0,
        "canon stint must reduce the delivered grip scale"
    );
    assert!(
        (0.02..0.5).contains(&front_left_wear_millimeters),
        "canon stint wear must stay inside the calibrated band: {front_left_wear_millimeters} mm/lap"
    );
    assert!(
        front_left.accumulated_friction_work_joules > 100_000.0,
        "canon stint must accumulate meaningful friction work"
    );
}

#[test]
fn invalid_wear_profile_is_rejected() {
    let json = r#"{
        "tires": {
            "wear": {
                "initial_tread_depth_millimeters": 4.0,
                "minimum_tread_depth_millimeters": 4.5
            }
        }
    }"#;
    assert!(VehicleConfig::from_json_str(json).is_err());
}
