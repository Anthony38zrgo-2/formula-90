# Formula90s Documentation

Single documentation root for the project. Naming and folder rules follow
[`docs/engineering/naming-conventions.md`](engineering/naming-conventions.md)
(approved 2026-08-19). All filenames are `kebab-case`, English, ASCII.

`PROJECT_STATE.md` (repo root) is the canonical **current-state** handoff and
remains the first thing to read. This index maps the rest.

## Architecture & design

| Doc | What it is |
|---|---|
| [architecture/runtime-map.md](architecture/runtime-map.md) | Canonical map of the live runtime (`run_f1_94.ps1` → `vehicle_test_session` → `RaceSession` → `F90Core`/`F194RustVehicle`/mountains). |
| [architecture/overview.md](architecture/overview.md) | High-level architecture overview. |
| [architecture/vehicle-system.md](architecture/vehicle-system.md) | Vehicle system architecture. |
| [architecture/telemetry.md](architecture/telemetry.md) | Telemetry design. |
| [architecture/data-driven-design.md](architecture/data-driven-design.md) | Data-driven configuration design. |
| [architecture/project-direction.md](architecture/project-direction.md) | Where the project is going (was `game/docs/PROJECT_DIRECTION.md`). |
| [architecture/manifesto.md](architecture/manifesto.md) | Architecture manifesto (was Spanish `MANIFIESTO_ARQUITECTURA.md`). |
| [architecture/gevp-cpp-plan.md](architecture/gevp-cpp-plan.md) | GEVP→C++ migration plan (was `PLAN_ARQUITECTURA_GEVP_CPP.md`). |
| [adr/](adr/) | Architecture Decision Records (`0001-*.md`, `0002-*.md`). |
| [vehicle-studio/architecture-spec.md](vehicle-studio/architecture-spec.md) | Architecture contract for semantic CAD views, constrained F1 geometry editing, Blender materials and immutable variants. |

## Engineering & operations

| Doc | What it is |
|---|---|
| [engineering/naming-conventions.md](engineering/naming-conventions.md) | The naming/structure standard (this convention). |
| [engineering/common-errors-and-fixes.md](engineering/common-errors-and-fixes.md) | Reusable failure patterns and fixes (was `game/docs/COMMON_ERRORS_AND_FIXES.md`). |
| [engineering/godot-traps.md](engineering/godot-traps.md) | Common Godot traps. |
| [engineering/failure-patterns.md](engineering/failure-patterns.md) | Failure patterns. |
| [engineering/known-issues.md](engineering/known-issues.md) | Known issues. |
| [engineering/garbage-inventory.md](engineering/garbage-inventory.md) | GARBAGE inventory: every file not in the `run_f1_94.ps1` pipeline. |
| [engineering/gevp-baseline-policy.md](engineering/gevp-baseline-policy.md) | Frozen GEVP baseline policy. |
| [engineering/gevp-clean-baseline.md](engineering/gevp-clean-baseline.md) | GEVP clean-baseline notes. |
| [engineering/tree.txt](engineering/tree.txt) | Repo tree snapshot (informational). |
| [build-and-run.md](build-and-run.md) | How to build and run. |
| [audio-pipeline.md](audio-pipeline.md), [cpp-dsp-architecture.md](cpp-dsp-architecture.md) | Audio pipeline / DSP architecture. |
| [sprite-pipeline.md](sprite-pipeline.md), [camera-and-directional-sprites.md](camera-and-directional-sprites.md) | Sprite + camera presentation. |

## Game design

| Doc | What it is |
|---|---|
| [game-design/vehicle-philosophy.md](game-design/vehicle-philosophy.md) | Vehicle handling philosophy. |
| [game-design/handling-philosophy.md](game-design/handling-philosophy.md) | Handling philosophy. |
| [game-design/powertrain-philosophy.md](game-design/powertrain-philosophy.md) | Powertrain philosophy. |
| [game-design/game-rules.md](game-design/game-rules.md) | Game rules. |
| [game-design/ai-physics-manual.md](game-design/ai-physics-manual.md) | AI/physics manual. |
| [game-design/hud-controls-aids-proposal.md](game-design/hud-controls-aids-proposal.md) | Controls/HUD/aids proposal (was `PROPUESTA_CONTROLES_HUD_AYUDAS.md`). |
| [game-design/vehicle-behavior/](game-design/vehicle-behavior/) | Vehicle-behavior contracts (handling, suspension, curb). |
| [art-direction.md](art-direction.md) | Visual art direction. |

## Tracks, vehicles, troubleshooting

| Doc | What it is |
|---|---|
| [tracks/](tracks/) | Test-track docs (`formula90s-test-track.md`, `la-chutana-test-track.md`). |
| [vehicles/](vehicles/) | Vehicle docs (`f1-94.md`, import standard, visual-asset contract, Jordan 1995 phases). |
| [track-studio/](track-studio/) | Track Studio tooling docs (`ts-*-status.md`, flows, decisions, migration plan). |
| [vehicle-studio/](vehicle-studio/) | Vehicle Studio architecture, sprint plan, contracts, Phase 0 evidence and implementation backlog. |
| [troubleshooting/](troubleshooting/) | Known issues, resolved incidents, retrospectives. |
| [roadmap.md](roadmap.md) | Active roadmap. |
| [arcade-references.md](arcade-references.md), [physics-model.md](physics-model.md), [v10-vehicle.md](v10-vehicle.md) | Supplementary reference docs. |
