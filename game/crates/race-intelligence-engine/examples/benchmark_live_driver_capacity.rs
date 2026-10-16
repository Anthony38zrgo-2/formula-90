use race_intelligence_engine::local_driver::autonomous_driver::{AutonomousDriver, DriverObservation};
use race_intelligence_engine::local_driver::driver_tuning::load_driver_tuning;
use race_intelligence_engine::track::path_reader::load_racing_line_from_document;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    let repository = std::path::Path::new(arguments.get(1).ok_or("Expected repository root")?);
    let tuning_document = std::fs::read_to_string(arguments.get(2).ok_or("Expected tuning JSON")?)?;
    let observation_document = std::fs::read_to_string(arguments.get(3).ok_or("Expected recorded observations")?)?;
    let output = arguments.get(4).ok_or("Expected output JSON")?;
    let driver_count: usize = arguments.get(5).ok_or("Expected driver count")?.parse()?;
    if driver_count == 0 { return Err("Driver count must be positive".into()); }
    let observations: Vec<DriverObservation> = observation_document.lines()
        .map(serde_json::from_str).collect::<Result<_, _>>()?;
    if observations.is_empty() { return Err("No recorded observations".into()); }
    let configuration = vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root(
        &repository.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"), repository)?;
    let line = load_racing_line_from_document(&std::fs::read_to_string(repository.join("game/tracks/fuji76_77/metadata/racing_line.json"))?)?;
    let mut reports = Vec::new();
    for mode in ["static_reference", "multirate", "all_at_control_frequency"] {
        let mut tuning = load_driver_tuning(&tuning_document)?;
        tuning.live_grip_control.enabled = mode != "static_reference";
        if mode == "all_at_control_frequency" {
            tuning.live_grip_control.capacity_estimation_hertz = tuning.live_grip_control.control_hertz;
            tuning.live_grip_control.local_planning_hertz = tuning.live_grip_control.control_hertz;
            tuning.live_grip_control.slow_state_hertz = tuning.live_grip_control.control_hertz;
        }
        let mut drivers = (0..driver_count).map(|_| AutonomousDriver::new(line.clone(), tuning, &configuration))
            .collect::<Result<Vec<_>, _>>()?;
        let mut batch_milliseconds = Vec::with_capacity(observations.len());
        let mut capacity = None;
        let mut slow = None;
        let mut final_diagnostics = None;
        for recorded in &observations {
            let mut observation = recorded.clone();
            if let Some(dynamics) = observation.vehicle_dynamics.as_mut() {
                capacity = dynamics.capacity_state.or(capacity);
                slow = dynamics.slow_state.or(slow);
                if mode == "all_at_control_frequency" {
                    dynamics.capacity_state = capacity;
                    dynamics.slow_state = slow;
                }
            }
            let start = Instant::now();
            for driver in &mut drivers {
                let command = driver.command(observation.clone())?;
                std::hint::black_box(command.throttle_fraction);
                final_diagnostics = Some(command.live_grip);
            }
            batch_milliseconds.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        batch_milliseconds.sort_by(f64::total_cmp);
        let total: f64 = batch_milliseconds.iter().sum();
        let percentile = |fraction: f64| batch_milliseconds[((batch_milliseconds.len() - 1) as f64 * fraction).round() as usize];
        reports.push(serde_json::json!({"mode": mode, "driver_count": driver_count,
            "control_batches": observations.len(), "mean_batch_milliseconds": total / observations.len() as f64,
            "median_batch_milliseconds": percentile(0.5), "percentile_95_batch_milliseconds": percentile(0.95),
            "percentile_99_batch_milliseconds": percentile(0.99), "maximum_batch_milliseconds": percentile(1.0),
            "total_controller_milliseconds": total, "final_diagnostics_per_driver": final_diagnostics}));
    }
    std::fs::write(output, serde_json::to_string_pretty(&serde_json::json!({
        "scope": "Recorded canonical Fuji observations replayed through independent controllers; excludes physics, rendering, transport and producer snapshot cost",
        "reports": reports
    }))?)?;
    Ok(())
}
