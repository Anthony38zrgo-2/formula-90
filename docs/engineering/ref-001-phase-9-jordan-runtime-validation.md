# REF-001 Phase 9 — Jordan runtime visual validation

## Scope

Validate the Phase 8 Jordan typed visual configuration against the real
generated visual assets, without committing those ignored runtime outputs.

## Local materialization

The tracked `game/assets/bundles/jordan_1995_runtime.zip` bundle was unpacked
into the ignored canonical runtime contract:

- `game/assets/generated/jordan_1995/jordan_191_1995_chassis.glb`
- `game/assets/generated/jordan_1995/jordan_191_1995_wheel_front.glb`
- `game/assets/generated/jordan_1995/jordan_191_1995_wheel_rear.glb`

The front/rear output names deliberately map to the bundle's left-side source
wheels, matching the canonical symmetric one-wheel-per-axle scene contract.

## Validation

1. Headless Godot editor import discovered and imported all three GLBs.
2. Headless `jordan_handling_test.tscn` load exited 0.
3. Runtime output confirmed:
   `Vehicle 3D visual ready; optional wheel nodes: 4`.

This proves the controller resolved `ChassisVisual` and the four configured
`Orientation` nodes using the Phase 8 resource paths. It also confirms that
the refactor does not require `physics_math.hpp`; that missing header blocks
only the separate native unit-test route.

## Repository boundary

The materialized GLBs and Godot import sidecars remain ignored local outputs.
They are intentionally not added to this commit. A clean checkout still needs
the existing `scripts/run_jordan_handling.ps1` bootstrap before it can perform
this runtime validation.
