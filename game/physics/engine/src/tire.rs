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
    #[allow(clippy::too_many_arguments)]
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
        effective_gear_ratio: f64,
        clutch_engagement: f64,
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

            // Spin decays slowly from air friction - include reflected inertia if driven
            let is_driven_air = drive_torque_nm.abs() > 1e-3;
            let reflected_air = if is_driven_air {
                let effective_ratio = effective_gear_ratio * config.final_drive;
                0.5 * config.motor_moment * effective_ratio * effective_ratio
                    * clutch_engagement.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let total_inertia_air = state.wheel_moment + reflected_air;
            let net_air_torque = drive_torque_nm - brake_torque_nm.min(drive_torque_nm.abs()) * state.spin.signum();
            state.spin += (net_air_torque / total_inertia_air) * dt;
            state.spin -= state.spin.signum() * (dt * 2.0 / total_inertia_air);
            return;
        }

        // In wheel local frame:
        // Forward speed is -local_wheel_velocity.z (since -Z is forward)
        // Lateral speed is local_wheel_velocity.x
        let v_forward = -local_wheel_velocity.z;
        let v_lateral = local_wheel_velocity.x;
        let v_wheel = state.spin * tire_radius;

        // 1. Slip angle alpha (rad) - regularized low-speed using atan2 with 0.25 m/s regularization
        // Spec Item 4: alpha = -atan2(v_lateral, sqrt(v_forward^2 + 0.25^2))
        state.slip_angle_rad = -((v_lateral).atan2((v_forward * v_forward + 0.25 * 0.25).sqrt()))
            .clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);

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

        // 4. Combined Brush Tire Model - C1-continuous Fiala/Svendenius unified (Spec Item 2)
        let sigma_x = long_stiffness * state.slip_ratio;
        let sigma_y = -cornering_stiffness * state.slip_angle_rad.tan();
        let sigma_comb = (sigma_x * sigma_x + sigma_y * sigma_y).sqrt();

        let braking_help = if state.slip_ratio < -0.1 && brake_torque_nm > 10.0 {
            1.0 + (config.braking_grip_multiplier - 1.0) * state.slip_ratio.abs().min(1.0)
        } else {
            1.0
        };

        // Unified brush: psi = C_comb * sigma_comb / (3 * mu * Fz), C_comb implicit =1.0 (sigma already includes stiffness)
        let mu_static = effective_friction_coef;
        let mu_kinetic = 0.90 * mu_static;
        // psi uses pure mu*Fz per spec; f_max with degressivity retained for legacy tractive clamp
        let psi: f64;
        let mut fx_brush: f64;
        let fy_brush: f64;
        if sigma_comb < 1e-9 {
            fx_brush = 0.0;
            fy_brush = 0.0;
            psi = 0.0;
        } else {
            psi = sigma_comb / (3.0 * mu_static * normal_force_n.max(1e-4));
            if psi < 1.0 {
                // C1-continuous: F_comb = 3*mu*Fz*psi*(1 - psi + 1/3 psi^2)
                // At psi=1, F_comb = mu*Fz, matching static limit; kinetic branch gives 0.9*mu*Fz (10% step intentional, old 33% eliminated)
                let f_comb = 3.0 * mu_static * normal_force_n * psi * (1.0 - psi + (1.0 / 3.0) * psi * psi);
                let f_comb_help = f_comb * braking_help;
                fx_brush = f_comb_help * (sigma_x / sigma_comb.max(1e-4));
                fy_brush = f_comb_help * (sigma_y / sigma_comb.max(1e-4));
            } else {
                // Sliding: kinetic friction 0.90*mu_static
                let f_comb = mu_kinetic * normal_force_n * braking_help;
                fx_brush = f_comb * (sigma_x / sigma_comb.max(1e-4));
                fy_brush = f_comb * (sigma_y / sigma_comb.max(1e-4));
            }
        }
        state.longitudinal_force = fx_brush;
        state.lateral_force = fy_brush;

        // Tractive & rolling force limit based on drive torque and rolling inertia (GEVP parity)
        // Kept for parity but after unified brush; clamped to max(drive_force, inertia_force) to avoid hiding brush errors.
        if brake_torque_nm <= 10.0 {
            let inertia_force = (state.spin_velocity_diff.abs() * state.wheel_moment) / (tire_radius * tire_radius * dt.max(1e-4));
            let drive_force = drive_torque_nm.abs() / tire_radius;
            let max_avail_force = drive_force.max(inertia_force).min(f_max);
            state.longitudinal_force = state.longitudinal_force.clamp(-max_avail_force, max_avail_force);
            // Keep fx_brush in sync with clamped value for torque reaction (chassis-consistent)
            fx_brush = state.longitudinal_force;
        } else {
            // Braking: no clamp, fx_brush already equals longitudinal_force
            fx_brush = state.longitudinal_force;
        }

        // Aligning torque (self-centering pneumatic trail) - updated to use psi
        let trail = contact_patch * 0.167 * (1.0 - psi.min(1.0));
        state.aligning_torque = -state.lateral_force * trail;

        // 5. Rolling resistance
        let speed_factor = (v_forward * 0.036).powi(2);
        let c_rr = 0.005 + 0.5 * (0.01 + 0.0095 * speed_factor);
        state.rolling_resistance = c_rr * normal_force_n;
        // Capture fx before rolling subtraction for wheel torque reaction (Spec Item 1)
        let fx_for_torque = fx_brush;
        let rolling_torque_mag = state.rolling_resistance * tire_radius;
        if v_forward.abs() > 0.05 {
            state.longitudinal_force -= state.rolling_resistance * v_forward.signum();
        }

        // 6. Wheel rotational dynamics integration - Item 1 & 3: longitudinal reaction torque and dynamic reflected inertia
        // Spec: I_w * dot(omega) = T_drive - T_brake*sgn(omega) - F_x*R_tire - T_rr
        // where I_ref = 0.5*I_engine*(gear_ratio*final_drive)^2 * clutch_engagement
        let is_driven = drive_torque_nm.abs() > 1e-3;
        let reflected_motor_inertia = if is_driven {
            let effective_ratio = effective_gear_ratio * config.final_drive;
            0.5 * config.motor_moment * effective_ratio * effective_ratio
                * clutch_engagement.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let total_wheel_inertia = state.wheel_moment + reflected_motor_inertia;

        let mut net_torque = drive_torque_nm;

        if is_driven {
            if state.spin.abs() > 0.01 {
                net_torque -= brake_torque_nm * state.spin.signum();
            } else if drive_torque_nm.abs() <= brake_torque_nm {
                net_torque = 0.0;
            }
            // Subtract longitudinal reaction torque Fx*R and rolling resistance torque T_rr = F_rr*R
            net_torque -= fx_for_torque * tire_radius;
            if v_forward.abs() > 0.05 {
                net_torque -= rolling_torque_mag * v_forward.signum();
            }
            let prev_spin = state.spin;
            let spin_accel = net_torque / total_wheel_inertia;
            let mut new_spin = state.spin + spin_accel * dt;

            // Dissipative braking: brakes cannot reverse wheel rotation direction
            if prev_spin != 0.0 && prev_spin.signum() != new_spin.signum() && brake_torque_nm > drive_torque_nm.abs() {
                new_spin = 0.0;
            }

            // Contact patch damping: prevents high-frequency 60 Hz numerical resonance
            if normal_force_n > 10.0 {
                let road_spin = v_forward / tire_radius;
                let slip_diff = new_spin - road_spin;
                let damping_rate = (normal_force_n * 0.08).min(350.0);
                let damp_factor = (1.0 - (-damping_rate * dt / total_wheel_inertia.max(0.1)).exp()).min(0.15);
                new_spin -= slip_diff * damp_factor;
            }

            state.spin = new_spin;
        } else {
            // Free-rolling wheel driven by road contact
            if brake_torque_nm > 10.0 {
                // Braking applied to non-driven wheel
                let prev_spin = state.spin;
                let brake_accel = (brake_torque_nm / state.wheel_moment) * state.spin.signum();
                let mut new_spin = state.spin - brake_accel * dt;
                if prev_spin != 0.0 && prev_spin.signum() != new_spin.signum() {
                    new_spin = 0.0;
                }
                state.spin = new_spin;
            } else {
                // Free rolling: spins with road contact
                let ideal_spin = v_forward / tire_radius;
                state.spin = ideal_spin;
            }
        }
    }
}
