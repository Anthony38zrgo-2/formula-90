use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use vehicle_physics_engine::suspension_coupled_solver::{
    CoupledSuspensionDefinition, CoupledSuspensionDiagnostics, CoupledSuspensionInput,
    CoupledSuspensionState,
};
use vehicle_physics_engine::VehicleConfig;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceScenario {
    schema_version: u32,
    initial_state: CoupledSuspensionState,
    input: CoupledSuspensionInput,
    duration_seconds: f64,
    output_interval_seconds: f64,
}

#[derive(Serialize)]
struct ReferenceSample {
    state: CoupledSuspensionState,
    diagnostics: CoupledSuspensionDiagnostics,
}

#[derive(Serialize)]
struct ReferenceReport {
    vehicle_profile: String,
    mass_definition: String,
    scenario: String,
    integration_owner: String,
    elapsed_wall_time_seconds: f64,
    samples: Vec<ReferenceSample>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() != 3 {
        return Err("Expected vehicle profile, coupled mass definition and scenario paths".into());
    }
    let configuration = VehicleConfig::from_json_path(Path::new(&arguments[0]))?;
    let definition: CoupledSuspensionDefinition =
        serde_json::from_str(&std::fs::read_to_string(&arguments[1])?)?;
    let scenario: ReferenceScenario =
        serde_json::from_str(&std::fs::read_to_string(&arguments[2])?)?;
    if scenario.schema_version != 1
        || !scenario.duration_seconds.is_finite()
        || scenario.duration_seconds <= 0.0
        || !scenario.output_interval_seconds.is_finite()
        || scenario.output_interval_seconds <= 0.0
        || scenario.output_interval_seconds > 1.0
        || scenario.duration_seconds / scenario.output_interval_seconds > 100000.0
    {
        return Err("Invalid reference scenario duration or output cadence".into());
    }
    let model = definition.build_model(configuration)?;
    let mut state = scenario.initial_state;
    let initial_time = state.simulated_time_seconds;
    let initial_diagnostics = model.evaluate(&state, &scenario.input)?.diagnostics;
    let mut samples = vec![ReferenceSample {
        state: state.clone(),
        diagnostics: initial_diagnostics,
    }];
    let started = Instant::now();
    while state.simulated_time_seconds - initial_time < scenario.duration_seconds - 1e-12 {
        let elapsed = state.simulated_time_seconds - initial_time;
        let duration = scenario
            .output_interval_seconds
            .min(scenario.duration_seconds - elapsed);
        let mut input = scenario.input.clone();
        input.steering_rack_metres += scenario.input.steering_rack_velocity_metres_per_second
            * elapsed
            + 0.5
                * scenario
                    .input
                    .steering_rack_acceleration_metres_per_second_squared
                * elapsed.powi(2);
        input.steering_rack_velocity_metres_per_second += scenario
            .input
            .steering_rack_acceleration_metres_per_second_squared
            * elapsed;
        for contact in input.contacts.iter_mut().flatten() {
            contact.surface_point_world_metres +=
                contact.surface_velocity_world_metres_per_second * elapsed;
        }
        let diagnostics = model.advance(&mut state, &input, duration)?;
        samples.push(ReferenceSample {
            state: state.clone(),
            diagnostics,
        });
    }
    let report = ReferenceReport {
        vehicle_profile: arguments[0].clone(),
        mass_definition: arguments[1].clone(),
        scenario: arguments[2].clone(),
        integration_owner: "Rust reference solver; no Godot integration".into(),
        elapsed_wall_time_seconds: started.elapsed().as_secs_f64(),
        samples,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
