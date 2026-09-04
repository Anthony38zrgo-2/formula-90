//! Allocation-free block facade for embedding the GF509 continuous engine.
//!
//! Asset loading and reset are control-thread operations. `render_block` owns
//! no temporary buffers and performs no file I/O or logging.

use std::path::PathBuf;
use std::{fs, path::Path};

use crate::{
    AcousticScene, AcousticSceneConfig, EngineConfig, EngineInput, SampleLayerInput,
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
    mid_ducker: ComplementaryMidDucker,
    telemetry: RuntimeTelemetry,
    rendered_telemetry: RuntimeTelemetry,
    interpolation_remaining: usize,
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
        let mid_ducker = ComplementaryMidDucker::new(
            sample_rate as f32,
            config.sample_layer.mid_duck_depth_db,
            config.sample_layer.mid_duck_threshold,
        );
        Ok(Self {
            config,
            engine,
            scene,
            sample_layer,
            mid_ducker,
            telemetry: RuntimeTelemetry::default(),
            rendered_telemetry: RuntimeTelemetry::default(),
            interpolation_remaining: 0,
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
        self.interpolation_remaining =
            (telemetry.dt_seconds * self.sample_rate() as f32).round() as usize;
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
                let remaining = self.interpolation_remaining as f32;
                self.rendered_telemetry.rpm +=
                    (target.rpm - self.rendered_telemetry.rpm) / remaining;
                self.rendered_telemetry.throttle +=
                    (target.throttle - self.rendered_telemetry.throttle) / remaining;
                self.rendered_telemetry.normalized_engine_load +=
                    (target.normalized_engine_load - self.rendered_telemetry.normalized_engine_load)
                        / remaining;
                self.rendered_telemetry.normalized_engine_torque +=
                    (target.normalized_engine_torque
                        - self.rendered_telemetry.normalized_engine_torque)
                        / remaining;
                self.rendered_telemetry.rpm_derivative +=
                    (target.rpm_derivative - self.rendered_telemetry.rpm_derivative) / remaining;
                self.rendered_telemetry.throttle_derivative += (target.throttle_derivative
                    - self.rendered_telemetry.throttle_derivative)
                    / remaining;
                self.interpolation_remaining -= 1;
            } else {
                self.rendered_telemetry = target;
            }
            self.rendered_telemetry.torque_sign = target.torque_sign;
            self.rendered_telemetry.gear = target.gear;
            self.rendered_telemetry.shift_phase = target.shift_phase;
            self.rendered_telemetry.dt_seconds = target.dt_seconds;
            let interpolated = self.rendered_telemetry;
            self.engine.set_input(EngineInput {
                rpm: interpolated.rpm,
                throttle: interpolated.throttle,
                load: interpolated.normalized_engine_load,
            })?;
            let engine_frame = self.engine.render_sample();
            let scene_frame = self.scene.process(&engine_frame);
            let sample_frame = self
                .sample_layer
                .as_mut()
                .map(|layer| {
                    layer.process(SampleLayerInput {
                        rpm: interpolated.rpm,
                        throttle: interpolated.throttle,
                        load: interpolated.normalized_engine_load,
                        crank_phase_deg: engine_frame.crank_phase_deg,
                    })
                })
                .transpose()?;
            let output = if let Some(sample_frame) = sample_frame {
                let (ducked_scene, _) = self
                    .mid_ducker
                    .process(scene_frame.output, sample_frame.mid_bus);
                (ducked_scene * self.config.sample_layer.physical_blend_weight
                    + sample_frame.output * self.config.sample_layer.sample_blend_weight)
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

fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read GF509 asset {}: {error}", path.display()))?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != expected {
        return Err(format!("GF509 asset hash mismatch: {}", path.display()));
    }
    Ok(())
}

struct OnePoleLowPass {
    alpha: f32,
    state: f32,
}

impl OnePoleLowPass {
    fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        Self {
            alpha: 1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp(),
            state: 0.0,
        }
    }
    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        self.state += self.alpha * (input - self.state);
        self.state
    }
}

struct ComplementaryMidDucker {
    low: OnePoleLowPass,
    high: OnePoleLowPass,
    envelope: f32,
    attack: f32,
    release: f32,
    threshold: f32,
    duck_floor_gain: f32,
}

impl ComplementaryMidDucker {
    fn new(sample_rate: f32, depth_db: f32, threshold: f32) -> Self {
        Self {
            low: OnePoleLowPass::new(350.0, sample_rate),
            high: OnePoleLowPass::new(2_000.0, sample_rate),
            envelope: 0.0,
            attack: 1.0 - (-1.0 / (0.020 * sample_rate)).exp(),
            release: 1.0 - (-1.0 / (0.140 * sample_rate)).exp(),
            threshold: threshold.max(0.001),
            duck_floor_gain: 10.0f32.powf(depth_db / 20.0),
        }
    }
    #[inline]
    fn process(&mut self, simulation: f32, sample_mid: f32) -> (f32, f32) {
        let detector = sample_mid.abs();
        let rate = if detector > self.envelope {
            self.attack
        } else {
            self.release
        };
        self.envelope += rate * (detector - self.envelope);
        let depth = (self.envelope / self.threshold).clamp(0.0, 1.0);
        let gain = 1.0 - (1.0 - self.duck_floor_gain) * depth;
        let below_high = self.high.process(simulation);
        let below_low = self.low.process(simulation);
        let mid = below_high - below_low;
        (simulation + mid * (gain - 1.0), gain)
    }
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
                dt_seconds: 1.0 / 120.0,
            })
            .unwrap();
        runtime
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
    fn data_driven_blend_weights_and_ducking_parameters_are_validated() {
        let mut config = ThreeZoneSampleLayerConfig::default();
        assert!(config.validate().is_ok());

        config.physical_blend_weight = 2.5; // > 2.0
        assert!(config.validate().is_err());
        config.physical_blend_weight = 0.5;

        config.mid_duck_depth_db = -30.0; // < -24.0
        assert!(config.validate().is_err());
        config.mid_duck_depth_db = -6.0;

        config.mid_duck_threshold = 0.0001; // < 0.001
        assert!(config.validate().is_err());
        config.mid_duck_threshold = 0.1;
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

