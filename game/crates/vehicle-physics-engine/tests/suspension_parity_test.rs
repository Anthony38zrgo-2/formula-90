use vehicle_physics_engine::*;

fn make_hit(dist: f64) -> RaycastHit {
    RaycastHit {
        is_colliding: true,
        distance: dist,
        point: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    }
}

fn make_sample(dist: f64) -> TriRaycastSample {
    TriRaycastSample {
        inner: make_hit(dist),
        center: make_hit(dist),
        outer: make_hit(dist),
    }
}

#[test]
fn flat_static_surface_600_ticks_stability() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut suspension = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;

    // Nominal ground distance corresponding to resting compression
    let resting_dist_fl = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let resting_dist_rl = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;

    let samples = [
        make_sample(resting_dist_fl),
        make_sample(resting_dist_fl),
        make_sample(resting_dist_rl),
        make_sample(resting_dist_rl),
    ];

    let mut prev_comp = [0.0; 4];
    for tick in 0..600 {
        suspension.step(&cfg, &samples, dt);

        for w in 0..4 {
            let wheel = &suspension.wheels[w];
            assert!(wheel.is_grounded, "Wheel {} lost contact at tick {}", w, tick);
            assert!(wheel.total_normal_force.is_finite(), "Wheel {} non-finite force at tick {}", w, tick);
            assert!(wheel.total_normal_force > 0.0, "Wheel {} zero force at tick {}", w, tick);
            assert!(wheel.compression_mm.is_finite(), "Wheel {} non-finite comp at tick {}", w, tick);
            assert!(wheel.compression_mm > 0.0, "Wheel {} zero comp at tick {}", w, tick);

            if tick > 100 {
                // Ensure compression remains stable without exploding oscillation
                let delta = (wheel.compression_mm - prev_comp[w]).abs();
                assert!(delta < 0.5, "Growing oscillation detected at tick {} on wheel {}: delta={}", tick, w, delta);
            }
            prev_comp[w] = wheel.compression_mm;
        }
    }
}

#[test]
fn upward_step_increases_compression_and_settles_at_geometric_target() {
    // New mechanical semantics: compression_mm is the integrated wheel-center DOF,
    // no longer raw ray penetration. A 10 mm upward step raises the filtered road
    // target; the suspension converges to it within a short settling window instead
    // of jumping in one tick (a small part of the step is absorbed by the carcass).
    let cfg = VehicleConfig::f1_94_canonical();
    let mut suspension = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;

    let base_dist = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let initial_samples = [make_sample(base_dist); 4];

    // Settle for 120 frames
    for _ in 0..120 {
        suspension.step(&cfg, &initial_samples, dt);
    }
    let comp_before = suspension.wheels[0].compression_mm;

    // Small upward bump of 10mm (distance reduces by 0.010m)
    let bump_samples = [make_sample(base_dist - 0.010); 4];
    let mut comp_first = 0.0;
    let mut comp_last = 0.0;
    for tick in 0..60 {
        suspension.step(&cfg, &bump_samples, dt);
        let c = suspension.wheels[0].compression_mm;
        if tick == 0 {
            comp_first = c;
        }
        comp_last = c;
        assert!(
            c > comp_before,
            "Compression must increase on upward step: tick={}, c={}, before={}",
            tick,
            c,
            comp_before
        );
    }
    // The response is integrated: the first tick moves only a fraction of the step.
    assert!(
        comp_first - comp_before < 10.0,
        "First-tick response must be a fraction of the geometric step (delta={})",
        comp_first - comp_before
    );
    // ...and it converges to the geometric target minus the carcass share (the tire
    // spring is much stiffer than the suspension, so 8-10 mm of the 10 mm lands in
    // suspension travel).
    let settled = comp_last - comp_before;
    assert!(
        settled > 8.0 && settled < 10.0,
        "Compression must settle near the geometric target: before={}, after={}, delta={}",
        comp_before,
        comp_last,
        settled
    );
}

#[test]
fn downward_step_rebound_remains_bounded_by_max_spring_length() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut suspension = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;

    let base_dist = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let initial_samples = [make_sample(base_dist); 4];

    for _ in 0..20 {
        suspension.step(&cfg, &initial_samples, dt);
    }

    // Downward step of 30mm
    let drop_samples = [make_sample(base_dist + 0.030); 4];
    for _ in 0..5 {
        suspension.step(&cfg, &drop_samples, dt);
        let wheel = &suspension.wheels[0];
        assert!(
            wheel.spring_current_length <= wheel.max_spring_length + 1e-6,
            "Spring length {} exceeded max_spring_length {}",
            wheel.spring_current_length,
            wheel.max_spring_length
        );
        assert!(wheel.max_spring_length <= cfg.front_spring_length + 1e-6);
    }
}

#[test]
fn contact_outside_reachable_suspension_travel_produces_zero_force() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut suspension = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;

    // Distance far beyond max ray length (spring_length + radius + 0.50m)
    let out_of_reach_dist = cfg.front_spring_length + cfg.front_tire_radius + 0.50;
    let unreachable_samples = [make_sample(out_of_reach_dist); 4];

    suspension.step(&cfg, &unreachable_samples, dt);

    for w in 0..4 {
        let wheel = &suspension.wheels[w];
        assert!(!wheel.is_grounded, "Wheel {} must not be grounded when hit is unreachable", w);
        assert_eq!(wheel.total_normal_force, 0.0, "Wheel {} total normal force must be 0.0", w);
        assert_eq!(wheel.spring_force, 0.0);
        assert_eq!(wheel.damping_force, 0.0);
    }
}

#[test]
fn one_ray_temporarily_losing_contact_does_not_triple_load() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut suspension = SuspensionSystem::new(&cfg);
    let dt = 1.0 / 120.0;

    let resting_dist = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;

    // All 3 rays colliding
    let sample_3rays = make_sample(resting_dist);
    let samples_full = [sample_3rays, sample_3rays, sample_3rays, sample_3rays];
    suspension.step(&cfg, &samples_full, dt);
    let force_3rays = suspension.wheels[0].total_normal_force;

    // 1 of 3 rays (e.g. outer) loses contact, 2 remain
    let mut sample_2rays = make_sample(resting_dist);
    sample_2rays.outer.is_colliding = false;
    let samples_partial = [sample_2rays, sample_3rays, sample_3rays, sample_3rays];

    suspension.step(&cfg, &samples_partial, dt);
    let force_2rays = suspension.wheels[0].total_normal_force;

    assert!(force_2rays.is_finite());
    assert!(force_3rays.is_finite());
    // The force must NOT double or triple just because one ray was lost (same compression depth)
    let ratio = force_2rays / force_3rays.max(1.0);
    assert!(
        (ratio - 1.0).abs() < 0.20,
        "Force ratio {} deviated too much when 1 ray was lost (force_3={}, force_2={})",
        ratio,
        force_3rays,
        force_2rays
    );
}
