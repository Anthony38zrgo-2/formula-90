use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonSuspension {
    #[serde(default)]
    pub(crate) front: JsonSuspensionAxle,
    #[serde(default)]
    pub(crate) use_authored_wheel_centers: bool,
    #[serde(default)]
    pub(crate) rear: JsonSuspensionAxle,
    #[serde(default = "default_tri_ray")]
    pub(crate) tri_ray_spacing_ratio: f64,
    /// Parser-only visual suspension geometry (per-corner hardpoints for
    /// UPPER_WISHBONE / LOWER_WISHBONE / PUSHROD / TRACKROD / UPRIGHT / ROCKER /
    /// DRIVESHAFT). Consumed exclusively by the GDScript visual kinematic layer;
    /// it never reaches runtime forces and is not part of the force model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) geometry: Option<serde_json::Value>,
    /// SUS-GEO-02 model selector: "legacy_1dof" (default) or "geometric".
    #[serde(default = "default_suspension_model_name")]
    pub(crate) model: String,
    /// SUS-GEO-02 model version. Must be 1.
    #[serde(default = "default_suspension_model_version")]
    pub(crate) model_version: u32,
    /// SUS-GEO-02 physical geometry + elements. Required for geometric.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) geometry_physical: Option<serde_json::Value>,
    /// Simplified-backend virtual unsprung mass scale over the resolved corner mass.
    #[serde(default = "default_virtual_unsprung_mass_multiplier")]
    pub(crate) virtual_unsprung_mass_multiplier: f64,
}
impl Default for JsonSuspension {
    fn default() -> Self {
        Self {
            front: JsonSuspensionAxle::default(),
            use_authored_wheel_centers: false,
            rear: JsonSuspensionAxle::default(),
            tri_ray_spacing_ratio: default_tri_ray(),
            geometry: None,
            model: default_suspension_model_name(),
            model_version: default_suspension_model_version(),
            geometry_physical: None,
            virtual_unsprung_mass_multiplier: default_virtual_unsprung_mass_multiplier(),
        }
    }
}
pub(crate) fn default_virtual_unsprung_mass_multiplier() -> f64 {
    1.0
}
pub(crate) fn default_suspension_model_name() -> String {
    "legacy_1dof".to_string()
}
pub(crate) fn default_tri_ray() -> f64 {
    0.40
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonSuspensionAxle {
    #[serde(default)]
    pub(crate) spring_length: f64,
    #[serde(default)]
    pub(crate) resting_ratio: f64,
    #[serde(default = "default_damping")]
    pub(crate) damping_ratio: f64,
    #[serde(default = "default_bump_mult")]
    pub(crate) bump_damp_multiplier: f64,
    #[serde(default = "default_rebound_mult")]
    pub(crate) rebound_damp_multiplier: f64,
    #[serde(default)]
    pub(crate) arb_ratio: f64,
    #[serde(default)]
    pub(crate) toe: f64,
    #[serde(default)]
    pub(crate) camber: f64,
    #[serde(default = "default_bump_stop")]
    pub(crate) bump_stop_multiplier: f64,
    /// Schema v3: camber variation per meter of suspension compression (parser-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) camber_gain_rad_per_m: Option<f64>,
    /// Schema v3: toe variation per meter of suspension compression (parser-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) toe_gain_rad_per_m: Option<f64>,
}
impl Default for JsonSuspensionAxle {
    fn default() -> Self {
        Self {
            spring_length: 0.25,
            resting_ratio: 0.28,
            damping_ratio: 0.80,
            bump_damp_multiplier: 1.3,
            rebound_damp_multiplier: 1.1,
            arb_ratio: 0.20,
            toe: 0.0017453,
            camber: -0.0174533,
            bump_stop_multiplier: 2.2,
            camber_gain_rad_per_m: None,
            toe_gain_rad_per_m: None,
        }
    }
}
pub(crate) fn default_damping() -> f64 {
    0.80
}
pub(crate) fn default_bump_mult() -> f64 {
    1.3
}
pub(crate) fn default_rebound_mult() -> f64 {
    1.1
}
pub(crate) fn default_bump_stop() -> f64 {
    2.2
}
