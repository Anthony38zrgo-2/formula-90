use vehicle_physics_engine::*;

fn flat_ground_samples(sim: &VehicleSimulator) -> [TriRaycastSample; 4] {
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
fn external_solver_returns_non_gravity_rigidbody_forces() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
    let samples = flat_ground_samples(&sim);
    let body = sim.state.body_kinematics();
    let (out, _) = sim.solve_external(body, &VehicleInput::default(), &samples, 1.0 / 120.0);

    assert!(out.force_world.x.is_finite());
    assert!(out.force_world.y.is_finite());
    assert!(out.force_world.z.is_finite());
    assert!(out.torque_world.x.is_finite());
    assert!(out.torque_world.y.is_finite());
    assert!(out.torque_world.z.is_finite());
    assert!(out.force_world.y > 0.0, "suspension should push the body upward; gravity is left to Godot");
}

#[test]
fn symmetric_flat_contact_has_negligible_yaw_torque() {
    let cfg = VehicleConfig::f1_94_canonical();
    let spawn = default_spawn_height(&cfg);
    let mut sim = VehicleSimulator::new(cfg, Vec3::new(0.0, spawn, 0.0), 0.0);
    let samples = flat_ground_samples(&sim);
    let body = sim.state.body_kinematics();
    let (out, _) = sim.solve_external(body, &VehicleInput::default(), &samples, 1.0 / 120.0);
    assert!(out.torque_world.y.abs() < 1e-6);
}
