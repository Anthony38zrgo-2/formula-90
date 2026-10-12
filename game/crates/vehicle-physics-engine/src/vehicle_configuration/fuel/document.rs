use super::*;
use super::super::chassis::document::JsonVec3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JsonFuel {
    pub(crate) capacity_kg: f64,
    pub(crate) initial_kg: f64,
    pub(crate) tank_position_local_m: JsonVec3,
    pub(crate) brake_specific_consumption_kg_per_kwh: f64,
    pub(crate) idle_consumption_kg_per_hour: f64,
    pub(crate) estimated_lap_consumption_kg: f64,
    pub(crate) reference_lap_time_s: f64,
}

impl Default for JsonFuel {
    fn default() -> Self {
        Self {
            capacity_kg: 0.0,
            initial_kg: 0.0,
            tank_position_local_m: JsonVec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            brake_specific_consumption_kg_per_kwh: 0.0,
            idle_consumption_kg_per_hour: 0.0,
            estimated_lap_consumption_kg: 0.0,
            reference_lap_time_s: 0.0,
        }
    }
}
