# Folder domain audit — AGENTS.md sprints

Provenance: branch `main-clean`, HEAD `5904e0b0` (fonts promotion commit),
worktree carrying the concurrent HUD sprint (modified `game/features/retro_hud/`,
`game/scripts/hud/`, `game/project.godot`,
`game/tests/smoke_test_tire_status_panel.gd`, rebuilt
`game/addons/formula90s/bin/` DLLs, `game/BUILD_SOURCE`) plus the uncommitted
folder-domain documentation sprint (AGENTS.md additions and edits,
`game/assets/fonts/` README and `.gitkeep` deletions, leftover-folder purges).
No branch move happened; the dirty files were inventoried, not touched.

Scope: the first documentation sprint added 175 dedicated `AGENTS.md` files and
recorded this audit here (`reports/` is Git-ignored, so curated audits live
here). The second documentation sprint corrected stale references (pit-stop
controller extension, sounds `.gdignore` folders, bundled-fonts placeholder),
clarified the `gf509` legacy source, created five missing parent files
(`game/data/`, `game/scripts/`, `game/features/`, `game/native/`, `tests/`),
moved sampling-format truth into `formats/audio_bank/`, added a glossary to
the root protocol, promoted `game/fonts/` into Git, replaced the
`.agents/AGENTS.md` protocol copy with a pointer to the root file, and purged
leftover folders (`game/audio/engine/`,
`game/crates/reports/`, `tools/sprites/`, `tools/asset_pipeline/`,
`game/resources/environment/tools/`). The `game/crates/.cargo-crap.toml`
candidate was reviewed and kept: it is the live cargo-crap coverage policy,
not residue.

## Coverage exclusions (deliberate, recorded here)

- Vendored foreign code carries no AGENTS.md and is covered by its parent:
  `game/addons/gdUnit4/` (46 folders), `game/addons/gevp/`, and the
  `third_party/godot-cpp` submodule. Never patch these in place.
- Pure binary leaves (only media plus `.import` sidecars, zero
  code/config/scene files) are covered by their parent: texture packs,
  `game/sounds/bank_sources/` (named in
  `game/sounds/AGENTS.md`), the `v10-gp3/backup/` leaf, `game/telemetry/`
  (placeholder), and `game/tracks/fuji76_77/` media.
- Tracked editor cache `.godot-user/` was untracked and removed; the folder is
  ignored by `.gitignore`.
- Ignore-blocked folders cannot carry a committable AGENTS.md and are covered
  by their parent: the seven `game/crates/*/src/bin/` runner folders, the
  `game/addons/formula90s/bin/` build-output folder, and
  `game/tools/diagnostics/` (see anti-pattern 9). `scratch/` is governed by
  its tracked README, and `.agents/skills/` by `.agents/README.md`; both stay
  file-free by ignore policy.

## Anti-patterns: misplaced or misstructured folders

| # | Folder | Finding | Status |
|---|--------|---------|--------|
| 1 | `game/audio/` | Deprecated historical sampler tree beside the canon `game/sounds/banks/` | Open; the canon now states the `gf509` diagnostic exception |
| 2 | `game/sounds/banks/v10-gp3/backup/`, `tobe/` | Backups and tonal analysis staged inside a runtime bank directory | Resolved: backup and staging removed; only the active `engine_coast_maximum` source remains for the intermediate generator |
| 3 | `game/crates/` vs `native/` vs `game/native/vehicle-audio-dsp/` | Three native homes | Open; `game/native/AGENTS.md` now records the third home |
| 4 | `.godot-user/` tracked | Editor user cache committed to Git | Resolved: untracked, removed, and ignored by `.gitignore` |
| 5 | `game/logs/`, `game/.godot*/` on disk | Generated logs, snapshots, and build outputs beside source | Open |
| 6 | `game/{core,physics,sim,graphics,diagnostics}/` untracked | Shadow structure beside tracked homes | Open |
| 7 | `.agents/skills/` vs `.agents/library/skills/` | Two skill homes | Resolved by policy: `skills/` holds active picks, `library/skills/` the catalog (`.agents/README.md`) |
| 8 | `game/data/` vs `game/assets/` | Undocumented boundary | Resolved: `game/data/AGENTS.md` states it; both homes cross-reference |
| 9 | `.gitignore` `bin/`, `diagnostics/`, `src/bin/` vs tracked sources | Ignore rules also match folders holding tracked `.rs`, `.gd`, and `.dll` sources | Open; parents document the coverage |
| 10 | `game/resources/environment/` | Tracked docs and a tracked test reference build scripts removed from HEAD (`1425f5d3`); no tracked consumer remained | Resolved: orphaned library removed with `game/resources/` |
| 11 | `.agents/AGENTS.md` | Byte-identical copy of the root protocol inside the agent kit | Resolved: replaced with a pointer to the root `AGENTS.md`; the protocol lives in one file only |

## Duplicate-function candidates for unification

| # | Folders | Reading |
|---|---------|---------|
| 1 | `scripts/` vs `tools/` vs `game/tools/` | Entry points vs pipeline implementation vs in-engine utilities; likely a real split, needs a stated contract |
| 2 | `game/tests/` vs `tests/` vs `tools/*/tests/` | Godot suites vs manual smoke gate vs Python suites; the split is now stated in `tests/AGENTS.md` |
| 3 | `tools/audio/` vs `formats/audio_bank/` | Builder versus schema contract; complementary — confirmed, and the sampling-format truth now lives in `formats/audio_bank/` |
| 4 | `tools/physics_diagnostics/` vs `game/tools/diagnostics/` | Offline analyzers vs in-engine probes; candidate for one diagnostics home |

## Resolved this documentation sprint

- `game/fonts/` promoted to Git as the runtime typeface home; `game/assets/fonts/`
  demoted to the bundled Barlow reference (stale README and `.gitkeep`
  removed).
- Sampling-format truth moved from `game/sounds/AGENTS.md` into
  `formats/audio_bank/AGENTS.md`; pointers updated in
  `game/crates/vehicle-audio-engine/` and `native/include/formula90s/audio/`.
- Leftover folders purged (see scope); the parent AGENTS.md files no longer
  reference them.
- Root protocol glossary added (`BUILD`, `promotion`, `binary leaf`,
  `ignore-blocked`, `golden`).
- `.agents/AGENTS.md` replaced with a pointer to the root protocol, leaving a
  single protocol file.

## Resolved in the dead-code cleanup sprint

- Unreferenced F1 2030 manifests, dormant V10 loop samples, and the retired
  F1 2026-2008 asset family removed. Texture sidecars deleted by the initial
  sweep were restored where Godot GLB import still references them (pit-crew
  wheel carriers, MP4/6 livery); only fully embedded-texture duplicates stayed
  deleted.
- Stale editor cache, duplicate per-crate `Cargo.lock` files, the unused DSP
  build-source template, and dead `.gitignore` whitelists removed.
- Uninvoked tools (`tools/validation/`, `tools/agent_validation/`, orphan
  Blender/audio/diagnostics scripts), orphan launchers, and tests bound to
  removed assets purged. Manual visual-review/capture suites stay in
  `game/tests/` per its AGENTS contract.
- Superseded grand-prix sampler staging (`engine_on_candidate/`, root event
  copies, `v10-gp3` backup/staging) removed; the five-engine sample bank stays
  as the canonical builder input, and `v10_f2002_*`/`v10_gf509` stay for the
  gf509 diagnostic profile and synth fixtures.
- Orphaned offline library `game/resources/` removed; its consumers
  (`game/assets/textures/track/AGENTS.md`, track-generation skill, architecture
  inventory) updated.
- Unused background/skybox feature removed (`game/scripts/background/`,
  `game/scenes/visuals/`, `game/crates/skybox-engine/`, HUD shaders, related
  tests). Fuji never activated it: the track declares no background preset and
  `RaceSession` used the legacy environment. `test_windows.ps1` dropped the
  mountains smoke from its list.
- The legacy F1 2026-2008 physics data and scene remain because
  `vehicle-physics-engine` embeds them via `include_str!` and Rust tests load
  them; retiring that default profile is a separate code backlog item.
- The bootstrap/main-menu entry path is kept: it is the project main scene and
  native `GameBootstrap` still loads it, so retiring it is a product decision,
  not dead-code cleanup.
- Known pre-existing gates outside this sprint: `cargo test` for
  vehicle-physics-engine fails one aero golden from `598f8e55`, and
  `tools/audio/tests/test_bank_determinism.py` fails two commons-bank
  provenance checks.

Moves and unifications remain explicitly out of scope until each row above
becomes a backlog item with its owning AGENTS.md files as reviewers.
