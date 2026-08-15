//! Semantic asset registry for Track Studio.
//!
//! Assets are referenced by semantic id, never by filesystem path. This crate
//! owns the asset metadata and its validation. It mirrors the behavior of the
//! existing Python `asset_registry.py` and returns stable `Diagnostic` records
//! so the build gate and the UI consume one validation format.
//!
//! Binary assets (GLB/PNG) are never embedded here; the registry only records
//! source paths, provenance hashes and metadata.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use track_domain::{Diagnostic, Severity};

pub const ALLOWED_KINDS: [&str; 6] = [
    "card",
    "flag",
    "vegetation",
    "building",
    "barrier_visual",
    "prop",
];
pub const ALLOWED_COLLISION_CLASSES: [&str; 2] = ["none", "collision"];
pub const PROCEDURAL_KINDS: [&str; 1] = ["flag"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Budget {
    #[serde(default)]
    pub min_instances: u32,
    #[serde(default)]
    pub max_instances: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetSpec {
    pub id: String,
    pub kind: String,
    pub category: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    #[serde(default)]
    pub dimensions_m: BTreeMap<String, f64>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub collision_class: String,
    #[serde(default)]
    pub budget: Budget,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct AssetRegistry {
    pub specs: Vec<AssetSpec>,
    by_id: BTreeMap<String, usize>,
}

impl AssetRegistry {
    pub fn from_specs(specs: Vec<AssetSpec>) -> Self {
        let mut by_id = BTreeMap::new();
        for (index, spec) in specs.iter().enumerate() {
            by_id.insert(spec.id.clone(), index);
        }
        Self { specs, by_id }
    }

    pub fn lookup(&self, id: &str) -> Option<&AssetSpec> {
        self.by_id.get(id).map(|index| &self.specs[*index])
    }

    pub fn len(&self) -> usize {
        self.specs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// Validate structure and, when a base directory is provided, external
    /// source/preview existence and SHA-256 provenance.
    pub fn validate(&self, base: Option<&Path>) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen: BTreeMap<String, ()> = BTreeMap::new();

        for spec in &self.specs {
            if spec.id.is_empty() {
                diagnostics.push(error("assets.empty_id", "asset id is empty"));
            } else if !is_valid_id(&spec.id) {
                diagnostics.push(error(
                    "assets.invalid_id",
                    format!("invalid asset id {:?} (use lowercase [a-z0-9_.-])", spec.id),
                ));
            } else if seen.insert(spec.id.clone(), ()).is_some() {
                diagnostics.push(error(
                    "assets.duplicate_id",
                    format!("duplicate asset id {:?}", spec.id),
                ));
            }

            if !ALLOWED_KINDS.contains(&spec.kind.as_str()) {
                diagnostics.push(error(
                    "assets.invalid_kind",
                    format!("asset {:?}: unknown kind {:?}", spec.id, spec.kind),
                ));
            }
            if !ALLOWED_COLLISION_CLASSES.contains(&spec.collision_class.as_str()) {
                diagnostics.push(error(
                    "assets.invalid_collision_class",
                    format!(
                        "asset {:?}: unknown collision_class {:?}",
                        spec.id, spec.collision_class
                    ),
                ));
            }

            for (axis, value) in &spec.dimensions_m {
                if !matches!(axis.as_str(), "width" | "height" | "depth") {
                    diagnostics.push(error(
                        "assets.invalid_dimension",
                        format!("asset {:?}: unknown dimension axis {:?}", spec.id, axis),
                    ));
                } else if *value <= 0.0 || !value.is_finite() {
                    diagnostics.push(error(
                        "assets.invalid_dimension",
                        format!("asset {:?}: dimension {axis} must be positive", spec.id),
                    ));
                }
            }

            if spec.budget.min_instances > spec.budget.max_instances
                && spec.budget.max_instances > 0
            {
                diagnostics.push(error(
                    "assets.budget",
                    format!(
                        "asset {:?}: min_instances {} exceeds max_instances {}",
                        spec.id, spec.budget.min_instances, spec.budget.max_instances
                    ),
                ));
            }

            match &spec.source {
                Some(source) => {
                    if let Some(base) = base {
                        let path = base.join(source);
                        if !path.is_file() {
                            diagnostics.push(error(
                                "assets.missing_source",
                                format!("asset {:?}: missing source {}", spec.id, source),
                            ));
                        } else if let Some(expected) = &spec.source_sha256 {
                            match sha256_file(&path) {
                                Ok(actual) if &actual == expected => {}
                                Ok(actual) => diagnostics.push(error(
                                    "assets.source_hash_mismatch",
                                    format!(
                                        "asset {:?}: source hash mismatch for {} ({} != {})",
                                        spec.id, source, actual, expected
                                    ),
                                )),
                                Err(_) => diagnostics.push(error(
                                    "assets.source_unreadable",
                                    format!("asset {:?}: cannot read source {}", spec.id, source),
                                )),
                            }
                        }
                    }
                }
                None => {
                    if !PROCEDURAL_KINDS.contains(&spec.kind.as_str()) {
                        diagnostics.push(error(
                            "assets.missing_source",
                            format!(
                                "asset {:?}: missing source (kind {:?} requires one)",
                                spec.id, spec.kind
                            ),
                        ));
                    }
                }
            }

            if let Some(preview) = &spec.preview {
                if let Some(base) = base {
                    let path = base.join(preview);
                    if !path.is_file() {
                        diagnostics.push(error(
                            "assets.missing_preview",
                            format!("asset {:?}: missing preview {}", spec.id, preview),
                        ));
                    }
                }
            }
        }

        diagnostics
    }
}

fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    let first = chars.next();
    if !matches!(first, Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return false;
    }
    id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'))
}

fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn error(code: impl Into<String>, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        object_id: None,
        station: None,
        message: message.into(),
        metadata: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str) -> AssetSpec {
        AssetSpec {
            id: id.into(),
            kind: "vegetation".into(),
            category: "trees".into(),
            source: None,
            source_sha256: None,
            dimensions_m: BTreeMap::from([
                ("width".into(), 5.0),
                ("height".into(), 10.0),
                ("depth".into(), 5.0),
            ]),
            preview: None,
            collision_class: "none".into(),
            budget: Budget::default(),
            metadata: BTreeMap::new(),
        }
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
        diagnostics.iter().map(|d| d.code.clone()).collect()
    }

    #[test]
    fn duplicate_and_invalid_ids_are_detected() {
        let registry = AssetRegistry::from_specs(vec![spec("tree_a"), spec("tree_a")]);
        let diagnostics = registry.validate(None);
        assert!(codes(&diagnostics).contains(&"assets.duplicate_id".into()));

        let mut bad = spec("Tree@A");
        bad.id = "Tree@A".into();
        let registry = AssetRegistry::from_specs(vec![bad]);
        assert!(codes(&registry.validate(None)).contains(&"assets.invalid_id".into()));
    }

    #[test]
    fn invalid_kind_and_non_procedural_missing_source_are_detected() {
        let mut vegetation = spec("tree_a");
        vegetation.kind = "unknown".into();
        let registry = AssetRegistry::from_specs(vec![vegetation]);
        let diagnostics = registry.validate(None);
        assert!(codes(&diagnostics).contains(&"assets.invalid_kind".into()));
        assert!(codes(&diagnostics).contains(&"assets.missing_source".into()));
    }

    #[test]
    fn procedural_kind_allows_missing_source() {
        let mut flag = spec("track_flag");
        flag.kind = "flag".into();
        let registry = AssetRegistry::from_specs(vec![flag]);
        assert!(registry.validate(None).is_empty());
    }

    #[test]
    fn budget_and_dimension_errors_are_detected() {
        let mut a = spec("a");
        a.budget = Budget {
            min_instances: 5,
            max_instances: 2,
        };
        let mut b = spec("b");
        b.dimensions_m.insert("width".into(), -1.0);
        let registry = AssetRegistry::from_specs(vec![a, b]);
        let diagnostics = registry.validate(None);
        assert!(codes(&diagnostics).contains(&"assets.budget".into()));
        assert!(codes(&diagnostics).contains(&"assets.invalid_dimension".into()));
    }

    #[test]
    fn source_hash_verification_rejects_mismatch() {
        let dir = std::env::temp_dir().join(format!("f90_asset_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("asset.glb");
        std::fs::write(&path, b"hello world").unwrap();
        let actual = sha256_file(&path).unwrap();

        let mut good = spec("tree_a");
        good.source = Some("asset.glb".into());
        good.source_sha256 = Some(actual.clone());
        let registry = AssetRegistry::from_specs(vec![good]);
        assert!(registry.validate(Some(&dir)).is_empty());

        let mut bad = spec("tree_b");
        bad.source = Some("asset.glb".into());
        bad.source_sha256 = Some("0".repeat(64));
        let registry = AssetRegistry::from_specs(vec![bad]);
        assert!(
            codes(&registry.validate(Some(&dir))).contains(&"assets.source_hash_mismatch".into())
        );

        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn lookup_returns_spec_by_semantic_id() {
        let registry = AssetRegistry::from_specs(vec![spec("tree_a"), spec("tree_b")]);
        assert_eq!(registry.lookup("tree_a").unwrap().id, "tree_a");
        assert!(registry.lookup("missing").is_none());
        assert_eq!(registry.len(), 2);
    }
}
