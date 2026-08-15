//! Legacy SVG importer.
//!
//! Reads the restricted F90 Track SVG profile and maps its `data-role` elements
//! into a semantic [`TrackDocument`], producing `Diagnostic` records for any
//! element that cannot be faithfully represented. SVG is treated as a legacy
//! import source; it is never the editable authority after this step.

use roxmltree::{Document, Node};
use serde_json::json;
use std::collections::BTreeMap;
use track_domain::{
    AssetInstance, ControlPoint, Diagnostic, Point2, ProfilePoint, Region, Severity, TrackDocument,
};
use track_geometry::parse_path;

pub const FLATTEN_TOLERANCE_M: f64 = 0.1;

/// Result of importing an SVG string.
#[derive(Debug)]
pub struct ImportedTrack {
    pub document: TrackDocument,
    pub diagnostics: Vec<Diagnostic>,
}

impl ImportedTrack {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

/// Parse an untrusted SVG into a `TrackDocument`. The returned document is a
/// compatibility representation; validation of the result is the caller's
/// responsibility before it is saved or built.
pub fn import_svg(svg: &str) -> ImportedTrack {
    let mut diagnostics = Vec::new();
    let parsed = match Document::parse(svg) {
        Ok(parsed) => parsed,
        Err(error) => {
            diagnostics.push(severity_error(
                "import.svg.parse",
                format!("invalid SVG XML: {error}"),
            ));
            return ImportedTrack {
                document: TrackDocument::new("untitled"),
                diagnostics,
            };
        }
    };
    let root = parsed.root_element();
    if !root.has_tag_name("svg") {
        diagnostics.push(severity_error(
            "import.svg.root",
            "SVG root element is missing",
        ));
        return ImportedTrack {
            document: TrackDocument::new("untitled"),
            diagnostics,
        };
    }

    let track_id = root
        .attribute("data-track-id")
        .unwrap_or("untitled")
        .to_owned();
    let mut document = TrackDocument::new(track_id);

    import_centerline(&mut document, &root, &mut diagnostics);
    import_road(&mut document, &root, &mut diagnostics);
    import_profiles(&mut document, &root, "banking", &mut diagnostics);
    import_profiles(&mut document, &root, "elevation", &mut diagnostics);
    import_terrain(&mut document, &root, &mut diagnostics);
    import_assets(&mut document, &root, &mut diagnostics);
    note_unimported(&root, &mut diagnostics);

    ImportedTrack {
        document,
        diagnostics,
    }
}

fn import_centerline(document: &mut TrackDocument, root: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let Some(centerline) = root
        .descendants()
        .find(|node| node.attribute("data-role") == Some("centerline"))
    else {
        diagnostics.push(severity_error(
            "import.centerline.missing",
            "centerline path is missing",
        ));
        return;
    };
    let Some(path) = centerline.attribute("d") else {
        diagnostics.push(severity_error(
            "import.centerline.missing_d",
            "centerline path has no d attribute",
        ));
        return;
    };
    let points = match parse_path(path, FLATTEN_TOLERANCE_M) {
        Ok(subpaths) => match subpaths.first() {
            Some(points) => {
                // Drop the closing duplicate point and store as control points.
                if points.first() == points.last() {
                    points.split_at(points.len().saturating_sub(1)).0.to_vec()
                } else {
                    points.clone()
                }
            }
            None => {
                diagnostics.push(severity_error(
                    "import.centerline.empty",
                    "centerline path produced no geometry",
                ));
                return;
            }
        },
        Err(_) => {
            diagnostics.push(severity_error(
                "import.centerline.parse",
                "centerline path could not be parsed",
            ));
            return;
        }
    };
    for (index, point) in points.iter().enumerate() {
        document.centerline.push(ControlPoint {
            id: format!("p{index}"),
            x: point.x,
            z: point.z,
            handle_in: None,
            handle_out: None,
        });
    }
}

fn import_road(document: &mut TrackDocument, root: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let Some(road) = root
        .descendants()
        .find(|node| node.attribute("data-role") == Some("road"))
    else {
        diagnostics.push(severity_warning(
            "import.road.missing",
            "road element is missing; using default width",
        ));
        return;
    };
    let width = numeric_attribute(road, "data-width-m", 12.0);
    let half = width / 2.0;
    document.profiles.width_left.push(ProfilePoint {
        station: 0.0,
        value: half,
    });
    document.profiles.width_right.push(ProfilePoint {
        station: 0.0,
        value: half,
    });
}

fn import_profiles(
    document: &mut TrackDocument,
    root: &Node,
    role: &str,
    _diagnostics: &mut Vec<Diagnostic>,
) {
    let target = match role {
        "banking" => &mut document.profiles.banking,
        "elevation" => &mut document.profiles.elevation,
        _ => return,
    };
    for node in root
        .descendants()
        .filter(|node| node.attribute("data-role") == Some(role))
    {
        let station = numeric_attribute(node, "data-s-m", 0.0);
        let value = if role == "banking" {
            numeric_attribute(node, "data-degrees", 0.0)
        } else {
            numeric_attribute(node, "data-height-m", 0.0)
        };
        target.push(ProfilePoint { station, value });
    }
}

fn import_terrain(document: &mut TrackDocument, root: &Node, diagnostics: &mut Vec<Diagnostic>) {
    for (index, node) in root
        .descendants()
        .filter(|node| node.attribute("data-role") == Some("terrain-zone"))
        .enumerate()
    {
        let Some(points) = node.attribute("points") else {
            diagnostics.push(severity_error(
                "import.terrain.missing_points",
                "terrain-zone has no points",
            ));
            continue;
        };
        match parse_points(points) {
            Some(polygon) => {
                let kind = node.attribute("data-kind").unwrap_or("grass").to_owned();
                document.terrain_regions.push(Region {
                    id: format!("terrain_{index}"),
                    region_type: kind,
                    polygon,
                    properties: BTreeMap::new(),
                });
            }
            None => diagnostics.push(severity_error(
                "import.terrain.points",
                "terrain-zone points are invalid",
            )),
        }
    }
}

fn import_assets(document: &mut TrackDocument, root: &Node, diagnostics: &mut Vec<Diagnostic>) {
    for node in root
        .descendants()
        .filter(|node| node.attribute("data-role") == Some("asset-instance"))
    {
        let Some(instance_id) = node.attribute("data-instance-id") else {
            diagnostics.push(severity_warning(
                "import.asset.no_instance_id",
                "asset-instance without data-instance-id is skipped",
            ));
            continue;
        };
        let Some(asset_id) = node.attribute("data-asset-id") else {
            diagnostics.push(severity_warning(
                "import.asset.no_asset_id",
                format!("asset {instance_id} has no data-asset-id"),
            ));
            continue;
        };
        let x = numeric_attribute(node, "cx", 0.0);
        let z = numeric_attribute(node, "cy", 0.0);
        let yaw = numeric_attribute(node, "data-yaw-rad", 0.0);
        let scale = numeric_attribute(node, "data-scale", 1.0);
        document.asset_instances.push(AssetInstance {
            id: instance_id.to_owned(),
            asset_id: asset_id.to_owned(),
            x,
            z,
            rotation: [0.0, yaw, 0.0],
            scale: [scale, scale, scale],
            properties: BTreeMap::from([(
                "kind".into(),
                json!(node.attribute("data-kind").unwrap_or("card")),
            )]),
        });
    }
}

fn note_unimported(root: &Node, diagnostics: &mut Vec<Diagnostic>) {
    for role in ["barrier", "vegetation-region"] {
        let count = root
            .descendants()
            .filter(|node| node.attribute("data-role") == Some(role))
            .count();
        if count > 0 {
            diagnostics.push(Diagnostic {
                code: format!("import.{role}.deferred"),
                severity: Severity::Info,
                object_id: None,
                station: None,
                message: format!("{count} {role} element(s) not imported yet"),
                metadata: BTreeMap::new(),
            });
        }
    }
}

fn parse_points(text: &str) -> Option<Vec<Point2>> {
    let mut numbers = Vec::new();
    for token in text.replace(',', " ").split_whitespace() {
        let value: f64 = token.parse().ok()?;
        if !value.is_finite() {
            return None;
        }
        numbers.push(value);
    }
    if numbers.len() < 6 || numbers.len() % 2 != 0 {
        return None;
    }
    Some(
        numbers
            .chunks(2)
            .map(|pair| Point2 {
                x: pair[0],
                z: pair[1],
            })
            .collect(),
    )
}

fn numeric_attribute(node: Node, name: &str, default: f64) -> f64 {
    node.attribute(name)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

fn severity_error(code: impl Into<String>, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        object_id: None,
        station: None,
        message: message.into(),
        metadata: BTreeMap::new(),
    }
}

fn severity_warning(code: impl Into<String>, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Warning,
        object_id: None,
        station: None,
        message: message.into(),
        metadata: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_SVG: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 60" data-track-id="fixture_minimal" data-road-surface-elevation-m="0.025">
  <g data-layer="centerline">
    <path data-role="centerline" d="M 10 30 L 30 10 L 70 10 L 90 30 L 70 50 L 30 50 Z"/>
  </g>
  <g data-layer="road">
    <path data-role="road" data-width-m="12"/>
    <path data-role="banking" data-s-m="40" data-degrees="3.0"/>
    <path data-role="elevation" data-s-m="120" data-height-m="1.5"/>
  </g>
  <g data-layer="terrain">
    <polygon data-role="terrain-zone" data-kind="grass" data-name="infiel" points="0,0 100,0 100,60 0,60"/>
  </g>
  <g data-layer="barrier">
    <line data-role="barrier" data-kind="guardrail" x1="4" y1="26" x2="4" y2="34"/>
  </g>
  <g data-layer="assets">
    <circle data-role="asset-instance" data-instance-id="tree_a" data-asset-id="tree_v2_01" data-kind="tree" cx="8" cy="8" r="0.4" data-scale="1.0" data-yaw-rad="0.3"/>
  </g>
</svg>
"#;

    #[test]
    fn import_populates_centerline_road_profiles_terrain_and_assets() {
        let imported = import_svg(MINIMAL_SVG);
        let document = &imported.document;

        assert_eq!(document.track_id, "fixture_minimal");
        assert!(
            document.centerline.len() >= 3,
            "centerline has control points"
        );
        assert_eq!(document.profiles.banking.len(), 1);
        assert_eq!(document.profiles.banking[0].value, 3.0);
        assert_eq!(document.profiles.elevation.len(), 1);
        assert_eq!(document.profiles.elevation[0].value, 1.5);
        assert_eq!(document.profiles.width_left[0].value, 6.0);
        assert_eq!(document.profiles.width_right[0].value, 6.0);
        assert_eq!(document.terrain_regions.len(), 1);
        assert_eq!(document.asset_instances.len(), 1);
        assert_eq!(document.asset_instances[0].asset_id, "tree_v2_01");
    }

    #[test]
    fn import_marks_deferred_elements_and_remains_valid() {
        let imported = import_svg(MINIMAL_SVG);
        assert!(imported
            .diagnostics
            .iter()
            .any(|d| d.code == "import.barrier.deferred"));
        assert!(!imported.has_errors());
        assert!(imported.document.validate().is_empty());
    }

    #[test]
    fn missing_centerline_reports_error() {
        let svg =
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" data-track-id="t"/> "#;
        let imported = import_svg(svg);
        assert!(imported
            .diagnostics
            .iter()
            .any(|d| d.code == "import.centerline.missing"));
        assert!(imported.has_errors());
    }
}
