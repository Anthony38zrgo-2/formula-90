use vehicle_physics_engine::*;

#[test]
fn steering_input_sign_matches_wheel_angle_convention() {
    let cfg = VehicleConfig::f1_94_canonical();

    // +1.0 = LEFT steering => positive steering angle in local Y-up coordinate frame
    let angle_left = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, 1.0, 0.0);
    let angle_right_wheel = steering_angle_for_wheel(&cfg, WheelIndex::FrontRight, 1.0, 0.0);
    assert!(angle_left > 0.0, "Steering +1.0 (LEFT) must produce positive wheel angle (angle={})", angle_left);
    assert!(angle_right_wheel > 0.0, "Steering +1.0 (LEFT) must produce positive wheel angle (angle={})", angle_right_wheel);

    // -1.0 = RIGHT steering => negative steering angle
    let angle_left_neg = steering_angle_for_wheel(&cfg, WheelIndex::FrontLeft, -1.0, 0.0);
    let angle_right_neg = steering_angle_for_wheel(&cfg, WheelIndex::FrontRight, -1.0, 0.0);
    assert!(angle_left_neg < 0.0, "Steering -1.0 (RIGHT) must produce negative wheel angle (angle={})", angle_left_neg);
    assert!(angle_right_neg < 0.0, "Steering -1.0 (RIGHT) must produce negative wheel angle (angle={})", angle_right_neg);
}

#[test]
fn countersteer_correction_opposes_lateral_velocity() {
    let cfg = VehicleConfig::f1_94_canonical();
    let dt = 1.0 / 120.0;
    let samples = [TriRaycastSample::default(); 4];

    // Scenario 1: Lateral velocity = +X (sliding right), zero requested steering
    // Must produce negative countersteer correction (steering right to catch the slide)
    let mut sim_pos_lat = VehicleSimulator::new(cfg.clone(), Vec3::ZERO, 0.0);
    sim_pos_lat.state.linear_velocity = Vec3::new(5.0, 0.0, -20.0); // +X lateral, -Z forward
    let input_zero = VehicleInput { steering: 0.0, ..VehicleInput::default() };
    let telem_pos = sim_pos_lat.step(&input_zero, &samples, dt);
    assert!(
        telem_pos.steering < 0.0,
        "Lateral velocity +X with zero input must produce negative countersteer, got {}",
        telem_pos.steering
    );

    // Scenario 2: Lateral velocity = -X (sliding left), zero requested steering
    // Must produce positive countersteer correction (steering left to catch the slide)
    let mut sim_neg_lat = VehicleSimulator::new(cfg.clone(), Vec3::ZERO, 0.0);
    sim_neg_lat.state.linear_velocity = Vec3::new(-5.0, 0.0, -20.0); // -X lateral, -Z forward
    let telem_neg = sim_neg_lat.step(&input_zero, &samples, dt);
    assert!(
        telem_neg.steering > 0.0,
        "Lateral velocity -X with zero input must produce positive countersteer, got {}",
        telem_neg.steering
    );
}

#[test]
fn front_slip_assist_limits_steering_into_slip_and_permits_countersteer() {
    let mut cfg = VehicleConfig::f1_94_canonical();
    cfg.steering_slip_assist = 0.30;
    cfg.countersteer_assist = 0.0; // Isolate slip assist test
    let dt = 1.0 / 120.0;
    let samples = [TriRaycastSample::default(); 4];

    // Setup simulator with excessive positive front slip (slipping left, slip_angle_rad > 0.30)
    let mut sim = VehicleSimulator::new(cfg.clone(), Vec3::ZERO, 0.0);
    sim.state.steer_input_smoothed = 0.40;
    sim.state.tires.wheels[0].slip_angle_rad = 0.50; // Above threshold
    sim.state.tires.wheels[1].slip_angle_rad = 0.50;

    // 1. Driver tries to steer further INTO the slip (e.g. requested = 0.90)
    let input_deeper = VehicleInput { steering: 0.90, ..VehicleInput::default() };
    let telem_deeper = sim.step(&input_deeper, &samples, dt);
    // Target is clamped to current steer_input_smoothed (0.40), so smoothed steering cannot increase towards 0.90
    assert!(
        telem_deeper.steering <= 0.40 + 1e-6,
        "Steering deeper into excessive front slip must be limited. Expected <= 0.40, got {}",
        telem_deeper.steering
    );

    // 2. Driver steers AGAINST the slip (requested = -0.50 or 0.0)
    let mut sim2 = VehicleSimulator::new(cfg.clone(), Vec3::ZERO, 0.0);
    sim2.state.steer_input_smoothed = 0.40;
    sim2.state.tires.wheels[0].slip_angle_rad = 0.50;
    sim2.state.tires.wheels[1].slip_angle_rad = 0.50;

    let input_against = VehicleInput { steering: -0.50, ..VehicleInput::default() };
    let telem_against = sim2.step(&input_against, &samples, dt);
    // Steering opposite the slip must be immediately allowed and move toward -0.50
    assert!(
        telem_against.steering < 0.40,
        "Steering against slip must be allowed immediately. Expected < 0.40, got {}",
        telem_against.steering
    );
}
