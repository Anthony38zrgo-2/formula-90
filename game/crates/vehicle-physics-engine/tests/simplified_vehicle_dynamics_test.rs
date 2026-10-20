use std::path::Path;
use std::sync::Arc;
use vehicle_physics_engine::coupled_vehicle_dynamics::{
    CoupledVehicleModel, CoupledVehicleState, VEHICLE_GENERALIZED_VELOCITY_COUNT,
};
use vehicle_physics_engine::suspension_coupled_solver::CoupledSuspensionInput;
use vehicle_physics_engine::suspension_kinematics_table::SuspensionKinematicsTable;
use vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root;
use vehicle_physics_engine::vehicle_dynamics_fidelity::VehicleDynamicsFidelity;
use vehicle_physics_engine::{Vec3, VehicleConfig};

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

#[test]
fn simplified_model_matches_full_model_at_rest() {
    let configuration = configuration();
    let settings = configuration.coupled_world.as_ref().unwrap().suspension.clone();
    let table = Arc::new(SuspensionKinematicsTable::build(&configuration, 0.02).unwrap());
    let full = CoupledVehicleModel::new(configuration.clone(), settings.clone()).unwrap();
    let simplified = CoupledVehicleModel::new_with_fidelity(configuration, settings,
        VehicleDynamicsFidelity::Simplified, Some(table)).unwrap();
    let state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.35, 0.0));
    let input = CoupledSuspensionInput::default();
    let full_evaluation = full.evaluate(&state, &input, [0.0; 4]).unwrap();
    let simplified_evaluation = simplified.evaluate(&state, &input, [0.0; 4]).unwrap();
    let mut maximum_matrix_error = 0.0_f64;
    let mut maximum_matrix_scale = 1.0_f64;
    for row in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
        for column in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
            maximum_matrix_error = maximum_matrix_error.max(
                (full_evaluation.mass_matrix[row][column]
                    - simplified_evaluation.mass_matrix[row][column]).abs());
            maximum_matrix_scale = maximum_matrix_scale.max(
                full_evaluation.mass_matrix[row][column].abs());
        }
    }
    let mut maximum_force_error = 0.0_f64;
    let mut maximum_force_scale = 1.0_f64;
    for coordinate in 0..VEHICLE_GENERALIZED_VELOCITY_COUNT {
        maximum_force_error = maximum_force_error.max(
            (full_evaluation.generalized_force[coordinate]
                - simplified_evaluation.generalized_force[coordinate]).abs());
        maximum_force_scale = maximum_force_scale.max(
            full_evaluation.generalized_force[coordinate].abs());
    }
    assert!(maximum_matrix_error / maximum_matrix_scale <= 1e-4,
        "relative mass matrix error {}", maximum_matrix_error / maximum_matrix_scale);
    assert!(maximum_force_error / maximum_force_scale <= 1e-4,
        "relative generalized force error {}", maximum_force_error / maximum_force_scale);
}
