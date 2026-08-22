//! Mechanical wheel/tire tuning shared by suspension and tire-force solvers.
//!
//! This module deliberately sits between the old GEVP-style raycast vehicle and a
//! fully constrained multibody suspension.  The Godot-facing contract stays simple
//! (three transverse raycasts per wheel), while Rust models a virtual unsprung mass,
//! tire carcass compliance and transient contact-patch behavior.

use crate::types::WheelIndex;
use crate::vehicle_config::VehicleConfig;

pub const GRAVITY_M_S2: f64 = 9.80665;

#[derive(Debug, Clone, Copy)]
pub struct WheelMechanicalTuning {
    /// Virtual unsprung mass. The rigid body still represents total vehicle mass;
    /// this mass is therefore used as an inertial contact filter, not added to gravity.
    pub unsprung_mass_kg: f64,
    /// Radial carcass spring stiffness.
    pub tire_vertical_stiffness_n_m: f64,
    /// Radial carcass damping.
    pub tire_vertical_damping_n_s_m: f64,
    /// Maximum compliant carcass deflection before the hard carcass progressively engages.
    pub max_tire_deflection_m: f64,
    /// Portion of carcass deflection removed from the effective rolling radius.
    pub rolling_radius_deflection_factor: f64,
    /// Tire force scales approximately with Fz^exponent. Values below 1 model load sensitivity.
    pub load_sensitivity_exponent: f64,
    /// Longitudinal distance required to build tire force.
    pub relaxation_length_m: f64,
    /// Converts signed camber into an equivalent lateral-slip contribution.
    pub camber_thrust_gain: f64,
    /// Base pneumatic trail used for self-aligning torque.
    pub pneumatic_trail_m: f64,
    /// Minimum carcass stiffness retained when only a fraction of the tricast is touching.
    pub partial_contact_stiffness_floor: f64,
    /// Target mechanical integration step. Suspension internally substeps to this value.
    pub mechanical_substep_s: f64,
    /// Low-pass time constant for the tricast road-height target.
    pub road_height_filter_tau_s: f64,
    /// Maximum accepted road-height velocity from raycast geometry changes.
    pub max_road_velocity_m_s: f64,
    /// Bump stop begins before geometric suspension travel is exhausted.
    pub bump_stop_start_ratio: f64,
    /// Small virtual overtravel allowed so bottom-out penetration remains measurable.
    pub hard_stop_overtravel_m: f64,
    /// Rotational inertia coefficient I = k*m*r^2 for wheel+tire assembly.
    pub rotational_inertia_factor: f64,
    /// Growth of the effective longitudinal contact patch with vertical load.
    pub contact_patch_load_gain: f64,
    /// Growth of the effective longitudinal contact patch with carcass deflection.
    pub contact_patch_deflection_gain: f64,
}

impl WheelMechanicalTuning {
    pub fn for_wheel(config: &VehicleConfig, wheel: WheelIndex) -> Self {
        let wheel_mass = if wheel.is_front() {
            config.front_wheel_mass
        } else {
            config.rear_wheel_mass
        }
        .max(1.0);
        let radius = if wheel.is_front() {
            config.front_tire_radius
        } else {
            config.rear_tire_radius
        }
        .max(0.05);
        let contact_patch = if wheel.is_front() {
            config.front_contact_patch
        } else {
            config.rear_contact_patch
        }
        .max(0.05);

        // The existing wheel mass contains the rotating assembly only. A modest multiplier
        // represents hub/brake and a fraction of the suspension links without changing the
        // Godot rigid-body mass contract.
        let unsprung_mass_kg = wheel_mass * 1.35;

        // Tune radial stiffness from the existing static corner load. Roughly 8 mm of
        // static radial deflection keeps the carcass visibly/mechanically relevant while
        // remaining stable at 120 Hz with internal substepping.
        let reference_load = static_wheel_load(config, wheel).max(100.0);
        let static_radial_deflection = 0.008;
        let tire_vertical_stiffness_n_m = reference_load / static_radial_deflection;
        let radial_damping_ratio = 0.28;
        let tire_vertical_damping_n_s_m =
            2.0 * radial_damping_ratio * (tire_vertical_stiffness_n_m * unsprung_mass_kg).sqrt();

        Self {
            unsprung_mass_kg,
            tire_vertical_stiffness_n_m,
            tire_vertical_damping_n_s_m,
            max_tire_deflection_m: radius * 0.12,
            rolling_radius_deflection_factor: 0.68,
            load_sensitivity_exponent: 0.92,
            relaxation_length_m: contact_patch * if wheel.is_front() { 1.65 } else { 1.80 },
            camber_thrust_gain: if wheel.is_front() { 0.18 } else { 0.15 },
            pneumatic_trail_m: contact_patch * if wheel.is_front() { 0.22 } else { 0.24 },
            partial_contact_stiffness_floor: 0.35,
            mechanical_substep_s: 1.0 / 480.0,
            road_height_filter_tau_s: 0.004,
            max_road_velocity_m_s: 15.0,
            bump_stop_start_ratio: 0.82,
            hard_stop_overtravel_m: 0.030,
            rotational_inertia_factor: 0.72,
            contact_patch_load_gain: 0.12,
            contact_patch_deflection_gain: 0.20,
        }
    }
}

#[inline]
pub fn static_wheel_load(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    config.mass_over_wheel(wheel) * GRAVITY_M_S2
}

#[inline]
pub fn tire_radius(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_tire_radius
    } else {
        config.rear_tire_radius
    }
}

#[inline]
pub fn tire_width(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_tire_width
    } else {
        config.rear_tire_width
    }
}

#[inline]
pub fn base_contact_patch(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_contact_patch
    } else {
        config.rear_contact_patch
    }
}

#[inline]
pub fn wheel_mass(config: &VehicleConfig, wheel: WheelIndex) -> f64 {
    if wheel.is_front() {
        config.front_wheel_mass
    } else {
        config.rear_wheel_mass
    }
}
