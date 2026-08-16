use crate::types::VehicleInput;
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// Dynamic runtime state of the powertrain, gearbox and brakes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowertrainState {
    pub rpm: f64,
    pub current_gear: i8, // -1 = Reverse, 0 = Neutral, 1..6 = Forward
    pub target_gear: i8,
    pub clutch_engagement: f64, // 0.0 = disengaged, 1.0 = fully engaged
    pub shift_timer: f64,
    pub engine_torque: f64,     // Current output torque from motor in N·m
    pub is_rev_limited: bool,

    // Per-wheel drive and brake torques in N·m
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
            is_rev_limited: false,
            drive_torques: [0.0; 4],
            brake_torques: [0.0; 4],
            abs_active: [false; 4],
            abs_timers: [0.0; 4],
        }
    }

    /// Step powertrain dynamics over timestep dt.
    pub fn step(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_angular_velocities: &[f64; 4], // rad/s for [FL, FR, RL, RR]
        forward_speed_m_s: f64,
        dt: f64,
    ) {
        // 1. Gear shifting logic
        self.process_shifting(config, input, wheel_angular_velocities, forward_speed_m_s, dt);

        // 2. Compute engine RPM & output torque
        self.process_engine(config, input, wheel_angular_velocities, dt);

        // 3. Distribute drive torque to wheels
        self.distribute_drive_torque(config);

        // 4. Compute brake torques with front/rear bias, handbrake and ABS
        self.process_brakes(config, input, wheel_angular_velocities, dt);
    }

    fn process_shifting(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_speeds: &[f64; 4],
        _forward_speed_m_s: f64,
        dt: f64,
    ) {
        let driven_tire_radius = if config.front_torque_split > 0.5 {
            config.front_tire_radius
        } else {
            config.rear_tire_radius
        };
        let driven_avg_spin = if config.front_torque_split > 0.5 {
            (wheel_speeds[0] + wheel_speeds[1]) * 0.5
        } else {
            (wheel_speeds[2] + wheel_speeds[3]) * 0.5
        };

        if self.shift_timer > 0.0 {
            self.shift_timer -= dt;
            if self.shift_timer <= 0.0 {
                self.current_gear = self.target_gear;
                let ratio = self.get_current_gear_ratio(config);
                let synced_rpm = (driven_avg_spin * ratio.abs() * config.final_drive * (60.0 / (2.0 * std::f64::consts::PI))).max(config.idle_rpm);
                self.rpm = synced_rpm;
            }
            return;
        }

        let mut desired_gear = self.current_gear;

        if let Some(req) = input.gear_request {
            // Manual gear request
            let max_gear = config.gear_ratios.len() as i8;
            if req >= -1 && req <= max_gear {
                desired_gear = req;
            }
        } else if config.automatic_transmission {
            // Automatic gear selection based on ideal gear RPM from chassis speed
            let real_wheel_speed_kmh = driven_avg_spin * driven_tire_radius * 3.6;

            if self.current_gear > 0 {
                let max_gear = config.gear_ratios.len() as i8;
                // Upshift when engine is high AND vehicle road speed supports next gear
                if self.rpm > config.max_rpm * 0.90 && self.current_gear < max_gear {
                    desired_gear = self.current_gear + 1;
                }
                // Downshift if RPM drops near idle
                else if self.rpm < (config.idle_rpm + 800.0) && self.current_gear > 1 {
                    desired_gear = self.current_gear - 1;
                }
                // Auto-reverse if stationary/reversing with brake
                if real_wheel_speed_kmh < -1.0 && input.brake > 0.5 && self.current_gear == 1 {
                    desired_gear = -1;
                }
            } else if self.current_gear == -1 && real_wheel_speed_kmh > 1.0 && input.throttle > 0.1 {
                desired_gear = 1;
            }
        }

        if desired_gear != self.current_gear {
            self.target_gear = desired_gear;
            self.shift_timer = config.shift_time;
        }
    }

    fn process_engine(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_speeds: &[f64; 4],
        dt: f64,
    ) {
        let gear_ratio = self.get_current_gear_ratio(config);
        let effective_ratio = gear_ratio * config.final_drive;

        // Driven axle average angular velocity (rad/s)
        let driven_axle_speed = if config.front_torque_split > 0.5 {
            (wheel_speeds[0] + wheel_speeds[1]) * 0.5
        } else {
            (wheel_speeds[2] + wheel_speeds[3]) * 0.5
        };

        let target_rpm_from_wheels = (driven_axle_speed * effective_ratio * (60.0 / (2.0 * std::f64::consts::PI))).abs();

        let throttle = input.throttle.clamp(0.0, 1.0);
        let rpm_span = (config.max_rpm - config.idle_rpm).max(1.0);
        let normalized_rpm = ((self.rpm - config.idle_rpm) / rpm_span).clamp(0.0, 1.0);
        let torque_factor = config.evaluate_torque_curve(normalized_rpm);
        let raw_motor_torque = torque_factor * config.max_torque * throttle;
        let motor_drag = (self.rpm / config.max_rpm) * (config.max_torque * 0.15);

        // Clutch engagement modulation (launch slipping only in 1st gear / reverse at standstill)
        let clutch_out_rpm = config.idle_rpm * 1.6; // Launch bite threshold (~5600 RPM)
        let shift_factor = if self.shift_timer > 0.0 {
            (1.0 - (self.shift_timer / config.shift_time.max(1e-4))).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let bite_factor = if self.current_gear == 0 {
            0.0
        } else if (self.current_gear == 1 || self.current_gear == -1) && target_rpm_from_wheels < clutch_out_rpm {
            ((self.rpm - config.idle_rpm) / (clutch_out_rpm - config.idle_rpm).max(1.0)).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.clutch_engagement = bite_factor * shift_factor * (1.0 - input.clutch.clamp(0.0, 1.0));

        // Direct coupling when clutch is engaged
        if self.clutch_engagement >= 0.85 && self.current_gear != 0 {
            let blended_rpm = target_rpm_from_wheels.max(config.idle_rpm);
            let diff = blended_rpm - self.rpm;
            self.rpm = (self.rpm + diff * (30.0 * dt).min(1.0)).clamp(config.idle_rpm, config.max_rpm + 500.0);
        } else {
            let clutch_load = self.clutch_engagement * (raw_motor_torque * 0.85);
            let net_torque = raw_motor_torque - motor_drag - clutch_load;
            let rpm_accel = (net_torque / config.motor_moment) * (60.0 / (2.0 * std::f64::consts::PI));
            self.rpm = (self.rpm + rpm_accel * dt).clamp(config.idle_rpm, config.max_rpm + 500.0);
        }

        // Rev limiter
        if self.rpm >= config.max_rpm {
            self.is_rev_limited = true;
            self.engine_torque = 0.0;
        } else {
            self.is_rev_limited = false;
            self.engine_torque = raw_motor_torque;
        }
    }

    fn distribute_drive_torque(&mut self, config: &VehicleConfig) {
        if self.current_gear == 0 {
            self.drive_torques = [0.0; 4];
            return;
        }

        let gear_ratio = self.get_current_gear_ratio(config);
        let total_wheel_torque = self.engine_torque * gear_ratio * config.final_drive * self.clutch_engagement;

        let front_torque = total_wheel_torque * config.front_torque_split;
        let rear_torque = total_wheel_torque * (1.0 - config.front_torque_split);

        // Open differential: 50/50 split per axle
        self.drive_torques[0] = front_torque * 0.5; // FL
        self.drive_torques[1] = front_torque * 0.5; // FR
        self.drive_torques[2] = rear_torque * 0.5;  // RL
        self.drive_torques[3] = rear_torque * 0.5;  // RR
    }

    fn process_brakes(
        &mut self,
        config: &VehicleConfig,
        input: &VehicleInput,
        wheel_speeds: &[f64; 4],
        dt: f64,
    ) {
        let total_brake_torque = input.brake.clamp(0.0, 1.0) * config.max_brake_torque;
        let front_torque = total_brake_torque * config.front_brake_bias;
        let rear_torque = total_brake_torque * (1.0 - config.front_brake_bias);

        // Handbrake adds directly to rear wheels
        let handbrake_torque = input.handbrake.clamp(0.0, 1.0) * config.max_brake_torque * 0.8;

        // ABS processing
        if config.enable_abs && input.brake > 0.1 {
            let avg_speed = (wheel_speeds[0].abs() + wheel_speeds[1].abs() + wheel_speeds[2].abs() + wheel_speeds[3].abs()) * 0.25;
            for (i, speed) in wheel_speeds.iter().enumerate().take(4) {
                let slip_diff = avg_speed - speed.abs();
                if slip_diff > config.abs_spin_diff_threshold && self.abs_timers[i] <= 0.0 {
                    self.abs_timers[i] = config.abs_pulse_time;
                }
            }
        }

        for i in 0..4 {
            if self.abs_timers[i] > 0.0 {
                self.abs_timers[i] -= dt;
                self.abs_active[i] = true;
            } else {
                self.abs_active[i] = false;
            }
        }

        // FL & FR
        let fl_brake = if self.abs_active[0] { 0.0 } else { front_torque * 0.5 };
        let fr_brake = if self.abs_active[1] { 0.0 } else { front_torque * 0.5 };

        // RL & RR
        let rl_brake = if self.abs_active[2] { 0.0 } else { (rear_torque + handbrake_torque) * 0.5 };
        let rr_brake = if self.abs_active[3] { 0.0 } else { (rear_torque + handbrake_torque) * 0.5 };

        self.brake_torques = [fl_brake, fr_brake, rl_brake, rr_brake];
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
}
