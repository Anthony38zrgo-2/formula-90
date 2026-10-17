use super::*;
use super::document::JsonSuspension;

impl JsonSuspension {
    pub(crate) fn validate_geometry(&self) -> Result<(), String> {
        for (name, axle) in [("front", &self.front), ("rear", &self.rear)] {
            for (field, value) in [
                ("camber_gain_rad_per_m", axle.camber_gain_rad_per_m),
                ("toe_gain_rad_per_m", axle.toe_gain_rad_per_m),
            ] {
                if let Some(v) = value {
                    if !v.is_finite() {
                        return Err(format!(
                            "suspension.{name}.{field} must be finite (got {v})"
                        ));
                    }
                }
            }
        }
        // SUS-GEO-02: explicit model selector, no silent fallback.
        let model = SuspensionModelKind::from_str_name(self.model.as_str()).ok_or_else(|| {
            format!(
                "suspension.model must be legacy_1dof|geometric (got {})",
                self.model
            )
        })?;
        if self.model_version != default_suspension_model_version() {
            return Err(format!(
                "suspension.model_version must be {} (got {})",
                default_suspension_model_version(),
                self.model_version
            ));
        }
        if self.use_authored_wheel_centers && self.geometry_physical.is_none() {
            return Err("Authored wheel centers require physical suspension geometry".to_string());
        }
        match model {
            SuspensionModelKind::Legacy1Dof => {
                if self.geometry_physical.is_some() {
                    return Err(
                        "suspension.geometry_physical must be absent for legacy_1dof (set suspension.model=geometric to activate)"
                            .to_string(),
                    );
                }
            }
            SuspensionModelKind::Geometric => {
                let v = self.geometry_physical.as_ref().ok_or_else(|| {
                    "suspension.geometry_physical is required for geometric (no silent fallback to legacy)"
                        .to_string()
                })?;
                // Full physical validation with wheel+field diagnostics.
                geometric_from_json_value(v)?;
            }
        }
        Ok(())
    }


    pub(crate) fn validate_spring_lengths(&self) -> Result<(), String> {
        if self.front.spring_length <= 0.0 || self.rear.spring_length <= 0.0 {
            return Err("Spring lengths must be positive".to_string());
        }
        Ok(())
    }
}
