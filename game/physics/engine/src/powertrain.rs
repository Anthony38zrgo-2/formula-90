// GEVP-aligned behavior; see THIRD_PARTY_NOTICES.md for upstream MIT attribution.
use crate::types::{VehicleInput, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

const RAD_S_TO_RPM: f64 = 60.0 / (2.0 * std::f64::consts::PI);
const GEVP_GEAR_INERTIA: f64 = 0.02;
// Traction control slip limiter: when a driven wheel exceeds the slip threshold,
// scale demanded torque down (proportionally to threshold/slip) so slip settles at
// the threshold instead of saturating to a full cut (which stalls the launch).
// Keep at least this fraction of demanded torque so the car always retains drive.
const MIN_TC_TORQUE_SCALE: f64 = 0.10;

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

    // Traction control (CT / TCS) state
    pub tc_active: bool,
    pub tc_cut_ratio: f64,
    pub tc_cut_ratio_smoothed: f64,
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
            tc_active: false,
            tc_cut_ratio: 0.0,
            tc_cut_ratio_smoothed: 0.0,
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
            true,
            true,
            config.enable_abs,
            dt,
        );
    }

    /// GEVP-style drivetrain update. Tire reaction torque is fed back from the
    /// previous force solve; this is the coupling that makes lift-off and engine
    /// braking continuous instead of switching a wheel between driven/free modes.
    ///
    /// `tc_enabled` gates traction control (CT/TCS): when true, drive torque is
    /// reduced on wheels whose longitudinal slip exceeds the profile threshold.
    pub fn step_with_reaction(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_spins: &[f64; 4],
        tire_reaction_torques: &[f64; 4],
        forward_speed_m_s: f64,
        tc_enabled: bool,
        brake_assist_enabled: bool,
        abs_enabled: bool,
        dt: f64,
    ) {
        let dt = dt.max(1e-5);
        self.process_shift(config, input, wheel_spins, forward_speed_m_s, dt);
        self.process_engine_and_clutch(config, input, wheel_spins, tire_reaction_torques, dt);
        self.distribute_drive_torque(config, wheel_spins, tc_enabled, forward_speed_m_s, dt);
        self.process_brakes(config, input, wheel_spins, dt, brake_assist_enabled, abs_enabled);
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
                    let target_spin = if (wheel_spin - road_spin).abs() > config.automatic_shift.wheel_road_spin_blend_threshold_rads {
                        road_spin * config.automatic_shift.road_spin_weight + wheel_spin * config.automatic_shift.wheel_spin_weight
                    } else {
                        wheel_spin
                    };
                    self.rpm = (target_spin * ratio * RAD_S_TO_RPM)
                        .clamp(config.idle_rpm, config.max_rpm);
                }
            }
            return;
        }

        let max_gear = config.gear_ratios.len() as i8;
        let mut desired = self.current_gear;
        if let Some(req) = input.gear_request {
            if req >= -1 && req <= max_gear { desired = req; }
        } else if config.automatic_transmission {
            let autoshift = &config.automatic_shift;
            if self.current_gear <= 0 && forward_speed_m_s >= -0.5 {
                desired = 1;
            } else if self.current_gear > 0 {
                let total_ratio = self.get_total_gear_ratio(config).abs();
                let radius = config.rear_tire_radius;
                let wheel_speed = self.drivetrain_spin(config, wheel_spins).abs() * radius;
                let road_speed = forward_speed_m_s.abs();

                // Per-throttle normalized-RPM shift thresholds (JSON authoritative).
                let up_norm = if input.throttle > 0.6 {
                    autoshift.upshift_normalized_rpm_full_throttle
                } else if input.throttle < 0.1 {
                    autoshift.upshift_coast_normalized_rpm
                } else {
                    autoshift.upshift_normalized_rpm_low_throttle
                };
                let down_norm = if input.throttle > 0.6 {
                    autoshift.downshift_normalized_rpm_full_throttle
                } else if input.throttle < 0.1 {
                    autoshift.downshift_coast_normalized_rpm
                } else {
                    autoshift.downshift_normalized_rpm_low_throttle
                };

                // Traction-aware gate: don't upshift while the driven wheels are breaking loose.
                let traction_ok = (wheel_speed - road_speed) < autoshift.wheel_road_spin_blend_threshold_rads;

                let is_at_redline = self.rpm >= config.max_rpm * autoshift.upshift_target_rpm_margin_high;
                let mut can_upshift = self.current_gear < max_gear
                    && ((self.rpm > config.max_rpm * up_norm && traction_ok) || is_at_redline);

                // Speed gate (retained from prior logic) prevents lugging / early upshifts in low gears.
                let shift_speed_ratio = if self.current_gear == 1 { 0.45 } else { 0.65 };
                let min_shift_speed = (config.max_rpm * shift_speed_ratio / (total_ratio.max(1e-6) * RAD_S_TO_RPM)) * radius;
                can_upshift = can_upshift && (road_speed >= min_shift_speed || traction_ok);

                let lower_ratio = if self.current_gear > 1 {
                    (config.gear_ratios[(self.current_gear - 2) as usize] * config.final_drive).abs()
                } else { 0.0 };
                let lower_rpm_est = (road_speed / radius.max(1e-6)) * lower_ratio * RAD_S_TO_RPM;

                // Kick-down engages under heavy throttle when bogged; the throttle window scales with
                // aggression, and the engage RPM is relaxed by the (inverse) kick-down delay factor.
                let kickdown_throttle = (0.60 + (1.0 - autoshift.downshift_throttle_aggression_factor) * 0.4).clamp(0.0, 1.0);
                let kickdown_engage_rpm = config.max_rpm
                    * autoshift.downshift_target_rpm_margin_high
                    * (1.0 - (1.0 - autoshift.kickdown_delay_factor) * 0.15);
                let can_downshift = self.current_gear > 1 && (
                    (self.rpm < config.max_rpm * down_norm && input.throttle < 0.35)
                    || (self.rpm < kickdown_engage_rpm && input.throttle > kickdown_throttle && lower_rpm_est < config.max_rpm * 0.88)
                );

                if can_upshift {
                    desired += 1;
                } else if can_downshift {
                    desired -= 1;
                }
            }
            // Reverse only below the max reverse-engage speed; auto-resume to 1st when crawling forward out of park.
            if self.current_gear == 1 && input.brake > 0.75 && forward_speed_m_s.abs() < autoshift.reverse_max_speed_ms {
                desired = -1;
            } else if self.current_gear == -1 && input.throttle > 0.1 && forward_speed_m_s.abs() < autoshift.parked_resume_speed_ms {
                desired = 1;
            }
        }

        if desired != self.current_gear {
            self.target_gear = desired;
            self.shift_timer = (config.shift_time + config.gear_inertia).max(0.0);
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
        // are now profile-tunable via JSON (variable_drag_ratio / constant_brake_ratio).
        let variable_drag = rpm_factor * config.max_torque * config.variable_drag_ratio;
        let constant_brake = config.max_torque * config.constant_brake_ratio * (1.0 - throttle);
        let mut torque_output = positive_torque - variable_drag - constant_brake;

        self.is_rev_limited = self.rpm >= config.max_rpm;
        if self.rpm >= config.max_rpm {
            torque_output = torque_output.min(0.0);
        }
        if self.shift_timer > 0.0 {
            torque_output = torque_output.min(0.0);
        }
        self.engine_torque = torque_output;

        // Free engine integration occurs before clutch reaction, as in GEVP.
        self.rpm += RAD_S_TO_RPM * dt * torque_output / config.motor_moment.max(1e-6);
        self.rpm = self.rpm.clamp(config.idle_rpm, config.max_rpm);

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
        // Hysteresis and offset are now profile-tunable via JSON.
        let clutch_out_rpm = config.idle_rpm + config.clutch_out_rpm_offset;
        let idle_floor = config.idle_rpm + config.idle_disengagement_hysteresis_rpm;
        let idle_disengagement = if self.rpm <= idle_floor {
            1.0
        } else if self.rpm >= clutch_out_rpm {
            0.0
        } else {
            (clutch_out_rpm - self.rpm) / (clutch_out_rpm - idle_floor).max(1e-6)
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
        let max_clutch_torque = config.max_torque * config.max_clutch_torque_ratio * clutch_factor;
        let raw_clutch = ((a - b + c)
            / (config.motor_moment + drivetrain_inertia_reflected).max(1e-8))
            * clutch_factor;
        self.clutch_torque = raw_clutch.clamp(-max_clutch_torque, max_clutch_torque);

        self.rpm -= RAD_S_TO_RPM * dt * self.clutch_torque / config.motor_moment.max(1e-6);
        self.rpm = self.rpm.clamp(config.idle_rpm, config.max_rpm);

        // This is the inertia passed to each driven wheel's torque integration,
        // equivalent in purpose to GEVP process_drive/process_torque.
        self.drive_inertia = config.motor_moment + total_ratio.powi(2) * GEVP_GEAR_INERTIA;
    }

    fn distribute_drive_torque(
        &mut self,
        config: &VehicleConfig,
        wheel_spins: &[f64; 4],
        tc_enabled: bool,
        forward_speed_m_s: f64,
        dt: f64,
    ) {
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
                config.diff_slip_transition_threshold_rad_s,
            );
            self.drive_torques[2] = t_rl;
            self.drive_torques[3] = t_rr;
        }

        // Traction control (CT / TCS): progressively cut drive torque on driven
        // wheels whose longitudinal slip exceeds the profile threshold. This is a
        // pure torque reduction (no braking intervention), matching GEVP's
        // `tcs_active` behavior. Always computed for telemetry, but only applied
        // when tc_enabled is true.
        self.tc_active = false;
        self.tc_cut_ratio = 0.0;
        if config.aids.traction_control_slip_threshold > 0.0
            && self.throttle_input() > 0.01
        {
            let road_speed = forward_speed_m_s.abs();
            let threshold = config.aids.traction_control_slip_threshold;
            let mut max_cut: f64 = 0.0;
            for wheel in WheelIndex::ALL {
                if !wheel_is_driven(config, wheel) { continue; }
                let i = wheel as usize;
                let radius = if wheel.is_front() { config.front_tire_radius } else { config.rear_tire_radius };
                if radius <= 1e-6 { continue; }
                let wheel_road_speed = wheel_spins[i].abs() * radius;
                // Longitudinal slip ratio: (wheel surface speed - vehicle speed) / vehicle speed.
                let denom = road_speed.max(1.0);
                let slip = if forward_speed_m_s >= 0.0 {
                    (wheel_road_speed - road_speed) / denom
                } else {
                    (road_speed - wheel_road_speed) / denom
                };
                if slip > threshold {
                    // Proportional slip limiter: reduce demanded torque so the wheel
                    // slip returns to the threshold. Scale = threshold/slip keeps a
                    // smooth, oscillation-free reduction instead of a bang-bang cut
                    // that fully stalls the launch at low speed.
                    let scale = (threshold / slip).clamp(MIN_TC_TORQUE_SCALE, 1.0);
                    let cut = (1.0 - scale).max(0.0);
                    max_cut = max_cut.max(cut);
                }
            }
        // Smooth the traction-control cut so the limiter does not hunt. The
        // proportional scale is stable per-step, but applying it instantly across
        // the 120 Hz step with a hard threshold created a ~15 Hz torque/slip
        // limit-cycle (DriveTorque swinging ~400<->2100 N·m). `cut_gain` controls
        // responsiveness of the smoothing.
        let smoothing_rate = (config.aids.traction_control_cut_gain * 20.0).clamp(5.0, 50.0);
        let smooth_alpha = (1.0 - (-smoothing_rate * dt).max(-50.0).min(50.0).exp()).clamp(0.0, 1.0);
        let target_cut = if tc_enabled && max_cut > 0.0 { max_cut } else { 0.0 };
        self.tc_cut_ratio_smoothed += (target_cut - self.tc_cut_ratio_smoothed) * smooth_alpha;
        self.tc_cut_ratio = self.tc_cut_ratio_smoothed;
        self.tc_active = tc_enabled && max_cut > 0.0;
        if self.tc_active {
            let scale = (1.0 - self.tc_cut_ratio_smoothed).max(MIN_TC_TORQUE_SCALE);
            for wheel in WheelIndex::ALL {
                if wheel_is_driven(config, wheel) {
                    let i = wheel as usize;
                    self.drive_torques[i] *= scale;
                }
            }
        }
        }
    }

    fn throttle_input(&self) -> f64 {
        // Engine torque sign already encodes commanded drive; derive from stored
        // engine_torque instead of input to avoid borrow conflicts.
        if self.engine_torque > 0.0 { 1.0 } else { 0.0 }
    }

    fn process_brakes(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_spins: &[f64; 4],
        dt: f64,
        brake_assist_enabled: bool,
        abs_enabled: bool,
    ) {
        let assist_mult = if brake_assist_enabled {
            config.aids.brake_assist_force_multiplier.max(0.0)
        } else {
            1.0
        };
        let total = input.brake.clamp(0.0, 1.0) * config.max_brake_torque * assist_mult;
        let front_each = total * config.front_brake_bias * 0.5;
        let rear_each = total * (1.0 - config.front_brake_bias) * 0.5;
        let hand_each = input.handbrake.clamp(0.0, 1.0) * config.max_brake_torque * config.handbrake_torque_fraction;

        if abs_enabled && input.brake > 0.05 {
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
    slip_transition_threshold_rad_s: f64,
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
    let delta_omega_threshold = slip_transition_threshold_rad_s; // rad/s (from f1_94_physics.json)
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

