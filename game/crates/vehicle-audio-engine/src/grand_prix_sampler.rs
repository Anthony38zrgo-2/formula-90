use std::collections::VecDeque;

use crate::dsp::biquad::Biquad;
use crate::grand_prix_sample_bank::{
    engine_loop_presence_weight, GrandPrixDecodedLoop, GrandPrixDecodedVariant, GrandPrixEventRole,
    GrandPrixSampleBank, GrandPrixSelection, GrandPrixTransition, GrandPrixTriggerEvent,
    GRAND_PRIX_MAXIMUM_LOOP_COUNT,
};

pub const GRAND_PRIX_MAX_LOOPS: usize = GRAND_PRIX_MAXIMUM_LOOP_COUNT;
pub const GRAND_PRIX_SHIFT_VOICE_COUNT: usize = 2;
pub const GRAND_PRIX_BACKFIRE_VOICE_INDEX: usize = 2;
pub const GRAND_PRIX_LIMITER_VOICE_INDEX: usize = 3;
pub const GRAND_PRIX_VOICE_COUNT: usize = 4;
pub const GRAND_PRIX_EVENT_QUEUE_CAPACITY: usize = 64;
pub const GRAND_PRIX_EVENT_FADE_FRAMES: usize = 96;
pub const GRAND_PRIX_MINIMUM_AUDIBLE_REVOLUTIONS_PER_MINUTE: f64 = 1_200.0;
pub const GRAND_PRIX_PITCH_SMOOTHING_SECONDS: f64 = 0.015;
pub const GRAND_PRIX_LOAD_SMOOTHING_SECONDS: f64 = 0.035;
pub const GRAND_PRIX_GATE_ATTACK_SECONDS: f64 = 0.010;
pub const GRAND_PRIX_GATE_RELEASE_SECONDS: f64 = 0.180;
pub const GRAND_PRIX_SHIFT_ENGINE_DUCK_ATTACK_SECONDS: f64 = 0.006;
pub const GRAND_PRIX_SHIFT_ENGINE_DUCK_RELEASE_SECONDS: f64 = 0.045;
pub const GRAND_PRIX_UPSHIFT_ENGINE_GAIN: f32 = 0.18;
pub const GRAND_PRIX_DOWNSHIFT_ENGINE_GAIN: f32 = 0.42;
pub const GRAND_PRIX_MAXIMUM_INGEST_GAP_SECONDS: f64 = 0.5;
pub const GRAND_PRIX_COAST_GAIN_DEFAULT: f32 = 0.55;
const GRAND_PRIX_LIMITER_CUT_WINDOW_CAPACITY: usize = 256;
const GRAND_PRIX_LIMITER_CUT_TRANSITION_SECONDS: f64 = 0.00025;
const GRAND_PRIX_LIMITER_LOOP_CROSSFADE_SECONDS: f64 = 0.002;
const GRAND_PRIX_LIMITER_HIGH_PASS_CUTOFF_HERTZ: f32 = 600.0;
const UPSHIFT_ACOUSTIC_RECOVERY_DURATION_SECONDS: f64 = 0.350;
const UPSHIFT_ACOUSTIC_RECOVERY_DECAY_SECONDS: f64 = 0.150;
const UPSHIFT_ACOUSTIC_RECOVERY_MAXIMUM_PITCH_DEPTH: f64 = 0.035;
const GRAND_PRIX_GEARBOX_WHINE_TOOTH_CONTACT_DUTY_CYCLE: f32 = 0.125;
const GRAND_PRIX_GEARBOX_WHINE_GEAR_CASING_RESONANCE_GAINS: [f32; 8] =
    [0.15, 0.35, 0.82, 1.0, 0.78, 0.50, 0.27, 0.12];
const GRAND_PRIX_GEARBOX_WHINE_FINAL_CASING_RESONANCE_GAINS: [f32; 6] =
    [0.20, 0.55, 1.0, 0.80, 0.45, 0.20];
const GRAND_PRIX_GEARBOX_WHINE_NOISE_GEAR_HARMONIC_ORDER: f32 = 3.0;
const GRAND_PRIX_GEARBOX_WHINE_NOISE_FINAL_HARMONIC_ORDER: f32 = 2.0;
const GRAND_PRIX_GEARBOX_WHINE_ATTACK_SECONDS: f64 = 0.025;
const GRAND_PRIX_GEARBOX_WHINE_RELEASE_SECONDS: f64 = 0.300;

const SHIFT_PHASE_UPSHIFT_CUT: i32 = 1;
const SHIFT_PHASE_DOWNSHIFT_CUT: i32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum GrandPrixSamplerError {
    #[error("unsupported sample rate {0}")]
    UnsupportedSampleRate(u32),
    #[error("bank coverage [{minimum}, {maximum}] does not include required range")]
    UnsupportedCoverage { minimum: f64, maximum: f64 },
    #[error("bank error: {0}")]
    Bank(#[from] crate::grand_prix_sample_bank::GrandPrixBankError),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GrandPrixTelemetry {
    pub rpm: f64,
    pub throttle: f32,
    pub normalized_transmitted_load: f32,
    pub transmitted_torque_sign: i32,
    pub gear: i32,
    pub shift_phase: i32,
    pub rev_limiter_active: bool,
    pub dt_seconds: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrandPrixEventKind {
    Upshift,
    Downshift,
    LiftBackfire,
    Limiter,
}

impl GrandPrixEventKind {
    fn index(self) -> usize {
        match self {
            Self::Upshift => 0,
            Self::Downshift => 1,
            Self::LiftBackfire => 2,
            Self::Limiter => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GrandPrixDiagnostics {
    pub rendered_revolutions_per_minute: f32,
    pub upshift_acoustic_pitch_modulation: f32,
    pub upshift_acoustic_amplitude_modulation: f32,
    pub gear_mesh_frequency_hertz: f32,
    pub final_mesh_frequency_hertz: f32,
    pub transmitted_load_gain: f32,
    pub engine_gain: f32,
    pub load_gain: f32,
    pub gate: f32,
    pub active_zone_count: u8,
    pub zone_weights: [f32; GRAND_PRIX_MAX_LOOPS],
    pub zone_rates: [f32; GRAND_PRIX_MAX_LOOPS],
    pub active_voice_count: u8,
    pub queued_events: u32,
    pub accepted_events: u64,
    pub accepted_upshift_events: u64,
    pub accepted_downshift_events: u64,
    pub accepted_backfire_events: u64,
    pub accepted_limiter_events: u64,
    pub suppressed_events: u64,
    pub late_events: u64,
    pub overflowed_events: u64,
    pub clamped_rate_samples: u64,
    pub stale_input_holds: u64,
    pub output_peak: f32,
    pub limiter_voice_active: bool,
    pub backfire_voice_active: bool,
}

#[derive(Clone, Copy, Debug)]
struct ScheduledEvent {
    kind: GrandPrixEventKind,
    frame: u64,
    event_index: usize,
    gain: f32,
    ratio: f64,
}

#[derive(Clone, Copy, Debug)]
struct LimiterCutWindow {
    start_frame: u64,
    end_frame_exclusive: u64,
}

#[derive(Clone, Copy, Debug)]
struct UpshiftAcousticRecovery {
    start_frame: u64,
    pitch_depth: f64,
    amplitude_depth: f64,
    frequency_hertz: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct EventVoice {
    active: bool,
    event_index: usize,
    cursor: f64,
    ratio: f64,
    gain: f32,
    fade_frames: usize,
    fade_position: usize,
    fade_release: bool,
}

impl EventVoice {
    fn start(&mut self, event: &ScheduledEvent, fade_frames: usize) {
        self.active = true;
        self.event_index = event.event_index;
        self.cursor = 0.0;
        self.ratio = event.ratio;
        self.gain = event.gain;
        self.fade_frames = fade_frames;
        self.fade_position = 0;
        self.fade_release = false;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GrandPrixSampleOutput {
    pub engine: f32,
    pub gearbox: f32,
    pub gearbox_whine: f32,
    pub backfire: f32,
    pub limiter: f32,
}

impl GrandPrixSampleOutput {
    pub fn sum(self) -> f32 {
        self.engine + self.gearbox + self.gearbox_whine + self.backfire + self.limiter
    }
}

pub struct GrandPrixSampler {
    bank: GrandPrixSampleBank,
    output_shaft_speed_hertz: Option<f64>,
    physical_transmitted_torque_sign: Option<i32>,
    target_gear: Option<i32>,
    target_transmitted_load: f32,
    transmitted_torque_sign: i32,
    upshift_acoustic_recoveries: VecDeque<UpshiftAcousticRecovery>,
    upshift_departure: Option<(i32, f64, f32)>,
    upshift_departure_expires_frame: u64,
    output_sample_rate: u32,
    loop_cursors: Vec<f64>,
    zone_rates: Vec<f64>,
    zone_anti_alias: Vec<Biquad>,
    zone_filter_cutoffs: Vec<f32>,
    coast_loop_cursors: Vec<f64>,
    coast_zone_anti_alias: Vec<Biquad>,
    coast_zone_filter_cutoffs: Vec<f32>,
    gearbox_whine_gear_phase: f64,
    gearbox_whine_final_phase: f64,
    gearbox_whine_gear_pulse_harmonic_gains: Vec<f32>,
    gearbox_whine_final_pulse_harmonic_gains: Vec<f32>,
    gearbox_whine_gear_mesh_frequency_hertz: f32,
    gearbox_whine_final_mesh_frequency_hertz: f32,
    gearbox_whine_envelope: f32,
    gearbox_whine_gain: f32,
    gearbox_whine_tone_gain: f32,
    gearbox_whine_noise_gain: f32,
    gearbox_whine_gear_ratios: Vec<f32>,
    gearbox_whine_final_drive: f32,
    gearbox_whine_reverse_ratio: f32,
    gearbox_whine_gear_teeth: f32,
    gearbox_whine_final_teeth: f32,
    gearbox_noise_state: u64,
    gearbox_noise_high_pass: Biquad,
    gearbox_noise_low_pass: Biquad,
    gearbox_noise_low_cutoff_hertz: f32,
    gearbox_noise_high_cutoff_hertz: f32,
    current_gear: i32,
    gearbox_whine_gear_level: f32,
    gearbox_whine_attack_alpha: f32,
    gearbox_whine_release_alpha: f32,
    group_indices: [Option<usize>; 4],
    event_voices: Vec<EventVoice>,
    event_queue: VecDeque<ScheduledEvent>,
    limiter_cut_variant: Option<GrandPrixDecodedVariant>,
    limiter_cut_windows: VecDeque<LimiterCutWindow>,
    limiter_cut_envelope: f32,
    limiter_cut_cursor: f64,
    limiter_high_pass: Biquad,
    target_rpm: f64,
    smoothed_rpm: f64,
    target_throttle: f32,
    smoothed_throttle: f32,
    gate_target: f32,
    gate: f32,
    rpm_alpha: f64,
    load_alpha: f32,
    gate_attack_alpha: f32,
    gate_release_alpha: f32,
    shift_engine_gain_target: f32,
    shift_engine_gain: f32,
    shift_engine_duck_attack_alpha: f32,
    shift_engine_duck_release_alpha: f32,
    engine_gain: f32,
    gearbox_gain: f32,
    backfire_gain: f32,
    limiter_gain: f32,
    coast_gain: f32,
    render_frame: u64,
    ingest_frame_cursor: u64,
    ingest_frame_remainder: f64,
    retrigger_until_frames: [u64; 4],
    lift_edge_cooldown_frames: u64,
    limiter_edge_cooldown_frames: u64,
    rng_state: u64,
    cycle_position_by_kind: [usize; 4],
    received_telemetry: bool,
    last_throttle: f32,
    last_shift_phase: i32,
    last_rev_limiter_active: bool,
    diagnostics: GrandPrixDiagnostics,
}

impl GrandPrixSampler {
    pub fn new(
        bank: GrandPrixSampleBank,
        output_sample_rate: u32,
        required_minimum_revolutions_per_minute: Option<f64>,
        required_maximum_revolutions_per_minute: Option<f64>,
    ) -> Result<Self, GrandPrixSamplerError> {
        if output_sample_rate == 0 {
            return Err(GrandPrixSamplerError::UnsupportedSampleRate(
                output_sample_rate,
            ));
        }
        if let (Some(required_minimum), Some(required_maximum)) = (
            required_minimum_revolutions_per_minute,
            required_maximum_revolutions_per_minute,
        ) {
            if bank.coverage_minimum_revolutions_per_minute > required_minimum + 1e-6
                || bank.coverage_maximum_revolutions_per_minute < required_maximum - 1e-6
            {
                return Err(GrandPrixSamplerError::UnsupportedCoverage {
                    minimum: bank.coverage_minimum_revolutions_per_minute,
                    maximum: bank.coverage_maximum_revolutions_per_minute,
                });
            }
        }
        let loop_count = bank.loops.len();
        let coast_loop_count = bank.coast_loops.len();
        let mut group_indices: [Option<usize>; 4] = [None; 4];
        for (index, group) in bank.groups.iter().enumerate() {
            match group.role {
                GrandPrixEventRole::GearboxUpshift => group_indices[0] = Some(index),
                GrandPrixEventRole::GearboxDownshift => group_indices[1] = Some(index),
                GrandPrixEventRole::LiftBackfire => group_indices[2] = Some(index),
                GrandPrixEventRole::LimiterEvent => group_indices[3] = Some(index),
            }
        }
        let sample_rate = output_sample_rate as f64;
        let limiter_cut_variant = group_indices[3].and_then(|group_index| {
            let group = &bank.groups[group_index];
            (group.trigger_event == GrandPrixTriggerEvent::LimiterCutWindow)
                .then(|| group.variants[0])
        });
        let event_voices = (0..GRAND_PRIX_VOICE_COUNT)
            .map(|_| EventVoice::default())
            .collect();
        let rng_state = bank.event_selection_seed.max(1);
        Ok(Self {
            bank,
            output_shaft_speed_hertz: None,
            physical_transmitted_torque_sign: None,
            target_gear: None,
            target_transmitted_load: 0.0,
            transmitted_torque_sign: 0,
            upshift_acoustic_recoveries: VecDeque::with_capacity(GRAND_PRIX_EVENT_QUEUE_CAPACITY),
            upshift_departure: None,
            upshift_departure_expires_frame: 0,
            output_sample_rate,
            loop_cursors: vec![0.0; loop_count],
            zone_rates: vec![1.0; loop_count],
            zone_anti_alias: (0..loop_count)
                .map(|_| Biquad::lowpass(output_sample_rate as f32, 19_000.0))
                .collect(),
            zone_filter_cutoffs: vec![19_000.0; loop_count],
            coast_loop_cursors: vec![0.0; coast_loop_count],
            coast_zone_anti_alias: (0..coast_loop_count)
                .map(|_| Biquad::lowpass(output_sample_rate as f32, 19_000.0))
                .collect(),
            coast_zone_filter_cutoffs: vec![19_000.0; coast_loop_count],
            gearbox_whine_gear_phase: 0.0,
            gearbox_whine_final_phase: 0.0,
            gearbox_whine_gear_pulse_harmonic_gains: tooth_pulse_harmonic_gains(
                GRAND_PRIX_GEARBOX_WHINE_TOOTH_CONTACT_DUTY_CYCLE,
                &GRAND_PRIX_GEARBOX_WHINE_GEAR_CASING_RESONANCE_GAINS,
            ),
            gearbox_whine_final_pulse_harmonic_gains: tooth_pulse_harmonic_gains(
                GRAND_PRIX_GEARBOX_WHINE_TOOTH_CONTACT_DUTY_CYCLE,
                &GRAND_PRIX_GEARBOX_WHINE_FINAL_CASING_RESONANCE_GAINS,
            ),
            gearbox_whine_gear_mesh_frequency_hertz: 0.0,
            gearbox_whine_final_mesh_frequency_hertz: 0.0,
            gearbox_whine_envelope: 0.0,
            gearbox_whine_gain: 0.0,
            gearbox_whine_tone_gain: 0.12,
            gearbox_whine_noise_gain: 0.025,
            gearbox_whine_gear_ratios: Vec::new(),
            gearbox_whine_final_drive: 1.0,
            gearbox_whine_reverse_ratio: 3.0,
            gearbox_whine_gear_teeth: 20.0,
            gearbox_whine_final_teeth: 40.0,
            gearbox_noise_state: (rng_state ^ 0x9E37_79B9_7F4A_7C15).max(1),
            gearbox_noise_high_pass: Biquad::highpass(output_sample_rate as f32, 1.0),
            gearbox_noise_low_pass: Biquad::lowpass(output_sample_rate as f32, 2.0),
            gearbox_noise_low_cutoff_hertz: 1.0,
            gearbox_noise_high_cutoff_hertz: 2.0,
            current_gear: 0,
            gearbox_whine_gear_level: 0.10,
            gearbox_whine_attack_alpha: (1.0
                - (-1.0 / (sample_rate * GRAND_PRIX_GEARBOX_WHINE_ATTACK_SECONDS)).exp())
                as f32,
            gearbox_whine_release_alpha: (1.0
                - (-1.0 / (sample_rate * GRAND_PRIX_GEARBOX_WHINE_RELEASE_SECONDS)).exp())
                as f32,
            group_indices,
            event_voices,
            event_queue: VecDeque::with_capacity(GRAND_PRIX_EVENT_QUEUE_CAPACITY),
            limiter_cut_variant,
            limiter_cut_windows: VecDeque::with_capacity(GRAND_PRIX_LIMITER_CUT_WINDOW_CAPACITY),
            limiter_cut_envelope: 0.0,
            limiter_cut_cursor: 0.0,
            limiter_high_pass: Biquad::highpass(
                output_sample_rate as f32,
                GRAND_PRIX_LIMITER_HIGH_PASS_CUTOFF_HERTZ,
            ),
            target_rpm: 0.0,
            smoothed_rpm: 0.0,
            target_throttle: 0.0,
            smoothed_throttle: 0.0,
            gate_target: 0.0,
            gate: 0.0,
            rpm_alpha: 1.0 - (-1.0 / (sample_rate * GRAND_PRIX_PITCH_SMOOTHING_SECONDS)).exp(),
            load_alpha: (1.0 - (-1.0 / (sample_rate * GRAND_PRIX_LOAD_SMOOTHING_SECONDS)).exp())
                as f32,
            gate_attack_alpha: (1.0 - (-1.0 / (sample_rate * GRAND_PRIX_GATE_ATTACK_SECONDS)).exp())
                as f32,
            gate_release_alpha: (1.0
                - (-1.0 / (sample_rate * GRAND_PRIX_GATE_RELEASE_SECONDS)).exp())
                as f32,
            shift_engine_gain_target: 1.0,
            shift_engine_gain: 1.0,
            shift_engine_duck_attack_alpha: (1.0
                - (-1.0 / (sample_rate * GRAND_PRIX_SHIFT_ENGINE_DUCK_ATTACK_SECONDS)).exp())
                as f32,
            shift_engine_duck_release_alpha: (1.0
                - (-1.0 / (sample_rate * GRAND_PRIX_SHIFT_ENGINE_DUCK_RELEASE_SECONDS)).exp())
                as f32,
            engine_gain: 1.0,
            gearbox_gain: 1.0,
            backfire_gain: 1.0,
            limiter_gain: 1.0,
            coast_gain: GRAND_PRIX_COAST_GAIN_DEFAULT,
            render_frame: 0,
            ingest_frame_cursor: 0,
            ingest_frame_remainder: 0.0,
            retrigger_until_frames: [0; 4],
            lift_edge_cooldown_frames: 0,
            limiter_edge_cooldown_frames: 0,
            rng_state,
            cycle_position_by_kind: [0; 4],
            received_telemetry: false,
            last_throttle: 0.0,
            last_shift_phase: 0,
            last_rev_limiter_active: false,
            diagnostics: GrandPrixDiagnostics::default(),
        })
    }

    pub fn set_output_shaft_speed_hertz(&mut self, speed_hertz: Option<f64>) {
        self.output_shaft_speed_hertz = speed_hertz.filter(|speed| speed.is_finite()).map(f64::abs);
    }

    pub fn set_transmitted_torque_sign(&mut self, torque_sign: Option<i32>) {
        self.physical_transmitted_torque_sign = torque_sign.map(i32::signum);
    }

    pub fn set_target_gear(&mut self, target_gear: Option<i32>) {
        self.target_gear = target_gear;
    }

    pub fn pending_event_delay_frames(&self) -> u64 {
        self.ingest_frame_cursor.saturating_sub(self.render_frame)
    }

    pub fn bank(&self) -> &GrandPrixSampleBank {
        &self.bank
    }

    pub fn bank_id(&self) -> &str {
        &self.bank.bank_id
    }

    pub fn bank_sha256(&self) -> &str {
        &self.bank.bank_sha256
    }

    pub fn loop_ids(&self) -> Vec<String> {
        self.bank
            .loops
            .iter()
            .map(|loop_asset| loop_asset.id.clone())
            .collect()
    }

    pub fn set_group_gains(&mut self, engine: f32, gearbox: f32, backfire: f32, limiter: f32) {
        self.engine_gain = engine.clamp(0.0, 4.0);
        self.gearbox_gain = gearbox.clamp(0.0, 4.0);
        self.backfire_gain = backfire.clamp(0.0, 4.0);
        self.limiter_gain = limiter.clamp(0.0, 4.0);
    }

    pub fn set_coast_gain(&mut self, coast_gain: f32) {
        self.coast_gain = coast_gain.clamp(0.0, 1.0);
    }

    pub fn set_gearbox_whine_gain(&mut self, gain: f32) {
        self.gearbox_whine_gain = gain.clamp(0.0, 4.0);
    }

    pub fn set_gearbox_whine_component_gains(&mut self, tone_gain: f32, noise_gain: f32) {
        self.gearbox_whine_tone_gain = tone_gain.clamp(0.0, 1.0);
        self.gearbox_whine_noise_gain = noise_gain.clamp(0.0, 1.0);
    }

    pub fn set_gearbox_whine_transmission(
        &mut self,
        gear_ratios: &[f32],
        final_drive: f32,
        reverse_ratio: f32,
        gear_teeth: f32,
        final_teeth: f32,
    ) {
        self.gearbox_whine_gear_ratios = gear_ratios
            .iter()
            .copied()
            .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
            .collect();
        if final_drive.is_finite() && final_drive > 0.0 {
            self.gearbox_whine_final_drive = final_drive;
        }
        if reverse_ratio.is_finite() && reverse_ratio > 0.0 {
            self.gearbox_whine_reverse_ratio = reverse_ratio;
        }
        if gear_teeth.is_finite() && gear_teeth > 0.0 {
            self.gearbox_whine_gear_teeth = gear_teeth;
        }
        if final_teeth.is_finite() && final_teeth > 0.0 {
            self.gearbox_whine_final_teeth = final_teeth;
        }
    }

    fn gearbox_mesh_frequencies(&self, revolutions_per_minute: f64, gear: i32) -> (f32, f32) {
        let gear_ratio = if gear > 0 {
            self.gearbox_whine_gear_ratios
                .get(gear as usize - 1)
                .copied()
                .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
                .or_else(|| self.gearbox_whine_gear_ratios.first().copied())
                .unwrap_or(1.0)
        } else if gear == -1 {
            self.gearbox_whine_reverse_ratio
        } else {
            self.gearbox_whine_gear_ratios
                .first()
                .copied()
                .unwrap_or(1.0)
        };
        let output_shaft_hertz = self
            .output_shaft_speed_hertz
            .map(|speed| speed as f32)
            .unwrap_or((revolutions_per_minute.max(0.0) as f32 / 60.0) / gear_ratio);
        let gear_mesh_hertz = output_shaft_hertz * self.gearbox_whine_gear_teeth;
        let final_mesh_hertz =
            output_shaft_hertz / self.gearbox_whine_final_drive * self.gearbox_whine_final_teeth;
        (gear_mesh_hertz, final_mesh_hertz)
    }

    fn update_gearbox_noise_filters(
        &mut self,
        gear_mesh_frequency_hertz: f32,
        final_mesh_frequency_hertz: f32,
    ) {
        let sample_rate = self.output_sample_rate as f32;
        let maximum_cutoff_hertz = sample_rate * 0.48;
        let noise_center_hertz = (gear_mesh_frequency_hertz
            * GRAND_PRIX_GEARBOX_WHINE_NOISE_GEAR_HARMONIC_ORDER)
            .max(final_mesh_frequency_hertz * GRAND_PRIX_GEARBOX_WHINE_NOISE_FINAL_HARMONIC_ORDER)
            .min(maximum_cutoff_hertz / 1.38);
        let low_cutoff_hertz = (noise_center_hertz * 0.72).clamp(1.0, maximum_cutoff_hertz * 0.65);
        let high_cutoff_hertz =
            (noise_center_hertz * 1.38).clamp(low_cutoff_hertz + 1.0, maximum_cutoff_hertz);
        if (low_cutoff_hertz - self.gearbox_noise_low_cutoff_hertz).abs() >= 20.0 {
            self.gearbox_noise_high_pass
                .update_highpass(sample_rate, low_cutoff_hertz);
            self.gearbox_noise_low_cutoff_hertz = low_cutoff_hertz;
        }
        if (high_cutoff_hertz - self.gearbox_noise_high_cutoff_hertz).abs() >= 20.0 {
            self.gearbox_noise_low_pass
                .update_lowpass(sample_rate, high_cutoff_hertz);
            self.gearbox_noise_high_cutoff_hertz = high_cutoff_hertz;
        }
    }

    pub fn diagnostics(&self) -> GrandPrixDiagnostics {
        self.diagnostics
    }

    pub fn reset(&mut self) {
        self.output_shaft_speed_hertz = None;
        self.physical_transmitted_torque_sign = None;
        self.target_gear = None;
        self.target_transmitted_load = 0.0;
        self.transmitted_torque_sign = 0;
        self.upshift_acoustic_recoveries.clear();
        self.upshift_departure = None;
        self.upshift_departure_expires_frame = 0;
        for cursor in &mut self.loop_cursors {
            *cursor = 0.0;
        }
        for cursor in &mut self.coast_loop_cursors {
            *cursor = 0.0;
        }
        self.gearbox_whine_gear_phase = 0.0;
        self.gearbox_whine_final_phase = 0.0;
        self.gearbox_whine_gear_mesh_frequency_hertz = 0.0;
        self.gearbox_whine_final_mesh_frequency_hertz = 0.0;
        self.gearbox_whine_envelope = 0.0;
        self.gearbox_noise_state = (self.bank.event_selection_seed ^ 0x9E37_79B9_7F4A_7C15).max(1);
        self.gearbox_noise_high_pass = Biquad::highpass(self.output_sample_rate as f32, 1.0);
        self.gearbox_noise_low_pass = Biquad::lowpass(self.output_sample_rate as f32, 2.0);
        self.gearbox_noise_low_cutoff_hertz = 1.0;
        self.gearbox_noise_high_cutoff_hertz = 2.0;
        self.current_gear = 0;
        self.gearbox_whine_gear_level = 0.10;
        for filter in self
            .zone_anti_alias
            .iter_mut()
            .chain(self.coast_zone_anti_alias.iter_mut())
        {
            *filter = Biquad::lowpass(self.output_sample_rate as f32, 19_000.0);
        }
        self.zone_filter_cutoffs.fill(19_000.0);
        self.coast_zone_filter_cutoffs.fill(19_000.0);
        for voice in &mut self.event_voices {
            *voice = EventVoice::default();
        }
        self.event_queue.clear();
        self.target_rpm = 0.0;
        self.smoothed_rpm = 0.0;
        self.target_throttle = 0.0;
        self.smoothed_throttle = 0.0;
        self.gate_target = 0.0;
        self.gate = 0.0;
        self.shift_engine_gain_target = 1.0;
        self.shift_engine_gain = 1.0;
        self.render_frame = 0;
        self.ingest_frame_cursor = 0;
        self.ingest_frame_remainder = 0.0;
        self.retrigger_until_frames = [0; 4];
        self.lift_edge_cooldown_frames = 0;
        self.limiter_edge_cooldown_frames = 0;
        self.cycle_position_by_kind = [0; 4];
        self.received_telemetry = false;
        self.last_throttle = 0.0;
        self.last_shift_phase = 0;
        self.last_rev_limiter_active = false;
        self.limiter_cut_windows.clear();
        self.limiter_cut_envelope = 0.0;
        self.limiter_cut_cursor = 0.0;
        self.limiter_high_pass.reset();
        self.diagnostics = GrandPrixDiagnostics::default();
    }

    pub fn ingest(&mut self, telemetry: &GrandPrixTelemetry) {
        if !telemetry.rpm.is_finite()
            || !telemetry.dt_seconds.is_finite()
            || telemetry.dt_seconds < 0.0
        {
            self.diagnostics.stale_input_holds =
                self.diagnostics.stale_input_holds.saturating_add(1);
            return;
        }
        let bounded_gap = telemetry
            .dt_seconds
            .min(GRAND_PRIX_MAXIMUM_INGEST_GAP_SECONDS);
        if telemetry.dt_seconds > GRAND_PRIX_MAXIMUM_INGEST_GAP_SECONDS {
            self.diagnostics.stale_input_holds =
                self.diagnostics.stale_input_holds.saturating_add(1);
            self.upshift_acoustic_recoveries.clear();
            self.upshift_departure = None;
            self.received_telemetry = false;
            self.ingest_frame_cursor = self.render_frame;
            self.ingest_frame_remainder = 0.0;
            self.limiter_cut_windows.clear();
            self.limiter_cut_envelope = 0.0;
            self.limiter_high_pass.reset();
        }
        let duration_frames =
            bounded_gap * self.output_sample_rate as f64 + self.ingest_frame_remainder;
        let dt_frames = duration_frames.round().max(0.0) as u64;
        self.ingest_frame_remainder = duration_frames - dt_frames as f64;
        let scheduled_frame = self.render_frame.max(self.ingest_frame_cursor);
        if self.limiter_cut_variant.is_some() && telemetry.rev_limiter_active && dt_frames > 0 {
            self.schedule_limiter_cut_window(scheduled_frame, dt_frames);
        }
        self.lift_edge_cooldown_frames = self.lift_edge_cooldown_frames.saturating_sub(dt_frames);
        self.limiter_edge_cooldown_frames =
            self.limiter_edge_cooldown_frames.saturating_sub(dt_frames);

        if self.received_telemetry {
            if scheduled_frame > self.upshift_departure_expires_frame {
                self.upshift_departure = None;
            }
            if telemetry.shift_phase == SHIFT_PHASE_UPSHIFT_CUT
                && self.last_shift_phase != SHIFT_PHASE_UPSHIFT_CUT
            {
                self.upshift_departure = Some((
                    self.current_gear,
                    self.target_rpm,
                    self.target_transmitted_load,
                ));
                self.upshift_departure_expires_frame = scheduled_frame
                    + (0.5 * self.output_sample_rate as f64) as u64;
            } else if telemetry.shift_phase != SHIFT_PHASE_UPSHIFT_CUT
                && telemetry.normalized_transmitted_load > 0.0
                && self
                    .upshift_departure
                    .is_some_and(|(previous_gear, previous_speed, _)| {
                        telemetry.gear > previous_gear && previous_speed > telemetry.rpm
                    })
            {
                if let Some((previous_gear, previous_speed, previous_load)) =
                    self.upshift_departure.take()
                {
                    let recovery_load = previous_load.max(telemetry.normalized_transmitted_load.clamp(0.0, 1.0));
                    if previous_gear > 0
                        && telemetry.gear > previous_gear
                        && previous_speed > telemetry.rpm
                    {
                        let speed_drop = ((previous_speed - telemetry.rpm)
                            / previous_speed.max(1.0))
                        .clamp(0.0, 0.5);
                        if self.upshift_acoustic_recoveries.len() < GRAND_PRIX_EVENT_QUEUE_CAPACITY
                        {
                            self.upshift_acoustic_recoveries
                                .push_back(UpshiftAcousticRecovery {
                                    start_frame: scheduled_frame,
                                    pitch_depth: (speed_drop * 1.5 * recovery_load as f64)
                                        .min(UPSHIFT_ACOUSTIC_RECOVERY_MAXIMUM_PITCH_DEPTH),
                                    amplitude_depth: 0.20 * recovery_load as f64,
                                    frequency_hertz: 22.0,
                                });
                        } else {
                            self.diagnostics.overflowed_events =
                                self.diagnostics.overflowed_events.saturating_add(1);
                        }
                    }
                }
            } else if telemetry.shift_phase == SHIFT_PHASE_DOWNSHIFT_CUT {
                self.upshift_departure = None;
            }
        }
        self.target_transmitted_load = if telemetry.normalized_transmitted_load.is_finite() {
            telemetry.normalized_transmitted_load.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.transmitted_torque_sign = self
            .physical_transmitted_torque_sign
            .unwrap_or(telemetry.transmitted_torque_sign.signum());
        self.target_rpm = telemetry.rpm.clamp(0.0, 30_000.0);
        self.current_gear = telemetry.gear;
        self.target_throttle = telemetry.throttle.clamp(0.0, 1.0);
        let (gear_mesh_frequency_hertz, final_mesh_frequency_hertz) =
            self.gearbox_mesh_frequencies(self.target_rpm, self.current_gear);
        self.update_gearbox_noise_filters(gear_mesh_frequency_hertz, final_mesh_frequency_hertz);
        self.gate_target = if telemetry.rpm >= GRAND_PRIX_MINIMUM_AUDIBLE_REVOLUTIONS_PER_MINUTE {
            1.0
        } else {
            0.0
        };
        self.shift_engine_gain_target = match telemetry.shift_phase {
            SHIFT_PHASE_UPSHIFT_CUT => GRAND_PRIX_UPSHIFT_ENGINE_GAIN,
            SHIFT_PHASE_DOWNSHIFT_CUT => GRAND_PRIX_DOWNSHIFT_ENGINE_GAIN,
            _ => 1.0,
        };

        if !self.received_telemetry {
            self.received_telemetry = true;
            self.smoothed_rpm = self.target_rpm;
            self.last_throttle = self.target_throttle;
            self.last_shift_phase = telemetry.shift_phase;
            self.last_rev_limiter_active = telemetry.rev_limiter_active;
            self.ingest_frame_cursor = scheduled_frame + dt_frames;
            return;
        }

        if telemetry.shift_phase == SHIFT_PHASE_UPSHIFT_CUT
            && self.last_shift_phase != SHIFT_PHASE_UPSHIFT_CUT
            && self.current_gear > 0
            && self.target_gear != Some(0)
        {
            self.schedule_event(
                GrandPrixEventKind::Upshift,
                scheduled_frame,
                self.target_rpm,
            );
        } else if telemetry.shift_phase == SHIFT_PHASE_DOWNSHIFT_CUT
            && self.last_shift_phase != SHIFT_PHASE_DOWNSHIFT_CUT
            && self.target_gear != Some(0)
        {
            self.schedule_event(
                GrandPrixEventKind::Downshift,
                scheduled_frame,
                self.target_rpm,
            );
        }

        let previous_throttle_min = self.bank.lift_edge_policy.previous_throttle_min as f32;
        let throttle_max = self.bank.lift_edge_policy.throttle_max as f32;
        let lift_rpm_minimum = self.bank.lift_edge_policy.revolutions_per_minute_min;
        let lift_cooldown_seconds = self.bank.lift_edge_policy.cooldown_seconds;
        if self.last_throttle >= previous_throttle_min
            && self.target_throttle <= throttle_max
            && self.target_rpm >= lift_rpm_minimum
            && self.lift_edge_cooldown_frames == 0
        {
            self.schedule_event(
                GrandPrixEventKind::LiftBackfire,
                scheduled_frame,
                self.target_rpm,
            );
            self.lift_edge_cooldown_frames =
                (lift_cooldown_seconds * self.output_sample_rate as f64).round() as u64;
        }

        let limiter_cooldown_seconds = self.bank.limiter_edge_policy.cooldown_seconds;
        if self.limiter_cut_variant.is_none()
            && telemetry.rev_limiter_active
            && !self.last_rev_limiter_active
            && self.limiter_edge_cooldown_frames == 0
        {
            self.schedule_event(
                GrandPrixEventKind::Limiter,
                scheduled_frame,
                self.target_rpm,
            );
            self.limiter_edge_cooldown_frames =
                (limiter_cooldown_seconds * self.output_sample_rate as f64).round() as u64;
        }

        self.last_throttle = self.target_throttle;
        self.last_shift_phase = telemetry.shift_phase;
        self.last_rev_limiter_active = telemetry.rev_limiter_active;
        self.ingest_frame_cursor = scheduled_frame + dt_frames;
    }

    pub fn ingest_direct(&mut self, kind: GrandPrixEventKind, rpm: f64) {
        let scheduled_frame = self.render_frame.max(self.ingest_frame_cursor);
        self.target_rpm = rpm.clamp(0.0, 30_000.0);
        self.schedule_event(kind, scheduled_frame, self.target_rpm);
    }

    fn schedule_event(&mut self, kind: GrandPrixEventKind, frame: u64, rpm: f64) {
        if kind == GrandPrixEventKind::Limiter && self.limiter_cut_variant.is_some() {
            return;
        }
        let index = kind.index();
        if frame < self.retrigger_until_frames[index] {
            self.diagnostics.suppressed_events =
                self.diagnostics.suppressed_events.saturating_add(1);
            return;
        }
        let Some(group_index) = self.group_indices[index] else {
            self.diagnostics.suppressed_events =
                self.diagnostics.suppressed_events.saturating_add(1);
            return;
        };
        let Some(variant) = self.select_variant(group_index, index) else {
            self.diagnostics.suppressed_events =
                self.diagnostics.suppressed_events.saturating_add(1);
            return;
        };
        if self.event_queue.len() >= GRAND_PRIX_EVENT_QUEUE_CAPACITY {
            self.diagnostics.overflowed_events =
                self.diagnostics.overflowed_events.saturating_add(1);
            return;
        }
        let ratio = variant
            .pitch_reference_revolutions_per_minute
            .map_or(1.0, |reference| (rpm / reference).clamp(0.5, 2.0));
        let group = &self.bank.groups[group_index];
        let event = ScheduledEvent {
            kind,
            frame: frame + group.timing_offset_frames as u64,
            event_index: variant.event_index,
            gain: variant.gain,
            ratio,
        };
        self.retrigger_until_frames[index] = event.frame + group.minimum_retrigger_frames as u64;
        self.event_queue.push_back(event);
    }

    fn select_variant(
        &mut self,
        group_index: usize,
        kind_index: usize,
    ) -> Option<GrandPrixDecodedVariant> {
        let variant_count = self.bank.groups[group_index].variants.len();
        if variant_count == 0 {
            return None;
        }
        let selection = self.bank.groups[group_index].selection;
        let selected = match selection {
            GrandPrixSelection::SingleVariant => 0,
            GrandPrixSelection::SeededCycle => {
                self.rng_state = next_rng_state(self.rng_state);
                (self.rng_state >> 33) as usize % variant_count
            }
            GrandPrixSelection::SeededNoImmediateRepeat => {
                self.rng_state = next_rng_state(self.rng_state);
                if variant_count == 1 {
                    0
                } else {
                    let step = 1 + (self.rng_state >> 33) as usize % (variant_count - 1);
                    let previous = self.cycle_position_by_kind[kind_index] % variant_count;
                    (previous + step) % variant_count
                }
            }
        };
        self.cycle_position_by_kind[kind_index] = selected;
        Some(self.bank.groups[group_index].variants[selected])
    }

    pub fn render_sample_components(&mut self) -> GrandPrixSampleOutput {
        self.smoothed_rpm += (self.target_rpm - self.smoothed_rpm) * self.rpm_alpha;
        self.smoothed_throttle += (self.target_throttle - self.smoothed_throttle) * self.load_alpha;
        let gate_alpha = if self.gate_target > self.gate {
            self.gate_attack_alpha
        } else {
            self.gate_release_alpha
        };
        self.gate += (self.gate_target - self.gate) * gate_alpha;
        let shift_engine_gain_alpha = if self.shift_engine_gain_target < self.shift_engine_gain {
            self.shift_engine_duck_attack_alpha
        } else {
            self.shift_engine_duck_release_alpha
        };
        self.shift_engine_gain +=
            (self.shift_engine_gain_target - self.shift_engine_gain) * shift_engine_gain_alpha;

        self.drain_due_events();

        let (recovery_modulation, recovery_amplitude_modulation) =
            self.render_upshift_acoustic_recovery();
        let rendered_rpm = self.smoothed_rpm * (1.0 + recovery_modulation);
        let playback_revolutions_per_minute = rendered_rpm.clamp(
            self.bank.coverage_minimum_revolutions_per_minute,
            self.bank.coverage_maximum_revolutions_per_minute,
        );
        let powered_loop_weights =
            powered_engine_loop_weights(&self.bank, playback_revolutions_per_minute);
        let mut loop_mix = 0.0f32;
        let mut active_zones = 0u8;
        let loop_count = self.bank.loops.len();
        for index in 0..loop_count {
            let reference = self.bank.loops[index].reference_revolutions_per_minute;
            let minimum = self.bank.loops[index].valid_playback_rate_min;
            let maximum = self.bank.loops[index].valid_playback_rate_max;
            let weight = powered_loop_weights[index];
            let rate = playback_revolutions_per_minute / reference;
            if (rate < minimum || rate > maximum) && weight > 0.0 {
                let within_rounding =
                    rate >= minimum * (1.0 - 1e-9) && rate <= maximum * (1.0 + 1e-9);
                if !within_rounding {
                    self.diagnostics.clamped_rate_samples =
                        self.diagnostics.clamped_rate_samples.saturating_add(1);
                }
            }
            self.zone_rates[index] = rate;
            self.diagnostics.zone_rates[index] = rate as f32;
            self.diagnostics.zone_weights[index] = weight;
            let sample = read_loop_sample(
                &self.bank.loops[index],
                &mut self.loop_cursors[index],
                &mut self.zone_anti_alias[index],
                &mut self.zone_filter_cutoffs[index],
                self.output_sample_rate,
                rate,
            );
            loop_mix += sample * weight * self.bank.loops[index].calibrated_gain;
            if weight > 0.0 {
                active_zones = active_zones.saturating_add(1);
            }
        }

        let (engine_mix, load_gain) = if self.bank.coast_loops.is_empty() {
            let load_gain = self.coast_gain + (1.0 - self.coast_gain) * self.smoothed_throttle;
            (loop_mix * load_gain, load_gain)
        } else {
            let (coast_from, coast_to, coast_transition_index) =
                zone_blend(&self.bank.coast_transitions, rendered_rpm);
            let mut coast_mix = 0.0f32;
            for index in 0..self.bank.coast_loops.len() {
                let coast_loop = &self.bank.coast_loops[index];
                let weight = if index == coast_transition_index {
                    coast_from
                } else if index == coast_transition_index + 1 {
                    coast_to
                } else {
                    0.0
                };
                let rate =
                    playback_revolutions_per_minute / coast_loop.reference_revolutions_per_minute;
                let sample = read_loop_sample(
                    coast_loop,
                    &mut self.coast_loop_cursors[index],
                    &mut self.coast_zone_anti_alias[index],
                    &mut self.coast_zone_filter_cutoffs[index],
                    self.output_sample_rate,
                    rate,
                );
                coast_mix += sample * weight * coast_loop.calibrated_gain;
            }
            let throttle = self.smoothed_throttle.clamp(0.0, 1.0);
            let smoothed_load = throttle * throttle * (3.0 - 2.0 * throttle);
            let load_angle = smoothed_load * std::f32::consts::FRAC_PI_2;
            let idle_transition = ((rendered_rpm - 4_500.0) / 1_500.0).clamp(0.0, 1.0) as f32;
            let idle_transition = idle_transition * idle_transition * (3.0 - 2.0 * idle_transition);
            let engine_weight = 1.0 - idle_transition + idle_transition * load_angle.sin();
            let coast_weight = idle_transition * load_angle.cos() * self.coast_gain;
            (
                loop_mix * engine_weight + coast_mix * coast_weight,
                engine_weight + coast_weight,
            )
        };
        let engine_sample = engine_mix
            * self.gate
            * self.shift_engine_gain
            * self.engine_gain
            * (1.0 + recovery_amplitude_modulation as f32);

        let mut output = GrandPrixSampleOutput {
            engine: engine_sample,
            ..GrandPrixSampleOutput::default()
        };
        let (target_gear_mesh_frequency_hertz, target_final_mesh_frequency_hertz) =
            self.gearbox_mesh_frequencies(self.target_rpm, self.current_gear);
        self.gearbox_whine_gear_mesh_frequency_hertz += (target_gear_mesh_frequency_hertz
            - self.gearbox_whine_gear_mesh_frequency_hertz)
            * self.rpm_alpha as f32;
        self.gearbox_whine_final_mesh_frequency_hertz += (target_final_mesh_frequency_hertz
            - self.gearbox_whine_final_mesh_frequency_hertz)
            * self.rpm_alpha as f32;
        self.update_gearbox_noise_filters(
            self.gearbox_whine_gear_mesh_frequency_hertz,
            self.gearbox_whine_final_mesh_frequency_hertz,
        );

        let target_whine_envelope = smoothstep(100.0, 900.0, self.smoothed_rpm) as f32;
        let whine_envelope_alpha = if target_whine_envelope > self.gearbox_whine_envelope {
            self.gearbox_whine_attack_alpha
        } else {
            self.gearbox_whine_release_alpha
        };
        self.gearbox_whine_envelope +=
            (target_whine_envelope - self.gearbox_whine_envelope) * whine_envelope_alpha;

        let sample_rate = self.output_sample_rate as f32;
        let gear_mesh_tone = render_band_limited_tooth_pulse(
            self.gearbox_whine_gear_phase,
            self.gearbox_whine_gear_mesh_frequency_hertz,
            &self.gearbox_whine_gear_pulse_harmonic_gains,
            sample_rate,
        );
        let final_mesh_tone = render_band_limited_tooth_pulse(
            self.gearbox_whine_final_phase,
            self.gearbox_whine_final_mesh_frequency_hertz,
            &self.gearbox_whine_final_pulse_harmonic_gains,
            sample_rate,
        );
        let tone_sample = gear_mesh_tone + final_mesh_tone;
        self.gearbox_whine_gear_phase = advance_phase(
            self.gearbox_whine_gear_phase,
            self.gearbox_whine_gear_mesh_frequency_hertz,
            sample_rate,
        );
        self.gearbox_whine_final_phase = advance_phase(
            self.gearbox_whine_final_phase,
            self.gearbox_whine_final_mesh_frequency_hertz,
            sample_rate,
        );

        self.gearbox_noise_state = next_rng_state(self.gearbox_noise_state.max(1));
        let white_noise = (self.gearbox_noise_state >> 40) as f32 / 8_388_607.5 - 1.0;
        let high_frequency_noise = self
            .gearbox_noise_low_pass
            .process(self.gearbox_noise_high_pass.process(white_noise));
        let transmission_side_gain = if self.transmitted_torque_sign < 0 {
            0.78
        } else {
            1.0
        };
        let target_gear_level = if self.current_gear == 0 {
            0.10
        } else {
            (0.10 + 0.90 * self.target_transmitted_load.sqrt()) * transmission_side_gain
        };
        self.gearbox_whine_gear_level +=
            (target_gear_level - self.gearbox_whine_gear_level) * self.load_alpha;
        output.gearbox_whine = (tone_sample * self.gearbox_whine_tone_gain
            + high_frequency_noise * self.gearbox_whine_noise_gain)
            * self.gearbox_whine_gain
            * self.gearbox_whine_envelope
            * self.gearbox_whine_gear_level
            * (1.0 + recovery_modulation as f32 * 4.0);
        let mut active_voices = 0u8;
        for index in 0..self.event_voices.len() {
            if self.event_voices[index].active {
                active_voices = active_voices.saturating_add(1);
                let voice_sample = self.render_event_voice(index);
                if index < GRAND_PRIX_SHIFT_VOICE_COUNT {
                    output.gearbox += voice_sample * self.gearbox_gain;
                } else if index == GRAND_PRIX_BACKFIRE_VOICE_INDEX {
                    output.backfire += voice_sample * self.backfire_gain;
                } else {
                    output.limiter += voice_sample * self.limiter_gain;
                }
            }
        }

        if self.limiter_cut_variant.is_some() {
            output.limiter = self.render_limiter_cut_sample();
            if self.limiter_cut_envelope > 0.0 {
                active_voices = active_voices.saturating_add(1);
            }
        }

        self.diagnostics.rendered_revolutions_per_minute = rendered_rpm as f32;
        self.diagnostics.upshift_acoustic_pitch_modulation = recovery_modulation as f32;
        self.diagnostics.upshift_acoustic_amplitude_modulation = recovery_amplitude_modulation as f32;
        self.diagnostics.gear_mesh_frequency_hertz = self.gearbox_whine_gear_mesh_frequency_hertz;
        self.diagnostics.final_mesh_frequency_hertz = self.gearbox_whine_final_mesh_frequency_hertz;
        self.diagnostics.transmitted_load_gain = self.gearbox_whine_gear_level;
        self.diagnostics.engine_gain = self.engine_gain;
        self.diagnostics.load_gain = load_gain;
        self.diagnostics.gate = self.gate;
        self.diagnostics.active_zone_count = active_zones;
        self.diagnostics.active_voice_count = active_voices;
        self.diagnostics.queued_events = self.event_queue.len() as u32;
        self.diagnostics.output_peak = self.diagnostics.output_peak.max(output.sum().abs());
        self.diagnostics.limiter_voice_active = self.limiter_cut_envelope > 0.0
            || self.event_voices[GRAND_PRIX_LIMITER_VOICE_INDEX].active;
        self.diagnostics.backfire_voice_active =
            self.event_voices[GRAND_PRIX_BACKFIRE_VOICE_INDEX].active;
        self.render_frame = self.render_frame.saturating_add(1);
        output
    }

    pub fn render_sample(&mut self) -> f32 {
        self.render_sample_components().sum()
    }

    fn schedule_limiter_cut_window(&mut self, start_frame: u64, duration_frames: u64) {
        let end_frame_exclusive = start_frame.saturating_add(duration_frames);
        if let Some(previous_window) = self.limiter_cut_windows.back_mut() {
            if previous_window.end_frame_exclusive == start_frame {
                previous_window.end_frame_exclusive = end_frame_exclusive;
                return;
            }
        }
        if self.limiter_cut_windows.len() == GRAND_PRIX_LIMITER_CUT_WINDOW_CAPACITY {
            self.diagnostics.overflowed_events =
                self.diagnostics.overflowed_events.saturating_add(1);
            self.limiter_cut_windows.clear();
            self.limiter_cut_envelope = 0.0;
            return;
        }
        self.limiter_cut_windows.push_back(LimiterCutWindow {
            start_frame,
            end_frame_exclusive,
        });
    }

    fn render_limiter_cut_sample(&mut self) -> f32 {
        while self
            .limiter_cut_windows
            .front()
            .is_some_and(|window| window.end_frame_exclusive <= self.render_frame)
        {
            self.limiter_cut_windows.pop_front();
        }
        let cut_active = self
            .limiter_cut_windows
            .front()
            .is_some_and(|window| window.start_frame <= self.render_frame);
        let transition_frames = (GRAND_PRIX_LIMITER_CUT_TRANSITION_SECONDS
            * self.output_sample_rate as f64)
            .round()
            .max(1.0) as f32;
        let envelope_step = 1.0 / transition_frames;
        self.limiter_cut_envelope = if cut_active {
            (self.limiter_cut_envelope + envelope_step).min(1.0)
        } else {
            (self.limiter_cut_envelope - envelope_step).max(0.0)
        };
        if self.limiter_cut_envelope == 0.0 {
            self.limiter_high_pass.reset();
            return 0.0;
        }
        let Some(variant) = self.limiter_cut_variant else {
            return 0.0;
        };
        let recording = &self.bank.events[variant.event_index].pcm;
        if recording.is_empty() {
            return 0.0;
        }
        let crossfade_frames = ((GRAND_PRIX_LIMITER_LOOP_CROSSFADE_SECONDS * 44_100.0).round()
            as usize)
            .min(recording.len() / 4);
        let recording_sample_at = |position: f64| {
            let first_frame = position.floor() as usize;
            let second_frame = (first_frame + 1).min(recording.len() - 1);
            let fraction = (position - first_frame as f64) as f32;
            let first_sample = recording[first_frame] as f32 / 32768.0;
            let second_sample = recording[second_frame] as f32 / 32768.0;
            first_sample + (second_sample - first_sample) * fraction
        };
        let recording_frame = self.limiter_cut_cursor as usize;
        let mut sample = recording_sample_at(self.limiter_cut_cursor);
        let crossfade_start = recording.len() - crossfade_frames;
        if crossfade_frames > 0 && recording_frame >= crossfade_start {
            let crossfade_position = self.limiter_cut_cursor - crossfade_start as f64;
            let blend = crossfade_position as f32 / crossfade_frames as f32;
            let beginning = recording_sample_at(crossfade_position);
            sample = sample * (1.0 - blend) + beginning * blend;
        }
        self.limiter_cut_cursor += 44_100.0 / self.output_sample_rate as f64;
        if self.limiter_cut_cursor >= recording.len() as f64 {
            self.limiter_cut_cursor = crossfade_frames as f64
                + (self.limiter_cut_cursor - recording.len() as f64)
                    % (recording.len() - crossfade_frames) as f64;
        }
        self.limiter_high_pass.process(sample)
            * variant.gain * self.limiter_gain * self.limiter_cut_envelope
    }

    fn drain_due_events(&mut self) {
        while let Some(event) = self.event_queue.front().copied() {
            if event.frame > self.render_frame {
                break;
            }
            self.event_queue.pop_front();
            if event.frame < self.render_frame {
                self.diagnostics.late_events = self.diagnostics.late_events.saturating_add(1);
            }
            self.start_voice(event);
            self.diagnostics.accepted_events = self.diagnostics.accepted_events.saturating_add(1);
            match event.kind {
                GrandPrixEventKind::Upshift => {
                    self.diagnostics.accepted_upshift_events =
                        self.diagnostics.accepted_upshift_events.saturating_add(1);
                }
                GrandPrixEventKind::Downshift => {
                    self.diagnostics.accepted_downshift_events =
                        self.diagnostics.accepted_downshift_events.saturating_add(1);
                }
                GrandPrixEventKind::LiftBackfire => {
                    self.diagnostics.accepted_backfire_events =
                        self.diagnostics.accepted_backfire_events.saturating_add(1);
                }
                GrandPrixEventKind::Limiter => {
                    self.diagnostics.accepted_limiter_events =
                        self.diagnostics.accepted_limiter_events.saturating_add(1);
                }
            }
        }
    }

    fn start_voice(&mut self, event: ScheduledEvent) {
        match event.kind {
            GrandPrixEventKind::Upshift | GrandPrixEventKind::Downshift => {
                let mut target: Option<usize> = None;
                for index in 0..GRAND_PRIX_SHIFT_VOICE_COUNT {
                    if !self.event_voices[index].active {
                        target = Some(index);
                        break;
                    }
                }
                if target.is_none() {
                    let mut oldest_remaining = usize::MAX;
                    for index in 0..GRAND_PRIX_SHIFT_VOICE_COUNT {
                        let event_length = self.bank.events[self.event_voices[index].event_index]
                            .pcm
                            .len();
                        let remaining = (event_length as f64 - self.event_voices[index].cursor)
                            .max(0.0) as usize;
                        if remaining < oldest_remaining {
                            oldest_remaining = remaining;
                            target = Some(index);
                        }
                    }
                }
                if let Some(index) = target {
                    self.event_voices[index].start(&event, GRAND_PRIX_EVENT_FADE_FRAMES);
                }
            }
            GrandPrixEventKind::LiftBackfire => {
                self.event_voices[GRAND_PRIX_BACKFIRE_VOICE_INDEX]
                    .start(&event, GRAND_PRIX_EVENT_FADE_FRAMES);
            }
            GrandPrixEventKind::Limiter => {
                self.event_voices[GRAND_PRIX_LIMITER_VOICE_INDEX]
                    .start(&event, GRAND_PRIX_EVENT_FADE_FRAMES);
            }
        }
    }

    fn render_upshift_acoustic_recovery(&mut self) -> (f64, f64) {
        let duration_frames =
            (UPSHIFT_ACOUSTIC_RECOVERY_DURATION_SECONDS * self.output_sample_rate as f64) as u64;
        while self
            .upshift_acoustic_recoveries
            .front()
            .is_some_and(|recovery| {
                self.render_frame >= recovery.start_frame.saturating_add(duration_frames)
            })
        {
            self.upshift_acoustic_recoveries.pop_front();
        }
        let mut pitch_modulation = 0.0;
        let mut amplitude_modulation = 0.0;
        for recovery in &self.upshift_acoustic_recoveries {
            if recovery.start_frame > self.render_frame {
                continue;
            }
            let elapsed_seconds = (self.render_frame - recovery.start_frame) as f64
                / self.output_sample_rate as f64;
            let oscillation = (-elapsed_seconds / UPSHIFT_ACOUSTIC_RECOVERY_DECAY_SECONDS).exp()
                * (std::f64::consts::TAU * recovery.frequency_hertz * elapsed_seconds).sin()
                * (1.0
                    - smoothstep(
                        0.300,
                        UPSHIFT_ACOUSTIC_RECOVERY_DURATION_SECONDS,
                        elapsed_seconds,
                    ));
            pitch_modulation += recovery.pitch_depth * oscillation;
            amplitude_modulation += recovery.amplitude_depth * oscillation;
        }
        (
            pitch_modulation.clamp(
                -UPSHIFT_ACOUSTIC_RECOVERY_MAXIMUM_PITCH_DEPTH,
                UPSHIFT_ACOUSTIC_RECOVERY_MAXIMUM_PITCH_DEPTH,
            ),
            amplitude_modulation.clamp(-0.20, 0.20),
        )
    }

    fn render_event_voice(&mut self, voice_index: usize) -> f32 {
        let event_index = self.event_voices[voice_index].event_index;
        let pcm = &self.bank.events[event_index].pcm;
        let length = pcm.len();
        let voice = &mut self.event_voices[voice_index];
        if length == 0 {
            voice.active = false;
            return 0.0;
        }
        let position = voice.cursor;
        let index = position.floor() as usize;
        if index >= length {
            voice.active = false;
            return 0.0;
        }
        let next = if index + 1 < length { index + 1 } else { index };
        let fraction = (position - index as f64) as f32;
        let first = pcm[index] as f32 / 32768.0;
        let second = pcm[next] as f32 / 32768.0;
        let mut sample = first + (second - first) * fraction;

        if voice.fade_frames > 0 {
            let attack = (voice.fade_position as f32 / voice.fade_frames as f32).min(1.0);
            let remaining = (length as f64 - position).max(0.0);
            if remaining <= voice.fade_frames as f64 {
                voice.fade_release = true;
            }
            let release = if voice.fade_release {
                (remaining as f32 / voice.fade_frames as f32).min(1.0)
            } else {
                1.0
            };
            sample *= attack.min(release);
        }

        voice.cursor += voice.ratio;
        voice.fade_position = voice.fade_position.saturating_add(1);
        if voice.cursor >= length as f64 {
            voice.active = false;
        }
        sample * voice.gain
    }
}

fn read_loop_sample(
    loop_asset: &GrandPrixDecodedLoop,
    cursor: &mut f64,
    anti_alias_filter: &mut Biquad,
    filter_cutoff: &mut f32,
    output_sample_rate: u32,
    rate: f64,
) -> f32 {
    let start = loop_asset.loop_start_frame;
    let end = loop_asset.loop_end_frame_exclusive;
    let pcm = &loop_asset.pcm;
    if pcm.is_empty() || end <= start || end > pcm.len() {
        return 0.0;
    }
    let start_f = start as f64;
    let end_f = end as f64;
    let mut position = *cursor;
    if position < start_f || position >= end_f {
        position = start_f;
    }
    let mut first_index = position.floor() as usize;
    if first_index >= end {
        first_index = start;
    }
    let mut second_index = first_index + 1;
    if second_index >= end {
        second_index = start;
    }
    let fraction = (position - first_index as f64) as f32;
    let first = pcm[first_index] as f32 / 32768.0;
    let second = pcm[second_index] as f32 / 32768.0;
    let mut sample = first + (second - first) * fraction;

    let cutoff = (0.45 / rate.max(1.0) * output_sample_rate as f64).min(19_000.0) as f32;
    let previous = *filter_cutoff;
    if (cutoff - previous).abs() / previous.max(1.0) > 0.01 {
        anti_alias_filter.update_lowpass(output_sample_rate as f32, cutoff);
        *filter_cutoff = cutoff;
    }
    sample = anti_alias_filter.process(sample);

    position += rate * 44_100.0 / output_sample_rate as f64;
    if position >= end_f {
        position -= end_f - start_f;
        if position >= end_f {
            position = start_f + position.rem_euclid(end_f - start_f);
        }
    }
    *cursor = position;
    sample
}

fn powered_engine_loop_weights(
    bank: &GrandPrixSampleBank,
    revolutions_per_minute: f64,
) -> [f32; GRAND_PRIX_MAXIMUM_LOOP_COUNT] {
    let mut weights = [0.0; GRAND_PRIX_MAXIMUM_LOOP_COUNT];
    if bank.loops[0].presence_curve.is_empty() {
        let (weight_from, weight_to, transition_index) =
            zone_blend(&bank.transitions, revolutions_per_minute);
        weights[transition_index] = weight_from;
        if transition_index + 1 < bank.loops.len() {
            weights[transition_index + 1] = weight_to;
        }
        return weights;
    }
    let mut squared_weight_sum = 0.0;
    for (index, loop_asset) in bank.loops.iter().enumerate() {
        let relative_weight =
            engine_loop_presence_weight(&loop_asset.presence_curve, revolutions_per_minute) as f32;
        weights[index] = relative_weight;
        squared_weight_sum += relative_weight * relative_weight;
    }
    if squared_weight_sum > 0.0 {
        let normalization = squared_weight_sum.sqrt().recip();
        for weight in weights.iter_mut().take(bank.loops.len()) {
            *weight *= normalization;
        }
    }
    weights
}

fn zone_blend(transitions: &[GrandPrixTransition], rpm: f64) -> (f32, f32, usize) {
    let mut transition_index = 0usize;
    for (index, transition) in transitions.iter().enumerate() {
        if rpm >= transition.end_revolutions_per_minute {
            transition_index = index + 1;
            continue;
        }
        if rpm >= transition.start_revolutions_per_minute {
            let width =
                transition.end_revolutions_per_minute - transition.start_revolutions_per_minute;
            let position = ((rpm - transition.start_revolutions_per_minute) / width) as f32;
            let control = position * position * (3.0 - 2.0 * position);
            let angle = control * std::f32::consts::FRAC_PI_2;
            return (angle.cos(), angle.sin(), index);
        }
        break;
    }
    (1.0, 0.0, transition_index)
}

fn tooth_pulse_harmonic_gains(duty_cycle: f32, casing_resonance_gains: &[f32]) -> Vec<f32> {
    let mut harmonic_gains: Vec<f32> = casing_resonance_gains
        .iter()
        .enumerate()
        .map(|(index, casing_resonance_gain)| {
            let harmonic_order = (index + 1) as f32;
            let pulse_fourier_gain = 2.0
                * (std::f32::consts::PI * harmonic_order * duty_cycle).sin()
                / (std::f32::consts::PI * harmonic_order);
            pulse_fourier_gain * casing_resonance_gain
        })
        .collect();
    let total_harmonic_gain = harmonic_gains.iter().map(|gain| gain.abs()).sum::<f32>();
    if total_harmonic_gain > f32::EPSILON {
        for harmonic_gain in &mut harmonic_gains {
            *harmonic_gain /= total_harmonic_gain;
        }
    }
    harmonic_gains
}

fn render_band_limited_tooth_pulse(
    phase: f64,
    fundamental_hertz: f32,
    gains: &[f32],
    sample_rate: f32,
) -> f32 {
    let nyquist_hertz = sample_rate * 0.5;
    let audible_gain_sum = gains
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let harmonic_order = index + 1;
            let harmonic_hertz = fundamental_hertz * harmonic_order as f32;
            harmonic_hertz > 0.0 && harmonic_hertz < nyquist_hertz
        })
        .map(|(_, gain)| gain.abs())
        .sum::<f32>();
    if audible_gain_sum <= f32::EPSILON {
        return 0.0;
    }
    gains
        .iter()
        .enumerate()
        .filter_map(|(index, gain)| {
            let harmonic_order = index + 1;
            let harmonic_hertz = fundamental_hertz * harmonic_order as f32;
            (harmonic_hertz > 0.0 && harmonic_hertz < nyquist_hertz)
                .then(|| (phase * harmonic_order as f64).cos() as f32 * gain / audible_gain_sum)
        })
        .sum()
}

fn advance_phase(phase: f64, frequency_hertz: f32, sample_rate: f32) -> f64 {
    (phase + std::f64::consts::TAU * frequency_hertz as f64 / sample_rate as f64)
        .rem_euclid(std::f64::consts::TAU)
}

fn next_rng_state(state: u64) -> u64 {
    let mut value = state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    value
}

fn smoothstep(edge_start: f64, edge_end: f64, value: f64) -> f64 {
    let position = ((value - edge_start) / (edge_end - edge_start)).clamp(0.0, 1.0);
    position * position * (3.0 - 2.0 * position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn schedule_loaded_upshift(sampler: &mut GrandPrixSampler, load: f32, next_gear: i32) {
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 18_000.0,
            gear: 3,
            throttle: 1.0,
            normalized_transmitted_load: load,
            dt_seconds: 0.01,
            ..GrandPrixTelemetry::default()
        });
        for _ in 0..441 {
            sampler.render_sample_components();
        }
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 17_500.0,
            gear: 3,
            shift_phase: SHIFT_PHASE_UPSHIFT_CUT,
            dt_seconds: 0.06,
            ..GrandPrixTelemetry::default()
        });
        for _ in 0..2646 {
            sampler.render_sample_components();
        }
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 15_375.0,
            gear: next_gear,
            shift_phase: 2,
            throttle: 1.0,
            normalized_transmitted_load: load,
            dt_seconds: 0.01,
            ..GrandPrixTelemetry::default()
        });
    }

    #[test]
    fn acoustic_upshift_recovery_decays_without_changing_telemetry_speed() {
        let mut sampler = shipped_sampler();
        schedule_loaded_upshift(&mut sampler, 1.0, 4);
        let mut early_peak = 0.0_f64;
        let mut late_peak = 0.0_f64;
        for frame in 0..17_640 {
            sampler.render_sample_components();
            let modulation = sampler.diagnostics.upshift_acoustic_pitch_modulation.abs() as f64;
            if frame < 2205 {
                early_peak = early_peak.max(modulation);
            }
            if frame > 13_230 {
                late_peak = late_peak.max(modulation);
            }
        }
        assert!(early_peak > 0.005 && early_peak <= 0.035);
        assert!(late_peak < early_peak * 0.20);
        assert_eq!(sampler.target_rpm, 15_375.0);
        assert!((sampler.smoothed_rpm - 15_375.0).abs() < 0.01);
        assert!(sampler.upshift_acoustic_recoveries.is_empty());
    }

    #[test]
    fn upshift_amplitude_tracks_load_independently_of_saturated_pitch() {
        let mut full_load_sampler = shipped_sampler();
        let mut light_load_sampler = shipped_sampler();
        schedule_loaded_upshift(&mut full_load_sampler, 1.0, 4);
        schedule_loaded_upshift(&mut light_load_sampler, 0.25, 4);
        let mut amplitude_peak = 0.0_f32;
        for _ in 0..17_640 {
            full_load_sampler.render_sample_components();
            light_load_sampler.render_sample_components();
            let full_load = full_load_sampler.diagnostics;
            let light_load = light_load_sampler.diagnostics;
            assert!((full_load.upshift_acoustic_pitch_modulation
                - light_load.upshift_acoustic_pitch_modulation).abs() < 0.000001);
            assert!((full_load.upshift_acoustic_amplitude_modulation * 0.25
                - light_load.upshift_acoustic_amplitude_modulation).abs() < 0.000001);
            amplitude_peak = amplitude_peak.max(full_load.upshift_acoustic_amplitude_modulation.abs());
        }
        assert!(amplitude_peak > 0.17 && amplitude_peak <= 0.20);
        assert_eq!(full_load_sampler.diagnostics.upshift_acoustic_amplitude_modulation, 0.0);
        assert_eq!(full_load_sampler.diagnostics.upshift_acoustic_pitch_modulation, 0.0);
    }

    #[test]
    fn acoustic_upshift_recovery_requires_load_and_confirmed_higher_gear() {
        for (load, next_gear) in [(0.0, 4), (1.0, 3), (1.0, 2), (1.0, 0)] {
            let mut sampler = shipped_sampler();
            schedule_loaded_upshift(&mut sampler, load, next_gear);
            for _ in 0..2205 {
                sampler.render_sample_components();
                assert_eq!(sampler.diagnostics.upshift_acoustic_pitch_modulation, 0.0);
            }
        }
    }

    #[test]
    fn physical_output_shaft_pitch_survives_engine_speed_and_gear_changes() {
        let mut sampler = shipped_sampler();
        sampler.set_gearbox_whine_transmission(&[3.0, 2.0], 4.0, 3.0, 20.0, 40.0);
        sampler.set_output_shaft_speed_hertz(Some(100.0));
        for (speed, gear) in [(18_000.0, 1), (12_000.0, 2), (7000.0, 0)] {
            assert_eq!(
                sampler.gearbox_mesh_frequencies(speed, gear),
                (2000.0, 1000.0)
            );
        }
        sampler.set_output_shaft_speed_hertz(Some(150.0));
        assert_eq!(
            sampler.gearbox_mesh_frequencies(12_000.0, 2),
            (3000.0, 1500.0)
        );
    }

    #[test]
    fn upshift_flutter_waits_for_manual_clutch_reengagement() {
        let mut sampler = shipped_sampler();
        for (gear, speed, phase, load) in [
            (3, 18_000.0, 0, 0.0),
            (3, 18_000.0, 1, 0.0),
            (4, 18_000.0, 2, 0.0),
        ] {
            sampler.ingest(&GrandPrixTelemetry {
                rpm: speed, gear, shift_phase: phase,
                throttle: 1.0, normalized_transmitted_load: load,
                dt_seconds: 0.02, ..GrandPrixTelemetry::default()
            });
            for _ in 0..882 {
                sampler.render_sample_components();
                assert_eq!(sampler.diagnostics.upshift_acoustic_pitch_modulation, 0.0);
            }
        }
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 15_375.0, gear: 4, throttle: 1.0,
            normalized_transmitted_load: 0.8, dt_seconds: 0.02,
            ..GrandPrixTelemetry::default()
        });
        let mut peak_modulation = 0.0_f32;
        for _ in 0..2205 {
            sampler.render_sample_components();
            peak_modulation = peak_modulation.max(sampler.diagnostics.upshift_acoustic_pitch_modulation.abs());
        }
        assert!(peak_modulation > 0.005);
        assert_eq!(sampler.target_rpm, 15_375.0);
    }

    #[test]
    fn gearbox_whine_responds_to_transmitted_load_and_drive_side() {
        let mut levels = Vec::new();
        for (load, sign) in [(0.0, 1), (1.0, 1), (1.0, -1)] {
            let mut sampler = shipped_sampler();
            sampler.set_output_shaft_speed_hertz(Some(100.0));
            sampler.set_transmitted_torque_sign(Some(sign));
            sampler.ingest(&GrandPrixTelemetry {
                rpm: 12_000.0,
                gear: 3,
                normalized_transmitted_load: load,
                transmitted_torque_sign: 1,
                dt_seconds: 0.01,
                ..GrandPrixTelemetry::default()
            });
            for _ in 0..88_200 {
                sampler.render_sample_components();
            }
            levels.push(sampler.diagnostics.transmitted_load_gain);
        }
        assert!(levels[1] > levels[0] * 8.0);
        assert!((levels[2] / levels[1] - 0.78).abs() < 0.01);
    }

    #[test]
    fn loaded_upshift_recovery_is_allocation_free_and_resettable() {
        let mut sampler = shipped_sampler();
        crate::allocation_probe::start();
        schedule_loaded_upshift(&mut sampler, 1.0, 4);
        for _ in 0..11_025 {
            sampler.render_sample_components();
        }
        let (allocations, _) = crate::allocation_probe::stop();
        assert_eq!(allocations, 0);
        schedule_loaded_upshift(&mut sampler, 1.0, 4);
        sampler.reset();
        assert!(sampler.upshift_acoustic_recoveries.is_empty());
        assert!(sampler.upshift_departure.is_none());
        assert_eq!(sampler.output_shaft_speed_hertz, None);
    }

    #[test]
    fn batched_upshift_recovery_starts_at_the_scheduled_completion_frame() {
        let mut sampler = shipped_sampler();
        for (gear, speed, phase, load) in [
            (3, 18_000.0, 0, 1.0),
            (3, 17_500.0, 1, 0.0),
            (4, 15_375.0, 2, 1.0),
        ] {
            sampler.ingest(&GrandPrixTelemetry {
                rpm: speed,
                gear,
                shift_phase: phase,
                normalized_transmitted_load: load,
                dt_seconds: 0.01,
                ..GrandPrixTelemetry::default()
            });
        }
        for _ in 0..882 {
            sampler.render_sample_components();
            assert_eq!(sampler.diagnostics.upshift_acoustic_pitch_modulation, 0.0);
        }
        let mut peak_modulation = 0.0_f32;
        for _ in 0..882 {
            sampler.render_sample_components();
            peak_modulation =
                peak_modulation.max(sampler.diagnostics.upshift_acoustic_pitch_modulation.abs());
        }
        assert!(peak_modulation > 0.005);
    }

    #[test]
    fn upshift_recording_preserves_its_audible_tail() {
        let mut sampler = shipped_sampler();
        sampler.ingest_direct(GrandPrixEventKind::Upshift, 18_000.0);
        let mut contact_peak = 0.0_f32;
        for _ in 0..1200 {
            contact_peak = contact_peak.max(sampler.render_sample_components().gearbox.abs());
        }
        assert!(contact_peak > 0.0);
        let mut tail_peak = 0.0_f32;
        for _ in 0..4410 {
            tail_peak = tail_peak.max(sampler.render_sample_components().gearbox.abs());
        }
        assert!(tail_peak > 0.01);
        for _ in 0..22050 {
            sampler.render_sample_components();
        }
        for _ in 0..4410 {
            assert_eq!(sampler.render_sample_components().gearbox, 0.0);
        }
    }

    fn shipped_bank_directory() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds/banks/v10-v2-bank")
    }

    fn shipped_sampler() -> GrandPrixSampler {
        let bank = GrandPrixSampleBank::load(&shipped_bank_directory()).expect("bank");
        GrandPrixSampler::new(bank, 44_100, Some(4500.0), Some(18_000.0)).expect("sampler")
    }

    fn advance(sampler: &mut GrandPrixSampler, frames: usize) -> Vec<f32> {
        (0..frames).map(|_| sampler.render_sample()).collect()
    }

    fn ingest_steady(sampler: &mut GrandPrixSampler, rpm: f64, throttle: f32, frames: usize) {
        sampler.ingest(&GrandPrixTelemetry {
            rpm,
            throttle,
            gear: 3,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: frames as f64 / 44_100.0,

            ..GrandPrixTelemetry::default()
        });
    }

    #[test]
    fn steady_holds_cover_the_physical_range() {
        let mut sampler = shipped_sampler();
        for rpm in [4500.0, 6500.0, 9000.0, 11500.0, 15000.0, 18000.0] {
            sampler.reset();
            ingest_steady(&mut sampler, rpm, 0.9, 4410);
            let output = advance(&mut sampler, 4410);
            let peak = output
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
            assert!(peak > 0.02, "rpm {rpm} rendered almost silence: {peak}");
            let diagnostics = sampler.diagnostics();
            assert!(
                (1..=3).contains(&diagnostics.active_zone_count),
                "rpm {rpm} active zones {}",
                diagnostics.active_zone_count
            );
            let power_sum: f32 = diagnostics
                .zone_weights
                .iter()
                .map(|weight| weight * weight)
                .sum();
            assert!(
                (power_sum - 1.0).abs() < 1e-3,
                "rpm {rpm} power sum {power_sum}"
            );
            assert!(
                diagnostics.clamped_rate_samples < 8,
                "rpm {rpm} clamped {} times rendered={} rates={:?}",
                diagnostics.clamped_rate_samples,
                diagnostics.rendered_revolutions_per_minute,
                (0..sampler.bank.loops.len())
                    .map(|index| format!(
                        "z{index}=({:.17},{:.17},{:.17})",
                        diagnostics.rendered_revolutions_per_minute as f64
                            / sampler.bank.loops[index].reference_revolutions_per_minute,
                        sampler.bank.loops[index].valid_playback_rate_min,
                        sampler.bank.loops[index].valid_playback_rate_max
                    ))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }

    #[test]
    fn inactive_powered_and_coast_loops_keep_the_same_engine_cycle_phase() {
        for output_sample_rate in [44_100, 48_000] {
            let bank = GrandPrixSampleBank::load(&shipped_bank_directory()).expect("bank");
            let mut sampler =
                GrandPrixSampler::new(bank, output_sample_rate, None, None).expect("sampler");
            let mut accumulated_cycles = 0.0;
            for frame in 0..output_sample_rate * 4 {
                if frame % 400 == 0 {
                    let revolutions_per_minute = if (frame / (output_sample_rate / 2)) % 2 == 0 {
                        5_000.0
                    } else {
                        17_000.0
                    };
                    ingest_steady(&mut sampler, revolutions_per_minute, 1.0, 400);
                }
                sampler.render_sample_components();
                accumulated_cycles += sampler.smoothed_rpm / (120.0 * output_sample_rate as f64);
            }
            for (loop_asset, cursor) in sampler.bank.loops.iter().zip(&sampler.loop_cursors).chain(
                sampler
                    .bank
                    .coast_loops
                    .iter()
                    .zip(&sampler.coast_loop_cursors),
            ) {
                let cycles_per_frame =
                    loop_asset.reference_revolutions_per_minute / (120.0 * 44_100.0);
                let cycle_count = loop_asset.pcm.len() as f64 * cycles_per_frame;
                let observed_cycles = cursor * cycles_per_frame;
                let expected_cycles = accumulated_cycles.rem_euclid(cycle_count);
                assert!(
                    (observed_cycles - expected_cycles).abs() < 1e-7,
                    "{} at {}: {} != {}",
                    loop_asset.id,
                    output_sample_rate,
                    observed_cycles,
                    expected_cycles
                );
            }
        }
    }

    #[test]
    fn reset_reproduces_the_same_filtered_engine_output() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 6_000.0, 1.0, 4410);
        let first_output = advance(&mut sampler, 4410);
        ingest_steady(&mut sampler, 18_000.0, 0.0, 4410);
        advance(&mut sampler, 4410);
        sampler.reset();
        ingest_steady(&mut sampler, 6_000.0, 1.0, 4410);
        assert_eq!(first_output, advance(&mut sampler, 4410));
    }

    #[test]
    fn powered_presence_weights_remain_continuous_and_normalized() {
        let bank = GrandPrixSampleBank::load(&shipped_bank_directory()).expect("bank");
        let mut previous_weights = powered_engine_loop_weights(&bank, 4500.0);
        for revolutions_per_minute in 4500..=18000 {
            let weights = powered_engine_loop_weights(&bank, revolutions_per_minute as f64);
            let squared_weight_sum: f32 = weights.iter().map(|weight| weight * weight).sum();
            assert!((squared_weight_sum - 1.0).abs() < 1e-6);
            for (weight, previous_weight) in weights.iter().zip(previous_weights) {
                assert!((0.0..=1.0).contains(weight));
                assert!((weight - previous_weight).abs() < 0.01);
            }
            previous_weights = weights;
        }
    }

    #[test]
    fn five_powered_samples_crossfade_and_medium_exits_at_thirteen_thousand() {
        let bank = GrandPrixSampleBank::load(&shipped_bank_directory()).expect("bank");
        assert_eq!(bank.loops[0].id, "engine_idle_loop");
        assert_eq!(bank.loops[2].id, "engine_mid_on_loop");
        for (revolutions_per_minute, sample_index) in [
            (4500.0, 0), (4800.0, 0), (5600.0, 1), (9000.0, 1),
            (11500.0, 2), (12000.0, 2), (13000.0, 3), (14000.0, 3),
            (16000.0, 4), (16500.0, 4), (17500.0, 5), (18000.0, 5),
        ] {
            let weights = powered_engine_loop_weights(&bank, revolutions_per_minute);
            assert_eq!(weights[sample_index], 1.0);
            assert_eq!(weights.iter().filter(|weight| **weight > 0.0).count(), 1);
        }
        for (revolutions_per_minute, sample_index) in [
            (5200.0, 0), (10500.0, 1), (12500.0, 2), (15250.0, 3), (17000.0, 4),
        ] {
            let weights = powered_engine_loop_weights(&bank, revolutions_per_minute);
            assert!((weights[sample_index] - weights[sample_index + 1]).abs() < 1e-6);
            assert_eq!(weights.iter().filter(|weight| **weight > 0.0).count(), 2);
        }
        for revolutions_per_minute in [13000.0, 14000.0, 15000.0, 16000.0, 18000.0] {
            let weights = powered_engine_loop_weights(&bank, revolutions_per_minute);
            assert_eq!(weights[2], 0.0);
        }
    }

    #[test]
    fn banks_without_presence_curves_keep_the_sequential_transition_mixer() {
        let mut bank = GrandPrixSampleBank::load(&shipped_bank_directory()).expect("bank");
        for loop_asset in &mut bank.loops {
            loop_asset.presence_curve.clear();
        }
        for revolutions_per_minute in (4500..=18000).step_by(25) {
            let weights = powered_engine_loop_weights(&bank, revolutions_per_minute as f64);
            let (weight_from, weight_to, transition_index) =
                zone_blend(&bank.transitions, revolutions_per_minute as f64);
            assert_eq!(weights[transition_index], weight_from);
            if transition_index + 1 < bank.loops.len() {
                assert_eq!(weights[transition_index + 1], weight_to);
            }
        }
    }

    #[test]
    fn upshift_edge_triggers_exactly_once_per_transaction() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 9000.0, 0.9, 441);
        let before = sampler.diagnostics().accepted_events;
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 9000.0,
            throttle: 0.0,
            gear: 4,
            shift_phase: 1,
            rev_limiter_active: false,
            dt_seconds: 0.01,

            ..GrandPrixTelemetry::default()
        });
        for _ in 0..4 {
            sampler.ingest(&GrandPrixTelemetry {
                rpm: 8800.0,
                throttle: 0.0,
                gear: 4,
                shift_phase: 1,
                rev_limiter_active: false,
                dt_seconds: 0.01,

                ..GrandPrixTelemetry::default()
            });
        }
        advance(&mut sampler, 4410);
        let diagnostics = sampler.diagnostics();
        assert_eq!(diagnostics.accepted_events - before, 1);
        assert_eq!(diagnostics.late_events, 0);
    }

    #[test]
    fn shift_phases_duck_and_restore_the_continuous_engine() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 12_000.0, 0.9, 441);
        advance(&mut sampler, 4_410);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 12_000.0,
            throttle: 0.0,
            gear: 4,
            shift_phase: SHIFT_PHASE_UPSHIFT_CUT,
            rev_limiter_active: false,
            dt_seconds: 0.02,

            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 1_323);
        assert!(sampler.shift_engine_gain < 0.30);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 10_100.0,
            throttle: 0.7,
            gear: 5,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.08,

            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 8_820);
        assert!(sampler.shift_engine_gain > 0.98);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 10_100.0,
            throttle: 0.2,
            gear: 4,
            shift_phase: SHIFT_PHASE_DOWNSHIFT_CUT,
            rev_limiter_active: false,
            dt_seconds: 0.02,

            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 1_323);
        assert!(sampler.shift_engine_gain > 0.40);
        assert!(sampler.shift_engine_gain < 0.52);
    }

    #[test]
    fn three_coast_samples_remain_audible_across_transition_boundaries() {
        let mut sampler = shipped_sampler();
        for revolutions_per_minute in [
            4500.0, 9000.0, 9500.0, 10500.0, 11500.0,
            13000.0, 14000.0, 15000.0, 16000.0, 18000.0,
        ] {
            sampler.reset();
            ingest_steady(&mut sampler, revolutions_per_minute, 0.0, 4410);
            advance(&mut sampler, 4410);
            let samples = advance(&mut sampler, 4410);
            assert!(samples.iter().all(|sample| sample.is_finite() && sample.abs() < 1.0));
            assert!(samples.iter().any(|sample| sample.abs() > 0.02));
            let (weight_from, weight_to, transition_index) =
                zone_blend(&sampler.bank.coast_transitions, revolutions_per_minute);
            assert!((weight_from * weight_from + weight_to * weight_to - 1.0).abs() < 1e-6);
            assert!(transition_index < 3);
            assert_eq!(sampler.diagnostics().clamped_rate_samples, 0);
        }
    }

    #[test]
    fn closed_throttle_uses_coast_loops_above_idle() {
        let mut sampler = shipped_sampler();
        assert_eq!(sampler.bank.coast_loops.len(), 3);
        sampler.set_coast_gain(1.0);
        ingest_steady(&mut sampler, 12_000.0, 0.0, 441);
        advance(&mut sampler, 8_820);
        let coast_samples = advance(&mut sampler, 4_410);
        let coast_level = coast_samples.iter().map(|sample| sample.abs()).sum::<f32>()
            / coast_samples.len() as f32;
        assert!(
            coast_level > 0.005,
            "coast loops were silent: {coast_level}"
        );

        sampler.reset();
        sampler.set_coast_gain(0.0);
        ingest_steady(&mut sampler, 12_000.0, 0.0, 441);
        advance(&mut sampler, 8_820);
        let muted_coast_samples = advance(&mut sampler, 4_410);
        let muted_coast_level = muted_coast_samples
            .iter()
            .map(|sample| sample.abs())
            .sum::<f32>()
            / muted_coast_samples.len() as f32;
        assert!(muted_coast_level < coast_level * 0.01);

        sampler.reset();
        ingest_steady(&mut sampler, 4_500.0, 0.0, 441);
        advance(&mut sampler, 8_820);
        let idle_samples = advance(&mut sampler, 4_410);
        let idle_level =
            idle_samples.iter().map(|sample| sample.abs()).sum::<f32>() / idle_samples.len() as f32;
        assert!(
            idle_level > 0.005,
            "shared idle loop was silent: {idle_level}"
        );
    }

    #[test]
    fn gear_change_without_shift_phase_does_not_trigger() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 9000.0, 0.9, 441);
        let before = sampler.diagnostics().accepted_events;
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 9000.0,
            throttle: 0.0,
            gear: 4,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.01,

            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 4410);
        assert_eq!(sampler.diagnostics().accepted_events, before);
    }

    #[test]
    fn lift_edge_respects_thresholds_and_cooldown() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 14_000.0, 0.9, 441);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 14_000.0,
            throttle: 0.1,
            gear: 5,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.01,

            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 4410);
        assert_eq!(sampler.diagnostics().accepted_events, 1);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 14_000.0,
            throttle: 0.9,
            gear: 5,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.2,

            ..GrandPrixTelemetry::default()
        });
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 14_000.0,
            throttle: 0.1,
            gear: 5,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.01,

            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 8820);
        assert_eq!(sampler.diagnostics().accepted_events, 1);
        assert_eq!(sampler.diagnostics().queued_events, 0);
    }

    #[test]
    fn limiter_entry_triggers_once_per_edge() {
        let mut bank = GrandPrixSampleBank::load(&shipped_bank_directory()).expect("bank");
        let limiter_group = bank
            .groups
            .iter_mut()
            .find(|group| group.role == GrandPrixEventRole::LimiterEvent)
            .expect("limiter group");
        limiter_group.trigger_event = GrandPrixTriggerEvent::LimiterEntryEdge;
        let mut sampler = GrandPrixSampler::new(bank, 44_100, None, None).expect("sampler");
        ingest_steady(&mut sampler, 17_500.0, 1.0, 441);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 17_500.0,
            throttle: 1.0,
            gear: 6,
            shift_phase: 0,
            rev_limiter_active: true,
            dt_seconds: 0.01,

            ..GrandPrixTelemetry::default()
        });
        for _ in 0..10 {
            sampler.ingest(&GrandPrixTelemetry {
                rpm: 17_500.0,
                throttle: 1.0,
                gear: 6,
                shift_phase: 0,
                rev_limiter_active: true,
                dt_seconds: 0.01,

                ..GrandPrixTelemetry::default()
            });
        }
        advance(&mut sampler, 4410);
        assert_eq!(sampler.diagnostics().accepted_events, 1);
    }

    fn ingest_limiter_cut(
        sampler: &mut GrandPrixSampler,
        cut_active: bool,
        duration_frames: usize,
    ) {
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 18_500.0,
            throttle: 1.0,
            gear: 6,
            shift_phase: 0,
            rev_limiter_active: cut_active,
            dt_seconds: duration_frames as f64 / 44_100.0,

            ..GrandPrixTelemetry::default()
        });
    }

    #[test]
    fn limiter_cut_windows_preserve_batched_pulses_without_cooldown() {
        let mut sampler = shipped_sampler();
        for cut_active in [false, true, false, true, false] {
            ingest_limiter_cut(&mut sampler, cut_active, 100);
        }
        let limiter_samples: Vec<f32> = (0..550)
            .map(|_| sampler.render_sample_components().limiter)
            .collect();
        assert!(limiter_samples[..100].iter().all(|sample| *sample == 0.0));
        assert!(limiter_samples[112..200]
            .iter()
            .any(|sample| sample.abs() > 0.01));
        assert!(limiter_samples[212..300]
            .iter()
            .all(|sample| *sample == 0.0));
        assert!(limiter_samples[312..400]
            .iter()
            .any(|sample| sample.abs() > 0.01));
        assert!(limiter_samples[412..].iter().all(|sample| *sample == 0.0));
        assert_eq!(sampler.diagnostics().accepted_limiter_events, 0);
    }

    #[test]
    fn limiter_high_pass_rejects_bass_and_preserves_high_frequencies() {
        let mut band_levels = Vec::new();
        for frequency_hertz in [150.0_f64, 3000.0] {
            let mut sampler = shipped_sampler();
            let event_index = sampler.limiter_cut_variant.expect("limiter variant").event_index;
            sampler.bank.events[event_index].pcm = (0..44_100)
                .map(|frame| {
                    (16_000.0
                        * (std::f64::consts::TAU * frequency_hertz * frame as f64 / 44_100.0).sin())
                        as i16
                })
                .collect();
            ingest_limiter_cut(&mut sampler, true, 20_000);
            let mut energy = 0.0_f64;
            for frame in 0..20_000 {
                let sample = sampler.render_sample_components().limiter as f64;
                if frame >= 5000 {
                    energy += sample * sample;
                }
            }
            band_levels.push((energy / 15_000.0).sqrt());
            for _ in 0..20 {
                sampler.render_sample_components();
            }
            assert_eq!(sampler.render_sample_components().limiter, 0.0);
        }
        assert!(band_levels[0] < band_levels[1] * 0.1);
        assert!(band_levels[1] > 0.1);
    }

    #[test]
    fn limiter_cut_audio_expires_even_without_recovery_telemetry() {
        let mut sampler = shipped_sampler();
        ingest_limiter_cut(&mut sampler, true, 100);
        let limiter_samples: Vec<f32> = (0..200)
            .map(|_| sampler.render_sample_components().limiter)
            .collect();
        assert!(limiter_samples[..100]
            .iter()
            .any(|sample| sample.abs() > 0.01));
        assert!(limiter_samples[112..].iter().all(|sample| *sample == 0.0));
        assert!(!sampler.diagnostics().limiter_voice_active);
    }

    #[test]
    fn limiter_cut_audio_never_uses_revolutions_threshold_or_direct_event() {
        let mut sampler = shipped_sampler();
        ingest_limiter_cut(&mut sampler, false, 100);
        sampler.ingest_direct(GrandPrixEventKind::Limiter, 19_000.0);
        for _ in 0..200 {
            assert_eq!(sampler.render_sample_components().limiter, 0.0);
        }
        assert_eq!(sampler.diagnostics().accepted_limiter_events, 0);
        assert!(sampler.event_queue.is_empty());
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 9_000.0,
            rev_limiter_active: true,
            dt_seconds: 0.01,
            ..GrandPrixTelemetry::default()
        });
        assert!((0..100).any(|_| sampler.render_sample_components().limiter.abs() > 0.01));
    }

    #[test]
    fn limiter_cut_windows_retain_fractional_physics_step_timing() {
        let mut sampler = shipped_sampler();
        for _ in 0..600 {
            sampler.ingest(&GrandPrixTelemetry {
                rpm: 18_000.0,
                rev_limiter_active: true,
                dt_seconds: 1.0 / 600.0,
                ..GrandPrixTelemetry::default()
            });
        }
        assert_eq!(sampler.ingest_frame_cursor, 44_100);
        assert_eq!(sampler.limiter_cut_windows.len(), 1);
        assert_eq!(sampler.limiter_cut_windows[0].end_frame_exclusive, 44_100);
    }

    #[test]
    fn limiter_cut_recording_preserves_pitch_at_alternate_output_sample_rate() {
        let bank = GrandPrixSampleBank::load(&shipped_bank_directory()).expect("bank");
        let mut sampler = GrandPrixSampler::new(bank, 48_000, None, None).expect("sampler");
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 18_000.0,
            rev_limiter_active: true,
            dt_seconds: 0.1,
            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 4800);
        assert!((sampler.limiter_cut_cursor - 4410.0).abs() < 1e-6);
        advance(&mut sampler, 12);
        assert_eq!(sampler.render_sample_components().limiter, 0.0);
    }

    #[test]
    fn limiter_cut_window_overflow_fails_silent_without_allocating() {
        let mut sampler = shipped_sampler();
        crate::allocation_probe::start();
        for _ in 0..=GRAND_PRIX_LIMITER_CUT_WINDOW_CAPACITY {
            ingest_limiter_cut(&mut sampler, true, 100);
            ingest_limiter_cut(&mut sampler, false, 100);
        }
        for _ in 0..100 {
            assert_eq!(sampler.render_sample_components().limiter, 0.0);
        }
        let (allocations, _) = crate::allocation_probe::stop();
        assert_eq!(allocations, 0);
        assert_eq!(sampler.diagnostics().overflowed_events, 1);
    }

    #[test]
    fn limiter_cut_audio_wraps_recording_during_sustained_physical_cut() {
        let mut sampler = shipped_sampler();
        let variant = sampler.limiter_cut_variant.expect("cut variant");
        let recording_length = sampler.bank.events[variant.event_index].pcm.len();
        for _ in 0..20 {
            ingest_limiter_cut(&mut sampler, true, 20_000);
        }
        advance(&mut sampler, recording_length - 5);
        let wrap_samples: Vec<f32> = (0..20)
            .map(|_| sampler.render_sample_components().limiter)
            .collect();
        assert!(wrap_samples.iter().all(|sample| sample.is_finite()));
        assert!(wrap_samples.iter().any(|sample| sample.abs() > 0.01));
        assert!(wrap_samples
            .windows(2)
            .all(|pair| (pair[1] - pair[0]).abs() < 0.2));
        assert_eq!(sampler.diagnostics().accepted_limiter_events, 0);
    }

    #[test]
    fn limiter_cut_reset_clears_pending_audio_and_playback_position() {
        let mut sampler = shipped_sampler();
        ingest_limiter_cut(&mut sampler, true, 100);
        advance(&mut sampler, 50);
        sampler.reset();
        assert!(sampler.limiter_cut_windows.is_empty());
        assert_eq!(sampler.limiter_cut_cursor, 0.0);
        for _ in 0..100 {
            assert_eq!(sampler.render_sample_components().limiter, 0.0);
        }
    }

    #[test]
    fn multiple_ingests_before_render_preserve_event_order() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 9000.0, 0.9, 1);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 9000.0,
            throttle: 0.0,
            gear: 4,
            shift_phase: 1,
            rev_limiter_active: false,
            dt_seconds: 0.05,

            ..GrandPrixTelemetry::default()
        });
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 8800.0,
            throttle: 0.0,
            gear: 3,
            shift_phase: 3,
            rev_limiter_active: false,
            dt_seconds: 0.05,

            ..GrandPrixTelemetry::default()
        });
        assert_eq!(sampler.event_queue.len(), 2);
        let events: Vec<(GrandPrixEventKind, u64)> = sampler
            .event_queue
            .iter()
            .map(|event| (event.kind, event.frame))
            .collect();
        assert_eq!(events[0].0, GrandPrixEventKind::Upshift);
        assert_eq!(events[1].0, GrandPrixEventKind::Downshift);
        assert!(
            events[0].1 < events[1].1,
            "queued events collapsed: {events:?}"
        );
        advance(&mut sampler, 8820);
        assert_eq!(sampler.diagnostics().accepted_events, 2);
    }

    #[test]
    fn direct_commands_schedule_without_telemetry() {
        let mut sampler = shipped_sampler();
        sampler.ingest_direct(GrandPrixEventKind::Downshift, 9000.0);
        sampler.ingest_direct(GrandPrixEventKind::LiftBackfire, 14_000.0);
        advance(&mut sampler, 44_100);
        assert_eq!(sampler.diagnostics().accepted_events, 2);
    }

    #[test]
    fn queue_overflow_is_counted_not_lost_silently() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 9000.0, 0.9, 441);
        for packet in 0..400 {
            sampler.ingest(&GrandPrixTelemetry {
                rpm: 9000.0,
                throttle: 0.0,
                gear: 4,
                shift_phase: if packet % 2 == 0 { 1 } else { 0 },
                rev_limiter_active: false,
                dt_seconds: 0.05,
                ..GrandPrixTelemetry::default()
            });
        }
        assert!(sampler.event_queue.len() <= GRAND_PRIX_EVENT_QUEUE_CAPACITY);
        assert!(sampler.diagnostics.overflowed_events > 0);
    }

    #[test]
    fn reset_clears_voices_and_schedule() {
        let mut sampler = shipped_sampler();
        sampler.ingest_direct(GrandPrixEventKind::Downshift, 9000.0);
        sampler.reset();
        assert_eq!(sampler.diagnostics().queued_events, 0);
        advance(&mut sampler, 4410);
        assert_eq!(sampler.diagnostics().accepted_events, 0);
        assert_eq!(sampler.diagnostics().active_voice_count, 0);
    }

    #[test]
    fn gate_silences_engine_at_zero_rpm() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 9000.0, 1.0, 4410);
        advance(&mut sampler, 4410);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 0.0,
            throttle: 0.0,
            gear: 1,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.5,

            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 176_400);
        assert!(sampler.diagnostics().gate <= 1e-3);
    }

    #[test]
    fn gearbox_oscillators_remain_audible_at_low_rpm_and_in_neutral() {
        let mut sampler = shipped_sampler();
        sampler.set_gearbox_whine_gain(0.55);
        sampler.set_gearbox_whine_component_gains(0.12, 0.025);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 600.0,
            throttle: 0.0,
            gear: 0,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.1,

            ..GrandPrixTelemetry::default()
        });
        let neutral_energy: f32 = (0..4_410)
            .map(|_| sampler.render_sample_components().gearbox_whine.abs())
            .sum();
        assert!(neutral_energy > 1.0);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 600.0,
            throttle: 0.0,
            gear: 1,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.01,

            ..GrandPrixTelemetry::default()
        });
        let in_gear_energy: f32 = (0..4_410)
            .map(|_| sampler.render_sample_components().gearbox_whine.abs())
            .sum();
        assert!(in_gear_energy > neutral_energy * 0.7);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 0.0,
            throttle: 0.0,
            gear: 0,
            shift_phase: 0,
            rev_limiter_active: false,
            dt_seconds: 0.5,

            ..GrandPrixTelemetry::default()
        });
        advance(&mut sampler, 176_400);
        assert!(sampler.render_sample_components().gearbox_whine.abs() < 1e-4);
    }

    #[test]
    fn gearbox_whine_tracks_profile_gear_mesh_frequencies() {
        let mut sampler = shipped_sampler();
        sampler.set_gearbox_whine_transmission(
            &[3.45, 2.75, 2.30, 1.95, 1.68, 1.46],
            4.25,
            4.20,
            21.0,
            46.0,
        );
        let cases = [(4_500.0, 1, 456.52), (18_000.0, 6, 4_315.07)];
        for (revolutions_per_minute, gear, expected_gear_mesh_frequency_hertz) in cases {
            sampler.reset();
            sampler.ingest(&GrandPrixTelemetry {
                rpm: revolutions_per_minute,
                throttle: 0.8,
                gear,
                shift_phase: 0,
                rev_limiter_active: false,
                dt_seconds: 0.1,

                ..GrandPrixTelemetry::default()
            });
            advance(&mut sampler, 4_410);
            assert!(
                (sampler.gearbox_whine_gear_mesh_frequency_hertz
                    - expected_gear_mesh_frequency_hertz)
                    .abs()
                    < 8.0,
                "gear {gear} at {revolutions_per_minute} rpm produced {} Hz",
                sampler.gearbox_whine_gear_mesh_frequency_hertz
            );
        }
        assert!(sampler.gearbox_noise_low_cutoff_hertz > 9_000.0);
        assert!(sampler.gearbox_noise_high_cutoff_hertz < 21_168.0);
    }

    #[test]
    fn gearbox_tooth_pulse_places_its_strongest_partial_in_orders_three_to_five() {
        let harmonic_gains = tooth_pulse_harmonic_gains(
            GRAND_PRIX_GEARBOX_WHINE_TOOTH_CONTACT_DUTY_CYCLE,
            &GRAND_PRIX_GEARBOX_WHINE_GEAR_CASING_RESONANCE_GAINS,
        );
        let strongest_harmonic_order = harmonic_gains
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.abs().total_cmp(&right.1.abs()))
            .map(|(index, _)| index + 1)
            .expect("gear tooth pulse must contain harmonics");
        assert!((3..=5).contains(&strongest_harmonic_order));
        assert!((harmonic_gains.iter().map(|gain| gain.abs()).sum::<f32>() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn render_is_allocation_free_after_initialization() {
        let mut sampler = shipped_sampler();
        ingest_steady(&mut sampler, 9000.0, 0.9, 4410);
        advance(&mut sampler, 4410);
        crate::allocation_probe::start();
        sampler.ingest_direct(GrandPrixEventKind::Upshift, 9000.0);
        sampler.ingest(&GrandPrixTelemetry {
            rpm: 9600.0,
            throttle: 0.2,
            gear: 5,
            shift_phase: 0,
            rev_limiter_active: true,
            dt_seconds: 0.05,

            ..GrandPrixTelemetry::default()
        });
        for _ in 0..20_000 {
            let _ = sampler.render_sample();
        }
        let (allocations, _) = crate::allocation_probe::stop();
        assert_eq!(allocations, 0, "render path allocated {allocations} times");
    }
}
