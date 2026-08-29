//! VehicleSoundBank — load and validate the v10_vehicle bank on disk.
//!
//! Reads `bank_manifest.json` plus the mono PCM16 44.1 kHz WAVs under
//! `game/sounds/banks/v10_vehicle/`, validates format (mono, 16-bit, 44.1 kHz)
//! and per-file sha256 against the manifest, and exposes buffers by key.
//! Pure Rust, no Godot deps.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::state::{EngineBandProfile, ENGINE_BAND_COUNT};

/// Minimum sample rate accepted by the native loader contract.
pub const SAMPLE_RATE: u32 = 44100;

/// Continuous sampled engine/exhaust voices retired in favour of the procedural
/// synth. The runtime bank must not contain any of these keys.
pub const RETIRED_KEYS: &[&str] = &[
    "engine_high",
    "engine_idle",
    "engine_limiter",
    "engine_low",
    "engine_mid",
    "engine_redline",
    "engine_tc",
    "exhaust-mic",
];

#[derive(Debug, thiserror::Error)]
pub enum BankError {
    #[error("bank directory not found: {0}")]
    MissingDir(PathBuf),
    #[error("bank_manifest.json missing in {0}")]
    MissingManifest(PathBuf),
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("invalid wav {0}: {1}")]
    InvalidWav(String, String),
    #[error("sha256 mismatch for {0}")]
    ShaMismatch(String),
    #[error("format error: {0}")]
    Format(String),
    #[error("invalid playback metadata: {0}")]
    InvalidPlayback(String),
}

impl From<std::io::Error> for BankError {
    fn from(e: std::io::Error) -> Self {
        BankError::Io(e)
    }
}

/// One manifest entry (subset of fields the runtime needs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub file: String,
    pub role: String,
    #[serde(default)]
    pub loop_: bool,
    pub duration_s: f64,
    #[serde(default)]
    pub sha256: String,
}

/// A loaded mono PCM16 sample buffer.
#[derive(Debug, Clone)]
pub struct Sample {
    pub key: String,
    pub role: String,
    pub sample_rate: u32,
    /// Raw PCM16 samples (mono).
    pub pcm: Vec<i16>,
    pub is_loop: bool,
}

impl Sample {
    pub fn duration_s(&self) -> f64 {
        self.pcm.len() as f64 / self.sample_rate as f64
    }
}

/// Loaded bank: buffers keyed by bank key, plus manifest-derived metadata.
#[derive(Debug, Clone)]
pub struct VehicleSoundBank {
    pub samples: BTreeMap<String, Sample>,
    pub bank_name: String,
    pub engine_bands: Vec<EngineBandProfile>,
    pub retired_names: Vec<String>,
    pub(crate) native_rpm: BTreeMap<String, f32>,
}

impl VehicleSoundBank {
    /// Load and validate a bank from `bank_dir` (must contain bank_manifest.json).
    pub fn load(bank_dir: &Path) -> Result<Self, BankError> {
        if !bank_dir.is_dir() {
            return Err(BankError::MissingDir(bank_dir.to_path_buf()));
        }
        let manifest_path = bank_dir.join("bank_manifest.json");
        if !manifest_path.is_file() {
            return Err(BankError::MissingManifest(bank_dir.to_path_buf()));
        }
        let manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(manifest_path)?)
            .map_err(|e| BankError::InvalidManifest(e.to_string()))?;
        let bank_name = manifest
            .get("bank_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let files = manifest
            .get("files")
            .and_then(|v| v.as_array())
            .ok_or_else(|| BankError::InvalidManifest("files missing".into()))?;

        let retired_names = manifest
            .get("retired_keys")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| RETIRED_KEYS.iter().map(|s| s.to_string()).collect());

        let mut samples = BTreeMap::new();
        let mut engine_bands = Vec::new();
        let mut native_rpm = BTreeMap::new();
        for entry in files {
            let file = entry
                .get("file")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BankError::InvalidManifest("file field missing".into()))?;
            let role = entry
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or(file)
                .to_string();
            let is_loop = entry.get("loop").and_then(|v| v.as_bool()).unwrap_or(false);
            let sha = entry
                .get("sha256")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let key = file.trim_end_matches(".wav").to_string();
            if RETIRED_KEYS.contains(&key.as_str()) {
                return Err(BankError::InvalidManifest(format!(
                    "retired key present in bank: {key}"
                )));
            }
            if let Some(playback) = entry.get("playback").and_then(|value| value.as_object()) {
                let native = playback
                    .get("native_rpm")
                    .and_then(|value| value.as_f64())
                    .ok_or_else(|| {
                        BankError::InvalidPlayback(format!("{file}: native_rpm missing"))
                    })? as f32;
                if !native.is_finite() || native <= 0.0 {
                    return Err(BankError::InvalidPlayback(format!(
                        "{file}: native_rpm must be positive"
                    )));
                }
                native_rpm.insert(key.clone(), native);
                if let Some(band) = playback
                    .get("engine_band")
                    .and_then(|value| value.as_object())
                {
                    let index = band
                        .get("index")
                        .and_then(|value| value.as_u64())
                        .ok_or_else(|| {
                            BankError::InvalidPlayback(format!("{file}: band index missing"))
                        })? as usize;
                    let center = band
                        .get("center")
                        .and_then(|value| value.as_f64())
                        .ok_or_else(|| {
                            BankError::InvalidPlayback(format!("{file}: band center missing"))
                        })? as f32;
                    let width = band
                        .get("width")
                        .and_then(|value| value.as_f64())
                        .ok_or_else(|| {
                            BankError::InvalidPlayback(format!("{file}: band width missing"))
                        })? as f32;
                    if index >= ENGINE_BAND_COUNT
                        || !(0.0..=1.0).contains(&center)
                        || !(0.0..=1.0).contains(&width)
                        || width == 0.0
                    {
                        return Err(BankError::InvalidPlayback(format!(
                            "{file}: invalid engine band index/center/width"
                        )));
                    }
                    engine_bands.push(EngineBandProfile {
                        key: key.clone(),
                        native_rpm: native,
                        index,
                        center,
                        width,
                    });
                }
            }
            let path = bank_dir.join(file);
            let pcm = read_wav_mono16(&path)?;
            if !sha.is_empty() {
                let actual = sha256_hex(&fs::read(&path)?);
                if actual != sha {
                    return Err(BankError::ShaMismatch(file.to_string()));
                }
            }
            samples.insert(
                key.clone(),
                Sample {
                    key,
                    role,
                    sample_rate: SAMPLE_RATE,
                    pcm,
                    is_loop,
                },
            );
        }
        engine_bands.sort_by_key(|band| band.index);
        // The continuous engine bands are retired, so zero bands is valid; when
        // bands are present they must still be the full, unique, ordered set.
        if !engine_bands.is_empty()
            && (engine_bands.len() != ENGINE_BAND_COUNT
                || engine_bands
                    .iter()
                    .enumerate()
                    .any(|(index, band)| band.index != index))
        {
            return Err(BankError::InvalidPlayback(format!(
                "expected {ENGINE_BAND_COUNT} unique ordered engine bands"
            )));
        }
        if samples.contains_key("exhaust-mic") && !native_rpm.contains_key("exhaust-mic") {
            return Err(BankError::InvalidPlayback(
                "exhaust-mic: native_rpm missing".into(),
            ));
        }
        Ok(Self {
            samples,
            bank_name,
            engine_bands,
            retired_names,
            native_rpm,
        })
    }

    /// Look up a sample by bank key (e.g. "engine_idle", "surf_sand").
    pub fn get(&self, key: &str) -> Option<&Sample> {
        self.samples.get(key)
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn keys(&self) -> Vec<String> {
        self.samples.keys().cloned().collect()
    }

    pub fn native_rpm(&self, key: &str) -> Option<f32> {
        self.native_rpm.get(key).copied()
    }
}

/// Read a mono PCM16 WAV file. Validates mono + 16-bit + 44.1 kHz.
fn read_wav_mono16(path: &Path) -> Result<Vec<i16>, BankError> {
    let bytes = fs::read(path)?;
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(BankError::InvalidWav(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            "not a RIFF/WAVE file".into(),
        ));
    }
    // fmt chunk
    let mut pos = 12usize;
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut data: Vec<u8> = Vec::new();
    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        if chunk_id == b"fmt " {
            if pos + 24 > bytes.len() {
                return Err(BankError::InvalidWav(
                    path.display().to_string(),
                    "truncated fmt".into(),
                ));
            }
            channels = u16::from_le_bytes(bytes[pos + 10..pos + 12].try_into().unwrap());
            sample_rate = u32::from_le_bytes(bytes[pos + 12..pos + 16].try_into().unwrap());
            bits = u16::from_le_bytes(bytes[pos + 22..pos + 24].try_into().unwrap());
        } else if chunk_id == b"data" {
            let start = pos + 8;
            let end = (start + size).min(bytes.len());
            data.extend_from_slice(&bytes[start..end]);
        }
        pos += 8 + size + (size & 1); // chunks are word-aligned
    }
    if channels != 1 {
        return Err(BankError::Format(format!(
            "{path:?}: expected mono, got {channels}"
        )));
    }
    if bits != 16 {
        return Err(BankError::Format(format!(
            "{path:?}: expected 16-bit, got {bits}"
        )));
    }
    if sample_rate != SAMPLE_RATE {
        return Err(BankError::Format(format!(
            "{path:?}: expected {SAMPLE_RATE} Hz, got {sample_rate}"
        )));
    }
    if !data.len().is_multiple_of(2) {
        data.truncate(data.len() & !1);
    }
    let mut pcm = Vec::with_capacity(data.len() / 2);
    for pair in data.chunks_exact(2) {
        pcm.push(i16::from_le_bytes([pair[0], pair[1]]));
    }
    Ok(pcm)
}

/// SHA-256 hex digest of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut s = String::with_capacity(64);
    for b in out {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
