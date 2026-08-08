# GEVP clean baseline

This branch isolates Godot Easy Vehicle Physics (GEVP) from Formula90s-specific runtime tuning.

## Goals

- Treat `game/addons/gevp` as vendor code.
- Keep F1-specific tuning in vehicle scenes/resources, not inside the addon.
- Keep runtime helpers such as camera, audio, HUD and driving aids out of the physics baseline.
- Make every handling change attributable to one explicit layer.

## Baseline test

Open:

`res://scenes/tracks/test_field/gevp_baseline.tscn`

It contains only:

1. the GEVP track scene,
2. the GEVP vehicle controller,
3. `f1_1996_car.tscn`.

No Formula90s driving-aids node is present.

## F1 baseline changes

The previous scene is preserved at:

`res://scenes/vehicles/legacy/f1_1996_car_pre_gevp_clean.tscn`

The active `f1_1996_car.tscn` keeps the model, wheel positions, mass, powertrain identity and gearbox, but resets handling values to GEVP-scale values.

Notable corrections:

- `tire_stiffnesses[Road]`: `220000 -> 10`
- `rolling_resistance[Road]`: `45 -> 1`
- removed `999` pseudo-disable values from steering/ABS
- restored normal steering smoothing and countersteer assist
- restored explicit TC/ABS/stability baseline behavior
- center-of-gravity offset reset from `-0.45` to `-0.20`
- bump stops reduced from `3.0` to `1.0`
- rear suspension travel normalized to match the initial baseline pass

These are debugging values, not the final Formula 1 handling model.

## Vendor-code rule

Do not add Formula90s features directly to `addons/gevp`.

Preferred architecture:

- `addons/gevp`: upstream/vendor physics implementation
- `addons/formula90s`: Formula90s behavior and integrations
- `scenes/vehicles`: vehicle-specific GEVP parameterization
- `resources/vehicles`: future engine/setup/tire/aero profiles

## Known remaining vendor differences

At the start of this branch, `vehicle.gd` and `wheel.gd` already contained local changes. The controller has been restored to upstream exactly. The remaining two files should be reconciled against a pinned GEVP upstream commit before declaring the entire addon byte-for-byte vendor-clean.

Known local concerns include:

- `vehicle.gd`: Formula90s `engine_config` hook added inside GEVP.
- `wheel.gd`: surface fallback behavior and rear bottom-out debug logging added inside GEVP.

These differences are intentionally documented rather than hidden. They should be moved out or separately justified in the next vendor-sync commit.

## Test order

1. Run `gevp_baseline.tscn` on flat asphalt.
2. Verify acceleration and automatic shifts.
3. Verify straight-line braking.
4. Verify progressive steering and recovery.
5. Verify suspension travel over low bumps.
6. Only then test curbs.
7. Reintroduce Formula90s systems one at a time.

Do not tune aero, driving aids or extreme tire values until the baseline is stable.
