# TS-070 track-import-svg Status

## Result

The legacy SVG importer is implemented in:

```text
tools/track_studio/crates/track-import-svg/
```

It reads the restricted F90 Track SVG profile with `roxmltree` and maps the
`data-role` elements into a semantic `TrackDocument`, emitting `Diagnostic`
records for anything that cannot be faithfully represented. SVG is treated as a
legacy import source, never as the editable authority.

## Implemented mapping

- `data-track-id` and metadata.
- Centerline `d` path parsed via `track-geometry::parse_path` (handles
  M/L/H/V/C/Q/Z) and stored as control points.
- Road `data-width-m` split into left/right width profiles.
- `banking` and `elevation` profile controls by station.
- `terrain-zone` polygons as terrain regions.
- `asset-instance` circles into asset instances (position, yaw, scale).

Deferred elements (`barrier`, `vegetation-region`) produce `Info` diagnostics
instead of being silently dropped or misrepresented.

## Validation

```text
cargo test --workspace                         PASS: 34 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- This is a compatibility adapter, not the final sanitizer; the Python
  `svg_sanitizer` remains authoritative for untrusted input safety until the
  importer is hardened.
- Barriers, vegetation regions and gameplay markers are not yet mapped into the
  document; they are reported as deferred.
- Centerline becomes a flattened polyline of control points without Bezier
  handles, matching the current normalized representation.
- The importer does not validate the resulting document against the asset
  registry; that gate is part of the import/build integration.

The next step is to harden the sanitizer boundary and wire this importer plus
`track-validation` and `track-assets` into a guarded import gate before a track
can be saved or built.
