# TS-110 BuildIR materialization boundary Status

## Result

The Rust half of the materialization boundary is implemented in `track-build`.
Blender now has a concrete, self-contained `track.build.json` contract and a
readiness gate, so a backend can materialize the track without reinterpreting
SVG or re-evaluating the spline.

The pure Python half is implemented under:

```text
blender/track_pipeline/blender_backend/
```

- `build_ir_loader.py`: loads `track.build.json` into plain dataclasses and
  applies the materialization-readiness gate (mirrors the Rust gate). No `bpy`.
- `road_builder.py`: `road_mesh_geometry(samples)` derives the closed road
  ribbon vertices/faces from the explicit samples (pure, unit-tested).
  `build_road_in_blender` creates the road mesh in Blender from the same
  samples, with a lazy `bpy` import and a fallback to the first scene.
- `object_builder.py`: `object_card_geometry(instance)` computes a base-anchored
  billboard card at the explicit position, yaw and scale (pure, unit-tested);
  `build_object_in_blender` creates it in Blender.
- `exporter.py`: `export_glb(objects, path)` and `save_blend(path)`.
- `terrain_builder.py`: `terrain_grid(cells)` derives the heightfield mesh from
  the explicit BuildIR terrain cells (pure, unit-tested); `build_terrain_in_blender`
  creates it in Blender.
- `vegetation_builder.py`: materializes explicit vegetation instances as cards
  (reuses the object geometry).
- Smoke tests: `_validate_road_blender.py`, `_validate_objects_blender.py` and
  `_validate_terrain_vegetation_blender.py`. All PASS in headless Blender.

## Implemented behavior

- `write_track_build_json(document, registry, path)`: writes `track.build.json`
  (deterministic serialization of `BuildIR`).
- `materialization_prerequisites(build_ir) -> Vec<String>`: lists what a Blender
  backend still needs (non-empty road samples, finite geometry, positive half
  widths).
- `materialization_ready(build_ir) -> bool`.
- Python loader + readiness gate and pure ribbon geometry, testable without
  Blender.

## Validation

```text
cargo test --workspace                         PASS: 51 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
python -m unittest discover -s blender/track_pipeline/tests  PASS
blender --background --factory-startup --python _validate_road_blender.py  PASS
blender --background --factory-startup --python _validate_objects_blender.py  PASS
blender --background --factory-startup --python _validate_terrain_vegetation_blender.py  PASS
```

## Explicit limitations and remaining work

- A dedicated barrier builder is not yet present because BuildIR does not yet
  carry barrier segments (barrier import is deferred in `TS-070`).
- Terrain and vegetation parity against the production `terrain_grid.py` /
  `vegetation_regions.py` output is pending.
- The Rust orchestrator that launches Blender/Godot is `TS-100`.

The road, terrain, vegetation and object materialization paths are now
validated end-to-end in real headless Blender runs, and the exporter emits GLB
from the materialized scene.
