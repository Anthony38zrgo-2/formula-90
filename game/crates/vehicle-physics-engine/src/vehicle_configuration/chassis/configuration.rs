use serde::{Deserialize, Serialize};

use super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChassisConfiguration {
    pub vehicle_name: String,
    pub vehicle_mass: f64,
    pub vehicle_mass_excludes_wheel_assemblies: bool,
    pub front_weight_distribution: f64,
    pub center_of_gravity_height_offset: f64,
    pub inertia_multipliers: Vec3,
    pub wheelbase: f64,
    pub front_track: f64,
    pub rear_track: f64,
    #[serde(default)]
    pub physical_mass_inventory: Option<crate::vehicle_mass_inventory::VehicleMassInventory>,
}
