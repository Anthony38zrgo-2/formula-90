# Current implementation analysis

The supplied source was inspected directly.

## Current tire thermal path

`simulation.rs` currently:
- calls `st.aero.step(...)`;
- computes `thermal_modifiers` from the current tire pressure/temperature state;
- calls `suspension.step_with_modifiers(...)`;
- computes drivetrain and `st.powertrain.brake_torques[i]`;
- applies wheel torques;
- solves combined-slip tire forces;
- then builds `TireThermalInput` and calls `st.tire_thermal.step_after_forces(...)`.

This is a good place to insert brake heat transfer because the current tire thermal update is already last in the mechanical chain.

## Current tire heat sources

`tire_thermals.rs` currently heats tread from:
- longitudinal slip work;
- lateral slip work;
- road conduction.

It heats carcass from:
- tread conduction;
- vertical flex / hysteresis.

It heats gas from:
- carcass conduction.

There is currently no brake/rim heat source.

## Current brakes

`powertrain.rs::process_brakes()` creates per-wheel torques:
- front each = total * front_bias * 0.5
- rear each = total * (1-front_bias) * 0.5
- ABS can zero a wheel's torque during a pulse.

That existing `brake_torques[4]` is the correct energy source for the thermal model.

## Current aero

`aero.rs` computes:
`drag_target = q * coefficient_of_drag * frontal_area`

There is no brake-duct drag term.

## Current ABI

The supplied `ffi.rs` declares:
`F1_94_PHYSICS_ABI_VERSION = 8`

Pressure and five tire temperatures per wheel are already appended to `FfiTelemetryOutput`.

Brake thermal telemetry therefore requires an append-only ABI 9 update.
