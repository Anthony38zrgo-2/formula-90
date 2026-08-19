# Track Studio Implementation Progress

Snapshot of the Formula-90 Track Studio migration as of the last update. The
backlog `track-studio` epic is the authority for status; this file summarizes
it and records the reprioritization rationale.

## Done (implemented and validated)

| ID | What is complete |
|---|---|
| TS-000 | Backlog decomposed and registered; `agentdb` seed/validate pass. |
| TS-001 | Current ownership, source-of-truth and baseline captured. |
| TS-002 | Versioned `TrackDocument`/`BuildIR`/`Diagnostic`/IPC contracts (`CONTRACTS_V0.md`). |
| TS-003 | Legacy SVG/JSON/raster/revision compatibility mapping defined. |
| TS-010 | Pure Rust `track-domain` (models, invariants, fixed-float serialization, reference layers). |
| TS-040 | Unified validation around `Diagnostic` (+ asset checks via registry). |
| TS-050 | `track-commands`: command pattern, undo/redo, preview-collapse batches. |
| TS-060 | `track-assets`: semantic registry with budget/hash validation. |
| TS-080 | `track-export`: canonical export + runtime metadata, deterministic. |
| TS-090 | `track-build`: versioned `TrackDocument -> BuildIR` compiler. |

## In progress (with remaining work)

| ID | Remaining |
|---|---|
| TS-020 | Recovery/backups, persistent revisions, locking, partial queries. |
| TS-030 | Bezier handle evaluation on semantic control points; La Chutana parity. |
| TS-070 | Import barriers, vegetation-region groups and gameplay markers; sanitizer hardening. |
| TS-100 | Real (non-dry-run) process execution and progress/cancel to the UI. |
| TS-110 | Barrier builder (needs barriers in BuildIR); wire builders into the orchestrator. |
| TS-120 | Real Blender/Godot execution command; full frontend integration. |
| TS-130 | Interactive viewport editing, handles, profile views. |
| TS-140 | Interactive viewport editing and measurement overlays. |
| TS-150 | Vegetation parity against `vegetation_regions.py`. |
| TS-151 | Terrain parity against `terrain_grid.py`. |

## Proposed (not started)

| ID | What |
|---|---|
| TS-160 | Godot integration: consume runtime metadata + headless GLB validation. |
| TS-170 | Migrate La Chutana as the golden Track Studio fixture. |
| TS-180 | Retire `server.py` and obsolete authoring scripts after parity. |
| TS-190 | Integration, recovery, performance and Windows packaging gates. |
| TS-200 | Rebuild `AGENTS.md`, `.agents/` and skills from the real architecture. |
| TS-210 | Final acceptance and legacy retirement. |

## Reprioritization (current context)

The vertical slice (import -> validate -> store -> export -> BuildIR -> build
graph -> Blender materialization -> Tauri shell -> TypeScript frontend) is now
working and tested (68 Rust tests + Python pipeline + headless Blender + Tauri
build). The highest remaining value is therefore to close the runtime loop and
prove the pipeline on a real circuit, then harden robustness and finish the UI.

New priority order (highest first):

1. `TS-160` Godot integration - validates the generated GLB and consumes the
   runtime metadata, closing build -> validate.
2. `TS-100` real process execution / progress to UI.
3. `TS-120` real build/playtest command in the Tauri shell.
4. `TS-110` barrier builder + orchestrator wiring.
5. `TS-170` La Chutana golden migration (real-world proof).
6. `TS-070` import barriers/gameplay (needed for La Chutana).
7. `TS-150` / `TS-151` vegetation and terrain parity.
8. `TS-030` Bezier handle evaluation + La Chutana parity.
9. `TS-020` storage recovery/revisions (robustness, after the loop works).
10. `TS-190` integration/performance/packaging tests.
11. `TS-130` / `TS-140` interactive viewport editing.
12. `TS-180` retire `server.py`.
13. `TS-200` / `TS-210` documentation reconstruction and final acceptance.

This reverses the earlier storage-first ordering because the working vertical
slice now makes build/validate integration and the golden-track migration more
valuable than storage robustness or viewport editing.

## Recurring validation

```text
cargo test --workspace                         PASS (68 tests)
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
python -m unittest discover -s blender/track_pipeline/tests  PASS
blender --background --factory-startup --python blender_backend/_validate_*.py  PASS
cargo build (app/src-tauri)                    PASS -> track-studio-tauri.exe
npm run typecheck && npm run build             PASS
agentdb seed && agentdb validate               PASS
```
