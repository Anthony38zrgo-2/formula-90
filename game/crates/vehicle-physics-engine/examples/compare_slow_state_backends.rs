use serde_json::{json, Value};
use std::fs::{self, File};
use std::hint::black_box;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;
use vehicle_physics_engine::aero::{AeroEnvironment, AerodynamicsBackend};
use vehicle_physics_engine::simulation::CoupledThermalAdvanceContext;
use vehicle_physics_engine::simulation::{default_spawn_height, BodyKinematics};
use vehicle_physics_engine::slow_states::SlowStateBackend;
use vehicle_physics_engine::suspension::SuspensionBackend;
use vehicle_physics_engine::tire::TireBackend;
use vehicle_physics_engine::tire_thermals::TireMechanicalModifiers;
use vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root;
use vehicle_physics_engine::wheel_mechanics::static_wheel_load;
use vehicle_physics_engine::{
    AidsMask, Mat3, Quat, RaycastHit, SurfaceType, Transform3D, TriRaycastSample, Vec3,
    VehicleConfig, VehicleInput, VehicleSimulator, WheelIndex,
};

const VEHICLE_COUNT: usize = 22;
const SIMULATION_FREQUENCY_HERTZ: usize = 120;

#[derive(Clone, Copy, Debug)]
enum Workload {
    SlowStatesOnly,
    ForceReplay,
    StandaloneFlatRoad,
}

fn percentile(values: &[f64], fraction: f64) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let position = (ordered.len() - 1) as f64 * fraction;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    ordered[lower] + (ordered[upper] - ordered[lower]) * position.fract()
}

fn finite_vector(vector: Vec3) -> bool {
    vector.x.is_finite() && vector.y.is_finite() && vector.z.is_finite()
}

fn plane_hit(origin: Vec3, direction: Vec3, maximum_distance: f64) -> RaycastHit {
    if direction.y >= -1e-8 {
        return RaycastHit::default();
    }
    let distance = -origin.y / direction.y;
    if !(0.0..=maximum_distance).contains(&distance) {
        return RaycastHit::default();
    }
    RaycastHit {
        is_colliding: true,
        distance,
        point: origin + direction * distance,
        normal: Vec3::UP,
        surface: SurfaceType::Road,
    }
}

fn wheel_contacts(configuration: &VehicleConfig, body: BodyKinematics) -> [TriRaycastSample; 4] {
    let direction = body.transform.basis.transform_vector(Vec3::DOWN);
    std::array::from_fn(|wheel_index| {
        let wheel = WheelIndex::ALL[wheel_index];
        let anchor = configuration.tires.wheel_anchor_local(
            &configuration.chassis,
            &configuration.suspension,
            wheel,
        );
        let (width, radius, spring_length) = if wheel.is_front() {
            (
                configuration.tires.front_tire_width,
                configuration.tires.front_tire_radius,
                configuration.suspension.front_spring_length,
            )
        } else {
            (
                configuration.tires.rear_tire_width,
                configuration.tires.rear_tire_radius,
                configuration.suspension.rear_spring_length,
            )
        };
        let span = width * configuration.suspension.tri_ray_spacing_ratio;
        let sample = |offset| {
            plane_hit(
                body.transform
                    .transform_point(anchor + Vec3::new(offset, 0.0, 0.0)),
                direction,
                radius + spring_length,
            )
        };
        TriRaycastSample {
            inner: sample(-span),
            center: sample(0.0),
            outer: sample(span),
        }
    })
}

fn floor_environment(body: BodyKinematics, backend: AerodynamicsBackend) -> AeroEnvironment {
    let mut environment = AeroEnvironment::unavailable();
    let positions = [
        Vec3::new(-0.45, -0.205, -0.90),
        Vec3::new(0.45, -0.205, -0.90),
        Vec3::new(0.0, -0.205, 0.0),
        Vec3::new(0.0, -0.165, 0.85),
        Vec3::new(0.0, -0.165, 1.65),
    ];
    let direction = body.transform.basis.transform_vector(Vec3::DOWN);
    for (index, mut position) in positions.into_iter().enumerate() {
        if backend == AerodynamicsBackend::Simplified {
            if index != 0 && index != 3 {
                continue;
            }
            if index == 0 {
                position.x = 0.0;
            }
        }
        let sample = plane_hit(body.transform.transform_point(position), direction, 0.35);
        if sample.is_colliding {
            environment.valid_mask |= 1 << index;
            environment.clearance_m[index] = sample.distance;
        }
    }
    environment
}

fn replay_body(
    configuration: &VehicleConfig,
    time_seconds: f64,
    vehicle_index: usize,
) -> BodyKinematics {
    let phase = time_seconds * std::f64::consts::TAU / 15.0
        + vehicle_index as f64 * std::f64::consts::TAU / VEHICLE_COUNT as f64;
    let basis = Mat3::from_euler_yxz(0.0, 0.006 * phase.sin(), 0.004 * (phase * 0.7).sin());
    BodyKinematics {
        transform: Transform3D {
            basis,
            origin: Vec3::new(
                vehicle_index as f64 * 10.0,
                default_spawn_height(configuration) + 0.003 * phase.sin(),
                -time_seconds * 40.0,
            ),
        },
        orientation: Quat::from_mat3(&basis),
        linear_velocity: Vec3::new(
            0.7 * phase.sin(),
            0.0,
            -(15.0 + 60.0 * (0.5 - 0.5 * phase.cos())),
        ),
        angular_velocity: Vec3::new(
            0.002 * phase.cos(),
            0.003 * phase.sin(),
            0.001 * phase.cos(),
        ),
    }
}

fn driver_input(configuration: &VehicleConfig, speed: f64, time_seconds: f64) -> VehicleInput {
    let phase = time_seconds.rem_euclid(12.0);
    let throttle = if phase < 7.0 {
        0.75
    } else if phase < 9.0 {
        0.0
    } else {
        0.25
    };
    let brake = if (7.0..9.0).contains(&phase) {
        0.25
    } else {
        0.0
    };
    let gear = configuration
        .powertrain
        .gear_ratios
        .iter()
        .enumerate()
        .find(|(_, ratio)| {
            speed / configuration.tires.rear_tire_radius
                * ratio.abs()
                * configuration.powertrain.final_drive
                * 60.0
                / std::f64::consts::TAU
                < configuration.powertrain.max_rpm * 0.85
        })
        .map(|(index, _)| index as i8 + 1)
        .unwrap_or(configuration.powertrain.gear_ratios.len() as i8);
    VehicleInput {
        throttle,
        brake,
        steering: 0.0,
        gear_request: Some(gear),
        ..VehicleInput::default()
    }
}

fn run_workload(
    configuration: &VehicleConfig,
    backend: SlowStateBackend,
    workload: Workload,
    warmup_seconds: usize,
    measured_seconds: usize,
    repetition: usize,
    writer: &mut impl Write,
    collect_coverage: bool,
) -> Result<Value, String> {
    let duration_seconds = 1.0 / SIMULATION_FREQUENCY_HERTZ as f64;
    let warmup_ticks = warmup_seconds * SIMULATION_FREQUENCY_HERTZ;
    let measured_ticks = measured_seconds * SIMULATION_FREQUENCY_HERTZ;
    let preparation_started = Instant::now();
    let mut vehicles: Vec<VehicleSimulator> = (0..VEHICLE_COUNT)
        .map(|index| {
            let mut vehicle = VehicleSimulator::new(
                configuration.clone(),
                Vec3::new(
                    index as f64 * 10.0,
                    default_spawn_height(configuration),
                    0.0,
                ),
                0.0,
            );
            vehicle.aerodynamics_backend = AerodynamicsBackend::Simplified;
            vehicle
                .state
                .suspension
                .select_backend(configuration, SuspensionBackend::Simplified);
            vehicle.state.tires.backend = TireBackend::SimplifiedWithRelaxation;
            vehicle.select_slow_state_backend(backend);
            vehicle.aids = AidsMask::from_bits(1 << 5);
            vehicle.state.linear_velocity = Vec3::new(0.0, 0.0, -30.0);
            for wheel in WheelIndex::ALL {
                vehicle.state.tires.wheels[wheel as usize].spin = 30.0
                    / if wheel.is_front() {
                        configuration.tires.front_tire_radius
                    } else {
                        configuration.tires.rear_tire_radius
                    };
            }
            vehicle
        })
        .collect();
    let preparation_grid_milliseconds = preparation_started.elapsed().as_secs_f64() * 1000.0;
    let mut measurements = Vec::with_capacity(measured_ticks);
    let mut numerical_trace = Vec::new();
    let mut checksum = 0.0;
    let mut maximum_tilt_degrees = 0.0_f64;
    let mut minimum_body_height = f64::INFINITY;
    let mut contact_wheel_ticks: usize = 0;
    let mut operating_vehicle_ticks: usize = 0;
    let mut maximum_speed = 0.0_f64;
    let mut minimum_speed = f64::INFINITY;
    for tick in 0..warmup_ticks + measured_ticks {
        let time_seconds = tick as f64 * duration_seconds;
        let replay: Vec<BodyKinematics> = (0..VEHICLE_COUNT)
            .map(|index| replay_body(configuration, time_seconds, index))
            .collect();

        let started = Instant::now();
        for vehicle_index in 0..VEHICLE_COUNT {
            match workload {
                Workload::SlowStatesOnly => {
                    let vehicle = &mut vehicles[vehicle_index];
                    let phase = time_seconds * 1.7 + vehicle_index as f64 * 0.2;
                    let speed = 30.0 + 20.0 * (phase * 0.3).sin();
                    let normalised_engine_speed = 0.65 + 0.2 * (phase * 0.5).sin();
                    vehicle.state.powertrain.rpm =
                        configuration.powertrain.max_rpm * normalised_engine_speed;
                    vehicle.state.powertrain.engine_torque = configuration.powertrain.max_torque
                        * configuration
                            .powertrain
                            .evaluate_torque_curve(normalised_engine_speed)
                        * (0.6 + 0.3 * phase.sin());
                    vehicle.state.brake_thermal.update_cooling_ducts(
                        &configuration.brakes.brake_thermal,
                        configuration.aerodynamics.air_density,
                        speed,
                    );
                    vehicle.state.powertrain_thermal.evaluate_cooling_ducts(
                        &configuration.powertrain.powertrain_thermal,
                        configuration.aerodynamics.air_density,
                        speed,
                    );
                    let mut context = CoupledThermalAdvanceContext {
                        applied_brake_torque_newton_metres: [configuration.brakes.max_brake_torque
                            * if time_seconds.rem_euclid(5.0) < 1.0 {
                                0.12
                            } else {
                                0.0
                            }; 4],
                        wheel_spin_before_radians_per_second: [0.0; 4],
                        wheel_lateral_slip_metres_per_second: [1.0 + 0.5 * phase.cos(); 4],
                        thermal_modifiers: [TireMechanicalModifiers::identity(); 4],
                        vehicle_speed_metres_per_second: speed,
                        driver_throttle_fraction: 0.7,
                        duration_seconds,
                    };
                    for wheel in WheelIndex::ALL {
                        let index = wheel as usize;
                        let load =
                            static_wheel_load(configuration, wheel) * (1.0 + 0.4 * phase.sin());
                        let suspension = &mut vehicle.state.suspension.wheels[index];
                        suspension.total_normal_force = load;
                        suspension.ray_grounded = [true; 3];
                        suspension.effective_friction =
                            configuration.tires.surface_friction[&SurfaceType::Road];
                        suspension.tire_deflection_m = 0.008;
                        suspension.tire_deflection_velocity_m_s = 0.01 * phase.cos();
                        let tire = &mut vehicle.state.tires.wheels[index];
                        tire.longitudinal_force = load * 0.6;
                        tire.lateral_force = load * 0.3 * phase.sin();
                        tire.spin_velocity_diff = 2.0 + phase.sin();
                        tire.spin = speed
                            / if wheel.is_front() {
                                configuration.tires.front_tire_radius
                            } else {
                                configuration.tires.rear_tire_radius
                            };
                        context.wheel_spin_before_radians_per_second[index] = tire.spin;
                    }
                    VehicleSimulator::advance_thermal_systems(
                        configuration,
                        &mut vehicle.state,
                        &context,
                        None,
                    )?;
                }
                Workload::ForceReplay | Workload::StandaloneFlatRoad => {
                    let vehicle = &mut vehicles[vehicle_index];
                    let body = if matches!(workload, Workload::ForceReplay) {
                        replay[vehicle_index]
                    } else {
                        vehicle.state.body_kinematics()
                    };
                    let samples = wheel_contacts(configuration, body);
                    let environment = floor_environment(body, AerodynamicsBackend::Simplified);
                    let input =
                        driver_input(configuration, body.linear_velocity.length(), time_seconds);
                    let telemetry = if matches!(workload, Workload::ForceReplay) {
                        let (forces, telemetry) = vehicle.solve_external_with_aero(
                            body,
                            &input,
                            &samples,
                            &environment,
                            duration_seconds,
                        );
                        black_box(forces);
                        telemetry
                    } else {
                        vehicle.step_with_aerodynamics_environment(
                            &input,
                            &samples,
                            &environment,
                            duration_seconds,
                        )
                    };
                    if vehicle.state.aero.invalid_input
                        || !finite_vector(vehicle.state.linear_velocity)
                        || !finite_vector(vehicle.state.transform.origin)
                        || !telemetry.speed_kmh.is_finite()
                    {
                        return Err(format!(
                            "Nonfinite simulation at tick {tick}, vehicle {vehicle_index}"
                        ));
                    }
                    checksum += black_box(
                        telemetry.speed_kmh
                            + vehicle.state.aero.total_downforce
                            + vehicle
                                .state
                                .tires
                                .wheels
                                .iter()
                                .map(|state| state.lateral_force + state.longitudinal_force)
                                .sum::<f64>(),
                    );
                }
            }
        }
        for vehicle in &vehicles {
            let tire = vehicle.state.tire_thermal.wheels[0];
            checksum += black_box(
                tire.average_tread_c()
                    + tire.pressure_kpa_gauge
                    + vehicle.state.brake_thermal.wheels[0].disc_c
                    + vehicle.state.powertrain_thermal.water_temperature_celsius,
            );
        }
        let elapsed_microseconds = started.elapsed().as_secs_f64() * 1_000_000.0;
        if tick >= warmup_ticks {
            measurements.push(elapsed_microseconds);
            if repetition == 0 && !collect_coverage {
                numerical_trace.push(
                    vehicles
                        .iter()
                        .map(|vehicle| {
                            [
                                vehicle.state.transform.origin.y,
                                vehicle.state.linear_velocity.length(),
                                vehicle.state.tire_thermal.wheels[0].average_tread_c(),
                                vehicle.state.tire_thermal.wheels[0].carcass_c,
                                vehicle.state.tire_thermal.wheels[0].gas_c,
                                vehicle.state.tire_thermal.wheels[0].pressure_kpa_gauge,
                                vehicle.state.brake_thermal.wheels[0].disc_c,
                                vehicle.state.brake_thermal.wheels[0].rim_c,
                                vehicle
                                    .state
                                    .powertrain_thermal
                                    .engine_block_temperature_celsius,
                                vehicle.state.powertrain_thermal.water_temperature_celsius,
                                vehicle.state.powertrain_thermal.oil_temperature_celsius,
                                vehicle.state.tire_wear.wheels[0].remaining_tread_fraction(),
                            ]
                        })
                        .collect::<Vec<_>>(),
                );
            }
            for vehicle in &vehicles {
                for wheel in &vehicle.state.suspension.wheels {
                    if !wheel.total_normal_force.is_finite()
                        || !wheel.suspension_compression_m.is_finite()
                        || !wheel.balance_residual_n.is_finite()
                    {
                        return Err("Nonfinite suspension state".into());
                    }
                }
            }
            if !matches!(workload, Workload::SlowStatesOnly) {
                for vehicle in &vehicles {
                    let body = vehicle.state.body_kinematics();
                    maximum_tilt_degrees = maximum_tilt_degrees.max(
                        body.transform
                            .basis
                            .y
                            .y
                            .clamp(-1.0, 1.0)
                            .acos()
                            .to_degrees(),
                    );
                    minimum_body_height = minimum_body_height.min(body.transform.origin.y);
                    let speed = body.linear_velocity.length();
                    maximum_speed = maximum_speed.max(speed);
                    minimum_speed = minimum_speed.min(speed);
                    contact_wheel_ticks += wheel_contacts(configuration, body)
                        .iter()
                        .filter(|sample| sample.has_any_contact())
                        .count();
                    operating_vehicle_ticks += 1;
                }
            }
        }
    }
    for (tick, measurement) in measurements.iter().enumerate() {
        writeln!(
            writer,
            "{workload:?},{backend:?},{repetition},{tick},{measurement:.6}"
        )
        .map_err(|error| error.to_string())?;
    }
    let mean = measurements.iter().sum::<f64>() / measurements.len() as f64;
    let timing = json!({"mean_grid_microseconds": mean, "p50_grid_microseconds": percentile(&measurements, 0.50),
        "p95_grid_microseconds": percentile(&measurements, 0.95), "p99_grid_microseconds": percentile(&measurements, 0.99),
        "mean_vehicle_microseconds": mean / VEHICLE_COUNT as f64,
        "single_thread_budget_percent_at_120_hertz": mean * SIMULATION_FREQUENCY_HERTZ as f64 / 10000.0,
        "measured_grid_ticks": measured_ticks, "measured_vehicle_ticks": measured_ticks * VEHICLE_COUNT,
        "checksum": checksum});
    let contact_fraction = if operating_vehicle_ticks > 0 {
        contact_wheel_ticks as f64 / (operating_vehicle_ticks * 4) as f64
    } else {
        0.0
    };
    let quality = if matches!(workload, Workload::SlowStatesOnly) {
        json!({"finite_outputs": true})
    } else {
        json!({"finite_outputs": true, "maximum_body_tilt_degrees": maximum_tilt_degrees,
            "minimum_body_origin_height_metres": minimum_body_height, "contact_wheel_tick_fraction": contact_fraction,
            "minimum_speed_metres_per_second": minimum_speed, "maximum_speed_metres_per_second": maximum_speed,
            "flat_road_motion_gate_passed": maximum_tilt_degrees < 45.0 && minimum_body_height > 0.0 && contact_fraction > 0.9})
    };
    Ok(
        json!({"workload": format!("{workload:?}"), "backend": format!("{backend:?}"), "repetition": repetition,
        "numerical_trace": numerical_trace, "slow_state_accounting": vehicles.iter().map(|vehicle| vehicle.state.slow_states.accounting).collect::<Vec<_>>(), "preparation_grid_milliseconds": preparation_grid_milliseconds,  "timing": timing, "quality": quality}),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("FORMULA90_SUSPENSION_BACKEND", "standard");
    std::env::set_var("FORMULA90_SLOW_STATE_BACKEND", "standard");
    let arguments: Vec<String> = std::env::args().collect();
    let repository_root =
        PathBuf::from(arguments.get(1).ok_or("Expected repository root")?).canonicalize()?;
    let output_directory = PathBuf::from(arguments.get(2).ok_or("Expected new output directory")?);
    let measured_seconds: usize = arguments
        .get(3)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(30);
    let repetitions: usize = arguments
        .get(4)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(5);
    let warmup_seconds: usize = arguments
        .get(5)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(5);
    if measured_seconds == 0 || repetitions == 0 {
        return Err("Duration and repetitions must be positive".into());
    }
    if output_directory.exists() {
        return Err("Output directory already exists".into());
    }
    fs::create_dir_all(&output_directory)?;
    let profile_path =
        repository_root.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json");
    let configuration =
        load_vehicle_configuration_from_authorized_root(&profile_path, &repository_root)?;
    let mut writer = BufWriter::new(File::create(
        output_directory.join("grid_tick_timings.csv"),
    )?);
    writeln!(writer, "workload,backend,repetition,tick,grid_microseconds")?;
    let mut runs = Vec::new();
    for workload in [
        Workload::SlowStatesOnly,
        Workload::ForceReplay,
        Workload::StandaloneFlatRoad,
    ] {
        for repetition in 0..repetitions {
            let mut order = [SlowStateBackend::Standard, SlowStateBackend::Simplified];
            if repetition % 2 != 0 {
                order.reverse();
            }
            for backend in order {
                eprintln!(
                    "Running {workload:?} {backend:?} repetition {}/{}",
                    repetition + 1,
                    repetitions
                );
                let mut result = run_workload(
                    &configuration,
                    backend,
                    workload,
                    warmup_seconds,
                    measured_seconds,
                    repetition,
                    &mut writer,
                    false,
                )?;
                eprintln!(
                    "{}",
                    json!({"timing": result["timing"], "quality": result["quality"]})
                );
                if repetition == 0 {
                    fs::write(
                        output_directory.join(format!("numerical_{workload:?}_{backend:?}.json")),
                        serde_json::to_vec(&result["numerical_trace"])?,
                    )?;
                }
                result.as_object_mut().unwrap().remove("numerical_trace");
                runs.push(result);
            }
        }
    }
    writer.flush()?;
    let report = json!({"vehicle_count": VEHICLE_COUNT, "simulation_frequency_hertz": SIMULATION_FREQUENCY_HERTZ,
        "measured_seconds_per_run": measured_seconds, "warmup_seconds_per_run": warmup_seconds,
        "repetitions_per_backend_per_workload": repetitions, "profile": profile_path,
        "retained_aerodynamics": "Simplified", "suspension_contact_rays": 12,
        "retained_suspension": "Simplified", "retained_tire_backend": "SimplifiedWithRelaxation",
        "slow_state_update_frequencies": vehicle_physics_engine::slow_states::SlowStateUpdateFrequencies::default(),
        "resolved_configuration": serde_json::to_value(&configuration)?,
        "execution": "release Rust; sequential batch of 22 independent simulators on one thread; no real-time sleeping",
        "aids": "only automatic clutch; stability, traction control, ABS and steering assistance disabled",
        "limitations": ["Synthetic flat road; no Fuji geometry", "No vehicle-to-vehicle collision, racecraft, audio, FFI, Godot raycast or rendering cost", "ForceReplay prescribes body motion for equal-work comparison", "StandaloneFlatRoad integrates body and wheel dynamics; motion can diverge between backends", "SlowStatesOnly prescribes identical mechanical work, loads and airflow; complete workloads retain thermal mechanical feedback and may diverge", "Ground contacts use analytic ray/plane intersections without scrape or rigid-chassis contact fusion"],
        "runs": runs});
    fs::write(
        output_directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", output_directory.join("report.json").display());
    Ok(())
}
