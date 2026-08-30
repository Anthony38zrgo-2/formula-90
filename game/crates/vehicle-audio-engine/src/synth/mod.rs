pub mod event_gen;
pub mod exhaust;
pub mod half_block_reconstruct;
pub mod impulse;
pub mod intake;
pub mod limiter_tc;

use crate::dsp::biquad::Biquad;
use crate::powertrain::{
    AudioPowertrainSynthesis, ExhaustConfig, HalfBlockConfig, IntakeConfig, LimiterConfig,
    QualityProfile, TcConfig,
};

use event_gen::EventJitter;
use exhaust::ExhaustSynth;
use half_block_reconstruct::HalfBlockReconstruct;
use impulse::{
    one_pole_alpha, torque_curve_value, IMPULSE_ATTACK_S, IMPULSE_DECAY_S, IMPULSE_LEVEL,
    SCAVENGE_DECAY_S,
};
use intake::IntakeSynth;
use limiter_tc::{tc_alpha, LimiterState, TcEnvelope};

pub const CYCLE_DEG: f64 = 720.0;
pub const DEFAULT_FIRING_PHASES_DEG: [f64; 5] = [0.0, 144.0, 288.0, 432.0, 576.0];

/// Distance-based detail level of the procedural engine (LOD). Higher LODs run
/// a reduced DSP model (fewer resonators, coarser control updates) and the
/// farthest level keeps only the mechanical phase ring alive without rendering
/// audio. Purely a DSP-cost selector: it never changes the modelled timbre
/// beyond limiting how many resonances are actually processed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LodLevel {
    Near,
    Mid,
    Far,
    Virtual,
}

impl LodLevel {
    #[inline]
    fn level_index(self) -> u8 {
        match self {
            LodLevel::Near => 0,
            LodLevel::Mid => 1,
            LodLevel::Far => 2,
            LodLevel::Virtual => 3,
        }
    }

    #[inline]
    fn from_index(index: u8) -> LodLevel {
        match index {
            0 => LodLevel::Near,
            1 => LodLevel::Mid,
            2 => LodLevel::Far,
            _ => LodLevel::Virtual,
        }
    }
}

/// Select the LOD from a listener distance using direction-aware hysteresis.
///
/// The ascending-distance boundaries are `near_max_m`, `mid_max_m`,
/// `far_max_m`. To *raise* the level (move farther away) a boundary is crossed
/// when `distance >= boundary * (1 + hysteresis_ratio)`; to *lower* the level
/// (move closer) it is crossed when `distance < boundary * (1 - hysteresis_ratio)`.
/// This leaves a stable band of half-width `boundary * hysteresis_ratio` around
/// each boundary where the current level is retained, preventing flapping.
pub fn select_lod(
    distance: f32,
    current: LodLevel,
    near_max_m: f32,
    mid_max_m: f32,
    far_max_m: f32,
    hysteresis_ratio: f32,
) -> LodLevel {
    if !distance.is_finite() {
        return current;
    }
    let up = 1.0 + hysteresis_ratio.max(0.0);
    let down = 1.0 - hysteresis_ratio.clamp(0.0, 0.9);
    let mut raw = current.level_index();
    // Move up (farther) one level at a time; the expanded upper boundary must be
    // crossed. `far_max_m` is the boundary into Virtual.
    loop {
        let break_boundary = match raw {
            0 => Some(near_max_m),
            1 => Some(mid_max_m),
            2 => Some(far_max_m),
            _ => None,
        };
        match break_boundary {
            Some(bound) if distance >= bound * up => raw = raw.saturating_add(1),
            _ => break,
        }
    }
    // Move down (closer) one level at a time; the contracted lower boundary must
    // be crossed.
    loop {
        let break_boundary = match raw {
            0 => None,
            1 => Some(near_max_m),
            2 => Some(mid_max_m),
            _ => Some(far_max_m),
        };
        match break_boundary {
            Some(bound) if distance < bound * down => raw = raw.saturating_sub(1),
            _ => break,
        }
    }
    LodLevel::from_index(raw)
}

pub struct PowertrainConfig {
    pub firing_phases_deg: [f64; 5],
    pub irregularity: f32,
    pub seed: u64,
    pub body_cutoff_hz: f32,
    pub scavenging_ratio: f32,
    pub torque_curve_weight: f32,
    pub throttle_response: f32,
    pub idle_combustion_gain: f32,
    pub attack_smoothing_s: f32,
    pub release_smoothing_s: f32,
    pub load_smoothing_s: f32,
    pub torque_curve: Vec<(f64, f64)>,
    pub intake: IntakeConfig,
    pub exhaust: ExhaustConfig,
    pub half_block: HalfBlockConfig,
    pub limiter: LimiterConfig,
    pub tc: TcConfig,
    /// Per-LOD DSP quality profile (resonator scales + coefficient update
    /// cadence). Copied directly from the contract's `distance_levels.dsp`.
    pub quality: QualityProfile,
}

impl Default for PowertrainConfig {
    fn default() -> Self {
        Self {
            firing_phases_deg: DEFAULT_FIRING_PHASES_DEG,
            irregularity: 0.01,
            seed: 0xF090_1994_D15C_A11D,
            body_cutoff_hz: 3200.0,
            scavenging_ratio: 0.38,
            torque_curve_weight: 1.0,
            throttle_response: 1.0,
            idle_combustion_gain: 0.08,
            attack_smoothing_s: 0.02,
            release_smoothing_s: 0.12,
            load_smoothing_s: 0.03,
            torque_curve: impulse::default_torque_curve(),
            intake: IntakeConfig::default(),
            exhaust: ExhaustConfig::default(),
            half_block: HalfBlockConfig::default(),
            limiter: LimiterConfig::default(),
            tc: TcConfig::default(),
            quality: QualityProfile::default(),
        }
    }
}

impl From<&AudioPowertrainSynthesis> for PowertrainConfig {
    fn from(contract: &AudioPowertrainSynthesis) -> Self {
        Self {
            firing_phases_deg: firing_phases(contract),
            irregularity: contract.combustion.irregularity,
            seed: contract.combustion.seed,
            body_cutoff_hz: contract.combustion.body_cutoff_hz,
            scavenging_ratio: contract.combustion.scavenging_ratio,
            torque_curve_weight: contract.energy.torque_curve_weight,
            throttle_response: contract.energy.throttle_response,
            idle_combustion_gain: contract.energy.idle_combustion_gain,
            attack_smoothing_s: contract.energy.attack_smoothing_s,
            release_smoothing_s: contract.energy.release_smoothing_s,
            load_smoothing_s: contract.energy.load_smoothing_s,
            torque_curve: impulse::default_torque_curve(),
            intake: contract.intake.clone(),
            exhaust: contract.exhaust.clone(),
            half_block: contract.half_block.clone(),
            limiter: contract.limiter.clone(),
            tc: contract.tc.clone(),
            quality: contract.distance_levels.dsp.clone(),
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
    scavenge_env: f32,
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
            scavenge_env: 0.0,
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

    fn step(
        &mut self,
        amp: f32,
        decay_alpha: f32,
        scavenge_decay_alpha: f32,
        scavenging_ratio: f32,
    ) -> f32 {
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
            self.scavenge_env = t;
        } else {
            self.impulse_env *= decay_alpha;
            self.scavenge_env *= scavenge_decay_alpha;
        }
        (self.impulse_env - scavenging_ratio * self.scavenge_env) * self.impulse_amp
    }
}

pub struct HalfBlock {
    cylinders: [CylState; 5],
    body_filter: Biquad,
    excitation_dc_block: Biquad,
    sample_rate: f64,
    torque_curve: Vec<(f64, f64)>,
    torque_curve_weight: f32,
    throttle_response: f32,
    idle_combustion_gain: f32,
    attack_smoothing_s: f32,
    release_smoothing_s: f32,
    load_smoothing_s: f32,
    attack_alpha: f32,
    release_alpha: f32,
    load_alpha: f32,
    decay_alpha: f32,
    scavenge_decay_alpha: f32,
    scavenging_ratio: f32,
    target_load: f32,
    smoothed_load: f32,
    energy: f32,
    reconstruct: HalfBlockReconstruct,
    intake: IntakeSynth,
    exhaust: ExhaustSynth,
    current_throttle: f32,
    limiter: LimiterState,
    limiter_config: LimiterConfig,
    limiter_target_cut: f32,
    limiter_attack_alpha: f32,
    limiter_release_alpha: f32,
    limiter_enabled: bool,
    tc: TcEnvelope,
    tc_config: TcConfig,
    tc_attack_alpha: f32,
    tc_release_alpha: f32,
    lod_quality: QualityProfile,
    coeff_update_steps: u32,
    coeff_update_counter: u32,
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
            body_filter: Biquad::lowpass(sample_rate as f32, config.body_cutoff_hz),
            excitation_dc_block: Biquad::highpass(sample_rate as f32, 20.0),
            sample_rate: sr,
            torque_curve: config.torque_curve.clone(),
            torque_curve_weight: config.torque_curve_weight,
            throttle_response: config.throttle_response,
            idle_combustion_gain: config.idle_combustion_gain,
            attack_smoothing_s: config.attack_smoothing_s,
            release_smoothing_s: config.release_smoothing_s,
            load_smoothing_s: config.load_smoothing_s,
            attack_alpha: 0.0,
            release_alpha: 0.0,
            load_alpha: 0.0,
            decay_alpha: (-1.0 / (sr * IMPULSE_DECAY_S)).exp() as f32,
            scavenge_decay_alpha: (-1.0 / (sr * SCAVENGE_DECAY_S)).exp() as f32,
            scavenging_ratio: config.scavenging_ratio.clamp(0.0, 0.9),
            target_load: 0.0,
            smoothed_load: 0.0,
            energy: 0.0,
            reconstruct: HalfBlockReconstruct::new(
                sample_rate,
                &config.half_block,
                config.body_cutoff_hz,
            ),
            intake: IntakeSynth::new(&config.intake, sample_rate as f32, config.seed),
            exhaust: ExhaustSynth::new(&config.exhaust, sample_rate as f32),
            current_throttle: 0.0,
            limiter: LimiterState::new(),
            limiter_config: config.limiter.clone(),
            limiter_target_cut: 0.0,
            limiter_attack_alpha: 0.0,
            limiter_release_alpha: 0.0,
            limiter_enabled: true,
            tc: TcEnvelope::new(&config.tc),
            tc_config: config.tc.clone(),
            tc_attack_alpha: 0.0,
            tc_release_alpha: 0.0,
            lod_quality: config.quality.clone(),
            coeff_update_steps: config.quality.coeff_update_steps.max(1),
            coeff_update_counter: 0,
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
        // RPM-dependent coefficients are re-derived only every `coeff_update_steps`
        // control blocks (LOD quality): at Near this is every block (1), at Far it
        // is every `far_coeff_update_steps` blocks. The envelope/limiter alphas are
        // fixed time constants and are refreshed every block regardless.
        if self.coeff_update_counter == 0 {
            let norm = if max_rpm > idle_rpm {
                ((rpm - idle_rpm) / (max_rpm - idle_rpm)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let torque = torque_curve_value(&self.torque_curve, norm);
            let weighted =
                self.torque_curve_weight as f64 * torque + (1.0 - self.torque_curve_weight as f64);
            let throttle = throttle.clamp(0.0, 1.0) as f64;
            let idle = self.idle_combustion_gain.clamp(0.0, 0.5) as f64;
            let combustion = idle + throttle * (1.0 - idle);
            self.target_load = (combustion * weighted) as f32;
            let increment = (rpm.max(0.0) / 120.0 * CYCLE_DEG) / self.sample_rate;
            self.reconstruct.update(increment);
            for cylinder in &mut self.cylinders {
                cylinder.increment = increment;
            }
            // Limiter gate + cut target are control-time; the smoothed envelope
            // advances per sample so entering/exiting the cut is click-free.
            if self.limiter_enabled {
                self.limiter_target_cut = self.limiter.evaluate(rpm, max_rpm, &self.limiter_config);
            } else {
                self.limiter.active = false;
                self.limiter_target_cut = 0.0;
            }
        }
        self.current_throttle = throttle.clamp(0.0, 1.0);
        self.load_alpha = one_pole_alpha(self.sample_rate, self.load_smoothing_s as f64);
        self.attack_alpha = one_pole_alpha(self.sample_rate, self.attack_smoothing_s as f64);
        self.release_alpha = one_pole_alpha(self.sample_rate, self.release_smoothing_s as f64);
        self.limiter_attack_alpha = one_pole_alpha(
            self.sample_rate,
            self.limiter_config.attack_ms as f64 / 1000.0,
        );
        self.limiter_release_alpha = one_pole_alpha(
            self.sample_rate,
            self.limiter_config.release_ms as f64 / 1000.0,
        );
        self.tc_attack_alpha = tc_alpha(self.sample_rate, self.tc_config.attack_ms);
        self.tc_release_alpha = tc_alpha(self.sample_rate, self.tc_config.release_ms);
        self.coeff_update_counter = (self.coeff_update_counter + 1) % self.coeff_update_steps;
    }

    /// Set the DSP detail level. Propagates the per-LOD resonator scale into the
    /// intake/exhaust layers (fewer Biquads actually processed) and switches the
    /// coefficient update cadence. Resets the update counter so the first block
    /// after a change re-derives the RPM-dependent coefficients.
    pub fn set_lod(&mut self, level: LodLevel) {
        let scale = match level {
            LodLevel::Near => self.lod_quality.near_resonator_scale,
            LodLevel::Mid => self.lod_quality.mid_resonator_scale,
            LodLevel::Far => self.lod_quality.far_resonator_scale,
            LodLevel::Virtual => 0.0,
        }
        .clamp(0.0, 1.0);
        self.intake.set_resonator_scale(scale);
        self.exhaust.set_resonator_scale(scale);
        self.coeff_update_steps = match level {
            LodLevel::Near | LodLevel::Mid => self.lod_quality.coeff_update_steps.max(1),
            LodLevel::Far | LodLevel::Virtual => self.lod_quality.far_coeff_update_steps.max(1),
        };
        self.coeff_update_counter = 0;
    }

    /// Advance only the mechanical phase accumulator ring (Virtual LOD). This
    /// keeps the firing/phase state coherent so re-entering an audible level
    /// resumes without a phonic discontinuity, while never rendering audio (and
    /// never mixing). It is sample-rate cheap: a single increment per cylinder
    /// plus the phase-into-event ring update.
    pub fn update_phase(&mut self) {
        let increment = self.cylinders[0].increment;
        if !(increment > 0.0) {
            return;
        }
        for cylinder in &mut self.cylinders {
            cylinder.phase_deg += increment;
            // Keep the event ring ahead of the phase without firing the impulse
            // envelope, so re-entry does not trigger a stale catch-up event.
            while cylinder.phase_deg >= cylinder.next_event_phase {
                cylinder.events_fired += 1;
                cylinder.event_index += 1;
                let jitter = cylinder.jitter.next_offset();
                cylinder.next_event_phase =
                    cylinder.firing_phase + cylinder.event_index as f64 * CYCLE_DEG + jitter;
            }
        }
        self.reconstruct.update(increment);
    }

    /// Resonator scale currently applied (0.0 at Virtual, 1.0 at Near).
    pub fn resonator_scale(&self) -> f32 {
        self.intake.resonator_scale()
    }

    /// Advance the simulated half block one sample and return the raw firing
    /// excitation (before body filtering). Shared by the mono and stereo paths
    /// so both renderers advance the same mechanical state. The limiter and TC
    /// envelopes also advance here: they are post-processing (they never feed
    /// back into the physical energy model) and must be stepped exactly once
    /// per rendered sample.
    fn advance_excitation(&mut self) -> f32 {
        self.smoothed_load += (self.target_load - self.smoothed_load) * (1.0 - self.load_alpha);
        let target_energy = self.throttle_response * self.smoothed_load;
        let alpha = if target_energy > self.energy {
            self.attack_alpha
        } else {
            self.release_alpha
        };
        self.energy += (target_energy - self.energy) * (1.0 - alpha);
        let mut excitation = 0.0f32;
        for cylinder in &mut self.cylinders {
            excitation += cylinder.step(
                self.energy,
                self.decay_alpha,
                self.scavenge_decay_alpha,
                self.scavenging_ratio,
            );
        }
        self.limiter.step(
            self.limiter_target_cut,
            self.limiter_attack_alpha,
            self.limiter_release_alpha,
        );
        self.tc.step(self.tc_attack_alpha, self.tc_release_alpha);
        // Combustion envelopes are unipolar. Remove accumulated DC before the
        // signal reaches body, reconstructed bank and exhaust; otherwise
        // overlapping high-RPM pulses collapse into a saturated plateau.
        self.excitation_dc_block.process(excitation)
    }

    pub fn render_sample(&mut self) -> f32 {
        let excitation = self.advance_excitation();
        let raw = excitation * IMPULSE_LEVEL;
        let body = self.body_filter.process(raw);
        // Limiter "apertura" sin recalcular el Biquad: mientras corta, el
        // tono de cuerpo se mezcla con la excitacion seca (brillo) y la
        // ganancia de corte suprime la senal.
        (body + self.limiter.air_amount() * (raw - body)) * self.limiter.gain() * self.tc.gain()
    }

    /// Render one stereo frame: the simulated bank plus the derived second
    /// bank, with the intake and exhaust layers mixed equally into both
    /// channels.
    pub fn render_stereo(&mut self) -> (f32, f32) {
        let excitation = self.advance_excitation();
        let (bank1, bank2) =
            self.reconstruct
                .process(excitation, &mut self.body_filter, IMPULSE_LEVEL);
        let intake = self
            .intake
            .process(excitation, self.current_throttle, self.smoothed_load);
        let exhaust = self.exhaust.process(excitation);
        // Same limiter treatment as the mono path: air mix over each bank's
        // own body tone (Biquad stays fixed) and one gain for suppression.
        let raw = excitation * IMPULSE_LEVEL;
        let air = self.limiter.air_amount();
        let cut = self.limiter.gain() * self.tc.gain();
        let mut left = bank1 + air * (raw - bank1);
        let mut right = bank2 + air * (raw - bank2);
        left = (left + intake + exhaust) * cut;
        right = (right + intake + exhaust) * cut;
        (left, right)
    }

    /// Current smoothed load [0.0, 1.0] (for intake gating diagnostics).
    pub fn load(&self) -> f32 {
        self.smoothed_load
    }

    /// Set the traction-control cut ratio [0.0, 1.0]. Clamped; the effective
    /// suppression is smoothed with the profile attack/release taus and a
    /// dead zone below `TcConfig::min_cut_threshold` (ABI parameter exposed by
    /// the next commit).
    pub fn set_tc_cut_ratio(&mut self, ratio: f32) {
        self.tc.set_target(ratio);
    }

    /// Enable/disable the RPM-limiter hard gate. While disabled the limiter
    /// target is forced open (no suppression and no pre-cut ramp).
    pub fn set_limiter_enabled(&mut self, enabled: bool) {
        self.limiter_enabled = enabled;
    }

    /// Current smoothed traction-control suppression [0.0, 1.0].
    pub fn tc_suppression(&self) -> f32 {
        self.tc.suppression()
    }

    /// Whether the RPM limiter hard gate is active (rpm >= threshold).
    pub fn limiter_active(&self) -> bool {
        self.limiter.active
    }

    /// Current smoothed limiter cut depth [0.0, 1.0].
    pub fn limiter_cut(&self) -> f32 {
        self.limiter.cut_smoothed
    }

    /// Sample-domain offset of the second bank's firing phase (see
    /// `HalfBlockReconstruct::offset_samples`).
    pub fn reconstruct_offset_samples(&self) -> f32 {
        self.reconstruct.offset_samples()
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
    fn energy_attack_uses_declared_time_constant() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let _ = block.render_sample();
        assert!(
            block.energy() > 0.0 && block.energy() < 0.01,
            "energy stepped instead of smoothing: {}",
            block.energy()
        );
    }

    #[test]
    fn closed_throttle_idle_keeps_combustion_audible() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(1000.0, 1000.0, 15000.0, 0.0);
        let mut signal = vec![0.0f32; 44100];
        for sample in &mut signal {
            *sample = block.render_sample();
        }
        assert!(block.energy() > 0.001, "idle energy collapsed to zero");
        assert!(rms(&signal[22050..]) > 1e-5, "idle output is silent");
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
        assert_eq!(config.intake, contract.intake);
        assert_eq!(config.exhaust, contract.exhaust);
        assert_eq!(config.half_block, contract.half_block);
    }

    fn pearson(a: &[f32], b: &[f32]) -> f32 {
        let n = a.len().min(b.len()).max(1);
        let mut mean_a = 0.0f64;
        let mut mean_b = 0.0f64;
        for i in 0..n {
            mean_a += a[i] as f64;
            mean_b += b[i] as f64;
        }
        mean_a /= n as f64;
        mean_b /= n as f64;
        let (mut cov, mut var_a, mut var_b) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..n {
            let da = a[i] as f64 - mean_a;
            let db = b[i] as f64 - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }
        (cov / (var_a * var_b).sqrt()) as f32
    }

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|v| v * v).sum::<f32>() / buf.len().max(1) as f32).sqrt()
    }

    #[test]
    fn render_stereo_keeps_five_cylinder_states() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..512 {
            let _ = block.render_stereo();
        }
        assert_eq!(block.cylinder_count(), 5);
    }

    #[test]
    fn reconstruct_offset_matches_phase_offset_at_rpm() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let increment = (9000.0 / 120.0 * CYCLE_DEG) / 44100.0;
        let expected = 72.0f32 / increment as f32;
        assert!(
            (block.reconstruct_offset_samples() - expected).abs() < 1e-3,
            "offset {} vs {expected}",
            block.reconstruct_offset_samples()
        );
        // Idle / zero RPM must not produce a run-away offset.
        block.update_controls(0.0, 1000.0, 15000.0, 0.0);
        assert_eq!(block.reconstruct_offset_samples(), 0.0);
    }

    #[test]
    fn render_stereo_channels_are_decorrelated_by_default() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let mut left = Vec::with_capacity(8192);
        let mut right = Vec::with_capacity(8192);
        for _ in 0..8192 {
            let (l, r) = block.render_stereo();
            left.push(l);
            right.push(r);
        }
        let corr = pearson(&left[4096..], &right[4096..]);
        assert!(
            corr < 0.999,
            "channels must decorrelate mechanically: corr={corr}"
        );
    }

    #[test]
    fn render_stereo_is_identical_with_reconstruction_neutralized() {
        let mut cfg = config();
        cfg.half_block.phase_offset_deg = 0.0;
        cfg.half_block.delay_s = 0.0;
        cfg.half_block.decorrelation = 0.0;
        cfg.half_block.gain = 1.0;
        cfg.half_block.timbre_diff = 0.0;
        let mut block = HalfBlock::new(&cfg, 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..4096 {
            let (l, r) = block.render_stereo();
            assert!(
                (l - r).abs() < 1e-6,
                "identical banks must render identical channels: {l} vs {r}"
            );
        }
    }

    #[test]
    fn render_stereo_energy_is_continuous_across_rpm_ramp() {
        let mut block = HalfBlock::new(&config(), 44100);
        let mut previous = 0.0f32;
        for step in 0..16 {
            let rpm = 3000.0 + step as f64 * (6000.0 / 16.0);
            block.update_controls(rpm, 1000.0, 15000.0, 1.0);
            for _ in 0..512 {
                let (l, r) = block.render_stereo();
                assert!(l.is_finite() && r.is_finite());
            }
            let energy = block.energy();
            let delta = (energy - previous).abs();
            assert!(
                delta < 0.25,
                "energy jumped across RPM step: {previous} -> {energy}"
            );
            previous = energy;
        }
    }

    #[test]
    fn limiter_enters_smoothly_no_click() {
        let mut block = HalfBlock::new(&config(), 44100);
        // Threshold is 0.995 * 15000 = 14925; 14000 sits below even the soft
        // pre-cut band (starts at 14178.75) so the baseline has no cut at all.
        block.update_controls(14000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..8192 {
            let _ = block.render_stereo();
        }
        // Baseline: máximo paso sample a sample bajo el umbral.
        let mut peak_step_below = 0.0f32;
        let mut previous = 0.0f32;
        for _ in 0..4096 {
            let (l, _) = block.render_stereo();
            assert!(l.is_finite(), "señal no finita bajo el umbral");
            peak_step_below = peak_step_below.max((l - previous).abs());
            previous = l;
        }
        assert!(!block.limiter_active());

        // Entrada: el cut suavizado no debe introducir un salto.
        block.update_controls(15000.0, 1000.0, 15000.0, 1.0);
        assert!(block.limiter_active());
        let mut peak_step_enter = 0.0f32;
        for _ in 0..4096 {
            let (l, _) = block.render_stereo();
            assert!(l.is_finite(), "señal no finita en la entrada del limiter");
            peak_step_enter = peak_step_enter.max((l - previous).abs());
            previous = l;
        }
        assert!(
            peak_step_enter < peak_step_below * 2.0 + 0.05,
            "limiter entro con click: paso {peak_step_enter} vs suelo {peak_step_below}"
        );
        assert!(
            block.limiter_cut() > 0.9,
            "cut no se asento: {}",
            block.limiter_cut()
        );

        // Salida: mismo criterio al volver por debajo del umbral.
        block.update_controls(14000.0, 1000.0, 15000.0, 1.0);
        assert!(!block.limiter_active());
        let mut peak_step_exit = 0.0f32;
        for _ in 0..4096 {
            let (l, _) = block.render_stereo();
            assert!(l.is_finite(), "señal no finita en la salida del limiter");
            peak_step_exit = peak_step_exit.max((l - previous).abs());
            previous = l;
        }
        assert!(
            peak_step_exit < peak_step_below * 2.0 + 0.05,
            "limiter salio con click: paso {peak_step_exit} vs suelo {peak_step_below}"
        );
    }

    #[test]
    fn limiter_reduces_gain_above_threshold() {
        let mut open = HalfBlock::new(&config(), 44100);
        let mut limited = HalfBlock::new(&config(), 44100);
        open.set_limiter_enabled(false);
        open.update_controls(15000.0, 1000.0, 15000.0, 1.0);
        limited.update_controls(15000.0, 1000.0, 15000.0, 1.0);
        let mut reference = vec![0.0f32; 8192];
        let mut above = vec![0.0f32; 8192];
        for (dry, cut) in reference.iter_mut().zip(&mut above) {
            *dry = open.render_sample();
            *cut = limited.render_sample();
        }
        let rms_reference = rms(&reference[4096..]);
        let rms_above = rms(&above[4096..]);
        assert!(
            above.iter().all(|v| v.is_finite()),
            "valores no finitos con limiter activo"
        );
        assert!(
            rms_above < rms_reference * 0.75,
            "el limiter no suprime a igual RPM: {rms_reference} -> {rms_above}"
        );
        let peak = above.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak < 1.0, "señal sobre unidad con limiter: {peak}");
    }

    #[test]
    fn tc_pulses_are_smoothed() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..8192 {
            let _ = block.render_stereo();
        }
        let mut baseline = vec![0.0f32; 2048];
        for s in &mut baseline {
            *s = block.render_sample();
        }
        let last_baseline = baseline[baseline.len() - 1];
        assert_eq!(block.tc_suppression(), 0.0);

        // Pulso: target 1.0, la primera muestra tras el cambio no salta.
        block.set_tc_cut_ratio(1.0);
        let first_after = block.render_sample();
        assert!(
            (first_after - last_baseline).abs() < 0.10,
            "TC ataco con salto: {last_baseline} -> {first_after}"
        );
        let mut suppressed = vec![0.0f32; 8192];
        for s in &mut suppressed {
            *s = block.render_sample();
        }
        let rms_base = rms(&baseline);
        let rms_sup = rms(&suppressed[4096..]);
        assert!(
            rms_sup < rms_base * 0.80,
            "TC no suprime la senal: {rms_base} -> {rms_sup}"
        );
        assert!(block.tc_suppression() > 0.8, "supresion no asentada");
        assert!(
            suppressed.iter().all(|v| v.is_finite()),
            "valores no finitos con TC activo"
        );

        // Release: de vuelta a 0, sin salto y con recuperacion de energia.
        let last_sup = suppressed[suppressed.len() - 1];
        block.set_tc_cut_ratio(0.0);
        let first_rel = block.render_sample();
        assert!(
            (first_rel - last_sup).abs() < 0.10,
            "TC solto con salto: {last_sup} -> {first_rel}"
        );
        let mut recovered = vec![0.0f32; 16384];
        for s in &mut recovered {
            *s = block.render_sample();
        }
        let rms_rec = rms(&recovered[8192..]);
        assert!(
            rms_rec > rms_sup * 1.30,
            "TC no se recupera tras el pulso: {rms_sup} -> {rms_rec}"
        );
        assert!(
            rms_rec < rms_base * 1.05,
            "recuperacion supera el nivel base: {rms_rec} vs {rms_base}"
        );
    }

    #[test]
    fn tc_ratio_clamps_and_renders_deterministic() {
        let mut a = HalfBlock::new(&config(), 44100);
        let mut b = HalfBlock::new(&config(), 44100);
        a.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        b.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        // Same block, clamped target vs 1.0: identical output (det + clamp).
        a.set_tc_cut_ratio(7.0);
        b.set_tc_cut_ratio(1.0);
        for _ in 0..4096 {
            let (al, _) = a.render_stereo();
            let (bl, _) = b.render_stereo();
            assert_eq!(al, bl);
        }
    }

    #[test]
    fn limiter_and_tc_do_not_feed_back_into_energy() {
        let mut engaged = HalfBlock::new(&config(), 44100);
        let mut cut = HalfBlock::new(&config(), 44100);
        engaged.update_controls(15000.0, 1000.0, 15000.0, 1.0);
        cut.update_controls(15000.0, 1000.0, 15000.0, 1.0);
        cut.set_tc_cut_ratio(1.0);
        for _ in 0..8192 {
            let _ = engaged.render_stereo();
            let _ = cut.render_stereo();
        }
        // El corte afecta la salida, no la energia fisica (exposicion):
        // last_synth_energy en el mixer sigue reflejando el proceso.
        assert!(
            (engaged.energy() - cut.energy()).abs() < 1e-6,
            "el corte se colo en la energia: {} vs {}",
            engaged.energy(),
            cut.energy()
        );
        assert!(
            engaged.energy() > 0.5,
            "energia fisica inesperadamente baja"
        );
    }

    #[test]
    fn select_lod_uses_direction_aware_hysteresis() {
        let (near, mid, far) = (25.0f32, 80.0, 200.0);
        // Ascend: a boundary is only crossed at boundary * (1 + ratio).
        assert_eq!(
            select_lod(0.0, LodLevel::Near, near, mid, far, 0.10),
            LodLevel::Near
        );
        assert_eq!(
            select_lod(27.5, LodLevel::Near, near, mid, far, 0.10),
            LodLevel::Mid
        );
        assert_eq!(
            select_lod(88.0, LodLevel::Mid, near, mid, far, 0.10),
            LodLevel::Far
        );
        assert_eq!(
            select_lod(220.0, LodLevel::Far, near, mid, far, 0.10),
            LodLevel::Virtual
        );
        // Descend uses a strict `< boundary * (1 - ratio)`; a value exactly on the
        // (lowered) boundary still retains the current level (no flapping).
        assert_eq!(
            select_lod(22.5, LodLevel::Mid, near, mid, far, 0.10),
            LodLevel::Mid
        );
        assert_eq!(
            select_lod(22.4, LodLevel::Mid, near, mid, far, 0.10),
            LodLevel::Near
        );
        assert_eq!(
            select_lod(72.0, LodLevel::Far, near, mid, far, 0.10),
            LodLevel::Far
        );
        assert_eq!(
            select_lod(71.9, LodLevel::Far, near, mid, far, 0.10),
            LodLevel::Mid
        );
        assert_eq!(
            select_lod(180.0, LodLevel::Virtual, near, mid, far, 0.10),
            LodLevel::Virtual
        );
        assert_eq!(
            select_lod(179.9, LodLevel::Virtual, near, mid, far, 0.10),
            LodLevel::Far
        );
        // Stay-bands: a value inside the hysteresis zone retains the current level.
        assert_eq!(
            select_lod(25.0, LodLevel::Mid, near, mid, far, 0.10),
            LodLevel::Mid
        );
        assert_eq!(
            select_lod(80.0, LodLevel::Far, near, mid, far, 0.10),
            LodLevel::Far
        );
        // Non-finite distance keeps the current level; boundaries are clamped.
        assert_eq!(
            select_lod(f32::NAN, LodLevel::Far, near, mid, far, 0.10),
            LodLevel::Far
        );
        assert_eq!(
            select_lod(10_000.0, LodLevel::Far, near, mid, far, 0.10),
            LodLevel::Virtual
        );
        assert_eq!(
            select_lod(220.0, LodLevel::Virtual, near, mid, far, 0.10),
            LodLevel::Virtual
        );
    }

    #[test]
    fn set_lod_scales_resonators_and_cadence() {
        let mut block = HalfBlock::new(&config(), 44100);
        // Default LOD starts at Near: full resonators, single-step coefs.
        assert_eq!(block.resonator_scale(), 1.0);
        block.set_lod(LodLevel::Virtual);
        assert_eq!(block.resonator_scale(), 0.0);
        let virtual_steps = config().quality.far_coeff_update_steps.max(1);
        assert_eq!(block.coeff_update_steps, virtual_steps);
    }
}
