# Track Studio Phase 0 Inventory

## Scope

Phase 0 is discovery, backlog registration, baseline capture and contract
preparation. It does not create the Rust workspace, Tauri shell, SQLite
storage or production migration code.

## Repository snapshot

- Branch: `refactor/gevp-clean-baseline`
- Commit: `a8740f5a5f63c1ab2da71a467d12f887ffda041d`
- HEAD: `feat(track): add SVG Track Authoring System end-to-end`
- Worktree change not owned by this work: `?? docs/editor_pipeline.csv`
- Existing Track Studio implementation: Python HTTP authoring plus SVG/JSON/Blender/Godot pipeline
- Target implementation present: none for Rust TrackDomain, Tauri or `.f90track`

## Current ownership findings

| Area | Current owner | Current authority | Target owner | Classification |
|---|---|---|---|---|
| Authoring session | `authoring_store.py` | canonical SVG in workspace | Rust commands/domain | compatibility |
| HTTP API | `authoring/server.py` | session state | Tauri IPC | compatibility |
| Undo/redo | `TrackSession` memory stacks | SVG strings | Rust command history | compatibility |
| Revisions | `authoring_store.py` + `git_service.py` | revision SVG and manifest | SQLite revisions plus explicit Git | compatibility |
| SVG safety | `svg_sanitizer.py` | restricted SVG profile | Rust SVG importer | compatibility |
| SVG normalization | `svg_normalizer.py` | `track.normalized.json` | Rust geometry/BuildIR | migration source |
| Asset registry | `asset_registry.py` + JSON | registry JSON | Rust asset registry | migration source |
| Terrain | `terrain_grid.py` and Blender builders | config plus normalized JSON | Rust semantic model/BuildIR | mixed ownership |
| Vegetation | semantic masks, regions and Python placement | layout PNG/catalog/compiled JSON | Rust regions and instances | mixed ownership |
| Blender output | `build_normalized_track_blender.py` | normalized JSON | Python materializer from BuildIR | refactor |
| Build gateway | `build_svg_track.py` | source/registry/compiler hashes | Rust BuildOrchestrator | compatibility |
| Runtime validation | Python wrapper + Godot GDScript | generated GLB pair | Godot validation invoked by Rust | active compatibility |

## Current source-of-truth findings

There is not one current authority for every path:

- The newer authoring path treats canonical SVG and its revision directory as
  the editable authority.
- The La Chutana semantic path uses semantic PNGs, marker PNGs, object catalog,
  layout configuration and compiled JSON together.
- Normalized JSON, manifests, `.blend` and GLB files are generated outputs.
- Godot consumes generated GLB resources and does not author track data.

The target `.f90track` authority must therefore be introduced through import
and compatibility adapters, not by deleting the existing paths first.

## Existing validation baseline

- Fixture compilation twice: PASS. Canonical, normalized and manifest hashes
  were byte-identical.
- Fixture canonical SHA-256: `617591b9ecccc25cffb9a14a4c38f8b381ac68d3c7beb6485f4bb0f419d0070f`
- Fixture normalized SHA-256: `9b13642b4566a3be033f7da8ad0c9f738571c98ec36803fbf7537de02abe071a`
- Fixture manifest SHA-256: `de292eefd89d080e2c93bc8da7c060fdbaecd0cb6a36a6ab09b47a102559a3a4`
- La Chutana import with `--check`: PASS.
- La Chutana parity: 124 objects, 9,560 vegetation instances, 9,684 total.
- La Chutana collision validation: PASS; all six reported invariants valid.
- Blender fixture build, atomic activation and Godot GLB gate: PASS.
- Build SHA-256: `0aae96ed63023a925407b1683817277784f188a6f1ff7b4f56b2347f216e9dda`
- Godot: `4.7.1.stable`.
- Blender executable used: Blender 5.2 path from the existing script contract.
- Python authoring suite: INCONCLUSIVE/FAIL under Python 3.14 because tests
  leave temporary asset files locked or already removed during teardown. The
  failures are cleanup errors, not failed functional assertions.

## Dirty-worktree rule

`docs/editor_pipeline.csv` was present before the Phase 0 edits. It is not
modified, removed, staged or classified as part of the Track Studio change.

## Phase 0 gate

Phase 0 is ready to close when this inventory, `CURRENT_FLOWS.md`,
`DECISIONS.md`, `MIGRATION_PLAN.md`, the backlog epic and the baseline report
are reviewed together. Production implementation must start with `TS-010`
only after the contracts in `TS-002` are approved.
