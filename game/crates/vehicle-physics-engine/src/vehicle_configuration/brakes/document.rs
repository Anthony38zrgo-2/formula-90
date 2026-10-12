use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonBrakes {
    #[serde(default = "default_brake_bias")]
    pub(crate) front_brake_bias: f64,
    #[serde(default = "default_max_brake")]
    pub(crate) max_brake_torque: f64,
    #[serde(default)]
    pub(crate) enable_abs: bool,
    #[serde(default = "default_abs_pulse")]
    pub(crate) abs_pulse_time: f64,
    #[serde(default = "default_abs_thresh")]
    pub(crate) abs_spin_diff_threshold: f64,
    #[serde(default)]
    pub(crate) wheel_lock_blend_spin_rad_s: Option<f64>,
    #[serde(default)]
    pub(crate) wheel_lock_stick_tau_s: Option<f64>,
    #[serde(default)]
    pub(crate) thermal: Option<JsonBrakeThermal>,
}
impl Default for JsonBrakes {
    fn default() -> Self {
        Self {
            front_brake_bias: default_brake_bias(),
            max_brake_torque: default_max_brake(),
            enable_abs: false,
            abs_pulse_time: default_abs_pulse(),
            abs_spin_diff_threshold: default_abs_thresh(),
            wheel_lock_blend_spin_rad_s: None,
            wheel_lock_stick_tau_s: None,
            thermal: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonBrakeThermal {
    pub(crate) initial_temperature_c: Option<f64>,
    pub(crate) optimal_min_temperature_c: Option<f64>,
    pub(crate) optimal_max_temperature_c: Option<f64>,
    pub(crate) fade_start_temperature_c: Option<f64>,
    pub(crate) critical_temperature_c: Option<f64>,
    pub(crate) cold_efficiency: Option<f64>,
    pub(crate) minimum_fade_efficiency: Option<f64>,
    pub(crate) braking_heat_fraction: Option<f64>,
    // Deprecated v2: the caliper node is part of the compact lumped rotor;
    // accepted for schema-v2 compatibility and ignored.
    pub(crate) direct_caliper_heat_fraction: Option<f64>,
    pub(crate) front: Option<JsonBrakeAxleThermal>,
    pub(crate) rear: Option<JsonBrakeAxleThermal>,
    pub(crate) front_duct: Option<JsonBrakeDuctAxle>,
    pub(crate) rear_duct: Option<JsonBrakeDuctAxle>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonBrakeAxleThermal {
    pub(crate) model: Option<BrakeThermalModelKind>,
    pub(crate) rotor_material: Option<BrakeRotorMaterial>,
    pub(crate) rotor_mass_kg: Option<f64>,
    pub(crate) rotor_outer_diameter_m: Option<f64>,
    pub(crate) rotor_inner_diameter_m: Option<f64>,
    pub(crate) rotor_radiation_emissivity: Option<f64>,
    pub(crate) rotor_radiation_ambient_view_factor: Option<f64>,
    pub(crate) rotor_ventilation: Option<BrakeRotorVentilation>,
    pub(crate) cooling_profile: Option<BrakeCoolingProfile>,
    pub(crate) installation_airflow_scale: Option<f64>,
    pub(crate) thermal_mass_scale: Option<f64>,
    pub(crate) rotor_to_rim_w_k: Option<f64>,
    pub(crate) rim_heat_capacity_j_k: Option<f64>,
    pub(crate) rim_base_air_w_k: Option<f64>,
    pub(crate) rim_to_tire_carcass_w_k: Option<f64>,
    pub(crate) rim_to_tire_gas_w_k: Option<f64>,
    // Deprecated v2 fields: sub-model nodes (surface/bulk/caliper/hub) and
    // explicit conductances are parsed for schema-v2 compatibility and mapped
    // into the compact model through physical lumping.
    pub(crate) surface_bulk_response_scale: Option<f64>,
    pub(crate) disc_heat_capacity_j_k: Option<f64>,
    pub(crate) disc_surface_heat_capacity_j_k: Option<f64>,
    pub(crate) disc_bulk_heat_capacity_j_k: Option<f64>,
    pub(crate) disc_surface_to_bulk_w_k: Option<f64>,
    pub(crate) disc_surface_base_air_w_k: Option<f64>,
    pub(crate) caliper_heat_capacity_j_k: Option<f64>,
    pub(crate) hub_heat_capacity_j_k: Option<f64>,
    pub(crate) disc_to_caliper_w_k: Option<f64>,
    pub(crate) disc_to_hub_w_k: Option<f64>,
    pub(crate) disc_to_rim_radiation_w_k: Option<f64>,
    pub(crate) hub_to_rim_w_k: Option<f64>,
    pub(crate) disc_base_air_w_k: Option<f64>,
    pub(crate) caliper_base_air_w_k: Option<f64>,
    pub(crate) hub_base_air_w_k: Option<f64>,
    pub(crate) disc_flow_cooling_gain_w_k: Option<f64>,
    pub(crate) caliper_flow_cooling_gain_w_k: Option<f64>,
    pub(crate) hub_flow_cooling_gain_w_k: Option<f64>,
    pub(crate) rim_flow_cooling_gain_w_k: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonBrakeDuctAxle {
    pub(crate) opening: Option<f64>,
    pub(crate) max_inlet_area_m2_per_wheel: Option<f64>,
    pub(crate) discharge_coefficient: Option<f64>,
    pub(crate) pressure_recovery: Option<f64>,
    pub(crate) cooling_reference_speed_ms: Option<f64>,
    pub(crate) drag_coefficient: Option<f64>,
    pub(crate) area_response_exponent: Option<f64>,
    pub(crate) cooling_flow_exponent: Option<f64>,
    pub(crate) minimum_cooling_flow_ratio: Option<f64>,
    pub(crate) air_specific_heat_j_kg_k: Option<f64>,
    pub(crate) heat_exchanger_ua_w_k: Option<f64>,
    pub(crate) rotor_cooling_fraction: Option<f64>,
    // Deprecated v2: per-node cooling weights are parsed for schema-v2
    // compatibility and mapped into the single rotor_cooling_fraction.
    pub(crate) disc_cooling_weight: Option<f64>,
    pub(crate) caliper_cooling_weight: Option<f64>,
    pub(crate) hub_cooling_weight: Option<f64>,
    pub(crate) rim_cooling_weight: Option<f64>,
}

pub(crate) fn default_brake_bias() -> f64 {
    0.57
}
pub(crate) fn default_max_brake() -> f64 {
    2800.0
}
pub(crate) fn default_abs_pulse() -> f64 {
    0.03
}
pub(crate) fn default_abs_thresh() -> f64 {
    12.0
}
