# Jordan 1995 — Phase A GEVP baseline

Purpose: validate the Jordan geometry on the same known-good GEVP physics foundation before any Formula90s-specific handling work.

## Scene

Run:

`res://scenes/tests/vehicle_track_combinations/jordan_handling_test.tscn`

Windows helper:

`.\scripts\run_jordan_handling.ps1`

The helper validates the committed runtime bundle and materializes only the canonical Formula90s runtime geometry into `game/assets/generated/jordan_1995/`. That generated directory is intentionally ignored by Git.

## Canonical visual geometry

The Jordan now follows `VEHICLE_VISUAL_ASSET_CONTRACT.md`.

Runtime requires only three visual geometries:

- `jordan_191_1995_chassis.glb`
- `jordan_191_1995_wheel_front.glb`
- `jordan_191_1995_wheel_rear.glb`

The original bundle still contains side-specific prototype wheels, but the launcher deliberately chooses one verified source per axle:

- front canonical source: `jordan_191_1995_wheel_fl.glb`
- rear canonical source: `jordan_191_1995_wheel_rl.glb`

Godot reuses the same front wheel `PackedScene` for FL/FR and the same rear wheel `PackedScene` for RL/RR. The right-side visual is oriented by a static child node below the GEVP `Pivot`; no negative scale is used.

This guarantees left/right visual geometry symmetry by construction and removes the need for per-wheel runtime mesh calibration.

## Physical wheel placement

The four GEVP `RayCast3D` wheels remain independent physics objects.

Front axle:

- FL: `(-0.739368, 0.0575, -1.4394)`
- FR: `(+0.739368, 0.0575, -1.4394)`

Rear axle:

- RL: `(-0.718479, 0.0525, +1.4906)`
- RR: `(+0.718479, 0.0525, +1.4906)`

Per axle, X is an exact mirror and Y/Z are identical.

## What changes versus the frozen baseline

- Jordan 1995 visual geometry.
- 2.930 m wheelbase.
- Jordan wheel positions and tire dimensions.
- Vehicle mass: 505 kg.
- Front weight distribution: 45%.
- Front tire: 0.3473 m radius, 335 mm width.
- Rear tire: 0.3473 m radius, 420 mm width.

## What deliberately does NOT change yet

- GEVP tire stiffness scale.
- Friction coefficients.
- Rolling resistance.
- Steering assists and response.
- ABS/TC/stability baseline behavior.
- Suspension baseline values.
- Reference powertrain.
- No custom downforce/aero layer.

This is intentional. If Phase A behaves incorrectly, investigate geometry, raycast placement, collision shapes, mass/CG, or wheel dimensions before tuning grip or aero.

## Runtime asset optimization

The bundle contains prototype-optimized Jordan GLBs. Runtime now imports only three canonical geometries instead of chassis plus four separate wheel meshes. This reduces redundant imports and prevents side-specific wheel defects from entering the active vehicle scene.

## Next phases

1. Validate straight-line acceleration/braking and wheel contact.
2. Validate low/medium-speed steering and recoverable oversteer.
3. Validate bumps and curbs without chassis strikes.
4. Only then tune mechanical grip and brake bias.
5. Tune steering response after mechanical grip is stable.
6. Tune Jordan-specific powertrain.
7. Add front/rear aero balance last.
