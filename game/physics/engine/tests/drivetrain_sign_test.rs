use vehicle_physics_engine::*;

#[test]
fn positive_drive_torque_produces_opposing_reaction_torque() {
    let cfg = VehicleConfig::f1_94_canonical();
    let wheel = WheelIndex::RearLeft;
    let mut tires = TireSystem::new(&cfg);
    let dt = 1.0 / 120.0;

    // Simulate grounded wheel accelerating forward: wheel spin is higher than forward road speed
    // e.g. spin = 50 rad/s (~16 m/s), vehicle speed = 10 m/s
    tires.wheels[wheel as usize].spin = 50.0;
    tires.wheels[wheel as usize].applied_torque = 500.0;

    let normal_force = cfg.mass_over_wheel(wheel) * 9.80665;
    tires.process_wheel_forces(
        &cfg,
        wheel,
        normal_force,
        SurfaceType::Road,
        2.9,
        8.75,
        1.0,
        false,
        Vec3::new(0.0, 0.0, -10.0), // -Z is forward in chassis frame
        dt,
    );

    let state = &tires.wheels[wheel as usize];
    assert!(
        state.longitudinal_force > 0.0,
        "Forward drive must produce positive longitudinal force, got {}",
        state.longitudinal_force
    );
    assert!(
        state.reaction_torque < 0.0,
        "Road-on-wheel reaction torque must oppose forward drive (< 0), got {}",
        state.reaction_torque
    );
    assert!(
        (state.reaction_torque + state.longitudinal_force * cfg.rear_tire_radius).abs() < 1e-6,
        "reaction_torque must equal -longitudinal_force * radius"
    );
}

#[test]
fn zero_throttle_at_forward_speed_does_not_accelerate_wheel() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = 1.0 / 120.0;

    // Full simulator test: vehicle coasting at 20 m/s with zero throttle must not accelerate
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::ZERO, 0.0);
    sim.state.linear_velocity = Vec3::new(0.0, 0.0, -20.0);
    for i in 0..4 {
        let r = if i < 2 { cfg.front_tire_radius } else { cfg.rear_tire_radius };
        sim.state.tires.wheels[i].spin = 20.0 / r;
    }
    let input_zero = VehicleInput { throttle: 0.0, ..VehicleInput::default() };
    let resting_dist_fl = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
    let resting_dist_rl = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
    let samples = [
        TriRaycastSample { inner: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, center: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, outer: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road } },
        TriRaycastSample { inner: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, center: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, outer: RaycastHit { is_colliding: true, distance: resting_dist_fl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road } },
        TriRaycastSample { inner: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, center: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, outer: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road } },
        TriRaycastSample { inner: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, center: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road }, outer: RaycastHit { is_colliding: true, distance: resting_dist_rl, point: Vec3::ZERO, normal: Vec3::UP, surface: SurfaceType::Road } },
    ];
    let telem = sim.step(&input_zero, &samples, dt);
    assert!(
        telem.long_g <= 1e-4,
        "Coasting vehicle must not produce positive forward acceleration (long_g={})",
        telem.long_g
    );
}

#[test]
fn increasing_road_load_moves_clutch_torque_in_resisting_direction() {
    let cfg = VehicleConfig::f1_94_canonical();
    let input = VehicleInput { throttle: 0.8, ..VehicleInput::default() };
    let spins = [0.0, 0.0, 60.0, 60.0];
    let dt = 1.0 / 120.0;

    let mut pt_light = PowertrainState::new(&cfg);
    pt_light.rpm = 10000.0;
    pt_light.step_with_reaction(&cfg, &input, &spins, &[0.0, 0.0, -100.0, -100.0], 15.0, dt);

    let mut pt_heavy = PowertrainState::new(&cfg);
    pt_heavy.rpm = 10000.0;
    pt_heavy.step_with_reaction(&cfg, &input, &spins, &[0.0, 0.0, -300.0, -300.0], 15.0, dt);

    assert!(
        pt_heavy.clutch_torque > pt_light.clutch_torque,
        "Heavy road load must yield higher resisting clutch torque: heavy={}, light={}",
        pt_heavy.clutch_torque,
        pt_light.clutch_torque
    );
}

#[test]
fn lift_throttle_coupling_remains_continuous() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut pt = PowertrainState::new(&cfg);
    pt.rpm = 9000.0;
    let spins = [0.0, 0.0, 50.0, 50.0];
    let dt = 1.0 / 120.0;

    // Steady positive drive
    let drive_input = VehicleInput { throttle: 0.6, ..VehicleInput::default() };
    pt.step_with_reaction(&cfg, &drive_input, &spins, &[0.0, 0.0, -150.0, -150.0], 12.0, dt);
    let drive_clutch = pt.clutch_torque;

    // Lift throttle (throttle = 0.0) next frame
    let lift_input = VehicleInput { throttle: 0.0, ..VehicleInput::default() };
    pt.step_with_reaction(&cfg, &lift_input, &spins, &[0.0, 0.0, -10.0, -10.0], 12.0, dt);
    let lift_clutch = pt.clutch_torque;

    assert!(drive_clutch.is_finite());
    assert!(lift_clutch.is_finite());
    // Coupling must remain smooth and finite without discontinuous divergence
    assert!((drive_clutch - lift_clutch).abs() < cfg.max_torque * 2.0);
}

#[test]
fn rwd_topology_invariants() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert!(!is_driven(&cfg, WheelIndex::FrontLeft));
    assert!(!is_driven(&cfg, WheelIndex::FrontRight));
    assert!(is_driven(&cfg, WheelIndex::RearLeft));
    assert!(is_driven(&cfg, WheelIndex::RearRight));
}

#[test]
fn zero_throttle_at_rest_produces_zero_drive_torque() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut pt = PowertrainState::new(&cfg);
    let dt = 1.0 / 120.0;
    let spins = [0.0, 0.0, 0.0, 0.0];
    let zero_input = VehicleInput { throttle: 0.0, brake: 0.0, ..VehicleInput::default() };

    pt.step_with_reaction(&cfg, &zero_input, &spins, &[0.0; 4], 0.0, dt);

    assert_eq!(pt.clutch_engagement, 0.0, "Clutch must be 100% disengaged at rest with zero throttle");
    assert_eq!(pt.clutch_torque, 0.0, "Clutch torque must be zero at rest with zero throttle");
    assert_eq!(pt.drive_torques[2], 0.0, "Rear left drive torque must be zero at rest");
    assert_eq!(pt.drive_torques[3], 0.0, "Rear right drive torque must be zero at rest");
}

#[test]
fn wheelspin_at_low_speed_does_not_trigger_premature_upshift() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut pt = PowertrainState::new(&cfg);
    let dt = 1.0 / 120.0;
    // Simulate excessive wheelspin: wheels at 80 rad/s (~95 km/h) but vehicle at 5 m/s (18 km/h)
    let spins = [0.0, 0.0, 80.0, 80.0];
    let input = VehicleInput { throttle: 1.0, ..VehicleInput::default() };
    pt.rpm = 16000.0; // High RPM from wheelspin

    pt.step_with_reaction(&cfg, &input, &spins, &[0.0, 0.0, -100.0, -100.0], 5.0, dt);

    assert_eq!(
        pt.current_gear, 1,
        "Transmission must stay in 1st gear when road speed is too low despite wheelspin"
    );
}

#[test]
fn kick_down_under_full_throttle_downshifts_when_bogged() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut pt = PowertrainState::new(&cfg);
    let dt = 1.0 / 120.0;
    pt.current_gear = 4;
    pt.target_gear = 4;
    pt.rpm = 7000.0; // Low RPM bogged down in 4th gear (~41% of max RPM)

    // Full throttle at 25 m/s (~90 km/h)
    let input = VehicleInput { throttle: 1.0, ..VehicleInput::default() };
    let wheel_spin = 25.0 / cfg.rear_tire_radius;
    let spins = [0.0, 0.0, wheel_spin, wheel_spin];

    pt.step_with_reaction(&cfg, &input, &spins, &[0.0, 0.0, -100.0, -100.0], 25.0, dt);

    assert_eq!(
        pt.target_gear, 3,
        "Transmission must trigger kick-down from 4th to 3rd when bogged down under full throttle"
    );
}
