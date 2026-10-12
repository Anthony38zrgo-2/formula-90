use crate::brake_thermals::{BrakeAxleThermalConfig, BrakeDuctAxleConfig, BrakeThermalConfig};
use super::document::*;

pub(crate) fn validate_brake_thermal(c: &BrakeThermalConfig) -> Result<(), String> {
    for (name, value) in [
        ("initial_temperature_c", c.initial_temperature_c),
        ("optimal_min_temperature_c", c.optimal_min_temperature_c),
        ("optimal_max_temperature_c", c.optimal_max_temperature_c),
        ("fade_start_temperature_c", c.fade_start_temperature_c),
        ("critical_temperature_c", c.critical_temperature_c),
    ] {
        if !value.is_finite() {
            return Err(format!("brakes.thermal.{name} must be finite"));
        }
    }
    if c.optimal_min_temperature_c >= c.optimal_max_temperature_c {
        return Err("brakes.thermal optimal_min must be below optimal_max".to_string());
    }
    if c.optimal_max_temperature_c > c.fade_start_temperature_c {
        return Err("brakes.thermal optimal_max must not exceed fade_start".to_string());
    }
    if c.fade_start_temperature_c >= c.critical_temperature_c {
        return Err("brakes.thermal fade_start must be below critical".to_string());
    }
    for (name, value) in [
        ("cold_efficiency", c.cold_efficiency),
        ("minimum_fade_efficiency", c.minimum_fade_efficiency),
        ("braking_heat_fraction", c.braking_heat_fraction),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!("brakes.thermal.{name} must be in [0,1]"));
        }
    }
    validate_brake_axle("front", &c.front)?;
    validate_brake_axle("rear", &c.rear)?;
    validate_brake_duct("front_duct", &c.front_duct)?;
    validate_brake_duct("rear_duct", &c.rear_duct)?;
    Ok(())
}

pub(crate) fn validate_brake_axle(name: &str, c: &BrakeAxleThermalConfig) -> Result<(), String> {
    if !c.rotor_radiation_ambient_view_factor.is_finite()
        || !(0.0..=1.0).contains(&c.rotor_radiation_ambient_view_factor)
    {
        return Err(format!(
            "brakes.thermal.{name}.rotor_radiation_ambient_view_factor must be in [0,1]"
        ));
    }
    if !c.rotor_radiation_emissivity.is_finite()
        || !(0.0..=1.0).contains(&c.rotor_radiation_emissivity)
    {
        return Err(format!(
            "brakes.thermal.{name}.rotor_radiation_emissivity must be in [0,1]"
        ));
    }
    for (field, value) in [
        ("rotor_mass_kg", c.rotor_mass_kg),
        ("rotor_outer_diameter_m", c.rotor_outer_diameter_m),
        ("rotor_inner_diameter_m", c.rotor_inner_diameter_m),
        ("installation_airflow_scale", c.installation_airflow_scale),
        ("thermal_mass_scale", c.thermal_mass_scale),
        ("rotor_to_rim_w_k", c.rotor_to_rim_w_k),
        ("rim_heat_capacity_j_k", c.rim_heat_capacity_j_k),
        ("rim_base_air_w_k", c.rim_base_air_w_k),
        ("rim_to_tire_carcass_w_k", c.rim_to_tire_carcass_w_k),
        ("rim_to_tire_gas_w_k", c.rim_to_tire_gas_w_k),
    ] {
        if !value.is_finite() {
            return Err(format!("brakes.thermal.{name}.{field} must be finite"));
        }
    }
    if c.rotor_mass_kg < 0.0 || c.thermal_mass_scale <= 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.rotor_mass_kg must be non-negative and thermal_mass_scale must be positive"
        ));
    }
    if c.rotor_outer_diameter_m <= 0.0 || c.rotor_inner_diameter_m < 0.0 {
        return Err(format!(
            "brakes.thermal.{name} rotor diameters must be non-negative"
        ));
    }
    if c.rotor_inner_diameter_m >= c.rotor_outer_diameter_m {
        return Err(format!(
            "brakes.thermal.{name} inner rotor diameter must be smaller than outer diameter"
        ));
    }
    if c.installation_airflow_scale < 0.0 || c.rotor_to_rim_w_k < 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.installation_airflow_scale and rotor_to_rim_w_k must be non-negative"
        ));
    }
    if c.rim_heat_capacity_j_k <= 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.rim_heat_capacity_j_k must be positive"
        ));
    }
    for (field, value) in [
        ("rim_base_air_w_k", c.rim_base_air_w_k),
        ("rim_to_tire_carcass_w_k", c.rim_to_tire_carcass_w_k),
        ("rim_to_tire_gas_w_k", c.rim_to_tire_gas_w_k),
    ] {
        if value < 0.0 {
            return Err(format!(
                "brakes.thermal.{name}.{field} must be non-negative"
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_brake_duct(name: &str, c: &BrakeDuctAxleConfig) -> Result<(), String> {
    if !c.opening.is_finite() || !(0.0..=1.0).contains(&c.opening) {
        return Err(format!("brakes.thermal.{name}.opening must be in [0,1]"));
    }
    if !c.max_inlet_area_m2_per_wheel.is_finite() || c.max_inlet_area_m2_per_wheel <= 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.max_inlet_area_m2_per_wheel must be positive"
        ));
    }
    if !c.discharge_coefficient.is_finite() || !(0.0..=1.5).contains(&c.discharge_coefficient) {
        return Err(format!(
            "brakes.thermal.{name}.discharge_coefficient is invalid"
        ));
    }
    if !c.pressure_recovery.is_finite() || !(0.0..=1.2).contains(&c.pressure_recovery) {
        return Err(format!(
            "brakes.thermal.{name}.pressure_recovery is invalid"
        ));
    }
    if !c.cooling_reference_speed_ms.is_finite() || c.cooling_reference_speed_ms <= 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.cooling_reference_speed_ms must be positive"
        ));
    }
    if !c.drag_coefficient.is_finite() || c.drag_coefficient < 0.0 {
        return Err(format!("brakes.thermal.{name}.drag_coefficient is invalid"));
    }
    if !c.area_response_exponent.is_finite() || c.area_response_exponent <= 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.area_response_exponent must be positive"
        ));
    }
    if !c.cooling_flow_exponent.is_finite() || c.cooling_flow_exponent <= 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.cooling_flow_exponent must be positive"
        ));
    }
    if !c.minimum_cooling_flow_ratio.is_finite()
        || !(0.0..=0.5).contains(&c.minimum_cooling_flow_ratio)
    {
        return Err(format!(
            "brakes.thermal.{name}.minimum_cooling_flow_ratio is invalid"
        ));
    }
    if !c.air_specific_heat_j_kg_k.is_finite() || c.air_specific_heat_j_kg_k <= 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.air_specific_heat_j_kg_k must be positive"
        ));
    }
    if !c.heat_exchanger_ua_w_k.is_finite() || c.heat_exchanger_ua_w_k < 0.0 {
        return Err(format!(
            "brakes.thermal.{name}.heat_exchanger_ua_w_k must be non-negative"
        ));
    }
    if !c.rotor_cooling_fraction.is_finite()
        || c.rotor_cooling_fraction <= 0.0
        || c.rotor_cooling_fraction > 1.0
    {
        return Err(format!(
            "brakes.thermal.{name}.rotor_cooling_fraction must be in (0,1]"
        ));
    }
    Ok(())
}


impl JsonBrakes {
    pub(crate) fn validate_wheel_lock(&self) -> Result<(), String> {
        if let Some(blend) = self.wheel_lock_blend_spin_rad_s {
            if !blend.is_finite() || !(0.0..=200.0).contains(&blend) {
                return Err("brakes.wheel_lock_blend_spin_rad_s must be in [0,200]".to_string());
            }
        }
        if let Some(tau) = self.wheel_lock_stick_tau_s {
            if !tau.is_finite() || !(0.0..=1.0).contains(&tau) {
                return Err("brakes.wheel_lock_stick_tau_s must be in [0,1]".to_string());
            }
        }
        Ok(())
    }

}
