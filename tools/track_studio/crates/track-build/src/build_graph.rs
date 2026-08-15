//! Incremental build graph and orchestration.
//!
//! A `BuildPlan` records the per-subsystem hashes of a compiled BuildIR. The
//! `BuildGraph` compares them against a `BuildCache` to decide which subsystems
//! need to be rebuilt, writes the `track.build.json` contract, and invokes an
//! external runner (Blender/Godot) only for dirty subsystems. A failed
//! candidate never replaces a previously activated build.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use track_assets::AssetRegistry;
use track_domain::TrackDocument;

use crate::{compile, BuildError, BuildIR};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Subsystem {
    Geometry,
    Terrain,
    Vegetation,
    Objects,
    Gameplay,
}

impl Subsystem {
    pub const ALL: [Subsystem; 5] = [
        Subsystem::Geometry,
        Subsystem::Terrain,
        Subsystem::Vegetation,
        Subsystem::Objects,
        Subsystem::Gameplay,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Subsystem::Geometry => "geometry",
            Subsystem::Terrain => "terrain",
            Subsystem::Vegetation => "vegetation",
            Subsystem::Objects => "objects",
            Subsystem::Gameplay => "gameplay",
        }
    }
}

/// Deterministic per-subsystem hashes of a compiled build.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildPlan {
    pub hashes: BTreeMap<Subsystem, String>,
}

impl BuildPlan {
    pub fn from_build_ir(build_ir: &BuildIR) -> Self {
        let hashes = BTreeMap::from([
            (Subsystem::Geometry, build_ir.geometry_hash.clone()),
            (Subsystem::Terrain, build_ir.terrain_hash.clone()),
            (Subsystem::Vegetation, build_ir.vegetation_hash.clone()),
            (Subsystem::Objects, build_ir.objects_hash.clone()),
            (Subsystem::Gameplay, build_ir.gameplay_hash.clone()),
        ]);
        Self { hashes }
    }
}

/// Persisted record of the last successfully built subsystem hashes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildCache {
    pub hashes: BTreeMap<Subsystem, String>,
}

impl BuildCache {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn load(path: impl AsRef<Path>) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        std::fs::write(path, bytes)
    }
}

/// Result of an orchestrated build run.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildResult {
    pub dirty: Vec<Subsystem>,
    pub track_build_bytes: Vec<u8>,
}

/// Abstraction over launching Blender/Godot so the graph is unit-testable.
pub trait Runner {
    /// Run an external build process for a subsystem. Returns Ok when it
    /// succeeds (exit code 0).
    fn run(&self, subsystem: Subsystem, args: &[String]) -> Result<(), BuildError>;
}

/// Runner that executes a real process (e.g. Blender headless) via std.
pub struct ProcessRunner {
    executable: String,
}

impl ProcessRunner {
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
        }
    }
}

impl Runner for ProcessRunner {
    fn run(&self, subsystem: Subsystem, args: &[String]) -> Result<(), BuildError> {
        let status = std::process::Command::new(&self.executable)
            .args(args)
            .status()
            .map_err(|error| BuildError::Process(format!("{}: {error}", subsystem.label())))?;
        if status.success() {
            Ok(())
        } else {
            Err(BuildError::Process(format!(
                "{}: exited with {status}",
                subsystem.label()
            )))
        }
    }
}

/// Build graph that decides what to rebuild and orchestrates the runner.
#[derive(Debug, Clone, Default)]
pub struct BuildGraph {
    cache: BuildCache,
}

impl BuildGraph {
    /// Graph with a cache of the last successfully built subsystem hashes.
    pub fn new(cache: BuildCache) -> Self {
        Self { cache }
    }

    /// Convenience: a graph with an empty cache.
    pub fn empty() -> Self {
        Self::new(BuildCache::empty())
    }

    pub fn cache(&self) -> &BuildCache {
        &self.cache
    }

    /// Compute which subsystems changed since the cache.
    pub fn dirty(
        &self,
        document: &TrackDocument,
        registry: Option<&AssetRegistry>,
    ) -> Result<Vec<Subsystem>, BuildError> {
        let plan = self.plan(document, registry)?;
        Ok(self.dirty_for(&plan))
    }

    pub fn plan(
        &self,
        document: &TrackDocument,
        registry: Option<&AssetRegistry>,
    ) -> Result<BuildPlan, BuildError> {
        let build_ir = compile(document, registry)?;
        Ok(BuildPlan::from_build_ir(&build_ir))
    }

    pub fn dirty_for(&self, plan: &BuildPlan) -> Vec<Subsystem> {
        Subsystem::ALL
            .iter()
            .copied()
            .filter(|subsystem| {
                let Some(hash) = plan.hashes.get(subsystem) else {
                    return true;
                };
                self.cache.hashes.get(subsystem) != Some(hash)
            })
            .collect()
    }

    /// Orchestrate a build: compile, write `track.build.json`, run dirty
    /// subsystems, and update the cache.
    pub fn run<R: Runner>(
        &self,
        document: &TrackDocument,
        registry: Option<&AssetRegistry>,
        runner: &R,
        build_json_path: impl AsRef<Path>,
    ) -> Result<(BuildResult, BuildCache), BuildError> {
        let build_ir = compile(document, registry)?;
        let plan = BuildPlan::from_build_ir(&build_ir);
        let dirty = self.dirty_for(&plan);
        let bytes = build_ir.to_bytes()?;

        std::fs::write(build_json_path.as_ref(), &bytes)
            .map_err(|error| BuildError::Process(format!("write track.build.json: {error}")))?;

        let mut cache = self.cache.clone();
        for subsystem in &dirty {
            let args = build_subsystem_args(*subsystem, build_json_path.as_ref());
            runner.run(*subsystem, &args)?;
            if let Some(hash) = plan.hashes.get(subsystem) {
                cache.hashes.insert(*subsystem, hash.clone());
            }
        }

        Ok((
            BuildResult {
                dirty,
                track_build_bytes: bytes,
            },
            cache,
        ))
    }
}

fn build_subsystem_args(subsystem: Subsystem, build_json: &Path) -> Vec<String> {
    vec![
        "--background".to_owned(),
        "--factory-startup".to_owned(),
        format!("--{}", subsystem.label()),
        build_json.to_string_lossy().into_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use track_domain::{ControlPoint, Point2, VegetationRegion};

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
        document
    }

    #[derive(Default)]
    struct FakeRunner {
        fail: bool,
    }

    impl Runner for FakeRunner {
        fn run(&self, subsystem: Subsystem, _args: &[String]) -> Result<(), BuildError> {
            if self.fail {
                return Err(BuildError::Process(format!("{} failed", subsystem.label())));
            }
            Ok(())
        }
    }

    #[test]
    fn unchanged_document_has_no_dirty_subsystems() {
        let document = square_document();
        let graph = BuildGraph::new(BuildCache::empty());
        let plan = graph.plan(&document, None).unwrap();
        let mut cache = BuildCache::empty();
        for (subsystem, hash) in &plan.hashes {
            cache.hashes.insert(*subsystem, hash.clone());
        }
        let graph = BuildGraph::new(cache);
        assert!(graph.dirty(&document, None).unwrap().is_empty());
    }

    #[test]
    fn geometry_change_marks_only_geometry_dirty() {
        let document = square_document();
        let graph = BuildGraph::new(BuildCache::empty());
        let plan = graph.plan(&document, None).unwrap();
        let mut cache = BuildCache::empty();
        for (subsystem, hash) in &plan.hashes {
            cache.hashes.insert(*subsystem, hash.clone());
        }
        let graph = BuildGraph::new(cache);

        let mut changed = square_document();
        changed.centerline[1].x = 11.0;
        let dirty = graph.dirty(&changed, None).unwrap();
        assert!(dirty.contains(&Subsystem::Geometry));
        assert!(!dirty.contains(&Subsystem::Vegetation));
        assert!(!dirty.contains(&Subsystem::Objects));
    }

    #[test]
    fn vegetation_change_marks_only_vegetation_dirty() {
        let mut document = square_document();
        document.vegetation_regions.push(VegetationRegion {
            id: "trees".into(),
            polygon: vec![
                Point2 { x: 0.0, z: 0.0 },
                Point2 { x: 10.0, z: 0.0 },
                Point2 { x: 10.0, z: 10.0 },
                Point2 { x: 0.0, z: 10.0 },
            ],
            seed: 1,
            density: 1.0,
            asset_set: vec!["tree".into()],
            min_spacing: 2.0,
            scale_range: [0.5, 1.0],
            rotation_range: [0.0, 1.0],
            track_exclusion: 1.0,
        });
        let graph = BuildGraph::new(BuildCache::empty());
        let plan = graph.plan(&document, None).unwrap();
        let mut cache = BuildCache::empty();
        for (subsystem, hash) in &plan.hashes {
            cache.hashes.insert(*subsystem, hash.clone());
        }
        let graph = BuildGraph::new(cache);

        let mut changed = document.clone();
        changed.vegetation_regions[0].seed = 2;
        let dirty = graph.dirty(&changed, None).unwrap();
        assert!(dirty.contains(&Subsystem::Vegetation));
        assert!(!dirty.contains(&Subsystem::Geometry));
    }

    #[test]
    fn run_writes_track_build_json_and_updates_cache() {
        let document = square_document();
        let graph = BuildGraph::new(BuildCache::empty());
        let runner = FakeRunner::default();
        let path = std::env::temp_dir().join(format!("f90_graph_{}.json", std::process::id()));

        let (result, cache) = graph.run(&document, None, &runner, &path).unwrap();
        assert!(!result.dirty.is_empty());
        assert!(path.exists());
        assert_eq!(cache.hashes.len(), Subsystem::ALL.len());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn subsystem_args_are_deterministic() {
        let args = build_subsystem_args(Subsystem::Geometry, Path::new("track.build.json"));
        assert_eq!(args[0], "--background");
        assert!(args[2].starts_with("--geometry"));
        assert!(args[3].contains("track.build.json"));
    }

    #[test]
    fn failed_run_does_not_update_cache() {
        let document = square_document();
        let graph = BuildGraph::new(BuildCache::empty());
        let runner = FakeRunner { fail: true };
        let path = std::env::temp_dir().join(format!("f90_graph_fail_{}.json", std::process::id()));
        assert!(graph.run(&document, None, &runner, &path).is_err());
        // A failed candidate never becomes part of the cache.
        assert!(graph.cache().hashes.is_empty());
        std::fs::remove_file(path).unwrap_or(());
    }
}
