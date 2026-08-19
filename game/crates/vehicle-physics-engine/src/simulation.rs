use crate::aero::AeroForces;
use crate::powertrain::PowertrainState;
use crate::suspension::SuspensionSystem;
use crate::telemetry::TelemetryFrame;
use crate::tire::TireSystem;
use crate::types::{Mat3, Quat, Transform3D, TriRaycastSample, Vec3, VehicleInput, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BodyKinematics {
    pub transform: Transform3D,
    pub orientation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ForceTorqueOutput {
    /// Force to apply to the Godot RigidBody3D. Gravity is intentionally excluded.
    pub force_world: Vec3,
    /// Torque around the actual configured center of mass.
    pub torque_world: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleState {
    pub transform: Transform3D,
    pub orientation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub linear_acceleration: Vec3,
    pub angular_acceleration: Vec3,
    pub steer_input_smoothed: f64,
    pub throttle_input_smoothed: f64,
    pub brake_input_smoothed: f64,
    pub powertrain: PowertrainState,
    pub suspension: SuspensionSystem,
    pub tires: TireSystem,
    pub aero: AeroForces,
    pub sim_time: f64,
    pub physics_hz: i32,
}

impl VehicleState {
    pub fn new(config: &VehicleConfig, spawn_position: Vec3, spawn_yaw_rad: f64) -> Self {
        let basis = Mat3::from_euler_yxz(spawn_yaw_rad, 0.0, 0.0);
        Self {
            transform: Transform3D::new(spawn_position, basis),
            orientation: Quat::from_mat3(&basis),
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            linear_acceleration: Vec3::ZERO,
            angular_acceleration: Vec3::ZERO,
            steer_input_smoothed: 0.0,
            throttle_input_smoothed: 0.0,
            brake_input_smoothed: 0.0,
            powertrain: PowertrainState::new(config),
            suspension: SuspensionSystem::new(config),
            tires: TireSystem::new(config),
            aero: AeroForces::zero(),
            sim_time: 0.0,
            physics_hz: 120,
        }
    }

    pub fn body_kinematics(&self) -> BodyKinematics {
        BodyKinematics {
            transform: self.transform,
            orientation: self.orientation,
            linear_velocity: self.linear_velocity,
            angular_velocity: self.angular_velocity,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AidsMask {
    pub abs: bool,
    pub traction_control: bool,
    pub stability: bool,
    pub steering_slip_assist: bool,
    pub countersteer: bool,
    pub auto_clutch: bool,
    pub launch_control: bool,
    pub brake_assist: bool,
}

impl AidsMask {
    /// Bit layout for the C-ABI `aids_enabled_mask` (ffi.rs):
    /// bit0=ABS, bit1=TC, bit2=stability, bit3=steering slip, bit4=countersteer,
    /// bit5=auto-clutch, bit6=launch, bit7=brake-assist.
    pub fn to_bits(&self) -> u32 {
        let mut b = 0u32;
        if self.abs { b |= 1 << 0; }
        if self.traction_control { b |= 1 << 1; }
        if self.stability { b |= 1 << 2; }
        if self.steering_slip_assist { b |= 1 << 3; }
        if self.countersteer { b |= 1 << 4; }
        if self.auto_clutch { b |= 1 << 5; }
        if self.launch_control { b |= 1 << 6; }
        if self.brake_assist { b |= 1 << 7; }
        b
    }

    pub fn from_bits(bits: u32) -> Self {
        Self {
            abs: (bits & (1 << 0)) != 0,
            traction_control: (bits & (1 << 1)) != 0,
            stability: (bits & (1 << 2)) != 0,
            steering_slip_assist: (bits & (1 << 3)) != 0,
            countersteer: (bits & (1 << 4)) != 0,
            auto_clutch: (bits & (1 << 5)) != 0,
            launch_control: (bits & (1 << 6)) != 0,
            brake_assist: (bits & (1 << 7)) != 0,
        }
    }

    /// Initial runtime mask derived from the profile's default policy. An aid is
    /// enabled only if it is available AND its default is on.
    pub fn from_config(config: &VehicleConfig) -> Self {
        let a = &config.aids;
        Self {
            abs: a.abs_available && a.abs_default_enabled,
            traction_control: a.traction_control_available && a.traction_control_default_enabled,
            stability: a.stability_available && a.stability_default_enabled,
            steering_slip_assist: a.steering_slip_assist_default_enabled,
            countersteer: a.countersteer_default_enabled,
            auto_clutch: a.auto_clutch_default_enabled,
            launch_control: a.launch_control_available && a.launch_control_default_enabled,
            brake_assist: a.brake_assist_available && a.brake_assist_default_enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleSimulator {
    pub config: VehicleConfig,
    pub state: VehicleState,
    pub aids: AidsMask,
}

impl VehicleSimulator {
    pub fn new(config: VehicleConfig, spawn_pos: Vec3, spawn_yaw: f64) -> Self {
        let state = VehicleState::new(&config, spawn_pos, spawn_yaw);
        let aids = AidsMask::from_config(&config);
        Self { config, state, aids }
    }

    /// Legacy standalone step. It uses the same GEVP force solver as external mode,
    /// then integrates a full rigid body locally. Prefer solve_external() when Godot
    /// already owns a RigidBody3D.
    pub fn step(&mut self, input: &VehicleInput, samples: &[TriRaycastSample; 4], dt: f64) -> TelemetryFrame {
        let (forces, _) = self.solve_forces(input, samples, dt);
        self.integrate_standalone(forces, dt);
        self.build_telemetry_frame()
    }

    /// Recommended integration path for Godot. Synchronizes the solver with the
    /// RigidBody3D state and returns force/torque for Godot to integrate.
    pub fn solve_external(
        &mut self,
        body: BodyKinematics,
        input: &VehicleInput,
        samples: &[TriRaycastSample; 4],
        dt: f64,
    ) -> (ForceTorqueOutput, TelemetryFrame) {
        self.state.transform = body.transform;
        self.state.orientation = body.orientation.normalized();
        self.state.transform.basis = self.state.orientation.to_mat3();
        self.state.linear_velocity = body.linear_velocity;
        self.state.angular_velocity = body.angular_velocity;
        let (forces, telemetry) = self.solve_forces(input, samples, dt);
        (forces, telemetry)
    }

    fn solve_forces(
        &mut self,
        input: &VehicleInput,
        samples: &[TriRaycastSample; 4],
        dt: f64,
    ) -> (ForceTorqueOutput, TelemetryFrame) {
        let dt = dt.clamp(1.0 / 1000.0, 1.0 / 20.0);
        self.state.sim_time += dt;
        self.state.physics_hz = (1.0 / dt).round() as i32;

        let cfg = &self.config;
        let st = &mut self.state;
        let basis = st.transform.basis;
        let local_velocity = basis.inverse_transform_vector(st.linear_velocity);
        let forward_speed = -local_velocity.z;
        let lateral_speed = local_velocity.x;

        Self::filter_inputs(
            cfg,
            st,
            input,
            forward_speed,
            lateral_speed,
            self.aids.steering_slip_assist,
            self.aids.countersteer,
            dt,
        );

        let effective_input = VehicleInput {
            throttle: st.throttle_input_smoothed,
            steering: st.steer_input_smoothed,
            brake: st.brake_input_smoothed,
            handbrake: input.handbrake.clamp(0.0, 1.0),
            clutch: input.clutch.clamp(0.0, 1.0),
            gear_request: input.gear_request,
        };

        st.aero.step(cfg, local_velocity, dt);
        st.suspension.step(cfg, samples, dt);

        let wheel_spins = [
            st.tires.wheels[0].spin,
            st.tires.wheels[1].spin,
            st.tires.wheels[2].spin,
            st.tires.wheels[3].spin,
        ];
        let reactions = st.tires.reaction_torques();
        let tc_enabled = self.aids.traction_control;
        let abs_enabled = self.aids.abs;
        let brake_assist_enabled = self.aids.brake_assist;
        st.powertrain.step_with_reaction(
            cfg,
            &effective_input,
            &wheel_spins,
            &reactions,
            forward_speed,
            tc_enabled,
            brake_assist_enabled,
            abs_enabled,
            dt,
        );

        // GEVP applies wheel torque before calculating this frame's tire force.
        for wheel in WheelIndex::ALL {
            let i = wheel as usize;
            st.tires.process_wheel_torque(
                cfg,
                wheel,
                st.powertrain.drive_torques[i],
                if crate::tire::is_driven(cfg, wheel) { st.powertrain.drive_inertia } else { 0.0 },
                st.powertrain.brake_torques[i],
                dt,
            );
        }

        let mut total_force = Vec3::ZERO;
        let mut total_torque = Vec3::ZERO;
        let cg_world = center_of_mass_world(cfg, &st.transform);

        // GEVP central aerodynamic drag.
        let drag_world = if st.linear_velocity.length() > 1e-6 {
            st.linear_velocity.normalized() * -st.aero.drag_force.abs()
        } else {
            Vec3::ZERO
        };
        total_force += drag_world;

        // 3-point aerodynamic load distribution:
        // Point 1: Front Wing / Axle (20%)
        // Point 2: Floor / Diffuser (60%) at chassis floor center
        // Point 3: Rear Wing / Axle (20%)
        if st.aero.total_downforce > 0.0 {
            let front_force = basis.transform_vector(Vec3::new(0.0, -st.aero.front_downforce, 0.0));
            let diffuser_force = basis.transform_vector(Vec3::new(0.0, -st.aero.diffuser_downforce, 0.0));
            let rear_force = basis.transform_vector(Vec3::new(0.0, -st.aero.rear_downforce, 0.0));

            let front_point = st.transform.transform_point(axle_center_local(cfg, true));
            let diffuser_point = st.transform.transform_point(diffuser_center_local(cfg));
            let rear_point = st.transform.transform_point(axle_center_local(cfg, false));

            total_force += front_force + diffuser_force + rear_force;
            total_torque += (front_point - cg_world).cross(front_force);
            total_torque += (diffuser_point - cg_world).cross(diffuser_force);
            total_torque += (rear_point - cg_world).cross(rear_force);
        }

        let braking = effective_input.brake > 0.0 || effective_input.handbrake > 0.0;
        for wheel in WheelIndex::ALL {
            let i = wheel as usize;
            let steer_angle = steering_angle_for_wheel(cfg, wheel, st.steer_input_smoothed);
            st.tires.wheels[i].steer_angle_rad = steer_angle;

            let contact_point = st.suspension.wheels[i].effective_contact_point;
            if !st.suspension.wheels[i].is_grounded {
                st.tires.process_wheel_forces(
                    cfg,
                    wheel,
                    0.0,
                    st.suspension.wheels[i].effective_surface,
                    st.suspension.wheels[i].effective_friction,
                    st.suspension.wheels[i].effective_stiffness,
                    st.suspension.wheels[i].effective_rolling_resistance,
                    braking,
                    Vec3::ZERO,
                    dt,
                );
                continue;
            }

            let arm = contact_point - cg_world;
            let point_velocity_world = st.linear_velocity + st.angular_velocity.cross(arm);
            let point_velocity_chassis = basis.inverse_transform_vector(point_velocity_world);
            let steer_basis = Mat3::from_euler_yxz(steer_angle, 0.0, 0.0);
            let point_velocity_wheel = steer_basis.inverse_transform_vector(point_velocity_chassis);

            st.tires.process_wheel_forces(
                cfg,
                wheel,
                st.suspension.wheels[i].total_normal_force,
                st.suspension.wheels[i].effective_surface,
                st.suspension.wheels[i].effective_friction,
                st.suspension.wheels[i].effective_stiffness,
                st.suspension.wheels[i].effective_rolling_resistance,
                braking,
                point_velocity_wheel,
                dt,
            );

            let tire = &st.tires.wheels[i];
            let tire_force_wheel = Vec3::new(tire.lateral_force, 0.0, -tire.longitudinal_force);
            let tire_force_chassis = steer_basis.transform_vector(tire_force_wheel);
            let tire_force_world = basis.transform_vector(tire_force_chassis);
            let suspension_force_world = st.suspension.wheels[i].effective_normal
                * st.suspension.wheels[i].total_normal_force;
            let wheel_force = tire_force_world + suspension_force_world;

            total_force += wheel_force;
            total_torque += arm.cross(wheel_force);

            // Port of GEVP's wheel-to-body longitudinal torque aid. Because this port
            // uses -Z-forward, positive forward tire force corresponds to +X pitch torque.
            let radius = if wheel.is_front() { cfg.front_tire_radius } else { cfg.rear_tire_radius };
            let grip = if wheel.is_front() { cfg.front_braking_grip } else { cfg.rear_braking_grip };
            let torque_multiplier = if braking {
                1.0 / (grip + 1.0).max(1e-6)
            } else {
                1.0
            };
            let extra_pitch_local = Vec3::new(tire.longitudinal_force * radius * torque_multiplier, 0.0, 0.0);
            total_torque += basis.transform_vector(extra_pitch_local);
        }

        // ESP / yaw-stability aid: counter only the excess yaw beyond the engage
        // threshold so the car stays controllable at the limit without killing
        // playful rotation.
        if self.aids.stability {
            total_torque += Self::stability_yaw_torque(cfg, &*st);
        }

        if !total_force.x.is_finite() || !total_force.y.is_finite() || !total_force.z.is_finite() {
            total_force = Vec3::ZERO;
        }
        if !total_torque.x.is_finite() || !total_torque.y.is_finite() || !total_torque.z.is_finite() {
            total_torque = Vec3::ZERO;
        }

        // In external mode this is force-derived acceleration for telemetry only;
        // Godot will add gravity itself.
        st.linear_acceleration = total_force / cfg.vehicle_mass.max(1e-6);
        let telemetry = self.build_telemetry_frame();
        (ForceTorqueOutput { force_world: total_force, torque_world: total_torque }, telemetry)
    }

    /// Yaw-stability (ESP) corrective torque in world space.
    ///
    /// Compares the actual yaw rate to the bicycle-model yaw rate commanded by the
    /// current steering angle and speed. Only the *excess* beyond
    /// `stability_yaw_engage_angle_rad` is corrected, so small playful over/understeer
    /// is preserved. The correction scales with `stability_yaw_strength` and is
    /// amplified by `stability_grounded_multiplier` when the car is planted.
    /// `stability_yaw_engage_angle_rad` is interpreted as a yaw-rate error threshold
    /// in rad/s (the field name is historical).
    fn stability_yaw_torque(cfg: &VehicleConfig, st: &VehicleState) -> Vec3 {
        let basis = st.transform.basis;

        let local_vel = basis.inverse_transform_vector(st.linear_velocity);
        let forward_speed = -local_vel.z;
        // Only act when clearly driving forward; reverse/creep is left to the driver.
        if forward_speed < 2.0 {
            return Vec3::ZERO;
        }

        let yaw_rate = basis.inverse_transform_vector(st.angular_velocity).y;
        let steer = steering_angle_for_wheel(cfg, WheelIndex::FrontLeft, st.steer_input_smoothed);
        let target_yaw = forward_speed * steer.tan() / cfg.wheelbase.max(1e-3);

        // True stability limiter: only counter rotation that EXCEEDS the yaw the
        // steering commands (plus a small margin). We never push the car toward the
        // kinematic target, so a straight / near-straight line at speed (where the
        // commanded yaw can be large from even a tiny steer) is left alone instead
        // of being force-yawed into a snap under throttle.
        let engage = cfg.aids.stability_yaw_engage_angle_rad.max(1e-4);
        let over = yaw_rate.abs() - target_yaw.abs() - engage;
        if over <= 0.0 {
            return Vec3::ZERO;
        }

        let mut grounded = 0u32;
        for w in WheelIndex::ALL {
            if st.suspension.wheels[w as usize].is_grounded {
                grounded += 1;
            }
        }
        let grounded_fraction = grounded as f64 / 4.0;
        let grounded_mult = cfg.aids.stability_grounded_multiplier.max(0.0);
        let ground_factor = (grounded_fraction * (grounded_mult - 1.0)).clamp(0.0, 5.0) + 1.0;

        let inertia_y = principal_inertia(cfg).y;
        let torque_y = -yaw_rate.signum() * over * cfg.aids.stability_yaw_strength * inertia_y * ground_factor;
        basis.transform_vector(Vec3::new(0.0, torque_y, 0.0))
    }

    fn integrate_standalone(&mut self, forces: ForceTorqueOutput, dt: f64) {
        let dt = dt.clamp(1.0 / 1000.0, 1.0 / 20.0);
        let cfg = &self.config;
        let st = &mut self.state;
        let old_basis = st.transform.basis;
        let cg_local = center_of_mass_local(cfg);
        let mut cg_world = st.transform.origin + old_basis.transform_vector(cg_local);

        let gravity_force = Vec3::new(0.0, -9.80665 * cfg.vehicle_mass, 0.0);
        st.linear_acceleration = (forces.force_world + gravity_force) / cfg.vehicle_mass.max(1e-6);
        st.linear_velocity += st.linear_acceleration * dt;
        cg_world += st.linear_velocity * dt;

        let inertia = principal_inertia(cfg);
        let torque_local = old_basis.inverse_transform_vector(forces.torque_world);
        let mut omega_local = old_basis.inverse_transform_vector(st.angular_velocity);
        let i_omega = Vec3::new(inertia.x * omega_local.x, inertia.y * omega_local.y, inertia.z * omega_local.z);
        let gyro = omega_local.cross(i_omega);
        let alpha_local = Vec3::new(
            (torque_local.x - gyro.x) / inertia.x.max(1e-6),
            (torque_local.y - gyro.y) / inertia.y.max(1e-6),
            (torque_local.z - gyro.z) / inertia.z.max(1e-6),
        );
        omega_local += alpha_local * dt;
        st.orientation = st.orientation.integrate_angular_velocity(omega_local, dt).normalized();
        let new_basis = st.orientation.to_mat3();
        st.transform.basis = new_basis;
        st.transform.origin = cg_world - new_basis.transform_vector(cg_local);
        st.angular_velocity = new_basis.transform_vector(omega_local);
        st.angular_acceleration = old_basis.transform_vector(alpha_local);
    }

    fn filter_inputs(
        cfg: &VehicleConfig,
        st: &mut VehicleState,
        input: &VehicleInput,
        forward_speed: f64,
        lateral_speed: f64,
        steering_slip_assist_on: bool,
        countersteer_on: bool,
        dt: f64,
    ) {
        let requested = input.steering.clamp(-1.0, 1.0);
        let mut target = requested;

        let slip_assist_on = steering_slip_assist_on && cfg.steering_slip_assist > 0.0;
        let countersteer_on = countersteer_on && cfg.countersteer_assist > 0.0;

        // Same purpose as GEVP steering_slip_assist: refuse additional steering into
        // excessive front slip, but still allow countersteer/recovery.
        // Both target (LEFT = +1.0, RIGHT = -1.0) and slip_angle_rad (positive when slipping left)
        // share the canonical sign convention.
        let front_slip = max_abs_signed(
            st.tires.wheels[0].slip_angle_rad,
            st.tires.wheels[1].slip_angle_rad,
        );
        if slip_assist_on && front_slip.abs() > cfg.steering_slip_assist {
            if target.signum() == front_slip.signum() && target.abs() > st.steer_input_smoothed.abs() {
                target = st.steer_input_smoothed;
            }
        }

        if forward_speed > 0.5 && countersteer_on {
            let speed = (forward_speed * forward_speed + lateral_speed * lateral_speed).sqrt();
            if speed > 1e-6 {
                let travel_angle = (lateral_speed / speed).clamp(-1.0, 1.0).asin();
                let countersteer_correction = -(travel_angle / cfg.max_steering_angle.max(1e-5))
                    * cfg.countersteer_assist
                    * (1.0 - requested.abs());
                target = (target + countersteer_correction).clamp(-1.0, 1.0);
            }
        }

        let countersteering = target.signum() != st.steer_input_smoothed.signum();
        let base_rate = if countersteering { cfg.countersteer_speed } else { cfg.steering_speed };
        let speed_decay = 1.0 + forward_speed.abs() * cfg.steering_speed_decay;
        let max_delta = (base_rate / speed_decay.max(1.0)) * dt;
        st.steer_input_smoothed = move_toward(st.steer_input_smoothed, target, max_delta).clamp(-1.0, 1.0);

        st.throttle_input_smoothed = move_toward(
            st.throttle_input_smoothed,
            input.throttle.clamp(0.0, 1.0),
            cfg.aids.input_smoothing_throttle_rate * dt,
        );
        st.brake_input_smoothed = move_toward(
            st.brake_input_smoothed,
            input.brake.clamp(0.0, 1.0),
            cfg.aids.input_smoothing_brake_rate * dt,
        );
    }

    fn build_telemetry_frame(&self) -> TelemetryFrame {
        let st = &self.state;
        let basis = st.transform.basis;
        let local_accel = basis.inverse_transform_vector(st.linear_acceleration);
        let front_slip = (st.tires.wheels[0].slip_ratio.abs() + st.tires.wheels[1].slip_ratio.abs()) * 0.5;
        let rear_slip = (st.tires.wheels[2].slip_ratio.abs() + st.tires.wheels[3].slip_ratio.abs()) * 0.5;
        TelemetryFrame {
            time_ms: (st.sim_time * 1000.0) as i64,
            speed_kmh: st.linear_velocity.length() * 3.6,
            rpm: st.powertrain.rpm,
            gear: st.powertrain.current_gear,
            throttle: st.throttle_input_smoothed,
            brake: st.brake_input_smoothed,
            steering: st.steer_input_smoothed,
            lat_g: local_accel.x / 9.80665,
            long_g: -local_accel.z / 9.80665,
            vert_g: local_accel.y / 9.80665,
            fl_comp_mm: st.suspension.wheels[0].compression_mm,
            fr_comp_mm: st.suspension.wheels[1].compression_mm,
            rl_comp_mm: st.suspension.wheels[2].compression_mm,
            rr_comp_mm: st.suspension.wheels[3].compression_mm,
            front_slip,
            rear_slip,
            tc_active: self.aids.traction_control,
            drive_torque: st.powertrain.drive_torques[2] + st.powertrain.drive_torques[3],
            session_id: "rust_gevp_tri_ray".to_string(),
            session_timestamp_utc: "".to_string(),
            physics_hz: st.physics_hz,
            test_id: "GEVP_TRI_RAY".to_string(),
            track_scene: "".to_string(),
            vehicle_node_path: "VehicleRigidBody".to_string(),
            vehicle_scene: "".to_string(),
            vehicle_script: "Rust GEVP-aligned force solver".to_string(),
            setup_schema_version: 2,
            setup_json: "{}".to_string(),
        }
    }
}

pub fn center_of_mass_local(config: &VehicleConfig) -> Vec3 {
    let com_z = (0.50 - config.front_weight_distribution) * config.wheelbase;
    Vec3::new(0.0, config.center_of_gravity_height_offset, com_z)
}

pub fn center_of_mass_world(config: &VehicleConfig, transform: &Transform3D) -> Vec3 {
    transform.transform_point(center_of_mass_local(config))
}

pub fn default_spawn_height(config: &VehicleConfig) -> f64 {
    (config.front_tire_radius + config.rear_tire_radius) * 0.5
}

fn axle_center_local(config: &VehicleConfig, front: bool) -> Vec3 {
    let z = if front {
        -config.wheelbase * 0.5
    } else {
        config.wheelbase * 0.5
    };
    Vec3::new(0.0, 0.0, z)
}

pub fn diffuser_center_local(config: &VehicleConfig) -> Vec3 {
    Vec3::new(0.0, config.center_of_gravity_height_offset, 0.0)
}

pub fn steering_angle_for_wheel(config: &VehicleConfig, wheel: WheelIndex, steering: f64) -> f64 {
    let ratio = if wheel.is_front() { config.front_steering_ratio } else { config.rear_steering_ratio };
    let input = steering.signum() * steering.abs().powf(config.steering_exponent) * ratio;

    // GEVP derives the Ackermann coefficient from wheelbase, track and max steering
    // angle, then mirrors the sign on the right wheel. Keep `config.ackermann` as a
    // compatibility fallback only for degenerate geometry.
    let track = if wheel.is_front() { config.front_track } else { config.rear_track };
    let tan_max = config.max_steering_angle.tan();
    let denominator = config.wheelbase - track * 0.5 * tan_max;
    let geometric_ackermann = if config.max_steering_angle.abs() > 1e-6 && denominator.abs() > 1e-6 {
        ((config.wheelbase * tan_max) / denominator).atan() / config.max_steering_angle - 1.0
    } else {
        config.ackermann
    };
    let ackermann = if wheel.is_left() { geometric_ackermann } else { -geometric_ackermann };
    let toe = if wheel.is_front() { config.front_toe } else { config.rear_toe };
    let signed_toe = if wheel.is_left() { -toe } else { toe };
    config.max_steering_angle
        * (input + (1.0 - (input * 0.5 * std::f64::consts::PI).cos()) * ackermann)
        + signed_toe
}

fn principal_inertia(config: &VehicleConfig) -> Vec3 {
    let w = (config.front_track + config.rear_track) * 0.5;
    let h = 0.75;
    let l = config.wheelbase;
    Vec3::new(
        (config.vehicle_mass / 12.0) * (h * h + l * l) * config.inertia_multipliers.x,
        (config.vehicle_mass / 12.0) * (w * w + l * l) * config.inertia_multipliers.y,
        (config.vehicle_mass / 12.0) * (w * w + h * h) * config.inertia_multipliers.z,
    )
}

fn move_toward(current: f64, target: f64, max_delta: f64) -> f64 {
    if (target - current).abs() <= max_delta { target } else { current + (target - current).signum() * max_delta }
}

fn max_abs_signed(a: f64, b: f64) -> f64 { if a.abs() >= b.abs() { a } else { b } }

#[cfg(test)]
mod tests {
    use crate::simulation::VehicleSimulator;
    use crate::types::{TriRaycastSample, Vec3, VehicleInput};
    use crate::vehicle_config::VehicleConfig;

    fn step_to_gear(sim: &mut VehicleSimulator, req: i8, max_steps: usize) -> i8 {
        let samples = [TriRaycastSample::default(); 4];
        let inp = VehicleInput {
            steering: 0.0,
            throttle: 0.0,
            brake: 0.0,
            handbrake: 0.0,
            clutch: 0.0,
            gear_request: Some(req),
        };
        for _ in 0..max_steps {
            let _ = sim.step(&inp, &samples, 0.02);
            if sim.state.powertrain.current_gear == req {
                return req;
            }
        }
        sim.state.powertrain.current_gear
    }

    #[test]
    fn gear_request_sentinel_neutral_and_reverse() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        cfg.automatic_transmission = false; // match data/vehicles/f1_94/f1_94_physics.json
        let mut sim = VehicleSimulator::new(cfg, Vec3::ZERO, 0.0);

        // Initially in gear 1 (or whatever the powertrain starts at).
        let start = sim.state.powertrain.current_gear;

        // Upshift to 2 (proves forwarding + manual shift works).
        let g2 = step_to_gear(&mut sim, 2, 2000);
        assert_eq!(g2, 2, "should shift up to gear 2 (start was {})", start);

        // Neutral (0) must now be reachable — this is the sentinel fix.
        let g0 = step_to_gear(&mut sim, 0, 2000);
        assert_eq!(g0, 0, "gear_request=0 must select Neutral (was broken: 0 mapped to no-change)");

        // Reverse (-1) must be reachable.
        let gR = step_to_gear(&mut sim, -1, 2000);
        assert_eq!(gR, -1, "gear_request=-1 must select Reverse");

        // No-change sentinel: None must leave the gear untouched.
        let samples = [TriRaycastSample::default(); 4];
        let hold = VehicleInput {
            steering: 0.0, throttle: 0.0, brake: 0.0, handbrake: 0.0, clutch: 0.0,
            gear_request: None,
        };
        let before = sim.state.powertrain.current_gear;
        for _ in 0..50 {
            let _ = sim.step(&hold, &samples, 0.02);
        }
        assert_eq!(sim.state.powertrain.current_gear, before, "None must not change gear");
    }

    #[test]
    fn stability_yaw_torque_only_corrects_excess() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        cfg.aids.stability_available = true;
        cfg.aids.stability_default_enabled = true;
        cfg.aids.stability_yaw_engage_angle_rad = 0.10;
        cfg.aids.stability_yaw_strength = 5.0;
        cfg.aids.stability_grounded_multiplier = 1.0;

        let mut sim = VehicleSimulator::new(cfg, Vec3::ZERO, 0.0);
        // Drive forward at 20 m/s (world -Z), basis identity, no steer.
        sim.state.linear_velocity = Vec3::new(0.0, 0.0, -20.0);
        sim.state.steer_input_smoothed = 0.0;

        // Within engage band: 0.05 rad/s < 0.10 -> no correction.
        sim.state.angular_velocity = Vec3::new(0.0, 0.05, 0.0);
        let t_within = VehicleSimulator::stability_yaw_torque(&sim.config, &sim.state);
        assert!(t_within.length() < 1e-9, "no correction within engage band");

        // Exceeds engage: 0.5 rad/s -> restoring torque opposes +Y (negative Y).
        sim.state.angular_velocity = Vec3::new(0.0, 0.5, 0.0);
        let t_excess = VehicleSimulator::stability_yaw_torque(&sim.config, &sim.state);
        assert!(t_excess.y < 0.0, "excess yaw must be countered (negative Y torque)");
        assert!(t_excess.x.abs() < 1e-9 && t_excess.z.abs() < 1e-9, "torque is pure yaw");
    }

    #[test]
    fn stability_yaw_torque_ignores_straight_line_command() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        cfg.aids.stability_available = true;
        cfg.aids.stability_default_enabled = true;
        cfg.aids.stability_yaw_engage_angle_rad = 0.15;
        cfg.aids.stability_yaw_strength = 6.0;
        cfg.aids.stability_grounded_multiplier = 2.0;

        let mut sim = VehicleSimulator::new(cfg, Vec3::ZERO, 0.0);
        // Straight-line at speed WITH a small steering command but no actual yaw.
        // This is the "pedal to the metal on a straight" case that used to snap.
        sim.state.linear_velocity = Vec3::new(0.0, 0.0, -50.0);
        sim.state.steer_input_smoothed = 0.05;
        sim.state.angular_velocity = Vec3::ZERO;

        let t = VehicleSimulator::stability_yaw_torque(&sim.config, &sim.state);
        assert!(t.length() < 1e-9, "must not force yaw on a straight with a small steer command");
    }
}
