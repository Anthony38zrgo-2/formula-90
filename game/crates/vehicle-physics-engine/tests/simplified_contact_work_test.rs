use vehicle_physics_engine::tire_thermals::TireMechanicalModifiers;
use vehicle_physics_engine::*;

fn configuration() -> VehicleConfig {
    let repository_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository_root
            .join("game/data/vehicles/f1_1997/williams/profiles/hockenheim/vehicle.json"),
        &repository_root,
    )
    .unwrap()
}

fn planar_samples(configuration: &VehicleConfig, body: BodyKinematics) -> [TriRaycastSample; 4] {
    std::array::from_fn(|index| {
        let wheel = WheelIndex::ALL[index];
        let anchor = configuration.tires.wheel_anchor_local(
            &configuration.chassis,
            &configuration.suspension,
            wheel,
        );
        let spring_length = if wheel.is_front() {
            configuration.suspension.front_spring_length
        } else {
            configuration.suspension.rear_spring_length
        };
        let radius = wheel_mechanics::tire_radius(configuration, wheel);
        let width = if wheel.is_front() {
            configuration.tires.front_tire_width
        } else {
            configuration.tires.rear_tire_width
        };
        let span = width * configuration.suspension.tri_ray_spacing_ratio;
        let direction = body.transform.basis.y;
        let hit = |offset| {
            let origin = body.transform.transform_point(anchor) + body.transform.basis.x * offset;
            let distance = if direction.y > 1e-6 {
                origin.y / direction.y
            } else {
                f64::INFINITY
            };
            RaycastHit {
                is_colliding: distance >= 0.0 && distance <= spring_length + radius,
                distance,
                point: origin - direction * distance,
                normal: Vec3::UP,
                surface: SurfaceType::Road,
            }
        };
        TriRaycastSample {
            inner: hit(span),
            center: hit(0.0),
            outer: hit(-span),
        }
    })
}

fn stationary_body(configuration: &VehicleConfig) -> BodyKinematics {
    BodyKinematics {
        transform: Transform3D::new(
            Vec3::new(0.0, default_spawn_height(configuration), 0.0),
            Mat3::IDENTITY,
        ),
        orientation: Quat::IDENTITY,
        linear_velocity: Vec3::ZERO,
        angular_velocity: Vec3::ZERO,
    }
}

#[test]
fn returning_to_stationary_ground_does_not_convert_detection_history_into_velocity() {
    let configuration = configuration();
    let body = stationary_body(&configuration);
    let samples = planar_samples(&configuration, body);
    let modifiers = [TireMechanicalModifiers::identity(); 4];
    let mut suspension = SuspensionSystem::new(&configuration);
    suspension.select_backend(&configuration, SuspensionBackend::Simplified);
    for _ in 0..360 {
        suspension.step_with_modifiers_and_body_kinematics(
            &configuration,
            &modifiers,
            &samples,
            1.0 / 120.0,
            body,
        );
    }
    let mut absent = samples;
    absent[2] = TriRaycastSample::default();
    absent[3] = TriRaycastSample::default();
    suspension.step_with_modifiers_and_body_kinematics(
        &configuration,
        &modifiers,
        &absent,
        1.0 / 120.0,
        body,
    );
    let mut corrupted_history = suspension.clone();
    for index in [2, 3] {
        corrupted_history.wheels[index].road_compression_m = -10.0;
        corrupted_history.wheels[index].normal_road_compression_metres = 20.0;
    }
    suspension.step_with_modifiers_and_body_kinematics(
        &configuration,
        &modifiers,
        &samples,
        1.0 / 120.0,
        body,
    );
    corrupted_history.step_with_modifiers_and_body_kinematics(
        &configuration,
        &modifiers,
        &samples,
        1.0 / 120.0,
        body,
    );
    for index in [2, 3] {
        assert_eq!(
            suspension.wheels[index].normal_road_velocity_metres_per_second,
            0.0
        );
        assert!(
            (suspension.wheels[index].normal_impulse_newton_seconds
                - corrupted_history.wheels[index].normal_impulse_newton_seconds)
                .abs()
                < 1e-8
        );
    }
}

#[test]
fn normal_contact_uses_conjugate_body_velocity_and_suspension_projection() {
    let configuration = configuration();
    let mut body = stationary_body(&configuration);
    body.transform.basis = Mat3::from_euler_yxz(0.2, 0.02, 0.12);
    body.orientation = Quat::from_mat3(&body.transform.basis);
    body.linear_velocity = Vec3::new(0.3, -0.2, 0.7);
    body.angular_velocity = Vec3::new(0.4, -0.3, 0.2);
    let samples = planar_samples(&configuration, body);
    let mut suspension = SuspensionSystem::new(&configuration);
    suspension.select_backend(&configuration, SuspensionBackend::Simplified);
    suspension.step_with_modifiers_and_body_kinematics(
        &configuration,
        &[TireMechanicalModifiers::identity(); 4],
        &samples,
        1.0 / 120.0,
        body,
    );
    let center = center_of_mass_world_for_suspension_backend(
        &configuration,
        SuspensionBackend::Simplified,
        &body.transform,
    );
    let mut supported = 0;
    for state in suspension.wheels {
        if state.total_normal_force <= 0.0 {
            continue;
        }
        supported += 1;
        let normal = state.effective_normal;
        let velocity = body.linear_velocity
            + body
                .angular_velocity
                .cross(state.effective_contact_point - center);
        let boundary_velocity = state.normal_road_velocity_metres_per_second
            - state.normal_contact_projection_rate_per_second
                * state.normal_force_application_compression_metres;
        assert!((normal.dot(velocity) + boundary_velocity).abs() < 1e-10);
        assert!(
            (state.normal_contact_projection - normal.dot(body.transform.basis.y)).abs() < 1e-12
        );
    }
    assert!(supported >= 2);
}

#[test]
fn periodic_mount_excitation_does_not_receive_net_energy_from_the_suspension() {
    let configuration = configuration();
    let base_body = stationary_body(&configuration);
    let modifiers = [TireMechanicalModifiers::identity(); 4];
    let amplitude = 0.0001;
    for frequency_hertz in [120.0, 240.0, 480.0] {
        let duration = 1.0 / frequency_hertz;
        let mut settled = SuspensionSystem::new(&configuration);
        settled.select_backend(&configuration, SuspensionBackend::Simplified);
        let samples = planar_samples(&configuration, base_body);
        for _ in 0..(3.0 * frequency_hertz) as usize {
            settled.step_with_modifiers_and_body_kinematics(
                &configuration,
                &modifiers,
                &samples,
                duration,
                base_body,
            );
        }
        for excitation_hertz in [5.0, 12.0, 24.0, 30.0, 40.0] {
            let mut suspension = settled.clone();
            let mut work = [0.0; 4];
            for tick in 0..(8.0 * frequency_hertz) as usize {
                let time = tick as f64 * duration;
                let angular_frequency = std::f64::consts::TAU * excitation_hertz;
                let displacement = amplitude * (angular_frequency * time).sin();
                let next_displacement = amplitude * (angular_frequency * (time + duration)).sin();
                let mut body = base_body;
                body.transform.origin.y += displacement;
                body.linear_velocity.y =
                    amplitude * angular_frequency * (angular_frequency * time).cos();
                let samples = planar_samples(&configuration, body);
                suspension.step_with_modifiers_and_body_kinematics(
                    &configuration,
                    &modifiers,
                    &samples,
                    duration,
                    body,
                );
                if tick >= (5.0 * frequency_hertz) as usize {
                    for index in 0..4 {
                        work[index] += suspension.wheels[index].total_normal_force
                            * (next_displacement - displacement);
                    }
                }
            }
            println!("PERIODIC_CONTACT_WORK frequency={frequency_hertz} excitation={excitation_hertz} work={work:?}");
            for delivered_work in work {
                assert!(
                    delivered_work <= 1e-6,
                    "active contact at {frequency_hertz}/{excitation_hertz}: {delivered_work} J"
                );
            }
        }
    }
}

#[test]
fn recorded_rollover_inputs_preserve_support_with_native_ray_lengths() {
    let commands: Vec<Vec<f64>> = include_str!("williams_contact_work_recorded_inputs.tsv")
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(|value| value.parse().unwrap())
                .collect()
        })
        .collect();
    for frequency_hertz in [120.0, 240.0, 480.0] {
        let configuration = configuration();
        let spawn_height = default_spawn_height(&configuration);
        let mut simulator =
            VehicleSimulator::new(configuration, Vec3::new(0.0, spawn_height, 0.0), 0.0);
        simulator.aerodynamics_backend = AerodynamicsBackend::Simplified;
        simulator
            .state
            .suspension
            .select_backend(&simulator.config, SuspensionBackend::Simplified);
        simulator.state.tires.backend = TireBackend::SimplifiedWithRelaxation;
        simulator.state.slow_states =
            slow_states::SlowStateSystem::new(slow_states::SlowStateBackend::Simplified);
        let mut cursor = 0;
        let mut maximum_inclination = 0.0_f64;
        for tick in
            0..((commands.last().unwrap()[0] - commands[0][0]) * frequency_hertz * 0.001) as usize
        {
            let time = commands[0][0] * 0.001 + tick as f64 / frequency_hertz;
            while cursor + 1 < commands.len() && commands[cursor + 1][0] * 0.001 <= time {
                cursor += 1;
            }
            let command = &commands[cursor];
            let input = VehicleInput {
                steering: command[1],
                throttle: command[2],
                brake: command[3],
                gear_request: Some(command[4] as i8),
                ..Default::default()
            };
            let samples = planar_samples(&simulator.config, simulator.state.body_kinematics());
            simulator.step(&input, &samples, 1.0 / frequency_hertz);
            let inclination = simulator
                .state
                .transform
                .basis
                .y
                .y
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            maximum_inclination = maximum_inclination.max(inclination);
        }
        println!("CONTACT_WORK_ROLLOVER_REPLAY frequency={frequency_hertz} maximum_inclination_degrees={maximum_inclination}");
        assert!(
            maximum_inclination < 30.0,
            "rollover at {frequency_hertz}: {maximum_inclination}"
        );
    }
}

#[test]
fn inclined_wheel_momentum_closes_with_the_projected_normal_impulse() {
    let configuration = configuration();
    let mut body = stationary_body(&configuration);
    body.transform.origin.y = 0.48;
    body.transform.basis = Mat3::from_euler_yxz(0.0, 0.0, 0.15);
    body.orientation = Quat::from_mat3(&body.transform.basis);
    let samples = planar_samples(&configuration, body);
    let modifiers = [TireMechanicalModifiers::identity(); 4];
    let mut suspension = SuspensionSystem::new(&configuration);
    suspension.select_backend(&configuration, SuspensionBackend::Simplified);
    let initial_velocities = suspension.wheels.map(|state| state.unsprung_velocity_m_s);
    suspension.step_with_modifiers_and_body_kinematics(
        &configuration,
        &modifiers,
        &samples,
        1.0 / 120.0,
        body,
    );
    for (index, state) in suspension.wheels.iter().enumerate() {
        let mass = WheelMechanicalTuning::for_wheel(&configuration, WheelIndex::ALL[index])
            .unsprung_mass_kg;
        let momentum = mass * (state.unsprung_velocity_m_s - initial_velocities[index]);
        let impulse = state.normal_contact_projection * state.normal_impulse_newton_seconds
            - state.suspension_impulse_newton_seconds
            + state.travel_limit_impulse_newton_seconds;
        assert!(
            (momentum - impulse).abs() < 1e-7,
            "wheel {index}: momentum={momentum} impulse={impulse}"
        );
    }
}
