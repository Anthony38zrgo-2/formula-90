use crate::aero::AeroForces;
use crate::powertrain::PowertrainState;
use crate::suspension::SuspensionSystem;
use crate::telemetry::TelemetryFrame;
use crate::tire::TireSystem;
use crate::types::{Mat3, Quat, Transform3D, TriRaycastSample, Vec3, VehicleInput, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// Dynamic 6-DOF state of the simulated vehicle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleState {
    pub transform: Transform3D,
    pub orientation: Quat,
    pub linear_velocity: Vec3,      // m/s in world frame
    pub angular_velocity: Vec3,     // rad/s in world frame
    pub linear_acceleration: Vec3,  // m/s² in world frame
    pub angular_acceleration: Vec3, // rad/s² in world frame

    // Filtered inputs
    pub steer_input_smoothed: f64,
    pub throttle_input_smoothed: f64,
    pub brake_input_smoothed: f64,

    // Subsystems
    pub powertrain: PowertrainState,
    pub suspension: SuspensionSystem,
    pub tires: TireSystem,
    pub aero: AeroForces,

    pub sim_time: f64, // seconds
}

impl VehicleState {
    pub fn new(config: &VehicleConfig, spawn_position: Vec3, spawn_yaw_rad: f64) -> Self {
        let basis = Mat3::from_euler_yxz(spawn_yaw_rad, 0.0, 0.0);
        let orientation = Quat::from_mat3(&basis);

        Self {
            transform: Transform3D::new(spawn_position, basis),
            orientation,
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
        }
    }
}

/// Standalone deterministic vehicle simulator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleSimulator {
    pub config: VehicleConfig,
    pub state: VehicleState,
}

impl VehicleSimulator {
    pub fn new(config: VehicleConfig, spawn_pos: Vec3, spawn_yaw: f64) -> Self {
        let state = VehicleState::new(&config, spawn_pos, spawn_yaw);
        Self { config, state }
    }

    /// Execute a single deterministic physics simulation step over timestep dt (e.g. 1/60s).
    pub fn step(&mut self, input: &VehicleInput, samples: &[TriRaycastSample; 4], dt: f64) -> TelemetryFrame {
        let cfg = &self.config;
        let st = &mut self.state;
        st.sim_time += dt;

        // 1. Local frame transformation
        let basis = st.transform.basis;
        let local_velocity = basis.inverse_transform_vector(st.linear_velocity);
        let forward_speed = -local_velocity.z; // Godot: -Z forward
        let lateral_speed = local_velocity.x;

        // 2. Input filtering & steering assists
        Self::filter_inputs(cfg, st, input, forward_speed, lateral_speed, dt);

        // 3. Compute Aerodynamics
        st.aero = AeroForces::calculate(cfg, local_velocity);

        // 4. Step Powertrain
        let wheel_spins = [
            st.tires.wheels[0].spin,
            st.tires.wheels[1].spin,
            st.tires.wheels[2].spin,
            st.tires.wheels[3].spin,
        ];
        st.powertrain.step(cfg, input, &wheel_spins, forward_speed, dt);

        // 5. Step 3-Raycast Suspension
        st.suspension.step(cfg, samples, dt);

        // 6. Step Tires and gather forces
        let mut total_force_world = Vec3::new(0.0, -9.80665 * cfg.vehicle_mass, 0.0); // Gravity
        let mut total_torque_world = Vec3::ZERO;

        // Aero downforce & drag forces
        let drag_world = basis.transform_vector(Vec3::new(0.0, 0.0, st.aero.drag_force)); // opposes forward motion
        total_force_world += drag_world;
        total_force_world += Vec3::new(0.0, -st.aero.total_downforce, 0.0);

        // Aero pitch moment from front/rear downforce split around CG
        let pitch_arm_front = -(1.0 - cfg.front_weight_distribution) * cfg.wheelbase;
        let pitch_arm_rear = cfg.front_weight_distribution * cfg.wheelbase;
        let tau_x = st.aero.front_downforce * pitch_arm_front + st.aero.rear_downforce * pitch_arm_rear;
        total_torque_world += basis.transform_vector(Vec3::new(tau_x, 0.0, 0.0));

        // Apply non-linear exponent curve to smoothed steer input
        let steer_sign = st.steer_input_smoothed.signum();
        let steer_mag = st.steer_input_smoothed.abs().powf(cfg.steering_exponent);
        let base_steer_angle = steer_sign * steer_mag * cfg.max_steering_angle;

        // Dynamic reflected motor inertia per active gear (Spec Item 3)
        let effective_gear_ratio = st.powertrain.get_current_gear_ratio(cfg);
        let clutch_engagement = st.powertrain.clutch_engagement;

        for i in 0..4 {
            let wheel = WheelIndex::ALL[i];
            let steer_angle = if wheel.is_front() {
                base_steer_angle * cfg.front_steering_ratio
            } else {
                base_steer_angle * cfg.rear_steering_ratio
            };
            st.tires.wheels[i].steer_angle_rad = steer_angle;

            // Normal load = suspension normal force + aero downforce share
            let aero_downforce_share = if wheel.is_front() {
                st.aero.front_downforce * 0.5
            } else {
                st.aero.rear_downforce * 0.5
            };
            let normal_load = st.suspension.wheels[i].total_normal_force + aero_downforce_share;

            // Hub world anchor and local velocity
            let hub_local = cfg.wheel_anchor_local(wheel);
            let hub_world = st.transform.transform_point(hub_local);
            let arm_from_cg = hub_world - st.transform.origin;

            // Wheel hub velocity = chassis linear velocity + (angular_velocity x arm)
            let hub_vel_world = st.linear_velocity + st.angular_velocity.cross(arm_from_cg);
            let hub_vel_chassis = basis.inverse_transform_vector(hub_vel_world);

            // Rotate hub velocity by steering angle around Y
            let steer_rot = Mat3::from_euler_yxz(steer_angle, 0.0, 0.0);
            let hub_vel_wheel = steer_rot.inverse_transform_vector(hub_vel_chassis);

            // Step tire
            let eff_friction = st.suspension.wheels[i].effective_friction;
            let drive_torque = st.powertrain.drive_torques[i];
            let brake_torque = st.powertrain.brake_torques[i];

            st.tires.step_wheel(
                cfg,
                wheel,
                normal_load,
                eff_friction,
                drive_torque,
                brake_torque,
                hub_vel_wheel,
                dt,
                effective_gear_ratio,
                clutch_engagement,
            );

            // Transform tire forces back to world frame
            let tire_st = &st.tires.wheels[i];
            let f_tire_wheel = Vec3::new(tire_st.lateral_force, 0.0, -tire_st.longitudinal_force); // -Z forward
            let f_tire_chassis = steer_rot.transform_vector(f_tire_wheel);
            let f_tire_world = basis.transform_vector(f_tire_chassis);

            // Suspension normal force in world frame
            let f_susp_world = st.suspension.wheels[i].effective_normal * st.suspension.wheels[i].total_normal_force;

            let f_wheel_world = f_tire_world + f_susp_world;
            total_force_world += f_wheel_world;

            // Torque from wheel force around CG
            total_torque_world += arm_from_cg.cross(f_wheel_world);

            // Aligning torque (Mz)
            let mz_world = basis.transform_vector(Vec3::new(0.0, tire_st.aligning_torque, 0.0));
            total_torque_world += mz_world;
        }

        // 7. 6-DOF Rigid Body Integration (Semi-Implicit Euler)
        let inv_mass = 1.0 / cfg.vehicle_mass;
        st.linear_acceleration = total_force_world * inv_mass;

        // Principal moments of inertia (approximated box)
        let (w, h, l) = (cfg.front_track, 0.95, cfg.wheelbase);
        let i_xx = (1.0 / 12.0) * cfg.vehicle_mass * (h * h + l * l) * cfg.inertia_multipliers.x;
        let i_yy = (1.0 / 12.0) * cfg.vehicle_mass * (w * w + l * l) * cfg.inertia_multipliers.y;
        let i_zz = (1.0 / 12.0) * cfg.vehicle_mass * (w * w + h * h) * cfg.inertia_multipliers.z;

        let inv_inertia = Vec3::new(1.0 / i_xx, 1.0 / i_yy, 1.0 / i_zz);
        let torque_local = basis.inverse_transform_vector(total_torque_world);
        let omega_local = basis.inverse_transform_vector(st.angular_velocity);

        // Euler gyroscopic torque: T - omega x (I * omega)
        let gyro = Vec3::new(
            (i_yy - i_zz) * omega_local.y * omega_local.z,
            (i_zz - i_xx) * omega_local.z * omega_local.x,
            (i_xx - i_yy) * omega_local.x * omega_local.y,
        );
        let ang_accel_local = Vec3::new(
            (torque_local.x + gyro.x) * inv_inertia.x,
            (torque_local.y + gyro.y) * inv_inertia.y,
            (torque_local.z + gyro.z) * inv_inertia.z,
        );
        st.angular_acceleration = basis.transform_vector(ang_accel_local);

        // Velocity & Position integration
        st.linear_velocity += st.linear_acceleration * dt;
        st.transform.origin += st.linear_velocity * dt;

        // Angular velocity & Orientation integration
        st.angular_velocity += st.angular_acceleration * dt;
        let omega_body = basis.inverse_transform_vector(st.angular_velocity);
        st.orientation = st.orientation.integrate_angular_velocity(omega_body, dt);
        st.transform.basis = st.orientation.to_mat3();

        // 8. Produce Telemetry Frame
        self.build_telemetry_frame()
    }

    fn filter_inputs(
        cfg: &VehicleConfig,
        st: &mut VehicleState,
        input: &VehicleInput,
        forward_speed: f64,
        lateral_speed: f64,
        dt: f64,
    ) {
        // High speed steering speed decay
        let decay = 1.0 / (1.0 + forward_speed.abs() * cfg.steering_speed_decay);
        let mut steer_target = input.steering.clamp(-1.0, 1.0);

        // Countersteer assist
        if cfg.countersteer_assist > 0.0 && forward_speed > 3.0 {
            let slip_angle = (lateral_speed / forward_speed.max(1.0)).atan();
            steer_target += slip_angle * cfg.countersteer_assist;
            steer_target = steer_target.clamp(-1.0, 1.0);
        }

        // Steer rate slew
        let is_countersteering = steer_target.signum() != st.steer_input_smoothed.signum()
            && steer_target.abs() < st.steer_input_smoothed.abs();
        let rate = if is_countersteering {
            cfg.countersteer_speed
        } else {
            cfg.steering_speed * decay
        };

        let steer_diff = steer_target - st.steer_input_smoothed;
        st.steer_input_smoothed = (st.steer_input_smoothed + steer_diff.clamp(-rate * dt, rate * dt)).clamp(-1.0, 1.0);

        // Throttle & Brake smoothing
        st.throttle_input_smoothed = input.throttle.clamp(0.0, 1.0);
        st.brake_input_smoothed = input.brake.clamp(0.0, 1.0);
    }

    fn build_telemetry_frame(&self) -> TelemetryFrame {
        let st = &self.state;
        let basis = st.transform.basis;
        let local_vel = basis.inverse_transform_vector(st.linear_velocity);
        let speed_kmh = (local_vel.z.abs() * 3.6).max(0.0);

        let local_accel = basis.inverse_transform_vector(st.linear_acceleration);
        let lat_g = local_accel.x / 9.80665;
        let long_g = -local_accel.z / 9.80665;
        let vert_g = local_accel.y / 9.80665;

        let front_slip = (st.tires.wheels[0].slip_ratio.abs() + st.tires.wheels[1].slip_ratio.abs()) * 0.5;
        let rear_slip = (st.tires.wheels[2].slip_ratio.abs() + st.tires.wheels[3].slip_ratio.abs()) * 0.5;

        TelemetryFrame {
            time_ms: (st.sim_time * 1000.0) as i64,
            speed_kmh,
            rpm: st.powertrain.rpm,
            gear: st.powertrain.current_gear,
            throttle: st.throttle_input_smoothed,
            brake: st.brake_input_smoothed,
            steering: st.steer_input_smoothed,
            lat_g,
            long_g,
            vert_g,
            fl_comp_mm: st.suspension.wheels[0].compression_mm,
            fr_comp_mm: st.suspension.wheels[1].compression_mm,
            rl_comp_mm: st.suspension.wheels[2].compression_mm,
            rr_comp_mm: st.suspension.wheels[3].compression_mm,
            front_slip,
            rear_slip,
            session_id: "rust_physics_benchmark".to_string(),
            session_timestamp_utc: "2026-08-15T00:00:00Z".to_string(),
            physics_hz: 60,
            test_id: "PHY_SIM".to_string(),
            track_scene: "res://scenes/tracks/test_field/la_chutana_generated.tscn".to_string(),
            vehicle_node_path: "VehicleRigidBody".to_string(),
            vehicle_scene: "res://scenes/vehicles/f1_94/f1_94.tscn".to_string(),
            vehicle_script: "res://addons/gevp/scripts/vehicle.gd".to_string(),
            setup_schema_version: 1,
            setup_json: "{}".to_string(),
        }
    }
}
