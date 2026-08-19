# Track Studio Phase 0 Decisions

## Confirmed decisions

1. `.f90track` is the target editable source of truth and is internally a
   SQLite database.
2. JSON is generated interchange, review or BuildIR output; it is not an
   editable authority.
3. Rust owns semantic data, deterministic geometry, validation, commands,
   persistence orchestration and build planning.
4. Python remains responsible for Blender-specific scene manipulation, mesh
   materialization, instancing and GLB/.blend export.
5. Godot remains the runtime and generated-output validation boundary.
6. The current server and Python pipeline remain compatibility layers until
   Tauri and Rust replacements pass equivalent tests.
7. La Chutana is the golden integration fixture.
8. `docs/editor_pipeline.csv` is unrelated work and must be preserved.

## Decisions required before implementation

| Decision | Recommended default | Gate |
|---|---|---|
| `.f90track` external assets | Store paths, hashes and metadata in SQLite; keep binary files on filesystem | TS-002 / TS-020 |
| Generated artifact location | `tracks/<id>/generated/` with export JSON, BuildIR, metadata and GLBs | TS-002 |
| Existing SVG revisions | Preserve as compatibility/history archive and import into SQLite | TS-003 / TS-070 |
| Rust crate granularity | Start with cohesive crates; combine tiny modules until real boundaries exist | TS-010 |
| Determinism | Canonical ordering, explicit float formatting, persistent seeds and compiler/tool versions | TS-002 |
| Build hash inputs | Include source project, registry, BuildIR/compiler and relevant external asset hashes | TS-002 / TS-100 |
| Revision and Git | SQLite revision is editor recovery; Git commit is explicit user action | TS-050 |
| Runtime metadata | Generate from TrackDocument/BuildIR; do not maintain a Godot duplicate | TS-080 |
| Python sunset | Remove only after Tauri/Rust parity and golden-track integration pass | TS-180 |

## Contradictory documentation to reconcile later

- `docs/roadmap.md` describes SVG as the editable source of truth.
- `docs/ai/opencode-track-authoring-handoff.txt` also describes SVG as
  canonical authority.
- `instrucciones.txt` changes the target authority to SQLite `.f90track`.
- `.agents/MIGRATION.md` discusses retiring instruction handoffs, but does not
  implement Track Studio architecture.

These references are not deleted during Phase 0. They are classified as
historical or compatibility documentation and will be reconciled after the
implemented architecture is real, as required by `TS-200`.

## Operational constraints

- No destructive Git operation.
- No broad commit containing unrelated work.
- No vehicle or GEVP changes to compensate for track migration issues.
- No production patch until the relevant contract and baseline exist.
- If the same migration failure survives two isolated attempts, stop and enter
  Diagnostic Mode instead of stacking another adapter.
