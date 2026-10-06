use crate::suspension_component_kinematics::{
    moving_inventory_with_geometry_cache, wheel_contact_component_with_geometry_cache,
    ComponentKinematicInput, SuspensionGeometryEvaluationCache, SharedSuspensionGeometrySolutions,
};
use crate::suspension_geo_config::AntiRollConfig;
use crate::suspension_kinematics::{jacobian, travel_envelope};
use crate::suspension_mass_properties::{
    composite_mass_properties, vector_is_finite, SuspensionMassInventory,
};
use crate::suspension_multibody::{
    CoupledSuspensionEquations, GeneralizedSuspensionVector, MovingSuspensionComponent,
    SUSPENSION_GENERALIZED_VELOCITY_COUNT,
};
use crate::types::{Quat, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoupledSuspensionSettings {
    pub maximum_substep_seconds: f64,
    pub differentiation_step_metres: f64,
    pub gravity_world_metres_per_second_squared: Vec3,
    pub tire_vertical_stiffness_newtons_per_metre: [f64; 4],
    pub tire_vertical_damping_newton_seconds_per_metre: [f64; 4],
    pub travel_stop_stiffness_newtons_per_cubic_metre: [f64; 4],
    pub travel_stop_damping_newton_seconds_per_metre: [f64; 4],
    pub travel_stop_engagement_fraction: [f64; 4],
}

impl CoupledSuspensionSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !self.maximum_substep_seconds.is_finite()
            || self.maximum_substep_seconds <= 0.0
            || self.maximum_substep_seconds > 0.01
            || !self.differentiation_step_metres.is_finite()
            || self.differentiation_step_metres <= 0.0
            || self.differentiation_step_metres > 0.001
            || !vector_is_finite(self.gravity_world_metres_per_second_squared)
        {
            return Err("Invalid coupled suspension integration settings".into());
        }
        for wheel in 0..4 {
            if !self.travel_stop_engagement_fraction[wheel].is_finite()
                || !(0.0..1.0).contains(&self.travel_stop_engagement_fraction[wheel])
            {
                return Err("Travel stop engagement fraction must lie between zero and one".into());
            }
            if !self.tire_vertical_stiffness_newtons_per_metre[wheel].is_finite()
                || self.tire_vertical_stiffness_newtons_per_metre[wheel] <= 0.0
            {
                return Err("Tire vertical stiffness must be finite and positive".into());
            }
            for coefficient in [
                self.tire_vertical_damping_newton_seconds_per_metre[wheel],
                self.travel_stop_stiffness_newtons_per_cubic_metre[wheel],
                self.travel_stop_damping_newton_seconds_per_metre[wheel],
            ] {
                if !coefficient.is_finite() || coefficient < 0.0 {
                    return Err("Invalid damping or travel stop coefficient".into());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoupledSuspensionState {
    pub body_origin_world_metres: Vec3,
    pub body_orientation_world: Quat,
    pub generalized_velocity: GeneralizedSuspensionVector,
    pub wheel_travel_metres: [f64; 4],
    pub simulated_time_seconds: f64,
    pub dissipated_energy_joules: f64,
    pub collision_energy_change_joules: f64,
    pub external_work_joules: f64,
    pub steering_actuator_work_joules: f64,
}

impl CoupledSuspensionState {
    pub fn at_rest(body_origin_world_metres: Vec3) -> Self {
        Self {
            body_origin_world_metres,
            body_orientation_world: Quat::IDENTITY,
            generalized_velocity: [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
            wheel_travel_metres: [0.0; 4],
            simulated_time_seconds: 0.0,
            dissipated_energy_joules: 0.0,
            collision_energy_change_joules: 0.0,
            external_work_joules: 0.0,
            steering_actuator_work_joules: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoupledWheelContact {
    pub surface_point_world_metres: Vec3,
    pub surface_normal_world: Vec3,
    pub surface_velocity_world_metres_per_second: Vec3,
    pub tangential_force_world_newtons: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoupledSuspensionInput {
    pub contacts: [Option<CoupledWheelContact>; 4],
    pub body_force_world_newtons: Vec3,
    pub body_torque_about_origin_world_newton_metres: Vec3,
    pub steering_rack_metres: f64,
    pub steering_rack_velocity_metres_per_second: f64,
    pub steering_rack_acceleration_metres_per_second_squared: f64,
}

impl Default for CoupledSuspensionInput {
    fn default() -> Self {
        Self {
            contacts: [None; 4],
            body_force_world_newtons: Vec3::ZERO,
            body_torque_about_origin_world_newton_metres: Vec3::ZERO,
            steering_rack_metres: 0.0,
            steering_rack_velocity_metres_per_second: 0.0,
            steering_rack_acceleration_metres_per_second_squared: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CoupledSuspensionDiagnostics {
    pub kinetic_energy_joules: f64,
    pub gravitational_potential_energy_joules: f64,
    pub elastic_potential_energy_joules: f64,
    pub total_energy_joules: f64,
    pub dissipated_power_watts: f64,
    pub external_power_watts: f64,
    pub steering_actuator_power_watts: f64,
    pub normal_force_newtons: [f64; 4],
    pub hub_velocity_world_metres_per_second: [Vec3; 4],
    pub spring_force_newtons: [f64; 4],
    pub damper_force_newtons: [f64; 4],
    pub motion_ratio: [f64; 4],
    pub center_of_mass_world_metres: Vec3,
    pub linear_momentum_world_kilogram_metres_per_second: Vec3,
    pub angular_momentum_about_world_origin_kilogram_square_metres_per_second: Vec3,
    pub generalized_acceleration: GeneralizedSuspensionVector,
}

pub struct CoupledSuspensionEvaluation {
    pub equations: CoupledSuspensionEquations,
    pub components: Vec<MovingSuspensionComponent>,
    pub diagnostics: CoupledSuspensionDiagnostics,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoupledFuelMassExchange {
    pub previous_mass_kilograms: f64,
    pub current_mass_kilograms: f64,
    pub system_energy_change_joules: f64,
    pub system_linear_momentum_change_kilogram_metres_per_second: Vec3,
    pub system_angular_momentum_change_kilogram_square_metres_per_second: Vec3,
}

#[derive(Debug, Clone)]
pub struct CoupledSuspensionModel {
    configuration: VehicleConfig,
    inventory: SuspensionMassInventory,
    settings: CoupledSuspensionSettings,
    wheel_travel_limits_metres: [(f64, f64); 4],
    antiroll_stiffness_newtons_per_metre: [f64; 2],
    shared_geometry_solutions: SharedSuspensionGeometrySolutions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoupledSuspensionDefinition {
    pub schema_version: u32,
    pub inventory: SuspensionMassInventory,
    pub settings: CoupledSuspensionSettings,
}

impl CoupledSuspensionDefinition {
    pub fn build_model(
        self,
        configuration: VehicleConfig,
    ) -> Result<CoupledSuspensionModel, String> {
        if self.schema_version != 1 {
            return Err("Unsupported coupled suspension definition version".into());
        }
        CoupledSuspensionModel::new(configuration, self.inventory, self.settings)
    }
}

impl CoupledSuspensionModel {
    pub fn update_fuel_mass(
        &mut self,
        state: &CoupledSuspensionState,
        input: &CoupledSuspensionInput,
        fuel_mass_kilograms: f64,
    ) -> Result<CoupledFuelMassExchange, String> {
        if !fuel_mass_kilograms.is_finite()
            || fuel_mass_kilograms < 0.0
            || fuel_mass_kilograms > self.configuration.fuel.capacity_kg
        {
            return Err("Fuel mass exchange lies outside tank capacity".into());
        }
        let previous_mass = self.configuration.fuel.effective_current_kg();
        if !vector_is_finite(state.body_origin_world_metres)
            || state.generalized_velocity.iter().any(|value| !value.is_finite())
            || !state.body_orientation_world.length_squared().is_finite()
            || (state.body_orientation_world.length_squared() - 1.0).abs() > 1e-6
            || !input.steering_rack_metres.is_finite()
        { return Err("Fuel exchange requires a finite physical state".into()); }
        let offset = state.body_orientation_world.to_mat3().transform_vector(self.configuration.fuel.tank_position_local_m);
        let position = state.body_origin_world_metres + offset;
        let angular_velocity = Vec3::new(state.generalized_velocity[3], state.generalized_velocity[4], state.generalized_velocity[5]);
        let velocity = Vec3::new(state.generalized_velocity[0], state.generalized_velocity[1], state.generalized_velocity[2])
            + angular_velocity.cross(offset);
        let current_mass = if self.configuration.fuel.is_enabled() { fuel_mass_kilograms } else { 0.0 };
        let mass_change = current_mass - previous_mass;
        let exchange = CoupledFuelMassExchange {
            previous_mass_kilograms: previous_mass,
            current_mass_kilograms: current_mass,
            system_energy_change_joules: mass_change * (0.5 * velocity.dot(velocity)
                - self.settings.gravity_world_metres_per_second_squared.dot(position)),
            system_linear_momentum_change_kilogram_metres_per_second: velocity * mass_change,
            system_angular_momentum_change_kilogram_square_metres_per_second: position.cross(velocity) * mass_change,
        };
        self.configuration.fuel.set_current_kg(fuel_mass_kilograms);
        Ok(exchange)
    }

    pub fn apply_host_collision_velocity_change(
        &self,
        state: &mut CoupledSuspensionState,
        input: &CoupledSuspensionInput,
        body_velocity_change: [f64; 6],
    ) -> Result<(), String> {
        let equations = self.evaluate(state, input)?.equations;
        let mut candidate = state.clone();
        candidate.collision_energy_change_joules += equations.apply_body_velocity_change(
            &mut candidate.generalized_velocity,
            body_velocity_change,
        )?;
        self.evaluate(&candidate, input)?;
        *state = candidate;
        Ok(())
    }
    pub fn new(
        configuration: VehicleConfig,
        inventory: SuspensionMassInventory,
        settings: CoupledSuspensionSettings,
    ) -> Result<Self, String> {
        inventory.validate()?;
        settings.validate()?;
        if configuration.geometric_suspension.is_none() {
            return Err("Coupled dynamics requires geometric hardpoints".into());
        }
        if (configuration.complete_dry_vehicle_mass()
            - inventory.declared_complete_dry_mass_kilograms)
            .abs()
            > 1e-8 * configuration.complete_dry_vehicle_mass()
        {
            return Err("Inventory dry mass disagrees with vehicle profile mass contract".into());
        }
        let geometry = configuration.geometric_suspension.as_ref().unwrap();
        let mut wheel_travel_limits_metres = [(0.0, 0.0); 4];
        for wheel in WheelIndex::ALL {
            let axle = geometry.axle(wheel);
            wheel_travel_limits_metres[wheel as usize] = travel_envelope(
                geometry.corners.get(wheel),
                axle.wheel_droop_m,
                axle.wheel_bump_m,
                0.0,
                wheel.is_front(),
            );
        }
        let mut antiroll_stiffness_newtons_per_metre = [0.0; 2];
        for (axle_index, wheel) in [WheelIndex::FrontLeft, WheelIndex::RearLeft]
            .into_iter()
            .enumerate()
        {
            let corner = geometry.corners.get(wheel);
            let axle = geometry.axle(wheel);
            let rest_ratio = jacobian(corner, 0.0, 0.0, wheel.is_front())
                .ok_or("Antiroll rest Jacobian failed")?
                .motion_ratio;
            let step = settings.differentiation_step_metres;
            let ratio_derivative = (jacobian(corner, step, 0.0, wheel.is_front())
                .ok_or("Antiroll tangent failed")?
                .motion_ratio
                - jacobian(corner, -step, 0.0, wheel.is_front())
                    .ok_or("Antiroll tangent failed")?
                    .motion_ratio)
                / (2.0 * step);
            let preload = axle.spring_rate_N_per_m
                * (axle.spring_free_length_m - axle.spring_installed_length_m);
            let rest_stiffness =
                axle.spring_rate_N_per_m * rest_ratio.powi(2) + preload * ratio_derivative;
            antiroll_stiffness_newtons_per_metre[axle_index] = match if wheel.is_front() {
                geometry.front_arb
            } else {
                geometry.rear_arb
            } {
                AntiRollConfig::LegacyRatio { ratio } => rest_stiffness * ratio,
                AntiRollConfig::MotionRatio {
                    bar_rate_N_per_m,
                    motion_ratio,
                } => bar_rate_N_per_m * motion_ratio.powi(2),
            };
            if !antiroll_stiffness_newtons_per_metre[axle_index].is_finite()
                || antiroll_stiffness_newtons_per_metre[axle_index] < 0.0
            {
                return Err("Invalid antiroll stiffness at design rest".into());
            }
        }
        let shared_geometry_solutions = SharedSuspensionGeometrySolutions::new(&configuration)?;
        let model = Self {
            configuration,
            inventory,
            settings,
            wheel_travel_limits_metres,
            antiroll_stiffness_newtons_per_metre,
            shared_geometry_solutions,
        };
        model.evaluate(
            &CoupledSuspensionState::at_rest(Vec3::ZERO),
            &CoupledSuspensionInput::default(),
        )?;
        Ok(model)
    }

    pub fn evaluate(
        &self,
        state: &CoupledSuspensionState,
        input: &CoupledSuspensionInput,
    ) -> Result<CoupledSuspensionEvaluation, String> {
        let geometry = self
            .configuration
            .geometric_suspension
            .as_ref()
            .ok_or("Missing geometric suspension")?;
        if [
            state.simulated_time_seconds,
            state.dissipated_energy_joules,
            state.collision_energy_change_joules,
            state.external_work_joules,
            state.steering_actuator_work_joules,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err("Nonfinite coupled state energy or clock".into());
        }
        if !vector_is_finite(state.body_origin_world_metres)
            || !state.body_orientation_world.length_squared().is_finite()
            || (state.body_orientation_world.length_squared() - 1.0).abs() > 1e-6
            || !vector_is_finite(input.body_force_world_newtons)
            || !vector_is_finite(input.body_torque_about_origin_world_newton_metres)
        {
            return Err("Invalid coupled body state or applied wrench".into());
        }
        let kinematic_input = ComponentKinematicInput {
            body_origin_world_metres: state.body_origin_world_metres,
            body_orientation_world: state.body_orientation_world.to_mat3(),
            generalized_velocity: state.generalized_velocity,
            wheel_travel_metres: state.wheel_travel_metres,
            steering_rack_metres: input.steering_rack_metres,
            steering_rack_velocity_metres_per_second: input
                .steering_rack_velocity_metres_per_second,
            steering_rack_acceleration_metres_per_second_squared: input
                .steering_rack_acceleration_metres_per_second_squared,
            differentiation_step_metres: self.settings.differentiation_step_metres,
        };
        let mut geometry_cache = SuspensionGeometryEvaluationCache::with_shared_solutions(&self.configuration, &self.shared_geometry_solutions)?;
        let mut components = moving_inventory_with_geometry_cache(
            &self.inventory,
            kinematic_input,
            &mut geometry_cache,
        )?;
        let fuel_mass = self.configuration.fuel.effective_current_kg();
        if fuel_mass > 0.0 {
            let fuel_offset = kinematic_input
                .body_orientation_world
                .transform_vector(self.configuration.fuel.tank_position_local_m);
            let mut linear_jacobian = [Vec3::ZERO; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
            let axes = [Vec3::RIGHT, Vec3::UP, Vec3::BACK];
            for axis in 0..3 {
                linear_jacobian[axis] = axes[axis];
                linear_jacobian[axis + 3] = axes[axis].cross(fuel_offset);
            }
            let angular_velocity = Vec3::new(
                state.generalized_velocity[3],
                state.generalized_velocity[4],
                state.generalized_velocity[5],
            );
            components.push(MovingSuspensionComponent {
                mass_kilograms: fuel_mass,
                inertia_world: crate::suspension_mass_properties::InertiaTensor {
                    entries_kilogram_square_metres: [
                        [0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0],
                        [0.0, 0.0, 0.0],
                    ],
                },
                center_of_mass_world_metres: state.body_origin_world_metres + fuel_offset,
                linear_velocity_jacobian: linear_jacobian,
                angular_velocity_jacobian: [Vec3::ZERO; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
                prescribed_linear_velocity_metres_per_second: Vec3::ZERO,
                prescribed_angular_velocity_radians_per_second: Vec3::ZERO,
                linear_acceleration_bias_metres_per_second_squared: angular_velocity
                    .cross(angular_velocity.cross(fuel_offset)),
                angular_acceleration_bias_radians_per_second_squared: Vec3::ZERO,
            });
        }
        let equations = CoupledSuspensionEquations::assemble(
            &components,
            state.generalized_velocity,
            self.settings.gravity_world_metres_per_second_squared,
        )?;
        let mut applied_force = [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
        applied_force[..6].copy_from_slice(&[
            input.body_force_world_newtons.x,
            input.body_force_world_newtons.y,
            input.body_force_world_newtons.z,
            input.body_torque_about_origin_world_newton_metres.x,
            input.body_torque_about_origin_world_newton_metres.y,
            input.body_torque_about_origin_world_newton_metres.z,
        ]);
        let mut diagnostics = CoupledSuspensionDiagnostics {
            kinetic_energy_joules: components
                .iter()
                .map(|component| component.kinetic_energy_joules(state.generalized_velocity))
                .sum(),
            gravitational_potential_energy_joules: components
                .iter()
                .map(|component| {
                    -component.mass_kilograms
                        * self
                            .settings
                            .gravity_world_metres_per_second_squared
                            .dot(component.center_of_mass_world_metres)
                })
                .sum(),
            elastic_potential_energy_joules: 0.0,
            total_energy_joules: 0.0,
            dissipated_power_watts: 0.0,
            external_power_watts: input.body_force_world_newtons.dot(Vec3::new(
                state.generalized_velocity[0],
                state.generalized_velocity[1],
                state.generalized_velocity[2],
            )) + input.body_torque_about_origin_world_newton_metres.dot(
                Vec3::new(
                    state.generalized_velocity[3],
                    state.generalized_velocity[4],
                    state.generalized_velocity[5],
                ),
            ),
            steering_actuator_power_watts: 0.0,
            normal_force_newtons: [0.0; 4],
            hub_velocity_world_metres_per_second: [Vec3::ZERO; 4],
            spring_force_newtons: [0.0; 4],
            damper_force_newtons: [0.0; 4],
            motion_ratio: [0.0; 4],
            center_of_mass_world_metres: Vec3::ZERO,
            linear_momentum_world_kilogram_metres_per_second: Vec3::ZERO,
            angular_momentum_about_world_origin_kilogram_square_metres_per_second: Vec3::ZERO,
            generalized_acceleration: [0.0; SUSPENSION_GENERALIZED_VELOCITY_COUNT],
        };
        for wheel in WheelIndex::ALL {
            let wheel_coordinate = wheel as usize;
            let travel = state.wheel_travel_metres[wheel_coordinate];
            let (minimum_travel, maximum_travel) =
                self.wheel_travel_limits_metres[wheel_coordinate];
            if travel < minimum_travel - 1e-10 || travel > maximum_travel + 1e-10 {
                return Err("Coupled travel lies outside the physical envelope".into());
            }
            let travel_velocity = state.generalized_velocity[6 + wheel_coordinate];
            let axle = geometry.axle(wheel);
            let solution = geometry_cache.solution(wheel, travel, input.steering_rack_metres)?;
            if solution.damper_length < axle.damper_min_m - 1e-8
                || solution.damper_length > axle.damper_max_m + 1e-8
            {
                return Err("Coupled spring element stroke lies outside physical limits".into());
            }
            let sensitivity_step = crate::suspension_kinematics::JAC_EPS_M;
            let positive_travel = geometry_cache.solution(wheel, travel + sensitivity_step, input.steering_rack_metres)?;
            let negative_travel = geometry_cache.solution(wheel, travel - sensitivity_step, input.steering_rack_metres)?;
            let sensitivity = crate::suspension_kinematics::KinematicJacobian {
                motion_ratio: (positive_travel.damper_compression - negative_travel.damper_compression) / (2.0 * sensitivity_step),
                dhub_dq: (positive_travel.hub - negative_travel.hub) / (2.0 * sensitivity_step),
                drocker_dq: (positive_travel.rocker_angle - negative_travel.rocker_angle) / (2.0 * sensitivity_step),
                ddamper_dq: (positive_travel.damper_length - negative_travel.damper_length) / (2.0 * sensitivity_step),
            };
            if !sensitivity.motion_ratio.is_finite() || sensitivity.motion_ratio <= 0.0 {
                return Err("Invalid spring motion ratio".into());
            }
            let compression = (axle.spring_free_length_m - axle.spring_installed_length_m
                + solution.damper_compression)
                .max(0.0);
            let rack_step = self.settings.differentiation_step_metres;
            let positive_rack =
                geometry_cache.solution(wheel, travel, input.steering_rack_metres + rack_step)?;
            let negative_rack =
                geometry_cache.solution(wheel, travel, input.steering_rack_metres - rack_step)?;
            let spring_steering_derivative = (positive_rack.damper_compression
                - negative_rack.damper_compression)
                / (2.0 * rack_step);
            let shaft_velocity = sensitivity.motion_ratio * travel_velocity
                + spring_steering_derivative * input.steering_rack_velocity_metres_per_second;
            let damper_coefficient = if shaft_velocity >= 0.0 {
                axle.damper_bump_Ns_per_m
            } else {
                axle.damper_rebound_Ns_per_m
            };
            let damper_force = shaft_velocity.signum()
                * damper_coefficient
                * (shaft_velocity.abs().min(axle.damper_knee_m_per_s)
                    + (shaft_velocity.abs() - axle.damper_knee_m_per_s).max(0.0)
                        * axle.damper_fast_factor);
            let spring_force = axle.spring_rate_N_per_m * compression;
            applied_force[6 + wheel_coordinate] -=
                (spring_force + damper_force) * sensitivity.motion_ratio;
            diagnostics.elastic_potential_energy_joules +=
                0.5 * axle.spring_rate_N_per_m * compression.powi(2);
            diagnostics.dissipated_power_watts += damper_force * shaft_velocity;
            diagnostics.steering_actuator_power_watts += (spring_force + damper_force)
                * spring_steering_derivative
                * input.steering_rack_velocity_metres_per_second;
            diagnostics.spring_force_newtons[wheel_coordinate] = spring_force;
            diagnostics.damper_force_newtons[wheel_coordinate] = damper_force;
            diagnostics.motion_ratio[wheel_coordinate] = sensitivity.motion_ratio;
            let stop_start_bump =
                axle.wheel_bump_m * self.settings.travel_stop_engagement_fraction[wheel_coordinate];
            let stop_start_droop = -axle.wheel_droop_m
                * self.settings.travel_stop_engagement_fraction[wheel_coordinate];
            let penetration = if travel > stop_start_bump {
                travel - stop_start_bump
            } else if travel < stop_start_droop {
                travel - stop_start_droop
            } else {
                0.0
            };
            let stop_stiffness =
                self.settings.travel_stop_stiffness_newtons_per_cubic_metre[wheel_coordinate];
            let stop_damping = if penetration != 0.0 && penetration * travel_velocity > 0.0 {
                self.settings.travel_stop_damping_newton_seconds_per_metre[wheel_coordinate]
            } else {
                0.0
            };
            applied_force[6 + wheel_coordinate] -=
                stop_stiffness * penetration.powi(3) + stop_damping * travel_velocity;
            diagnostics.elastic_potential_energy_joules +=
                0.25 * stop_stiffness * penetration.powi(4);
            diagnostics.dissipated_power_watts += stop_damping * travel_velocity.powi(2);
            let hub = wheel_contact_component_with_geometry_cache(
                wheel,
                kinematic_input,
                &mut geometry_cache,
            )?;
            let hub_velocity = hub.linear_velocity(state.generalized_velocity);
            diagnostics.hub_velocity_world_metres_per_second[wheel_coordinate] = hub_velocity;
            if let Some(contact) = input.contacts[wheel_coordinate] {
                if !vector_is_finite(contact.surface_point_world_metres)
                    || !vector_is_finite(contact.surface_normal_world)
                    || !vector_is_finite(contact.surface_velocity_world_metres_per_second)
                    || !vector_is_finite(contact.tangential_force_world_newtons)
                    || (contact.surface_normal_world.length() - 1.0).abs() > 1e-6
                {
                    return Err("Invalid coupled tire contact".into());
                }
                let radius = if wheel.is_front() {
                    self.configuration.front_tire_radius
                } else {
                    self.configuration.rear_tire_radius
                };
                let deflection = (radius
                    - (hub.center_of_mass_world_metres - contact.surface_point_world_metres)
                        .dot(contact.surface_normal_world))
                .max(0.0);
                if deflection > 0.0 {
                    let closing_velocity = -(hub_velocity
                        - contact.surface_velocity_world_metres_per_second)
                        .dot(contact.surface_normal_world);
                    let elastic_force = self.settings.tire_vertical_stiffness_newtons_per_metre
                        [wheel_coordinate]
                        * deflection;
                    let normal_force = (elastic_force
                        + self.settings.tire_vertical_damping_newton_seconds_per_metre
                            [wheel_coordinate]
                            * closing_velocity)
                        .max(0.0);
                    let tangential_force = contact.tangential_force_world_newtons
                        - contact.surface_normal_world
                            * contact
                                .tangential_force_world_newtons
                                .dot(contact.surface_normal_world);
                    let contact_force =
                        contact.surface_normal_world * normal_force + tangential_force;
                    let contact_torque =
                        (-contact.surface_normal_world * radius).cross(tangential_force);
                    let generalized_contact = hub.generalized_force(contact_force, contact_torque);
                    for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
                        applied_force[coordinate] += generalized_contact[coordinate];
                    }
                    diagnostics.normal_force_newtons[wheel_coordinate] = normal_force;
                    diagnostics.elastic_potential_energy_joules += 0.5
                        * self.settings.tire_vertical_stiffness_newtons_per_metre[wheel_coordinate]
                        * deflection.powi(2);
                    diagnostics.dissipated_power_watts +=
                        (normal_force - elastic_force) * closing_velocity;
                    diagnostics.external_power_watts += contact_force
                        .dot(contact.surface_velocity_world_metres_per_second)
                        + tangential_force
                            .dot(hub_velocity - contact.surface_velocity_world_metres_per_second)
                        + contact_torque.dot(hub.angular_velocity(state.generalized_velocity));
                    diagnostics.steering_actuator_power_watts -= contact_force
                        .dot(hub.prescribed_linear_velocity_metres_per_second)
                        + contact_torque.dot(hub.prescribed_angular_velocity_radians_per_second);
                }
            }
        }
        for (axle_index, (left, right)) in [(0, 1), (2, 3)].into_iter().enumerate() {
            let stiffness = self.antiroll_stiffness_newtons_per_metre[axle_index];
            let difference = state.wheel_travel_metres[left] - state.wheel_travel_metres[right];
            applied_force[6 + left] -= stiffness * difference;
            applied_force[6 + right] += stiffness * difference;
            diagnostics.elastic_potential_energy_joules += 0.5 * stiffness * difference.powi(2);
        }
        let mass_properties: Vec<_> = components
            .iter()
            .map(|component| {
                (
                    component.mass_kilograms,
                    component.center_of_mass_world_metres,
                    component.inertia_world,
                )
            })
            .collect();
        diagnostics.center_of_mass_world_metres = composite_mass_properties(&mass_properties)?.1;
        for component in &components {
            let linear_momentum =
                component.linear_velocity(state.generalized_velocity) * component.mass_kilograms;
            diagnostics.linear_momentum_world_kilogram_metres_per_second += linear_momentum;
            diagnostics.angular_momentum_about_world_origin_kilogram_square_metres_per_second +=
                component.center_of_mass_world_metres.cross(linear_momentum)
                    + component
                        .inertia_world
                        .multiply(component.angular_velocity(state.generalized_velocity));
        }
        diagnostics.total_energy_joules = diagnostics.kinetic_energy_joules
            + diagnostics.gravitational_potential_energy_joules
            + diagnostics.elastic_potential_energy_joules;
        diagnostics.generalized_acceleration = equations.acceleration(applied_force)?;
        for component in &components {
            let linear_acceleration = component
                .linear_velocity_jacobian
                .iter()
                .zip(diagnostics.generalized_acceleration)
                .fold(
                    component.linear_acceleration_bias_metres_per_second_squared,
                    |sum, (derivative, acceleration)| sum + *derivative * acceleration,
                );
            let angular_acceleration = component
                .angular_velocity_jacobian
                .iter()
                .zip(diagnostics.generalized_acceleration)
                .fold(
                    component.angular_acceleration_bias_radians_per_second_squared,
                    |sum, (derivative, acceleration)| sum + *derivative * acceleration,
                );
            let angular_velocity = component.angular_velocity(state.generalized_velocity);
            let inertial_torque = component.inertia_world.multiply(angular_acceleration)
                + angular_velocity.cross(component.inertia_world.multiply(angular_velocity));
            diagnostics.steering_actuator_power_watts += (linear_acceleration
                - self.settings.gravity_world_metres_per_second_squared)
                .dot(component.prescribed_linear_velocity_metres_per_second)
                * component.mass_kilograms
                + inertial_torque.dot(component.prescribed_angular_velocity_radians_per_second);
        }
        Ok(CoupledSuspensionEvaluation {
            equations,
            components,
            diagnostics,
        })
    }

    pub fn advance(
        &self,
        state: &mut CoupledSuspensionState,
        input: &CoupledSuspensionInput,
        duration_seconds: f64,
    ) -> Result<CoupledSuspensionDiagnostics, String> {
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 || duration_seconds > 1.0 {
            return Err("Invalid coupled integration duration".into());
        }
        let substep_count =
            (duration_seconds / self.settings.maximum_substep_seconds).ceil() as usize;
        if substep_count > 10000 {
            return Err("Coupled substep budget exceeded".into());
        }
        let substep_seconds = duration_seconds / substep_count as f64;
        let mut candidate = state.clone();
        for substep in 0..substep_count {
            let mut substep_input = input.clone();
            let elapsed = substep as f64 * substep_seconds;
            substep_input.steering_rack_metres += input.steering_rack_velocity_metres_per_second
                * elapsed
                + 0.5
                    * input.steering_rack_acceleration_metres_per_second_squared
                    * elapsed.powi(2);
            substep_input.steering_rack_velocity_metres_per_second +=
                input.steering_rack_acceleration_metres_per_second_squared * elapsed;
            for contact in substep_input.contacts.iter_mut().flatten() {
                contact.surface_point_world_metres +=
                    contact.surface_velocity_world_metres_per_second * elapsed;
            }
            let evaluation = self.evaluate(&candidate, &substep_input)?;
            for coordinate in 0..SUSPENSION_GENERALIZED_VELOCITY_COUNT {
                candidate.generalized_velocity[coordinate] +=
                    evaluation.diagnostics.generalized_acceleration[coordinate] * substep_seconds;
            }
            candidate.body_origin_world_metres += Vec3::new(
                candidate.generalized_velocity[0],
                candidate.generalized_velocity[1],
                candidate.generalized_velocity[2],
            ) * substep_seconds;
            let angular_velocity_world = Vec3::new(
                candidate.generalized_velocity[3],
                candidate.generalized_velocity[4],
                candidate.generalized_velocity[5],
            );
            let angular_velocity_local = candidate
                .body_orientation_world
                .to_mat3()
                .inverse_transform_vector(angular_velocity_world);
            candidate.body_orientation_world = candidate
                .body_orientation_world
                .integrate_angular_velocity(angular_velocity_local, substep_seconds)
                .normalized();
            let mut active_stops = [false; 4];
            for wheel in WheelIndex::ALL {
                let wheel_coordinate = wheel as usize;
                candidate.wheel_travel_metres[wheel_coordinate] +=
                    candidate.generalized_velocity[6 + wheel_coordinate] * substep_seconds;
                let limited = candidate.wheel_travel_metres[wheel_coordinate].clamp(
                    self.wheel_travel_limits_metres[wheel_coordinate].0,
                    self.wheel_travel_limits_metres[wheel_coordinate].1,
                );
                if limited != candidate.wheel_travel_metres[wheel_coordinate] {
                    candidate.wheel_travel_metres[wheel_coordinate] = limited;
                    active_stops[wheel_coordinate] = true;
                }
            }
            if active_stops.iter().any(|active| *active) {
                candidate.dissipated_energy_joules +=
                    evaluation.equations.project_travel_stop_velocities(
                        &mut candidate.generalized_velocity,
                        active_stops,
                    )?;
            }
            candidate.dissipated_energy_joules +=
                evaluation.diagnostics.dissipated_power_watts * substep_seconds;
            candidate.external_work_joules +=
                evaluation.diagnostics.external_power_watts * substep_seconds;
            candidate.steering_actuator_work_joules +=
                evaluation.diagnostics.steering_actuator_power_watts * substep_seconds;
            candidate.simulated_time_seconds += substep_seconds;
        }
        let mut final_input = input.clone();
        final_input.steering_rack_metres += input.steering_rack_velocity_metres_per_second
            * duration_seconds
            + 0.5
                * input.steering_rack_acceleration_metres_per_second_squared
                * duration_seconds.powi(2);
        final_input.steering_rack_velocity_metres_per_second +=
            input.steering_rack_acceleration_metres_per_second_squared * duration_seconds;
        for contact in final_input.contacts.iter_mut().flatten() {
            contact.surface_point_world_metres +=
                contact.surface_velocity_world_metres_per_second * duration_seconds;
        }
        let diagnostics = self.evaluate(&candidate, &final_input)?.diagnostics;
        *state = candidate;
        Ok(diagnostics)
    }
}
