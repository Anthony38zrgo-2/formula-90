# Jordan 1995 — Phase B mechanical grip and wheel stability

Purpose: stabilize the Jordan mechanical baseline before steering, powertrain or aero work. Phase B covers mechanical grip, brake balance, rear differential behavior and visual wheel-axis correctness while preserving the validated GEVP vendor physics architecture.

## Active scene

`res://scenes/tracks/test_field/jordan_handling_test.tscn`

Windows helper:

`.\scripts\run_jordan_handling.ps1`

The test scene instantiates:

`res://scenes/vehicles/jordan_1995/jordan_1995_phase_b.tscn`

Phase A remains preserved at:

`res://scenes/vehicles/jordan_1995/jordan_1995.tscn`

The GEVP baseline vehicle remains separate and must not be tuned from Jordan observations.

## Phase B mechanical changes

- Front brake bias: 0.60 -> 0.57.
- Rear differential lock engage torque: 120 Nm -> 170 Nm.
- Contact patch: 0.22 -> 0.21.
- Braking grip multiplier: 1.15 -> 1.08.
- Road tire stiffness: 10.0 -> 8.75.
- Road coefficient of friction: 3.0 -> 2.65.
- Road lateral grip assist: 0.05 -> 0.02.
- Road longitudinal grip ratio: 0.50 -> 0.48.
- Curb grip is reduced proportionally so curbs do not behave like asphalt.

## Phase B wheel-axis correction

The imported wheel GLBs are not equally centered around their local rotation axes. The front-left visual is displaced by roughly 7 mm in the radial plane, and the front-right visual also has a radial-size asymmetry. Rotating those meshes around local X can therefore look like a bent axle or unstable suspension even when the RayCast3D physics are stable.

`jordan_wheel_visual_calibrator.gd` now performs a visual-only correction at runtime:

1. Calculates the combined imported mesh bounds for each wheel.
2. Centers the mesh geometry on the GEVP wheel rotation axis.
3. Normalizes X to the physical tire width configured on the vehicle.
4. Normalizes Y/Z to the physical tire diameter configured on the vehicle.
5. Leaves RayCast3D position, suspension force, tire force and wheel spin physics untouched.

This is intentionally kept outside `addons/gevp/`.

## Wheel diagnostics

The Jordan test contains a read-only wheel overlay. Toggle it with the existing `ShowDebug` input.

It reports for FL / FR / RL / RR:

- contact state,
- suspension compression,
- change in spring length between displayed frames,
- lateral slip,
- longitudinal slip.

Use it only to determine whether a remaining oscillation is physical after visual normalization. Do not tune springs or damping merely to hide a visual wobble.

## Controls available during Phase B

- `1`: automatic/manual transmission aid.
- `2`: stability aid.
- `3`: steering aid.
- `4`: braking aid.
- `5`: grip aid.
- `A`: shift up in manual mode.
- `Z`: shift down in manual mode.
- `Up / Throttle`: throttle in forward and reverse gears.
- `Down / Brakes`: brake in forward and reverse gears.

For baseline Phase B handling evaluation, leave aids 2–5 OFF. AUTO may be ON or OFF depending on the test.

Manual reverse is gated by the Formula90s controller: Neutral -> Reverse is allowed only when vehicle speed is at or below 0.15 m/s.

## Deliberately unchanged

- Chassis geometry and collision shapes.
- Wheelbase and physical track widths.
- Physical tire radius and configured axle tire widths.
- Mass and weight distribution.
- Suspension spring/damping/ARB values.
- Steering speed, steering decay, steering exponent and countersteer assist.
- Base TC, ABS and stability parameters.
- Reference powertrain and gear ratios.
- Aero/downforce layers.
- GEVP vendor controller and wheel implementation.

The suspension is deliberately not retuned in response to the observed front-wheel wobble because the imported visual geometry provides a concrete visual cause. Mechanical suspension changes require evidence from the wheel diagnostics.

## Validation sequence

1. Stationary inspection: front wheels should no longer look eccentric while rotating slowly.
2. Straight acceleration: FL and FR should show similar contact/compression behavior.
3. Straight-line braking from medium speed: rear must remain stable without excessive front-lock tendency.
4. Low-speed corner entry: car should accept rotation without instant snap oversteer.
5. Constant-radius medium-speed corner: grip limit should arrive progressively rather than feeling glued to the road.
6. Corner exit throttle: rear slip may occur but should remain recoverable; one-wheel spin should not dominate.
7. Lift-off recovery: releasing throttle should help regain line without an abrupt artificial correction.
8. Curb crossing: curb grip must be lower than Road without minor contacts generating artificial spins.
9. Wheel overlay check: if one wheel repeatedly loses contact or its spring-length delta is substantially larger than its opposite wheel on flat road, treat that as a physical suspension/raycast issue before Phase C.

## Phase B completion criteria

Phase B is ready to close when:

- no visible eccentric wheel rotation remains,
- left/right wheel contact is stable on flat road,
- braking is predictable,
- low/medium-speed grip loss is progressive,
- throttle-on rear slip is recoverable,
- curb behavior is predictable,
- no persistent left/right suspension anomaly is visible in wheel diagnostics.

## Next phase

Phase C: steering response and countersteer calibration. Do not start steering or aero tuning until the Phase B completion criteria above are satisfied.
