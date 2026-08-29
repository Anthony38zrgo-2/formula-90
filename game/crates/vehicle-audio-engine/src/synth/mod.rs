pub mod event_gen;
pub mod impulse;

use crate::dsp::biquad::Biquad;
use crate::powertrain::AudioPowertrainSynthesis;

use event_gen::EventJitter;
use impulse::{
    one_pole_alpha, torque_curve_value, BODY_LOWPASS_HZ, IMPULSE_ATTACK_S, IMPULSE_DECAY_S,
    IMPULSE_LEVEL,
};

pub const CYCLE_DEG: f64 = 720.0;
pub const DEFAULT_FIRING_PHASES_DEG: [f64; 5] = [0.0, 144.0, 288.0, 432.0, 576.0];

pub struct PowertrainConfig {
    pub firing_phases_deg: [f64; 5],
    pub irregularity: f32,
    pub seed: u64,
    pub torque_curve_weight: f32,
    pub throttle_response: f32,
    pub attack_smoothing_s: f32,
    pub release_smoothing_s: f32,
    pub load_smoothing_s: f32,
    pub torque_curve: Vec<(f64, f64)>,
}

impl Default for PowertrainConfig {
    fn default() -> Self {
        Self {
            firing_phases_deg: DEFAULT_FIRING_PHASES_DEG,
            irregularity: 0.01,
            seed: 0xF090_1994_D15C_A11D,
            torque_curve_weight: 1.0,
            throttle_response: 1.0,
            attack_smoothing_s: 0.02,
            release_smoothing_s: 0.12,
            load_smoothing_s: 0.03,
            torque_curve: impulse::default_torque_curve(),
        }
    }
}

impl From<&AudioPowertrainSynthesis> for PowertrainConfig {
    fn from(contract: &AudioPowertrainSynthesis) -> Self {
        Self {
            firing_phases_deg: firing_phases(contract),
            irregularity: contract.combustion.irregularity,
            seed: contract.combustion.seed,
            torque_curve_weight: contract.energy.torque_curve_weight,
            throttle_response: contract.energy.throttle_response,
            attack_smoothing_s: contract.energy.attack_smoothing_s,
            release_smoothing_s: contract.energy.release_smoothing_s,
            load_smoothing_s: contract.energy.load_smoothing_s,
            torque_curve: impulse::default_torque_curve(),
        }
    }
}

fn firing_phases(contract: &AudioPowertrainSynthesis) -> [f64; 5] {
    contract
        .firing_phases_deg
        .as_deref()
        .filter(|phases| phases.len() == 5)
        .map(|phases| {
            let mut out = DEFAULT_FIRING_PHASES_DEG;
            for (slot, phase) in out.iter_mut().zip(phases.iter()) {
                *slot = *phase as f64;
            }
            out
        })
        .unwrap_or(DEFAULT_FIRING_PHASES_DEG)
}

pub struct CylState {
    phase_deg: f64,
    increment: f64,
    jitter: EventJitter,
    next_event_phase: f64,
    firing_phase: f64,
    event_index: u64,
    events_fired: u64,
    impulse_env: f32,
    impulse_amp: f32,
    attack_samples: u32,
    attack_rem: u32,
}

impl CylState {
    fn new(firing_phase: f64, jitter: EventJitter, attack_samples: u32) -> Self {
        Self {
            phase_deg: 0.0,
            increment: 0.0,
            jitter,
            next_event_phase: firing_phase,
            firing_phase,
            event_index: 0,
            events_fired: 0,
            impulse_env: 0.0,
            impulse_amp: 0.0,
            attack_samples,
            attack_rem: 0,
        }
    }

    pub fn phase_deg(&self) -> f64 {
        self.phase_deg
    }

    pub fn events_fired(&self) -> u64 {
        self.events_fired
    }

    fn step(&mut self, amp: f32, decay_alpha: f32) -> f32 {
        self.phase_deg += self.increment;
        if self.phase_deg >= self.next_event_phase {
            self.events_fired += 1;
            self.event_index += 1;
            let jitter = self.jitter.next_offset();
            self.next_event_phase =
                self.firing_phase + self.event_index as f64 * CYCLE_DEG + jitter;
            self.impulse_amp = amp;
            self.attack_rem = self.attack_samples;
        }
        if self.attack_rem > 0 {
            self.attack_rem -= 1;
            let t = (self.attack_samples - self.attack_rem) as f32 / self.attack_samples as f32;
            self.impulse_env = t;
        } else {
            self.impulse_env *= decay_alpha;
        }
        self.impulse_env * self.impulse_amp
    }
}

pub struct HalfBlock {
    cylinders: [CylState; 5],
    body_filter: Biquad,
    sample_rate: f64,
    torque_curve: Vec<(f64, f64)>,
    torque_curve_weight: f32,
    throttle_response: f32,
    attack_smoothing_s: f32,
    release_smoothing_s: f32,
    load_smoothing_s: f32,
    attack_alpha: f32,
    release_alpha: f32,
    load_alpha: f32,
    decay_alpha: f32,
    target_load: f32,
    smoothed_load: f32,
    energy: f32,
}

impl HalfBlock {
    pub fn new(config: &PowertrainConfig, sample_rate: u32) -> Self {
        let sr = sample_rate as f64;
        let attack_samples = ((IMPULSE_ATTACK_S * sr).round() as u32).max(1);
        let jitter_amp_deg = (config.irregularity as f64).clamp(0.0, 0.25) * CYCLE_DEG;
        let cylinders: [CylState; 5] = std::array::from_fn(|index| {
            CylState::new(
                config.firing_phases_deg[index],
                EventJitter::new(
                    config
                        .seed
                        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                        .wrapping_add(index as u64 + 1),
                    jitter_amp_deg,
                ),
                attack_samples,
            )
        });
        Self {
            cylinders,
            body_filter: Biquad::lowpass(sample_rate as f32, BODY_LOWPASS_HZ),
            sample_rate: sr,
            torque_curve: config.torque_curve.clone(),
            torque_curve_weight: config.torque_curve_weight,
            throttle_response: config.throttle_response,
            attack_smoothing_s: config.attack_smoothing_s,
            release_smoothing_s: config.release_smoothing_s,
            load_smoothing_s: config.load_smoothing_s,
            attack_alpha: 0.0,
            release_alpha: 0.0,
            load_alpha: 0.0,
            decay_alpha: (-1.0 / (sr * IMPULSE_DECAY_S)).exp() as f32,
            target_load: 0.0,
            smoothed_load: 0.0,
            energy: 0.0,
        }
    }

    pub fn cylinder_count(&self) -> usize {
        self.cylinders.len()
    }

    pub fn phase_deg(&self) -> f64 {
        self.cylinders[0].phase_deg()
    }

    pub fn events_fired(&self) -> u64 {
        self.cylinders.iter().map(|c| c.events_fired()).sum()
    }

    pub fn energy(&self) -> f32 {
        self.energy
    }

    pub fn update_controls(&mut self, rpm: f64, idle_rpm: f64, max_rpm: f64, throttle: f32) {
        let norm = if max_rpm > idle_rpm {
            ((rpm - idle_rpm) / (max_rpm - idle_rpm)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let torque = torque_curve_value(&self.torque_curve, norm);
        let weighted =
            self.torque_curve_weight as f64 * torque + (1.0 - self.torque_curve_weight as f64);
        self.target_load = (throttle.clamp(0.0, 1.0) as f64 * weighted) as f32;
        let increment = (rpm.max(0.0) / 120.0 * CYCLE_DEG) / self.sample_rate;
        for cylinder in &mut self.cylinders {
            cylinder.increment = increment;
        }
        self.load_alpha = one_pole_alpha(self.sample_rate, self.load_smoothing_s as f64);
        self.attack_alpha = one_pole_alpha(self.sample_rate, self.attack_smoothing_s as f64);
        self.release_alpha = one_pole_alpha(self.sample_rate, self.release_smoothing_s as f64);
    }

    pub fn render_sample(&mut self) -> f32 {
        self.smoothed_load += (self.target_load - self.smoothed_load) * self.load_alpha;
        let target_energy = self.throttle_response * self.smoothed_load;
        let alpha = if target_energy > self.energy {
            self.attack_alpha
        } else {
            self.release_alpha
        };
        self.energy += (target_energy - self.energy) * alpha;
        let mut excitation = 0.0f32;
        for cylinder in &mut self.cylinders {
            excitation += cylinder.step(self.energy, self.decay_alpha);
        }
        self.body_filter.process(excitation * IMPULSE_LEVEL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PowertrainConfig {
        PowertrainConfig::default()
    }

    #[test]
    fn half_block_has_exactly_five_cylinder_states() {
        let block = HalfBlock::new(&config(), 44100);
        assert_eq!(block.cylinder_count(), 5);
        let per = std::mem::size_of::<CylState>();
        let array = std::mem::size_of::<[CylState; 5]>();
        assert_eq!(array, per * 5);
    }

    #[test]
    fn event_cadence_matches_bank_rate_at_9000_rpm() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..5 * 44100 {
            block.render_sample();
        }
        let expected = 5.0 * 9000.0 / 24.0;
        let got = block.events_fired() as f64;
        assert!(
            (got - expected).abs() / expected < 0.02,
            "event rate off: got {got}, expected ±2% of {expected}"
        );
    }

    #[test]
    fn phase_is_continuous_across_render_blocks() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(12000.0, 1000.0, 15000.0, 0.6);
        for _ in 0..512 {
            block.render_sample();
        }
        let mid = block.phase_deg();
        for _ in 0..512 {
            block.render_sample();
        }
        let end = block.phase_deg();
        let increment = 12000.0 / 120.0 * CYCLE_DEG / 44100.0;
        assert!(
            (mid - 512.0 * increment).abs() < 1e-6,
            "first block phase {mid} vs {}",
            512.0 * increment
        );
        assert!(
            (end - 1024.0 * increment).abs() < 1e-6,
            "second block phase {end} vs {}",
            1024.0 * increment
        );
    }

    #[test]
    fn identical_configs_render_identical_samples() {
        let cfg = config();
        let mut a = HalfBlock::new(&cfg, 44100);
        let mut b = HalfBlock::new(&cfg, 44100);
        for block in [&mut a, &mut b] {
            block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        }
        let samples_a: Vec<f32> = (0..2048).map(|_| a.render_sample()).collect();
        let samples_b: Vec<f32> = (0..2048).map(|_| b.render_sample()).collect();
        assert_eq!(samples_a, samples_b);
    }

    #[test]
    fn different_seeds_render_different_samples() {
        let mut cfg = config();
        cfg.seed = 1234;
        let mut seeded = HalfBlock::new(&cfg, 44100);
        let mut plain = HalfBlock::new(&config(), 44100);
        for block in [&mut seeded, &mut plain] {
            block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        }
        let samples_seeded: Vec<f32> = (0..2048).map(|_| seeded.render_sample()).collect();
        let samples_plain: Vec<f32> = (0..2048).map(|_| plain.render_sample()).collect();
        assert_ne!(samples_seeded, samples_plain);
    }

    #[test]
    fn output_is_finite_and_bounded_at_full_energy() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let mut peak = 0.0f32;
        for _ in 0..4096 {
            let sample = block.render_sample();
            assert!(sample.is_finite());
            peak = peak.max(sample.abs());
        }
        assert!(peak < 1.0, "impulse exceeds unity: peak={peak}");
        assert!(peak > 0.0, "no audible output");
    }

    #[test]
    fn energy_follows_throttle_and_settles_high() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..44100 {
            block.render_sample();
        }
        let loaded = block.energy();
        assert!(loaded > 0.5, "energy too low at full throttle: {loaded}");
        block.update_controls(9000.0, 1000.0, 15000.0, 0.0);
        for _ in 0..44100 {
            block.render_sample();
        }
        assert!(block.energy() < loaded * 0.2, "throttle cut not releasing");
    }

    #[test]
    fn contract_maps_into_powertrain_config() {
        let mut contract = AudioPowertrainSynthesis::default();
        contract.combustion.irregularity = 0.03;
        contract.combustion.seed = 99;
        contract.energy.throttle_response = 1.4;
        contract.firing_phases_deg = Some(vec![0.0, 144.0, 288.0, 432.0, 576.0]);
        let config = PowertrainConfig::from(&contract);
        assert_eq!(config.firing_phases_deg, DEFAULT_FIRING_PHASES_DEG);
        assert_eq!(config.irregularity, 0.03);
        assert_eq!(config.seed, 99);
        assert_eq!(config.throttle_response, 1.4);
        assert!(config.torque_curve.len() >= 2);
    }
}
