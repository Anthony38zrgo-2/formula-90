# Folder domain audit — AGENTS.md sprint

Provenance: branch `main-clean`, HEAD `2a07cf41`, worktree carrying modified
`game/addons/formula90s/bin/` DLLs plus untracked
`game/assets/models/pit_crew/racer/source/`. No branch move happened during
this sprint; the dirty binaries were inventoried, not touched.

Scope: 238 folders tracked by Git. This sprint adds 175 dedicated `AGENTS.md`
files, refactors `game/AGENTS.md` into a project-wide canon, moves the audio
canon to `game/sounds/AGENTS.md`, and records this audit at
`docs/folder-domain-audit.md` (`reports/` is Git-ignored, so curated audits
live here).

## Coverage exclusions (deliberate, recorded here)

- Vendored foreign code carries no AGENTS.md and is covered by its parent:
  `game/addons/gdUnit4/` (46 folders), `game/addons/gevp/`, and the
  `third_party/godot-cpp` submodule. Never patch these in place.
- Pure binary leaves (only media plus `.import` sidecars, zero
  code/config/scene files) are covered by their parent: environment media
  leaves, texture packs, `game/sounds/bank_sources/`, the `v10-gp3/backup/`
  leaf, `game/telemetry/` (placeholder), and `game/tracks/fuji76_77/` media.
- Tracked editor cache `.godot-user/` gets no AGENTS.md; see anti-pattern 4.
- Ignore-blocked folders cannot carry a committable AGENTS.md and are covered
  by their parent: the seven `game/crates/*/src/bin/` runner folders, the
  `game/addons/formula90s/bin/` build-output folder, and
  `game/tools/diagnostics/` (see anti-pattern 9). `scratch/` is governed by
  its tracked README, and `.agents/skills/` by `.agents/AGENTS.md`; both stay
  file-free by ignore policy.
- `diagnostics/`, `implementation/`, and `docs/` (before this file) are empty
  or untracked and therefore out of scope.

## Anti-patterns: misplaced or misstructured folders

| # | Folder | Finding | Proposed mapping (next sprint) |
|---|--------|---------|-------------------------------|
| 1 | `game/audio/` | Deprecated historical sampler tree beside the canon `game/sounds/banks/` | Freeze and redirect; keep one shim note |
| 2 | `game/sounds/banks/v10-gp3/backup/`, `game/sounds/banks/v10-gp3/tobe/` | Backups and tonal analysis staged inside a runtime bank directory | Move to `scratch/` or an untracked archive |
| 3 | `game/crates/` vs `native/` vs `game/native/vehicle-audio-dsp/` | Three native homes: Rust workspace, C++ bridge, lone C DSP library | Unify under one native tree with one AGENTS.md root |
| 4 | `.godot-user/` tracked | Editor user cache committed to Git | Untrack and extend `.gitignore` |
| 5 | `diagnostics/`, `game/logs/`, `game/.godot*/`, `game/crates/target/` on disk | Generated logs, snapshots, and build outputs beside source | Single Git-ignored policy; `docs/` keeps curated audits, `reports/` stays ignored scratch |
| 6 | `game/{core,physics,sim,graphics,diagnostics}/` on disk, untracked | Shadow structure beside tracked `game/crates/`, `game/scripts/`, `game/tools/` | Inventory, then delete or promote deliberately |
| 7 | `.agents/skills/` vs `.agents/library/skills/` | Two skill homes: an empty placeholder and the live catalog | Unify on one home; update registry, profiles, and validator together |
| 8 | `game/data/` vs `game/resources/` vs `game/assets/` | Runtime tuning vs generated pipelines vs binary art with an undocumented boundary | Boundary now stated in the trio's AGENTS.md files; moves next sprint |
| 9 | `.gitignore` `bin/`, `diagnostics/`, `src/bin/` vs tracked sources | Ignore rules written for build output also match folders holding tracked `.rs`, `.gd`, and `.dll` sources, blocking new files there | Narrow the rules to true outputs or relocate the tracked sources; AGENTS.md coverage now lives in the parents |

## Duplicate-function candidates for unification

| # | Folders | Reading |
|---|---------|---------|
| 1 | `scripts/` vs `tools/` vs `game/tools/` | Entry points vs pipeline implementation vs in-engine utilities; likely a real split, needs a stated contract |
| 2 | `game/tests/` vs `tests/` vs `tools/*/tests/` | Godot suites vs manual smoke gate vs Python suites; keep per-runtime homes, define the split |
| 3 | `tools/audio/` vs `formats/audio_bank/` | Builder vs schema contract; complementary, not duplicate — confirmed this sprint |
| 4 | `tools/physics_diagnostics/` vs `game/tools/diagnostics/` | Offline analyzers vs in-engine probes; candidate for one diagnostics home |
| 5 | `tools/blender/` vs `tools/pit_crew/` vs `tools/asset_pipeline/` on disk | Overlapping art-pipeline helpers; consolidate entry points |

Moves and unifications are explicitly out of scope for this sprint. Each row
above becomes a backlog item with its owning AGENTS.md files as reviewers.
