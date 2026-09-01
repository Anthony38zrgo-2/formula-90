pub mod event_gen;
pub mod exhaust;
pub mod half_block_reconstruct;
pub mod impulse;
pub mod intake;
pub mod limiter_tc;
pub mod modular;
pub mod rasp;

use crate::dsp::biquad::Biquad;
use crate::powertrain::{
    AudioPowertrainSynthesis, CombustionVoicesConfig, ExhaustConfig, HalfBlockConfig, IntakeConfig,
    LimiterConfig, ModularConfig, ModularVoiceConfig, QualityProfile, RaspConfig, TcConfig,
    Waveform,
};

use event_gen::EventJitter;
use exhaust::ExhaustSynth;
use half_block_reconstruct::{BankExcitation as ReconstructedExcitation, HalfBlockReconstruct};
use impulse::{
    one_pole_alpha, torque_curve_value, IMPULSE_DECAY_S, IMPULSE_LEVEL, SCAVENGE_DECAY_S,
};
use intake::IntakeSynth;
use limiter_tc::{tc_alpha, LimiterState, TcEnvelope};
use modular::ModularSynth;
use rasp::RaspSynth;

pub const CYCLE_DEG: f64 = 720.0;
pub const DEFAULT_FIRING_PHASES_DEG: [f64; 5] = [0.0, 144.0, 288.0, 432.0, 576.0];
/// One-pole corner for the smoothed combustion pressure derivative. Keeps the
/// event attack crack while removing the 6 dB/oct overbright fuzz above ~4 kHz.
pub const DERIVATIVE_CUTOFF_HZ: f64 = 3500.0;

/// One-pole corner for the *rasp* excitation. Deliberately far above the body's
/// 3.5 kHz corner: the grit lives in 2.5-7 kHz, so the body envelope (which is
/// low-passed at 3.5 kHz) cannot be its source. High enough to leave the band
/// essentially intact, still a one-pole (cheap) and never a noise generator.
pub const RASP_DERIVATIVE_CUTOFF_HZ: f64 = 16000.0;

/// The continuous engine is combustion-only: intake, exhaust and limiter "air"
/// are disabled for this iteration and must contribute nothing to the mix.
/// The rasp layer is NOT gated here: it follows the profile `rasp.enabled` flag
/// so its timbre can be auditioned without reactivating the other voices.
const COMBUSTION_ONLY: bool = true;

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

#[derive(Clone)]
pub struct PowertrainConfig {
    pub firing_phases_deg: [f64; 5],
    pub irregularity: f32,
    pub seed: u64,
    pub body_cutoff_hz: f32,
    pub scavenging_ratio: f32,
    pub cylinder_variation: f32,
    pub cycle_variation: f32,
    pub pressure_derivative_mix: f32,
    pub combustion_attack_deg: f32,
    pub combustion_decay_deg: f32,
    pub exhaust_open_offset_deg: f32,
    pub exhaust_blowdown_attack_deg: f32,
    pub exhaust_blowdown_decay_deg: f32,
    pub intake_open_offset_deg: f32,
    pub intake_event_width_deg: f32,
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
    pub combustion_voices: CombustionVoicesConfig,
    pub rasp: RaspConfig,
    pub modular: ModularConfig,
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
            cylinder_variation: 0.055,
            cycle_variation: 0.075,
            pressure_derivative_mix: 0.58,
            combustion_attack_deg: 18.0,
            combustion_decay_deg: 110.0,
            exhaust_open_offset_deg: 18.0,
            exhaust_blowdown_attack_deg: 12.0,
            exhaust_blowdown_decay_deg: 150.0,
            intake_open_offset_deg: 36.0,
            intake_event_width_deg: 90.0,
            torque_curve_weight: 1.0,
            throttle_response: 1.0,
            idle_combustion_gain: 0.35,
            attack_smoothing_s: 0.02,
            release_smoothing_s: 0.12,
            load_smoothing_s: 0.03,
            torque_curve: impulse::default_torque_curve(),
            intake: IntakeConfig::default(),
            exhaust: ExhaustConfig::default(),
            half_block: HalfBlockConfig::default(),
            combustion_voices: CombustionVoicesConfig::default(),
            rasp: RaspConfig::default(),
            modular: ModularConfig::default(),
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
            cylinder_variation: contract.combustion.cylinder_variation,
            cycle_variation: contract.combustion.cycle_variation,
            pressure_derivative_mix: contract.combustion.pressure_derivative_mix,
            combustion_attack_deg: contract.combustion.combustion_attack_deg,
            combustion_decay_deg: contract.combustion.combustion_decay_deg,
            exhaust_open_offset_deg: contract.combustion.exhaust_open_offset_deg,
            exhaust_blowdown_attack_deg: contract.combustion.exhaust_blowdown_attack_deg,
            exhaust_blowdown_decay_deg: contract.combustion.exhaust_blowdown_decay_deg,
            intake_open_offset_deg: contract.combustion.intake_open_offset_deg,
            intake_event_width_deg: contract.combustion.intake_event_width_deg,
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
            combustion_voices: contract.combustion_voices.clone(),
            rasp: contract.rasp.clone(),
            modular: contract.modular.clone(),
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
    cylinder_gain: f32,
    cycle_variation: f32,
    last_body_pressure: f32,
    body_derivative_env: f32,
    /// Rasp excitation: combustion-pressure derivative smoothed by its OWN
    /// much higher corner, fed from the raw first difference before the 3.5 kHz
    /// body smoothing. This is what preserves 2.5-7 kHz content; reusing
    /// `body_derivative_env` left nothing above ~3.5 kHz to band-pass.
    rasp_derivative_env: f32,
    rasp_derivative_alpha: f32,
    last_exhaust_pressure: f32,
    exhaust_derivative_env: f32,
    derivative_alpha: f32,
    combustion_event: AngularEventEnvelope,
    exhaust_event: AngularEventEnvelope,
    intake_event: AngularEventEnvelope,
    event_age_deg: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct AngularEventEnvelope {
    pub open_offset_deg: f32,
    pub attack_deg: f32,
    pub hold_deg: f32,
    pub decay_deg: f32,
}

impl AngularEventEnvelope {
    #[inline]
    fn value(self, age_deg: f64) -> f32 {
        if age_deg < self.open_offset_deg.max(0.0) as f64 {
            return 0.0;
        }
        let age = age_deg - self.open_offset_deg.max(0.0) as f64;
        let attack = self.attack_deg.max(1.0) as f64;
        if age < attack {
            return (age / attack) as f32;
        }
        if age < attack + self.hold_deg.max(0.0) as f64 {
            return 1.0;
        }
        (1.0 - (age - attack - self.hold_deg.max(0.0) as f64) / self.decay_deg.max(1.0) as f64)
            .max(0.0) as f32
    }
}

#[derive(Clone, Copy, Default)]
struct CylinderExcitation {
    body: f32,
    intake: f32,
    exhaust: f32,
    /// Event-driven rasp (grit) excitation, derived from the raw combustion
    /// pressure edge (own high corner, see `rasp_derivative_env`) so it is
    /// inherently event-synced and decays between firing events.
    rasp: f32,
    fired: bool,
    crank_phase_deg: f32,
    pressure_peak: f32,
    pressure_derivative_delta: f32,
    event_energy: f32,
    cycle_variation: f32,
}

#[derive(Clone, Copy, Default)]
struct PhysicalCombustion {
    cylinder: u8,
    crank_phase_deg: f32,
    pressure_peak: f32,
    pressure_derivative: f32,
    energy: f32,
    cycle_variation: f32,
}

impl CylState {
    fn new(
        firing_phase: f64,
        jitter: EventJitter,
        cylinder_gain: f32,
        cycle_variation: f32,
        derivative_alpha: f32,
        rasp_derivative_alpha: f32,
        event_params: [f32; 7],
    ) -> Self {
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
            cylinder_gain,
            cycle_variation,
            last_body_pressure: 0.0,
            body_derivative_env: 0.0,
            rasp_derivative_env: 0.0,
            rasp_derivative_alpha,
            last_exhaust_pressure: 0.0,
            exhaust_derivative_env: 0.0,
            derivative_alpha,
            event_age_deg: 1.0e9,
            combustion_event: AngularEventEnvelope {
                open_offset_deg: 0.0,
                attack_deg: event_params[0],
                hold_deg: 0.0,
                decay_deg: event_params[1],
            },
            exhaust_event: AngularEventEnvelope {
                open_offset_deg: event_params[2],
                attack_deg: event_params[3],
                hold_deg: 0.0,
                decay_deg: event_params[4],
            },
            intake_event: AngularEventEnvelope {
                open_offset_deg: event_params[5],
                attack_deg: event_params[6] * 0.2,
                hold_deg: event_params[6] * 0.3,
                decay_deg: event_params[6] * 0.5,
            },
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
        _scavenging_ratio: f32,
        pressure_derivative_mix: f32,
    ) -> CylinderExcitation {
        self.phase_deg += self.increment;
        self.event_age_deg += self.increment;
        let mut fired = false;
        let mut crank_phase_deg = 0.0f32;
        let mut event_cycle_variation = 0.0f32;
        if self.phase_deg >= self.next_event_phase {
            fired = true;
            crank_phase_deg = self.next_event_phase.rem_euclid(CYCLE_DEG) as f32;
            self.events_fired += 1;
            self.event_index += 1;
            let jitter = self.jitter.next_offset();
            self.next_event_phase =
                self.firing_phase + self.event_index as f64 * CYCLE_DEG + jitter;
            let cycle_gain = 1.0 + self.jitter.next_signed() * self.cycle_variation;
            event_cycle_variation = cycle_gain - 1.0;
            self.impulse_amp = amp * self.cylinder_gain * cycle_gain;
            self.event_age_deg = 0.0;
        }
        let x = self.event_age_deg.max(0.0);
        self.impulse_env = self.combustion_event.value(x);
        self.scavenge_env = self.intake_event.value(x);
        // Cheap pressure proxy: rounded crown during the burn plus a slower
        // negative gas-exchange tail. Its derivative supplies broadband attack
        // energy without a sine oscillator or an additional physical cylinder.
        // The raw first difference is 6 dB/oct overbright, so it is smoothed
        // once and weighted to preserve the body (fundamental + low harmonics)
        // instead of dominating the timbre with thin high-range energy.
        let burn = self.impulse_env * (2.0 - self.impulse_env);
        // Combustion, intake flow and exhaust blowdown are independent event
        // channels; intake width must not retune body/exhaust pressure.
        let pressure = burn * self.impulse_amp;
        let raw_body_derivative = pressure - self.last_body_pressure;
        self.last_body_pressure = pressure;
        // Body path unchanged: 3.5 kHz corner keeps the approved mid timbre.
        self.body_derivative_env +=
            (raw_body_derivative - self.body_derivative_env) * self.derivative_alpha;
        // Rasp path: the SAME raw event edge, but smoothed by its own much
        // higher corner. This keeps the 2.5-7 kHz band alive for the rasp
        // band-pass instead of reusing the already-lowpassed body envelope.
        self.rasp_derivative_env +=
            (raw_body_derivative - self.rasp_derivative_env) * self.rasp_derivative_alpha;

        // Blowdown is its own delayed pressure event. Its derivative must not
        // reuse the combustion-pressure derivative, otherwise the exhaust edge
        // exists before the exhaust valve event and the two sources are not
        // independently controllable.
        let exhaust_pressure = self.exhaust_event.value(x) * self.impulse_amp;
        let raw_exhaust_derivative = exhaust_pressure - self.last_exhaust_pressure;
        self.last_exhaust_pressure = exhaust_pressure;
        self.exhaust_derivative_env +=
            (raw_exhaust_derivative - self.exhaust_derivative_env) * self.derivative_alpha;
        let edge = pressure_derivative_mix.clamp(0.0, 1.0);
        CylinderExcitation {
            body: pressure * (1.0 - edge) + self.body_derivative_env * (1.5 * edge),
            intake: self.scavenge_env * self.impulse_amp,
            exhaust: exhaust_pressure * (1.0 - edge) + self.exhaust_derivative_env * (1.5 * edge),
            rasp: self.rasp_derivative_env,
            fired,
            crank_phase_deg,
            pressure_peak: self.impulse_amp,
            // The transport converts this real model delta to pressure units/s.
            // At the exact threshold sample the attack envelope is zero, so use
            // the model's next-sample pressure rather than fabricating a value.
            pressure_derivative_delta: if fired {
                let next_env = self.combustion_event.value(self.increment);
                let next_burn = next_env * (2.0 - next_env);
                next_burn * self.impulse_amp - pressure
            } else {
                raw_body_derivative
            },
            event_energy: amp * self.cylinder_gain,
            cycle_variation: event_cycle_variation,
        }
    }
}

pub struct DiagnosticFrame {
    pub bank_a_raw: f32,
    pub bank_b_raw: f32,
    pub full_block_raw: f32,
    pub body_a: f32,
    pub body_b: f32,
    /// Body excitation *before* the low-pass (per bank). Voice 1 + voice 2 must
    /// reconstruct this exactly.
    pub pre_body_a: f32,
    pub pre_body_b: f32,
    /// Voice 1: approved body path, summed mono (the audible body).
    pub combustion_body: f32,
    /// Voice 2: complementary residual `pre_body - body`, per bank and summed.
    pub combustion_edge_a: f32,
    pub combustion_edge_b: f32,
    pub combustion_edge: f32,
    pub intake_a: f32,
    pub intake_b: f32,
    /// Rasp (grit) excitation after band-limiting + soft saturation, per bank.
    pub rasp_a: f32,
    pub rasp_b: f32,
    /// Post-simulator modular layer (PolyBLEP voices + ADSR), per bank and
    /// summed. Like every layer, both banks stay separate until the final mono
    /// sum.
    pub modular_a: f32,
    pub modular_b: f32,
    pub modular: f32,
    /// Exhaust excitation before the per-bank exhaust DSP/DC guard.
    pub exhaust_raw_a: f32,
    pub exhaust_raw_b: f32,
    pub collector_a: f32,
    pub collector_b: f32,
    pub exhaust_a: f32,
    pub exhaust_b: f32,
    /// Pre-spatial composite mix per bank (body + intake + exhaust + rasp with
    /// identical per-channel layer gains), useful for audit/telemetry.
    /// Mono engine mix (body + intake + exhaust + rasp, both banks summed)
    /// before the DC block and before channel duplication.
    pub mono_engine: f32,
    pub body: f32,
    pub intake: f32,
    pub exhaust: f32,
    pub rasp: f32,
    pub master: (f32, f32),
}

pub struct BankDsp {
    pub body_filter: Biquad,
    pub intake: IntakeSynth,
    pub exhaust: ExhaustSynth,
    pub exhaust_dc: Biquad,
}

impl BankDsp {
    fn new(config: &PowertrainConfig, sample_rate: u32, seed: u64, body_cutoff: f32) -> Self {
        Self {
            body_filter: Biquad::lowpass(sample_rate as f32, body_cutoff),
            intake: IntakeSynth::new(&config.intake, sample_rate as f32, seed),
            exhaust: ExhaustSynth::new(&config.exhaust, sample_rate as f32),
            exhaust_dc: Biquad::highpass(sample_rate as f32, 20.0),
        }
    }
}

/// Mono master stage for the procedural engine.
///
/// The continuous engine is strictly dual-mono: `L` and `R` carry the *same*
/// sample. There is no delay ring, no crossfeed, no decorrelation and no
/// per-channel filter — the two banks are summed per layer before this point,
/// and a single DC block + a single limiter/TC gain are applied once to the
/// summed mono signal. The result is duplicated verbatim, so both channels are
/// bit-identical by construction rather than by balance-matching.
pub struct MonoMaster {
    dc: Biquad,
}

impl MonoMaster {
    fn new(sample_rate: u32) -> Self {
        Self {
            dc: Biquad::highpass(sample_rate as f32, 20.0),
        }
    }

    #[inline]
    fn process(&mut self, mono: f32, cut: f32) -> (f32, f32) {
        let out = self.dc.process(mono * cut);
        (out, out)
    }
}

pub struct HalfBlock {
    cylinders: [CylState; 5],
    bank_a_dsp: BankDsp,
    bank_b_dsp: BankDsp,
    excitation_dc_block: Biquad,
    intake_dc_block: Biquad,
    exhaust_pre_dc_blocks: [Biquad; 5],
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
    scavenging_ratio: f32,
    pressure_derivative_mix: f32,
    target_load: f32,
    smoothed_load: f32,
    energy: f32,
    reconstruct: HalfBlockReconstruct,
    rasp_a: RaspSynth,
    rasp_b: RaspSynth,
    rasp_config: RaspConfig,
    modular_engine: ModularSynth,
    modular_config: ModularConfig,
    combustion_voices: CombustionVoicesConfig,
    current_rpm_norm: f32,
    /// LOD gate for the rasp path (Near full, Mid reduced, Far/Virtual off).
    rasp_lod_enabled: bool,
    /// LOD gate for the modular layer (Near/Mid full, Far/Virtual off).
    modular_lod_enabled: bool,
    master: MonoMaster,
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
    last_physical_events: [Option<PhysicalCombustion>; 5],
}

impl HalfBlock {
    pub fn new(config: &PowertrainConfig, sample_rate: u32) -> Self {
        let sr = sample_rate as f64;
        let jitter_amp_deg = (config.irregularity as f64).clamp(0.0, 0.25) * CYCLE_DEG;
        let variation = config.cylinder_variation.clamp(0.0, 0.25);
        let derivative_alpha =
            1.0 - one_pole_alpha(sr, 1.0 / (std::f64::consts::TAU * DERIVATIVE_CUTOFF_HZ));
        let rasp_derivative_alpha = 1.0
            - one_pole_alpha(
                sr,
                1.0 / (std::f64::consts::TAU * RASP_DERIVATIVE_CUTOFF_HZ),
            );
        let cylinders: [CylState; 5] = std::array::from_fn(|index| {
            // Fixed zero-mean spread: manufacturing/header differences remain
            // stable, while the event RNG supplies the slower cycle variation.
            const SPREAD: [f32; 5] = [-0.72, 0.38, 0.91, -0.21, -0.36];
            CylState::new(
                config.firing_phases_deg[index],
                EventJitter::new(
                    config
                        .seed
                        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                        .wrapping_add(index as u64 + 1),
                    jitter_amp_deg,
                ),
                1.0 + SPREAD[index] * variation,
                config.cycle_variation.clamp(0.0, 0.25),
                derivative_alpha,
                rasp_derivative_alpha,
                [
                    config.combustion_attack_deg,
                    config.combustion_decay_deg,
                    config.exhaust_open_offset_deg,
                    config.exhaust_blowdown_attack_deg,
                    config.exhaust_blowdown_decay_deg,
                    config.intake_open_offset_deg,
                    config.intake_event_width_deg,
                ],
            )
        });
        Self {
            cylinders,
            bank_a_dsp: BankDsp::new(config, sample_rate, config.seed, config.body_cutoff_hz),
            // Both bodies are acoustically identical: no lateral cutoff
            // multiplier, so timbre differences can only come from the 72-degree
            // mechanical offset, never from an asymmetric filter.
            bank_b_dsp: BankDsp::new(
                config,
                sample_rate,
                config.seed.wrapping_add(1),
                config.body_cutoff_hz,
            ),
            excitation_dc_block: Biquad::highpass(sample_rate as f32, 20.0),
            intake_dc_block: Biquad::highpass(sample_rate as f32, 20.0),
            exhaust_pre_dc_blocks: std::array::from_fn(|_| {
                Biquad::highpass(sample_rate as f32, 20.0)
            }),
            master: MonoMaster::new(sample_rate),
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
            scavenging_ratio: config.scavenging_ratio.clamp(0.0, 0.9),
            pressure_derivative_mix: config.pressure_derivative_mix.clamp(0.0, 1.0),
            target_load: 0.0,
            smoothed_load: 0.0,
            energy: 0.0,
            reconstruct: HalfBlockReconstruct::new(&config.half_block),
            rasp_a: RaspSynth::new(
                sample_rate,
                config.rasp.highpass_hz,
                config.rasp.lowpass_hz,
                config.rasp.saturation,
            ),
            rasp_b: RaspSynth::new(
                sample_rate,
                config.rasp.highpass_hz,
                config.rasp.lowpass_hz,
                config.rasp.saturation,
            ),
            rasp_config: config.rasp.clone(),
            modular_engine: ModularSynth::new(
                &config.modular,
                sample_rate,
                config.seed,
                config.half_block.phase_offset_deg as f64,
            ),
            modular_config: config.modular.clone(),
            combustion_voices: config.combustion_voices.clone(),
            current_rpm_norm: 0.0,
            rasp_lod_enabled: true,
            modular_lod_enabled: true,
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
            last_physical_events: [None; 5],
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
            // The idle combustion base is kept independent of the torque curve:
            // otherwise torque_curve(0.0) * idle_gain collapses to silence at
            // closed throttle. The torque curve only shapes the throttle part.
            let combustion = idle + throttle * (1.0 - idle) * weighted;
            self.target_load = combustion as f32;
            self.current_rpm_norm = norm as f32;
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
        // Pulse decay compresses with RPM: the single-pulse length is modelled at
        // idle, but a fixed 6 ms decay overlaps into a near-flat (low-crest) sum
        // above ~10k RPM where pulse spacing drops below 2 ms.
        let ratio = (idle_rpm.max(1000.0) / rpm.max(1000.0)).clamp(0.25, 1.0);
        let pulse_decay_s = (IMPULSE_DECAY_S * ratio).clamp(0.0015, IMPULSE_DECAY_S);
        let _ = pulse_decay_s;
        let scavenge_s = (SCAVENGE_DECAY_S * ratio).clamp(0.003, SCAVENGE_DECAY_S);
        let _ = scavenge_s;
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
        self.bank_a_dsp.intake.set_resonator_scale(scale);
        self.bank_a_dsp.exhaust.set_resonator_scale(scale);
        self.bank_b_dsp.intake.set_resonator_scale(scale);
        self.bank_b_dsp.exhaust.set_resonator_scale(scale);
        let distant = matches!(level, LodLevel::Far | LodLevel::Virtual);
        self.bank_a_dsp.exhaust.set_distant(distant);
        self.bank_b_dsp.exhaust.set_distant(distant);
        // Rasp budget: full near, reduced mid, and completely bypassed at
        // Far/Virtual (no Biquads and no tanh in the distant path).
        self.rasp_lod_enabled = !matches!(level, LodLevel::Far | LodLevel::Virtual);
        // Modular budget: same doctrine as rasp (16 biquads + 4 oscillators per
        // bank stay out of the distant path).
        self.modular_lod_enabled = !matches!(level, LodLevel::Far | LodLevel::Virtual);
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
        self.bank_a_dsp.intake.resonator_scale()
    }

    /// Test helper: copy of bank A's body filter (to prove A/B parity).
    #[cfg(test)]
    pub fn clone_body_filter_a(&self) -> Biquad {
        self.bank_a_dsp.body_filter
    }

    /// Test helper: copy of bank B's body filter (to prove A/B parity).
    #[cfg(test)]
    pub fn clone_body_filter_b(&self) -> Biquad {
        self.bank_b_dsp.body_filter
    }

    /// Advance the simulated half block one sample and return the raw firing
    /// excitation (before body filtering). Shared by the mono and stereo paths
    /// so both renderers advance the same mechanical state. The limiter and TC
    /// envelopes also advance here: they are post-processing (they never feed
    /// back into the physical energy model) and must be stepped exactly once
    /// per rendered sample.
    fn advance_excitation(&mut self) -> ReconstructedExcitation {
        self.smoothed_load += (self.target_load - self.smoothed_load) * (1.0 - self.load_alpha);
        let target_energy = self.throttle_response * self.smoothed_load;
        let alpha = if target_energy > self.energy {
            self.attack_alpha
        } else {
            self.release_alpha
        };
        self.energy += (target_energy - self.energy) * (1.0 - alpha);
        let mut excitation = CylinderExcitation::default();
        let mut exhaust_headers = [0.0f32; 5];
        self.last_physical_events = [None; 5];
        for (index, cylinder) in self.cylinders.iter_mut().enumerate() {
            let event = cylinder.step(
                self.energy,
                self.scavenging_ratio,
                self.pressure_derivative_mix,
            );
            excitation.body += event.body;
            excitation.intake += event.intake;
            excitation.exhaust += event.exhaust;
            excitation.rasp += event.rasp * self.rasp_config.input_gain;
            exhaust_headers[index] = event.exhaust;
            if event.fired {
                self.last_physical_events[index] = Some(PhysicalCombustion {
                    cylinder: index as u8,
                    crank_phase_deg: event.crank_phase_deg,
                    pressure_peak: event.pressure_peak,
                    pressure_derivative: event.pressure_derivative_delta * self.sample_rate as f32,
                    energy: event.event_energy,
                    cycle_variation: event.cycle_variation,
                });
            }
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
        excitation.body = self.excitation_dc_block.process(excitation.body);
        excitation.intake = self.intake_dc_block.process(excitation.intake);
        for (index, value) in exhaust_headers.iter_mut().enumerate() {
            *value = self.exhaust_pre_dc_blocks[index].process(*value);
        }
        excitation.exhaust = exhaust_headers.iter().sum();
        ReconstructedExcitation {
            body: excitation.body,
            intake: excitation.intake,
            exhaust: excitation.exhaust,
            rasp: excitation.rasp,
            exhaust_headers,
        }
    }

    pub fn render_sample(&mut self) -> f32 {
        let excitation = self.advance_excitation();
        let raw = excitation.body * IMPULSE_LEVEL;
        let body = self.bank_a_dsp.body_filter.process(raw);
        // Limiter "apertura" sin recalcular el Biquad: mientras corta, el
        // tono de cuerpo se mezcla con la excitacion seca (brillo) y la
        // ganancia de corte suprime la senal.
        (body + self.limiter.air_amount() * (raw - body)) * self.limiter.gain() * self.tc.gain()
    }

    /// Render one frame of the procedural engine.
    ///
    /// Both banks keep independent acoustic processing until the very end, then
    /// every layer is summed to mono. The engine is intentionally dual-mono:
    /// there is no width, crossfeed, delay or per-channel processing anywhere in
    /// this path, so `master.0` and `master.1` are the same value by construction.
    pub fn render_diagnostic_frame(&mut self) -> DiagnosticFrame {
        let raw_excitation = self.advance_excitation();
        let (bank_a, bank_b) = self.reconstruct.process_excitation(raw_excitation);
        let bank1 = self
            .bank_a_dsp
            .body_filter
            .process(bank_a.body * IMPULSE_LEVEL);
        let bank2 = if self.reconstruct.offset_samples() == 0.0 {
            bank1
        } else {
            self.bank_b_dsp
                .body_filter
                .process(bank_b.body * IMPULSE_LEVEL)
        };
        // Voice 2: `combustion_edge` is the complementary residual of the body
        // low-pass (pre_body - body). It is the exact high-pass counterpart of
        // the existing path: no second filter, no tanh, no resonator, no
        // envelope, no RPM/load modulation. Summing body + edge reconstructs
        // pre_body without inter-filter phase error.
        let pre_body_a = bank_a.body * IMPULSE_LEVEL;
        let pre_body_b = bank_b.body * IMPULSE_LEVEL;
        let edge_a = pre_body_a - bank1;
        let edge_b = pre_body_b - bank2;
        let combustion_body = bank1 + bank2;
        let combustion_edge = edge_a + edge_b;
        let edge_gain = if self.combustion_voices.edge_enabled {
            self.combustion_voices.edge_gain
        } else {
            0.0
        };

        // Intake/exhaust are disabled for the combustion-only test. Their synths
        // are not merely configured off internally: they are not called at all,
        // so the outputs below are exact zeros.
        let intake_a = 0.0f32;
        let intake_b = 0.0f32;
        let exhaust_a = 0.0f32;
        let exhaust_b = 0.0f32;
        let intake = intake_a + intake_b;
        let exhaust = exhaust_a + exhaust_b;
        // Event-driven rasp (grit): band-limited combustion-pressure derivative.
        // Gain is RPM/load modulated so it grows toward high/redline while
        // keeping a floor in the mid range. Disabled entirely if !enabled.
        let rasp_gain = if self.rasp_config.enabled {
            RaspSynth::rpm_gain(
                self.rasp_config.gain,
                self.rasp_config.rpm_start_ratio,
                self.rasp_config.rpm_full_ratio,
                self.current_rpm_norm,
            )
        } else {
            0.0
        };
        // Rasp follows the profile `rasp.enabled` flag even in combustion-only
        // mode: it is the only non-body voice allowed to reach the mix while
        // intake/exhaust/air stay hard-disabled.
        let rasp_active = rasp_gain != 0.0 && self.rasp_lod_enabled;
        let rasp_a = if rasp_active {
            self.rasp_a.process(bank_a.rasp)
        } else {
            0.0
        };
        let rasp_b = if rasp_active {
            self.rasp_b.process(bank_b.rasp)
        } else {
            0.0
        };
        // Modular layer: post-simulator PolyBLEP voices + ADSR, keyed by the
        // same bank firing clock. Follows the profile `modular.enabled` flag,
        // its own RPM curve and the simulated energy. All voices are banded, so
        // the layer only fills registers the combustion body lacks.
        let modular_gain = if self.modular_config.enabled {
            ModularSynth::rpm_gain(
                self.modular_config.gain,
                self.modular_config.rpm_start_ratio,
                self.modular_config.rpm_full_ratio,
                self.current_rpm_norm,
            )
        } else {
            0.0
        };
        let modular_active = modular_gain != 0.0 && self.modular_lod_enabled;
        let (modular_a, modular_b) = if modular_active {
            let (a, b) = self
                .modular_engine
                .process(self.cylinders[0].increment, self.energy);
            (a * modular_gain, b * modular_gain)
        } else {
            (0.0, 0.0)
        };
        // The limiter "air" blend re-injects dry excitation while cutting. That
        // would be a *third* sharp voice on top of the two declared ones, so it
        // is held at zero for the combustion-only test: the output is composed
        // of combustion_body + combustion_edge (+ rasp when enabled).
        let air = if COMBUSTION_ONLY {
            0.0
        } else {
            self.limiter.air_amount()
        };
        let body_mono_air = combustion_body + air * ((pre_body_a + pre_body_b) - combustion_body);
        // Body + edge + optional rasp: intake/exhaust/air are forced to zero
        // above; the only global controls left are the engine limiter and TC.
        let rasp_mono = rasp_gain * (rasp_a + rasp_b);
        let modular_mono = modular_a + modular_b;
        let mono_engine = body_mono_air + edge_gain * combustion_edge + rasp_mono + modular_mono;
        debug_assert!(
            intake == 0.0 && exhaust == 0.0,
            "combustion-only mix received a non-combustion voice"
        );
        let cut = self.limiter.gain() * self.tc.gain();
        let master = self.master.process(mono_engine, cut);
        DiagnosticFrame {
            bank_a_raw: bank_a.body,
            bank_b_raw: bank_b.body,
            full_block_raw: bank_a.body + bank_b.body,
            body_a: bank1,
            body_b: bank2,
            pre_body_a,
            pre_body_b,
            combustion_body,
            combustion_edge_a: edge_a,
            combustion_edge_b: edge_b,
            combustion_edge,
            intake_a,
            intake_b,
            rasp_a,
            rasp_b,
            modular_a,
            modular_b,
            modular: modular_mono,
            exhaust_raw_a: bank_a.exhaust,
            exhaust_raw_b: bank_b.exhaust,
            collector_a: self.bank_a_dsp.exhaust.collector_output(),
            collector_b: self.bank_b_dsp.exhaust.collector_output(),
            exhaust_a,
            exhaust_b,
            mono_engine,
            body: bank1 + bank2,
            intake,
            exhaust,
            rasp: rasp_mono,
            master,
        }
    }

    /// ABI-compatible stereo render. The procedural engine is dual-mono, so both
    /// returned samples are always bit-identical.
    pub fn render_stereo(&mut self) -> (f32, f32) {
        self.render_diagnostic_frame().master
    }

    /// Render one sample and packetize the combustion events produced by the
    /// same `CylState::step` call. This is the only C++ event ingress.
    pub fn render_stereo_with_events(
        &mut self,
        sink: &mut crate::dsp_contract::EventBuilder,
        sample_offset: u32,
    ) -> (f32, f32) {
        let master = self.render_diagnostic_frame().master;
        let bank_b_delay = self.reconstruct.offset_samples();
        for event in self.last_physical_events.iter().flatten() {
            sink.record_physical(
                sample_offset,
                event.cylinder,
                event.crank_phase_deg,
                event.pressure_peak,
                event.pressure_derivative,
                event.energy,
                event.cycle_variation,
                bank_b_delay,
            );
        }
        master
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

    fn captured_events(block_size: usize, total: usize) -> Vec<(u64, u8, u8, u32, u32, u32)> {
        let mut synth = HalfBlock::new(&config(), 44_100);
        synth.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let mut sink = crate::dsp_contract::EventBuilder::new(44_100.0, 0);
        let mut result = Vec::new();
        let mut base = 0usize;
        while base < total {
            let count = block_size.min(total - base);
            sink.begin_block(count as u32);
            for offset in 0..count {
                synth.render_stereo_with_events(&mut sink, offset as u32);
            }
            let events = sink.finish_block();
            for event in &events.events[..events.event_count as usize] {
                result.push((
                    base as u64 + event.sample_offset as u64,
                    event.bank,
                    event.cylinder,
                    event.pressure.to_bits(),
                    event.pressure_derivative.to_bits(),
                    event.cycle_variation.to_bits(),
                ));
            }
            base += count;
        }
        result
    }

    #[test]
    fn physical_v10_cycle_has_exactly_ten_events_and_both_banks() {
        // At 9000 RPM and 44.1 kHz one 720-degree cycle is exactly 588 samples.
        let events = captured_events(588, 588);
        assert_eq!(events.len(), 10);
        assert_eq!(events.iter().filter(|e| e.1 == 0).count(), 5);
        assert_eq!(events.iter().filter(|e| e.1 == 1).count(), 5);
        for cylinder in 0..5u8 {
            assert_eq!(events.iter().filter(|e| e.2 == cylinder).count(), 2);
        }
    }

    #[test]
    fn physical_event_transport_is_block_partition_invariant() {
        let reference = captured_events(2048, 2048);
        assert_eq!(reference, captured_events(1024, 2048));
        assert_eq!(reference, captured_events(512, 2048));
        assert_eq!(reference, captured_events(256, 2048));
        for pair in reference.windows(2) {
            assert!((pair[0].0, pair[0].1, pair[0].2) <= (pair[1].0, pair[1].1, pair[1].2));
        }
    }

    fn config() -> PowertrainConfig {
        PowertrainConfig::default()
    }

    #[test]
    fn no_events_produce_no_stochastic_combustion_noise() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(0.0, 1000.0, 15000.0, 0.0);
        let mut sum = 0.0f64;
        for _ in 0..4096 {
            let frame = block.render_diagnostic_frame();
            sum += (frame.full_block_raw as f64) * (frame.full_block_raw as f64);
        }
        assert!(
            sum.sqrt() < 1e-5,
            "free-running combustion noise RMS={}",
            sum.sqrt() / 4096.0f64.sqrt()
        );
    }

    /// The dual-mono contract must hold in every operating region, including
    /// while the limiter and traction control are actively gating.
    #[test]
    fn render_stereo_is_bit_identical_in_all_regions() {
        let regions = [
            (1000.0, 0.0, "idle"),
            (4500.0, 0.3, "low"),
            (9000.0, 0.8, "high"),
            (14900.0, 1.0, "redline"),
        ];
        for (rpm, throttle, name) in regions {
            let mut block = HalfBlock::new(&config(), 44100);
            block.set_limiter_enabled(true);
            block.set_tc_cut_ratio(0.5);
            block.update_controls(rpm, 1000.0, 15000.0, throttle);
            for index in 0..4096 {
                let (l, r) = block.render_stereo();
                assert_eq!(
                    l.to_bits(),
                    r.to_bits(),
                    "{name}: channels diverged at sample {index} ({l} vs {r})"
                );
                assert!(l.is_finite(), "{name}: non-finite sample");
            }
        }
    }

    #[test]
    fn render_stereo_is_bit_identical_during_rpm_sweep() {
        let mut block = HalfBlock::new(&config(), 44100);
        for step in 0..2205 {
            let rpm = 4500.0 + step as f64 * (15000.0 - 4500.0) / 2205.0;
            block.update_controls(rpm, 4500.0, 15000.0, 1.0);
            for _ in 0..10 {
                let (l, r) = block.render_stereo();
                assert_eq!(l.to_bits(), r.to_bits(), "sweep diverged at rpm {rpm}");
            }
        }
    }

    fn modular_test_config() -> ModularConfig {
        let mut modular = ModularConfig::default();
        modular.enabled = true;
        modular.gain = 0.5;
        modular.voices[0] = ModularVoiceConfig {
            enabled: true,
            waveform: Waveform::Saw,
            frequency_factor: 0.5,
            gain: 0.30,
            highpass_hz: 300.0,
            lowpass_hz: 3500.0,
        };
        modular.voices[1] = ModularVoiceConfig {
            enabled: true,
            waveform: Waveform::Square,
            frequency_factor: 1.25,
            gain: 0.12,
            highpass_hz: 1200.0,
            lowpass_hz: 5000.0,
        };
        modular.voices[2] = ModularVoiceConfig {
            enabled: true,
            waveform: Waveform::Noise,
            frequency_factor: 1.0,
            gain: 0.08,
            highpass_hz: 3500.0,
            lowpass_hz: 9000.0,
        };
        modular.voices[3] = ModularVoiceConfig {
            enabled: true,
            waveform: Waveform::Saw,
            frequency_factor: 7.0,
            gain: 0.035,
            highpass_hz: 7000.0,
            lowpass_hz: 11000.0,
        };
        modular
    }

    #[test]
    fn modular_disabled_is_exact_zero() {
        // Default profile: modular.enabled = false, the layer must be silence
        // even while the engine and the other voices are running.
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(12000.0, 1000.0, 15000.0, 0.85);
        let mut engine_ran = false;
        for _ in 0..4096 {
            let frame = block.render_diagnostic_frame();
            assert_eq!(frame.modular_a.to_bits(), 0.0f32.to_bits());
            assert_eq!(frame.modular_b.to_bits(), 0.0f32.to_bits());
            assert_eq!(frame.modular.to_bits(), 0.0f32.to_bits());
            engine_ran = engine_ran || frame.mono_engine != 0.0;
        }
        assert!(engine_ran, "engine body did not run during silence check");
    }

    #[test]
    fn modular_enabled_outputs_band_content_and_stays_finite() {
        let mut powertrain = config();
        powertrain.modular = modular_test_config();
        let mut block = HalfBlock::new(&powertrain, 44100);
        block.update_controls(12000.0, 1000.0, 15000.0, 0.85);
        let mut peak = 0.0f32;
        let mut nonzero = 0usize;
        for _ in 0..4096 {
            let frame = block.render_diagnostic_frame();
            assert!(frame.modular_a.is_finite() && frame.modular_b.is_finite());
            peak = peak.max(frame.modular.abs());
            if frame.modular != 0.0 {
                nonzero += 1;
            }
        }
        assert!(peak > 0.0, "modular layer produced no output");
        assert!(peak <= 1.2, "modular layer unbounded: {peak}");
        assert!(
            nonzero > 2048,
            "modular layer mostly silent: {nonzero}/4096"
        );
    }

    #[test]
    fn modular_bank_b_is_offset_from_bank_a() {
        let mut powertrain = config();
        powertrain.modular = modular_test_config();
        let mut block = HalfBlock::new(&powertrain, 44100);
        block.update_controls(12000.0, 1000.0, 15000.0, 0.85);
        let mut same = true;
        for _ in 0..4096 {
            let frame = block.render_diagnostic_frame();
            if frame.modular_a.to_bits() != frame.modular_b.to_bits() {
                same = false;
                break;
            }
        }
        assert!(
            !same,
            "modular banks are identical: bank B offset is not applied"
        );
    }

    #[test]
    fn mono_master_is_exactly_monophonic() {
        // A mono stage cannot create a difference: identical inputs yield
        // bit-identical outputs for any gain.
        let mut m = MonoMaster::new(44100);
        for cut in [1.0, 0.5, 0.0] {
            let (l, r) = m.process(0.37, cut);
            assert_eq!(l.to_bits(), r.to_bits(), "cut {cut} broke dual-mono");
        }
    }

    #[test]
    fn mono_master_removes_dc_and_stays_finite() {
        let mut m = MonoMaster::new(44100);
        let mut last = 0.0f32;
        for _ in 0..200_000 {
            last = m.process(1.0, 1.0).0;
        }
        assert!(last.abs() < 3e-4, "dc residue {last}");
        assert!(last.is_finite());
    }

    #[test]
    fn no_events_produce_no_rasp() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(0.0, 1000.0, 15000.0, 0.0);
        for _ in 0..4096 {
            let frame = block.render_diagnostic_frame();
            assert!(
                frame.rasp.abs() < 1.0e-6,
                "rasp must be silent without events"
            );
        }
    }

    /// Helper: render frames and collect a named scalar from each.
    fn collect(cfg: &PowertrainConfig, rpm: f64, throttle: f32, frames: usize) -> Vec<f32> {
        let mut block = HalfBlock::new(cfg, 44100);
        block.update_controls(rpm, 4500.0, 15000.0, throttle);
        for _ in 0..4410 {
            block.render_diagnostic_frame();
        }
        (0..frames)
            .map(|_| block.render_diagnostic_frame().mono_engine)
            .collect()
    }

    #[test]
    fn edge_disabled_reproduces_the_approved_body_alone() {
        // With edge off, the mix must be exactly the body: no other voice may
        // reach the output.
        let mut cfg = config();
        cfg.combustion_voices.edge_enabled = false;
        cfg.rasp.enabled = false;
        let mut block = HalfBlock::new(&cfg, 44100);
        block.update_controls(12000.0, 4500.0, 15000.0, 1.0);
        for _ in 0..8192 {
            let frame = block.render_diagnostic_frame();
            // The residual is still reported for inspection, but its
            // contribution to the mix is gated to zero.
            assert!(
                (frame.mono_engine - frame.combustion_body).abs() < 1e-6,
                "mono must equal the body alone: {} vs {}",
                frame.mono_engine,
                frame.combustion_body
            );
        }
    }

    #[test]
    fn body_plus_edge_reconstructs_pre_body_per_bank() {
        // The residual is the exact complementary high-pass: body + edge ==
        // pre_body, so summing the two voices cannot introduce inter-filter
        // phase error.
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(11000.0, 4500.0, 15000.0, 1.0);
        for index in 0..8192 {
            let frame = block.render_diagnostic_frame();
            for bank in ["a", "b"] {
                let (pre, body, edge) = if bank == "a" {
                    (frame.pre_body_a, frame.body_a, frame.combustion_edge_a)
                } else {
                    (frame.pre_body_b, frame.body_b, frame.combustion_edge_b)
                };
                assert!(
                    (pre - (body + edge)).abs() < 1e-5,
                    "bank {bank} reconstruction failed at {index}: {pre} vs {}",
                    body + edge
                );
            }
        }
    }

    #[test]
    fn edge_is_zero_for_stable_dc_after_the_transient() {
        // The residual of a low-pass cannot pass DC. Feeding a constant
        // excitation, `pre_body - body` must decay to zero once the filter
        // transient settles: the edge carries no DC of its own.
        let block = HalfBlock::new(&config(), 44100);
        let mut filter = block.clone_body_filter_a();
        let constant = 0.5f32;
        let mut worst = 0.0f32;
        for index in 0..200_000 {
            let body = filter.process(constant);
            // Only judge after the transient has died away.
            if index >= 100_000 {
                worst = worst.max((constant - body).abs());
            }
        }
        assert!(
            worst < 1e-3,
            "edge must vanish for stable dc: residual {worst}"
        );
    }

    #[test]
    fn disabled_voices_are_never_processed_or_mixed() {
        // intake/exhaust/rasp are off in this configuration: their stems must be
        // exact zeros and the mono mix must contain no trace of them.
        let mut cfg = config();
        cfg.rasp.enabled = false;
        let effective_edge_gain = if cfg.combustion_voices.edge_enabled {
            cfg.combustion_voices.edge_gain
        } else {
            0.0
        };
        let mut block = HalfBlock::new(&cfg, 44100);
        block.update_controls(13000.0, 4500.0, 15000.0, 1.0);
        for _ in 0..8192 {
            let frame = block.render_diagnostic_frame();
            assert_eq!(frame.intake, 0.0, "intake must be exact zero");
            assert_eq!(frame.exhaust, 0.0, "exhaust must be exact zero");
            assert_eq!(frame.rasp, 0.0, "rasp must be exact zero");
            // mono is exactly body + edge_gain * edge.
            let expected = frame.combustion_body + effective_edge_gain * frame.combustion_edge;
            assert!(
                (frame.mono_engine - expected).abs() < 1e-6,
                "mono {} must be body+edge {}",
                frame.mono_engine,
                expected
            );
        }
    }

    #[test]
    fn edge_adds_upper_band_energy_without_touching_the_body() {
        let mut off = config();
        off.combustion_voices.edge_enabled = false;
        let mut on = config();
        on.combustion_voices.edge_enabled = true;
        let body_only = rms(&collect(&off, 12000.0, 1.0, 22050));
        let with_edge = rms(&collect(&on, 12000.0, 1.0, 22050));
        assert!(
            with_edge > body_only,
            "edge must add energy: {body_only} -> {with_edge}"
        );
        // And it must not change the body itself.
        let mut a = HalfBlock::new(&off, 44100);
        let mut b = HalfBlock::new(&on, 44100);
        a.update_controls(12000.0, 4500.0, 15000.0, 1.0);
        b.update_controls(12000.0, 4500.0, 15000.0, 1.0);
        for _ in 0..8192 {
            let fa = a.render_diagnostic_frame();
            let fb = b.render_diagnostic_frame();
            assert_eq!(
                fa.combustion_body.to_bits(),
                fb.combustion_body.to_bits(),
                "enabling the edge must not alter the approved body"
            );
        }
    }

    /// Helper: RMS of the rendered rasp contribution for a control region.
    fn rasp_rms(cfg: &PowertrainConfig, rpm: f64, throttle: f32) -> f32 {
        let mut block = HalfBlock::new(cfg, 44100);
        block.update_controls(rpm, 4500.0, 15000.0, throttle);
        // Let the energy/load envelopes settle before measuring.
        for _ in 0..4410 {
            block.render_diagnostic_frame();
        }
        let mut sum = 0.0f64;
        let mut count = 0usize;
        for _ in 0..22050 {
            let value = block.render_diagnostic_frame().rasp as f64;
            sum += value * value;
            count += 1;
        }
        (sum / count as f64).sqrt() as f32
    }

    #[test]
    fn rasp_follows_profile_flag_in_combustion_only_mode() {
        // Rasp is allowed to reach the mix in combustion-only mode when the
        // profile enables it; intake/exhaust/air remain hard-disabled. The
        // layer must be exactly silent when the profile disables it.
        let mut cfg = config();
        cfg.rasp.enabled = true;
        let audible = rasp_rms(&cfg, 14000.0, 1.0);
        assert!(
            audible > 1e-4,
            "enabled rasp must be audible in combustion-only mode, got {audible}"
        );
        cfg.rasp.enabled = false;
        let silent = rasp_rms(&cfg, 14000.0, 1.0);
        assert_eq!(silent, 0.0, "disabled rasp must be exact zero");
    }

    #[test]
    fn rasp_disabled_produces_no_audible_processing() {
        let mut cfg = config();
        cfg.rasp.enabled = false;
        let mut block = HalfBlock::new(&cfg, 44100);
        block.update_controls(14000.0, 4500.0, 15000.0, 1.0);
        for _ in 0..8192 {
            let frame = block.render_diagnostic_frame();
            assert_eq!(
                frame.rasp, 0.0,
                "disabled rasp must contribute exactly zero"
            );
            assert!(frame.mono_engine.is_finite());
        }
        // The rest of the engine still renders when rasp is off.
        let energy = (0..4096)
            .map(|_| block.render_diagnostic_frame().mono_engine)
            .fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(energy > 1e-4, "engine body must remain with rasp disabled");
    }

    #[test]
    fn rasp_is_bypassed_at_far_and_virtual_lod() {
        let cfg = config();
        for level in [LodLevel::Far, LodLevel::Virtual] {
            let mut block = HalfBlock::new(&cfg, 44100);
            block.set_lod(level);
            block.update_controls(14000.0, 4500.0, 15000.0, 1.0);
            for _ in 0..4096 {
                assert_eq!(
                    block.render_diagnostic_frame().rasp,
                    0.0,
                    "{level:?} must not run the rasp path"
                );
            }
        }
    }

    #[test]
    fn edge_survives_near_and_mid_lod() {
        // Unlike rasp, `combustion_edge` is a declared voice: it must remain
        // present wherever the engine is audible (only Far/Virtual may drop it).
        let cfg = config();
        for level in [LodLevel::Near, LodLevel::Mid] {
            let mut block = HalfBlock::new(&cfg, 44100);
            block.set_lod(level);
            block.update_controls(14000.0, 4500.0, 15000.0, 1.0);
            let peak = (0..8192)
                .map(|_| block.render_diagnostic_frame().combustion_edge)
                .fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.0, "{level:?} must keep the edge voice");
        }
    }

    #[test]
    fn body_filters_are_identical_even_with_timbre_diff_set() {
        // The lateral cutoff multiplier is gone, so `timbre_diff` can no longer
        // make bank B brighter than bank A. Both bodies must be acoustically
        // identical even when the field is non-zero; bank B's only difference is
        // the 72-degree mechanical offset applied before the mono sum.
        let mut cfg = config();
        cfg.half_block.timbre_diff = 0.6;
        let mut block = HalfBlock::new(&cfg, 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let mut a = block.clone_body_filter_a();
        let mut b = block.clone_body_filter_b();
        for index in 0..4096 {
            let x = (index as f32 * 0.017).sin() * 0.3;
            assert_eq!(
                a.process(x).to_bits(),
                b.process(x).to_bits(),
                "body filters must be identical at sample {index}"
            );
        }
    }

    #[test]
    fn second_bank_keeps_seventy_two_degree_offset_before_mono_sum() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..512 {
            block.render_diagnostic_frame();
        }
        let increment = (9000.0 / 120.0 * 720.0) / 44100.0;
        let expected = 72.0f32 / increment as f32;
        assert!(
            (block.reconstruct_offset_samples() - expected).abs() < 1e-3,
            "offset {} vs expected {expected}",
            block.reconstruct_offset_samples()
        );
    }

    #[test]
    fn angular_event_width_scales_inversely_with_rpm() {
        let event = AngularEventEnvelope {
            open_offset_deg: 0.0,
            attack_deg: 20.0,
            hold_deg: 20.0,
            decay_deg: 180.0,
        };
        let mut counts = Vec::new();
        for rpm in [4500.0, 9000.0] {
            let step = rpm / 120.0 * 720.0 / 44100.0;
            let mut age = 0.0;
            let mut n = 0;
            while age < (event.attack_deg + event.hold_deg + event.decay_deg) as f64 {
                age += step;
                n += 1;
            }
            counts.push(n);
        }
        assert!((counts[0] as f32 / counts[1] as f32 - 2.0).abs() < 0.05);
    }

    #[test]
    fn event_branches_are_independent() {
        let mut a = config();
        let mut b = a.clone();
        b.exhaust_open_offset_deg = 240.0;
        let mut x = HalfBlock::new(&a, 44100);
        let mut y = HalfBlock::new(&b, 44100);
        x.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        y.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..2000 {
            let fx = x.render_diagnostic_frame();
            let fy = y.render_diagnostic_frame();
            assert!((fx.body - fy.body).abs() < 1e-5);
            assert!((fx.intake - fy.intake).abs() < 1e-5);
        }
        a.intake_event_width_deg = 20.0;
        b = config();
        let mut p = HalfBlock::new(&a, 44100);
        let mut q = HalfBlock::new(&b, 44100);
        p.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        q.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..2000 {
            let fp = p.render_diagnostic_frame();
            let fq = q.render_diagnostic_frame();
            assert!((fp.body - fq.body).abs() < 1e-5);
            assert!((fp.exhaust - fq.exhaust).abs() < 1e-5);
        }
        // Combustion shaping is independent of the exhaust event timing.
        let mut combustion_fast = config();
        combustion_fast.combustion_attack_deg = 4.0;
        combustion_fast.combustion_decay_deg = 40.0;
        let mut combustion_slow = config();
        combustion_slow.combustion_attack_deg = 40.0;
        combustion_slow.combustion_decay_deg = 220.0;
        let mut fast = HalfBlock::new(&combustion_fast, 44100);
        let mut slow = HalfBlock::new(&combustion_slow, 44100);
        fast.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        slow.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let mut fast_start = None;
        let mut slow_start = None;
        for i in 0..3000 {
            if fast.render_diagnostic_frame().exhaust_raw_a.abs() > 1e-6 && fast_start.is_none() {
                fast_start = Some(i);
            }
            if slow.render_diagnostic_frame().exhaust_raw_a.abs() > 1e-6 && slow_start.is_none() {
                slow_start = Some(i);
            }
            if fast_start.is_some() && slow_start.is_some() {
                break;
            }
        }
        assert_eq!(
            fast_start, slow_start,
            "combustion pulse changed exhaust timing"
        );

        // Exhaust envelope shaping must not feed back into body or intake.
        let mut exhaust_short = config();
        exhaust_short.exhaust_blowdown_attack_deg = 4.0;
        exhaust_short.exhaust_blowdown_decay_deg = 40.0;
        let mut exhaust_long = config();
        exhaust_long.exhaust_blowdown_attack_deg = 40.0;
        exhaust_long.exhaust_blowdown_decay_deg = 220.0;
        let mut es = HalfBlock::new(&exhaust_short, 44100);
        let mut el = HalfBlock::new(&exhaust_long, 44100);
        es.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        el.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..2000 {
            let fs = es.render_diagnostic_frame();
            let fl = el.render_diagnostic_frame();
            assert!((fs.body_a - fl.body_a).abs() < 1e-5);
            assert!((fs.intake_a - fl.intake_a).abs() < 1e-5);
        }
        let mut late = config();
        late.exhaust_open_offset_deg = 720.0;
        let mut probe = HalfBlock::new(&late, 44100);
        probe.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let mut checked = false;
        for _ in 0..300 {
            let f = probe.render_diagnostic_frame();
            if f.body_a.abs() > 1e-5 {
                checked = true;
                assert!(
                    f.exhaust_raw_a.abs() < 1e-6,
                    "pre-offset exhaust={} body={}",
                    f.exhaust_raw_a,
                    f.body_a
                );
                break;
            }
        }
        assert!(checked);
    }

    #[test]
    fn blowdown_is_zero_before_open_offset() {
        let event = AngularEventEnvelope {
            open_offset_deg: 120.0,
            attack_deg: 10.0,
            hold_deg: 0.0,
            decay_deg: 100.0,
        };
        assert_eq!(event.value(119.99), 0.0);
        assert_eq!(event.value(120.0), 0.0);
        assert!(event.value(125.0) > 0.0);
    }

    #[test]
    fn rpm_change_keeps_frames_finite_and_continuous() {
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(4500.0, 1000.0, 15000.0, 1.0);
        for _ in 0..1000 {
            let f = block.render_diagnostic_frame();
            assert!(f.master.0.is_finite() && f.master.1.is_finite());
        }
        let before = block.render_diagnostic_frame().master;
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let after = block.render_diagnostic_frame().master;
        assert!((after.0 - before.0).abs() < 1.0 && (after.1 - before.1).abs() < 1.0);
    }

    #[test]
    fn full_block_has_four_orders_without_noise() {
        let mut block = HalfBlock::new(&config(), 44100);
        let rpm = 9000.0;
        block.update_controls(rpm, 1000.0, 15000.0, 1.0);
        let n = 44100;
        let mut samples = Vec::with_capacity(n);
        let mut bank_a = Vec::with_capacity(n);
        for _ in 0..n {
            let f = block.render_diagnostic_frame();
            samples.push(f.full_block_raw);
            bank_a.push(f.bank_a_raw);
        }
        let mut energies = Vec::new();
        let mut bank_energies = Vec::new();
        let orders: [f64; 16] = [
            0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 7.5, 10.0, 12.5, 15.0, 20.0, 25.0,
        ];
        for order in orders {
            let f = rpm / 60.0 * order;
            let mut re = 0.0;
            let mut im = 0.0;
            for (i, sample) in samples.iter().enumerate() {
                let phase = std::f64::consts::TAU * f * i as f64 / 44100.0;
                let w = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
                re += *sample as f64 * w * phase.cos();
                im -= *sample as f64 * w * phase.sin();
            }
            energies.push(re * re + im * im);
            let mut re = 0.0;
            let mut im = 0.0;
            for (i, sample) in bank_a.iter().enumerate() {
                let phase = std::f64::consts::TAU * f * i as f64 / 44100.0;
                let w = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
                re += *sample as f64 * w * phase.cos();
                im -= *sample as f64 * w * phase.sin();
            }
            bank_energies.push(re * re + im * im);
        }
        let max = energies.iter().copied().fold(0.0, f64::max);
        let significant = energies
            .iter()
            .filter(|energy| **energy >= max * 10f64.powf(-2.4))
            .count();
        assert!(
            significant >= 4,
            "significant orders={significant}, energies={energies:?}"
        );
        assert!(
            bank_energies[4] > bank_energies[9],
            "bank A negative control: {bank_energies:?}"
        );
        assert!(
            energies[9] >= energies[4] * 10f64.powf(0.3),
            "full block E5/E2.5 < +3 dB: {energies:?}"
        );
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
        let cfg = config();
        let mut block = HalfBlock::new(&cfg, 44100);
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
        let released = block.energy();
        assert!(
            released < loaded * 0.5,
            "throttle cut not releasing: {released}"
        );
        assert!(
            (released - cfg.idle_combustion_gain).abs() < 0.05,
            "closed throttle must settle on the audible idle base: {released}"
        );
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
    fn render_stereo_channels_are_dual_mono_by_default() {
        // Replaces the old decorrelation test: the procedural engine is now
        // intentionally monophonic, so the two channels are the same sample.
        let mut block = HalfBlock::new(&config(), 44100);
        block.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        let mut left = Vec::with_capacity(8192);
        let mut right = Vec::with_capacity(8192);
        for _ in 0..8192 {
            let (l, r) = block.render_stereo();
            left.push(l);
            right.push(r);
        }
        for (index, (l, r)) in left.iter().zip(right.iter()).enumerate() {
            assert_eq!(
                l.to_bits(),
                r.to_bits(),
                "channels must be dual-mono (sample {index}: {l} vs {r})"
            );
        }
        let corr = pearson(&left[4096..], &right[4096..]);
        assert!(corr > 0.999, "dual-mono must be fully correlated: {corr}");
    }

    #[test]
    fn steady_rpm_outputs_are_dc_guarded() {
        for rpm in [4500.0, 9000.0, 15000.0] {
            let mut block = HalfBlock::new(&config(), 44100);
            block.update_controls(rpm, 1000.0, 15000.0, 1.0);
            let mut left = Vec::with_capacity(44100);
            let mut right = Vec::with_capacity(44100);
            for _ in 0..44100 {
                let (l, r) = block.render_stereo();
                left.push(l);
                right.push(r);
            }
            for channel in [&left, &right] {
                let mean = channel.iter().sum::<f32>() / channel.len() as f32;
                let rms = rms(channel);
                assert!(
                    mean.abs() / rms.max(1e-9) <= 0.05,
                    "rpm={rpm} dc ratio={}",
                    mean.abs() / rms.max(1e-9)
                );
            }
        }
    }

    #[test]
    fn diagnostic_frame_uses_one_state_advance_and_has_finite_headroom() {
        let mut a = HalfBlock::new(&config(), 44100);
        let mut b = HalfBlock::new(&config(), 44100);
        a.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        b.update_controls(9000.0, 1000.0, 15000.0, 1.0);
        for _ in 0..1024 {
            let master_a = a.render_stereo();
            let frame_b = b.render_diagnostic_frame();
            assert_eq!(master_a, frame_b.master);
            assert!(frame_b.master.0.is_finite() && frame_b.master.1.is_finite());
            assert!(frame_b.master.0.abs() <= 1.0 && frame_b.master.1.abs() <= 1.0);
        }
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
