# Agent implementation prompt

Implement **Brake Heat Transfer + Brake Duct Cooling/Drag** on top of the exact current source in this package.

Read in this order:
1. repository `AGENTS.md`
2. package `README.md`
3. `CURRENT_IMPLEMENTATION_ANALYSIS.md`
4. `integration/VEHICLE_CONFIG.md`
5. `integration/SIMULATION_PIPELINE.md`
6. `integration/TIRE_THERMAL_COUPLING.md`
7. `integration/AERO_DUCT_DRAG.md`
8. `integration/ABI_HUD.md`
9. `validation/ACCEPTANCE.md`

Use `drop_in/game/crates/vehicle-physics-engine/src/brake_thermals.rs` as the implementation baseline.

Required outcome:
- Add `BrakeThermalSystem` to Rust vehicle state.
- Generate heat from actual post-ABS/post-fade brake torque × wheel angular velocity.
- Model disc, caliper, hub and rim temperature per wheel.
- Transfer rim heat into the existing tire carcass and gas nodes through energy-conserving W heat fluxes.
- Add brake operating temperature window and fade.
- Add front/rear duct opening config.
- Correlate duct cooling and duct aerodynamic drag through the SAME effective area / airflow model.
- Add duct drag to vehicle drag without rewriting the main aero model.
- Extend strict vehicle JSON parser/serializer/validation.
- Append brake telemetry; physics ABI 8 -> 9.
- Propagate through current formula90-core/native/Godot ABI layers after inspecting their current versions.
- Extend the existing tyre HUD above the tachometer with a compact `BRK D/C/R` row per wheel.
- Do not touch unrelated aero, powertrain, suspension or HUD behaviour.

Implementation slices:
1. config + parser tests
2. new brake thermal module tests
3. VehicleState lifecycle
4. fade applied to final wheel brake torques
5. brake thermal step
6. rim -> tire heat coupling
7. duct drag
8. FFI/core/native propagation
9. HUD
10. acceptance validation

Important:
- Do not replace the existing tire thermal implementation.
- Do not heat tread directly from brakes.
- Do not compute cooling and drag from separate arbitrary percentages.
- Do not apply fade by mutating permanent `max_brake_torque`.
- Preserve append-only ABI layout.

At completion report:
- changed files;
- final JSON fields/defaults;
- final ABI versions;
- tests run/results;
- measured front/rear brake temperatures from a repeatable braking test;
- measured tire carcass/gas pressure delta with brake heat transfer on vs off;
- duct opening vs drag/cooling comparison;
- any constants changed from the handoff defaults;
- remaining limitations.
