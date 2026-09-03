# Formula-90 V10 Physical-Acoustic Refactor — Agent-Oriented Backlog

## 0. Mission

Refactor and extend the existing Formula-90 `v10-engine-synth` crate using the locally available `engine-sim` repository as a technical and algorithmic reference.

The objective is **not** to port `engine-sim` wholesale.

The objective is to improve the existing hybrid Formula-90 model:

```text
physical/procedural simulation
        +
sample tonal/residual layers
        ↓
vehicle-audio-engine
        ↓
Godot runtime
```

The target physical chain is:

```text
crank angle
    ↓
instantaneous cylinder volume
    ↓
compression
    ↓
combustion heat release
    ↓
P / T gas state
    ↓
exhaust valve effective area
    ↓
pressure differential
    ↓
mass flow
    ↓
exhaust runner pressure pulse
```

---

# 1. Mandatory Local Reference

`engine-sim` is already available locally.

Do **not** clone or download another copy unless the local repository is confirmed incomplete or corrupted.

```text
D:\engine-sim
```

Primary files to inspect first:

```text
D:\engine-sim\src\combustion_chamber.cpp
D:\engine-sim\include\combustion_chamber.h

D:\engine-sim\src\piston_engine_simulator.cpp
D:\engine-sim\include\piston_engine_simulator.h

D:\engine-sim\src\synthesizer.cpp
D:\engine-sim\include\synthesizer.h
```

Trace dependencies where necessary, especially:

```text
GasSystem
ExhaustSystem
Intake
CylinderHead
Piston
Crankshaft
Fuel
IgnitionModule
DelayFilter
ConvolutionFilter
DerivativeFilter
ButterworthLowPassFilter
LevelingFilter
```

The local repository is a **reference implementation and algorithm donor**, not an architectural target.

---

# 2. Primary Scope

Primary crate:

```text
game/crates/v10-engine-synth
```

Secondary integration scope, only when required:

```text
game/crates/vehicle-audio-engine
```

Do not move the physical combustion/exhaust model into `vehicle-audio-engine`.

Current architecture to preserve:

```text
vehicle-physics-engine
        │
        │ authoritative telemetry
        ▼
RuntimeTelemetry
        │
        ▼
v10-engine-synth
        │
        ├── physical/procedural V10
        ├── structural response
        ├── exhaust response
        └── sample tonal/residual layer
        │
        ▼
vehicle-audio-engine
        │
        ▼
C ABI / Godot integration
```

The audio engine is a **shadow acoustic engine**.

It must never become authoritative for:

```text
vehicle RPM
vehicle torque
gearbox state
drivetrain acceleration
vehicle physics
```

---

# 3. Agent Operating Rules

## 3.1 Required Behavior

Each agent must:

1. Inspect relevant current Formula-90 code before editing.
2. Inspect relevant `D:\engine-sim` source before porting or adapting behavior.
3. Prefer small, independently testable changes.
4. Preserve the current Rust realtime boundary.
5. Add or update tests with every physical-model change.
6. Record assumptions when physical constants or approximations are introduced.
7. Avoid unrelated cleanup.
8. Keep commits/task patches narrow.
9. Produce diagnostics for behavior that cannot be validated from unit tests alone.
10. Stop and report rather than silently changing vehicle physics ownership.

## 3.2 Forbidden Changes

Agents must not:

- port the entire `engine-sim` codebase;
- create an independent `engine-sim-rs` project;
- duplicate drivetrain simulation;
- move RPM authority into the audio engine;
- add file I/O to the realtime render path;
- add unbounded memory allocations to `render_block`;
- replace the hybrid model with a sample-only or procedural-only system;
- introduce free-running oscillators as substitutes for physically excited structures;
- copy the C++ threading/ring-buffer architecture from `engine-sim`;
- modify Godot integration unless the task explicitly requires it.

---

# 4. Suggested Agent Roles

Use subagents where supported.

## A. `reference-researcher`

Responsibilities:

- inspect `D:\engine-sim`;
- map physical equations and data flow;
- identify reusable algorithms;
- document dependencies;
- distinguish essential physics from application-specific architecture.

Do not edit Formula-90 runtime code unless explicitly assigned.

## B. `rust-physical-modeler`

Responsibilities:

- crank-slider geometry;
- chamber volume;
- gas state;
- compression/expansion;
- combustion heat release;
- valve flow;
- mass flow.

Primary scope:

```text
game/crates/v10-engine-synth/src/thermodynamics/
game/crates/v10-engine-synth/src/cylinder.rs
game/crates/v10-engine-synth/src/config.rs
```

## C. `acoustics-engineer`

Responsibilities:

- runner propagation;
- collector behavior;
- pressure-pulse conversion;
- structural response;
- antialiasing and source conditioning.

Primary scope:

```text
game/crates/v10-engine-synth/src/acoustics.rs
game/crates/v10-engine-synth/src/exhaust/
game/crates/v10-engine-synth/src/scene.rs
```

## D. `hybrid-audio-integrator`

Responsibilities:

- preserve tonal/residual samples;
- phase-aware hybrid mixing;
- procedural/sample responsibility split;
- avoid doubled firing behavior.

Primary scope:

```text
game/crates/v10-engine-synth/src/sample_layer.rs
game/crates/v10-engine-synth/src/runtime.rs
```

## E. `realtime-reviewer`

Responsibilities:

- allocation checks;
- callback safety;
- CPU benchmark review;
- lock/I/O inspection;
- ABI regression review.

## F. `audio-validation-agent`

Responsibilities:

- render reference sweeps;
- stems;
- FFT/spectral comparison;
- aliasing detection;
- transient continuity;
- deterministic regression tests.

---

# 5. Dependency Graph

Recommended execution order:

```text
PHY-001 ─┬─> PHY-002 ──> PHY-010 ──> PHY-011 ──> PHY-012
         │
         └─> PHY-003

PHY-011 ──> PHY-020 ──> PHY-021 ──> PHY-022
PHY-020 ──> PHY-030 ──> PHY-031 ──> PHY-032 ──> PHY-033

PHY-031 ──> PHY-040 ──> PHY-041 ──> PHY-042
PHY-042 ──> PHY-050 ──> PHY-051 ──> PHY-052

PHY-052 ──> PHY-060 ──> PHY-061 ──> PHY-062 ──> PHY-063
PHY-062 ──> PHY-070 ──> PHY-071

PHY-030 ──> PHY-080 ──> PHY-081
PHY-052 ──> PHY-090 ──> PHY-091 ──> PHY-092

PHY-071 ─┬─> PHY-100 ──> PHY-101 ──> PHY-102 ──> PHY-103
PHY-081 ─┘

PHY-103 ──> PHY-110 ──> PHY-111 ──> PHY-112

PHY-052 ──> PHY-120 ──> PHY-121 ──> PHY-122

PHY-020 ──> PHY-130 ──> PHY-131

PHY-071 ──> PHY-140 ──> PHY-141
PHY-110 ──> PHY-150 ──> PHY-151 ──> PHY-152 ──> PHY-153
PHY-110 ──> PHY-160 ──> PHY-161 ──> PHY-162

PHY-110 ──> PHY-170 ──> PHY-171
PHY-003 ──> PHY-180 ──> PHY-181
```

---

# 6. Backlog

## PHY-001 — Freeze Current Audio Baseline

**Priority:** P0  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** none

### Objective

Capture the current `v10-engine-synth` behavior before physical-model changes.

### Tasks

Render reference points at:

```text
3000
5000
8000
11000
14000
15000 RPM
```

Use representative:

```text
throttle = 0.25 / 0.70 / 1.00
load = coast / partial / high
```

Record:

- commit SHA;
- configuration;
- RMS;
- peak;
- crest factor;
- spectral centroid;
- dominant harmonic regions;
- output WAV;
- available stems.

### Deliverables

```text
reports/audio/physical-refactor/baseline/
reports/audio/physical-refactor/baseline_manifest.json
```

### Acceptance Criteria

- Current output can be reproduced deterministically.
- Every future milestone can be A/B compared against this baseline.
- No production code changes.

---

## PHY-002 — Map Current Formula-90 Audio Architecture

**Priority:** P0  
**Owner:** `reference-researcher`  
**Parallelizable:** Yes  
**Blocked by:** none

### Inspect

```text
crank.rs
cylinder.rs
engine.rs
acoustics.rs
config.rs
runtime.rs
scene.rs
sample_layer.rs
```

### Document

- ownership of state;
- current pressure-generation path;
- firing-event generation;
- sample-rate assumptions;
- allocation behavior;
- header/collector architecture;
- sample-layer insertion point;
- realtime facade;
- physical vs nonphysical parameters.

### Deliverable

```text
reports/audio/physical-refactor/current_architecture.md
```

### Acceptance Criteria

Document contains a data-flow diagram from `RuntimeTelemetry` to final audio output.

---

## PHY-003 — Analyze Local `engine-sim`

**Priority:** P0  
**Owner:** `reference-researcher`  
**Parallelizable:** Yes  
**Blocked by:** none

### Mandatory Source

```text
D:\engine-sim
```

### Objective

Trace:

```text
ignition
→ combustion
→ chamber state
→ exhaust valve flow
→ exhaust system
→ delayed pulse
→ synthesizer
```

### For Each Candidate Algorithm Record

```text
source file
source symbol/function
purpose
inputs
outputs
persistent state
equations
dependencies
runtime cost
port / simplify / reject
reason
Formula-90 destination
```

### Deliverable

```text
reports/audio/physical-refactor/engine_sim_reference_map.md
```

### Acceptance Criteria

The report explicitly distinguishes:

- physical equations worth adapting;
- engine-sim-specific rigid-body/application architecture;
- DSP concepts worth preserving;
- components that must not be ported.

---

## PHY-010 — Extend Physical Engine Configuration

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-002, PHY-003

### Add Candidate Parameters

```text
bore_m
stroke_m
rod_length_m
compression_ratio
bank_angle_deg
```

Derived, not duplicated where possible:

```text
crank_radius
swept_volume
clearance_volume
cylinder_displacement
```

### Acceptance Criteria

- Config validation rejects physically impossible geometry.
- Existing default configuration remains loadable or has an explicit migration.
- No arbitrary sound gain is introduced for geometry.

---

## PHY-011 — Implement Crank-Slider Geometry

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-010

### Input

```text
crank_angle
crank_radius
rod_length
```

### Output

```text
piston_position
instantaneous_volume
```

### Constraints

Do not port the rigid-body piston/rod solver from `engine-sim`.

Use analytic slider-crank geometry.

### Acceptance Criteria

- TDC → minimum volume.
- BDC → maximum volume.
- Swept volume matches configured geometry.
- Compression ratio matches config tolerance.
- Pure deterministic function or minimal deterministic state.

---

## PHY-012 — Geometry Unit Tests

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** Yes  
**Blocked by:** PHY-011

Test representative crank positions.

### Acceptance Criteria

```text
V > 0
Vmax > Vmin
Vmax / Vmin ≈ compression_ratio
```

No NaN/Inf.

---

## PHY-020 — Add Lightweight `GasState`

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-011

### Proposed Module

```text
src/thermodynamics/gas.rs
```

### Minimum State

```text
pressure_pa
temperature_k
mass_kg
volume_m3
```

### Derived Quantities

As needed:

```text
density
specific internal energy
speed of sound
```

### Acceptance Criteria

- State is finite and physically bounded.
- No full species chemistry solver.
- No dynamic allocation per cylinder sample.

---

## PHY-021 — Compression and Expansion

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-020

Start with adiabatic/polytropic behavior.

Candidate relationship:

```text
P * V^γ = constant
```

### Acceptance Criteria

- Compression raises P/T.
- Expansion lowers P/T.
- Behavior is continuous over crank angle.
- No handcrafted pressure envelope is required for compression.

---

## PHY-022 — Gas-State Stability Tests

**Priority:** P0  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** PHY-021

Test:

```text
idle
5000
10000
15000+
```

Reject:

```text
NaN
Inf
negative absolute temperature
negative gas mass
nonpositive volume
unbounded pressure
```

---

## PHY-030 — Replace Direct Pressure Envelope With Heat Release

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-021

### Current Concept

```text
fire
→ pressure_shape()
→ pressure
```

### Target

```text
fire
→ heat release
→ internal energy
→ P/T state
```

### Migration Rule

Keep the old pressure path temporarily behind a compatibility/test mode until the new model passes validation.

### Acceptance Criteria

The production physical path can generate cylinder pressure without the legacy handcrafted pressure curve being the primary source.

---

## PHY-031 — Combustion Heat-Release Model

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-030

Candidate model:

```text
Wiebe-style burn fraction
```

Potential parameters:

```text
ignition_angle_deg
burn_duration_deg
shape_factor
combustion_efficiency
fuel_energy_per_cycle
```

### Acceptance Criteria

- Heat release is crank-angle coherent.
- No discontinuous pressure impulses.
- Output remains deterministic from config/seed.

---

## PHY-032 — Couple Combustion Energy to Telemetry

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-031

Inputs remain:

```text
RPM
throttle
load
```

### Acceptance Criteria

At constant RPM, changing load changes:

- cylinder pressure;
- heat release;
- exhaust flow;

not just final master gain.

---

## PHY-033 — Preserve Controlled Cylinder Variation

**Priority:** P1  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** Yes  
**Blocked by:** PHY-031

Preserve:

- fixed cylinder signature;
- low-amplitude per-cycle variation;
- deterministic seeded behavior.

Avoid per-sample random gain.

---

## PHY-040 — Introduce Physical Exhaust Valve Model

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-031

Target:

```text
crank angle
→ valve lift
→ effective area
→ pressure-driven flow
```

---

## PHY-041 — Continuous Valve-Lift Curve

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-040

Candidate config:

```text
EVO
EVC
max_lift_m
valve_diameter_m
discharge_coefficient
opening_shape
closing_shape
```

### Acceptance Criteria

No discontinuities at opening/closing.

---

## PHY-042 — Effective Valve Flow Area

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-041

Convert lift into effective area.

Approximation is acceptable if physically interpretable.

---

## PHY-050 — Exhaust Runner Gas Boundary

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-042

Minimum interface:

```text
cylinder_pressure
cylinder_temperature
runner_pressure
runner_temperature
effective_valve_area
```

Output:

```text
mass_flow
```

---

## PHY-051 — Pressure-Differential Mass Flow

**Priority:** P0  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** No  
**Blocked by:** PHY-050, PHY-003

Use `engine-sim` gas-flow behavior as reference.

### Required Investigation

Inspect the local implementation around:

```text
GasSystem::flow
CombustionChamber::flow
ExhaustSystem
```

### Constraints

Prefer simplified compressible flow over blindly porting the full gas network.

### Acceptance Criteria

Supports stable:

```text
forward flow
near equilibrium
optional reverse flow
```

---

## PHY-052 — Derive Acoustic Excitation From Physical Flow

**Priority:** P0  
**Owner:** `acoustics-engineer`  
**Parallelizable:** No  
**Blocked by:** PHY-051

Evaluate:

```text
mass_flow
Δmass_flow
runner_pressure
Δrunner_pressure
```

for:

```text
header excitation
collector excitation
radiation source
```

### Deliverable

Short decision note with measured waveform/spectral comparison.

---

## PHY-060 — Refactor Header Into Exhaust Runner

**Priority:** P0  
**Owner:** `acoustics-engineer`  
**Parallelizable:** No  
**Blocked by:** PHY-052

Preserve useful existing behavior:

```text
header length
wave delay
reflection
loss
```

Suggested destination:

```text
src/exhaust/runner.rs
```

---

## PHY-061 — Temperature-Dependent Wave Speed

**Priority:** P1  
**Owner:** `acoustics-engineer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-060, PHY-020

Investigate:

```text
speed_of_sound = f(temperature)
```

Preserve optional fixed-speed compatibility mode.

**Status:** Implemented + committed (`1e39d61e`, default ON; fixed `545 m/s` retained bit-exact behind `use_temperature_dependent_wave_speed=false`). `c=sqrt(γRT)`, γ=1.4, R≈287.68 J/(kg·K); runner gas ≈1796 K → ≈850 m/s (vs 545). Running the earlier summary test run showed `63 passed, 1 ignored`; a re-run after the fix shows `98 passed, 0 failed`.

**Regression found during 54-point re-render:** `RunnerWaveguide::process` panicked at `src/exhaust/runner.rs` (`index out of bounds: the len is 96 but the index is 96`) at low-load/off-idle points (reproduced at `r5000_t0.25_l0.50`). Root cause: the fractional read position wraps across the circular-buffer seam — `read = cursor - delay` (tiny negative) `+ len`, and `rem_euclid`/float addition rounds to exactly `len` at fp32 precision (tie-to-even, ULP≈7.6e-6), yielding `i0 = len` → OOB. Secondary: `i1` was clamped to `len-1` so the interpolation never crossed the seam to index 0.

**Fix (not yet committed on top of `1e39d61e`):** in `src/exhaust/runner.rs::process`, use explicit wrap (`if read < 0 { read += n }`), a guard (`if read >= n { read -= n }`) against the fp32 rounding edge, `i1 = (i0+1) % len` for seam interpolation, and floor the temperature at `MIN_RUNNER_TEMPERATURE_K` (buffer-sizing invariant). Verified: rerendering the previously-crashing point completes (`events=2500 peak=0.605448 rms=0.196495 limiter_db=0.000`).

**Remaining:** commit the `runner.rs` crash fix; re-run the full 54-point `render_v10_baseline.ps1`; build manifest; A/B vs frozen PHY-001 + perceptual gate; then mark done.

**Files changed:** `src/config.rs`, `src/engine.rs`, `src/exhaust/runner.rs`, `src/thermodynamics/exhaust_runner.rs`, `src/thermodynamics/gas.rs` (PHY-020 dep), `examples/exhaust_excitation_probe.rs`, `examples/phy061_wave_speed_probe.rs` (new). Decision note: `reports/audio/physical-refactor/phy061_wave_speed_decision.md` (untracked, source-only commit convention).

---

## PHY-062 — Lightweight Runner Pressure State

**Priority:** P1  
**Owner:** `acoustics-engineer`  
**Parallelizable:** No  
**Blocked by:** PHY-060

Preferred complexity:

```text
delay line
+
pressure state
+
loss
+
reflection
+
temperature-aware propagation
```

Do not implement full 1D CFD in this phase.

---

## PHY-063 — Preserve Unequal Primary Lengths

**Priority:** P1  
**Owner:** `acoustics-engineer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-060

Retain per-cylinder physical lengths.

---

## PHY-070 — Preserve Two-Bank Collector Topology

**Priority:** P0  
**Owner:** `acoustics-engineer`  
**Parallelizable:** No  
**Blocked by:** PHY-062

Default:

```text
5 cylinders → collector A
5 cylinders → collector B
```

Only change topology if canonical V10 configuration data justifies it.

---

## PHY-071 — Add Collector Pressure Interaction

**Priority:** P1  
**Owner:** `acoustics-engineer`  
**Parallelizable:** No  
**Blocked by:** PHY-070

Target:

```text
runner pulses
→ collector pressure state
→ pulse interference
→ radiated pressure
```

Keep bounded CPU/state complexity.

---

## PHY-080 — Preserve Physically Excited Block/Head Response

**Priority:** P1  
**Owner:** `acoustics-engineer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-030

Do not replace with autonomous oscillators.

Structural output must remain event/pressure excited.

---

## PHY-081 — Reevaluate Structural Excitation Sources

**Priority:** P1  
**Owner:** `acoustics-engineer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-080

Compare:

```text
pressure
dP/dt
heat-release rate
torque impulse proxy
```

Document selected sources.

---

## PHY-090 — Analyze Useful `engine-sim` DSP Concepts

**Priority:** P1  
**Owner:** `reference-researcher`, `acoustics-engineer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-003, PHY-052

Inspect:

```text
D:\engine-sim\src\synthesizer.cpp
```

Evaluate:

```text
DC removal
derivative path
antialias filtering
jitter
air-noise modulation
convolution
leveling
```

Every imported DSP concept must have an explicit acoustic purpose.

---

## PHY-091 — Reject `engine-sim` Runtime Architecture

**Priority:** P0  
**Owner:** `realtime-reviewer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-090

Explicitly ensure no port of:

```text
std::thread render loop
mutex/condition variable architecture
manual new/delete audio ownership
int16 primary audio path
engine-sim ring-buffer ownership
```

---

## PHY-092 — Prototype Optional IR Stage

**Priority:** P2  
**Owner:** `acoustics-engineer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-090

Possible use:

```text
exhaust radiation
onboard microphone response
external microphone response
engine-bay coloration
```

Physical core must work without IR.

---

## PHY-100 — Preserve Existing Sample Layers

**Priority:** P0  
**Owner:** `hybrid-audio-integrator`  
**Parallelizable:** No  
**Blocked by:** PHY-071, PHY-081

Do not remove:

```text
low
med
max
```

sample zones without a separate approved migration.

---

## PHY-101 — Define Physical vs Sample Responsibilities

**Priority:** P0  
**Owner:** `hybrid-audio-integrator`  
**Parallelizable:** No  
**Blocked by:** PHY-100

Target:

```text
physical model → dynamics, timing, transients, pulse behavior
samples → timbre, recording character, difficult spectral detail
```

Avoid duplicate RPM/load envelopes.

---

## PHY-102 — Phase-Aware Sample Integration

**Priority:** P1  
**Owner:** `hybrid-audio-integrator`  
**Parallelizable:** No  
**Blocked by:** PHY-101

Use where beneficial:

```text
crank_phase_deg
firing phase
physical exhaust envelope
```

Reject obvious:

```text
double firing
beating
chorusing
phase cancellation
```

---

## PHY-103 — Data-Driven Physical/Sample Blend

**Priority:** P1  
**Owner:** `hybrid-audio-integrator`  
**Parallelizable:** Yes  
**Blocked by:** PHY-102

Investigate blend behavior across RPM/load.

Do not assume a final curve before listening tests.

---

## PHY-110 — Preserve `Gf509Runtime` Boundary

**Priority:** P0  
**Owner:** `realtime-reviewer`  
**Parallelizable:** No  
**Blocked by:** PHY-103

Keep runtime contract based on:

```text
RPM
throttle
load
gear
dt_seconds
```

unless a strictly necessary extension is identified.

---

## PHY-111 — Enforce Allocation-Free Render Path

**Priority:** P0  
**Owner:** `realtime-reviewer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-110

The path:

```text
telemetry
→ render_block
→ physical model
→ hybrid sample mix
→ output buffers
```

must perform no steady-state heap allocations.

Preallocate:

```text
delay lines
IR state
sample buffers
LUTs
working state
```

---

## PHY-112 — Enforce No I/O or Blocking in Callback

**Priority:** P0  
**Owner:** `realtime-reviewer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-110

Forbidden in render path:

```text
filesystem
JSON parsing
logging
asset loading
unbounded locks
blocking waits
```

---

## PHY-120 — Investigate Split Physical and Audio Sample Rates

**Priority:** P1  
**Owner:** `audio-validation-agent`, `realtime-reviewer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-052

Benchmark physical simulation at:

```text
12 kHz
16 kHz
24 kHz
48 kHz
```

Audio output remains:

```text
48 kHz
```

---

## PHY-121 — Add Proper Reconstruction if Downsampled

**Priority:** P1  
**Owner:** `acoustics-engineer`  
**Parallelizable:** No  
**Blocked by:** PHY-120

If physical rate < audio rate:

```text
physical source
→ reconstruction / antialias stage
→ 48 kHz acoustic output
```

Reject naive zero-order hold if audible.

---

## PHY-122 — High-RPM Aliasing Validation

**Priority:** P0  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** PHY-121

Test:

```text
12000
15000
17000
18000 RPM
```

Measure:

- harmonic spectrum;
- alias products;
- high-frequency noise;
- spectral folding.

---

## PHY-130 — Separate Physical and Mix Configuration

**Priority:** P1  
**Owner:** `rust-physical-modeler`  
**Parallelizable:** Yes  
**Blocked by:** PHY-020

Physical:

```text
bore
stroke
rod length
compression ratio
ignition angle
burn duration
EVO/EVC
valve geometry
header geometry
collector geometry
gas constants
```

Acoustic/mix:

```text
block gain
head gain
exhaust radiation gain
sample blend
IR amount
master gain
```

---

## PHY-131 — Reduce Arbitrary Tuning Parameters

**Priority:** P2  
**Owner:** `rust-physical-modeler`, `acoustics-engineer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-130

Replace magic constants with physical parameters where practical.

Nonphysical coefficients are allowed only with documented acoustic purpose.

---

## PHY-140 — Add Offline Physical Diagnostic Stems

**Priority:** P1  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** PHY-071

Suggested stems:

```text
cylinder_pressure.wav
combustion_heat_release.wav
valve_area.wav
mass_flow.wav
runner_pressure.wav
headers_A.wav
headers_B.wav
collector_A.wav
collector_B.wav
block.wav
head.wav
exhaust.wav
turbulence.wav
sample_tonal.wav
sample_residual.wav
physical_master.wav
sample_master.wav
hybrid_master.wav
```

No requirement to expose these in runtime.

---

## PHY-141 — Add Offline Physical Telemetry CSV

**Priority:** P1  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** PHY-140

Suggested columns:

```text
time
rpm
crank_angle
cylinder_id
volume
pressure
temperature
valve_lift
valve_area
mass_flow
runner_pressure
collector_pressure
```

---

## PHY-150 — Physical Invariant Tests

**Priority:** P0  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** PHY-110

Validate:

```text
volume > 0
temperature > 0 K
finite pressure
finite mass
finite flow
finite acoustic output
```

---

## PHY-151 — Timing Invariant Tests

**Priority:** P0  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** PHY-150

Validate:

```text
10 firing events / 720°
72° nominal firing spacing
correct firing order
blowdown cannot precede EVO
flow follows valve opening
```

---

## PHY-152 — Load Response Tests

**Priority:** P0  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** PHY-151

At constant RPM compare:

```text
coast
25%
50%
75%
100% load
```

Expected physical state must change before the master mix stage.

---

## PHY-153 — Continuous RPM Sweep Validation

**Priority:** P0  
**Owner:** `audio-validation-agent`  
**Parallelizable:** Yes  
**Blocked by:** PHY-152

Required:

```text
2500 → 15000 RPM
15000 → 2500 RPM
```

Reject:

```text
clicks
phase resets
buffer periodicity
unstable resonance
discontinuities
obvious oscillator tracking
```

---

## PHY-160 — Benchmark Physical Runtime

**Priority:** P1  
**Owner:** `realtime-reviewer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-110

Measure release build:

```text
ns/sample
µs/block
CPU %
allocations
```

for physical-only mode.

---

## PHY-161 — Benchmark Hybrid Runtime

**Priority:** P1  
**Owner:** `realtime-reviewer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-160

Repeat with:

```text
physical model
+
tonal samples
+
residual samples
+
scene processing
```

---

## PHY-162 — Optimize Measured Hotspots Only

**Priority:** P2  
**Owner:** `realtime-reviewer`, implementation agent  
**Parallelizable:** Yes  
**Blocked by:** PHY-161

Likely candidates:

```text
exp
powf
per-cylinder thermodynamics
valve flow
delay lines
modal banks
sample interpolation
```

Do not add LUTs/approximations without before/after validation.

---

## PHY-170 — Preserve Dependency Direction

**Priority:** P0  
**Owner:** `realtime-reviewer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-110

Keep:

```text
vehicle-audio-engine
        ↓
v10-engine-synth
```

Never invert it.

---

## PHY-171 — Preserve Existing C ABI Where Possible

**Priority:** P0  
**Owner:** `hybrid-audio-integrator`, `realtime-reviewer`  
**Parallelizable:** Yes  
**Blocked by:** PHY-170

Godot should not depend on internal physical variables.

Preferred external contract:

```text
telemetry in
audio buffers out
```

---

## PHY-180 — Track Derived `engine-sim` Work

**Priority:** P1  
**Owner:** `reference-researcher`  
**Parallelizable:** Yes  
**Blocked by:** PHY-003

For directly adapted code record:

```text
engine-sim source file
source function/class
Formula-90 destination
nature of adaptation
```

---

## PHY-181 — Add Third-Party Attribution

**Priority:** P1  
**Owner:** implementation agent  
**Parallelizable:** Yes  
**Blocked by:** PHY-180

Create/update:

```text
THIRD_PARTY_NOTICES.md
```

Preserve appropriate Ange Yaghi / `engine-sim` MIT attribution for substantially derived code.

---

# 7. Suggested Sprint Partition

## Sprint A — Research + Physical Foundations

```text
PHY-001
PHY-002
PHY-003
PHY-010
PHY-011
PHY-012
PHY-020
PHY-021
PHY-022
```

Exit condition:

```text
crank angle → physical cylinder volume → stable P/T compression state
```

## Sprint B — Combustion

```text
PHY-030
PHY-031
PHY-032
PHY-033
```

Exit condition:

```text
combustion heat release → physically evolving cylinder pressure/temperature
```

## Sprint C — Valve + Flow

```text
PHY-040
PHY-041
PHY-042
PHY-050
PHY-051
PHY-052
```

Exit condition:

```text
P/T chamber state → valve area → pressure differential → mass flow
```

## Sprint D — Exhaust Acoustics

```text
PHY-060
PHY-061
PHY-062
PHY-063
PHY-070
PHY-071
PHY-080
PHY-081
PHY-090
PHY-091
```

Exit condition:

```text
mass flow → runner pulse → collectors → structural/exhaust acoustic source
```

## Sprint E — Hybrid Integration

```text
PHY-100
PHY-101
PHY-102
PHY-103
PHY-110
PHY-111
PHY-112
```

Exit condition:

Hybrid physical + samples works through the existing realtime boundary.

## Sprint F — Validation + Optimization

```text
PHY-120
PHY-121
PHY-122
PHY-130
PHY-131
PHY-140
PHY-141
PHY-150
PHY-151
PHY-152
PHY-153
PHY-160
PHY-161
PHY-162
PHY-170
PHY-171
PHY-180
PHY-181
```

---

# 8. Agent Handoff Contract

Every completed task should leave a concise handoff containing:

```text
Task:
Status:
Files changed:
Tests added/changed:
Commands executed:
Reference engine-sim files inspected:
Physical assumptions introduced:
Known limitations:
Artifacts/reports generated:
Recommended next task:
```

Do not leave undocumented tuning changes.

---

# 9. Review Gates

## Gate A — Geometry

Required before combustion work is accepted:

- valid crank-slider geometry;
- correct compression ratio;
- deterministic volume.

## Gate B — Thermodynamics

Required before valve-flow work is accepted:

- stable P/T;
- no invalid state;
- compression/expansion coherent.

## Gate C — Combustion

Required before exhaust work is accepted:

- heat-release-driven pressure;
- load-dependent energy;
- deterministic timing.

## Gate D — Flow

Required before acoustics integration:

- valve area continuous;
- flow caused by pressure differential;
- no pre-EVO blowdown.

## Gate E — Realtime

Required before merge to runtime path:

- allocation-free steady state;
- no callback I/O;
- no new blocking locks.

## Gate F — Audio

Required before final acceptance:

- no severe aliasing;
- no discontinuous RPM transitions;
- sample and procedural layers do not double-fire;
- baseline comparison documented;
- high-RPM V10 remains spectrally stable.

---

# 10. Final Target Architecture

```text
Formula-90 vehicle physics
        ↓
RPM / throttle / load
        ↓
crank angle
        ↓
instantaneous cylinder volume
        ↓
compression
        ↓
combustion heat release
        ↓
P / T gas state
        ↓
exhaust valve effective area
        ↓
pressure differential
        ↓
mass flow
        ↓
exhaust runner pressure pulse
        ↓
collector interaction
        ↓
exhaust radiation
        ├────────────────────────────┐
        │                            │
        ▼                            ▼
block/head response           physical exhaust
        │                            │
        └─────────────┬──────────────┘
                      ▼
               physical engine bus
                      │
             ┌────────┴────────┐
             ▼                 ▼
        tonal samples     residual samples
             │                 │
             └────────┬────────┘
                      ▼
                 hybrid V10
                      ↓
               acoustic scene
                      ↓
             vehicle-audio-engine
                      ↓
                    Godot
```

---

# 11. Definition of Done

The refactor is complete when:

- `D:\engine-sim` has been analyzed and its relevant algorithms documented;
- no full `engine-sim` port has been introduced;
- `v10-engine-synth` remains the primary physical/procedural V10 generator;
- crank angle drives instantaneous cylinder volume;
- cylinder geometry drives compression;
- combustion modifies gas internal energy;
- pressure and temperature evolve from physical state;
- exhaust valve area varies continuously with crank angle;
- pressure differential drives exhaust mass flow;
- mass flow excites individual exhaust runners;
- runners preserve physical propagation delay;
- collectors combine bank pulses;
- block/head structures remain physically excited;
- tonal and residual sample layers remain active;
- sample layers complement rather than duplicate physical dynamics;
- vehicle physics remains authoritative for RPM/load/gear;
- realtime rendering performs no file I/O;
- steady-state rendering performs no heap allocation;
- the system remains deterministic from config/seed;
- high-RPM operation is free of severe aliasing;
- diagnostic stems and telemetry can be generated offline;
- existing `vehicle-audio-engine` / C ABI / Godot integration continues to function;
- directly adapted `engine-sim` work is properly attributed.
