//! Pressure + thermal tire subsystem for Formula-90.
//!
//! Intended integration:
//! - `simulation.rs` owns tick ordering.
//! - `suspension.rs` consumes `TireMechanicalModifiers` for vertical carcass behaviour.
//! - `tire.rs` consumes the same modifiers for contact-patch / force response.
//! - thermal state is updated AFTER tire forces, so pressure changes apply on the next tick.
//!
//! This module deliberately does not depend on `VehicleConfig`; the JSON parser can own
//! these config structs without creating a circular module dependency.

use crate::types::WheelIndex;
use serde::{Deserialize, Serialize};

const KELVIN_OFFSET: f64 = 273.15;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TirePressureConfig {
    /// Player/setup values. Gauge pressure, kPa.
    pub cold_kpa_gauge: [f64; 4],
    /// Mechanical reference pressure expected near normal hot running state.
    pub reference_hot_kpa_gauge: [f64; 4],
    /// Temperature at which cold pressure was specified.
    pub reference_temperature_c: f64,
    pub atmospheric_pressure_kpa: f64,
}

impl Default for TirePressureConfig {
    fn default() -> Self {
        Self {
            cold_kpa_gauge: [110.0, 110.0, 105.0, 105.0],
            reference_hot_kpa_gauge: [145.0, 145.0, 140.0, 140.0],
            reference_temperature_c: 25.0,
            atmospheric_pressure_kpa: 101.325,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TireThermalConfig {
    pub initial_temperature_c: f64,
    pub ambient_fallback_c: f64,
    pub track_fallback_c: f64,

    pub optimal_tread_temperature_c: f64,
    pub optimal_tread_window_c: f64,
    pub overheat_tread_temperature_c: f64,
    pub optimal_carcass_temperature_c: f64,

    /// Effective lumped heat capacities for one zone / carcass / enclosed gas.
    pub tread_zone_heat_capacity_j_k: f64,
    pub carcass_heat_capacity_j_k: f64,
    pub gas_heat_capacity_j_k: f64,

    /// Lumped conductances.
    pub tread_to_carcass_w_k: f64,
    pub carcass_to_gas_w_k: f64,
    pub tread_to_air_w_k: f64,
    pub carcass_to_air_w_k: f64,
    pub road_conductance_w_k: f64,
    pub lateral_tread_conductance_w_k: f64,

    /// Fraction of slip work deposited in tread and of vertical hysteresis work
    /// deposited in carcass.
    pub slip_heat_efficiency: f64,
    pub carcass_hysteresis_efficiency: f64,

    /// Mechanical pressure sensitivities around `reference_hot_kpa_gauge`.
    pub pressure_stiffness_exponent: f64,
    pub pressure_damping_exponent: f64,
    pub pressure_max_deflection_exponent: f64,
    pub pressure_patch_exponent: f64,
    pub pressure_relaxation_exponent: f64,
    pub pressure_rr_exponent: f64,
    pub pressure_force_stiffness_exponent: f64,
    pub pressure_trail_exponent: f64,

    /// Small ideal-gas volume correction from carcass compression.
    pub volume_deflection_gain: f64,

    /// Safety clamps for runtime pressure.
    pub minimum_pressure_kpa_gauge: f64,
    pub maximum_pressure_kpa_gauge: f64,
}

impl Default for TireThermalConfig {
    fn default() -> Self {
        Self {
            initial_temperature_c: 25.0,
            ambient_fallback_c: 25.0,
            track_fallback_c: 35.0,

            optimal_tread_temperature_c: 95.0,
            optimal_tread_window_c: 12.0,
            overheat_tread_temperature_c: 120.0,
            optimal_carcass_temperature_c: 80.0,

            tread_zone_heat_capacity_j_k: 4200.0,
            carcass_heat_capacity_j_k: 11000.0,
            gas_heat_capacity_j_k: 1600.0,

            tread_to_carcass_w_k: 45.0,
            carcass_to_gas_w_k: 20.0,
            tread_to_air_w_k: 25.0,
            carcass_to_air_w_k: 12.0,
            road_conductance_w_k: 55.0,
            lateral_tread_conductance_w_k: 18.0,

            slip_heat_efficiency: 0.88,
            carcass_hysteresis_efficiency: 0.75,

            pressure_stiffness_exponent: 0.72,
            pressure_damping_exponent: -0.22,
            pressure_max_deflection_exponent: -0.55,
            pressure_patch_exponent: -0.32,
            pressure_relaxation_exponent: -0.18,
            pressure_rr_exponent: -0.35,
            pressure_force_stiffness_exponent: 0.12,
            pressure_trail_exponent: -0.16,

            volume_deflection_gain: 0.04,

            minimum_pressure_kpa_gauge: 55.0,
            maximum_pressure_kpa_gauge: 260.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TireEnvironment {
    pub ambient_temperature_c: f64,
    pub track_temperature_c: f64,
}

impl TireEnvironment {
    pub fn fallback(cfg: &TireThermalConfig) -> Self {
        Self {
            ambient_temperature_c: cfg.ambient_fallback_c,
            track_temperature_c: cfg.track_fallback_c,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WheelThermalState {
    pub tread_inner_c: f64,
    pub tread_center_c: f64,
    pub tread_outer_c: f64,
    pub carcass_c: f64,
    pub gas_c: f64,
    pub pressure_kpa_gauge: f64,

    /// Last mechanical tire deflection ratio [0..~1.5]. Used only for the small
    /// enclosed-volume correction of pressure.
    pub deflection_ratio: f64,
}

impl WheelThermalState {
    fn new(wheel: WheelIndex, pressure: &TirePressureConfig, thermal: &TireThermalConfig) -> Self {
        let i = wheel as usize;
        Self {
            tread_inner_c: thermal.initial_temperature_c,
            tread_center_c: thermal.initial_temperature_c,
            tread_outer_c: thermal.initial_temperature_c,
            carcass_c: thermal.initial_temperature_c,
            gas_c: thermal.initial_temperature_c,
            pressure_kpa_gauge: pressure.cold_kpa_gauge[i],
            deflection_ratio: 0.0,
        }
    }

    pub fn average_tread_c(&self) -> f64 {
        (self.tread_inner_c + self.tread_center_c + self.tread_outer_c) / 3.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TireMechanicalModifiers {
    /// Multiplies radial tire spring stiffness used by suspension.
    pub vertical_stiffness_scale: f64,
    /// Multiplies radial tire damping.
    pub vertical_damping_scale: f64,
    /// Multiplies maximum compliant carcass deflection.
    pub max_deflection_scale: f64,

    /// Applied by tire.rs after the base mechanical contact-patch calculation.
    pub contact_patch_scale: f64,
    pub relaxation_length_scale: f64,
    pub rolling_resistance_scale: f64,
    pub force_stiffness_scale: f64,
    pub grip_scale: f64,
    pub pneumatic_trail_scale: f64,
}

impl TireMechanicalModifiers {
    /// Neutral modifiers (all scales 1.0). Compatibility paths and callers that
    /// never feed pressure/thermal data must use identity so no mechanical term is
    /// accidentally zeroed.
    pub fn identity() -> Self {
        Self {
            vertical_stiffness_scale: 1.0,
            vertical_damping_scale: 1.0,
            max_deflection_scale: 1.0,
            contact_patch_scale: 1.0,
            relaxation_length_scale: 1.0,
            rolling_resistance_scale: 1.0,
            force_stiffness_scale: 1.0,
            grip_scale: 1.0,
            pneumatic_trail_scale: 1.0,
        }
    }
}

impl Default for TireMechanicalModifiers {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TireThermalInput {
    pub normal_force_n: f64,
    pub longitudinal_force_n: f64,
    pub lateral_force_n: f64,

    /// Road-relative slip velocities at the contact patch.
    pub slip_velocity_long_ms: f64,
    pub slip_velocity_lat_ms: f64,

    pub tire_deflection_m: f64,
    pub tire_deflection_velocity_m_s: f64,
    pub max_tire_deflection_m: f64,
    pub dynamic_camber_rad: f64,
    pub vehicle_speed_ms: f64,

    /// Raw supported zones in Inner / Center / Outer order.
    /// Recommended input from tricast: [1 or 0, 2 or 0, 1 or 0].
    pub zone_contact_weights: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TireThermalSystem {
    pub wheels: [WheelThermalState; 4],
}

impl TireThermalSystem {
    pub fn new(pressure: &TirePressureConfig, thermal: &TireThermalConfig) -> Self {
        Self {
            wheels: [
                WheelThermalState::new(WheelIndex::FrontLeft, pressure, thermal),
                WheelThermalState::new(WheelIndex::FrontRight, pressure, thermal),
                WheelThermalState::new(WheelIndex::RearLeft, pressure, thermal),
                WheelThermalState::new(WheelIndex::RearRight, pressure, thermal),
            ],
        }
    }

    pub fn reset(&mut self, pressure: &TirePressureConfig, thermal: &TireThermalConfig) {
        *self = Self::new(pressure, thermal);
    }

    pub fn mechanical_modifiers(
        &self,
        wheel: WheelIndex,
        pressure: &TirePressureConfig,
        thermal: &TireThermalConfig,
    ) -> TireMechanicalModifiers {
        let i = wheel as usize;
        let st = &self.wheels[i];

        let p_ref = pressure.reference_hot_kpa_gauge[i].max(20.0);
        let p_ratio = (st.pressure_kpa_gauge / p_ref).clamp(0.45, 1.80);

        // Hot rubber generally becomes less structurally stiff. Keep this a modest
        // correction; inflation pressure remains the dominant radial-stiffness term.
        let carcass_delta = st.carcass_c - thermal.optimal_carcass_temperature_c;
        let carcass_structure = (1.0 - 0.0015 * carcass_delta).clamp(0.88, 1.12);

        let average_tread = st.average_tread_c();
        let thermal_grip = thermal_grip_scale(average_tread, thermal);

        // Pressure itself should have only a mild direct peak-grip penalty away from
        // nominal. Its main handling effects come through mechanics/transients.
        let pressure_grip = (-0.30 * p_ratio.ln().powi(2)).exp().clamp(0.88, 1.0);

        TireMechanicalModifiers {
            vertical_stiffness_scale: (p_ratio.powf(thermal.pressure_stiffness_exponent)
                * carcass_structure)
                .clamp(0.55, 1.65),

            vertical_damping_scale: (p_ratio.powf(thermal.pressure_damping_exponent)
                * (2.0 - carcass_structure))
                .clamp(0.65, 1.55),

            max_deflection_scale: p_ratio
                .powf(thermal.pressure_max_deflection_exponent)
                .clamp(0.65, 1.55),

            contact_patch_scale: p_ratio
                .powf(thermal.pressure_patch_exponent)
                .clamp(0.75, 1.35),

            relaxation_length_scale: p_ratio
                .powf(thermal.pressure_relaxation_exponent)
                .clamp(0.80, 1.30),

            rolling_resistance_scale: p_ratio
                .powf(thermal.pressure_rr_exponent)
                .clamp(0.75, 1.50),

            force_stiffness_scale: (p_ratio
                .powf(thermal.pressure_force_stiffness_exponent)
                * (0.92 + 0.08 * thermal_grip))
                .clamp(0.75, 1.25),

            grip_scale: (thermal_grip * pressure_grip).clamp(0.60, 1.03),

            pneumatic_trail_scale: p_ratio
                .powf(thermal.pressure_trail_exponent)
                .clamp(0.78, 1.30),
        }
    }

    /// Update one wheel AFTER combined-slip forces have been solved.
    pub fn step_after_forces(
        &mut self,
        wheel: WheelIndex,
        pressure: &TirePressureConfig,
        thermal: &TireThermalConfig,
        environment: TireEnvironment,
        input: TireThermalInput,
        dt: f64,
    ) {
        let dt = dt.clamp(1.0 / 2000.0, 0.05);
        let i = wheel as usize;
        let st = &mut self.wheels[i];

        let p_ref = pressure.reference_hot_kpa_gauge[i].max(20.0);
        let p_ratio = (st.pressure_kpa_gauge / p_ref).clamp(0.45, 1.80);

        let zone_weights = normalized_zone_weights(
            input.zone_contact_weights,
            p_ratio,
            input.dynamic_camber_rad,
        );
        let contact_total = zone_weights.iter().sum::<f64>().clamp(0.0, 1.0);

        // Slip work at the footprint. Both longitudinal and lateral work contribute.
        let slip_power_w = (
            input.longitudinal_force_n.abs() * input.slip_velocity_long_ms.abs()
                + input.lateral_force_n.abs() * input.slip_velocity_lat_ms.abs()
        ) * thermal.slip_heat_efficiency.clamp(0.0, 1.0);

        let airflow = (1.0 + input.vehicle_speed_ms.abs() * 0.020).clamp(1.0, 4.0);

        let old_t = [st.tread_inner_c, st.tread_center_c, st.tread_outer_c];
        let mut new_t = old_t;

        for z in 0..3 {
            let supported = zone_weights[z];

            let q_slip = slip_power_w * supported;
            let q_road = thermal.road_conductance_w_k
                * supported
                * (environment.track_temperature_c - old_t[z]);
            let q_air = thermal.tread_to_air_w_k
                * airflow
                * (environment.ambient_temperature_c - old_t[z]);
            let q_carcass = thermal.tread_to_carcass_w_k * (st.carcass_c - old_t[z]);

            let q_lateral = match z {
                0 => thermal.lateral_tread_conductance_w_k * (old_t[1] - old_t[0]),
                1 => thermal.lateral_tread_conductance_w_k
                    * ((old_t[0] - old_t[1]) + (old_t[2] - old_t[1])),
                _ => thermal.lateral_tread_conductance_w_k * (old_t[1] - old_t[2]),
            };

            let net_w = q_slip + q_road + q_air + q_carcass + q_lateral;
            new_t[z] = finite_temp(
                old_t[z]
                    + net_w / thermal.tread_zone_heat_capacity_j_k.max(100.0) * dt,
            );
        }

        st.tread_inner_c = new_t[0];
        st.tread_center_c = new_t[1];
        st.tread_outer_c = new_t[2];

        // Carcass receives tread conduction plus vertical hysteresis/flex work.
        let tread_to_carcass_w = thermal.tread_to_carcass_w_k
            * ((st.tread_inner_c - st.carcass_c)
                + (st.tread_center_c - st.carcass_c)
                + (st.tread_outer_c - st.carcass_c));

        let flex_power_w = input.normal_force_n.max(0.0)
            * input.tire_deflection_velocity_m_s.abs()
            * thermal.carcass_hysteresis_efficiency.clamp(0.0, 1.0);

        let carcass_to_gas_w = thermal.carcass_to_gas_w_k * (st.carcass_c - st.gas_c);
        let carcass_to_air_w = thermal.carcass_to_air_w_k
            * airflow.sqrt()
            * (st.carcass_c - environment.ambient_temperature_c);

        let carcass_net_w =
            tread_to_carcass_w + flex_power_w - carcass_to_gas_w - carcass_to_air_w;

        st.carcass_c = finite_temp(
            st.carcass_c
                + carcass_net_w / thermal.carcass_heat_capacity_j_k.max(500.0) * dt,
        );

        // Gas changes much more slowly and is heated primarily through carcass conduction.
        let gas_net_w = thermal.carcass_to_gas_w_k * (st.carcass_c - st.gas_c);
        st.gas_c = finite_temp(
            st.gas_c + gas_net_w / thermal.gas_heat_capacity_j_k.max(100.0) * dt,
        );

        st.deflection_ratio = if input.max_tire_deflection_m > 1e-5 {
            (input.tire_deflection_m.max(0.0) / input.max_tire_deflection_m).clamp(0.0, 1.5)
        } else {
            0.0
        };

        st.pressure_kpa_gauge = pressure_from_gas(
            pressure.cold_kpa_gauge[i],
            pressure.reference_temperature_c,
            pressure.atmospheric_pressure_kpa,
            st.gas_c,
            st.deflection_ratio,
            thermal.volume_deflection_gain,
        )
        .clamp(
            thermal.minimum_pressure_kpa_gauge,
            thermal.maximum_pressure_kpa_gauge,
        );

        // When completely airborne, contact power is zero but air/carcass/gas still evolve.
        let _ = contact_total;
    }
}

pub fn pressure_from_gas(
    cold_pressure_kpa_gauge: f64,
    reference_temperature_c: f64,
    atmospheric_pressure_kpa: f64,
    gas_temperature_c: f64,
    deflection_ratio: f64,
    volume_deflection_gain: f64,
) -> f64 {
    let p_cold_abs = cold_pressure_kpa_gauge.max(0.0) + atmospheric_pressure_kpa.max(1.0);
    let t_ref_k = (reference_temperature_c + KELVIN_OFFSET).max(150.0);
    let t_gas_k = (gas_temperature_c + KELVIN_OFFSET).max(150.0);

    // Deflection reduces the enclosed volume slightly. This is intentionally small
    // because the solver does not model exact tire cross-section geometry.
    let volume_ratio =
        (1.0 - volume_deflection_gain.clamp(0.0, 0.10) * deflection_ratio.clamp(0.0, 1.5))
            .clamp(0.92, 1.02);

    let p_hot_abs = p_cold_abs * (t_gas_k / t_ref_k) / volume_ratio;
    (p_hot_abs - atmospheric_pressure_kpa).max(0.0)
}

fn normalized_zone_weights(
    raw: [f64; 3],
    pressure_ratio: f64,
    dynamic_camber_rad: f64,
) -> [f64; 3] {
    let mut w = [raw[0].max(0.0), raw[1].max(0.0), raw[2].max(0.0)];
    let original_sum = w.iter().sum::<f64>();
    if original_sum <= 1e-9 {
        return [0.0; 3];
    }

    // High pressure concentrates load/heat toward the center.
    // Low pressure increases shoulder contribution.
    let delta = (pressure_ratio - 1.0).clamp(-0.55, 0.55);
    if delta >= 0.0 {
        w[1] *= 1.0 + 0.85 * delta;
        w[0] *= 1.0 - 0.22 * delta;
        w[2] *= 1.0 - 0.22 * delta;
    } else {
        let low = -delta;
        w[0] *= 1.0 + 0.45 * low;
        w[2] *= 1.0 + 0.45 * low;
        w[1] *= 1.0 - 0.28 * low;
    }

    // Zones are defined physically as inner/center/outer, so negative camber biases
    // the inner shoulder regardless of whether the wheel is on the left or right side.
    let camber_bias = (-dynamic_camber_rad / 0.080).clamp(-0.35, 0.35);
    w[0] *= 1.0 + camber_bias;
    w[2] *= 1.0 - camber_bias;

    let sum = w.iter().sum::<f64>().max(1e-9);
    [
        (w[0] / sum).clamp(0.0, 1.0),
        (w[1] / sum).clamp(0.0, 1.0),
        (w[2] / sum).clamp(0.0, 1.0),
    ]
}

fn thermal_grip_scale(temp_c: f64, cfg: &TireThermalConfig) -> f64 {
    let opt = cfg.optimal_tread_temperature_c;
    let window = cfg.optimal_tread_window_c.max(2.0);
    let low_opt = opt - window * 0.5;
    let high_opt = opt + window * 0.5;
    let overheat = cfg.overheat_tread_temperature_c.max(high_opt + 2.0);

    if temp_c <= 20.0 {
        0.68
    } else if temp_c < low_opt {
        lerp(0.68, 1.0, ((temp_c - 20.0) / (low_opt - 20.0).max(1.0)).clamp(0.0, 1.0))
    } else if temp_c <= high_opt {
        1.0
    } else if temp_c < overheat {
        lerp(1.0, 0.94, ((temp_c - high_opt) / (overheat - high_opt)).clamp(0.0, 1.0))
    } else {
        lerp(0.94, 0.78, ((temp_c - overheat) / 35.0).clamp(0.0, 1.0))
    }
}

#[inline]
fn finite_temp(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(-40.0, 220.0)
    } else {
        25.0
    }
}

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ideal_gas_pressure_rises_with_temperature() {
        let cold = pressure_from_gas(110.0, 25.0, 101.325, 25.0, 0.0, 0.04);
        let hot = pressure_from_gas(110.0, 25.0, 101.325, 80.0, 0.0, 0.04);
        assert!((cold - 110.0).abs() < 1e-6);
        assert!(hot > cold);
    }

    #[test]
    fn lower_pressure_softens_vertical_tire() {
        let p = TirePressureConfig::default();
        let t = TireThermalConfig::default();
        let mut sys = TireThermalSystem::new(&p, &t);
        sys.wheels[0].pressure_kpa_gauge = 90.0;
        sys.wheels[1].pressure_kpa_gauge = 160.0;

        let low = sys.mechanical_modifiers(WheelIndex::FrontLeft, &p, &t);
        let high = sys.mechanical_modifiers(WheelIndex::FrontRight, &p, &t);

        assert!(low.vertical_stiffness_scale < high.vertical_stiffness_scale);
        assert!(low.max_deflection_scale > high.max_deflection_scale);
        assert!(low.contact_patch_scale > high.contact_patch_scale);
        assert!(low.relaxation_length_scale > high.relaxation_length_scale);
        assert!(low.rolling_resistance_scale > high.rolling_resistance_scale);
    }

    #[test]
    fn hot_tire_over_optimum_loses_grip() {
        let p = TirePressureConfig::default();
        let t = TireThermalConfig::default();
        let mut sys = TireThermalSystem::new(&p, &t);

        sys.wheels[0].tread_inner_c = 95.0;
        sys.wheels[0].tread_center_c = 95.0;
        sys.wheels[0].tread_outer_c = 95.0;

        sys.wheels[1].tread_inner_c = 145.0;
        sys.wheels[1].tread_center_c = 145.0;
        sys.wheels[1].tread_outer_c = 145.0;

        let optimum = sys.mechanical_modifiers(WheelIndex::FrontLeft, &p, &t);
        let overheated = sys.mechanical_modifiers(WheelIndex::FrontRight, &p, &t);

        assert!(overheated.grip_scale < optimum.grip_scale);
    }
}
