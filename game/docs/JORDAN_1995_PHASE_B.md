# Jordan 1995 — Phase B mechanical grip

Purpose: tune mechanical grip, brake balance and rear differential behavior while preserving the validated GEVP baseline architecture.

## Active scene

`res://scenes/tracks/test_field/jordan_handling_test.tscn`

Windows helper:

`.\scripts\run_jordan_handling.ps1`

The test scene now instantiates:

`res://scenes/vehicles/jordan_1995/jordan_1995_phase_b.tscn`

Phase A remains preserved at:

`res://scenes/vehicles/jordan_1995/jordan_1995.tscn`

## Phase B changes

Only these handling areas change:

- Front brake bias: 0.60 -> 0.57.
- Rear differential lock engage torque: 120 Nm -> 170 Nm.
- Contact patch: 0.22 -> 0.21.
- Braking grip multiplier: 1.15 -> 1.08.
- Road tire stiffness: 10.0 -> 8.75.
- Road coefficient of friction: 3.0 -> 2.65.
- Road lateral grip assist: 0.05 -> 0.02.
- Road longitudinal grip ratio: 0.50 -> 0.48.
- Curb grip is reduced proportionally so curbs do not feel like asphalt.

## Deliberately unchanged

- Geometry and collision shapes.
- Wheelbase, track and tire dimensions.
- Mass and weight distribution.
- Suspension values.
- Steering speed, steering decay, steering exponent and countersteer assist.
- TC, ABS and stability behavior.
- Reference powertrain and gearbox.
- Aero/downforce layers.
- Formula90s DrivingAids.

Keeping those systems unchanged makes Phase B an isolated mechanical-grip experiment.

## Validation sequence

1. Straight-line braking from medium speed. The rear must remain stable without excessive front lock tendency.
2. Low-speed corner entry. The car should accept rotation without instant snap oversteer.
3. Constant-radius medium-speed corner. It should progressively approach the grip limit instead of feeling glued to the road.
4. Throttle application at corner exit. Rear slip should be possible but recoverable; one-wheel spin should not dominate.
5. Lift-off recovery. Releasing throttle should help the vehicle regain line without an abrupt stability correction.
6. Curb crossing. Curb grip must be visibly lower than Road but should not create artificial spins from minor contact.

## What to report after testing

Record only observed behavior, not desired parameter changes:

- braking: stable / front-heavy / rear unstable
- entry: understeer / neutral / oversteer
- mid-corner: too much grip / progressive / too loose
- exit: planted / recoverable wheelspin / snap oversteer
- countersteer: easy / delayed / ineffective
- curbs: stable / too grippy / destabilizing

Do not tune steering or aero in response to Phase B observations. First correct tire, brake-bias or differential behavior until mechanical grip is stable.

## Next phase

Phase C: steering response and countersteer calibration. Begin only after Phase B produces predictable low- and medium-speed behavior.
