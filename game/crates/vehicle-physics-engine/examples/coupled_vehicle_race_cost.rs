use std::path::Path;
use std::time::Instant;
use vehicle_physics_engine::coupled_vehicle_world::CoupledVehicleWorld;
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldPackage};
use vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration;
use vehicle_physics_engine::{Vec3, WheelIndex};

fn percentile(sorted_values: &[f64], fraction: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let index = ((sorted_values.len() - 1) as f64 * fraction).ceil() as usize;
    sorted_values[index.min(sorted_values.len() - 1)]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if !(6..=8).contains(&arguments.len()) {
        return Err("Expected repository root, vehicle profile, physical package, track metadata, vehicle count, measured steps, optional warmup steps and optional mode".into());
    }
    let repository_root = Path::new(&arguments[0]);
    let configuration = load_vehicle_configuration(Path::new(&arguments[1]))?;
    let package: PhysicalWorldPackage = serde_json::from_str(&std::fs::read_to_string(&arguments[2])?)?;
    let collision = PhysicalCollisionWorld::from_package(package, repository_root)?;
    let metadata: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&arguments[3])?)?;
    let vehicle_count: usize = arguments[4].parse()?;
    let measured_steps: usize = arguments[5].parse()?;
    let warmup_steps: usize = arguments.get(6).map(|value| value.parse()).transpose()?.unwrap_or(5);
    let mode = arguments.get(7).map(String::as_str).unwrap_or("idle");
    if vehicle_count == 0 || vehicle_count > 64 {
        return Err("Vehicle count must be between one and 64".into());
    }
    if measured_steps == 0 || measured_steps > 10_000 {
        return Err("Measured steps must be between one and ten thousand".into());
    }
    if !matches!(mode, "idle" | "rolling" | "mixed" | "mixed-rolling") {
        return Err("Mode must be idle, rolling, mixed or mixed-rolling".into());
    }
    let mixed = matches!(mode, "mixed" | "mixed-rolling");
    let moving = matches!(mode, "rolling" | "mixed-rolling");
    let grid_spots = metadata["grid_spots"].as_array().ok_or("Track metadata has no grid spots")?;
    if grid_spots.len() < vehicle_count {
        return Err(format!("Requested fleet of {vehicle_count} exceeds the {} authored grid spots", grid_spots.len()).into());
    }
    let spawn = &metadata["primary_spawn"];
    let yaw = spawn["yaw_radians"].as_f64().ok_or("Track spawn yaw is missing")?;
    let geometry = configuration.suspension.geometric_suspension.as_ref().ok_or("Benchmark requires physical hardpoints")?;
    let origin_height = WheelIndex::ALL.iter().map(|wheel| {
        let radius = if wheel.is_front() { configuration.tires.front_tire_radius } else { configuration.tires.rear_tire_radius };
        radius - geometry.corners.get(*wheel).hub_center.y
    }).fold(0.0, f64::max);
    let settings = configuration.coupled_world.clone().ok_or("Profile does not select candidate world settings")?;
    let mut world = CoupledVehicleWorld::new(collision, settings)?;
    let mut identifiers = Vec::with_capacity(vehicle_count);
    let mut physical_frequencies = Vec::with_capacity(vehicle_count);
    for (grid_index, grid_spot) in grid_spots.iter().take(vehicle_count).enumerate() {
        let coordinate = |index: usize| grid_spot["pos"][index].as_f64().ok_or("Grid position is missing");
        let position = Vec3::new(coordinate(0)?, coordinate(1)? + origin_height, coordinate(2)?);
        let frequency = if mixed && grid_index > 0 { 60 } else { 120 };
        let identifier = world.register_vehicle_with_frequency(configuration.clone(), position, yaw, frequency)?;
        identifiers.push(identifier);
        physical_frequencies.push(frequency);
    }
    let step_seconds = 1.0 / configuration.coupled_world.as_ref().unwrap().physical_frequency_hertz as f64;
    for _ in 0..warmup_steps {
        world.advance_host_interval(step_seconds)?;
    }
    if moving {
        for identifier in &identifiers {
            world.vehicles.get_mut(identifier).unwrap().state.suspension.generalized_velocity[2] = -30.0;
        }
    }
    world.reset_step_profile();
    let mut step_milliseconds = Vec::with_capacity(measured_steps);
    let mut ai_phase_milliseconds = Vec::with_capacity(measured_steps / 2 + 1);
    for _ in 0..measured_steps {
        let ai_phase_count_before = world.step_profile.ai_phase_count;
        let started = Instant::now();
        world.advance_host_interval(step_seconds)?;
        step_milliseconds.push(started.elapsed().as_secs_f64() * 1000.0);
        if world.step_profile.ai_phase_count > ai_phase_count_before {
            ai_phase_milliseconds.push(world.step_profile.last_ai_phase_seconds * 1000.0);
        }
    }
    let mut sorted = step_milliseconds.clone();
    sorted.sort_by(|first, second| first.partial_cmp(second).unwrap_or(std::cmp::Ordering::Equal));
    let mean = step_milliseconds.iter().sum::<f64>() / step_milliseconds.len() as f64;
    let p95 = percentile(&sorted, 0.95);
    let interval_milliseconds = step_seconds * 1000.0;
    let mut ai_sorted = ai_phase_milliseconds.clone();
    ai_sorted.sort_by(|first, second| first.partial_cmp(second).unwrap_or(std::cmp::Ordering::Equal));
    let ai_mean = if ai_phase_milliseconds.is_empty() { 0.0 } else {
        ai_phase_milliseconds.iter().sum::<f64>() / ai_phase_milliseconds.len() as f64
    };
    let ai_p95 = percentile(&ai_sorted, 0.95);
    let ai_interval_milliseconds = 1000.0 / 60.0;
    let player_gate_passed = if vehicle_count <= 20 {
        mean / interval_milliseconds <= 0.30 && p95 / interval_milliseconds <= 0.50
    } else {
        p95 / interval_milliseconds <= 0.60
    };
    let ai_gate_passed = if mixed {
        let p95_fraction = if vehicle_count <= 20 { 0.50 } else { 0.60 };
        ai_mean / ai_interval_milliseconds <= 0.30 && ai_p95 / ai_interval_milliseconds <= p95_fraction
    } else {
        true
    };
    let build_source = std::fs::read_to_string(repository_root.join("game/BUILD_SOURCE"))
        .map(|document| document.trim().to_string()).unwrap_or_default();
    let result = serde_json::json!({
        "integration_owner": "rust_world_candidate",
        "qualification": "offline Rust world-step benchmark; excludes rendering, transport, audio and presentation",
        "mode": mode,
        "vehicle_count": vehicle_count,
        "authored_grid_spot_count": grid_spots.len(),
        "measured_steps": measured_steps,
        "warmup_steps": warmup_steps,
        "physical_frequency_hertz": configuration.coupled_world.as_ref().unwrap().physical_frequency_hertz,
        "physical_frequencies_hertz": physical_frequencies,
        "step_interval_milliseconds": interval_milliseconds,
        "simulated_time_seconds": world.time_seconds,
        "source_identifier": env!("COUPLED_WORLD_SOURCE_IDENTIFIER"),
        "build_source": build_source,
        "repository_root": repository_root,
        "vehicle_profile": &arguments[1],
        "physical_package": &arguments[2],
        "track_metadata": &arguments[3],
        "hardware": {
            "operating_system": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "available_parallelism": std::thread::available_parallelism().map(|value| value.get()).unwrap_or(0),
            "processor_identifier": std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default(),
            "number_of_processors": std::env::var("NUMBER_OF_PROCESSORS").unwrap_or_default()
        },
        "timing_milliseconds": {
            "minimum": sorted.first().copied().unwrap_or(0.0),
            "mean": mean,
            "p50": percentile(&sorted, 0.50),
            "p95": p95,
            "p99": percentile(&sorted, 0.99),
            "maximum": sorted.last().copied().unwrap_or(0.0),
            "mean_ratio_to_interval": mean / interval_milliseconds,
            "p95_ratio_to_interval": p95 / interval_milliseconds,
            "p99_ratio_to_interval": percentile(&sorted, 0.99) / interval_milliseconds
        },
        "mixed_frequency": {
            "enabled": mixed,
            "player_frequency_hertz": if mixed { 120 } else { configuration.coupled_world.as_ref().unwrap().physical_frequency_hertz },
            "ai_frequency_hertz": if mixed { 60 } else { configuration.coupled_world.as_ref().unwrap().physical_frequency_hertz },
            "ai_update_ratio": if mixed { 2 } else { 1 },
            "ai_phase_timing_milliseconds": {
                "sample_count": ai_phase_milliseconds.len(),
                "mean": ai_mean,
                "p95": ai_p95,
                "interval": ai_interval_milliseconds,
                "mean_ratio_to_ai_interval": ai_mean / ai_interval_milliseconds,
                "p95_ratio_to_ai_interval": ai_p95 / ai_interval_milliseconds
            }
        },
        "step_profile": world.step_profile,
        "provisional_gate": {
            "twenty_car_mean_interval_fraction": 0.30,
            "twenty_car_p95_interval_fraction": 0.50,
            "fifty_car_p95_interval_fraction": 0.60,
            "player_frame_passed": player_gate_passed,
            "ai_batch_passed": ai_gate_passed,
            "passed": player_gate_passed && ai_gate_passed,
            "approval_state": "provisional_pending_human_approval"
        },
        "vehicle_identifiers": identifiers
    });
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
