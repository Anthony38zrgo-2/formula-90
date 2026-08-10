# Formula-90s current project state

Last reviewed: 2026-08-10
Branch baseline: `codex/ref-001-phase-1` from `bb56af7`

This is the compact current-state reference for the tracked repository. Runtime
scenes and resources outrank this document if they disagree.

## Active runtime authority

- **Vehicle simulation:** GEVP GDScript (`vehicle.gd`, `wheel.gd`, and
  `vehicle_controllergd.gd`) owns vehicle motion, wheels, contacts,
  suspension, transmission, and assists.
- **Native presentation:** C++20 GDExtension provides bootstrap, camera,
  vehicle presentation, audio, reset, and menu integration. It reads vehicle
  state through `VehicleStateReader`; it must not become a second physics engine.
- **Chase camera calculation:** `ChaseCameraSolver` owns the stateful follow,
  look-ahead, inertia, turn-offset, and FOV calculation. `ArcadeChaseCamera`
  only reads vehicle state and applies the resulting pose to Godot nodes.
- **World/UI composition:** `WorldHudCompositor` renders the 3D world at
  640x360 and moves gameplay controls to the unfiltered root `HudLayer`.

## Current gameplay assets

- Current development circuit: La Chutana.
- Current handling vehicle: Jordan 1995, using GEVP physical hierarchy.
- The canonical La Chutana runtime GLB and validated Texture Forge bank are
  committed. Jordan visual GLBs are intentionally materialized locally from
  the committed runtime bundle by `scripts/run_jordan_handling.ps1`.

## HUD status

- The normal HUD is GDScript-based: speed, gear, transient aid messages, and
  a presentation-only La Chutana minimap.
- The minimap marker is driven by the active vehicle transform; it is not lap
  progress, race position, or session authority.
- `DebugHudController`, `StaticMinimapController`, `ResetManager`, and
  `DirectionalSpriteValidationController` were removed after REF-001 Phase 4
  confirmed that no scene, script, resource, test, or native consumer used
  them.

## Validation baseline and known gaps

- The native extension builds in the isolated REF-001 baseline.
- `smoke_test_arcade_hud_scene.gd` passes after the standard Godot editor
  import/class-cache pass.
- The official test route currently has two pre-existing reproducibility gaps:
  it does not materialize the ignored Jordan runtime GLBs before world smoke
  tests, and `native/tests/unit_tests.cpp` includes a missing
  `formula90s/vehicle/physics_math.hpp` header.

## Refactor guardrails

1. Do not retune vehicle physics during REF-001.
2. Preserve GEVP as physical authority.
3. Do not delete registered classes, generated outputs, or assets until their
   consumers have been traced and a focused validation has passed.
4. Update this file only with verified runtime evidence in the same commit
   family as the change.

## Documentation hierarchy

1. Root and subsystem `AGENTS.md`: operational rules.
2. This file: current verified state.
3. `docs/architecture.md`: structural ownership and composition.
4. `docs/game-design/`: desired product direction.
5. `docs/decisions/` and files marked historical: rationale, not current
   implementation authority.
