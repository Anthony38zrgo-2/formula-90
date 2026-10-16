use std::path::Path;
use vehicle_physics_engine::tire::{TireBackend, TireSystem, WheelTireState};
use vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root;
use vehicle_physics_engine::{SurfaceType, Vec3, VehicleConfig, WheelIndex};

fn configuration() -> VehicleConfig {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    load_vehicle_configuration_from_authorized_root(
        &repository.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"),
        &repository,
    )
    .unwrap()
}

fn advance(
    configuration: &VehicleConfig,
    system: &mut TireSystem,
    wheel: WheelIndex,
    load: f64,
    angle: f64,
    slip: f64,
    camber: f64,
    duration: f64,
) {
    system.set_mechanical_state(
        configuration,
        wheel,
        load,
        camber,
        0.008,
        if load > 0.0 { 1.0 } else { 0.0 },
    );
    let radius = system.wheels[wheel as usize].effective_rolling_radius;
    system.wheels[wheel as usize].spin = if slip >= 0.0 {
        20.0 / (1.0 - slip) / radius
    } else {
        20.0 * (1.0 + slip) / radius
    };
    system.process_wheel_forces(
        configuration,
        wheel,
        load,
        SurfaceType::Road,
        2.7,
        7.2,
        1.0,
        false,
        Vec3::new(-angle.tan() * 20.0, 0.0, -20.0),
        duration,
    );
}

fn settled(
    configuration: &VehicleConfig,
    backend: TireBackend,
    wheel: WheelIndex,
    load: f64,
    angle: f64,
    slip: f64,
    camber: f64,
) -> WheelTireState {
    let mut system = TireSystem::new(configuration);
    system.backend = backend;
    for _ in 0..600 {
        advance(
            configuration,
            &mut system,
            wheel,
            load,
            angle,
            slip,
            camber,
            1.0 / 120.0,
        );
    }
    system.wheels[wheel as usize].clone()
}

#[test]
fn simplified_pure_curves_preserve_standard_steady_envelopes() {
    let configuration = configuration();
    for wheel in WheelIndex::ALL {
        for load in [900.0, 1800.0, 3600.0] {
            for slip in [-0.45, -0.12, -0.03, 0.0, 0.03, 0.12, 0.45] {
                for lateral in [false, true] {
                    let (angle, longitudinal) = if lateral { (slip, 0.0) } else { (0.0, slip) };
                    let standard = settled(
                        &configuration,
                        TireBackend::Standard,
                        wheel,
                        load,
                        angle,
                        longitudinal,
                        0.0,
                    );
                    for backend in [
                        TireBackend::SimplifiedWithoutRelaxation,
                        TireBackend::SimplifiedWithRelaxation,
                    ] {
                        let candidate = settled(
                            &configuration,
                            backend,
                            wheel,
                            load,
                            angle,
                            longitudinal,
                            0.0,
                        );
                        assert!((standard.lateral_force - candidate.lateral_force).abs() < 0.0001);
                        assert!(
                            (standard.longitudinal_force - candidate.longitudinal_force).abs()
                                < 0.0001
                        );
                        assert!(
                            (standard.aligning_torque - candidate.aligning_torque).abs() < 0.0001
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn instantaneous_candidate_has_no_post_peak_history() {
    let configuration = configuration();
    let mut first = TireSystem::new(&configuration);
    first.backend = TireBackend::SimplifiedWithoutRelaxation;
    let mut second = first.clone();
    for _ in 0..120 {
        advance(
            &configuration,
            &mut first,
            WheelIndex::FrontLeft,
            1800.0,
            0.5,
            0.5,
            0.0,
            1.0 / 120.0,
        );
    }
    advance(
        &configuration,
        &mut first,
        WheelIndex::FrontLeft,
        1800.0,
        0.03,
        0.02,
        0.0,
        1.0 / 120.0,
    );
    advance(
        &configuration,
        &mut second,
        WheelIndex::FrontLeft,
        1800.0,
        0.03,
        0.02,
        0.0,
        1.0 / 120.0,
    );
    assert_eq!(
        first.wheels[0].lateral_force,
        second.wheels[0].lateral_force
    );
    assert_eq!(
        first.wheels[0].longitudinal_force,
        second.wheels[0].longitudinal_force
    );
}

#[test]
fn simplified_combined_slip_respects_budget_without_peak_migration() {
    let configuration = configuration();
    for backend in [
        TireBackend::SimplifiedWithoutRelaxation,
        TireBackend::SimplifiedWithRelaxation,
    ] {
        for wheel in WheelIndex::ALL {
            for angle in [-0.45, -0.115, -0.03, 0.0, 0.03, 0.115, 0.45] {
                for slip in [-0.45, -0.12, 0.0, 0.12, 0.45] {
                    let state = settled(&configuration, backend, wheel, 1800.0, angle, slip, -0.04);
                    let friction = 2.7 * state.load_sensitivity_scale;
                    let longitudinal_ratio =
                        configuration.tires.surface_longitudinal_grip_ratio[&SurfaceType::Road];
                    let longitudinal = (state.longitudinal_force + state.rolling_resistance)
                        / (friction * 1800.0 * longitudinal_ratio);
                    let lateral = state.lateral_force / (friction * 1800.0);
                    assert!(longitudinal * longitudinal + lateral * lateral <= 1.000000001);
                    let profile = if wheel.is_front() {
                        configuration.tires.front_tire_force
                    } else {
                        configuration.tires.rear_tire_force
                    };
                    assert_eq!(
                        state.effective_lateral_peak_slip_rad,
                        profile.lateral_peak_slip_angle_rad
                    );
                    assert_eq!(
                        state.effective_longitudinal_peak_slip_ratio,
                        profile.longitudinal_peak_slip_ratio
                    );
                }
            }
        }
    }
}

#[test]
fn simplified_retains_load_sensitivity_camber_and_aligning_torque() {
    let configuration = configuration();
    let lower = settled(
        &configuration,
        TireBackend::SimplifiedWithoutRelaxation,
        WheelIndex::FrontLeft,
        1800.0,
        0.115,
        0.0,
        0.0,
    );
    let higher = settled(
        &configuration,
        TireBackend::SimplifiedWithoutRelaxation,
        WheelIndex::FrontLeft,
        3600.0,
        0.115,
        0.0,
        0.0,
    );
    assert!(higher.lateral_force > lower.lateral_force);
    assert!(higher.lateral_force < 2.0 * lower.lateral_force);
    let left = settled(
        &configuration,
        TireBackend::SimplifiedWithoutRelaxation,
        WheelIndex::FrontLeft,
        1800.0,
        0.0,
        0.0,
        -0.04,
    );
    let right = settled(
        &configuration,
        TireBackend::SimplifiedWithoutRelaxation,
        WheelIndex::FrontRight,
        1800.0,
        0.0,
        0.0,
        -0.04,
    );
    assert!(left.lateral_force.abs() > 1.0);
    assert!((left.lateral_force + right.lateral_force).abs() < 1e-8);
    assert!(left.aligning_torque * left.lateral_force < 0.0);
}

#[test]
fn simplified_peak_crossing_is_continuous_and_slide_plateau_is_retained() {
    let configuration = configuration();
    let peak = configuration
        .tires
        .front_tire_force
        .lateral_peak_slip_angle_rad;
    let before = settled(
        &configuration,
        TireBackend::SimplifiedWithoutRelaxation,
        WheelIndex::FrontLeft,
        1800.0,
        peak - 1e-6,
        0.0,
        0.0,
    );
    let after = settled(
        &configuration,
        TireBackend::SimplifiedWithoutRelaxation,
        WheelIndex::FrontLeft,
        1800.0,
        peak + 1e-6,
        0.0,
        0.0,
    );
    let sliding = settled(
        &configuration,
        TireBackend::SimplifiedWithoutRelaxation,
        WheelIndex::FrontLeft,
        1800.0,
        0.6,
        0.0,
        0.0,
    );
    assert!((before.lateral_force - after.lateral_force).abs() < 0.01);
    assert!(sliding.lateral_force < before.lateral_force);
    assert!(sliding.lateral_force > 0.8 * before.lateral_force);
}

#[test]
fn simplified_loses_contact_immediately_and_lands_without_fictitious_slip() {
    let configuration = configuration();
    for backend in [
        TireBackend::SimplifiedWithoutRelaxation,
        TireBackend::SimplifiedWithRelaxation,
    ] {
        let mut system = TireSystem::new(&configuration);
        system.backend = backend;
        advance(
            &configuration,
            &mut system,
            WheelIndex::FrontLeft,
            1800.0,
            0.4,
            0.4,
            0.0,
            1.0 / 120.0,
        );
        system.set_mechanical_state(&configuration, WheelIndex::FrontLeft, 0.0, 0.0, 0.0, 0.0);
        system.process_wheel_forces(
            &configuration,
            WheelIndex::FrontLeft,
            0.0,
            SurfaceType::Road,
            2.7,
            7.2,
            1.0,
            false,
            Vec3::ZERO,
            1.0 / 120.0,
        );
        assert_eq!(system.wheels[0].lateral_force, 0.0);
        assert_eq!(system.wheels[0].longitudinal_force, 0.0);
        assert_eq!(system.wheels[0].effective_slip_ratio, 0.0);
        assert_eq!(system.wheels[0].post_peak_decay_lon, 0.0);
        advance(
            &configuration,
            &mut system,
            WheelIndex::FrontLeft,
            1800.0,
            0.0,
            0.0,
            0.0,
            1.0 / 120.0,
        );
        assert!(
            (system.wheels[0].longitudinal_force + system.wheels[0].rolling_resistance).abs()
                < 1e-8
        );
        assert_eq!(system.wheels[0].lateral_force, 0.0);
    }
}

#[test]
fn single_relaxation_is_symmetric_and_frequency_consistent() {
    let configuration = configuration();
    let mut traces = Vec::new();
    for frequency in [120, 240, 480] {
        let mut system = TireSystem::new(&configuration);
        system.backend = TireBackend::SimplifiedWithRelaxation;
        for _ in 0..frequency / 10 {
            advance(
                &configuration,
                &mut system,
                WheelIndex::FrontLeft,
                1800.0,
                0.08,
                0.04,
                0.0,
                1.0 / frequency as f64,
            );
        }
        traces.push((
            system.wheels[0].effective_slip_angle_rad,
            system.wheels[0].effective_slip_ratio,
        ));
    }
    for trace in &traces[1..] {
        assert!((trace.0 - traces[0].0).abs() < 1e-10);
        assert!((trace.1 - traces[0].1).abs() < 1e-10);
    }
    let mut system = TireSystem::new(&configuration);
    system.backend = TireBackend::SimplifiedWithRelaxation;
    advance(
        &configuration,
        &mut system,
        WheelIndex::FrontLeft,
        1800.0,
        0.08,
        0.0,
        0.0,
        1.0 / 120.0,
    );
    let build_fraction = system.wheels[0].effective_slip_angle_rad / 0.08;
    system.wheels[0].effective_slip_angle_rad = 0.08;
    advance(
        &configuration,
        &mut system,
        WheelIndex::FrontLeft,
        1800.0,
        0.0,
        0.0,
        0.0,
        1.0 / 120.0,
    );
    let release_fraction = 1.0 - system.wheels[0].effective_slip_angle_rad / 0.08;
    assert!((build_fraction - release_fraction).abs() < 0.001);
}

#[test]
fn backend_selection_survives_serialization_and_legacy_state_defaults_to_standard() {
    let configuration = configuration();
    let mut system = TireSystem::new(&configuration);
    system.backend = TireBackend::SimplifiedWithRelaxation;
    let encoded = serde_json::to_value(&system).unwrap();
    let restored: TireSystem = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(restored.backend, system.backend);
    let mut legacy = encoded;
    legacy.as_object_mut().unwrap().remove("backend");
    let restored: TireSystem = serde_json::from_value(legacy).unwrap();
    assert_eq!(restored.backend, TireBackend::Standard);
}

#[test]
fn single_relaxation_settles_free_wheel_feedback_at_runtime_frequency() {
    let configuration = configuration();
    for speed in [5.0, 20.0, 60.0] {
        for load_scale in [0.5, 1.0, 2.0] {
            let mut system = TireSystem::new(&configuration);
            system.backend = TireBackend::SimplifiedWithRelaxation;
            let wheel = WheelIndex::FrontLeft;
            let load =
                vehicle_physics_engine::wheel_mechanics::static_wheel_load(&configuration, wheel)
                    * load_scale;
            system.set_mechanical_state(&configuration, wheel, load, 0.0, 0.008, 1.0);
            system.wheels[0].spin = speed * 1.01 / system.wheels[0].effective_rolling_radius;
            let mut squared_slips = 0.0;
            for tick in 0..240 {
                system.process_wheel_torque(&configuration, wheel, 0.0, 0.0, 0.0, 1.0 / 120.0);
                system.process_wheel_forces(
                    &configuration,
                    wheel,
                    load,
                    SurfaceType::Road,
                    2.7,
                    7.2,
                    0.0,
                    false,
                    Vec3::new(0.0, 0.0, -speed),
                    1.0 / 120.0,
                );
                if tick >= 120 {
                    squared_slips += system.wheels[0].slip_ratio.powi(2);
                }
            }
            assert!(
                (squared_slips / 120.0).sqrt() < 0.01,
                "Free rolling wheel failed to settle at speed {speed} and load scale {load_scale}"
            );
        }
    }
}

#[test]
fn single_relaxation_recovers_from_braking_without_recovery_memory() {
    let configuration = configuration();
    let wheel = WheelIndex::FrontLeft;
    let load = vehicle_physics_engine::wheel_mechanics::static_wheel_load(&configuration, wheel);
    let mut system = TireSystem::new(&configuration);
    system.backend = TireBackend::SimplifiedWithRelaxation;
    system.set_mechanical_state(&configuration, wheel, load, 0.0, 0.008, 1.0);
    let radius = system.wheels[0].effective_rolling_radius;
    system.wheels[0].spin = 20.0 / radius;
    let brake_torque = load * 2.7 * radius * 1.5;
    let mut maximum_braking_slip = 0.0_f64;
    for tick in 0..240 {
        let braking = tick < 90;
        system.process_wheel_torque(
            &configuration,
            wheel,
            0.0,
            0.0,
            if braking { brake_torque } else { 0.0 },
            1.0 / 120.0,
        );
        system.process_wheel_forces(
            &configuration,
            wheel,
            load,
            SurfaceType::Road,
            2.7,
            7.2,
            0.0,
            braking,
            Vec3::new(0.0, 0.0, -20.0),
            1.0 / 120.0,
        );
        if braking {
            maximum_braking_slip = maximum_braking_slip.max(system.wheels[0].slip_ratio.abs());
        }
        assert!(system.wheels[0].longitudinal_force.is_finite());
        assert!(system.wheels[0].spin.is_finite());
    }
    assert!(maximum_braking_slip > 0.5);
    assert!(system.wheels[0].slip_ratio.abs() < 0.01);
}
