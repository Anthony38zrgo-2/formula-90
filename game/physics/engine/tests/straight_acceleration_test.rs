use vehicle_physics_engine::*;

/// Helper to simulate a flat ground (plane at Y = 0.0) with accurate wheel hub sampling.
fn flat_ground_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
    let mut samples = [TriRaycastSample::default(); 4];
    for i in 0..4 {
        let wheel = WheelIndex::ALL[i];
        let hub_local = sim.config.wheel_anchor_local(wheel);
        let hub_world = sim.state.transform.transform_point(hub_local);
        
        let tire_w = if wheel.is_front() { sim.config.front_tire_width } else { sim.config.rear_tire_width };
        let span = tire_w * sim.config.tri_ray_spacing_ratio;

        let left_offset = sim.state.transform.basis.transform_vector(Vec3::new(-span, 0.0, 0.0));
        let right_offset = sim.state.transform.basis.transform_vector(Vec3::new(span, 0.0, 0.0));

        let p_in = hub_world + left_offset;
        let p_mid = hub_world;
        let p_out = hub_world + right_offset;

        samples[i] = TriRaycastSample {
            inner: RaycastHit {
                is_colliding: true,
                distance: p_in.y.max(0.0),
                point: Vec3::new(p_in.x, 0.0, p_in.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            center: RaycastHit {
                is_colliding: true,
                distance: p_mid.y.max(0.0),
                point: Vec3::new(p_mid.x, 0.0, p_mid.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
            outer: RaycastHit {
                is_colliding: true,
                distance: p_out.y.max(0.0),
                point: Vec3::new(p_out.x, 0.0, p_out.z),
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            },
        };
    }
    samples
}

#[test]
fn test_straight_line_acceleration() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn_height = cfg.front_tire_radius + cfg.front_spring_length * (1.0 - cfg.front_resting_ratio);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn_height, 0.0), 0.0);

    let dt = 1.0 / 60.0;
    let mut last_telemetry = None;

    // Simulate 3.5 seconds of full throttle
    for _ in 0..210 {
        let input = VehicleInput {
            throttle: 1.0,
            steering: 0.0,
            brake: 0.0,
            handbrake: 0.0,
            clutch: 0.0,
            gear_request: None,
        };
        let samples = flat_ground_samples(&sim);
        let telem = sim.step(&input, &samples, dt);
        last_telemetry = Some(telem);
    }

    let t = last_telemetry.expect("Telemetry produced");
    println!("Telemetry at 3.5s: Speed={:.1} km/h, RPM={:.0}, Gear={}", t.speed_kmh, t.rpm, t.gear);

    // Car must have accelerated substantially from standstill
    assert!(t.speed_kmh > 80.0, "Car should reach >80 km/h in 3.5s, got {:.1}", t.speed_kmh);
    assert!(t.gear >= 2, "Automatic gearbox should have shifted into at least 2nd gear");
    assert!(t.rpm >= 3500.0, "RPM should stay above idle");

    // CSV format validation
    let csv = t.to_csv_line();
    assert!(csv.contains("Jordan 197") || csv.contains("rust_physics_benchmark"));
    assert_eq!(csv.split(',').count(), 26);
}

#[test]
fn test_braking_deceleration() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn_height = cfg.front_tire_radius + cfg.front_spring_length * (1.0 - cfg.front_resting_ratio);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn_height, 0.0), 0.0);

    let dt = 1.0 / 60.0;

    // 1. Accelerate for 2.0 seconds
    for _ in 0..120 {
        let input = VehicleInput {
            throttle: 1.0,
            steering: 0.0,
            brake: 0.0,
            handbrake: 0.0,
            clutch: 0.0,
            gear_request: None,
        };
        let samples = flat_ground_samples(&sim);
        sim.step(&input, &samples, dt);
    }

    let speed_before_braking = (-sim.state.transform.basis.inverse_transform_vector(sim.state.linear_velocity).z) * 3.6;
    assert!(speed_before_braking > 40.0, "Speed before braking was {:.1}", speed_before_braking);

    // 2. Full brake for 1.5 seconds
    for _ in 0..90 {
        let input = VehicleInput {
            throttle: 0.0,
            steering: 0.0,
            brake: 1.0,
            handbrake: 0.0,
            clutch: 1.0,
            gear_request: None,
        };
        let samples = flat_ground_samples(&sim);
        sim.step(&input, &samples, dt);
    }

    let speed_after_braking = (-sim.state.transform.basis.inverse_transform_vector(sim.state.linear_velocity).z) * 3.6;
    println!("Speed before brake={:.1} km/h, after={:.1} km/h", speed_before_braking, speed_after_braking);

    // Speed should have dropped significantly
    assert!(speed_after_braking < speed_before_braking * 0.4);
}

#[test]
fn test_tri_raycast_curb_compliance() {
    let mut sample = TriRaycastSample::default();
    
    // Road baseline hit at 0.374m
    let road_hit = RaycastHit {
        is_colliding: true,
        distance: 0.374,
        point: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };

    // Curb hit at 0.324m (50mm raised curb on outer edge)
    let curb_hit = RaycastHit {
        is_colliding: true,
        distance: 0.324,
        point: Vec3::new(0.15, 0.05, 0.0),
        normal: Vec3::new(-0.2, 0.98, 0.0).normalized(),
        surface: SurfaceType::Curb,
    };

    sample.inner = road_hit;
    sample.center = road_hit;
    sample.outer = curb_hit;

    // Weighted distance with 1:2:1 weighting
    let w_dist = sample.weighted_distance(0.5);
    // (0.374 + 2*0.374 + 0.324) / 4 = 1.446 / 4 = 0.3615 m
    assert!((w_dist - 0.3615).abs() < 1e-5);

    // Blended normal should tilt slightly towards the curb normal
    let w_norm = sample.weighted_normal();
    assert!(w_norm.x < 0.0, "Weighted normal should account for curb edge angle");
}
