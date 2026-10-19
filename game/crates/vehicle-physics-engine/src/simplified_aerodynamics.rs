use super::*;

impl AeroForces {
    pub fn step_simplified_with_kinematics(
        &mut self,
        configuration: &VehicleConfig,
        kinematics: &AeroKinematics,
        environment: &AeroEnvironment,
        duration_seconds: f64,
    ) {
        *self = Self::zero();
        if self.input_valid(configuration, kinematics, duration_seconds) {
            self.invalid_input = true;
            return;
        }
        let model = &configuration.aerodynamics.aero_model;
        let floor_configuration = &model.underfloor;
        let airflow = AirFlow::at(configuration, kinematics, kinematics.center_of_mass_m);
        let front_position = model.front_wing.position_m.unwrap_or(Vec3::new(
            0.0,
            0.0,
            -0.5 * configuration.chassis.wheelbase,
        ));
        let rear_position = model.rear_wing.position_m.unwrap_or(Vec3::new(
            0.0,
            0.0,
            0.5 * configuration.chassis.wheelbase,
        ));
        let floor_position = floor_configuration.position_m.unwrap_or(Vec3::new(
            0.0,
            configuration.chassis.center_of_gravity_height_offset,
            0.0,
        ));
        let front_angle = model.front_wing.incidence_deg + airflow.angle_deg;
        let rear_angle = model.rear_wing.incidence_deg + airflow.angle_deg;
        let front_coefficients = wing_polar_at(&model.front_wing, front_angle);
        let rear_coefficients = wing_polar_at(&model.rear_wing, rear_angle);
        let wing_force = |wing: &WingAeroConfig, coefficients: (f64, f64)| {
            airflow.lift_direction()
                * (airflow.q
                    * wing.area_m2
                    * coefficients.0
                    * airflow.exposure(wing.yaw_decay_exponent))
        };
        let front_force = wing_force(&model.front_wing, front_coefficients);
        let rear_force = wing_force(&model.rear_wing, rear_coefficients);
        let sensor_valid = |index: usize| {
            kinematics.probe_mode != AeroProbeMode::Disabled
                && environment.valid_mask & (1 << index) != 0
                && environment.clearance_m[index].is_finite()
                && environment.clearance_m[index] >= 0.0
        };
        let front_valid = sensor_valid(0);
        let throat_valid = sensor_valid(3);
        let mut floor_factor = 0.0;
        let mut pressure_shift = 0.0;
        if front_valid && throat_valid {
            let front_height = environment.clearance_m[0];
            let throat_height =
                (environment.clearance_m[3] - floor_configuration.throat_probe_offset_m).max(0.0);
            let rake_radians = ((front_height - throat_height) / 1.75).atan();
            if let Some(map) = &floor_configuration.map {
                let sample = map.sample(front_height, throat_height);
                floor_factor = sample.load_factor;
                pressure_shift = sample.pressure_shift_m;
                self.floor_rake_factor = 1.0;
            } else {
                let front_efficiency = height_efficiency(front_height, floor_configuration);
                let throat_efficiency = height_efficiency(throat_height, floor_configuration);
                self.floor_rake_factor = 1.0
                    - smoothstep(
                        (rake_radians.to_degrees() - floor_configuration.optimal_rake_deg).abs()
                            / floor_configuration.rake_window_deg,
                    );
                floor_factor =
                    (front_efficiency * throat_efficiency).sqrt() * self.floor_rake_factor;
                pressure_shift = floor_configuration.max_pressure_shift_m
                    * (throat_efficiency - front_efficiency)
                    / (front_efficiency + throat_efficiency).max(1e-8);
            }
            self.ground_confidence = 1.0;
            self.ground_status = AeroGroundStatus::Complete;
            self.state.last_ground = GroundGeometry {
                front_height_m: front_height,
                rear_height_m: throat_height,
                rake_rad: rake_radians,
                confidence: 1.0,
                valid_probe_mask: 0b01001,
                status: AeroGroundStatus::Complete,
                ..GroundGeometry::default()
            };
            self.state.has_ground_sample = true;
        } else {
            self.ground_status = if kinematics.probe_mode == AeroProbeMode::Disabled {
                AeroGroundStatus::Disabled
            } else if front_valid || throat_valid {
                AeroGroundStatus::Partial
            } else {
                AeroGroundStatus::OutOfRange
            };
        }
        self.floor_height_factor = floor_factor;
        self.floor_seal_factor = 1.0;
        self.diffuser_stall_factor = 1.0;
        self.state.pressure_shift_m = pressure_shift;
        let floor_application = floor_position + Vec3::new(0.0, 0.0, pressure_shift);
        let floor_force = airflow.lift_direction()
            * (airflow.q
                * (floor_configuration.free_air_lift_area_m2
                    + floor_configuration.lift_area_m2 * floor_factor)
                * airflow.exposure(floor_configuration.yaw_decay_exponent));
        let raw_downforce = (-front_force.y - rear_force.y - floor_force.y).max(0.0);
        let vehicle_weight = configuration
            .chassis
            .total_vehicle_mass(&configuration.tires, &configuration.fuel)
            * 9.81;
        let hard_limit = (vehicle_weight * model.limits.hard_max_load_ratio)
            .min(model.limits.hard_max_downforce_n);
        let limited_downforce = soft_limit(
            raw_downforce,
            (vehicle_weight * model.limits.soft_max_load_ratio).min(hard_limit),
            hard_limit,
        );
        let limit_factor = if raw_downforce > 1e-8 {
            limited_downforce / raw_downforce
        } else {
            1.0
        };
        let mut body_drag_area = configuration.aerodynamics.coefficient_of_drag
            * configuration.aerodynamics.frontal_area;
        if model.body.drag_coefficient_scope == DragCoefficientScope::WholeVehicleReference {
            body_drag_area = (body_drag_area
                - model.front_wing.area_m2
                    * polar_at(&model.front_wing.polar, model.front_wing.incidence_deg).1
                - model.rear_wing.area_m2
                    * polar_at(&model.rear_wing.polar, model.rear_wing.incidence_deg).1
                - (floor_configuration.free_air_lift_area_m2 + floor_configuration.lift_area_m2)
                    * floor_configuration.induced_drag_ratio)
                .max(0.0);
        }
        body_drag_area -= model.body.wheel_reference_drag_area_m2;
        let body_drag = Vec3::new(
            -airflow.q * model.body.side_drag_area_m2 * airflow.direction.x,
            -airflow.q * model.body.vertical_drag_area_m2 * airflow.direction.y,
            -airflow.q * body_drag_area * airflow.direction.z,
        );
        let crossflow_fraction = 1.0 - airflow.forward_projection;
        let front_drag_area = model.front_wing.area_m2
            * (front_coefficients.1 * (1.0 - crossflow_fraction)
                + model.front_wing.stalled_drag_coefficient * crossflow_fraction);
        let rear_drag_area = model.rear_wing.area_m2
            * (rear_coefficients.1 * (1.0 - crossflow_fraction)
                + model.rear_wing.stalled_drag_coefficient * crossflow_fraction);
        let floor_drag_magnitude =
            floor_force.length() * limit_factor * floor_configuration.induced_drag_ratio;
        let front_drag = airflow.direction * (-airflow.q * front_drag_area);
        let rear_drag = airflow.direction * (-airflow.q * rear_drag_area);
        let floor_drag = airflow.direction * -floor_drag_magnitude;
        let mut wheel_drag_magnitude = 0.0;
        for wheel in WheelIndex::ALL {
            let (width, radius, coefficient) = if wheel.is_front() {
                (
                    configuration.tires.front_tire_width,
                    configuration.tires.front_tire_radius,
                    model.wheels.front_drag_coefficient,
                )
            } else {
                (
                    configuration.tires.rear_tire_width,
                    configuration.tires.rear_tire_radius,
                    model.wheels.rear_drag_coefficient,
                )
            };
            let magnitude = airflow.q * width * 2.0 * radius * coefficient;
            self.wheel_drag_forces[wheel as usize] = magnitude;
            wheel_drag_magnitude += magnitude;
        }
        let body_application = model.body.position_m.unwrap_or(kinematics.center_of_mass_m);
        let wheel_drag = airflow.direction * -wheel_drag_magnitude;
        let total_drag = body_drag + front_drag + rear_drag + floor_drag + wheel_drag;
        let total_force = (front_force + rear_force + floor_force) * limit_factor + total_drag;
        let total_torque = (front_position - kinematics.center_of_mass_m)
            .cross(front_force * limit_factor + front_drag)
            + (rear_position - kinematics.center_of_mass_m)
                .cross(rear_force * limit_factor + rear_drag)
            + (floor_application - kinematics.center_of_mass_m)
                .cross(floor_force * limit_factor + floor_drag)
            + (body_application - kinematics.center_of_mass_m).cross(body_drag);
        self.total_downforce = limited_downforce;
        self.front_downforce = -front_force.y * limit_factor;
        self.rear_downforce = -rear_force.y * limit_factor;
        self.diffuser_downforce = -floor_force.y * limit_factor;
        self.raw_downforce = raw_downforce;
        self.global_limit_factor = limit_factor;
        self.force_local = total_force;
        self.torque_local = total_torque;
        self.drag_force_local = total_drag;
        self.body_drag_force = body_drag.length();
        self.wing_and_floor_drag_force =
            front_drag.length() + rear_drag.length() + floor_drag_magnitude;
        self.drag_force =
            self.body_drag_force + self.wing_and_floor_drag_force + wheel_drag_magnitude;
        self.center_of_pressure_local = if limited_downforce > 1e-8 {
            (front_position * self.front_downforce
                + rear_position * self.rear_downforce
                + floor_application * self.diffuser_downforce)
                / limited_downforce
        } else {
            floor_position
        };
        let axle_load = -total_force.y;
        let moment_at_origin = total_torque + kinematics.center_of_mass_m.cross(total_force);
        let (front_axle_downforce, rear_axle_downforce) =
            aero_axle_load_split(configuration, axle_load, moment_at_origin.x);
        self.front_axle_downforce = front_axle_downforce;
        self.rear_axle_downforce = rear_axle_downforce;
        self.balance_front = if axle_load > 1e-8 {
            self.front_axle_downforce / axle_load
        } else {
            0.5
        };
        self.effective_cl = if airflow.q * configuration.aerodynamics.frontal_area > 1e-8 {
            limited_downforce / (airflow.q * configuration.aerodynamics.frontal_area)
        } else {
            0.0
        };
        self.load_ratio = limited_downforce / vehicle_weight;
        self.yaw_decay_factor = airflow.cos_yaw;
        self.blend_factor = airflow.blend;
        self.front_wing_angle_deg = front_angle;
        self.rear_wing_angle_deg = rear_angle;
        self.front_wing_cl = front_coefficients.0;
        self.rear_wing_cl = rear_coefficients.0;
        self.state.front_wing.cl = front_coefficients.0;
        self.state.front_wing.cd = front_coefficients.1;
        self.state.rear_wing.cl = rear_coefficients.0;
        self.state.rear_wing.cd = rear_coefficients.1;
        if self.output_invalid(total_force, total_torque) {
            *self = Self::zero();
            self.invalid_input = true;
        }
    }
}
