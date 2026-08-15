# TS-030 track-geometry Status

## Result

The first geometry slice is implemented in:

```text
tools/track_studio/crates/track-geometry/
```

It is independent of SVG, Blender, SQLite, Tauri and Godot. It consumes the
domain `ControlPoint` type and currently provides closed centerline length,
station wrapping, interpolated position, tangent, discrete curvature, fixed
spacing resampling for closed polylines, deterministic cubic Bezier flattening
and SVG path `d` parsing (M/L/H/V/C/Q/Z, absolute and relative) that mirrors the
Python normalizer tokenizer and command handling.

## Python parity

Golden centerline fixtures are generated from `compile_track.svg` (lines) and
`curve_track.svg` (cubic curves) using the exact production Python code paths
(`svg_sanitizer` + `svg_normalizer`) via `tests/generate_centerline_golden.py`.
The Rust integration test `tests/parity_python.rs` asserts:

- identical resampled points and lengths for the line fixture;
- identical flattened polyline, raw length and resampled output for the curve
  fixture (119 samples, raw length 237.921815 m);
- golden generation is byte-deterministic across runs.

```text
generator:  python tools/track_studio/crates/track-geometry/tests/generate_centerline_golden.py \
              --source <fixture.svg> \
              --output tools/track_studio/crates/track-geometry/tests/fixtures/<name>_centerline.golden.json
```

## Validation

```text
cargo test --workspace                         PASS: 15 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- Bezier handle evaluation on the semantic `ControlPoint.handle_in/out` fields
  is not implemented yet; `parse_path` consumes raw SVG command coordinates.
- Curvature is a discrete segment transition estimate, not the final spline
  curvature contract.
- No Python normalizer or terrain behavior has been changed.
- Parity covers the compile and curve fixtures; La Chutana centerline parity is
  still pending.
