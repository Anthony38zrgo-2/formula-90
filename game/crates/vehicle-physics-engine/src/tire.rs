// Transient combined-slip tire model for Formula-90.
//
// The legacy GEVP brush implementation has been replaced by a compact model that
// consumes the new mechanical wheel state: dynamic Fz, carcass deflection, effective
// rolling radius, contact coverage and camber.  It intentionally remains tuneable and
// stable for a game solver rather than attempting a full Pacejka parameter set.
use crate::tire_thermals::TireMechanicalModifiers;
use crate::types::{SurfaceType, Vec3, WheelIndex};
use crate::vehicle_config::{TireForceProfile, VehicleConfig};
use crate::wheel_mechanics::{
    base_contact_patch, static_wheel_load, tire_radius, wheel_mass, WheelMechanicalTuning,
};
use serde::{Deserialize, Serialize};

/// Axis regime reported by the pure-slip envelope (TIRE-101/103 diagnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TireSlipRegime {
    PrePeak,
    PostPeak,
}

/// Result of evaluating one pure-slip axis (TIRE-101). Magnitude only; sign is
/// applied by the caller so odd symmetry is easy to test.
#[derive(Debug, Clone, Copy)]
pub struct PureSlipResponse {
    pub force_coefficient: f64,
    pub regime: TireSlipRegime,
    /// How far the axis sits on the post-peak decay branch (0 on the pre-peak
    /// rise, approaching 1 deep into the slide). GRIP-02 uses this as the target
    /// of the per-wheel sliding-memory state.
    pub decay_weight: f64,
}

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
    /// Raw combined-slip demand `sqrt(nx^2 + ny^2)` before the budget scale.
    /// May exceed 1 (up to sqrt(2)); kept separate from the effective value so
    /// telemetry and diagnostics can distinguish demand from delivery.
    #[serde(default)]
    pub combined_demand: f64,
    /// Effective friction-budget utilization of the traction forces actually
    /// applied (0..=1). Never exceeds 1 by construction, including the GRIP-01
    /// smoothing band.
    #[serde(default)]
    pub combined_utilization: f64,
    /// Axis regime diagnostic: 0 = pre-peak both axes, 1 = lateral post-peak,
    /// 2 = longitudinal post-peak, 3 = both post-peak.
    #[serde(default)]
    pub tire_regime: i32,
    /// GRIP-02 sliding memory per axis: filtered post-peak decay weight in
    /// `0..=1`. `0` behaves as a fully gripping carcass, `1` as fully committed
    /// sliding. Each axis follows its own target (`slip_loss_tau_s` on the way
    /// in, `slip_recovery_tau_s` on the way out), so sliding one axis cannot
    /// penalize the other outside the explicit combined-slip projection.
    #[serde(default)]
    pub post_peak_decay_lat: f64,
    #[serde(default)]
    pub post_peak_decay_lon: f64,
    /// GRIP-04 combined-slip peak migration diagnostics: the peaks actually used
    /// this tick after the opposite axis' demand shrank them.
    #[serde(default)]
    pub effective_lateral_peak_slip_rad: f64,
    #[serde(default)]
    pub effective_longitudinal_peak_slip_ratio: f64,

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
    /// Pressure/thermal mechanical modifiers applied at force time. Identity by
    /// default so legacy callers that never feed thermal state behave unchanged.
    #[serde(default)]
    pub mechanical_modifiers: TireMechanicalModifiers,
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
            combined_demand: 0.0,
            combined_utilization: 0.0,
            tire_regime: 0,
            post_peak_decay_lat: 0.0,
            post_peak_decay_lon: 0.0,
            effective_lateral_peak_slip_rad: 0.0,
            effective_longitudinal_peak_slip_ratio: 0.0,
            camber_rad: 0.0,
            tire_deflection_m: 0.0,
            contact_fraction: 1.0,
            effective_rolling_radius: radius,
            dynamic_contact_patch: base_contact_patch(config, wheel),
            load_sensitivity_scale: 1.0,
            mechanical_modifiers: TireMechanicalModifiers::identity(),
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

    /// Apply the pressure/thermal mechanical modifiers computed this tick. Called
    /// BEFORE force generation so the combined-slip solver sees the current pressure.
    pub fn set_mechanical_modifiers(
        &mut self,
        wheel: WheelIndex,
        modifiers: TireMechanicalModifiers,
    ) {
        self.wheels[wheel as usize].mechanical_modifiers = modifiers;
    }

    /// Update wheel angular speed from applied torque and the previous tire reaction torque.
    ///
    /// GRIP-03: when `wheel_lock_blend_spin_rad_s > 0` the brake torque fades
    /// linearly to zero inside that spin band and the static stick is integrated
    /// implicitly, so the wheel approaches lock progressively instead of
    /// snapping to zero. `blend = 0` keeps the exact legacy algorithm.
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
        let inertia = (state.wheel_moment
            + if is_driven {
                drive_inertia.max(0.0)
            } else {
                0.0
            })
        .max(1e-6);
        let previous_spin = state.spin;
        let unbraked_torque = drive_torque_nm + state.reaction_torque;
        let blend = config.wheel_lock_blend_spin_rad_s.max(0.0);

        if blend <= 1e-9 {
            Self::legacy_wheel_torque_step(
                state,
                drive_torque_nm,
                brake_torque_nm,
                unbraked_torque,
                previous_spin,
                inertia,
                dt,
            );
            return;
        }

        // GRIP-03 regularized path: brake torque always opposes the wheel and
        // fades linearly to zero inside the band, so no torque discontinuity
        // can teleport the spin to lock.
        let spin = finite_or_zero(state.spin);
        let spin_abs = spin.abs();
        let brake_torque = if spin_abs < blend {
            brake_torque_nm * (spin / blend)
        } else {
            brake_torque_nm * spin.signum()
        };
        state.applied_torque = (unbraked_torque - brake_torque).abs();
        // Static friction balances the unbraked torque as long as the brake
        // capacity covers it; otherwise the soft kinetic band drives the spin.
        let brake_holds = brake_torque_nm >= unbraked_torque.abs();
        let park_rate = if config.wheel_lock_stick_tau_s > 1e-9 {
            1.0 / config.wheel_lock_stick_tau_s
        } else {
            brake_torque_nm / (blend * inertia)
        };
        let mut new_spin = if spin_abs < blend && brake_holds {
            spin / (1.0 + dt * park_rate)
        } else if spin_abs < blend {
            // I*s' = a - Tb*s/blend; implicit integration is stable at any dt.
            let denom = 1.0 + dt * (brake_torque_nm / blend) / inertia;
            (spin + dt * unbraked_torque / inertia) / denom
        } else {
            spin + dt * (unbraked_torque - brake_torque) / inertia
        };
        // Stiction: while the brake can hold the unbraked torque the wheel
        // cannot roll through zero. This is the physical stop, not a snap.
        if brake_holds && spin * new_spin < 0.0 {
            new_spin = 0.0;
        }
        state.spin = finite_or_zero(new_spin);
    }
    /// Legacy hard-stick wheel torque step (GRIP-03 fallback, exact parity with
    /// the pre-regularization model).
    fn legacy_wheel_torque_step(
        state: &mut WheelTireState,
        drive_torque_nm: f64,
        brake_torque_nm: f64,
        unbraked_torque: f64,
        previous_spin: f64,
        inertia: f64,
        dt: f64,
    ) {
        state.applied_torque = if state.spin.abs() < 1e-6 {
            (drive_torque_nm - brake_torque_nm).abs()
        } else {
            (drive_torque_nm - brake_torque_nm * state.spin.signum()).abs()
        };

        let mut net_torque = unbraked_torque;
        if state.spin.abs() > 1e-5 {
            net_torque -= brake_torque_nm * state.spin.signum();
        } else if brake_torque_nm >= net_torque.abs() {
            net_torque = 0.0;
        }

        let mut new_spin = state.spin + (net_torque / inertia) * dt;
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
        _braking: bool,
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
            state.combined_demand = 0.0;
            state.combined_utilization = 0.0;
            state.tire_regime = 0;
            // GRIP-02: an unloaded tire is not sliding; release both memories
            // with the profile recovery tau so landing does not inherit a slide.
            let recovery_tau = tire_force_profile(config, wheel).slip_recovery_tau_s;
            let release = if recovery_tau <= 1e-6 {
                0.0
            } else {
                (-dt / recovery_tau.max(1e-6)).exp()
            };
            state.post_peak_decay_lat *= release;
            state.post_peak_decay_lon *= release;
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
        let modifiers = state.mechanical_modifiers;
        let base_patch = base_contact_patch(config, wheel).max(0.03);
        let patch_ratio =
            (state.dynamic_contact_patch / base_patch).clamp(0.7, 1.5) * modifiers.contact_patch_scale;
        let relaxation_length = tuning.relaxation_length_m
            * patch_ratio.sqrt()
            * modifiers.relaxation_length_scale;
        let relaxation_tau = relaxation_length / planar_speed.max(4.0);
        let relax = 1.0 - (-dt / relaxation_tau.max(1e-4)).exp();
        state.effective_slip_angle_rad +=
            (target_alpha - state.effective_slip_angle_rad) * relax;
        state.effective_slip_ratio +=
            (target_kappa - state.effective_slip_ratio) * relax;

        let fz = normal_force_n.max(0.0);
        // TIRE-600: braking no longer multiplies mu. Peak tire capacity comes only from
        // friction, load sensitivity, the surface longitudinal grip ratio and the thermal
        // grip window; brake torque is consumed through the shared friction budget below.
        let mu = effective_friction.max(0.0)
            * state.load_sensitivity_scale
            * modifiers.grip_scale.clamp(0.05, 2.0);
        let longitudinal_ratio = config
            .surface_longitudinal_grip_ratio
            .get(&surface)
            .copied()
            .unwrap_or(0.5)
            .clamp(0.10, 1.50);

        let fx_max = (mu * longitudinal_ratio * fz).max(1e-6);
        let fy_max = (mu * fz).max(1e-6);
        let stiffness_scale = (effective_stiffness.max(0.0) / 5.0)
            .clamp(0.15, 2.5)
            .sqrt();
        let patch_stiffness_scale = patch_ratio.sqrt();
        // TIRE-203: the legacy tanh shape constants are retired. A small pre-peak
        // rise scaling (rise_gamma) may shape force build, but it can never erase the
        // configured peak slip / slide-ratio relationship of the pure-slip envelope.
        let rise_gamma = (2.0 * stiffness_scale * patch_stiffness_scale * modifiers.force_stiffness_scale)
            .clamp(1.2, 4.0);

        let profile = tire_force_profile(config, wheel);

        // Config camber is expressed with the same sign on both sides. Mirror the right
        // side so static camber thrust is symmetric and cancels on a straight, flat road.
        let signed_camber = if wheel.is_right() {
            -state.camber_rad
        } else {
            state.camber_rad
        };
        let alpha_with_camber = state.effective_slip_angle_rad
            + signed_camber * tuning.camber_thrust_gain;

        // TIRE-101 / GRIP-02 / GRIP-04: pure-slip curve evaluation lives outside
        // combined-slip projection. Base responses first measure each axis' demand,
        // then the opposite axis' peak is migrated (combined-slip peak migration)
        // and the curves are re-evaluated before the budget, so the axes still
        // report their own post-peak decay weight.
        let lat_base = pure_curve_coefficient(
            alpha_with_camber.abs(),
            profile.lateral_peak_slip_angle_rad,
            profile.lateral_slide_mu_ratio,
            rise_gamma,
            profile.falloff_sharpness,
        );
        let lon_base = pure_curve_coefficient(
            state.effective_slip_ratio.abs(),
            profile.longitudinal_peak_slip_ratio,
            profile.longitudinal_slide_mu_ratio,
            rise_gamma,
            profile.falloff_sharpness,
        );
        // Slip-based demand keeps the migration monotone in slip: once the axis
        // reaches its nominal peak the peak shift stays at full strength instead
        // of recovering as the force curve decays.
        let lateral_demand = (alpha_with_camber.abs()
            / profile.lateral_peak_slip_angle_rad.max(1e-6))
        .clamp(0.0, 1.0);
        let longitudinal_demand = (state.effective_slip_ratio.abs()
            / profile.longitudinal_peak_slip_ratio.max(1e-6))
        .clamp(0.0, 1.0);
        let lateral_peak = profile.lateral_peak_slip_angle_rad
            * (1.0
                - profile.combined_lateral_peak_migration.clamp(0.0, 0.9)
                    * longitudinal_demand)
                .max(0.1);
        let longitudinal_peak = profile.longitudinal_peak_slip_ratio
            * (1.0
                - profile.combined_longitudinal_peak_migration.clamp(0.0, 0.9)
                    * lateral_demand)
                .max(0.1);
        state.effective_lateral_peak_slip_rad = lateral_peak;
        state.effective_longitudinal_peak_slip_ratio = longitudinal_peak;
        let lat_response = pure_curve_coefficient(
            alpha_with_camber.abs(),
            lateral_peak,
            profile.lateral_slide_mu_ratio,
            rise_gamma,
            profile.falloff_sharpness,
        );
        let lon_response = pure_curve_coefficient(
            state.effective_slip_ratio.abs(),
            longitudinal_peak,
            profile.longitudinal_slide_mu_ratio,
            rise_gamma,
            profile.falloff_sharpness,
        );

        // GRIP-02 sliding memory, independent per axis. Each state follows its
        // own post-peak decay target with an asymmetric first order lag (loss
        // faster than recovery). Taus are profile data in seconds; `0.0` snaps
        // to the target, exactly preserving the legacy instantaneous curve.
        let advance_memory = |state: f64, target: f64| -> f64 {
            let tau = if target > state {
                profile.slip_loss_tau_s
            } else {
                profile.slip_recovery_tau_s
            };
            if tau <= 1e-6 {
                target
            } else {
                let relax = 1.0 - (-dt / tau.max(1e-6)).exp();
                (state + (target - state) * relax).clamp(0.0, 1.0)
            }
        };
        state.post_peak_decay_lat =
            advance_memory(state.post_peak_decay_lat, lat_response.decay_weight);
        state.post_peak_decay_lon =
            advance_memory(state.post_peak_decay_lon, lon_response.decay_weight);

        // With each memory at its target the effective coefficient is exactly the
        // pure envelope (steady state unchanged for any tau). During transients
        // the flat, slide-free curve is rescaled by its own memory state, which
        // delays the loss on entry and forces a progressive grip release on exit.
        let lat_flat = lat_response.force_coefficient
            / (1.0 - lat_response.decay_weight * (1.0 - profile.lateral_slide_mu_ratio))
                .max(1e-6);
        let lon_flat = lon_response.force_coefficient
            / (1.0 - lon_response.decay_weight * (1.0 - profile.longitudinal_slide_mu_ratio))
                .max(1e-6);
        let lat_coefficient = lat_flat
            * (1.0 - state.post_peak_decay_lat * (1.0 - profile.lateral_slide_mu_ratio));
        let lon_coefficient = lon_flat
            * (1.0 - state.post_peak_decay_lon * (1.0 - profile.longitudinal_slide_mu_ratio));
        let fx_candidate =
            fx_max * lon_coefficient * state.effective_slip_ratio.signum();
        let fy_candidate =
            fy_max * lat_coefficient * alpha_with_camber.signum();

        // TIRE-102: one shared normalized friction budget. Normalized demand is scaled
        // coherently on both axes with a fixed internal ellipse exponent; no single-axis
        // clip and no user-visible combined-slip exponent. Rolling resistance is applied
        // AFTER the budget so it cannot inflate demand beyond traction capacity.
        // GRIP-01: the legacy hard clip (scale = 1/u for u > 1) left a slope
        // discontinuity at the knee. The profile can spread that transition over a
        // C1 band that never exceeds the budget; the slight early easing near the
        // knee is the price of keeping C1 and the cap simultaneously. Width 0
        // keeps legacy behaviour exact.
        // GRIP-04: the budget demand is the larger of the nominal and migrated
        // envelopes. When migration lowers the coefficients the nominal envelope
        // binds, so the migrated tire delivers less force instead of freeing
        // headroom to boost the opposite axis; when migration raises them (peak
        // shifted toward the current slip) the migrated envelope binds and the cap
        // is still respected.
        let base_norm = (lon_base.force_coefficient * lon_base.force_coefficient
            + lat_base.force_coefficient * lat_base.force_coefficient)
            .sqrt();
        let migrated_norm =
            (lon_coefficient * lon_coefficient + lat_coefficient * lat_coefficient).sqrt();
        let demand = base_norm.max(migrated_norm);
        let budget_scale = combined_budget_scale(demand, profile.budget_blend_width);
        let (mut fx, fy) = (fx_candidate * budget_scale, fy_candidate * budget_scale);
        // Demand and effective utilization stay separate: the demand can exceed 1
        // (up to sqrt(2)), while the traction actually delivered is always inside
        // the shared budget.
        state.combined_demand = demand;
        state.combined_utilization = (migrated_norm * budget_scale).clamp(0.0, 1.0);

        // TIRE-103: saturation state derives from utilization and post-peak regime
        // instead of fixed slip-threshold decisions.
        let post_peak = lat_response.regime == TireSlipRegime::PostPeak
            || lon_response.regime == TireSlipRegime::PostPeak;
        state.limit_spin = state.combined_utilization >= 0.995 || post_peak;
        state.tire_regime = match (lat_response.regime, lon_response.regime) {
            (TireSlipRegime::PrePeak, TireSlipRegime::PrePeak) => 0,
            (TireSlipRegime::PostPeak, TireSlipRegime::PrePeak) => 1,
            (TireSlipRegime::PrePeak, TireSlipRegime::PostPeak) => 2,
            (TireSlipRegime::PostPeak, TireSlipRegime::PostPeak) => 3,
        };

        state.rolling_resistance = rolling_resistance_force(v_forward, fz)
            * effective_rolling_resistance.max(0.0)
            * modifiers.rolling_resistance_scale;
        if v_forward.abs() > 0.05 {
            fx -= state.rolling_resistance * v_forward.signum();
        }

        state.lateral_force = finite_or_zero(fy);
        state.longitudinal_force = finite_or_zero(fx);

        // TIRE-104: pneumatic-trail collapse depends on lateral saturation/post-peak
        // state and longitudinal utilization, not a fixed absolute slip-angle decay.
        let nx_final = (fx.abs() / fx_max).clamp(0.0, 1.0);
        let ny_final = (fy.abs() / fy_max).clamp(0.0, 1.0);
        let alpha_decay = (-2.5 * ny_final).exp();
        let kappa_decay = (1.0 - 0.65 * nx_final * nx_final).max(0.0);
        let trail = tuning.pneumatic_trail_m
            * alpha_decay
            * kappa_decay
            * state.contact_fraction.sqrt()
            * modifiers.pneumatic_trail_scale;
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

fn tire_force_profile(config: &VehicleConfig, wheel: WheelIndex) -> &TireForceProfile {
    if wheel.is_front() {
        &config.front_tire_force
    } else {
        &config.rear_tire_force
    }
}

/// Compact C1 pure-slip envelope shared by both axes (TIRE-200).
///
/// Pre-peak: `c = 1 - (1 - u)^gamma` with `u = slip_abs / peak_slip`, rising
/// monotonically to `1.0` at `u = 1`. For `gamma > 1` the derivative approaches
/// zero at the peak, matching the post-peak derivative of zero (C1).
///
/// Post-peak: `c = slide + (1 - slide) / (1 + (s * (u - 1))^2)`, decaying
/// monotonically toward the configured sliding plateau `slide` and holding a
/// zero derivative at the peak. `falloff_sharpness = s` is profile data since
/// GRIP-02; `2.0` matches the legacy internal constant.
///
/// The returned `decay_weight` is `d^2 / (1 + d^2)`, i.e. the normalized
/// progress of the post-peak decay (`0` pre-peak, approaching `1` deep into the
/// slide). It satisfies `force_coefficient = 1 - decay_weight * (1 - slide)`.
pub fn pure_curve_coefficient(
    slip_abs: f64,
    peak_slip: f64,
    slide_mu_ratio: f64,
    rise_gamma: f64,
    falloff_sharpness: f64,
) -> PureSlipResponse {
    let peak = peak_slip.max(1e-6);
    let slide = slide_mu_ratio.clamp(0.0, 1.0);
    let gamma = rise_gamma.max(1.2);
    let u = slip_abs.max(0.0) / peak;
    if u <= 1.0 {
        let coefficient = (1.0 - (1.0 - u).powf(gamma)).clamp(0.0, 1.0);
        PureSlipResponse {
            force_coefficient: coefficient,
            regime: TireSlipRegime::PrePeak,
            decay_weight: 0.0,
        }
    } else {
        let sharpness = falloff_sharpness.clamp(0.1, 16.0);
        let d = sharpness * (u - 1.0);
        let decay_weight = (d * d) / (1.0 + d * d);
        let coefficient = (slide + (1.0 - slide) * (1.0 - decay_weight)).clamp(0.0, 1.0);
        PureSlipResponse {
            force_coefficient: coefficient,
            regime: TireSlipRegime::PostPeak,
            decay_weight,
        }
    }
}

/// C1 smoothing of the combined-slip budget knee (GRIP-01).
///
/// The legacy projection scaled both force axes by `1 / utilization` once the
/// normalized demand exceeded 1.0. That is continuous but leaves a slope jump at
/// the knee. With `blend_width = w > 0` the transition is spread over
/// `utilization in [1 - w, 1 + w]` with a cubic Hermite arc that matches both
/// the value and the slope of the constant branches (`1.0` below, `1/u` above),
/// so force and its derivative are C1 everywhere.
///
/// The arc is monotone and bounded by the hard clip from above: the applied
/// force never exceeds the shared friction budget (`utilization * scale <= 1`).
/// The cost is a slight early easing near the knee (~2.5% at `u = 1` for
/// `w = 0.1`), which acts as a progressive warning before the cap. The
/// invariant is impossible to keep together with zero sub-limit
/// deviation: any C1 scale that equals 1 up to the knee and stays below `1/u`
/// after it would need a positive slope below the knee, which would break the
/// constant branch. `w = 0` returns the legacy hard clip exactly.
pub fn combined_budget_scale(utilization: f64, blend_width: f64) -> f64 {
    let u = utilization.max(0.0);
    let w = blend_width.clamp(0.0, 0.5);
    if w <= 1e-9 {
        return if u > 1.0 { 1.0 / u } else { 1.0 };
    }
    let lo = 1.0 - w;
    let hi = 1.0 + w;
    if u <= lo {
        return 1.0;
    }
    if u >= hi {
        return 1.0 / u;
    }
    let h = hi - lo;
    let t = (u - lo) / h;
    let p1 = 1.0 / hi;
    let m1 = -1.0 / (hi * hi);
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    // Hermite: p0 = 1.0, m0 = 0.0 (constant branch below the band).
    let value = h00 + h10 * h * 0.0 + h01 * p1 + h11 * h * m1;
    value.clamp(p1, 1.0)
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

    // ── TIRE-100/TIRE-200 acceptance tests ──────────────────────────────────────

    fn curve_profile() -> TireForceProfile {
        TireForceProfile {
            lateral_peak_slip_angle_rad: 0.10,
            longitudinal_peak_slip_ratio: 0.12,
            lateral_slide_mu_ratio: 0.82,
            longitudinal_slide_mu_ratio: 0.78,
            budget_blend_width: 0.0,
            falloff_sharpness: 2.0,
            slip_loss_tau_s: 0.0,
            slip_recovery_tau_s: 0.0,
            combined_lateral_peak_migration: 0.0,
            combined_longitudinal_peak_migration: 0.0,
        }
    }

    #[test]
    fn pure_curve_reaches_peak_then_decays_to_plateau() {
        let p = curve_profile();
        let gamma = 2.0;
        let mut prev = -1.0;
        for i in 0..=100 {
            let x = p.lateral_peak_slip_angle_rad * (i as f64 / 100.0);
            let r = pure_curve_coefficient(
                x,
                p.lateral_peak_slip_angle_rad,
                p.lateral_slide_mu_ratio,
                gamma,
                p.falloff_sharpness,
            );
            assert_eq!(r.regime, TireSlipRegime::PrePeak);
            assert!(r.force_coefficient + 1e-9 >= prev, "pre-peak must rise monotonically");
            prev = r.force_coefficient;
        }
        let at_peak = pure_curve_coefficient(
            p.lateral_peak_slip_angle_rad,
            p.lateral_peak_slip_angle_rad,
            p.lateral_slide_mu_ratio,
            gamma,
            p.falloff_sharpness,
        );
        assert!(
            (at_peak.force_coefficient - 1.0).abs() < 1e-9,
            "force coefficient must peak at the configured slip"
        );

        let mut prev = 2.0;
        for i in 1..=100 {
            let x = p.lateral_peak_slip_angle_rad * (1.0 + i as f64 * 0.05);
            let r = pure_curve_coefficient(
                x,
                p.lateral_peak_slip_angle_rad,
                p.lateral_slide_mu_ratio,
                gamma,
                p.falloff_sharpness,
            );
            assert_eq!(r.regime, TireSlipRegime::PostPeak);
            assert!(r.force_coefficient + 1e-9 <= prev, "post-peak must decay monotonically");
            prev = r.force_coefficient;
        }
        let deep = pure_curve_coefficient(
            p.lateral_peak_slip_angle_rad * 12.0,
            p.lateral_peak_slip_angle_rad,
            p.lateral_slide_mu_ratio,
            gamma,
            p.falloff_sharpness,
        );
        assert!(
            (deep.force_coefficient - p.lateral_slide_mu_ratio).abs() < 0.01,
            "deep slide must converge to the configured plateau: {}",
            deep.force_coefficient
        );
    }

    #[test]
    fn pure_curve_is_smooth_at_peak_boundary() {
        let p = curve_profile();
        let gamma = 2.0;
        let epsilon = 1e-6;
        let below = pure_curve_coefficient(
            p.lateral_peak_slip_angle_rad - epsilon,
            p.lateral_peak_slip_angle_rad,
            p.lateral_slide_mu_ratio,
            gamma,
            p.falloff_sharpness,
        );
        let above = pure_curve_coefficient(
            p.lateral_peak_slip_angle_rad + epsilon,
            p.lateral_peak_slip_angle_rad,
            p.lateral_slide_mu_ratio,
            gamma,
            p.falloff_sharpness,
        );
        assert!(
            (below.force_coefficient - above.force_coefficient).abs() < 1e-6,
            "curve must be continuous across the peak boundary"
        );
    }

    // ── GRIP-01 combined-budget knee tests ─────────────────────────────────────

    #[test]
    fn budget_scale_zero_width_is_exact_legacy_clip() {
        for u in [0.0, 0.5, 0.9, 0.99, 1.0, 1.01, 1.2, 1.4142135, 3.0] {
            let expected = if u > 1.0 { 1.0 / u } else { 1.0 };
            assert_eq!(
                combined_budget_scale(u, 0.0),
                expected,
                "w=0 must reproduce the hard clip at u={u}"
            );
        }
    }

    #[test]
    fn budget_scale_is_c1_at_band_edges() {
        let w = 0.10;
        let lo = 1.0 - w;
        let hi = 1.0 + w;
        for edge in [lo, hi] {
            let eps = 1e-7;
            let left = combined_budget_scale(edge - eps, w);
            let right = combined_budget_scale(edge + eps, w);
            assert!(
                (left - right).abs() < 1e-5,
                "value must be continuous at {edge}: {left} vs {right}"
            );
            let eps = 1e-5;
            let slope_left =
                (combined_budget_scale(edge, w) - combined_budget_scale(edge - eps, w)) / eps;
            let slope_right =
                (combined_budget_scale(edge + eps, w) - combined_budget_scale(edge, w)) / eps;
            assert!(
                (slope_left - slope_right).abs() < 1e-3,
                "slope must be continuous at {edge}: {slope_left} vs {slope_right}"
            );
        }
        // The legacy clip jumps from slope 0 to slope -1 at u = 1 while the
        // smoothed profile keeps the whole band C1.
        let eps = 1e-5;
        let legacy_jump = (combined_budget_scale(1.0 + eps, 0.0) - combined_budget_scale(1.0, 0.0))
            / eps
            - (combined_budget_scale(1.0, 0.0) - combined_budget_scale(1.0 - eps, 0.0)) / eps;
        let soft_jump = (combined_budget_scale(1.0 + eps, w) - combined_budget_scale(1.0, w)) / eps
            - (combined_budget_scale(1.0, w) - combined_budget_scale(1.0 - eps, w)) / eps;
        assert!(legacy_jump.abs() > 0.9, "legacy clip must show the knee");
        assert!(soft_jump.abs() < 1e-3, "smoothed knee must be C1");
    }

    #[test]
    fn budget_scale_is_monotone_and_never_exceeds_the_budget() {
        for &w in &[0.02, 0.05, 0.10, 0.15, 0.5] {
            let mut prev = 1.0;
            for i in 0..=3000 {
                let u = 3.0 * (i as f64) / 3000.0;
                let s = combined_budget_scale(u, w);
                assert!(
                    s <= 1.0 + 1e-12,
                    "budget scale must stay at or below one (u={u}, w={w}, s={s})"
                );
                assert!(
                    u * s <= 1.0 + 1e-12,
                    "delivered demand must respect the budget (u={u}, w={w}, s={s}, u*s={})",
                    u * s
                );
                if u <= 1.0 - w {
                    assert_eq!(s, 1.0, "below the band the legacy scale must be exact (u={u}, w={w})");
                }
                if u >= 1.0 + w {
                    assert!(
                        (s - 1.0 / u).abs() < 1e-12,
                        "above the band the legacy scale must be exact (u={u}, w={w}, s={s})"
                    );
                }
                assert!(
                    s <= prev + 1e-12,
                    "budget scale must be non-increasing (u={u}, w={w}, s={s}, prev={prev})"
                );
                prev = s;
            }
            assert_eq!(combined_budget_scale(2.0, w), 0.5);
            // The hard clip has a slope jump at u = 1; the smoothed version eases
            // the total force below the cap inside the band.
            let at_knee = combined_budget_scale(1.0, w);
            assert!(at_knee < 1.0, "smoothing must ease the cap at the knee: {at_knee}");
            assert!(at_knee >= 1.0 / (1.0 + w) - 1e-12);
        }
    }

    #[test]
    fn profile_budget_blend_width_only_affects_the_knee_band() {
        let profile = curve_profile();
        let run = |blend_width: f64, alpha: f64, kappa: f64| -> (f64, f64, f64) {
            let mut cfg = VehicleConfig::f1_94_canonical();
            cfg.front_tire_force = TireForceProfile {
                budget_blend_width: blend_width,
                ..profile
            };
            cfg.rear_tire_force = cfg.front_tire_force;
            let wheel = WheelIndex::FrontLeft;
            let fz = static_wheel_load(&cfg, wheel);
            let dt = 1.0 / 120.0;
            let radius = cfg.front_tire_radius.max(0.05);
            let mut tires = TireSystem::new(&cfg);
            tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
            tires.wheels[wheel as usize].spin = 20.0 * (1.0 + kappa) / radius;
            let vel = Vec3::new(-(alpha.tan()) * 20.0, 0.0, -20.0);
            for _ in 0..60 {
                tires.process_wheel_forces(
                    &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
                );
            }
            let s = &tires.wheels[wheel as usize];
            (s.longitudinal_force, s.lateral_force, s.combined_utilization)
        };

        // Low pure-slip demand stays well below the blend band: the soft budget
        // cannot alter it.
        let pure_lat_hard = run(0.0, 0.02, 0.0);
        let pure_lat_soft = run(0.10, 0.02, 0.0);
        assert!((pure_lat_hard.0 - pure_lat_soft.0).abs() < 1e-9);
        assert!((pure_lat_hard.1 - pure_lat_soft.1).abs() < 1e-9);
        let pure_lon_hard = run(0.0, 0.0, 0.02);
        let pure_lon_soft = run(0.10, 0.0, 0.02);
        assert!((pure_lon_hard.0 - pure_lon_soft.0).abs() < 1e-9);
        assert!((pure_lon_hard.1 - pure_lon_soft.1).abs() < 1e-9);

        // At least one combined operating point whose normalized demand lands
        // inside the band must change, but only slightly.
        let mut saw_in_band_change = false;
        for (alpha, kappa) in [(0.08, 0.0), (0.09, 0.02), (0.10, 0.0), (0.12, 0.0)] {
            let hard = run(0.0, alpha, kappa);
            let soft = run(0.10, alpha, kappa);
            let delta = (hard.1 - soft.1).abs();
            assert!(
                delta < hard.1.abs() * 0.05,
                "in-band change must stay small (alpha={alpha}, kappa={kappa}, delta={delta})"
            );
            if delta > 1e-4 {
                saw_in_band_change = true;
            }
        }
        assert!(saw_in_band_change, "no scanned point landed inside the blend band");

        // Deep combined slip is far beyond the band: parity with the legacy clip.
        let deep_hard = run(0.0, 0.35, 0.5);
        let deep_soft = run(0.10, 0.35, 0.5);
        assert!((deep_hard.0 - deep_soft.0).abs() < 1e-9);
        assert!((deep_hard.1 - deep_soft.1).abs() < 1e-9);
        assert!(deep_soft.2 <= 1.0 + 1e-9);
    }

    // ── GRIP-02 sliding-memory tests ───────────────────────────────────────────

    fn grip02_profile(loss_tau: f64, recovery_tau: f64, slide: f64) -> TireForceProfile {
        TireForceProfile {
            lateral_peak_slip_angle_rad: 0.10,
            longitudinal_peak_slip_ratio: 0.12,
            lateral_slide_mu_ratio: slide,
            longitudinal_slide_mu_ratio: slide,
            budget_blend_width: 0.0,
            falloff_sharpness: 2.0,
            slip_loss_tau_s: loss_tau,
            slip_recovery_tau_s: recovery_tau,
            combined_lateral_peak_migration: 0.0,
            combined_longitudinal_peak_migration: 0.0,
        }
    }

    /// Runs `phase_a_ticks` of deep lateral slip (0.30 rad) followed by
    /// `phase_b_ticks` of zero slip at a constant 20 m/s forward speed and
    /// returns `(lateral_force, post_peak_decay_lat)` per tick.
    fn run_slip_sequence(
        profile: TireForceProfile,
        phase_a_ticks: usize,
        phase_b_ticks: usize,
    ) -> Vec<(f64, f64)> {
        run_slip_sequence_dt(profile, phase_a_ticks, phase_b_ticks, 1.0 / 120.0)
    }

    fn run_slip_sequence_dt(
        profile: TireForceProfile,
        phase_a_ticks: usize,
        phase_b_ticks: usize,
        dt: f64,
    ) -> Vec<(f64, f64)> {
        let mut cfg = VehicleConfig::f1_94_canonical();
        cfg.front_tire_force = profile;
        cfg.rear_tire_force = profile;
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let radius = cfg.front_tire_radius.max(0.05);
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
        tires.wheels[wheel as usize].spin = 20.0 / radius;
        let alpha = 0.30_f64;
        let deep = Vec3::new(-(alpha.tan()) * 20.0, 0.0, -20.0);
        let grip = Vec3::new(0.0, 0.0, -20.0);
        let mut out = Vec::with_capacity(phase_a_ticks + phase_b_ticks);
        for i in 0..(phase_a_ticks + phase_b_ticks) {
            let vel = if i < phase_a_ticks { deep } else { grip };
            tires.process_wheel_forces(
                &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
            );
            let s = &tires.wheels[wheel as usize];
            out.push((s.lateral_force, s.post_peak_decay_lat));
        }
        out
    }

    #[test]
    fn sliding_memory_delays_loss_and_slows_recovery() {
        let legacy = run_slip_sequence(grip02_profile(0.0, 0.0, 0.5), 60, 60);
        let hysteretic = run_slip_sequence(grip02_profile(0.06, 0.30, 0.5), 60, 60);

        // Loss: right after entering deep slip the memory still holds force high.
        let entry = 6;
        assert!(
            hysteretic[entry].0.abs() > legacy[entry].0.abs() * 1.02,
            "loss must be delayed: hyst={} legacy={}",
            hysteretic[entry].0,
            legacy[entry].0
        );
        // Both converge to the same deep-slide plateau by the end of phase A.
        let end_a = 59;
        assert!(
            (hysteretic[end_a].0 - legacy[end_a].0).abs() < legacy[end_a].0.abs() * 0.02,
            "steady deep slide must match the pure envelope: hyst={} legacy={}",
            hysteretic[end_a].0,
            legacy[end_a].0
        );

        // Recovery: force returns later than the instantaneous envelope.
        let mid_recovery = 60 + 10;
        assert!(
            hysteretic[mid_recovery].0.abs() < legacy[mid_recovery].0.abs() * 0.98,
            "recovery must lag: hyst={} legacy={}",
            hysteretic[mid_recovery].0,
            legacy[mid_recovery].0
        );
        assert!(hysteretic[end_a].1 > 0.5, "memory must be committed by deep slip");
        assert!(
            hysteretic[mid_recovery].1 > 0.05,
            "memory must still be releasing shortly after grip returns"
        );
        assert!(
            hysteretic[119].1 < hysteretic[mid_recovery].1,
            "memory must keep releasing while gripping"
        );

        // No force discontinuity between consecutive ticks.
        let peak = legacy[0].0.abs().max(1.0);
        for pair in hysteretic.windows(2) {
            assert!(
                (pair[1].0 - pair[0].0).abs() < peak * 0.25,
                "force must not jump between ticks: {} -> {}",
                pair[0].0,
                pair[1].0
            );
        }
    }

    #[test]
    fn sliding_memory_recovery_is_slower_than_loss() {
        let profile = grip02_profile(0.06, 0.30, 0.5);
        let ticks = 300;
        let seq = run_slip_sequence(profile, ticks, ticks);
        let target = pure_curve_coefficient(0.30, 0.10, 0.5, 2.0, 2.0).decay_weight;

        let loss_t90 = (0..ticks)
            .find(|&i| seq[i].1 >= 0.9 * target)
            .expect("memory must commit during deep slip");
        let committed = seq[ticks - 1].1;
        let recovery_t90 = (ticks..2 * ticks)
            .find(|&i| seq[i].1 <= 0.1 * committed)
            .expect("memory must release after the slide") - ticks;

        assert!(
            recovery_t90 > loss_t90 * 3,
            "recovery must be clearly slower than loss: loss={loss_t90} ticks recovery={recovery_t90} ticks"
        );
    }

    #[test]
    fn sliding_memory_preserves_steady_state_envelope() {
        let legacy = run_slip_sequence(grip02_profile(0.0, 0.0, 0.6), 240, 0);
        let hysteretic = run_slip_sequence(grip02_profile(0.05, 0.25, 0.6), 240, 0);
        let last = 239;
        assert!(
            (legacy[last].0 - hysteretic[last].0).abs() < legacy[last].0.abs() * 0.005,
            "steady state must match the pure envelope: hyst={} legacy={}",
            hysteretic[last].0,
            legacy[last].0
        );
        let pure = pure_curve_coefficient(0.30, 0.10, 0.6, 2.0, 2.0);
        assert!(
            (hysteretic[last].1 - pure.decay_weight).abs() < 0.02,
            "memory must converge to the instantaneous decay weight: state={} target={}",
            hysteretic[last].1,
            pure.decay_weight
        );
    }

    #[test]
    fn sliding_memory_cannot_create_force_at_zero_slip() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        let profile = grip02_profile(0.06, 0.30, 0.6);
        cfg.front_tire_force = profile;
        cfg.rear_tire_force = profile;
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let radius = cfg.front_tire_radius.max(0.05);
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
        tires.wheels[wheel as usize].spin = 20.0 / radius;
        // Fully committed memory with zero instantaneous lateral demand.
        tires.wheels[wheel as usize].post_peak_decay_lat = 1.0;
        tires.wheels[wheel as usize].post_peak_decay_lon = 1.0;
        let vel = Vec3::new(0.0, 0.0, -20.0);
        for _ in 0..10 {
            tires.process_wheel_forces(
                &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, 1.0 / 120.0,
            );
        }
        assert!(
            tires.wheels[wheel as usize].lateral_force.abs() < 1e-9,
            "sliding memory must not create force without slip: {}",
            tires.wheels[wheel as usize].lateral_force
        );
    }

    fn run_open_loop_force(
        profile: TireForceProfile,
        alpha: f64,
        kappa: f64,
        ticks: usize,
    ) -> (f64, f64, f64, f64) {
        let mut cfg = VehicleConfig::f1_94_canonical();
        cfg.front_tire_force = profile;
        cfg.rear_tire_force = profile;
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let dt = 1.0 / 120.0;
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
        let radius = tires.wheels[wheel as usize]
            .effective_rolling_radius
            .max(0.05);
        // Exact target slip: the solver normalizes by max(|v|, |wheel speed|).
        let wheel_speed = if kappa >= 0.0 {
            20.0 / (1.0 - kappa)
        } else {
            20.0 * (1.0 + kappa)
        };
        tires.wheels[wheel as usize].spin = wheel_speed / radius;
        let vel = Vec3::new(-(alpha.tan()) * 20.0, 0.0, -20.0);
        for _ in 0..ticks {
            tires.process_wheel_forces(
                &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
            );
        }
        let s = &tires.wheels[wheel as usize];
        (
            s.longitudinal_force,
            s.lateral_force,
            s.effective_lateral_peak_slip_rad,
            s.effective_longitudinal_peak_slip_ratio,
        )
    }

    #[test]
    fn sliding_memory_is_axis_independent() {
        // Lateral deep slide with a small longitudinal demand that keeps the
        // combined demand below the budget knee: the explicit combined projection
        // cannot scale either axis, so the longitudinal delivery must be exactly
        // the same as with no lateral slip. The sliding memory must not leak.
        for (loss, recovery) in [(0.0, 0.0), (0.06, 0.30)] {
            let profile = grip02_profile(loss, recovery, 0.8);
            let (fx_asym, fy_asym, _, _) = run_open_loop_force(profile, 0.30, 0.01, 120);
            let (fx_sym, fy_sym, _, _) = run_open_loop_force(profile, 0.0, 0.01, 120);
            assert!(
                (fx_asym - fx_sym).abs() <= fx_sym.abs().max(1.0) * 1e-9,
                "lateral slide must not alter longitudinal force (taus=({loss},{recovery})): {fx_asym} vs {fx_sym}"
            );
            assert!(fy_sym.abs() < 1e-9, "zero lateral slip must keep lateral force at zero");
            assert!(fy_asym.abs() > 0.0);
        }

        // Converse: a deep longitudinal slide must not alter a small lateral
        // demand, also kept below the budget knee.
        for (loss, recovery) in [(0.0, 0.0), (0.06, 0.30)] {
            let profile = grip02_profile(loss, recovery, 0.8);
            let (_, fy_pre, _, _) = run_open_loop_force(profile, 0.01, 0.2, 120);
            let (_, fy_ref, _, _) = run_open_loop_force(profile, 0.01, 0.0, 120);
            assert!(
                (fy_pre - fy_ref).abs() <= fy_ref.abs().max(1.0) * 1e-9,
                "longitudinal slide must not alter lateral force (taus=({loss},{recovery})): {fy_pre} vs {fy_ref}"
            );
        }
    }

    #[test]
    fn sliding_memory_timing_is_frequency_consistent() {
        let profile = grip02_profile(0.06, 0.30, 0.5);
        let ticks = 300;
        let seq120 = run_slip_sequence_dt(profile, ticks, ticks, 1.0 / 120.0);
        let seq240 = run_slip_sequence_dt(profile, ticks, ticks, 1.0 / 240.0);
        let target = pure_curve_coefficient(0.30, 0.10, 0.5, 2.0, 2.0).decay_weight;

        let loss120 = (0..ticks)
            .find(|&i| seq120[i].1 >= 0.9 * target)
            .expect("120 Hz loss t90") as f64
            / 120.0;
        let loss240 = (0..ticks)
            .find(|&i| seq240[i].1 >= 0.9 * target)
            .expect("240 Hz loss t90") as f64
            / 240.0;
        let committed120 = seq120[ticks - 1].1;
        let committed240 = seq240[ticks - 1].1;
        let rec120 = ((ticks..2 * ticks)
            .find(|&i| seq120[i].1 <= 0.1 * committed120)
            .expect("120 Hz recovery t90")
            - ticks) as f64
            / 120.0;
        let rec240 = ((ticks..2 * ticks)
            .find(|&i| seq240[i].1 <= 0.1 * committed240)
            .expect("240 Hz recovery t90")
            - ticks) as f64
            / 240.0;

        assert!(
            (loss120 - loss240).abs() < 0.02,
            "loss t90 must be dt-independent: {loss120} vs {loss240}"
        );
        assert!(
            (rec120 - rec240).abs() < 0.03,
            "recovery t90 must be dt-independent: {rec120} vs {rec240}"
        );
    }

    // ── GRIP-04 combined-slip peak migration tests ─────────────────────────────

    #[test]
    fn combined_peak_migration_shifts_only_the_loaded_axis() {
        let base = curve_profile();
        let migrated = TireForceProfile {
            combined_lateral_peak_migration: 0.5,
            combined_longitudinal_peak_migration: 0.5,
            ..base
        };

        // Pure lateral demand carries no longitudinal demand: peak untouched.
        let pure_lat_hard = run_open_loop_force(base, 0.10, 0.0, 120);
        let pure_lat_soft = run_open_loop_force(migrated, 0.10, 0.0, 120);
        assert_eq!(pure_lat_hard.0, pure_lat_soft.0);
        assert_eq!(pure_lat_hard.1, pure_lat_soft.1);
        assert!(
            (pure_lat_soft.2 - base.lateral_peak_slip_angle_rad).abs() < 1e-15,
            "pure lateral demand must not migrate its own peak"
        );

        // Pure longitudinal demand behaves the same way on its own axis.
        let pure_lon_hard = run_open_loop_force(base, 0.0, 0.12, 120);
        let pure_lon_soft = run_open_loop_force(migrated, 0.0, 0.12, 120);
        assert_eq!(pure_lon_hard.0, pure_lon_soft.0);
        assert_eq!(pure_lon_soft.3, base.longitudinal_peak_slip_ratio);

        // Combined demand shrinks the opposite peak on both axes.
        let comb_hard = run_open_loop_force(base, 0.10, 0.12, 120);
        let comb_soft = run_open_loop_force(migrated, 0.10, 0.12, 120);
        assert!(
            comb_soft.2 < base.lateral_peak_slip_angle_rad * 0.75,
            "lateral peak must migrate under longitudinal demand: {}",
            comb_soft.2
        );
        assert!(
            comb_soft.3 < base.longitudinal_peak_slip_ratio * 0.75,
            "longitudinal peak must migrate under lateral demand: {}",
            comb_soft.3
        );
        assert!(
            comb_soft.1.abs() < comb_hard.1.abs() * 0.98,
            "migrated lateral force must be lower: {} vs {}",
            comb_soft.1,
            comb_hard.1
        );
        assert!(
            comb_soft.0.abs() < comb_hard.0.abs() * 0.98,
            "migrated longitudinal force must be lower: {} vs {}",
            comb_soft.0,
            comb_hard.0
        );
    }

    #[test]
    fn combined_peak_migration_is_continuous_across_kappa() {
        let profile = TireForceProfile {
            combined_lateral_peak_migration: 0.5,
            combined_longitudinal_peak_migration: 0.0,
            ..curve_profile()
        };
        let mut prev_peak = profile.lateral_peak_slip_angle_rad;
        for i in 0..=30 {
            let kappa = i as f64 * 0.01;
            let (_, _, lat_peak, _) = run_open_loop_force(profile, 0.10, kappa, 120);
            assert!(
                lat_peak <= prev_peak + 1e-12,
                "migrated lateral peak must shrink monotonically (kappa={kappa}): {lat_peak} vs {prev_peak}"
            );
            assert!(
                (lat_peak - prev_peak).abs() < 0.02,
                "migrated lateral peak must change continuously (kappa={kappa}): {lat_peak} vs {prev_peak}"
            );
            prev_peak = lat_peak;
        }
        assert!(
            prev_peak < profile.lateral_peak_slip_angle_rad * 0.8,
            "full longitudinal demand must shrink the lateral peak: {prev_peak}"
        );
    }

    #[test]
    fn falloff_sharpness_shapes_post_peak_without_touching_pre_peak() {
        for x in [0.0, 0.05, 0.10] {
            let soft = pure_curve_coefficient(x, 0.10, 0.8, 2.0, 1.0);
            let hard = pure_curve_coefficient(x, 0.10, 0.8, 2.0, 4.0);
            assert!((soft.force_coefficient - hard.force_coefficient).abs() < 1e-12);
        }
        let soft = pure_curve_coefficient(0.15, 0.10, 0.8, 2.0, 1.0);
        let hard = pure_curve_coefficient(0.15, 0.10, 0.8, 2.0, 4.0);
        assert!(soft.force_coefficient > hard.force_coefficient);
        assert!(soft.decay_weight < hard.decay_weight);
        assert!(
            (hard.force_coefficient - (1.0 - hard.decay_weight * (1.0 - 0.8))).abs() < 1e-12,
            "force must equal 1 - decay_weight * (1 - slide)"
        );
    }

    // ── GRIP-03 regularized wheel lock tests ───────────────────────────────────

    fn run_wheel_brake(
        cfg: &VehicleConfig,
        blend: f64,
        stick_tau: f64,
        initial_spin: f64,
        brake_torque_nm: f64,
        drive_torque_nm: f64,
        ticks: usize,
        dt: f64,
    ) -> Vec<(f64, f64)> {
        let mut cfg = cfg.clone();
        cfg.wheel_lock_blend_spin_rad_s = blend;
        cfg.wheel_lock_stick_tau_s = stick_tau;
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
        tires.wheels[wheel as usize].spin = initial_spin;
        let vel = Vec3::new(0.0, 0.0, -20.0);
        let mut out = Vec::with_capacity(ticks);
        for _ in 0..ticks {
            tires.process_wheel_torque(&cfg, wheel, drive_torque_nm, 0.0, brake_torque_nm, dt);
            tires.process_wheel_forces(
                &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
            );
            let s = &tires.wheels[wheel as usize];
            out.push((s.spin, s.longitudinal_force));
        }
        out
    }

    #[test]
    fn regularized_wheel_lock_removes_the_zero_snap() {
        let cfg = VehicleConfig::f1_94_canonical();
        let legacy = run_wheel_brake(&cfg, 0.0, 0.0, 60.0, 840.0, 0.0, 240, 1.0 / 120.0);
        let reg = run_wheel_brake(&cfg, 8.0, 0.02, 60.0, 840.0, 0.0, 240, 1.0 / 120.0);

        assert!(legacy[239].0.abs() < 0.5, "legacy must stop: {}", legacy[239].0);
        assert!(reg[239].0.abs() < 0.5, "regularized must stop: {}", reg[239].0);

        let max_step_near_zero = |seq: &[(f64, f64)]| {
            let mut m = 0.0_f64;
            for pair in seq.windows(2) {
                if pair[0].0.abs() < 8.0 {
                    m = m.max((pair[1].0 - pair[0].0).abs());
                }
            }
            m
        };
        let legacy_step = max_step_near_zero(&legacy);
        let reg_step = max_step_near_zero(&reg);
        assert!(
            reg_step < legacy_step,
            "regularized lock must be smoother near zero: legacy={legacy_step} reg={reg_step}"
        );
        assert!(
            reg_step < 2.0,
            "regularized lock must approach zero progressively: {reg_step}"
        );
        let legacy_snaps = legacy
            .windows(2)
            .any(|p| p[0].0.abs() > 0.5 && p[1].0 == 0.0);
        let reg_snaps = reg.windows(2).any(|p| p[0].0.abs() > 0.5 && p[1].0 == 0.0);
        assert!(legacy_snaps, "legacy must teleport to exactly zero");
        assert!(!reg_snaps, "regularized lock must not teleport to zero");
        assert!(
            reg.iter().all(|r| r.0 >= -1e-9),
            "regularized spin must never reverse sign"
        );
        assert!(reg.iter().all(|r| r.0.is_finite() && r.1.is_finite()));
    }

    #[test]
    fn regularized_wheel_lock_parks_and_holds() {
        let cfg = VehicleConfig::f1_94_canonical();
        let seq = run_wheel_brake(&cfg, 8.0, 0.02, 2.0, 840.0, 0.0, 60, 1.0 / 120.0);
        assert!(
            seq[59].0.abs() < 1e-3,
            "wheel must park under brake: {}",
            seq[59].0
        );
        let hold = run_wheel_brake(&cfg, 8.0, 0.02, 0.0, 840.0, 0.0, 120, 1.0 / 120.0);
        assert!(
            hold.iter().all(|r| r.0.abs() < 1e-9),
            "parked wheel must not creep under brake"
        );
    }

    #[test]
    fn regularized_wheel_lock_releases_without_snap() {
        let run_release = |blend: f64, stick: f64| -> Vec<f64> {
            let mut cfg = VehicleConfig::f1_94_canonical();
            cfg.wheel_lock_blend_spin_rad_s = blend;
            cfg.wheel_lock_stick_tau_s = stick;
            let wheel = WheelIndex::FrontLeft;
            let fz = static_wheel_load(&cfg, wheel);
            let mut tires = TireSystem::new(&cfg);
            tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
            let dt = 1.0 / 120.0;
            let vel = Vec3::new(0.0, 0.0, -20.0);
            let mut spins = Vec::new();
            for i in 0..120 {
                let drive = if i < 60 { 0.0 } else { 1100.0 };
                tires.process_wheel_torque(&cfg, wheel, drive, 0.0, 840.0, dt);
                tires.process_wheel_forces(
                    &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
                );
                spins.push(tires.wheels[wheel as usize].spin);
            }
            spins
        };
        let legacy = run_release(0.0, 0.0);
        let spins = run_release(8.0, 0.02);

        assert!(
            spins[..60].iter().all(|s| s.abs() < 1e-9),
            "brake must hold the wheel at rest"
        );
        assert!(spins[60] > 0.0, "drive above brake must break away");
        assert!(
            spins[60] <= legacy[60] + 1e-9,
            "the band must not accelerate faster than the legacy step: reg={} legacy={}",
            spins[60],
            legacy[60]
        );
        assert!(spins[119] > spins[60], "wheel must keep spinning up");
        assert!(spins.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn effective_force_respects_budget_with_smoothing_enabled() {
        let mut cfg = VehicleConfig::f1_94_canonical();
        let profile = TireForceProfile {
            budget_blend_width: 0.1,
            ..curve_profile()
        };
        cfg.front_tire_force = profile;
        cfg.rear_tire_force = profile;

        let wheel = WheelIndex::FrontLeft;
        let reference = static_wheel_load(&cfg, wheel);
        let alpha_max = profile.lateral_peak_slip_angle_rad * 2.0;
        let kappa_max = profile.longitudinal_peak_slip_ratio * 2.0;
        let long_ratio = cfg
            .surface_longitudinal_grip_ratio
            .get(&SurfaceType::Road)
            .copied()
            .unwrap_or(0.5);
        let dt = 1.0 / 120.0;

        let mut saw_smoothing = false;
        for fz_scale in [0.5, 1.0, 1.5] {
            let fz = reference * fz_scale;
            for alpha_steps in 0..=20 {
                let alpha = alpha_max * (alpha_steps as f64 / 20.0);
                for kappa_steps in 0..=20 {
                    let kappa = kappa_max * (kappa_steps as f64 / 20.0);
                    let mut tires = TireSystem::new(&cfg);
                    tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
                    let radius = tires.wheels[wheel as usize].effective_rolling_radius.max(0.05);
                    tires.wheels[wheel as usize].spin = (20.0 * (1.0 + kappa)) / radius;
                    let vel = Vec3::new(-(alpha.tan()) * 20.0, 0.0, -20.0);
                    for _ in 0..60 {
                        tires.process_wheel_forces(
                            &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
                        );
                    }
                    let s = &tires.wheels[wheel as usize];
                    // Effective utilization reported by the solver must match the
                    // traction norm computed from the applied forces (rolling
                    // resistance excluded), and stay inside the shared budget.
                    let mu = 2.9
                        * s.load_sensitivity_scale
                        * s.mechanical_modifiers.grip_scale.clamp(0.05, 2.0);
                    let fx_max = (mu * long_ratio * fz).max(1e-6);
                    let fy_max = (mu * fz).max(1e-6);
                    let fx_traction = s.longitudinal_force + s.rolling_resistance;
                    let effective =
                        ((fx_traction / fx_max).powi(2) + (s.lateral_force / fy_max).powi(2)).sqrt();
                    assert!(
                        effective <= 1.0 + 1e-6,
                        "applied force must respect the budget (fz={fz_scale} alpha={alpha} kappa={kappa}: effective={effective})"
                    );
                    assert!(
                        (effective - s.combined_utilization).abs() < 1e-6,
                        "telemetry utilization must match the applied traction norm: {effective} vs {}",
                        s.combined_utilization
                    );
                    assert!(
                        s.combined_demand + 1e-12 >= s.combined_utilization,
                        "demand must never be below the delivered utilization"
                    );
                    if s.combined_demand - s.combined_utilization > 1e-4 {
                        saw_smoothing = true;
                    }
                    assert!(s.longitudinal_force.is_finite() && s.lateral_force.is_finite());
                    assert!(s.aligning_torque.is_finite());
                }
            }
        }
        assert!(
            saw_smoothing,
            "the sweep must include demand absorbed by the smoothing band"
        );
    }

    #[test]
    fn combined_slip_forces_are_odd_symmetric() {
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let dt = 1.0 / 120.0;
        let radius = cfg.front_tire_radius.max(0.05);

        let run_ws = |lat_speed: f64, wheel_speed: f64| -> (f64, f64, f64) {
            let mut tires = TireSystem::new(&cfg);
            tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
            tires.wheels[wheel as usize].spin = wheel_speed / radius;
            let vel = Vec3::new(lat_speed, 0.0, -20.0);
            for _ in 0..60 {
                tires.process_wheel_forces(
                    &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
                );
            }
            let s = &tires.wheels[wheel as usize];
            (s.longitudinal_force, s.lateral_force, s.rolling_resistance)
        };

        let (fx_neg_lat, fy_neg_lat, _) = run_ws(-3.0, 22.0);
        let (fx_pos_lat, fy_pos_lat, _) = run_ws(3.0, 22.0);
        assert!((fx_neg_lat - fx_pos_lat).abs() < 1e-6, "Fx must be left/right invariant");
        assert!((fy_neg_lat + fy_pos_lat).abs() < 1e-6, "Fy must flip sign with lateral slip");

        // Symmetric kappa ±0.3 relative to v_forward=20 under the solver's
        // denominator rule: kappa+ = w/(w-v) with w = v/(1-k), kappa- = (w-v)/v with w = v(1-k).
        // Tire force is odd in kappa, but rolling resistance always opposes forward
        // motion, so the odd-symmetry check accounts for exactly 2 * rolling resistance.
        let (fx_neg_k, _, rr_neg) = run_ws(0.0, 20.0 * 0.7);
        let (fx_pos_k, _, _) = run_ws(0.0, 20.0 / 0.7);
        assert!(
            ((fx_neg_k.abs() - fx_pos_k.abs()) - 2.0 * rr_neg).abs() < 1e-6,
            "Fx must flip sign with longitudinal slip modulo rolling resistance: neg={} pos={} rr={rr_neg}",
            fx_neg_k,
            fx_pos_k
        );
    }

    #[test]
    fn trail_braking_consumes_lateral_capacity() {
        // Same Fz and slip angle: adding brake slip must reduce |Fy|.
        let mut cfg = VehicleConfig::f1_94_canonical();
        let profile = curve_profile();
        cfg.front_tire_force = profile;
        cfg.rear_tire_force = profile;
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let dt = 1.0 / 120.0;
        let radius = cfg.front_tire_radius.max(0.05);
        let alpha_peak = profile.lateral_peak_slip_angle_rad;

        let fy_at_alpha = move |kappa: f64| -> f64 {
            let mut tires = TireSystem::new(&cfg);
            tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
            tires.wheels[wheel as usize].spin = 20.0 * (1.0 + kappa) / radius;
            let vel = Vec3::new(-(alpha_peak.tan()) * 20.0, 0.0, -20.0);
            for _ in 0..60 {
                tires.process_wheel_forces(
                    &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
                );
            }
            tires.wheels[wheel as usize].lateral_force.abs()
        };

        let pure = fy_at_alpha(0.0);
        let braking = fy_at_alpha(0.30);
        assert!(
            braking < pure * 0.95,
            "braking slip must reduce lateral force (pure={pure}, braking={braking})"
        );
    }

    #[test]
    fn power_exit_consumes_longitudinal_capacity() {
        // Same Fz and slip ratio: adding lateral slip must reduce |Fx|.
        let mut cfg = VehicleConfig::f1_94_canonical();
        let profile = curve_profile();
        cfg.front_tire_force = profile;
        cfg.rear_tire_force = profile;
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let dt = 1.0 / 120.0;
        let radius = cfg.front_tire_radius.max(0.05);
        let kappa = 0.30;

        let fx_at_kappa = move |alpha: f64| -> f64 {
            let mut tires = TireSystem::new(&cfg);
            tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
            tires.wheels[wheel as usize].spin = 20.0 * (1.0 + kappa) / radius;
            let vel = Vec3::new(-(alpha.tan()) * 20.0, 0.0, -20.0);
            for _ in 0..60 {
                tires.process_wheel_forces(
                    &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
                );
            }
            tires.wheels[wheel as usize].longitudinal_force.abs()
        };

        let pure = fx_at_kappa(0.0);
        let cornering = fx_at_kappa(0.30);
        assert!(
            cornering < pure * 0.95,
            "lateral slip must reduce drive force (pure={pure}, cornering={cornering})"
        );
    }

    #[test]
    fn near_zero_speed_produces_no_nan_and_keeps_finite_forces() {
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let dt = 1.0 / 120.0;
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
        tires.wheels[wheel as usize].spin = 1.0;
        let vel = Vec3::new(0.0, 0.0, -0.05);
        for _ in 0..10 {
            tires.process_wheel_forces(
                &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false, vel, dt,
            );
        }
        let s = &tires.wheels[wheel as usize];
        assert!(s.longitudinal_force.is_finite(), "Fx must be finite near zero speed");
        assert!(s.lateral_force.is_finite(), "Fy must be finite near zero speed");
        assert!(s.aligning_torque.is_finite(), "aligning torque must be finite");
        assert!(s.combined_utilization.is_finite());
        assert!(s.combined_utilization >= 0.0);
    }

    #[test]
    fn limit_spin_and_regime_reflect_post_peak_state() {
        let cfg = VehicleConfig::f1_94_canonical();
        let wheel = WheelIndex::FrontLeft;
        let fz = static_wheel_load(&cfg, wheel);
        let dt = 1.0 / 120.0;
        let radius = cfg.front_tire_radius.max(0.05);
        let mut tires = TireSystem::new(&cfg);
        tires.set_mechanical_state(&cfg, wheel, fz, 0.0, 0.008, 1.0);
        tires.wheels[wheel as usize].spin = 20.0 * 1.5 / radius;
        for _ in 0..60 {
            tires.process_wheel_forces(
                &cfg, wheel, fz, SurfaceType::Road, 2.9, 8.75, 1.0, false,
                Vec3::new(0.0, 0.0, -20.0), dt,
            );
        }
        let s = &tires.wheels[wheel as usize];
        assert!(s.limit_spin, "deep longitudinal slip must flag limit_spin");
        assert_eq!(s.tire_regime, 2, "wheel must report longitudinal post-peak regime");
        assert!(s.combined_utilization <= 1.0 + 1e-9);
    }
}

