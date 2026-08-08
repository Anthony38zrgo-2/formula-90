# Jordan 1995 — Phase A GEVP baseline

Purpose: validate the Jordan geometry on the same known-good GEVP physics foundation before any Formula90s-specific handling work.

## Scene

Run:

`res://scenes/tracks/test_field/jordan_handling_test.tscn`

Windows helper:

`.\scripts\run_jordan_handling.ps1`

The helper extracts the committed runtime bundle into `game/assets/generated/jordan_1995/` before Godot starts. That generated directory is intentionally ignored by Git.

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
- No Formula90s DrivingAids node.
- No custom downforce/aero layer.

This is intentional. If Phase A behaves incorrectly, investigate geometry, raycast placement, collision shapes, mass/CG, or wheel dimensions before tuning grip or aero.

## Runtime asset optimization

The committed bundle contains a prototype-optimized copy of the previously prepared Jordan GLBs. Vertex clustering was used only to reduce repository/runtime size; overall scale and extents were preserved. The high-detail source remains the reference for later visual refinement.

## Next phases

1. Validate straight-line acceleration/braking and wheel contact.
2. Validate low/medium-speed steering and recoverable oversteer.
3. Validate bumps and curbs without chassis strikes.
4. Only then tune mechanical grip and brake bias.
5. Tune steering response after mechanical grip is stable.
6. Tune Jordan-specific powertrain.
7. Add front/rear aero balance last.
