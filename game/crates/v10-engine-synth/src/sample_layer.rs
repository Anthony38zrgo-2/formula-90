use serde::Deserialize;
use std::f64::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};

const ZONE_COUNT: usize = 3;
const SINC_RADIUS: isize = 4;
/// AUD-07: fractional-phase resolution of the precomputed sinc tables.
const SINC_TABLE_PHASES: usize = 1024;
/// AUD-07: radius of the ratio-aware antialias kernel.  The kernel is
/// precomputed at init; 32 taps are enough for the normal 1..2x operating
/// range and remain bounded for the validated worst case (26.9x at 8 kHz).
const AA_SINC_RADIUS: isize = 16;
const AA_SINC_TAPS: usize = (2 * AA_SINC_RADIUS) as usize;
/// Uniform samples of `1 - 1/ratio`, which gives useful resolution close to
/// ratio 1 without a logarithm in the callback.  The final row is the exact
/// per-asset maximum ratio, so no fixed <=4x assumption is made.
const AA_RATIO_LEVELS: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct SampleLayerInput {
    pub rpm: f32,
    pub throttle: f32,
    pub load: f32,
    /// Signed engine torque normalized to the physical maximum.
    pub normalized_engine_torque: f32,
    /// Physical clutch engagement; used to gate retention-only off audio.
    pub clutch_engagement: f32,
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
        if !self.normalized_engine_torque.is_finite()
            || !(-1.0..=1.0).contains(&self.normalized_engine_torque)
        {
            return Err(format!(
                "sample-layer normalized torque out of range: {}",
                self.normalized_engine_torque
            ));
        }
        if !self.clutch_engagement.is_finite()
            || !(0.0..=1.0).contains(&self.clutch_engagement)
        {
            return Err(format!(
                "sample-layer clutch engagement out of range: {}",
                self.clutch_engagement
            ));
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
    pub max_fade_start_rpm: f32,
    pub max_full_rpm: f32,
    /// Physical simulation blend weight (default 1.0).
    pub physical_blend_weight: f32,
    /// Sample layer blend weight (default 1.0).
    pub sample_blend_weight: f32,
    /// Maximum complementary mid-band ducking depth in dB (default -3.0 dB).
    pub mid_duck_depth_db: f32,
    /// Complementary mid ducker envelope detector threshold (default 0.080).
    pub mid_duck_threshold: f32,
    /// Off-throttle overrun layer blend gain (default 0.20).
    pub off_throttle_gain: f32,
    /// AUD-06: skip sinc resampling + mid DSP for zones whose crossfade weight
    /// is exactly zero (or the off stem when its weight is zero), while still
    /// advancing their cursors so phase continuity is preserved. Zones with any
    /// nonzero weight — including fade edges — always render (default true).
    pub suspend_inaudible_zones: bool,
    /// AUD-06 (fix.txt): deep-silence margin in RPM. A zero-weight zone is
    /// only suspended when the RPM lies this far outside every crossfade it
    /// takes part in; zones approaching a fade keep rendering ("cercanas a un
    /// crossfade disponibles") so their filters track and are converged at
    /// fade entry. The stateless off stem needs no margin (default 500.0).
    pub suspend_margin_rpm: f32,
    /// AUD-07a (cost): resample through the precomputed windowed-sinc table
    /// instead of evaluating sin/cos/division per tap per sample. The table
    /// bakes the same radius-4 Hann kernel including per-phase normalization,
    /// so it is a transparent accelerator, not an antialias fix (default true).
    pub use_tabled_sinc: bool,
    /// AUD-07: pitch-up antialias package. Above ratio 1.0 the resampler uses
    /// a pretabulated kernel whose cutoff is `min(1, 1/ratio)`, before the
    /// output sample is produced. The reference sinc path remains available
    /// through `use_tabled_sinc=false`; this option only controls the new
    /// ratio-aware table.
    pub pitch_up_antialias: bool,
}

impl Default for ThreeZoneSampleLayerConfig {
    fn default() -> Self {
        Self {
            tonal_gain_closed: 0.05,
            tonal_gain_loaded: 0.22,
            residual_gain_closed: 0.12,
            residual_gain_loaded: 0.40,
            max_fade_start_rpm: 5_970.0,
            max_full_rpm: 8_725.0,
            physical_blend_weight: 1.0,
            sample_blend_weight: 1.0,
            mid_duck_depth_db: -3.0,
            mid_duck_threshold: 0.080,
            off_throttle_gain: 0.20,
            suspend_inaudible_zones: true,
            suspend_margin_rpm: 500.0,
            use_tabled_sinc: true,
            pitch_up_antialias: true,
        }
    }
}

impl ThreeZoneSampleLayerConfig {
    pub fn validate(self) -> Result<Self, String> {
        for (name, value) in [
            ("tonal_gain_closed", self.tonal_gain_closed),
            ("tonal_gain_loaded", self.tonal_gain_loaded),
            ("residual_gain_closed", self.residual_gain_closed),
            ("residual_gain_loaded", self.residual_gain_loaded),
            ("physical_blend_weight", self.physical_blend_weight),
            ("sample_blend_weight", self.sample_blend_weight),
            ("off_throttle_gain", self.off_throttle_gain),
        ] {
            if !value.is_finite() || !(0.0..=2.0).contains(&value) {
                return Err(format!("{name} outside 0..2.0: {value}"));
            }
        }
        if !self.mid_duck_depth_db.is_finite() || !(-24.0..=0.0).contains(&self.mid_duck_depth_db) {
            return Err(format!("mid_duck_depth_db outside -24..0 dB: {}", self.mid_duck_depth_db));
        }
        if !self.mid_duck_threshold.is_finite() || !(0.001..=1.0).contains(&self.mid_duck_threshold) {
            return Err(format!("mid_duck_threshold outside 0.001..1.0: {}", self.mid_duck_threshold));
        }
        if !self.suspend_margin_rpm.is_finite() || !(0.0..=5_000.0).contains(&self.suspend_margin_rpm) {
            return Err(format!("suspend_margin_rpm outside 0..5000: {}", self.suspend_margin_rpm));
        }
        if !self.max_fade_start_rpm.is_finite()
            || !self.max_full_rpm.is_finite()
            || self.max_fade_start_rpm < 1_000.0
            || self.max_full_rpm > 25_000.0
            || self.max_full_rpm - self.max_fade_start_rpm < 500.0
        {
            return Err("invalid max-zone RPM crossfade window".into());
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SampleLayerFrame {
    pub tonal: f32,
    pub residual: f32,
    pub mid_bus: f32,
    pub max_rasp: f32,
    pub off_throttle: f32,
    pub output: f32,
    pub zone_weights: [f32; ZONE_COUNT],
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

struct TrackingNotch {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl TrackingNotch {
    fn new() -> Self {
        Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32, frequency_hz: f32, sample_rate: f32, wet: f32) -> f32 {
        if wet <= 0.0 || frequency_hz >= sample_rate * 0.45 {
            return input;
        }
        let omega = std::f32::consts::TAU * frequency_hz / sample_rate;
        let alpha = omega.sin() / (2.0 * 10.0);
        let inverse_a0 = 1.0 / (1.0 + alpha);
        let b0 = inverse_a0;
        let b1 = -2.0 * omega.cos() * inverse_a0;
        let b2 = inverse_a0;
        let a1 = b1;
        let a2 = (1.0 - alpha) * inverse_a0;
        let filtered = b0 * input + b1 * self.x1 + b2 * self.x2 - a1 * self.y1 - a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = filtered;
        input + (filtered - input) * wet
    }
}

struct ZoneMidProcessor {
    tonal_low: OnePoleLowPass,
    tonal_high: OnePoleLowPass,
    residual_low: OnePoleLowPass,
    residual_high: OnePoleLowPass,
    rasp_tonal_low: OnePoleLowPass,
    rasp_tonal_high: OnePoleLowPass,
    rasp_residual_low: OnePoleLowPass,
    rasp_residual_high: OnePoleLowPass,
    tonal_gain: f32,
    residual_gain: f32,
    compressor_envelope: f32,
    compressor_attack: f32,
    compressor_release: f32,
    compressor_threshold: f32,
    compressor_parallel_mix: f32,
    saturation_drive: f32,
    saturation_mix: f32,
    order_five_wet: f32,
    rasp_gain: f32,
    zone_tonal_gain: f32,
    zone_residual_gain: f32,
    order_five_notch: TrackingNotch,
}

impl ZoneMidProcessor {
    fn new(zone: usize, sample_rate: f32) -> Self {
        let (
            low_hz,
            high_hz,
            tonal_db,
            residual_db,
            order_five_wet,
            rasp_db,
            zone_tonal_db,
            zone_residual_db,
        ) = match zone {
            0 => (250.0, 900.0, 1.0, 4.0, 0.0, 0.0, 0.0, 3.0),
            1 => (350.0, 1_800.0, 1.5, 3.0, 0.0, 0.0, 2.5, 3.5),
            _ => (500.0, 2_500.0, 0.75, 2.5, 0.30, 2.5, 2.5, 3.0),
        };
        Self {
            tonal_low: OnePoleLowPass::new(low_hz, sample_rate),
            tonal_high: OnePoleLowPass::new(high_hz, sample_rate),
            residual_low: OnePoleLowPass::new(low_hz, sample_rate),
            residual_high: OnePoleLowPass::new(high_hz, sample_rate),
            rasp_tonal_low: OnePoleLowPass::new(1_800.0, sample_rate),
            rasp_tonal_high: OnePoleLowPass::new(6_500.0, sample_rate),
            rasp_residual_low: OnePoleLowPass::new(1_800.0, sample_rate),
            rasp_residual_high: OnePoleLowPass::new(6_500.0, sample_rate),
            tonal_gain: 10.0f32.powf(tonal_db / 20.0),
            residual_gain: 10.0f32.powf(residual_db / 20.0),
            compressor_envelope: 0.0,
            compressor_attack: 1.0 - (-1.0 / (0.025 * sample_rate)).exp(),
            compressor_release: 1.0 - (-1.0 / (0.120 * sample_rate)).exp(),
            compressor_threshold: if zone == 2 { 0.0275 } else { 0.040 },
            compressor_parallel_mix: if zone == 2 { 0.425 } else { 0.30 },
            saturation_drive: if zone == 2 { 1.7 } else { 1.4 },
            saturation_mix: if zone == 2 { 0.275 } else { 0.20 },
            order_five_wet,
            rasp_gain: 10.0f32.powf(rasp_db / 20.0),
            zone_tonal_gain: 10.0f32.powf(zone_tonal_db / 20.0),
            zone_residual_gain: 10.0f32.powf(zone_residual_db / 20.0),
            order_five_notch: TrackingNotch::new(),
        }
    }

    #[inline]
    fn process(
        &mut self,
        tonal: f32,
        residual: f32,
        rpm: f32,
        sample_rate: f32,
    ) -> (f32, f32, f32, f32, f32, f32) {
        let tonal_band = self.tonal_high.process(tonal) - self.tonal_low.process(tonal);
        let residual_band =
            self.residual_high.process(residual) - self.residual_low.process(residual);
        let tonal_rasp = self.rasp_tonal_high.process(tonal) - self.rasp_tonal_low.process(tonal);
        let residual_rasp =
            self.rasp_residual_high.process(residual) - self.rasp_residual_low.process(residual);
        let tonal_shaped =
            tonal + tonal_band * (self.tonal_gain - 1.0) + tonal_rasp * (self.rasp_gain - 1.0);
        let tonal_shaped = self.order_five_notch.process(
            tonal_shaped,
            rpm / 12.0,
            sample_rate,
            self.order_five_wet,
        );

        let level = residual_band.abs();
        let envelope_rate = if level > self.compressor_envelope {
            self.compressor_attack
        } else {
            self.compressor_release
        };
        self.compressor_envelope += envelope_rate * (level - self.compressor_envelope);
        let compressed_gain = if self.compressor_envelope > self.compressor_threshold {
            (self.compressor_threshold / self.compressor_envelope).sqrt()
        } else {
            1.0
        };
        let boosted_band = residual_band * self.residual_gain;
        let parallel_band = boosted_band
            * ((1.0 - self.compressor_parallel_mix)
                + self.compressor_parallel_mix * compressed_gain * 1.25);
        let saturated_band =
            (parallel_band * self.saturation_drive).tanh() / self.saturation_drive.tanh();
        let processed_band =
            parallel_band * (1.0 - self.saturation_mix) + saturated_band * self.saturation_mix;
        let residual_shaped =
            residual - residual_band + processed_band + residual_rasp * (self.rasp_gain - 1.0);
        (
            tonal_shaped * self.zone_tonal_gain,
            residual_shaped * self.zone_residual_gain,
            tonal_band * self.zone_tonal_gain,
            processed_band * self.zone_residual_gain,
            if self.rasp_gain > 1.0 {
                tonal_rasp * self.rasp_gain * self.zone_tonal_gain
            } else {
                0.0
            },
            if self.rasp_gain > 1.0 {
                residual_rasp * self.rasp_gain * self.zone_residual_gain
            } else {
                0.0
            },
        )
    }
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
        if !(8_000..=192_000).contains(&metadata.sample_rate) {
            return Err(format!(
                "sample-layer source rate out of range in {}: {}",
                metadata_path.display(),
                metadata.sample_rate
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
        self.advance(rpm, output_sample_rate);
        (tonal, residual)
    }

    /// Playback ratio in source samples per output sample.
    #[inline]
    fn rate(&self, rpm: f32, output_sample_rate: u32) -> f64 {
        rpm as f64 / self.rpm_anchor as f64 * self.source_sample_rate as f64
            / output_sample_rate as f64
    }

    /// Maximum ratio permitted by the sample-layer input contract at this
    /// output rate.  Keeping this derived from the asset metadata makes the
    /// antialias table valid for every supported output rate, including the
    /// 26.9x case for a 44.1 kHz asset rendered at 8 kHz.
    #[inline]
    fn max_rate(&self, output_sample_rate: u32) -> f64 {
        25_000.0 / self.rpm_anchor as f64 * self.source_sample_rate as f64
            / output_sample_rate as f64
    }

    /// AUD-07: resample through a precomputed table, then advance with the
    /// caller-supplied ratio (computed once per zone per sample in `process`).
    #[inline]
    fn render_tabled(
        &mut self,
        rate: f64,
        table: &[f32],
        table_phases: usize,
        tap_lo: isize,
        taps: usize,
    ) -> (f32, f32) {
        let tonal = sinc_sample_tabled(&self.tonal, self.cursor, table, table_phases, tap_lo, taps);
        let residual =
            sinc_sample_tabled(&self.residual, self.cursor, table, table_phases, tap_lo, taps);
        self.advance_with_rate(rate);
        (tonal, residual)
    }

    #[inline]
    fn render_antialiased_tabled(
        &mut self,
        rate: f64,
        table: &[f32],
        table_phases: usize,
        ratio_levels: usize,
        ratio_max: f64,
        tap_lo: isize,
        taps: usize,
    ) -> (f32, f32) {
        let tonal = sinc_sample_antialiased_tabled(
            &self.tonal,
            self.cursor,
            rate,
            table,
            table_phases,
            ratio_levels,
            ratio_max,
            tap_lo,
            taps,
        );
        let residual = sinc_sample_antialiased_tabled(
            &self.residual,
            self.cursor,
            rate,
            table,
            table_phases,
            ratio_levels,
            ratio_max,
            tap_lo,
            taps,
        );
        self.advance_with_rate(rate);
        (tonal, residual)
    }

    /// AUD-06: advances the loop cursor without resampling, so a suspended
    /// zone keeps its mechanical phase and resumes sample-aligned. Uses the
    /// identical rate formula as `render`, keeping a single phase authority.
    #[inline]
    fn advance(&mut self, rpm: f32, output_sample_rate: u32) {
        self.advance_with_rate(self.rate(rpm, output_sample_rate));
    }

    /// Cursor advance for an already-computed ratio (AUD-07 callers).
    #[inline]
    fn advance_with_rate(&mut self, rate: f64) {
        self.cursor = (self.cursor + rate).rem_euclid(self.tonal.len() as f64);
    }
}

pub struct ThreeZoneSampleLayer {
    output_sample_rate: u32,
    config: ThreeZoneSampleLayerConfig,
    zones: [SampleZone; ZONE_COUNT],
    mid_processors: [ZoneMidProcessor; ZONE_COUNT],
    off_zone: Option<SampleZone>,
    phase_aligned: bool,
    /// AUD-06: count of suspended (cursor-only) zone renders since construction.
    /// Control-thread diagnostic for measuring skipped work; written only on the
    /// audio thread, never allocated, locked, or logged in the callback.
    suspended_zone_samples: u64,
    /// AUD-07a: precomputed radius-4 kernel (transparent accelerator).
    tabled_narrow: Vec<f32>,
    /// AUD-07: ratio-aware pre-resampling kernels. Layout is
    /// `[ratio_level][phase][tap]`; rows interpolate both dimensions in the
    /// callback. The first ratio row is the radius-4 table padded into the
    /// radius-16 footprint, making ratio 1 exactly continuous with the
    /// transparent path.
    aa_table: Vec<f32>,
    aa_ratio_max: f64,
    off_smoothed: f32,
}

impl ThreeZoneSampleLayer {
    pub fn load_directory(
        output_sample_rate: u32,
        directory: &Path,
        config: ThreeZoneSampleLayerConfig,
    ) -> Result<Self, String> {
        let entries = fs::read_dir(directory)
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let off_path = entries.iter().find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".off-layer.json"))
        }).cloned();
        let mut metadata_paths: Vec<PathBuf> = entries
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".sample-layer.json"))
            })
            .collect();
        metadata_paths.sort();
        let metadata_paths: [PathBuf; ZONE_COUNT] =
            metadata_paths.try_into().map_err(|paths: Vec<_>| {
                format!(
                    "ThreeZoneSampleLayer expected 3 metadata files in {}, found {}",
                    directory.display(),
                    paths.len()
                )
            })?;
        let off_zone = off_path.as_deref().map(SampleZone::load).transpose()?;
        Self::load_with_off(output_sample_rate, metadata_paths, off_zone, config)
    }

    pub fn load(
        output_sample_rate: u32,
        metadata_paths: [PathBuf; ZONE_COUNT],
        config: ThreeZoneSampleLayerConfig,
    ) -> Result<Self, String> {
        Self::load_with_off(output_sample_rate, metadata_paths, None, config)
    }

    fn load_with_off(
        output_sample_rate: u32,
        metadata_paths: [PathBuf; ZONE_COUNT],
        off_zone: Option<SampleZone>,
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
        if config.max_fade_start_rpm < loaded[1].rpm_anchor
            || config.max_full_rpm > loaded[2].rpm_anchor
        {
            return Err("max-zone crossfade window must stay between med and max anchors".into());
        }
        let zones: [SampleZone; ZONE_COUNT] = loaded
            .try_into()
            .map_err(|_| "ThreeZoneSampleLayer requires exactly three zones".to_string())?;
        let aa_ratio_max = zones
            .iter()
            .map(|zone| zone.max_rate(output_sample_rate))
            .chain(off_zone.iter().map(|zone| zone.max_rate(output_sample_rate)))
            .fold(1.0, f64::max);
        Ok(Self {
            output_sample_rate,
            config,
            zones,
            mid_processors: std::array::from_fn(|zone| {
                ZoneMidProcessor::new(zone, output_sample_rate as f32)
            }),
            off_zone,
            phase_aligned: false,
            suspended_zone_samples: 0,
            tabled_narrow: build_sinc_table(SINC_TABLE_PHASES, SINC_RADIUS),
            aa_table: {
                if config.pitch_up_antialias {
                    build_antialias_table(SINC_TABLE_PHASES, AA_RATIO_LEVELS, aa_ratio_max)
                } else {
                    Vec::new()
                }
            },
            aa_ratio_max,
            off_smoothed: 0.0,
        })
    }

    pub fn reset_phase(&mut self, crank_phase_deg: f32) -> Result<(), String> {
        if !crank_phase_deg.is_finite() {
            return Err("sample-layer crank phase must be finite".into());
        }
        for zone in &mut self.zones {
            zone.align(crank_phase_deg);
        }
        if let Some(off_zone) = &mut self.off_zone {
            off_zone.align(crank_phase_deg);
        }
        self.off_smoothed = 0.0;
        self.phase_aligned = true;
        Ok(())
    }

    pub fn rpm_anchors(&self) -> [f32; ZONE_COUNT] {
        std::array::from_fn(|index| self.zones[index].rpm_anchor)
    }

    /// AUD-06 diagnostic: zone renders skipped (cursor advanced, no DSP) since
    /// construction. Read between blocks, never in the callback.
    pub fn suspended_zone_samples(&self) -> u64 {
        self.suspended_zone_samples
    }

    #[inline]
    pub fn process(&mut self, input: SampleLayerInput) -> Result<SampleLayerFrame, String> {
        let input = input.validate()?;
        if !self.phase_aligned {
            self.reset_phase(input.crank_phase_deg)?;
        }
        let weights = zone_weights(
            input.rpm,
            self.rpm_anchors(),
            self.config.max_fade_start_rpm,
            self.config.max_full_rpm,
        );
        let mut tonal = 0.0;
        let mut residual = 0.0;
        let mut tonal_mid = 0.0;
        let mut residual_mid = 0.0;
        let mut tonal_rasp = 0.0;
        let mut residual_rasp = 0.0;
        let output_sample_rate = self.output_sample_rate;
        let suspend = self.config.suspend_inaudible_zones;
        let margin = self.config.suspend_margin_rpm;
        let tabled = self.config.use_tabled_sinc;
        let antialias = self.config.pitch_up_antialias;
        let anchors = self.rpm_anchors();
        let fade_start = self.config.max_fade_start_rpm;
        let fade_full = self.config.max_full_rpm;
        for (index, (zone, weight)) in self.zones.iter_mut().zip(weights).enumerate() {
            // AUD-06 (fix.txt): suspend only in deep silence — zero weight AND
            // the RPM a full margin outside every crossfade this zone takes
            // part in. Zones approaching a fade keep rendering so their filters
            // track and are converged at fade entry; the fade itself then
            // suppresses any residual. Cursor always advances (phase).
            let deep_silence = weight == 0.0
                && match index {
                    0 => input.rpm > anchors[1] + margin,
                    1 => {
                        input.rpm < anchors[0] - margin
                            || (input.rpm > anchors[1] + margin
                                && input.rpm < fade_start - margin)
                            || input.rpm > fade_full + margin
                    }
                    _ => input.rpm < fade_start - margin,
                };
            if suspend && deep_silence {
                zone.advance(input.rpm, output_sample_rate);
                self.suspended_zone_samples += 1;
                continue;
            }
            let rate = zone.rate(input.rpm, output_sample_rate);
            // AUD-07: the antialias kernel is applied inside the resampler,
            // before the output sample is produced. The ratio-aware table is
            // selected continuously, so there is no post-resample LP state
            // and no transient when a ratio crosses one.
            let (zone_tonal, zone_residual) = if !tabled {
                zone.render(input.rpm, output_sample_rate)
            } else if antialias && rate > 1.0 {
                zone.render_antialiased_tabled(
                    rate,
                    &self.aa_table,
                    SINC_TABLE_PHASES,
                    AA_RATIO_LEVELS,
                    self.aa_ratio_max,
                    -AA_SINC_RADIUS + 1,
                    AA_SINC_TAPS,
                )
            } else {
                zone.render_tabled(
                    rate,
                    &self.tabled_narrow,
                    SINC_TABLE_PHASES,
                    -SINC_RADIUS + 1,
                    (2 * SINC_RADIUS) as usize,
                )
            };
            let (
                zone_tonal,
                zone_residual,
                zone_tonal_mid,
                zone_residual_mid,
                zone_tonal_rasp,
                zone_residual_rasp,
            ) = self.mid_processors[index].process(
                zone_tonal,
                zone_residual,
                input.rpm,
                self.output_sample_rate as f32,
            );
            tonal += zone_tonal * weight;
            residual += zone_residual * weight;
            tonal_mid += zone_tonal_mid * weight;
            residual_mid += zone_residual_mid * weight;
            tonal_rasp += zone_tonal_rasp * weight;
            residual_rasp += zone_residual_rasp * weight;
        }
        let charge = input.load * (0.35 + 0.65 * input.throttle);
        let tonal_gain = self.config.tonal_gain_closed
            + (self.config.tonal_gain_loaded - self.config.tonal_gain_closed) * charge;
        let residual_gain = self.config.residual_gain_closed
            + (self.config.residual_gain_loaded - self.config.residual_gain_closed) * charge;
        tonal *= tonal_gain;
        residual *= residual_gain;
        let mid_bus = tonal_mid * tonal_gain + residual_mid * residual_gain;
        let max_rasp = tonal_rasp * tonal_gain + residual_rasp * residual_gain;

        let mut off_throttle_stem = 0.0;
        if let Some(off_zone) = &mut self.off_zone {
            let off_weight = off_throttle_weight(input, self.config.off_throttle_gain);
            // fix.txt: slew the weight itself (32 samples) so a 0 -> audible
            // jump on a throttle slam cannot click against filter state.
            // Steady weights pass through untouched after the slew.
            let target = off_weight;
            let delta = (target - self.off_smoothed).clamp(-1.0 / 32.0, 1.0 / 32.0);
            self.off_smoothed += delta;
            let off_weight = self.off_smoothed;
            // AUD-06: same suspension for the off stem (stateless beyond its
            // cursor, so reactivation is exact).
            let (off_tonal, off_residual) = if suspend && off_weight == 0.0 {
                off_zone.advance(input.rpm, output_sample_rate);
                self.suspended_zone_samples += 1;
                (0.0, 0.0)
            } else if !tabled {
                off_zone.render(input.rpm, output_sample_rate)
            } else {
                let rate = off_zone.rate(input.rpm, output_sample_rate);
                let (raw_tonal, raw_residual) = if antialias && rate > 1.0 {
                    off_zone.render_antialiased_tabled(
                        rate,
                        &self.aa_table,
                        SINC_TABLE_PHASES,
                        AA_RATIO_LEVELS,
                        self.aa_ratio_max,
                        -AA_SINC_RADIUS + 1,
                        AA_SINC_TAPS,
                    )
                } else {
                    off_zone.render_tabled(
                        rate,
                        &self.tabled_narrow,
                        SINC_TABLE_PHASES,
                        -SINC_RADIUS + 1,
                        (2 * SINC_RADIUS) as usize,
                    )
                };
                (raw_tonal, raw_residual)
            };
            tonal += off_tonal * off_weight;
            residual += off_residual * off_weight;
            off_throttle_stem = (off_tonal + off_residual) * off_weight;
        }

        Ok(SampleLayerFrame {
            tonal,
            residual,
            mid_bus,
            max_rasp,
            off_throttle: off_throttle_stem,
            output: tonal + residual,
            zone_weights: weights,
        })
    }
}

#[inline]
fn off_throttle_weight(input: SampleLayerInput, gain: f32) -> f32 {    let overrun = ((0.30 - input.throttle) / 0.30).clamp(0.0, 1.0);
    let low_load = ((0.40 - input.load) / 0.40).clamp(0.0, 1.0);
    // A high absolute clutch load can still be engine retention. Use signed
    // torque to preserve that off character. Both the low-load coast term and
    // the retention term require an engaged clutch: a coherent free-rev packet
    // from physics carries clutch=0/load=0, so it must stay silent in this
    // layer instead of falling back to low_load=1. TC is already represented
    // by the physical load packet and is deliberately not reapplied.
    let retention = (-input.normalized_engine_torque).max(0.0) * input.clutch_engagement;
    let low_load_engaged = low_load * input.clutch_engagement;
    overrun * low_load_engaged.max(retention) * gain
}

#[inline]
fn zone_weights(
    rpm: f32,
    anchors: [f32; ZONE_COUNT],
    max_fade_start_rpm: f32,
    max_full_rpm: f32,
) -> [f32; ZONE_COUNT] {
    if rpm <= anchors[0] {
        return [1.0, 0.0, 0.0];
    }
    if rpm < anchors[1] {
        let t = ((rpm - anchors[0]) / (anchors[1] - anchors[0])).clamp(0.0, 1.0);
        let theta = t * std::f32::consts::FRAC_PI_2;
        return [theta.cos(), theta.sin(), 0.0];
    }
    if rpm < max_fade_start_rpm {
        return [0.0, 1.0, 0.0];
    }
    if rpm < max_full_rpm {
        let t = ((rpm - max_fade_start_rpm) / (max_full_rpm - max_fade_start_rpm)).clamp(0.0, 1.0);
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

/// AUD-07a: windowed-sinc weight for one tap, factored so tables and the
/// reference evaluator share the exact kernel definition.
fn windowed_sinc_weight(distance: f64, radius: isize) -> f64 {
    let sinc = if distance.abs() < 1.0e-12 {
        1.0
    } else {
        (PI * distance).sin() / (PI * distance)
    };
    let normalized = distance / radius as f64;
    let window = if normalized.abs() <= 1.0 {
        0.5 + 0.5 * (PI * normalized).cos()
    } else {
        0.0
    };
    sinc * window
}

/// AUD-07a: precomputes one normalized kernel table row per fractional phase.
/// Entry layout is `[phase * taps + tap]`; each phase row sums to one, baking
/// in the reference's per-call `weight_sum` division. An extra closing row
/// holds the phase-1024 weights (identical to phase 0 one sample later, i.e.
/// taps shifted by one with a zero entering): interpolating toward it at the
/// wrap seam keeps tap alignment exact instead of blending toward the wrong
/// row. Built once on the control thread at layer construction; the callback
/// only reads the table.
fn build_sinc_table(phases: usize, radius: isize) -> Vec<f32> {
    let taps = (2 * radius) as usize;
    let tap_lo = -radius + 1;
    let mut table = vec![0.0f32; (phases + 1) * taps];
    for phase in 0..=phases {
        let fraction = phase as f64 / phases as f64;
        let mut sum = 0.0f64;
        for tap in 0..taps {
            // Beyond the seam the kernel is evaluated one sample later:
            // offset k at fraction 1 equals offset k-1 at fraction 0, and the
            // tap sliding out of the window is exactly zero.
            let weight = if phase < phases {
                let distance = (tap_lo + tap as isize) as f64 - fraction;
                windowed_sinc_weight(distance, radius)
            } else if tap > 0 {
                let distance = (tap_lo + tap as isize - 1) as f64;
                windowed_sinc_weight(distance, radius)
            } else {
                0.0
            };
            table[phase * taps + tap] = weight as f32;
            sum += weight;
        }
        let scale = 1.0 / sum.max(1.0e-12);
        for tap in 0..taps {
            table[phase * taps + tap] = (table[phase * taps + tap] as f64 * scale) as f32;
        }
    }
    table
}

/// Builds a ratio-aware table whose cutoff is `1/ratio` in source-sample
/// frequency. The ratio coordinate is `1 - cutoff`, so it is dense near
/// ratio 1 while still reaching the exact validated maximum without a
/// logarithm in the callback. Every phase row is normalized for unity DC.
fn build_antialias_table(phases: usize, ratio_levels: usize, ratio_max: f64) -> Vec<f32> {
    let ratio_max = ratio_max.max(1.0).min(1.0e6);
    let mut table = vec![0.0f32; (ratio_levels + 1) * (phases + 1) * AA_SINC_TAPS];
    let narrow = build_sinc_table(phases, SINC_RADIUS);
    for ratio_level in 0..=ratio_levels {
        let q = ratio_level as f64 / ratio_levels as f64;
        let cutoff = 1.0 - q * (1.0 - 1.0 / ratio_max);
        for phase in 0..=phases {
            let row = (ratio_level * (phases + 1) + phase) * AA_SINC_TAPS;
            if ratio_level == 0 {
                // Preserve the exact narrow kernel at the ratio-one boundary.
                let narrow_row = phase * (2 * SINC_RADIUS) as usize;
                for tap in 0..(2 * SINC_RADIUS) as usize {
                    let target = tap + (AA_SINC_RADIUS - SINC_RADIUS) as usize;
                    table[row + target] = narrow[narrow_row + tap];
                }
                continue;
            }
            let fraction = phase as f64 / phases as f64;
            let mut sum = 0.0f64;
            for tap in 0..AA_SINC_TAPS {
                let distance = (-AA_SINC_RADIUS + 1 + tap as isize) as f64 - fraction;
                let sinc = if (distance * cutoff).abs() < 1.0e-12 {
                    1.0
                } else {
                    (PI * distance * cutoff).sin() / (PI * distance * cutoff)
                };
                let normalized = distance / AA_SINC_RADIUS as f64;
                let window = if normalized.abs() <= 1.0 {
                    0.5 + 0.5 * (PI * normalized).cos()
                } else {
                    0.0
                };
                let weight = sinc * window * cutoff;
                table[row + tap] = weight as f32;
                sum += weight;
            }
            let scale = 1.0 / sum.max(1.0e-12);
            for tap in 0..AA_SINC_TAPS {
                table[row + tap] = (table[row + tap] as f64 * scale) as f32;
            }
        }
    }
    table
}

/// AUD-07a: table lookup with linear phase interpolation. No sin/cos/division
/// in the loop — 8 (or 16) lerps + multiply-accumulates per call. The table
/// carries one extra closing row so the wrap seam needs no branch: phase0
/// always has a valid successor with correct tap alignment.
#[inline]
fn sinc_sample_tabled(
    samples: &[f32],
    cursor: f64,
    table: &[f32],
    phases: usize,
    tap_lo: isize,
    taps: usize,
) -> f32 {
    let base = cursor.floor() as isize;
    let position = (cursor - cursor.floor()) * phases as f64;
    let phase0 = position.floor() as usize;
    debug_assert!(phase0 < phases);
    let phase0 = phase0.min(phases - 1);
    let phase1 = phase0 + 1;
    let blend = (position - position.floor()) as f32;
    let row0 = phase0 * taps;
    let row1 = phase1 * taps;
    let mut output = 0.0f32;
    for tap in 0..taps {
        let weight =
            table[row0 + tap] + (table[row1 + tap] - table[row0 + tap]) * blend;
        let index = (base + tap_lo + tap as isize).rem_euclid(samples.len() as isize) as usize;
        output += samples[index] * weight;
    }
    output
}

/// Ratio-aware table lookup. The source loop is periodic, so every tap wraps
/// circularly and no filter state can create a seam at the loop boundary.
/// Only arithmetic, table reads and multiply-adds occur per sample.
#[inline]
fn sinc_sample_antialiased_tabled(
    samples: &[f32],
    cursor: f64,
    rate: f64,
    table: &[f32],
    phases: usize,
    ratio_levels: usize,
    ratio_max: f64,
    tap_lo: isize,
    taps: usize,
) -> f32 {
    let base = cursor.floor() as isize;
    let phase_position = (cursor - cursor.floor()) * phases as f64;
    let phase0 = (phase_position.floor() as usize).min(phases - 1);
    let phase1 = phase0 + 1;
    let phase_blend = (phase_position - phase_position.floor()) as f32;
    let ratio_max = ratio_max.max(1.0);
    let cutoff_coordinate = if ratio_max <= 1.0 {
        0.0
    } else {
        (1.0 - 1.0 / rate.max(1.0)) / (1.0 - 1.0 / ratio_max)
    }
    .clamp(0.0, 1.0);
    let ratio_position = cutoff_coordinate * ratio_levels as f64;
    let ratio0 = (ratio_position.floor() as usize).min(ratio_levels - 1);
    let ratio1 = ratio0 + 1;
    // At the exact maximum `ratio_position == ratio_levels`; retain the
    // endpoint instead of wrapping to the penultimate row with blend zero.
    let ratio_blend = (ratio_position - ratio0 as f64).clamp(0.0, 1.0) as f32;
    let phase_stride = taps;
    let ratio_stride = (phases + 1) * phase_stride;
    let row00 = ratio0 * ratio_stride + phase0 * phase_stride;
    let row01 = ratio0 * ratio_stride + phase1 * phase_stride;
    let row10 = ratio1 * ratio_stride + phase0 * phase_stride;
    let row11 = ratio1 * ratio_stride + phase1 * phase_stride;
    let mut output = 0.0f32;
    for tap in 0..taps {
        let phase0_weight = table[row00 + tap]
            + (table[row01 + tap] - table[row00 + tap]) * phase_blend;
        let phase1_weight = table[row10 + tap]
            + (table[row11 + tap] - table[row10 + tap]) * phase_blend;
        let weight = phase0_weight + (phase1_weight - phase0_weight) * ratio_blend;
        let index = (base + tap_lo + tap as isize).rem_euclid(samples.len() as isize) as usize;
        output += samples[index] * weight;
    }
    output
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
        let anchors = [7_500.0, 8_200.0, 15_000.0];
        for rpm in [7_500.0, 8_750.0, 10_000.0, 12_500.0, 15_000.0] {
            let weights = zone_weights(rpm, anchors, 10_000.0, 13_750.0);
            let power: f32 = weights.iter().map(|weight| weight * weight).sum();
            assert!((power - 1.0).abs() < 1.0e-6);
        }
        assert_eq!(zone_weights(9_999.0, anchors, 10_000.0, 13_750.0)[2], 0.0);
        assert_eq!(
            zone_weights(13_750.0, anchors, 10_000.0, 13_750.0),
            [0.0, 0.0, 1.0]
        );
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

    fn deterministic_source(len: usize) -> Vec<f32> {
        // Pseudo-audio with harmonics + edge detail (no randomness allowed).
        (0..len)
            .map(|n| {
                let t = n as f64;
                (0.5 * (t * 0.11).sin() + 0.3 * (t * 0.031 + 1.0).sin() + 0.2 * (t * 0.53 + 2.0).sin())
                    as f32
            })
            .collect()
    }

    #[test]
    fn tabled_narrow_kernel_matches_reference_across_pitch_ratios() {
        // AUD-07a: pitch low, near one, and high, sweeping RPM both directions.
        // The table must stay transparent w.r.t. the evaluated kernel.
        let source = deterministic_source(4096);
        let table = build_sinc_table(SINC_TABLE_PHASES, SINC_RADIUS);
        let taps = (2 * SINC_RADIUS) as usize;
        let mut max_diff = 0.0f32;
        for rate in [0.35, 0.5, 0.9, 1.0, 1.25, 1.9, 2.4] {
            for direction in [1.0, -1.0] {
                let mut cursor = 123.456f64;
                for _ in 0..4_000 {
                    let reference = sinc_sample(&source, cursor);
                    let tabled = sinc_sample_tabled(
                        &source,
                        cursor,
                        &table,
                        SINC_TABLE_PHASES,
                        -SINC_RADIUS + 1,
                        taps,
                    );
                    assert!(reference.is_finite() && tabled.is_finite());
                    max_diff = max_diff.max((reference - tabled).abs());
                    cursor = (cursor + direction * rate).rem_euclid(source.len() as f64);
                }
            }
        }
        assert!(
            max_diff < 1.0e-4,
            "tabled radius-4 deviates {max_diff} from the evaluated kernel"
        );
    }

    fn fit_amplitude(samples: &[f32], frequency_hz: f64, sample_rate: f64) -> f64 {
        let mut sin_sum = 0.0f64;
        let mut cos_sum = 0.0f64;
        for (n, &value) in samples.iter().enumerate() {
            let angle = 2.0 * std::f64::consts::PI * frequency_hz * n as f64 / sample_rate;
            sin_sum += value as f64 * angle.sin();
            cos_sum += value as f64 * angle.cos();
        }
        2.0 * sin_sum.hypot(cos_sum) / samples.len() as f64
    }

    #[test]
    fn ratio_aware_kernel_suppresses_folded_alias_and_preserves_passband() {
        let source_rate = 44_100.0f64;
        let ratio_max = 26.95f64;
        let aa = build_antialias_table(SINC_TABLE_PHASES, AA_RATIO_LEVELS, ratio_max);
        let narrow = build_sinc_table(SINC_TABLE_PHASES, SINC_RADIUS);
        let render = |tone_hz: f64, rate: f64, antialias: bool| {
            let source: Vec<f32> = (0..65_536)
                .map(|n| {
                    (2.0 * std::f64::consts::PI * tone_hz * n as f64 / source_rate).sin() as f32
                })
                .collect();
            (0..8_192)
                .map(|n| {
                    let cursor = n as f64 * rate;
                    if antialias {
                        sinc_sample_antialiased_tabled(
                            &source,
                            cursor,
                            rate,
                            &aa,
                            SINC_TABLE_PHASES,
                            AA_RATIO_LEVELS,
                            ratio_max,
                            -AA_SINC_RADIUS + 1,
                            AA_SINC_TAPS,
                        )
                    } else {
                        sinc_sample_tabled(
                            &source,
                            cursor,
                            &narrow,
                            SINC_TABLE_PHASES,
                            -SINC_RADIUS + 1,
                            (2 * SINC_RADIUS) as usize,
                        )
                    }
                })
                .collect::<Vec<_>>()
        };
        let rate = 2.0;
        let passband_hz = 3_000.0 * rate;
        let passband = render(3_000.0, rate, true);
        let passband_amp = fit_amplitude(&passband, passband_hz, source_rate);
        assert!(passband_amp > 0.80, "passband amplitude {passband_amp}");

        let source_hz = 19_000.0;
        let folded_hz = (source_rate - source_hz * rate).abs();
        let narrow_alias = fit_amplitude(&render(source_hz, rate, false), folded_hz, source_rate);
        let aa_alias = fit_amplitude(&render(source_hz, rate, true), folded_hz, source_rate);
        assert!(narrow_alias > 0.05, "reference fold must be measurable: {narrow_alias}");
        assert!(
            aa_alias < narrow_alias * 0.35,
            "ratio-aware prefilter fold {aa_alias} vs narrow {narrow_alias}"
        );
    }

    #[test]
    fn ratio_aware_alias_matrix_covers_ratios_and_output_rates() {
        let source_rate = 44_100.0f64;
        let narrow = build_sinc_table(SINC_TABLE_PHASES, SINC_RADIUS);
        for (output_rate, rate, source_hz) in [
            (44_100.0, 1.3, 19_000.0),
            (44_100.0, 2.0, 18_000.0),
            (8_000.0, 10.0, 10_000.0),
            (8_000.0, 26.9, 3_000.0),
        ] {
            let ratio_max = 25_000.0 / 5_122.0 * source_rate / output_rate;
            let aa = build_antialias_table(SINC_TABLE_PHASES, AA_RATIO_LEVELS, ratio_max);
            let tone: Vec<f32> = (0..65_536)
                .map(|n| {
                    (2.0 * std::f64::consts::PI * source_hz * n as f64 / source_rate).sin() as f32
                })
                .collect();
            let raw_hz = source_hz * rate * output_rate / source_rate;
            let folded_hz = (raw_hz % output_rate).min(output_rate - (raw_hz % output_rate));
            let mut aa_out = Vec::with_capacity(8_192);
            let mut narrow_out = Vec::with_capacity(8_192);
            for n in 0..8_192 {
                let cursor = n as f64 * rate;
                aa_out.push(sinc_sample_antialiased_tabled(
                    &tone,
                    cursor,
                    rate,
                    &aa,
                    SINC_TABLE_PHASES,
                    AA_RATIO_LEVELS,
                    ratio_max,
                    -AA_SINC_RADIUS + 1,
                    AA_SINC_TAPS,
                ));
                narrow_out.push(sinc_sample_tabled(
                    &tone,
                    cursor,
                    &narrow,
                    SINC_TABLE_PHASES,
                    -SINC_RADIUS + 1,
                    (2 * SINC_RADIUS) as usize,
                ));
            }
            let narrow_alias = fit_amplitude(&narrow_out, folded_hz, output_rate);
            let aa_alias = fit_amplitude(&aa_out, folded_hz, output_rate);
            assert!(narrow_alias > 0.02, "{output_rate} Hz ratio {rate} fold is not measurable");
            assert!(
                aa_alias < narrow_alias * 0.55,
                "{output_rate} Hz ratio {rate} AA alias {aa_alias} vs narrow {narrow_alias}"
            );
        }
    }

    #[test]
    fn ratio_one_fractional_sweep_is_continuous_and_max_ratio_is_supported() {
        let source = deterministic_source(4096);
        let narrow = build_sinc_table(SINC_TABLE_PHASES, SINC_RADIUS);
        let ratio_max = 26.95f64;
        let aa = build_antialias_table(SINC_TABLE_PHASES, AA_RATIO_LEVELS, ratio_max);
        let mut boundary_delta = 0.0f32;
        for cursor in [0.125, 7.37, 123.456, 2047.875, 4095.25] {
            let reference = sinc_sample_tabled(
                &source,
                cursor,
                &narrow,
                SINC_TABLE_PHASES,
                -SINC_RADIUS + 1,
                (2 * SINC_RADIUS) as usize,
            );
            for rate in [0.9999, 1.0, 1.0001] {
                let value = if rate <= 1.0 {
                    sinc_sample_tabled(
                        &source,
                        cursor,
                        &narrow,
                        SINC_TABLE_PHASES,
                        -SINC_RADIUS + 1,
                        (2 * SINC_RADIUS) as usize,
                    )
                } else {
                    sinc_sample_antialiased_tabled(
                        &source,
                        cursor,
                        rate,
                        &aa,
                        SINC_TABLE_PHASES,
                        AA_RATIO_LEVELS,
                        ratio_max,
                        -AA_SINC_RADIUS + 1,
                        AA_SINC_TAPS,
                    )
                };
                boundary_delta = boundary_delta.max((value - reference).abs());
            }
        }
        assert!(
            boundary_delta < 0.002,
            "ratio-one kernel boundary moved by {boundary_delta}"
        );

        let mut cursor = 123.37f64;
        let mut previous = 0.0f32;
        let mut max_step = 0.0f32;
        let mut max_vs_reference = 0.0f32;
        for pass in 0..2 {
            for n in 0..4_096 {
                let t = n as f64 / 4_095.0;
                let u = if pass == 0 { t } else { 1.0 - t };
                let rate = 0.98 + 0.04 * u;
                let reference = sinc_sample_tabled(
                    &source,
                    cursor,
                    &narrow,
                    SINC_TABLE_PHASES,
                    -SINC_RADIUS + 1,
                    (2 * SINC_RADIUS) as usize,
                );
                let value = if rate <= 1.0 {
                    sinc_sample_tabled(
                        &source,
                        cursor,
                    &narrow,
                    SINC_TABLE_PHASES,
                    -SINC_RADIUS + 1,
                    (2 * SINC_RADIUS) as usize,
                )
            } else {
                sinc_sample_antialiased_tabled(
                    &source,
                    cursor,
                    rate,
                    &aa,
                    SINC_TABLE_PHASES,
                    AA_RATIO_LEVELS,
                    ratio_max,
                    -AA_SINC_RADIUS + 1,
                    AA_SINC_TAPS,
                )
            };
                assert!(value.is_finite(), "rate {rate} output is not finite");
                max_vs_reference = max_vs_reference.max((value - reference).abs());
                if pass != 0 || n != 0 {
                    max_step = max_step.max((value - previous).abs());
                }
                previous = value;
                cursor = (cursor + rate).rem_euclid(source.len() as f64);
            }
        }
        assert!(max_step < 1.0, "ratio sweep step {max_step} indicates a discontinuity");
        assert!(
            max_vs_reference < 0.15,
            "near-one AA deviation {max_vs_reference} exceeds the kernel transition bound"
        );

        let zone = SampleZone {
            rpm_anchor: 5122.0,
            firing_phase_deg: 0.0,
            source_sample_rate: 44_100,
            tonal: vec![0.0; 32],
            residual: vec![0.0; 32],
            cursor: 0.0,
        };
        assert!((zone.max_rate(8_000) - 26.94).abs() < 0.05);
        assert!(zone.max_rate(192_000).is_finite() && zone.max_rate(192_000) > 1.0);
        let sample = sinc_sample_antialiased_tabled(
            &zone.tonal,
            31.75,
            26.9,
            &aa,
            SINC_TABLE_PHASES,
            AA_RATIO_LEVELS,
            zone.max_rate(8_000),
            -AA_SINC_RADIUS + 1,
            AA_SINC_TAPS,
        );
        assert!(sample.is_finite());
        let endpoint_max = zone.max_rate(8_000);
        let endpoint_delta = (0..64)
            .map(|n| {
                let cursor = n as f64 * 0.37 + 0.125;
                let at_max = sinc_sample_antialiased_tabled(
                    &source,
                    cursor,
                    endpoint_max,
                    &aa,
                    SINC_TABLE_PHASES,
                    AA_RATIO_LEVELS,
                    endpoint_max,
                    -AA_SINC_RADIUS + 1,
                    AA_SINC_TAPS,
                );
                let just_below = sinc_sample_antialiased_tabled(
                    &source,
                    cursor,
                    endpoint_max - 1.0e-4,
                    &aa,
                    SINC_TABLE_PHASES,
                    AA_RATIO_LEVELS,
                    endpoint_max,
                    -AA_SINC_RADIUS + 1,
                    AA_SINC_TAPS,
                );
                (at_max - just_below).abs()
            })
            .fold(0.0f32, f32::max);
        assert!(endpoint_delta < 0.01, "ratio maximum endpoint step {endpoint_delta}");
    }

    #[test]
    fn off_weight_preserves_retention_at_fixed_rpm_without_double_tc() {
        let base = SampleLayerInput {
            rpm: 8_000.0,
            throttle: 0.0,
            load: 0.8,
            normalized_engine_torque: -0.8,
            clutch_engagement: 1.0,
            crank_phase_deg: 0.0,
        };
        let retention = off_throttle_weight(base, 0.20);
        assert!((retention - 0.16).abs() < 1.0e-6);

        // Coherent free-rev mapping from physics: clutch open implies load=0.
        // Must stay silent instead of falling back to low_load=1 (R1).
        let free_rev = SampleLayerInput {
            rpm: 8_000.0,
            throttle: 0.0,
            load: 0.0,
            normalized_engine_torque: 0.0,
            clutch_engagement: 0.0,
            crank_phase_deg: 0.0,
        };
        assert_eq!(off_throttle_weight(free_rev, 0.20), 0.0);

        // The legacy incoherent combination (clutch=0 with high load) also
        // stays silent because both terms are clutch-gated.
        let mut incoherent = base;
        incoherent.clutch_engagement = 0.0;
        assert_eq!(off_throttle_weight(incoherent, 0.20), 0.0);

        // TC is already reflected by the load supplied by physics. Changing
        // only an external TC control must not attenuate this off weight again.
        let mut same_state = base;
        let no_tc = off_throttle_weight(same_state, 0.20);
        same_state.load = 0.48;
        let tc_cut_load = off_throttle_weight(same_state, 0.20);
        assert_eq!(no_tc, tc_cut_load);
    }

    #[test]
    fn off_weight_silences_coherent_free_rev_but_keeps_low_load_coast() {
        // Free-rev: closed throttle with clutch open, no delivered load.
        // Overrun alone must not open the stem without clutch engagement.
        let free_rev = SampleLayerInput {
            rpm: 9_000.0,
            throttle: 0.0,
            load: 0.0,
            normalized_engine_torque: 0.0,
            clutch_engagement: 0.0,
            crank_phase_deg: 0.0,
        };
        assert_eq!(off_throttle_weight(free_rev, 0.20), 0.0);

        // Same throttle/load with clutch engaged keeps low-load coast audible.
        let coast = SampleLayerInput {
            clutch_engagement: 1.0,
            ..free_rev
        };
        assert!(off_throttle_weight(coast, 0.20) > 0.0);

        // Partial clutch scales the low-load term proportionally.
        let half = SampleLayerInput {
            clutch_engagement: 0.5,
            ..coast
        };
        let full = off_throttle_weight(coast, 0.20);
        let partial = off_throttle_weight(half, 0.20);
        assert!((partial - full * 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn off_weight_uses_low_load_when_torque_is_positive() {
        let input = SampleLayerInput {
            rpm: 8_000.0,
            throttle: 0.0,
            load: 0.2,
            normalized_engine_torque: 0.5,
            clutch_engagement: 1.0,
            crank_phase_deg: 0.0,
        };
        assert!((off_throttle_weight(input, 0.20) - 0.10).abs() < 1.0e-6);
    }

    #[test]
    fn ratio_aware_antialias_program_transient_stays_bounded_without_clicks() {
        // AUD-07: pre-resampling ratio-aware kernels vs the reference path on
        // program material across both crossfades up/down with power and
        // retention. Bounds the top-end effect; timbre remains a human gate.
        use std::path::Path;
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../audio/v10_gf509");
        let base_config = ThreeZoneSampleLayerConfig::default();
        // Keep the reference side explicit so the comparison is observable.
        let mut narrow = ThreeZoneSampleLayer::load_directory(
            44_100,
            &assets,
            ThreeZoneSampleLayerConfig {
                pitch_up_antialias: false,
                ..base_config
            },
        )
        .unwrap();
        let mut aa_layer = ThreeZoneSampleLayer::load_directory(
            44_100,
            &assets,
            ThreeZoneSampleLayerConfig {
                pitch_up_antialias: true,
                ..base_config
            },
        )
        .unwrap();
        let anchors = narrow.rpm_anchors();
        let rpm_lo = (anchors[0] - 2_000.0).max(1_000.0);
        let rpm_hi = anchors[2] + 1_000.0;
        let mut inputs = Vec::new();
        for pass in [0, 1] {
            let steps = 20_000usize;
            for i in 0..steps {
                let u = i as f32 / (steps - 1) as f32;
                let rpm = if pass == 0 {
                    rpm_lo + (rpm_hi - rpm_lo) * u
                } else {
                    rpm_hi + (rpm_lo - rpm_hi) * u
                };
                let power = pass == 0;
                inputs.push(SampleLayerInput {
                    rpm,
                    throttle: if power { 0.9 } else { 0.0 },
                    load: if power { 0.85 } else { 0.1 },
                    normalized_engine_torque: if power { 0.8 } else { -0.45 },
                    clutch_engagement: 1.0,
                    crank_phase_deg: 0.0,
                });
            }
        }
        let mut max_diff = 0.0f32;
        let mut max_step_narrow = 0.0f32;
        let mut max_step_aa = 0.0f32;
        let mut previous_narrow = 0.0f32;
        let mut previous_aa = 0.0f32;
        let mut aa_engaged = false;
        for (n, input) in inputs.iter().enumerate() {
            let out_narrow = narrow.process(*input).unwrap().output;
            let out_aa = aa_layer.process(*input).unwrap().output;
            assert!(out_narrow.is_finite() && out_aa.is_finite(), "sample {n}");
            max_diff = max_diff.max((out_narrow - out_aa).abs());
            if n > 0 {
                max_step_narrow = max_step_narrow.max((out_narrow - previous_narrow).abs());
                max_step_aa = max_step_aa.max((out_aa - previous_aa).abs());
            }
            previous_narrow = out_narrow;
            previous_aa = out_aa;
            aa_engaged |= input.rpm > anchors[0] && input.rpm != 0.0;
        }
        assert!(aa_engaged, "sweep must cross pitch-up ratios");
        assert!(
            max_diff <= 0.05,
            "ratio-aware program deviation {max_diff} exceeds provisional bound"
        );
        assert!(
            max_step_aa <= max_step_narrow * 1.5,
            "AA max step {max_step_aa} vs reference {max_step_narrow}: click?"
        );
    }

    #[test]
    fn tabled_kernel_cost_is_a_fraction_of_evaluated_sinc() {
        // AUD-07a CPU evidence at kernel level: time N resamples through the
        // evaluated kernel vs the tabled one (same trace of cursors/rates).
        // Full-route confirmation comes from aud_path_bench runs.
        let source = deterministic_source(8192);
        let narrow = build_sinc_table(SINC_TABLE_PHASES, SINC_RADIUS);
        let aa = build_antialias_table(SINC_TABLE_PHASES, AA_RATIO_LEVELS, 26.95);
        let narrow_taps = (2 * SINC_RADIUS) as usize;
        let cursors: Vec<f64> = (0..20_000)
            .map(|n| (n as f64 * 1.37).rem_euclid(source.len() as f64))
            .collect();
        let time_it = |work: &mut dyn FnMut()| {
            let mut best = u128::MAX;
            for _ in 0..5 {
                let start = std::time::Instant::now();
                work();
                best = best.min(start.elapsed().as_nanos());
            }
            best
        };
        let mut sink = 0.0f32;
        let reference_ns = time_it(&mut || {
            for cursor in &cursors {
                sink += sinc_sample(&source, *cursor);
            }
        });
        let mut sink_tabled = 0.0f32;
        let tabled_ns = time_it(&mut || {
            for cursor in &cursors {
                sink_tabled += sinc_sample_tabled(
                    &source,
                    *cursor,
                    &narrow,
                    SINC_TABLE_PHASES,
                    -SINC_RADIUS + 1,
                    narrow_taps,
                );
            }
        });
        let mut sink_aa = 0.0f32;
        let aa_ns = time_it(&mut || {
            for cursor in &cursors {
                sink_aa += sinc_sample_antialiased_tabled(
                    &source,
                    *cursor,
                    1.37,
                    &aa,
                    SINC_TABLE_PHASES,
                    AA_RATIO_LEVELS,
                    26.95,
                    -AA_SINC_RADIUS + 1,
                    AA_SINC_TAPS,
                );
            }
        });
        assert!(sink.is_finite() && sink_tabled.is_finite() && sink_aa.is_finite());
        assert!(sink.abs() > 1.0, "sinks must observe real work");
        println!(
            "AUD-07 kernel ns best-of-5 for 20k resamples: reference={reference_ns} tabled={tabled_ns} aa={aa_ns}"
        );
        // Directional lock: the table must cost strictly less than evaluating
        // the kernel. Measured debug ratios live in the println above; the
        // shippable ratio comes from release aud_path_bench runs.
        assert!(
            tabled_ns < reference_ns,
            "tabled {tabled_ns}ns must cost less than evaluated {reference_ns}ns"
        );
    }

    #[test]
    fn tabled_default_stays_transparent_on_program_material() {
        // AUD-07a shipped-default transparency on the real bank: tabled vs
        // evaluated kernel across a full-range sweep (up/down + retention).
        // The antialias package is pinned OFF on BOTH sides to isolate the
        // table lookup itself (measured separately below). Transparency
        // bound 1e-5 (-100 dBFS; measured 7.4e-7 on this sweep).
        use std::path::Path;
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../audio/v10_gf509");
        let base_config = ThreeZoneSampleLayerConfig::default();
        assert!(base_config.use_tabled_sinc, "shipped default must be tabled");
        let mut reference = ThreeZoneSampleLayer::load_directory(
            44_100,
            &assets,
            ThreeZoneSampleLayerConfig {
                use_tabled_sinc: false,
                pitch_up_antialias: false,
                ..base_config
            },
        )
        .unwrap();
        let mut tabled = ThreeZoneSampleLayer::load_directory(
            44_100,
            &assets,
            ThreeZoneSampleLayerConfig {
                pitch_up_antialias: false,
                ..base_config
            },
        )
        .unwrap();
        let anchors = reference.rpm_anchors();
        let rpm_lo = (anchors[0] - 2_000.0).max(1_000.0);
        let rpm_hi = anchors[2] + 1_000.0;
        let mut max_diff = 0.0f32;
        let mut max_step_reference = 0.0f32;
        let mut max_step_tabled = 0.0f32;
        let mut previous_reference = 0.0f32;
        let mut previous_tabled = 0.0f32;
        for pass in [0, 1] {
            let steps = 20_000usize;
            for i in 0..steps {
                let u = i as f32 / (steps - 1) as f32;
                let rpm = if pass == 0 {
                    rpm_lo + (rpm_hi - rpm_lo) * u
                } else {
                    rpm_hi + (rpm_lo - rpm_hi) * u
                };
                let power = pass == 0;
                let input = SampleLayerInput {
                    rpm,
                    throttle: if power { 0.9 } else { 0.0 },
                    load: if power { 0.85 } else { 0.1 },
                    normalized_engine_torque: if power { 0.8 } else { -0.45 },
                    clutch_engagement: 1.0,
                    crank_phase_deg: 0.0,
                };
                let out_reference = reference.process(input).unwrap().output;
                let out_tabled = tabled.process(input).unwrap().output;
                assert!(out_reference.is_finite() && out_tabled.is_finite());
                max_diff = max_diff.max((out_reference - out_tabled).abs());
                if i > 0 || pass > 0 {
                    max_step_reference =
                        max_step_reference.max((out_reference - previous_reference).abs());
                    max_step_tabled =
                        max_step_tabled.max((out_tabled - previous_tabled).abs());
                }
                previous_reference = out_reference;
                previous_tabled = out_tabled;
            }
        }
        // Transparency bound 1e-5 (-100 dBFS; measured 7.4e-7 on this sweep).
        assert!(
            max_diff <= 1.0e-5,
            "tabled program deviation {max_diff} exceeds transparency bound"
        );
        assert!(
            max_step_tabled <= max_step_reference * 1.5,
            "tabled max step {max_step_tabled} vs reference {max_step_reference}: click?"
        );
    }

    #[test]
    fn suspended_zones_keep_phase_and_reactivate_without_clicks() {
        // AUD-06: A/B against the packaged bank. The suspended layer must skip
        // real work, keep every cursor bit-identical (phase continuity), and
        // reactivate across both crossfades (up/down), throttle swings, and
        // retention without introducing clicks. Proposed audibility criterion
        // (not an approved threshold): reactivation transient peak <= -40 dBFS
        // (0.01) and no new sample step above the reference's own maximum.
        use std::path::Path;
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../audio/v10_gf509");
        let base_config = ThreeZoneSampleLayerConfig::default();
        // fix.txt: the reference must render every zone every sample, or the
        // A/B compares identical behavior and cannot catch reactivation issues.
        let mut reference = ThreeZoneSampleLayer::load_directory(
            44_100,
            &assets,
            ThreeZoneSampleLayerConfig {
                suspend_inaudible_zones: false,
                ..base_config
            },
        )
        .unwrap();
        let mut suspended = ThreeZoneSampleLayer::load_directory(
            44_100,
            &assets,
            ThreeZoneSampleLayerConfig {
                suspend_inaudible_zones: true,
                ..base_config
            },
        )
        .unwrap();
        assert!(suspended.off_zone.is_some(), "bank must carry an off stem");
        let anchors = reference.rpm_anchors();
        let rpm_lo = (anchors[0] - 2_000.0).max(1_000.0);
        let rpm_hi = anchors[2] + 1_000.0;

        // Identical drive: power sweep up, retention coast down, free-rev.
        let mut inputs = Vec::new();
        let up = 20_000usize;
        for i in 0..up {
            let u = i as f32 / (up - 1) as f32;
            inputs.push(SampleLayerInput {
                rpm: rpm_lo + (rpm_hi - rpm_lo) * u,
                throttle: 0.9,
                load: 0.85,
                normalized_engine_torque: 0.8,
                clutch_engagement: 1.0,
                crank_phase_deg: 0.0,
            });
        }
        let down = 20_000usize;
        for i in 0..down {
            let u = i as f32 / (down - 1) as f32;
            inputs.push(SampleLayerInput {
                rpm: rpm_hi + (rpm_lo - rpm_hi) * u,
                throttle: 0.0,
                load: 0.1,
                normalized_engine_torque: -0.45,
                clutch_engagement: 1.0,
                crank_phase_deg: 0.0,
            });
        }
        let free = 10_000usize;
        for i in 0..free {
            let u = i as f32 / (free - 1) as f32;
            let rpm = rpm_lo + (9_000.0 - rpm_lo) * (2.0 * u - 1.0).abs();
            inputs.push(SampleLayerInput {
                rpm,
                throttle: 0.5 * (1.0 - u),
                load: 0.0,
                normalized_engine_torque: 0.0,
                clutch_engagement: 0.0,
                crank_phase_deg: 0.0,
            });
        }

        let mut max_diff = 0.0f32;
        let mut max_step_reference = 0.0f32;
        let mut max_step_suspended = 0.0f32;
        let mut previous_reference = 0.0f32;
        let mut previous_suspended = 0.0f32;
        let mut zone_weights_seen_zero = [false; ZONE_COUNT];
        for (n, input) in inputs.iter().enumerate() {
            let out_reference = reference.process(*input).unwrap().output;
            let out_suspended = suspended.process(*input).unwrap().output;
            assert!(out_reference.is_finite() && out_suspended.is_finite(), "block {n}");
            max_diff = max_diff.max((out_reference - out_suspended).abs());
            if n > 0 {
                max_step_reference =
                    max_step_reference.max((out_reference - previous_reference).abs());
                max_step_suspended =
                    max_step_suspended.max((out_suspended - previous_suspended).abs());
            }
            previous_reference = out_reference;
            previous_suspended = out_suspended;
            let weights = zone_weights(
                input.rpm,
                anchors,
                base_config.max_fade_start_rpm,
                base_config.max_full_rpm,
            );
            for (seen, weight) in zone_weights_seen_zero.iter_mut().zip(weights) {
                *seen |= weight == 0.0;
            }
        }
        assert!(
            zone_weights_seen_zero.iter().all(|seen| *seen),
            "sweep must park every zone at zero weight at some point"
        );
        let skipped = suspended.suspended_zone_samples();
        let total_zone_renders = inputs.len() as u64 * (ZONE_COUNT as u64 + 1);
        let skip_ratio = skipped as f64 / total_zone_renders as f64;
        assert!(skipped > 0, "suspension must skip real work");
        assert!(
            skip_ratio > 0.20,
            "skip ratio {skip_ratio:.3} must clear 20% on a full-range sweep"
        );
        // Phase continuity: every cursor (zones + off stem) bit-identical.
        for index in 0..ZONE_COUNT {
            assert_eq!(
                reference.zones[index].cursor.to_bits(),
                suspended.zones[index].cursor.to_bits(),
                "zone {index} cursor drifted while suspended"
            );
        }
        assert_eq!(
            reference.off_zone.as_ref().map(|zone| zone.cursor.to_bits()),
            suspended.off_zone.as_ref().map(|zone| zone.cursor.to_bits()),
            "off-stem cursor drifted while suspended"
        );
        // fix.txt measured 2026-09-05 on this sweep: transient 0.0039 with a
        // 500 rpm deep-silence margin (was 0.0120 with suspend-anywhere), skip
        // ratio 0.43. Bounds below carry ~2.5x / ~2x margin.
        assert!(
            max_diff <= 0.01,
            "reactivation transient peak {max_diff} exceeds proposed -40 dBFS criterion"
        );
        assert!(
            max_step_suspended <= max_step_reference * 1.5,
            "suspended max step {max_step_suspended} vs reference {max_step_reference}: click?"
        );
    }
}
