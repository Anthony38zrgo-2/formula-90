# Autonomous driver correction and Fuji validation

## Continuous pedals and utilization comparison (2026-10-16)

The AI now uses continuous signed longitudinal demand, separate brake entry/release thresholds, progressive pedal actuation and acceleration-only braking anticipation. At fixed utilization 0.80, short brake pulses fell from 42 to 3 and abrupt accelerator cuts from 39 to zero; the flying lap changed from 1:10.350 to 1:12.083. The 0.85 and 0.90 variants completed Fuji in 1:10.383 and 1:09.100 respectively, with the same physical binaries, vehicle profiles and nominal grip calibration. Read `docs/autonomous_driver_longitudinal_control.md` for the controlled comparison, preserved 0.80 default and separate pre-existing coupled-world regression failure.

## Latest canonical simplified-physics hotlap (2026-10-16)

The canonical Godot rigid-body path with all four simplified backends completed two Fuji laps with the unrestricted Rust driver: standing lap 1:15.567, flying lap 1:10.350, maximum speed 296.379 km/h. Stability and automatic clutch remained enabled. A source-derived steering conversion is required because the historical AI harness uses coupled rack commands. Read `docs/simplified_autonomous_driver_fuji_validation.md` for evidence, adapter correction and limits. This result does not close the separate coupled-world or ten-lap race-AI gates described below.

The solitary driver now follows a point ahead on the racing line, interpolated
within a segment. It limits steering rate, combines proportional and integral
speed feedback with maintenance throttle, and clears throttle when braking.
Conservative speed limits are explicit tuning data, not vehicle grip or power
changes. These controls replace the original independent error-only helpers.

The scheduler retains fractional time and provides the number of pending control
and tactical updates. Projection uses the local segment window first and falls
back to a global search only after losing the path. Loading validates loop
closure, branch continuity and progress. The existing Fuji document has 989
entries; its known disconnected tail leaves 819 continuous main-loop entries.
This exception does not permit arbitrary tracks to truncate malformed geometry.

Speed profiling rejects degenerate geometry and iterates acceleration and braking
constraints to convergence around the closed lap. It no longer forces a minimum
speed above a configured maximum. Transmission loading validates supervision
speeds and hysteresis between upshift and downshift thresholds.

The geometric profile now selects 120 physical steps per simulated second, with
an 8.333333 millisecond maximum suspension step. Its manifest domain and legacy
compatibility document remain synchronized. Driver control remains at 60 Hz,
so each ordinary control interval advances two physical steps.

Automatic selection forbids upshifts while lifting or braking, requires both
engine and road-derived revolutions to reach the configured threshold, and checks
the predicted next gear against all downshift thresholds. Downshifts reject a
predicted regime that would immediately request an upshift. Automatic requests
retain a cooldown consisting of the configured shift duration plus the existing
minimum gear-change interval; explicit manual requests retain their own path.
Wheel slip is compared in angular-velocity units consistently. Artificial-driver
part-throttle upshifts use 94 percent of maximum revolutions, full-throttle
upshifts 96.5 percent. The conservative 129.6 km/h test target remains in place.

## Reproduction

For an unrestricted straight-line speed test, add
`-DriverTuningPath D:/Formula90s/game/data/race_intelligence/unrestricted_driver_tuning.json`.
Its global maximum is explicitly `null`. The planner retains finite corner and
braking targets around the closed circuit, but does not apply the conservative
acceleration-profile pass in this mode: available engine power and traction
determine acceleration. This mode preserves the existing corner envelope and
does not establish the vehicle's ultimate cornering or lap-time capability.
The ordinary solitary-driver tuning remains available for comparisons.

Run from the repository root:

```powershell
./scripts/run_autonomous_driver_fuji.ps1 -PhysicalWorldPackage D:/Formula90s/scratch/coupled_vehicle_complete_implementation/fuji_physical_world.json -Laps 1 -Rendered
```

The package is an existing candidate generated from the actual Fuji collision
GLB. The native loader validates its source and geometry digests; an outdated
candidate must be regenerated rather than bypassing these checks.

The launcher records branch, HEAD, dirty inventory and hashes, invokes the
canonical runtime preflight, then builds the driver executable and optimized
physical-world library in a fresh evidence directory. Installed game libraries
are not overwritten. The native bridge still requires matching BUILD identities.
Dirty source hashes are additional provenance; matching HEAD alone does not
represent a clean checkout.

Godot loads the actual Fuji vehicle session. A separate local Rust process
produces controls from observations; it does not simulate the vehicle. Godot
submits those controls to one `CoupledVehicleWorld`, using the source-verified
collision package, and publishes authoritative snapshots to the native vehicle
presentation. The player profile remains manual; the validation instance selects
automatic transmission through the artificial-driver tuning document.

Outputs include `fuji_report.json`, `fuji_telemetry.csv`, build/runtime logs,
hashes and, in rendered mode, `fuji_driver.png`. The test fails on disconnected
control, invalid projection progress, an off-track vehicle footprint, a stalled
car, rejected snapshots, or expiry before completing the requested laps.

This is an opt-in validation adapter. It does not establish production race-menu
integration, multi-car performance, overtaking, ten-lap endurance or human visual
acceptance. Wall time includes local transport and rendering and is not an
isolated physics benchmark.

## Verified 120 Hz results

Evidence: `scratch/autonomous_driver_fuji_120_hertz_verified_20261014/`.
Source branch `main-clean`, HEAD `f7fa7d46b559538a8dd80819f2f52e0dd7997573`,
with uncommitted changes recorded by inventory and hashes.

The rendered Fuji test passed one complete 4,310.55 metre lap in 145.75 simulated
seconds. Maximum lateral error was 1.63957 metres, with zero footprint violations.
Physical frequency was 120 Hz and driver control 60 Hz. Wall time was 164.701
seconds, versus 421.587 seconds in the preceding 960 Hz test: 2.56 times faster,
but still approximately 0.885 times real time in this validation adapter.

Measured wall-time categories were 71.396 seconds waiting for the driver process,
57.670 seconds advancing and serializing the physical world, and 31.776 seconds
publishing native presentation snapshots. These measurements include boundary
overhead; they do not isolate pure solver cost. The remaining slowdown is not
evidence that the physics frequency differs from its configured 120 Hz.

Validation passed 11 artificial-driver tests, 28 coupled-world tests, six
powertrain tests, six seven-gear tests, the canonical Godot smoke and both DSP
build-configuration suites. The higher-speed flat-ground test reached second
gear and 47.1 metres per second while maintaining less than one metre maximum
lateral error. Ten installed Rust DLL copies were checked against build hashes;
a transient copy lock in the canonical build was resolved before launching.

The complete physics library suite passed 156 of 157 tests. Its existing wheel
drag reference test fails independently of these changes: both the backed-up
profile and current profile store 0.0792 square metres while their unchanged
wheel geometry and coefficients imply 0.0804 square metres. The calculation and
baseline comparison are recorded in
`scratch/physical_frequency_120_hertz_correction/preexisting_wheel_drag_reference_failure.json`.
Aerodynamic calibration was not changed to suppress this failure.

## Vehicle-derived pace controller

The controller now requires the selected physical vehicle configuration. Its
speed envelope uses vehicle mass and fuel, tire friction and load sensitivity,
axle aerodynamic load, brake torque, gearing, and track curvature and grade.
Braking also reserves grip for cornering. This is a nominal predictor with
explicit grip reserves; it does not observe live tire temperature or dynamic
wheel loads and does not establish the vehicle's ultimate achievable lap time.
`unrestricted_driver_tuning.json` leaves the global speed cap unset.

The longitudinal target is interpolated at the current path position. Steering
preview no longer applies a second advance to the braking profile. Pedals use
profile deceleration feedforward, speed feedback with anti-windup, full throttle
when below the tracking band, exclusive throttle/brake operation, and bounded
actuator rates. Steering commands are calibrated against the actual suspension
rack geometry instead of a nominal maximum steering angle.

Recovery selects the edge on the displaced side and subtracts the vehicle's
actual footprint. A continuous clearance curve and rate-limited recovery factor
replace the immediate 65 percent target-speed drop.

The higher-speed regression also exposed suspension travel event localization:
projection at an active stop changes other wheel velocities, and a reversing
quadratic trajectory can cross a stop even when its final endpoint is inside.
Localization now considers the coupled active set and the earliest positive
boundary root. The world frequency remains 120 Hz; event splitting does not
change that configured frequency.

## Automatic upshift limiter correction

The former upshift condition required both engine speed and road-equivalent
engine speed to exceed the same shift threshold, and separately rejected wheel
speed differences above an absolute threshold. Normal traction slip could hold
the engine at its limiter while the road gate remained just below its threshold.
Upshifts now use the configured engine regime and require the next gear's
road-equivalent engine speed to exceed its downshift thresholds. Low-road-speed
wheelspin remains blocked, with cooldown, braking exclusion and kickdown
hysteresis retained. Seven automatic-selection regression cases pass, including
normal tire slip across every forward upshift and excessive wheelspin rejection.

The first unrestricted rendered candidate did not complete Fuji: the physical
world rejected its collision speed domain at 15.9167 simulated seconds, after
585.42 metres and with zero track footprint violations. This is a failed runtime
gate, independent of the passing unit suites. Its report is
`scratch/autonomous_driver_fuji_physical_pace_20261015/fuji_report.json`.
