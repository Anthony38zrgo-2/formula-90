use super::vertical_suspension_integration::VerticalTireResponse;
use super::*;

#[derive(Clone, Copy)]
pub(super) struct SimplifiedNormalContact {
    pub radial_contact: VerticalTireContact,
    pub initial_normal_projection: f64,
    pub normal_projection_rate_per_second: f64,
}

impl SimplifiedNormalContact {
    pub fn projection_at(&self, elapsed_seconds: f64) -> f64 {
        (self.initial_normal_projection + self.normal_projection_rate_per_second * elapsed_seconds)
            .clamp(0.0, 1.0)
    }

    pub fn evaluate(
        &self,
        compression_metres: f64,
        velocity_metres_per_second: f64,
        elapsed_seconds: f64,
    ) -> VerticalTireResponse {
        let projection = self.projection_at(elapsed_seconds);
        let unconstrained_projection = self.initial_normal_projection
            + self.normal_projection_rate_per_second * elapsed_seconds;
        let projection_velocity = if (0.0..=1.0).contains(&unconstrained_projection) {
            self.normal_projection_rate_per_second
        } else {
            0.0
        };
        let contact = VerticalTireContact {
            road_compression_metres: self.radial_contact.road_compression_metres
                + self.radial_contact.road_velocity_metres_per_second * elapsed_seconds,
            road_velocity_metres_per_second: self.radial_contact.road_velocity_metres_per_second
                - projection_velocity * compression_metres,
            ..self.radial_contact
        };
        if projection <= 1e-6 {
            return VerticalTireResponse {
                force_newtons: 0.0,
                deflection_metres: 0.0,
                deflection_velocity_metres_per_second: 0.0,
            };
        }
        contact.evaluate(
            compression_metres * projection,
            velocity_metres_per_second * projection,
        )
    }
}

impl SuspensionSystem {
    pub(super) fn configure_simplified_contact_kinematics(
        &mut self,
        configuration: &VehicleConfig,
        body: crate::simulation::BodyKinematics,
    ) {
        let axis = body.transform.basis.y;
        let axis_velocity = body.angular_velocity.cross(axis);
        let center = crate::simulation::center_of_mass_world_for_suspension_backend(
            configuration,
            self.backend,
            &body.transform,
        );
        for wheel in WheelIndex::ALL {
            let state = &mut self.wheels[wheel as usize];
            if state.contact_fraction <= 0.0 {
                continue;
            }
            let normal = state.effective_normal.normalized();
            let anchor = configuration.tires.wheel_anchor_local(
                &configuration.chassis,
                &configuration.suspension,
                wheel,
            );
            let base =
                body.transform.transform_point(anchor) - axis * spring_length(configuration, wheel);
            state.normal_contact_projection = normal.dot(axis).clamp(0.0, 1.0);
            state.normal_contact_projection_rate_per_second = normal.dot(axis_velocity);
            state.normal_road_compression_metres = tire_radius(configuration, wheel)
                - normal.dot(base - state.effective_contact_point);
            state.normal_road_velocity_metres_per_second =
                -normal.dot(body.linear_velocity + body.angular_velocity.cross(base - center));
        }
    }

    pub(super) fn update_simplified_contact_application_points(
        &mut self,
        configuration: &VehicleConfig,
        body: crate::simulation::BodyKinematics,
    ) {
        let axis = body.transform.basis.y;
        for wheel in WheelIndex::ALL {
            let state = &mut self.wheels[wheel as usize];
            if state.contact_fraction <= 0.0 {
                continue;
            }
            let normal = state.effective_normal.normalized();
            let anchor = configuration.tires.wheel_anchor_local(
                &configuration.chassis,
                &configuration.suspension,
                wheel,
            );
            let hub = body.transform.transform_point(anchor)
                - axis
                    * (spring_length(configuration, wheel)
                        - state.normal_force_application_compression_metres);
            state.effective_contact_point =
                hub - normal * normal.dot(hub - state.effective_contact_point);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_force_is_the_gradient_of_radial_elastic_energy() {
        for angle_degrees in [0.0_f64, 20.0, 45.0, 70.0] {
            let projection = angle_degrees.to_radians().cos();
            let contact = SimplifiedNormalContact {
                radial_contact: VerticalTireContact {
                    road_compression_metres: 0.1 * projection,
                    road_velocity_metres_per_second: 0.0,
                    stiffness_newtons_per_metre: 120000.0,
                    damping_newton_seconds_per_metre: 0.0,
                    maximum_deflection_metres: 0.04,
                    maximum_force_newtons: 20000.0,
                },
                initial_normal_projection: projection,
                normal_projection_rate_per_second: 0.0,
            };
            let compression = 0.09;
            let energy = |coordinate: f64| {
                0.5 * 120000.0 * (0.1 * projection - coordinate * projection).powi(2)
            };
            let gradient = (energy(compression + 1e-7) - energy(compression - 1e-7)) / 2e-7;
            let force = contact.evaluate(compression, 0.0, 0.0).force_newtons;
            assert!((force * projection + gradient).abs() < 1e-5);
        }
    }

    #[test]
    fn projection_limit_does_not_create_a_deformation_velocity() {
        let contact = SimplifiedNormalContact {
            radial_contact: VerticalTireContact {
                road_compression_metres: 0.1,
                road_velocity_metres_per_second: 0.0,
                stiffness_newtons_per_metre: 120000.0,
                damping_newton_seconds_per_metre: 1000.0,
                maximum_deflection_metres: 0.04,
                maximum_force_newtons: 20000.0,
            },
            initial_normal_projection: 0.999,
            normal_projection_rate_per_second: 1.0,
        };
        let response = contact.evaluate(0.09, 0.0, 0.01);
        assert_eq!(response.deflection_velocity_metres_per_second, 0.0);
    }

    #[test]
    fn radial_gap_advances_with_the_same_velocity_used_by_the_damper() {
        let contact = SimplifiedNormalContact {
            radial_contact: VerticalTireContact {
                road_compression_metres: 0.1,
                road_velocity_metres_per_second: 0.2,
                stiffness_newtons_per_metre: 120000.0,
                damping_newton_seconds_per_metre: 1000.0,
                maximum_deflection_metres: 0.04,
                maximum_force_newtons: 20000.0,
            },
            initial_normal_projection: 0.8,
            normal_projection_rate_per_second: 0.03,
        };
        let compression = 0.1;
        let velocity = 0.04;
        let elapsed = 0.002;
        let response = contact.evaluate(compression, velocity, elapsed);
        assert!(
            (response.deflection_metres
                - (0.1 + 0.2 * elapsed - (0.8 + 0.03 * elapsed) * compression))
                .abs()
                < 1e-12
        );
        assert!(
            (response.deflection_velocity_metres_per_second
                - (0.2 - 0.03 * compression - (0.8 + 0.03 * elapsed) * velocity))
                .abs()
                < 1e-12
        );
    }
}
