// Transient combined-slip tire model for Formula-90.
//
// The legacy GEVP brush implementation has been replaced by a compact model that
// consumes the new mechanical wheel state: dynamic Fz, carcass deflection, effective
// rolling radius, contact coverage and camber.  It intentionally remains tuneable and
// stable for a game solver rather than attempting a full Pacejka parameter set.
use crate::types::{SurfaceType, Vec3, WheelIndex};
use crate::vehicle_config::VehicleConfig;
use crate::wheel_mechanics::{
    base_contact_patch, static_wheel_load, tire_radius, wheel_mass, WheelMechanicalTuning,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelTireState {
    pub spin: f64,
    pub steer_angle_rad: f64,
    /// Instantaneous geometric slip targets.
    pub slip_angle_rad: f64,
    pub slip_ratio: f64,
    /// Relaxed carcass slips actually used to build force.
    #[serde(default)]
    pub effective_slip_angle_rad: f64,
    #[serde(default)]
    pub effective_slip_ratio: f64,
    pub lateral_force: f64,
    pub longitudinal_force: f64,
    pub rolling_resistance: f64,
    pub aligning_torque: f64,
    pub spin_velocity_diff: f64,
    pub wheel_moment: f64,
    pub applied_torque: f64,
    pub limit_spin: bool,
    pub reaction_torque: f64,

    // Mechanical coupling supplied by suspension.rs each tick.
    #[serde(default)]
    pub camber_rad: f64,
    #[serde(default)]
    pub tire_deflection_m: f64,
    /// Defaults to full contact (1.0) so legacy callers of `process_wheel_forces`
    /// that never feed mechanical state keep producing forces.
    #[serde(default)]
    pub contact_fraction: f64,
    #[serde(default)]
    pub effective_rolling_radius: f64,
    #[serde(default)]
    pub dynamic_contact_patch: f64,
    #[serde(default)]
    pub load_sensitivity_scale: f64,
}

impl WheelTireState {
    pub fn new(config: &VehicleConfig, wheel: WheelIndex) -> Self {
        let radius = tire_radius(config, wheel);
        let mass = wheel_mass(config, wheel);
        let tuning = WheelMechanicalTuning::for_wheel(config, wheel);
        Self {
            spin: 0.0,
            steer_angle_rad: 0.0,
            slip_angle_rad: 0.0,
            slip_ratio: 0.0,
            effective_slip_angle_rad: 0.0,
            effective_slip_ratio: 0.0,
            lateral_force: 0.0,
            longitudinal_force: 0.0,
            rolling_resistance: 0.0,
            aligning_torque: 0.0,
            spin_velocity_diff: 0.0,
            wheel_moment: tuning.rotational_inertia_factor * mass * radius * radius,
            applied_torque: 0.0,
            limit_spin: false,
            reaction_torque: 0.0,
            camber_rad: 0.0,
            tire_deflection_m: 0.0,
            contact_fraction: 1.0,
            effective_rolling_radius: radius,
            dynamic_contact_patch: base_contact_patch(config, wheel),
            load_sensitivity_scale: 1.0,
        }
    }
}

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

    pub fn reaction_torques(&self) -> [f64; 4] {
        [
            self.wheels[0].reaction_torque,
            self.wheels[1].reaction_torque,
            self.wheels[2].reaction_torque,
            self.wheels[3].reaction_torque,
        ]
    }

    /// Update tire geometry/load coupling before force generation. Keeping this as a
    /// separate call preserves the old process_wheel_forces signature for external callers.
    pub fn set_mechanical_state(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        normal_force_n: f64,
        camber_rad: f64,
        tire_deflection_m: f64,
        contact_fraction: f64,
    ) {
        let state = &mut self.wheels[wheel as usize];
        let tuning = WheelMechanicalTuning::for_wheel(config, wheel);
        let radius = tire_radius(config, wheel).max(0.05);
        let base_patch = base_contact_patch(config, wheel).max(0.03);
        let reference_load = static_wheel_load(config, wheel).max(100.0);
        let load_ratio = (normal_force_n.max(0.0) / reference_load).max(0.05);
        let deflection = tire_deflection_m.max(0.0).min(tuning.max_tire_deflection_m * 1.5);

        state.camber_rad = finite_or_zero(camber_rad);
        state.tire_deflection_m = deflection;
        state.contact_fraction = contact_fraction.clamp(0.0, 1.0);
        state.effective_rolling_radius = (radius
            - deflection * tuning.rolling_radius_deflection_factor)
            .clamp(radius * 0.86, radius);

        let deflection_ratio =
            (deflection / tuning.max_tire_deflection_m.max(1e-4)).clamp(0.0, 1.5);
        let patch_scale = (1.0
            + tuning.contact_patch_load_gain * (load_ratio - 1.0)
            + tuning.contact_patch_deflection_gain * deflection_ratio)
            .clamp(0.70, 1.45);
        state.dynamic_contact_patch = base_patch * patch_scale;
        state.load_sensitivity_scale = load_ratio
            .powf(tuning.load_sensitivity_exponent - 1.0)
            .clamp(0.75, 1.20);
    }

    /// Update wheel angular speed from applied torque and the previous tire reaction torque.
    pub fn process_wheel_torque(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        drive_torque_nm: f64,
        drive_inertia: f64,
        brake_torque_nm: f64,
        dt: f64,
    ) {
        let state = &mut self.wheels[wheel as usize];
        let dt = dt.max(1e-5);
        let is_driven = is_driven(config, wheel);
        let inertia = state.wheel_moment
            + if is_driven {
                drive_inertia.max(0.0)
            } else {
                0.0
            };
        let previous_spin = state.spin;

        state.applied_torque = if state.spin.abs() < 1e-6 {
            (drive_torque_nm - brake_torque_nm).abs()
        } else {
            (drive_torque_nm - brake_torque_nm * state.spin.signum()).abs()
        };

        let mut net_torque = drive_torque_nm + state.reaction_torque;
        if state.spin.abs() > 1e-5 {
            net_torque -= brake_torque_nm * state.spin.signum();
        } else if brake_torque_nm >= net_torque.abs() {
            net_torque = 0.0;
        }

        let mut new_spin = state.spin + (net_torque / inertia.max(1e-6)) * dt;
        if previous_spin.abs() > 1e-6
            && previous_spin.signum() != new_spin.signum()
            && brake_torque_nm > drive_torque_nm.abs()
        {
            new_spin = 0.0;
        }
        state.spin = finite_or_zero(new_spin);
    }

    /// Transient combined-slip tire force calculation.
    #[allow(clippy::too_many_arguments)]
    pub fn process_wheel_forces(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        normal_force_n: f64,
        surface: SurfaceType,
        effective_friction: f64,
        effective_stiffness: f64,
        effective_rolling_resistance: f64,
        braking: bool,
        local_wheel_velocity: Vec3,
        dt: f64,
    ) {
        let state = &mut self.wheels[wheel as usize];
        let dt = dt.max(1e-5);
        let tuning = WheelMechanicalTuning::for_wheel(config, wheel);

        if normal_force_n <= 1e-4 || state.contact_fraction <= 0.0 {
            let decay = (-dt / 0.08).exp();
            state.slip_angle_rad = 0.0;
            state.slip_ratio = 0.0;
            state.effective_slip_angle_rad *= decay;
            state.effective_slip_ratio *= decay;
            state.lateral_force = 0.0;
            state.longitudinal_force = 0.0;
            state.rolling_resistance = 0.0;
            state.aligning_torque = 0.0;
            state.spin_velocity_diff = 0.0;
            state.reaction_torque = 0.0;
            state.limit_spin = false;
            state.spin -= state.spin.signum()
                * (tire_airborne_decay(config, wheel) / state.wheel_moment.max(1e-6))
                * dt;
            if state.spin.abs() < 1e-4 {
                state.spin = 0.0;
            }
            return;
        }

        let v_forward = -local_wheel_velocity.z;
        let v_lateral = local_wheel_velocity.x;
        let rolling_radius = state.effective_rolling_radius.max(0.05);
        let wheel_velocity = state.spin * rolling_radius;
        state.spin_velocity_diff = wheel_velocity - v_forward;

        // Geometric slip targets. atan2 form is well-behaved near zero speed and forces
        // oppose lateral motion with the existing +X-right / -Z-forward convention.
        let target_alpha = (-v_lateral)
            .atan2(v_forward.abs().max(0.50))
            .clamp(-0.75, 0.75);
        let slip_denominator = v_forward
            .abs()
            .max(wheel_velocity.abs())
            .max(1.0);
        let target_kappa = ((wheel_velocity - v_forward) / slip_denominator).clamp(-3.0, 3.0);
        state.slip_angle_rad = target_alpha;
        state.slip_ratio = target_kappa;

        // Relaxation length makes force build over distance instead of appearing in one frame.
        let planar_speed = (v_forward * v_forward + v_lateral * v_lateral).sqrt();
        let base_patch = base_contact_patch(config, wheel).max(0.03);
        let patch_ratio = (state.dynamic_contact_patch / base_patch).clamp(0.7, 1.5);
        let relaxation_length = tuning.relaxation_length_m * patch_ratio.sqrt();
        let relaxation_tau = relaxation_length / planar_speed.max(4.0);
        let relax = 1.0 - (-dt / relaxation_tau.max(1e-4)).exp();
        state.effective_slip_angle_rad +=
            (target_alpha - state.effective_slip_angle_rad) * relax;
        state.effective_slip_ratio +=
            (target_kappa - state.effective_slip_ratio) * relax;

        let fz = normal_force_n.max(0.0);
        let mut mu = effective_friction.max(0.0) * state.load_sensitivity_scale;
        // Preserve the per-axle braking-grip multiplier config contract: it scales the
        // contact friction while braking (replaces the old GEVP braking_help hack with
        // a direct brake-compound mu multiplier).
        if braking {
            let grip = if wheel.is_front() {
                config.front_braking_grip
            } else {
                config.rear_braking_grip
            };
            mu *= grip.max(0.0);
        }
        let longitudinal_ratio = config
            .surface_longitudinal_grip_ratio
            .get(&surface)
            .copied()
            .unwrap_or(0.5)
            .clamp(0.10, 1.50);
        let lateral_assist = config
            .surface_lateral_grip_assist
            .get(&surface)
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);

        let fx_max = (mu * longitudinal_ratio * fz).max(1e-6);
        let fy_max = (mu * fz).max(1e-6);
        let stiffness_scale = (effective_stiffness.max(0.0) / 5.0)
            .clamp(0.15, 2.5)
            .sqrt();
        let patch_stiffness_scale = patch_ratio.sqrt();
        let longitudinal_shape = 9.5 * stiffness_scale * patch_stiffness_scale;
        let lateral_shape = 7.5
            * stiffness_scale
            * patch_stiffness_scale
            * (1.0 + 0.25 * lateral_assist);

        // Config camber is expressed with the same sign on both sides. Mirror the right
        // side so static camber thrust is symmetric and cancels on a straight, flat road.
        let signed_camber = if wheel.is_right() {
            -state.camber_rad
        } else {
            state.camber_rad
        };
        let alpha_with_camber = state.effective_slip_angle_rad
            + signed_camber * tuning.camber_thrust_gain;

        let mut fx = fx_max * (longitudinal_shape * state.effective_slip_ratio).tanh();
        let mut fy = fy_max * (lateral_shape * alpha_with_camber).tanh();

        // Friction ellipse: longitudinal and lateral demands consume the same contact patch.
        let utilization = ((fx / fx_max).powi(2) + (fy / fy_max).powi(2)).sqrt();
        if utilization > 1.0 {
            fx /= utilization;
            fy /= utilization;
        }
        state.limit_spin = utilization >= 0.995 || state.effective_slip_ratio.abs() > 0.20;

        state.rolling_resistance =
            rolling_resistance_force(v_forward, fz) * effective_rolling_resistance.max(0.0);
        if v_forward.abs() > 0.05 {
            fx -= state.rolling_resistance * v_forward.signum();
        }

        state.lateral_force = finite_or_zero(fy);
        state.longitudinal_force = finite_or_zero(fx);

        // Pneumatic trail collapses past the force peak and under heavy longitudinal slip,
        // giving the steering a natural load-up then release at front-tire saturation.
        let alpha_decay =
            (1.0 - (state.effective_slip_angle_rad.abs() / 0.35).clamp(0.0, 1.0)).max(0.0);
        let kappa_decay =
            (1.0 - 0.65 * state.effective_slip_ratio.abs().clamp(0.0, 1.0)).max(0.0);
        let trail = tuning.pneumatic_trail_m
            * alpha_decay
            * kappa_decay
            * state.contact_fraction.sqrt();
        state.aligning_torque = finite_or_zero(-state.lateral_force * trail);

        // Torque exerted BY THE ROAD ON THE WHEEL; positive traction opposes positive spin.
        state.reaction_torque =
            finite_or_zero(-state.longitudinal_force * state.effective_rolling_radius);
    }

    /// Compatibility entry point retained for callers outside simulation.rs.
    #[allow(clippy::too_many_arguments)]
    pub fn step_wheel(
        &mut self,
        config: &VehicleConfig,
        wheel: WheelIndex,
        normal_force_n: f64,
        effective_friction_coef: f64,
        drive_torque_nm: f64,
        brake_torque_nm: f64,
        local_wheel_velocity: Vec3,
        dt: f64,
        effective_gear_ratio: f64,
        clutch_engagement: f64,
    ) {
        let total_ratio = effective_gear_ratio * config.final_drive;
        let drive_inertia = if is_driven(config, wheel) {
            (config.motor_moment
                * total_ratio
                * total_ratio
                * clutch_engagement.clamp(0.0, 1.0))
            .max(0.0)
        } else {
            0.0
        };
        self.process_wheel_torque(
            config,
            wheel,
            drive_torque_nm,
            drive_inertia,
            brake_torque_nm,
            dt,
        );
        let surface = SurfaceType::Road;
        let stiffness = *config.surface_stiffness.get(&surface).unwrap_or(&5.0);
        let rolling = *config
            .surface_rolling_resistance
            .get(&surface)
            .unwrap_or(&1.0);

        // Compatibility callers do not have suspension mechanical state. Use neutral
        // full-contact values so old API users still get deterministic tire forces.
        self.set_mechanical_state(config, wheel, normal_force_n, 0.0, 0.0, 1.0);
        self.process_wheel_forces(
            config,
            wheel,
            normal_force_n,
            surface,
            effective_friction_coef,
            stiffness,
            rolling,
            brake_torque_nm > 0.0,
            local_wheel_velocity,
            dt,
        );
    }
}

pub fn is_driven(config: &VehicleConfig, wheel: WheelIndex) -> bool {
    if wheel.is_front() {
        config.front_torque_split > 0.0
    } else {
        config.front_torque_split < 1.0
    }
}

fn tire_airborne_decay(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_airborne_decay
    } else {
        config.rear_airborne_decay
    }
}

fn rolling_resistance_force(v_forward: f64, normal_force: f64) -> f64 {
    let c = 0.005 + 0.5 * (0.01 + 0.0095 * (v_forward * 0.036).powi(2));
    c * normal_force
}

fn finite_or_zero(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tire_deflection_reduces_rolling_radius_and_grows_patch() {
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::FrontLeft;
        let mut tires = TireSystem::new(&cfg);
        let base_radius = cfg.front_tire_radius;
        let base_patch = cfg.front_contact_patch;
        let fz = static_wheel_load(&cfg, wheel) * 1.5;
        tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.015, 1.0);
        let s = &tires.wheels[wheel as usize];
        assert!(s.effective_rolling_radius < base_radius);
        assert!(s.dynamic_contact_patch > base_patch);
    }

    #[test]
    fn load_sensitivity_reduces_mu_scale_at_high_load() {
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::FrontLeft;
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(
            &cfg,
            wheel,
            static_wheel_load(&cfg, wheel) * 2.0,
            0.0,
            0.008,
            1.0,
        );
        assert!(tires.wheels[wheel as usize].load_sensitivity_scale < 1.0);
    }

    #[test]
    fn legacy_callers_without_mechanical_state_still_produce_forces() {
        // Regression: `contact_fraction` defaults to full contact so direct
        // `process_wheel_forces` calls (external API users) do not hit the airborne path.
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::RearLeft;
        let mut tires = TireSystem::new(&cfg);
        tires.wheels[wheel as usize].spin = 50.0;
        let normal = cfg.mass_over_wheel(wheel) * 9.80665;
        tires.process_wheel_forces(
            &cfg,
            wheel,
            normal,
            SurfaceType::Road,
            2.9,
            8.75,
            1.0,
            false,
            Vec3::new(0.0, 0.0, -10.0),
            1.0 / 120.0,
        );
        assert!(
            tires.wheels[wheel as usize].longitudinal_force > 0.0,
            "legacy direct caller must produce traction force"
        );
    }

    #[test]
    fn slip_relaxation_builds_force_over_distance() {
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::RearLeft;
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(
            &cfg,
            wheel,
            static_wheel_load(&cfg, wheel),
            0.0,
            0.008,
            1.0,
        );
        tires.wheels[wheel as usize].spin = 60.0;
        let normal = static_wheel_load(&cfg, wheel);
        let dt = 1.0 / 120.0;
        let vel = Vec3::new(0.0, 0.0, -20.0);

        let mut first_force = 0.0;
        for tick in 0..40 {
            tires.process_wheel_forces(
                &cfg, wheel, normal, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
            );
            let s = &tires.wheels[wheel as usize];
            if tick == 0 {
                first_force = s.longitudinal_force.abs();
            }
            if tick == 39 {
                let steady = s.longitudinal_force.abs();
                assert!(
                    steady > first_force * 2.0,
                    "relaxed force must build well beyond the first tick: first={first_force} steady={steady}"
                );
                assert!(
                    (s.effective_slip_ratio - s.slip_ratio).abs() < 0.05,
                    "effective slip must converge to the geometric target"
                );
            }
        }
    }

    #[test]
    fn aligning_torque_opposes_lateral_force_and_collapses_at_saturation() {
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::FrontLeft;
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(
            &cfg,
            wheel,
            static_wheel_load(&cfg, wheel),
            0.0,
            0.008,
            1.0,
        );
        tires.wheels[wheel as usize].spin = 60.0;
        let normal = static_wheel_load(&cfg, wheel);
        let dt = 1.0 / 120.0;

        // Lateral motion to the right (+X) produces positive lateral force... the slip
        // convention makes fy oppose the motion; check torque sign consistency instead.
        let vel = Vec3::new(3.0, 0.0, -20.0);
        for _ in 0..20 {
            tires.process_wheel_forces(
                &cfg, wheel, normal, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
            );
        }
        let s = &tires.wheels[wheel as usize];
        assert!(s.lateral_force.abs() > 100.0, "lateral force must build");
        assert!(
            (s.aligning_torque * s.lateral_force).signum() <= 0.0,
            "aligning torque must oppose lateral force sign: mz={} fy={}",
            s.aligning_torque,
            s.lateral_force
        );
        assert!(
            s.aligning_torque.abs() > 0.0,
            "aligning torque must be non-zero below saturation"
        );

        // Deep slip collapses the pneumatic trail.
        let hard = Vec3::new(20.0, 0.0, -20.0);
        let mut tires2 = TireSystem::new(&cfg);
        tires2.set_mechanical_state(&cfg, wheel, normal, 0.0, 0.008, 1.0);
        tires2.wheels[wheel as usize].spin = 60.0;
        for _ in 0..40 {
            tires2.process_wheel_forces(
                &cfg, wheel, normal, SurfaceType::Road, 2.9, 8.75, 1.0, false, hard, dt,
            );
        }
        let s2 = &tires2.wheels[wheel as usize];
        assert!(
            s2.aligning_torque.abs() < tires.wheels[wheel as usize].aligning_torque.abs(),
            "aligning torque must collapse at high slip"
        );
    }

    #[test]
    fn mirrored_static_camber_thrust_cancels_on_straight_line() {
        let cfg = VehicleConfig::f1_94_canonical();
        let mut left = TireSystem::new(&cfg);
        let mut right = TireSystem::new(&cfg);
        let normal = static_wheel_load(&cfg, WheelIndex::FrontLeft);
        let dt = 1.0 / 120.0;
        let vel = Vec3::new(0.0, 0.0, -20.0);
        left.set_mechanical_state(&cfg, WheelIndex::FrontLeft, normal, -0.0383972, 0.008, 1.0);
        right.set_mechanical_state(&cfg, WheelIndex::FrontRight, normal, -0.0383972, 0.008, 1.0);
        left.wheels[0].spin = 60.0;
        right.wheels[1].spin = 60.0;
        for _ in 0..20 {
            left.process_wheel_forces(
                &cfg, WheelIndex::FrontLeft, normal, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
            );
            right.process_wheel_forces(
                &cfg, WheelIndex::FrontRight, normal, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
            );
        }
        let fl = left.wheels[0].lateral_force;
        let fr = right.wheels[1].lateral_force;
        assert!(
            (fl + fr).abs() < fl.abs() * 0.05,
            "left/right camber thrust must cancel on a straight: fl={fl} fr={fr}"
        );
    }
}
