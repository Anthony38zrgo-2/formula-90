# TS-040 track-validation Status

## Result

The unified validation layer is implemented in:

```text
tools/track_studio/crates/track-validation/
```

It is the single authority for domain and geometry rules that do not require
the asset registry. It combines the domain invariants with geometric and
gameplay checks and returns stable `Diagnostic` records that the UI and the
build gate can consume.

## Implemented rules

- Centerline self-intersection (via `track-geometry::closed_polyline_self_intersects`).
- Extreme banking and elevation ranges.
- Non-positive width profile values.
- Missing start/finish marker (warning).
- Out-of-order checkpoints (error).
- `validate_with_registry`: unknown asset ids and per-asset budget violations
  (checked against `track-assets`).

It also reuses `TrackDocument::validate()` for empty IDs, duplicate IDs,
non-finite values, invalid station ordering and the minimum centerline rule.

## Validation

```text
cargo test --workspace                         PASS: 37 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- Asset-in-road-envelope and vegetation-in-exclusion-zone checks still require
  road envelope sampling (`TS-150`).
- Barrier/road intersection, extreme slope and terrain discontinuity checks
  require terrain and barrier sampling not yet available.
- Diagnostics carry `object_id`/`station` where relevant; the interactive focus
  wiring is part of the UI work.

The next step is to wire this validator (with a registry) into the import and
build gates so that invalid authoring cannot reach Blender.
