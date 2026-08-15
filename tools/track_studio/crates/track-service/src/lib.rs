//! Application service layer for Track Studio.
//!
//! This is the Rust application service behind the typed IPC surface
//! (`TS-120`). It owns the current `TrackDocument`, the storage connection, the
//! in-memory command history, the asset registry and the build graph, and
//! exposes the command families defined in `CONTRACTS_V0` (project lifecycle,
//! snapshot/validate, commands/undo/redo, export/BuildIR, build plan/run).
//!
//! It is intentionally free of Tauri: the thin `#[tauri::command]` wrappers map
//! onto these methods. Keeping the logic here makes it fully unit-testable.

use serde::Serialize;
use std::path::Path;
use track_assets::AssetRegistry;
use track_build::build_graph::{BuildGraph, BuildPlan, Subsystem};
use track_build::{compile, BuildIR};
use track_commands::{Command, CommandHistory};
use track_domain::{Diagnostic, TrackDocument};
use track_export::RuntimeMetadata;
use track_storage::{StorageError, TrackStorage};
use track_validation::validate_with_registry;

#[derive(Debug)]
pub enum ServiceError {
    Storage(StorageError),
    Command(track_commands::CommandError),
    Build(track_build::BuildError),
    Json(serde_json::Error),
    NoProject,
    InvalidProject(Vec<Diagnostic>),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "storage: {error}"),
            Self::Command(error) => write!(formatter, "command: {error}"),
            Self::Build(error) => write!(formatter, "build: {error}"),
            Self::Json(error) => write!(formatter, "json: {error}"),
            Self::NoProject => write!(formatter, "no project is open"),
            Self::InvalidProject(_) => write!(formatter, "document is invalid"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<StorageError> for ServiceError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<track_commands::CommandError> for ServiceError {
    fn from(error: track_commands::CommandError) -> Self {
        Self::Command(error)
    }
}

impl From<track_build::BuildError> for ServiceError {
    fn from(error: track_build::BuildError) -> Self {
        Self::Build(error)
    }
}

impl From<serde_json::Error> for ServiceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// A snapshot of the current project for the UI.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProjectSnapshot {
    pub track_id: String,
    pub canonical_json: serde_json::Value,
    pub runtime_metadata: Option<RuntimeMetadata>,
    pub diagnostics: Vec<Diagnostic>,
}

/// The application service: one owner of the editable state.
pub struct AppService {
    storage: Option<TrackStorage>,
    registry: Option<AssetRegistry>,
    document: TrackDocument,
    history: CommandHistory,
    build_graph: BuildGraph,
}

impl AppService {
    /// Create an in-memory service around a starting document.
    pub fn new(document: TrackDocument, registry: Option<AssetRegistry>) -> Self {
        Self {
            storage: None,
            registry,
            document,
            history: CommandHistory::new(),
            build_graph: BuildGraph::empty(),
        }
    }

    /// Open a `.f90track` project from disk.
    pub fn open_project(
        path: impl AsRef<Path>,
        registry: Option<AssetRegistry>,
    ) -> Result<Self, ServiceError> {
        let storage = TrackStorage::open(path)?;
        let track_id = storage.load_any_track_id()?;
        let document = storage.load(&track_id)?.ok_or(ServiceError::NoProject)?;
        Ok(Self {
            storage: Some(storage),
            registry,
            document,
            history: CommandHistory::new(),
            build_graph: BuildGraph::empty(),
        })
    }

    /// Persist the current document to the open project.
    pub fn save_project(&self) -> Result<(), ServiceError> {
        let storage = self.storage.as_ref().ok_or(ServiceError::NoProject)?;
        storage.save(&self.document)?;
        Ok(())
    }

    pub fn track_id(&self) -> &str {
        &self.document.track_id
    }

    pub fn document(&self) -> &TrackDocument {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut TrackDocument {
        &mut self.document
    }

    pub fn reference_layers(&self) -> &[track_domain::ReferenceLayer] {
        &self.document.reference_layers
    }

    /// Add a reference layer to the document.
    pub fn add_reference_layer(
        &mut self,
        layer: track_domain::ReferenceLayer,
    ) -> Result<(), ServiceError> {
        self.document.reference_layers.push(layer);
        Ok(())
    }

    /// Calibrate a reference layer through the undoable command history.
    pub fn calibrate_reference(
        &mut self,
        layer_id: String,
        image_distance_px: f64,
        real_distance_m: f64,
    ) -> Result<(), ServiceError> {
        self.execute(Box::new(track_commands::SetReferenceCalibration::new(
            layer_id,
            image_distance_px,
            real_distance_m,
        )))
    }

    /// World distance between two points (authoring measurement).
    pub fn measure(&self, a: [f64; 2], b: [f64; 2]) -> f64 {
        track_geometry::world_distance(
            &track_domain::Point2 { x: a[0], z: a[1] },
            &track_domain::Point2 { x: b[0], z: b[1] },
        )
    }

    /// Snap a point to a regular world grid.
    pub fn snap(&self, point: [f64; 2], grid_m: f64) -> [f64; 2] {
        let snapped = track_geometry::snap_to_grid(
            &track_domain::Point2 {
                x: point[0],
                z: point[1],
            },
            grid_m,
        );
        [snapped.x, snapped.z]
    }

    /// Validate the current document, including asset checks.
    pub fn validate(&self) -> Vec<Diagnostic> {
        match &self.registry {
            Some(registry) => validate_with_registry(&self.document, registry),
            None => track_validation::validate(&self.document),
        }
    }

    /// Produce a snapshot for the UI.
    pub fn snapshot(&self) -> Result<ProjectSnapshot, ServiceError> {
        let canonical_json = serde_json::from_slice(&self.document.canonical_json()?)?;
        let runtime_metadata = track_export::RuntimeMetadata::compute(&self.document).ok();
        Ok(ProjectSnapshot {
            track_id: self.document.track_id.clone(),
            canonical_json,
            runtime_metadata,
            diagnostics: self.validate(),
        })
    }

    /// Apply a command and record it in the undo history.
    pub fn execute(&mut self, command: Box<dyn Command + Send>) -> Result<(), ServiceError> {
        self.history.execute(command, &mut self.document)?;
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), ServiceError> {
        self.history.undo(&mut self.document)?;
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), ServiceError> {
        self.history.redo(&mut self.document)?;
        Ok(())
    }

    pub fn undo_depth(&self) -> usize {
        self.history.undo_depth()
    }

    /// Compile the current document into BuildIR.
    pub fn compile_build_ir(&self) -> Result<BuildIR, ServiceError> {
        Ok(compile(&self.document, self.registry.as_ref())?)
    }

    /// Compute the incremental build plan (which subsystems are dirty).
    pub fn build_plan(&self) -> Result<BuildPlan, ServiceError> {
        Ok(self
            .build_graph
            .plan(&self.document, self.registry.as_ref())?)
    }

    /// Dry-run the build graph: returns the dirty subsystems without executing
    /// any external process. A real process runner is wired through the IPC layer.
    pub fn dry_run_build(&self) -> Result<Vec<Subsystem>, ServiceError> {
        Ok(self
            .build_graph
            .dirty(&self.document, self.registry.as_ref())?)
    }
}

impl Default for AppService {
    fn default() -> Self {
        Self::new(TrackDocument::new("untitled"), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use track_domain::{ControlPoint, Point2};

    fn document() -> TrackDocument {
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
        document
    }

    #[test]
    fn snapshot_exposes_canonical_and_metadata() {
        let service = AppService::new(document(), None);
        let snapshot = service.snapshot().unwrap();
        assert_eq!(snapshot.track_id, "square");
        assert_eq!(snapshot.canonical_json["track_id"], "square");
        assert!(snapshot.runtime_metadata.is_some());
    }

    #[test]
    fn execute_undo_redo_updates_history() {
        let mut service = AppService::new(document(), None);
        let target = Point2 { x: 99.0, z: 55.0 };
        service
            .execute(Box::new(track_commands::MoveControlPoint::new(
                "p1",
                target.clone(),
            )))
            .unwrap();
        assert_eq!(service.document().centerline[1].x, 99.0);
        assert_eq!(service.undo_depth(), 1);

        service.undo().unwrap();
        assert_eq!(service.document().centerline[1].x, 10.0);

        service.redo().unwrap();
        assert_eq!(service.document().centerline[1].x, 99.0);
    }

    #[test]
    fn validation_reports_asset_problems_when_registry_given() {
        let mut document = document();
        document.asset_instances.push(track_domain::AssetInstance {
            id: "a".into(),
            asset_id: "missing".into(),
            x: 1.0,
            z: 1.0,
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: BTreeMap::new(),
        });
        let registry = AssetRegistry::from_specs(vec![]);
        let service = AppService::new(document, Some(registry));
        assert!(service
            .validate()
            .iter()
            .any(|d| d.code == "validation.asset.unknown"));
    }

    #[test]
    fn save_and_reopen_preserves_document() {
        let path =
            std::env::temp_dir().join(format!("f90_service_{}.f90track", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let storage = TrackStorage::open(&path).unwrap();
            storage.save(&document()).unwrap();
        }
        let service = AppService::open_project(&path, None).unwrap();
        assert_eq!(service.track_id(), "square");
        assert_eq!(service.document().centerline.len(), 4);
        drop(service);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn build_plan_and_dry_run_work() {
        let service = AppService::new(document(), None);
        let plan = service.build_plan().unwrap();
        assert_eq!(plan.hashes.len(), 5);
        // Empty cache => everything is dirty on the first run.
        assert_eq!(service.dry_run_build().unwrap().len(), 5);
    }
}
