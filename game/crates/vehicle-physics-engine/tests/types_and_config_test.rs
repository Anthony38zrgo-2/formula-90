use vehicle_physics_engine::*;

#[test]
fn vec3_default_is_zero() {
    assert_eq!(Vec3::default(), Vec3::ZERO);
}

#[test]
fn tri_raycast_uses_center_biased_weighting() {
    let hit = |distance: f64| RaycastHit {
        is_colliding: true,
        distance,
        point: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    let sample = TriRaycastSample {
        inner: hit(0.35),
        center: hit(0.30),
        outer: hit(0.25),
    };
    assert!((sample.weighted_distance(1.0) - 0.30).abs() < 1e-9);
}

#[test]
fn configured_center_of_mass_is_live() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert_eq!(center_of_mass_local(&cfg).y, cfg.center_of_gravity_height_offset);
    assert!(cfg.center_of_gravity_height_offset < 0.0);
}

#[test]
fn spawn_height_accounts_for_both_axles() {
    let cfg = VehicleConfig::f1_94_canonical();
    let front = cfg.front_tire_radius;
    let rear = cfg.rear_tire_radius;
    let h = default_spawn_height(&cfg);
    assert!(h >= front.min(rear) && h <= front.max(rear));
}

#[test]
fn spring_rate_supports_static_wheel_load() {
    let cfg = VehicleConfig::f1_94_canonical();
    for wheel in WheelIndex::ALL {
        let k = cfg.calculate_spring_rate(wheel);
        let travel = if wheel.is_front() {
            cfg.front_spring_length * cfg.front_resting_ratio
        } else {
            cfg.rear_spring_length * cfg.rear_resting_ratio
        };
        let supported = k * travel;
        let expected = cfg.mass_over_wheel(wheel) * 9.80665;
        assert!((supported - expected).abs() < 1e-6);
    }
}

#[test]
fn aero_pitch_moment_is_zero_around_center_of_mass() {
    let cfg = VehicleConfig::f1_94_canonical();
    let cg = center_of_mass_local(&cfg);

    // Front wing at front axle center, Diffuser at (0, -0.12, 0), Rear wing at rear axle center
    let r_front_z = -cfg.wheelbase * 0.5 - cg.z;
    let r_diff_z = 0.0 - cg.z;
    let r_rear_z = cfg.wheelbase * 0.5 - cg.z;

    // Pitch torque per unit of downforce: tau = sum(r_z * (-F_split)) -> net moment
    let net_pitch_moment_arm = cfg.aero_split_front * r_front_z
        + cfg.aero_split_diffuser * r_diff_z
        + cfg.aero_split_rear * r_rear_z;

    assert!(
        net_pitch_moment_arm.abs() < 1e-6,
        "Aerodynamic center of pressure must align with center of mass, net arm={}",
        net_pitch_moment_arm
    );
}
