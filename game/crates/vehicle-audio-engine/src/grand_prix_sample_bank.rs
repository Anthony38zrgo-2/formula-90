use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::bank::{read_wav_mono16, sha256_hex};

pub const GRAND_PRIX_SAMPLE_RATE: u32 = 44100;
pub const GRAND_PRIX_SCHEMA_VERSION: u64 = 1;
pub const GRAND_PRIX_MINIMUM_LOOP_COUNT: usize = 2;
pub const GRAND_PRIX_MAXIMUM_LOOP_COUNT: usize = 16;

#[derive(Debug, thiserror::Error)]
pub enum GrandPrixBankError {
    #[error("grand prix bank directory not found: {0}")]
    MissingDirectory(PathBuf),
    #[error("manifest.json missing in {0}")]
    MissingManifest(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("sha256 mismatch for {0}")]
    Sha256Mismatch(String),
    #[error("invalid asset {0}: {1}")]
    InvalidAsset(String, String),
    #[error("decode error for {0}: {1}")]
    Decode(String, String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixCoverage {
    pub minimum_revolutions_per_minute: f64,
    pub maximum_revolutions_per_minute: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixLoopAsset {
    pub id: String,
    pub role: String,
    pub source_filename: String,
    pub source_sha256: String,
    pub derived_filename: String,
    pub derived_sha256: String,
    pub derived_frames: u64,
    pub derived_rms_dbfs: f64,
    pub derived_peak_dbfs: f64,
    pub comb_spacing_hz: f64,
    pub reference_revolutions_per_minute: f64,
    pub calibrated_gain: f64,
    pub valid_playback_rate_min: f64,
    pub valid_playback_rate_max: f64,
    pub active_coverage_revolutions_per_minute: [f64; 2],
    pub loop_start_frame: u64,
    pub loop_end_frame_exclusive: u64,
    pub loop_crossfade_frames: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixTransition {
    pub from_loop_id: String,
    pub to_loop_id: String,
    pub start_revolutions_per_minute: f64,
    pub end_revolutions_per_minute: f64,
    pub center_revolutions_per_minute: f64,
    pub half_width_revolutions_per_minute: f64,
    pub blend_law: String,
    pub gain_law: String,
    pub measured_level_compensation_db: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixEventAsset {
    pub id: String,
    pub role: String,
    pub source_filename: String,
    pub source_sha256: String,
    pub derived_filename: String,
    pub derived_sha256: String,
    pub derived_frames: u64,
    pub derived_rms_dbfs: f64,
    pub derived_peak_dbfs: f64,
    pub preparation_recipe: String,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixEventVariant {
    pub asset_id: String,
    pub event_gain: f64,
    pub trigger_pitch_reference_revolutions_per_minute: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixEventGroup {
    pub id: String,
    pub role: String,
    pub selection: String,
    pub voice_limit: u32,
    pub minimum_retrigger_seconds: f64,
    pub timing_offset_seconds: f64,
    pub trigger_event: String,
    pub variants: Vec<GrandPrixEventVariant>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixLiftEdgePolicy {
    pub previous_throttle_min: f64,
    pub throttle_max: f64,
    pub revolutions_per_minute_min: f64,
    pub cooldown_seconds: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixLimiterEdgePolicy {
    pub cooldown_seconds: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixEventTriggerPolicy {
    pub lift_edge: GrandPrixLiftEdgePolicy,
    pub limiter_entry_edge: GrandPrixLimiterEdgePolicy,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrandPrixManifest {
    pub schema_version: u64,
    pub bank_id: String,
    pub preparation_tool: String,
    pub preparation_tool_revision: u64,
    pub source_inventory_sha256: String,
    pub output_sample_rate: u32,
    pub event_selection_seed: u64,
    pub coverage: GrandPrixCoverage,
    pub loops: Vec<GrandPrixLoopAsset>,
    pub transitions: Vec<GrandPrixTransition>,
    pub events: Vec<GrandPrixEventAsset>,
    pub event_groups: Vec<GrandPrixEventGroup>,
    pub event_trigger_policy: GrandPrixEventTriggerPolicy,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GrandPrixEventRole {
    GearboxUpshift,
    GearboxDownshift,
    LiftBackfire,
    LimiterEvent,
}

impl GrandPrixEventRole {
    pub fn from_role_name(name: &str) -> Result<Self, GrandPrixBankError> {
        match name {
            "gearbox_upshift" => Ok(Self::GearboxUpshift),
            "gearbox_downshift" => Ok(Self::GearboxDownshift),
            "lift_backfire" => Ok(Self::LiftBackfire),
            "limiter_event" => Ok(Self::LimiterEvent),
            other => Err(GrandPrixBankError::InvalidAsset(
                other.to_string(),
                "unknown event role".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GrandPrixSelection {
    SingleVariant,
    SeededCycle,
    SeededNoImmediateRepeat,
}

impl GrandPrixSelection {
    pub fn from_selection_name(name: &str) -> Result<Self, GrandPrixBankError> {
        match name {
            "single_variant" => Ok(Self::SingleVariant),
            "seeded_cycle" => Ok(Self::SeededCycle),
            "seeded_no_immediate_repeat" => Ok(Self::SeededNoImmediateRepeat),
            other => Err(GrandPrixBankError::InvalidAsset(
                other.to_string(),
                "unknown selection policy".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GrandPrixTriggerEvent {
    UpshiftCutEntry,
    DownshiftCutEntry,
    LiftEdge,
    LimiterEntryEdge,
}

impl GrandPrixTriggerEvent {
    pub fn from_trigger_name(name: &str) -> Result<Self, GrandPrixBankError> {
        match name {
            "upshift_cut_entry" => Ok(Self::UpshiftCutEntry),
            "downshift_cut_entry" => Ok(Self::DownshiftCutEntry),
            "lift_edge" => Ok(Self::LiftEdge),
            "limiter_entry_edge" => Ok(Self::LimiterEntryEdge),
            other => Err(GrandPrixBankError::InvalidAsset(
                other.to_string(),
                "unknown trigger event".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GrandPrixDecodedLoop {
    pub id: String,
    pub pcm: Vec<i16>,
    pub reference_revolutions_per_minute: f64,
    pub calibrated_gain: f32,
    pub valid_playback_rate_min: f64,
    pub valid_playback_rate_max: f64,
    pub loop_start_frame: usize,
    pub loop_end_frame_exclusive: usize,
    pub crossfade_frames: usize,
}

#[derive(Debug, Clone)]
pub struct GrandPrixDecodedEvent {
    pub id: String,
    pub role: GrandPrixEventRole,
    pub pcm: Vec<i16>,
}

#[derive(Debug, Clone, Copy)]
pub struct GrandPrixDecodedVariant {
    pub event_index: usize,
    pub gain: f32,
    pub pitch_reference_revolutions_per_minute: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct GrandPrixDecodedGroup {
    pub id: String,
    pub role: GrandPrixEventRole,
    pub selection: GrandPrixSelection,
    pub voice_limit: usize,
    pub minimum_retrigger_frames: usize,
    pub timing_offset_frames: usize,
    pub trigger_event: GrandPrixTriggerEvent,
    pub variants: Vec<GrandPrixDecodedVariant>,
}

pub struct GrandPrixSampleBank {
    pub bank_id: String,
    pub bank_sha256: String,
    pub event_selection_seed: u64,
    pub coverage_minimum_revolutions_per_minute: f64,
    pub coverage_maximum_revolutions_per_minute: f64,
    pub loops: Vec<GrandPrixDecodedLoop>,
    pub transitions: Vec<GrandPrixTransition>,
    pub events: Vec<GrandPrixDecodedEvent>,
    pub groups: Vec<GrandPrixDecodedGroup>,
    pub lift_edge_policy: GrandPrixLiftEdgePolicy,
    pub limiter_edge_policy: GrandPrixLimiterEdgePolicy,
}

impl std::fmt::Debug for GrandPrixSampleBank {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GrandPrixSampleBank")
            .field("bank_id", &self.bank_id)
            .field("bank_sha256", &self.bank_sha256)
            .field("event_selection_seed", &self.event_selection_seed)
            .field("loops", &self.loops.len())
            .field("transitions", &self.transitions.len())
            .field("events", &self.events.len())
            .field("groups", &self.groups.len())
            .finish()
    }
}

fn is_finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn in_unit_range(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

impl GrandPrixManifest {
    pub fn validate(&self) -> Result<(), GrandPrixBankError> {
        if self.schema_version != GRAND_PRIX_SCHEMA_VERSION {
            return Err(GrandPrixBankError::InvalidManifest(format!(
                "unsupported schema_version {}",
                self.schema_version
            )));
        }
        if self.bank_id.is_empty() {
            return Err(GrandPrixBankError::InvalidManifest(
                "bank_id missing".to_string(),
            ));
        }
        if self.preparation_tool.is_empty() || self.preparation_tool_revision == 0 {
            return Err(GrandPrixBankError::InvalidManifest(
                "preparation provenance missing".to_string(),
            ));
        }
        if !is_hex_sha256(&self.source_inventory_sha256) {
            return Err(GrandPrixBankError::InvalidManifest(
                "source_inventory_sha256 invalid".to_string(),
            ));
        }
        if self.output_sample_rate != GRAND_PRIX_SAMPLE_RATE {
            return Err(GrandPrixBankError::InvalidManifest(format!(
                "output_sample_rate must be {GRAND_PRIX_SAMPLE_RATE}"
            )));
        }
        if self.loops.len() < GRAND_PRIX_MINIMUM_LOOP_COUNT
            || self.loops.len() > GRAND_PRIX_MAXIMUM_LOOP_COUNT
        {
            return Err(GrandPrixBankError::InvalidManifest(format!(
                "loop count {} outside [{GRAND_PRIX_MINIMUM_LOOP_COUNT}, {GRAND_PRIX_MAXIMUM_LOOP_COUNT}]",
                self.loops.len()
            )));
        }
        let coverage = &self.coverage;
        if !is_finite_positive(coverage.minimum_revolutions_per_minute)
            || !is_finite_positive(coverage.maximum_revolutions_per_minute)
            || coverage.minimum_revolutions_per_minute >= coverage.maximum_revolutions_per_minute
        {
            return Err(GrandPrixBankError::InvalidManifest(
                "coverage range invalid".to_string(),
            ));
        }
        let mut loop_ids = BTreeSet::new();
        let mut previous_reference = 0.0_f64;
        for (index, loop_asset) in self.loops.iter().enumerate() {
            if loop_asset.role != "engine_loop" {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "loop role must be engine_loop".to_string(),
                ));
            }
            if !loop_ids.insert(loop_asset.id.clone()) {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "duplicate loop id".to_string(),
                ));
            }
            if !is_hex_sha256(&loop_asset.derived_sha256)
                || !is_hex_sha256(&loop_asset.source_sha256)
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "sha256 fields invalid".to_string(),
                ));
            }
            if !is_finite_positive(loop_asset.reference_revolutions_per_minute) {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "reference rpm must be positive and finite".to_string(),
                ));
            }
            if loop_asset.reference_revolutions_per_minute <= previous_reference {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "loop references must be strictly ascending".to_string(),
                ));
            }
            previous_reference = loop_asset.reference_revolutions_per_minute;
            if !loop_asset.calibrated_gain.is_finite() || loop_asset.calibrated_gain <= 0.0 {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "calibrated_gain must be finite and positive".to_string(),
                ));
            }
            if !is_finite_positive(loop_asset.valid_playback_rate_min)
                || !is_finite_positive(loop_asset.valid_playback_rate_max)
                || loop_asset.valid_playback_rate_min > loop_asset.valid_playback_rate_max
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "playback rate bounds invalid".to_string(),
                ));
            }
            if loop_asset.loop_start_frame >= loop_asset.loop_end_frame_exclusive
                || loop_asset.loop_end_frame_exclusive > loop_asset.derived_frames
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "loop bounds invalid".to_string(),
                ));
            }
            let loop_length = loop_asset.loop_end_frame_exclusive - loop_asset.loop_start_frame;
            if loop_asset.loop_crossfade_frames == 0
                || loop_asset.loop_crossfade_frames * 4 > loop_length
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "loop crossfade invalid".to_string(),
                ));
            }
            let active_coverage = loop_asset.active_coverage_revolutions_per_minute;
            let expected_coverage_start = if index == 0 {
                coverage.minimum_revolutions_per_minute
            } else {
                self.transitions[index - 1].start_revolutions_per_minute
            };
            let expected_coverage_end = if index == self.loops.len() - 1 {
                coverage.maximum_revolutions_per_minute
            } else {
                self.transitions[index].end_revolutions_per_minute
            };
            if (active_coverage[0] - expected_coverage_start).abs() > 1e-6
                || (active_coverage[1] - expected_coverage_end).abs() > 1e-6
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "active coverage disagrees with transitions".to_string(),
                ));
            }
            let reference = loop_asset.reference_revolutions_per_minute;
            let observed_min = active_coverage[0] / reference;
            let observed_max = active_coverage[1] / reference;
            let required_min = observed_min.min(observed_max);
            let required_max = observed_min.max(observed_max);
            if loop_asset.valid_playback_rate_min > required_min + 1e-6
                || loop_asset.valid_playback_rate_max < required_max - 1e-6
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "active coverage exceeds declared playback rate bounds".to_string(),
                ));
            }
            if index == 0 {
                if active_coverage[0] > coverage.minimum_revolutions_per_minute + 1e-9 {
                    return Err(GrandPrixBankError::InvalidAsset(
                        loop_asset.id.clone(),
                        "first loop does not reach minimum coverage".to_string(),
                    ));
                }
            }
            if index == self.loops.len() - 1
                && active_coverage[1] < coverage.maximum_revolutions_per_minute - 1e-9
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    "last loop does not reach maximum coverage".to_string(),
                ));
            }
        }
        if self.transitions.len() != self.loops.len() - 1 {
            return Err(GrandPrixBankError::InvalidManifest(
                "transition count does not match loop count".to_string(),
            ));
        }
        for (index, transition) in self.transitions.iter().enumerate() {
            if transition.from_loop_id != self.loops[index].id
                || transition.to_loop_id != self.loops[index + 1].id
            {
                return Err(GrandPrixBankError::InvalidManifest(
                    "transition endpoints do not match loop order".to_string(),
                ));
            }
            if !is_finite_positive(transition.start_revolutions_per_minute)
                || !is_finite_positive(transition.end_revolutions_per_minute)
                || transition.start_revolutions_per_minute >= transition.end_revolutions_per_minute
            {
                return Err(GrandPrixBankError::InvalidManifest(
                    "transition window invalid".to_string(),
                ));
            }
            if transition.blend_law != "smoothstep" || transition.gain_law != "equal_power" {
                return Err(GrandPrixBankError::InvalidManifest(
                    "unsupported blend or gain law".to_string(),
                ));
            }
            let start = transition.start_revolutions_per_minute;
            if index > 0 {
                let previous_end = self.transitions[index - 1].end_revolutions_per_minute;
                if start < previous_end - 1e-9 {
                    return Err(GrandPrixBankError::InvalidManifest(
                        "transition windows overlap".to_string(),
                    ));
                }
            }
        }
        if self
            .transitions
            .first()
            .is_some_and(|transition| {
                transition.start_revolutions_per_minute < coverage.minimum_revolutions_per_minute
            })
            || self
                .transitions
                .last()
                .is_some_and(|transition| {
                    transition.end_revolutions_per_minute > coverage.maximum_revolutions_per_minute
                })
        {
            return Err(GrandPrixBankError::InvalidManifest(
                "transitions exceed declared coverage".to_string(),
            ));
        }
        let mut event_ids = BTreeSet::new();
        let mut event_roles = BTreeMap::new();
        for event_asset in &self.events {
            if !event_ids.insert(event_asset.id.clone()) {
                return Err(GrandPrixBankError::InvalidAsset(
                    event_asset.id.clone(),
                    "duplicate event id".to_string(),
                ));
            }
            let role = GrandPrixEventRole::from_role_name(&event_asset.role)?;
            event_roles.insert(event_asset.id.clone(), role);
            if !is_hex_sha256(&event_asset.derived_sha256)
                || !is_hex_sha256(&event_asset.source_sha256)
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    event_asset.id.clone(),
                    "sha256 fields invalid".to_string(),
                ));
            }
            if event_asset.derived_frames == 0 || !is_finite_positive(event_asset.duration_seconds) {
                return Err(GrandPrixBankError::InvalidAsset(
                    event_asset.id.clone(),
                    "event duration invalid".to_string(),
                ));
            }
        }
        let mut group_ids = BTreeSet::new();
        for group in &self.event_groups {
            if !group_ids.insert(group.id.clone()) {
                return Err(GrandPrixBankError::InvalidAsset(
                    group.id.clone(),
                    "duplicate event group id".to_string(),
                ));
            }
            let group_role = GrandPrixEventRole::from_role_name(&group.role)?;
            GrandPrixSelection::from_selection_name(&group.selection)?;
            GrandPrixTriggerEvent::from_trigger_name(&group.trigger_event)?;
            if group.voice_limit == 0 || group.voice_limit > 8 {
                return Err(GrandPrixBankError::InvalidAsset(
                    group.id.clone(),
                    "voice_limit must be within 1..=8".to_string(),
                ));
            }
            if !is_finite_positive(group.minimum_retrigger_seconds)
                || !group.timing_offset_seconds.is_finite()
                || group.timing_offset_seconds < 0.0
            {
                return Err(GrandPrixBankError::InvalidAsset(
                    group.id.clone(),
                    "group timing invalid".to_string(),
                ));
            }
            if group.variants.is_empty() {
                return Err(GrandPrixBankError::InvalidAsset(
                    group.id.clone(),
                    "group has no variants".to_string(),
                ));
            }
            for variant in &group.variants {
                if !event_ids.contains(&variant.asset_id) {
                    return Err(GrandPrixBankError::InvalidAsset(
                        group.id.clone(),
                        format!("unknown variant asset {}", variant.asset_id),
                    ));
                }
                if event_roles.get(&variant.asset_id) != Some(&group_role) {
                    return Err(GrandPrixBankError::InvalidAsset(
                        group.id.clone(),
                        format!("variant {} role mismatch", variant.asset_id),
                    ));
                }
                if !variant.event_gain.is_finite() || variant.event_gain <= 0.0 {
                    return Err(GrandPrixBankError::InvalidAsset(
                        group.id.clone(),
                        "variant gain invalid".to_string(),
                    ));
                }
                if variant
                    .trigger_pitch_reference_revolutions_per_minute
                    .is_some_and(|reference| !is_finite_positive(reference))
                {
                    return Err(GrandPrixBankError::InvalidAsset(
                        group.id.clone(),
                        "variant pitch reference invalid".to_string(),
                    ));
                }
            }
        }
        let policy = &self.event_trigger_policy;
        if !in_unit_range(policy.lift_edge.previous_throttle_min)
            || !in_unit_range(policy.lift_edge.throttle_max)
            || policy.lift_edge.previous_throttle_min <= policy.lift_edge.throttle_max
            || policy.lift_edge.revolutions_per_minute_min < 0.0
            || !policy.lift_edge.revolutions_per_minute_min.is_finite()
            || policy.lift_edge.cooldown_seconds < 0.0
            || !policy.lift_edge.cooldown_seconds.is_finite()
        {
            return Err(GrandPrixBankError::InvalidManifest(
                "lift edge policy invalid".to_string(),
            ));
        }
        if policy.limiter_entry_edge.cooldown_seconds < 0.0
            || !policy.limiter_entry_edge.cooldown_seconds.is_finite()
        {
            return Err(GrandPrixBankError::InvalidManifest(
                "limiter edge policy invalid".to_string(),
            ));
        }
        Ok(())
    }
}

impl GrandPrixSampleBank {
    pub fn load(bank_directory: &Path) -> Result<Self, GrandPrixBankError> {
        if !bank_directory.is_dir() {
            return Err(GrandPrixBankError::MissingDirectory(
                bank_directory.to_path_buf(),
            ));
        }
        let manifest_path = bank_directory.join("manifest.json");
        if !manifest_path.is_file() {
            return Err(GrandPrixBankError::MissingManifest(
                bank_directory.to_path_buf(),
            ));
        }
        let manifest_bytes = fs::read(&manifest_path)?;
        let bank_sha256 = sha256_hex(&manifest_bytes);
        let manifest: GrandPrixManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| GrandPrixBankError::InvalidManifest(error.to_string()))?;
        manifest.validate()?;

        let mut loops = Vec::with_capacity(manifest.loops.len());
        for loop_asset in &manifest.loops {
            let pcm = load_verified_pcm(bank_directory, &loop_asset.derived_filename, &loop_asset.derived_sha256)?;
            if pcm.len() as u64 != loop_asset.derived_frames {
                return Err(GrandPrixBankError::InvalidAsset(
                    loop_asset.id.clone(),
                    format!(
                        "decoded {} frames but manifest declares {}",
                        pcm.len(),
                        loop_asset.derived_frames
                    ),
                ));
            }
            loops.push(GrandPrixDecodedLoop {
                id: loop_asset.id.clone(),
                pcm,
                reference_revolutions_per_minute: loop_asset.reference_revolutions_per_minute,
                calibrated_gain: loop_asset.calibrated_gain as f32,
                valid_playback_rate_min: loop_asset.valid_playback_rate_min,
                valid_playback_rate_max: loop_asset.valid_playback_rate_max,
                loop_start_frame: loop_asset.loop_start_frame as usize,
                loop_end_frame_exclusive: loop_asset.loop_end_frame_exclusive as usize,
                crossfade_frames: loop_asset.loop_crossfade_frames as usize,
            });
        }

        let mut events = Vec::with_capacity(manifest.events.len());
        let mut event_index_by_id = BTreeMap::new();
        for event_asset in &manifest.events {
            let pcm = load_verified_pcm(
                bank_directory,
                &event_asset.derived_filename,
                &event_asset.derived_sha256,
            )?;
            if pcm.len() as u64 != event_asset.derived_frames {
                return Err(GrandPrixBankError::InvalidAsset(
                    event_asset.id.clone(),
                    format!(
                        "decoded {} frames but manifest declares {}",
                        pcm.len(),
                        event_asset.derived_frames
                    ),
                ));
            }
            event_index_by_id.insert(event_asset.id.clone(), events.len());
            events.push(GrandPrixDecodedEvent {
                id: event_asset.id.clone(),
                role: GrandPrixEventRole::from_role_name(&event_asset.role)?,
                pcm,
            });
        }

        let mut groups = Vec::with_capacity(manifest.event_groups.len());
        for group in &manifest.event_groups {
            let mut variants = Vec::with_capacity(group.variants.len());
            for variant in &group.variants {
                let event_index = *event_index_by_id.get(&variant.asset_id).ok_or_else(|| {
                    GrandPrixBankError::InvalidAsset(
                        group.id.clone(),
                        format!("unknown variant asset {}", variant.asset_id),
                    )
                })?;
                variants.push(GrandPrixDecodedVariant {
                    event_index,
                    gain: variant.event_gain as f32,
                    pitch_reference_revolutions_per_minute: variant
                        .trigger_pitch_reference_revolutions_per_minute,
                });
            }
            groups.push(GrandPrixDecodedGroup {
                id: group.id.clone(),
                role: GrandPrixEventRole::from_role_name(&group.role)?,
                selection: GrandPrixSelection::from_selection_name(&group.selection)?,
                voice_limit: group.voice_limit as usize,
                minimum_retrigger_frames: seconds_to_frames(group.minimum_retrigger_seconds),
                timing_offset_frames: seconds_to_frames(group.timing_offset_seconds),
                trigger_event: GrandPrixTriggerEvent::from_trigger_name(&group.trigger_event)?,
                variants,
            });
        }

        Ok(Self {
            bank_id: manifest.bank_id.clone(),
            bank_sha256,
            event_selection_seed: manifest.event_selection_seed,
            coverage_minimum_revolutions_per_minute: manifest
                .coverage
                .minimum_revolutions_per_minute,
            coverage_maximum_revolutions_per_minute: manifest
                .coverage
                .maximum_revolutions_per_minute,
            loops,
            transitions: manifest.transitions.clone(),
            events,
            groups,
            lift_edge_policy: manifest.event_trigger_policy.lift_edge.clone(),
            limiter_edge_policy: manifest.event_trigger_policy.limiter_entry_edge.clone(),
        })
    }

    pub fn group(&self, id: &str) -> Option<&GrandPrixDecodedGroup> {
        self.groups.iter().find(|group| group.id == id)
    }
}

fn seconds_to_frames(seconds: f64) -> usize {
    (seconds.max(0.0) * GRAND_PRIX_SAMPLE_RATE as f64).round() as usize
}

fn load_verified_pcm(
    bank_directory: &Path,
    filename: &str,
    expected_sha256: &str,
) -> Result<Vec<i16>, GrandPrixBankError> {
    let path = bank_directory.join(filename);
    let bytes = fs::read(&path)?;
    let actual = sha256_hex(&bytes);
    if actual != expected_sha256 {
        return Err(GrandPrixBankError::Sha256Mismatch(filename.to_string()));
    }
    read_wav_mono16(&path).map_err(|error| GrandPrixBankError::Decode(filename.to_string(), error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped_bank_directory() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../audio/formula_one_2030_grand_prix_sampler")
    }

    fn temporary_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "f90_grand_prix_bank_{}_{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("temporary directory");
        directory
    }

    #[test]
    fn shipped_bank_loads_and_matches_manifest() {
        let bank = GrandPrixSampleBank::load(&shipped_bank_directory()).expect("shipped bank");
        assert_eq!(bank.bank_id, "formula_one_2030_grand_prix_sampler");
        assert_eq!(bank.bank_sha256.len(), 64);
        assert_eq!(bank.loops.len(), 5);
        assert_eq!(bank.transitions.len(), 4);
        assert_eq!(bank.events.len(), 8);
        assert_eq!(bank.groups.len(), 4);
        assert_eq!(
            bank.coverage_minimum_revolutions_per_minute,
            4500.0
        );
        assert_eq!(
            bank.coverage_maximum_revolutions_per_minute,
            18000.0
        );
        let references: Vec<f64> = bank
            .loops
            .iter()
            .map(|loop_asset| loop_asset.reference_revolutions_per_minute)
            .collect();
        assert!(references.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(references[0] < 5000.0 && references[4] > 17000.0);
        let downshift = bank.group("downshift").expect("downshift group");
        assert_eq!(downshift.selection, GrandPrixSelection::SingleVariant);
        assert_eq!(downshift.variants.len(), 1);
        assert_eq!(downshift.minimum_retrigger_frames, 4410);
        let limiter = bank.group("limiter").expect("limiter group");
        assert_eq!(limiter.role, GrandPrixEventRole::LimiterEvent);
        assert_eq!(limiter.voice_limit, 1);
        assert_eq!(limiter.minimum_retrigger_frames, 11025);
        let limiter_event = bank
            .events
            .iter()
            .find(|event| event.role == GrandPrixEventRole::LimiterEvent)
            .expect("limiter event");
        assert_eq!(limiter_event.pcm.len(), 15939);
        assert_eq!(limiter_event.id, "limiter_event");
    }

    #[test]
    fn corrupt_loop_asset_is_rejected() {
        let source = shipped_bank_directory();
        let target = temporary_directory("corrupt");
        for entry in fs::read_dir(&source).expect("read bank directory") {
            let entry = entry.expect("entry");
            if entry.path().is_file() {
                fs::copy(entry.path(), target.join(entry.file_name())).expect("copy asset");
            }
        }
        let loop_path = target.join("engine_idle_loop.wav");
        let mut bytes = fs::read(&loop_path).expect("read loop");
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        fs::write(&loop_path, bytes).expect("corrupt loop");
        let error = GrandPrixSampleBank::load(&target).expect_err("corrupt loop must fail");
        match error {
            GrandPrixBankError::Sha256Mismatch(_) => {}
            other => panic!("unexpected error variant: {other:?}"),
        }
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn unsupported_schema_is_rejected() {
        let target = temporary_directory("schema");
        let mut manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(shipped_bank_directory().join("manifest.json")).expect("manifest"),
        )
        .expect("parse manifest");
        manifest["schema_version"] = serde_json::json!(2);
        fs::write(
            target.join("manifest.json"),
            serde_json::to_vec(&manifest).expect("serialize"),
        )
        .expect("write manifest");
        let error = GrandPrixSampleBank::load(&target).expect_err("schema 2 must fail");
        assert!(matches!(error, GrandPrixBankError::InvalidManifest(_)));
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn missing_playback_rate_bounds_are_rejected() {
        let target = temporary_directory("rates");
        let mut manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(shipped_bank_directory().join("manifest.json")).expect("manifest"),
        )
        .expect("parse manifest");
        manifest["loops"][0]["valid_playback_rate_min"] = serde_json::Value::Null;
        fs::write(
            target.join("manifest.json"),
            serde_json::to_vec(&manifest).expect("serialize"),
        )
        .expect("write manifest");
        let error = GrandPrixSampleBank::load(&target).expect_err("missing bounds must fail");
        assert!(matches!(error, GrandPrixBankError::InvalidManifest(_)));
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn transition_gaps_are_rejected() {        let target = temporary_directory("gap");
        let mut manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(shipped_bank_directory().join("manifest.json")).expect("manifest"),
        )
        .expect("parse manifest");
        manifest["transitions"][1]["start_revolutions_per_minute"] =
            serde_json::json!(4000.0);
        manifest["loops"][1]["active_coverage_revolutions_per_minute"] =
            serde_json::json!([4000.0, manifest["transitions"][1]["end_revolutions_per_minute"]]);
        fs::write(
            target.join("manifest.json"),
            serde_json::to_vec(&manifest).expect("serialize"),
        )
        .expect("write manifest");
        let error = GrandPrixSampleBank::load(&target).expect_err("overlap must fail");
        assert!(matches!(
            error,
            GrandPrixBankError::InvalidManifest(_) | GrandPrixBankError::InvalidAsset(_, _)
        ));
        let _ = fs::remove_dir_all(&target);
    }
}
