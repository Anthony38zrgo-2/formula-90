use super::document::JsonFuel;

pub(crate) fn validate_fuel(fuel: &JsonFuel) -> Result<(), String> {
        if !fuel.capacity_kg.is_finite() || fuel.capacity_kg < 0.0 {
            return Err("fuel.capacity_kg must be finite and non-negative".to_string());
        }
        if !fuel.initial_kg.is_finite()
            || fuel.initial_kg < 0.0
            || fuel.initial_kg > fuel.capacity_kg
        {
            return Err("fuel.initial_kg must be finite and within [0, fuel.capacity_kg]".to_string());
        }
        if !fuel.brake_specific_consumption_kg_per_kwh.is_finite()
            || fuel.brake_specific_consumption_kg_per_kwh < 0.0
        {
            return Err(
                "fuel.brake_specific_consumption_kg_per_kwh must be finite and non-negative"
                    .to_string(),
            );
        }
        if !fuel.idle_consumption_kg_per_hour.is_finite()
            || fuel.idle_consumption_kg_per_hour < 0.0
        {
            return Err(
                "fuel.idle_consumption_kg_per_hour must be finite and non-negative".to_string(),
            );
        }
        if !fuel.estimated_lap_consumption_kg.is_finite()
            || fuel.estimated_lap_consumption_kg < 0.0
        {
            return Err(
                "fuel.estimated_lap_consumption_kg must be finite and non-negative".to_string(),
            );
        }
        if !fuel.reference_lap_time_s.is_finite() || fuel.reference_lap_time_s < 0.0 {
            return Err("fuel.reference_lap_time_s must be finite and non-negative".to_string());
        }
        if !fuel.tank_position_local_m.x.is_finite()
            || !fuel.tank_position_local_m.y.is_finite()
            || !fuel.tank_position_local_m.z.is_finite()
        {
            return Err("fuel.tank_position_local_m must be finite".to_string());
        }
    Ok(())
}
