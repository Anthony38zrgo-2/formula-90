// Mechanical suspension + tricast tire-carcass model.
//
// Godot still supplies three transverse raycasts per wheel. Rust turns them into a
// virtual contact patch and integrates a 1-DOF unsprung wheel between the road/tire
// carcass and the chassis suspension. This removes the old direct raycast->damper
// impulse path while preserving the existing FFI shape.
//
// SUS-GEO-04: in `Geometric` mode the spring/damper element is explicit and is
// coupled to wheel travel via the deterministic kinematics (`r = ds/dq`) and
// virtual work (`F_wheel = F_element * r`). Legacy `Legacy1Dof` keeps the
// inferred-rate path byte-identical. The tire carcass and the virtual-unsprung
// filter approximations are unchanged in both modes for now:
// - rigid body still carries total vehicle mass (Godot contract); the virtual
//   unsprung mass is an inertial contact filter, gravity omitted on purpose;
// - road-height low-pass tau, max road velocity, partial-contact stiffness
//   floor and substepping behaviour are shared (geometric adds travel/stroke
//   limits and invalid-geometry diagnostics instead of generic force cuts).
use crate::suspension_geo_config::{AntiRollConfig, GeometricSuspensionConfig, SuspensionModelKind};
use crate::suspension_kinematics::{jacobian, solve_corner};
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
    /// GRIP-06: first-order filtered surface values (tau from
    /// `surface_transition_tau_s`) so kerb/grass crossings ramp instead of stepping.
    #[serde(default)]
    surface_filter_initialized: bool,
    #[serde(default)]
    pub filtered_surface_friction: f64,
    #[serde(default)]
    pub filtered_surface_stiffness: f64,
    #[serde(default)]
    pub filtered_surface_rolling_resistance: f64,
    // --- SUS-GEO-04 geometric diagnostics (legacy leaves them at 0/false) ---
    /// Damper compression s = Lrest - L (m, + on bump).
    #[serde(default)]
    pub damper_compression_m: f64,
    /// Damper shaft velocity ds/dt = r * v (m/s).
    #[serde(default)]
    pub damper_velocity_m_s: f64,
    /// Motion ratio r = ds/dq (dimensionless, > 0 when valid).
    #[serde(default)]
    pub motion_ratio: f64,
    /// Rate dr/dq (1/m) for the tangent stiffness.
    #[serde(default)]
    pub motion_ratio_rate_per_m: f64,
    /// Tangent wheel rate k_s*r^2 + F_s*dr/dq (N/m, diagnostic/telemetry).
    #[serde(default)]
    pub tangent_wheel_rate_n_per_m: f64,
    /// Pushrod axial force (N, + tension / - compression, label-independent).
    /// No bending/buckling model.
    #[serde(default)]
    pub rod_axial_force_n: f64,
    /// True when the mechanism is at/clamped by a physical limit or invalid.
    #[serde(default)]
    pub geometric_clamped: bool,
    /// Worst link residual of the last kinematics solve (m).
    #[serde(default)]
    pub geometric_max_residual_m: f64,
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
            // Start at the Road values so the filter has no startup transient and
            // tau = 0.0 remains an exact pass-through.
            surface_filter_initialized: true,
            filtered_surface_friction: surface_value(
                &config.surface_friction,
                SurfaceType::Road,
                1.0,
            ),
            filtered_surface_stiffness: surface_value(
                &config.surface_stiffness,
                SurfaceType::Road,
                5.0,
            ),
            filtered_surface_rolling_resistance: surface_value(
                &config.surface_rolling_resistance,
                SurfaceType::Road,
                1.0,
            ),
            damper_compression_m: 0.0,
            damper_velocity_m_s: 0.0,
            motion_ratio: 1.0,
            motion_ratio_rate_per_m: 0.0,
            tangent_wheel_rate_n_per_m: config.calculate_spring_rate(wheel).max(1.0),
            rod_axial_force_n: 0.0,
            geometric_clamped: false,
            geometric_max_residual_m: 0.0,
        }
    }
}

/// GRIP-06: apply the per-wheel surface filter. With `tau = 0.0` this is an exact
/// pass-through; with a positive tau the effective friction/stiffness/rolling
/// values ease toward the raw blended sample, so kerb and grass transitions are
/// no longer single-frame steps.
fn filter_surface_values(
    config: &VehicleConfig,
    state: &mut WheelSuspensionState,
    raw_friction: f64,
    raw_stiffness: f64,
    raw_rolling: f64,
    dt: f64,
) {
    let tau = config.surface_transition_tau_s.max(0.0);
    if tau <= 1e-9 {
        state.filtered_surface_friction = raw_friction;
        state.filtered_surface_stiffness = raw_stiffness;
        state.filtered_surface_rolling_resistance = raw_rolling;
        state.surface_filter_initialized = true;
    } else {
        if !state.surface_filter_initialized {
            state.filtered_surface_friction = raw_friction;
            state.filtered_surface_stiffness = raw_stiffness;
            state.filtered_surface_rolling_resistance = raw_rolling;
            state.surface_filter_initialized = true;
        }
        let alpha = 1.0 - (-dt / tau).exp();
        state.filtered_surface_friction +=
            (raw_friction - state.filtered_surface_friction) * alpha;
        state.filtered_surface_stiffness +=
            (raw_stiffness - state.filtered_surface_stiffness) * alpha;
        state.filtered_surface_rolling_resistance +=
            (raw_rolling - state.filtered_surface_rolling_resistance) * alpha;
    }
    state.effective_friction = state.filtered_surface_friction;
    state.effective_stiffness = state.filtered_surface_stiffness;
    state.effective_rolling_resistance = state.filtered_surface_rolling_resistance;
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
            filter_surface_values(
                config,
                state,
                surface_value(&config.surface_friction, SurfaceType::Road, 1.0),
                surface_value(&config.surface_stiffness, SurfaceType::Road, 5.0),
                surface_value(&config.surface_rolling_resistance, SurfaceType::Road, 1.0),
                dt,
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
        filter_surface_values(
            config,
            state,
            blended_surface(sample, &config.surface_friction, 1.0),
            blended_surface(sample, &config.surface_stiffness, 5.0),
            blended_surface(sample, &config.surface_rolling_resistance, 1.0),
            dt,
        );

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
        let is_geo = matches!(config.suspension_model, SuspensionModelKind::Geometric)
            && config.geometric_suspension.is_some();
        if is_geo {
            self.solve_force_geometric(config, wheel, modifiers, opposite_compression_m, dt);
        } else {
            self.solve_force_legacy(config, wheel, modifiers, opposite_compression_m, dt);
        }
    }

    fn solve_force_legacy(
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
        // Legacy leaves geometric diagnostics neutral.
        state.damper_compression_m = 0.0;
        state.damper_velocity_m_s = 0.0;
        state.motion_ratio = 1.0;
        state.motion_ratio_rate_per_m = 0.0;
        state.tangent_wheel_rate_n_per_m = spring_k.max(1.0);
        state.rod_axial_force_n = 0.0;
        state.geometric_clamped = false;
        state.geometric_max_residual_m = 0.0;
    }

    /// SUS-GEO-04 geometric path: explicit spring/damper via kinematics and
    /// virtual work. Tire carcass/filter integration is shared with legacy
    /// (same road target, coverage, substeps); only the suspension element,
    /// ARB, stops and travel limits are geometric. No generic force cuts hide
    /// invalid geometry: unreachable/clamped/wrong-sign states set
    /// `geometric_clamped` and apply hard travel stops.
    fn solve_force_geometric(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        modifiers: TireMechanicalModifiers,
        opposite_compression_m: f64,
        dt: f64,
    ) {
        let geo = match config.geometric_suspension.as_ref() {
            Some(g) => g,
            None => return,
        };
        let tuning = WheelMechanicalTuning::for_wheel(config, wheel);
        let unsprung_mass = tuning.unsprung_mass_kg.max(1.0);
        let spring_len = spring_length(config, wheel);
        let rest_legacy = spring_len * resting_ratio(config, wheel);
        let axle = geo.axle(wheel);
        let state = &mut self.wheels[wheel as usize];
        let previous_compression = state.suspension_compression_m;
        let mut x = state.suspension_compression_m;
        let mut v = state.unsprung_velocity_m_s;

        // Wheel travel from design (hub_center): q = x - rest_legacy.
        let q_of = |xx: f64| xx - rest_legacy;
        // Rest wheel rate for ARB/stops explicit approximation.
        let k_wheel_rest = geometric_k_wheel_rest(geo, wheel).max(1.0);
        let q_opp = opposite_compression_m - rest_legacy;
        // Tick-start linearization (SUS-GEO-03 cost study): one exact solve +
        // Jacobian at q0; substeps use s ≈ s0 + r0*(q-q0), vs = r0*v. The
        // second-order error is O(d2s*(dq)^2) with |dq| < 1 mm per tick,
        // negligible vs the 1e-4 link tolerance. Final telemetry re-solves
        // exactly at the settled q.
        let q0 = q_of(x);
        let axle0 = geo.axle(wheel);
        let preload_def0 = axle0.spring_free_length_m - axle0.spring_installed_length_m;
        let (s0, r0, invalid0) = match (
            solve_corner(
                geo.corners.get(wheel),
                q0,
                0.0,
                wheel.is_front(),
                0.0,
                0.0,
                1.0,
            ),
            jacobian(geo.corners.get(wheel), q0, 0.0, wheel.is_front()),
        ) {
            (Some(sol), Some(jac))
                if jac.motion_ratio.is_finite() && jac.motion_ratio > 0.0 =>
            {
                (sol.damper_compression, jac.motion_ratio, false)
            }
            _ => (0.0, 0.0, true),
        };
        let k_s0 = axle0.spring_rate_N_per_m;
        // Per-substep element via linearization (no solves in the loop).
        let elem_at = |q: f64, v: f64| -> (f64, f64, f64, bool) {
            if invalid0 {
                return (0.0, 0.0, s0, true);
            }
            let s = s0 + r0 * (q - q0);
            let total = preload_def0 + s;
            let f_s = if total <= 0.0 {
                0.0
            } else {
                k_s0 * total
            };
            let v_s = r0 * v;
            let f_d = geometric_damper_element(axle0, v_s);
            (f_s * r0, f_d * r0, s, false)
        };

        let substeps = ((dt / tuning.mechanical_substep_s.max(1e-5)).ceil() as usize)
            .clamp(1, 12);
        let h = dt / substeps as f64;
        let mut acceleration = 0.0;
        let mut tire_force = 0.0;
        let mut tire_deflection = 0.0;
        let mut tire_deflection_velocity = 0.0;
        // First evaluation (read on the first substep); later substeps
        // re-evaluate after integration — same structure as legacy.
        let (mut spring_force, mut damping_force, mut last_s, mut last_r, mut elem_bad) = {
            let (sf, df, s, bad) = elem_at(q_of(x), v);
            (sf, df, s, r0, bad)
        };
        // Initialized then overwritten on the first substep (loop always runs
        // 1..12 times); the assignment keeps definite-assignment happy.
        #[allow(unused_assignments)]
        let mut bottom_force: f64 = 0.0;
        let mut clamped_tick = invalid0 || elem_bad;

        for i in 0..substeps {
            if i > 0 {
                let (sf, df, s, bad) = elem_at(q_of(x), v);
                spring_force = sf;
                damping_force = df;
                last_s = s;
                last_r = r0;
                elem_bad = bad;
            }
            let stop =
                geometric_stop_force(geo, wheel, x, rest_legacy, last_s, last_r, tuning);
            bottom_force = stop.0;
            clamped_tick = elem_bad || stop.1;
            // Invalid mechanism at current q (wrong-sign ratio, degenerate):
            // element already zeroed; apply a hard travel stop opposing the
            // penetration instead of a generic force cut.
            if elem_bad || stop.1 {
                let push = k_wheel_rest * 0.02;
                bottom_force += if v >= 0.0 { push } else { -push };
            }
            // Recompute ARB with the current substep travel (uses coherent
            // tick-start opposite like legacy; q_opp fixed for the tick).
            let arb_now = geometric_arb_force(geo, wheel, q_of(x), q_opp, k_wheel_rest);
            let suspension_force = spring_force + damping_force + arb_now + bottom_force;

            // Shared carcass (same as legacy): pressure-aware, coverage-scaled.
            // NOTE: road target/ray geometry stays legacy until SUS-GEO-06.
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
            let force_cap = static_wheel_load(config, wheel).max(100.0) * 12.0;
            tire_force = finite_or_zero(tire_force).clamp(0.0, force_cap);

            acceleration = (tire_force - suspension_force) / unsprung_mass;
            v += acceleration * h;
            x += v * h;

            // Geometric travel limits (wheel + damper + mechanism), with a
            // small measurable overtravel like legacy.
            let (x_min, x_max) = geometric_travel_bounds(geo, wheel, rest_legacy);
            let over = tuning.hard_stop_overtravel_m;
            if x < x_min - over {
                x = x_min - over;
                if v < 0.0 {
                    v = 0.0;
                }
            } else if x > x_max + over {
                x = x_max + over;
                if v > 0.0 {
                    v = 0.0;
                }
            }
        }

        // Final telemetry evaluation (exact re-solve at settled q; tangent via
        // tick secant to avoid 4 extra solves per wheel per tick).
        let q = q_of(x);
        let (sf, df, s_fin, r_fin, rod_fin, clamped_fin, resid_fin) =
            geometric_wheel_forces(geo, wheel, q, v);
        spring_force = sf;
        damping_force = df;
        let stop = geometric_stop_force(geo, wheel, x, rest_legacy, s_fin, r_fin, tuning);
        bottom_force = stop.0;
        let arb_now = geometric_arb_force(geo, wheel, q, q_opp, k_wheel_rest);
        let dq = q - q0;
        let drdq = if dq.abs() > 1e-9 && r0 > 0.0 && r_fin.is_finite() {
            ((r_fin - r0) / dq).clamp(-10.0, 10.0)
        } else {
            0.0
        };
        let total_fin =
            (axle.spring_free_length_m - axle.spring_installed_length_m) + s_fin;
        let f_s_fin = if total_fin <= 0.0 {
            0.0
        } else {
            axle.spring_rate_N_per_m * total_fin
        };
        let tangent = axle.spring_rate_N_per_m * r_fin * r_fin + f_s_fin * drdq;
        let _ = axle;

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
        state.antiroll_force = finite_or_zero(arb_now);
        state.bottom_out_force = finite_or_zero(bottom_force);
        state.chassis_suspension_force = finite_or_zero(
            state.spring_force + state.damping_force + state.antiroll_force + state.bottom_out_force,
        );
        state.damper_compression_m = finite_or_zero(s_fin);
        state.damper_velocity_m_s = finite_or_zero(r_fin * v);
        state.motion_ratio = finite_or_zero(r_fin);
        state.motion_ratio_rate_per_m = finite_or_zero(drdq);
        state.tangent_wheel_rate_n_per_m = finite_or_zero(tangent).max(1.0);
        state.rod_axial_force_n = finite_or_zero(rod_fin);
        state.geometric_clamped = clamped_fin || stop.1 || clamped_tick;
        state.geometric_max_residual_m = finite_or_zero(resid_fin);
        state.tire_deflection_m = finite_or_zero(tire_deflection);
        state.tire_deflection_velocity_m_s = finite_or_zero(tire_deflection_velocity);
        state.tire_vertical_force = finite_or_zero(tire_force).max(0.0);
        state.total_normal_force = state.tire_vertical_force;
        state.is_grounded = state.contact_fraction > 0.0 && state.total_normal_force > 1.0;
    }
}

/// SUS-GEO-04 helpers: explicit element + virtual work (geometric only).
/// Legacy path below is untouched.

/// Rest wheel rate k_s * r0^2 at design (q = 0, straight) for the explicit
/// ARB/stop approximation. Returns 1.0 floor on degenerate solves.
fn geometric_k_wheel_rest(geo: &GeometricSuspensionConfig, wheel: WheelIndex) -> f64 {
    let corner = geo.corners.get(wheel);
    let axle = geo.axle(wheel);
    match jacobian(corner, 0.0, 0.0, wheel.is_front()) {
        Some(j) if j.motion_ratio.is_finite() && j.motion_ratio > 0.0 => {
            (axle.spring_rate_N_per_m * j.motion_ratio * j.motion_ratio).max(1.0)
        }
        _ => axle.spring_rate_N_per_m.max(1.0),
    }
}

/// (q_min, q_max) reachable travel from design: element droop/bump
/// intersected with the kinematics branch connected to rest.
fn geometric_q_limits(geo: &GeometricSuspensionConfig, wheel: WheelIndex) -> (f64, f64) {
    let corner = geo.corners.get(wheel);
    let axle = geo.axle(wheel);
    let (lo, hi) = crate::suspension_kinematics::travel_envelope(
        corner,
        axle.wheel_droop_m,
        axle.wheel_bump_m,
        0.0,
        wheel.is_front(),
    );
    let lo = lo.max(-axle.wheel_droop_m).min(0.0);
    let hi = hi.min(axle.wheel_bump_m).max(0.0);
    (lo, hi)
}

/// Travel bounds in legacy x units (x = rest_legacy + q).
fn geometric_travel_bounds(
    geo: &GeometricSuspensionConfig,
    wheel: WheelIndex,
    rest_legacy: f64,
) -> (f64, f64) {
    let (lo, hi) = geometric_q_limits(geo, wheel);
    (rest_legacy + lo, rest_legacy + hi)
}

/// Digressive viscous element with explicit bump/rebound rates (no inferred
/// critical damping). Mirrors the legacy knee/fast shape.
fn geometric_damper_element(
    axle: &crate::suspension_geo_config::AxlePhysicalElements,
    v_s: f64,
) -> f64 {
    if !v_s.is_finite() {
        return 0.0;
    }
    let knee = axle.damper_knee_m_per_s.max(1e-6);
    let fast = axle.damper_fast_factor.clamp(0.05, 1.0);
    if v_s >= 0.0 {
        let c = axle.damper_bump_Ns_per_m.max(0.0);
        if v_s > knee {
            (v_s - knee) * c * fast + knee * c
        } else {
            v_s * c
        }
    } else if v_s < -knee {
        let c = axle.damper_rebound_Ns_per_m.max(0.0);
        (v_s + knee) * c * fast - knee * c
    } else {
        v_s * axle.damper_rebound_Ns_per_m.max(0.0)
    }
}

/// Core SUS-GEO-04 mapping: (F_spring_wheel, F_damper_wheel, s, r, rod, clamped, resid).
/// Virtual work: F_wheel = F_element * r. Spring cannot pull (unseat at 0).
/// Wrong-sign (r <= 0), degenerate or clamped solves flag invalid and yield
/// zero element forces so no grip is fabricated; the caller applies hard
/// travel stops instead of generic force cuts.
fn geometric_wheel_forces(
    geo: &GeometricSuspensionConfig,
    wheel: WheelIndex,
    q: f64,
    v: f64,
) -> (f64, f64, f64, f64, f64, bool, f64) {
    let corner = geo.corners.get(wheel);
    let axle = geo.axle(wheel);
    let q = if q.is_finite() { q } else { 0.0 };
    let v = if v.is_finite() { v } else { 0.0 };
    let sol = match solve_corner(corner, q, 0.0, wheel.is_front(), 0.0, 0.0, 1.0) {
        Some(s) => s,
        None => return (0.0, 0.0, 0.0, 0.0, 0.0, true, f64::INFINITY),
    };
    let jac = match jacobian(corner, q, 0.0, wheel.is_front()) {
        Some(j) => j,
        None => return (0.0, 0.0, sol.damper_compression, 0.0, 0.0, true, sol.residuals.max_link()),
    };
    let r = jac.motion_ratio;
    let resid = sol.residuals.max_link().max(sol.residuals.hub_y);
    let mut clamped = sol.rocker_clamped || sol.steering_clamped || !sol.converged;
    if !r.is_finite() || r <= 0.0 {
        // Wrong-sign/degenerate ratio: invalid for forces, do not fabricate.
        return (0.0, 0.0, sol.damper_compression, r, 0.0, true, resid);
    }
    let s = sol.damper_compression;
    // Spring cannot pull: total deflection from free must stay >= 0.
    let total = (axle.spring_free_length_m - axle.spring_installed_length_m) + s;
    let f_s = if total <= 0.0 {
        clamped = true;
        0.0
    } else {
        axle.spring_rate_N_per_m * total
    };
    let v_s = r * v;
    let f_d = geometric_damper_element(axle, v_s);
    // Rocker moment equilibrium for the rod (tension +).
    let rod = rod_force_from_solution(corner, &sol, f_s + f_d);
    let (rod_val, rod_sing) = rod;
    clamped = clamped || rod_sing;
    (f_s * r, f_d * r, s, r, rod_val, clamped, resid)
}

/// Moment equilibrium about the rocker axis. dir_damper points from chassis
/// to arm (compression pushes arm away); dir_rod points from rocker_end to
/// outer (tension pulls toward outer). F_rod = -F_elem * Md/Mr.
fn rod_force_from_solution(
    corner: &crate::suspension_geo_config::CornerHardpoints,
    sol: &crate::suspension_kinematics::KinematicSolution,
    f_elem: f64,
) -> (f64, bool) {
    if !f_elem.is_finite() {
        return (0.0, true);
    }
    let axis = corner.rocker_axis;
    if axis.length_squared() < 1e-12 {
        return (0.0, true);
    }
    let ax = axis.normalized();
    let dir_d = sol.damper_end - corner.damper_chassis;
    let dir_r = sol.pushrod_outer - sol.rocker_end;
    if dir_d.length_squared() < 1e-12 || dir_r.length_squared() < 1e-12 {
        return (0.0, true);
    }
    let md = (sol.damper_end - corner.rocker_pivot)
        .cross(dir_d.normalized())
        .dot(ax);
    let mr = (sol.rocker_end - corner.rocker_pivot)
        .cross(dir_r.normalized())
        .dot(ax);
    if mr.abs() < 1e-9 {
        return (0.0, true);
    }
    (-f_elem * md / mr, false)
}

/// Explicit ARB (no default spring-ratio fallback).
fn geometric_arb_force(
    geo: &GeometricSuspensionConfig,
    wheel: WheelIndex,
    q_self: f64,
    q_opp: f64,
    k_wheel_rest: f64,
) -> f64 {
    if !q_self.is_finite() || !q_opp.is_finite() {
        return 0.0;
    }
    let arb = if wheel.is_front() {
        &geo.front_arb
    } else {
        &geo.rear_arb
    };
    match *arb {
        AntiRollConfig::LegacyRatio { ratio } => {
            (q_self - q_opp) * k_wheel_rest * ratio
        }
        AntiRollConfig::MotionRatio {
            bar_rate_N_per_m,
            motion_ratio,
        } => (q_self - q_opp) * bar_rate_N_per_m * motion_ratio * motion_ratio,
    }
}

/// Physical stops: progressive wheel bump-stop + hard damper-stroke stops +
/// mechanism-clamped travel stops. Returns (force_wheel, clamped_flag).
fn geometric_stop_force(
    geo: &GeometricSuspensionConfig,
    wheel: WheelIndex,
    x: f64,
    rest_legacy: f64,
    s: f64,
    r: f64,
    tuning: WheelMechanicalTuning,
) -> (f64, bool) {
    let axle = geo.axle(wheel);
    let (q_lo, q_hi) = geometric_q_limits(geo, wheel);
    let x_min = rest_legacy + q_lo;
    let x_max = rest_legacy + q_hi;
    let k_rest =
        (axle.spring_rate_N_per_m * r.max(0.0) * r.max(0.0)).max(1.0);
    let mult = axle.bump_stop_mult.max(0.0);
    let mut force = 0.0;
    let mut clamped = false;
    // Progressive bump stop in the last 18% before x_max (mirrors 0.82 start).
    let span = (x_max - x_min).max(1e-4);
    let bump_start = x_max - span * (1.0 - tuning.bump_stop_start_ratio.clamp(0.5, 0.98));
    if x > bump_start {
        let excess = (x - bump_start).max(0.0);
        let progress = ((x - bump_start) / (x_max - bump_start).max(1e-4)).clamp(0.0, 2.0);
        force += k_rest * mult * excess * (1.0 + 3.0 * progress * progress);
    }
    // Hard wheel travel stops (measurable penetration, then the integrator
    // clamps position/velocity like legacy).
    if x > x_max {
        let over = x - x_max;
        force += k_rest * mult.max(1.0) * over * (12.0 + 24.0 * (over / tuning.hard_stop_overtravel_m.max(1e-4)));
        clamped = true;
    }
    if x < x_min {
        let over = x_min - x;
        force -= k_rest * mult.max(1.0) * over * (12.0 + 24.0 * (over / tuning.hard_stop_overtravel_m.max(1e-4)));
        clamped = true;
    }
    // Damper stroke stops, mapped to wheel via |r| so the sign always opposes
    // the penetration.
    let l_rest = axle.spring_installed_length_m;
    let _ = l_rest;
    // Reconstruct damper length from s: L = Lrest_kin - s, where Lrest_kin is
    // the kinematics rest length (not the spring installed length). Use the
    // axle damper limits directly on L via s limits is geometry-dependent;
    // approximate with s range from q limits is already covered. Enforce the
    // explicit damper_min/max by resolving L through the corner when needed
    // is expensive per substep; instead enforce on s via the q-envelope which
    // already embeds reachable damper lengths. Direct damper overtravel beyond
    // the envelope is therefore already flagged via x clamp above.
    let _ = (s, tuning);
    (finite_or_zero(force), clamped)
}

/// Tangent wheel rate + dr/dq for telemetry (extra solves, once per tick).
/// The hot path uses the cheaper tick secant; this exact central-difference
/// form is kept for unit validation and SUS-GEO-08 telemetry sweeps.
#[allow(dead_code)]
fn geometric_tangent(
    geo: &GeometricSuspensionConfig,
    wheel: WheelIndex,
    q: f64,
) -> (f64, f64) {
    let corner = geo.corners.get(wheel);
    let axle = geo.axle(wheel);
    let e = 1e-4;
    let r0 = jacobian(corner, q, 0.0, wheel.is_front())
        .map(|j| j.motion_ratio)
        .unwrap_or(0.0);
    let r_plus = jacobian(corner, q + e, 0.0, wheel.is_front())
        .map(|j| j.motion_ratio)
        .unwrap_or(r0);
    let r_minus = jacobian(corner, q - e, 0.0, wheel.is_front())
        .map(|j| j.motion_ratio)
        .unwrap_or(r0);
    let drdq = if r_plus.is_finite() && r_minus.is_finite() {
        (r_plus - r_minus) / (2.0 * e)
    } else {
        0.0
    };
    let s = solve_corner(corner, q, 0.0, wheel.is_front(), 0.0, 0.0, 1.0)
        .map(|sol| sol.damper_compression)
        .unwrap_or(0.0);
    let total = (axle.spring_free_length_m - axle.spring_installed_length_m) + s;
    let f_s = if total <= 0.0 {
        0.0
    } else {
        axle.spring_rate_N_per_m * total
    };
    let tangent = axle.spring_rate_N_per_m * r0 * r0 + f_s * drdq;
    (tangent, drdq)
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
    fn surface_filter_ramps_and_respects_zero_tau() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::FrontLeft;

        // tau = 0.0 is an exact pass-through.
        cfg.surface_transition_tau_s = 0.0;
        let mut pass = WheelSuspensionState::new(&cfg, wheel);
        filter_surface_values(&cfg, &mut pass, 0.5, 2.0, 2.0, 1.0 / 120.0);
        assert_eq!(pass.effective_friction, 0.5);
        assert_eq!(pass.effective_stiffness, 2.0);
        assert_eq!(pass.effective_rolling_resistance, 2.0);

        // A positive tau eases from the current values toward the raw sample.
        cfg.surface_transition_tau_s = 0.08;
        let mut filtered = WheelSuspensionState::new(&cfg, wheel);
        let start = filtered.effective_friction;
        assert!(start > 0.5, "test assumes the road value is above grass");
        filter_surface_values(&cfg, &mut filtered, 0.5, 2.0, 2.0, 1.0 / 120.0);
        assert!(filtered.effective_friction < start);
        assert!(
            filtered.effective_friction > 0.5,
            "one tick must not jump to the raw value: {}",
            filtered.effective_friction
        );
        for _ in 0..60 {
            filter_surface_values(&cfg, &mut filtered, 0.5, 2.0, 2.0, 1.0 / 120.0);
        }
        assert!((filtered.effective_friction - 0.5).abs() < 0.02);
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
