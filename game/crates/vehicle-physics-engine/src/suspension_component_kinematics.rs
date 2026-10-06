use crate::suspension_kinematics::{solve_corner, KinematicSolution};
use crate::suspension_mass_properties::{
    MassComponentAttachment, SuspensionMassComponent, SuspensionMassInventory,
};
use crate::suspension_multibody::{
    GeneralizedSuspensionVector, MovingSuspensionComponent, SUSPENSION_GENERALIZED_VELOCITY_COUNT,
};
use crate::types::{Mat3, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct SharedSuspensionGeometrySolutions {
    geometry: crate::suspension_geo_config::GeometricSuspensionConfig,
    solutions: Arc<Mutex<HashMap<(usize, u64, u64), KinematicSolution>>>,
}

impl SharedSuspensionGeometrySolutions {
    pub fn new(configuration: &VehicleConfig) -> Result<Self, String> {
        Ok(Self { geometry: configuration.geometric_suspension.clone().ok_or("Missing physical geometry")?,
            solutions: Arc::new(Mutex::new(HashMap::new())) })
    }
}

pub struct SuspensionGeometryEvaluationCache<'configuration> {
    configuration: &'configuration VehicleConfig,
    solutions: HashMap<(usize, u64, u64), KinematicSolution>,
    shared_solutions: Option<&'configuration SharedSuspensionGeometrySolutions>,
}

impl<'configuration> SuspensionGeometryEvaluationCache<'configuration> {
    pub fn new(configuration: &'configuration VehicleConfig) -> Self {
        Self {
            configuration,
            solutions: HashMap::new(),
            shared_solutions: None,
        }
    }

    pub fn with_shared_solutions(configuration: &'configuration VehicleConfig,
        shared_solutions: &'configuration SharedSuspensionGeometrySolutions) -> Result<Self, String> {
        if configuration.geometric_suspension.as_ref() != Some(&shared_solutions.geometry) {
            return Err("Shared geometry cache does not match the configured hardpoints".into());
        }
        Ok(Self { configuration, solutions: HashMap::new(), shared_solutions: Some(shared_solutions) })
    }

    pub fn solution(
        &mut self,
        wheel: WheelIndex,
        travel_metres: f64,
        steering_rack_metres: f64,
    ) -> Result<KinematicSolution, String> {
        if !travel_metres.is_finite() || !steering_rack_metres.is_finite() {
            return Err("Nonfinite cached geometry request".into());
        }
        let key = (
            wheel as usize,
            travel_metres.to_bits(),
            steering_rack_metres.to_bits(),
        );
        if let Some(solution) = self.solutions.get(&key) {
            return Ok(*solution);
        }
        if let Some(shared) = self.shared_solutions {
            let solution = shared.solutions.lock().map_err(|_| "Shared geometry cache lock was poisoned")?.get(&key).copied();
            if let Some(solution) = solution {
                self.solutions.insert(key, solution);
                return Ok(solution);
            }
        }
        let geometry = self
            .configuration
            .geometric_suspension
            .as_ref()
            .ok_or("Missing physical geometry")?;
        let solution = solve_corner(
            geometry.corners.get(wheel),
            travel_metres,
            steering_rack_metres,
            wheel.is_front(),
            0.0,
            0.0,
            1.0,
        )
        .ok_or("Component trajectory solve failed")?;
        if !solution.converged || solution.rocker_clamped || solution.steering_clamped {
            return Err(format!(
                "Unreachable physical component trajectory for {wheel:?}"
            ));
        }
        self.solutions.insert(key, solution);
        if let Some(shared) = self.shared_solutions {
            let mut solutions = shared.solutions.lock().map_err(|_| "Shared geometry cache lock was poisoned")?;
            if solutions.len() >= 4096 { solutions.clear(); }
            solutions.insert(key, solution);
        }
        Ok(solution)
    }

    pub fn distinct_solution_count(&self) -> usize {
        self.solutions.len()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SuspensionComponentPose {
    pub center_of_mass_local_metres: Vec3,
    pub orientation_local: Mat3,
}

fn multiply_rotations(first: Mat3, second: Mat3) -> Mat3 {
    Mat3::from_cols(
        first.transform_vector(second.x),
        first.transform_vector(second.y),
        first.transform_vector(second.z),
    )
}

fn attachment_frame(
    origin: Vec3,
    axis: Vec3,
    radial_reference: Vec3,
) -> Result<(Vec3, Mat3), String> {
    if axis.length() < 1e-9 {
        return Err("Degenerate component attachment axis".into());
    }
    let first_axis = axis.normalized();
    let second_axis =
        (radial_reference - first_axis * radial_reference.dot(first_axis)).normalized();
    if second_axis.length() < 0.9 {
        return Err("Degenerate component attachment frame".into());
    }
    Ok((
        origin,
        Mat3::from_cols(first_axis, second_axis, first_axis.cross(second_axis)),
    ))
}

pub fn component_pose(
    configuration: &VehicleConfig,
    component: &SuspensionMassComponent,
    wheel_travel_metres: [f64; 4],
    steering_rack_metres: f64,
) -> Result<SuspensionComponentPose, String> {
    component_pose_with_geometry_cache(
        component,
        wheel_travel_metres,
        steering_rack_metres,
        &mut SuspensionGeometryEvaluationCache::new(configuration),
    )
}

fn component_pose_with_geometry_cache(
    component: &SuspensionMassComponent,
    wheel_travel_metres: [f64; 4],
    steering_rack_metres: f64,
    cache: &mut SuspensionGeometryEvaluationCache<'_>,
) -> Result<SuspensionComponentPose, String> {
    let (origin, orientation_local) = if let Some(wheel) = component.attachment.wheel() {
        let solution = cache.solution(
            wheel,
            wheel_travel_metres[wheel as usize],
            steering_rack_metres,
        )?;
        let geometry = cache
            .configuration
            .geometric_suspension
            .as_ref()
            .ok_or("Physical component trajectories require geometric suspension")?;
        let corner = geometry.corners.get(wheel);
        match component.attachment {
            MassComponentAttachment::Upright { .. } => (solution.hub, solution.upright_basis),
            MassComponentAttachment::WheelRotor { .. } => {
                let configuration = cache.configuration;
                let camber = if wheel.is_front() { configuration.front_camber } else { configuration.rear_camber };
                let toe = if wheel.is_front() { configuration.front_toe } else { configuration.rear_toe };
                let side = if wheel.is_left() { 1.0 } else { -1.0 };
                let alignment = multiply_rotations(Mat3::from_axis_angle(Vec3::UP, toe * side),
                    Mat3::from_axis_angle(Vec3::BACK, camber * side));
                (solution.hub, multiply_rotations(solution.upright_basis, alignment))
            }
            MassComponentAttachment::LowerWishbone { .. } => {
                let pivot = (corner.lower.inner_front + corner.lower.inner_rear) * 0.5;
                attachment_frame(
                    pivot,
                    corner.lower.inner_rear - corner.lower.inner_front,
                    solution.lbj - pivot,
                )?
            }
            MassComponentAttachment::UpperWishbone { .. } => {
                let pivot = (corner.upper.inner_front + corner.upper.inner_rear) * 0.5;
                attachment_frame(
                    pivot,
                    corner.upper.inner_rear - corner.upper.inner_front,
                    solution.ubj - pivot,
                )?
            }
            MassComponentAttachment::TrackRod { .. } => {
                let inner = corner.trackrod_inner
                    + if wheel.is_front() {
                        Vec3::RIGHT * steering_rack_metres
                    } else {
                        Vec3::ZERO
                    };
                rod_attachment_frame(inner, solution.trackrod_outer)?
            }
            MassComponentAttachment::ActuationRod { .. } => {
                rod_attachment_frame(solution.rocker_end, solution.pushrod_outer)?
            }
            MassComponentAttachment::Rocker { .. } => (
                corner.rocker_pivot,
                Mat3::from_axis_angle(corner.rocker_axis, solution.rocker_angle),
            ),
            MassComponentAttachment::DamperCylinder { .. } => {
                rod_attachment_frame(corner.damper_chassis, solution.damper_end)?
            }
            MassComponentAttachment::DamperPiston { .. } => {
                rod_attachment_frame(solution.damper_end, corner.damper_chassis)?
            }
            MassComponentAttachment::Driveshaft { .. } => {
                let inner = corner.driveshaft_inner.ok_or("Driveshaft attachment requires measured inner hardpoint")?;
                rod_attachment_frame(inner, solution.hub)?
            }
            MassComponentAttachment::Body | MassComponentAttachment::SteeringRack => unreachable!(),
        }
    } else {
        (
            if matches!(component.attachment, MassComponentAttachment::SteeringRack) {
                Vec3::RIGHT * steering_rack_metres
            } else {
                Vec3::ZERO
            },
            Mat3::IDENTITY,
        )
    };
    Ok(SuspensionComponentPose {
        center_of_mass_local_metres: origin
            + orientation_local.transform_vector(component.center_of_mass_in_attachment_metres),
        orientation_local,
    })
}

fn rod_attachment_frame(inner: Vec3, outer: Vec3) -> Result<(Vec3, Mat3), String> {
    let axis = outer - inner;
    let reference = if axis.normalized().dot(Vec3::UP).abs() < 0.95 {
        Vec3::UP
    } else {
        Vec3::RIGHT
    };
    attachment_frame(inner, axis, reference)
}

fn angular_derivative(center: Mat3, positive: Mat3, negative: Mat3, separation: f64) -> Vec3 {
    (center.x.cross((positive.x - negative.x) / separation)
        + center.y.cross((positive.y - negative.y) / separation)
        + center.z.cross((positive.z - negative.z) / separation))
        * 0.5
}

#[derive(Debug, Clone, Copy)]
pub struct ComponentKinematicInput {
    pub body_origin_world_metres: Vec3,
    pub body_orientation_world: Mat3,
    pub generalized_velocity: GeneralizedSuspensionVector,
    pub wheel_travel_metres: [f64; 4],
    pub steering_rack_metres: f64,
    pub steering_rack_velocity_metres_per_second: f64,
    pub steering_rack_acceleration_metres_per_second_squared: f64,
    pub differentiation_step_metres: f64,
}

pub fn moving_component(
    configuration: &VehicleConfig,
    component: &SuspensionMassComponent,
    input: ComponentKinematicInput,
) -> Result<MovingSuspensionComponent, String> {
    moving_component_with_geometry_cache(
        component,
        input,
        &mut SuspensionGeometryEvaluationCache::new(configuration),
    )
}

pub fn moving_component_with_geometry_cache(
    component: &SuspensionMassComponent,
    input: ComponentKinematicInput,
    cache: &mut SuspensionGeometryEvaluationCache<'_>,
) -> Result<MovingSuspensionComponent, String> {
    let rotation = input.body_orientation_world;
    if !crate::suspension_mass_properties::vector_is_finite(rotation.x)
        || !crate::suspension_mass_properties::vector_is_finite(rotation.y)
        || !crate::suspension_mass_properties::vector_is_finite(rotation.z)
        || (rotation.x.length_squared() - 1.0).abs() > 1e-8
        || (rotation.y.length_squared() - 1.0).abs() > 1e-8
        || (rotation.z.length_squared() - 1.0).abs() > 1e-8
        || rotation.x.dot(rotation.y).abs() > 1e-8
        || rotation.x.dot(rotation.z).abs() > 1e-8
        || rotation.y.dot(rotation.z).abs() > 1e-8
        || rotation.x.cross(rotation.y).dot(rotation.z) < 1.0 - 1e-8
    {
        return Err("Component body orientation must be a proper orthonormal rotation".into());
    }
    let differentiation_step = input.differentiation_step_metres;
    if !differentiation_step.is_finite()
        || differentiation_step <= 0.0
        || differentiation_step > 0.01
    {
        return Err("Invalid component differentiation step".into());
    }
    if input
        .wheel_travel_metres
        .iter()
        .any(|value| !value.is_finite())
        || !input.steering_rack_metres.is_finite()
        || !input.steering_rack_velocity_metres_per_second.is_finite()
        || !input
            .steering_rack_acceleration_metres_per_second_squared
            .is_finite()
    {
        return Err("Nonfinite suspension trajectory input".into());
    }
    let center = component_pose_with_geometry_cache(
        component,
        input.wheel_travel_metres,
        input.steering_rack_metres,
        cache,
    )?;
    let mut linear_travel_derivative = Vec3::ZERO;
    let mut angular_travel_derivative = Vec3::ZERO;
    let mut linear_steering_derivative = Vec3::ZERO;
    let mut angular_steering_derivative = Vec3::ZERO;
    let mut relative_linear_velocity = Vec3::ZERO;
    let mut relative_angular_velocity = Vec3::ZERO;
    let mut relative_linear_acceleration = Vec3::ZERO;
    let mut relative_angular_acceleration = Vec3::ZERO;
    if let Some(wheel) = component.attachment.wheel() {
        let wheel_coordinate = wheel as usize;
        let travel_velocity = input.generalized_velocity[6 + wheel_coordinate];
        let mut positive_travel = input.wheel_travel_metres;
        let mut negative_travel = input.wheel_travel_metres;
        positive_travel[wheel_coordinate] += differentiation_step;
        negative_travel[wheel_coordinate] -= differentiation_step;
        let positive = component_pose_with_geometry_cache(
            component,
            positive_travel,
            input.steering_rack_metres,
            cache,
        )?;
        let negative = component_pose_with_geometry_cache(
            component,
            negative_travel,
            input.steering_rack_metres,
            cache,
        )?;
        linear_travel_derivative = (positive.center_of_mass_local_metres
            - negative.center_of_mass_local_metres)
            / (2.0 * differentiation_step);
        angular_travel_derivative = angular_derivative(
            center.orientation_local,
            positive.orientation_local,
            negative.orientation_local,
            2.0 * differentiation_step,
        );
        let steering_positive = component_pose_with_geometry_cache(
            component,
            input.wheel_travel_metres,
            input.steering_rack_metres + differentiation_step,
            cache,
        )?;
        let steering_negative = component_pose_with_geometry_cache(
            component,
            input.wheel_travel_metres,
            input.steering_rack_metres - differentiation_step,
            cache,
        )?;
        linear_steering_derivative = (steering_positive.center_of_mass_local_metres
            - steering_negative.center_of_mass_local_metres)
            / (2.0 * differentiation_step);
        angular_steering_derivative = angular_derivative(
            center.orientation_local,
            steering_positive.orientation_local,
            steering_negative.orientation_local,
            2.0 * differentiation_step,
        );
        relative_linear_velocity = linear_travel_derivative * travel_velocity
            + linear_steering_derivative * input.steering_rack_velocity_metres_per_second;
        relative_angular_velocity = angular_travel_derivative * travel_velocity
            + angular_steering_derivative * input.steering_rack_velocity_metres_per_second;
        let largest_relative_speed = travel_velocity
            .abs()
            .max(input.steering_rack_velocity_metres_per_second.abs());
        if largest_relative_speed > 1e-10 {
            let time_separation = differentiation_step / largest_relative_speed;
            let mut forward_travel = input.wheel_travel_metres;
            let mut backward_travel = input.wheel_travel_metres;
            forward_travel[wheel_coordinate] += travel_velocity * time_separation;
            backward_travel[wheel_coordinate] -= travel_velocity * time_separation;
            let forward = component_pose_with_geometry_cache(
                component,
                forward_travel,
                input.steering_rack_metres
                    + input.steering_rack_velocity_metres_per_second * time_separation,
                cache,
            )?;
            let backward = component_pose_with_geometry_cache(
                component,
                backward_travel,
                input.steering_rack_metres
                    - input.steering_rack_velocity_metres_per_second * time_separation,
                cache,
            )?;
            relative_linear_acceleration = (forward.center_of_mass_local_metres
                - center.center_of_mass_local_metres * 2.0
                + backward.center_of_mass_local_metres)
                / time_separation.powi(2);
            relative_angular_acceleration = (center.orientation_local.x.cross(
                (forward.orientation_local.x - center.orientation_local.x * 2.0
                    + backward.orientation_local.x)
                    / time_separation.powi(2),
            ) + center.orientation_local.y.cross(
                (forward.orientation_local.y - center.orientation_local.y * 2.0
                    + backward.orientation_local.y)
                    / time_separation.powi(2),
            ) + center.orientation_local.z.cross(
                (forward.orientation_local.z - center.orientation_local.z * 2.0
                    + backward.orientation_local.z)
                    / time_separation.powi(2),
            )) * 0.5;
        }
        relative_linear_acceleration +=
            linear_steering_derivative * input.steering_rack_acceleration_metres_per_second_squared;
        relative_angular_acceleration += angular_steering_derivative
            * input.steering_rack_acceleration_metres_per_second_squared;
    }
    if matches!(component.attachment, MassComponentAttachment::SteeringRack) {
        linear_steering_derivative = Vec3::RIGHT;
        relative_linear_velocity = Vec3::RIGHT * input.steering_rack_velocity_metres_per_second;
        relative_linear_acceleration = Vec3::RIGHT * input.steering_rack_acceleration_metres_per_second_squared;
    }
    let rotation = input.body_orientation_world;
    let offset_world = rotation.transform_vector(center.center_of_mass_local_metres);
    let body_angular_velocity = Vec3::new(
        input.generalized_velocity[3],
        input.generalized_velocity[4],
        input.generalized_velocity[5],
    );
    let relative_linear_velocity_world = rotation.transform_vector(relative_linear_velocity);
    let relative_angular_velocity_world = rotation.transform_vector(relative_angular_velocity);
    let mut linear_velocity_jacobian = [Vec3::ZERO; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    let mut angular_velocity_jacobian = [Vec3::ZERO; SUSPENSION_GENERALIZED_VELOCITY_COUNT];
    let axes = [Vec3::RIGHT, Vec3::UP, Vec3::BACK];
    for axis in 0..3 {
        linear_velocity_jacobian[axis] = axes[axis];
        linear_velocity_jacobian[3 + axis] = axes[axis].cross(offset_world);
        angular_velocity_jacobian[3 + axis] = axes[axis];
    }
    if let Some(wheel) = component.attachment.wheel() {
        linear_velocity_jacobian[6 + wheel as usize] =
            rotation.transform_vector(linear_travel_derivative);
        angular_velocity_jacobian[6 + wheel as usize] =
            rotation.transform_vector(angular_travel_derivative);
    }
    let orientation_world = multiply_rotations(rotation, center.orientation_local);
    let moving = MovingSuspensionComponent {
        mass_kilograms: component.mass_kilograms,
        inertia_world: component
            .inertia_about_center_of_mass
            .rotated(orientation_world),
        center_of_mass_world_metres: input.body_origin_world_metres + offset_world,
        linear_velocity_jacobian,
        angular_velocity_jacobian,
        prescribed_linear_velocity_metres_per_second: rotation.transform_vector(
            linear_steering_derivative * input.steering_rack_velocity_metres_per_second,
        ),
        prescribed_angular_velocity_radians_per_second: rotation.transform_vector(
            angular_steering_derivative * input.steering_rack_velocity_metres_per_second,
        ),
        linear_acceleration_bias_metres_per_second_squared: body_angular_velocity
            .cross(body_angular_velocity.cross(offset_world))
            + body_angular_velocity.cross(relative_linear_velocity_world) * 2.0
            + rotation.transform_vector(relative_linear_acceleration),
        angular_acceleration_bias_radians_per_second_squared: body_angular_velocity
            .cross(relative_angular_velocity_world)
            + rotation.transform_vector(relative_angular_acceleration),
    };
    moving.validate()?;
    Ok(moving)
}

pub fn moving_inventory(
    configuration: &VehicleConfig,
    inventory: &SuspensionMassInventory,
    input: ComponentKinematicInput,
) -> Result<Vec<MovingSuspensionComponent>, String> {
    moving_inventory_with_geometry_cache(
        inventory,
        input,
        &mut SuspensionGeometryEvaluationCache::new(configuration),
    )
}

pub fn moving_inventory_with_geometry_cache(
    inventory: &SuspensionMassInventory,
    input: ComponentKinematicInput,
    cache: &mut SuspensionGeometryEvaluationCache<'_>,
) -> Result<Vec<MovingSuspensionComponent>, String> {
    inventory.validate()?;
    inventory
        .components
        .iter()
        .map(|component| moving_component_with_geometry_cache(component, input, cache))
        .collect()
}

pub fn wheel_contact_component(
    configuration: &VehicleConfig,
    wheel: WheelIndex,
    input: ComponentKinematicInput,
) -> Result<MovingSuspensionComponent, String> {
    wheel_contact_component_with_geometry_cache(
        wheel,
        input,
        &mut SuspensionGeometryEvaluationCache::new(configuration),
    )
}

pub fn wheel_contact_component_with_geometry_cache(
    wheel: WheelIndex,
    input: ComponentKinematicInput,
    cache: &mut SuspensionGeometryEvaluationCache<'_>,
) -> Result<MovingSuspensionComponent, String> {
    let component = SuspensionMassComponent {
        name: "contact_jacobian".into(),
        mass_kilograms: 1.0,
        attachment: MassComponentAttachment::Upright { wheel },
        center_of_mass_in_attachment_metres: Vec3::ZERO,
        inertia_about_center_of_mass: crate::suspension_mass_properties::InertiaTensor {
            entries_kilogram_square_metres: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        provenance: crate::suspension_mass_properties::MassPropertyProvenance {
            mass_origin: crate::suspension_mass_properties::MassPropertyOrigin::Estimated,
            center_of_mass_origin:
                crate::suspension_mass_properties::MassPropertyOrigin::ProfileDeclared,
            inertia_origin: crate::suspension_mass_properties::MassPropertyOrigin::Estimated,
            source: "Kinematic evaluation only; excluded from mass inventory".into(),
        },
    };
    moving_component_with_geometry_cache(&component, input, cache)
}
