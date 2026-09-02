// Mechanical suspension + tricast tire-carcass model.
//
// Godot still supplies three transverse raycasts per wheel. Rust turns them into a
// virtual contact patch and integrates a 1-DOF unsprung wheel between the road/tire
// carcass and the chassis suspension. This removes the old direct raycast->damper
// impulse path while preserving the existing FFI shape.
use crate::tire_thermals::TireMechanicalModifiers;
use crate::types::{RaycastHit, SurfaceType, TriRaycastSample, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use crate::wheel_mechanics::{
    static_wheel_load, tire_radius, tire_width, WheelMechanicalTuning,
};
use serde::{Deserialize, Serialize};

/// Mechanical state for one wheel. Distances are SI internally; legacy millimetre
/// fields remain for telemetry/API compatibility.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WheelSuspensionState {
    /// spring_length - suspension_compression_m. May become negative during bottom-out.
    pub spring_current_length: f64,
    /// Compatibility field: current usable spring length clamped to geometric travel.
    pub max_spring_length: f64,
    pub compression_mm: f64,
    pub previous_compression_mm: f64,
    pub spring_speed_mm_s: f64,
    pub spring_force: f64,
    pub damping_force: f64,
    pub antiroll_force: f64,
    pub bottom_out_force: f64,
    /// Road/tire normal load. This is the Fz consumed by the tire model and applied
    /// externally to the single vehicle rigid body.
    pub total_normal_force: f64,
    /// Force transmitted by the suspension element itself; exposed for telemetry.
    #[serde(default)]
    pub chassis_suspension_force: f64,
    pub is_grounded: bool,
    pub effective_contact_point: Vec3,
    pub effective_normal: Vec3,
    pub effective_surface: SurfaceType,
    pub effective_friction: f64,
    pub effective_stiffness: f64,
    pub effective_rolling_resistance: f64,
    /// Effective camber (rad) at the contact patch — kinematic camber combined with
    /// the road-plane (tri-ray footprint) contribution. Consumed by the tire model
    /// and thermal inputs via `dynamic_camber` (kept in sync).
    #[serde(default)]
    pub effective_contact_camber_rad: f64,
    /// Wheel-kinematic camber (rad): static camber plus the camber gain applied to
    /// suspension travel relative to rest. Never includes the road-plane incline.
    #[serde(default)]
    pub kinematic_camber_rad: f64,
    pub dynamic_camber: f64,

    // --- New mechanical wheel state ---
    /// Fraction of the 1:2:1 tricast footprint currently touching [0, 1].
    #[serde(default)]
    pub contact_fraction: f64,
    /// Per-ray support state in Inner / Center / Outer order (tricast thermal zones).
    #[serde(default)]
    pub ray_grounded: [bool; 3],
    /// Individual geometric compression targets from Inner/Center/Outer rays.
    #[serde(default)]
    pub ray_compressions_m: [f64; 3],
    /// Filtered road compression target seen by the virtual wheel center.
    #[serde(default)]
    pub road_compression_m: f64,
    #[serde(default)]
    pub previous_road_compression_m: f64,
    #[serde(default)]
    pub road_velocity_m_s: f64,
    /// Actual virtual suspension compression (wheel-center DOF).
    #[serde(default)]
    pub suspension_compression_m: f64,
    #[serde(default)]
    pub unsprung_velocity_m_s: f64,
    #[serde(default)]
    pub unsprung_acceleration_m_s2: f64,
    /// Radial tire carcass state.
    #[serde(default)]
    pub tire_deflection_m: f64,
    #[serde(default)]
    pub tire_deflection_velocity_m_s: f64,
    #[serde(default)]
    pub tire_vertical_force: f64,
    /// Internal initialization guard prevents a startup drop when the car is spawned on ground.
    #[serde(default)]
    mechanical_initialized: bool,
}

impl WheelSuspensionState {
    pub fn new(config: &VehicleConfig, wheel: WheelIndex) -> Self {
        let spring_length = spring_length(config, wheel);
        let resting_ratio = resting_ratio(config, wheel);
        let compression = spring_length * resting_ratio;
        let current_length = spring_length - compression;
        let static_force = static_wheel_load(config, wheel);
        Self {
            spring_current_length: current_length,
            max_spring_length: current_length,
            compression_mm: compression * 1000.0,
            previous_compression_mm: compression * 1000.0,
            spring_speed_mm_s: 0.0,
            spring_force: static_force,
            damping_force: 0.0,
            antiroll_force: 0.0,
            bottom_out_force: 0.0,
            total_normal_force: static_force,
            chassis_suspension_force: static_force,
            is_grounded: false,
            effective_contact_point: Vec3::ZERO,
            effective_normal: Vec3::UP,
            effective_surface: SurfaceType::Road,
            effective_friction: surface_value(&config.surface_friction, SurfaceType::Road, 1.0),
            effective_stiffness: surface_value(&config.surface_stiffness, SurfaceType::Road, 5.0),
            effective_rolling_resistance: surface_value(
                &config.surface_rolling_resistance,
                SurfaceType::Road,
                1.0,
            ),
            dynamic_camber: camber(config, wheel),
            effective_contact_camber_rad: camber(config, wheel),
            kinematic_camber_rad: camber(config, wheel),
            contact_fraction: 0.0,
            ray_grounded: [false; 3],
            ray_compressions_m: [0.0; 3],
            road_compression_m: compression,
            previous_road_compression_m: compression,
            road_velocity_m_s: 0.0,
            suspension_compression_m: compression,
            unsprung_velocity_m_s: 0.0,
            unsprung_acceleration_m_s2: 0.0,
            tire_deflection_m: 0.0,
            tire_deflection_velocity_m_s: 0.0,
            tire_vertical_force: static_force,
            mechanical_initialized: false,
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

    /// Convert the three transverse raycasts into a contact-patch target, then integrate
    /// the virtual unsprung wheel/tire against the chassis suspension.
    ///
    /// Compatibility entry point: uses neutral (identity) pressure/thermal modifiers.
    pub fn step(&mut self, config: &VehicleConfig, samples: &[TriRaycastSample; 4], dt: f64) {
        self.step_with_modifiers(config, &[TireMechanicalModifiers::identity(); 4], samples, dt);
    }

    /// Same as Self::step but applies pressure/thermal mechanical modifiers to the
    /// radial tire carcass (stiffness, damping, max deflection). Modifiers are computed
    /// BEFORE forces (current tick pressure); thermal state updates afterwards so the
    /// new pressure only affects the next tick.
    pub fn step_with_modifiers(
        &mut self,
        config: &VehicleConfig,
        modifiers: &[TireMechanicalModifiers; 4],
        samples: &[TriRaycastSample; 4],
        dt: f64,
    ) {
        let dt = dt.max(1e-5);
        for i in 0..4 {
            self.sample_contact(config, WheelIndex::ALL[i], &samples[i], dt);
        }

        // ARB must use one coherent compression snapshot, not values modified by solve order.
        let compressions_m = [
            self.wheels[0].suspension_compression_m,
            self.wheels[1].suspension_compression_m,
            self.wheels[2].suspension_compression_m,
            self.wheels[3].suspension_compression_m,
        ];
        self.solve_force(config, WheelIndex::FrontLeft, modifiers[0], compressions_m[1], dt);
        self.solve_force(config, WheelIndex::FrontRight, modifiers[1], compressions_m[0], dt);
        self.solve_force(config, WheelIndex::RearLeft, modifiers[2], compressions_m[3], dt);
        self.solve_force(config, WheelIndex::RearRight, modifiers[3], compressions_m[2], dt);
    }

    fn sample_contact(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        sample: &TriRaycastSample,
        dt: f64,
    ) {
        let state = &mut self.wheels[wheel as usize];
        let spring_len = spring_length(config, wheel);
        let radius = tire_radius(config, wheel);
        let max_ray_length = spring_len + radius;
        let tuning = WheelMechanicalTuning::for_wheel(config, wheel);

        state.ray_compressions_m = [
            ray_compression(&sample.inner, max_ray_length),
            ray_compression(&sample.center, max_ray_length),
            ray_compression(&sample.outer, max_ray_length),
        ];
        state.ray_grounded = [
            sample.inner.is_colliding,
            sample.center.is_colliding,
            sample.outer.is_colliding,
        ];
        state.contact_fraction = tricast_contact_fraction(sample);

        let target = if sample.has_any_contact() {
            // Geometry remains an active-hit weighted average, while contact_fraction retains
            // how much of the tread is supported. Tire stiffness is scaled by that fraction.
            let distance = sample.weighted_distance(max_ray_length).min(max_ray_length);
            (max_ray_length - distance).max(0.0)
        } else {
            0.0
        };

        if !state.mechanical_initialized {
            state.road_compression_m = target;
            state.previous_road_compression_m = target;
            if sample.has_any_contact() {
                let static_deflection = static_wheel_load(config, wheel)
                    / tuning.tire_vertical_stiffness_n_m.max(1.0);
                state.suspension_compression_m =
                    (target - static_deflection).clamp(0.0, spring_len);
                state.compression_mm = state.suspension_compression_m * 1000.0;
                state.previous_compression_mm = state.compression_mm;
            }
            state.mechanical_initialized = true;
        } else {
            state.previous_road_compression_m = state.road_compression_m;
            let tau = tuning.road_height_filter_tau_s.max(1e-5);
            let alpha = 1.0 - (-dt / tau).exp();
            state.road_compression_m += (target - state.road_compression_m) * alpha;
        }

        state.road_velocity_m_s = ((state.road_compression_m
            - state.previous_road_compression_m)
            / dt)
            .clamp(-tuning.max_road_velocity_m_s, tuning.max_road_velocity_m_s);

        if !sample.has_any_contact() {
            state.effective_normal = Vec3::UP;
            state.effective_contact_point = Vec3::ZERO;
            state.effective_surface = SurfaceType::Road;
            state.effective_friction =
                surface_value(&config.surface_friction, SurfaceType::Road, 1.0);
            state.effective_stiffness =
                surface_value(&config.surface_stiffness, SurfaceType::Road, 5.0);
            state.effective_rolling_resistance = surface_value(
                &config.surface_rolling_resistance,
                SurfaceType::Road,
                1.0,
            );
            // No road plane: effective camber equals the wheel-kinematic camber derived
            // from the current suspension travel. Continuous with the contact branch
            // because the compression state itself is continuous.
            let travel_delta_m = state.suspension_compression_m
                - spring_len * resting_ratio(config, wheel);
            let kinematic_camber =
                camber(config, wheel) + camber_gain(config, wheel) * travel_delta_m;
            state.kinematic_camber_rad = kinematic_camber;
            state.dynamic_camber = kinematic_camber;
            state.effective_contact_camber_rad = kinematic_camber;
            return;
        }

        state.effective_normal = sample.weighted_normal();
        state.effective_contact_point = weighted_point(sample);
        state.effective_surface = dominant_surface(sample);
        state.effective_friction =
            blended_surface(sample, &config.surface_friction, 1.0);
        state.effective_stiffness =
            blended_surface(sample, &config.surface_stiffness, 5.0);
        state.effective_rolling_resistance =
            blended_surface(sample, &config.surface_rolling_resistance, 1.0);

        let base_camber = camber(config, wheel);
        let travel_delta_m =
            state.suspension_compression_m - spring_len * resting_ratio(config, wheel);
        let kinematic_camber = base_camber + camber_gain(config, wheel) * travel_delta_m;
        state.kinematic_camber_rad = kinematic_camber;
        let span = tire_width(config, wheel) * config.tri_ray_spacing_ratio * 2.0;
        if sample.inner.is_colliding && sample.outer.is_colliding && span > 1e-6 {
            let delta_h = sample.inner.distance - sample.outer.distance;
            let incline = (delta_h / span).atan();
            // Effective camber combines the kinematic travel term with the road plane:
            // the right side mirrors the incline (existing banked-road convention).
            state.dynamic_camber =
                kinematic_camber + if wheel.is_right() { -incline } else { incline };
        } else {
            state.dynamic_camber = kinematic_camber;
        }
        state.effective_contact_camber_rad = state.dynamic_camber;
    }

    fn solve_force(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        modifiers: TireMechanicalModifiers,
        opposite_compression_m: f64,
        dt: f64,
    ) {
        let state = &mut self.wheels[wheel as usize];
        let spring_len = spring_length(config, wheel);
        let spring_k = config.calculate_spring_rate(wheel).max(1.0);
        let tuning = WheelMechanicalTuning::for_wheel(config, wheel);
        let unsprung_mass = tuning.unsprung_mass_kg.max(1.0);
        let arb_ratio = if wheel.is_front() {
            config.front_arb_ratio
        } else {
            config.rear_arb_ratio
        };

        let previous_compression = state.suspension_compression_m;
        let mut x = state.suspension_compression_m;
        let mut v = state.unsprung_velocity_m_s;

        // Correct ARB sign: the more-compressed wheel receives positive force and the
        // less-compressed wheel negative force. Net axle contribution stays zero.
        let arb_force =
            (x - opposite_compression_m) * spring_k * arb_ratio;

        let substeps = ((dt / tuning.mechanical_substep_s.max(1e-5)).ceil() as usize)
            .clamp(1, 12);
        let h = dt / substeps as f64;
        let mut acceleration = 0.0;
        let mut tire_force = 0.0;
        let mut tire_deflection = 0.0;
        let mut tire_deflection_velocity = 0.0;

        // First force evaluation happens before the loop (no dead initializers);
        // each subsequent iteration re-evaluates after the wheel state integrates.
        let (mut spring_force, mut damping_force, mut bottom_force) =
            suspension_element_forces(config, wheel, x, v, spring_k, spring_len, tuning);

        for i in 0..substeps {
            if i > 0 {
                let (sf, df, bf) = suspension_element_forces(
                    config,
                    wheel,
                    x,
                    v,
                    spring_k,
                    spring_len,
                    tuning,
                );
                spring_force = sf;
                damping_force = df;
                bottom_force = bf;
            }
            let suspension_force = spring_force + damping_force + arb_force + bottom_force;

            // Pressure-aware carcass: inflation pressure scales the radial
            // stiffness/damping and the compliant deflection envelope. The hard
            // carcass engages at the EFFECTIVE (scaled) max deflection, so low
            // pressure reaches the hard stop sooner and high pressure sharpens the
            // vertical response. Ray geometry itself is never changed by pressure.
            let stiffness_scale = modifiers.vertical_stiffness_scale.max(0.05);
            let damping_scale = modifiers.vertical_damping_scale.max(0.05);
            let effective_max_deflection =
                tuning.max_tire_deflection_m * modifiers.max_deflection_scale.max(0.05);

            let raw_deflection = (state.road_compression_m - x).max(0.0);
            tire_deflection = raw_deflection.min(effective_max_deflection);
            tire_deflection_velocity = if raw_deflection > 0.0 {
                state.road_velocity_m_s - v
            } else {
                0.0
            };

            let coverage = if state.contact_fraction > 0.0 {
                tuning.partial_contact_stiffness_floor
                    + (1.0 - tuning.partial_contact_stiffness_floor)
                        * state.contact_fraction.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let kt = tuning.tire_vertical_stiffness_n_m * stiffness_scale * coverage;
            let ct = tuning.tire_vertical_damping_n_s_m * damping_scale * coverage;
            tire_force = if coverage > 0.0 && raw_deflection > 0.0 {
                let compliant = kt * tire_deflection + ct * tire_deflection_velocity;
                let hard_carcass = if raw_deflection > effective_max_deflection {
                    let over = raw_deflection - effective_max_deflection;
                    kt * over * 8.0
                } else {
                    0.0
                };
                (compliant + hard_carcass).max(0.0)
            } else {
                0.0
            };

            // Limit only catastrophic numerical impulses. Normal operation remains far below this.
            let force_cap = static_wheel_load(config, wheel).max(100.0) * 12.0;
            tire_force = finite_or_zero(tire_force).clamp(0.0, force_cap);

            // Hybrid quarter-car filter: vehicle_mass already includes wheel mass in Godot,
            // therefore virtual unsprung gravity is intentionally omitted. This state shapes
            // contact transients without double-counting static vehicle weight.
            acceleration = (tire_force - suspension_force) / unsprung_mass;
            v += acceleration * h;
            x += v * h;

            let max_x = spring_len + tuning.hard_stop_overtravel_m;
            if x < 0.0 {
                x = 0.0;
                if v < 0.0 {
                    v = 0.0;
                }
            } else if x > max_x {
                x = max_x;
                if v > 0.0 {
                    v = 0.0;
                }
            }
        }

        // Re-evaluate final suspension values after integration for telemetry and next tick.
        let (sf, df, bf) = suspension_element_forces(
            config,
            wheel,
            x,
            v,
            spring_k,
            spring_len,
            tuning,
        );
        spring_force = sf;
        damping_force = df;
        bottom_force = bf;

        state.previous_compression_mm = previous_compression * 1000.0;
        state.suspension_compression_m = finite_or_zero(x);
        state.unsprung_velocity_m_s = finite_or_zero(v);
        state.unsprung_acceleration_m_s2 = finite_or_zero(acceleration);
        state.compression_mm = state.suspension_compression_m * 1000.0;
        state.spring_speed_mm_s = state.unsprung_velocity_m_s * 1000.0;
        state.spring_current_length = spring_len - state.suspension_compression_m;
        state.max_spring_length = state.spring_current_length.clamp(0.0, spring_len);
        state.spring_force = finite_or_zero(spring_force);
        state.damping_force = finite_or_zero(damping_force);
        state.antiroll_force = finite_or_zero(arb_force);
        state.bottom_out_force = finite_or_zero(bottom_force);
        state.chassis_suspension_force = finite_or_zero(
            state.spring_force + state.damping_force + state.antiroll_force + state.bottom_out_force,
        );
        state.tire_deflection_m = finite_or_zero(tire_deflection);
        state.tire_deflection_velocity_m_s = finite_or_zero(tire_deflection_velocity);
        state.tire_vertical_force = finite_or_zero(tire_force).max(0.0);
        state.total_normal_force = state.tire_vertical_force;
        state.is_grounded = state.contact_fraction > 0.0 && state.total_normal_force > 1.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn suspension_element_forces(
    config: &VehicleConfig,
    wheel: WheelIndex,
    compression_m: f64,
    compression_velocity_m_s: f64,
    spring_k: f64,
    spring_len: f64,
    tuning: WheelMechanicalTuning,
) -> (f64, f64, f64) {
    let mass = config.mass_over_wheel(wheel).max(1.0);
    let damping_ratio = if wheel.is_front() {
        config.front_damping_ratio
    } else {
        config.rear_damping_ratio
    };
    let bump_mult = if wheel.is_front() {
        config.front_bump_damp_multiplier
    } else {
        config.rear_bump_damp_multiplier
    };
    let rebound_mult = if wheel.is_front() {
        config.front_rebound_damp_multiplier
    } else {
        config.rear_rebound_damp_multiplier
    };
    let bump_stop_mult = if wheel.is_front() {
        config.front_bump_stop_multiplier
    } else {
        config.rear_bump_stop_multiplier
    };

    let spring_force = compression_m.max(0.0) * spring_k;

    let critical_c = 2.0 * (spring_k * mass).sqrt();
    let base_c = critical_c * damping_ratio;
    let fast_threshold = 0.127; // m/s
    let slow_bump = base_c * bump_mult;
    let fast_bump = slow_bump * 0.5;
    let slow_rebound = base_c * rebound_mult;
    let fast_rebound = slow_rebound * 0.5;
    let damping_force = if compression_velocity_m_s > fast_threshold {
        (compression_velocity_m_s - fast_threshold) * fast_bump
            + fast_threshold * slow_bump
    } else if compression_velocity_m_s >= 0.0 {
        compression_velocity_m_s * slow_bump
    } else if compression_velocity_m_s < -fast_threshold {
        (compression_velocity_m_s + fast_threshold) * fast_rebound
            - fast_threshold * slow_rebound
    } else {
        compression_velocity_m_s * slow_rebound
    };

    // Progressive bump stop starts before full geometric travel. Unlike the old code,
    // compression may also exceed spring_len, so true bottom-out penetration is measurable.
    let bump_start = spring_len * tuning.bump_stop_start_ratio.clamp(0.5, 0.98);
    let bump_range = (spring_len - bump_start).max(1e-4);
    let bump_excess = (compression_m - bump_start).max(0.0);
    let progress = (bump_excess / bump_range).clamp(0.0, 2.0);
    let soft_stop = spring_k
        * bump_stop_mult
        * bump_excess
        * (1.0 + 3.0 * progress * progress);
    let hard_overtravel = (compression_m - spring_len).max(0.0);
    let hard_stop = if hard_overtravel > 0.0 {
        spring_k * bump_stop_mult * hard_overtravel
            * (12.0 + 24.0 * (hard_overtravel / tuning.hard_stop_overtravel_m.max(1e-4)))
    } else {
        0.0
    };

    (
        finite_or_zero(spring_force),
        finite_or_zero(damping_force),
        finite_or_zero(soft_stop + hard_stop),
    )
}

fn spring_length(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_spring_length
    } else {
        config.rear_spring_length
    }
}
fn resting_ratio(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_resting_ratio
    } else {
        config.rear_resting_ratio
    }
}
fn camber(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_camber
    } else {
        config.rear_camber
    }
}
fn camber_gain(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_camber_gain_rad_per_m
    } else {
        config.rear_camber_gain_rad_per_m
    }
}

/// Static/rest suspension compression (m): spring length x resting ratio.
/// `suspension_compression_m - rest_compression_m` is the travel delta consumed by
/// the camber/toe gain terms. Positive delta = bump, negative delta = rebound.
pub fn rest_compression_m(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    spring_length(config, wheel) * resting_ratio(config, wheel)
}

fn ray_compression(hit: &RaycastHit, max_ray_length: f64) -> f64 {
    if hit.is_colliding {
        (max_ray_length - hit.distance).max(0.0)
    } else {
        0.0
    }
}

fn tricast_contact_fraction(sample: &TriRaycastSample) -> f64 {
    let mut weights = 0.0;
    if sample.inner.is_colliding {
        weights += 1.0;
    }
    if sample.center.is_colliding {
        weights += 2.0;
    }
    if sample.outer.is_colliding {
        weights += 1.0;
    }
    weights / 4.0
}

fn weighted_point(sample: &TriRaycastSample) -> Vec3 {
    let mut sum = Vec3::ZERO;
    let mut weights = 0.0;
    add_point(&mut sum, &mut weights, &sample.inner, 1.0);
    add_point(&mut sum, &mut weights, &sample.center, 2.0);
    add_point(&mut sum, &mut weights, &sample.outer, 1.0);
    if weights > 0.0 {
        sum / weights
    } else {
        Vec3::ZERO
    }
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
        if sample.inner.distance <= sample.outer.distance {
            sample.inner.surface
        } else {
            sample.outer.surface
        }
    } else if sample.inner.is_colliding {
        sample.inner.surface
    } else {
        sample.outer.surface
    }
}

fn blended_surface(
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
    if weights > 0.0 {
        sum / weights
    } else {
        fallback
    }
}

fn surface_value(
    map: &std::collections::HashMap<SurfaceType, f64>,
    surface: SurfaceType,
    fallback: f64,
) -> f64 {
    *map.get(&surface).unwrap_or(&fallback)
}

fn finite_or_zero(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_hit(distance: f64) -> RaycastHit {
        RaycastHit {
            is_colliding: true,
            distance,
            point: Vec3::ZERO,
            normal: Vec3::UP,
            surface: SurfaceType::Road,
        }
    }

    #[test]
    fn arb_loads_more_compressed_wheel_and_is_axle_neutral() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        cfg.front_arb_ratio = 0.20;
        let mut sus = SuspensionSystem::new(&cfg);
        sus.wheels[WheelIndex::FrontLeft as usize].suspension_compression_m = 0.050;
        sus.wheels[WheelIndex::FrontRight as usize].suspension_compression_m = 0.090;
        sus.solve_force(&cfg, WheelIndex::FrontRight, TireMechanicalModifiers::identity(), 0.050, 1.0 / 120.0);
        sus.solve_force(&cfg, WheelIndex::FrontLeft, TireMechanicalModifiers::identity(), 0.090, 1.0 / 120.0);
        let right = sus.wheels[WheelIndex::FrontRight as usize].antiroll_force;
        let left = sus.wheels[WheelIndex::FrontLeft as usize].antiroll_force;
        assert!(right > 0.0, "outer/more-compressed wheel must gain ARB load: {right}");
        assert!(left < 0.0, "inner/less-compressed wheel must lose ARB load: {left}");
        assert!((right + left).abs() < 1e-6);
    }

    #[test]
    fn bump_stop_activates_before_and_beyond_full_travel() {
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::FrontLeft;
        let spring_len = cfg.front_spring_length;
        let k = cfg.calculate_spring_rate(wheel);
        let tuning = WheelMechanicalTuning::for_wheel(&cfg, wheel);
        let (_, _, before) = suspension_element_forces(
            &cfg, wheel, spring_len * 0.70, 0.0, k, spring_len, tuning,
        );
        let (_, _, near) = suspension_element_forces(
            &cfg, wheel, spring_len * 0.95, 0.0, k, spring_len, tuning,
        );
        let (_, _, over) = suspension_element_forces(
            &cfg, wheel, spring_len + 0.010, 0.0, k, spring_len, tuning,
        );
        assert_eq!(before, 0.0);
        assert!(near > 0.0);
        assert!(over > near);
    }

    #[test]
    fn tricast_retains_partial_contact_fraction() {
        let sample = TriRaycastSample {
            inner: RaycastHit::default(),
            center: flat_hit(0.4),
            outer: flat_hit(0.4),
        };
        assert!((tricast_contact_fraction(&sample) - 0.75).abs() < 1e-9);
    }

    #[test]
    fn static_equilibrium_matches_wheel_load_and_rest_travel() {
        let cfg = VehicleConfig::f1_94_canonical();
        let mut suspension = SuspensionSystem::new(&cfg);
        let dt = 1.0 / 120.0;
        // Model the settled body: rays sit at nominal rest distance minus the static
        // carcass deflection (the body settles ~8 mm lower because the tire squashes).
        let static_deflection = static_wheel_load(&cfg, WheelIndex::FrontLeft)
            / WheelMechanicalTuning::for_wheel(&cfg, WheelIndex::FrontLeft)
                .tire_vertical_stiffness_n_m;
        let rest_fl = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio)
            + cfg.front_tire_radius
            - static_deflection;
        let rest_rl = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio)
            + cfg.rear_tire_radius
            - static_deflection;
        let samples = [
            TriRaycastSample { inner: flat_hit(rest_fl), center: flat_hit(rest_fl), outer: flat_hit(rest_fl) },
            TriRaycastSample { inner: flat_hit(rest_fl), center: flat_hit(rest_fl), outer: flat_hit(rest_fl) },
            TriRaycastSample { inner: flat_hit(rest_rl), center: flat_hit(rest_rl), outer: flat_hit(rest_rl) },
            TriRaycastSample { inner: flat_hit(rest_rl), center: flat_hit(rest_rl), outer: flat_hit(rest_rl) },
        ];
        for _ in 0..240 {
            suspension.step(&cfg, &samples, dt);
        }
        for wheel in WheelIndex::ALL {
            let s = &suspension.wheels[wheel as usize];
            let static_load = static_wheel_load(&cfg, wheel);
            assert!(
                (s.total_normal_force - static_load).abs() < static_load * 0.02,
                "wheel {:?} Fz {} must approach static load {}",
                wheel, s.total_normal_force, static_load
            );
            // Suspension settles at rest travel; the carcass absorbs the static deflection.
            let rest_travel = if wheel.is_front() { cfg.front_spring_length * cfg.front_resting_ratio } else { cfg.rear_spring_length * cfg.rear_resting_ratio };
            assert!(
                (s.compression_mm - rest_travel * 1000.0).abs() < 2.0,
                "wheel {:?} compression {}mm must approach rest travel {}mm",
                wheel, s.compression_mm, rest_travel * 1000.0
            );
            assert!(
                s.tire_deflection_m > 0.004 && s.tire_deflection_m < 0.020,
                "wheel {:?} tire deflection {}m must be in the compliant carcass range",
                wheel, s.tire_deflection_m
            );
        }
    }

    #[test]
    fn curb_strike_deflects_tire_before_suspension_compresses() {
        let cfg = VehicleConfig::f1_94_canonical();
        let mut suspension = SuspensionSystem::new(&cfg);
        let dt = 1.0 / 120.0;
        let rest_fl = cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
        let rest_rl = cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
        let flat = [
            TriRaycastSample { inner: flat_hit(rest_fl), center: flat_hit(rest_fl), outer: flat_hit(rest_fl) },
            TriRaycastSample { inner: flat_hit(rest_fl), center: flat_hit(rest_fl), outer: flat_hit(rest_fl) },
            TriRaycastSample { inner: flat_hit(rest_rl), center: flat_hit(rest_rl), outer: flat_hit(rest_rl) },
            TriRaycastSample { inner: flat_hit(rest_rl), center: flat_hit(rest_rl), outer: flat_hit(rest_rl) },
        ];
        for _ in 0..120 {
            suspension.step(&cfg, &flat, dt);
        }
        let comp_before = suspension.wheels[0].compression_mm;
        let deflection_before = suspension.wheels[0].tire_deflection_m;

        // 40 mm curb under the front-left wheel.
        let mut bump = flat.clone();
        bump[0].inner.distance -= 0.040;
        bump[0].center.distance -= 0.040;
        bump[0].outer.distance -= 0.040;
        suspension.step(&cfg, &bump, dt);

        let s = &suspension.wheels[0];
        let d_deflection = s.tire_deflection_m - deflection_before;
        let d_compression = (s.compression_mm - comp_before) * 0.001;
        // The carcass must absorb the first tick of the strike: deflection leads,
        // and suspension compression must not jump the full 40 mm in one tick.
        assert!(d_deflection > 0.0, "tire deflection must grow on first curb tick");
        assert!(
            d_compression < 0.040 * 0.5,
            "suspension must lag the curb strike on the first tick, compression delta {}m",
            d_compression
        );
        assert!(s.total_normal_force > static_wheel_load(&cfg, WheelIndex::FrontLeft));
    }

    #[test]
    fn camber_gain_zero_preserves_legacy_static_camber() {
        let cfg = VehicleConfig::f1_94_canonical();
        let mut sus = SuspensionSystem::new(&cfg);
        let wheel = WheelIndex::FrontLeft;
        let max_len = cfg.front_spring_length + cfg.front_tire_radius;
        let rest = rest_compression_m(&cfg, wheel);
        let dt = 1.0 / 120.0;
        let sample = TriRaycastSample {
            inner: flat_hit(max_len - rest),
            center: flat_hit(max_len - rest),
            outer: flat_hit(max_len - rest),
        };
        // First call performs the one-shot snapshot initialization; afterwards the
        // state compression feeds the travel delta directly.
        sus.sample_contact(&cfg, wheel, &sample, dt);
        sus.wheels[wheel as usize].suspension_compression_m = rest;
        sus.sample_contact(&cfg, wheel, &sample, dt);
        let s = &sus.wheels[wheel as usize];
        assert!(
            (s.kinematic_camber_rad - cfg.front_camber).abs() < 1e-12,
            "kinematic camber at rest must equal static camber, got {}",
            s.kinematic_camber_rad
        );
        assert!((s.effective_contact_camber_rad - cfg.front_camber).abs() < 1e-12);
        assert!((s.dynamic_camber - cfg.front_camber).abs() < 1e-12);
    }

    #[test]
    fn camber_gain_bump_rebound_direction_and_flat_mirror_symmetry() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        cfg.front_camber_gain_rad_per_m = -0.021;
        cfg.rear_camber_gain_rad_per_m = 0.0;
        let mut sus = SuspensionSystem::new(&cfg);
        let dt = 1.0 / 120.0;
        let max_len = cfg.front_spring_length + cfg.front_tire_radius;
        let rest = rest_compression_m(&cfg, WheelIndex::FrontLeft);
        let sample_at =
            |x: f64| TriRaycastSample {
                inner: flat_hit(max_len - x),
                center: flat_hit(max_len - x),
                outer: flat_hit(max_len - x),
            };
        // Initialize both wheels with a rest-time contact snapshot, then drive the
        // state compression directly so the travel delta is exact.
        for wheel in [WheelIndex::FrontLeft, WheelIndex::FrontRight] {
            sus.sample_contact(&cfg, wheel, &sample_at(rest), dt);
        }

        // Bump: +20 mm travel => camber shifts by gain * travel (negative direction).
        let bump = rest + 0.020;
        sus.wheels[WheelIndex::FrontLeft as usize].suspension_compression_m = bump;
        sus.wheels[WheelIndex::FrontRight as usize].suspension_compression_m = bump;
        sus.sample_contact(&cfg, WheelIndex::FrontLeft, &sample_at(bump), dt);
        sus.sample_contact(&cfg, WheelIndex::FrontRight, &sample_at(bump), dt);
        let fl = &sus.wheels[WheelIndex::FrontLeft as usize];
        let fr = &sus.wheels[WheelIndex::FrontRight as usize];
        let expected = cfg.front_camber + (-0.021 * 0.020);
        assert!(
            (fl.kinematic_camber_rad - expected).abs() < 1e-9,
            "bump must shift kinematic camber by gain*travel, got {} expected {}",
            fl.kinematic_camber_rad,
            expected
        );
        assert!(
            (fr.kinematic_camber_rad - fl.kinematic_camber_rad).abs() < 1e-12,
            "flat road must keep left/right kinematic camber symmetric"
        );

        // Rebound: -10 mm travel shifts opposite direction; mirror symmetry holds.
        let rebound = rest - 0.010;
        sus.wheels[WheelIndex::FrontLeft as usize].suspension_compression_m = rebound;
        sus.wheels[WheelIndex::FrontRight as usize].suspension_compression_m = rebound;
        sus.sample_contact(&cfg, WheelIndex::FrontLeft, &sample_at(rebound), dt);
        sus.sample_contact(&cfg, WheelIndex::FrontRight, &sample_at(rebound), dt);
        let rl = &sus.wheels[WheelIndex::FrontLeft as usize];
        let rr = &sus.wheels[WheelIndex::FrontRight as usize];
        let expected_rebound = cfg.front_camber + (-0.021 * -0.010);
        assert!(
            (rl.kinematic_camber_rad - expected_rebound).abs() < 1e-9,
            "rebound must shift camber opposite to bump, got {}",
            rl.kinematic_camber_rad
        );
        assert!((rl.dynamic_camber - rr.dynamic_camber).abs() < 1e-12);
    }

    #[test]
    fn banked_road_splits_kinematic_camber_from_effective_contact_camber() {
        let cfg = VehicleConfig::f1_94_canonical();
        let mut sus = SuspensionSystem::new(&cfg);
        let dt = 1.0 / 120.0;
        let max_len = cfg.front_spring_length + cfg.front_tire_radius;
        let rest = rest_compression_m(&cfg, WheelIndex::FrontLeft);
        let span = cfg.front_tire_width * cfg.tri_ray_spacing_ratio * 2.0;
        // Inner ray 10 mm closer than the outer: a road-plane incline of atan(0.010/span).
        let sample = TriRaycastSample {
            inner: flat_hit(max_len - rest - 0.010),
            center: flat_hit(max_len - rest),
            outer: flat_hit(max_len - rest),
        };
        sus.sample_contact(&cfg, WheelIndex::FrontLeft, &sample, dt);
        let fl = &sus.wheels[WheelIndex::FrontLeft as usize];
        assert!(
            (fl.kinematic_camber_rad - cfg.front_camber).abs() < 1e-12,
            "kinematic camber must ignore the road plane"
        );
        let incline = (0.010_f64 / span).atan();
        let expected_effective = cfg.front_camber - incline;
        assert!(
            (fl.effective_contact_camber_rad - expected_effective).abs() < 1e-9,
            "effective camber must combine kinematic camber with the road incline"
        );
        assert!((fl.dynamic_camber - expected_effective).abs() < 1e-9);
        let fl_kinematic = fl.kinematic_camber_rad;
        let fl_dynamic = fl.dynamic_camber;

        // Same geometric relationship on the right wheel mirrors the incline sign,
        // keeping the axle symmetric across identical (mirrored) road shapes.
        sus.sample_contact(&cfg, WheelIndex::FrontRight, &sample, dt);
        let fr = &sus.wheels[WheelIndex::FrontRight as usize];
        let expected_right = cfg.front_camber + incline;
        assert!(
            (fr.effective_contact_camber_rad - expected_right).abs() < 1e-9,
            "right wheel must mirror the incline, got {}",
            fr.effective_contact_camber_rad
        );
        assert!((fl_kinematic - fr.kinematic_camber_rad).abs() < 1e-12);
        assert!(
            (fl_dynamic + fr.dynamic_camber - 2.0 * cfg.front_camber).abs() < 1e-9,
            "symmetric banked samples must keep the axle camber neutral"
        );
    }

    #[test]
    fn curb_strike_camber_stays_continuous() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        cfg.front_camber_gain_rad_per_m = -0.021;
        cfg.rear_camber_gain_rad_per_m = -0.015;
        let mut suspension = SuspensionSystem::new(&cfg);
        let dt = 1.0 / 120.0;
        let rest_fl =
            cfg.front_spring_length * (1.0 - cfg.front_resting_ratio) + cfg.front_tire_radius;
        let rest_rl =
            cfg.rear_spring_length * (1.0 - cfg.rear_resting_ratio) + cfg.rear_tire_radius;
        let flat = [
            TriRaycastSample { inner: flat_hit(rest_fl), center: flat_hit(rest_fl), outer: flat_hit(rest_fl) },
            TriRaycastSample { inner: flat_hit(rest_fl), center: flat_hit(rest_fl), outer: flat_hit(rest_fl) },
            TriRaycastSample { inner: flat_hit(rest_rl), center: flat_hit(rest_rl), outer: flat_hit(rest_rl) },
            TriRaycastSample { inner: flat_hit(rest_rl), center: flat_hit(rest_rl), outer: flat_hit(rest_rl) },
        ];
        for _ in 0..120 {
            suspension.step(&cfg, &flat, dt);
        }
        let static_camber = suspension.wheels[0].dynamic_camber;

        // 40 mm curb under the front-left wheel.
        let mut bump = flat.clone();
        bump[0].inner.distance -= 0.040;
        bump[0].center.distance -= 0.040;
        bump[0].outer.distance -= 0.040;
        for _ in 0..60 {
            suspension.step(&cfg, &bump, dt);
            let s = &suspension.wheels[0];
            assert!(
                (s.dynamic_camber - static_camber).abs() < 0.03,
                "camber must not jump on a curb strike, delta {}",
                s.dynamic_camber - static_camber
            );
            assert!(
                (s.dynamic_camber - s.effective_contact_camber_rad).abs() < 1e-12,
                "effective contact camber must stay in sync with the tire feed"
            );
        }
        assert!(
            suspension.wheels[0].kinematic_camber_rad < cfg.front_camber,
            "negative gain on a bump must shift camber toward negative"
        );
    }
}
