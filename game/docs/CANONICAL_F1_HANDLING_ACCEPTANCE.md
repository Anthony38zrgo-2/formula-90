# Canonical F1 Handling v1 — acceptance matrix

Canonical scene:

```text
res://scenes/tracks/test_field/jordan_handling_test.tscn
res://scenes/vehicles/jordan_191/jordan_191_k3_historical_phase_b.tscn
```

## Runtime authority

The K3 historical Jordan 191 is the authoritative runtime baseline for
Formula-90 handling work. It is mounted in La Chutana through
`FormulaVehicleController/VehicleRigidBody` and inherits the approved Phase B
physics configuration. The K3 chassis and axle wheel assets are visual-only:
wheel raycasts, collision shapes, controller ownership and handling values stay
in the inherited Phase B scene.

Use `scripts/run_jordan_handling.ps1` to launch this exact route. Do not use the
isolated `jordan_191_handling_test.tscn` or the K3 candidate-validation scene as
the handling authority; they remain useful for focused asset or controller
checks. Future handling changes must start from this K3/La Chutana baseline and
be evaluated against the matrix below.

This matrix defines observable behavior. It does not prescribe final numerical
parameters. Fuel load, fuel burn, tire wear and dynamic center-of-mass changes
are future features; this iteration uses a fixed mass and fixed tire condition.

## Current runtime baseline (2026-08-11, approved)

Approved after the full handling/powertrain pass of 2026-08-11. Source of
authority: `game/scenes/vehicles/jordan_191/jordan_191_phase_b.tscn` (inherited
by `jordan_191_k3_historical_phase_b.tscn`, mounted in `jordan_handling_test.tscn`).

**Engine:**
```text
max_torque = 340.0   max_rpm = 17000.0   idle_rpm = 4500.0
motor_drag = 0.006   motor_brake = 15.0   motor_moment = 0.08
clutch_out_rpm = 5000.0   max_clutch_torque_ratio = 1.4   throttle_speed = 20.0
```

**Transmission:**
```text
gear_ratios = [2.85, 2.29, 1.89, 1.60, 1.38, 1.20]
final_drive = 6.30   reverse_ratio = 3.0   shift_time = 0.12
```

**Tires (Road surface):**
```text
coefficient_of_friction = 2.9   longitudinal_grip_ratio = 0.52
lateral_grip_assist = 0.02      tire_stiffness = 8.75
```

**Aero:**
```text
coefficient_of_drag = 0.15   frontal_area = 0.45   air_density = 1.225
```

**Suspension:**
```text
front: spring 0.15  resting 0.50  damping 0.80  bump_stop 2.2  arb 0.25
rear : spring 0.15  resting 0.50  damping 0.85  bump_stop 2.2  arb 0.05
```

**Steering / assists:**
```text
steering_slip_assist = 0.54   countersteer_assist = 0.89   steering_exponent = 1.5
stability_yaw_engage_angle = 0.01   stability_yaw_strength = 5.25
stability_yaw_ground_multiplier = 2.0
```

**Torque curve override:** `Curve_jordan191_phase_b` (in `jordan_191_phase_b.tscn`)

**Telemetry observed across the pass:** up to 251.8 km/h top speed; full-throttle
accel 0.70 / 0.55 / 0.46 / 0.29 G in 1ª/2ª/3ª/4ª; rear bottom-out reduced from
~1.12/s to ~0.68/s and front from ~0.40/s to ~0.21/s (suspension pass);
correlation |Steering|-|LatG| improved toward 0.4+ (direction/stability pass).

## Baseline behavior

- Steering begins softly and builds progressively, including keyboard input.
- Steering correction is easy to make without a snap or oscillation.
- The neutral setup has a mild, predictable understeer bias.
- Understeer appears when entry speed is excessive, the useful steering angle is
  exceeded, or the driver asks for more lateral force than the tires/aero can
  provide.
- Oversteer is not the default balance. It appears after an aggressive setup or
  a clear driver/surface error: excessive steering, poor braking/rear bias,
  aero imbalance at speed, extreme suspension, or gravel.
- Early countersteer recovers the car; late correction can still result in a
  spin.
- Straight braking is stable and repeatable. Small parameter changes must not
  create random yaw.
- Acceleration has only a subtle initial slip; fresh tires provide good traction.
- Lifting the throttle does not abruptly destabilize the car and should permit a
  confident corner entry.
- Gravel degrades grip progressively and can induce understandable oversteer.

## Speed and cause separation

At slow speed, steering angle, tire state, braking and weight transfer dominate.
At medium/high speed, aero balance may add understeer or oversteer. A test must
not label every fast-corner understeer event as a tire or steering defect.

## Test loop

Use the canonical scene and test one behavior family at a time:

1. Straight-line brake.
2. Slow corner turn-in and correction.
3. Medium-speed corner at a safe entry speed.
4. Same corner entered too fast.
5. Fast corner with a clean line and a deliberate aggressive input.
6. Light throttle exit and lift-off entry.
7. One controlled curb/gravel excursion.

Accept the iteration when the neutral setup preserves the baseline behavior and
the failure cases have a clear driver or configuration cause.
