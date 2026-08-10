# REF-001 Phase 6 — chase camera SRP extraction

## Scope

Extract the stateful chase-camera calculation from `ArcadeChaseCamera` without
changing vehicle authority, scene configuration, exported properties, or
runtime tuning.

## Ownership after the change

- `ChaseCameraSolver` owns node-free camera state and calculation: initial
  placement, discontinuity recovery, heading smoothing, velocity lead,
  acceleration inertia, turn offsets, follow lag, look target, and FOV.
- `ArcadeChaseCamera` owns Godot integration only: resolve the vehicle through
  `VehicleStateReader`, collect the current input, resolve `Camera3D`, and
  apply the solver output.
- The solver reads no scene nodes and mutates no Godot object. It receives a
  vehicle transform, velocity, speed, steering, current camera position, FOV,
  and frame delta explicitly.

## Compatibility decisions

- All existing exported camera properties remain on `ArcadeChaseCamera` and
  are sampled every frame, preserving live editor/runtime tuning.
- `vertical_smoothing` and `vertical_dead_zone` remain exported for serialized
  scene compatibility. They were unused before this extraction and remain
  intentionally unused; this phase does not alter camera tuning.
- `locked_pitch` remains in the Godot integration because it constrains the
  applied `Node3D` rotation, not the follow calculation.

## Validation

- `powershell -ExecutionPolicy Bypass -File scripts/build_windows.ps1` passed
  and compiled both `arcade_chase_camera.cpp` and `chase_camera_solver.cpp`.
- Headless Godot editor import plus `test_field.tscn` and
  `static_directional_car.tscn` smoke runs exited 0. The existing
  `EngineAudioController requires EngineAudioConfig` message still appears in
  the static scene and is unrelated to this camera extraction.
- The complete `scripts/test_windows.ps1` route remains blocked before smoke
  tests by the pre-existing missing
  `formula90s/vehicle/physics_math.hpp` include in `native/tests/unit_tests.cpp`.
