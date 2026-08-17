use vehicle_physics_engine::powertrain::solve_salisbury_differential;
use vehicle_physics_engine::vehicle_config::VehicleConfig;

#[test]
fn test_salisbury_differential_symmetric_when_wheel_spins_are_equal() {
    let drive_torque = 1000.0;
    let spin_left = 50.0;
    let spin_right = 50.0; // Zero speed delta
    let (t_left, t_right) = solve_salisbury_differential(
        drive_torque,
        spin_left,
        spin_right,
        90.0,  // Preload
        45.0,  // Power ramp
        60.0,  // Coast ramp
        4.0,   // Clutches
        0.25,  // Friction coeff
    );

    assert!((t_left - 500.0).abs() < 1e-6, "Left torque must be exactly 500.0 N·m");
    assert!((t_right - 500.0).abs() < 1e-6, "Right torque must be exactly 500.0 N·m");
    assert!((t_left + t_right - drive_torque).abs() < 1e-6, "Total torque must be conserved");
}

#[test]
fn test_salisbury_differential_power_lock_transfers_torque_to_slower_wheel() {
    let drive_torque = 1000.0; // Positive throttle
    let spin_left = 60.0;      // Spinning wheel (e.g. over curb or low grip)
    let spin_right = 40.0;     // Wheel with traction (slower)

    let (t_left, t_right) = solve_salisbury_differential(
        drive_torque,
        spin_left,
        spin_right,
        90.0,
        45.0, // 45° power ramp -> high locking
        60.0,
        4.0,
        0.25,
    );

    assert!(
        t_right > t_left,
        "Slower wheel with grip must receive more drive torque than spinning wheel (T_right={}, T_left={})",
        t_right,
        t_left
    );
    assert!((t_left + t_right - drive_torque).abs() < 1e-6, "Torque must be strictly conserved");
}

#[test]
fn test_salisbury_differential_coast_lock_transfers_torque_under_engine_braking() {
    let drive_torque = -300.0; // Engine braking / coasting
    let spin_left = 55.0;
    let spin_right = 45.0;

    let (t_left, t_right) = solve_salisbury_differential(
        drive_torque,
        spin_left,
        spin_right,
        90.0,
        45.0,
        60.0, // 60° coast ramp
        4.0,
        0.25,
    );

    assert!((t_left + t_right - drive_torque).abs() < 1e-6, "Coast torque must be conserved");
    assert!(
        t_left < t_right,
        "Faster spinning wheel must absorb more negative braking torque (T_left={}, T_right={})",
        t_left,
        t_right
    );
}

#[test]
fn test_salisbury_differential_preload_acts_at_zero_drive_torque() {
    let drive_torque = 0.0; // Off throttle / neutral coast
    let spin_left = 30.0;
    let spin_right = 20.0; // Differential speed present
    let preload = 90.0;

    let (t_left, t_right) = solve_salisbury_differential(
        drive_torque,
        spin_left,
        spin_right,
        preload,
        45.0,
        60.0,
        4.0,
        0.25,
    );

    assert!((t_left + t_right).abs() < 1e-6, "Sum of torques at zero input must be zero");
    assert!(
        t_left < 0.0 && t_right > 0.0,
        "Preload must apply opposing resistive torque to fast wheel and forward torque to slow wheel (T_left={}, T_right={})",
        t_left,
        t_right
    );
    assert!(
        (t_right - preload).abs() < 1e-6,
        "Transfer torque must equal preload at full slip (T_right={}, preload={})",
        t_right,
        preload
    );
}

#[test]
fn test_salisbury_differential_canonical_config_integration() {
    let cfg = VehicleConfig::f1_94_canonical();
    assert_eq!(cfg.diff_preload, 170.0);
    assert_eq!(cfg.diff_power_ramp_angle_deg, 65.0);
    assert_eq!(cfg.diff_coast_ramp_angle_deg, 75.0);
    assert_eq!(cfg.diff_clutches, 4.0);
    assert_eq!(cfg.diff_clutch_friction_coeff, 0.0);
}
