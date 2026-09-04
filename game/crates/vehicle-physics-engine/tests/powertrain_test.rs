use vehicle_physics_engine::*;

#[test]
fn rwd_drive_torque_is_only_sent_to_rear_axle() {
    let cfg = VehicleConfig::f1_94_canonical();
    let mut pt = PowertrainState::new(&cfg);
    let input = VehicleInput { throttle: 1.0, ..VehicleInput::default() };
    pt.step_with_reaction(&cfg, &input, &[0.0; 4], &[0.0; 4], 0.0, true, true, cfg.enable_abs, 1.0 / 120.0);
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
    unloaded.step_with_reaction(&cfg, &input, &spins, &[0.0; 4], 10.0, true, true, cfg.enable_abs, 1.0 / 120.0);

    let mut loaded = PowertrainState::new(&cfg);
    loaded.rpm = 9000.0;
    // Road exerts resisting reaction torque (negative) on rear wheels
    loaded.step_with_reaction(&cfg, &input, &spins, &[0.0, 0.0, -200.0, -200.0], 10.0, true, true, cfg.enable_abs, 1.0 / 120.0);

    assert!(
        loaded.clutch_torque > unloaded.clutch_torque,
        "Loaded clutch torque ({}) must exceed unloaded ({}) in resisting direction",
        loaded.clutch_torque,
        unloaded.clutch_torque
    );
}

#[test]
fn traction_control_reduces_drive_torque_when_enabled() {
    let mut cfg = VehicleConfig::f1_94_canonical();
    // Force a high slip state: spinning rear wheels, low road speed, full throttle.
    let input = VehicleInput { throttle: 1.0, ..VehicleInput::default() };
    let spins = [0.0, 0.0, 120.0, 120.0]; // ~38 m/s wheel surface speed
    let reactions = [0.0; 4];

    // Enabled
    let mut enabled = PowertrainState::new(&cfg);
    enabled.rpm = 12000.0;
    enabled.current_gear = 2;
    enabled.step_with_reaction(&cfg, &input, &spins, &reactions, 2.0, true, true, cfg.enable_abs, 1.0 / 120.0);
    assert!(enabled.tc_active, "TC must be active under high slip");
    let enabled_drive = enabled.drive_torques[2].abs();

    // Disabled
    let mut disabled = PowertrainState::new(&cfg);
    disabled.rpm = 12000.0;
    disabled.current_gear = 2;
    disabled.step_with_reaction(&cfg, &input, &spins, &reactions, 2.0, false, true, cfg.enable_abs, 1.0 / 120.0);
    assert!(!disabled.tc_active, "TC must not be active (cut) when disabled");
    let disabled_drive = disabled.drive_torques[2].abs();

    assert!(
        enabled_drive < disabled_drive,
        "Enabled TC drive torque ({}) must be below disabled ({}); enabled.tc_active={} cut={} disabled.tc_active={} cut={}",
        enabled_drive, disabled_drive, enabled.tc_active, enabled.tc_cut_ratio, disabled.tc_active, disabled.tc_cut_ratio
    );
}

#[test]
fn traction_control_inactive_without_slip() {
    let cfg = VehicleConfig::f1_94_canonical();
    let input = VehicleInput { throttle: 1.0, ..VehicleInput::default() };
    // Wheel surface speed matches road speed => no slip.
    let road = 20.0;
    let radius = cfg.rear_tire_radius;
    let spin = road / radius;
    let spins = [0.0, 0.0, spin, spin];
    let reactions = [0.0; 4];

    let mut pt = PowertrainState::new(&cfg);
    pt.rpm = 12000.0;
    pt.current_gear = 2;
    pt.step_with_reaction(&cfg, &input, &spins, &reactions, road, true, true, cfg.enable_abs, 1.0 / 120.0);
    assert!(!pt.tc_active, "TC must be inactive when slip is below threshold");
    assert_eq!(pt.tc_cut_ratio, 0.0);
}

#[test]
fn traction_control_eligibility_contract() {
    let cfg = VehicleConfig::f1_94_canonical();
    let reactions = [0.0; 4];
    let spins = [0.0, 0.0, 50.0, 50.0];

    // 1. Fully eligible: enabled, RWD, in gear 1, throttle 0.5
    let mut pt = PowertrainState::new(&cfg);
    pt.rpm = 8000.0;
    pt.current_gear = 1;
    let input = VehicleInput { throttle: 0.5, ..VehicleInput::default() };
    pt.step_with_reaction(&cfg, &input, &spins, &reactions, 15.0, true, true, cfg.enable_abs, 1.0 / 120.0);
    assert!(pt.tc_eligible, "TC must be eligible when enabled, in gear, and throttle applied");

    // 2. Ineligible: TC disabled
    let mut pt_disabled = PowertrainState::new(&cfg);
    pt_disabled.rpm = 8000.0;
    pt_disabled.current_gear = 1;
    pt_disabled.step_with_reaction(&cfg, &input, &spins, &reactions, 15.0, false, true, cfg.enable_abs, 1.0 / 120.0);
    assert!(!pt_disabled.tc_eligible, "TC must be ineligible when TC is disabled");

    // 3. Ineligible: zero throttle
    let mut pt_no_throttle = PowertrainState::new(&cfg);
    pt_no_throttle.rpm = 8000.0;
    pt_no_throttle.current_gear = 1;
    let input_idle = VehicleInput { throttle: 0.0, ..VehicleInput::default() };
    pt_no_throttle.step_with_reaction(&cfg, &input_idle, &spins, &reactions, 15.0, true, true, cfg.enable_abs, 1.0 / 120.0);
    assert!(!pt_no_throttle.tc_eligible, "TC must be ineligible when throttle is not applied");

    // 4. Ineligible: neutral gear (0)
    let mut pt_neutral = PowertrainState::new(&cfg);
    pt_neutral.rpm = 8000.0;
    pt_neutral.current_gear = 0;
    pt_neutral.step_with_reaction(&cfg, &input, &spins, &reactions, 15.0, true, true, cfg.enable_abs, 1.0 / 120.0);
    assert!(!pt_neutral.tc_eligible, "TC must be ineligible in neutral");
}

#[test]
fn brake_assist_multiplies_brake_torque() {
    let mut cfg = VehicleConfig::f1_94_canonical();
    cfg.aids.brake_assist_force_multiplier = 1.5;
    let input = VehicleInput { brake: 1.0, ..VehicleInput::default() };
    let spins = [0.0; 4];
    let reactions = [0.0; 4];

    let mut on = PowertrainState::new(&cfg);
    on.rpm = 5000.0;
    on.current_gear = 1;
    on.step_with_reaction(&cfg, &input, &spins, &reactions, 0.0, false, true, false, 1.0 / 120.0);
    let on_brake = on.brake_torques[2];

    let mut off = PowertrainState::new(&cfg);
    off.rpm = 5000.0;
    off.current_gear = 1;
    off.step_with_reaction(&cfg, &input, &spins, &reactions, 0.0, false, false, false, 1.0 / 120.0);
    let off_brake = off.brake_torques[2];

    assert!(
        (on_brake - off_brake * 1.5).abs() < 1e-6,
        "Brake assist must scale brake torque by multiplier (on={}, off*1.5={})",
        on_brake, off_brake * 1.5
    );
}
