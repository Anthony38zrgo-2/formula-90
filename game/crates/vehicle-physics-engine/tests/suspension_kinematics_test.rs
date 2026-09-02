use vehicle_physics_engine::*;

fn flat_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
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
fn toe_gain_zero_preserves_legacy_static_steer() {
    let cfg = VehicleConfig::f1_94_canonical();
    // At static/rest travel the signed per-wheel angle equals the legacy static
    // toe expression, and L/R stays axle-neutral on every axle (front+rear).
    let fl = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, 0.0, 0.0);
    let fr = steering_angle_for_wheel(&cfg, WheelIndex::FrontRight, 0.0, 0.0);
    let rl = steering_angle_for_wheel(&cfg, WheelIndex::RearLeft, 0.0, 0.0);
    let rr = steering_angle_for_wheel(&cfg, WheelIndex::RearRight, 0.0, 0.0);
    assert!(
        (fl - -cfg.front_toe).abs() < 1e-12,
        "left wheel must negate signed toe, got {fl}"
    );
    assert!((fr - cfg.front_toe).abs() < 1e-12);
    assert!((fl + fr).abs() < 1e-12, "front axle must be neutral at rest");
    assert!((rl + rr).abs() < 1e-12, "rear axle must be neutral at rest");
    // Gains default to 0.0: travel must not affect the angle at all.
    let bump = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, 0.0, 0.05);
    assert!((bump - fl).abs() < 1e-12);
}

#[test]
fn toe_gain_bump_rebound_and_rear_zero_ratio() {
    let mut cfg = VehicleConfig::f1_94_canonical();
    cfg.front_steering_ratio = 1.0;
    cfg.rear_steering_ratio = 0.0;
    cfg.front_toe = 0.0;
    cfg.rear_toe = 0.0;
    cfg.front_toe_gain_rad_per_m = -0.02;
    cfg.rear_toe_gain_rad_per_m = 0.012;

    // Bump +20 mm on the front-left: effective toe = -0.02 * 0.020 = -0.0004 rad.
    let bump_left = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, 0.0, 0.020);
    assert!((bump_left - 0.0004).abs() < 1e-12, "bump steer sign, got {bump_left}");
    // Rebound -10 mm: effective toe = -0.02 * -0.010 = +0.0002 rad.
    let rebound_left = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, 0.0, -0.010);
    assert!(
        (rebound_left - -0.0002).abs() < 1e-12,
        "rebound must invert the toe shift, got {rebound_left}"
    );
    assert!(
        rebound_left < 0.0 && bump_left > 0.0,
        "bump and rebound must shift toe in opposite directions"
    );

    // Rear axle must honour toe gain even with rear_steering_ratio = 0.
    let rear = steering_angle_for_wheel(&cfg, WheelIndex::RearRight, 0.0, 0.020);
    assert!(
        (rear - 0.012 * 0.020).abs() < 1e-12,
        "rear toe gain must work at zero steering ratio, got {rear}"
    );

    // Symmetric travel on an axle keeps it neutral (L/R mirror).
    let l = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, 0.0, 0.030);
    let r = steering_angle_for_wheel(&cfg, WheelIndex::FrontRight, 0.0, 0.030);
    assert!((l + r).abs() < 1e-12, "symmetric bump must not yaw the axle");

    // effective_toe_rad is the raw toe (no left/right mirror): symmetric travel
    // gives identical values, while the mirrored calling angles stay neutral.
    let eff_l = effective_toe_rad(&cfg, WheelIndex::FrontLeft, 0.030);
    let eff_r = effective_toe_rad(&cfg, WheelIndex::FrontRight, 0.030);
    assert!((eff_l - eff_r).abs() < 1e-12, "raw toe is symmetric on flat");
    assert!((l - -eff_l).abs() < 1e-12, "left angle must be the negated toe");
}

#[test]
fn toe_gain_past_clamp_is_bounded() {
    let mut cfg = VehicleConfig::f1_94_canonical();
    cfg.front_steering_ratio = 1.0;
    cfg.front_toe = 0.0;
    cfg.front_toe_gain_rad_per_m = -0.02;
    let angle = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, 0.0, 10.0);
    assert!(
        angle.abs() <= MAX_EFFECTIVE_TOE_RAD + 1e-12,
        "corrupt gain/travel must be clamped, got {angle}"
    );
    assert!(
        (effective_toe_rad(&cfg, WheelIndex::FrontLeft, 10.0) + MAX_EFFECTIVE_TOE_RAD).abs()
            < 1e-12,
        "static+dynamic total must clamp at the final bound"
    );
}

#[test]
fn one_wheel_curb_transient_toe_stays_smooth_and_asymmetric() {
    let mut cfg = VehicleConfig::f1_94_canonical();
    cfg.front_toe_gain_rad_per_m = -0.02;
    cfg.rear_toe_gain_rad_per_m = 0.012;
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::new(0.0, spawn, 0.0), 0.0);
    let dt = 1.0 / 120.0;
    let input = VehicleInput::default();

    // Settle on a flat road.
    for _ in 0..120 {
        let samples = flat_samples(&sim);
        let _ = sim.step(&input, &samples, dt);
    }

    // 40 mm curb under the front-left wheel only.
    let mut max_abs_step = 0.0f64;
    let mut max_front_asymmetry = 0.0f64;
    let mut prev: Option<[f64; 4]> = None;
    for _ in 0..120 {
        let mut samples = flat_samples(&sim);
        samples[0].inner.distance -= 0.040;
        samples[0].center.distance -= 0.040;
        samples[0].outer.distance -= 0.040;
        let telem = sim.step(&input, &samples, dt);
        let toe = telem.wheel_effective_toe_rad;
        for v in toe.iter() {
            assert!(
                v.abs() <= MAX_EFFECTIVE_TOE_RAD + 1e-9,
                "toe must stay inside the safety clamp, got {v}"
            );
        }
        if let Some(p) = prev {
            for i in 0..4 {
                max_abs_step = max_abs_step.max((toe[i] - p[i]).abs());
            }
        }
        max_front_asymmetry = max_front_asymmetry.max((toe[0] - toe[1]).abs());
        // Steering input is zero: per-wheel steer angle is exactly the negated
        // mirrored effective toe.
        assert!(
            (telem.wheel_effective_steer_angle_rad[0] + toe[0]).abs() < 1e-9,
            "steer angle must equal the mirrored toe at zero input"
        );
        // Rear axle sits on the same road: the front-only curb rolls the chassis
        // slightly, so mere-millimeter rear compression asymmetry is expected.
        assert!(
            (toe[2] - toe[3]).abs() < 5.0e-4,
            "rear axle toe must stay near-symmetric on flat, got {} vs {}",
            toe[2],
            toe[3]
        );
        prev = Some(toe);
    }
    assert!(
        max_abs_step < 0.002,
        "toe must change smoothly per tick, max step {max_abs_step}"
    );
    assert!(
        max_front_asymmetry > 1e-5,
        "the one-wheel curb must produce asymmetric front toe, max {max_front_asymmetry}"
    );
}
