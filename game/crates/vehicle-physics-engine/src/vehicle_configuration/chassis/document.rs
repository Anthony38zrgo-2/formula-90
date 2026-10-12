use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonChassis {
    #[serde(default = "default_vehicle_name")]
    pub(crate) vehicle_name: String,
    #[serde(default = "default_mass")]
    pub(crate) vehicle_mass: f64,
    pub(crate) vehicle_mass_excludes_wheel_assemblies: bool,
    #[serde(default = "default_front_weight_dist")]
    pub(crate) front_weight_distribution: f64,
    #[serde(default = "default_cog_height")]
    pub(crate) center_of_gravity_height_offset: f64,
    #[serde(default)]
    pub(crate) inertia_multipliers: JsonVec3,
}
impl Default for JsonChassis {
    fn default() -> Self {
        Self {
            vehicle_name: default_vehicle_name(),
            vehicle_mass: default_mass(),
            vehicle_mass_excludes_wheel_assemblies: false,
            front_weight_distribution: default_front_weight_dist(),
            center_of_gravity_height_offset: default_cog_height(),
            inertia_multipliers: JsonVec3::default(),
        }
    }
}
pub(crate) fn default_vehicle_name() -> String {
    "F1 1994 (V10)".to_string()
}
pub(crate) fn default_mass() -> f64 {
    575.0
}
pub(crate) fn default_front_weight_dist() -> f64 {
    0.45
}
pub(crate) fn default_cog_height() -> f64 {
    -0.12
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonVec3 {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
}
impl Default for JsonVec3 {
    fn default() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonGeometry {
    #[serde(default = "default_wheelbase")]
    pub(crate) wheelbase: f64,
    #[serde(default = "default_front_track")]
    pub(crate) front_track: f64,
    #[serde(default = "default_rear_track")]
    pub(crate) rear_track: f64,
    #[serde(default)]
    pub(crate) wheel_hubs: Option<HashMap<String, JsonVec3>>,
}
impl Default for JsonGeometry {
    fn default() -> Self {
        Self {
            wheelbase: default_wheelbase(),
            front_track: default_front_track(),
            rear_track: default_rear_track(),
            wheel_hubs: None,
        }
    }
}
pub(crate) fn default_wheelbase() -> f64 {
    2.92065
}
pub(crate) fn default_front_track() -> f64 {
    1.5925
}
pub(crate) fn default_rear_track() -> f64 {
    1.5246
}
