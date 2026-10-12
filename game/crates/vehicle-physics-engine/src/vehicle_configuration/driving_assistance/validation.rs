use super::document::JsonAids;
use super::super::powertrain::document::JsonPowertrain;

impl JsonAids {
    pub(crate) fn validate(&self, powertrain: &JsonPowertrain) -> Result<(), String> {
        let gear_count = powertrain.gear_ratios.len();
        if self.traction_control_gear_authority.len() != gear_count
            || self.traction_control_gear_slip_target.len() != gear_count
            || self.traction_control_gear_max_cut.len() != gear_count
        {
            return Err(format!(
                "Traction-control gear arrays (authority/slip_target/max_cut) must have exactly {gear_count} entries matching gear_ratios"
            ));
        }
        for gear in 0..gear_count {
            let authority = self.traction_control_gear_authority[gear];
            let slip_target = self.traction_control_gear_slip_target[gear];
            let max_cut = self.traction_control_gear_max_cut[gear];
            if !authority.is_finite() || !(0.0..=1.0).contains(&authority) {
                return Err(format!(
                    "aids.traction_control_gear_authority[{gear}] must be in [0,1]"
                ));
            }
            if !max_cut.is_finite() || !(0.0..=1.0).contains(&max_cut) {
                return Err(format!(
                    "aids.traction_control_gear_max_cut[{gear}] must be in [0,1]"
                ));
            }
            if authority > 0.0 && (!slip_target.is_finite() || slip_target <= 0.0) {
                return Err(format!(
                    "aids.traction_control_gear_slip_target[{gear}] must be positive when TC authority is non-zero"
                ));
            }
        }
        if !self.traction_control_attack_rate.is_finite()
            || self.traction_control_attack_rate <= 0.0
            || !self.traction_control_release_rate.is_finite()
            || self.traction_control_release_rate <= 0.0
        {
            return Err("aids TC attack/release rates must be positive and finite".to_string());
        }
        Ok(())
    }
}
