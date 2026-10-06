use std::path::Path;
use std::time::Instant;
use vehicle_physics_engine::coupled_vehicle_world::CoupledVehicleWorld;
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldPackage};
use vehicle_physics_engine::{Vec3, VehicleConfig, WheelIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if !(5..=6).contains(&arguments.len()) { return Err("Expected repository root, vehicle profile, physical package, track metadata, duration seconds and optional vehicle count".into()); }
    let configuration = VehicleConfig::from_json_path(Path::new(&arguments[1]))?;
    let package: PhysicalWorldPackage = serde_json::from_str(&std::fs::read_to_string(&arguments[2])?)?;
    let collision = PhysicalCollisionWorld::from_package(package, Path::new(&arguments[0]))?;
    let metadata: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&arguments[3])?)?;
    let spawn = &metadata["primary_spawn"];
    let coordinate = |index: usize| spawn["position_m"][index].as_f64().ok_or("Track spawn position is missing");
    let mut position = Vec3::new(coordinate(0)?, coordinate(1)?, coordinate(2)?);
    let yaw = spawn["yaw_radians"].as_f64().ok_or("Track spawn yaw is missing")?;
    let geometry = configuration.geometric_suspension.as_ref().ok_or("Diagnostic requires physical hardpoints")?;
    let origin_height = WheelIndex::ALL.iter().map(|wheel| {
        let radius = if wheel.is_front() { configuration.front_tire_radius } else { configuration.rear_tire_radius };
        radius - geometry.corners.get(*wheel).hub_center.y
    }).fold(0.0, f64::max);
    position.y += origin_height;
    let duration: f64 = arguments[4].parse()?;
    if !duration.is_finite() || duration <= 0.0 || duration > 120.0 { return Err("Diagnostic duration must be positive and at most 120 seconds".into()); }
    let settings = configuration.coupled_world.clone().ok_or("Profile does not select candidate world settings")?;
    let mut world = CoupledVehicleWorld::new(collision, settings)?;
    let vehicle_count: usize = arguments.get(5).map(|value| value.parse()).transpose()?.unwrap_or(1);
    if vehicle_count == 0 || vehicle_count > 64 { return Err("Diagnostic requires between one and 64 vehicles".into()); }
    let mut first_identifier = None;
    for vehicle_index in 0..vehicle_count {
        let vehicle_position = if vehicle_count > 1 {
            let entry = metadata["grid_spots"].get(vehicle_index).ok_or("Requested fleet exceeds the authored track grid")?;
            let coordinate = |index: usize| entry["pos"][index].as_f64().ok_or("Grid position is missing");
            Vec3::new(coordinate(0)?, coordinate(1)? + origin_height, coordinate(2)?)
        } else { position };
        let identifier = world.register_vehicle(configuration.clone(), vehicle_position, yaw)?;
        first_identifier.get_or_insert(identifier);
    }
    let started = Instant::now();
    while world.time_seconds + world.accumulated_host_time_seconds < duration - 1e-12 {
        let interval = (duration - world.time_seconds - world.accumulated_host_time_seconds).min(0.05);
        world.advance_host_interval(interval)?;
    }
    let vehicle = world.vehicles.get(&first_identifier.unwrap()).unwrap();
    let evaluation = vehicle.model.evaluate(&vehicle.state, &vehicle.last_force_input, [0.0; 4])?;
    println!("{}", serde_json::json!({"integration_owner": "rust_world_candidate", "simulated_time_seconds": world.time_seconds,
        "unconsumed_host_time_seconds": world.accumulated_host_time_seconds, "elapsed_wall_time_seconds": started.elapsed().as_secs_f64(),
        "spawn_origin_height_above_metadata_metres": origin_height, "state": vehicle.state,
        "operating_mass_kilograms": vehicle.model.configuration.total_vehicle_mass(), "suspension_diagnostics": evaluation.suspension_diagnostics,
        "kinetic_energy_joules": evaluation.kinetic_energy_joules,
        "vehicle_count": vehicle_count,
        "collision_event_count_last_interval": world.collision_events.len()}));
    Ok(())
}
