# REF-001 Phase 1 — isolated baseline

## Scope

- Worktree: `D:\Formula90s-ref001-phase-1`
- Branch: `codex/ref-001-phase-1`
- Baseline commit: `bb56af748a43decb4e2e8e97304548e96678869b`
- Date: 2026-08-10

This phase establishes evidence only. It makes no vehicle, camera, audio,
scene, asset, or behaviour change.

## Isolation result

The source worktree was intentionally left untouched because it contains
uncommitted vegetation, skybox, configuration, and agent-infrastructure work.
The isolated worktree starts clean at the baseline commit. `godot-cpp` was
initialized at the submodule revision recorded by the repository:
`7e18e40d7591429f915035a7de7cf79457d555cc`.

## Validation results

| Check | Result | Evidence |
| --- | --- | --- |
| Native GDExtension build | PASS | SCons reports `libformula90s.windows.template_debug.x86_64.dll` up to date after a clean worktree build. |
| `smoke_test_arcade_hud_scene.gd` | PASS | `Arcade HUD scene, map data, gauge, and hidden notification state are valid.` |
| Official `scripts/test_windows.ps1` | BLOCKED | Its standalone `cl` unit-test invocation includes only `native/include`, while `native/tests/unit_tests.cpp` includes the non-existent `formula90s/vehicle/physics_math.hpp`. |
| World/HUD compositor smoke | BLOCKED | `game/scenes/vehicles/jordan_1995/jordan_1995.tscn` references three missing files below ignored `game/assets/generated/jordan_1995/`. The source checkout contains them, but a clean worktree does not. |

## Confirmed baseline gaps

1. `native/tests/unit_tests.cpp` references `formula90s/vehicle/physics_math.hpp`, but no file with that path exists in `native/include/` or `native/src/`. The test harness therefore cannot compile in the baseline.
2. The tracked Jordan scene depends on generated chassis and wheel GLBs that are excluded by `.gitignore`. A fresh checkout cannot instantiate the vehicle or run the world/compositor smoke test without a documented deterministic asset-generation/bootstrap step.
3. Godot smoke tests require the standard editor import pass before script-class resolution. This is already encoded in `scripts/test_windows.ps1`; calling the smoke scripts directly before that pass is not a valid test route.

## Refactor guardrails for later phases

- Do not treat either blocked validation as a regression introduced by REF-001.
- Do not delete, untrack, or regenerate assets as part of this phase.
- Repairing test reproducibility and generated-asset bootstrap belongs to the repository-hygiene audit stage, after a complete producer/consumer trace.
- All subsequent refactor stages must record results against this baseline and keep GEVP as the physical-simulation authority.
