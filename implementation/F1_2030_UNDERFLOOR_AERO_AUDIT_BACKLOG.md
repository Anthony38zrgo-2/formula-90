# F1 2030 Underfloor & Aero Audit Backlog

## Purpose

Audit the current aerodynamic implementation used by the F1 2030 vehicle profile, with special focus on validating that the flat floor + diffuser system is actually simulated correctly and is the dominant source of aerodynamic load.

This task is intentionally limited to **verification, instrumentation, deterministic testing, and correction of clear implementation defects**.

Do **not** redesign the entire aerodynamic model in this phase. Larger conceptual changes belong to the future **Physics V2** audit.

---

## Scope

Primary implementation target:

- `aero.rs`

Correlated systems to inspect only as needed:

- `vehicle_config.rs`
- `simulation.rs`
- underfloor probe / clearance code
- `telemetry.rs`
- FFI/runtime configuration path if required to validate JSON ingestion

JSON sections in scope:

```text
aero
aero.model.front_wing
aero.model.rear_wing
aero.model.underfloor
aero.model.limits
```

Relevant F1 2030 underfloor target configuration:

```json
{
  "underfloor": {
    "choke_height_m": 0.016,
    "high_height_m": 0.15,
    "induced_drag_ratio": 0.13,
    "lift_area_m2": 1.45,
    "optimal_height_m": 0.04,
    "optimal_rake_deg": 0.5,
    "rake_window_deg": 2.5,
    "stall_attack_tau_s": 0.03,
    "stall_recovery_tau_s": 0.13,
    "yaw_decay_exponent": 1.45
  }
}
```

Design target:

```text
Floor + diffuser: ~55–65% of total downforce
Front + rear wings: ~35–45% of total downforce
```

This ratio must be an emergent result of the configured physics, **not hardcoded**.

---

# Phase 0 — Establish the Data Path

## AERO-000 — Trace JSON → Runtime Aero Configuration

**Priority:** P0

### Goal

Prove that every aerodynamic field in the F1 2030 JSON reaches the Rust runtime with the expected value.

### Tasks

- Inspect `vehicle_config.rs`.
- Map every JSON aero field to the Rust struct field that consumes it.
- Check:
  - serde names
  - aliases
  - defaults
  - optional fields
  - unit expectations
  - ignored or dead fields
- Confirm the runtime is using the expected F1 2030 profile rather than legacy scene defaults.

### Required mapping

Produce a table with:

```text
JSON field
→ Rust struct field
→ consumer function
→ units
→ affected output
→ telemetry field
```

At minimum include:

```text
air_density
drag_coefficient
frontal_area
lag_tau_s

front_wing.area_m2
front_wing.incidence_deg
front_wing.polar
front_wing.yaw_decay_exponent

rear_wing.area_m2
rear_wing.incidence_deg
rear_wing.polar
rear_wing.yaw_decay_exponent

underfloor.choke_height_m
underfloor.high_height_m
underfloor.induced_drag_ratio
underfloor.lift_area_m2
underfloor.optimal_height_m
underfloor.optimal_rake_deg
underfloor.rake_window_deg
underfloor.stall_attack_tau_s
underfloor.stall_recovery_tau_s
underfloor.yaw_decay_exponent

limits.soft_max_load_ratio
limits.hard_max_load_ratio
limits.hard_max_downforce_n
```

### Acceptance criteria

- No relevant aero JSON field remains unexplained.
- Runtime values match the loaded JSON.
- Any dead/ignored field is documented as a bug or V2 debt.

---

## AERO-001 — Verify Runtime Vehicle Profile

**Priority:** P0

### Goal

Ensure tests are actually using the F1 2030 configuration.

### Tasks

Log or expose at initialization:

```text
vehicle_id
profile_name
vehicle_mass
front_wing.area_m2
rear_wing.area_m2
underfloor.lift_area_m2
underfloor.optimal_height_m
hard_max_downforce_n
```

### Acceptance criteria

Runtime values must match the selected F1 2030 JSON.

If the scene or GDScript overrides them, document and fix only the configuration path required for the aero audit.

---

# Phase 1 — Instrument the Aerodynamic Pipeline

## AERO-010 — Document the Real Aero Equations

**Priority:** P0

### Goal

Write down the exact equations implemented in `aero.rs`.

### Tasks

Trace how the solver computes:

```text
dynamic pressure
front wing CL/CD
rear wing CL/CD
floor coefficient
height factor
rake factor
seal factor
yaw factor
stall factor
front downforce
floor downforce
rear downforce
body drag
wing drag
floor drag
raw downforce
final limited downforce
aero balance
```

### Important

Do not rewrite equations to match an assumed model.

Document the equations **as implemented**.

### Acceptance criteria

A developer can reproduce every major aero output from inputs and intermediate factors.

---

## AERO-011 — Expose Missing Telemetry

**Priority:** P0

### Goal

Ensure the entire underfloor calculation can be diagnosed from telemetry.

### Required telemetry

Verify or add:

```text
Aero_TotalDownforce_N
Aero_RawDownforce_N
Aero_FrontDownforce_N
Aero_FloorDownforce_N
Aero_RearDownforce_N
Aero_Drag_N

Aero_FrontWingAngle_deg
Aero_RearWingAngle_deg
Aero_FrontWing_CL
Aero_RearWing_CL

Aero_FloorHeightFactor
Aero_FloorRakeFactor
Aero_FloorSealFactor

Aero_DiffuserExpansion_deg
Aero_DiffuserStallFactor

Aero_GlobalLimitFactor
Aero_LoadRatio
Aero_BalanceFront
```

Underfloor geometry/probe inputs:

```text
UF_FL_Clearance_m
UF_FR_Clearance_m
UF_Center_Clearance_m
UF_DiffuserThroat_Clearance_m
UF_DiffuserExit_Clearance_m

UF_ValidMask
UF_MinClearance_m
UF_Rake_rad
UF_Roll_rad
UF_ContactConfidence
```

### Acceptance criteria

One telemetry row is sufficient to reconstruct why the floor is producing its current load.

---

# Phase 2 — Validate Underfloor Geometry Inputs

## AERO-020 — Audit Underfloor Probe Geometry

**Priority:** P0

### Goal

Verify that aero calculations use real chassis-to-ground geometry.

### Tasks

Trace where these values originate:

```text
front-left clearance
front-right clearance
center clearance
diffuser throat clearance
diffuser exit clearance
rake
roll
valid mask
contact confidence
```

Check whether values are based on:

- raycasts
- suspension state
- world-space ground plane
- chassis transform

### Failure conditions

Treat as critical if:

```text
vehicle moving on track
AND
UF_ValidMask == 0 for long periods
```

or:

```text
suspension moves
BUT
underfloor clearances remain constant
```

### Acceptance criteria

Ride height and rake respond continuously to chassis motion.

---

## AERO-021 — Validate Units and Coordinate Conventions

**Priority:** P0

### Goal

Eliminate hidden unit/sign errors.

### Verify

```text
clearance: meters
rake: radians internally / degrees only where explicitly converted
velocity: m/s
forces: N
angles: rad or deg consistently
```

Also verify signs:

```text
positive rake definition
positive downforce direction
ground clearance never accidentally negative due to axis convention
```

### Acceptance criteria

No implicit degree/radian conversion remains ambiguous.

---

# Phase 3 — Validate Core Underfloor Physics

## AERO-030 — Verify Dynamic Pressure Scaling

**Priority:** P0

### Goal

Confirm load scales approximately with velocity squared before limiting.

### Deterministic test

Use fixed geometry and test speeds:

```text
100 km/h
150 km/h
200 km/h
250 km/h
300 km/h
```

Record:

```text
Aero_FloorDownforce_N
Aero_RawDownforce_N
Aero_GlobalLimitFactor
```

### Expected behavior

Before limiters:

```text
F ∝ v²
```

Approximate ratio example:

```text
200 km/h → baseline
300 km/h → ~2.25 × baseline
```

if geometry factors stay constant.

### Acceptance criteria

- Floor load follows expected velocity-squared trend before saturation.
- Any deviation is explained by known height, stall, lag, yaw, or limiter effects.

---

## AERO-031 — Validate `lift_area_m2`

**Priority:** P0

### Goal

Ensure `lift_area_m2` is used once and with area semantics.

### Test

Temporarily run deterministic cases:

```text
lift_area_m2 = 0.75
lift_area_m2 = 1.00
lift_area_m2 = 1.45
lift_area_m2 = 2.00
```

Keep all other variables fixed.

### Expected behavior

Before limits:

```text
FloorDownforce ∝ lift_area_m2
```

### Failure cases

- Field has no effect.
- Field is squared.
- Field alters unrelated drag/body aero.
- Hidden hardcoded area dominates it.

### Acceptance criteria

Near-linear scaling before limiters.

---

## AERO-032 — Ride Height Sweep

**Priority:** P0

### Goal

Validate the height-response curve of the underfloor.

### Fixed conditions

Run at:

```text
200 km/h
300 km/h
```

Test heights:

```text
0.010 m
0.016 m
0.025 m
0.040 m
0.060 m
0.080 m
0.120 m
0.150 m
0.180 m
```

### Record

```text
Aero_FloorDownforce_N
Aero_FloorHeightFactor
Aero_Drag_N
Aero_RawDownforce_N
Aero_TotalDownforce_N
```

### Expected behavior

```text
very low ride height
→ degraded/choked floor

~0.040 m
→ near maximum effective floor load

high ride height
→ progressively weaker floor
```

### Acceptance criteria

- No NaN or negative unintended load.
- No unexplained discontinuity.
- Peak/near-peak occurs around configured optimum.
- Above `high_height_m`, floor effectiveness is clearly reduced.

---

## AERO-033 — Choke Height Sweep

**Priority:** P0

### Goal

Verify behavior around:

```text
choke_height_m = 0.016
```

### Test points

```text
20 mm
18 mm
16 mm
14 mm
12 mm
10 mm
```

### Determine explicitly

What does `choke_height_m` do?

Possible implementations:

- loss factor
- stall trigger
- clamp
- smooth attenuation
- binary cutoff

### Requirement

The effect should be stable and progressive unless the current design explicitly documents otherwise.

### Reject

```text
16.1 mm → 100%
15.9 mm → 0%
```

without a deliberate justification.

---

## AERO-034 — High Ride Height Decay

**Priority:** P1

### Goal

Verify:

```text
high_height_m = 0.15
```

actually defines floor degradation at high ride height.

### Test

Sweep:

```text
0.08
0.10
0.12
0.15
0.18
0.22 m
```

### Acceptance criteria

Floor effectiveness decreases predictably above the nominal operating region.

---

# Phase 4 — Validate Rake, Roll and Yaw

## AERO-040 — Rake Sweep

**Priority:** P0

### Goal

Validate:

```text
optimal_rake_deg = 0.5
rake_window_deg = 2.5
```

### Fixed height

Approximately:

```text
40 mm
```

### Test rake

```text
-2.0°
-1.0°
0.0°
+0.5°
+1.0°
+2.0°
+3.0°
```

### Record

```text
Aero_FloorRakeFactor
Aero_FloorDownforce_N
```

### Acceptance criteria

- Maximum/near maximum around +0.5°.
- Smooth degradation outside the operating window.
- No incorrect deg/rad behavior.

---

## AERO-041 — Roll Sensitivity

**Priority:** P1

### Goal

Determine whether chassis roll affects floor sealing or ride-height interpretation.

### Test

At fixed average ride height:

```text
roll = 0°
roll = ±1°
roll = ±2°
roll = ±3°
```

### Acceptance criteria

Behavior must be explainable and stable.

If roll is intentionally ignored, document it as Physics V2 debt.

---

## AERO-042 — Yaw Sensitivity

**Priority:** P1

### Goal

Validate:

```text
underfloor.yaw_decay_exponent = 1.45
```

### Test

```text
yaw/slip angle =
0°
2°
4°
6°
8°
10°
```

### Record

```text
Aero_FloorDownforce_N
floor yaw factor
```

### Acceptance criteria

- Load decreases progressively with yaw.
- Small yaw must not abruptly destroy the floor.
- Exponent actually affects the curve.

---

# Phase 5 — Validate Diffuser Behavior

## AERO-050 — Diffuser Geometry Derivation

**Priority:** P0

### Goal

Verify the relationship between:

```text
UF_DiffuserThroat_Clearance_m
UF_DiffuserExit_Clearance_m
Aero_DiffuserExpansion_deg
```

### Questions to answer

- Is expansion angle calculated dynamically?
- What longitudinal diffuser length is assumed?
- Is the angle physically derived or hardcoded?
- Are throat and exit values both used?
- Can invalid probe geometry create impossible expansion angles?

### Acceptance criteria

`Aero_DiffuserExpansion_deg` must respond predictably to throat/exit geometry.

---

## AERO-051 — Diffuser Geometry Sweep

**Priority:** P0

### Goal

Test sensitivity to diffuser geometry.

### Cases

```text
low throat / high exit
medium throat / medium exit
high throat / high exit
```

### Record

```text
DiffuserExpansion_deg
DiffuserStallFactor
FloorDownforce_N
```

### Acceptance criteria

Changes in diffuser geometry produce coherent changes in floor behavior.

---

## AERO-052 — Diffuser Stall Trigger Audit

**Priority:** P0

### Goal

Determine exactly what causes diffuser stall.

### Trace

Find all conditions affecting:

```text
Aero_DiffuserStallFactor
```

Possible inputs:

- ride height
- expansion angle
- rake
- yaw
- sealing
- validity mask

### Acceptance criteria

Every stall trigger is documented and testable.

---

## AERO-053 — Stall Attack / Recovery Dynamics

**Priority:** P0

### Goal

Validate:

```text
stall_attack_tau_s = 0.03
stall_recovery_tau_s = 0.13
```

### Test sequence

```text
stable floor condition
→ force stall
→ hold
→ restore valid geometry
```

### Record every physics tick

```text
time
Aero_DiffuserStallFactor
Aero_FloorDownforce_N
```

### Expected behavior

```text
stall loss occurs faster than recovery
```

### Acceptance criteria

Attack is clearly faster than recovery and both JSON constants influence timing.

---

# Phase 6 — Validate Aero Component Separation

## AERO-060 — Component Sum Integrity

**Priority:** P0

### Goal

Confirm the three sources are independently computed.

### Verify

Before global limits:

```text
RawDownforce
≈ FrontDownforce
+ FloorDownforce
+ RearDownforce
```

### Acceptance criteria

Difference must be zero or explained by an explicitly documented term.

---

## AERO-061 — Disable Components Independently

**Priority:** P0

### Goal

Prove there is no hidden coupling/double count.

### Test cases

Run temporary test-only configurations:

```text
front wing disabled
rear wing disabled
floor disabled
wings disabled
floor only
```

### Acceptance criteria

Each component disappears only from its intended outputs.

---

## AERO-062 — F1 2030 Aero Distribution Sweep

**Priority:** P0

### Goal

Measure actual aero philosophy of the F1 2030 profile.

### Speeds

```text
100
150
200
250
300 km/h
```

### Nominal geometry

Use stable nominal ride height/rake.

### Record

```text
FrontDownforce
FloorDownforce
RearDownforce
TotalDownforce
FloorShare
WingShare
Aero_BalanceFront
```

### Design target

At medium/high speed nominal conditions:

```text
FloorShare ≈ 55–65%
CombinedWingShare ≈ 35–45%
```

### Important

Do not force 60/40 through code.

If the target is not reached, determine whether the cause is:

- implementation bug
- limiter behavior
- underfloor factor
- wing calibration
- JSON calibration

Only then recommend parameter changes.

---

# Phase 7 — Validate Global Aero Limits

## AERO-070 — Document Limiter Order

**Priority:** P0

### Goal

Determine the exact processing order of:

```text
raw component forces
soft_max_load_ratio
hard_max_load_ratio
hard_max_downforce_n
lag/smoothing
final downforce
```

### Acceptance criteria

The limiter pipeline is documented with code references.

---

## AERO-071 — Hard Downforce Cap Test

**Priority:** P0

### Goal

Verify:

```text
hard_max_downforce_n = 10800
```

### Test

Temporarily increase floor area in a test-only configuration until raw load exceeds the limit.

### Expected

```text
RawDownforce > TotalDownforce
GlobalLimitFactor < 1.0
TotalDownforce <= 10800 N
```

### Acceptance criteria

Hard cap is never exceeded.

---

## AERO-072 — Load Ratio Limit Test

**Priority:** P0

### Goal

Verify:

```text
soft_max_load_ratio
hard_max_load_ratio
```

actually respond to vehicle weight/load.

### Test

Use deterministic mass and speed cases.

### Acceptance criteria

- Correct vehicle mass is used.
- Load ratio has a clear dimensionless definition.
- Hard limit cannot be exceeded.

---

## AERO-073 — Preserve Aero Distribution Through Limiting

**Priority:** P0

### Goal

Ensure global limiting does not distort aero balance.

### Example

Input:

```text
Front = 2000 N
Floor = 6000 N
Rear = 2000 N
```

If global factor is 0.8, expected approximately:

```text
Front = 1600 N
Floor = 4800 N
Rear = 1600 N
```

### Acceptance criteria

Relative contribution remains stable unless a different behavior is explicitly intended.

---

# Phase 8 — Validate Drag Accounting

## AERO-080 — Trace Total Drag Formula

**Priority:** P0

### Goal

Determine whether drag is double-counted.

### Document whether total drag includes

```text
base/body CdA
front wing Cd
rear wing Cd
underfloor induced drag
brake duct drag
other drag terms
```

### Required output

Explicit equation:

```text
TotalDrag =
...
```

### Acceptance criteria

Every term appears once.

---

## AERO-081 — Validate Base `drag_coefficient`

**Priority:** P0

### Goal

Determine whether:

```text
aero.drag_coefficient
```

means:

- body-only drag coefficient
- full vehicle drag coefficient
- legacy/global drag term

### Acceptance criteria

The meaning is unambiguous and documented.

Do not tune its numerical value before this is known.

---

## AERO-082 — Underfloor Induced Drag Audit

**Priority:** P1

### Goal

Verify:

```text
underfloor.induced_drag_ratio = 0.13
```

only modifies floor-associated drag.

### Acceptance criteria

Changing the value affects floor drag without multiplying unrelated body or wing drag.

---

# Phase 9 — Validate Aerodynamic Balance

## AERO-090 — Document `Aero_BalanceFront`

**Priority:** P0

### Goal

Identify the exact equation producing:

```text
Aero_BalanceFront
```

### Verify whether it accounts for

```text
front wing force and CP
floor force and CP
rear wing force and CP
wheelbase
vehicle COM
```

### Acceptance criteria

The output has a physically interpretable definition.

---

## AERO-091 — Audit Aero Force Application Points

**Priority:** P1

### Goal

Verify where each aerodynamic force is applied to the chassis.

### Inspect

```text
front wing center of pressure
floor center of pressure
rear wing center of pressure
```

### Special requirement

If the floor center of pressure is currently hardcoded/fixed, document it.

Do not implement a dynamic floor CP model in this task unless the current implementation is clearly broken.

Dynamic CP belongs to Physics V2.

---

## AERO-092 — Balance vs Speed Test

**Priority:** P1

### Goal

Ensure aero balance does not drift unexpectedly with speed.

### Test

```text
100
150
200
250
300 km/h
```

with fixed geometry.

### Acceptance criteria

Any speed-dependent balance shift must be explained by:

- CL curves
- floor response
- limiter behavior
- known geometry response

---

# Phase 10 — Regression and Automated Tests

## AERO-100 — Add Deterministic Unit/Integration Tests

**Priority:** P0

Implement at minimum:

```text
aero_downforce_scales_approximately_with_velocity_squared

underfloor_lift_area_scales_load_linearly_before_limits

underfloor_peaks_near_optimal_height

underfloor_degrades_below_choke_height

underfloor_degrades_above_high_height

underfloor_rake_factor_peaks_near_optimum

underfloor_yaw_reduces_downforce_monotonically

diffuser_geometry_changes_expansion_angle

diffuser_stall_attack_is_faster_than_recovery

raw_downforce_equals_component_sum_before_limits

hard_downforce_limit_is_respected

hard_load_ratio_is_respected

global_limit_preserves_aero_distribution

underfloor_induced_drag_is_not_double_counted
```

### Testing guidance

Prefer invariant/trend assertions instead of exact values unless the formula is intentionally exact.

---

## AERO-101 — Create Golden Telemetry Scenario

**Priority:** P1

### Goal

Create one reproducible reference run for later Physics V2 comparisons.

### Suggested scenario

```text
speed: 250 km/h
ride height: ~40 mm
rake: +0.5°
yaw: 0°
no braking
constant throttle
flat road
```

Record:

```text
FrontDownforce
FloorDownforce
RearDownforce
TotalDownforce
Drag
BalanceFront

FloorHeightFactor
FloorRakeFactor
FloorSealFactor
DiffuserExpansion
DiffuserStallFactor
GlobalLimitFactor
```

Store as a baseline fixture or documented expected range.

---

# Phase 11 — Bug Fix Policy

## AERO-110 — Fix Only Confirmed Defects

**Priority:** P0

Allowed changes in this task:

- incorrect units
- deg/rad mistakes
- ignored JSON fields
- incorrect field mapping
- dead configuration
- double-counted drag
- duplicated downforce
- bad limiter ordering
- invalid underfloor probe inputs
- broken diffuser geometry derivation
- stall state that never recovers
- factors incorrectly clamped
- telemetry that reports values different from runtime physics
- hard limit not respected

Do not implement:

- new ground-effect theory
- CFD approximation
- dynamic pressure center migration
- new vortex/sealing system
- complex porpoising model
- flexible floors
- dynamic wing flex
- major suspension/aero coupling redesign

These belong to Physics V2.

---

# Phase 12 — Final Audit Report

## AERO-120 — Produce `AERO_UNDERFLOOR_AUDIT.md`

**Priority:** P0

The report must include:

### 1. Architecture

```text
JSON
→ config
→ aero.rs
→ simulation.rs
→ chassis force/torque
→ telemetry
```

### 2. Equations

Document actual equations used for:

- wing downforce
- wing drag
- floor downforce
- floor drag
- height factor
- rake factor
- yaw factor
- diffuser stall
- global limiting
- aero balance

### 3. Parameter Matrix

For every JSON aero parameter:

```text
used / unused
units
function
effect
test coverage
```

### 4. F1 2030 Results

Report at least:

```text
100 / 150 / 200 / 250 / 300 km/h

front downforce
floor downforce
rear downforce
total
floor share
wing share
drag
aero balance
limiter factor
```

### 5. Confirmed Bugs

For each bug:

```text
symptom
root cause
fix
regression test
```

### 6. Physics V2 Debt

List conceptual limitations found but intentionally not fixed.

---

# Definition of Done

The underfloor/aero audit is complete only when all of the following are demonstrated:

- [ ] F1 2030 aero JSON is confirmed as the runtime source of truth.
- [ ] Every underfloor JSON parameter has a traced consumer.
- [ ] No unexplained dead underfloor fields remain.
- [ ] Floor load scales approximately with `v²` before limits.
- [ ] `lift_area_m2` scales floor load correctly.
- [ ] Ride height materially affects floor load.
- [ ] ~40 mm is near the configured optimum.
- [ ] The floor degrades below choke height.
- [ ] The floor degrades at excessive ride height.
- [ ] Rake response peaks near +0.5°.
- [ ] Yaw causes progressive floor degradation.
- [ ] Diffuser throat/exit affect the diffuser calculation.
- [ ] Diffuser stall attack/recovery constants work.
- [ ] Raw downforce can be decomposed into front/floor/rear.
- [ ] F1 2030 nominal floor share is measured.
- [ ] Global limits are validated.
- [ ] Global limits preserve component proportions.
- [ ] Drag accounting contains no double counting.
- [ ] `Aero_BalanceFront` has a documented physical meaning.
- [ ] Aero application points are verified.
- [ ] Deterministic regression tests exist.
- [ ] A golden telemetry scenario exists.
- [ ] `AERO_UNDERFLOOR_AUDIT.md` is produced.

---

# Recommended Execution Order

```text
AERO-000
AERO-001
    ↓
AERO-010
AERO-011
    ↓
AERO-020
AERO-021
    ↓
AERO-030
AERO-031
AERO-032
AERO-033
AERO-034
    ↓
AERO-040
AERO-041
AERO-042
    ↓
AERO-050
AERO-051
AERO-052
AERO-053
    ↓
AERO-060
AERO-061
AERO-062
    ↓
AERO-070
AERO-071
AERO-072
AERO-073
    ↓
AERO-080
AERO-081
AERO-082
    ↓
AERO-090
AERO-091
AERO-092
    ↓
AERO-100
AERO-101
    ↓
AERO-110
    ↓
AERO-120
```

---

# Agent Constraint

> Do not redesign `aero.rs` before proving what the current model does. Start with traceability, deterministic tests, telemetry correlation, and component isolation. Modify production physics only when a concrete defect is demonstrated. Record larger conceptual changes as Physics V2 backlog items instead of implementing them in this task.
