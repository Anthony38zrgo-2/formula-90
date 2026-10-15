use parry3d_f64::na::DMatrix;
use vehicle_physics_engine::coupled_vehicle_dynamics::{CoupledVehicleModel, CoupledVehicleState, integrate_implicit_suspension_damping, generalized_vehicle_energy};
use vehicle_physics_engine::suspension_coupled_solver::{CoupledSuspensionInput, CoupledWheelContact};
use vehicle_physics_engine::{Vec3, VehicleConfig};

fn configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!("../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json")).unwrap()
}

fn flat_contacts() -> CoupledSuspensionInput {
    CoupledSuspensionInput {
        contacts: [Some(CoupledWheelContact {
            surface_point_world_metres: Vec3::ZERO,
            surface_normal_world: Vec3::new(0.0, 1.0, 0.0),
            surface_velocity_world_metres_per_second: Vec3::ZERO,
            tangential_force_world_newtons: Vec3::ZERO,
        }); 4],
        ..CoupledSuspensionInput::default()
    }
}

#[test]
fn stiff_damper_is_passive_and_matches_the_implicit_analytical_solution() {
    use vehicle_physics_engine::suspension_damping::{SuspensionDampingElement, SuspensionDampingLaw};
    let configuration = configuration();
    let model = CoupledVehicleModel::new(configuration.clone(), configuration.coupled_world.as_ref().unwrap().suspension.clone()).unwrap();
    let mut evaluation = model.evaluate(&CoupledVehicleState::at_rest(Vec3::new(0.0, 0.3, 0.0)), &flat_contacts(), [0.0; 4]).unwrap();
    evaluation.mass_matrix = [[0.0; 14]; 14];
    for coordinate in 0..14 { evaluation.mass_matrix[coordinate][coordinate] = 20.0; }
    let mut derivative = [0.0; 10];
    derivative[6] = 1.0;
    evaluation.damping_elements = vec![SuspensionDampingElement { velocity_derivative: derivative, prescribed_velocity: 0.0,
        law: SuspensionDampingLaw::Digressive {compression: 10000.0, rebound: 10000.0, knee: 1.0, fast_fraction: 1.0} }];
    for initial_speed in [-0.1, 0.1] {
        let mut velocity = [0.0; 14];
        velocity[6] = initial_speed;
        evaluation.generalized_force = [0.0; 14];
        evaluation.generalized_force[6] = -10000.0 * initial_speed;
        let result = integrate_implicit_suspension_damping(&evaluation, velocity, 1.0 / 120.0).unwrap();
        let damping_ratio = 0.5 * 10000.0 / 20.0 / 120.0;
        assert!((result.0[6] - initial_speed * (1.0 - damping_ratio) / (1.0 + damping_ratio)).abs() < 1e-12);
        assert!(result.0[6].abs() < initial_speed.abs());
        assert!((generalized_vehicle_energy(&evaluation.mass_matrix, velocity)
            - generalized_vehicle_energy(&evaluation.mass_matrix, result.0) - result.1).abs() < 1e-12);
    }
}

#[test]
fn unforced_roll_and_wheel_oscillations_decay_at_one_hundred_twenty_hertz() {
    let configuration = configuration();
    let model = CoupledVehicleModel::new(configuration.clone(), configuration.coupled_world.as_ref().unwrap().suspension.clone()).unwrap();
    let input = flat_contacts();
    let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.30, 0.0));
    state.suspension.generalized_velocity[5] = 0.05;
    let initial_energy = model.evaluate(&state, &input, [0.0; 4]).unwrap().suspension_diagnostics.total_energy_joules;
    let mut final_peak_velocity = 0.0_f64;
    for tick in 0..360 {
        model.advance(&mut state, &input, [0.0; 4], 1.0 / 120.0, 128).unwrap();
        if tick >= 240 {
            final_peak_velocity = state.suspension.generalized_velocity[3..10].iter().fold(final_peak_velocity, |maximum, velocity| maximum.max(velocity.abs()));
        }
    }
    assert!(final_peak_velocity < 0.001, "Unforced suspension retained velocity {final_peak_velocity}");
    let final_energy = model.evaluate(&state, &input, [0.0; 4]).unwrap().suspension_diagnostics.total_energy_joules;
    let final_residual = final_energy + state.suspension.dissipated_energy_joules - initial_energy;
    println!("FINAL_SETTLING_ENERGY_RESIDUAL {final_residual}");
    assert!(final_residual.abs() < 1.0, "Unforced suspension energy residual {final_residual}");
}

#[test]
fn simultaneous_travel_stops_do_not_reintroduce_unforced_oscillations() {
    let configuration = configuration();
    let model = CoupledVehicleModel::new(configuration.clone(), configuration.coupled_world.as_ref().unwrap().suspension.clone()).unwrap();
    for direction in [-1.0_f64, 1.0] {
        let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.30, 0.0));
        for wheel in 0..4 {
            state.suspension.wheel_travel_metres[wheel] = if direction > 0.0 { model.travel_limits_metres[wheel].1 } else { model.travel_limits_metres[wheel].0 };
            state.suspension.generalized_velocity[6 + wheel] = direction * 0.5;
        }
        for _ in 0..360 { model.advance(&mut state, &flat_contacts(), [0.0; 4], 1.0 / 120.0, 128).unwrap(); }
        assert!(state.suspension.generalized_velocity[3..10].iter().all(|velocity| velocity.abs() < 0.001));
    }
}

#[test]
fn repeated_roll_excitation_decays_after_the_external_torque_stops() {
    let configuration = configuration();
    let model = CoupledVehicleModel::new(configuration.clone(), configuration.coupled_world.as_ref().unwrap().suspension.clone()).unwrap();
    let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.30, 0.0));
    let mut input = flat_contacts();
    let mut final_peak_velocity = 0.0_f64;
    for tick in 0..720 {
        let time = tick as f64 / 120.0;
        input.body_torque_about_origin_world_newton_metres.z = if time < 3.0 { 800.0 * (time * std::f64::consts::TAU * 2.0).sin() } else { 0.0 };
        model.advance(&mut state, &input, [0.0; 4], 1.0 / 120.0, 128).unwrap();
        if tick >= 600 {
            final_peak_velocity = state.suspension.generalized_velocity[3..10].iter().fold(final_peak_velocity, |maximum, velocity| maximum.max(velocity.abs()));
        }
    }
    assert!(final_peak_velocity < 0.001, "Repeated forcing retained velocity {final_peak_velocity}");
}

#[test]
fn suspension_reference_integrator_also_settles_at_one_hundred_twenty_hertz() {
    use vehicle_physics_engine::suspension_coupled_solver::{CoupledSuspensionModel, CoupledSuspensionState};
    let configuration = configuration();
    let model = CoupledSuspensionModel::new(configuration.clone(), configuration.chassis.physical_mass_inventory.as_ref().unwrap().reference_inventory(),
        configuration.coupled_world.as_ref().unwrap().suspension.clone()).unwrap();
    let mut state = CoupledSuspensionState::at_rest(Vec3::new(0.0, 0.30, 0.0));
    state.generalized_velocity[5] = 0.05;
    let mut final_peak_velocity = 0.0_f64;
    for tick in 0..360 {
        model.advance(&mut state, &flat_contacts(), 1.0 / 120.0).unwrap();
        if tick >= 240 {
            final_peak_velocity = state.generalized_velocity[3..10].iter().fold(final_peak_velocity, |maximum, velocity| maximum.max(velocity.abs()));
        }
    }
    assert!(final_peak_velocity < 0.001, "Reference suspension retained velocity {final_peak_velocity}");
}

#[test]
#[ignore]
fn replay_recorded_fuji_inputs_compares_physical_cadences_without_driver_feedback() {
    use vehicle_physics_engine::coupled_vehicle_world::{CoupledVehicleWorld, CoupledWorldSnapshot, TimestampedVehicleInput};
    use vehicle_physics_engine::physical_collision_world::{PhysicalCollisionWorld, PhysicalWorldPackage};
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let document = std::fs::read_to_string(repository.join("scratch/autonomous_driver_fuji_integrator_corrected_20261015/physical_diagnostics.jsonl")).unwrap();
    let snapshots: Vec<CoupledWorldSnapshot> = document.lines().map(|line| {
        let document: serde_json::Value = serde_json::from_str(line).unwrap();
        serde_json::from_value(document["snapshot"].clone()).unwrap()
    }).collect();
    let package: PhysicalWorldPackage = serde_json::from_str(&std::fs::read_to_string(repository.join("scratch/coupled_vehicle_complete_implementation/fuji_physical_world.json")).unwrap()).unwrap();
    let terrain = PhysicalCollisionWorld::from_package(package, &repository).unwrap();
    for frequency in [120, 240, 480] {
        let mut configuration = configuration();
        let tuning: serde_json::Value = serde_json::from_str(include_str!("../../../data/race_intelligence/automatic_transmission_tuning.json")).unwrap();
        configuration.powertrain.automatic_transmission = true;
        configuration.powertrain.automatic_shift = serde_json::from_value(tuning["artificial_driver_transmission"]["automatic_shift"].clone()).unwrap();
        configuration.coupled_world.as_mut().unwrap().physical_frequency_hertz = frequency;
        configuration.coupled_world.as_mut().unwrap().suspension.maximum_substep_seconds = 1.0 / frequency as f64;
        let mut world = CoupledVehicleWorld::new(terrain.clone(), configuration.coupled_world.clone().unwrap()).unwrap();
        let initial = &snapshots[0].state.suspension;
        let forward = initial.body_orientation_world.to_mat3().transform_vector(Vec3::FORWARD);
        let identifier = world.register_vehicle(configuration, initial.body_origin_world_metres, (-forward.x).atan2(-forward.z)).unwrap();
        let mut failure = None;
        let mut maximum_tilt = 0.0_f64;
        for snapshot in snapshots.iter().skip(1) {
            world.enqueue_input(identifier, TimestampedVehicleInput {time_seconds: world.time_seconds,
                input: snapshot.driver_input, driving_aids_mask: snapshot.driving_aids_mask}).unwrap();
            if let Err(error) = world.advance_host_interval(1.0 / 60.0) { failure = Some(error); break; }
            let orientation = world.vehicles[&identifier].state.suspension.body_orientation_world;
            let tilt = (1.0 - 2.0 * (orientation.x.powi(2) + orientation.z.powi(2))).clamp(-1.0, 1.0).acos().to_degrees();
            maximum_tilt = maximum_tilt.max(tilt);
        }
        println!("FUJI_REPLAY {}", serde_json::json!({"frequency": frequency, "seconds": world.time_seconds,
            "maximum_tilt_degrees": maximum_tilt, "failure": failure}));
    }
}

#[test]
#[ignore]
fn audit_unforced_suspension_energy_and_damping_stability() {
    let configuration = configuration();
    let settings = configuration.coupled_world.as_ref().unwrap().suspension.clone();
    let model = CoupledVehicleModel::new(configuration.clone(), settings).unwrap();
    let input = flat_contacts();
    let reference = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.30, 0.0));
    let reference_acceleration = model.evaluate(&reference, &input, [0.0; 4]).unwrap().suspension_diagnostics.generalized_acceleration;
    let mut damping_acceleration = DMatrix::zeros(10, 10);
    for coordinate in 0..10 {
        let mut perturbed = reference.clone();
        perturbed.suspension.generalized_velocity[coordinate] = 0.00001;
        let acceleration = model.evaluate(&perturbed, &input, [0.0; 4]).unwrap().suspension_diagnostics.generalized_acceleration;
        for row in 0..10 {
            damping_acceleration[(row, coordinate)] = -(acceleration[row] - reference_acceleration[row]) / 0.00001;
        }
    }
    let eigenvalues = damping_acceleration.complex_eigenvalues();
    println!("DAMPING_MODES {}", serde_json::to_string(&eigenvalues.iter().map(|value| [value.re, value.im]).collect::<Vec<_>>()).unwrap());
    for frequency in [120, 240, 480, 960] {
        let mut settings = configuration.coupled_world.as_ref().unwrap().suspension.clone();
        settings.maximum_substep_seconds = 1.0 / frequency as f64;
        let model = CoupledVehicleModel::new(configuration.clone(), settings).unwrap();
        let mut state = reference.clone();
        state.suspension.generalized_velocity[5] = 0.05;
        let initial_energy = model.evaluate(&state, &input, [0.0; 4]).unwrap().suspension_diagnostics.total_energy_joules;
        let mut final_roll_speed = 0.0_f64;
        let mut final_travel_speed = 0.0_f64;
        let mut maximum_energy_residual = 0.0_f64;
        let mut maximum_absolute_energy_residual = 0.0_f64;
        let mut failure = None;
        for tick in 0..frequency * 3 {
            if let Err(error) = model.advance(&mut state, &input, [0.0; 4], 1.0 / frequency as f64, 128) {
                failure = Some(error);
                break;
            }
            let evaluation = model.evaluate(&state, &input, [0.0; 4]).unwrap();
            let residual = evaluation.suspension_diagnostics.total_energy_joules + state.suspension.dissipated_energy_joules
                - state.suspension.external_work_joules - state.suspension.steering_actuator_work_joules - initial_energy;
            maximum_energy_residual = maximum_energy_residual.max(residual);
            maximum_absolute_energy_residual = maximum_absolute_energy_residual.max(residual.abs());
            if tick >= frequency * 2 {
                final_roll_speed = final_roll_speed.max(state.suspension.generalized_velocity[5].abs());
                final_travel_speed = state.suspension.generalized_velocity[6..10].iter().fold(final_travel_speed, |maximum, velocity| maximum.max(velocity.abs()));
            }
        }
        println!("UNFORCED_SUSPENSION {}", serde_json::json!({"frequency": frequency, "final_roll_speed": final_roll_speed,
            "final_travel_speed": final_travel_speed, "maximum_energy_residual_joules": maximum_energy_residual,
            "maximum_absolute_energy_residual_joules": maximum_absolute_energy_residual,
            "dissipated_energy_joules": state.suspension.dissipated_energy_joules, "failure": failure}));
    }
}
