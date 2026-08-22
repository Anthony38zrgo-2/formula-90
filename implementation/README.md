# Formula-90 — Brake Heat Transfer + Brake Duct / Drag Handoff

Source baseline: the user-supplied current `vehicle-physics-engine/src` archive.

## What is already implemented

The supplied source already contains:
- `TireThermalSystem` in `VehicleState`.
- 5 tire thermal nodes per wheel: Inner / Center / Outer / Carcass / Gas.
- Dynamic hot pressure.
- Pressure-dependent suspension/tire mechanical modifiers.
- Tricast contact-zone thermal weighting.
- Tire thermal update after combined-slip force generation.
- Tire pressure/temperature FFI telemetry.
- Vehicle physics ABI version 8.

This package therefore does NOT reimplement tire pressure. It extends the current model with:

1. Brake thermal state per wheel.
2. Brake energy generation from actual applied brake torque and wheel angular speed.
3. Disc -> caliper / hub -> rim heat propagation.
4. Rim -> tire carcass and gas heat transfer.
5. Brake temperature operating window and fade.
6. Front/rear brake duct opening setup.
7. One physical brake-duct airflow model shared by:
   - brake cooling;
   - rim cooling;
   - aerodynamic drag.
8. Brake/rim telemetry for HUD and diagnostics.

## Required architecture

The new thermal path is:

```text
actual brake torque × wheel angular speed
                |
                v
              DISC
             /    \
        CALIPER   HUB
                    |
                    v
                   RIM
                 /     \
          CARCASS      GAS
              |          |
              v          v
        tread I/C/O   pressure
```

The duct path is:

```text
duct opening
     |
effective inlet area
     |
     +----------> mass airflow ----------> brake/rim cooling
     |
     +----------> CdA -------------------> aerodynamic drag
```

The SAME effective opening/area model must drive both cooling and drag.

## Tick ordering

Use previous-tick brake temperature to determine current brake effectiveness.

Recommended order:

1. `aero.step(...)`
2. tire pressure mechanical modifiers
3. suspension
4. drivetrain / brake torque demand
5. apply brake thermal efficiency scale to the actual wheel brake torques
6. wheel torque + tire force solve
7. evaluate brake duct airflow / drag
8. update brake thermal system using actual brake torque and wheel omega
9. receive brake-to-tire heat flux for each wheel
10. feed those heat watts into `TireThermalInput`
11. update tire thermals and pressure for next tick
12. publish telemetry

Do not solve brakes twice inside one tick.

## Key non-negotiables

- Brake temperature must not be cosmetic.
- Brake fade must reduce the torque actually sent to `process_wheel_torque`.
- Heat generation must use ACTUAL post-ABS, post-fade brake torque.
- No direct `brake_temp -> tire tread temp` shortcut.
- Heat must reach the tire through the rim/carcass/gas path.
- Duct opening 0.0 still has baseline external/natural cooling.
- Duct drag scales with dynamic pressure (`~v²`), not a constant speed penalty.
- Do not fold duct drag into tire rolling resistance.
- Do not change the 12-ray tricast geometry.
- Do not modify powertrain/aero beyond the explicit brake torque fade and duct drag coupling.

## Starting values

All numerical constants in this handoff are calibration starting points, not claims of exact 1994 team data.
