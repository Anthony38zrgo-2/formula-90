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
