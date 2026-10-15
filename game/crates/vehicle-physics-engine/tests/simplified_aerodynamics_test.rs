use vehicle_physics_engine::aero::{AeroEnvironment, AeroForces, AeroKinematics, AeroProbeMode};
use vehicle_physics_engine::{Vec3, VehicleConfig};

fn configuration() -> VehicleConfig {
    VehicleConfig::f1_94_canonical()
}

fn kinematics(speed_metres_per_second: f64) -> AeroKinematics {
    AeroKinematics {
        velocity_m_s: Vec3::new(0.0, 0.0, -speed_metres_per_second),
        ..AeroKinematics::default()
    }
}

#[test]
fn simplified_equilibrium_preserves_standard_load_and_drag() {
    let configuration = configuration();
    let environment = AeroEnvironment::nominal(&configuration.aerodynamics.aero_model.underfloor);
    for speed in [22.222, 44.444, 66.667, 83.333] {
        let kinematics = kinematics(speed);
        let mut standard = AeroForces::zero();
        for _ in 0..3000 {
            standard.step_with_kinematics(&configuration, &kinematics, &environment, 1.0 / 240.0);
        }
        let mut simplified = AeroForces::zero();
        simplified.step_simplified_with_kinematics(
            &configuration,
            &kinematics,
            &environment,
            1.0 / 240.0,
        );
        assert!(!simplified.invalid_input);
        assert!((simplified.total_downforce / standard.total_downforce - 1.0).abs() < 0.05);
        assert!((simplified.drag_force / standard.drag_force - 1.0).abs() < 0.05);
        assert!((simplified.balance_front - standard.balance_front).abs() < 0.03);
    }
}

#[test]
fn missing_measurements_cannot_fabricate_ground_effect() {
    let configuration = configuration();
    let mut environment =
        AeroEnvironment::nominal(&configuration.aerodynamics.aero_model.underfloor);
    let kinematics = kinematics(50.0);
    let mut simplified = AeroForces::zero();
    simplified.step_simplified_with_kinematics(
        &configuration,
        &kinematics,
        &environment,
        1.0 / 120.0,
    );
    let measured_floor_load = simplified.diffuser_downforce;
    for mask in [0, 1, 8, 0b10110] {
        environment.valid_mask = mask;
        simplified.step_simplified_with_kinematics(
            &configuration,
            &kinematics,
            &environment,
            1.0 / 120.0,
        );
        assert_eq!(simplified.ground_confidence, 0.0);
        assert!(simplified.diffuser_downforce < measured_floor_load);
    }
}

#[test]
fn simplified_forces_use_air_velocity_and_reject_nonfinite_input() {
    let configuration = configuration();
    let environment = AeroEnvironment::nominal(&configuration.aerodynamics.aero_model.underfloor);
    let mut kinematics = kinematics(50.0);
    kinematics.wind_velocity_m_s = kinematics.velocity_m_s;
    let mut simplified = AeroForces::zero();
    simplified.step_simplified_with_kinematics(
        &configuration,
        &kinematics,
        &environment,
        1.0 / 120.0,
    );
    assert_eq!(simplified.total_downforce, 0.0);
    assert_eq!(simplified.drag_force, 0.0);
    kinematics.velocity_m_s.x = f64::NAN;
    simplified.step_simplified_with_kinematics(
        &configuration,
        &kinematics,
        &environment,
        1.0 / 120.0,
    );
    assert!(simplified.invalid_input);
    assert_eq!(simplified.total_downforce, 0.0);
}

#[test]
fn simplified_model_is_independent_of_timestep_and_unused_sensors() {
    let configuration = configuration();
    let mut environment =
        AeroEnvironment::nominal(&configuration.aerodynamics.aero_model.underfloor);
    let kinematics = kinematics(50.0);
    let mut reference = AeroForces::zero();
    reference.step_simplified_with_kinematics(
        &configuration,
        &kinematics,
        &environment,
        1.0 / 120.0,
    );
    environment.clearance_m[1] = f64::NAN;
    environment.clearance_m[2] = 0.0;
    environment.clearance_m[4] = 100.0;
    for frequency in [120.0, 240.0, 480.0] {
        let mut simplified = AeroForces::zero();
        simplified.step_simplified_with_kinematics(
            &configuration,
            &kinematics,
            &environment,
            1.0 / frequency,
        );
        assert_eq!(simplified.force_local, reference.force_local);
        assert_eq!(simplified.torque_local, reference.torque_local);
    }
    let disabled_kinematics = AeroKinematics {
        probe_mode: AeroProbeMode::Disabled,
        ..kinematics
    };
    reference.step_simplified_with_kinematics(
        &configuration,
        &disabled_kinematics,
        &environment,
        1.0 / 120.0,
    );
    assert_eq!(reference.ground_confidence, 0.0);
}
