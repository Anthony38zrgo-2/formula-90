use std::hint::black_box;
use std::path::Path;
use std::time::Instant;
use vehicle_physics_engine::coupled_vehicle_dynamics::{CoupledVehicleModel, CoupledVehicleState};
use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldPackage};
use vehicle_physics_engine::suspension_component_kinematics::{moving_inventory, wheel_contact_component};
use vehicle_physics_engine::suspension_coupled_solver::CoupledSuspensionInput;
use vehicle_physics_engine::{Vec3, VehicleConfig, WheelIndex};

fn measure_stage<T>(repetitions: usize, mut evaluate: impl FnMut(usize) -> Result<T, String>) -> Result<f64, String> {
    let started = Instant::now();
    for repetition in 0..repetitions { black_box(evaluate(repetition)?); }
    Ok(started.elapsed().as_secs_f64() / repetitions as f64)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.len() != 3 { return Err("Expected repository root, vehicle profile and physical package".into()); }
    let configuration = VehicleConfig::from_json_path(Path::new(&arguments[1]))?;
    let settings = configuration.coupled_world.as_ref().ok_or("Missing physical world settings")?.suspension.clone();
    let model = CoupledVehicleModel::new(configuration, settings)?;
    let package: PhysicalWorldPackage = serde_json::from_str(&std::fs::read_to_string(&arguments[2])?)?;
    let collision_world = PhysicalCollisionWorld::from_package(package, Path::new(&arguments[0]))?;
    let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.4, 0.0));
    let input = CoupledSuspensionInput::default();
    let repetitions = 200;
    let component_trajectory_seconds = measure_stage(repetitions, |repetition| {
        state.suspension.wheel_travel_metres = [0.005 * (repetition as f64 * 0.1).sin(); 4];
        moving_inventory(&model.configuration, &model.inventory.reference_inventory(), model.kinematic_input(&state, &input))
    })?;
    let coupled_matrix_seconds = measure_stage(repetitions, |repetition| {
        state.suspension.wheel_travel_metres = [0.005 * (repetition as f64 * 0.1).sin(); 4];
        model.evaluate(&state, &input, [0.0; 4])
    })?;
    let wheel_contact_trajectory_seconds = measure_stage(repetitions, |_| {
        WheelIndex::ALL.iter().map(|wheel| wheel_contact_component(&model.configuration, *wheel,
            model.kinematic_input(&state, &input))).collect::<Result<Vec<_>, _>>()
    })?;
    let road_query_seconds = measure_stage(repetitions, |_| {
        collision_world.raycast_driveable(Vec3::new(0.0, 3.0, 0.0), -Vec3::UP, 10.0)
    })?;
    println!("{}", serde_json::json!({"repetitions": repetitions,
        "component_count": model.inventory.components.len(),
        "component_trajectory_seconds_per_evaluation": component_trajectory_seconds,
        "coupled_matrix_seconds_per_evaluation": coupled_matrix_seconds,
        "four_wheel_trajectory_seconds_per_evaluation": wheel_contact_trajectory_seconds,
        "road_query_seconds_per_evaluation": road_query_seconds,
        "qualification": "isolated kernel timing; does not include shared-world collision handling or prove race performance"}));
    Ok(())
}
