//! Brake thermal + brake-duct airflow subsystem for Formula-90.
//!
//! Design goals:
//! - braking heat comes from actual torque * wheel angular velocity;
//! - temperature affects usable brake torque on the NEXT mechanical solve;
//! - heat reaches tire through hub/rim, not directly to tread;
//! - the same duct effective area controls both cooling airflow and aerodynamic drag;
//! - all values are deterministic and allocation-free per physics tick.

use crate::types::WheelIndex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BrakeAxleThermalConfig {
    pub disc_heat_capacity_j_k: f64,
    pub caliper_heat_capacity_j_k: f64,
    pub hub_heat_capacity_j_k: f64,
    pub rim_heat_capacity_j_k: f64,

    pub disc_to_caliper_w_k: f64,
    pub disc_to_hub_w_k: f64,
    pub disc_to_rim_radiation_w_k: f64,
    pub hub_to_rim_w_k: f64,

    pub rim_to_tire_carcass_w_k: f64,
    pub rim_to_tire_gas_w_k: f64,

    /// Cooling with no duct ram flow.
    pub disc_base_air_w_k: f64,
    pub caliper_base_air_w_k: f64,
    pub hub_base_air_w_k: f64,
    pub rim_base_air_w_k: f64,

    /// Extra conductance produced by duct mass flow.
    pub disc_flow_cooling_gain_w_k: f64,
    pub caliper_flow_cooling_gain_w_k: f64,
    pub hub_flow_cooling_gain_w_k: f64,
    pub rim_flow_cooling_gain_w_k: f64,
}

impl Default for BrakeAxleThermalConfig {
    fn default() -> Self {
        Self {
            disc_heat_capacity_j_k: 18_000.0,
            caliper_heat_capacity_j_k: 8_000.0,
            hub_heat_capacity_j_k: 7_000.0,
            rim_heat_capacity_j_k: 14_000.0,

            disc_to_caliper_w_k: 26.0,
            disc_to_hub_w_k: 38.0,
            disc_to_rim_radiation_w_k: 9.0,
            hub_to_rim_w_k: 31.0,

            rim_to_tire_carcass_w_k: 14.0,
            rim_to_tire_gas_w_k: 8.0,

            disc_base_air_w_k: 18.0,
            caliper_base_air_w_k: 8.0,
            hub_base_air_w_k: 5.0,
            rim_base_air_w_k: 10.0,

            disc_flow_cooling_gain_w_k: 105.0,
            caliper_flow_cooling_gain_w_k: 42.0,
            hub_flow_cooling_gain_w_k: 18.0,
            rim_flow_cooling_gain_w_k: 24.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BrakeDuctAxleConfig {
    /// Setup value: 0.0 = nominally closed, 1.0 = fully open.
    pub opening: f64,

    /// Geometric maximum inlet area for ONE wheel.
    pub max_inlet_area_m2_per_wheel: f64,

    /// Converts geometric inlet area into flowing area.
    pub discharge_coefficient: f64,

    /// Fraction of freestream dynamic pressure recovered at the inlet.
    pub pressure_recovery: f64,

    /// Aerodynamic drag coefficient referred to effective inlet area.
    pub drag_coefficient: f64,

    /// Allows non-linear setup slider -> effective area response.
    pub area_response_exponent: f64,

    /// Heat-transfer response to mass flow.
    pub cooling_flow_exponent: f64,

    /// Fraction of full-flow equivalent cooling retained at opening=0.
    /// Represents exposed wheel/brake airflow and prevents zero cooling.
    pub minimum_cooling_flow_ratio: f64,
}

impl Default for BrakeDuctAxleConfig {
    fn default() -> Self {
        Self {
            opening: 0.50,
            max_inlet_area_m2_per_wheel: 0.0060,
            discharge_coefficient: 0.72,
            pressure_recovery: 0.65,
            drag_coefficient: 0.90,
            area_response_exponent: 1.15,
            cooling_flow_exponent: 0.80,
            minimum_cooling_flow_ratio: 0.12,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BrakeThermalConfig {
    pub initial_temperature_c: f64,

    pub optimal_min_temperature_c: f64,
    pub optimal_max_temperature_c: f64,
    pub fade_start_temperature_c: f64,
    pub critical_temperature_c: f64,

    /// Effective brake torque when the disc is near ambient/cold.
    pub cold_efficiency: f64,
    /// Lower clamp at extreme overheat.
    pub minimum_fade_efficiency: f64,

    /// Fraction of mechanical braking power deposited in the brake assembly.
    pub braking_heat_fraction: f64,

    /// Split of generated heat. Remaining heat enters the disc.
    pub direct_caliper_heat_fraction: f64,

    pub front: BrakeAxleThermalConfig,
    pub rear: BrakeAxleThermalConfig,
    pub front_duct: BrakeDuctAxleConfig,
    pub rear_duct: BrakeDuctAxleConfig,
}

impl Default for BrakeThermalConfig {
    fn default() -> Self {
        let front = BrakeAxleThermalConfig::default();
        let mut rear = BrakeAxleThermalConfig::default();
        // Slightly lower rear thermal mass as a useful F1-94 calibration starting point.
        rear.disc_heat_capacity_j_k = 15_500.0;
        rear.caliper_heat_capacity_j_k = 7_200.0;
        rear.hub_heat_capacity_j_k = 6_500.0;

        let front_duct = BrakeDuctAxleConfig::default();
        let mut rear_duct = BrakeDuctAxleConfig::default();
        rear_duct.opening = 0.40;
        rear_duct.max_inlet_area_m2_per_wheel = 0.0050;
        rear_duct.drag_coefficient = 0.85;

        Self {
            initial_temperature_c: 25.0,
            optimal_min_temperature_c: 400.0,
            optimal_max_temperature_c: 800.0,
            fade_start_temperature_c: 900.0,
            critical_temperature_c: 1100.0,
            cold_efficiency: 0.78,
            minimum_fade_efficiency: 0.58,
            braking_heat_fraction: 0.94,
            direct_caliper_heat_fraction: 0.04,
            front,
            rear,
            front_duct,
            rear_duct,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BrakeDuctFlowState {
    pub effective_area_m2: f64,
    pub mass_flow_kg_s: f64,
    pub cooling_flow_ratio: f64,
    pub drag_force_n: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WheelBrakeThermalState {
    pub disc_c: f64,
    pub caliper_c: f64,
    pub hub_c: f64,
    pub rim_c: f64,

    /// Current multiplier applied to actual brake torque.
    pub efficiency: f64,

    /// Diagnostics.
    pub brake_power_w: f64,
    pub duct: BrakeDuctFlowState,
}

impl WheelBrakeThermalState {
    fn new(cfg: &BrakeThermalConfig) -> Self {
        Self {
            disc_c: cfg.initial_temperature_c,
            caliper_c: cfg.initial_temperature_c,
            hub_c: cfg.initial_temperature_c,
            rim_c: cfg.initial_temperature_c,
            efficiency: brake_efficiency(cfg.initial_temperature_c, cfg),
            brake_power_w: 0.0,
            duct: BrakeDuctFlowState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BrakeThermalInput {
    /// ACTUAL torque after ABS and after current thermal fade scale.
    pub applied_brake_torque_nm: f64,
    pub wheel_angular_speed_rad_s: f64,
    pub vehicle_speed_ms: f64,
    pub air_density_kg_m3: f64,
    pub ambient_temperature_c: f64,

    /// Current tire thermal nodes. Used only to evaluate rim -> tire heat flux.
    pub tire_carcass_temperature_c: f64,
    pub tire_gas_temperature_c: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BrakeToTireHeat {
    /// Positive = heat entering the tire node.
    pub carcass_heat_w: f64,
    pub gas_heat_w: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrakeThermalSystem {
    pub wheels: [WheelBrakeThermalState; 4],
}

impl BrakeThermalSystem {
    pub fn new(cfg: &BrakeThermalConfig) -> Self {
        Self {
            wheels: [
                WheelBrakeThermalState::new(cfg),
                WheelBrakeThermalState::new(cfg),
                WheelBrakeThermalState::new(cfg),
                WheelBrakeThermalState::new(cfg),
            ],
        }
    }

    pub fn reset(&mut self, cfg: &BrakeThermalConfig) {
        *self = Self::new(cfg);
    }

    /// Read BEFORE drivetrain braking is applied this tick.
    pub fn efficiency_scales(&self) -> [f64; 4] {
        [
            self.wheels[0].efficiency,
            self.wheels[1].efficiency,
            self.wheels[2].efficiency,
            self.wheels[3].efficiency,
        ]
    }

    pub fn total_duct_drag_force_n(
        &self,
        cfg: &BrakeThermalConfig,
        air_density_kg_m3: f64,
        vehicle_speed_ms: f64,
    ) -> f64 {
        WheelIndex::ALL
            .iter()
            .map(|&wheel| {
                evaluate_duct_flow(
                    self.duct_config(cfg, wheel),
                    air_density_kg_m3,
                    vehicle_speed_ms,
                )
                .drag_force_n
            })
            .sum()
    }

    /// Update AFTER actual brake torque for the tick is known.
    /// Returns heat flux that MUST be added to TireThermalInput this same thermal stage.
    pub fn step_after_braking(
        &mut self,
        wheel: WheelIndex,
        cfg: &BrakeThermalConfig,
        input: BrakeThermalInput,
        dt: f64,
    ) -> BrakeToTireHeat {
        let dt = dt.clamp(1.0 / 2000.0, 0.05);
        let i = wheel as usize;
        let axle = *self.axle_config(cfg, wheel);
        let duct_cfg = *self.duct_config(cfg, wheel);
        let st = &mut self.wheels[i];

        let duct = evaluate_duct_flow(
            &duct_cfg,
            input.air_density_kg_m3,
            input.vehicle_speed_ms,
        );
        st.duct = duct;

        // Mechanical power removed at the brake. Since applied torque is already
        // post-ABS and post-fade, the thermal model cannot invent braking energy.
        let brake_power_w =
            input.applied_brake_torque_nm.abs() * input.wheel_angular_speed_rad_s.abs();
        st.brake_power_w = brake_power_w;

        let generated_heat_w =
            brake_power_w * cfg.braking_heat_fraction.clamp(0.0, 1.0);
        let direct_caliper_w =
            generated_heat_w * cfg.direct_caliper_heat_fraction.clamp(0.0, 0.20);
        let direct_disc_w = generated_heat_w - direct_caliper_w;

        // Evaluate every conductive/radiative flux from OLD state before updates.
        // Positive q_ab means heat flows from node A to node B.
        let q_disc_caliper = axle.disc_to_caliper_w_k * (st.disc_c - st.caliper_c);
        let q_disc_hub = axle.disc_to_hub_w_k * (st.disc_c - st.hub_c);
        let q_disc_rim = axle.disc_to_rim_radiation_w_k * (st.disc_c - st.rim_c);
        let q_hub_rim = axle.hub_to_rim_w_k * (st.hub_c - st.rim_c);

        let q_rim_to_carcass =
            axle.rim_to_tire_carcass_w_k * (st.rim_c - input.tire_carcass_temperature_c);
        let q_rim_to_gas =
            axle.rim_to_tire_gas_w_k * (st.rim_c - input.tire_gas_temperature_c);

        let flow_factor = duct
            .cooling_flow_ratio
            .max(0.0)
            .powf(duct_cfg.cooling_flow_exponent.clamp(0.2, 1.5));

        let h_disc = axle.disc_base_air_w_k + axle.disc_flow_cooling_gain_w_k * flow_factor;
        let h_caliper =
            axle.caliper_base_air_w_k + axle.caliper_flow_cooling_gain_w_k * flow_factor;
        let h_hub = axle.hub_base_air_w_k + axle.hub_flow_cooling_gain_w_k * flow_factor;
        let h_rim = axle.rim_base_air_w_k + axle.rim_flow_cooling_gain_w_k * flow_factor;

        let q_disc_air = h_disc * (st.disc_c - input.ambient_temperature_c);
        let q_caliper_air = h_caliper * (st.caliper_c - input.ambient_temperature_c);
        let q_hub_air = h_hub * (st.hub_c - input.ambient_temperature_c);
        let q_rim_air = h_rim * (st.rim_c - input.ambient_temperature_c);

        let disc_net_w =
            direct_disc_w - q_disc_caliper - q_disc_hub - q_disc_rim - q_disc_air;
        let caliper_net_w = direct_caliper_w + q_disc_caliper - q_caliper_air;
        let hub_net_w = q_disc_hub - q_hub_rim - q_hub_air;
        let rim_net_w =
            q_disc_rim + q_hub_rim - q_rim_to_carcass - q_rim_to_gas - q_rim_air;

        st.disc_c = finite_temp(
            st.disc_c + disc_net_w / axle.disc_heat_capacity_j_k.max(500.0) * dt,
        );
        st.caliper_c = finite_temp(
            st.caliper_c + caliper_net_w / axle.caliper_heat_capacity_j_k.max(500.0) * dt,
        );
        st.hub_c = finite_temp(
            st.hub_c + hub_net_w / axle.hub_heat_capacity_j_k.max(500.0) * dt,
        );
        st.rim_c = finite_temp(
            st.rim_c + rim_net_w / axle.rim_heat_capacity_j_k.max(500.0) * dt,
        );

        // Efficiency is for NEXT tick. No same-tick algebraic brake loop.
        st.efficiency = brake_efficiency(st.disc_c, cfg);

        BrakeToTireHeat {
            carcass_heat_w: q_rim_to_carcass,
            gas_heat_w: q_rim_to_gas,
        }
    }

    fn axle_config<'a>(
        &self,
        cfg: &'a BrakeThermalConfig,
        wheel: WheelIndex,
    ) -> &'a BrakeAxleThermalConfig {
        if wheel.is_front() { &cfg.front } else { &cfg.rear }
    }

    fn duct_config<'a>(
        &self,
        cfg: &'a BrakeThermalConfig,
        wheel: WheelIndex,
    ) -> &'a BrakeDuctAxleConfig {
        if wheel.is_front() { &cfg.front_duct } else { &cfg.rear_duct }
    }
}

pub fn evaluate_duct_flow(
    cfg: &BrakeDuctAxleConfig,
    air_density_kg_m3: f64,
    vehicle_speed_ms: f64,
) -> BrakeDuctFlowState {
    let rho = air_density_kg_m3.clamp(0.5, 1.6);
    let v = vehicle_speed_ms.abs().max(0.0);
    let opening = cfg.opening.clamp(0.0, 1.0);

    let area_fraction = opening.powf(cfg.area_response_exponent.clamp(0.2, 3.0));
    let effective_area =
        cfg.max_inlet_area_m2_per_wheel.max(0.0) * area_fraction;

    let q_dynamic = 0.5 * rho * v * v;
    let delta_p = q_dynamic * cfg.pressure_recovery.clamp(0.0, 1.2);

    let ram_mass_flow = if effective_area > 0.0 && delta_p > 0.0 {
        cfg.discharge_coefficient.clamp(0.0, 1.5)
            * effective_area
            * (2.0 * rho * delta_p).sqrt()
    } else {
        0.0
    };

    // Full-open reference at this same speed. Allows cooling curves to remain
    // normalized while still respecting v-dependent mass flow.
    let full_area = cfg.max_inlet_area_m2_per_wheel.max(0.0);
    let full_mass_flow = if full_area > 0.0 && delta_p > 0.0 {
        cfg.discharge_coefficient.clamp(0.0, 1.5)
            * full_area
            * (2.0 * rho * delta_p).sqrt()
    } else {
        0.0
    };

    let normalized_ram = if full_mass_flow > 1e-9 {
        (ram_mass_flow / full_mass_flow).clamp(0.0, 1.5)
    } else {
        0.0
    };

    let cooling_flow_ratio = cfg.minimum_cooling_flow_ratio.clamp(0.0, 0.5)
        + (1.0 - cfg.minimum_cooling_flow_ratio.clamp(0.0, 0.5)) * normalized_ram;

    // Inlet drag uses the SAME effective area as cooling.
    let drag_force_n =
        q_dynamic * cfg.drag_coefficient.max(0.0) * effective_area;

    BrakeDuctFlowState {
        effective_area_m2: effective_area,
        mass_flow_kg_s: ram_mass_flow,
        cooling_flow_ratio,
        drag_force_n,
    }
}

pub fn brake_efficiency(disc_temperature_c: f64, cfg: &BrakeThermalConfig) -> f64 {
    let t = disc_temperature_c;
    let opt_min = cfg.optimal_min_temperature_c;
    let opt_max = cfg.optimal_max_temperature_c.max(opt_min + 1.0);
    let fade_start = cfg.fade_start_temperature_c.max(opt_max);
    let critical = cfg.critical_temperature_c.max(fade_start + 1.0);

    if t <= 25.0 {
        cfg.cold_efficiency.clamp(0.3, 1.0)
    } else if t < opt_min {
        lerp(
            cfg.cold_efficiency.clamp(0.3, 1.0),
            1.0,
            ((t - 25.0) / (opt_min - 25.0).max(1.0)).clamp(0.0, 1.0),
        )
    } else if t <= fade_start {
        1.0
    } else {
        lerp(
            1.0,
            cfg.minimum_fade_efficiency.clamp(0.2, 1.0),
            ((t - fade_start) / (critical - fade_start)).clamp(0.0, 1.0),
        )
    }
}

#[inline]
fn finite_temp(v: f64) -> f64 {
    if v.is_finite() { v.clamp(-40.0, 1400.0) } else { 25.0 }
}

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotter_disc_fades_after_fade_start() {
        let cfg = BrakeThermalConfig::default();
        assert_eq!(brake_efficiency(600.0, &cfg), 1.0);
        assert!(brake_efficiency(1050.0, &cfg) < 1.0);
    }

    #[test]
    fn open_duct_increases_both_flow_and_drag() {
        let mut closed = BrakeDuctAxleConfig::default();
        closed.opening = 0.10;
        let mut open = closed;
        open.opening = 0.90;

        let a = evaluate_duct_flow(&closed, 1.225, 70.0);
        let b = evaluate_duct_flow(&open, 1.225, 70.0);

        assert!(b.mass_flow_kg_s > a.mass_flow_kg_s);
        assert!(b.drag_force_n > a.drag_force_n);
    }

    #[test]
    fn duct_drag_rises_with_speed_squared() {
        let cfg = BrakeDuctAxleConfig::default();
        let slow = evaluate_duct_flow(&cfg, 1.225, 30.0);
        let fast = evaluate_duct_flow(&cfg, 1.225, 60.0);
        assert!(fast.drag_force_n > slow.drag_force_n * 3.9);
    }
}
