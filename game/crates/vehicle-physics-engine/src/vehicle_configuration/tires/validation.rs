use super::document::JsonTires;

impl JsonTires {
    pub(crate) fn validate(&self) -> Result<(), String> {
        self.validate_tire_pressure()?;
        self.validate_tire_thermal()?;
        self.validate_tire_wear()?;
        self.validate_tire_force_model()?;
        self.validate_tire_pressure_mechanics()
    }
    pub(crate) fn validate_tire_pressure(&self) -> Result<(), String> {
        if let Some(p) = &self.pressure {
            for (name, v) in p
                .cold_kpa_gauge
                .iter()
                .flatten()
                .chain(p.reference_hot_kpa_gauge.iter().flatten())
            {
                if !v.is_finite() || *v <= 0.0 {
                    return Err(format!("Tire pressure {name} must be positive and finite"));
                }
            }
            if let Some(t) = p.reference_temperature_c {
                if !t.is_finite() || !(-50.0..150.0).contains(&t) {
                    return Err("reference_temperature_c out of range".to_string());
                }
            }
            if let Some(v) = p.atmospheric_pressure_kpa {
                if !v.is_finite() || v <= 0.0 {
                    return Err("atmospheric_pressure_kpa must be positive and finite".to_string());
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate_tire_thermal(&self) -> Result<(), String> {
        if let Some(t) = &self.thermal {
            for (name, profile) in [
                ("shared", t),
                ("front", t.front.as_deref().unwrap_or(t)),
                ("rear", t.rear.as_deref().unwrap_or(t)),
            ] {
                let min_p = profile.minimum_pressure_kpa_gauge.unwrap_or(55.0);
                let max_p = profile.maximum_pressure_kpa_gauge.unwrap_or(260.0);
                if min_p >= max_p {
                    return Err(format!(
                        "tires.thermal.{name}.minimum_pressure_kpa_gauge must be below maximum_pressure_kpa_gauge"
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate_tire_wear(&self) -> Result<(), String> {
        if let Some(w) = &self.wear {
            for (name, profile) in [
                ("shared", w),
                ("front", w.front.as_deref().unwrap_or(w)),
                ("rear", w.rear.as_deref().unwrap_or(w)),
            ] {
                let initial = profile.initial_tread_depth_millimeters.unwrap_or(6.0);
                let minimum = profile.minimum_tread_depth_millimeters.unwrap_or(0.5);
                if !initial.is_finite() || initial <= 0.0 {
                    return Err(format!(
                        "tires.wear.{name}.initial_tread_depth_millimeters must be positive and finite"
                    ));
                }
                if !minimum.is_finite() || minimum < 0.0 || minimum >= initial {
                    return Err(format!(
                        "tires.wear.{name}.minimum_tread_depth_millimeters must be non-negative and below the initial depth"
                    ));
                }
                let rate = profile.tread_wear_per_megajoule_millimeters.unwrap_or(0.0);
                if !rate.is_finite() || rate < 0.0 {
                    return Err(format!(
                        "tires.wear.{name}.tread_wear_per_megajoule_millimeters must be finite and non-negative"
                    ));
                }
                for (field, value) in [
                    ("friction_reference_coefficient", profile.friction_reference_coefficient),
                    ("friction_sensitivity_exponent", profile.friction_sensitivity_exponent),
                    ("inner_shoulder_wear_multiplier", profile.inner_shoulder_wear_multiplier),
                    ("center_wear_multiplier", profile.center_wear_multiplier),
                    ("outer_shoulder_wear_multiplier", profile.outer_shoulder_wear_multiplier),
                    ("cliff_start_wear_fraction", profile.cliff_start_wear_fraction),
                    ("end_of_life_grip_scale", profile.end_of_life_grip_scale),
                    ("progressive_grip_loss_fraction", profile.progressive_grip_loss_fraction),
                    ("cold_temperature_wear_multiplier", profile.cold_temperature_wear_multiplier),
                    ("overheat_temperature_wear_multiplier", profile.overheat_temperature_wear_multiplier),
                    ("extreme_temperature_wear_multiplier", profile.extreme_temperature_wear_multiplier),
                ] {
                    if let Some(v) = value {
                        if !v.is_finite() || !(0.0..=10.0).contains(&v) {
                            return Err(format!(
                                "tires.wear.{name}.{field} must be finite and in [0.0,10.0]"
                            ));
                        }
                    }
                }
                let cliff = profile.cliff_start_wear_fraction.unwrap_or(0.75);
                let end_of_life = profile.end_of_life_grip_scale.unwrap_or(0.55);
                let progressive = profile.progressive_grip_loss_fraction.unwrap_or(0.05);
                if cliff > 1.0 || end_of_life > 1.0 || progressive > 1.0 {
                    return Err(format!(
                        "tires.wear.{name} fractions must not exceed 1.0"
                    ));
                }
                if progressive + end_of_life > 1.0 {
                    return Err(format!(
                        "tires.wear.{name}.progressive_grip_loss_fraction plus end_of_life_grip_scale must not exceed 1.0"
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate_tire_force_model(&self) -> Result<(), String> {
        if let Some(tau) = self.surface_transition_tau_s {
            if !tau.is_finite() || !(0.0..=1.0).contains(&tau) {
                return Err("tires.surface_transition_tau_s must be in [0.0,1.0]".to_string());
            }
        }
        if let Some(fm) = &self.force_model {
            for (name, profile) in [("front", &fm.front), ("rear", &fm.rear)] {
                for (field, value) in [
                    ("lateral_peak_slip_angle_rad", profile.lateral_peak_slip_angle_rad),
                    ("longitudinal_peak_slip_ratio", profile.longitudinal_peak_slip_ratio),
                    ("lateral_slide_mu_ratio", profile.lateral_slide_mu_ratio),
                    ("longitudinal_slide_mu_ratio", profile.longitudinal_slide_mu_ratio),
                    ("budget_blend_width", profile.budget_blend_width),
                    ("falloff_sharpness", profile.falloff_sharpness),
                    ("slip_loss_tau_s", profile.slip_loss_tau_s),
                    ("slip_recovery_tau_s", profile.slip_recovery_tau_s),
                    (
                        "combined_lateral_peak_migration",
                        profile.combined_lateral_peak_migration,
                    ),
                    (
                        "combined_longitudinal_peak_migration",
                        profile.combined_longitudinal_peak_migration,
                    ),
                    ("min_relaxation_tau_s", profile.min_relaxation_tau_s),
                    (
                        "relaxation_recovery_scale",
                        profile.relaxation_recovery_scale,
                    ),
                    ("contact_ramp_tau_s", profile.contact_ramp_tau_s),
                ] {
                    if !value.is_finite() {
                        return Err(format!(
                            "tires.force_model.{name}.{field} must be finite, got {value}"
                        ));
                    }
                }
            }
            for (name, profile) in [
                ("front", &fm.front),
                ("rear", &fm.rear),
            ] {
                if !(0.02..=0.60).contains(&profile.lateral_peak_slip_angle_rad) {
                    return Err(format!(
                        "tires.force_model.{name}.lateral_peak_slip_angle_rad must be in [0.02,0.60]"
                    ));
                }
                if !(0.02..=1.5).contains(&profile.longitudinal_peak_slip_ratio) {
                    return Err(format!(
                        "tires.force_model.{name}.longitudinal_peak_slip_ratio must be in [0.02,1.5]"
                    ));
                }
                if !(0.10..=1.0).contains(&profile.lateral_slide_mu_ratio) {
                    return Err(format!(
                        "tires.force_model.{name}.lateral_slide_mu_ratio must be in [0.10,1.0]"
                    ));
                }
                if !(0.10..=1.0).contains(&profile.longitudinal_slide_mu_ratio) {
                    return Err(format!(
                        "tires.force_model.{name}.longitudinal_slide_mu_ratio must be in [0.10,1.0]"
                    ));
                }
                if !(0.0..=0.5).contains(&profile.budget_blend_width) {
                    return Err(format!(
                        "tires.force_model.{name}.budget_blend_width must be in [0.0,0.5]"
                    ));
                }
                if !(0.5..=8.0).contains(&profile.falloff_sharpness) {
                    return Err(format!(
                        "tires.force_model.{name}.falloff_sharpness must be in [0.5,8.0]"
                    ));
                }
                if !(0.0..=2.0).contains(&profile.slip_loss_tau_s) {
                    return Err(format!(
                        "tires.force_model.{name}.slip_loss_tau_s must be in [0.0,2.0]"
                    ));
                }
                if !(0.0..=2.0).contains(&profile.slip_recovery_tau_s) {
                    return Err(format!(
                        "tires.force_model.{name}.slip_recovery_tau_s must be in [0.0,2.0]"
                    ));
                }
                if !(0.0..=0.9).contains(&profile.combined_lateral_peak_migration) {
                    return Err(format!(
                        "tires.force_model.{name}.combined_lateral_peak_migration must be in [0.0,0.9]"
                    ));
                }
                if !(0.0..=0.9).contains(&profile.combined_longitudinal_peak_migration) {
                    return Err(format!(
                        "tires.force_model.{name}.combined_longitudinal_peak_migration must be in [0.0,0.9]"
                    ));
                }
                if !(0.0..=0.5).contains(&profile.min_relaxation_tau_s) {
                    return Err(format!(
                        "tires.force_model.{name}.min_relaxation_tau_s must be in [0.0,0.5]"
                    ));
                }
                if !(0.1..=5.0).contains(&profile.relaxation_recovery_scale) {
                    return Err(format!(
                        "tires.force_model.{name}.relaxation_recovery_scale must be in [0.1,5.0]"
                    ));
                }
                if !(0.0..=1.0).contains(&profile.contact_ramp_tau_s) {
                    return Err(format!(
                        "tires.force_model.{name}.contact_ramp_tau_s must be in [0.0,1.0]"
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate_tire_pressure_mechanics(&self) -> Result<(), String> {
        if let Some(pm) = &self.pressure_mechanics {
            for (field, value) in [
                ("compliance_sensitivity", pm.compliance_sensitivity),
                ("response_sensitivity", pm.response_sensitivity),
                ("rolling_resistance_sensitivity", pm.rolling_resistance_sensitivity),
            ] {
                if !value.is_finite() {
                    return Err(format!("tires.pressure_mechanics.{field} must be finite"));
                }
            }
        }
        Ok(())
    }

}


pub(crate) fn validate_axle_radii(tires: &JsonTires) -> Result<(), String> {
        if tires.front.radius <= 0.0 || tires.rear.radius <= 0.0 {
            return Err("Tire radii must be positive".to_string());
        }
    Ok(())
}
