use vehicle_physics_engine::*;

#[test]
fn rwd_drive_torque_is_only_sent_to_rear_axle() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut pt = PowertrainState::new(&cfg);
    let input = VehicleInput { throttle: 1.0, ..VehicleInput::default() };
    pt.step_with_reaction(&cfg, &input, &[0.0; 4], &[0.0; 4], 0.0, 1.0 / 120.0);
    assert_eq!(pt.drive_torques[0], 0.0);
    assert_eq!(pt.drive_torques[1], 0.0);
    assert!(pt.drive_torques[2].is_finite());
    assert!(pt.drive_torques[3].is_finite());
}

#[test]
fn tire_reaction_torque_feeds_back_into_clutch() {
    let cfg = VehicleConfig::f1_94_canonical();
    let input = VehicleInput { throttle: 0.5, ..VehicleInput::default() };
    let spins = [0.0, 0.0, 52.5, 52.5];

    let mut unloaded = PowertrainState::new(&cfg);
    unloaded.rpm = 9000.0;
    unloaded.step_with_reaction(&cfg, &input, &spins, &[0.0; 4], 10.0, 1.0 / 120.0);

    let mut loaded = PowertrainState::new(&cfg);
    loaded.rpm = 9000.0;
    loaded.step_with_reaction(&cfg, &input, &spins, &[0.0, 0.0, 200.0, 200.0], 10.0, 1.0 / 120.0);

    assert!((loaded.clutch_torque - unloaded.clutch_torque).abs() > 1e-9);
}
