#[derive(Clone, Copy)]
pub(super) struct VerticalTireContact {
    pub road_compression_metres: f64,
    pub road_velocity_metres_per_second: f64,
    pub stiffness_newtons_per_metre: f64,
    pub damping_newton_seconds_per_metre: f64,
    pub maximum_deflection_metres: f64,
    pub maximum_force_newtons: f64,
}

pub(super) struct VerticalTireResponse {
    pub force_newtons: f64,
    pub deflection_metres: f64,
    pub deflection_velocity_metres_per_second: f64,
}

impl VerticalTireContact {
    pub fn evaluate(
        &self,
        compression_metres: f64,
        velocity_metres_per_second: f64,
    ) -> VerticalTireResponse {
        let raw_deflection_metres = (self.road_compression_metres - compression_metres).max(0.0);
        let deflection_metres = raw_deflection_metres.min(self.maximum_deflection_metres);
        let deflection_velocity_metres_per_second = if raw_deflection_metres > 0.0 {
            self.road_velocity_metres_per_second - velocity_metres_per_second
        } else {
            0.0
        };
        let force_newtons = if raw_deflection_metres > 0.0 {
            self.stiffness_newtons_per_metre * deflection_metres
                + self.damping_newton_seconds_per_metre * deflection_velocity_metres_per_second
                + self.stiffness_newtons_per_metre
                    * (raw_deflection_metres - self.maximum_deflection_metres).max(0.0)
                    * 8.0
        } else {
            0.0
        };
        VerticalTireResponse {
            force_newtons: force_newtons.clamp(0.0, self.maximum_force_newtons),
            deflection_metres,
            deflection_velocity_metres_per_second,
        }
    }
}

pub(super) struct VerticalSuspensionStep {
    pub compression_metres: f64,
    pub velocity_metres_per_second: f64,
    pub tire_impulse_newton_seconds: f64,
    pub suspension_impulse_newton_seconds: f64,
    pub travel_limit_impulse_newton_seconds: f64,
}

pub(super) fn integrate_vertical_suspension(
    use_implicit_integration: bool,
    compression_metres: f64,
    velocity_metres_per_second: f64,
    mass_kilograms: f64,
    duration_seconds: f64,
    compression_bounds_metres: (f64, f64),
    force_at: impl Fn(f64, f64) -> (f64, f64),
) -> VerticalSuspensionStep {
    if !use_implicit_integration {
        let (tire_force_newtons, suspension_force_newtons) =
            force_at(compression_metres, velocity_metres_per_second);
        let mut next_velocity_metres_per_second = velocity_metres_per_second
            + (tire_force_newtons - suspension_force_newtons) / mass_kilograms * duration_seconds;
        let mut next_compression_metres =
            compression_metres + next_velocity_metres_per_second * duration_seconds;
        if next_compression_metres < compression_bounds_metres.0 {
            next_compression_metres = compression_bounds_metres.0;
            next_velocity_metres_per_second = next_velocity_metres_per_second.max(0.0);
        } else if next_compression_metres > compression_bounds_metres.1 {
            next_compression_metres = compression_bounds_metres.1;
            next_velocity_metres_per_second = next_velocity_metres_per_second.min(0.0);
        }
        return VerticalSuspensionStep {
            compression_metres: next_compression_metres,
            velocity_metres_per_second: next_velocity_metres_per_second,
            tire_impulse_newton_seconds: tire_force_newtons * duration_seconds,
            suspension_impulse_newton_seconds: suspension_force_newtons * duration_seconds,
            travel_limit_impulse_newton_seconds: mass_kilograms
                * (next_velocity_metres_per_second - velocity_metres_per_second)
                - duration_seconds * (tire_force_newtons - suspension_force_newtons),
        };
    }
    let momentum_residual = |next_velocity_metres_per_second: f64| {
        let next_compression_metres =
            compression_metres + duration_seconds * next_velocity_metres_per_second;
        let (tire_force_newtons, suspension_force_newtons) =
            force_at(next_compression_metres, next_velocity_metres_per_second);
        mass_kilograms * (next_velocity_metres_per_second - velocity_metres_per_second)
            - duration_seconds * (tire_force_newtons - suspension_force_newtons)
    };
    let mut lower_velocity_metres_per_second =
        (compression_bounds_metres.0 - compression_metres) / duration_seconds;
    let mut upper_velocity_metres_per_second =
        (compression_bounds_metres.1 - compression_metres) / duration_seconds;
    let mut next_velocity_metres_per_second;
    let constrained = if momentum_residual(lower_velocity_metres_per_second) >= 0.0 {
        next_velocity_metres_per_second = lower_velocity_metres_per_second;
        true
    } else if momentum_residual(upper_velocity_metres_per_second) <= 0.0 {
        next_velocity_metres_per_second = upper_velocity_metres_per_second;
        true
    } else {
        next_velocity_metres_per_second = velocity_metres_per_second.clamp(
            lower_velocity_metres_per_second,
            upper_velocity_metres_per_second,
        );
        for _ in 0..48 {
            let residual_newton_seconds = momentum_residual(next_velocity_metres_per_second);
            if residual_newton_seconds.abs() <= 1e-9 {
                break;
            }
            if residual_newton_seconds > 0.0 {
                upper_velocity_metres_per_second = next_velocity_metres_per_second;
            } else {
                lower_velocity_metres_per_second = next_velocity_metres_per_second;
            }
            next_velocity_metres_per_second =
                0.5 * (lower_velocity_metres_per_second + upper_velocity_metres_per_second);
        }
        false
    };
    let next_compression_metres =
        compression_metres + duration_seconds * next_velocity_metres_per_second;
    let (tire_force_newtons, suspension_force_newtons) =
        force_at(next_compression_metres, next_velocity_metres_per_second);
    VerticalSuspensionStep {
        compression_metres: next_compression_metres,
        velocity_metres_per_second: next_velocity_metres_per_second,
        tire_impulse_newton_seconds: tire_force_newtons * duration_seconds,
        suspension_impulse_newton_seconds: suspension_force_newtons * duration_seconds,
        travel_limit_impulse_newton_seconds: if constrained {
            momentum_residual(next_velocity_metres_per_second)
        } else {
            0.0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::integrate_vertical_suspension;

    #[test]
    fn stiff_damped_mode_dissipates_energy_at_every_substep() {
        let mass_kilograms = 15.0;
        let stiffness_newtons_per_metre = 14_000_000.0;
        let damping_newton_seconds_per_metre = 16_000.0;
        let mut compression_metres = 0.01;
        let mut velocity_metres_per_second = 0.0;
        let mut previous_energy_joules =
            0.5 * stiffness_newtons_per_metre * compression_metres * compression_metres;
        for _ in 0..480 {
            let result = integrate_vertical_suspension(
                true,
                compression_metres,
                velocity_metres_per_second,
                mass_kilograms,
                1.0 / 480.0,
                (-0.1, 0.1),
                |compression, velocity| {
                    (
                        0.0,
                        stiffness_newtons_per_metre * compression
                            + damping_newton_seconds_per_metre * velocity,
                    )
                },
            );
            let energy_joules =
                0.5 * stiffness_newtons_per_metre * result.compression_metres.powi(2)
                    + 0.5 * mass_kilograms * result.velocity_metres_per_second.powi(2);
            assert!(energy_joules <= previous_energy_joules + 1e-10);
            compression_metres = result.compression_metres;
            velocity_metres_per_second = result.velocity_metres_per_second;
            previous_energy_joules = energy_joules;
        }
        assert!(previous_energy_joules < 1e-12);
    }

    #[test]
    fn integrated_forces_close_virtual_wheel_momentum_including_travel_limits() {
        for compression_bounds_metres in [(-1.0, 1.0), (0.0, 0.00001)] {
            let result = integrate_vertical_suspension(
                true,
                0.0,
                0.2,
                18.0,
                1.0 / 480.0,
                compression_bounds_metres,
                |compression, velocity| (3000.0, 400_000.0 * compression + 2000.0 * velocity),
            );
            let momentum_change_newton_seconds = 18.0 * (result.velocity_metres_per_second - 0.2);
            let impulse_newton_seconds = result.tire_impulse_newton_seconds
                - result.suspension_impulse_newton_seconds
                + result.travel_limit_impulse_newton_seconds;
            assert!((momentum_change_newton_seconds - impulse_newton_seconds).abs() < 2e-9);
            assert!(result.compression_metres >= compression_bounds_metres.0 - 1e-12);
            assert!(result.compression_metres <= compression_bounds_metres.1 + 1e-12);
        }
    }

    #[test]
    fn transient_contact_transfers_the_integrated_impulse_instead_of_the_last_force() {
        let contact = super::VerticalTireContact {
            road_compression_metres: 0.02,
            road_velocity_metres_per_second: 0.0,
            stiffness_newtons_per_metre: 200_000.0,
            damping_newton_seconds_per_metre: 1000.0,
            maximum_deflection_metres: 0.04,
            maximum_force_newtons: 20_000.0,
        };
        let mut compression_metres = 0.0;
        let mut velocity_metres_per_second = 0.0;
        let mut normal_impulse_newton_seconds = 0.0;
        for _ in 0..4 {
            let result = integrate_vertical_suspension(
                true,
                compression_metres,
                velocity_metres_per_second,
                18.0,
                1.0 / 480.0,
                (-0.1, 0.1),
                |compression, velocity| {
                    (
                        contact.evaluate(compression, velocity).force_newtons,
                        100_000.0 * compression + 7000.0 * velocity,
                    )
                },
            );
            compression_metres = result.compression_metres;
            velocity_metres_per_second = result.velocity_metres_per_second;
            normal_impulse_newton_seconds += result.tire_impulse_newton_seconds;
        }
        let last_force_newtons = contact
            .evaluate(compression_metres, velocity_metres_per_second)
            .force_newtons;
        assert!((normal_impulse_newton_seconds - last_force_newtons / 120.0).abs() > 0.1);
    }
}
