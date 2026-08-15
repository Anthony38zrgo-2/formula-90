# TS-080 track-export Status

## Result

Deterministic export is implemented in:

```text
tools/track_studio/crates/track-export/
```

It generates two JSON representations from the same semantic `TrackDocument`:
the canonical review/interchange document and the Godot runtime metadata.

## Implemented behavior

- `export_canonical`: deterministic canonical JSON of the whole document
  (reuses the fixed-float serializer from `track-domain`).
- `RuntimeMetadata` computed from the document:
  - `bounds` from the centerline;
  - `length_m` from the closed centerline;
  - `minimap_polyline` resampled at 10 m spacing;
  - `start_finish` marker with station, position and heading derived from the
    geometry tangent;
  - gameplay markers grouped by type.
- Coordinates are rounded to six decimals and serialization is
  byte-deterministic for the same input.

## Validation

```text
cargo test --workspace                         PASS: 41 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- Spawn, grid, sector and pit-lane data are currently exported as grouped
  markers; grid/sector derivation rules are not yet modeled.
- The metadata is not yet consumed by Godot; that wiring is part of `TS-160`.
- Barriers and vegetation are not included in runtime metadata yet.

The next step is to produce `track.build.json` (BuildIR) from the same document
so that Blender receives explicit, deterministic instructions (`TS-090`).
