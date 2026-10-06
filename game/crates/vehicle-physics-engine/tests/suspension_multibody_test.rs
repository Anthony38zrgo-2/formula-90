use vehicle_physics_engine::suspension_component_kinematics::{
    moving_component, ComponentKinematicInput,
};
use vehicle_physics_engine::suspension_coupled_solver::{
    CoupledSuspensionInput, CoupledSuspensionModel, CoupledSuspensionSettings,
    CoupledSuspensionState, CoupledWheelContact,
};
use vehicle_physics_engine::suspension_mass_properties::{
    composite_mass_properties, InertiaTensor, MassComponentAttachment, MassPropertyOrigin,
    MassPropertyProvenance, SuspensionMassComponent, SuspensionMassInventory,
};
use vehicle_physics_engine::suspension_multibody::{
    CoupledSuspensionEquations, GeneralizedSuspensionVector, MovingSuspensionComponent,
    SUSPENSION_GENERALIZED_VELOCITY_COUNT,
};
use vehicle_physics_engine::{Mat3, Vec3, VehicleConfig, WheelIndex};

fn diagonal_inertia(first: f64, second: f64, third: f64) -> InertiaTensor {
    InertiaTensor {
        entries_kilogram_square_metres: [[first, 0.0, 0.0], [0.0, second, 0.0], [0.0, 0.0, third]],
    }
}

fn test_configuration() -> VehicleConfig {
    let mut configuration = VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap();
    configuration.fuel.current_kg = 0.0;
    configuration.fuel.initial_kg = 0.0;
    configuration
}

fn test_component(
    name: &str,
    mass: f64,
    attachment: MassComponentAttachment,
    center: Vec3,
    inertia: InertiaTensor,
) -> SuspensionMassComponent {
    SuspensionMassComponent {
        name: name.into(),
        mass_kilograms: mass,
        attachment,
        center_of_mass_in_attachment_metres: center,
        inertia_about_center_of_mass: inertia,
        provenance: MassPropertyProvenance {
            mass_origin: MassPropertyOrigin::Estimated,
            center_of_mass_origin: MassPropertyOrigin::Estimated,
            inertia_origin: MassPropertyOrigin::Estimated,
            source: "Synthetic analytic test fixture; not vehicle calibration".into(),
        },
    }
}

fn test_inventory(configuration: &VehicleConfig) -> SuspensionMassInventory {
    let mut components = vec![test_component(
        "body",
        600.0,
        MassComponentAttachment::Body,
        Vec3::ZERO,
        diagonal_inertia(300.0, 450.0, 250.0),
    )];
    for wheel in WheelIndex::ALL {
        let mass = if wheel.is_front() {
            configuration.front_wheel_mass
        } else {
            configuration.rear_wheel_mass
        };
        components.push(test_component(
            &format!("{wheel:?}"),
            mass,
            MassComponentAttachment::Upright { wheel },
            Vec3::ZERO,
            diagonal_inertia(1.0, 1.0, 1.0),
        ));
    }
    SuspensionMassInventory {
        schema_version: 1,
        declared_complete_dry_mass_kilograms: 698.0,
        components,
    }
}

fn test_settings() -> CoupledSuspensionSettings {
    CoupledSuspensionSettings {
        maximum_substep_seconds: 1.0 / 1920.0,
        differentiation_step_metres: 1e-4,
        gravity_world_metres_per_second_squared: Vec3::DOWN * 9.80665,
        tire_vertical_stiffness_newtons_per_metre: [180000.0; 4],
        tire_vertical_damping_newton_seconds_per_metre: [1200.0; 4],
        travel_stop_stiffness_newtons_per_cubic_metre: [1e8; 4],
        travel_stop_damping_newton_seconds_per_metre: [1000.0; 4],
        travel_stop_engagement_fraction: [0.8; 4],
    }
}

fn synthetic_components() -> Vec<MovingSuspensionComponent> {
    let mut components = Vec::new();
    let mut body_linear = [Vec3::ZERO; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    let mut body_angular = [Vec3::ZERO; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    for (axis, direction) in [Vec3::RIGHT, Vec3::UP, Vec3::BACK].into_iter().enumerate() {
        body_linear[axis] = direction;
        body_angular[3 + axis] = direction;
    }
    let body = MovingSuspensionComponent {
        mass_kilograms: 600.0,
        inertia_world: diagonal_inertia(300.0, 450.0, 250.0),
        center_of_mass_world_metres: Vec3::ZERO,
        linear_velocity_jacobian: body_linear,
        angular_velocity_jacobian: body_angular,
        prescribed_linear_velocity_metres_per_second: Vec3::ZERO,
        prescribed_angular_velocity_radians_per_second: Vec3::ZERO,
        linear_acceleration_bias_metres_per_second_squared: Vec3::ZERO,
        angular_acceleration_bias_radians_per_second_squared: Vec3::ZERO,
    };
    components.push(body.clone());
    for wheel in 0..4 {
        let mut component = body.clone();
        component.mass_kilograms = 25.0;
        component.inertia_world = diagonal_inertia(0.0, 0.0, 0.0);
        component.angular_velocity_jacobian = [Vec3::ZERO; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        component.linear_velocity_jacobian[6 + wheel] = Vec3::UP;
        components.push(component);
    }
    components
}

#[test]
fn coupled_matrix_matches_component_energy_and_is_symmetric() {
    let components = synthetic_components();
    let velocity: GeneralizedSuspensionVector =
        [1.1, -0.7, 2.3, 0.2, -0.3, 0.8, 0.5, -0.2, 0.9, -0.4];
    let equations =
        CoupledSuspensionEquations::assemble(&components, velocity, Vec3::ZERO).unwrap();
    let component_energy: f64 = components
        .iter()
        .map(|component| component.kinetic_energy_joules(velocity))
        .sum();
    assert!((equations.kinetic_energy_joules(velocity) - component_energy).abs() < 1e-10);
    assert_eq!(equations.mass_matrix[1][6], 25.0);
    assert_eq!(equations.mass_matrix[1][1], 700.0);
    for row in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
        for column in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            assert_eq!(
                equations.mass_matrix[row][column],
                equations.mass_matrix[column][row]
            );
        }
    }
}

#[test]
fn freefall_accelerates_whole_vehicle_without_relative_wheel_motion() {
    let equations = CoupledSuspensionEquations::assemble(
        &synthetic_components(),
        [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
        Vec3::DOWN * 9.80665,
    )
    .unwrap();
    let acceleration = equations
        .acceleration([0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT])
        .unwrap();
    assert!((acceleration[1] + 9.80665).abs() < 1e-12);
    for coordinate in
        (0..SUSPENSION_GENERALIZED_VELOCITY_COUNT).filter(|coordinate| *coordinate != 1)
    {
        assert!(acceleration[coordinate].abs() < 1e-12);
    }
}

#[test]
fn internal_spring_preserves_center_of_mass_acceleration() {
    let components = synthetic_components();
    let equations = CoupledSuspensionEquations::assemble(
        &components,
        [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
        Vec3::ZERO,
    )
    .unwrap();
    let mut applied = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    applied[6] = -1000.0;
    let acceleration = equations.acceleration(applied).unwrap();
    assert!(
        (600.0 * acceleration[1]
            + 25.0 * (4.0 * acceleration[1] + acceleration[6..].iter().sum::<f64>()))
        .abs()
            < 1e-9
    );
    assert!(acceleration[1] > 0.0);
    assert!(acceleration[6] < 0.0);
}

#[test]
fn host_collision_correction_preserves_unforced_wheel_absolute_velocity() {
    let equations = CoupledSuspensionEquations::assemble(
        &synthetic_components(),
        [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
        Vec3::ZERO,
    )
    .unwrap();
    let mut velocity = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    let collision_energy = equations
        .apply_body_velocity_change(&mut velocity, [0.0, 2.0, 0.0, 0.0, 0.0, 0.0])
        .unwrap();
    for coordinate in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
        assert!((velocity[1] + velocity[coordinate]).abs() < 1e-12);
    }
    assert!((collision_energy - 1200.0).abs() < 1e-9);
}

#[test]
fn rotated_composite_inertia_obeys_parallel_axis_theorem() {
    let inertia = diagonal_inertia(2.0, 3.0, 4.0);
    let rotated = inertia.rotated(Mat3::from_axis_angle(Vec3::UP, std::f64::consts::FRAC_PI_2));
    assert!((rotated.entries_kilogram_square_metres[0][0] - 4.0).abs() < 1e-12);
    let (mass, center, composite) =
        composite_mass_properties(&[(5.0, Vec3::LEFT, inertia), (5.0, Vec3::RIGHT, inertia)])
            .unwrap();
    assert_eq!(mass, 10.0);
    assert_eq!(center, Vec3::ZERO);
    assert_eq!(composite.entries_kilogram_square_metres[0][0], 4.0);
    assert_eq!(composite.entries_kilogram_square_metres[1][1], 16.0);
    assert_eq!(composite.entries_kilogram_square_metres[2][2], 18.0);
    assert!(diagonal_inertia(1.0, 1.0, 10.0).validate().is_err());
}

#[test]
fn inventory_rejects_duplicate_names_missing_corners_and_unclosed_mass() {
    let configuration = test_configuration();
    let mut inventory = test_inventory(&configuration);
    inventory.validate().unwrap();
    inventory.components[1].name = inventory.components[0].name.clone();
    assert!(inventory.validate().is_err());
    inventory = test_inventory(&configuration);
    inventory.components.pop();
    assert!(inventory.validate().is_err());
    inventory = test_inventory(&configuration);
    inventory.declared_complete_dry_mass_kilograms += 1.0;
    assert!(inventory.validate().is_err());
}

#[test]
fn actual_hardpoint_derivatives_are_stable_when_difference_step_is_halved() {
    let configuration = test_configuration();
    let inventory = test_inventory(&configuration);
    for component in &inventory.components {
        let input = ComponentKinematicInput {
            body_origin_world_metres: Vec3::ZERO,
            body_orientation_world: Mat3::IDENTITY,
            generalized_velocity: [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
            wheel_travel_metres: [0.01; 4],
            steering_rack_metres: 0.005,
            steering_rack_velocity_metres_per_second: 0.0,
            steering_rack_acceleration_metres_per_second_squared: 0.0,
            differentiation_step_metres: 1e-4,
        };
        let coarse = moving_component(&configuration, component, input).unwrap();
        let fine = moving_component(
            &configuration,
            component,
            ComponentKinematicInput {
                differentiation_step_metres: 5e-5,
                ..input
            },
        )
        .unwrap();
        for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            assert!(
                (coarse.linear_velocity_jacobian[coordinate]
                    - fine.linear_velocity_jacobian[coordinate])
                    .length()
                    < 1e-4,
                "{} linear",
                component.name
            );
            assert!(
                (coarse.angular_velocity_jacobian[coordinate]
                    - fine.angular_velocity_jacobian[coordinate])
                    .length()
                    < 1e-3,
                "{} angular",
                component.name
            );
        }
    }
}

#[test]
fn normal_contact_is_unilateral_and_uses_current_relative_hub_velocity() {
    let configuration = test_configuration();
    let model = CoupledSuspensionModel::new(
        configuration.clone(),
        test_inventory(&configuration),
        test_settings(),
    )
    .unwrap();
    let mut state = CoupledSuspensionState::at_rest(Vec3::UP * configuration.front_tire_radius);
    let mut input = CoupledSuspensionInput::default();
    let contact = CoupledWheelContact {
        surface_point_world_metres: Vec3::UP * 0.01,
        surface_normal_world: Vec3::UP,
        surface_velocity_world_metres_per_second: Vec3::ZERO,
        tangential_force_world_newtons: Vec3::ZERO,
    };
    input.contacts[0] = Some(contact);
    let rest = model.evaluate(&state, &input).unwrap();
    assert!(rest.diagnostics.normal_force_newtons[0] > 0.0);
    state.generalized_velocity[6] = 0.2;
    let unloading = model.evaluate(&state, &input).unwrap();
    assert!(
        unloading.diagnostics.normal_force_newtons[0] < rest.diagnostics.normal_force_newtons[0]
    );
    assert!(unloading.diagnostics.hub_velocity_world_metres_per_second[0].y > 0.19);
    state.generalized_velocity[6] = 10.0;
    assert_eq!(
        model
            .evaluate(&state, &input)
            .unwrap()
            .diagnostics
            .normal_force_newtons[0],
        0.0
    );
    state.body_origin_world_metres.y += 1.0;
    assert_eq!(
        model
            .evaluate(&state, &input)
            .unwrap()
            .diagnostics
            .normal_force_newtons[0],
        0.0
    );
}

#[test]
fn coupled_model_counts_fuel_once_and_rejects_incomplete_inputs() {
    let mut configuration = test_configuration();
    let inventory = test_inventory(&configuration);
    configuration.fuel.current_kg = 7.6;
    let model = CoupledSuspensionModel::new(configuration, inventory, test_settings()).unwrap();
    let evaluation = model
        .evaluate(
            &CoupledSuspensionState::at_rest(Vec3::ZERO),
            &CoupledSuspensionInput::default(),
        )
        .unwrap();
    let total_mass: f64 = evaluation
        .components
        .iter()
        .map(|component| component.mass_kilograms)
        .sum();
    assert!((total_mass - 705.6).abs() < 1e-10);
    let mut state = CoupledSuspensionState::at_rest(Vec3::ZERO);
    let original = state.clone();
    assert!(model
        .advance(&mut state, &CoupledSuspensionInput::default(), f64::NAN)
        .is_err());
    assert_eq!(
        state.body_origin_world_metres,
        original.body_origin_world_metres
    );
    assert_eq!(state.generalized_velocity, original.generalized_velocity);
}

fn symmetric_oscillator_force(
    position: GeneralizedSuspensionVector,
    velocity: GeneralizedSuspensionVector,
    damping: f64,
) -> GeneralizedSuspensionVector {
    let mut force = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    for wheel_coordinate in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
        let tire_force = -200000.0 * (position[1] + position[wheel_coordinate]);
        force[1] += tire_force;
        force[wheel_coordinate] = tire_force
            - 30000.0 * position[wheel_coordinate]
            - damping * velocity[wheel_coordinate];
    }
    force
}

fn symmetric_oscillator_energy(
    equations: &CoupledSuspensionEquations,
    position: GeneralizedSuspensionVector,
    velocity: GeneralizedSuspensionVector,
) -> f64 {
    equations.kinetic_energy_joules(velocity)
        + (6..SUSPENSION_GENERALIZED_VELOCITY_COUNT)
            .map(|coordinate| {
                0.5 * 200000.0 * (position[1] + position[coordinate]).powi(2)
                    + 0.5 * 30000.0 * position[coordinate].powi(2)
            })
            .sum::<f64>()
}

#[test]
fn analytic_heave_frequency_and_conservative_energy_match_coupled_matrix() {
    let equations = CoupledSuspensionEquations::assemble(
        &synthetic_components(),
        [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
        Vec3::ZERO,
    )
    .unwrap();
    let suspended_mass: f64 = 600.0;
    let unsuspended_mass = 100.0;
    let suspension_stiffness = 120000.0;
    let tire_stiffness = 800000.0;
    let characteristic_middle = suspension_stiffness * unsuspended_mass
        + suspended_mass * (suspension_stiffness + tire_stiffness);
    let squared_frequency = (characteristic_middle
        - (characteristic_middle * characteristic_middle
            - 4.0 * suspended_mass * unsuspended_mass * suspension_stiffness * tire_stiffness)
            .sqrt())
        / (2.0 * suspended_mass * unsuspended_mass);
    let expected_period = 2.0 * std::f64::consts::PI / squared_frequency.sqrt();
    let absolute_wheel_to_body_ratio =
        1.0 - suspended_mass * squared_frequency / suspension_stiffness;
    let mut position = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    position[1] = 0.01;
    for coordinate in 6..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
        position[coordinate] = 0.01 * (absolute_wheel_to_body_ratio - 1.0);
    }
    let mut velocity = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    let initial_energy = symmetric_oscillator_energy(&equations, position, velocity);
    let timestep = 1.0 / 1920.0;
    let mut crossings = Vec::new();
    let mut maximum_energy_error: f64 = 0.0;
    let steps = (expected_period * 3.0 / timestep).ceil() as usize;
    for step in 0..steps {
        let old_height = position[1];
        let acceleration = equations
            .acceleration(symmetric_oscillator_force(position, velocity, 0.0))
            .unwrap();
        for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            position[coordinate] +=
                velocity[coordinate] * timestep + 0.5 * acceleration[coordinate] * timestep.powi(2);
            velocity[coordinate] += 0.5 * acceleration[coordinate] * timestep;
        }
        let next_acceleration = equations
            .acceleration(symmetric_oscillator_force(position, velocity, 0.0))
            .unwrap();
        for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
            velocity[coordinate] += 0.5 * next_acceleration[coordinate] * timestep;
        }
        if old_height > 0.0 && position[1] <= 0.0 {
            crossings
                .push(step as f64 * timestep + timestep * old_height / (old_height - position[1]));
        }
        maximum_energy_error = maximum_energy_error.max(
            (symmetric_oscillator_energy(&equations, position, velocity) - initial_energy).abs()
                / initial_energy,
        );
    }
    assert!(crossings.len() >= 2);
    assert!(((crossings[1] - crossings[0]) / expected_period - 1.0).abs() < 1e-4);
    assert!(maximum_energy_error < 1e-4);
}

#[test]
fn nonlinear_reference_step_converges_and_accounts_for_damper_work() {
    let configuration = test_configuration();
    let inventory = test_inventory(&configuration);
    let mut settings = test_settings();
    settings.gravity_world_metres_per_second_squared = Vec3::ZERO;
    let coarse =
        CoupledSuspensionModel::new(configuration.clone(), inventory.clone(), settings.clone())
            .unwrap();
    settings.maximum_substep_seconds *= 0.5;
    let fine = CoupledSuspensionModel::new(configuration, inventory, settings).unwrap();
    let input = CoupledSuspensionInput::default();
    let mut coarse_state = CoupledSuspensionState::at_rest(Vec3::ZERO);
    let mut fine_state = coarse_state.clone();
    let initial = fine
        .evaluate(&fine_state, &input)
        .unwrap()
        .diagnostics
        .total_energy_joules;
    let coarse_result = coarse.advance(&mut coarse_state, &input, 0.01).unwrap();
    let fine_result = fine.advance(&mut fine_state, &input, 0.01).unwrap();
    assert!(
        (coarse_state.body_origin_world_metres - fine_state.body_origin_world_metres).length()
            < 1e-4
    );
    assert!(
        (fine_result.total_energy_joules + fine_state.dissipated_energy_joules - initial).abs()
            / initial
            < 0.015
    );
    assert!(fine_state.dissipated_energy_joules >= 0.0);
    assert!(coarse_result.dissipated_power_watts >= 0.0);
    assert!(
        fine_result
            .linear_momentum_world_kilogram_metres_per_second
            .length()
            < 0.05
    );
}

#[test]
fn all_link_attachments_have_finite_trajectories_and_positive_mass_matrix() {
    let configuration = test_configuration();
    let mut inventory = test_inventory(&configuration);
    let wheel = WheelIndex::RearLeft;
    for (name, attachment) in [
        (
            "lower_wishbone",
            MassComponentAttachment::LowerWishbone { wheel },
        ),
        (
            "upper_wishbone",
            MassComponentAttachment::UpperWishbone { wheel },
        ),
        ("track_rod", MassComponentAttachment::TrackRod { wheel }),
        (
            "actuation_rod",
            MassComponentAttachment::ActuationRod { wheel },
        ),
        ("rocker", MassComponentAttachment::Rocker { wheel }),
    ] {
        inventory.components.push(test_component(
            name,
            0.1,
            attachment,
            Vec3::new(0.01, 0.02, 0.01),
            diagonal_inertia(0.001, 0.001, 0.001),
        ));
        inventory.components[0].mass_kilograms -= 0.1;
    }
    let model = CoupledSuspensionModel::new(configuration, inventory, test_settings()).unwrap();
    let mut state = CoupledSuspensionState::at_rest(Vec3::ZERO);
    state.wheel_travel_metres = [0.005; 4];
    state.generalized_velocity[3] = 0.1;
    state.generalized_velocity[8] = 0.05;
    let evaluation = model
        .evaluate(&state, &CoupledSuspensionInput::default())
        .unwrap();
    assert!(evaluation
        .diagnostics
        .generalized_acceleration
        .iter()
        .all(|value| value.is_finite()));
}

#[test]
fn simultaneous_travel_impacts_preserve_total_momentum_and_dissipate_energy() {
    let equations = CoupledSuspensionEquations::assemble(
        &synthetic_components(),
        [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
        Vec3::ZERO,
    )
    .unwrap();
    let mut velocity = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    velocity[1] = -0.3;
    velocity[6] = 1.0;
    velocity[7] = 2.0;
    let momentum_before = 700.0 * velocity[1] + 25.0 * velocity[6..].iter().sum::<f64>();
    let dissipated = equations
        .project_travel_stop_velocities(&mut velocity, [true, true, false, false])
        .unwrap();
    let momentum_after = 700.0 * velocity[1] + 25.0 * velocity[6..].iter().sum::<f64>();
    assert!((momentum_after - momentum_before).abs() < 1e-10);
    assert!(velocity[6].abs() < 1e-12 && velocity[7].abs() < 1e-12);
    assert!(dissipated > 0.0);
    let effective_body_mass = equations.effective_body_mass_matrix().unwrap();
    assert!((effective_body_mass[1][1] - 600.0).abs() < 1e-10);
    assert!((equations.mass_matrix[1][1] - effective_body_mass[1][1]).abs() > 99.0);
}

#[test]
fn prescribed_steering_work_is_included_in_energy_balance() {
    let configuration = test_configuration();
    let mut settings = test_settings();
    settings.gravity_world_metres_per_second_squared = Vec3::ZERO;
    settings.maximum_substep_seconds = 1.0 / 3840.0;
    let model = CoupledSuspensionModel::new(
        configuration.clone(),
        test_inventory(&configuration),
        settings,
    )
    .unwrap();
    let mut input = CoupledSuspensionInput::default();
    input.steering_rack_velocity_metres_per_second = 0.01;
    let mut state = CoupledSuspensionState::at_rest(Vec3::ZERO);
    let initial = model
        .evaluate(&state, &input)
        .unwrap()
        .diagnostics
        .total_energy_joules;
    let final_diagnostics = model.advance(&mut state, &input, 0.01).unwrap();
    let energy_residual = final_diagnostics.total_energy_joules + state.dissipated_energy_joules
        - initial
        - state.steering_actuator_work_joules;
    assert!(state.steering_actuator_work_joules.is_finite());
    assert!(energy_residual.abs() / initial < 0.015);
}

#[test]
fn fuel_mass_exchange_changes_inertia_without_velocity_impulse() {
    let configuration = test_configuration();
    let mut model = CoupledSuspensionModel::new(
        configuration.clone(),
        test_inventory(&configuration),
        test_settings(),
    )
    .unwrap();
    let mut state = CoupledSuspensionState::at_rest(Vec3::UP);
    state.generalized_velocity[0] = 2.0;
    let input = CoupledSuspensionInput::default();
    let velocity_before = state.generalized_velocity;
    let mass_before = model
        .evaluate(&state, &input)
        .unwrap()
        .equations
        .mass_matrix[0][0];
    let exchange = model.update_fuel_mass(&state, &input, 7.6).unwrap();
    let mass_after = model
        .evaluate(&state, &input)
        .unwrap()
        .equations
        .mass_matrix[0][0];
    assert_eq!(state.generalized_velocity, velocity_before);
    assert!((mass_after - mass_before - 7.6).abs() < 1e-10);
    assert!(
        (exchange
            .system_linear_momentum_change_kilogram_metres_per_second
            .x
            - 15.2)
            .abs()
            < 1e-10
    );
    assert!(exchange.system_energy_change_joules > 0.0);
    assert!(model.update_fuel_mass(&state, &input, 111.0).is_err());
    assert_eq!(
        model
            .evaluate(&state, &input)
            .unwrap()
            .equations
            .mass_matrix[0][0],
        mass_after
    );
}

#[test]
fn component_geometry_cache_reuses_exact_solutions_without_changing_kinematics() {
    use vehicle_physics_engine::suspension_component_kinematics::{
        moving_component_with_geometry_cache, SuspensionGeometryEvaluationCache,
    };
    let configuration = test_configuration();
    let inventory = test_inventory(&configuration);
    let component = &inventory.components[3];
    let input = ComponentKinematicInput {
        body_origin_world_metres: Vec3::ZERO,
        body_orientation_world: Mat3::IDENTITY,
        generalized_velocity: [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
        wheel_travel_metres: [0.005; 4],
        steering_rack_metres: 0.0,
        steering_rack_velocity_metres_per_second: 0.0,
        steering_rack_acceleration_metres_per_second_squared: 0.0,
        differentiation_step_metres: 1e-4,
    };
    let expected = moving_component(&configuration, component, input).unwrap();
    let mut cache = SuspensionGeometryEvaluationCache::new(&configuration);
    let first = moving_component_with_geometry_cache(component, input, &mut cache).unwrap();
    let solution_count = cache.distinct_solution_count();
    let second = moving_component_with_geometry_cache(component, input, &mut cache).unwrap();
    assert_eq!(cache.distinct_solution_count(), solution_count);
    assert_eq!(
        first.linear_velocity_jacobian,
        expected.linear_velocity_jacobian
    );
    assert_eq!(
        second.angular_velocity_jacobian,
        expected.angular_velocity_jacobian
    );
    assert_eq!(
        second.linear_acceleration_bias_metres_per_second_squared,
        expected.linear_acceleration_bias_metres_per_second_squared
    );
}
