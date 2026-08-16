// GEVP-aligned behavior; see THIRD_PARTY_NOTICES.md for upstream MIT attribution.
use crate::types::{RaycastHit, SurfaceType, TriRaycastSample, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use serde::{Deserialize, Serialize};

/// GEVP-style suspension state for one wheel. Distances are SI internally;
/// compression_mm is retained for telemetry/API compatibility.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WheelSuspensionState {
    pub spring_current_length: f64,
    pub max_spring_length: f64,
    pub compression_mm: f64,
    pub previous_compression_mm: f64,
    pub spring_speed_mm_s: f64,
    pub spring_force: f64,
    pub damping_force: f64,
    pub antiroll_force: f64,
    pub bottom_out_force: f64,
    pub total_normal_force: f64,
    pub is_grounded: bool,
    pub effective_contact_point: Vec3,
    pub effective_normal: Vec3,
    pub effective_surface: SurfaceType,
    pub effective_friction: f64,
    pub effective_stiffness: f64,
    pub effective_rolling_resistance: f64,
    pub dynamic_camber: f64,
}

impl WheelSuspensionState {
    pub fn new(config: &VehicleConfig, wheel: WheelIndex) -> Self {
        let spring_length = spring_length(config, wheel);
        let resting_ratio = resting_ratio(config, wheel);
        let current_length = spring_length * (1.0 - resting_ratio);
        let static_force = config.mass_over_wheel(wheel) * 9.80665;
        Self {
            spring_current_length: current_length,
            max_spring_length: spring_length,
            compression_mm: (spring_length - current_length) * 1000.0,
            previous_compression_mm: (spring_length - current_length) * 1000.0,
            spring_speed_mm_s: 0.0,
            spring_force: static_force,
            damping_force: 0.0,
            antiroll_force: 0.0,
            bottom_out_force: 0.0,
            total_normal_force: static_force,
            is_grounded: false,
            effective_contact_point: Vec3::ZERO,
            effective_normal: Vec3::UP,
            effective_surface: SurfaceType::Road,
            effective_friction: surface_value(&config.surface_friction, SurfaceType::Road, 1.0),
            effective_stiffness: surface_value(&config.surface_stiffness, SurfaceType::Road, 5.0),
            effective_rolling_resistance: surface_value(&config.surface_rolling_resistance, SurfaceType::Road, 1.0),
            dynamic_camber: camber(config, wheel),
        }
    }
}

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

    /// Three rays are reduced to one effective contact patch using center-biased
    /// weights. Only colliding rays participate, and the surface properties are
    /// blended with the same weights. The force itself is still a single GEVP-style
    /// spring/tire contact force, so three rays do not triple the wheel load.
    pub fn step(&mut self, config: &VehicleConfig, samples: &[TriRaycastSample; 4], dt: f64) {
        let dt = dt.max(1e-5);
        for i in 0..4 {
            self.sample_contact(config, WheelIndex::ALL[i], &samples[i]);
        }

        let compressions = [
            self.wheels[0].compression_mm,
            self.wheels[1].compression_mm,
            self.wheels[2].compression_mm,
            self.wheels[3].compression_mm,
        ];
        self.solve_force(config, WheelIndex::FrontLeft, compressions[1], dt);
        self.solve_force(config, WheelIndex::FrontRight, compressions[0], dt);
        self.solve_force(config, WheelIndex::RearLeft, compressions[3], dt);
        self.solve_force(config, WheelIndex::RearRight, compressions[2], dt);
    }

    fn sample_contact(&mut self, config: &VehicleConfig, wheel: WheelIndex, sample: &TriRaycastSample) {
        let state = &mut self.wheels[wheel as usize];
        let spring_length = spring_length(config, wheel);
        let radius = tire_radius(config, wheel);
        let max_ray_length = spring_length + radius;

        if !sample.has_any_contact() {
            state.is_grounded = false;
            state.spring_current_length = spring_length;
            state.compression_mm = 0.0;
            state.effective_normal = Vec3::UP;
            state.effective_contact_point = Vec3::ZERO;
            state.effective_surface = SurfaceType::Road;
            state.effective_friction = surface_value(&config.surface_friction, SurfaceType::Road, 1.0);
            state.effective_stiffness = surface_value(&config.surface_stiffness, SurfaceType::Road, 5.0);
            state.effective_rolling_resistance = surface_value(&config.surface_rolling_resistance, SurfaceType::Road, 1.0);
            state.dynamic_camber = camber(config, wheel);
            return;
        }

        let distance = sample.weighted_distance(max_ray_length);
        let raw_spring_length = distance - radius;
        // GEVP limits rebound with max_spring_length so a ray hit outside the wheel's
        // reachable suspension range cannot generate a tensile/phantom contact force.
        state.is_grounded = raw_spring_length <= state.max_spring_length + 1e-6;
        state.spring_current_length = raw_spring_length.min(state.max_spring_length).min(spring_length);
        state.compression_mm = (spring_length - state.spring_current_length).max(0.0) * 1000.0;
        state.effective_normal = sample.weighted_normal();
        state.effective_contact_point = weighted_point(sample);
        state.effective_surface = dominant_surface(sample);
        state.effective_friction = blended_surface(config, sample, &config.surface_friction, 1.0);
        state.effective_stiffness = blended_surface(config, sample, &config.surface_stiffness, 5.0);
        state.effective_rolling_resistance = blended_surface(config, sample, &config.surface_rolling_resistance, 1.0);

        let base_camber = camber(config, wheel);
        let span = tire_width(config, wheel) * config.tri_ray_spacing_ratio * 2.0;
        if sample.inner.is_colliding && sample.outer.is_colliding && span > 1e-6 {
            let delta_h = sample.inner.distance - sample.outer.distance;
            let incline = (delta_h / span).atan();
            state.dynamic_camber = base_camber + if wheel.is_right() { -incline } else { incline };
        } else {
            state.dynamic_camber = base_camber;
        }
    }

    fn solve_force(&mut self, config: &VehicleConfig, wheel: WheelIndex, opposite_compression_mm: f64, dt: f64) {
        let state = &mut self.wheels[wheel as usize];
        if !state.is_grounded {
            state.spring_force = 0.0;
            state.damping_force = 0.0;
            state.antiroll_force = 0.0;
            state.bottom_out_force = 0.0;
            state.total_normal_force = 0.0;
            state.spring_speed_mm_s = 0.0;
            state.previous_compression_mm = state.compression_mm;
            state.max_spring_length = spring_length(config, wheel);
            return;
        }

        let spring_k = config.calculate_spring_rate(wheel); // N/m
        let mass = config.mass_over_wheel(wheel);
        let spring_len = spring_length(config, wheel);
        let damping_ratio = if wheel.is_front() { config.front_damping_ratio } else { config.rear_damping_ratio };
        let bump_mult = if wheel.is_front() { config.front_bump_damp_multiplier } else { config.rear_bump_damp_multiplier };
        let rebound_mult = if wheel.is_front() { config.front_rebound_damp_multiplier } else { config.rear_rebound_damp_multiplier };
        let arb_ratio = if wheel.is_front() { config.front_arb_ratio } else { config.rear_arb_ratio };
        let bump_stop_mult = if wheel.is_front() { config.front_bump_stop_multiplier } else { config.rear_bump_stop_multiplier };

        let speed_mm_s = (state.compression_mm - state.previous_compression_mm) / dt;
        state.spring_speed_mm_s = if speed_mm_s.is_finite() { speed_mm_s.clamp(-10000.0, 10000.0) } else { 0.0 };
        state.previous_compression_mm = state.compression_mm;

        let compression_m = state.compression_mm * 0.001;
        state.spring_force = compression_m * spring_k;
        state.antiroll_force = (state.compression_mm - opposite_compression_mm) * 0.001 * spring_k * arb_ratio;

        let critical_c = 2.0 * (spring_k * mass).sqrt();
        let base_c = critical_c * damping_ratio;
        let suspension_velocity_m_s = state.spring_speed_mm_s * 0.001;
        let fast_threshold = 0.127; // 127 mm/s, matching GEVP's default threshold.
        let slow_bump = base_c * bump_mult;
        let fast_bump = slow_bump * 0.5;
        let slow_rebound = base_c * rebound_mult;
        let fast_rebound = slow_rebound * 0.5;
        state.damping_force = if suspension_velocity_m_s > fast_threshold {
            (suspension_velocity_m_s - fast_threshold) * fast_bump + fast_threshold * slow_bump
        } else if suspension_velocity_m_s >= 0.0 {
            suspension_velocity_m_s * slow_bump
        } else if suspension_velocity_m_s < -fast_threshold {
            (suspension_velocity_m_s + fast_threshold) * fast_rebound - fast_threshold * slow_rebound
        } else {
            suspension_velocity_m_s * slow_rebound
        };

        // GEVP has an explicit bottom-out force when spring length becomes negative.
        let bottom_penetration = (-state.spring_current_length).max(0.0);
        state.bottom_out_force = if bottom_penetration > 0.0 {
            (spring_k * bump_stop_mult * bottom_penetration)
                + (mass * 9.80665 * bump_stop_mult)
        } else {
            0.0
        };

        state.total_normal_force = (state.spring_force + state.antiroll_force + state.damping_force + state.bottom_out_force).max(0.0);

        // Equivalent purpose to GEVP max_spring_length: avoid a numerical rebound impulse
        // causing contact to persist farther than the configured suspension travel.
        state.max_spring_length = (((state.total_normal_force / mass.max(1e-6))
            - suspension_velocity_m_s)
            * dt
            + state.spring_current_length)
            .clamp(0.0, spring_len);
    }
}

fn spring_length(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_spring_length } else { config.rear_spring_length }
}
fn resting_ratio(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_resting_ratio } else { config.rear_resting_ratio }
}
fn tire_radius(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_tire_radius } else { config.rear_tire_radius }
}
fn tire_width(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_tire_width } else { config.rear_tire_width }
}
fn camber(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() { config.front_camber } else { config.rear_camber }
}

fn weighted_point(sample: &TriRaycastSample) -> Vec3 {
    let mut sum = Vec3::ZERO;
    let mut weights = 0.0;
    add_point(&mut sum, &mut weights, &sample.inner, 1.0);
    add_point(&mut sum, &mut weights, &sample.center, 2.0);
    add_point(&mut sum, &mut weights, &sample.outer, 1.0);
    if weights > 0.0 { sum / weights } else { Vec3::ZERO }
}

fn add_point(sum: &mut Vec3, weights: &mut f64, hit: &RaycastHit, weight: f64) {
    if hit.is_colliding {
        *sum += hit.point * weight;
        *weights += weight;
    }
}

fn dominant_surface(sample: &TriRaycastSample) -> SurfaceType {
    if sample.center.is_colliding {
        sample.center.surface
    } else if sample.inner.is_colliding && sample.outer.is_colliding {
        if sample.inner.distance <= sample.outer.distance { sample.inner.surface } else { sample.outer.surface }
    } else if sample.inner.is_colliding {
        sample.inner.surface
    } else {
        sample.outer.surface
    }
}

fn blended_surface(
    _config: &VehicleConfig,
    sample: &TriRaycastSample,
    map: &std::collections::HashMap<SurfaceType, f64>,
    fallback: f64,
) -> f64 {
    let mut sum = 0.0;
    let mut weights = 0.0;
    if sample.inner.is_colliding {
        sum += surface_value(map, sample.inner.surface, fallback);
        weights += 1.0;
    }
    if sample.center.is_colliding {
        sum += 2.0 * surface_value(map, sample.center.surface, fallback);
        weights += 2.0;
    }
    if sample.outer.is_colliding {
        sum += surface_value(map, sample.outer.surface, fallback);
        weights += 1.0;
    }
    if weights > 0.0 { sum / weights } else { fallback }
}

fn surface_value(map: &std::collections::HashMap<SurfaceType, f64>, surface: SurfaceType, fallback: f64) -> f64 {
    *map.get(&surface).unwrap_or(&fallback)
}
