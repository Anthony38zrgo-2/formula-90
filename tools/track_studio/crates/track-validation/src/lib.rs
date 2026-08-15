//! Unified track validation around the domain `Diagnostic` type.
//!
//! This crate is the single validation authority for domain and geometry rules
//! that do not require the asset registry. It combines the domain's basic
//! invariants with geometric and gameplay rule checks so the UI, the build gate
//! and the importer all consume the same `Diagnostic` records.

use track_assets::AssetRegistry;
use track_domain::{Diagnostic, Severity, TrackDocument};
use track_geometry::{closed_polyline_self_intersects, Point2};

pub const MAX_BANKING_DEG: f64 = 60.0;
pub const MAX_ELEVATION_M: f64 = 100.0;
pub const MAX_CURVATURE_PER_M: f64 = 0.5;

/// Run all rules and return an ordered list of diagnostics.
pub fn validate(document: &TrackDocument) -> Vec<Diagnostic> {
    let mut diagnostics = document.validate();
    diagnostics.extend(validate_centerline_geometry(document));
    diagnostics.extend(validate_profiles(document));
    diagnostics.extend(validate_gameplay(document));
    diagnostics
}

/// Run all rules plus asset checks against a registry: unknown asset ids and
/// budget violations are reported as errors.
pub fn validate_with_registry(
    document: &TrackDocument,
    registry: &AssetRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = validate(document);
    diagnostics.extend(validate_assets(document, registry));
    diagnostics
}

fn validate_assets(document: &TrackDocument, registry: &AssetRegistry) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();

    for instance in &document.asset_instances {
        if registry.lookup(&instance.asset_id).is_none() {
            diagnostics.push(Diagnostic {
                code: "validation.asset.unknown".into(),
                severity: Severity::Error,
                object_id: Some(instance.id.clone()),
                station: None,
                message: format!("asset {} is not in the registry", instance.asset_id),
                metadata: Default::default(),
            });
            continue;
        }
        *counts.entry(instance.asset_id.clone()).or_insert(0) += 1;
    }

    for (asset_id, count) in &counts {
        let Some(spec) = registry.lookup(asset_id) else {
            continue;
        };
        let min = spec.budget.min_instances;
        let max = spec.budget.max_instances;
        if *count < min || (max > 0 && *count > max) {
            diagnostics.push(Diagnostic {
                code: "validation.asset.budget".into(),
                severity: Severity::Error,
                object_id: Some(asset_id.clone()),
                station: None,
                message: format!("asset {asset_id} count {count} outside budget [{min}, {max}]"),
                metadata: Default::default(),
            });
        }
    }

    diagnostics
}

/// Convenience helper: is the document valid with no diagnostics at all?
pub fn is_valid(document: &TrackDocument) -> bool {
    validate(document).is_empty()
}

fn validate_centerline_geometry(document: &TrackDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let points: Vec<Point2> = document
        .centerline
        .iter()
        .map(|point| Point2 {
            x: point.x,
            z: point.z,
        })
        .collect();
    if points.len() >= 3 && closed_polyline_self_intersects(&points) {
        diagnostics.push(Diagnostic {
            code: "validation.centerline.self_intersection".into(),
            severity: Severity::Error,
            object_id: None,
            station: None,
            message: "centerline self-intersects".into(),
            metadata: Default::default(),
        });
    }
    diagnostics
}

fn validate_profiles(document: &TrackDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for point in &document.profiles.banking {
        if point.value.abs() > MAX_BANKING_DEG {
            diagnostics.push(Diagnostic {
                code: "validation.banking.range".into(),
                severity: Severity::Error,
                object_id: None,
                station: Some(point.station),
                message: format!(
                    "banking value {} exceeds {} degrees",
                    point.value, MAX_BANKING_DEG
                ),
                metadata: Default::default(),
            });
        }
    }
    for point in &document.profiles.elevation {
        if point.value.abs() > MAX_ELEVATION_M {
            diagnostics.push(Diagnostic {
                code: "validation.elevation.range".into(),
                severity: Severity::Error,
                object_id: None,
                station: Some(point.station),
                message: format!(
                    "elevation value {} exceeds {} meters",
                    point.value, MAX_ELEVATION_M
                ),
                metadata: Default::default(),
            });
        }
    }
    for point in document
        .profiles
        .width_left
        .iter()
        .chain(document.profiles.width_right.iter())
    {
        if point.value <= 0.0 {
            diagnostics.push(Diagnostic {
                code: "validation.width.positive".into(),
                severity: Severity::Error,
                object_id: None,
                station: Some(point.station),
                message: "width profile value must be positive".into(),
                metadata: Default::default(),
            });
        }
    }
    diagnostics
}

fn validate_gameplay(document: &TrackDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let has_start_finish = document
        .gameplay
        .markers
        .iter()
        .any(|marker| marker.marker_type == "start_finish");
    if !has_start_finish {
        diagnostics.push(Diagnostic {
            code: "validation.gameplay.missing_start_finish".into(),
            severity: Severity::Warning,
            object_id: None,
            station: None,
            message: "no start/finish marker present".into(),
            metadata: Default::default(),
        });
    }

    let checkpoints: Vec<&track_domain::GameplayMarker> = document
        .gameplay
        .markers
        .iter()
        .filter(|marker| marker.marker_type == "checkpoint")
        .collect();
    for pair in checkpoints.windows(2) {
        if pair[1].station < pair[0].station {
            diagnostics.push(Diagnostic {
                code: "validation.gameplay.checkpoint_ordering".into(),
                severity: Severity::Error,
                object_id: Some(pair[1].id.clone()),
                station: Some(pair[1].station),
                message: "checkpoints are not ordered by station".into(),
                metadata: Default::default(),
            });
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use track_assets::{AssetRegistry, AssetSpec, Budget};
    use track_domain::{AssetInstance, ControlPoint, GameplayMarker, ProfilePoint, Profiles};

    fn document() -> TrackDocument {
        let mut document = TrackDocument::new("test");
        document.centerline = vec![
            ControlPoint {
                id: "a".into(),
                x: 0.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "b".into(),
                x: 10.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "c".into(),
                x: 10.0,
                z: 10.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "d".into(),
                x: 0.0,
                z: 10.0,
                handle_in: None,
                handle_out: None,
            },
        ];
        document
    }

    fn diagnostics_codes(diagnostics: &[Diagnostic]) -> Vec<String> {
        diagnostics.iter().map(|d| d.code.clone()).collect()
    }

    fn asset_spec(id: &str, min: u32, max: u32) -> AssetSpec {
        AssetSpec {
            id: id.into(),
            kind: "vegetation".into(),
            category: "trees".into(),
            source: None,
            source_sha256: None,
            dimensions_m: Default::default(),
            preview: None,
            collision_class: "none".into(),
            budget: Budget {
                min_instances: min,
                max_instances: max,
            },
            metadata: Default::default(),
        }
    }

    fn instance(id: &str, asset_id: &str) -> AssetInstance {
        AssetInstance {
            id: id.into(),
            asset_id: asset_id.into(),
            x: 1.0,
            z: 1.0,
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: Default::default(),
        }
    }

    #[test]
    fn unknown_asset_id_is_detected_against_registry() {
        let mut document = document();
        document
            .asset_instances
            .push(instance("i1", "missing_asset"));
        let registry = AssetRegistry::from_specs(vec![asset_spec("tree_a", 0, 10)]);
        let codes = diagnostics_codes(&validate_with_registry(&document, &registry));
        assert!(codes.contains(&"validation.asset.unknown".into()));
    }

    #[test]
    fn budget_violations_are_detected() {
        let mut document = document();
        document.asset_instances.push(instance("i1", "tree_a"));
        document.asset_instances.push(instance("i2", "tree_a"));
        let registry = AssetRegistry::from_specs(vec![asset_spec("tree_a", 0, 1)]);
        let codes = diagnostics_codes(&validate_with_registry(&document, &registry));
        assert!(codes.contains(&"validation.asset.budget".into()));
    }

    #[test]
    fn matching_assets_and_budget_produce_no_asset_errors() {
        let mut document = document();
        document.asset_instances.push(instance("i1", "tree_a"));
        let registry = AssetRegistry::from_specs(vec![asset_spec("tree_a", 1, 2)]);
        let codes = diagnostics_codes(&validate_with_registry(&document, &registry));
        assert!(!codes.contains(&"validation.asset.unknown".into()));
        assert!(!codes.contains(&"validation.asset.budget".into()));
    }

    #[test]
    fn simple_document_has_no_errors() {
        let document = document();
        assert!(!diagnostics_codes(&validate(&document))
            .contains(&"validation.centerline.self_intersection".into()));
    }

    #[test]
    fn self_intersecting_centerline_is_detected() {
        let mut document = document();
        document.centerline[1].z = 10.0;
        document.centerline[2].z = 0.0;
        assert!(diagnostics_codes(&validate(&document))
            .contains(&"validation.centerline.self_intersection".into()));
    }

    #[test]
    fn extreme_banking_and_elevation_are_detected() {
        let mut document = document();
        document.profiles = Profiles {
            banking: vec![ProfilePoint {
                station: 10.0,
                value: 75.0,
            }],
            elevation: vec![ProfilePoint {
                station: 20.0,
                value: 200.0,
            }],
            ..Default::default()
        };
        let codes = diagnostics_codes(&validate(&document));
        assert!(codes.contains(&"validation.banking.range".into()));
        assert!(codes.contains(&"validation.elevation.range".into()));
    }

    #[test]
    fn missing_start_finish_is_a_warning_and_bad_checkpoint_order_an_error() {
        let mut document = document();
        document.gameplay.markers = vec![
            GameplayMarker {
                id: "cp1".into(),
                marker_type: "checkpoint".into(),
                station: 50.0,
                x: 0.0,
                z: 0.0,
                properties: Default::default(),
            },
            GameplayMarker {
                id: "cp2".into(),
                marker_type: "checkpoint".into(),
                station: 20.0,
                x: 0.0,
                z: 0.0,
                properties: Default::default(),
            },
        ];
        let codes = diagnostics_codes(&validate(&document));
        assert!(codes.contains(&"validation.gameplay.missing_start_finish".into()));
        assert!(codes.contains(&"validation.gameplay.checkpoint_ordering".into()));
    }
}
