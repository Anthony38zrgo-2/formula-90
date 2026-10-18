use serde::Deserialize;
use vehicle_physics_engine::slow_states::{SlowStateBackend, SlowStateSystem};
use vehicle_physics_engine::*;

#[derive(Deserialize)]
struct RecordedSuspensionInputs {
    samples: Vec<RecordedSuspensionInput>,
}

#[derive(Deserialize)]
struct RecordedSuspensionInput {
    time_seconds: f64,
    steering: f64,
    throttle: f64,
    brake: f64,
    gear: i8,
}

fn williams_configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root
            .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    )
    .unwrap()
}

fn planar_samples(simulator: &VehicleSimulator) -> [TriRaycastSample; 4] {
    std::array::from_fn(|index| {
        let wheel = WheelIndex::ALL[index];
        let anchor_local = simulator.config.tires.wheel_anchor_local(
            &simulator.config.chassis,
            &simulator.config.suspension,
            wheel,
        );
        let anchor_world = simulator.state.transform.transform_point(anchor_local);
        let suspension_direction_world = simulator.state.transform.basis.y;
        let span_metres = if wheel.is_front() {
            simulator.config.tires.front_tire_width
        } else {
            simulator.config.tires.rear_tire_width
        } * simulator.config.suspension.tri_ray_spacing_ratio;
        let hit = |offset_metres| {
            let origin_world = anchor_world + simulator.state.transform.basis.x * offset_metres;
            let distance_metres = if suspension_direction_world.y > 0.1 {
                origin_world.y / suspension_direction_world.y
            } else {
                100.0
            };
            RaycastHit {
                is_colliding: distance_metres >= 0.0 && distance_metres < 1.0,
                distance: distance_metres.max(0.0),
                point: origin_world - suspension_direction_world * distance_metres,
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            }
        };
        TriRaycastSample {
            inner: hit(span_metres),
            center: hit(0.0),
            outer: hit(-span_metres),
        }
    })
}

#[test]
fn recorded_williams_inputs_remain_upright_at_runtime_frequency() {
    let recorded: RecordedSuspensionInputs =
        serde_json::from_str(include_str!("williams_recorded_suspension_inputs.json")).unwrap();
    for frequency_hertz in [120, 240, 480] {
        let configuration = williams_configuration();
        let spawn_height_metres = default_spawn_height(&configuration);
        let mut simulator =
            VehicleSimulator::new(configuration, Vec3::new(0.0, spawn_height_metres, 0.0), 0.0);
        simulator.aerodynamics_backend = AerodynamicsBackend::Simplified;
        simulator
            .state
            .suspension
            .select_backend(&simulator.config, SuspensionBackend::Simplified);
        simulator.state.tires.backend = TireBackend::SimplifiedWithRelaxation;
        simulator.state.slow_states = SlowStateSystem::new(SlowStateBackend::Simplified);
        let duration_seconds = 1.0 / frequency_hertz as f64;
        let first_time_seconds = recorded.samples[0].time_seconds;
        let last_time_seconds = recorded.samples.last().unwrap().time_seconds;
        let mut cursor = 0;
        let mut maximum_vertical_speed_metres_per_second = 0.0_f64;
        let mut maximum_inclination_radians = 0.0_f64;
        let mut maximum_normal_force_newtons = 0.0_f64;
        let mut maximum_speed_kilometres_per_hour = 0.0_f64;
        for tick in 0..((last_time_seconds - first_time_seconds) * frequency_hertz as f64) as usize
        {
            let time_seconds = first_time_seconds + tick as f64 * duration_seconds;
            while cursor + 1 < recorded.samples.len()
                && recorded.samples[cursor + 1].time_seconds <= time_seconds
            {
                cursor += 1;
            }
            let command = &recorded.samples[cursor];
            let input = VehicleInput {
                steering: command.steering,
                throttle: command.throttle,
                brake: command.brake,
                gear_request: Some(command.gear),
                ..VehicleInput::default()
            };
            let samples = planar_samples(&simulator);
            simulator.step(&input, &samples, duration_seconds);
            let inclination_radians = simulator.state.transform.basis.y.y.clamp(-1.0, 1.0).acos();
            maximum_inclination_radians = maximum_inclination_radians.max(inclination_radians);
            maximum_vertical_speed_metres_per_second = maximum_vertical_speed_metres_per_second
                .max(simulator.state.linear_velocity.y.abs());
            maximum_speed_kilometres_per_hour = maximum_speed_kilometres_per_hour
                .max(simulator.state.linear_velocity.length() * 3.6);
            maximum_normal_force_newtons = maximum_normal_force_newtons.max(
                simulator
                    .state
                    .suspension
                    .wheels
                    .iter()
                    .map(|wheel| wheel.total_normal_force)
                    .sum(),
            );
            assert!(
                inclination_radians < 30.0_f64.to_radians(),
                "{frequency_hertz} Hz inclination {} at {time_seconds}",
                inclination_radians.to_degrees()
            );
            for wheel in &simulator.state.suspension.wheels {
                assert!(
                    (wheel.total_normal_force * duration_seconds
                        - wheel.normal_impulse_newton_seconds)
                        .abs()
                        < 1e-9
                );
            }
        }
        println!(
            "WILLIAMS_VERTICAL_REPLAY {}",
            serde_json::json!({
                "frequency_hertz":frequency_hertz,
                "maximum_vertical_speed_metres_per_second":maximum_vertical_speed_metres_per_second,
                "maximum_inclination_degrees":maximum_inclination_radians.to_degrees(),
                "maximum_normal_force_newtons":maximum_normal_force_newtons,
                "maximum_speed_kilometres_per_hour":maximum_speed_kilometres_per_hour
            })
        );
        assert!(maximum_speed_kilometres_per_hour > 240.0);
        assert!(maximum_vertical_speed_metres_per_second < 1.5);
        assert!(maximum_normal_force_newtons < 24_000.0);
    }
}

#[test]
fn wheel_impulses_close_momentum_and_match_the_external_body_force() {
    for configuration in [VehicleConfig::f1_94_canonical(), williams_configuration()] {
        let spawn_height_metres = default_spawn_height(&configuration);
        let mut simulator =
            VehicleSimulator::new(configuration, Vec3::new(0.0, spawn_height_metres, 0.0), 0.0);
        simulator
            .state
            .suspension
            .select_backend(&simulator.config, SuspensionBackend::Simplified);
        let duration_seconds = 1.0 / 120.0;
        for tick in 0..60 {
            let mut samples = planar_samples(&simulator);
            if tick >= 30 {
                for sample in &mut samples {
                    sample.inner.distance -= 0.08;
                    sample.center.distance -= 0.08;
                    sample.outer.distance -= 0.08;
                }
            }
            let previous_velocities: [f64; 4] = std::array::from_fn(|index| {
                simulator.state.suspension.wheels[index].unsprung_velocity_m_s
            });
            let (forces, _) = simulator.solve_external(
                simulator.state.body_kinematics(),
                &VehicleInput::default(),
                &samples,
                duration_seconds,
            );
            let normal_impulse_newton_seconds = simulator
                .state
                .suspension
                .wheels
                .iter()
                .map(|wheel| wheel.normal_impulse_newton_seconds)
                .sum::<f64>();
            let suspension_force_newtons =
                forces.force_world.y - simulator.state.aero.force_local.y;
            assert!((suspension_force_newtons * duration_seconds - normal_impulse_newton_seconds).abs()
                    < 1e-7 * normal_impulse_newton_seconds.abs().max(1.0),
                "tick {tick}: body impulse {}, wheel impulse {normal_impulse_newton_seconds}, aerodynamic force {}",
                suspension_force_newtons * duration_seconds, simulator.state.aero.force_local.y);
            for index in 0..4 {
                let wheel = &simulator.state.suspension.wheels[index];
                let mass_kilograms =
                    WheelMechanicalTuning::for_wheel(&simulator.config, WheelIndex::ALL[index])
                        .unsprung_mass_kg
                        .max(1.0);
                let momentum_change_newton_seconds =
                    mass_kilograms * (wheel.unsprung_velocity_m_s - previous_velocities[index]);
                let net_impulse_newton_seconds = wheel.normal_impulse_newton_seconds
                    - wheel.suspension_impulse_newton_seconds
                    + wheel.travel_limit_impulse_newton_seconds;
                assert!((momentum_change_newton_seconds - net_impulse_newton_seconds).abs() < 1e-7);
            }
        }
    }
}

#[test]
fn invalid_linkage_keeps_the_external_contact_impulse_and_moment() {
    let configuration = williams_configuration();
    let mut suspension = SuspensionSystem::new(&configuration);
    suspension.select_backend(&configuration, SuspensionBackend::Simplified);
    let body = Transform3D::new(
        Vec3::new(3.0, 0.6, -8.0),
        Mat3::from_euler_yxz(0.7, 0.1, -0.2),
    );
    let center_of_gravity_local = center_of_mass_local(&configuration);
    let contact_point_world = body.transform_point(Vec3::new(-0.8, -0.5, 1.1));
    let contact_force_world = Vec3::new(2500.0, 11_000.0, -1300.0);
    let aligning_couple_world = Vec3::new(0.0, 60.0, 0.0);
    let mut invalid_geometry = configuration.suspension.geometric_suspension.as_ref().unwrap().clone();
    invalid_geometry.corners.rl.lower.inner_rear = invalid_geometry.corners.rl.lower.inner_front;
    for grounded in [false, true] {
        let wrench = suspension.wheel_wrench(
            &invalid_geometry,
            WheelIndex::RearLeft,
            2.0,
            0.0,
            contact_force_world,
            contact_point_world,
            aligning_couple_world,
            grounded,
            0.0,
            0.0,
            800.0,
            100.0,
            0.0,
            0.4,
            0.0,
            body,
            center_of_gravity_local,
        );
        assert!(wrench.paths.singular);
        let expected_force_world = if grounded {
            contact_force_world
        } else {
            Vec3::ZERO
        };
        let expected_torque_world = if grounded {
            (contact_point_world - body.transform_point(center_of_gravity_local))
                .cross(contact_force_world)
                + aligning_couple_world
        } else {
            Vec3::ZERO
        };
        assert!((wrench.force_world - expected_force_world).length() < 1e-10);
        assert!((wrench.torque_world - expected_torque_world).length() < 1e-10);
    }
}
