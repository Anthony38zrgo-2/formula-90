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
fn contact_patch_is_per_axle() {
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

    t_high.process_wheel_forces(&high, WheelIndex::FrontLeft, 1500.0, fr, ef, es, er, false, vel, dt);
    t_low.process_wheel_forces(&low, WheelIndex::FrontLeft, 1500.0, fr, ef, es, er, false, vel, dt);

    let h = t_high.wheels[WheelIndex::FrontLeft as usize].lateral_force.abs()
        + t_high.wheels[WheelIndex::FrontLeft as usize].longitudinal_force.abs();
    let l = t_low.wheels[WheelIndex::FrontLeft as usize].lateral_force.abs()
        + t_low.wheels[WheelIndex::FrontLeft as usize].longitudinal_force.abs();
    assert!(h.is_finite() && l.is_finite());
    assert!(h > l && (h - l) > 1.0, "front contact_patch=0.60 must produce more grip than 0.10 ({} vs {})", h, l);
}

#[test]
fn braking_grip_is_per_axle() {
    let fr = SurfaceType::Road;
    let base = VehicleConfig::f1_94_canonical();
    let ef = *base.surface_friction.get(&fr).unwrap();
    let es = *base.surface_stiffness.get(&fr).unwrap();
    let er = *base.surface_rolling_resistance.get(&fr).unwrap();

    let mut high = VehicleConfig::f1_94_canonical();
    high.front_braking_grip = 2.0;
    let mut low = VehicleConfig::f1_94_canonical();
    low.front_braking_grip = 0.5;

    let mut t_high = TireSystem::new(&high);
    let mut t_low = TireSystem::new(&low);
    let vel = Vec3::new(0.0, 0.0, -20.0); // sy > 0.3 so braking_help applies
    let dt = 1.0 / 120.0;

    t_high.process_wheel_forces(&high, WheelIndex::FrontLeft, 1500.0, fr, ef, es, er, true, vel, dt);
    t_low.process_wheel_forces(&low, WheelIndex::FrontLeft, 1500.0, fr, ef, es, er, true, vel, dt);

    let h = t_high.wheels[WheelIndex::FrontLeft as usize].longitudinal_force.abs();
    let l = t_low.wheels[WheelIndex::FrontLeft as usize].longitudinal_force.abs();
    assert!(h > l, "higher front_braking_grip must increase braking force ({} vs {})", h, l);
}
