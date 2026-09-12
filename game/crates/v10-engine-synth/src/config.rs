#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub sample_rate: u32,
    pub seed: u64,
    pub bore_m: f32,
    pub stroke_m: f32,
    pub rod_length_m: f32,
    pub compression_ratio: f32,
    pub bank_angle_deg: f32,
    pub firing_order: [usize; 10],
    pub combustion_start_deg: f32,
    pub combustion_rise_deg: f32,
    pub combustion_shape_factor: f32,
    pub combustion_efficiency: f32,
    pub fuel_energy_per_cycle: f32,
    /// Chamber pressure (Pa) that maps to an acoustic source of 1.0, so the
    /// physical combustion model can be normalised to the frozen mix level.
    pub combustion_pressure_reference_pa: f32,
    /// Select the thermodynamic pressure path (`true`) or the legacy handcrafted
    /// `pressure_shape` envelope (`false`). Legacy is retained only as a
    /// compatibility/test mode pending full validation of the physical model.
    pub use_physical_pressure: bool,
    pub expansion_decay_deg: f32,
    /// Exhaust valve opening angle (deg, EVO).
    pub exhaust_open_deg: f32,
    pub exhaust_opening_ramp_deg: f32,
    /// Exhaust valve closing angle (deg, EVC) = `exhaust_open_deg +
    /// exhaust_duration_deg`.
    pub exhaust_duration_deg: f32,
    pub exhaust_closing_ramp_deg: f32,
    /// Poppet exhaust-valve seat diameter (m).
    pub exhaust_valve_diameter_m: f32,
    /// Maximum exhaust-valve lift (m).
    pub exhaust_max_lift_m: f32,
    /// Discharge coefficient relating effective (vena-contracta) curtain area
    /// to geometric poppet curtain area.
    pub exhaust_discharge_coefficient: f32,
    /// Opening-ramp shape exponent (> 0). 1.0 is a raised-cosine ramp.
    pub exhaust_opening_shape: f32,
    /// Closing-ramp shape exponent (> 0). 1.0 is a raised-cosine ramp.
    pub exhaust_closing_shape: f32,
    pub cycle_variation: f32,
    pub cylinder_spread: f32,
    pub cylinder_signature: [f32; 10],
    pub header_lengths_m: [f32; 10],
    pub exhaust_wave_speed_mps: f32,
    pub header_reflection: f32,
    /// Derive the exhaust wave speed from the runner gas temperature
    /// (`sqrt(gamma * R * T)`, PHY-061) instead of the fixed `exhaust_wave_speed_mps`.
    pub use_temperature_dependent_wave_speed: bool,
    /// Use the physical runner mass-flow delta (PHY-052) as the header
    /// excitation, instead of the legacy `pressure * lift^1.35` proxy.
    pub use_physical_exhaust_excitation: bool,
    /// Level-matching gain from normalised runner mass-flow delta to the
    /// acoustic excitation amplitude (measured in PHY-052).
    pub exhaust_excitation_gain: f32,
    pub pressure_direct_gain: f32,
    pub crankcase_gain: f32,
    pub block_gain: f32,
    pub head_gain: f32,
    pub exhaust_gain: f32,
    pub turbulence_gain: f32,
    pub master_gain: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            seed: 0xF090_0010,
            bore_m: 0.096,
            stroke_m: 0.042,
            rod_length_m: 0.135,
            compression_ratio: 12.5,
            bank_angle_deg: 72.0,
            // Alternating banks: A0, B0, A1, B1 ... at uniform 72° spacing.
            firing_order: [0, 5, 1, 6, 2, 7, 3, 8, 4, 9],
            combustion_start_deg: 0.0,
            combustion_rise_deg: 24.0,
            // Wiebe shape exponent `m`; higher values bias burn toward the end of
            // the window. 3.0 is a conventional mid-range gasoline value.
            combustion_shape_factor: 3.0,
            // Fraction of fuel chemical energy converted to heat in the charge.
            combustion_efficiency: 0.96,
            // Total chemical energy (J) of a full-charge burn at energy = 1.0.
            // Sized so peak pressure stays well inside the 20 MPa gas bound.
            fuel_energy_per_cycle: 600.0,
            // Full-load peak (~7.8 MPa for the default geometry) mapped onto the
            // frozen Sprint-A full-load acoustic level.
            combustion_pressure_reference_pa: 17_000_000.0,
            use_physical_pressure: true,
            expansion_decay_deg: 112.0,
            exhaust_open_deg: 116.0,
            exhaust_opening_ramp_deg: 5.0,
            exhaust_duration_deg: 142.0,
            exhaust_closing_ramp_deg: 24.0,
            exhaust_valve_diameter_m: 0.03456,
            exhaust_max_lift_m: 0.010368,
            exhaust_discharge_coefficient: 0.62,
            exhaust_opening_shape: 1.0,
            exhaust_closing_shape: 1.0,
            // Fast independent cycle jitter reads as a 720-degree flutter.
            // Keep it subordinate to the fixed ten-cylinder signature.
            cycle_variation: 0.004,
            cylinder_spread: 0.012,
            cylinder_signature: [1.10, 0.94, 1.04, 0.90, 1.07, 0.96, 1.12, 0.92, 1.02, 0.97],
            header_lengths_m: [
                0.535, 0.557, 0.548, 0.571, 0.562, 0.541, 0.566, 0.552, 0.578, 0.559,
            ],
            exhaust_wave_speed_mps: 545.0,
            header_reflection: -0.34,
            use_temperature_dependent_wave_speed: true,
            use_physical_exhaust_excitation: true,
            // Level-matched so the physical mass-flow delta lands at the same
            // acoustic amplitude as the proxy blowdown at the PHY-052 reference
            // point (rpm=7499, throttle=0.72, load=0.66): proxy max peak ~0.0159
            // vs routed dmass ~0.0182 (with signature) -> gain ~0.096.
            exhaust_excitation_gain: 0.096,
            pressure_direct_gain: 0.025,
            crankcase_gain: 0.008,
            block_gain: 0.56,
            head_gain: 0.48,
            exhaust_gain: 2.20,
            turbulence_gain: 0.40,
            master_gain: 7.0,
        }
    }
}

impl EngineConfig {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_geometry()?;
        self.validate_combustion()?;
        self.validate_exhaust()?;
        self.validate_gains()
    }

    fn validate_geometry(&self) -> Result<(), String> {
        if !(8_000..=192_000).contains(&self.sample_rate) {
            return Err(format!("sample_rate out of range: {}", self.sample_rate));
        }
        if !self.bore_m.is_finite() || !(0.05..=0.20).contains(&self.bore_m) {
            return Err("bore_m outside supported range".into());
        }
        if !self.stroke_m.is_finite() || !(0.030..=0.200).contains(&self.stroke_m) {
            return Err("stroke_m outside supported range".into());
        }
        let crank_radius_m = self.stroke_m * 0.5;
        if !self.rod_length_m.is_finite()
            || !(crank_radius_m..=0.50).contains(&self.rod_length_m)
        {
            return Err("rod_length_m too short for crank radius or outside range".into());
        }
        if !self.compression_ratio.is_finite() || !(6.0..=20.0).contains(&self.compression_ratio) {
            return Err("compression_ratio outside supported range".into());
        }
        if !self.bank_angle_deg.is_finite() || !(10.0..=180.0).contains(&self.bank_angle_deg) {
            return Err("bank_angle_deg outside supported range".into());
        }
        let mut seen = [false; 10];
        for &cylinder in &self.firing_order {
            if cylinder >= 10 || seen[cylinder] {
                return Err("firing_order must contain every cylinder exactly once".into());
            }
            seen[cylinder] = true;
        }
        Ok(())
    }

    fn validate_combustion(&self) -> Result<(), String> {
        if !(20.0..=160.0).contains(&self.combustion_rise_deg) {
            return Err("combustion_rise_deg outside physical experiment bounds".into());
        }
        if !self.combustion_shape_factor.is_finite() || !(1.0..=10.0).contains(&self.combustion_shape_factor) {
            return Err("combustion_shape_factor outside supported range".into());
        }
        if !self.combustion_efficiency.is_finite() || !(0.0..=1.0).contains(&self.combustion_efficiency) {
            return Err("combustion_efficiency outside supported range".into());
        }
        if !self.fuel_energy_per_cycle.is_finite() || self.fuel_energy_per_cycle <= 0.0 {
            return Err("fuel_energy_per_cycle must be positive".into());
        }
        if !self.combustion_pressure_reference_pa.is_finite() || self.combustion_pressure_reference_pa <= 0.0 {
            return Err("combustion_pressure_reference_pa must be positive".into());
        }
        if !(30.0..=260.0).contains(&self.expansion_decay_deg) {
            return Err("expansion_decay_deg outside physical experiment bounds".into());
        }
        Ok(())
    }

    fn validate_exhaust(&self) -> Result<(), String> {
        self.validate_exhaust_timing()?;
        self.validate_exhaust_wave()
    }

    fn validate_exhaust_timing(&self) -> Result<(), String> {
        if !(60.0..=240.0).contains(&self.exhaust_open_deg) {
            return Err("exhaust_open_deg outside physical experiment bounds".into());
        }
        if !(1.0..=30.0).contains(&self.exhaust_opening_ramp_deg) {
            return Err("exhaust_opening_ramp_deg outside physical experiment bounds".into());
        }
        if !(30.0..=240.0).contains(&self.exhaust_duration_deg) {
            return Err("exhaust_duration_deg outside physical experiment bounds".into());
        }
        if !(1.0..=60.0).contains(&self.exhaust_closing_ramp_deg) {
            return Err("exhaust_closing_ramp_deg outside physical experiment bounds".into());
        }
        if self.exhaust_opening_ramp_deg + self.exhaust_closing_ramp_deg >= self.exhaust_duration_deg {
            return Err("exhaust opening + closing ramps must fit within the lift window".into());
        }
        if !self.exhaust_valve_diameter_m.is_finite() || !(0.005..=0.10).contains(&self.exhaust_valve_diameter_m) {
            return Err("exhaust_valve_diameter_m outside supported range".into());
        }
        if !self.exhaust_max_lift_m.is_finite() || !(0.001..=0.020).contains(&self.exhaust_max_lift_m) {
            return Err("exhaust_max_lift_m outside supported range".into());
        }
        if !self.exhaust_discharge_coefficient.is_finite()
            || !(0.10..=0.95).contains(&self.exhaust_discharge_coefficient)
        {
            return Err("exhaust_discharge_coefficient outside supported range".into());
        }
        for (name, shape) in [
            ("exhaust_opening_shape", self.exhaust_opening_shape),
            ("exhaust_closing_shape", self.exhaust_closing_shape),
        ] {
            if !shape.is_finite() || shape <= 0.0 || shape > 10.0 {
                return Err(format!("{name} outside supported range (must be > 0)"));
            }
        }
        Ok(())
    }

    fn validate_exhaust_wave(&self) -> Result<(), String> {
        if !(0.0..=0.15).contains(&self.cycle_variation)
            || !(0.0..=0.10).contains(&self.cylinder_spread)
        {
            return Err("combustion variation exceeds safe bounds".into());
        }
        if self
            .cylinder_signature
            .iter()
            .any(|&x| !x.is_finite() || !(0.75..=1.25).contains(&x))
        {
            return Err("cylinder signature outside supported range".into());
        }
        if self.exhaust_wave_speed_mps < 250.0 || self.exhaust_wave_speed_mps > 900.0 {
            return Err("exhaust wave speed outside supported range".into());
        }
        if self
            .header_lengths_m
            .iter()
            .any(|&x| !(0.2..=1.5).contains(&x))
        {
            return Err("header length outside supported range".into());
        }
        if self.header_reflection.abs() >= 0.75 {
            return Err("header reflection must remain safely below unity".into());
        }
        if !self.exhaust_excitation_gain.is_finite() || !(0.0..=2.0).contains(&self.exhaust_excitation_gain)
        {
            return Err("exhaust excitation gain outside declared safety bounds".into());
        }
        Ok(())
    }

    fn validate_gains(&self) -> Result<(), String> {
        if !(0.0..=2.0).contains(&self.pressure_direct_gain)
            || !(0.0..=2.0).contains(&self.crankcase_gain)
            || !(0.0..=2.0).contains(&self.block_gain)
            || !(0.0..=2.0).contains(&self.head_gain)
            || !(0.0..=4.0).contains(&self.exhaust_gain)
            || !(0.0..=2.0).contains(&self.turbulence_gain)
            || !(0.0..=20.0).contains(&self.master_gain)
        {
            return Err("mix gain outside declared safety bounds".into());
        }
        Ok(())
    }

    /// Distance from crankshaft axis to the crankpin at TDC. Half the stroke.
    pub fn crank_radius_m(&self) -> f32 {
        self.stroke_m * 0.5
    }

    /// Piston crown area derived from bore.
    pub fn bore_area_m2(&self) -> f32 {
        let bore = self.bore_m as f64;
        (std::f64::consts::PI * 0.25 * bore * bore) as f32
    }

    /// Per-cylinder volume displaced between TDC and BDC.
    pub fn swept_volume_m3(&self) -> f32 {
        self.bore_area_m2() * self.stroke_m
    }

    /// Combustion chamber volume at TDC.
    pub fn clearance_volume_m3(&self) -> f32 {
        self.swept_volume_m3() / (self.compression_ratio - 1.0)
    }

    /// Per-cylinder displacement in liters.
    pub fn cylinder_displacement_liters(&self) -> f32 {
        self.swept_volume_m3() * 1_000.0
    }

    /// Total engine displacement in liters for the full ten-cylinder V10.
    pub fn engine_displacement_liters(&self) -> f32 {
        self.cylinder_displacement_liters() * crate::crank::CYLINDER_COUNT as f32
    }

    /// Extract the dedicated physical engine configuration (PHY-130).
    pub fn physical(&self) -> PhysicalEngineConfig {
        PhysicalEngineConfig {
            bore_m: self.bore_m,
            stroke_m: self.stroke_m,
            rod_length_m: self.rod_length_m,
            compression_ratio: self.compression_ratio,
            bank_angle_deg: self.bank_angle_deg,
            firing_order: self.firing_order,
            combustion_start_deg: self.combustion_start_deg,
            combustion_rise_deg: self.combustion_rise_deg,
            combustion_shape_factor: self.combustion_shape_factor,
            combustion_efficiency: self.combustion_efficiency,
            fuel_energy_per_cycle: self.fuel_energy_per_cycle,
            combustion_pressure_reference_pa: self.combustion_pressure_reference_pa,
            use_physical_pressure: self.use_physical_pressure,
            expansion_decay_deg: self.expansion_decay_deg,
            exhaust_open_deg: self.exhaust_open_deg,
            exhaust_opening_ramp_deg: self.exhaust_opening_ramp_deg,
            exhaust_duration_deg: self.exhaust_duration_deg,
            exhaust_closing_ramp_deg: self.exhaust_closing_ramp_deg,
            exhaust_valve_diameter_m: self.exhaust_valve_diameter_m,
            exhaust_max_lift_m: self.exhaust_max_lift_m,
            exhaust_discharge_coefficient: self.exhaust_discharge_coefficient,
            exhaust_opening_shape: self.exhaust_opening_shape,
            exhaust_closing_shape: self.exhaust_closing_shape,
            cycle_variation: self.cycle_variation,
            cylinder_spread: self.cylinder_spread,
            cylinder_signature: self.cylinder_signature,
            header_lengths_m: self.header_lengths_m,
            exhaust_wave_speed_mps: self.exhaust_wave_speed_mps,
            header_reflection: self.header_reflection,
            use_temperature_dependent_wave_speed: self.use_temperature_dependent_wave_speed,
            use_physical_exhaust_excitation: self.use_physical_exhaust_excitation,
            exhaust_excitation_gain: self.exhaust_excitation_gain,
        }
    }

    /// Extract the dedicated acoustic mix configuration (PHY-130).
    pub fn mix(&self) -> AcousticMixConfig {
        AcousticMixConfig {
            pressure_direct_gain: self.pressure_direct_gain,
            crankcase_gain: self.crankcase_gain,
            block_gain: self.block_gain,
            head_gain: self.head_gain,
            exhaust_gain: self.exhaust_gain,
            turbulence_gain: self.turbulence_gain,
            master_gain: self.master_gain,
        }
    }

    /// Construct a unified EngineConfig from separate physical and mix configurations (PHY-130).
    pub fn from_subsystems(
        sample_rate: u32,
        seed: u64,
        physical: PhysicalEngineConfig,
        mix: AcousticMixConfig,
    ) -> Self {
        Self {
            sample_rate,
            seed,
            bore_m: physical.bore_m,
            stroke_m: physical.stroke_m,
            rod_length_m: physical.rod_length_m,
            compression_ratio: physical.compression_ratio,
            bank_angle_deg: physical.bank_angle_deg,
            firing_order: physical.firing_order,
            combustion_start_deg: physical.combustion_start_deg,
            combustion_rise_deg: physical.combustion_rise_deg,
            combustion_shape_factor: physical.combustion_shape_factor,
            combustion_efficiency: physical.combustion_efficiency,
            fuel_energy_per_cycle: physical.fuel_energy_per_cycle,
            combustion_pressure_reference_pa: physical.combustion_pressure_reference_pa,
            use_physical_pressure: physical.use_physical_pressure,
            expansion_decay_deg: physical.expansion_decay_deg,
            exhaust_open_deg: physical.exhaust_open_deg,
            exhaust_opening_ramp_deg: physical.exhaust_opening_ramp_deg,
            exhaust_duration_deg: physical.exhaust_duration_deg,
            exhaust_closing_ramp_deg: physical.exhaust_closing_ramp_deg,
            exhaust_valve_diameter_m: physical.exhaust_valve_diameter_m,
            exhaust_max_lift_m: physical.exhaust_max_lift_m,
            exhaust_discharge_coefficient: physical.exhaust_discharge_coefficient,
            exhaust_opening_shape: physical.exhaust_opening_shape,
            exhaust_closing_shape: physical.exhaust_closing_shape,
            cycle_variation: physical.cycle_variation,
            cylinder_spread: physical.cylinder_spread,
            cylinder_signature: physical.cylinder_signature,
            header_lengths_m: physical.header_lengths_m,
            exhaust_wave_speed_mps: physical.exhaust_wave_speed_mps,
            header_reflection: physical.header_reflection,
            use_temperature_dependent_wave_speed: physical.use_temperature_dependent_wave_speed,
            use_physical_exhaust_excitation: physical.use_physical_exhaust_excitation,
            exhaust_excitation_gain: physical.exhaust_excitation_gain,
            pressure_direct_gain: mix.pressure_direct_gain,
            crankcase_gain: mix.crankcase_gain,
            block_gain: mix.block_gain,
            head_gain: mix.head_gain,
            exhaust_gain: mix.exhaust_gain,
            turbulence_gain: mix.turbulence_gain,
            master_gain: mix.master_gain,
        }
    }
}

/// Standalone physical engine geometry, combustion and gas boundary configuration (PHY-130).
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalEngineConfig {
    pub bore_m: f32,
    pub stroke_m: f32,
    pub rod_length_m: f32,
    pub compression_ratio: f32,
    pub bank_angle_deg: f32,
    pub firing_order: [usize; 10],
    pub combustion_start_deg: f32,
    pub combustion_rise_deg: f32,
    pub combustion_shape_factor: f32,
    pub combustion_efficiency: f32,
    pub fuel_energy_per_cycle: f32,
    pub combustion_pressure_reference_pa: f32,
    pub use_physical_pressure: bool,
    pub expansion_decay_deg: f32,
    pub exhaust_open_deg: f32,
    pub exhaust_opening_ramp_deg: f32,
    pub exhaust_duration_deg: f32,
    pub exhaust_closing_ramp_deg: f32,
    pub exhaust_valve_diameter_m: f32,
    pub exhaust_max_lift_m: f32,
    pub exhaust_discharge_coefficient: f32,
    pub exhaust_opening_shape: f32,
    pub exhaust_closing_shape: f32,
    pub cycle_variation: f32,
    pub cylinder_spread: f32,
    pub cylinder_signature: [f32; 10],
    pub header_lengths_m: [f32; 10],
    pub exhaust_wave_speed_mps: f32,
    pub header_reflection: f32,
    pub use_temperature_dependent_wave_speed: bool,
    pub use_physical_exhaust_excitation: bool,
    pub exhaust_excitation_gain: f32,
}

impl Default for PhysicalEngineConfig {
    fn default() -> Self {
        EngineConfig::default().physical()
    }
}

/// Standalone acoustic mix and structural gain configuration (PHY-130).
#[derive(Clone, Debug, PartialEq)]
pub struct AcousticMixConfig {
    pub pressure_direct_gain: f32,
    pub crankcase_gain: f32,
    pub block_gain: f32,
    pub head_gain: f32,
    pub exhaust_gain: f32,
    pub turbulence_gain: f32,
    pub master_gain: f32,
}

impl Default for AcousticMixConfig {
    fn default() -> Self {
        EngineConfig::default().mix()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_and_mix_subsystems_roundtrip_identically() {
        let base = EngineConfig::default();
        let phys = base.physical();
        let mix = base.mix();
        let roundtrip = EngineConfig::from_subsystems(base.sample_rate, base.seed, phys.clone(), mix.clone());
        assert_eq!(roundtrip.physical(), phys);
        assert_eq!(roundtrip.mix(), mix);
        assert!(roundtrip.validate().is_ok());
    }
}

