use super::{geometric_damper_element, rod_force_from_solution, PreparedGeometricSuspension};
use crate::suspension_geo_config::GeometricSuspensionConfig;
use crate::suspension_kinematics::{jacobian, solve_corner, KinematicSolution};
use crate::suspension_loads::{
    anchor_wrench, link_frame, reactions_with_prepared_response, shaft_outer_at_pose, solve_links,
    steered_trackrod_outer, GeoWheelWrench, LinkFrame,
};
use crate::types::{Mat3, Transform3D, Vec3, WheelIndex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum SuspensionBackend {
    #[default]
    Standard,
    Simplified,
}

impl SuspensionBackend {
    pub fn from_execution_environment() -> Self {
        match std::env::var("FORMULA90_SUSPENSION_BACKEND").as_deref() {
            Ok("simplified") => Self::Simplified,
            Ok("standard") | Err(_) => Self::Standard,
            Ok(value) => panic!("Unsupported suspension backend: {value}"),
        }
    }
}

const TRAVEL_SAMPLE_COUNT: usize = 129;
const STEERING_SAMPLE_COUNT: usize = 33;
const MAXIMUM_STEERING_RADIANS: f64 = 0.8;

#[derive(Debug, Clone)]
struct PreparedTravelSample {
    solution: KinematicSolution,
    motion_ratio: f64,
    steering_responses: Vec<[[f64; 7]; 6]>,
}

#[derive(Debug, Clone)]
struct PreparedCorner {
    minimum_travel_metres: f64,
    maximum_travel_metres: f64,
    samples: Vec<Option<PreparedTravelSample>>,
}

#[derive(Debug, Clone)]
pub(super) struct SimplifiedSuspension {
    corners: [PreparedCorner; 4],
}

fn frame_at(
    geometry: &GeometricSuspensionConfig,
    wheel: WheelIndex,
    solution: &KinematicSolution,
    steering_radians: f64,
) -> LinkFrame {
    let corner = geometry.corners.get(wheel);
    let steered = steered_trackrod_outer(
        solution.lbj,
        solution.ubj,
        corner.trackrod_outer,
        corner.lower.outer,
        steering_radians,
    );
    let shaft = wheel.is_rear().then(|| {
        shaft_outer_at_pose(
            solution.hub,
            corner.hub_center,
            corner.driveshaft_outer.unwrap_or(corner.hub_center),
        )
    });
    link_frame(corner, solution, steered, shaft)
}

fn prepare_response(frame: &LinkFrame) -> Option<[[f64; 7]; 6]> {
    let mut responses = [[0.0; 7]; 6];
    for component in 0..6 {
        let mut values = [0.0; 6];
        values[component] = 1.0;
        let (forces, _, singular) = solve_links(
            frame,
            Vec3::new(values[0], values[1], values[2]),
            Vec3::new(values[3], values[4], values[5]),
        );
        if singular || forces.iter().any(|force| !force.is_finite()) {
            return None;
        }
        responses[component][..forces.len()].copy_from_slice(&forces);
    }
    Some(responses)
}

fn interpolation_position(
    value: f64,
    minimum: f64,
    maximum: f64,
    count: usize,
) -> Option<(usize, f64)> {
    if !value.is_finite() || value < minimum || value > maximum || maximum <= minimum {
        return None;
    }
    let position = (value - minimum) / (maximum - minimum) * (count - 1) as f64;
    let index = (position.floor() as usize).min(count - 2);
    Some((index, position - index as f64))
}

fn interpolate_solution(
    first: KinematicSolution,
    second: KinematicSolution,
    fraction: f64,
) -> KinematicSolution {
    let interpolate_vector = |first: Vec3, second: Vec3| first + (second - first) * fraction;
    let interpolate_scalar = |first: f64, second: f64| first + (second - first) * fraction;
    let interpolate_basis = |first: Mat3, second: Mat3| Mat3 {
        x: interpolate_vector(first.x, second.x).normalized(),
        y: interpolate_vector(first.y, second.y).normalized(),
        z: interpolate_vector(first.z, second.z).normalized(),
    };
    KinematicSolution {
        hub: interpolate_vector(first.hub, second.hub),
        hub_target: interpolate_vector(first.hub_target, second.hub_target),
        lbj: interpolate_vector(first.lbj, second.lbj),
        ubj: interpolate_vector(first.ubj, second.ubj),
        upright_basis: interpolate_basis(first.upright_basis, second.upright_basis),
        wheel_basis: interpolate_basis(first.wheel_basis, second.wheel_basis),
        rocker_angle: interpolate_scalar(first.rocker_angle, second.rocker_angle),
        rocker_clamped: first.rocker_clamped || second.rocker_clamped,
        steering_clamped: first.steering_clamped || second.steering_clamped,
        damper_length: interpolate_scalar(first.damper_length, second.damper_length),
        damper_compression: interpolate_scalar(first.damper_compression, second.damper_compression),
        rocker_end: interpolate_vector(first.rocker_end, second.rocker_end),
        pushrod_outer: interpolate_vector(first.pushrod_outer, second.pushrod_outer),
        damper_end: interpolate_vector(first.damper_end, second.damper_end),
        trackrod_outer: interpolate_vector(first.trackrod_outer, second.trackrod_outer),
        travel_limited: first.travel_limited || second.travel_limited,
        requested_travel: interpolate_scalar(first.requested_travel, second.requested_travel),
        solved_travel: interpolate_scalar(first.solved_travel, second.solved_travel),
        residuals: if first.residuals.max_link() >= second.residuals.max_link() {
            first.residuals
        } else {
            second.residuals
        },
        converged: first.converged && second.converged,
    }
}

impl SimplifiedSuspension {
    pub(super) fn build(
        geometry: &GeometricSuspensionConfig,
        prepared: &PreparedGeometricSuspension,
    ) -> Self {
        Self {
            corners: std::array::from_fn(|index| {
                let wheel = WheelIndex::ALL[index];
                let envelope = prepared.for_wheel(wheel);
                let samples = (0..TRAVEL_SAMPLE_COUNT)
                    .map(|index| {
                        let travel = envelope.q_min
                            + (envelope.q_max - envelope.q_min) * index as f64
                                / (TRAVEL_SAMPLE_COUNT - 1) as f64;
                        let solution = solve_corner(
                            geometry.corners.get(wheel),
                            travel,
                            0.0,
                            wheel.is_front(),
                            0.0,
                            0.0,
                            1.0,
                        )?;
                        let motion_ratio =
                            jacobian(geometry.corners.get(wheel), travel, 0.0, wheel.is_front())?
                                .motion_ratio;
                        if !motion_ratio.is_finite()
                            || motion_ratio <= 0.0
                            || !solution.converged
                            || solution.rocker_clamped
                            || solution.steering_clamped
                        {
                            return None;
                        }
                        let steering_responses = (0..STEERING_SAMPLE_COUNT)
                            .map(|index| {
                                let steering = -MAXIMUM_STEERING_RADIANS
                                    + 2.0 * MAXIMUM_STEERING_RADIANS * index as f64
                                        / (STEERING_SAMPLE_COUNT - 1) as f64;
                                prepare_response(&frame_at(geometry, wheel, &solution, steering))
                            })
                            .collect::<Option<Vec<_>>>()?;
                        Some(PreparedTravelSample {
                            solution,
                            motion_ratio,
                            steering_responses,
                        })
                    })
                    .collect();
                PreparedCorner {
                    minimum_travel_metres: envelope.q_min,
                    maximum_travel_metres: envelope.q_max,
                    samples,
                }
            }),
        }
    }

    fn bracket(
        &self,
        wheel: WheelIndex,
        travel_metres: f64,
    ) -> Option<(&PreparedTravelSample, &PreparedTravelSample, f64)> {
        let corner = &self.corners[wheel as usize];
        let (index, fraction) = interpolation_position(
            travel_metres,
            corner.minimum_travel_metres,
            corner.maximum_travel_metres,
            TRAVEL_SAMPLE_COUNT,
        )?;
        Some((
            corner.samples[index].as_ref()?,
            corner.samples[index + 1].as_ref()?,
            fraction,
        ))
    }

    pub(super) fn geometry_at(
        &self,
        wheel: WheelIndex,
        travel_metres: f64,
    ) -> Option<(KinematicSolution, f64)> {
        let (first, second, fraction) = self.bracket(wheel, travel_metres)?;
        Some((
            interpolate_solution(first.solution, second.solution, fraction),
            first.motion_ratio + (second.motion_ratio - first.motion_ratio) * fraction,
        ))
    }

    pub(super) fn forces_at(
        &self,
        geometry: &GeometricSuspensionConfig,
        wheel: WheelIndex,
        travel_metres: f64,
        velocity_metres_per_second: f64,
    ) -> Option<(
        f64,
        f64,
        f64,
        f64,
        f64,
        bool,
        f64,
        Option<KinematicSolution>,
    )> {
        let (solution, motion_ratio) = self.geometry_at(wheel, travel_metres)?;
        let velocity_metres_per_second = if velocity_metres_per_second.is_finite() {
            velocity_metres_per_second
        } else {
            0.0
        };
        let axle = geometry.axle(wheel);
        let deflection = axle.spring_free_length_m - axle.spring_installed_length_m
            + solution.damper_compression;
        let spring = axle.spring_rate_N_per_m * deflection.max(0.0);
        let damping = geometric_damper_element(axle, motion_ratio * velocity_metres_per_second);
        let (rod_force, singular) =
            rod_force_from_solution(geometry.corners.get(wheel), &solution, spring + damping);
        Some((
            spring * motion_ratio,
            damping * motion_ratio,
            solution.damper_compression,
            motion_ratio,
            rod_force,
            singular || deflection <= 0.0,
            solution.residuals.max_link().max(solution.residuals.hub_y),
            Some(solution),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn wheel_wrench(
        &self,
        geometry: &GeometricSuspensionConfig,
        wheel: WheelIndex,
        travel_metres: f64,
        steering_radians: f64,
        contact_force_world: Vec3,
        contact_point_world: Vec3,
        aligning_couple_world: Vec3,
        grounded: bool,
        unsprung_mass_kilograms: f64,
        unsprung_acceleration_metres_per_second_squared: f64,
        spring_force_newtons: f64,
        damping_force_newtons: f64,
        auxiliary_force_newtons: f64,
        motion_ratio: f64,
        rod_reference_newtons: f64,
        body: Transform3D,
        centre_of_gravity_local: Vec3,
    ) -> Option<GeoWheelWrench> {
        let (first, second, travel_fraction) = self.bracket(wheel, travel_metres)?;
        let (steering_index, steering_fraction) = interpolation_position(
            steering_radians,
            -MAXIMUM_STEERING_RADIANS,
            MAXIMUM_STEERING_RADIANS,
            STEERING_SAMPLE_COUNT,
        )?;
        let response = std::array::from_fn(|component| {
            std::array::from_fn(|link| {
                let first_value = first.steering_responses[steering_index][component][link]
                    * (1.0 - steering_fraction)
                    + first.steering_responses[steering_index + 1][component][link]
                        * steering_fraction;
                let second_value = second.steering_responses[steering_index][component][link]
                    * (1.0 - steering_fraction)
                    + second.steering_responses[steering_index + 1][component][link]
                        * steering_fraction;
                first_value + (second_value - first_value) * travel_fraction
            })
        });
        let solution = interpolate_solution(first.solution, second.solution, travel_fraction);
        let frame = frame_at(geometry, wheel, &solution, steering_radians);
        let corner = geometry.corners.get(wheel);
        let (contact_force, contact_point, aligning_couple) = if grounded {
            (
                body.basis.inverse_transform_vector(contact_force_world),
                body.inverse_transform_point(contact_point_world),
                body.basis.inverse_transform_vector(aligning_couple_world),
            )
        } else {
            (Vec3::ZERO, solution.hub, Vec3::ZERO)
        };
        let element_force = spring_force_newtons + damping_force_newtons + auxiliary_force_newtons;
        let element_force = if motion_ratio.is_finite() && motion_ratio > 0.05 {
            element_force / motion_ratio
        } else {
            element_force
        };
        let (reactions, paths) = reactions_with_prepared_response(
            &frame,
            contact_force,
            contact_point,
            aligning_couple,
            unsprung_mass_kilograms.max(0.0),
            if unsprung_acceleration_metres_per_second_squared.is_finite() {
                unsprung_acceleration_metres_per_second_squared
            } else {
                0.0
            },
            element_force,
            solution.damper_end - corner.damper_chassis,
            corner.rocker_pivot,
            corner.damper_chassis,
            rod_reference_newtons,
            Some(&response),
        );
        let (force, torque) = anchor_wrench(&reactions, centre_of_gravity_local);
        let mut element_vertical = 0.0;
        let mut wishbone_vertical = 0.0;
        let mut other_vertical = 0.0;
        for reaction in reactions {
            match reaction.kind {
                0..=3 => wishbone_vertical += reaction.force.y,
                4 | 5 => other_vertical += reaction.force.y,
                _ => element_vertical += reaction.force.y,
            }
        }
        Some(GeoWheelWrench {
            force_world: body.basis.transform_vector(force),
            torque_world: body.basis.transform_vector(torque),
            paths,
            elem_y_n: element_vertical,
            wish_y_n: wishbone_vertical,
            other_y_n: other_vertical,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suspension::{rest_compression_m, SuspensionSystem};
    use crate::types::{RaycastHit, TriRaycastSample};
    use crate::vehicle_config::VehicleConfig;
    use crate::vehicle_configuration::load_vehicle_configuration_from_authorized_root;
    use std::sync::OnceLock;

    fn configuration() -> VehicleConfig {
        let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        load_vehicle_configuration_from_authorized_root(
            &repository.join("game/data/vehicles/f1_2030/profiles/geometric/vehicle.json"),
            &repository,
        )
        .unwrap()
    }

    fn prepared(configuration: &VehicleConfig) -> &'static SimplifiedSuspension {
        static PREPARED: OnceLock<SimplifiedSuspension> = OnceLock::new();
        PREPARED.get_or_init(|| {
            let geometry = configuration
                .suspension
                .geometric_suspension
                .as_ref()
                .unwrap();
            SimplifiedSuspension::build(geometry, &PreparedGeometricSuspension::build(geometry))
        })
    }

    #[test]
    fn interpolated_geometry_and_elements_preserve_reference() {
        let configuration = configuration();
        let geometry = configuration
            .suspension
            .geometric_suspension
            .as_ref()
            .unwrap();
        let prepared = prepared(&configuration);
        let mut evaluated = 0;
        for wheel in WheelIndex::ALL {
            let corner = &prepared.corners[wheel as usize];
            for index in 0..301 {
                let travel = corner.minimum_travel_metres
                    + (corner.maximum_travel_metres - corner.minimum_travel_metres)
                        * (index as f64 + 0.37)
                        / 302.0;
                let Some((solution, ratio)) = prepared.geometry_at(wheel, travel) else {
                    continue;
                };
                let reference = solve_corner(
                    geometry.corners.get(wheel),
                    travel,
                    0.0,
                    wheel.is_front(),
                    0.0,
                    0.0,
                    1.0,
                )
                .unwrap();
                let reference_ratio =
                    jacobian(geometry.corners.get(wheel), travel, 0.0, wheel.is_front())
                        .unwrap()
                        .motion_ratio;
                assert!((solution.hub - reference.hub).length() < 0.0001);
                assert!((solution.upright_basis.x - reference.upright_basis.x).length() < 0.0001);
                assert!((solution.upright_basis.z - reference.upright_basis.z).length() < 0.0001);
                assert!(
                    (solution.damper_compression - reference.damper_compression).abs() < 0.00002
                );
                assert!((ratio - reference_ratio).abs() < 0.005);
                for velocity in [-1.0, -0.1, 0.0, 0.1, 1.0] {
                    let result = prepared
                        .forces_at(geometry, wheel, travel, velocity)
                        .unwrap();
                    let expected =
                        super::super::geometric_wheel_forces(geometry, wheel, travel, velocity);
                    assert!((result.0 - expected.0).abs() < 10.0 + expected.0.abs() * 0.005);
                    assert!((result.1 - expected.1).abs() < 10.0 + expected.1.abs() * 0.005);
                    assert!(result.1 * velocity >= 0.0);
                }
                evaluated += 1;
            }
        }
        assert!(
            evaluated > 1000,
            "Insufficient interpolation coverage: {evaluated}"
        );
    }

    #[test]
    fn interpolated_anchor_wrench_preserves_grounded_and_airborne_loads() {
        let configuration = configuration();
        let geometry = configuration
            .suspension
            .geometric_suspension
            .as_ref()
            .unwrap();
        let prepared = prepared(&configuration);
        let mut evaluated = 0;
        let mut accepted = 0;
        let mut maximum_force_error: f64 = 0.0;
        let mut maximum_torque_error: f64 = 0.0;
        for wheel in WheelIndex::ALL {
            for travel in [-0.02, -0.005, 0.0, 0.007, 0.025] {
                for steering in [-0.37, -0.07, 0.0, 0.13, 0.43] {
                    for grounded in [true, false] {
                        let force = Vec3::new(650.0, 4500.0, -1700.0);
                        let point = geometry.corners.get(wheel).hub_center - Vec3::UP * 0.33;
                        let body = Transform3D::new(
                            Vec3::new(2.0, 0.4, -5.0),
                            Mat3::from_euler_yxz(0.4, 0.04, -0.03),
                        );
                        let centre_of_gravity = Vec3::new(0.0, 0.15, -0.1);
                        let Some(result) = prepared.wheel_wrench(
                            geometry,
                            wheel,
                            travel,
                            steering,
                            body.basis.transform_vector(force),
                            body.transform_point(point),
                            Vec3::UP * 27.0,
                            grounded,
                            25.0,
                            3.0,
                            3500.0,
                            -300.0,
                            120.0,
                            0.8,
                            -4000.0,
                            body,
                            centre_of_gravity,
                        ) else {
                            continue;
                        };
                        let reference = crate::suspension_loads::geometric_wheel_wrench(
                            geometry,
                            wheel,
                            travel,
                            steering,
                            body.basis.transform_vector(force),
                            body.transform_point(point),
                            Vec3::UP * 27.0,
                            grounded,
                            25.0,
                            3.0,
                            3500.0,
                            -300.0,
                            120.0,
                            0.8,
                            -4000.0,
                            body,
                            centre_of_gravity,
                        );
                        let force_error = (result.force_world - reference.force_world).length();
                        let torque_error = (result.torque_world - reference.torque_world).length();
                        maximum_force_error = maximum_force_error.max(force_error);
                        maximum_torque_error = maximum_torque_error.max(torque_error);
                        assert!(
                            force_error < 2.0,
                            "force error {force_error} wheel={wheel:?}"
                        );
                        assert!(
                            torque_error < 5.0,
                            "torque error {torque_error} wheel={wheel:?}"
                        );
                        assert!(result.paths.balance_residual_n.is_finite());
                        accepted += usize::from(
                            result.paths.balance_residual_n <= 1.0 && !result.paths.singular,
                        );
                        evaluated += 1;
                    }
                }
            }
        }
        println!("suspension wrench interpolation: evaluated={evaluated} accepted={accepted} maximum_force_error_newtons={maximum_force_error} maximum_torque_error_newton_metres={maximum_torque_error}");
        assert!(evaluated >= 150);
        assert!(accepted * 100 >= evaluated * 90);
        assert!(prepared
            .geometry_at(WheelIndex::FrontLeft, f64::NAN)
            .is_none());
        assert!(prepared.geometry_at(WheelIndex::FrontLeft, 10.0).is_none());
        let nonfinite_velocity = prepared
            .forces_at(geometry, WheelIndex::FrontLeft, 0.0, f64::NAN)
            .unwrap();
        assert_eq!(nonfinite_velocity.1, 0.0);
    }

    #[test]
    fn twelve_contact_rays_and_vertical_dynamics_remain_consistent() {
        let configuration = configuration();
        let mut standard = SuspensionSystem::new(&configuration);
        standard.select_backend(&configuration, SuspensionBackend::Standard);
        let mut simplified = standard.clone();
        simplified.backend = SuspensionBackend::Simplified;
        simplified.simplified_prepared =
            Some(std::sync::Arc::new(prepared(&configuration).clone()));
        let mut maximum_force_difference: f64 = 0.0;
        let mut standard_geometry_solves = 0;
        let mut simplified_geometry_solves = 0;
        for tick in 0..1200 {
            let samples = std::array::from_fn(|index| {
                let wheel = WheelIndex::ALL[index];
                let radius = crate::wheel_mechanics::tire_radius(&configuration, wheel);
                let spring_length = super::super::spring_length(&configuration, wheel);
                let baseline =
                    spring_length + radius - rest_compression_m(&configuration, wheel) - 0.004;
                let road_offset = if tick > 240 {
                    (tick as f64 * 0.035 + index as f64 * 0.4).sin() * 0.006
                } else {
                    0.0
                };
                let hit = |offset: f64, colliding: bool| RaycastHit {
                    distance: baseline + road_offset + offset,
                    is_colliding: colliding,
                    normal: Vec3::UP,
                    ..RaycastHit::default()
                };
                TriRaycastSample {
                    inner: hit(-0.002, tick % 300 < 260),
                    center: hit(0.0, tick % 300 < 280),
                    outer: hit(0.002, tick % 300 < 270),
                }
            });
            crate::suspension_kinematics::counters::reset();
            standard.step_with_integration_method(&configuration,
                &[crate::tire_thermals::TireMechanicalModifiers::identity(); 4], &samples, 1.0 / 120.0, true);
            standard_geometry_solves +=
                crate::suspension_kinematics::counters::solve_corner_calls();
            crate::suspension_kinematics::counters::reset();
            simplified.step_with_integration_method(&configuration,
                &[crate::tire_thermals::TireMechanicalModifiers::identity(); 4], &samples, 1.0 / 120.0, true);
            simplified_geometry_solves +=
                crate::suspension_kinematics::counters::solve_corner_calls();
            for index in 0..4 {
                let reference = &standard.wheels[index];
                let result = &simplified.wheels[index];
                assert_eq!(reference.ray_grounded, result.ray_grounded);
                assert_eq!(reference.contact_fraction, result.contact_fraction);
                assert!(result.total_normal_force.is_finite());
                assert!(
                    (reference.suspension_compression_m - result.suspension_compression_m).abs()
                        < 0.0002
                );
                let difference = (reference.total_normal_force - result.total_normal_force).abs();
                maximum_force_difference = maximum_force_difference.max(difference);
                assert!(difference < 30.0);
                assert!(result.damper_dissipated_j >= 0.0);
            }
        }
        println!(
            "suspension dynamic parity maximum_force_difference_newtons={maximum_force_difference}"
        );
        println!("suspension dynamic geometry solves standard={standard_geometry_solves} simplified={simplified_geometry_solves}");
        assert!(simplified_geometry_solves * 5 < standard_geometry_solves);
        let serialized = serde_json::to_string(&simplified).unwrap();
        let mut restored: SuspensionSystem = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored.backend, SuspensionBackend::Simplified);
        restored.step(
            &configuration,
            &[TriRaycastSample::default(); 4],
            1.0 / 120.0,
        );
        assert!(restored.simplified_prepared.is_some());
    }
}
