//! Versioned powertrain synthesis contract (`audio.powertrain_synthesis`).
//!
//! Parses the `powertrain_synthesis` section from a physics-vehicle profile
//! JSON, produces a typed configuration, and validates physical coherence.
//! All generic defaults live in this module and reproduce a naturally
//! aspirated, 4-stroke, uniformly firing V10 (10 cylinders, 5 physically
//! simulated, 2 banks). This is a contract/parser/validation module only:
//! no synthesis, no runtime state.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::config::ConfigDiagnostic;

/// Dot path of the powertrain synthesis section inside a physics profile JSON.
pub const PROFILE_SECTION: &str = "audio.powertrain_synthesis";

/// Engine synthesis model family.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisModel {
    /// Half-block V10: one cylinder bank is physically simulated, the second
    /// bank is derived from it.
    #[default]
    #[serde(rename = "half_block_5_four_stroke")]
    HalfBlock5FourStroke,
}

/// Combustion cycle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Cycle {
    /// Complete combustion cycle over four strokes.
    #[default]
    FourStroke,
}

/// Aspiration type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Aspiration {
    /// No turbocharger or supercharger.
    #[default]
    NaturallyAspirated,
}

/// Limiter cut curve shape.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CutShape {
    /// Gradual cut starting before the limiter threshold.
    #[default]
    Soft,
    /// Hard cut right at the limiter threshold.
    Hard,
}

/// Combustion irregularity behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CombustionConfig {
    /// Cycle-to-cycle firing irregularity [0.0, 0.25].
    pub irregularity: f32,
    /// Deterministic RNG seed.
    pub seed: u64,
}

impl Default for CombustionConfig {
    fn default() -> Self {
        Self {
            irregularity: 0.01,
            seed: 0xF090_1994_D15C_A11D,
        }
    }
}

/// Energy response configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnergyConfig {
    /// Weight of the torque curve in the energy model [0.0, 1.0].
    pub torque_curve_weight: f32,
    /// Throttle response gain [0.1, 2.0].
    pub throttle_response: f32,
    /// Basal combustion energy retained at closed throttle [0.0, 0.5].
    pub idle_combustion_gain: f32,
    /// Attack smoothing in seconds (0.0, 0.5].
    pub attack_smoothing_s: f32,
    /// Release smoothing in seconds (0.0, 2.0].
    pub release_smoothing_s: f32,
    /// Load step smoothing in seconds (0.0, 0.5].
    pub load_smoothing_s: f32,
}

impl Default for EnergyConfig {
    fn default() -> Self {
        Self {
            torque_curve_weight: 1.0,
            throttle_response: 1.0,
            idle_combustion_gain: 0.08,
            attack_smoothing_s: 0.02,
            release_smoothing_s: 0.12,
            load_smoothing_s: 0.03,
        }
    }
}

/// A single band-pass resonance used by intake/exhaust filters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Resonance {
    /// Center frequency in Hz [20, 20000].
    pub frequency_hz: f32,
    /// Filter bandwidth factor [0.1, 30].
    pub q: f32,
    /// Peak gain in dB [-18, 18].
    pub gain_db: f32,
}

impl Default for Resonance {
    fn default() -> Self {
        Self {
            frequency_hz: 0.0,
            q: 1.0,
            gain_db: 0.0,
        }
    }
}

/// Intake resonance configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IntakeConfig {
    /// Enable the intake layer.
    pub enabled: bool,
    /// Broadband intake noise gain [0.0, 1.0].
    pub noise_gain: f32,
    /// How much throttle modulates intake noise [0.0, 1.0].
    pub throttle_follow: f32,
    /// Intake resonances.
    pub resonances: Vec<Resonance>,
}

impl Default for IntakeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            noise_gain: 0.35,
            throttle_follow: 0.8,
            resonances: vec![
                Resonance {
                    frequency_hz: 320.0,
                    q: 2.0,
                    gain_db: 3.0,
                },
                Resonance {
                    frequency_hz: 640.0,
                    q: 3.0,
                    gain_db: 1.5,
                },
            ],
        }
    }
}

/// Exhaust pulse configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExhaustConfig {
    /// Enable the exhaust layer.
    pub enabled: bool,
    /// Firing pulse gain [0.0, 2.0].
    pub pulse_gain: f32,
    /// Soft saturation amount [0.0, 1.0].
    pub saturation: f32,
    /// Phase cancellation damping [0.0, 0.99].
    pub damping: f32,
    /// Collector resonances.
    pub collector_resonances: Vec<Resonance>,
}

impl Default for ExhaustConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pulse_gain: 1.0,
            saturation: 0.30,
            damping: 0.50,
            collector_resonances: vec![
                Resonance {
                    frequency_hz: 190.0,
                    q: 4.0,
                    gain_db: 4.0,
                },
                Resonance {
                    frequency_hz: 380.0,
                    q: 2.5,
                    gain_db: 1.0,
                },
            ],
        }
    }
}

/// Half-block derivation (second bank from the first).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HalfBlockConfig {
    /// Firing phase offset of the second bank in degrees [0.0, 720.0].
    pub phase_offset_deg: f32,
    /// Delay of the second bank in seconds [0.0, 0.05].
    pub delay_s: f32,
    /// Decorrelation between banks [0.0, 1.0].
    pub decorrelation: f32,
    /// Second bank gain [0.0, 2.0].
    pub gain: f32,
    /// Timbre difference between banks [0.0, 1.0].
    pub timbre_diff: f32,
}

impl Default for HalfBlockConfig {
    fn default() -> Self {
        Self {
            // A four-stroke V10 fires every 720 / 10 = 72 degrees. The
            // reconstructed bank sits halfway between the 144-degree events
            // of the physically simulated five-cylinder bank.
            phase_offset_deg: 72.0,
            delay_s: 0.004,
            decorrelation: 0.35,
            gain: 0.98,
            timbre_diff: 0.06,
        }
    }
}

/// Global RPM limiter configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LimiterConfig {
    /// Threshold as a ratio of max RPM [0.90, 1.0].
    pub threshold_rpm_ratio: f32,
    /// Cut attack in ms [0.0, 100.0].
    pub attack_ms: f32,
    /// Cut release in ms [1.0, 1000.0].
    pub release_ms: f32,
    /// Cut curve shape.
    pub cut_shape: CutShape,
}

impl Default for LimiterConfig {
    fn default() -> Self {
        Self {
            threshold_rpm_ratio: 0.995,
            attack_ms: 2.0,
            release_ms: 60.0,
            cut_shape: CutShape::Soft,
        }
    }
}

/// Traction control suppression configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TcConfig {
    /// Suppression gain applied during traction loss [0.0, 1.0].
    pub suppress_gain: f32,
    /// Suppression attack in ms [0.0, 200.0].
    pub attack_ms: f32,
    /// Suppression release in ms [1.0, 1000.0].
    pub release_ms: f32,
    /// Minimum cut threshold [0.0, 1.0].
    pub min_cut_threshold: f32,
}

impl Default for TcConfig {
    fn default() -> Self {
        Self {
            suppress_gain: 0.55,
            attack_ms: 8.0,
            release_ms: 120.0,
            min_cut_threshold: 0.02,
        }
    }
}

/// DSP quality profile per distance level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct QualityProfile {
    /// Near-field resonator scaling.
    pub near_resonator_scale: f32,
    /// Mid-field resonator scaling.
    pub mid_resonator_scale: f32,
    /// Far-field resonator scaling.
    pub far_resonator_scale: f32,
    /// Coefficient update steps near.
    pub coeff_update_steps: u32,
    /// Coefficient update steps far.
    pub far_coeff_update_steps: u32,
    /// Organic jitter scaling.
    pub organic_jitter_scale: f32,
}

impl Default for QualityProfile {
    fn default() -> Self {
        Self {
            near_resonator_scale: 1.0,
            mid_resonator_scale: 0.6,
            far_resonator_scale: 0.35,
            coeff_update_steps: 1,
            far_coeff_update_steps: 4,
            organic_jitter_scale: 1.0,
        }
    }
}

/// Distance-based level configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DistanceLevels {
    /// Near field max distance in m.
    pub near_max_m: f32,
    /// Mid field max distance in m.
    pub mid_max_m: f32,
    /// Far field max distance in m.
    pub far_max_m: f32,
    /// Hysteresis between level transitions (0.0, 0.5].
    pub hysteresis_ratio: f32,
    /// DSP quality profile used per level.
    pub dsp: QualityProfile,
}

impl Default for DistanceLevels {
    fn default() -> Self {
        Self {
            near_max_m: 25.0,
            mid_max_m: 80.0,
            far_max_m: 200.0,
            hysteresis_ratio: 0.10,
            dsp: QualityProfile::default(),
        }
    }
}

/// CPU budget (percent of a full frame) per distance level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CpuBudget {
    /// Near-field CPU percent [0.5, 20.0].
    pub near_percent: f32,
    /// Far-field CPU percent [0.2, 10.0].
    pub far_percent: f32,
}

impl Default for CpuBudget {
    fn default() -> Self {
        Self {
            near_percent: 5.0,
            far_percent: 2.0,
        }
    }
}

/// Root of the `audio.powertrain_synthesis` contract (schema version 1).
///
/// Deserialization fills every missing field from [`Default`], so a minimal
/// section such as `{"enabled": true}` yields the generic V10 defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioPowertrainSynthesis {
    /// Contract schema version (currently 1).
    pub schema_version: u32,
    /// Master enable for powertrain synthesis.
    pub enabled: bool,
    /// Synthesis model family.
    pub synthesis_model: SynthesisModel,
    /// Combustion cycle.
    pub cycle: Cycle,
    /// Aspiration type.
    pub aspiration: Aspiration,
    /// Total cylinder count.
    pub total_cylinders: u32,
    /// Number of cylinders actually synthesized (rest derived).
    pub physically_simulated_cylinders: u32,
    /// Number of cylinder banks.
    pub bank_count: u32,
    /// Firing order (one cylinder number per firing event), if specified.
    pub firing_order: Option<Vec<u8>>,
    /// Firing phase offset in degrees for each simulated cylinder, if specified.
    pub firing_phases_deg: Option<Vec<f32>>,
    /// Whether firing is evenly spaced.
    pub even_firing: bool,
    /// Combustion irregularity behaviour.
    pub combustion: CombustionConfig,
    /// Energy response tuning.
    pub energy: EnergyConfig,
    /// Intake layer tuning.
    pub intake: IntakeConfig,
    /// Exhaust layer tuning.
    pub exhaust: ExhaustConfig,
    /// Half-block derivation tuning.
    pub half_block: HalfBlockConfig,
    /// RPM limiter tuning.
    pub limiter: LimiterConfig,
    /// Traction control tuning.
    pub tc: TcConfig,
    /// Distance level tuning.
    pub distance_levels: DistanceLevels,
    /// CPU budget tuning.
    pub cpu_budget: CpuBudget,
}

impl Default for AudioPowertrainSynthesis {
    fn default() -> Self {
        Self {
            schema_version: 1,
            enabled: true,
            synthesis_model: SynthesisModel::HalfBlock5FourStroke,
            cycle: Cycle::FourStroke,
            aspiration: Aspiration::NaturallyAspirated,
            total_cylinders: 10,
            physically_simulated_cylinders: 5,
            bank_count: 2,
            firing_order: None,
            firing_phases_deg: None,
            even_firing: true,
            combustion: CombustionConfig::default(),
            energy: EnergyConfig::default(),
            intake: IntakeConfig::default(),
            exhaust: ExhaustConfig::default(),
            half_block: HalfBlockConfig::default(),
            limiter: LimiterConfig::default(),
            tc: TcConfig::default(),
            distance_levels: DistanceLevels::default(),
            cpu_budget: CpuBudget::default(),
        }
    }
}

/// Raw deserialization target for the profile section. The contract type is
/// used directly; the alias documents the parse-then-validate step.
type PowertrainSynthesisRaw = AudioPowertrainSynthesis;

/// Parse the `audio.powertrain_synthesis` section from a physics profile JSON.
///
/// Returns `Ok(None)` when the section is absent (or explicitly `null`) so the
/// caller can fall back to [`Default`]. Invalid JSON or structural type errors
/// are reported as `Err`. Physical coherence is validated afterwards: any
/// diagnostic yields a single `Err` aggregating `path: message` lines.
pub fn from_profile_json(json: &str) -> Result<Option<AudioPowertrainSynthesis>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| format!("invalid JSON: {error}"))?;
    let Some(section) = value
        .get("audio")
        .and_then(|audio| audio.get("powertrain_synthesis"))
        .filter(|section| !section.is_null())
    else {
        return Ok(None);
    };
    let raw: PowertrainSynthesisRaw = serde_json::from_value(section.clone())
        .map_err(|error| format!("{PROFILE_SECTION}: {error}"))?;
    let diagnostics = raw.validate();
    if diagnostics.is_empty() {
        return Ok(Some(raw));
    }
    let messages: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.path, diagnostic.message))
        .collect();
    Err(messages.join("\n"))
}

/// Returns true for finite floats.
fn finite(v: f32) -> bool {
    v.is_finite()
}

/// Pushes a non-finite diagnostic and returns false when `v` is NaN/infinite.
fn check_finite(diags: &mut Vec<ConfigDiagnostic>, path: String, v: f32) -> bool {
    if finite(v) {
        return true;
    }
    diags.push(ConfigDiagnostic {
        path,
        message: format!("value is not finite: {v}"),
    });
    false
}

/// Emits a diagnostic when `v` is not finite or outside the closed range.
fn check_range_closed(diags: &mut Vec<ConfigDiagnostic>, path: String, v: f32, min: f32, max: f32) {
    if !check_finite(diags, path.clone(), v) {
        return;
    }
    if !(min..=max).contains(&v) {
        diags.push(ConfigDiagnostic {
            path,
            message: format!("value {v} must be in [{min}, {max}]"),
        });
    }
}

/// Emits a diagnostic when `v` is not finite or outside the open-lower range.
fn check_range_open_low(
    diags: &mut Vec<ConfigDiagnostic>,
    path: String,
    v: f32,
    min: f32,
    max: f32,
) {
    if !check_finite(diags, path.clone(), v) {
        return;
    }
    if !(v > min && v <= max) {
        diags.push(ConfigDiagnostic {
            path,
            message: format!("value {v} must be in ({min}, {max}]"),
        });
    }
}

/// Validates physical coherence of resonances (intake/exhaust rules).
fn check_resonances(diags: &mut Vec<ConfigDiagnostic>, prefix: &str, resonances: &[Resonance]) {
    for (index, resonance) in resonances.iter().enumerate() {
        let path = format!("{prefix}[{index}]");
        check_range_closed(
            diags,
            format!("{path}.frequency_hz"),
            resonance.frequency_hz,
            20.0,
            20000.0,
        );
        check_range_closed(diags, format!("{path}.q"), resonance.q, 0.1, 30.0);
        check_range_closed(
            diags,
            format!("{path}.gain_db"),
            resonance.gain_db,
            -18.0,
            18.0,
        );
    }
}

impl AudioPowertrainSynthesis {
    /// Validates physical coherence of the contract. Returns one diagnostic
    /// per offending field; an empty vector means the contract is usable.
    pub fn validate(&self) -> Vec<ConfigDiagnostic> {
        let mut diags = Vec::new();
        let path = |field: &str| format!("{PROFILE_SECTION}.{field}");

        if self.schema_version != 1 {
            diags.push(ConfigDiagnostic {
                path: path("schema_version"),
                message: format!(
                    "unsupported schema_version {}; contract version 1 is required",
                    self.schema_version
                ),
            });
        }

        if self.total_cylinders != 10 {
            diags.push(ConfigDiagnostic {
                path: path("total_cylinders"),
                message: "must be 10 for half_block_5_four_stroke model".into(),
            });
        }
        if self.physically_simulated_cylinders != 5 {
            diags.push(ConfigDiagnostic {
                path: path("physically_simulated_cylinders"),
                message: "must be 5 for half_block_5_four_stroke model".into(),
            });
        }
        if self.bank_count != 2 {
            diags.push(ConfigDiagnostic {
                path: path("bank_count"),
                message: "must be 2 for half_block_5_four_stroke model".into(),
            });
        }

        if let Some(order) = &self.firing_order {
            if order.len() != self.total_cylinders as usize {
                diags.push(ConfigDiagnostic {
                    path: path("firing_order"),
                    message: format!(
                        "must list one entry per cylinder; total_cylinders is {} but got {}",
                        self.total_cylinders,
                        order.len()
                    ),
                });
            }
            let mut seen = HashSet::new();
            for (index, &cylinder) in order.iter().enumerate() {
                if cylinder == 0 || cylinder as u32 > self.total_cylinders {
                    diags.push(ConfigDiagnostic {
                        path: format!("{PROFILE_SECTION}.firing_order[{index}]"),
                        message: format!(
                            "cylinder {cylinder} outside 1..={}",
                            self.total_cylinders
                        ),
                    });
                }
                if !seen.insert(cylinder) {
                    diags.push(ConfigDiagnostic {
                        path: format!("{PROFILE_SECTION}.firing_order[{index}]"),
                        message: format!("duplicate cylinder {cylinder} in firing_order"),
                    });
                }
            }
        }

        if let Some(phases) = &self.firing_phases_deg {
            if phases.len() != self.physically_simulated_cylinders as usize {
                diags.push(ConfigDiagnostic {
                    path: path("firing_phases_deg"),
                    message: format!(
                        "must list one phase per simulated cylinder; \
                         physically_simulated_cylinders is {} but got {}",
                        self.physically_simulated_cylinders,
                        phases.len()
                    ),
                });
            }
            let mut previous: Option<f32> = None;
            for (index, &phase) in phases.iter().enumerate() {
                let phase_path = format!("{PROFILE_SECTION}.firing_phases_deg[{index}]");
                if !check_finite(&mut diags, phase_path.clone(), phase) {
                    previous = Some(phase);
                    continue;
                }
                if !(0.0..720.0).contains(&phase) {
                    diags.push(ConfigDiagnostic {
                        path: phase_path.clone(),
                        message: format!("phase {phase} is outside [0.0, 720.0)"),
                    });
                }
                if let Some(prev_phase) = previous {
                    if phase <= prev_phase {
                        diags.push(ConfigDiagnostic {
                            path: phase_path.clone(),
                            message: format!(
                                "firing phases must be strictly ascending; {phase} <= {prev_phase}"
                            ),
                        });
                    }
                }
                previous = Some(phase);
            }
        }

        check_range_closed(
            &mut diags,
            path("combustion.irregularity"),
            self.combustion.irregularity,
            0.0,
            0.25,
        );

        check_range_closed(
            &mut diags,
            path("energy.torque_curve_weight"),
            self.energy.torque_curve_weight,
            0.0,
            1.0,
        );
        check_range_closed(
            &mut diags,
            path("energy.throttle_response"),
            self.energy.throttle_response,
            0.1,
            2.0,
        );
        check_range_closed(
            &mut diags,
            path("energy.idle_combustion_gain"),
            self.energy.idle_combustion_gain,
            0.0,
            0.5,
        );
        check_range_open_low(
            &mut diags,
            path("energy.attack_smoothing_s"),
            self.energy.attack_smoothing_s,
            0.0,
            0.5,
        );
        check_range_open_low(
            &mut diags,
            path("energy.release_smoothing_s"),
            self.energy.release_smoothing_s,
            0.0,
            2.0,
        );
        check_range_open_low(
            &mut diags,
            path("energy.load_smoothing_s"),
            self.energy.load_smoothing_s,
            0.0,
            0.5,
        );

        check_range_closed(
            &mut diags,
            path("intake.noise_gain"),
            self.intake.noise_gain,
            0.0,
            1.0,
        );
        check_range_closed(
            &mut diags,
            path("intake.throttle_follow"),
            self.intake.throttle_follow,
            0.0,
            1.0,
        );
        check_resonances(
            &mut diags,
            "audio.powertrain_synthesis.intake.resonances",
            &self.intake.resonances,
        );

        check_range_closed(
            &mut diags,
            path("exhaust.pulse_gain"),
            self.exhaust.pulse_gain,
            0.0,
            2.0,
        );
        check_range_closed(
            &mut diags,
            path("exhaust.saturation"),
            self.exhaust.saturation,
            0.0,
            1.0,
        );
        check_range_closed(
            &mut diags,
            path("exhaust.damping"),
            self.exhaust.damping,
            0.0,
            0.99,
        );
        check_resonances(
            &mut diags,
            "audio.powertrain_synthesis.exhaust.collector_resonances",
            &self.exhaust.collector_resonances,
        );

        check_range_closed(
            &mut diags,
            path("half_block.phase_offset_deg"),
            self.half_block.phase_offset_deg,
            0.0,
            720.0,
        );
        check_range_closed(
            &mut diags,
            path("half_block.delay_s"),
            self.half_block.delay_s,
            0.0,
            0.05,
        );
        check_range_closed(
            &mut diags,
            path("half_block.decorrelation"),
            self.half_block.decorrelation,
            0.0,
            1.0,
        );
        check_range_closed(
            &mut diags,
            path("half_block.gain"),
            self.half_block.gain,
            0.0,
            2.0,
        );
        check_range_closed(
            &mut diags,
            path("half_block.timbre_diff"),
            self.half_block.timbre_diff,
            0.0,
            1.0,
        );

        check_range_closed(
            &mut diags,
            path("limiter.threshold_rpm_ratio"),
            self.limiter.threshold_rpm_ratio,
            0.90,
            1.0,
        );
        check_range_closed(
            &mut diags,
            path("limiter.attack_ms"),
            self.limiter.attack_ms,
            0.0,
            100.0,
        );
        check_range_closed(
            &mut diags,
            path("limiter.release_ms"),
            self.limiter.release_ms,
            1.0,
            1000.0,
        );

        check_range_closed(
            &mut diags,
            path("tc.suppress_gain"),
            self.tc.suppress_gain,
            0.0,
            1.0,
        );
        check_range_closed(
            &mut diags,
            path("tc.attack_ms"),
            self.tc.attack_ms,
            0.0,
            200.0,
        );
        check_range_closed(
            &mut diags,
            path("tc.release_ms"),
            self.tc.release_ms,
            1.0,
            1000.0,
        );
        check_range_closed(
            &mut diags,
            path("tc.min_cut_threshold"),
            self.tc.min_cut_threshold,
            0.0,
            1.0,
        );

        let near = self.distance_levels.near_max_m;
        let mid = self.distance_levels.mid_max_m;
        let far = self.distance_levels.far_max_m;
        let near_finite = check_finite(&mut diags, path("distance_levels.near_max_m"), near);
        let mid_finite = check_finite(&mut diags, path("distance_levels.mid_max_m"), mid);
        let far_finite = check_finite(&mut diags, path("distance_levels.far_max_m"), far);
        if near_finite && !(near > 0.0) {
            diags.push(ConfigDiagnostic {
                path: path("distance_levels.near_max_m"),
                message: "must be greater than 0.0".into(),
            });
        }
        if near_finite && mid_finite && !(near < mid) {
            diags.push(ConfigDiagnostic {
                path: path("distance_levels.near_max_m"),
                message: "must be strictly less than mid_max_m".into(),
            });
        }
        if near_finite && mid_finite && !(mid > near) {
            diags.push(ConfigDiagnostic {
                path: path("distance_levels.mid_max_m"),
                message: "must be strictly greater than near_max_m".into(),
            });
        }
        if mid_finite && far_finite && !(mid < far) {
            diags.push(ConfigDiagnostic {
                path: path("distance_levels.mid_max_m"),
                message: "must be strictly less than far_max_m".into(),
            });
        }
        if mid_finite && far_finite && !(far > mid) {
            diags.push(ConfigDiagnostic {
                path: path("distance_levels.far_max_m"),
                message: "must be strictly greater than mid_max_m".into(),
            });
        }
        if far_finite && !(far <= 1000.0) {
            diags.push(ConfigDiagnostic {
                path: path("distance_levels.far_max_m"),
                message: "must be less than or equal to 1000.0".into(),
            });
        }
        check_range_open_low(
            &mut diags,
            path("distance_levels.hysteresis_ratio"),
            self.distance_levels.hysteresis_ratio,
            0.0,
            0.5,
        );

        check_range_closed(
            &mut diags,
            path("cpu_budget.near_percent"),
            self.cpu_budget.near_percent,
            0.5,
            20.0,
        );
        check_range_closed(
            &mut diags,
            path("cpu_budget.far_percent"),
            self.cpu_budget.far_percent,
            0.2,
            10.0,
        );
        if self.cpu_budget.far_percent > self.cpu_budget.near_percent {
            diags.push(ConfigDiagnostic {
                path: path("cpu_budget.far_percent"),
                message: "must be less than or equal to near_percent".into(),
            });
        }

        diags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(body: &str) -> String {
        format!(r#"{{"audio": {{"powertrain_synthesis": {body}}}}}"#)
    }

    #[test]
    fn defaults_are_generic_uniform_v10_four_stroke() {
        let defaults = AudioPowertrainSynthesis::default();
        assert!(defaults.enabled);
        assert_eq!(defaults.schema_version, 1);
        assert_eq!(
            defaults.synthesis_model,
            SynthesisModel::HalfBlock5FourStroke
        );
        assert_eq!(defaults.cycle, Cycle::FourStroke);
        assert_eq!(defaults.aspiration, Aspiration::NaturallyAspirated);
        assert_eq!(defaults.total_cylinders, 10);
        assert_eq!(defaults.physically_simulated_cylinders, 5);
        assert_eq!(defaults.bank_count, 2);
        assert!(defaults.even_firing);
        assert_eq!(defaults.firing_order, None);
        assert_eq!(defaults.firing_phases_deg, None);
        assert_eq!(defaults.half_block.phase_offset_deg, 72.0);
        assert_eq!(defaults.cpu_budget.near_percent, 5.0);
        assert_eq!(defaults.cpu_budget.far_percent, 2.0);
        assert!(defaults.validate().is_empty());
    }

    #[test]
    fn full_profile_section_roundtrips_valid() {
        let json = r#"{
            "audio": {
                "powertrain_synthesis": {
                    "schema_version": 1,
                    "enabled": true,
                    "synthesis_model": "half_block_5_four_stroke",
                    "cycle": "four_stroke",
                    "aspiration": "naturally_aspirated",
                    "total_cylinders": 10,
                    "physically_simulated_cylinders": 5,
                    "bank_count": 2,
                    "firing_order": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                    "firing_phases_deg": [0.0, 144.0, 288.0, 432.0, 576.0],
                    "even_firing": true,
                    "combustion": {"irregularity": 0.02, "seed": 42},
                    "energy": {"torque_curve_weight": 0.9, "throttle_response": 1.2, "idle_combustion_gain": 0.08, "attack_smoothing_s": 0.01, "release_smoothing_s": 0.1, "load_smoothing_s": 0.05},
                    "intake": {"enabled": true, "noise_gain": 0.4, "throttle_follow": 0.85, "resonances": [{"frequency_hz": 320.0, "q": 2.0, "gain_db": 3.0}]},
                    "exhaust": {"enabled": true, "pulse_gain": 1.1, "saturation": 0.3, "damping": 0.5, "collector_resonances": [{"frequency_hz": 190.0, "q": 4.0, "gain_db": 4.0}, {"frequency_hz": 380.0, "q": 2.5, "gain_db": 1.0}]},
                    "half_block": {"phase_offset_deg": 72.0, "delay_s": 0.004, "decorrelation": 0.35, "gain": 0.98, "timbre_diff": 0.06},
                    "limiter": {"threshold_rpm_ratio": 0.995, "attack_ms": 2.0, "release_ms": 60.0, "cut_shape": "soft"},
                    "tc": {"suppress_gain": 0.55, "attack_ms": 8.0, "release_ms": 120.0, "min_cut_threshold": 0.02},
                    "distance_levels": {"near_max_m": 25.0, "mid_max_m": 80.0, "far_max_m": 200.0, "hysteresis_ratio": 0.1, "dsp": {"near_resonator_scale": 1.0, "mid_resonator_scale": 0.6, "far_resonator_scale": 0.35, "coeff_update_steps": 1, "far_coeff_update_steps": 4, "organic_jitter_scale": 1.0}},
                    "cpu_budget": {"near_percent": 5.0, "far_percent": 2.0}
                }
            }
        }"#;
        let parsed = from_profile_json(json).expect("valid profile JSON");
        let config = parsed.expect("section present");
        assert_eq!(config.total_cylinders, 10);
        assert!(config.validate().is_empty());
        let roundtrip: AudioPowertrainSynthesis =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(config, roundtrip);
    }

    #[test]
    fn missing_section_returns_none() {
        assert!(from_profile_json("{}").unwrap().is_none());
        assert!(from_profile_json(r#"{"audio": {}}"#).unwrap().is_none());
    }

    #[test]
    fn invalid_json_is_err() {
        assert!(from_profile_json("not json").is_err());
    }

    #[test]
    fn invalid_model_shape_reports_diagnostics() {
        let mut config = AudioPowertrainSynthesis::default();
        config.total_cylinders = 12;
        config.physically_simulated_cylinders = 4;
        let diagnostics = config.validate();
        assert!(diagnostics
            .iter()
            .any(|d| d.path == "audio.powertrain_synthesis.total_cylinders"));
        assert!(diagnostics
            .iter()
            .any(|d| d.path == "audio.powertrain_synthesis.physically_simulated_cylinders"));
    }

    #[test]
    fn non_finite_values_rejected() {
        let mut config = AudioPowertrainSynthesis::default();
        config.energy.throttle_response = f32::NAN;
        assert!(config
            .validate()
            .iter()
            .any(|d| d.path == "audio.powertrain_synthesis.energy.throttle_response"));

        let mut config = AudioPowertrainSynthesis::default();
        config.exhaust.pulse_gain = f32::INFINITY;
        assert!(config
            .validate()
            .iter()
            .any(|d| d.path == "audio.powertrain_synthesis.exhaust.pulse_gain"));
    }

    #[test]
    fn nonzero_float_ranges_enforced() {
        let mut config = AudioPowertrainSynthesis::default();
        config.intake.resonances[0].q = 0.0;
        config.intake.resonances[0].gain_db = 100.0;
        config.limiter.threshold_rpm_ratio = 0.5;
        config.distance_levels.hysteresis_ratio = 0.0;
        let paths: Vec<String> = config.validate().iter().map(|d| d.path.clone()).collect();
        assert!(paths.contains(&"audio.powertrain_synthesis.intake.resonances[0].q".into()));
        assert!(paths.contains(&"audio.powertrain_synthesis.intake.resonances[0].gain_db".into()));
        assert!(paths.contains(&"audio.powertrain_synthesis.limiter.threshold_rpm_ratio".into()));
        assert!(
            paths.contains(&"audio.powertrain_synthesis.distance_levels.hysteresis_ratio".into())
        );
    }

    #[test]
    fn firing_phases_len_must_match_cylinders() {
        let mut config = AudioPowertrainSynthesis::default();
        config.firing_phases_deg = Some(vec![0.0, 144.0, 288.0, 432.0, 576.0, 700.0]);
        assert!(config
            .validate()
            .iter()
            .any(|d| d.path == "audio.powertrain_synthesis.firing_phases_deg"));
    }

    #[test]
    fn firing_order_must_fit_total_cylinders() {
        let mut config = AudioPowertrainSynthesis::default();
        config.firing_order = Some(vec![1, 2, 3, 4, 5, 6]);
        assert!(config
            .validate()
            .iter()
            .any(|d| d.path == "audio.powertrain_synthesis.firing_order"));

        let mut config = AudioPowertrainSynthesis::default();
        config.firing_order = Some(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 1]);
        assert!(config
            .validate()
            .iter()
            .any(|d| d.message.contains("duplicate cylinder")));
    }

    #[test]
    fn minimal_enabled_section_uses_defaults() {
        let parsed = from_profile_json(&section(r#"{"enabled": true}"#)).unwrap();
        let config = parsed.expect("section present");
        assert!(config.enabled);
        assert_eq!(config.total_cylinders, 10);
        assert_eq!(config.physically_simulated_cylinders, 5);
        assert_eq!(config.bank_count, 2);
        assert!(config.validate().is_empty());
    }

    #[test]
    fn distance_levels_must_ascend() {
        let mut config = AudioPowertrainSynthesis::default();
        config.distance_levels.near_max_m = 80.0;
        config.distance_levels.mid_max_m = 25.0;
        let diagnostics = config.validate();
        assert!(diagnostics
            .iter()
            .any(|d| d.path == "audio.powertrain_synthesis.distance_levels.near_max_m"));
        assert!(diagnostics
            .iter()
            .any(|d| d.path == "audio.powertrain_synthesis.distance_levels.mid_max_m"));
    }

    #[test]
    fn from_profile_json_reports_aggregated_errors() {
        let json = section(r#"{"total_cylinders": 12, "energy": {"throttle_response": 1e39}}"#);
        let error = from_profile_json(&json).expect_err("expected validation error");
        assert!(error.contains("audio.powertrain_synthesis.total_cylinders"));
        assert!(error.contains("audio.powertrain_synthesis.energy.throttle_response"));
    }
}
