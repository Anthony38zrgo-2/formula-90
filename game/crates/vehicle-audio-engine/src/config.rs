//! Runtime mixer configuration loaded from `sound_mixer_config.json`.
//!
//! The file lives in the sounds root (`game/sounds/`), two levels above the bank
//! directory (`game/sounds/banks/v10_vehicle` -> `../../sound_mixer_config.json`)
//! and lets tuning change how loud a specific bank sample can sound without
//! recompiling the DLL. Missing, invalid or malformed JSON degrades to an empty
//! config (gain 1.0 everywhere): the mixer must never fail to start because of
//! a tuning file.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Filename of the runtime mixer config, resolved relative to the bank parent.
pub const MIXER_CONFIG_FILENAME: &str = "sound_mixer_config.json";

/// Runtime parameters for the exhaust microphone layer.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ExhaustConfig {
    /// If false, the exhaust layer is silent.
    pub enabled: bool,
    /// Base linear gain of the exhaust sample (0..1).
    pub base_gain: f32,
    /// How much throttle modulates the exhaust amplitude (0..1). At 0.0 the
    /// layer stays at base_gain regardless of throttle; at 1.0 it follows
    /// throttle directly.
    pub throttle_sensitivity: f32,
    /// Reserved for additive gas-flow noise. Kept at 0.0 for now; the pitch-
    /// shifted exhaust sample already carries the hiss character.
    pub gas_flow_gain: f32,
    /// Extra gain added during backfire events to simulate the exhaust pop.
    pub crackle_gain: f32,
}

impl ExhaustConfig {
    /// Safe defaults: active, moderate level, strong throttle tracking,
    /// noticeable backfire crackle.
    pub fn default_runtime() -> Self {
        Self {
            enabled: true,
            base_gain: 0.7,
            throttle_sensitivity: 0.8,
            gas_flow_gain: 0.0,
            crackle_gain: 0.6,
        }
    }
}

/// Raw `"exhaust"` section as it appears in `sound_mixer_config.json`. Every
/// field is optional so the file can be partially written; missing values fall
/// back to the runtime defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsonExhaustConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub base_gain: Option<f32>,
    #[serde(default)]
    pub throttle_sensitivity: Option<f32>,
    #[serde(default)]
    pub gas_flow_gain: Option<f32>,
    #[serde(default)]
    pub crackle_gain: Option<f32>,
}

impl JsonExhaustConfig {
    /// Merge optional JSON values into safe runtime defaults, clamping to [0,1]
    /// where appropriate.
    pub fn to_runtime(&self) -> ExhaustConfig {
        let mut cfg = ExhaustConfig::default_runtime();
        if let Some(v) = self.enabled {
            cfg.enabled = v;
        }
        if let Some(v) = self.base_gain {
            cfg.base_gain = v.clamp(0.0, 1.0);
        }
        if let Some(v) = self.throttle_sensitivity {
            cfg.throttle_sensitivity = v.clamp(0.0, 1.0);
        }
        if let Some(v) = self.gas_flow_gain {
            cfg.gas_flow_gain = v.clamp(0.0, 1.0);
        }
        if let Some(v) = self.crackle_gain {
            cfg.crackle_gain = v.clamp(0.0, 1.0);
        }
        cfg
    }
}

/// Linear per-sample gain ceiling loaded from the JSON file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SoundMixerConfig {
    #[serde(default)]
    pub schema_version: u32,
    /// `bank key` -> linear gain in [0.0, 1.0]. Absent keys default to 1.0.
    #[serde(default)]
    pub gains: BTreeMap<String, f32>,
    /// Behavioural configuration for the exhaust microphone layer.
    #[serde(default)]
    pub exhaust: JsonExhaustConfig,
}

impl SoundMixerConfig {
    /// Load from `<bank_dir>/../../sound_mixer_config.json` (bank lives in
    /// `sounds/banks/<bank>`, the config in `sounds/`). Any failure (missing
    /// file, unreadable, invalid JSON) yields an empty config — never an error.
    pub fn load_from_bank_dir(bank_dir: &Path) -> Self {
        let Some(sounds_dir) = bank_dir.parent().and_then(|p| p.parent()) else {
            return Self::default();
        };
        let path = sounds_dir.join(MIXER_CONFIG_FILENAME);
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        match serde_json::from_str::<SoundMixerConfig>(&raw) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "[vehicle_audio_engine] {MIXER_CONFIG_FILENAME} ignored (invalid JSON): {e}"
                );
                Self::default()
            }
        }
    }

    /// Sanitize: clamp gains to [0.0, 1.0] and drop non-finite values so a bad
    /// tuning file can never push a sample over the limiter.
    pub fn sanitized(mut self) -> Self {
        self.gains.retain(|_, g| g.is_finite());
        for g in self.gains.values_mut() {
            *g = g.clamp(0.0, 1.0);
        }
        // Sanitize the optional exhaust section as well.
        if let Some(v) = self.exhaust.base_gain {
            self.exhaust.base_gain = Some(v.clamp(0.0, 1.0));
        }
        if let Some(v) = self.exhaust.throttle_sensitivity {
            self.exhaust.throttle_sensitivity = Some(v.clamp(0.0, 1.0));
        }
        if let Some(v) = self.exhaust.gas_flow_gain {
            self.exhaust.gas_flow_gain = Some(v.clamp(0.0, 1.0));
        }
        if let Some(v) = self.exhaust.crackle_gain {
            self.exhaust.crackle_gain = Some(v.clamp(0.0, 1.0));
        }
        self
    }

    /// Exhaust runtime config merged from JSON + defaults.
    pub fn exhaust_config(&self) -> ExhaustConfig {
        self.exhaust.to_runtime()
    }

    /// Gain for a bank key (1.0 when not configured).
    pub fn gain(&self, key: &str) -> f32 {
        self.gains.get(key).copied().unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir;

    fn write_config(parent: &std::path::Path, body: &str) {
        std::fs::write(parent.join(MIXER_CONFIG_FILENAME), body)
            .expect("write test config");
    }

    struct DirGuard(std::path::PathBuf);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn make_bank_dir() -> (DirGuard, std::path::PathBuf) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("f90_audio_cfg_{unique}"));
        let bank = root.join("banks").join("v10_vehicle");
        std::fs::create_dir_all(&bank).expect("create temp bank dir");
        (DirGuard(root), bank)
    }

    #[test]
    fn missing_file_yields_empty_config() {
        let (_guard, bank) = make_bank_dir();
        let cfg = SoundMixerConfig::load_from_bank_dir(&bank);
        assert!(cfg.gains.is_empty());
        assert_eq!(cfg.gain("int_backfire"), 1.0);
    }

    #[test]
    fn invalid_json_yields_empty_config() {
        let (_guard, bank) = make_bank_dir();
        let root = bank.parent().unwrap().parent().unwrap();
        write_config(root, "{ not json !!");
        let cfg = SoundMixerConfig::load_from_bank_dir(&bank);
        assert!(cfg.gains.is_empty());
    }

    #[test]
    fn parses_gains_and_defaults_unknown_keys() {
        let (_guard, bank) = make_bank_dir();
        let root = bank.parent().unwrap().parent().unwrap();
        write_config(
            root,
            r#"{"schema_version": 1, "gains": {"int_backfire": 0.5, "shift_up": 0.25}}"#,
        );
        let cfg = SoundMixerConfig::load_from_bank_dir(&bank).sanitized();
        assert_eq!(cfg.gain("int_backfire"), 0.5);
        assert_eq!(cfg.gain("shift_up"), 0.25);
        assert_eq!(cfg.gain("surf_grass"), 1.0, "unconfigured key defaults to 1.0");
        let ex = cfg.exhaust_config();
        assert!(ex.enabled);
        assert_eq!(ex.base_gain, 0.7);
    }

    #[test]
    fn parses_exhaust_section_and_clamps_bad_values() {
        let (_guard, bank) = make_bank_dir();
        let root = bank.parent().unwrap().parent().unwrap();
        write_config(
            root,
            r#"{"exhaust": {"enabled": false, "base_gain": 1.2, "throttle_sensitivity": -0.5, "crackle_gain": 0.4}}"#,
        );
        let cfg = SoundMixerConfig::load_from_bank_dir(&bank).sanitized();
        let ex = cfg.exhaust_config();
        assert!(!ex.enabled);
        assert_eq!(ex.base_gain, 1.0);
        assert_eq!(ex.throttle_sensitivity, 0.0);
        assert_eq!(ex.crackle_gain, 0.4);
    }

    #[test]
    fn sanitized_clamps_out_of_range_and_non_finite_gains() {
        let cfg = SoundMixerConfig {
            schema_version: 1,
            gains: BTreeMap::from([
                ("a".to_string(), 2.5),
                ("b".to_string(), -1.0),
                ("c".to_string(), f32::NAN),
                ("d".to_string(), f32::INFINITY),
                ("e".to_string(), 0.7),
            ]),
            exhaust: JsonExhaustConfig::default(),
        }
        .sanitized();
        assert_eq!(cfg.gain("a"), 1.0);
        assert_eq!(cfg.gain("b"), 0.0);
        assert_eq!(cfg.gain("c"), 1.0);
        assert_eq!(cfg.gain("d"), 1.0);
        assert_eq!(cfg.gain("e"), 0.7);
    }
}