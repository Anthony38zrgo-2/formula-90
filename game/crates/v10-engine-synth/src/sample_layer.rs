use serde::Deserialize;
use std::f64::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};

const ZONE_COUNT: usize = 3;
const SINC_RADIUS: isize = 4;

#[derive(Clone, Copy, Debug)]
pub struct SampleLayerInput {
    pub rpm: f32,
    pub throttle: f32,
    pub load: f32,
    pub crank_phase_deg: f32,
}

impl SampleLayerInput {
    fn validate(self) -> Result<Self, String> {
        if !self.rpm.is_finite() || !(0.0..=25_000.0).contains(&self.rpm) {
            return Err(format!("sample-layer rpm out of range: {}", self.rpm));
        }
        if !self.throttle.is_finite() || !(0.0..=1.0).contains(&self.throttle) {
            return Err(format!(
                "sample-layer throttle out of range: {}",
                self.throttle
            ));
        }
        if !self.load.is_finite() || !(0.0..=1.0).contains(&self.load) {
            return Err(format!("sample-layer load out of range: {}", self.load));
        }
        if !self.crank_phase_deg.is_finite() {
            return Err("sample-layer crank phase must be finite".into());
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ThreeZoneSampleLayerConfig {
    pub tonal_gain_closed: f32,
    pub tonal_gain_loaded: f32,
    pub residual_gain_closed: f32,
    pub residual_gain_loaded: f32,
}

impl Default for ThreeZoneSampleLayerConfig {
    fn default() -> Self {
        Self {
            tonal_gain_closed: 0.05,
            tonal_gain_loaded: 0.22,
            residual_gain_closed: 0.12,
            residual_gain_loaded: 0.40,
        }
    }
}

impl ThreeZoneSampleLayerConfig {
    fn validate(self) -> Result<Self, String> {
        for (name, value) in [
            ("tonal_gain_closed", self.tonal_gain_closed),
            ("tonal_gain_loaded", self.tonal_gain_loaded),
            ("residual_gain_closed", self.residual_gain_closed),
            ("residual_gain_loaded", self.residual_gain_loaded),
        ] {
            if !value.is_finite() || !(0.0..=1.5).contains(&value) {
                return Err(format!("{name} outside 0..1.5: {value}"));
            }
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SampleLayerFrame {
    pub tonal: f32,
    pub residual: f32,
    pub output: f32,
    pub zone_weights: [f32; ZONE_COUNT],
}

#[derive(Deserialize)]
struct PreparedFiles {
    loop_tonal: String,
    loop_residual: String,
}

#[derive(Deserialize)]
struct PreparedMetadata {
    schema_version: u32,
    sample_rate: u32,
    rpm_anchor: f32,
    firing_phase_degrees_at_loop_start: f32,
    loop_length_samples: usize,
    files: PreparedFiles,
}

struct SampleZone {
    rpm_anchor: f32,
    firing_phase_deg: f32,
    source_sample_rate: u32,
    tonal: Vec<f32>,
    residual: Vec<f32>,
    cursor: f64,
}

impl SampleZone {
    fn load(metadata_path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(metadata_path)
            .map_err(|error| format!("cannot read {}: {error}", metadata_path.display()))?;
        let metadata: PreparedMetadata = serde_json::from_str(&raw)
            .map_err(|error| format!("invalid {}: {error}", metadata_path.display()))?;
        if metadata.schema_version != 1 {
            return Err(format!(
                "unsupported sample-layer schema {} in {}",
                metadata.schema_version,
                metadata_path.display()
            ));
        }
        if !metadata.rpm_anchor.is_finite() || !(1_000.0..=25_000.0).contains(&metadata.rpm_anchor)
        {
            return Err(format!("invalid RPM anchor in {}", metadata_path.display()));
        }
        if !metadata.firing_phase_degrees_at_loop_start.is_finite() {
            return Err(format!(
                "invalid firing phase in {}",
                metadata_path.display()
            ));
        }
        let parent = metadata_path.parent().unwrap_or_else(|| Path::new("."));
        let tonal = read_mono_pcm16(&parent.join(&metadata.files.loop_tonal))?;
        let residual = read_mono_pcm16(&parent.join(&metadata.files.loop_residual))?;
        if tonal.sample_rate != metadata.sample_rate || residual.sample_rate != metadata.sample_rate
        {
            return Err(format!(
                "WAV/metadata sample-rate mismatch in {}",
                metadata_path.display()
            ));
        }
        if tonal.samples.len() != metadata.loop_length_samples
            || residual.samples.len() != metadata.loop_length_samples
            || tonal.samples.len() < 32
        {
            return Err(format!(
                "WAV/metadata loop-length mismatch in {}",
                metadata_path.display()
            ));
        }
        Ok(Self {
            rpm_anchor: metadata.rpm_anchor,
            firing_phase_deg: metadata
                .firing_phase_degrees_at_loop_start
                .rem_euclid(360.0),
            source_sample_rate: metadata.sample_rate,
            tonal: tonal.samples,
            residual: residual.samples,
            cursor: 0.0,
        })
    }

    fn align(&mut self, crank_phase_deg: f32) {
        let target_firing_phase = (crank_phase_deg * 5.0).rem_euclid(360.0);
        let phase_delta = (target_firing_phase - self.firing_phase_deg).rem_euclid(360.0);
        let firing_cycle_samples = self.source_sample_rate as f64 * 12.0 / self.rpm_anchor as f64;
        self.cursor = phase_delta as f64 / 360.0 * firing_cycle_samples;
    }

    #[inline]
    fn render(&mut self, rpm: f32, output_sample_rate: u32) -> (f32, f32) {
        let tonal = sinc_sample(&self.tonal, self.cursor);
        let residual = sinc_sample(&self.residual, self.cursor);
        let rate = rpm as f64 / self.rpm_anchor as f64 * self.source_sample_rate as f64
            / output_sample_rate as f64;
        self.cursor = (self.cursor + rate).rem_euclid(self.tonal.len() as f64);
        (tonal, residual)
    }
}

pub struct ThreeZoneSampleLayer {
    output_sample_rate: u32,
    config: ThreeZoneSampleLayerConfig,
    zones: [SampleZone; ZONE_COUNT],
    phase_aligned: bool,
}

impl ThreeZoneSampleLayer {
    pub fn load_directory(
        output_sample_rate: u32,
        directory: &Path,
        config: ThreeZoneSampleLayerConfig,
    ) -> Result<Self, String> {
        let mut metadata_paths = fs::read_dir(directory)
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        metadata_paths.retain(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".sample-layer.json"))
        });
        metadata_paths.sort();
        let metadata_paths: [PathBuf; ZONE_COUNT] =
            metadata_paths.try_into().map_err(|paths: Vec<_>| {
                format!(
                    "ThreeZoneSampleLayer expected 3 metadata files in {}, found {}",
                    directory.display(),
                    paths.len()
                )
            })?;
        Self::load(output_sample_rate, metadata_paths, config)
    }

    pub fn load(
        output_sample_rate: u32,
        metadata_paths: [PathBuf; ZONE_COUNT],
        config: ThreeZoneSampleLayerConfig,
    ) -> Result<Self, String> {
        if !(8_000..=192_000).contains(&output_sample_rate) {
            return Err(format!(
                "sample-layer output rate out of range: {output_sample_rate}"
            ));
        }
        let config = config.validate()?;
        let mut loaded = metadata_paths
            .iter()
            .map(|path| SampleZone::load(path))
            .collect::<Result<Vec<_>, _>>()?;
        loaded.sort_by(|left, right| left.rpm_anchor.total_cmp(&right.rpm_anchor));
        if loaded
            .windows(2)
            .any(|pair| pair[1].rpm_anchor - pair[0].rpm_anchor < 100.0)
        {
            return Err("sample-layer RPM anchors must be distinct and ordered".into());
        }
        let zones: [SampleZone; ZONE_COUNT] = loaded
            .try_into()
            .map_err(|_| "ThreeZoneSampleLayer requires exactly three zones".to_string())?;
        Ok(Self {
            output_sample_rate,
            config,
            zones,
            phase_aligned: false,
        })
    }

    pub fn reset_phase(&mut self, crank_phase_deg: f32) -> Result<(), String> {
        if !crank_phase_deg.is_finite() {
            return Err("sample-layer crank phase must be finite".into());
        }
        for zone in &mut self.zones {
            zone.align(crank_phase_deg);
        }
        self.phase_aligned = true;
        Ok(())
    }

    pub fn rpm_anchors(&self) -> [f32; ZONE_COUNT] {
        std::array::from_fn(|index| self.zones[index].rpm_anchor)
    }

    #[inline]
    pub fn process(&mut self, input: SampleLayerInput) -> Result<SampleLayerFrame, String> {
        let input = input.validate()?;
        if !self.phase_aligned {
            self.reset_phase(input.crank_phase_deg)?;
        }
        let weights = zone_weights(input.rpm, self.rpm_anchors());
        let mut tonal = 0.0;
        let mut residual = 0.0;
        for (zone, weight) in self.zones.iter_mut().zip(weights) {
            let (zone_tonal, zone_residual) = zone.render(input.rpm, self.output_sample_rate);
            tonal += zone_tonal * weight;
            residual += zone_residual * weight;
        }
        let charge = input.load * (0.35 + 0.65 * input.throttle);
        let tonal_gain = self.config.tonal_gain_closed
            + (self.config.tonal_gain_loaded - self.config.tonal_gain_closed) * charge;
        let residual_gain = self.config.residual_gain_closed
            + (self.config.residual_gain_loaded - self.config.residual_gain_closed) * charge;
        tonal *= tonal_gain;
        residual *= residual_gain;
        Ok(SampleLayerFrame {
            tonal,
            residual,
            output: tonal + residual,
            zone_weights: weights,
        })
    }
}

#[inline]
fn zone_weights(rpm: f32, anchors: [f32; ZONE_COUNT]) -> [f32; ZONE_COUNT] {
    if rpm <= anchors[0] {
        return [1.0, 0.0, 0.0];
    }
    if rpm < anchors[1] {
        let t = ((rpm - anchors[0]) / (anchors[1] - anchors[0])).clamp(0.0, 1.0);
        let theta = t * std::f32::consts::FRAC_PI_2;
        return [theta.cos(), theta.sin(), 0.0];
    }
    if rpm < anchors[2] {
        let t = ((rpm - anchors[1]) / (anchors[2] - anchors[1])).clamp(0.0, 1.0);
        let theta = t * std::f32::consts::FRAC_PI_2;
        return [0.0, theta.cos(), theta.sin()];
    }
    [0.0, 0.0, 1.0]
}

#[inline]
fn sinc_sample(samples: &[f32], cursor: f64) -> f32 {
    let base = cursor.floor() as isize;
    let fraction = cursor - cursor.floor();
    let mut output = 0.0f64;
    let mut weight_sum = 0.0f64;
    for tap in -SINC_RADIUS + 1..=SINC_RADIUS {
        let distance = tap as f64 - fraction;
        let sinc = if distance.abs() < 1.0e-12 {
            1.0
        } else {
            (PI * distance).sin() / (PI * distance)
        };
        let normalized = distance / SINC_RADIUS as f64;
        let window = if normalized.abs() <= 1.0 {
            0.5 + 0.5 * (PI * normalized).cos()
        } else {
            0.0
        };
        let index = (base + tap).rem_euclid(samples.len() as isize) as usize;
        let weight = sinc * window;
        output += samples[index] as f64 * weight;
        weight_sum += weight;
    }
    (output / weight_sum.max(1.0e-12)) as f32
}

struct Pcm16Wav {
    sample_rate: u32,
    samples: Vec<f32>,
}

fn read_mono_pcm16(path: &Path) -> Result<Pcm16Wav, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(format!("not a RIFF/WAVE file: {}", path.display()));
    }
    let mut cursor = 12usize;
    let mut format = None;
    let mut data = None;
    while cursor + 8 <= bytes.len() {
        let id = &bytes[cursor..cursor + 4];
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let start = cursor + 8;
        let end = start
            .checked_add(size)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| format!("truncated WAV chunk in {}", path.display()))?;
        if id == b"fmt " && size >= 16 {
            format = Some((
                u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
            ));
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        cursor = end + (size & 1);
    }
    let (encoding, channels, sample_rate, bits) =
        format.ok_or_else(|| format!("missing WAV fmt chunk: {}", path.display()))?;
    if encoding != 1 || channels != 1 || bits != 16 {
        return Err(format!("expected mono PCM16 WAV: {}", path.display()));
    }
    let data = data.ok_or_else(|| format!("missing WAV data chunk: {}", path.display()))?;
    if data.len() % 2 != 0 {
        return Err(format!("odd PCM16 data length: {}", path.display()));
    }
    let samples = data
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]) as f32 / 32_768.0)
        .collect();
    Ok(Pcm16Wav {
        sample_rate,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossfades_have_constant_power() {
        let anchors = [7_500.0, 10_000.0, 15_000.0];
        for rpm in [7_500.0, 8_750.0, 10_000.0, 12_500.0, 15_000.0] {
            let weights = zone_weights(rpm, anchors);
            let power: f32 = weights.iter().map(|weight| weight * weight).sum();
            assert!((power - 1.0).abs() < 1.0e-6);
        }
    }

    #[test]
    fn sinc_resampler_wraps_without_index_failure() {
        let source = [0.0, 1.0, 0.0, -1.0];
        for cursor in [0.0, 0.5, 3.75, 4.0, 9.25] {
            assert!(sinc_sample(&source, cursor).is_finite());
        }
        assert!(sinc_sample(&source, 0.0).abs() < 1.0e-6);
        assert!((sinc_sample(&source, 1.0) - 1.0).abs() < 1.0e-6);
    }
}
