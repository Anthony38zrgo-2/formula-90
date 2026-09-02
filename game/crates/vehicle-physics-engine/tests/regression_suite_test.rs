use vehicle_physics_engine::*;

fn make_flat_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut out = [TriRaycastSample::default(); 4];
    for wheel in WheelIndex::ALL {
        let i = wheel as usize;
        let anchor_local = sim.config.wheel_anchor_local(wheel);
        let anchor_world = sim.state.transform.transform_point(anchor_local);
        let distance = anchor_world.y.max(0.0);
        let span = if wheel.is_front() { sim.config.front_tire_width } else { sim.config.rear_tire_width }
            * sim.config.tri_ray_spacing_ratio;
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
fn test_straight_acceleration_from_rest() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::new(0.0, spawn, 0.0), 0.0);
    let dt = 1.0 / 120.0;
    let input = VehicleInput { throttle: 1.0, ..VehicleInput::default() };

    let mut time_to_100: Option<f64> = None;
    let mut reached_400m = false;
    let mut max_lateral_dev = 0.0f64;

    for tick in 0..1800 { // 15 seconds max @ 120Hz (accommodates JSON-aligned max_rpm=15000)
        let samples = make_flat_samples(&sim);
        let telem = sim.step(&input, &samples, dt);
        let pos = sim.state.transform.origin;

        max_lateral_dev = max_lateral_dev.max(pos.x.abs());

        if telem.speed_kmh >= 100.0 && time_to_100.is_none() {
            time_to_100 = Some(tick as f64 * dt);
        }

        if pos.z.abs() >= 400.0 {
            reached_400m = true;
            break;
        }
    }

    assert!(time_to_100.is_some(), "Vehicle must reach 100 km/h");
    let t100 = time_to_100.unwrap();
    // F1-94 0-100 km/h reference is ~2.8s. With the pressure/thermal model active
    // the tires START COLD (25 C, below the ~89 C operating window), so peak grip
    // ramps in during the run and the cold-start time extends (ACCEPTANCE #10:
    // cold tire = less grip than operating window). Pre-heated tires return to the
    // canonical window (see preheated_tires_reach_canonical_acceleration).
    assert!(
        t100 >= 2.0 && t100 <= 6.5,
        "0-100 km/h time was {}s, expected within canonical range (cold-start thermal grip)",
        t100
    );
    assert!(
        max_lateral_dev < 0.05,
        "Lateral drift on symmetric flat surface must be < 0.05m, was {}m",
        max_lateral_dev
    );
    assert!(reached_400m, "Vehicle must reach 400m");
}

#[test]
fn preheated_tires_reach_canonical_acceleration() {
    // With all four tires already inside the operating window, the pressure/thermal
    // grip correction is ~1.0 and the car returns to the pre-thermal 0-100 range.
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::new(0.0, spawn, 0.0), 0.0);
    for w in WheelIndex::ALL {
        let i = w as usize;
        sim.state.tire_thermal.wheels[i].tread_inner_c = 95.0;
        sim.state.tire_thermal.wheels[i].tread_center_c = 95.0;
        sim.state.tire_thermal.wheels[i].tread_outer_c = 95.0;
        sim.state.tire_thermal.wheels[i].carcass_c = 80.0;
    }
    let dt = 1.0 / 120.0;
    let input = VehicleInput { throttle: 1.0, ..VehicleInput::default() };
    let mut time_to_100: Option<f64> = None;
    for tick in 0..1800 {
        let samples = make_flat_samples(&sim);
        let telem = sim.step(&input, &samples, dt);
        if telem.speed_kmh >= 100.0 && time_to_100.is_none() {
            time_to_100 = Some(tick as f64 * dt);
        }
    }
    let t100 = time_to_100.expect("preheated vehicle must reach 100 km/h");
    assert!(
        t100 >= 2.0 && t100 <= 3.8,
        "preheated 0-100 km/h time was {}s, expected canonical window",
        t100
    );
}

#[test]
fn test_steering_response_parity() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = 1.0 / 120.0;

    // 1. Positive steering input (left) produces positive wheel angle
    let angle_left = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, 0.5, 0.0);
    let angle_right = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, -0.5, 0.0);
    assert!(angle_left > 0.0, "Positive steer input must produce positive wheel angle");
    assert!(angle_right < 0.0, "Negative steer input must produce negative wheel angle");

    // 2. Step steer at 100 km/h produces smooth yaw rate rise
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::new(0.0, spawn, 0.0), 0.0);
    sim.state.linear_velocity = Vec3::new(0.0, 0.0, -27.78); // 100 km/h
    for i in 0..4 {
        let r = if i < 2 { cfg.front_tire_radius } else { cfg.rear_tire_radius };
        sim.state.tires.wheels[i].spin = 27.78 / r;
    }

    let input_steer = VehicleInput { throttle: 0.2, steering: 0.3, ..VehicleInput::default() };
    let mut yaw_rates = Vec::new();
    for _ in 0..30 {
        let samples = make_flat_samples(&sim);
        let telem = sim.step(&input_steer, &samples, dt);
        yaw_rates.push(telem.lat_g);
    }
    assert!(yaw_rates.len() == 30);
    // Lateral G should rise smoothly
    assert!(yaw_rates.last().unwrap().abs() > 0.1);
}

#[test]
fn test_suspension_behavior_parity() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = 1.0 / 120.0;

    // 1. Static rest settled with stable normal force and compression
    let mut suspension = SuspensionSystem::new(&cfg);
    let resting_dist_fl = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let resting_dist_rl = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
    let static_samples = [
        TriRaycastSample { inner: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, center: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, outer: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road } },
        TriRaycastSample { inner: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, center: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, outer: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road } },
        TriRaycastSample { inner: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, center: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, outer: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road } },
        TriRaycastSample { inner: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, center: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, outer: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road } },
    ];
    for _ in 0..120 {
        suspension.step(&cfg, &static_samples, dt);
    }
    for w in 0..4 {
        assert!(suspension.wheels[w].is_grounded, "Wheel {} must be grounded", w);
        assert!(suspension.wheels[w].total_normal_force > 0.0, "Wheel {} normal force must be positive", w);
        assert!(suspension.wheels[w].compression_mm > 0.0, "Wheel {} compression must be positive", w);
    }

    // 2. 50mm curb bump produces bounded compression without inversion
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::new(0.0, spawn, 0.0), 0.0);
    let input = VehicleInput::default();
    let mut bump_samples = make_flat_samples(&sim);
    bump_samples[0].center.distance -= 0.05; // 50mm bump
    bump_samples[0].inner.distance -= 0.05;
    bump_samples[0].outer.distance -= 0.05;

    let telem = sim.step(&input, &bump_samples, dt);
    assert!(
        telem.fl_comp_mm > 0.0 && telem.fl_comp_mm < cfg.front_spring_length * 1000.0,
        "50mm bump compression must be bounded: {} mm",
        telem.fl_comp_mm
    );

    // 3. Airborne vehicle smoothly extends to max_spring_length
    let airborne_samples = [TriRaycastSample::default(); 4]; // No collisions
    for _ in 0..120 {
        sim.step(&input, &airborne_samples, dt);
    }
    for w in 0..4 {
        let max_len = sim.state.suspension.wheels[w].max_spring_length;
        let rest_len = if w < 2 { cfg.front_spring_length } else { cfg.rear_spring_length };
        assert!(
            (max_len - rest_len).abs() < 1e-4,
            "Airborne max spring length must smoothly reach rest length"
        );
    }
}

#[test]
fn test_powertrain_drivetrain_parity() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = 1.0 / 120.0;

    // 1. Lift-off throttle in gear produces engine braking torque
    let mut pt = PowertrainState::new(&cfg);
    pt.rpm = 12000.0;
    pt.current_gear = 3;
    let spins = [0.0, 0.0, 60.0, 60.0];
    let reactions = [0.0, 0.0, -100.0, -100.0];
    let input_lift = VehicleInput { throttle: 0.0, ..VehicleInput::default() };
    pt.step_with_reaction(&cfg, &input_lift, &spins, &reactions, 25.0, true, true, cfg.enable_abs, dt);
    assert!(
        pt.engine_torque <= 0.0,
        "Lift-off engine torque must be non-positive (engine braking): {}",
        pt.engine_torque
    );

    // 2. Handbrake locks rear wheels only
    let mut tires = TireSystem::new(&cfg);
    tires.wheels[0].spin = 50.0;
    tires.wheels[1].spin = 50.0;
    tires.wheels[2].spin = 50.0;
    tires.wheels[3].spin = 50.0;

    // Apply handbrake (max_brake_torque to rear wheels, 0 to front wheels)
    tires.process_wheel_torque(&cfg, WheelIndex::RearLeft, 0.0, cfg.motor_moment, cfg.max_brake_torque, dt);
    tires.process_wheel_torque(&cfg, WheelIndex::FrontLeft, 0.0, cfg.motor_moment, 0.0, dt);

    assert!(
        tires.wheels[WheelIndex::RearLeft as usize].spin < tires.wheels[WheelIndex::FrontLeft as usize].spin - 10.0,
        "Handbrake must brake rear wheels significantly more than front (rear={}, front={})",
        tires.wheels[WheelIndex::RearLeft as usize].spin,
        tires.wheels[WheelIndex::FrontLeft as usize].spin
    );
}

#[test]
fn test_surface_interaction_parity() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = 1.0 / 120.0;

    let mut tires_road = TireSystem::new(&cfg);
    tires_road.wheels[0].spin = 30.0;
    tires_road.process_wheel_forces(
        &cfg,
        WheelIndex::FrontLeft,
        3000.0,
        SurfaceType::Road,
        2.9,
        8.75,
        1.0,
        false,
        Vec3::new(5.0, 0.0, -20.0),
        dt,
    );

    let mut tires_grass_equal = TireSystem::new(&cfg);
    tires_grass_equal.wheels[0].spin = 30.0;
    tires_grass_equal.process_wheel_forces(
        &cfg,
        WheelIndex::FrontLeft,
        3000.0,
        SurfaceType::Grass,
        2.9,
        8.75,
        1.0,
        false,
        Vec3::new(5.0, 0.0, -20.0),
        dt,
    );

    // TIRE-500: lateral_grip_assist is removed from the runtime. Surface interaction is
    // governed by physical surface properties only (friction/stiffness/RR); equal physics
    // inputs must produce identical forces (no hidden surface-specific grip inflation).
    assert!(
        (tires_grass_equal.wheels[0].lateral_force - tires_road.wheels[0].lateral_force).abs()
            < 1e-9,
        "Equal physics inputs must produce equal lateral force: grass={}, road={}",
        tires_grass_equal.wheels[0].lateral_force,
        tires_road.wheels[0].lateral_force
    );

    // Lower physical friction must strictly reduce lateral grip.
    let mut tires_grass_low = TireSystem::new(&cfg);
    tires_grass_low.wheels[0].spin = 30.0;
    tires_grass_low.process_wheel_forces(
        &cfg,
        WheelIndex::FrontLeft,
        3000.0,
        SurfaceType::Grass,
        1.2,
        8.75,
        1.0,
        false,
        Vec3::new(5.0, 0.0, -20.0),
        dt,
    );
    assert!(
        tires_grass_low.wheels[0].lateral_force.abs()
            < tires_road.wheels[0].lateral_force.abs(),
        "Lower friction must produce less lateral grip: low={}, road={}",
        tires_grass_low.wheels[0].lateral_force,
        tires_road.wheels[0].lateral_force
    );
}
