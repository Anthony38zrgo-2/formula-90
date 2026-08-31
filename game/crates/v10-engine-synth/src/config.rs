#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub sample_rate: u32,
    pub seed: u64,
    pub firing_order: [usize; 10],
    pub combustion_start_deg: f32,
    pub combustion_rise_deg: f32,
    pub expansion_decay_deg: f32,
    pub exhaust_open_deg: f32,
    pub exhaust_opening_ramp_deg: f32,
    pub exhaust_duration_deg: f32,
    pub cycle_variation: f32,
    pub cylinder_spread: f32,
    pub cylinder_signature: [f32; 10],
    pub header_lengths_m: [f32; 10],
    pub exhaust_wave_speed_mps: f32,
    pub header_reflection: f32,
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
            // Alternating banks: A0, B0, A1, B1 ... at uniform 72° spacing.
            firing_order: [0, 5, 1, 6, 2, 7, 3, 8, 4, 9],
            combustion_start_deg: 0.0,
            combustion_rise_deg: 24.0,
            expansion_decay_deg: 112.0,
            exhaust_open_deg: 116.0,
            exhaust_opening_ramp_deg: 5.0,
            exhaust_duration_deg: 142.0,
            // Fast independent cycle jitter reads as a 720-degree flutter.
            // Keep it subordinate to the fixed ten-cylinder signature.
            cycle_variation: 0.004,
            cylinder_spread: 0.012,
            cylinder_signature: [
                1.10, 0.94, 1.04, 0.90, 1.07, 0.96, 1.12, 0.92, 1.02, 0.97,
            ],
            header_lengths_m: [
                0.535, 0.557, 0.548, 0.571, 0.562, 0.541, 0.566, 0.552, 0.578, 0.559,
            ],
            exhaust_wave_speed_mps: 545.0,
            header_reflection: -0.34,
            pressure_direct_gain: 0.07,
            crankcase_gain: 0.025,
            block_gain: 0.56,
            head_gain: 0.48,
            exhaust_gain: 1.55,
            turbulence_gain: 0.40,
            master_gain: 7.0,
        }
    }
}

impl EngineConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(8_000..=192_000).contains(&self.sample_rate) {
            return Err(format!("sample_rate out of range: {}", self.sample_rate));
        }
        let mut seen = [false; 10];
        for &cylinder in &self.firing_order {
            if cylinder >= 10 || seen[cylinder] {
                return Err("firing_order must contain every cylinder exactly once".into());
            }
            seen[cylinder] = true;
        }
        if !(20.0..=160.0).contains(&self.combustion_rise_deg) {
            return Err("combustion_rise_deg outside physical experiment bounds".into());
        }
        if !(30.0..=260.0).contains(&self.expansion_decay_deg) {
            return Err("expansion_decay_deg outside physical experiment bounds".into());
        }
        if !(60.0..=240.0).contains(&self.exhaust_open_deg) {
            return Err("exhaust_open_deg outside physical experiment bounds".into());
        }
        if !(1.0..=30.0).contains(&self.exhaust_opening_ramp_deg) {
            return Err("exhaust_opening_ramp_deg outside physical experiment bounds".into());
        }
        if !(30.0..=240.0).contains(&self.exhaust_duration_deg) {
            return Err("exhaust_duration_deg outside physical experiment bounds".into());
        }
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
}
