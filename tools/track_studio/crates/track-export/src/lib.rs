//! Deterministic export of a `TrackDocument`.
//!
//! Produces two generated JSON representations from the same semantic source:
//!
//! - `track.export.json` (canonical review/interchange document);
//! - `track_runtime_metadata.json` (Godot runtime metadata: bounds, minimap,
//!   start/finish and gameplay markers).
//!
//! Both are byte-deterministic for the same input and are generated output;
//! they are never edited by hand.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use track_domain::TrackDocument;
use track_geometry::{
    closed_polyline_length, resample_closed_polyline, sample_closed_centerline, Point2,
};

pub const RUNTIME_METADATA_SCHEMA_VERSION: u32 = 1;
pub const MINIMAP_SPACING_M: f64 = 10.0;

#[derive(Debug)]
pub enum ExportError {
    Serialization(serde_json::Error),
    Geometry(track_geometry::GeometryError),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialization(error) => write!(formatter, "serialization: {error}"),
            Self::Geometry(error) => write!(formatter, "geometry: {error}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<serde_json::Error> for ExportError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

impl From<track_geometry::GeometryError> for ExportError {
    fn from(error: track_geometry::GeometryError) -> Self {
        Self::Geometry(error)
    }
}

/// Deterministic canonical export of the whole document.
pub fn export_canonical(document: &TrackDocument) -> Result<Vec<u8>, ExportError> {
    Ok(document.canonical_json()?)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Bounds {
    pub min_x: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_z: f64,
}

/// Computed runtime metadata for a track.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RuntimeMetadata {
    pub schema_version: u32,
    pub track_id: String,
    pub bounds: Option<Bounds>,
    pub length_m: f64,
    pub minimap_polyline: Vec<[f64; 2]>,
    pub start_finish: Option<StartFinish>,
    pub markers: BTreeMap<String, Vec<MarkerRef>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StartFinish {
    pub station: f64,
    pub x: f64,
    pub z: f64,
    pub heading_rad: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MarkerRef {
    pub id: String,
    pub station: f64,
    pub x: f64,
    pub z: f64,
}

impl RuntimeMetadata {
    pub fn compute(document: &TrackDocument) -> Result<Self, ExportError> {
        let points: Vec<Point2> = document
            .centerline
            .iter()
            .map(|point| Point2 {
                x: point.x,
                z: point.z,
            })
            .collect();

        let bounds = bounds(&points);
        let length_m = if points.len() >= 3 {
            closed_polyline_length(&points)?
        } else {
            0.0
        };
        let minimap_polyline = if points.len() >= 3 {
            resample_closed_polyline(&points, MINIMAP_SPACING_M)?
                .iter()
                .map(|p| [round6(p.x), round6(p.z)])
                .collect()
        } else {
            Vec::new()
        };
        let start_finish = compute_start_finish(document, &points)?;

        let mut markers: BTreeMap<String, Vec<MarkerRef>> = BTreeMap::new();
        for marker in &document.gameplay.markers {
            markers
                .entry(marker.marker_type.clone())
                .or_default()
                .push(MarkerRef {
                    id: marker.id.clone(),
                    station: round6(marker.station),
                    x: round6(marker.x),
                    z: round6(marker.z),
                });
        }

        Ok(RuntimeMetadata {
            schema_version: RUNTIME_METADATA_SCHEMA_VERSION,
            track_id: document.track_id.clone(),
            bounds,
            length_m: round6(length_m),
            minimap_polyline,
            start_finish,
            markers,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ExportError> {
        let value = serde_json::to_value(self)?;
        let object = value
            .as_object()
            .map(|map| Value::Object(map.clone()))
            .unwrap_or(value);
        let mut buffer = Vec::new();
        let mut serializer = serde_json::Serializer::new(&mut buffer);
        object.serialize(&mut serializer)?;
        Ok(buffer)
    }
}

fn bounds(points: &[Point2]) -> Option<Bounds> {
    let mut iter = points.iter();
    let first = iter.next()?;
    let mut min_x = first.x;
    let mut min_z = first.z;
    let mut max_x = first.x;
    let mut max_z = first.z;
    for point in iter {
        min_x = min_x.min(point.x);
        min_z = min_z.min(point.z);
        max_x = max_x.max(point.x);
        max_z = max_z.max(point.z);
    }
    Some(Bounds {
        min_x: round6(min_x),
        min_z: round6(min_z),
        max_x: round6(max_x),
        max_z: round6(max_z),
    })
}

fn compute_start_finish(
    document: &TrackDocument,
    points: &[Point2],
) -> Result<Option<StartFinish>, ExportError> {
    let Some(marker) = document
        .gameplay
        .markers
        .iter()
        .find(|marker| marker.marker_type == "start_finish")
    else {
        return Ok(None);
    };
    let controls: Vec<track_domain::ControlPoint> = document.centerline.clone();
    let station = marker.station;
    let heading_rad = if points.len() >= 3 {
        let sample = sample_closed_centerline(&controls, station)?;
        sample.tangent.z.atan2(sample.tangent.x)
    } else {
        0.0
    };
    Ok(Some(StartFinish {
        station: round6(station),
        x: round6(marker.x),
        z: round6(marker.z),
        heading_rad: round6(heading_rad),
    }))
}

fn round6(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

/// Convenience that returns the runtime metadata as a JSON document for
/// `track_runtime_metadata.json`.
pub fn export_runtime_metadata(document: &TrackDocument) -> Result<Vec<u8>, ExportError> {
    RuntimeMetadata::compute(document)?.to_bytes()
}

/// Build the `track_runtime_metadata.json` value directly for tests/tools.
pub fn runtime_metadata_json(document: &TrackDocument) -> Result<Value, ExportError> {
    Ok(serde_json::to_value(RuntimeMetadata::compute(document)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use track_domain::{ControlPoint, GameplayMarker};

    fn square_document() -> TrackDocument {
        let mut document = TrackDocument::new("square");
        document.centerline = vec![
            ControlPoint {
                id: "p0".into(),
                x: 0.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "p1".into(),
                x: 10.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "p2".into(),
                x: 10.0,
                z: 10.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "p3".into(),
                x: 0.0,
                z: 10.0,
                handle_in: None,
                handle_out: None,
            },
        ];
        document.gameplay.markers.push(GameplayMarker {
            id: "sf".into(),
            marker_type: "start_finish".into(),
            station: 0.0,
            x: 0.0,
            z: 0.0,
            properties: Default::default(),
        });
        document
    }

    #[test]
    fn runtime_metadata_has_bounds_length_and_minimap() {
        let document = square_document();
        let metadata = RuntimeMetadata::compute(&document).unwrap();

        let bounds = metadata.bounds.unwrap();
        assert_eq!(bounds.min_x, 0.0);
        assert_eq!(bounds.max_x, 10.0);
        assert_eq!(bounds.max_z, 10.0);
        assert_eq!(metadata.length_m, 40.0);
        assert!(!metadata.minimap_polyline.is_empty());
    }

    #[test]
    fn start_finish_is_derived_from_geometry() {
        let document = square_document();
        let metadata = RuntimeMetadata::compute(&document).unwrap();
        let start = metadata.start_finish.unwrap();
        assert_eq!(start.station, 0.0);
        // Heading along the first edge (positive x) => atan2(0, 1) = 0.
        assert_eq!(start.heading_rad, 0.0);
    }

    #[test]
    fn runtime_metadata_is_byte_deterministic() {
        let document = square_document();
        let a = export_runtime_metadata(&document).unwrap();
        let b = export_runtime_metadata(&document).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn export_canonical_is_valid_json_and_deterministic() {
        let document = square_document();
        let a = export_canonical(&document).unwrap();
        let b = export_canonical(&document).unwrap();
        assert_eq!(a, b);
        serde_json::from_slice::<Value>(&a).expect("canonical export is valid JSON");
    }
}
