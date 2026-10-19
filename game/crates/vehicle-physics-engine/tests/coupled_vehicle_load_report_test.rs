use vehicle_physics_engine::coupled_vehicle_dynamics::{
    apply_generalized_wheel_braking, CoupledVehicleModel, CoupledVehicleState,
    GeneralizedVehicleVelocity,
};
use vehicle_physics_engine::coupled_vehicle_loads::{
    CoupledComponentLoad, CoupledLoadAccelerationSource, CoupledVehicleLoadReport,
};
use vehicle_physics_engine::suspension_coupled_solver::{
    CoupledSuspensionInput, CoupledWheelContact,
};
use vehicle_physics_engine::suspension_mass_properties::MassComponentAttachment;
use vehicle_physics_engine::{Vec3, VehicleConfig, WheelIndex};

fn actual_configuration() -> VehicleConfig {
    VehicleConfig::from_json_str(include_str!(
        "../../../data/vehicles/f1_2030/f1_2030_v10_geometric.json"
    ))
    .unwrap()
}

fn actual_model() -> CoupledVehicleModel {
    let configuration = actual_configuration();
    let settings = configuration.coupled_world.as_ref().unwrap().suspension.clone();
    CoupledVehicleModel::new(configuration, settings).unwrap()
}

fn tire_radius(model: &CoupledVehicleModel, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        model.configuration.tires.front_tire_radius
    } else {
        model.configuration.tires.rear_tire_radius
    }
}

fn contact_input(
    model: &CoupledVehicleModel,
    state: &CoupledVehicleState,
    penetration_metres: f64,
    front_left_tangential_force_world: Vec3,
) -> CoupledSuspensionInput {
    let base = CoupledSuspensionInput::default();
    let evaluation = model.evaluate(state, &base, [0.0; 4]).unwrap();
    let mut input = base.clone();
    for wheel in WheelIndex::ALL {
        let index = wheel as usize;
        let hub = evaluation
            .components
            .iter()
            .zip(model.inventory.components.iter())
            .find(|(_, definition)| definition.rotating_wheel == Some(wheel))
            .map(|(moving, _)| moving.center_of_mass_world_metres)
            .unwrap();
        input.contacts[index] = Some(CoupledWheelContact {
            surface_point_world_metres: hub
                - Vec3::UP * (tire_radius(model, wheel) - penetration_metres),
            surface_normal_world: Vec3::UP,
            surface_velocity_world_metres_per_second: Vec3::ZERO,
            tangential_force_world_newtons: if wheel == WheelIndex::FrontLeft {
                front_left_tangential_force_world
            } else {
                Vec3::ZERO
            },
        });
    }
    input
}

fn load_for(report: &CoupledVehicleLoadReport, wheel: WheelIndex) -> &CoupledComponentLoad {
    report
        .components
        .iter()
        .find(|component| {
            component.rotating_wheel == Some(wheel) && component.component_name.ends_with("_tire")
        })
        .or_else(|| {
            report
                .components
                .iter()
                .find(|component| component.rotating_wheel == Some(wheel))
        })
        .unwrap()
}

fn assert_action_reaction_closes(report: &CoupledVehicleLoadReport) {
    assert!(
        report.maximum_free_body_force_residual_newtons < 1e-9,
        "component force residual {}",
        report.maximum_free_body_force_residual_newtons
    );
    assert!(
        report.maximum_free_body_moment_residual_newton_metres < 1e-9,
        "component moment residual {}",
        report.maximum_free_body_moment_residual_newton_metres
    );
    let effective_scale = report
        .components
        .iter()
        .map(|component| component.effective_force_world_newtons.length())
        .fold(1.0_f64, f64::max);
    match report.acceleration_source {
        CoupledLoadAccelerationSource::EvaluatedInstantaneous => {
            assert!(
                report.action_reaction_force_residual_newtons < 1e-6,
                "instantaneous action reaction force residual {}",
                report.action_reaction_force_residual_newtons
            );
            let couple_scale = report
                .components
                .iter()
                .map(|component| component.attachment_couple_newton_metres)
                .fold(1.0_f64, f64::max);
            assert!(
                report.action_reaction_moment_residual_newton_metres < 1e-4 * couple_scale,
                "instantaneous action reaction moment residual {} against scale {}",
                report.action_reaction_moment_residual_newton_metres,
                couple_scale
            );
        }
        CoupledLoadAccelerationSource::SuppliedStepAverage => {
            assert!(
                report.action_reaction_force_residual_newtons < 1e-2 * effective_scale,
                "step average action reaction force residual {} against scale {}",
                report.action_reaction_force_residual_newtons,
                effective_scale
            );
            let couple_scale = report
                .components
                .iter()
                .map(|component| component.attachment_couple_newton_metres)
                .fold(1.0_f64, f64::max);
            assert!(
                report.action_reaction_moment_residual_newton_metres < 1e-2 * couple_scale,
                "step average action reaction moment residual {} against scale {}",
                report.action_reaction_moment_residual_newton_metres,
                couple_scale
            );
        }
    }
}

#[test]
fn static_contact_loads_close_the_action_reaction_balance() {
    let model = actual_model();
    let state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.30, 0.0));
    let input = contact_input(&model, &state, 0.012, Vec3::ZERO);
    let report = model.component_load_report(&state, &input, [0.0; 4]).unwrap();
    assert_eq!(
        report.acceleration_source,
        CoupledLoadAccelerationSource::EvaluatedInstantaneous
    );
    assert!(report.components.iter().all(|component| component.finite));
    assert_action_reaction_closes(&report);
    assert!(report.total_contact_force_world_newtons.y > 0.0);
    assert!(report
        .front
        .normal_force_newtons
        .iter()
        .all(|force| *force > 0.0));
    assert!(report
        .rear
        .normal_force_newtons
        .iter()
        .all(|force| *force > 0.0));
    let wheel = load_for(&report, WheelIndex::FrontLeft);
    assert!(wheel.contact_force_world_newtons.y > 0.0);
    assert!(wheel.attachment_force_world_newtons.y < 0.0);
    let rod = report
        .components
        .iter()
        .find(|component| {
            matches!(
                component.attachment,
                MassComponentAttachment::ActuationRod { .. }
                    | MassComponentAttachment::TrackRod { .. }
            )
        })
        .unwrap();
    assert!(rod.attachment_force_local_newtons.x.is_finite());
    assert!(rod.attachment_force_local_newtons.length() > 0.0);
    let damper = report
        .components
        .iter()
        .find(|component| component.component_name.contains("damper"))
        .unwrap();
    let spring_force = report.front.spring_force_newtons[0]
        .max(report.rear.spring_force_newtons[0]);
    assert!(
        (damper.attachment_force_world_newtons.length() - spring_force).abs() > 1.0,
        "the damper attachment load must stay distinct from the spring element force"
    );
}

#[test]
fn braking_reacts_through_the_wheel_loads_without_a_residual() {
    let model = actual_model();
    let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 10.0, 0.0));
    state.suspension.generalized_velocity[2] = -30.0;
    state.wheel_angular_velocity_radians_per_second = [80.0, 80.0, 80.0, 80.0];
    let input = CoupledSuspensionInput::default();
    let duration_seconds = 1.0 / 960.0;
    let before = state.velocity();
    let evaluation = model.evaluate(&state, &input, [0.0; 4]).unwrap();
    let instantaneous = evaluation.generalized_acceleration;
    let mut velocity = before;
    let braking = apply_generalized_wheel_braking(
        &evaluation.mass_matrix,
        &mut velocity,
        [1500.0; 4],
        duration_seconds,
        64,
        1e-9,
    )
    .unwrap();
    assert!(braking.dissipated_energy_joules > 0.0);
    assert!(velocity[10] < before[10]);
    let acceleration: GeneralizedVehicleVelocity = std::array::from_fn(|coordinate| {
        instantaneous[coordinate] + (velocity[coordinate] - before[coordinate]) / duration_seconds
    });
    let report = model
        .component_load_report_from_acceleration(&state, &input, [0.0; 4], acceleration)
        .unwrap();
    assert_eq!(
        report.acceleration_source,
        CoupledLoadAccelerationSource::SuppliedStepAverage
    );
    assert_action_reaction_closes(&report);
    let braked = load_for(&report, WheelIndex::FrontLeft);
    assert!(braked.attachment_moment_about_origin_world_newton_metres.length() > 1.0);
    assert!(report.mass_matrix_pivot_ratio_estimate >= 1.0);
}

#[test]
fn roll_moves_load_between_the_two_sides() {
    let model = actual_model();
    let state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.30, 0.0));
    let mut input = contact_input(&model, &state, 0.012, Vec3::ZERO);
    input.body_torque_about_origin_world_newton_metres = Vec3::new(0.0, 0.0, 1500.0);
    let report = model.component_load_report(&state, &input, [0.0; 4]).unwrap();
    assert_action_reaction_closes(&report);
    let left = load_for(&report, WheelIndex::FrontLeft);
    let right = load_for(&report, WheelIndex::FrontRight);
    assert!(
        (left.attachment_force_world_newtons.y - right.attachment_force_world_newtons.y).abs()
            > 1.0,
        "roll torque must transfer vertical attachment load between the sides"
    );
}

#[test]
fn one_wheel_bump_keeps_the_other_corner_contact_free() {
    let model = actual_model();
    let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.30, 0.0));
    let bump_travel = 0.03_f64
        .min(model.travel_limits_metres[WheelIndex::FrontLeft as usize].1 - 1e-4);
    state.suspension.wheel_travel_metres[WheelIndex::FrontLeft as usize] = bump_travel;
    state.suspension.generalized_velocity[6 + WheelIndex::FrontLeft as usize] = 0.20;
    let input = contact_input(&model, &state, 0.010, Vec3::new(500.0, 0.0, 0.0));
    let mut input = input;
    for wheel in [WheelIndex::FrontRight, WheelIndex::RearLeft, WheelIndex::RearRight] {
        input.contacts[wheel as usize] = None;
    }
    let report = model.component_load_report(&state, &input, [0.0; 4]).unwrap();
    assert_action_reaction_closes(&report);
    let bump = load_for(&report, WheelIndex::FrontLeft);
    let other = load_for(&report, WheelIndex::FrontRight);
    assert!(bump.contact_force_world_newtons.length() > 0.0);
    assert!((bump.contact_force_world_newtons.x - 500.0).abs() < 1e-9);
    assert_eq!(other.contact_force_world_newtons, Vec3::ZERO);
    assert!(
        (bump.attachment_force_world_newtons.y - other.attachment_force_world_newtons.y).abs()
            > 1.0
    );
}

#[test]
fn travel_stop_impact_recovers_an_event_scale_load() {
    let model = actual_model();
    let wheel = WheelIndex::FrontLeft;
    let upper = model.travel_limits_metres[wheel as usize].1;
    let mut state = CoupledVehicleState::at_rest(Vec3::new(0.0, 0.30, 0.0));
    state.suspension.wheel_travel_metres[wheel as usize] = upper - 1e-5;
    state.suspension.generalized_velocity[6 + wheel as usize] = 1.0;
    let input = CoupledSuspensionInput::default();
    let duration_seconds = 1.0 / 960.0;
    let before = state.velocity();
    model
        .advance(&mut state, &input, [0.0; 4], duration_seconds, 128)
        .unwrap();
    assert!(state.suspension.wheel_travel_metres[wheel as usize] <= upper + 1e-9);
    let after = state.velocity();
    assert!(after[6 + wheel as usize] < before[6 + wheel as usize]);
    let acceleration: GeneralizedVehicleVelocity =
        std::array::from_fn(|coordinate| (after[coordinate] - before[coordinate]) / duration_seconds);
    let report = model
        .component_load_report_from_acceleration(&state, &input, [0.0; 4], acceleration)
        .unwrap();
    assert_action_reaction_closes(&report);
    let impacted_corner_force: f64 = report
        .components
        .iter()
        .filter(|component| {
            matches!(
                component.attachment,
                MassComponentAttachment::Upright { wheel: attached }
                    | MassComponentAttachment::LowerWishbone { wheel: attached }
                    | MassComponentAttachment::UpperWishbone { wheel: attached }
                    | MassComponentAttachment::ActuationRod { wheel: attached }
                    | MassComponentAttachment::Rocker { wheel: attached }
                    | MassComponentAttachment::DamperCylinder { wheel: attached }
                    | MassComponentAttachment::DamperPiston { wheel: attached }
                    | MassComponentAttachment::WheelRotor { wheel: attached }
                    if attached == wheel
            )
        })
        .map(|component| component.attachment_force_world_newtons.length())
        .fold(0.0, f64::max);
    assert!(
        impacted_corner_force > 1000.0,
        "travel stop impact must appear in the corner load, got {impacted_corner_force}"
    );
    assert!(report.mass_matrix_pivot_ratio_estimate.is_finite());
    assert!(report.mass_matrix_pivot_ratio_estimate >= 1.0);
}
