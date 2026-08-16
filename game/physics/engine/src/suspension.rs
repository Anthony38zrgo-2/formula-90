use crate::types::{SurfaceType, TriRaycastSample, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// Dynamic state of a single wheel's suspension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelSuspensionState {
    pub spring_current_length: f64,
    pub compression_mm: f64,
    pub previous_compression_mm: f64,
    pub spring_speed_mm_s: f64,
    pub spring_force: f64,        // N
    pub damping_force: f64,       // N
    pub antiroll_force: f64,      // N
    pub bottom_out_force: f64,     // N
    pub total_normal_force: f64,  // N
    pub is_grounded: bool,
    pub effective_contact_point: Vec3,
    pub effective_normal: Vec3,
    pub effective_surface: SurfaceType,
    pub effective_friction: f64,
    pub dynamic_camber: f64,      // radians
}

impl WheelSuspensionState {
    pub fn new(config: &VehicleConfig, wheel: WheelIndex) -> Self {
        let max_len = if wheel.is_front() {
            config.front_spring_length
        } else {
            config.rear_spring_length
        };
        let rest_ratio = if wheel.is_front() {
            config.front_resting_ratio
        } else {
            config.rear_resting_ratio
        };
        let nominal_len = max_len * (1.0 - rest_ratio);
        let base_camber = if wheel.is_front() {
            config.front_camber
        } else {
            config.rear_camber
        };

        Self {
            spring_current_length: nominal_len,
            compression_mm: (max_len - nominal_len) * 1000.0,
            previous_compression_mm: (max_len - nominal_len) * 1000.0,
            spring_speed_mm_s: 0.0,
            spring_force: 0.0,
            damping_force: 0.0,
            antiroll_force: 0.0,
            bottom_out_force: 0.0,
            total_normal_force: 0.0,
            is_grounded: false,
            effective_contact_point: Vec3::ZERO,
            effective_normal: Vec3::UP,
            effective_surface: SurfaceType::Road,
            effective_friction: 2.0,
            dynamic_camber: base_camber,
        }
    }
}

/// Suspension system manager simulating all 4 wheels with transverse tri-raycast integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspensionSystem {
    pub wheels: [WheelSuspensionState; 4],
}

impl SuspensionSystem {
    pub fn new(config: &VehicleConfig) -> Self {
        Self {
            wheels: [
                WheelSuspensionState::new(config, WheelIndex::FrontLeft),
                WheelSuspensionState::new(config, WheelIndex::FrontRight),
                WheelSuspensionState::new(config, WheelIndex::RearLeft),
                WheelSuspensionState::new(config, WheelIndex::RearRight),
            ],
        }
    }

    /// Update suspension forces for all 4 wheels over timestep dt.
    pub fn step(
        &mut self,
        config: &VehicleConfig,
        samples: &[TriRaycastSample; 4],
        dt: f64,
    ) {
        // 1. Process individual wheel tri-raycast samples and spring compression
        for i in 0..4 {
            let wheel = WheelIndex::ALL[i];
            self.process_wheel_compression(config, wheel, &samples[i], dt);
        }

        // 2. Process anti-roll bar (ARB) coupling and damping forces
        let fl_comp = self.wheels[0].compression_mm;
        let fr_comp = self.wheels[1].compression_mm;
        let rl_comp = self.wheels[2].compression_mm;
        let rr_comp = self.wheels[3].compression_mm;

        self.process_wheel_forces(config, WheelIndex::FrontLeft, fr_comp, dt);
        self.process_wheel_forces(config, WheelIndex::FrontRight, fl_comp, dt);
        self.process_wheel_forces(config, WheelIndex::RearLeft, rr_comp, dt);
        self.process_wheel_forces(config, WheelIndex::RearRight, rl_comp, dt);
    }

    fn process_wheel_compression(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        sample: &TriRaycastSample,
        _dt: f64,
    ) {
        let idx = wheel as usize;
        let state = &mut self.wheels[idx];

        let spring_length = if wheel.is_front() {
            config.front_spring_length
        } else {
            config.rear_spring_length
        };
        let tire_radius = if wheel.is_front() {
            config.front_tire_radius
        } else {
            config.rear_tire_radius
        };
        let max_ray_dist = spring_length + tire_radius;

        if sample.has_any_contact() {
            state.is_grounded = true;
            let weighted_dist = sample.weighted_distance(max_ray_dist);
            state.spring_current_length = (weighted_dist - tire_radius).clamp(0.0, spring_length);
            state.effective_normal = sample.weighted_normal();

            // Contact point: prefer center ray, fallback to any active ray
            state.effective_contact_point = if sample.center.is_colliding {
                sample.center.point
            } else if sample.inner.is_colliding {
                sample.inner.point
            } else {
                sample.outer.point
            };

            // Blended surface type & friction
            state.effective_surface = sample.center.surface;
            state.effective_friction = Self::calculate_blended_friction(config, sample);

            // Dynamic camber from transverse ground inclination
            let base_camber = if wheel.is_front() { config.front_camber } else { config.rear_camber };
            let tire_width = if wheel.is_front() { config.front_tire_width } else { config.rear_tire_width };
            let lateral_span = tire_width * config.tri_ray_spacing_ratio * 2.0;

            if sample.inner.is_colliding && sample.outer.is_colliding && lateral_span > 1e-4 {
                let delta_h = sample.inner.distance - sample.outer.distance;
                let ground_incline = (delta_h / lateral_span).atan();
                state.dynamic_camber = base_camber + ground_incline;
            } else {
                state.dynamic_camber = base_camber;
            }
        } else {
            state.is_grounded = false;
            state.spring_current_length = spring_length;
            state.effective_normal = Vec3::UP;
            state.dynamic_camber = if wheel.is_front() { config.front_camber } else { config.rear_camber };
            state.effective_surface = SurfaceType::Road;
            state.effective_friction = *config.surface_friction.get(&SurfaceType::Road).unwrap_or(&2.0);
        }

        state.compression_mm = (spring_length - state.spring_current_length) * 1000.0;
    }

    fn process_wheel_forces(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        opposite_compression_mm: f64,
        dt: f64,
    ) {
        let idx = wheel as usize;
        let state = &mut self.wheels[idx];

        if !state.is_grounded {
            state.spring_force = 0.0;
            state.damping_force = 0.0;
            state.antiroll_force = 0.0;
            state.bottom_out_force = 0.0;
            state.total_normal_force = 0.0;
            state.spring_speed_mm_s = 0.0;
            state.previous_compression_mm = state.compression_mm;
            return;
        }

        // Suspension velocity (positive = compressing into bump, negative = rebounding)
        state.spring_speed_mm_s = (state.compression_mm - state.previous_compression_mm) / dt.max(1e-4);
        state.previous_compression_mm = state.compression_mm;

        let spring_rate_n_m = config.calculate_spring_rate(wheel);
        let spring_length_m = if wheel.is_front() { config.front_spring_length } else { config.rear_spring_length };

        // Linear spring force
        state.spring_force = (state.compression_mm * 0.001) * spring_rate_n_m;

        // Anti-roll bar (ARB) force coupling
        let arb_ratio = if wheel.is_front() { config.front_arb_ratio } else { config.rear_arb_ratio };
        let arb_stiffness = spring_rate_n_m * arb_ratio;
        state.antiroll_force = ((state.compression_mm - opposite_compression_mm) * 0.001) * arb_stiffness;
        state.spring_force += state.antiroll_force;

        // Damping force (critical damping ratio c = 2 * damping_ratio * sqrt(k * m))
        let mass = config.mass_over_wheel(wheel);
        let c_crit = 2.0 * (spring_rate_n_m * mass).sqrt();
        let damp_ratio = if wheel.is_front() { config.front_damping_ratio } else { config.rear_damping_ratio };
        let base_damp = c_crit * damp_ratio; // N·s/m

        let bump_mult = if wheel.is_front() { config.front_bump_damp_multiplier } else { config.rear_bump_damp_multiplier };
        let reb_mult = if wheel.is_front() { config.front_rebound_damp_multiplier } else { config.rear_rebound_damp_multiplier };

        let v_m_s = state.spring_speed_mm_s * 0.001;
        state.damping_force = if v_m_s >= 0.0 {
            v_m_s * base_damp * bump_mult
        } else {
            v_m_s * base_damp * reb_mult
        };

        // Bottom-out bump-stop progressive force if spring travel <= 5%
        let max_comp_mm = spring_length_m * 1000.0;
        let bump_stop_mult = if wheel.is_front() { config.front_bump_stop_multiplier } else { config.rear_bump_stop_multiplier };

        if state.compression_mm >= max_comp_mm * 0.95 {
            let penetration = ((state.compression_mm - max_comp_mm * 0.95) / (max_comp_mm * 0.05)).clamp(0.0, 2.0);
            let bump_stop_k = spring_rate_n_m * bump_stop_mult * 4.0;
            state.bottom_out_force = (penetration * max_comp_mm * 0.001) * bump_stop_k;
        } else {
            state.bottom_out_force = 0.0;
        }

        state.total_normal_force = (state.spring_force + state.damping_force + state.bottom_out_force).max(0.0);
    }

    fn calculate_blended_friction(config: &VehicleConfig, sample: &TriRaycastSample) -> f64 {
        let mu_in = *config.surface_friction.get(&sample.inner.surface).unwrap_or(&2.0);
        let mu_mid = *config.surface_friction.get(&sample.center.surface).unwrap_or(&2.0);
        let mu_out = *config.surface_friction.get(&sample.outer.surface).unwrap_or(&2.0);

        let w_in = if sample.inner.is_colliding { 1.0 } else { 0.0 };
        let w_mid = if sample.center.is_colliding { 2.0 } else { 0.0 };
        let w_out = if sample.outer.is_colliding { 1.0 } else { 0.0 };
        let sum_w = w_in + w_mid + w_out;

        if sum_w > 0.0 {
            (mu_in * w_in + mu_mid * w_mid + mu_out * w_out) / sum_w
        } else {
            mu_mid
        }
    }
}
