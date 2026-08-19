# Naming & Structure Conventions (Formula90s)

> Status: `[ACTIVE]` — approved 2026-08-19; the rename manifest
> (`rename_manifest.csv`, committed with the Phase 0 commit) is the executable
> map of this standard. This file is the **single authority** for names.

## 1. Purpose

The repository previously had no naming convention: mixed casing
(`PROJECT_STATE.md` vs `context.md`), mixed languages (Spanish `MANIFIESTO_*`,
English `build-and-run.md`), duplicated doc trees (`docs/` vs `game/docs/`),
inconsistent Rust crate depth, and stray files at root. This document fixes
that by defining one rule per file type, one folder hierarchy, and a single
language for names (English; content language is out of scope).

## 2. Naming rules per type

| Type | Convention | Example |
|---|---|---|
| GDScript `.gd` | `snake_case.gd` | `driving_aids.gd` |
| Scenes `.tscn` / resources `.tres` | `snake_case`, named after role | `vehicle_test_session.tscn` |
| `class_name` / node classes | `PascalCase` | `F194RustVehicle`, `DrivingAidsController` |
| Rust files/modules | `snake_case.rs` | `vehicle_config.rs` |
| Rust crates (directories) | kebab-case dir, short lowercase | `game/crates/vehicle-physics-engine/` |
| Rust package/lib name (`[package]`, `[lib]`) | **kept `snake_case`** — these are the cdylib names loaded by C++ | `vehicle_physics_engine` |
| C++ files | `snake_case.cpp/.hpp`, one pair per PascalCase class | `f1_94_rust_vehicle.cpp` → `F194RustVehicle` |
| C++ headers | `snake_case.hpp` under `native/include/formula90s/<area>/` | `formula90s/vehicle/f1_94_rust_vehicle.hpp` |
| Docs `.md` | **`kebab-case.md`**, lowercase, English | `build-and-run.md` |
| ADR decisions | `docs/adr/000N-title.md` | `docs/adr/0001-gdextension-architecture.md` |
| Repo meta files | `UPPER_SNAKE` (no extension lowercasing) | `README.md`, `LICENSE`, `CHANGELOG.md`, `AGENTS.md`, `PROJECT_STATE.md` |
| Shell/PS/Python tooling | `snake_case` | `run_f1_94.ps1`, `build_windows.ps1` |
| JSON/YAML/data | `snake_case` keys **and** files; **ASCII only** (no accents) | `f1_94_physics.json` |
| Folders (code/Godot) | `snake_case` | `game/scenes/runtime/` |
| Folders (docs) | `kebab-case` | `docs/game-design/` |

### 2.1 Rules that apply everywhere

1. **English only** for all file/folder names. Legacy Spanish names get English
   equivalents on rename; content translation is a separate task.
2. **ASCII only** — no accented characters in any path (`canónico` → `canonical`).
3. One concept per name; no abbreviations except established domain ones
   (`f1_94`, `hud`, `gd`, `ps1`).
4. Version/date suffixes: `-2026-08-19` (kebab) or `_20260819` (snake),
   never mixed.
5. **Never rename** without updating every reference (see §4).

## 3. Canonical folder structure

```text
D:\Formula90s\
├─ README.md  LICENSE  CHANGELOG.md  AGENTS.md  THIRD_PARTY.md  PROJECT_STATE.md
├─ docs/                        # single doc root (absorbed game/docs/)
│  ├─ README.md                 # doc index
│  ├─ adr/                      # 0001-*.md …
│  ├─ architecture/             # overview, vehicle-system, telemetry, data-driven-design, runtime-map, project-direction, manifesto, gevp-cpp-plan
│  ├─ engineering/              # godot-traps, failure-patterns, known-issues, common-errors-and-fixes, naming-conventions, garbage-inventory, gevp-baseline-policy, gevp-clean-baseline, manual-checklist
│  ├─ game-design/              # vehicle/handling/powertrain philosophy, game-rules, ai-physics-manual, hud-controls-aids-proposal, vehicle-behavior/
│  ├─ troubleshooting/          # known-issues, resolved-incidents, jordan-197-orientation-retrospective
│  ├─ track-studio/             # ts-*-status.md, current-flows, decisions, contracts-v0, migration-plan, phase-0-inventory, implementation-progress, la-chutana-background-backlog
│  ├─ tracks/                   # formula90s-test-track, la-chutana-test-track
│  └─ vehicles/                 # f1-94, vehicle-import-standard, vehicle-visual-asset-contract, jordan-1995-phase-a/b
├─ game/                        # Godot project (res://)
│  ├─ project.godot  formula90s.gdextension
│  ├─ addons/                   # vendor plugins only: gdUnit4/, psx_visuals_gd4/
│  ├─ scripts/                  # project-owned GDScript (was addons/formula90s/scripts)
│  │  ├─ input/  audio/  hud/  vehicle/  track/  telemetry/  runtime/  background/
│  ├─ assets/                   # runtime content (absorbed resources/)
│  │  ├─ models/ textures/ backgrounds/ sprites/ fonts/ skybox/ trackside/ sounds/ generated/ materials/ themes/ environment/
│  ├─ scenes/                   # bootstrap/ runtime/ ui/ vehicles/ tracks/ visuals/
│  ├─ data/                     # vehicles/ tracks/ race_sessions/ engines/
│  ├─ crates/                   # Rust workspace
│  │  ├─ Cargo.toml
│  │  ├─ vehicle-physics-engine/  vehicle-audio-engine/  game-sim/  formula90-core/  skybox-engine/
│  ├─ tests/                    # gdunit/ + scene tests (absorbed scenes/tests/)
│  └─ resources/  →  merged into assets/
├─ native/                      # C++ GDExtension (unchanged layout)
│  ├─ include/formula90s/{core,vehicle,audio,camera,presentation,sim,ui}/
│  └─ src/{core,vehicle,audio,camera,presentation,sim,ui}/
├─ scripts/                     # build/run tooling
├─ tools/                       # python/blender/asset/audio/diagnostics
├─ third_party/                 # godot-cpp
├─ assets-lowpoly-python/       # source bundle (NOT runtime; documented)
├─ blender/                     # source pipeline (NOT runtime)
├─ references/                  # reference images (single README)
└─ config/  tracks/             # absorbed into game/data/ (see manifest)
```

## 4. Reference-update discipline

Renaming is only valid when every reference moves with the file:

1. **Godot path refs** (`res://...` in `.tscn`/`.tres`/`.gd`/`.godot`): move
   files with their `.uid` sibling; Godot 4.4+ re-links `ext_resource` by uid;
   then fix string-path refs (`load()`, `preload()`, `@export` NodePaths).
2. **Autoloads** (`project.godot [autoload]`): script paths update; autoload
   *names* stay.
3. **Rust**: `cargo --manifest-path` in `scripts/`, `SConstruct`, `run_f1_94.ps1`;
   cdylib `[lib] name` never changes (C++ `LoadLibrary` depends on it).
4. **Docs**: every `.md` cross-link + references in `AGENTS.md`,
   `PROJECT_STATE.md`, `COMMON_ERRORS_AND_FIXES.md`.
5. **`.gitignore`**: patterns must be updated to new paths.
6. **Verification per batch**: `run_f1_94.ps1 -ValidateRuntimeOnly` +
   `-Smoke` (Godot), `cargo test` (Rust), `git status` clean of strays.

## 5. Anti-patterns (forbidden)

- `instrucciones.txt`-style mixed-language roots at repo top level.
- Accented filenames (`docs/engineering/scan-pipeline-unused-inventory.json`).
- Duplicate doc trees (`docs/` + `game/docs/`).
- SCREAMING_SNAKE for docs that are not repo meta.
- Crate dirs at inconsistent depth (`game/graphics/engine/skybox/...`).
- `.gitkeep`/logs/db files committed under `diagnostics/`, `reports/`.

## 6. Rename manifest

`rename_manifest.csv` (repo root, committed with Phase 0) is the machine
checklist: `old_path,new_path,ref_scope`. Execute phases in order; each phase
ends with a commit and a verification gate.
