// GEVP-aligned behavior; see THIRD_PARTY_NOTICES.md for upstream MIT attribution.
use crate::types::{VehicleInput, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

const RAD_S_TO_RPM: f64 = 60.0 / (2.0 * std::f64::consts::PI);
const GEVP_GEAR_INERTIA: f64 = 0.02;
const MAX_CLUTCH_TORQUE_RATIO: f64 = 1.6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowertrainState {
    pub rpm: f64,
    pub current_gear: i8,
    pub target_gear: i8,
    pub clutch_engagement: f64,
    pub shift_timer: f64,
    pub engine_torque: f64,
    pub clutch_torque: f64,
    pub is_rev_limited: bool,
    pub drive_inertia: f64,
    pub drive_torques: [f64; 4],
    pub brake_torques: [f64; 4],
    pub abs_active: [bool; 4],
    pub abs_timers: [f64; 4],
}

impl PowertrainState {
    pub fn new(config: &VehicleConfig) -> Self {
        Self {
            rpm: config.idle_rpm,
            current_gear: 1,
            target_gear: 1,
            clutch_engagement: 1.0,
            shift_timer: 0.0,
            engine_torque: 0.0,
            clutch_torque: 0.0,
            is_rev_limited: false,
            drive_inertia: config.motor_moment,
            drive_torques: [0.0; 4],
            brake_torques: [0.0; 4],
            abs_active: [false; 4],
            abs_timers: [0.0; 4],
        }
    }

    /// Compatibility path for callers that do not provide tire reaction torque.
    pub fn step(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_angular_velocities: &[f64; 4],
        forward_speed_m_s: f64,
        dt: f64,
    ) {
        self.step_with_reaction(
            config,
            input,
            wheel_angular_velocities,
            &[0.0; 4],
            forward_speed_m_s,
            dt,
        );
    }

    /// GEVP-style drivetrain update. Tire reaction torque is fed back from the
    /// previous force solve; this is the coupling that makes lift-off and engine
    /// braking continuous instead of switching a wheel between driven/free modes.
    pub fn step_with_reaction(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_spins: &[f64; 4],
        tire_reaction_torques: &[f64; 4],
        forward_speed_m_s: f64,
        dt: f64,
    ) {
        let dt = dt.max(1e-5);
        self.process_shift(config, input, wheel_spins, forward_speed_m_s, dt);
        self.process_engine_and_clutch(config, input, wheel_spins, tire_reaction_torques, dt);
        self.distribute_drive_torque(config, wheel_spins);
        self.process_brakes(config, input, wheel_spins, dt);
    }

    fn process_shift(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_spins: &[f64; 4],
        forward_speed_m_s: f64,
        dt: f64,
    ) {
        if self.shift_timer > 0.0 {
            self.shift_timer = (self.shift_timer - dt).max(0.0);
            if self.shift_timer == 0.0 {
                self.current_gear = self.target_gear;
                let ratio = self.get_total_gear_ratio(config).abs();
                if ratio > 0.0 {
                    let wheel_spin = self.drivetrain_spin(config, wheel_spins).abs();
                    let road_spin = forward_speed_m_s.abs() / config.rear_tire_radius.max(1e-6);
                    let target_spin = if (wheel_spin - road_spin).abs() > 10.0 {
                        road_spin * 0.65 + wheel_spin * 0.35
                    } else {
                        wheel_spin
                    };
                    self.rpm = (target_spin * ratio * RAD_S_TO_RPM)
                        .clamp(config.idle_rpm, config.max_rpm * 1.01);
                }
            }
            return;
        }

        let max_gear = config.gear_ratios.len() as i8;
        let mut desired = self.current_gear;
        if let Some(req) = input.gear_request {
            if req >= -1 && req <= max_gear { desired = req; }
        } else if config.automatic_transmission {
            if self.current_gear <= 0 && forward_speed_m_s >= -0.5 {
                desired = 1;
            } else if self.current_gear > 0 {
                let total_ratio = self.get_total_gear_ratio(config).abs();
                let radius = config.rear_tire_radius;
                let wheel_speed = self.drivetrain_spin(config, wheel_spins).abs() * radius;
                let road_speed = forward_speed_m_s.abs();
                let shift_speed_ratio = if self.current_gear == 1 { 0.45 } else { 0.65 };
                let min_shift_speed = (config.max_rpm * shift_speed_ratio / (total_ratio.max(1e-6) * RAD_S_TO_RPM)) * radius;

                // Traction-aware upshift: prevent wheelspin from triggering premature upshifts,
                // while enforcing redline protection (>= 98% RPM) to prevent over-revving.
                let is_at_redline = self.rpm >= config.max_rpm * 0.98;
                let can_upshift = self.current_gear < max_gear
                    && (
                        (self.rpm > config.max_rpm * 0.92 && (road_speed >= min_shift_speed || (wheel_speed - road_speed) < 6.0))
                        || is_at_redline
                    );

                let lower_ratio = if self.current_gear > 1 {
                    (config.gear_ratios[(self.current_gear - 2) as usize] * config.final_drive).abs()
                } else {
                    0.0
                };
                let lower_rpm_est = (road_speed / radius.max(1e-6)) * lower_ratio * RAD_S_TO_RPM;

                // Intelligent downshift: off-throttle coasting OR kick-down under heavy throttle when genuinely bogged
                let can_downshift = self.current_gear > 1 && (
                    (self.rpm < config.max_rpm * 0.40 && input.throttle < 0.35)
                    || (self.rpm < config.max_rpm * 0.45 && input.throttle > 0.60 && lower_rpm_est < config.max_rpm * 0.88)
                );

                if can_upshift {
                    desired += 1;
                } else if can_downshift {
                    desired -= 1;
                }
            }
            if forward_speed_m_s.abs() < 1.0 && input.brake > 0.75 && self.current_gear == 1 {
                desired = -1;
            }
        }

        if desired != self.current_gear {
            self.target_gear = desired;
            self.shift_timer = config.shift_time.max(0.0);
            if self.shift_timer == 0.0 { self.current_gear = desired; }
        }
    }

    fn process_engine_and_clutch(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_spins: &[f64; 4],
        tire_reaction_torques: &[f64; 4],
        dt: f64,
    ) {
        let throttle = input.throttle.clamp(0.0, 1.0);
        let rpm_factor = (self.rpm / config.max_rpm.max(1.0)).clamp(0.0, 1.0);
        let curve = config.evaluate_torque_curve(rpm_factor);
        let positive_torque = curve * config.max_torque * throttle;

        // GEVP has variable motor drag and a constant motor brake term. These values
        // are derived from the existing Formula-90 config to avoid a schema break.
        let variable_drag = rpm_factor * config.max_torque * 0.10;
        let constant_brake = config.max_torque * 0.02 * (1.0 - throttle);
        let mut torque_output = positive_torque - variable_drag - constant_brake;

        self.is_rev_limited = self.rpm >= config.max_rpm;
        if self.rpm >= config.max_rpm * 1.01 {
            torque_output = torque_output.min(0.0);
        }
        if self.shift_timer > 0.0 {
            torque_output = torque_output.min(0.0);
        }
        self.engine_torque = torque_output;

        // Free engine integration occurs before clutch reaction, as in GEVP.
        self.rpm += RAD_S_TO_RPM * dt * torque_output / config.motor_moment.max(1e-6);
        self.rpm = self.rpm.max(config.idle_rpm);

        if self.current_gear == 0 {
            self.clutch_engagement = 0.0;
            self.clutch_torque = 0.0;
            self.drive_inertia = 0.0;
            return;
        }

        let total_ratio = self.get_total_gear_ratio(config);
        let manual_clutch = input.clutch.clamp(0.0, 1.0);
        let shift_clutch = if self.shift_timer > 0.0 { 1.0 } else { 0.0 };
        // GEVP anti-stall clutch: at idle (or below clutch-out RPM when launching from rest),
        // disengage clutch completely so idle governor does not produce artificial creep torque.
        let clutch_out_rpm = config.idle_rpm + 1000.0;
        let idle_disengagement = if self.rpm <= config.idle_rpm + 50.0 {
            1.0
        } else if self.rpm >= clutch_out_rpm {
            0.0
        } else {
            (clutch_out_rpm - self.rpm) / (clutch_out_rpm - (config.idle_rpm + 50.0))
        };
        let clutch_disengagement = manual_clutch.max(shift_clutch).max(idle_disengagement);
        self.clutch_engagement = 1.0 - clutch_disengagement;

        let drive_axles_inertia = driven_wheel_inertia(config);
        let drivetrain_inertia = config.motor_moment
            + total_ratio.powi(2) * GEVP_GEAR_INERTIA
            + drive_axles_inertia;
        let ratio_sq = total_ratio.powi(2).max(1e-8);
        let drivetrain_inertia_reflected = drivetrain_inertia / ratio_sq;
        let reaction_torque = driven_reaction_torque(config, tire_reaction_torques) / total_ratio;
        let drivetrain_spin = self.drivetrain_spin(config, wheel_spins);
        let mut speed_difference = (self.rpm / RAD_S_TO_RPM) - drivetrain_spin * total_ratio;
        if speed_difference < 0.0 { speed_difference = -(-speed_difference).sqrt(); }

        let a = (config.motor_moment * drivetrain_inertia_reflected * speed_difference) / dt;
        let b = config.motor_moment * reaction_torque;
        let c = drivetrain_inertia_reflected * torque_output;
        let clutch_factor = self.clutch_engagement;
        let max_clutch_torque = config.max_torque * MAX_CLUTCH_TORQUE_RATIO * clutch_factor;
        let raw_clutch = ((a - b + c)
            / (config.motor_moment + drivetrain_inertia_reflected).max(1e-8))
            * clutch_factor;
        self.clutch_torque = raw_clutch.clamp(-max_clutch_torque, max_clutch_torque);

        self.rpm -= RAD_S_TO_RPM * dt * self.clutch_torque / config.motor_moment.max(1e-6);
        self.rpm = self.rpm.clamp(config.idle_rpm, config.max_rpm * 1.01);

        // This is the inertia passed to each driven wheel's torque integration,
        // equivalent in purpose to GEVP process_drive/process_torque.
        self.drive_inertia = config.motor_moment + total_ratio.powi(2) * GEVP_GEAR_INERTIA;
    }

    fn distribute_drive_torque(&mut self, config: &VehicleConfig, wheel_spins: &[f64; 4]) {
        self.drive_torques = [0.0; 4];
        if self.current_gear == 0 { return; }
        let wheel_torque = self.clutch_torque * self.get_total_gear_ratio(config);
        let front_total = wheel_torque * config.front_torque_split;
        let rear_total = wheel_torque * (1.0 - config.front_torque_split);

        // Front axle (open 50/50 split if driven, e.g. AWD)
        if config.front_torque_split > 0.0 {
            self.drive_torques[0] = front_total * 0.5;
            self.drive_torques[1] = front_total * 0.5;
        }

        // Rear axle: Salisbury Clutch-Pack LSD (AMS2 / Reiza aligned 1.5-Way differential)
        if config.front_torque_split < 1.0 {
            let (t_rl, t_rr) = solve_salisbury_differential(
                rear_total,
                wheel_spins[2], // Rear Left
                wheel_spins[3], // Rear Right
                config.diff_preload,
                config.diff_power_ramp_angle_deg,
                config.diff_coast_ramp_angle_deg,
                config.diff_clutches,
                config.diff_clutch_friction_coeff,
            );
            self.drive_torques[2] = t_rl;
            self.drive_torques[3] = t_rr;
        }
    }

    fn process_brakes(&mut self, config: &VehicleConfig, input: &VehicleInput, wheel_spins: &[f64; 4], dt: f64) {
        let total = input.brake.clamp(0.0, 1.0) * config.max_brake_torque;
        let front_each = total * config.front_brake_bias * 0.5;
        let rear_each = total * (1.0 - config.front_brake_bias) * 0.5;
        let hand_each = input.handbrake.clamp(0.0, 1.0) * config.max_brake_torque * 0.4;

        if config.enable_abs && input.brake > 0.05 {
            let avg = wheel_spins.iter().map(|v| v.abs()).sum::<f64>() * 0.25;
            for i in 0..4 {
                if avg - wheel_spins[i].abs() > config.abs_spin_diff_threshold && self.abs_timers[i] <= 0.0 {
                    self.abs_timers[i] = config.abs_pulse_time;
                }
            }
        }
        for i in 0..4 {
            if self.abs_timers[i] > 0.0 {
                self.abs_timers[i] = (self.abs_timers[i] - dt).max(0.0);
                self.abs_active[i] = true;
            } else {
                self.abs_active[i] = false;
            }
        }
        self.brake_torques = [
            if self.abs_active[0] { 0.0 } else { front_each },
            if self.abs_active[1] { 0.0 } else { front_each },
            if self.abs_active[2] { 0.0 } else { rear_each + hand_each },
            if self.abs_active[3] { 0.0 } else { rear_each + hand_each },
        ];
    }

    pub fn get_current_gear_ratio(&self, config: &VehicleConfig) -> f64 {
        if self.current_gear > 0 && (self.current_gear as usize) <= config.gear_ratios.len() {
            config.gear_ratios[(self.current_gear - 1) as usize]
        } else if self.current_gear == -1 {
            -config.reverse_ratio
        } else {
            0.0
        }
    }

    pub fn get_total_gear_ratio(&self, config: &VehicleConfig) -> f64 {
        self.get_current_gear_ratio(config) * config.final_drive
    }

    fn drivetrain_spin(&self, config: &VehicleConfig, wheel_spins: &[f64; 4]) -> f64 {
        let mut sum = 0.0;
        let mut count = 0.0;
        for wheel in WheelIndex::ALL {
            if wheel_is_driven(config, wheel) {
                sum += wheel_spins[wheel as usize];
                count += 1.0;
            }
        }
        if count > 0.0 { sum / count } else { 0.0 }
    }
}

fn wheel_is_driven(config: &VehicleConfig, wheel: WheelIndex) -> bool {
    if wheel.is_front() { config.front_torque_split > 0.0 } else { config.front_torque_split < 1.0 }
}

fn driven_wheel_inertia(config: &VehicleConfig) -> f64 {
    let mut inertia = 0.0;
    for wheel in WheelIndex::ALL {
        if wheel_is_driven(config, wheel) {
            let radius = if wheel.is_front() { config.front_tire_radius } else { config.rear_tire_radius };
            let mass = if wheel.is_front() { config.front_wheel_mass } else { config.rear_wheel_mass };
            inertia += 0.5 * mass * radius * radius;
        }
    }
    inertia
}

fn driven_reaction_torque(config: &VehicleConfig, reactions: &[f64; 4]) -> f64 {
    let mut sum = 0.0;
    for wheel in WheelIndex::ALL {
        if wheel_is_driven(config, wheel) { sum += reactions[wheel as usize]; }
    }
    sum
}

/// Solves torque distribution across a Salisbury clutch-pack limited slip differential (LSD 1.5-Way).
/// Models asymmetric ramp angles (power vs coast), static preload, and multi-plate clutch friction.
/// 
/// Returns `(torque_left, torque_right)`.
pub fn solve_salisbury_differential(
    drive_torque: f64,
    spin_left: f64,
    spin_right: f64,
    preload: f64,
    power_ramp_angle_deg: f64,
    coast_ramp_angle_deg: f64,
    clutches: f64,
    clutch_friction_coeff: f64,
) -> (f64, f64) {
    let half_torque = drive_torque * 0.5;

    // Convert ramp angles from degrees to radians, clamping within safe physical ranges (10° to 85°)
    let deg_to_rad = std::f64::consts::PI / 180.0;
    let power_angle_rad = (power_ramp_angle_deg * deg_to_rad).clamp(10.0 * deg_to_rad, 85.0 * deg_to_rad);
    let coast_angle_rad = (coast_ramp_angle_deg * deg_to_rad).clamp(10.0 * deg_to_rad, 85.0 * deg_to_rad);

    // Ramp clamping force generated by cross-pin wedge action:
    // F_ramp = |T_drive| / (r_cam * tan(theta))
    // Multiplied by clutch pack parameters: T_ramp = F_ramp * N_clutches * mu * r_plate
    // The ratio r_plate / r_cam is approximately 0.90 for standard racing Salisbury differentials.
    let ramp_angle_rad = if drive_torque >= 0.0 { power_angle_rad } else { coast_angle_rad };
    let ramp_tan = ramp_angle_rad.tan().max(0.01);
    let ramp_lock_torque = (drive_torque.abs() / ramp_tan) * clutches.max(1.0) * clutch_friction_coeff.max(0.0) * 0.90;

    // Total maximum friction locking capacity
    let max_locking_capacity = preload.max(0.0) + ramp_lock_torque;

    // Differential wheel speed (rad/s)
    let delta_omega = spin_left - spin_right;

    // Smooth anti-chatter slip transition factor around zero speed delta
    let delta_omega_threshold = 0.50; // rad/s
    let slip_factor = (delta_omega / delta_omega_threshold).clamp(-1.0, 1.0);

    // Cross-transfer locking torque
    let mut delta_torque = max_locking_capacity * slip_factor;

    // Conservative limit: locking torque cannot exceed half of drive torque + preload
    let torque_cap = drive_torque.abs() * 0.5 + preload.max(0.0);
    delta_torque = delta_torque.clamp(-torque_cap, torque_cap);

    // Subtract from faster spinning wheel, transfer to slower wheel with traction
    let torque_left = half_torque - delta_torque;
    let torque_right = half_torque + delta_torque;

    (torque_left, torque_right)
}

