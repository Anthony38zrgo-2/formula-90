# Current Track Authoring Flows

## Authoring and revision flow

```text
browser
  -> authoring/server.py HTTP API
  -> TrackSession
  -> sanitize_svg / canonical_xml
  -> workspace/*.source.svg
  -> tracks/<track>/revisions/<sha>/track.canonical.svg
  -> optional scoped Git commit
```

`TrackSession` also keeps in-memory SVG strings for undo and redo. Git is used
for revisions, but the current implementation is not SQLite-backed.

## SVG build flow

```text
SVG source
  -> svg_sanitizer.py
  -> svg_normalizer.py
  -> track.normalized.json
  -> build_svg_track.py
  -> build_normalized_track_blender.py in Blender headless
  -> track.blend
  -> track_runtime_environment.glb
  -> track_runtime_vegetation.glb
  -> validate_godot_load.py
  -> game/tools/validate_generated_track.gd
  -> temporary or activated runtime pair
```

The gateway already performs source recheck, staging, manifest validation and
atomic activation. That behavior is a useful compatibility baseline for the
future Rust BuildOrchestrator.

## La Chutana semantic flow

```text
semantic_layout.png + object_markers.png
  -> compile_semantic_layout.py
  -> compiled_layout.json
  -> import_la_chutana.py
  -> la_chutana.source.svg
  -> compile_svg_track.py
  -> normalized JSON and manifest
```

This path still contains La Chutana-specific counts and raster-derived
placement decisions. It must remain a compatibility input until the Rust
importer and deterministic region model replace it.

## Asset flow

```text
asset_registry.json
  -> asset_registry.py
  -> semantic asset IDs
  -> normalized JSON placements
  -> Blender registry resolution
  -> external GLB/PNG files
```

Binary assets are not embedded in the current JSON or revisions. Their source
hashes are part of registry validation and build provenance.

## Runtime validation flow

```text
environment GLB + vegetation GLB
  -> GLTFDocument in Godot headless
  -> required collision proxy checks
  -> vegetation collision-leak checks
  -> PASS/FAIL exit code
```

Runtime metadata for spawn, checkpoints, sectors, pit lane and minimap is not
yet generated from one TrackDocument authority.

## Target ownership transition

```text
TypeScript UI
  -> Tauri typed IPC
  -> Rust application service
  -> TrackDomain
  +-> SQLite repository
  +-> Geometry / Validation
  +-> BuildIR compiler
       -> generated JSON
       -> Blender materializer
       -> GLB
       -> Godot validation/runtime
```

The migration must preserve the current Python path until this target flow
passes the same fixture and La Chutana gates.
