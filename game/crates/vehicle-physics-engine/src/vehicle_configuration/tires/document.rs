use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonTires {
    #[serde(default)]
    pub(crate) front: JsonTireAxle,
    #[serde(default)]
    pub(crate) rear: JsonTireAxle,
    #[serde(default = "default_contact_patch")]
    pub(crate) contact_patch: f64,
    /// DEPRECATED (TIRE-600): parsed only so legacy schema v2 profiles load; never
    /// reaches runtime config and never influences forces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) braking_grip_multiplier: Option<f64>,
    #[serde(default = "default_airborne_decay")]
    pub(crate) airborne_spin_decay_torque: f64,
    #[serde(default)]
    pub(crate) surfaces: HashMap<String, JsonSurfaceEntry>,
    /// Optional pressure section (gauge kPa per wheel). Absent -> canonical defaults.
    #[serde(default)]
    pub(crate) pressure: Option<JsonTirePressure>,
    /// Optional thermal section (5-node thermal model tuning). Absent -> canonical defaults.
    #[serde(default)]
    pub(crate) thermal: Option<JsonTireThermal>,
    #[serde(default)]
    pub(crate) wear: Option<JsonTireWear>,
    /// Schema v3: per-axle steady tire force curve (TIRE-200). Absent -> parser-only defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) force_model: Option<JsonTireForceModel>,
    /// Schema v3: consolidated pressure mechanics sensitivities (THERM-700). Absent -> parser-only defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_mechanics: Option<JsonPressureMechanics>,
    /// Schema v3: temporal filter for per-wheel surface values (GRIP-06).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) surface_transition_tau_s: Option<f64>,
    /// Radial carcass damping ratio; absent keeps the 0.28 calibration.
    #[serde(default = "default_radial_damping_ratio")]
    pub(crate) radial_damping_ratio: f64,
}
impl Default for JsonTires {
    fn default() -> Self {
        Self {
            front: JsonTireAxle::default(),
            rear: JsonTireAxle::default(),
            contact_patch: default_contact_patch(),
            braking_grip_multiplier: None,
            airborne_spin_decay_torque: default_airborne_decay(),
            surfaces: HashMap::new(),
            pressure: None,
            thermal: None,
            wear: None,
            force_model: None,
            pressure_mechanics: None,
            surface_transition_tau_s: None,
            radial_damping_ratio: default_radial_damping_ratio(),
        }
    }
}
pub(crate) fn default_radial_damping_ratio() -> f64 {
    0.28
}

// ── Schema v3: tire force model + pressure mechanics ────────────────────────────
// Parsed and validated so v3 profiles carry TIRE-200/THERM-700 data. tire.rs
// consumes `tires.force_model` through `VehicleConfig.front_tire_force` /
// `rear_tire_force`; absent sections fall back to the parser default profile.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonTireForceModel {
    pub(crate) front: JsonTireForceProfile,
    pub(crate) rear: JsonTireForceProfile,
}
impl Default for JsonTireForceModel {
    fn default() -> Self {
        Self {
            front: JsonTireForceProfile::default(),
            rear: JsonTireForceProfile::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonTireForceProfile {
    #[serde(default = "default_peak_slip_angle_rad")]
    pub(crate) lateral_peak_slip_angle_rad: f64,
    #[serde(default = "default_peak_slip_ratio")]
    pub(crate) longitudinal_peak_slip_ratio: f64,
    #[serde(default = "default_slide_mu_ratio")]
    pub(crate) lateral_slide_mu_ratio: f64,
    #[serde(default = "default_slide_mu_ratio")]
    pub(crate) longitudinal_slide_mu_ratio: f64,
    #[serde(default = "default_budget_blend_width")]
    pub(crate) budget_blend_width: f64,
    #[serde(default = "default_falloff_sharpness")]
    pub(crate) falloff_sharpness: f64,
    #[serde(default = "default_slip_tau_s")]
    pub(crate) slip_loss_tau_s: f64,
    #[serde(default = "default_slip_tau_s")]
    pub(crate) slip_recovery_tau_s: f64,
    #[serde(default = "default_peak_migration")]
    pub(crate) combined_lateral_peak_migration: f64,
    #[serde(default = "default_peak_migration")]
    pub(crate) combined_longitudinal_peak_migration: f64,
    #[serde(default = "default_min_relaxation_tau")]
    pub(crate) min_relaxation_tau_s: f64,
    #[serde(default = "default_relaxation_recovery_scale")]
    pub(crate) relaxation_recovery_scale: f64,
    #[serde(default = "default_contact_ramp_tau")]
    pub(crate) contact_ramp_tau_s: f64,
}
impl Default for JsonTireForceProfile {
    fn default() -> Self {
        Self {
            lateral_peak_slip_angle_rad: default_peak_slip_angle_rad(),
            longitudinal_peak_slip_ratio: default_peak_slip_ratio(),
            lateral_slide_mu_ratio: default_slide_mu_ratio(),
            longitudinal_slide_mu_ratio: default_slide_mu_ratio(),
            budget_blend_width: default_budget_blend_width(),
            falloff_sharpness: default_falloff_sharpness(),
            slip_loss_tau_s: default_slip_tau_s(),
            slip_recovery_tau_s: default_slip_tau_s(),
            combined_lateral_peak_migration: default_peak_migration(),
            combined_longitudinal_peak_migration: default_peak_migration(),
            min_relaxation_tau_s: default_min_relaxation_tau(),
            relaxation_recovery_scale: default_relaxation_recovery_scale(),
            contact_ramp_tau_s: default_contact_ramp_tau(),
        }
    }
}
pub(crate) fn default_peak_slip_angle_rad() -> f64 {
    0.10
}
pub(crate) fn default_peak_slip_ratio() -> f64 {
    0.12
}
pub(crate) fn default_slide_mu_ratio() -> f64 {
    // 1.0 = no post-peak drop (matches current saturating tanh baseline).
    // TIRE-200 introduces an explicit post-peak decay via profile values.
    1.0
}
pub(crate) fn default_budget_blend_width() -> f64 {
    // 0.0 = legacy hard clip. GRIP-01 opts in through the active profile.
    0.0
}
pub(crate) fn default_falloff_sharpness() -> f64 {
    // 2.0 = legacy internal post-peak falloff constant.
    2.0
}
pub(crate) fn default_slip_tau_s() -> f64 {
    // 0.0 = legacy instant sliding memory. GRIP-02 opts in through the profile.
    0.0
}
pub(crate) fn default_peak_migration() -> f64 {
    // 0.0 = independent axes. GRIP-04 opts in through the profile.
    0.0
}
pub(crate) fn default_min_relaxation_tau() -> f64 {
    // 0.0 = pure distance-based tau. GRIP-05 opts in through the profile.
    0.0
}
pub(crate) fn default_relaxation_recovery_scale() -> f64 {
    // 1.0 = symmetric relaxation (legacy).
    1.0
}
pub(crate) fn default_contact_ramp_tau() -> f64 {
    // 0.0 = legacy hard contact switch. GRIP-06 opts in through the profile.
    0.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonPressureMechanics {
    #[serde(default)]
    pub(crate) compliance_sensitivity: f64,
    #[serde(default)]
    pub(crate) response_sensitivity: f64,
    #[serde(default)]
    pub(crate) rolling_resistance_sensitivity: f64,
}
impl Default for JsonPressureMechanics {
    fn default() -> Self {
        Self {
            compliance_sensitivity: 0.0,
            response_sensitivity: 0.0,
            rolling_resistance_sensitivity: 0.0,
        }
    }
}
pub(crate) fn default_contact_patch() -> f64 {
    0.21
}
pub(crate) fn default_airborne_decay() -> f64 {
    2.0
}

// ── Tire pressure + thermal (schema extension) ─────────────────────────────────
// These JSON sections are parsed explicitly (deny_unknown_fields stays active on
// JsonTires and every nested struct). Optional fields mean "keep the canonical
// module default"; per-wheel maps FL/FR/RL/RR override only the listed wheels.

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonTirePressure {
    #[serde(default)]
    pub(crate) units: Option<String>,
    /// Gauge kPa at the cold setup temperature, keys FL/FR/RL/RR.
    #[serde(default)]
    pub(crate) cold_kpa_gauge: Option<HashMap<String, f64>>,
    /// Mechanical reference pressure near normal hot running, keys FL/FR/RL/RR.
    #[serde(default)]
    pub(crate) reference_hot_kpa_gauge: Option<HashMap<String, f64>>,
    #[serde(default)]
    pub(crate) reference_temperature_c: Option<f64>,
    #[serde(default)]
    pub(crate) atmospheric_pressure_kpa: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonTireThermal {
    #[serde(default)]
    pub(crate) initial_temperature_c: Option<f64>,
    #[serde(default)]
    pub(crate) ambient_fallback_c: Option<f64>,
    #[serde(default)]
    pub(crate) track_fallback_c: Option<f64>,
    #[serde(default)]
    pub(crate) optimal_tread_temperature_c: Option<f64>,
    #[serde(default)]
    pub(crate) optimal_tread_window_c: Option<f64>,
    #[serde(default)]
    pub(crate) overheat_tread_temperature_c: Option<f64>,
    #[serde(default)]
    pub(crate) optimal_carcass_temperature_c: Option<f64>,
    #[serde(default)]
    pub(crate) tread_zone_heat_capacity_j_k: Option<f64>,
    #[serde(default)]
    pub(crate) carcass_heat_capacity_j_k: Option<f64>,
    #[serde(default)]
    pub(crate) gas_heat_capacity_j_k: Option<f64>,
    #[serde(default)]
    pub(crate) tread_to_carcass_w_k: Option<f64>,
    #[serde(default)]
    pub(crate) carcass_to_gas_w_k: Option<f64>,
    #[serde(default)]
    pub(crate) tread_to_air_w_k: Option<f64>,
    #[serde(default)]
    pub(crate) carcass_to_air_w_k: Option<f64>,
    #[serde(default)]
    pub(crate) road_conductance_w_k: Option<f64>,
    #[serde(default)]
    pub(crate) lateral_tread_conductance_w_k: Option<f64>,
    #[serde(default)]
    pub(crate) slip_heat_efficiency: Option<f64>,
    #[serde(default)]
    pub(crate) carcass_hysteresis_efficiency: Option<f64>,
    /// DEPRECATED (THERM-700): legacy schema v2 keys parsed for compatibility only.
    /// Each is ignored; the three `tires.pressure_mechanics` sensitivities are the
    /// only pressure knobs. Never emitted for new profiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_stiffness_exponent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_damping_exponent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_max_deflection_exponent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_patch_exponent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_relaxation_exponent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_rr_exponent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_force_stiffness_exponent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pressure_trail_exponent: Option<f64>,
    #[serde(default)]
    pub(crate) volume_deflection_gain: Option<f64>,
    #[serde(default)]
    pub(crate) minimum_pressure_kpa_gauge: Option<f64>,
    #[serde(default)]
    pub(crate) maximum_pressure_kpa_gauge: Option<f64>,
    /// New schema: independent front/rear thermal profiles. The legacy scalar
    /// fields above remain a shared fallback for older vehicle JSON files.
    #[serde(default)]
    pub(crate) front: Option<Box<JsonTireThermal>>,
    #[serde(default)]
    pub(crate) rear: Option<Box<JsonTireThermal>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonTireWear {
    #[serde(default)]
    pub(crate) initial_tread_depth_millimeters: Option<f64>,
    #[serde(default)]
    pub(crate) minimum_tread_depth_millimeters: Option<f64>,
    #[serde(default)]
    pub(crate) tread_wear_per_megajoule_millimeters: Option<f64>,
    #[serde(default)]
    pub(crate) cold_tread_temperature_celsius: Option<f64>,
    #[serde(default)]
    pub(crate) optimal_tread_temperature_celsius: Option<f64>,
    #[serde(default)]
    pub(crate) overheat_tread_temperature_celsius: Option<f64>,
    #[serde(default)]
    pub(crate) extreme_tread_temperature_celsius: Option<f64>,
    #[serde(default)]
    pub(crate) cold_temperature_wear_multiplier: Option<f64>,
    #[serde(default)]
    pub(crate) overheat_temperature_wear_multiplier: Option<f64>,
    #[serde(default)]
    pub(crate) extreme_temperature_wear_multiplier: Option<f64>,
    #[serde(default)]
    pub(crate) friction_reference_coefficient: Option<f64>,
    #[serde(default)]
    pub(crate) friction_sensitivity_exponent: Option<f64>,
    #[serde(default)]
    pub(crate) inner_shoulder_wear_multiplier: Option<f64>,
    #[serde(default)]
    pub(crate) center_wear_multiplier: Option<f64>,
    #[serde(default)]
    pub(crate) outer_shoulder_wear_multiplier: Option<f64>,
    #[serde(default)]
    pub(crate) cliff_start_wear_fraction: Option<f64>,
    #[serde(default)]
    pub(crate) end_of_life_grip_scale: Option<f64>,
    #[serde(default)]
    pub(crate) progressive_grip_loss_fraction: Option<f64>,
    #[serde(default)]
    pub(crate) front: Option<Box<JsonTireWear>>,
    #[serde(default)]
    pub(crate) rear: Option<Box<JsonTireWear>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonTireAxle {
    #[serde(default = "default_tire_radius_front")]
    pub(crate) radius: f64,
    #[serde(default = "default_tire_width_front")]
    pub(crate) width: f64,
    #[serde(default = "default_wheel_mass_front")]
    pub(crate) wheel_mass: f64,
    #[serde(default)]
    pub(crate) contact_patch: Option<f64>,
    /// DEPRECATED (TIRE-600): parsed only so legacy schema v2 profiles load; never
    /// reaches runtime config and never influences forces.
    #[serde(default)]
    pub(crate) braking_grip_multiplier: Option<f64>,
    #[serde(default)]
    pub(crate) airborne_spin_decay_torque: Option<f64>,
}
impl Default for JsonTireAxle {
    fn default() -> Self {
        Self {
            radius: default_tire_radius_front(),
            width: default_tire_width_front(),
            wheel_mass: default_wheel_mass_front(),
            contact_patch: None,
            braking_grip_multiplier: None,
            airborne_spin_decay_torque: None,
        }
    }
}
pub(crate) fn default_tire_radius_front() -> f64 {
    0.31695
}
pub(crate) fn default_tire_width_front() -> f64 {
    0.30030
}
pub(crate) fn default_wheel_mass_front() -> f64 {
    12.0
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonSurfaceEntry {
    pub(crate) friction: f64,
    pub(crate) stiffness: f64,
    pub(crate) rolling_resistance: f64,
    /// DEPRECATED (TIRE-500): parsed only so legacy schema v2 profiles load; never
    /// reaches runtime config and never influences lateral force.
    #[serde(default = "default_lat_assist")]
    pub(crate) lateral_grip_assist: f64,
    #[serde(default = "default_long_ratio")]
    pub(crate) longitudinal_grip_ratio: f64,
}
pub(crate) fn default_lat_assist() -> f64 {
    0.05
}
pub(crate) fn default_long_ratio() -> f64 {
    0.5
}
