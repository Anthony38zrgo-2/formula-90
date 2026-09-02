use vehicle_physics_engine::*;

#[test]
fn rwd_topology_does_not_depend_on_instantaneous_torque() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert!(!is_driven(&cfg, WheelIndex::FrontLeft));
    assert!(!is_driven(&cfg, WheelIndex::FrontRight));
    assert!(is_driven(&cfg, WheelIndex::RearLeft));
    assert!(is_driven(&cfg, WheelIndex::RearRight));
}

#[test]
fn zero_drive_torque_does_not_snap_driven_wheel_to_road_speed() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut tires = TireSystem::new(&cfg);
    let i = WheelIndex::RearLeft as usize;
    tires.wheels[i].spin = 100.0;
    tires.wheels[i].reaction_torque = 0.0;
    tires.process_wheel_torque(&cfg, WheelIndex::RearLeft, 0.0, 0.0, 0.0, 1.0 / 120.0);
    assert!((tires.wheels[i].spin - 100.0).abs() < 1e-9);
}

#[test]
fn surface_stiffness_changes_brush_force() {
    let cfg = VehicleConfig::f1_94_canonical();
    let wheel = WheelIndex::RearLeft;
    let mut road = TireSystem::new(&cfg);
    let mut dirt = TireSystem::new(&cfg);
    road.wheels[wheel as usize].spin = 80.0;
    dirt.wheels[wheel as usize].spin = 80.0;
    road.wheels[wheel as usize].applied_torque = 4000.0;
    dirt.wheels[wheel as usize].applied_torque = 4000.0;

    road.process_wheel_forces(
        &cfg, wheel, 1500.0, SurfaceType::Road,
        *cfg.surface_friction.get(&SurfaceType::Road).unwrap(),
        *cfg.surface_stiffness.get(&SurfaceType::Road).unwrap(),
        *cfg.surface_rolling_resistance.get(&SurfaceType::Road).unwrap(),
        false, Vec3::new(2.0, 0.0, -20.0), 1.0 / 120.0,
    );
    dirt.process_wheel_forces(
        &cfg, wheel, 1500.0, SurfaceType::Dirt,
        *cfg.surface_friction.get(&SurfaceType::Dirt).unwrap(),
        *cfg.surface_stiffness.get(&SurfaceType::Dirt).unwrap(),
        *cfg.surface_rolling_resistance.get(&SurfaceType::Dirt).unwrap(),
        false, Vec3::new(2.0, 0.0, -20.0), 1.0 / 120.0,
    );

    let road_total = road.wheels[wheel as usize].lateral_force.abs()
        + road.wheels[wheel as usize].longitudinal_force.abs();
    let dirt_total = dirt.wheels[wheel as usize].lateral_force.abs()
        + dirt.wheels[wheel as usize].longitudinal_force.abs();
    assert!(road_total.is_finite() && dirt_total.is_finite());
    assert!((road_total - dirt_total).abs() > 1e-6);
}

#[test]
fn airborne_spin_decay_uses_configured_per_axle_torque() {
    // Regression: tire.rs previously hard-coded 2.0 and ignored the config value,
    // so changing airborne_spin_decay_torque in JSON had no effect. This fails pre-fix.
    let mut cfg = VehicleConfig::f1_94_canonical();
    cfg.front_airborne_decay = 5.0;
    cfg.rear_airborne_decay = 1.0;
    let mut tires = TireSystem::new(&cfg);
    let dt = 1.0 / 120.0;

    let fw = WheelIndex::FrontLeft as usize;
    tires.wheels[fw].spin = 100.0;
    tires.process_wheel_forces(
        &cfg, WheelIndex::FrontLeft, 0.0, SurfaceType::Road,
        1.0, 1.0, 1.0, false, Vec3::new(0.0, 0.0, 0.0), dt,
    );
    let expected_front = 100.0 - 5.0 / tires.wheels[fw].wheel_moment * dt;
    assert!(
        (tires.wheels[fw].spin - expected_front).abs() < 1e-9,
        "front airborne decay must track cfg.front_airborne_decay (got {}, expected {})",
        tires.wheels[fw].spin, expected_front
    );

    let rw = WheelIndex::RearLeft as usize;
    tires.wheels[rw].spin = 100.0;
    tires.process_wheel_forces(
        &cfg, WheelIndex::RearLeft, 0.0, SurfaceType::Road,
        1.0, 1.0, 1.0, false, Vec3::new(0.0, 0.0, 0.0), dt,
    );
    let expected_rear = 100.0 - 1.0 / tires.wheels[rw].wheel_moment * dt;
    assert!(
        (tires.wheels[rw].spin - expected_rear).abs() < 1e-9,
        "rear airborne decay must track cfg.rear_airborne_decay (got {}, expected {})",
        tires.wheels[rw].spin, expected_rear
    );
}

#[test]
fn contact_patch_scales_relaxation_and_converges_in_steady_state() {
    // New transient model: a longer contact patch increases the relaxation length,
    // so force builds more slowly (first tick), while both patches converge to the
    // same saturated force in steady state. The patch no longer multiplies peak grip.
    let fr = SurfaceType::Road;
    let base = VehicleConfig::f1_94_canonical();
    let ef = *base.surface_friction.get(&fr).unwrap();
    let es = *base.surface_stiffness.get(&fr).unwrap();
    let er = *base.surface_rolling_resistance.get(&fr).unwrap();

    let mut high = VehicleConfig::f1_94_canonical();
    high.front_contact_patch = 0.60;
    let mut low = VehicleConfig::f1_94_canonical();
    low.front_contact_patch = 0.10;

    let mut t_high = TireSystem::new(&high);
    let mut t_low = TireSystem::new(&low);
    let vel = Vec3::new(2.0, 0.0, -20.0);
    let dt = 1.0 / 120.0;
    let wheel = WheelIndex::FrontLeft;

    let mut first_high = 0.0;
    let mut first_low = 0.0;
    for tick in 0..60 {
        t_high.process_wheel_forces(&high, wheel, 1500.0, fr, ef, es, er, false, vel, dt);
        t_low.process_wheel_forces(&low, wheel, 1500.0, fr, ef, es, er, false, vel, dt);
        if tick == 0 {
            first_high = t_high.wheels[wheel as usize].lateral_force.abs()
                + t_high.wheels[wheel as usize].longitudinal_force.abs();
            first_low = t_low.wheels[wheel as usize].lateral_force.abs()
                + t_low.wheels[wheel as usize].longitudinal_force.abs();
        }
    }
    let h = t_high.wheels[wheel as usize].lateral_force.abs()
        + t_high.wheels[wheel as usize].longitudinal_force.abs();
    let l = t_low.wheels[wheel as usize].lateral_force.abs()
        + t_low.wheels[wheel as usize].longitudinal_force.abs();

    assert!(
        first_low > first_high,
        "Smaller patch must build force faster on the first tick (low={}, high={})",
        first_low,
        first_high
    );
    assert!(h.is_finite() && l.is_finite());
    assert!(h > 500.0, "Steady-state grip must remain substantial (h={})", h);
    assert!(
        (h - l).abs() < h * 0.05,
        "Both patches must converge to the same steady-state force (high={}, low={})",
        h,
        l
    );
}

#[test]
fn brake_input_does_not_change_available_mu() {
    // TIRE-600: peak tire capacity must be independent of the brake input flag.
    // Same tire/Fz/slip state must produce identical forces with braking on or off.
    let fr = SurfaceType::Road;
    let base = VehicleConfig::f1_94_canonical();
    let ef = *base.surface_friction.get(&fr).unwrap();
    let es = *base.surface_stiffness.get(&fr).unwrap();
    let er = *base.surface_rolling_resistance.get(&fr).unwrap();

    let mut braking = TireSystem::new(&base);
    let mut coasting = TireSystem::new(&base);
    let vel = Vec3::new(0.0, 0.0, -20.0);
    let dt = 1.0 / 120.0;
    braking.wheels[WheelIndex::FrontLeft as usize].spin = 40.0;
    coasting.wheels[WheelIndex::FrontLeft as usize].spin = 40.0;

    for _ in 0..40 {
        braking.process_wheel_forces(&base, WheelIndex::FrontLeft, 1500.0, fr, ef, es, er, true, vel, dt);
        coasting.process_wheel_forces(&base, WheelIndex::FrontLeft, 1500.0, fr, ef, es, er, false, vel, dt);
    }

    let b = &braking.wheels[WheelIndex::FrontLeft as usize];
    let c = &coasting.wheels[WheelIndex::FrontLeft as usize];
    assert!(
        (b.longitudinal_force - c.longitudinal_force).abs() < 1e-9,
        "brake flag must not change longitudinal force (braking={}, coasting={})",
        b.longitudinal_force,
        c.longitudinal_force
    );
    assert!(
        (b.lateral_force - c.lateral_force).abs() < 1e-9,
        "brake flag must not change lateral force"
    );
}
