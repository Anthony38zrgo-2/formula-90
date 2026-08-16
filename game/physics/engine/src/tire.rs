use crate::types::{Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// Dynamic tire state for a single wheel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelTireState {
    pub spin: f64,               // rad/s
    pub steer_angle_rad: f64,     // radians
    pub slip_angle_rad: f64,     // alpha (radians)
    pub slip_ratio: f64,         // kappa (unitless)
    pub lateral_force: f64,      // Fy in N (positive = right)
    pub longitudinal_force: f64, // Fx in N (positive = forward)
    pub rolling_resistance: f64, // N
    pub aligning_torque: f64,    // Mz in N·m
    pub spin_velocity_diff: f64,
    pub wheel_moment: f64,       // kg·m²
}

impl WheelTireState {
    pub fn new(config: &VehicleConfig, wheel: WheelIndex) -> Self {
        let radius = if wheel.is_front() { config.front_tire_radius } else { config.rear_tire_radius };
        let mass = if wheel.is_front() { config.front_wheel_mass } else { config.rear_wheel_mass };
        let wheel_moment = 0.5 * mass * radius * radius;

        Self {
            spin: 0.0,
            steer_angle_rad: 0.0,
            slip_angle_rad: 0.0,
            slip_ratio: 0.0,
            lateral_force: 0.0,
            longitudinal_force: 0.0,
            rolling_resistance: 0.0,
            aligning_torque: 0.0,
            spin_velocity_diff: 0.0,
            wheel_moment,
        }
    }
}

/// Tire system managing all 4 wheel tire contact dynamics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireSystem {
    pub wheels: [WheelTireState; 4],
}

impl TireSystem {
    pub fn new(config: &VehicleConfig) -> Self {
        Self {
            wheels: [
                WheelTireState::new(config, WheelIndex::FrontLeft),
                WheelTireState::new(config, WheelIndex::FrontRight),
                WheelTireState::new(config, WheelIndex::RearLeft),
                WheelTireState::new(config, WheelIndex::RearRight),
            ],
        }
    }

    /// Step tire dynamics for a wheel.
    /// local_velocity: velocity of wheel hub in wheel plane (+X = right, +Y = up, -Z = forward in Godot convention).
    pub fn step_wheel(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        normal_force_n: f64,
        effective_friction_coef: f64,
        drive_torque_nm: f64,
        brake_torque_nm: f64,
        local_wheel_velocity: Vec3, // in wheel local frame
        dt: f64,
    ) {
        let idx = wheel as usize;
        let state = &mut self.wheels[idx];
        let tire_radius = if wheel.is_front() { config.front_tire_radius } else { config.rear_tire_radius };
        let tire_width = if wheel.is_front() { config.front_tire_width } else { config.rear_tire_width };

        if normal_force_n <= 1e-3 {
            // Airborne wheel
            state.slip_angle_rad = 0.0;
            state.slip_ratio = 0.0;
            state.lateral_force = 0.0;
            state.longitudinal_force = 0.0;
            state.rolling_resistance = 0.0;
            state.aligning_torque = 0.0;

            // Spin decays slowly from air friction
            let net_air_torque = drive_torque_nm - brake_torque_nm.min(drive_torque_nm.abs()) * state.spin.signum();
            state.spin += (net_air_torque / state.wheel_moment) * dt;
            state.spin -= state.spin.signum() * (dt * 2.0 / state.wheel_moment);
            return;
        }

        // In wheel local frame:
        // Forward speed is -local_wheel_velocity.z (since -Z is forward)
        // Lateral speed is local_wheel_velocity.x
        let v_forward = -local_wheel_velocity.z;
        let v_lateral = local_wheel_velocity.x;
        let v_wheel = state.spin * tire_radius;

        // 1. Slip angle alpha (rad)
        let v_mag = (v_forward * v_forward + v_lateral * v_lateral).sqrt();
        state.slip_angle_rad = if v_mag > 0.1 {
            (v_lateral / v_mag.max(1.0)).asin().clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2)
        } else {
            0.0
        };

        // 2. Slip ratio kappa (unitless)
        state.spin_velocity_diff = v_wheel - v_forward;
        state.slip_ratio = if v_forward.abs() > 0.1 {
            ((v_wheel - v_forward) / v_forward.abs()).clamp(-2.0, 2.0)
        } else {
            ((v_wheel - v_forward) / 1.0).clamp(-2.0, 2.0)
        };

        // 3. Peak friction and stiffness
        let base_stiffness = 1_000_000.0 + 8_000_000.0 * 5.0; // 41 MN/m
        let contact_patch = config.contact_patch;
        let cornering_stiffness = 0.5 * base_stiffness * contact_patch * contact_patch;
        let long_stiffness = cornering_stiffness * 1.1;

        // Load-dependent friction with slight degressivity (tire_width in mm)
        let tire_width_mm = tire_width * 1000.0;
        let degressivity = normal_force_n / (tire_width_mm * contact_patch * 0.20).max(1e-4);
        let f_max = (effective_friction_coef * normal_force_n - degressivity).max(10.0);

        // 4. Combined Brush Tire Model
        let sigma_x = long_stiffness * state.slip_ratio;
        let sigma_y = -cornering_stiffness * state.slip_angle_rad.tan();
        let sigma_comb = (sigma_x * sigma_x + sigma_y * sigma_y).sqrt();

        let braking_help = if state.slip_ratio < -0.1 && brake_torque_nm > 10.0 {
            1.0 + (config.braking_grip_multiplier - 1.0) * state.slip_ratio.abs().min(1.0)
        } else {
            1.0
        };

        let crit_deflection = 0.5 * f_max * (1.0 - state.slip_ratio.abs()).max(0.1);

        if sigma_comb < crit_deflection {
            // Linear sticking zone
            state.longitudinal_force = sigma_x * braking_help;
            state.lateral_force = sigma_y;
        } else {
            // Non-linear sliding zone with parabolic brush falloff
            let brush_factor = (1.0 - crit_deflection / (3.0 * sigma_comb)).max(0.0);
            let force_comb = f_max * brush_factor * braking_help;
            state.longitudinal_force = force_comb * (sigma_x / sigma_comb.max(1e-4));
            state.lateral_force = force_comb * (sigma_y / sigma_comb.max(1e-4));
        }

        // Aligning torque (self-centering pneumatic trail)
        let trail = contact_patch * 0.167 * (1.0 - (sigma_comb / (3.0 * crit_deflection)).min(1.0));
        state.aligning_torque = -state.lateral_force * trail;

        // 5. Rolling resistance
        let speed_factor = (v_forward * 0.036).powi(2);
        let c_rr = 0.005 + 0.5 * (0.01 + 0.0095 * speed_factor);
        state.rolling_resistance = c_rr * normal_force_n;
        if v_forward.abs() > 0.05 {
            state.longitudinal_force -= state.rolling_resistance * v_forward.signum();
        }

        // 6. Wheel rotational dynamics integration
        let reaction_torque = state.longitudinal_force * tire_radius;
        let mut net_torque = drive_torque_nm - reaction_torque;

        if state.spin.abs() > 0.1 {
            net_torque -= brake_torque_nm * state.spin.signum();
        } else if brake_torque_nm > net_torque.abs() {
            // Brake locks wheel near zero speed
            state.spin = 0.0;
            net_torque = 0.0;
        } else {
            net_torque -= brake_torque_nm * net_torque.signum();
        }

        let spin_accel = net_torque / state.wheel_moment;
        state.spin += spin_accel * dt;
    }
}
