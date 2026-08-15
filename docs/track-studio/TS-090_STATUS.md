# TS-090 track-build Status

## Result

The versioned `TrackDocument -> BuildIR` compiler is implemented in:

```text
tools/track_studio/crates/track-build/
```

It produces the explicit, sampled, destructive, compiler-facing representation
that Blender materializes without reinterpreting the semantic document.

## Implemented behavior

- `BuildIR` with `build_ir_version`, `compiler_version`, `source_project_hash`
  and per-subsystem hashes.
- `road_samples`: station, position, tangent, normal, width_left/right,
  elevation and bank, resampled at 2 m spacing with piecewise-linear profile
  evaluation.
- `asset_instances`: explicit positions, yaw and scale, with kind/source
  resolved from an optional `AssetRegistry`.
- Subsystem hashes (`geometry`, `terrain`, `vegetation`, `objects`,
  `gameplay`) change independently when only one subsystem changes.
- Serialization is byte-deterministic for the same input.

## Validation

```text
cargo test --workspace                         PASS: 45 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- Terrain sampling and barrier segments are not yet compiled into BuildIR;
  those belong to `TS-151` / `TS-110`.
- Vegetation regions are not yet expanded into explicit instances; that is
  `TS-150`.
- The compiler is not yet wired to the Blender backend or the build orchestrator.

The next step is to feed BuildIR into a materializer boundary (`TS-110`) and to
drive Blender/Godot through the Rust build orchestrator (`TS-100`).
