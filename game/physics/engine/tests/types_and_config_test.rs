use vehicle_physics_engine::*;

#[test]
fn test_vec3_operations() {
    let v1 = Vec3::new(1.0, 2.0, 3.0);
    let v2 = Vec3::new(4.0, 5.0, 6.0);

    let sum = v1 + v2;
    assert_eq!(sum, Vec3::new(5.0, 7.0, 9.0));

    let diff = v2 - v1;
    assert_eq!(diff, Vec3::new(3.0, 3.0, 3.0));

    let dot = v1.dot(v2);
    assert_eq!(dot, 1.0 * 4.0 + 2.0 * 5.0 + 3.0 * 6.0); // 32.0

    let cross = Vec3::RIGHT.cross(Vec3::UP);
    assert_eq!(cross, Vec3::BACK); // (1,0,0) x (0,1,0) = (0,0,1) = BACK in Godot
}

#[test]
fn test_quat_rotation() {
    let q = Quat::from_mat3(&Mat3::from_euler_yxz(std::f64::consts::PI * 0.5, 0.0, 0.0)); // 90 deg yaw
    let rotated = q.rotate_vec3(Vec3::FORWARD); // Forward is (0, 0, -1)
    
    // Rotating (0, 0, -1) by 90 deg counter-clockwise around Y should point LEFT (-1, 0, 0)
    assert!((rotated.x - (-1.0)).abs() < 1e-5);
    assert!(rotated.y.abs() < 1e-5);
    assert!(rotated.z.abs() < 1e-5);
}

#[test]
fn test_tri_raycast_sample_weighted_distance() {
    let mut sample = TriRaycastSample::default();
    sample.inner = RaycastHit {
        is_colliding: true,
        distance: 0.35,
        point: Vec3::new(-0.15, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    sample.center = RaycastHit {
        is_colliding: true,
        distance: 0.30,
        point: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    sample.outer = RaycastHit {
        is_colliding: true,
        distance: 0.25,
        point: Vec3::new(0.15, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Curb,
    };

    assert!(sample.has_any_contact());
    assert_eq!(sample.contact_count(), 3);

    // Weighted distance = (0.35 + 2*0.30 + 0.25) / 4 = 1.20 / 4 = 0.30
    let w_dist = sample.weighted_distance(1.0);
    assert!((w_dist - 0.30).abs() < 1e-6);
}

#[test]
fn test_tri_raycast_weighted_distance_non_colliding_outer() {
    let mut sample = TriRaycastSample::default();
    sample.inner = RaycastHit {
        is_colliding: true,
        distance: 0.35,
        point: Vec3::new(-0.15, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    sample.center = RaycastHit {
        is_colliding: true,
        distance: 0.30,
        point: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    sample.outer = RaycastHit {
        is_colliding: false,
        distance: 0.99,
        point: Vec3::new(0.15, 0.0, 0.0),
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };

    assert!(sample.has_any_contact());
    assert_eq!(sample.contact_count(), 2);

    // Only inner+center colliding: (0.35 + 2*0.30) / 3 = 0.95 / 3 ≈ 0.31667
    let w_dist = sample.weighted_distance(1.0);
    let expected = (0.35 + 2.0 * 0.30) / 3.0;
    assert!((w_dist - expected).abs() < 1e-6, "Expected {:.6}, got {:.6}", expected, w_dist);
}

#[test]
fn test_tri_raycast_weighted_distance_only_center() {
    let mut sample = TriRaycastSample::default();
    sample.inner = RaycastHit {
        is_colliding: false,
        distance: 0.99,
        point: Vec3::ZERO,
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    sample.center = RaycastHit {
        is_colliding: true,
        distance: 0.28,
        point: Vec3::ZERO,
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    sample.outer = RaycastHit {
        is_colliding: false,
        distance: 0.99,
        point: Vec3::ZERO,
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };

    // Only center: distance = 0.28, weight = 2.0
    let w_dist = sample.weighted_distance(1.0);
    assert!((w_dist - 0.28).abs() < 1e-6);
}

#[test]
fn test_tri_raycast_weighted_distance_no_collision() {
    let sample = TriRaycastSample::default();
    // No rays colliding: should return max_length
    let w_dist = sample.weighted_distance(0.65);
    assert!((w_dist - 0.65).abs() < 1e-6);
}

#[test]
fn test_tri_raycast_weighted_normal_non_colliding_outer() {
    let mut sample = TriRaycastSample::default();
    let tilted_normal = Vec3::new(-0.2, 0.98, 0.0).normalized();
    sample.inner = RaycastHit {
        is_colliding: true,
        distance: 0.35,
        point: Vec3::ZERO,
        normal: tilted_normal,
        surface: SurfaceType::Road,
    };
    sample.center = RaycastHit {
        is_colliding: true,
        distance: 0.30,
        point: Vec3::ZERO,
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };
    sample.outer = RaycastHit {
        is_colliding: false,
        distance: 0.99,
        point: Vec3::ZERO,
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    };

    let w_norm = sample.weighted_normal();
    // Should only use inner (weight 1) + center (weight 2)
    assert!(w_norm.x < 0.0, "Weighted normal should be tilted by inner ray normal, got x={}", w_norm.x);
    assert!((w_norm.length() - 1.0).abs() < 1e-6, "Normal should be unit length");
}

#[test]
fn test_vehicle_config_defaults() {
    let cfg = VehicleConfig::default();
    assert_eq!(cfg.vehicle_name, "F1 1994 (V10)");
    assert_eq!(cfg.vehicle_mass, 505.0);
    assert_eq!(cfg.gear_ratios.len(), 6);

    // Torque curve evaluation
    let t_idle = cfg.evaluate_torque_curve(0.0);
    assert_eq!(t_idle, 0.38);

    let t_peak = cfg.evaluate_torque_curve(0.82);
    assert_eq!(t_peak, 1.00);

    let t_mid = cfg.evaluate_torque_curve(0.45);
    assert_eq!(t_mid, 0.82);

    // Mass distribution
    let fl_mass = cfg.mass_over_wheel(WheelIndex::FrontLeft);
    let rl_mass = cfg.mass_over_wheel(WheelIndex::RearLeft);
    assert_eq!(fl_mass, 505.0 * 0.45 * 0.5);
    assert_eq!(rl_mass, 505.0 * 0.55 * 0.5);

    // Spring rates
    let k_front = cfg.calculate_spring_rate(WheelIndex::FrontLeft);
    let k_rear = cfg.calculate_spring_rate(WheelIndex::RearLeft);
    assert!(k_front > 10000.0 && k_front < 100000.0);
    assert!(k_rear > 10000.0 && k_rear < 100000.0);
}

#[test]
fn test_f1_94_rear_damping_ratio_spec() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert_eq!(cfg.rear_damping_ratio, 0.80, "Spec 2.2: rear zeta must be 0.80, not 0.85");
    assert_eq!(cfg.front_damping_ratio, 0.80, "Front zeta must be 0.80");
}

#[test]
fn test_f1_94_arb_ratios_spec() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert_eq!(cfg.front_arb_ratio, 0.20, "Spec 2.5: front ARB ratio must be 0.20");
    assert_eq!(cfg.rear_arb_ratio, 0.05, "Spec 2.5: rear ARB ratio must be 0.05");
}

#[test]
fn test_f1_94_spring_rates_numerical() {
    let cfg = VehicleConfig::f1_94_canonical();
    let k_front = cfg.calculate_spring_rate(WheelIndex::FrontLeft);
    let k_rear = cfg.calculate_spring_rate(WheelIndex::RearLeft);

    // k_front = (505 * 0.45 * 0.5) * 9.80665 / (0.250 * 0.400) = 1114.28 / 0.100 = 11143 N/m
    let expected_front = (505.0 * 0.45 * 0.5) * 9.80665 / (0.250 * 0.400);
    assert!((k_front - expected_front).abs() < 1.0, "k_front expected {:.1}, got {:.1}", expected_front, k_front);

    // k_rear = (505 * 0.55 * 0.5) * 9.80665 / (0.180 * 0.350) = 1361.93 / 0.063 = 21618 N/m
    let expected_rear = (505.0 * 0.55 * 0.5) * 9.80665 / (0.180 * 0.350);
    assert!((k_rear - expected_rear).abs() < 1.0, "k_rear expected {:.1}, got {:.1}", expected_rear, k_rear);
}

#[test]
fn test_quat_mat3_roundtrip_all_angles() {
    let angles = [
        0.0,
        std::f64::consts::PI * 0.25,
        std::f64::consts::PI * 0.5,
        std::f64::consts::PI * 0.75,
        std::f64::consts::PI,
        -std::f64::consts::PI * 0.5,
    ];

    for &yaw in &angles {
        for &pitch in &angles {
            for &roll in &angles {
                let m_orig = Mat3::from_euler_yxz(yaw, pitch, roll);
                let q = Quat::from_mat3(&m_orig);
                let m_reconstructed = q.to_mat3();

                let diff_x = (m_orig.x - m_reconstructed.x).length();
                let diff_y = (m_orig.y - m_reconstructed.y).length();
                let diff_z = (m_orig.z - m_reconstructed.z).length();

                assert!(
                    diff_x < 1e-4 && diff_y < 1e-4 && diff_z < 1e-4,
                    "Quat roundtrip failed for yaw={}, pitch={}, roll={}: max_diff={}",
                    yaw, pitch, roll, diff_x.max(diff_y).max(diff_z)
                );
            }
        }
    }
}

#[test]
fn test_f1_94_aero_coefficients_spec() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert!((cfg.coefficient_of_drag - 0.78).abs() < 1e-6, "Cd must be 0.78, got {}", cfg.coefficient_of_drag);
    assert!((cfg.frontal_area - 1.25).abs() < 1e-6, "frontal area must be 1.25, got {}", cfg.frontal_area);
    assert!((cfg.coefficient_of_downforce - 2.85).abs() < 1e-6, "CL must be 2.85, got {}", cfg.coefficient_of_downforce);
    assert!((cfg.aero_balance_front - 0.44).abs() < 1e-6, "aero balance front must be 0.44, got {}", cfg.aero_balance_front);
}

