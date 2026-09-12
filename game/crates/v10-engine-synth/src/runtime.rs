//! Allocation-free block facade for embedding the GF509 continuous engine.
//!
//! Asset loading and reset are control-thread operations. `render_block` owns
//! no temporary buffers and performs no file I/O or logging.

use std::path::PathBuf;
use std::{fs, path::Path};

use crate::{
    AcousticScene, AcousticSceneConfig, EngineConfig, EngineInput, SampleLayerFrame, SampleLayerInput,
    ThreeZoneSampleLayer, ThreeZoneSampleLayerConfig, V10Engine,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const GF509_HEADROOM_GAIN: f32 = 0.61;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TorqueSign {
    Negative = -1,
    #[default]
    Neutral = 0,
    Positive = 1,
}

impl TorqueSign {
    pub fn from_i32(v: i32) -> Result<Self, String> {
        match v {
            -1 => Ok(Self::Negative),
            0 => Ok(Self::Neutral),
            1 => Ok(Self::Positive),
            _ => Err(format!("invalid torque sign: {}", v)),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShiftPhase {
    #[default]
    None = 0,
    UpshiftCut = 1,
    UpshiftRecovery = 2,
    DownshiftCut = 3,
    DownshiftBlip = 4,
    DownshiftRecovery = 5,
}

impl ShiftPhase {
    pub fn from_i32(v: i32) -> Result<Self, String> {
        match v {
            0 => Ok(Self::None),
            1 => Ok(Self::UpshiftCut),
            2 => Ok(Self::UpshiftRecovery),
            3 => Ok(Self::DownshiftCut),
            4 => Ok(Self::DownshiftBlip),
            5 => Ok(Self::DownshiftRecovery),
            _ => Err(format!("invalid shift phase: {}", v)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeTelemetry {
    pub rpm: f32,
    pub throttle: f32,
    pub normalized_engine_load: f32,
    pub normalized_engine_torque: f32,
    pub torque_sign: TorqueSign,
    pub rpm_derivative: f32,
    pub throttle_derivative: f32,
    pub gear: i8,
    pub shift_phase: ShiftPhase,
    /// Clutch engagement from the physics powertrain, continuously smoothed.
    pub clutch_engagement: f32,
    /// Physical traction-control cut. The load packet already includes this
    /// cut; the synth consumes it only for a dedicated, non-gain modulation.
    pub tc_cut_ratio: f32,
    /// Discrete physical rev limiter state; changed sample-accurately and never
    /// interpolated with a continuous control.
    pub rev_limiter_active: bool,
    pub dt_seconds: f32,
}

impl RuntimeTelemetry {
    pub fn validate(self) -> Result<Self, String> {
        if !self.rpm.is_finite() || !(0.0..=25_000.0).contains(&self.rpm) {
            return Err(format!("rpm out of range: {}", self.rpm));
        }
        if !self.throttle.is_finite() || !(0.0..=1.0).contains(&self.throttle) {
            return Err(format!("throttle out of range: {}", self.throttle));
        }
        if !self.normalized_engine_load.is_finite()
            || !(0.0..=1.0).contains(&self.normalized_engine_load)
        {
            return Err(format!(
                "normalized_engine_load out of range: {}",
                self.normalized_engine_load
            ));
        }
        if !self.normalized_engine_torque.is_finite()
            || !(-1.0..=1.0).contains(&self.normalized_engine_torque)
        {
            return Err(format!(
                "normalized_engine_torque out of range: {}",
                self.normalized_engine_torque
            ));
        }
        if !self.clutch_engagement.is_finite()
            || !(0.0..=1.0).contains(&self.clutch_engagement)
        {
            return Err(format!("clutch_engagement out of range: {}", self.clutch_engagement));
        }
        if !self.tc_cut_ratio.is_finite() || !(0.0..=1.0).contains(&self.tc_cut_ratio) {
            return Err(format!("tc_cut_ratio out of range: {}", self.tc_cut_ratio));
        }
        if !self.rpm_derivative.is_finite() {
            return Err(format!("rpm_derivative not finite: {}", self.rpm_derivative));
        }
        if !self.throttle_derivative.is_finite() {
            return Err(format!(
                "throttle_derivative not finite: {}",
                self.throttle_derivative
            ));
        }
        if !(-1..=12).contains(&self.gear) {
            return Err(format!("gear out of range: {}", self.gear));
        }
        if !self.dt_seconds.is_finite() || !(0.0..=1.0).contains(&self.dt_seconds) {
            return Err(format!("dt_seconds out of range: {}", self.dt_seconds));
        }
        Ok(self)
    }
}

impl Default for RuntimeTelemetry {
    fn default() -> Self {
        Self {
            rpm: 0.0,
            throttle: 0.0,
            normalized_engine_load: 0.0,
            normalized_engine_torque: 0.0,
            torque_sign: TorqueSign::Neutral,
            rpm_derivative: 0.0,
            throttle_derivative: 0.0,
            gear: 0,
            shift_phase: ShiftPhase::None,
            clutch_engagement: 0.0,
            tc_cut_ratio: 0.0,
            rev_limiter_active: false,
            dt_seconds: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Gf509RuntimeConfig {
    pub engine: EngineConfig,
    pub scene: AcousticSceneConfig,
    pub sample_layer: ThreeZoneSampleLayerConfig,
    /// Prepared sample directory. `None` is the procedural-only INT-01 mode.
    pub sample_layer_directory: Option<PathBuf>,
    pub max_block_frames: usize,
}

impl Default for Gf509RuntimeConfig {
    fn default() -> Self {
        Self {
            engine: EngineConfig::default(),
            scene: AcousticSceneConfig::default(),
            sample_layer: ThreeZoneSampleLayerConfig::default(),
            sample_layer_directory: None,
            max_block_frames: 4096,
        }
    }
}

pub struct Gf509Runtime {
    config: Gf509RuntimeConfig,
    engine: V10Engine,
    scene: AcousticScene,
    sample_layer: Option<ThreeZoneSampleLayer>,
    /// Reused per-sample sample-layer output. Keeps the audio callback
    /// allocation-free by avoiding a fresh `SampleLayerFrame` per sample.
    sample_frame: SampleLayerFrame,
    telemetry: RuntimeTelemetry,
    rendered_telemetry: RuntimeTelemetry,
    interpolation_remaining: usize,
    /// AUD-05: linear interpolation steps for the 8 continuous controls
    /// [rpm, throttle, load, torque, rpm_dot, throttle_dot, clutch, tc].
    /// Computed once per `update_telemetry` (control rate) instead of dividing
    /// by the decreasing remainder on every sample. The exact remainder
    /// division is mathematically the same constant step; precomputing it
    /// removes 8 divisions per sample with only float-rounding differences.
    interp_steps: [f32; 8],
}

impl Gf509Runtime {
    pub fn new(config: Gf509RuntimeConfig) -> Result<Self, String> {
        if config.max_block_frames == 0 {
            return Err("max_block_frames must be greater than zero".into());
        }
        let sample_rate = config.engine.sample_rate;
        let engine = V10Engine::new(config.engine.clone())?;
        let scene = AcousticScene::new(sample_rate as f32, config.scene)?;
        if let Some(directory) = config.sample_layer_directory.as_deref() {
            validate_asset_manifest(directory, sample_rate)?;
        }
        let sample_layer = config
            .sample_layer_directory
            .as_deref()
            .map(|path| {
                ThreeZoneSampleLayer::load_directory(sample_rate, path, config.sample_layer)
            })
            .transpose()?;
        Ok(Self {
            config,
            engine,
            scene,
            sample_layer,
            sample_frame: SampleLayerFrame::default(),
            telemetry: RuntimeTelemetry::default(),
            rendered_telemetry: RuntimeTelemetry::default(),
            interpolation_remaining: 0,
            interp_steps: [0.0; 8],
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.engine.sample_rate
    }
    pub fn max_block_frames(&self) -> usize {
        self.config.max_block_frames
    }
    pub fn telemetry(&self) -> RuntimeTelemetry {
        self.telemetry
    }
    pub fn rendered_telemetry(&self) -> RuntimeTelemetry {
        self.rendered_telemetry
    }

    pub fn update_telemetry(&mut self, telemetry: RuntimeTelemetry) -> Result<(), String> {
        self.telemetry = telemetry.validate()?;
        let remaining =
            (telemetry.dt_seconds * self.sample_rate() as f32).round() as usize;
        self.interpolation_remaining = remaining;
        // AUD-05 control-rate step: constant linear increment from the current
        // rendered state to the new target. Applied per sample by addition.
        // Discrete states (sign/gear/phase/limiter) still switch immediately
        // in render_block. Zero-remaining (legacy dt=0) keeps snap behavior.
        if remaining > 0 {
            let from = self.rendered_telemetry;
            let inv = 1.0 / remaining as f32;
            self.interp_steps = [
                (telemetry.rpm - from.rpm) * inv,
                (telemetry.throttle - from.throttle) * inv,
                (telemetry.normalized_engine_load - from.normalized_engine_load) * inv,
                (telemetry.normalized_engine_torque - from.normalized_engine_torque) * inv,
                (telemetry.rpm_derivative - from.rpm_derivative) * inv,
                (telemetry.throttle_derivative - from.throttle_derivative) * inv,
                (telemetry.clutch_engagement - from.clutch_engagement) * inv,
                (telemetry.tc_cut_ratio - from.tc_cut_ratio) * inv,
            ];
        } else {
            self.interp_steps = [0.0; 8];
        }
        Ok(())
    }

    /// Renders equal-length planar stereo buffers and returns their frame count.
    /// GF509 is currently a centered continuous source, so both channels match.
    pub fn render_block(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<usize, String> {
        if left.len() != right.len() {
            return Err("left and right buffers must have equal lengths".into());
        }
        if left.len() > self.config.max_block_frames {
            return Err(format!(
                "block has {} frames; configured maximum is {}",
                left.len(),
                self.config.max_block_frames
            ));
        }
        let target = self.telemetry;
        for (left_sample, right_sample) in left.iter_mut().zip(right.iter_mut()) {
            if self.interpolation_remaining > 0 {
                // AUD-05: precomputed linear steps (control rate); no per-sample
                // division. Snap exactly to target on the final step to avoid
                // float accumulation drift.
                let steps = self.interp_steps;
                self.rendered_telemetry.rpm += steps[0];
                self.rendered_telemetry.throttle += steps[1];
                self.rendered_telemetry.normalized_engine_load += steps[2];
                self.rendered_telemetry.normalized_engine_torque += steps[3];
                self.rendered_telemetry.rpm_derivative += steps[4];
                self.rendered_telemetry.throttle_derivative += steps[5];
                self.rendered_telemetry.clutch_engagement += steps[6];
                self.rendered_telemetry.tc_cut_ratio += steps[7];
                self.interpolation_remaining -= 1;
                if self.interpolation_remaining == 0 {
                    self.rendered_telemetry.rpm = target.rpm;
                    self.rendered_telemetry.throttle = target.throttle;
                    self.rendered_telemetry.normalized_engine_load =
                        target.normalized_engine_load;
                    self.rendered_telemetry.normalized_engine_torque =
                        target.normalized_engine_torque;
                    self.rendered_telemetry.rpm_derivative = target.rpm_derivative;
                    self.rendered_telemetry.throttle_derivative =
                        target.throttle_derivative;
                    self.rendered_telemetry.clutch_engagement = target.clutch_engagement;
                    self.rendered_telemetry.tc_cut_ratio = target.tc_cut_ratio;
                }
            } else {
                self.rendered_telemetry = target;
            }
            self.rendered_telemetry.torque_sign = target.torque_sign;
            self.rendered_telemetry.gear = target.gear;
            self.rendered_telemetry.shift_phase = target.shift_phase;
            self.rendered_telemetry.rev_limiter_active = target.rev_limiter_active;
            self.rendered_telemetry.dt_seconds = target.dt_seconds;
            let interpolated = self.rendered_telemetry;
            self.engine.set_input(EngineInput {
                rpm: interpolated.rpm,
                throttle: interpolated.throttle,
                load: interpolated.normalized_engine_load,
            })?;
            self.engine.set_mechanical_state(
                interpolated.normalized_engine_torque,
                interpolated.clutch_engagement,
                interpolated.tc_cut_ratio,
                interpolated.rev_limiter_active,
                interpolated.shift_phase,
            );
            let engine_frame = self.engine.render_sample();
            let scene_frame = self.scene.process(&engine_frame);
            let sample_frame = if let Some(layer) = self.sample_layer.as_mut() {
                layer.process_into(
                    SampleLayerInput {
                        rpm: interpolated.rpm,
                        throttle: interpolated.throttle,
                        load: interpolated.normalized_engine_load,
                        normalized_engine_torque: interpolated.normalized_engine_torque,
                        clutch_engagement: interpolated.clutch_engagement,
                        crank_phase_deg: engine_frame.crank_phase_deg,
                    },
                    &mut self.sample_frame,
                )?;
                Some(self.sample_frame.output)
            } else {
                None
            };
            let output = if let Some(sample_frame) = sample_frame {
                (scene_frame.output * self.config.sample_layer.physical_blend_weight
                    + sample_frame * self.config.sample_layer.sample_blend_weight)
                    * GF509_HEADROOM_GAIN
            } else {
                scene_frame.output
            };
            *left_sample = output;
            *right_sample = output;
        }
        Ok(left.len())
    }

    /// Restores deterministic post-construction state while retaining config.
    /// This may reload assets and must not be called from the audio callback.
    pub fn reset(&mut self) -> Result<(), String> {
        let replacement = Self::new(self.config.clone())?;
        *self = replacement;
        Ok(())
    }
}

#[derive(Deserialize)]
struct AssetManifest {
    schema_version: u32,
    key: String,
    output_gain: f32,
    samples: Vec<AssetEntry>,
}

#[derive(Deserialize)]
struct AssetEntry {
    key: String,
    metadata: String,
    native_rpm: f32,
    sample_rate: u32,
    channels: u16,
    loop_frames: usize,
    tonal: String,
    tonal_sha256: String,
    residual: String,
    residual_sha256: String,
}

fn validate_asset_manifest(directory: &Path, output_sample_rate: u32) -> Result<(), String> {
    let path = directory.join("manifest.json");
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read GF509 manifest {}: {error}", path.display()))?;
    let peek: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid GF509 manifest {}: {error}", path.display()))?;
    let schema = peek
        .get("schema_version")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    if schema == 2 {
        return validate_experimental_manifest(directory, output_sample_rate, &raw);
    }
    let manifest: AssetManifest = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid GF509 manifest {}: {error}", path.display()))?;
    if manifest.schema_version != 1 || manifest.key != "v10_gf509" {
        return Err("GF509 manifest schema/key mismatch".into());
    }
    if (manifest.output_gain - GF509_HEADROOM_GAIN).abs() > f32::EPSILON {
        return Err("GF509 manifest output gain does not match the frozen profile".into());
    }
    if manifest.samples.len() != 3 {
        return Err(format!(
            "GF509 manifest expected 3 samples, found {}",
            manifest.samples.len()
        ));
    }
    for (expected_key, entry) in ["low", "med", "max"].into_iter().zip(&manifest.samples) {
        if entry.key != expected_key
            || !entry.native_rpm.is_finite()
            || entry.sample_rate != output_sample_rate
            || entry.channels != 1
            || entry.loop_frames < 32
        {
            return Err(format!("invalid GF509 manifest entry: {}", entry.key));
        }
        for name in [&entry.metadata, &entry.tonal, &entry.residual] {
            let candidate = Path::new(name);
            if candidate.is_absolute() || candidate.components().count() != 1 {
                return Err(format!("GF509 asset path must be a local filename: {name}"));
            }
        }
        if !directory.join(&entry.metadata).is_file() {
            return Err(format!("missing GF509 metadata: {}", entry.metadata));
        }
        verify_sha256(&directory.join(&entry.tonal), &entry.tonal_sha256)?;
        verify_sha256(&directory.join(&entry.residual), &entry.residual_sha256)?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct ExperimentalVariant {
    group: String,
    position: usize,
    count: usize,
}

#[derive(Deserialize)]
struct ExperimentalEntry {
    key: String,
    role: String,
    metadata: String,
    native_rpm: f32,
    sample_rate: u32,
    channels: u16,
    loop_frames: usize,
    tonal: String,
    tonal_sha256: String,
    residual: String,
    residual_sha256: String,
    variant: Option<ExperimentalVariant>,
}

#[derive(Deserialize)]
struct ExperimentalManifest {
    schema_version: u32,
    key: String,
    output_gain: f32,
    samples: Vec<ExperimentalEntry>,
    off_samples: Vec<ExperimentalEntry>,
}

/// Schema-2 F2002 experimental bank: fail-closed validation. Any load
/// failure must abort with a clear message; silent fallback is forbidden.
fn validate_experimental_manifest(
    directory: &Path,
    output_sample_rate: u32,
    raw: &str,
) -> Result<(), String> {
    let manifest: ExperimentalManifest = serde_json::from_str(raw)
        .map_err(|error| format!("invalid experimental manifest: {error}"))?;
    if manifest.schema_version != 2 || manifest.key != "v10_f2002_experimental" {
        return Err("experimental manifest schema/key mismatch".into());
    }
    if (manifest.output_gain - GF509_HEADROOM_GAIN).abs() > f32::EPSILON {
        return Err("experimental manifest output gain does not match 0.61".into());
    }
    if manifest.samples.is_empty() || manifest.off_samples.is_empty() {
        return Err("experimental manifest needs ON and OFF collections".into());
    }
    for entry in manifest.samples.iter().chain(manifest.off_samples.iter()) {
        let expected_role = if manifest.samples.iter().any(|sample| sample.key == entry.key) {
            "on"
        } else {
            "off"
        };
        if entry.role != expected_role
            || !entry.native_rpm.is_finite()
            || entry.sample_rate != output_sample_rate
            || entry.channels != 1
            || entry.loop_frames < 32
        {
            return Err(format!("invalid experimental manifest entry: {}", entry.key));
        }
        for name in [&entry.metadata, &entry.tonal, &entry.residual] {
            let candidate = Path::new(name);
            if candidate.is_absolute() || candidate.components().count() != 1 {
                return Err(format!("experimental asset path must be a local filename: {name}"));
            }
            if !directory.join(name).is_file() {
                return Err(format!("missing experimental asset: {name}"));
            }
        }
        verify_sha256(&directory.join(&entry.tonal), &entry.tonal_sha256)?;
        verify_sha256(&directory.join(&entry.residual), &entry.residual_sha256)?;
        if let Some(variant) = &entry.variant {
            if variant.count < 2
                || variant.position >= variant.count
                || variant.group.is_empty()
            {
                return Err(format!(
                    "invalid variant record for experimental entry: {}",
                    entry.key
                ));
            }
        }
    }
    // Variant groups must be complete and consistent per role collection.
    for collection in [&manifest.samples, &manifest.off_samples] {
        let mut groups: std::collections::BTreeMap<&str, Vec<(&str, usize, usize)>> =
            std::collections::BTreeMap::new();
        for entry in collection {
            if let Some(variant) = &entry.variant {
                groups.entry(variant.group.as_str()).or_default().push((
                    entry.key.as_str(),
                    variant.position,
                    variant.count,
                ));
            }
        }
        for (group, members) in &groups {
            if members.iter().any(|(_, _, count)| *count != members.len()) {
                return Err(format!("experimental variant group {group} is incomplete"));
            }
            let mut positions: Vec<usize> =
                members.iter().map(|(_, position, _)| *position).collect();
            positions.sort_unstable();
            if positions != (0..members.len()).collect::<Vec<_>>() {
                return Err(format!("experimental variant group {group} positions invalid"));
            }
        }
    }
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read GF509 asset {}: {error}", path.display()))?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != expected {
        return Err(format!("GF509 asset hash mismatch: {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running_runtime(max_block_frames: usize) -> Gf509Runtime {
        let mut config = Gf509RuntimeConfig::default();
        config.max_block_frames = max_block_frames;
        let mut runtime = Gf509Runtime::new(config).unwrap();
        runtime
            .update_telemetry(RuntimeTelemetry {
                rpm: 7_499.0,
                throttle: 0.92,
                normalized_engine_load: 0.88,
                normalized_engine_torque: 0.75,
                torque_sign: TorqueSign::Positive,
                rpm_derivative: 1200.0,
                throttle_derivative: 0.5,
                gear: 3,
                shift_phase: ShiftPhase::None,
                clutch_engagement: 1.0,
                tc_cut_ratio: 0.0,
                rev_limiter_active: false,
                dt_seconds: 1.0 / 120.0,
            })
            .unwrap();
        runtime
    }

    #[test]
    fn control_rate_steps_converge_without_zipper() {
        // AUD-05: precomputed linear steps must reach the target exactly,
        // advance monotonically per sample (no step discontinuities), and stay
        // within float-rounding distance of the ideal linear ramp.
        let mut runtime = running_runtime(1024);
        // Settle to the initial target first.
        let mut settle_l = [0.0; 512];
        let mut settle_r = [0.0; 512];
        for _ in 0..4 {
            runtime.render_block(&mut settle_l, &mut settle_r).unwrap();
        }
        let from = runtime.rendered_telemetry();
        let target = RuntimeTelemetry {
            rpm: 12_000.0,
            throttle: 0.4,
            normalized_engine_load: 0.35,
            normalized_engine_torque: -0.2,
            torque_sign: TorqueSign::Negative,
            rpm_derivative: -2000.0,
            throttle_derivative: -2.0,
            gear: 4,
            shift_phase: ShiftPhase::None,
            clutch_engagement: 0.6,
            tc_cut_ratio: 0.3,
            rev_limiter_active: false,
            dt_seconds: 1.0 / 120.0,
        };
        runtime.update_telemetry(target).unwrap();
        let steps = runtime.interp_steps;
        let n = runtime.interpolation_remaining;
        assert!(n > 0);
        // Steps match the ideal linear ramp from the settled state.
        let expect = |a: f32, b: f32| (b - a) / n as f32;
        assert!((steps[0] - expect(from.rpm, target.rpm)).abs() < 1e-6);
        assert!((steps[6] - expect(from.clutch_engagement, target.clutch_engagement)).abs() < 1e-6);
        // Render sample by sample (1-frame blocks track rendered state).
        let mut prev_rpm = from.rpm;
        let mut max_step = 0.0f32;
        let mut one_l = [0.0; 1];
        let mut one_r = [0.0; 1];
        for i in 0..n {
            runtime.render_block(&mut one_l, &mut one_r).unwrap();
            assert!(one_l[0].is_finite());
            let rendered = runtime.rendered_telemetry();
            let ideal = from.rpm + (target.rpm - from.rpm) * (i + 1) as f32 / n as f32;
            // Snap on the final step keeps exact parity with the target.
            let tolerance = if i + 1 == n { 1e-6 } else { 2.0 };
            assert!((rendered.rpm - ideal).abs() < tolerance,
                "sample {i}: rpm {} vs ideal {ideal}", rendered.rpm);
            // Monotonic approach toward the target (no zipper reversals).
            if target.rpm > from.rpm {
                assert!(rendered.rpm + 1e-3 >= prev_rpm, "rpm stepped back at {i}");
            } else {
                assert!(rendered.rpm - 1e-3 <= prev_rpm, "rpm stepped back at {i}");
            }
            max_step = max_step.max((rendered.rpm - prev_rpm).abs());
            prev_rpm = rendered.rpm;
        }
        // Largest single-sample RPM step is the linear increment (no spikes).
        let linear = (target.rpm - from.rpm).abs() / n as f32;
        assert!(max_step <= linear + 1e-3, "max step {max_step} vs linear {linear}");
        let rendered = runtime.rendered_telemetry();
        assert!((rendered.rpm - target.rpm).abs() < 1e-6);
        assert!((rendered.clutch_engagement - target.clutch_engagement).abs() < 1e-6);
        assert!((rendered.tc_cut_ratio - target.tc_cut_ratio).abs() < 1e-6);
    }

    #[test]
    fn variable_blocks_are_finite_and_boundary_continuous() {
        let mut whole = running_runtime(1024);
        let mut split = running_runtime(1024);
        let mut whole_l = [0.0; 1024];
        let mut whole_r = [0.0; 1024];
        whole.render_block(&mut whole_l, &mut whole_r).unwrap();
        let mut joined = Vec::new();
        for frames in [1usize, 17, 255, 3, 512, 236] {
            let mut l = vec![0.0; frames];
            let mut r = vec![0.0; frames];
            split.render_block(&mut l, &mut r).unwrap();
            assert_eq!(l, r);
            assert!(l.iter().all(|sample| sample.is_finite()));
            joined.extend(l);
        }
        assert_eq!(joined, whole_l);
    }

    #[test]
    fn reset_restores_deterministic_output() {
        let mut runtime = running_runtime(128);
        let telemetry = runtime.telemetry();
        let mut first = [0.0; 128];
        let mut right = [0.0; 128];
        runtime.render_block(&mut first, &mut right).unwrap();
        runtime.reset().unwrap();
        runtime.update_telemetry(telemetry).unwrap();
        let mut after = [0.0; 128];
        runtime.render_block(&mut after, &mut right).unwrap();
        assert_eq!(first, after);
    }

    #[test]
    fn packaged_manifest_and_all_three_zones_load() {
        let mut config = Gf509RuntimeConfig::default();
        config.engine.sample_rate = 44_100;
        config.sample_layer_directory =
            Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../audio/v10_gf509"));
        let runtime = Gf509Runtime::new(config).unwrap();
        assert_eq!(runtime.sample_rate(), 44_100);
    }

    #[test]
    fn pitch_sweep_stays_finite_and_bounded() {
        // Full-route smoke: an RPM sweep crossing pitch-up ratios must render
        // finite audio under the PCM ceiling.
        let mut config = Gf509RuntimeConfig::default();
        config.engine.sample_rate = 44_100;
        config.max_block_frames = 512;
        config.sample_layer_directory =
            Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../audio/v10_gf509"));
        let mut runtime = Gf509Runtime::new(config).unwrap();
        let mut left = [0.0f32; 512];
        let mut right = [0.0f32; 512];
        let mut peak = 0.0f32;
        for block in 0..64 {
            // Sweep up then down through the zone anchors.
            let rpm = if block < 32 {
                4_000.0 + 12_000.0 * (block as f32 / 31.0)
            } else {
                16_000.0 - 12_000.0 * ((block - 32) as f32 / 31.0)
            };
            runtime
                .update_telemetry(RuntimeTelemetry {
                    rpm,
                    throttle: 0.9,
                    normalized_engine_load: 0.85,
                    normalized_engine_torque: 0.8,
                    torque_sign: TorqueSign::Positive,
                    rpm_derivative: 0.0,
                    throttle_derivative: 0.0,
                    gear: 4,
                    shift_phase: ShiftPhase::None,
                    clutch_engagement: 1.0,
                    tc_cut_ratio: 0.0,
                    rev_limiter_active: false,
                    dt_seconds: 1.0 / 120.0,
                })
                .unwrap();
            runtime.render_block(&mut left, &mut right).unwrap();
            assert!(left.iter().all(|sample| sample.is_finite()));
            assert_eq!(left, right);
            peak = peak.max(left.iter().map(|sample| sample.abs()).fold(0.0f32, f32::max));
        }
        assert!(peak > 1e-4, "route must stay audible");
        assert!(peak <= 1.0, "route must stay under PCM ceiling, got {peak}");
    }

    #[test]
    fn data_driven_blend_weights_are_validated() {
        let mut config = ThreeZoneSampleLayerConfig::default();
        assert!(config.validate().is_ok());

        config.physical_blend_weight = 2.5; // > 2.0
        assert!(config.validate().is_err());
        config.physical_blend_weight = 0.5;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn runtime_telemetry_validation_rejects_out_of_range() {
        let valid = RuntimeTelemetry {
            rpm: 9000.0,
            throttle: 0.8,
            normalized_engine_load: 0.7,
            normalized_engine_torque: 0.6,
            torque_sign: TorqueSign::Positive,
            rpm_derivative: 500.0,
            throttle_derivative: 1.0,
            gear: 4,
            shift_phase: ShiftPhase::None,
            clutch_engagement: 1.0,
            tc_cut_ratio: 0.0,
            rev_limiter_active: false,
            dt_seconds: 0.016,
        };
        assert!(valid.validate().is_ok());

        // Negative load rejected
        let mut invalid = valid;
        invalid.normalized_engine_load = -0.1;
        assert!(invalid.validate().is_err());

        // Load > 1.0 rejected
        invalid = valid;
        invalid.normalized_engine_load = 1.05;
        assert!(invalid.validate().is_err());

        // Torque < -1.0 rejected
        invalid = valid;
        invalid.normalized_engine_torque = -1.1;
        assert!(invalid.validate().is_err());

        // Torque > 1.0 rejected
        invalid = valid;
        invalid.normalized_engine_torque = 1.1;
        assert!(invalid.validate().is_err());

        // Non-finite derivatives rejected
        invalid = valid;
        invalid.rpm_derivative = f32::NAN;
        assert!(invalid.validate().is_err());

        invalid = valid;
        invalid.throttle_derivative = f32::INFINITY;
        assert!(invalid.validate().is_err());

        // Gear range
        invalid = valid;
        invalid.gear = -2;
        assert!(invalid.validate().is_err());
        invalid.gear = 13;
        assert!(invalid.validate().is_err());

        invalid = valid;
        invalid.clutch_engagement = -0.1;
        assert!(invalid.validate().is_err());
        invalid = valid;
        invalid.tc_cut_ratio = 1.1;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn continuous_mechanics_interpolate_discrete_states_do_not() {
        let mut runtime = running_runtime(1024);
        runtime
            .update_telemetry(RuntimeTelemetry {
                rpm: 8_000.0,
                throttle: 0.8,
                normalized_engine_load: 0.7,
                normalized_engine_torque: -0.4,
                torque_sign: TorqueSign::Negative,
                rpm_derivative: -100.0,
                throttle_derivative: -1.0,
                gear: 4,
                shift_phase: ShiftPhase::UpshiftCut,
                clutch_engagement: 0.2,
                tc_cut_ratio: 0.8,
                rev_limiter_active: true,
                dt_seconds: 1.0 / 120.0,
            })
            .unwrap();
        let mut left = [0.0; 1];
        let mut right = [0.0; 1];
        runtime.render_block(&mut left, &mut right).unwrap();
        let rendered = runtime.rendered_telemetry();
        assert_eq!(rendered.shift_phase, ShiftPhase::UpshiftCut);
        assert!(rendered.clutch_engagement > 0.0 && rendered.clutch_engagement < 0.2);
        assert!(rendered.tc_cut_ratio > 0.0 && rendered.tc_cut_ratio < 0.8);
        assert!(rendered.rev_limiter_active);
    }

    #[test]
    fn cut_and_recovery_are_temporally_smooth_at_fixed_rpm() {
        let mut runtime = running_runtime(1024);
        let steady = RuntimeTelemetry {
            rpm: 10_000.0,
            throttle: 0.9,
            normalized_engine_load: 0.9,
            normalized_engine_torque: 0.9,
            torque_sign: TorqueSign::Positive,
            rpm_derivative: 0.0,
            throttle_derivative: 0.0,
            gear: 4,
            shift_phase: ShiftPhase::None,
            clutch_engagement: 1.0,
            tc_cut_ratio: 0.0,
            rev_limiter_active: false,
            dt_seconds: 1.0 / 120.0,
        };
        runtime.update_telemetry(steady).unwrap();
        let mut left = [0.0; 512];
        let mut right = [0.0; 512];
        for _ in 0..40 {
            runtime.render_block(&mut left, &mut right).unwrap();
        }
        let steady_rms = render_average_rms(&mut runtime, &mut left, &mut right, 16);
        let steady_peak = left.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(steady_rms.is_finite() && steady_rms > 1e-6);

        // Control without event: an identical runtime held at steady must stay
        // near steady, proving any later drop comes from the cut (R3).
        let mut control = running_runtime(1024);
        control.update_telemetry(steady).unwrap();
        let mut control_left = [0.0; 512];
        let mut control_right = [0.0; 512];
        for _ in 0..40 {
            control.render_block(&mut control_left, &mut control_right).unwrap();
        }
        let control_rms = render_average_rms(&mut control, &mut control_left, &mut control_right, 16);
        assert!((control_rms - steady_rms).abs() / steady_rms < 0.05,
            "control without event must track steady: steady={steady_rms} control={control_rms}");

        // Cut onset: the first blocks after the event already fall, and the
        // settled cut attenuates effectively (a no-op cut would fail here).
        let mut cut = steady;
        cut.shift_phase = ShiftPhase::UpshiftCut;
        runtime.update_telemetry(cut).unwrap();
        let onset_rms = render_average_rms(&mut runtime, &mut left, &mut right, 4);
        assert!(onset_rms < steady_rms,
            "cut onset must start falling immediately: steady={steady_rms} onset={onset_rms}");
        for _ in 0..36 {
            runtime.render_block(&mut left, &mut right).unwrap();
        }
        let cut_rms = render_average_rms(&mut runtime, &mut left, &mut right, 16);
        let cut_peak = left.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(cut_rms.is_finite() && cut_rms < steady_rms * 1.5,
            "cut response must remain bounded: steady={steady_rms} cut={cut_rms}");
        assert!(cut_rms < steady_rms * 0.95,
            "cut must attenuate effectively vs steady control: steady={steady_rms} cut={cut_rms}");
        assert!((control_rms - cut_rms).abs() / steady_rms > 0.05,
            "cut must separate from the no-event control: control={control_rms} cut={cut_rms}");
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(cut_peak.is_finite() && cut_peak <= steady_peak * 1.5 + 1e-6);

        // Recovery: rises from the cut toward steady without overshoot.
        let mut recovery = cut;
        recovery.shift_phase = ShiftPhase::UpshiftRecovery;
        runtime.update_telemetry(recovery).unwrap();
        let recovery_onset = render_average_rms(&mut runtime, &mut left, &mut right, 4);
        assert!(recovery_onset > cut_rms * 0.9,
            "recovery must not collapse below the cut: cut={cut_rms} recovery_onset={recovery_onset}");
        for _ in 0..36 {
            runtime.render_block(&mut left, &mut right).unwrap();
        }
        let recovery_rms = render_average_rms(&mut runtime, &mut left, &mut right, 16);
        assert!(recovery_rms.is_finite() && recovery_rms < steady_rms * 1.5,
            "recovery response must remain bounded: steady={steady_rms} recovery={recovery_rms}");
        assert!(recovery_rms > cut_rms,
            "recovery must rise from the cut: cut={cut_rms} recovery={recovery_rms}");
        assert!(recovery_rms < steady_rms * 1.05,
            "recovery must not overshoot steady: steady={steady_rms} recovery={recovery_rms}");
        assert!(left.iter().all(|sample| sample.is_finite()));

        // Return to steady restores the baseline level (duration/recovery closed).
        runtime.update_telemetry(steady).unwrap();
        for _ in 0..40 {
            runtime.render_block(&mut left, &mut right).unwrap();
        }
        let restored = render_average_rms(&mut runtime, &mut left, &mut right, 16);
        assert!((restored - steady_rms).abs() / steady_rms < 0.10,
            "steady must be restored after the event: steady={steady_rms} restored={restored}");
    }

    fn render_average_rms(
        runtime: &mut Gf509Runtime,
        left: &mut [f32; 512],
        right: &mut [f32; 512],
        blocks: usize,
    ) -> f32 {
        let mut energy = 0.0;
        for _ in 0..blocks {
            runtime.render_block(left, right).unwrap();
            energy += left.iter().map(|sample| sample * sample).sum::<f32>();
        }
        (energy / (blocks * left.len()) as f32).sqrt()
    }

    #[test]
    fn torque_sign_and_shift_phase_conversions() {
        assert_eq!(TorqueSign::from_i32(-1).unwrap(), TorqueSign::Negative);
        assert_eq!(TorqueSign::from_i32(0).unwrap(), TorqueSign::Neutral);
        assert_eq!(TorqueSign::from_i32(1).unwrap(), TorqueSign::Positive);
        assert!(TorqueSign::from_i32(2).is_err());

        assert_eq!(ShiftPhase::from_i32(0).unwrap(), ShiftPhase::None);
        assert_eq!(ShiftPhase::from_i32(1).unwrap(), ShiftPhase::UpshiftCut);
        assert_eq!(ShiftPhase::from_i32(2).unwrap(), ShiftPhase::UpshiftRecovery);
        assert_eq!(ShiftPhase::from_i32(3).unwrap(), ShiftPhase::DownshiftCut);
        assert_eq!(ShiftPhase::from_i32(4).unwrap(), ShiftPhase::DownshiftBlip);
        assert_eq!(ShiftPhase::from_i32(5).unwrap(), ShiftPhase::DownshiftRecovery);
        assert!(ShiftPhase::from_i32(6).is_err());
    }

    #[test]
    fn offline_load_scenarios_produce_measurably_different_audio() {
        // Section 14.3 test cases:
        // A: 15000 RPM, throttle 1.0, load 1.0, positive torque
        // B: 15000 RPM, throttle 1.0, load 0.25, positive torque
        // C: 15000 RPM, throttle 0.0, load 0.55, negative torque
        // D: 15000 RPM, throttle 0.0, load 0.05, near-zero torque
        let make_runtime = |throttle: f32, load: f32, torque: f32, t_sign: TorqueSign| -> Gf509Runtime {
            let mut config = Gf509RuntimeConfig::default();
            config.max_block_frames = 1024;
            let mut rt = Gf509Runtime::new(config).unwrap();
            rt.update_telemetry(RuntimeTelemetry {
                rpm: 15_000.0,
                throttle,
                normalized_engine_load: load,
                normalized_engine_torque: torque,
                torque_sign: t_sign,
                rpm_derivative: 0.0,
                throttle_derivative: 0.0,
                gear: 5,
                shift_phase: ShiftPhase::None,
                clutch_engagement: 1.0,
                tc_cut_ratio: 0.0,
                rev_limiter_active: false,
                dt_seconds: 0.0,
            }).unwrap();
            rt
        };

        let mut rt_a = make_runtime(1.0, 1.0, 1.0, TorqueSign::Positive);
        let mut rt_b = make_runtime(1.0, 0.25, 0.25, TorqueSign::Positive);
        let mut rt_c = make_runtime(0.0, 0.55, -0.55, TorqueSign::Negative);
        let mut rt_d = make_runtime(0.0, 0.05, 0.0, TorqueSign::Neutral);

        let render_block = |rt: &mut Gf509Runtime| -> Vec<f32> {
            let mut l = [0.0f32; 1024];
            let mut r = [0.0f32; 1024];
            // Warm up
            for _ in 0..5 {
                rt.render_block(&mut l, &mut r).unwrap();
            }
            rt.render_block(&mut l, &mut r).unwrap();
            l.to_vec()
        };

        let out_a = render_block(&mut rt_a);
        let out_b = render_block(&mut rt_b);
        let out_c = render_block(&mut rt_c);
        let out_d = render_block(&mut rt_d);

        let max_diff = |s1: &[f32], s2: &[f32]| -> f32 {
            s1.iter().zip(s2.iter()).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max)
        };

        let diff_ab = max_diff(&out_a, &out_b);
        let diff_ac = max_diff(&out_a, &out_c);
        let diff_ad = max_diff(&out_a, &out_d);
        let diff_bc = max_diff(&out_b, &out_c);
        let diff_cd = max_diff(&out_c, &out_d);

        assert!(diff_ab > 1e-4, "Scenario A and B must differ (same throttle 1.0, different load 1.0 vs 0.25): got {diff_ab}");
        assert!(diff_ac > 1e-4, "Scenario A and C must differ: got {diff_ac}");
        assert!(diff_ad > 1e-4, "Scenario A and D must differ: got {diff_ad}");
        assert!(diff_bc > 1e-4, "Scenario B and C must differ: got {diff_bc}");
        assert!(diff_cd > 1e-4, "Scenario C and D must differ (same throttle 0.0, different load/torque): got {diff_cd}");
    }
}
