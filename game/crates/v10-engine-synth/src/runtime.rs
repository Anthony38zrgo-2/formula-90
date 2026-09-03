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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeTelemetry {
    pub rpm: f32,
    pub throttle: f32,
    pub load: f32,
    pub gear: i8,
    pub dt_seconds: f32,
}

impl RuntimeTelemetry {
    fn validate(self) -> Result<Self, String> {
        EngineInput {
            rpm: self.rpm,
            throttle: self.throttle,
            load: self.load,
        }
        .validate()?;
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
            load: 0.0,
            gear: 0,
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
                self.rendered_telemetry.load +=
                    (target.load - self.rendered_telemetry.load) / remaining;
                self.interpolation_remaining -= 1;
            } else {
                self.rendered_telemetry = target;
            }
            self.rendered_telemetry.gear = target.gear;
            self.rendered_telemetry.dt_seconds = target.dt_seconds;
            let interpolated = self.rendered_telemetry;
            self.engine.set_input(EngineInput {
                rpm: interpolated.rpm,
                throttle: interpolated.throttle,
                load: interpolated.load,
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
                        load: interpolated.load,
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
                load: 0.88,
                gear: 3,
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
}

