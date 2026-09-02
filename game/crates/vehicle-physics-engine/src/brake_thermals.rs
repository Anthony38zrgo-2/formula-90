//! Brake thermal and brake-duct airflow subsystem.
//!
//! The compact v3 model uses two thermal nodes per wheel: a lumped rotor node
//! (friction surface + rotor bulk + caliper heat) and a rim node. Brake heat is
//! generated from the actual wheel brake torque after ABS and thermal fade.
//! The rotor cools through natural airflow, speed-derived forced convection and
//! the brake duct; heat conducts through a lumped rotor->rim conductance; the
//! rim rejects to airflow and couples the remaining heat into the tire
//! carcass/gas nodes. Duct cooling and duct drag both consume the same
//! effective inlet area and airflow evaluation.

use crate::types::WheelIndex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrakeThermalModelKind {
    LegacyTwoNode,
    ScaledTwoNodeV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrakeRotorMaterial {
    CarbonCarbon,
    CastIron,
    CarbonCeramic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrakeRotorVentilation {
    Solid,
    Vented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrakeCoolingProfile {
    OpenWheelDucted,
    RoadVented,
    RoadSolid,
}

/// Per-axle brake thermal configuration. The compact v3 shape derives the
/// lumped rotor capacity from the physical rotor material/mass (v2 explicit
/// capacity keys are parse-only and mapped through physical lumping).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BrakeAxleThermalConfig {
    /// JSON compatibility marker; both variants run the compact two-node model.
    pub model: BrakeThermalModelKind,
    pub rotor_material: BrakeRotorMaterial,
    pub rotor_mass_kg: f64,
    pub rotor_outer_diameter_m: f64,
    pub rotor_inner_diameter_m: f64,
    pub rotor_ventilation: BrakeRotorVentilation,
    pub cooling_profile: BrakeCoolingProfile,
    pub installation_airflow_scale: f64,
    pub thermal_mass_scale: f64,
    /// Lumped rotor-to-rim conductance: series rotor->hub->rim conduction plus
    /// rotor-to-rim radiation, combined into a single effective W/K.
    pub rotor_to_rim_w_k: f64,
    pub rim_heat_capacity_j_k: f64,
    pub rim_base_air_w_k: f64,
    pub rim_to_tire_carcass_w_k: f64,
    pub rim_to_tire_gas_w_k: f64,
}

impl Default for BrakeAxleThermalConfig {
    fn default() -> Self {
        Self {
            model: BrakeThermalModelKind::LegacyTwoNode,
            rotor_material: BrakeRotorMaterial::CarbonCarbon,
            // 1.35 kg of carbon-carbon = 1500 J/K lumped rotor capacity
            // (matches the historical front calibration exactly).
            rotor_mass_kg: 1.35,
            rotor_outer_diameter_m: 0.278,
            rotor_inner_diameter_m: 0.105,
            rotor_ventilation: BrakeRotorVentilation::Solid,
            cooling_profile: BrakeCoolingProfile::RoadSolid,
            installation_airflow_scale: 1.0,
            thermal_mass_scale: 1.0,
            // Series 38 disc->hub + 31 hub->rim plus 9 W/K radiation.
            rotor_to_rim_w_k: 26.0,
            rim_heat_capacity_j_k: 6_500.0,
            rim_base_air_w_k: 10.0,
            rim_to_tire_carcass_w_k: 14.0,
            rim_to_tire_gas_w_k: 8.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BrakeDuctAxleConfig {
    pub opening: f64,
    pub max_inlet_area_m2_per_wheel: f64,
    pub discharge_coefficient: f64,
    pub pressure_recovery: f64,
    /// Full-opening mass-flow reference point used to turn the physical
    /// duct flow into a cooling coefficient. This keeps cooling and drag on
    /// the same effective-area/airflow model without cancelling vehicle speed.
    pub cooling_reference_speed_ms: f64,
    pub drag_coefficient: f64,
    pub area_response_exponent: f64,
    pub cooling_flow_exponent: f64,
    pub minimum_cooling_flow_ratio: f64,
    /// Specific heat capacity of the air used by the dynamic cooling model.
    pub air_specific_heat_j_kg_k: f64,
    /// Effective heat-exchanger UA for the complete duct path at saturated
    /// flow. The actual conductance is limited by m_dot * Cp.
    pub heat_exchanger_ua_w_k: f64,
    /// Share of the dynamic cooling conductance assigned to the rotor;
    /// the remainder cools the rim.
    pub rotor_cooling_fraction: f64,
}

impl Default for BrakeDuctAxleConfig {
    fn default() -> Self {
        Self {
            opening: 0.50,
            max_inlet_area_m2_per_wheel: 0.0060,
            discharge_coefficient: 0.72,
            pressure_recovery: 0.65,
            cooling_reference_speed_ms: 50.0,
            drag_coefficient: 0.90,
            area_response_exponent: 1.15,
            cooling_flow_exponent: 0.80,
            minimum_cooling_flow_ratio: 0.12,
            air_specific_heat_j_kg_k: 1_005.0,
            heat_exchanger_ua_w_k: 450.0,
            // Historical per-node weights lumped onto the rotor path
            // (disc 0.556 + caliper 0.222 + hub 0.095 of the total).
            rotor_cooling_fraction: 0.873,
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
    pub cold_efficiency: f64,
    pub minimum_fade_efficiency: f64,
    pub braking_heat_fraction: f64,
    pub front: BrakeAxleThermalConfig,
    pub rear: BrakeAxleThermalConfig,
    pub front_duct: BrakeDuctAxleConfig,
    pub rear_duct: BrakeDuctAxleConfig,
}

impl Default for BrakeThermalConfig {
    fn default() -> Self {
        let front = BrakeAxleThermalConfig::default();
        let mut rear = front;
        rear.rotor_mass_kg = 1.80;
        rear.rotor_to_rim_w_k = 34.0;
        rear.rim_heat_capacity_j_k = 6_000.0;

        let front_duct = BrakeDuctAxleConfig::default();
        let mut rear_duct = front_duct;
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
            front,
            rear,
            front_duct,
            rear_duct,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedBrakeAxleThermalConfig {
    /// Lumped rotor capacity in J/K (material cp * rotor mass * mass scale).
    pub rotor_capacity_j_k: f64,
    /// Natural (static) convection conductance of the rotor to airflow.
    pub rotor_natural_w_k: f64,
    /// Speed-derived forced convection conductance at the cooling reference speed.
    pub rotor_forced_at_reference_w_k: f64,
    pub cooling_reference_speed_ms: f64,
    pub cooling_speed_exponent: f64,
    /// Fraction of generated braking heat deposited directly into the rotor.
    pub rotor_heat_fraction: f64,
}

impl BrakeAxleThermalConfig {
    pub fn resolve(&self) -> ResolvedBrakeAxleThermalConfig {
        let material = material_properties(self.rotor_material);
        let scale = self.thermal_mass_scale.max(0.01);
        let total_capacity = if self.rotor_mass_kg > 0.0 {
            self.rotor_mass_kg * material.specific_heat_j_kg_k * scale
        } else {
            // Degenerate fallback for configs that never declared a mass.
            2_400.0 * scale
        };
        let area = annular_area(self.rotor_outer_diameter_m, self.rotor_inner_diameter_m);
        let profile = profile_properties(self.cooling_profile);
        let ventilation_multiplier = match self.rotor_ventilation {
            BrakeRotorVentilation::Solid => profile.solid_area_multiplier,
            BrakeRotorVentilation::Vented => profile.vented_area_multiplier,
        };
        let exposed_area = (area * 2.0 * ventilation_multiplier).max(0.0);
        let airflow_scale = self.installation_airflow_scale.max(0.0);
        let natural_total = profile.natural_h_w_m2_k * exposed_area * airflow_scale;
        let forced_total =
            profile.forced_h_at_reference_w_m2_k * exposed_area * airflow_scale;
        ResolvedBrakeAxleThermalConfig {
            rotor_capacity_j_k: total_capacity.max(1.0),
            rotor_natural_w_k: natural_total,
            rotor_forced_at_reference_w_k: forced_total,
            cooling_reference_speed_ms: profile.reference_speed_ms,
            cooling_speed_exponent: profile.speed_exponent,
            rotor_heat_fraction: material.rotor_heat_fraction.clamp(0.50, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MaterialProperties {
    specific_heat_j_kg_k: f64,
    rotor_heat_fraction: f64,
}

fn material_properties(material: BrakeRotorMaterial) -> MaterialProperties {
    match material {
        BrakeRotorMaterial::CarbonCarbon => MaterialProperties {
            specific_heat_j_kg_k: 1_111.111,
            rotor_heat_fraction: 0.96,
        },
        BrakeRotorMaterial::CastIron => MaterialProperties {
            specific_heat_j_kg_k: 500.0,
            rotor_heat_fraction: 0.90,
        },
        BrakeRotorMaterial::CarbonCeramic => MaterialProperties {
            specific_heat_j_kg_k: 850.0,
            rotor_heat_fraction: 0.94,
        },
    }
}

#[derive(Debug, Clone, Copy)]
struct CoolingProfileProperties {
    natural_h_w_m2_k: f64,
    forced_h_at_reference_w_m2_k: f64,
    reference_speed_ms: f64,
    speed_exponent: f64,
    solid_area_multiplier: f64,
    vented_area_multiplier: f64,
}

fn profile_properties(profile: BrakeCoolingProfile) -> CoolingProfileProperties {
    match profile {
        BrakeCoolingProfile::OpenWheelDucted => CoolingProfileProperties {
            natural_h_w_m2_k: 5.0,
            forced_h_at_reference_w_m2_k: 65.0,
            reference_speed_ms: 50.0,
            speed_exponent: 0.8,
            solid_area_multiplier: 1.0,
            vented_area_multiplier: 1.8,
        },
        BrakeCoolingProfile::RoadVented => CoolingProfileProperties {
            natural_h_w_m2_k: 5.0,
            forced_h_at_reference_w_m2_k: 45.0,
            reference_speed_ms: 40.0,
            speed_exponent: 0.8,
            solid_area_multiplier: 1.0,
            vented_area_multiplier: 1.6,
        },
        BrakeCoolingProfile::RoadSolid => CoolingProfileProperties {
            natural_h_w_m2_k: 4.0,
            forced_h_at_reference_w_m2_k: 25.0,
            reference_speed_ms: 40.0,
            speed_exponent: 0.75,
            solid_area_multiplier: 1.0,
            vented_area_multiplier: 1.0,
        },
    }
}

fn annular_area(outer_diameter_m: f64, inner_diameter_m: f64) -> f64 {
    let outer = (outer_diameter_m * 0.5).max(0.0);
    let inner = (inner_diameter_m * 0.5).clamp(0.0, outer);
    std::f64::consts::PI * (outer * outer - inner * inner)
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BrakeDuctFlowState {
    pub effective_area_m2: f64,
    pub mass_flow_kg_s: f64,
    pub cooling_flow_ratio: f64,
    /// Effective dynamic cooling conductance from m_dot * Cp and the duct UA.
    pub cooling_conductance_w_k: f64,
    pub drag_force_n: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WheelBrakeThermalState {
    /// Lumped rotor (surface + bulk + caliper heat) temperature; the
    /// canonical brake/HUD value.
    pub disc_c: f64,
    pub rim_c: f64,
    pub efficiency: f64,
    pub applied_brake_torque_nm: f64,
    pub wheel_spin_pre_rad_s: f64,
    pub wheel_spin_post_rad_s: f64,
    pub brake_power_w: f64,
    pub brake_energy_j: f64,
    /// Resolved lumped rotor thermal capacity (J/K).
    pub resolved_rotor_capacity_j_k: f64,
    /// Natural (static) airflow cooling conductance of the rotor.
    pub natural_cooling_w_k: f64,
    /// Speed-derived forced convection cooling conductance of the rotor.
    pub speed_cooling_w_k: f64,
    /// Heat flux from the rotor node into the rim node (W).
    pub to_rim_heat_w: f64,
    pub duct: BrakeDuctFlowState,
}

impl WheelBrakeThermalState {
    fn new(cfg: &BrakeThermalConfig) -> Self {
        Self {
            disc_c: cfg.initial_temperature_c,
            rim_c: cfg.initial_temperature_c,
            efficiency: brake_efficiency(cfg.initial_temperature_c, cfg),
            applied_brake_torque_nm: 0.0,
            wheel_spin_pre_rad_s: 0.0,
            wheel_spin_post_rad_s: 0.0,
            brake_power_w: 0.0,
            brake_energy_j: 0.0,
            // Filled from the front/rear resolved axle profile by `new`; keep
            // construction neutral so a state is never briefly labeled as
            // front-axle data when the config is rear-specific.
            resolved_rotor_capacity_j_k: 0.0,
            natural_cooling_w_k: 0.0,
            speed_cooling_w_k: 0.0,
            to_rim_heat_w: 0.0,
            duct: BrakeDuctFlowState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BrakeThermalInput {
    /// Actual torque after ABS and current-tick thermal efficiency.
    pub applied_brake_torque_nm: f64,
    /// Wheel angular speed before the mechanical wheel-torque solve.
    pub wheel_spin_pre_rad_s: f64,
    /// Wheel angular speed after the mechanical wheel-torque solve.
    pub wheel_spin_post_rad_s: f64,
    pub vehicle_speed_ms: f64,
    pub air_density_kg_m3: f64,
    pub ambient_temperature_c: f64,
    pub tire_carcass_temperature_c: f64,
    pub tire_gas_temperature_c: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BrakeToTireHeat {
    pub carcass_heat_w: f64,
    pub gas_heat_w: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrakeThermalSystem {
    pub wheels: [WheelBrakeThermalState; 4],
}

impl BrakeThermalSystem {
    pub fn new(cfg: &BrakeThermalConfig) -> Self {
        let mut system = Self {
            wheels: [
                WheelBrakeThermalState::new(cfg),
                WheelBrakeThermalState::new(cfg),
                WheelBrakeThermalState::new(cfg),
                WheelBrakeThermalState::new(cfg),
            ],
        };
        for wheel in WheelIndex::ALL {
            let resolved = axle_config(cfg, wheel).resolve();
            let state = &mut system.wheels[wheel as usize];
            state.resolved_rotor_capacity_j_k = resolved.rotor_capacity_j_k;
        }
        system
    }

    pub fn reset(&mut self, cfg: &BrakeThermalConfig) {
        *self = Self::new(cfg);
    }

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
        let _ = self;
        WheelIndex::ALL
            .iter()
            .map(|&wheel| {
                evaluate_duct_flow(duct_config(cfg, wheel), air_density_kg_m3, vehicle_speed_ms)
                    .drag_force_n
            })
            .sum()
    }

    pub fn step_after_braking(
        &mut self,
        wheel: WheelIndex,
        cfg: &BrakeThermalConfig,
        input: BrakeThermalInput,
        dt: f64,
    ) -> BrakeToTireHeat {
        let dt = dt.clamp(1.0 / 2000.0, 0.05);
        let i = wheel as usize;
        let axle = axle_config(cfg, wheel);
        let duct_cfg = duct_config(cfg, wheel);
        let st = &mut self.wheels[i];

        let duct = evaluate_duct_flow(duct_cfg, input.air_density_kg_m3, input.vehicle_speed_ms);
        st.duct = duct;

        let average_wheel_spin_rad_s =
            ((input.wheel_spin_pre_rad_s + input.wheel_spin_post_rad_s) * 0.5).abs();
        let brake_power_w = input.applied_brake_torque_nm.abs() * average_wheel_spin_rad_s;
        st.applied_brake_torque_nm = finite_nonnegative(input.applied_brake_torque_nm);
        st.wheel_spin_pre_rad_s = finite_signed(input.wheel_spin_pre_rad_s);
        st.wheel_spin_post_rad_s = finite_signed(input.wheel_spin_post_rad_s);
        st.brake_power_w = finite_nonnegative(brake_power_w);
        st.brake_energy_j = finite_nonnegative(st.brake_energy_j + st.brake_power_w * dt);

        let resolved = axle.resolve();
        st.resolved_rotor_capacity_j_k = resolved.rotor_capacity_j_k;
        let generated_heat_w = st.brake_power_w * cfg.braking_heat_fraction.clamp(0.0, 1.0);
        let rotor_heat_w = generated_heat_w * resolved.rotor_heat_fraction;
        // The non-rotor share (historically the caliper direct-heat path) is
        // deposited directly on the rim node so total heat is conserved in
        // the two-node lumping.
        let rim_direct_heat_w = (generated_heat_w - rotor_heat_w).max(0.0);

        let q_rotor_rim = axle.rotor_to_rim_w_k.max(0.0) * (st.disc_c - st.rim_c);
        let q_rim_to_carcass =
            axle.rim_to_tire_carcass_w_k * (st.rim_c - input.tire_carcass_temperature_c);
        let q_rim_to_gas = axle.rim_to_tire_gas_w_k * (st.rim_c - input.tire_gas_temperature_c);

        let speed_ratio = if resolved.cooling_reference_speed_ms > 0.0 {
            input.vehicle_speed_ms.abs() / resolved.cooling_reference_speed_ms
        } else {
            0.0
        };
        let speed_factor = speed_ratio
            .max(0.0)
            .powf(resolved.cooling_speed_exponent.max(0.01))
            .clamp(0.0, 4.0);
        let dynamic_h = duct.cooling_conductance_w_k;
        let rotor_share = duct_cfg.rotor_cooling_fraction.clamp(0.0, 1.0);
        let h_rotor_air = resolved.rotor_natural_w_k
            + resolved.rotor_forced_at_reference_w_k * speed_factor
            + dynamic_h * rotor_share;
        let h_rim_air = axle.rim_base_air_w_k + dynamic_h * (1.0 - rotor_share);
        let q_rotor_air = h_rotor_air * (st.disc_c - input.ambient_temperature_c);
        let q_rim_air = h_rim_air * (st.rim_c - input.ambient_temperature_c);

        let rotor_net_w = rotor_heat_w - q_rotor_rim - q_rotor_air;
        let rim_net_w = rim_direct_heat_w + q_rotor_rim - q_rim_to_carcass - q_rim_to_gas - q_rim_air;

        let rotor_capacity = resolved.rotor_capacity_j_k.max(100.0);
        let rim_capacity = axle.rim_heat_capacity_j_k.max(200.0);
        st.disc_c = finite_temp(st.disc_c + rotor_net_w / rotor_capacity * dt);
        st.rim_c = finite_temp(st.rim_c + rim_net_w / rim_capacity * dt);
        st.efficiency = brake_efficiency(st.disc_c, cfg);
        st.natural_cooling_w_k = resolved.rotor_natural_w_k;
        st.speed_cooling_w_k = resolved.rotor_forced_at_reference_w_k * speed_factor;
        st.to_rim_heat_w = q_rotor_rim;

        BrakeToTireHeat {
            carcass_heat_w: q_rim_to_carcass,
            gas_heat_w: q_rim_to_gas,
        }
    }
}

fn axle_config(cfg: &BrakeThermalConfig, wheel: WheelIndex) -> &BrakeAxleThermalConfig {
    if wheel.is_front() {
        &cfg.front
    } else {
        &cfg.rear
    }
}

fn duct_config(cfg: &BrakeThermalConfig, wheel: WheelIndex) -> &BrakeDuctAxleConfig {
    if wheel.is_front() {
        &cfg.front_duct
    } else {
        &cfg.rear_duct
    }
}

pub fn evaluate_duct_flow(
    cfg: &BrakeDuctAxleConfig,
    air_density_kg_m3: f64,
    vehicle_speed_ms: f64,
) -> BrakeDuctFlowState {
    let rho = air_density_kg_m3.clamp(0.5, 1.6);
    let v = vehicle_speed_ms.abs();
    let opening = cfg.opening.clamp(0.0, 1.0);
    let effective_area = cfg.max_inlet_area_m2_per_wheel.max(0.0)
        * opening.powf(cfg.area_response_exponent.clamp(0.2, 3.0));
    let q_dynamic = 0.5 * rho * v * v;
    let delta_p = q_dynamic * cfg.pressure_recovery.clamp(0.0, 1.2);
    let coefficient = cfg.discharge_coefficient.clamp(0.0, 1.5);
    let ram_mass_flow = if effective_area > 0.0 && delta_p > 0.0 {
        coefficient * effective_area * (2.0 * rho * delta_p).sqrt()
    } else {
        0.0
    };
    let reference_speed = cfg.cooling_reference_speed_ms.max(0.1);
    let reference_dynamic_pressure = 0.5 * rho * reference_speed * reference_speed;
    let reference_delta_p = reference_dynamic_pressure * cfg.pressure_recovery.clamp(0.0, 1.2);
    let full_area = cfg.max_inlet_area_m2_per_wheel.max(0.0);
    let full_reference_mass_flow = if full_area > 0.0 && reference_delta_p > 0.0 {
        coefficient * full_area * (2.0 * rho * reference_delta_p).sqrt()
    } else {
        0.0
    };
    let normalized_reference_flow = if full_reference_mass_flow > 1e-9 {
        (ram_mass_flow / full_reference_mass_flow).clamp(0.0, 1.5)
    } else {
        0.0
    };
    // Keep the normalized reference flow for telemetry and calibration, but
    // do not use it as a cooling floor. Physical cooling must go to zero when
    // the actual duct mass flow goes to zero.
    let cooling_flow_ratio = normalized_reference_flow;
    let air_cp = cfg.air_specific_heat_j_kg_k.max(0.0);
    let capacity_rate_w_k = ram_mass_flow * air_cp;
    let ua = cfg.heat_exchanger_ua_w_k.max(0.0);
    let cooling_conductance_w_k = if capacity_rate_w_k > 1e-9 && ua > 0.0 {
        capacity_rate_w_k * (1.0 - (-ua / capacity_rate_w_k).exp())
    } else {
        0.0
    };
    BrakeDuctFlowState {
        effective_area_m2: effective_area,
        mass_flow_kg_s: finite_nonnegative(ram_mass_flow),
        cooling_flow_ratio: cooling_flow_ratio.clamp(0.0, 1.5),
        cooling_conductance_w_k: finite_nonnegative(cooling_conductance_w_k),
        drag_force_n: finite_nonnegative(
            q_dynamic * cfg.drag_coefficient.max(0.0) * effective_area,
        ),
    }
}

pub fn brake_efficiency(disc_temperature_c: f64, cfg: &BrakeThermalConfig) -> f64 {
    let t = if disc_temperature_c.is_finite() {
        disc_temperature_c
    } else {
        cfg.initial_temperature_c
    };
    let cold = cfg.initial_temperature_c;
    let opt_min = cfg.optimal_min_temperature_c;
    let opt_max = cfg.optimal_max_temperature_c.max(opt_min + 1.0);
    let fade_start = cfg.fade_start_temperature_c.max(opt_max);
    let critical = cfg.critical_temperature_c.max(fade_start + 1.0);
    if t <= cold {
        cfg.cold_efficiency.clamp(0.3, 1.0)
    } else if t < opt_min {
        lerp(
            cfg.cold_efficiency.clamp(0.3, 1.0),
            1.0,
            ((t - cold) / (opt_min - cold).max(1.0)).clamp(0.0, 1.0),
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
    if v.is_finite() {
        v.clamp(-40.0, 1400.0)
    } else {
        25.0
    }
}

#[inline]
fn finite_nonnegative(v: f64) -> f64 {
    if v.is_finite() {
        v.max(0.0)
    } else {
        0.0
    }
}

#[inline]
fn finite_signed(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        0.0
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
    fn zero_torque_generates_no_brake_power() {
        let cfg = BrakeThermalConfig::default();
        let mut system = BrakeThermalSystem::new(&cfg);
        let before = system.wheels[0].disc_c;
        let heat = system.step_after_braking(
            WheelIndex::FrontLeft,
            &cfg,
            BrakeThermalInput {
                wheel_spin_pre_rad_s: 100.0,
                wheel_spin_post_rad_s: 100.0,
                ambient_temperature_c: 25.0,
                tire_carcass_temperature_c: 25.0,
                tire_gas_temperature_c: 25.0,
                ..Default::default()
            },
            1.0 / 120.0,
        );
        assert_eq!(system.wheels[0].brake_power_w, 0.0);
        assert_eq!(heat.carcass_heat_w, 0.0);
        assert!(system.wheels[0].disc_c <= before);
    }

    #[test]
    fn brake_power_matches_actual_torque_times_omega() {
        let cfg = BrakeThermalConfig::default();
        let mut system = BrakeThermalSystem::new(&cfg);
        system.step_after_braking(
            WheelIndex::FrontLeft,
            &cfg,
            BrakeThermalInput {
                applied_brake_torque_nm: 250.0,
                wheel_spin_pre_rad_s: 100.0,
                wheel_spin_post_rad_s: 60.0,
                ambient_temperature_c: 25.0,
                tire_carcass_temperature_c: 25.0,
                tire_gas_temperature_c: 25.0,
                ..Default::default()
            },
            1.0 / 120.0,
        );
        assert!((system.wheels[0].brake_power_w - 20_000.0).abs() < 1e-9);
        assert!((system.wheels[0].brake_energy_j - (20_000.0 / 120.0)).abs() < 1e-9);
    }

    #[test]
    fn fade_reduces_efficiency_after_critical_temperature() {
        let cfg = BrakeThermalConfig::default();
        assert_eq!(brake_efficiency(600.0, &cfg), 1.0);
        assert!(brake_efficiency(1050.0, &cfg) < 1.0);
        assert!(brake_efficiency(1200.0, &cfg) <= cfg.minimum_fade_efficiency);
    }

    #[test]
    fn duct_opening_controls_shared_flow_and_drag() {
        let mut closed = BrakeDuctAxleConfig::default();
        closed.opening = 0.10;
        let mut open = closed;
        open.opening = 0.90;
        let a = evaluate_duct_flow(&closed, 1.225, 70.0);
        let b = evaluate_duct_flow(&open, 1.225, 70.0);
        assert!(b.effective_area_m2 > a.effective_area_m2);
        assert!(b.mass_flow_kg_s > a.mass_flow_kg_s);
        assert!(b.cooling_flow_ratio > a.cooling_flow_ratio);
        assert!(b.drag_force_n > a.drag_force_n);
    }

    #[test]
    fn duct_drag_scales_with_speed_squared() {
        let cfg = BrakeDuctAxleConfig::default();
        let slow = evaluate_duct_flow(&cfg, 1.225, 30.0);
        let fast = evaluate_duct_flow(&cfg, 1.225, 60.0);
        assert!(fast.mass_flow_kg_s > slow.mass_flow_kg_s * 1.9);
        assert!(fast.cooling_flow_ratio > slow.cooling_flow_ratio);
        assert!(fast.drag_force_n > slow.drag_force_n * 3.9);
    }

    #[test]
    fn closed_duct_has_no_dynamic_cooling_conductance() {
        let mut cfg = BrakeDuctAxleConfig::default();
        cfg.opening = 0.0;
        let state = evaluate_duct_flow(&cfg, 1.225, 200.0);
        assert_eq!(state.mass_flow_kg_s, 0.0);
        assert_eq!(state.cooling_conductance_w_k, 0.0);
        assert_eq!(state.cooling_flow_ratio, 0.0);
    }

    #[test]
    fn dynamic_cooling_is_limited_by_air_capacity_rate() {
        let mut cfg = BrakeDuctAxleConfig::default();
        cfg.opening = 1.0;
        let state = evaluate_duct_flow(&cfg, 1.225, cfg.cooling_reference_speed_ms);
        let capacity_rate = state.mass_flow_kg_s * cfg.air_specific_heat_j_kg_k;
        assert!(state.cooling_conductance_w_k > 0.0);
        assert!(state.cooling_conductance_w_k <= capacity_rate + 1e-9);
        assert!((state.cooling_conductance_w_k - 188.0).abs() < 2.0);
    }

    #[test]
    fn reference_speed_changes_diagnostic_ratio_not_physical_cooling() {
        let mut low_reference = BrakeDuctAxleConfig::default();
        low_reference.cooling_reference_speed_ms = 25.0;
        let mut high_reference = low_reference;
        high_reference.cooling_reference_speed_ms = 100.0;

        let low = evaluate_duct_flow(&low_reference, 1.225, 50.0);
        let high = evaluate_duct_flow(&high_reference, 1.225, 50.0);
        assert!((low.mass_flow_kg_s - high.mass_flow_kg_s).abs() < 1e-12);
        assert!((low.cooling_conductance_w_k - high.cooling_conductance_w_k).abs() < 1e-12);
        assert!(low.cooling_flow_ratio > high.cooling_flow_ratio);
    }

    #[test]
    fn compact_axle_resolves_lumped_rotor_from_physical_inputs() {
        let mut cfg = BrakeAxleThermalConfig::default();
        cfg.model = BrakeThermalModelKind::ScaledTwoNodeV1;
        cfg.rotor_material = BrakeRotorMaterial::CarbonCarbon;
        cfg.rotor_mass_kg = 1.35;
        cfg.rotor_outer_diameter_m = 0.278;
        cfg.rotor_inner_diameter_m = 0.105;
        cfg.rotor_ventilation = BrakeRotorVentilation::Vented;
        cfg.cooling_profile = BrakeCoolingProfile::OpenWheelDucted;
        cfg.installation_airflow_scale = 0.70;
        let resolved = cfg.resolve();
        assert!((resolved.rotor_capacity_j_k - 1500.0).abs() < 0.1);
        assert!(resolved.rotor_natural_w_k > 0.0);
        assert!(resolved.rotor_forced_at_reference_w_k > 0.0);
        assert!((resolved.rotor_heat_fraction - 0.96).abs() < 1e-9);
    }

    #[test]
    fn compact_axle_has_no_speed_cooling_at_standstill() {
        let mut cfg = BrakeThermalConfig::default();
        cfg.front.cooling_profile = BrakeCoolingProfile::OpenWheelDucted;
        let mut system = BrakeThermalSystem::new(&cfg);
        system.step_after_braking(
            WheelIndex::FrontLeft,
            &cfg,
            BrakeThermalInput {
                vehicle_speed_ms: 0.0,
                air_density_kg_m3: 1.225,
                ambient_temperature_c: 25.0,
                ..Default::default()
            },
            1.0 / 120.0,
        );
        assert_eq!(system.wheels[0].speed_cooling_w_k, 0.0);
        assert!(system.wheels[0].natural_cooling_w_k > 0.0);
    }

    #[test]
    fn hard_braking_heats_rotor_before_rim() {
        let cfg = BrakeThermalConfig::default();
        let mut system = BrakeThermalSystem::new(&cfg);
        for _ in 0..240 {
            system.step_after_braking(
                WheelIndex::FrontLeft,
                &cfg,
                BrakeThermalInput {
                    applied_brake_torque_nm: 1_200.0,
                    wheel_spin_pre_rad_s: 100.0,
                    wheel_spin_post_rad_s: 100.0,
                    vehicle_speed_ms: 40.0,
                    air_density_kg_m3: 1.225,
                    ambient_temperature_c: 25.0,
                    tire_carcass_temperature_c: 25.0,
                    tire_gas_temperature_c: 25.0,
                },
                1.0 / 120.0,
            );
        }
        let w = system.wheels[0];
        assert!(w.disc_c > w.rim_c);
        assert!(w.rim_c > 25.0);
        assert!(w.to_rim_heat_w > 0.0);
    }

    #[test]
    fn rotor_before_rim_lag_is_preserved_after_short_burst() {
        let cfg = BrakeThermalConfig::default();
        let mut system = BrakeThermalSystem::new(&cfg);
        for _ in 0..30 {
            system.step_after_braking(
                WheelIndex::FrontLeft,
                &cfg,
                BrakeThermalInput {
                    applied_brake_torque_nm: 1_200.0,
                    wheel_spin_pre_rad_s: 100.0,
                    wheel_spin_post_rad_s: 100.0,
                    vehicle_speed_ms: 30.0,
                    air_density_kg_m3: 1.225,
                    ambient_temperature_c: 25.0,
                    tire_carcass_temperature_c: 25.0,
                    tire_gas_temperature_c: 25.0,
                },
                1.0 / 120.0,
            );
        }
        let w = system.wheels[0];
        // Compact lumped rotor (1500 J/K) trails the rim by a clear margin on
        // this 0.25 s burst; the exact split with the old surface capacity
        // calibration was 20+, now roughly 18 K.
        assert!(w.disc_c - w.rim_c > 12.0, "disc={} rim={}", w.disc_c, w.rim_c);
    }

    #[test]
    fn single_stop_energy_is_conserved_through_rotor_and_rim() {
        let mut cfg = BrakeThermalConfig::default();
        cfg.front.installation_airflow_scale = 0.0;
        cfg.front.rim_base_air_w_k = 0.0;
        cfg.front.rim_to_tire_carcass_w_k = 0.0;
        cfg.front.rim_to_tire_gas_w_k = 0.0;
        cfg.front_duct.opening = 0.0;
        let mut system = BrakeThermalSystem::new(&cfg);
        let ticks = 180;
        let dt = 1.0 / 120.0;
        for _ in 0..ticks {
            system.step_after_braking(
                WheelIndex::FrontLeft,
                &cfg,
                BrakeThermalInput {
                    applied_brake_torque_nm: 1_000.0,
                    wheel_spin_pre_rad_s: 80.0,
                    wheel_spin_post_rad_s: 80.0,
                    vehicle_speed_ms: 0.0,
                    air_density_kg_m3: 1.225,
                    ambient_temperature_c: 25.0,
                    tire_carcass_temperature_c: 25.0,
                    tire_gas_temperature_c: 25.0,
                },
                dt,
            );
        }
        let w = system.wheels[0];
        let rotor_capacity = cfg.front.resolve().rotor_capacity_j_k;
        let stored = (w.disc_c - 25.0) * rotor_capacity + (w.rim_c - 25.0) * cfg.front.rim_heat_capacity_j_k;
        let generated = 1_000.0 * 80.0 * cfg.braking_heat_fraction * (ticks as f64) * dt;
        assert!((stored / generated - 1.0).abs() < 0.05, "stored={stored} generated={generated}");
    }

    #[test]
    fn rotor_cooling_fraction_splits_duct_airflow() {
        fn run(fraction: f64) -> f64 {
            let mut cfg = BrakeThermalConfig::default();
            cfg.front_duct.opening = 0.80;
            cfg.front_duct.rotor_cooling_fraction = fraction;
            let mut system = BrakeThermalSystem::new(&cfg);
            system.wheels[0].disc_c = 450.0;
            system.wheels[0].rim_c = 100.0;
            for _ in 0..120 {
                system.step_after_braking(
                    WheelIndex::FrontLeft,
                    &cfg,
                    BrakeThermalInput {
                        vehicle_speed_ms: 55.0,
                        air_density_kg_m3: 1.225,
                        ambient_temperature_c: 25.0,
                        tire_carcass_temperature_c: 25.0,
                        tire_gas_temperature_c: 25.0,
                        ..Default::default()
                    },
                    1.0 / 120.0,
                );
            }
            system.wheels[0].disc_c
        }
        let rotor_first = run(1.0);
        let rim_first = run(0.0);
        assert!(rotor_first < rim_first);
    }

    #[test]
    fn rim_heat_enters_tire_carcass_and_gas_nodes() {
        let brake_cfg = BrakeThermalConfig::default();
        let mut brakes = BrakeThermalSystem::new(&brake_cfg);
        brakes.wheels[0].rim_c = 200.0;
        let tire_pressure = crate::tire_thermals::TirePressureConfig::default();
        let tire_cfg = crate::tire_thermals::TireThermalConfig::default();
        let environment = crate::tire_thermals::TireEnvironment::fallback(&tire_cfg);
        let mut tires = crate::tire_thermals::TireThermalSystem::new(&tire_pressure, &tire_cfg);

        for _ in 0..120 {
            let heat = brakes.step_after_braking(
                WheelIndex::FrontLeft,
                &brake_cfg,
                BrakeThermalInput {
                    ambient_temperature_c: 25.0,
                    tire_carcass_temperature_c: tires.wheels[0].carcass_c,
                    tire_gas_temperature_c: tires.wheels[0].gas_c,
                    ..Default::default()
                },
                1.0 / 120.0,
            );
            tires.step_after_forces(
                WheelIndex::FrontLeft,
                &tire_pressure,
                &tire_cfg,
                environment,
                crate::tire_thermals::TireThermalInput {
                    external_carcass_heat_w: heat.carcass_heat_w,
                    external_gas_heat_w: heat.gas_heat_w,
                    ..Default::default()
                },
                1.0 / 120.0,
            );
        }
        assert!(tires.wheels[0].carcass_c > 25.0);
        assert!(tires.wheels[0].gas_c > 25.0);
    }

    #[test]
    fn repeated_braking_produces_measurable_tire_heat_soak() {
        fn run(
            coupling_scale: f64,
        ) -> (
            WheelBrakeThermalState,
            crate::tire_thermals::WheelThermalState,
        ) {
            let mut brake_cfg = BrakeThermalConfig::default();
            brake_cfg.front_duct.opening = 0.20;
            brake_cfg.front.rim_to_tire_carcass_w_k *= coupling_scale;
            brake_cfg.front.rim_to_tire_gas_w_k *= coupling_scale;
            let mut brakes = BrakeThermalSystem::new(&brake_cfg);
            let tire_pressure = crate::tire_thermals::TirePressureConfig::default();
            let tire_cfg = crate::tire_thermals::TireThermalConfig::default();
            let environment = crate::tire_thermals::TireEnvironment::fallback(&tire_cfg);
            let mut tires = crate::tire_thermals::TireThermalSystem::new(&tire_pressure, &tire_cfg);
            let dt = 1.0 / 120.0;

            // Eighteen representative braking zones over a three-minute run.
            for tick in 0..(180 * 120) {
                let cycle_s = (tick as f64 * dt) % 10.0;
                let braking = cycle_s < 2.0;
                let heat = brakes.step_after_braking(
                    WheelIndex::FrontLeft,
                    &brake_cfg,
                    BrakeThermalInput {
                        applied_brake_torque_nm: if braking { 900.0 } else { 0.0 },
                        wheel_spin_pre_rad_s: if braking { 100.0 } else { 0.0 },
                        wheel_spin_post_rad_s: if braking { 100.0 } else { 0.0 },
                        vehicle_speed_ms: 45.0,
                        air_density_kg_m3: 1.225,
                        ambient_temperature_c: 25.0,
                        tire_carcass_temperature_c: tires.wheels[0].carcass_c,
                        tire_gas_temperature_c: tires.wheels[0].gas_c,
                    },
                    dt,
                );
                tires.step_after_forces(
                    WheelIndex::FrontLeft,
                    &tire_pressure,
                    &tire_cfg,
                    environment,
                    crate::tire_thermals::TireThermalInput {
                        vehicle_speed_ms: 45.0,
                        external_carcass_heat_w: heat.carcass_heat_w,
                        external_gas_heat_w: heat.gas_heat_w,
                        ..Default::default()
                    },
                    dt,
                );
            }
            (brakes.wheels[0], tires.wheels[0])
        }

        let (brake_on, tire_on) = run(1.0);
        let (_, tire_off) = run(0.0);
        assert!(brake_on.disc_c > brake_on.rim_c);
        assert!(tire_on.carcass_c - tire_off.carcass_c > 0.40);
        assert!(tire_on.gas_c - tire_off.gas_c > 1.50);
    }
}
