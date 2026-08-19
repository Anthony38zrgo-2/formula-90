# TS-140 Semantic Authoring Tools Status

## Result

The reference-image entity, the deterministic authoring math and the end-to-end
wiring through the Tauri IPC to the frontend are implemented.

## Implemented behavior

- `track-domain`: `ReferenceLayer` + `Calibration` (first-class), validated in
  `TrackDocument.reference_layers`.
- `track-geometry`: `pixels_per_meter`, `world_distance`, `snap_to_grid`,
  `nearest_control_point`.
- `track-commands`: `SetReferenceCalibration` (undoable).
- `track-service`:
  - `reference_layers()`, `add_reference_layer`, `calibrate_reference`
    (undoable), `measure(a, b)`, `snap(point, grid)`.
- Tauri commands: `reference_add`, `reference_calibrate`, `measure_world`,
  `snap_world`.
- Frontend: `api.ts` typed wrappers, `panels/reference-panel.ts` and a
  Reference section in `index.html` to add a layer.

## Validation

```text
cargo test --workspace                         PASS: 68 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
npm run typecheck                              PASS
npm run build                                  PASS
cargo build (app/src-tauri)                    PASS
```

## Explicit limitations

- The interactive viewport editing, Bezier handle dragging and profile views
  are still frontend work (`TS-130` remainder); this task delivers the
  primitives and the IPC/UI wiring for reference images, measurement and
  snapping.
- Multi-selection and measurement overlays are not yet surfaced in the viewport.

Next is the Godot runtime integration (`TS-160`), which consumes the generated
metadata and validates the GLB output headlessly.
