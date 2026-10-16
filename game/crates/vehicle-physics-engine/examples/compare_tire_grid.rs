use serde_json::{json, Value};
use std::fs::{self, File};
use std::hint::black_box;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;
use vehicle_physics_engine::aero::{AeroEnvironment, AerodynamicsBackend};
use vehicle_physics_engine::simulation::{default_spawn_height, BodyKinematics};
use vehicle_physics_engine::suspension::SuspensionBackend;
use vehicle_physics_engine::tire::{TireBackend, TireSystem};
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
    TiresOnly,
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
    backend: TireBackend,
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
            vehicle.state.tires.backend = backend;
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
                Workload::TiresOnly => {
                    let vehicle = &mut vehicles[vehicle_index];
                    let phase = time_seconds * 1.7 + vehicle_index as f64 * 0.2;
                    let speed = 40.0 + 25.0 * (phase * 0.3).sin();
                    let angle = 0.3 * phase.sin();
                    let slip = 0.3 * (phase * 0.7).sin();
                    for wheel in WheelIndex::ALL {
                        let load = static_wheel_load(configuration, wheel)
                            * (1.0 + 0.5 * (phase * 0.5).sin());
                        vehicle.state.tires.set_mechanical_state(
                            configuration,
                            wheel,
                            load,
                            -0.04,
                            0.008,
                            1.0,
                        );
                        let radius =
                            vehicle.state.tires.wheels[wheel as usize].effective_rolling_radius;
                        vehicle.state.tires.wheels[wheel as usize].spin = if slip >= 0.0 {
                            speed / (1.0 - slip) / radius
                        } else {
                            speed * (1.0 + slip) / radius
                        };
                        vehicle.state.tires.process_wheel_forces(
                            configuration,
                            wheel,
                            load,
                            SurfaceType::Road,
                            2.7,
                            7.2,
                            1.0,
                            false,
                            Vec3::new(-angle.tan() * speed, 0.0, -speed),
                            duration_seconds,
                        );
                        let state = &vehicle.state.tires.wheels[wheel as usize];
                        if !state.lateral_force.is_finite()
                            || !state.longitudinal_force.is_finite()
                            || !state.aligning_torque.is_finite()
                            || state.combined_utilization > 1.000000001
                        {
                            return Err("Invalid isolated tire state".into());
                        }
                        checksum += black_box(
                            state.lateral_force + state.longitudinal_force + state.aligning_torque,
                        );
                    }
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
        let elapsed_microseconds = started.elapsed().as_secs_f64() * 1_000_000.0;
        if tick >= warmup_ticks {
            measurements.push(elapsed_microseconds);
            if repetition == 0 && !collect_coverage {
                numerical_trace.push(
                    vehicles
                        .iter()
                        .map(|vehicle| {
                            let wheels = &vehicle.state.suspension.wheels;
                            [
                                vehicle.state.transform.origin.y,
                                vehicle.state.linear_velocity.length(),
                                wheels[0].total_normal_force,
                                wheels[1].total_normal_force,
                                wheels[2].total_normal_force,
                                wheels[3].total_normal_force,
                                wheels[0].suspension_compression_m,
                                wheels[1].suspension_compression_m,
                                wheels[2].suspension_compression_m,
                                wheels[3].suspension_compression_m,
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
            if !matches!(workload, Workload::TiresOnly) {
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
    let quality = if matches!(workload, Workload::TiresOnly) {
        json!({"finite_outputs": true})
    } else {
        json!({"finite_outputs": true, "maximum_body_tilt_degrees": maximum_tilt_degrees,
            "minimum_body_origin_height_metres": minimum_body_height, "contact_wheel_tick_fraction": contact_fraction,
            "minimum_speed_metres_per_second": minimum_speed, "maximum_speed_metres_per_second": maximum_speed,
            "flat_road_motion_gate_passed": maximum_tilt_degrees < 45.0 && minimum_body_height > 0.0 && contact_fraction > 0.9})
    };
    Ok(
        json!({"workload": format!("{workload:?}"), "backend": format!("{backend:?}"), "repetition": repetition,
        "numerical_trace": numerical_trace, "preparation_grid_milliseconds": preparation_grid_milliseconds,  "timing": timing, "quality": quality}),
    )
}

fn relaxation_ablation(configuration: &VehicleConfig) -> Value {
    let mut results = Vec::new();
    for backend in [
        TireBackend::Standard,
        TireBackend::SimplifiedWithoutRelaxation,
        TireBackend::SimplifiedWithRelaxation,
    ] {
        for frequency in [120, 240, 480] {
            for speed in [5.0, 20.0, 60.0] {
                for load_scale in [0.5, 1.0, 2.0] {
                    let mut system = TireSystem::new(configuration);
                    system.backend = backend;
                    let wheel = WheelIndex::FrontLeft;
                    let load = static_wheel_load(configuration, wheel) * load_scale;
                    system.set_mechanical_state(configuration, wheel, load, 0.0, 0.008, 1.0);
                    let radius = system.wheels[0].effective_rolling_radius;
                    system.wheels[0].spin = speed * 1.01 / radius;
                    let mut squared_slips = 0.0;
                    let mut maximum_slip = 0.0_f64;
                    let mut samples = 0;
                    let mut sign_changes = 0;
                    let mut previous_slip = 0.0_f64;
                    for tick in 0..frequency * 2 {
                        system.process_wheel_torque(
                            configuration,
                            wheel,
                            0.0,
                            0.0,
                            0.0,
                            1.0 / frequency as f64,
                        );
                        system.process_wheel_forces(
                            configuration,
                            wheel,
                            load,
                            SurfaceType::Road,
                            2.7,
                            7.2,
                            0.0,
                            false,
                            Vec3::new(0.0, 0.0, -speed),
                            1.0 / frequency as f64,
                        );
                        let slip = system.wheels[0].slip_ratio;
                        if tick >= frequency {
                            squared_slips += slip * slip;
                            maximum_slip = maximum_slip.max(slip.abs());
                            if previous_slip * slip < 0.0 {
                                sign_changes += 1;
                            }
                            samples += 1;
                        }
                        previous_slip = slip;
                    }
                    results.push(json!({"backend":format!("{backend:?}"),"frequency_hertz":frequency,"speed_metres_per_second":speed,"load_scale":load_scale,"final_second_slip_root_mean_square":(squared_slips/samples as f64).sqrt(),"final_second_maximum_slip":maximum_slip,"final_second_sign_changes":sign_changes}));
                }
            }
        }
    }
    json!({"scenario":"Prescribed constant chassis speed; freely spinning wheel with 1 percent initial excess spin, zero drive/brake torque and zero rolling resistance; actual angular inertia and tire feedback", "criterion":"Prefer no relaxation if final-second slip root mean square is below 0.01 for every 120 Hz case; retain one symmetric relaxation only if it resolves failures", "runs":results})
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("FORMULA90_SUSPENSION_BACKEND", "standard");
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
        Workload::TiresOnly,
        Workload::ForceReplay,
        Workload::StandaloneFlatRoad,
    ] {
        for repetition in 0..repetitions {
            let mut order = [
                TireBackend::Standard,
                TireBackend::SimplifiedWithoutRelaxation,
                TireBackend::SimplifiedWithRelaxation,
            ];
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
        "retained_suspension": "Simplified",
        "resolved_configuration": serde_json::to_value(&configuration)?,
        "execution": "release Rust; sequential batch of 22 independent simulators on one thread; no real-time sleeping",
        "aids": "only automatic clutch; stability, traction control, ABS and steering assistance disabled",
        "limitations": ["Synthetic flat road; no Fuji geometry", "No vehicle-to-vehicle collision, racecraft, audio, FFI, Godot raycast or rendering cost", "ForceReplay prescribes body motion for equal-work comparison", "StandaloneFlatRoad integrates body and wheel dynamics; motion can diverge between backends", "TiresOnly prescribes identical load, slips, camber and neutral pressure/thermal modifiers; complete workloads share initial conditions but evolve internal state", "Ground contacts use analytic ray/plane intersections without scrape or rigid-chassis contact fusion"],
        "relaxation_ablation": relaxation_ablation(&configuration), "runs": runs});
    fs::write(
        output_directory.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", output_directory.join("report.json").display());
    Ok(())
}
