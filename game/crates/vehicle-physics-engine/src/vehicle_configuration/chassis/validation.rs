use super::document::{JsonChassis, JsonGeometry};

pub(crate) fn validate_chassis(chassis: &JsonChassis) -> Result<(), String> {
        if chassis.vehicle_mass <= 0.0 {
            return Err("vehicle_mass must be positive".to_string());
        }
    Ok(())
}

pub(crate) fn validate_geometry(geometry: &JsonGeometry) -> Result<(), String> {
        if geometry.wheelbase <= 0.0 {
            return Err("wheelbase must be positive".to_string());
        }
    Ok(())
}
