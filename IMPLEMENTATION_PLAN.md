# Diagnostic & Implementation Plan - GEVP & C++ Integration Fixes

## Executive Summary & Commit History Review

Following an audit of the repository history (`git log`) and current codebase state, we identified the progression of vehicle simulation in `formula-90s`:

1. **Bootstrap & Phase 2 (`c5e0697` to `1aa464f`)**: Initial 3D vehicle physics, chase camera, audio DSP, and HUD were implemented as native C++ GDExtension classes (`ArcadeCarController`, `ArcadeChaseCamera`, `EngineAudioController`, `VehicleVisual3DController`, `ResetManager`).
2. **GEVP Integration (`28404cd`)**: GEVP (`addons/gevp/scripts/vehicle.gd`) was introduced as the target GDScript physics simulation engine, adding `f1_1996_car.tscn` and `f1_2026_car.tscn`.
3. **SOLID Clean Pass & Fixes (`d219d85`, `cbdbad7`)**: Refactored GEVP parameters and inputs, but created architectural divergence:
   - `player_car.tscn` remained on C++ `ArcadeCarController`.
   - `test_field.tscn` switched to GEVP `f1_1996_car.tscn`, but replaced C++ camera and audio with GDScript fallbacks (`camera.gd`, `engine_sound.tscn`) because C++ presentation nodes failed to bind to `Vehicle`.

---

## Root Cause Diagnosis

### 1. Interface Conflict: Hardcoded C++ Cast to `ArcadeCarController`
All C++ presentation & UI controllers (`ArcadeChaseCamera`, `VehicleVisual3DController`, `EngineAudioController`, `ResetManager`, `DirectionalVehicleSprite`) contain hardcoded checks:
```cpp
car = Object::cast_to<ArcadeCarController>(get_node_or_null(car_path));
if (!car) return;
```
When a GEVP vehicle (which inherits `RigidBody3D` / `Vehicle` in GDScript) is assigned to `car_path`:
- `Object::cast_to<ArcadeCarController>` returns `nullptr`.
- **Camera (`ArcadeChaseCamera`)**: Aborts `_process()`, freezing camera movement.
- **Visual 3D (`VehicleVisual3DController`)**: Aborts `_process()`, disabling chassis roll/pitch lean, steering angle visuals, wheel spin, and kerb vibration.
- **Audio DSP (`EngineAudioController`)**: Logs `car not found at ...` and outputs silent/broken audio.
- **Reset Manager (`ResetManager`)**: Fails to locate or reset vehicle.

### 2. Violations of `AGENTS.md` Mandatory Architecture
`AGENTS.md` explicitly specifies:
> *"La física del vehículo usa GEVP (GDScript) como motor de simulación. HUD, cámara, bootstrap, reset y presentación visual viven en C++20 mediante GDExtension."*

Because C++ nodes failed on GEVP `Vehicle`, GDScript fallbacks were temporarily patched into `test_field.tscn` instead of updating C++ controllers to interface with GEVP `Node3D`/`RigidBody3D`.

### 3. Control & Keybind Mismatches
- **Input Map Action Names**: `project.godot` defines actions `"Throttle"`, `"Brakes"`, `"Steer Left"`, `"Steer Right"`, `"Handbrake"`, `"Clutch"`, `"Shift Up"`, `"Shift Down"`.
- **Missing Action**: `vehicle_controllergd.gd` attempts `string_toggle_transmission = "Toggle Transmission"`, which is missing in `project.godot` (or explicitly cleared).
- **Reverse Input Swap Conflict**: `vehicle_controllergd.gd` swaps throttle/brake inputs in reverse (`current_gear == -1`), which conflicts with GEVP's built-in gear ratio logic and produces inverted pedal responses during reverse gear.
- **Legacy Input Action Mapping**: Deprecated `ArcadeCarController` expected `"accelerate"`, `"brake"`, `"toggle_automatic"`, `"shift_up"`, `"shift_down"`, `"steer_left"`, `"steer_right"`.

---

## Proposed Technical Changes

### Phase 1: Native C++ Vehicle Adapter / Duck-Typing Interface
Instead of hardcoding `Object::cast_to<ArcadeCarController>`, update C++ controllers (`ArcadeChaseCamera`, `VehicleVisual3DController`, `EngineAudioController`, `ResetManager`, `DebugHudController`, `DirectionalVehicleSprite`) to support generic `Node3D` vehicle nodes (supporting both GEVP GDScript `Vehicle` and any `Node3D`/`RigidBody3D` with vehicle properties).

Specifically, create a light C++ helper or adapter interface (`VehicleAdapter`) in `native/src/vehicle/vehicle_adapter.hpp`:
- Reads `global_transform` and `global_position` from `Node3D`.
- Queries velocity via `linear_velocity` / `get_velocity()` / frame position delta.
- Calculates `world_acceleration` via frame velocity delta.
- Extracts telemetry via Godot `Object::get()` and duck-typing:
  - `speed` (m/s) -> `speed_kph` = speed * 3.6
  - `motor_rpm` / `rpm`
  - `current_gear` / `gear`
  - `throttle_amount` / `throttle_input`
  - `true_steering_amount` / `steering_input`
  - `automatic_transmission`

### Phase 2: Refactor C++ Presentation & UI Nodes
1. **`ArcadeChaseCamera`**: Accept any `Node3D` car node using `VehicleAdapter`. Calculate visual pose, smoothed forward vector, and speed-based FOV directly from GEVP vehicle state.
2. **`VehicleVisual3DController`**: Bind to GEVP `Vehicle`, animates chassis lean, steering angle, wheel spin, and kerb vibration.
3. **`EngineAudioController`**: Query `motor_rpm`, `throttle_amount`, `speed`, and `current_gear` from GEVP `Vehicle`, streaming synthesized V10 engine audio via its C++ DSP pipeline.
4. **`ResetManager`**: Reset GEVP `RigidBody3D` by setting `global_transform`, clearing `linear_velocity` and `angular_velocity`.
5. **`DebugHudController`**: Unified readout for speed, gear, RPM, transmission mode, and active driving aids.

### Phase 3: Input & Scene Clean-up
1. **`project.godot`**: Ensure all input actions (`Throttle`, `Brakes`, `Steer Left`, `Steer Right`, `Handbrake`, `Clutch`, `Shift Up`, `Shift Down`, `Toggle Transmission`, `Reset Vehicle`) are registered with consistent naming and keybinds.
2. **`vehicle_controllergd.gd`**: Remove invalid reverse input inversion. Ensure `Toggle Transmission` action is properly wired.
3. **`scenes/vehicles/player_car.tscn`**: Update `player_car.tscn` to use GEVP physics (`f1_1996_car.tscn` / `f1_2026_car.tscn`) equipped with C++ `ArcadeChaseCamera`, `VehicleVisual3DController`, `EngineAudioController`, and `ResetManager`.
4. **`scenes/tracks/test_field/test_field.tscn`**: Replace GDScript fallback camera and audio nodes with the native C++ GDExtension components.

---

## Decisions on Open Questions

1. **Deprecate `ArcadeCarController` completely.** GEVP `Vehicle` will be the sole simulation engine, as mandated by `AGENTS.md`. All C++ `ArcadeCarController` code will be removed.
2. **Reverse gear behavior:**
   - **Automatic transmission active**: pressing Brake while stopped automatically switches to Reverse (classic arcade style).
   - **Manual transmission active**: reverse must be engaged manually by shifting down to `current_gear == -1`.

---

## Verification Plan

### Automated Tests
- Run `tests/smoke_test_vehicle.gd` using Godot headlessly to verify GLB loading, gear ratios, and torque curve evaluation:
  ```powershell
  godot --headless -s game/tests/smoke_test_vehicle.gd
  ```
- Rebuild C++ GDExtension library using SCons:
  ```powershell
  python -m SCons platform=windows target=template_debug arch=x86_64
  ```

### Manual Verification
- Run `test_field.tscn` via `scripts/run_windows.ps1` or Godot executable.
- Verify camera smooth chasing, visual roll/pitch responsiveness, C++ V10 engine audio playback, HUD telemetry update, and keybind steering/acceleration responsiveness.
