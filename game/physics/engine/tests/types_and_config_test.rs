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

