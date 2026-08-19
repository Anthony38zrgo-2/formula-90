# Jordan 1995 — Phase B mechanical grip and wheel stability

Purpose: stabilize the Jordan mechanical baseline before steering, powertrain or aero work. Phase B covers mechanical grip, brake balance, rear differential behavior and wheel stability while preserving the validated GEVP vendor physics architecture.

## Active scene

`res://scenes/tests/vehicle_track_combinations/jordan_handling_test.tscn`

Windows helper:

`.\scripts\run_jordan_handling.ps1`

The test scene instantiates:

`res://scenes/vehicles/jordan_1995/jordan_1995_phase_b.tscn`

and the dedicated Formula90s handling circuit:

`res://scenes/tracks/test_field/formula90s_test_track.tscn`

The GEVP vendor demo track remains untouched. See `FORMULA90S_TEST_TRACK.md` for the circuit design/test rules.

Phase A remains the geometry/reference parent at:

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

## Wheel stability architecture

Wheel visual asymmetry is no longer repaired at runtime. The Jordan follows the project-wide canonical vehicle visual contract:

- one chassis geometry;
- one canonical front-wheel geometry reused by FL and FR;
- one canonical rear-wheel geometry reused by RL and RR.

The four GEVP `RayCast3D` wheels remain independent physics objects. Their visuals are shared `PackedScene` instances, and the right-side orientation lives below the GEVP-controlled `Pivot`.

This guarantees that the two wheels of an axle cannot diverge because of independently-authored GLB geometry. The old `jordan_wheel_visual_calibrator.gd` workaround is intentionally removed.

The front RayCast positions are exact mirrors in X and share identical Y/Z values. Rear RayCast positions follow the same invariant.

See `VEHICLE_VISUAL_ASSET_CONTRACT.md` for the project-wide rule.

## Wheel diagnostics

The Jordan test contains a read-only wheel overlay. Toggle it with the existing `ShowDebug` input.

It reports for FL / FR / RL / RR:

- contact state,
- suspension compression,
- change in spring length between displayed frames,
- lateral slip,
- longitudinal slip.

With visual geometry now shared per axle, any repeatable left/right difference in those values is much stronger evidence of a physical RayCast/suspension/chassis issue rather than an imported-wheel modeling defect.

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
- Wheelbase and physical track dimensions other than exact L/R centering of the front axle.
- Physical tire radius and configured axle tire widths.
- Mass and weight distribution.
- Suspension spring/damping/ARB values.
- Steering speed, steering decay, steering exponent and countersteer assist.
- Base TC, ABS and stability parameters.
- Reference powertrain and gear ratios.
- Aero/downforce layers.
- GEVP vendor controller, wheel implementation and demo track.

Suspension is deliberately not retuned merely because a visual wheel problem was previously observed. Mechanical suspension changes require evidence from the wheel diagnostics.

## Validation sequence

1. Stationary/slow-spin inspection: FL and FR must present the same geometry and apparent radius; RL and RR likewise.
2. Flat straight acceleration: FL and FR should show similar contact/compression behavior.
3. Straight-line braking from medium speed: rear must remain stable without excessive front-lock tendency.
4. Two-wheel grass excursion: cross a white line gently and confirm the lower-grip transition is recoverable.
5. Full grass excursion/rejoin: verify the car can leave and rejoin without an artificial continuous guardrail blocking the test.
6. Low-speed corner entry: car should accept rotation without instant snap oversteer.
7. Constant-radius medium-speed corner: grip limit should arrive progressively rather than feeling glued to the road.
8. Corner exit throttle: rear slip may occur but should remain recoverable; one-wheel spin should not dominate.
9. Lift-off recovery: releasing throttle should help regain line without an abrupt artificial correction.
10. Local curb crossing: curb grip must be lower than Road without minor contacts generating artificial spins.
11. Banked corners: compression should remain controlled and no longitudinal ramp/jump should be present.
12. Wheel overlay check: if one wheel repeatedly loses contact or its spring-length delta is substantially larger than its opposite wheel on flat road, treat that as a physical suspension/raycast issue before Phase C.

## Phase B completion criteria

Phase B is ready to close when:

- no side-specific visual wheel geometry anomaly remains,
- left/right wheel contact is stable on flat road,
- braking is predictable,
- Road -> Grass -> Road transitions are controllable,
- low/medium-speed grip loss is progressive,
- throttle-on rear slip is recoverable,
- local curb behavior is predictable,
- banked sections do not introduce artificial ramp behavior,
- no persistent left/right suspension anomaly is visible in wheel diagnostics.

## Next phase

Phase C: steering response and countersteer calibration. Do not start steering or aero tuning until the Phase B completion criteria above are satisfied.
