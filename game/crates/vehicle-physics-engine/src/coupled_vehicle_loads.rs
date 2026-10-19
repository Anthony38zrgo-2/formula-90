use crate::coupled_vehicle_dynamics::{
    factor_generalized_vehicle_matrix, CoupledVehicleModel, CoupledVehicleState,
    GeneralizedVehicleMatrix, GeneralizedVehicleVelocity,
};
use crate::suspension_coupled_solver::CoupledSuspensionInput;
use crate::suspension_mass_properties::MassComponentAttachment;
use crate::types::{Vec3, WheelIndex};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoupledLoadAccelerationSource {
    EvaluatedInstantaneous,
    SuppliedStepAverage,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoupledComponentLoad {
    pub component_name: String,
    pub component_perimeter: String,
    pub attachment: MassComponentAttachment,
    pub rotating_wheel: Option<WheelIndex>,
    pub mass_kilograms: f64,
    pub center_of_mass_world_metres: Vec3,
    pub attachment_origin_world_metres: Vec3,
    pub linear_acceleration_world_metres_per_second_squared: Vec3,
    pub angular_acceleration_world_radians_per_second_squared: Vec3,
    pub effective_force_world_newtons: Vec3,
    pub contact_force_world_newtons: Vec3,
    pub contact_moment_about_center_world_newton_metres: Vec3,
    pub attachment_force_world_newtons: Vec3,
    pub attachment_force_local_newtons: Vec3,
    pub attachment_moment_about_origin_world_newton_metres: Vec3,
    pub attachment_moment_local_newton_metres: Vec3,
    pub attachment_couple_newton_metres: f64,
    pub free_body_force_residual_newtons: f64,
    pub free_body_moment_residual_newton_metres: f64,
    pub finite: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoupledAxleLoadSummary {
    pub axle_label: String,
    pub rotating_component_count: usize,
    pub suspension_component_count: usize,
    pub rotating_attachment_force_world_newtons: Vec3,
    pub rotating_vertical_attachment_force_newtons: f64,
    pub maximum_rotating_attachment_couple_newton_metres: f64,
    pub spring_force_newtons: [f64; 2],
    pub damper_force_newtons: [f64; 2],
    pub normal_force_newtons: [f64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct CoupledVehicleLoadReport {
    pub acceleration_source: CoupledLoadAccelerationSource,
    pub generalized_acceleration: GeneralizedVehicleVelocity,
    pub components: Vec<CoupledComponentLoad>,
    pub body_applied_force_world_newtons: Vec3,
    pub body_applied_torque_about_origin_world_newton_metres: Vec3,
    pub total_attachment_force_world_newtons: Vec3,
    pub total_contact_force_world_newtons: Vec3,
    pub action_reaction_force_residual_newtons: f64,
    pub total_attachment_moment_about_origin_world_newton_metres: Vec3,
    pub action_reaction_moment_residual_newton_metres: f64,
    pub maximum_free_body_force_residual_newtons: f64,
    pub maximum_free_body_moment_residual_newton_metres: f64,
    pub mass_matrix_pivot_ratio_estimate: f64,
    pub front: CoupledAxleLoadSummary,
    pub rear: CoupledAxleLoadSummary,
}

impl CoupledVehicleModel {
    pub fn component_load_report(
        &self,
        state: &CoupledVehicleState,
        input: &CoupledSuspensionInput,
        actuator_torque_newton_metres: [f64; 4],
    ) -> Result<CoupledVehicleLoadReport, String> {
        let evaluation = self.evaluate(state, input, actuator_torque_newton_metres)?;
        let acceleration = evaluation.generalized_acceleration;
        self.assemble_component_load_report(
            state,
            input,
            &evaluation,
            acceleration,
            CoupledLoadAccelerationSource::EvaluatedInstantaneous,
        )
    }

    pub fn component_load_report_from_acceleration(
        &self,
        state: &CoupledVehicleState,
        input: &CoupledSuspensionInput,
        actuator_torque_newton_metres: [f64; 4],
        generalized_acceleration: GeneralizedVehicleVelocity,
    ) -> Result<CoupledVehicleLoadReport, String> {
        if generalized_acceleration.iter().any(|value| !value.is_finite()) {
            return Err("Load recovery requires a finite generalized acceleration".into());
        }
        let evaluation = self.evaluate(state, input, actuator_torque_newton_metres)?;
        self.assemble_component_load_report(
            state,
            input,
            &evaluation,
            generalized_acceleration,
            CoupledLoadAccelerationSource::SuppliedStepAverage,
        )
    }

    fn assemble_component_load_report(
        &self,
        state: &CoupledVehicleState,
        input: &CoupledSuspensionInput,
        evaluation: &crate::coupled_vehicle_dynamics::CoupledVehicleEvaluation,
        generalized_acceleration: GeneralizedVehicleVelocity,
        acceleration_source: CoupledLoadAccelerationSource,
    ) -> Result<CoupledVehicleLoadReport, String> {
        let velocity = state.velocity();
        let suspension_velocity: [f64; 10] = std::array::from_fn(|index| velocity[index]);
        let gravity = self.settings.gravity_world_metres_per_second_squared;
        let mut wheel_contact_carrier: [Option<usize>; 4] = [None; 4];
        for (index, definition) in self.inventory.components.iter().enumerate() {
            if let Some(wheel) = definition.rotating_wheel {
                let slot = &mut wheel_contact_carrier[wheel as usize];
                let carries_tire_name = definition.properties.name.ends_with("_tire");
                match slot {
                    None => *slot = Some(index),
                    Some(current) if carries_tire_name
                        && !self.inventory.components[*current]
                            .properties
                            .name
                            .ends_with("_tire") =>
                    {
                        *slot = Some(index)
                    }
                    _ => {}
                }
            }
        }
        let mut components = Vec::with_capacity(evaluation.components.len());
        let mut total_attachment_force = Vec3::ZERO;
        let mut total_contact_force = Vec3::ZERO;
        let mut total_attachment_moment = Vec3::ZERO;
        let mut maximum_free_body_residual: f64 = 0.0;
        let mut maximum_free_body_moment_residual: f64 = 0.0;
        for (index, moving) in evaluation.components.iter().enumerate() {
            let (name, perimeter, attachment, rotating_wheel) = if index < self.inventory.components.len() {
                let definition = &self.inventory.components[index];
                (
                    definition.properties.name.clone(),
                    definition.component_perimeter.clone(),
                    definition.properties.attachment.clone(),
                    definition.rotating_wheel,
                )
            } else {
                (
                    "fuel_tank_point_reservoir".to_string(),
                    "fuel mass at the configured tank position".to_string(),
                    MassComponentAttachment::Body,
                    None,
                )
            };
            let parent_angular_velocity = moving.angular_velocity(suspension_velocity);
            let mut linear_acceleration = moving.linear_acceleration_bias_metres_per_second_squared;
            let mut angular_acceleration = moving.angular_acceleration_bias_radians_per_second_squared;
            for coordinate in 0..10 {
                linear_acceleration += moving.linear_velocity_jacobian[coordinate]
                    * generalized_acceleration[coordinate];
                angular_acceleration += moving.angular_velocity_jacobian[coordinate]
                    * generalized_acceleration[coordinate];
            }
            let mut angular_velocity = parent_angular_velocity;
            if let Some(wheel) = rotating_wheel {
                let axis = evaluation.wheel_axes_world[wheel as usize];
                let spin_rate = velocity[10 + wheel as usize];
                let spin_acceleration = generalized_acceleration[10 + wheel as usize];
                angular_velocity += axis * spin_rate;
                angular_acceleration += parent_angular_velocity.cross(axis) * spin_rate
                    + axis * spin_acceleration;
            }
            let effective_force = (linear_acceleration - gravity) * moving.mass_kilograms;
            let effective_moment = moving.inertia_world.multiply(angular_acceleration)
                + angular_velocity.cross(moving.inertia_world.multiply(angular_velocity));
            let mut contact_force = Vec3::ZERO;
            let mut contact_moment_about_center = Vec3::ZERO;
            if let Some(wheel) = rotating_wheel {
                let normal_force = evaluation.suspension_diagnostics.normal_force_newtons[wheel as usize];
                if normal_force > 0.0 && wheel_contact_carrier[wheel as usize] == Some(index) {
                    if let Some(contact) = input.contacts[wheel as usize] {
                        let surface_normal = contact.surface_normal_world;
                        let tangential = contact.tangential_force_world_newtons
                            - surface_normal
                                * contact.tangential_force_world_newtons.dot(surface_normal);
                        contact_force = surface_normal * normal_force + tangential;
                        let radius = if wheel.is_front() {
                            self.configuration.tires.front_tire_radius
                        } else {
                            self.configuration.tires.rear_tire_radius
                        };
                        contact_moment_about_center =
                            (-surface_normal * radius).cross(tangential);
                    }
                }
            }
            let attachment_force = effective_force - contact_force;
            let attachment_moment_about_center = effective_moment - contact_moment_about_center;
            let arm = moving.center_of_mass_world_metres - moving.attachment_origin_world_metres;
            let attachment_moment_about_origin =
                attachment_moment_about_center + arm.cross(attachment_force);
            let attachment_force_local = moving
                .attachment_orientation_world
                .inverse_transform_vector(attachment_force);
            let attachment_moment_local = moving
                .attachment_orientation_world
                .inverse_transform_vector(attachment_moment_about_origin);
            let free_body_force_residual = attachment_force + contact_force
                + gravity * moving.mass_kilograms
                - linear_acceleration * moving.mass_kilograms;
            let free_body_moment_residual = attachment_moment_about_center
                + contact_moment_about_center
                - effective_moment;
            let residual_magnitude = free_body_force_residual.length();
            let residual_moment_magnitude = free_body_moment_residual.length();
            let finite = [
                attachment_force,
                attachment_moment_about_origin,
                linear_acceleration,
                angular_acceleration,
            ]
            .iter()
            .all(|vector| {
                vector.x.is_finite() && vector.y.is_finite() && vector.z.is_finite()
            }) && moving.mass_kilograms.is_finite();
            maximum_free_body_residual = maximum_free_body_residual.max(residual_magnitude);
            maximum_free_body_moment_residual =
                maximum_free_body_moment_residual.max(residual_moment_magnitude);
            total_attachment_force += attachment_force;
            total_contact_force += contact_force;
            total_attachment_moment += attachment_moment_about_origin
                + (moving.attachment_origin_world_metres
                    - state.suspension.body_origin_world_metres)
                    .cross(attachment_force);
            components.push(CoupledComponentLoad {
                component_name: name,
                component_perimeter: perimeter,
                attachment,
                rotating_wheel,
                mass_kilograms: moving.mass_kilograms,
                center_of_mass_world_metres: moving.center_of_mass_world_metres,
                attachment_origin_world_metres: moving.attachment_origin_world_metres,
                linear_acceleration_world_metres_per_second_squared: linear_acceleration,
                angular_acceleration_world_radians_per_second_squared: angular_acceleration,
                effective_force_world_newtons: effective_force,
                contact_force_world_newtons: contact_force,
                contact_moment_about_center_world_newton_metres: contact_moment_about_center,
                attachment_force_world_newtons: attachment_force,
                attachment_force_local_newtons: attachment_force_local,
                attachment_moment_about_origin_world_newton_metres: attachment_moment_about_origin,
                attachment_moment_local_newton_metres: attachment_moment_local,
                attachment_couple_newton_metres: attachment_moment_about_origin.length(),
                free_body_force_residual_newtons: residual_magnitude,
                free_body_moment_residual_newton_metres: residual_moment_magnitude,
                finite,
            });
        }
        let force_residual = (total_attachment_force - input.body_force_world_newtons).length();
        let moment_residual = (total_attachment_moment
            - input.body_torque_about_origin_world_newton_metres)
            .length();
        let front = axle_summary(
            "front",
            &components,
            &evaluation.suspension_diagnostics,
            [WheelIndex::FrontLeft, WheelIndex::FrontRight],
        );
        let rear = axle_summary(
            "rear",
            &components,
            &evaluation.suspension_diagnostics,
            [WheelIndex::RearLeft, WheelIndex::RearRight],
        );
        Ok(CoupledVehicleLoadReport {
            acceleration_source,
            generalized_acceleration,
            components,
            body_applied_force_world_newtons: input.body_force_world_newtons,
            body_applied_torque_about_origin_world_newton_metres: input
                .body_torque_about_origin_world_newton_metres,
            total_attachment_force_world_newtons: total_attachment_force,
            total_contact_force_world_newtons: total_contact_force,
            action_reaction_force_residual_newtons: force_residual,
            total_attachment_moment_about_origin_world_newton_metres: total_attachment_moment,
            action_reaction_moment_residual_newton_metres: moment_residual,
            maximum_free_body_force_residual_newtons: maximum_free_body_residual,
            maximum_free_body_moment_residual_newton_metres: maximum_free_body_moment_residual,
            mass_matrix_pivot_ratio_estimate: mass_matrix_pivot_ratio_estimate(
                &evaluation.mass_matrix,
            )?,
            front,
            rear,
        })
    }
}

fn axle_summary(
    axle_label: &str,
    components: &[CoupledComponentLoad],
    diagnostics: &crate::suspension_coupled_solver::CoupledSuspensionDiagnostics,
    wheels: [WheelIndex; 2],
) -> CoupledAxleLoadSummary {
    let mut force = Vec3::ZERO;
    let mut rotating_count = 0;
    let mut suspension_count = 0;
    let mut maximum_couple: f64 = 0.0;
    for component in components {
        if component.rotating_wheel.is_some_and(|wheel| wheels.contains(&wheel)) {
            force += component.attachment_force_world_newtons;
            rotating_count += 1;
            maximum_couple = maximum_couple.max(component.attachment_couple_newton_metres);
        } else if component
            .attachment
            .wheel()
            .is_some_and(|wheel| wheels.contains(&wheel))
        {
            suspension_count += 1;
        }
    }
    CoupledAxleLoadSummary {
        axle_label: axle_label.to_string(),
        rotating_component_count: rotating_count,
        suspension_component_count: suspension_count,
        rotating_attachment_force_world_newtons: force,
        rotating_vertical_attachment_force_newtons: force.y,
        maximum_rotating_attachment_couple_newton_metres: maximum_couple,
        spring_force_newtons: [
            diagnostics.spring_force_newtons[wheels[0] as usize],
            diagnostics.spring_force_newtons[wheels[1] as usize],
        ],
        damper_force_newtons: [
            diagnostics.damper_force_newtons[wheels[0] as usize],
            diagnostics.damper_force_newtons[wheels[1] as usize],
        ],
        normal_force_newtons: [
            diagnostics.normal_force_newtons[wheels[0] as usize],
            diagnostics.normal_force_newtons[wheels[1] as usize],
        ],
    }
}

fn mass_matrix_pivot_ratio_estimate(matrix: &GeneralizedVehicleMatrix) -> Result<f64, String> {
    let factor = factor_generalized_vehicle_matrix(matrix)?;
    let mut minimum_pivot = f64::INFINITY;
    let mut maximum_pivot: f64 = 0.0;
    for row in 0..crate::coupled_vehicle_dynamics::VEHICLE_GENERALIZED_VELOCITY_COUNT {
        let pivot = factor[row][row];
        minimum_pivot = minimum_pivot.min(pivot);
        maximum_pivot = maximum_pivot.max(pivot);
    }
    if !minimum_pivot.is_finite() || minimum_pivot <= 0.0 || !maximum_pivot.is_finite() {
        return Err("Generalized mass matrix has no usable pivot estimate".into());
    }
    Ok(maximum_pivot / minimum_pivot)
}
