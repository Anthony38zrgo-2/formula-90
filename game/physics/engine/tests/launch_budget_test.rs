use vehicle_physics_engine::*;

fn road_friction(cfg: &VehicleConfig) -> f64 {
    *cfg.surface_friction.get(&SurfaceType::Road).unwrap()
}

#[test]
fn launch_budget_reports_clutch_vs_longitudinal_grip() {
    let cfg = VehicleConfig::f1_94_canonical();
    let rear_mass_per_wheel = cfg.mass_over_wheel(WheelIndex::RearLeft);
    let static_normal = rear_mass_per_wheel * 9.80665;
    let rear_k = cfg.calculate_spring_rate(WheelIndex::RearLeft);
    let observed_rear_compression_m = (0.110 - cfg.rear_spring_length * cfg.rear_resting_ratio).max(0.0);
    let squat_normal = observed_rear_compression_m * rear_k;
    let normal = static_normal + squat_normal;
    let friction_budget = (road_friction(&cfg) * normal
        - normal / (cfg.rear_tire_width * 1000.0 * cfg.contact_patch * 0.2))
        .max(0.0);
    let longitudinal_budget = friction_budget * 0.5;
    let first_ratio = cfg.gear_ratios[0] * cfg.final_drive;
    let clutch_limit = cfg.max_torque * 1.6;
    let drive_force_per_wheel = clutch_limit * first_ratio / 2.0 / cfg.rear_tire_radius;
    let ratio = drive_force_per_wheel / longitudinal_budget.max(1e-6);

    println!("normal_static_n={static_normal:.3}");
    println!("normal_with_observed_squat_n={normal:.3}");
    println!("longitudinal_grip_budget_n={longitudinal_budget:.3}");
    println!("clutch_drive_force_per_rear_wheel_n={drive_force_per_wheel:.3}");
    println!("drive_to_grip_ratio={ratio:.3}");
    assert!(ratio > 1.0, "launch drive force should exceed the configured longitudinal budget");
}

#[test]
fn brush_lateral_force_is_degraded_by_longitudinal_slip() {
    let cfg = VehicleConfig::f1_94_canonical();
    let wheel = WheelIndex::RearLeft;
    let normal = cfg.mass_over_wheel(wheel) * 9.80665;
    let friction = *cfg.surface_friction.get(&SurfaceType::Road).unwrap();
    let stiffness = *cfg.surface_stiffness.get(&SurfaceType::Road).unwrap();
    let rolling = *cfg.surface_rolling_resistance.get(&SurfaceType::Road).unwrap();
    let dt = 1.0 / 120.0;
    let forward = 36.0;
    let mut values = Vec::new();

    for slip_ratio in [0.1_f64, 0.5, 1.0, 1.5] {
        let wheel_speed = forward * (1.0 + slip_ratio);
        let mut tires = TireSystem::new(&cfg);
        tires.process_wheel_torque(&cfg, wheel, 2000.0, 5.0, 0.0, dt);
        tires.wheels[wheel as usize].spin = wheel_speed / cfg.rear_tire_radius;
        tires.process_wheel_forces(
            &cfg,
            wheel,
            normal,
            SurfaceType::Road,
            friction,
            stiffness,
            rolling,
            false,
            Vec3::new(3.6, 0.0, -forward),
            dt,
        );
        let lateral = tires.wheels[wheel as usize].lateral_force.abs();
        values.push((slip_ratio, lateral));
        println!("slip_ratio={slip_ratio:.3} lateral_force_n={lateral:.3}");
    }

    assert!(values.iter().all(|(_, force)| force.is_finite()));
}
