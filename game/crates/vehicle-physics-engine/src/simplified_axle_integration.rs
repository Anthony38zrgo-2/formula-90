use super::vertical_suspension_integration::{
    integrate_vertical_suspension, VerticalSuspensionStep, VerticalTireContact,
};
use super::*;

#[derive(Clone, Copy)]
pub(super) struct GeometricAxleCornerIntegration {
    pub initial_spring_compression_metres: f64,
    pub initial_motion_ratio: f64,
    pub initial_geometry_invalid: bool,
    pub compression_metres: f64,
    pub velocity_metres_per_second: f64,
    pub normal_impulse_newton_seconds: f64,
    pub suspension_impulse_newton_seconds: f64,
    pub travel_limit_impulse_newton_seconds: f64,
    pub damper_dissipated_joules: f64,
    pub clamped: bool,
    pub antiroll_force_newtons: f64,
    pub final_normal_force_newtons: f64,
    pub final_tire_deflection_metres: f64,
    pub final_tire_velocity_metres_per_second: f64,
    pub force_application_compression_metres: f64,
}

struct PreparedAxleCorner {
    wheel: WheelIndex,
    reference_compression_metres: f64,
    initial_compression_metres: f64,
    initial_velocity_metres_per_second: f64,
    spring_compression_metres: f64,
    motion_ratio: f64,
    geometry_invalid: bool,
    envelope: PreparedWheelEnvelope,
    tuning: WheelMechanicalTuning,
    contact: SimplifiedNormalContact,
}

impl PreparedAxleCorner {
    fn element_forces(
        &self,
        geometry: &GeometricSuspensionConfig,
        compression_metres: f64,
        velocity_metres_per_second: f64,
    ) -> (f64, f64, f64) {
        let spring_compression_metres = self.spring_compression_metres
            + self.motion_ratio * (compression_metres - self.initial_compression_metres);
        if self.geometry_invalid {
            return (0.0, 0.0, self.spring_compression_metres);
        }
        let axle = geometry.axle(self.wheel);
        let total_deflection_metres =
            axle.spring_free_length_m - axle.spring_installed_length_m + spring_compression_metres;
        let spring_force_newtons =
            axle.spring_rate_N_per_m * total_deflection_metres.max(0.0) * self.motion_ratio;
        let damping_force_newtons =
            geometric_damper_element(axle, self.motion_ratio * velocity_metres_per_second)
                * self.motion_ratio;
        (
            spring_force_newtons,
            damping_force_newtons,
            spring_compression_metres,
        )
    }

    fn stop_force(
        &self,
        geometry: &GeometricSuspensionConfig,
        compression_metres: f64,
        spring_compression_metres: f64,
    ) -> (f64, bool) {
        geometric_stop_force(
            geometry,
            self.wheel,
            compression_metres,
            self.reference_compression_metres,
            self.envelope,
            spring_compression_metres,
            self.motion_ratio,
            self.tuning,
        )
    }

    fn compression_bounds_metres(&self) -> (f64, f64) {
        let bounds = geometric_travel_bounds(self.envelope, self.reference_compression_metres);
        (
            bounds.0 - self.tuning.hard_stop_overtravel_m,
            bounds.1 + self.tuning.hard_stop_overtravel_m,
        )
    }
}

fn integrate_implicit_axle(
    compressions_metres: [f64; 2],
    velocities_metres_per_second: [f64; 2],
    masses_kilograms: [f64; 2],
    duration_seconds: f64,
    compression_bounds_metres: [(f64, f64); 2],
    reference_compressions_metres: [f64; 2],
    antiroll_stiffness_newtons_per_metre: f64,
    force_at: impl Fn(usize, f64, f64) -> (f64, f64),
) -> [VerticalSuspensionStep; 2] {
    let bounds = std::array::from_fn::<_, 2, _>(|index| {
        (
            (compression_bounds_metres[index].0 - compressions_metres[index]) / duration_seconds,
            (compression_bounds_metres[index].1 - compressions_metres[index]) / duration_seconds,
        )
    });
    let corner_step = |index: usize, opposite_velocity_metres_per_second: f64| {
        let opposite_index = 1 - index;
        let opposite_travel_metres = compressions_metres[opposite_index]
            + duration_seconds * opposite_velocity_metres_per_second
            - reference_compressions_metres[opposite_index];
        integrate_vertical_suspension(
            true,
            compressions_metres[index],
            velocities_metres_per_second[index],
            masses_kilograms[index],
            duration_seconds,
            compression_bounds_metres[index],
            |compression_metres, velocity_metres_per_second| {
                let (tire_force_newtons, suspension_force_newtons) =
                    force_at(index, compression_metres, velocity_metres_per_second);
                let antiroll_force_newtons = antiroll_stiffness_newtons_per_metre
                    * (compression_metres
                        - reference_compressions_metres[index]
                        - opposite_travel_metres);
                (
                    tire_force_newtons,
                    suspension_force_newtons + antiroll_force_newtons,
                )
            },
        )
    };
    let residual_at = |index: usize, candidate_velocities: [f64; 2]| {
        let compression_metres =
            compressions_metres[index] + duration_seconds * candidate_velocities[index];
        let opposite_index = 1 - index;
        let opposite_compression_metres = compressions_metres[opposite_index]
            + duration_seconds * candidate_velocities[opposite_index];
        let (tire_force_newtons, suspension_force_newtons) =
            force_at(index, compression_metres, candidate_velocities[index]);
        let antiroll_force_newtons = antiroll_stiffness_newtons_per_metre
            * (compression_metres
                - reference_compressions_metres[index]
                - opposite_compression_metres
                + reference_compressions_metres[opposite_index]);
        masses_kilograms[index]
            * (candidate_velocities[index] - velocities_metres_per_second[index])
            - duration_seconds
                * (tire_force_newtons - suspension_force_newtons - antiroll_force_newtons)
    };
    let projected_residual = |index: usize, candidate_velocities: [f64; 2]| {
        let residual = residual_at(index, candidate_velocities);
        if (candidate_velocities[index] <= bounds[index].0 && residual >= 0.0)
            || (candidate_velocities[index] >= bounds[index].1 && residual <= 0.0)
        {
            0.0
        } else {
            residual
        }
    };
    let mut next_velocities = std::array::from_fn(|index| {
        velocities_metres_per_second[index].clamp(bounds[index].0, bounds[index].1)
    });
    let mut converged = false;
    for _ in 0..32 {
        next_velocities[0] = corner_step(0, next_velocities[1]).velocity_metres_per_second;
        next_velocities[1] = corner_step(1, next_velocities[0]).velocity_metres_per_second;
        if projected_residual(0, next_velocities)
            .abs()
            .max(projected_residual(1, next_velocities).abs())
            <= 2e-9
        {
            converged = true;
            break;
        }
    }
    if !converged {
        let left_step = integrate_vertical_suspension(
            true,
            compressions_metres[0],
            velocities_metres_per_second[0],
            masses_kilograms[0],
            duration_seconds,
            compression_bounds_metres[0],
            |compression_metres, velocity_metres_per_second| {
                let opposite_step = corner_step(1, velocity_metres_per_second);
                let (tire_force_newtons, suspension_force_newtons) =
                    force_at(0, compression_metres, velocity_metres_per_second);
                let antiroll_force_newtons = antiroll_stiffness_newtons_per_metre
                    * (compression_metres
                        - reference_compressions_metres[0]
                        - opposite_step.compression_metres
                        + reference_compressions_metres[1]);
                (
                    tire_force_newtons,
                    suspension_force_newtons + antiroll_force_newtons,
                )
            },
        );
        next_velocities[0] = left_step.velocity_metres_per_second;
        next_velocities[1] = corner_step(1, next_velocities[0]).velocity_metres_per_second;
    }
    let next_compressions = std::array::from_fn::<_, 2, _>(|index| {
        compressions_metres[index] + duration_seconds * next_velocities[index]
    });
    let antiroll_force_newtons = antiroll_stiffness_newtons_per_metre
        * (next_compressions[0] - reference_compressions_metres[0] - next_compressions[1]
            + reference_compressions_metres[1]);
    std::array::from_fn(|index| {
        let (tire_force_newtons, suspension_force_newtons) =
            force_at(index, next_compressions[index], next_velocities[index]);
        let suspension_force_newtons = suspension_force_newtons
            + if index == 0 {
                antiroll_force_newtons
            } else {
                -antiroll_force_newtons
            };
        let residual = masses_kilograms[index]
            * (next_velocities[index] - velocities_metres_per_second[index])
            - duration_seconds * (tire_force_newtons - suspension_force_newtons);
        let constrained = (next_velocities[index] <= bounds[index].0 && residual >= 0.0)
            || (next_velocities[index] >= bounds[index].1 && residual <= 0.0);
        VerticalSuspensionStep {
            compression_metres: next_compressions[index],
            velocity_metres_per_second: next_velocities[index],
            tire_impulse_newton_seconds: tire_force_newtons * duration_seconds,
            suspension_impulse_newton_seconds: suspension_force_newtons * duration_seconds,
            travel_limit_impulse_newton_seconds: if constrained { residual } else { 0.0 },
        }
    })
}

impl SuspensionSystem {
    fn prepare_simplified_axle_corner(
        &self,
        configuration: &VehicleConfig,
        wheel: WheelIndex,
        modifiers: TireMechanicalModifiers,
    ) -> PreparedAxleCorner {
        let state = &self.wheels[wheel as usize];
        let reference_compression_metres = rest_compression_m(configuration, wheel);
        let travel_metres = state.suspension_compression_m - reference_compression_metres;
        let prepared_pose = self
            .simplified_prepared
            .as_ref()
            .and_then(|prepared| prepared.vertical_reference_at(wheel, travel_metres));
        if let Some(counts) = &self.benchmark_evaluation_counts {
            counts[0].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if prepared_pose.is_some() {
                counts[1].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        let (spring_compression_metres, motion_ratio) = prepared_pose
            .expect("Equivalent suspension parameters and travel must be valid");
        let geometry_invalid = false;
        let tuning = WheelMechanicalTuning::for_wheel(configuration, wheel);
        PreparedAxleCorner {
            wheel,
            reference_compression_metres,
            initial_compression_metres: state.suspension_compression_m,
            initial_velocity_metres_per_second: state.unsprung_velocity_m_s,
            spring_compression_metres,
            motion_ratio,
            geometry_invalid,
            envelope: self.geometric_prepared.as_ref().unwrap().for_wheel(wheel),
            tuning,
            contact: SimplifiedNormalContact {
                radial_contact: VerticalTireContact {
                    road_compression_metres: state.normal_road_compression_metres,
                    road_velocity_metres_per_second: state.normal_road_velocity_metres_per_second,
                    ..vertical_tire_contact(configuration, wheel, modifiers, state, tuning)
                },
                initial_normal_projection: state.normal_contact_projection,
                normal_projection_rate_per_second: state.normal_contact_projection_rate_per_second,
            },
        }
    }

    pub(super) fn simplified_antiroll_stiffness(
        &self,
        configuration: &VehicleConfig,
        wheels: [WheelIndex; 2],
    ) -> f64 {
        let geometry = configuration
            .suspension
            .geometric_suspension
            .as_ref()
            .unwrap();
        let wheel_rate = |wheel| {
            self.geometric_prepared
                .as_ref()
                .map(|prepared| prepared.for_wheel(wheel).k_wheel_rest_n_per_m)
                .unwrap_or_else(|| geometric_k_wheel_rest(geometry, wheel).max(1.0))
        };
        let axle_ratio = if wheels[0].is_front() {
            configuration.suspension.front_arb_ratio
        } else {
            configuration.suspension.rear_arb_ratio
        };
        let antiroll_configuration = self
            .backend
            .effective_antiroll_configuration(*geometry.arb(wheels[0].is_front()), axle_ratio);
        geometric_arb_force(
            &antiroll_configuration,
            1.0,
            0.0,
            0.5 * (wheel_rate(wheels[0]) + wheel_rate(wheels[1])),
        )
    }

    pub(super) fn solve_simplified_geometric_axle(
        &mut self,
        configuration: &VehicleConfig,
        modifiers: &[TireMechanicalModifiers; 4],
        wheels: [WheelIndex; 2],
        duration_seconds: f64,
    ) {
        let geometry = configuration
            .suspension
            .geometric_suspension
            .as_ref()
            .unwrap();
        self.ensure_geometric_prepared(configuration);
        let corners = wheels.map(|wheel| {
            self.prepare_simplified_axle_corner(configuration, wheel, modifiers[wheel as usize])
        });
        let antiroll_stiffness = self.simplified_antiroll_stiffness(configuration, wheels);
        let target_duration_seconds = corners
            .iter()
            .map(|corner| corner.tuning.mechanical_substep_s.max(1e-5))
            .fold(f64::INFINITY, f64::min);
        let substeps = ((duration_seconds / target_duration_seconds).ceil() as usize).clamp(1, 12);
        let substep_duration_seconds = duration_seconds / substeps as f64;
        let mut results = std::array::from_fn::<_, 2, _>(|index| GeometricAxleCornerIntegration {
            initial_spring_compression_metres: corners[index].spring_compression_metres,
            initial_motion_ratio: corners[index].motion_ratio,
            initial_geometry_invalid: corners[index].geometry_invalid,
            compression_metres: corners[index].initial_compression_metres,
            velocity_metres_per_second: corners[index].initial_velocity_metres_per_second,
            normal_impulse_newton_seconds: 0.0,
            suspension_impulse_newton_seconds: 0.0,
            travel_limit_impulse_newton_seconds: 0.0,
            damper_dissipated_joules: 0.0,
            clamped: corners[index].geometry_invalid,
            antiroll_force_newtons: 0.0,
            final_normal_force_newtons: 0.0,
            final_tire_deflection_metres: 0.0,
            final_tire_velocity_metres_per_second: 0.0,
            force_application_compression_metres: 0.0,
        });
        for substep_index in 0..substeps {
            let elapsed_seconds = (substep_index + 1) as f64 * substep_duration_seconds;
            let steps = integrate_implicit_axle(
                results.map(|result| result.compression_metres),
                results.map(|result| result.velocity_metres_per_second),
                std::array::from_fn(|index| corners[index].tuning.unsprung_mass_kg.max(1.0)),
                substep_duration_seconds,
                std::array::from_fn(|index| corners[index].compression_bounds_metres()),
                std::array::from_fn(|index| corners[index].reference_compression_metres),
                antiroll_stiffness,
                |index, compression_metres, velocity_metres_per_second| {
                    let corner = &corners[index];
                    let (spring_force, damping_force, spring_compression) = corner.element_forces(
                        geometry,
                        compression_metres,
                        velocity_metres_per_second,
                    );
                    (
                        corner
                            .contact
                            .evaluate(compression_metres, velocity_metres_per_second, elapsed_seconds)
                            .force_newtons * corner.contact.projection_at(elapsed_seconds),
                        spring_force
                            + damping_force
                            + corner
                                .stop_force(geometry, compression_metres, spring_compression)
                                .0,
                    )
                },
            );
            for (index, step) in steps.into_iter().enumerate() {
                let result = &mut results[index];
                let corner = &corners[index];
                result.compression_metres = step.compression_metres;
                result.velocity_metres_per_second = step.velocity_metres_per_second;
                let response = corner.contact.evaluate(step.compression_metres,
                    step.velocity_metres_per_second, elapsed_seconds);
                let normal_impulse = response.force_newtons * substep_duration_seconds;
                result.normal_impulse_newton_seconds += normal_impulse;
                result.force_application_compression_metres += step.compression_metres * normal_impulse;
                result.final_normal_force_newtons = response.force_newtons;
                result.final_tire_deflection_metres = response.deflection_metres;
                result.final_tire_velocity_metres_per_second = response.deflection_velocity_metres_per_second;
                result.suspension_impulse_newton_seconds += step.suspension_impulse_newton_seconds;
                result.travel_limit_impulse_newton_seconds +=
                    step.travel_limit_impulse_newton_seconds;
                let (_, damping_force, spring_compression) = corner.element_forces(
                    geometry,
                    step.compression_metres,
                    step.velocity_metres_per_second,
                );
                result.damper_dissipated_joules +=
                    (damping_force * step.velocity_metres_per_second * substep_duration_seconds)
                        .max(0.0);
                result.clamped |= corner
                    .stop_force(geometry, step.compression_metres, spring_compression)
                    .1;
            }
        }
        let antiroll_force = antiroll_stiffness
            * (results[0].compression_metres
                - corners[0].reference_compression_metres
                - results[1].compression_metres
                + corners[1].reference_compression_metres);
        results[0].antiroll_force_newtons = antiroll_force;
        results[1].antiroll_force_newtons = -antiroll_force;
        for index in 0..2 {
            results[index].force_application_compression_metres = if results[index].normal_impulse_newton_seconds > 0.0 {
                results[index].force_application_compression_metres / results[index].normal_impulse_newton_seconds
            } else { results[index].compression_metres };
            self.solve_force_geometric(
                configuration,
                wheels[index],
                modifiers[wheels[index] as usize],
                results[1 - index].compression_metres,
                duration_seconds,
                true,
                Some(&results[index]),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::integrate_implicit_axle;

    #[test]
    fn differential_mode_dissipates_energy_and_preserves_axle_momentum() {
        for stiffness in [20_000.0, 20_000_000.0] {
            let mut compressions: [f64; 2] = [0.01, -0.01];
            let mut velocities = [0.0, 0.0];
            let mut previous_energy = 0.5 * stiffness * (compressions[0] - compressions[1]).powi(2);
            for _ in 0..120 {
                let steps = integrate_implicit_axle(
                    compressions,
                    velocities,
                    [18.0; 2],
                    1.0 / 480.0,
                    [(-0.1, 0.1); 2],
                    [0.0; 2],
                    stiffness,
                    |_, _, _| (0.0, 0.0),
                );
                for index in 0..2 {
                    let momentum_change =
                        18.0 * (steps[index].velocity_metres_per_second - velocities[index]);
                    assert!(
                        (momentum_change + steps[index].suspension_impulse_newton_seconds).abs()
                            < 1e-7
                    );
                }
                assert!(
                    (steps[0].suspension_impulse_newton_seconds
                        + steps[1].suspension_impulse_newton_seconds)
                        .abs()
                        < 1e-12
                );
                compressions = steps.each_ref().map(|step| step.compression_metres);
                velocities = steps.each_ref().map(|step| step.velocity_metres_per_second);
                let energy = 0.5 * stiffness * (compressions[0] - compressions[1]).powi(2)
                    + 9.0 * (velocities[0].powi(2) + velocities[1].powi(2));
                assert!(
                    energy <= previous_energy + 1e-7,
                    "{stiffness}: {energy} > {previous_energy}"
                );
                previous_energy = energy;
            }
        }
    }

    #[test]
    fn common_motion_is_independent_of_bar_stiffness() {
        for stiffness in [0.0, 20_000.0, 20_000_000.0] {
            let steps = integrate_implicit_axle(
                [0.01; 2],
                [0.3; 2],
                [18.0; 2],
                1.0 / 120.0,
                [(-0.1, 0.1); 2],
                [0.0; 2],
                stiffness,
                |_, _, _| (0.0, 0.0),
            );
            for step in steps {
                assert!((step.velocity_metres_per_second - 0.3).abs() < 1e-8);
                assert!(step.suspension_impulse_newton_seconds.abs() < 1e-7);
            }
        }
    }

    #[test]
    fn unequal_masses_and_travel_limits_close_momentum() {
        let steps = integrate_implicit_axle(
            [0.0; 2],
            [0.2, -0.3],
            [15.0, 18.0],
            1.0 / 120.0,
            [(0.0, 0.00001), (-0.03, 0.02)],
            [0.0; 2],
            50_000.0,
            |index, compression, velocity| {
                (
                    if index == 0 { 3000.0 } else { 1000.0 },
                    400_000.0 * compression + 2000.0 * velocity,
                )
            },
        );
        for (index, step) in steps.into_iter().enumerate() {
            let momentum_change =
                [15.0, 18.0][index] * (step.velocity_metres_per_second - [0.2, -0.3][index]);
            let impulse = step.tire_impulse_newton_seconds - step.suspension_impulse_newton_seconds
                + step.travel_limit_impulse_newton_seconds;
            assert!((momentum_change - impulse).abs() < 1e-7);
        }
    }
}
