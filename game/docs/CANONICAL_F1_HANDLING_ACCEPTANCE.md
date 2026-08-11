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
