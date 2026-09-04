# V10 Audio Engine Load Telemetry — Implementation Backlog

**Repository:** `Anthony38zrgo-2/formula-90`  
**Target branch:** `fix/v10-audio-physical-boundary`  
**Primary objective:** decouple acoustic engine load from accelerator pedal position and expose physically meaningful powertrain state to GF509 / `vehicle-audio-engine`.

---

## 1. Problem statement

The current GF509 update path maps engine load directly to pedal position:

```rust
v10_engine_synth::RuntimeTelemetry {
    rpm: rpm.clamp(0.0, 25_000.0) as f32,
    throttle: throttle.clamp(0.0, 1.0),
    load: throttle.clamp(0.0, 1.0),
    gear: gear.clamp(-1, 12) as i8,
    dt_seconds: 0.0,
}
```

This collapses two different concepts into one value:

- **Throttle** = driver command / throttle opening.
- **Engine load** = mechanical/acoustic operating state of the engine and drivetrain.

As a result, GF509 cannot distinguish:

- closed throttle with strong engine braking;
- partial throttle at high RPM;
- open throttle with reduced delivered torque under traction-control intervention;
- gear shifts with torque cut;
- downshift blips;
- rev-limiter operation;
- acceleration versus steady-state cruising at similar throttle;
- unloaded/free-rev conditions versus a coupled drivetrain.

The required first-order telemetry contract is:

```text
rpm
throttle
normalized_engine_load
normalized_engine_torque
torque_sign
rpm_derivative
throttle_derivative
gear
shift_phase
```

The change must preserve the existing sample-accurate rendering architecture and must not move powertrain physics into the audio engine.

---

## 2. Repository state relevant to this change

### Vehicle physics already exposes useful authoritative state

The Rust vehicle powertrain already tracks:

```text
rpm
current_gear
target_gear
clutch_engagement
shift_timer
engine_torque
clutch_torque
is_rev_limited
drive_torques
drive_torques_pre_tc
tc_active
tc_cut_ratio
tc_cut_ratio_smoothed
engine_attack_limited_torque
```

The Godot-facing `F194RustVehicle` already exposes at least:

```text
get_engine_torque()
get_clutch_torque()
get_clutch_engagement()
get_drive_torques()
get_powertrain_state_snapshot()
```

Therefore, the audio layer should consume this state instead of reconstructing engine load from throttle.

### Current audio bridge

Current flow:

```text
F194RustVehicle
    ↓
VehicleAudioControllerNative::_physics_process()
    ↓
vehicle_audio_set_state(...)
    ↓
VehicleAudioEngine::set_state(...)
    ↓
Gf509Runtime::update_telemetry(...)
    ↓
GF509 synthesis
```

### Current ABI inconsistency to resolve during migration

At the time this backlog was written:

- Rust `vehicle-audio-engine/src/ffi.rs` declares `VEHICLE_AUDIO_ABI_VERSION = 2`.
- C++ `vehicle_audio_controller_native.hpp` declares `EXPECTED_ABI_VERSION = 1`.

Do not build the new telemetry contract on top of an unresolved ABI mismatch.

---

# 3. Target architecture

The canonical ownership boundaries should be:

```text
PHYSICS / POWERTRAIN
authoritative mechanical state
        │
        ▼
AUDIO TELEMETRY ADAPTER
normalization + derivatives + semantic shift state
        │
        ▼
VERSIONED AUDIO ABI
plain POD telemetry packet
        │
        ▼
VEHICLE AUDIO ENGINE
smoothing / routing / event logic
        │
        ▼
GF509
acoustic response to RPM + throttle + load + torque state
```

Rules:

1. Physics owns torque, clutch, TC and shift truth.
2. The adapter may normalize, clamp, derive temporal rates and classify state.
3. GF509 must not query vehicle nodes or reproduce vehicle physics.
4. The audio callback must remain allocation-free.
5. Telemetry values must be deterministic and unit-defined.
6. `throttle` and `normalized_engine_load` must never be aliases after this migration.

---

# 4. Canonical telemetry semantics

## 4.1 Required contract

| Field | Type | Range / unit | Meaning |
|---|---:|---|---|
| `rpm` | `f32`/`f64` | RPM | Authoritative engine speed |
| `throttle` | `f32` | `[0,1]` | Driver throttle command/opening |
| `normalized_engine_load` | `f32` | `[0,1]` | Magnitude of mechanically delivered/coupled engine load used for acoustic timbre |
| `normalized_engine_torque` | `f32` | `[-1,1]` | Signed engine torque normalized against canonical maximum torque |
| `torque_sign` | enum / `i8` | `-1,0,+1` | Coast/neutral/power sign after dead-zone |
| `rpm_derivative` | `f32` | RPM/s | Smoothed `dRPM/dt` |
| `throttle_derivative` | `f32` | 1/s | Smoothed `dThrottle/dt` |
| `gear` | `i8`/`i32` | `-1..N` | Current engaged gear |
| `shift_phase` | enum | see below | Acoustic phase of current transmission event |

Additional existing telemetry should remain available:

```text
idle_rpm
max_rpm
speed_kph
slip
surface
tc_cut_ratio
rev_limiter_active
clutch_engagement
```

The additional fields are recommended even if they are not part of the minimum GF509 contract.

---

# 5. Definition of `normalized_engine_torque`

Use the authoritative signed `engine_torque` from the powertrain.

First implementation:

```text
normalized_engine_torque =
    clamp(engine_torque / max_torque, -1.0, 1.0)
```

Do not use:

```text
throttle * torque_curve
```

inside the audio subsystem if authoritative `engine_torque` is already available.

Reason:

`engine_torque` already contains the effects of:

- torque curve;
- throttle;
- variable engine drag;
- constant engine braking;
- rev limiter;
- shift torque cut.

This allows audio to distinguish positive combustion torque from negative overrun torque.

---

# 6. Definition of `torque_sign`

Use a dead-zone to avoid rapid sign toggling around zero.

Recommended initial rule:

```text
if normalized_engine_torque > +0.025:
    torque_sign = +1
elif normalized_engine_torque < -0.025:
    torque_sign = -1
else:
    torque_sign = 0
```

Semantic meaning:

```text
+1 = positive/power
 0 = neutral/light/no meaningful torque
-1 = negative/coast/engine braking
```

The dead-zone must be configurable or centralized as a named constant.

---

# 7. Definition of `normalized_engine_load`

## 7.1 Important distinction

`normalized_engine_load` should not be a synonym for signed engine torque.

For acoustic purposes it should answer:

> How strongly is the engine mechanically coupled and working against the drivetrain at this instant?

A useful first implementation can be derived from clutch-transmitted torque and actual delivered torque.

## 7.2 V1 recommended formula

Define:

```text
clutch_capacity =
    max_torque * max_clutch_torque_ratio
```

Then:

```text
coupled_load =
    abs(clutch_torque) / max(clutch_capacity, epsilon)
```

Clamp:

```text
coupled_load = clamp(coupled_load, 0.0, 1.0)
```

If traction control is active:

```text
tc_delivery =
    1.0 - clamp(tc_cut_ratio_smoothed, 0.0, 1.0)
```

Then:

```text
normalized_engine_load =
    clamp(coupled_load * tc_delivery, 0.0, 1.0)
```

This gives several useful properties:

| Situation | Throttle | Torque sign | Load |
|---|---:|---:|---:|
| Full acceleration | high | + | high |
| Partial throttle cruise | medium | + | low/medium |
| Lift-off engine braking | low | - | medium/high |
| Clutch disengaged | any | any | near zero |
| Shift torque cut | any | 0/- | near zero |
| TC intervention | high | + | reduced |
| Free rev | high | + | low |
| High RPM steady load | medium | + | load dependent |

## 7.3 Fallback

If clutch torque/capacity is temporarily unavailable:

```text
normalized_engine_load =
    abs(normalized_engine_torque)
```

This fallback must be explicitly logged or exposed in diagnostics.

Do not silently fall back to:

```text
load = throttle
```

---

# 8. Shift phase contract

Do not represent shifting only by detecting a changed integer gear.

Introduce a small state machine intended for audio.

Recommended enum:

```rust
#[repr(u8)]
enum ShiftPhase {
    None = 0,
    UpshiftCut = 1,
    UpshiftRecovery = 2,
    DownshiftCut = 3,
    DownshiftBlip = 4,
    DownshiftRecovery = 5,
}
```

Optional future states:

```text
NeutralTransition
ClutchSlip
MissedShift
LimiterCut
```

Do not add these until required.

## 8.1 Authoritative inputs

Use:

```text
current_gear
target_gear
shift_timer
clutch_engagement
rpm
rpm_derivative
```

## 8.2 Initial phase derivation

### Upshift

When:

```text
shift_timer > 0
target_gear > current_gear
```

publish:

```text
UpshiftCut
```

When the gear engages and `shift_timer` reaches zero, latch:

```text
UpshiftRecovery
```

for an audio-only recovery window, initially:

```text
40–90 ms
```

### Downshift

When:

```text
shift_timer > 0
target_gear < current_gear
```

publish:

```text
DownshiftCut
```

If a positive RPM acceleration / throttle pulse associated with the downshift is detected, publish:

```text
DownshiftBlip
```

Otherwise the audio layer may synthesize a short blip phase from the downshift event until the physics model gains an explicit throttle-blip state.

After gear engagement:

```text
DownshiftRecovery
```

for an initial:

```text
60–120 ms
```

### Important

The recovery timer is an acoustic state and may live in the audio telemetry adapter.

The actual `shift_timer`, gear selection and torque cut remain physics-owned.

---

# 9. Derivative signals

## 9.1 `rpm_derivative`

Calculate from authoritative RPM:

```text
raw_rpm_derivative =
    (rpm_now - rpm_previous) / max(dt, epsilon)
```

Unit:

```text
RPM/s
```

Apply a low-pass filter to prevent physics-step noise from entering GF509.

Initial time constant:

```text
tau = 20–40 ms
```

Recommended clamp for defensive safety:

```text
[-100000, +100000] RPM/s
```

The clamp is a safety bound, not normal operating behavior.

## 9.2 `throttle_derivative`

```text
raw_throttle_derivative =
    (throttle_now - throttle_previous) / max(dt, epsilon)
```

Unit:

```text
normalized throttle units per second
```

Use the same or slightly faster filter:

```text
tau = 15–30 ms
```

Recommended safety clamp:

```text
[-50, +50] 1/s
```

## 9.3 Why derivatives matter

GF509 can use these values to distinguish:

```text
constant throttle
fast lift-off
fast pedal application
RPM flare
RPM collapse during shift
downshift blip
limiter oscillation
```

without inferring all temporal behavior from a single instantaneous value.

---

# 10. ABI redesign

## P0 recommendation

Do not keep extending the existing positional function:

```c
vehicle_audio_set_state(
    handle,
    rpm,
    idle_rpm,
    max_rpm,
    throttle,
    speed_kph,
    gear,
    slip,
    surface
)
```

The signature is already large and will become fragile.

Introduce a versioned POD telemetry packet.

Example conceptual layout:

```c
typedef struct VehicleAudioTelemetryV3 {
    uint32_t schema_version;
    uint32_t struct_size;

    double rpm;
    double idle_rpm;
    double max_rpm;

    float throttle;
    float normalized_engine_load;
    float normalized_engine_torque;
    float rpm_derivative;
    float throttle_derivative;

    double speed_kph;
    float slip;

    int32_t gear;
    int32_t torque_sign;
    int32_t shift_phase;

    float clutch_engagement;
    float tc_cut_ratio;
    uint32_t rev_limiter_active;
} VehicleAudioTelemetryV3;
```

Surface may remain a separate argument initially:

```c
vehicle_audio_set_telemetry(
    void* handle,
    const VehicleAudioTelemetryV3* telemetry,
    const char* surface
)
```

This keeps string ownership outside the POD struct and limits migration scope.

## ABI rules

- Use `#[repr(C)]` on the Rust mirror.
- Add `schema_version`.
- Add `struct_size`.
- Avoid Rust `bool` in the C struct; use fixed-width integer flags.
- Add compile-time/static size assertions on C++.
- Add Rust tests for field offsets if practical.
- Bump the audio ABI version once.
- Update both Rust and C++ expected versions in the same commit.
- Keep the legacy `vehicle_audio_set_state` temporarily only if a compatibility consumer still requires it.

---

# 11. Runtime telemetry model in `v10-engine-synth`

Replace:

```rust
pub struct RuntimeTelemetry {
    pub rpm: f32,
    pub throttle: f32,
    pub load: f32,
    pub gear: i8,
    pub dt_seconds: f32,
}
```

with an expanded contract.

Recommended:

```rust
pub struct RuntimeTelemetry {
    pub rpm: f32,
    pub throttle: f32,

    pub normalized_engine_load: f32,
    pub normalized_engine_torque: f32,
    pub torque_sign: TorqueSign,

    pub rpm_derivative: f32,
    pub throttle_derivative: f32,

    pub gear: i8,
    pub shift_phase: ShiftPhase,

    pub dt_seconds: f32,
}
```

Optional fields to retain inside the parent vehicle-audio runtime:

```text
tc_cut_ratio
rev_limiter_active
clutch_engagement
```

They do not need to reach every lower DSP component immediately.

---

# 12. GF509 control mapping

## 12.1 Preserve throttle as its own control

Throttle should primarily affect:

- combustion excitation;
- intake opening;
- transient pedal response;
- positive power intent.

It should not directly define all acoustic load.

## 12.2 Use normalized engine load for timbre

`normalized_engine_load` should influence:

- exhaust body/resonance;
- combustion pressure weighting;
- low/mid harmonic density;
- sample-layer load interpolation;
- intake/exhaust balance.

## 12.3 Use signed torque state for coast/power separation

Example conceptual mapping:

```text
torque_sign > 0:
    power character

torque_sign < 0:
    overrun / negative-torque character

torque_sign == 0:
    unloaded / transition character
```

Avoid a hard timbral switch at zero torque.

Use a short crossfade or smoothed signed-torque control.

## 12.4 Use derivatives for transient emphasis

Possible first-order behavior:

```text
throttle_derivative >> 0
    transient intake / attack

throttle_derivative << 0
    lift-off / exhaust transition

rpm_derivative >> 0
    acceleration / flare

rpm_derivative << 0
    torque cut / braking / downshift settling
```

Keep transient gains small initially.

Do not use derivatives to redefine base loudness.

---

# 13. Detailed implementation backlog

---

## EPIC A — Baseline and contract freeze

### A-01 — Capture current telemetry behavior

**Priority:** P0

Record deterministic scenarios before changing behavior:

```text
idle
free rev
full-throttle acceleration
partial-throttle acceleration
steady high-RPM throttle hold
lift-off
TC intervention
upshift
downshift
rev limiter
```

Capture:

```text
rpm
throttle
engine_torque
clutch_torque
clutch_engagement
drive_torques_pre_tc
drive_torques
tc_cut_ratio_smoothed
shift_timer
current_gear
target_gear
is_rev_limited
current GF509 load
```

**Deliverable**

```text
docs/audio-design/telemetry-load-baseline.json/csv
```

**Acceptance**

A replay/test can prove that current GF509 receives `load == throttle`.

---

### A-02 — Freeze field semantics

**Priority:** P0

Document exact definitions and units for all new telemetry fields.

**Acceptance**

No field is described with ambiguous terms such as “power” or “load” without a formula.

---

## EPIC B — Fix and version the audio ABI

### B-01 — Reconcile current ABI version mismatch

**Priority:** P0

Files:

```text
game/crates/vehicle-audio-engine/src/ffi.rs
native/include/formula90s/presentation/vehicle_audio_controller_native.hpp
native/src/presentation/vehicle_audio_controller_native.cpp
```

Tasks:

- determine the intended current ABI version;
- align Rust exported version and C++ expected version;
- ensure build SHA checks still work;
- add a regression test or startup assertion.

**Acceptance**

The audio DLL loads with no ABI mismatch on the target branch.

---

### B-02 — Introduce `VehicleAudioTelemetryV3`

**Priority:** P0

Create one POD packet shared semantically by Rust and C++.

Tasks:

- add C++ struct;
- add `#[repr(C)]` Rust struct;
- add `schema_version`;
- add `struct_size`;
- use fixed-width scalar types;
- add validation.

**Acceptance**

The packet round-trips through an ABI test with exact expected values.

---

### B-03 — Add `vehicle_audio_set_telemetry`

**Priority:** P0

New API:

```text
vehicle_audio_set_telemetry(handle, packet, surface)
```

Keep `vehicle_audio_set_state` temporarily if required by tests or legacy callers.

**Acceptance**

GF509 can be driven exclusively from the new packet.

---

## EPIC C — Expose authoritative powertrain state to the audio adapter

### C-01 — Consume `get_powertrain_state_snapshot()`

**Priority:** P0

Prefer a single snapshot call over many independent property reads if the snapshot already contains the required values.

Required source values:

```text
engine_torque
clutch_torque
clutch_engagement
shift_timer
current_gear
target_gear
is_rev_limited
tc_cut_ratio_smoothed
```

Fallback to existing getters only where snapshot data is missing.

**Acceptance**

No new audio load value is reconstructed from throttle alone.

---

### C-02 — Expose missing max-clutch/load normalization inputs

**Priority:** P1

If the audio bridge cannot access:

```text
max_torque
max_clutch_torque_ratio
```

either:

1. expose the canonical values through vehicle telemetry/config access; or
2. expose a pre-normalized physics-side load metric.

Prefer avoiding duplicated configuration parsing inside the audio controller.

**Acceptance**

Normalization denominator comes from the same canonical vehicle config as physics.

---

## EPIC D — Implement normalized torque and load

### D-01 — Calculate `normalized_engine_torque`

**Priority:** P0

Formula:

```text
engine_torque / max_torque
```

Clamp to:

```text
[-1, 1]
```

**Tests**

- full acceleration > 0;
- lift-off at RPM < 0;
- limiter <= 0 when torque cut is active;
- shift cut <= 0 or near zero.

---

### D-02 — Calculate `torque_sign`

**Priority:** P0

Initial dead-zone:

```text
±0.025 normalized torque
```

**Tests**

No sign chatter around zero under a steady idle/transition trace.

---

### D-03 — Calculate coupled engine load

**Priority:** P0

Initial formula:

```text
coupled_load =
    abs(clutch_torque) /
    (max_torque * max_clutch_torque_ratio)

delivery =
    1 - tc_cut_ratio_smoothed

normalized_engine_load =
    clamp(coupled_load * delivery, 0, 1)
```

Protect all denominators.

**Tests**

- free rev load lower than acceleration at similar throttle;
- clutch-open shift load falls sharply;
- TC reduces load despite unchanged throttle;
- engine braking retains nonzero load with negative torque;
- no NaN/Inf.

---

### D-04 — Add diagnostic fallback

**Priority:** P1

If authoritative clutch/load inputs are unavailable:

```text
normalized_engine_load =
    abs(normalized_engine_torque)
```

Expose diagnostic flag:

```text
load_source = authoritative | torque_fallback
```

Never fall back to throttle.

---

## EPIC E — Derivatives and temporal state

### E-01 — Add stable physics delta to audio adapter

**Priority:** P0

Change:

```cpp
_physics_process(double)
```

to actually consume its `delta`.

Protect against:

```text
delta <= 0
large pause/resume delta
first-frame derivative spikes
```

---

### E-02 — Implement RPM derivative

**Priority:** P0

Store previous RPM.

Compute:

```text
dRPM/dt
```

Filter with named constant.

Suggested initial:

```text
tau = 0.030 s
```

Reset derivative history on:

```text
_ready
vehicle reset
teleport/reset
large discontinuity
```

---

### E-03 — Implement throttle derivative

**Priority:** P0

Same pattern as RPM derivative.

Suggested initial:

```text
tau = 0.020 s
```

---

### E-04 — Add derivative telemetry tests

**Priority:** P1

Verify signs for:

```text
acceleration
lift-off
upshift RPM drop
downshift RPM rise
steady throttle
```

---

## EPIC F — Shift phase state machine

### F-01 — Define shared `ShiftPhase`

**Priority:** P0

Canonical enum:

```text
None
UpshiftCut
UpshiftRecovery
DownshiftCut
DownshiftBlip
DownshiftRecovery
```

Rust/C++ numeric values must match.

---

### F-02 — Derive active shift direction from powertrain state

**Priority:** P0

Use:

```text
target_gear vs current_gear
shift_timer
```

Do not infer direction solely from a post-change gear integer.

---

### F-03 — Add recovery latch

**Priority:** P1

After engagement:

```text
upshift recovery: 40–90 ms
downshift recovery: 60–120 ms
```

Expose constants in one audio configuration location.

---

### F-04 — Add provisional downshift blip detection

**Priority:** P1

Until physics owns an explicit blip state, classify a blip only during a downshift window and when one or more conditions are met:

```text
throttle_derivative > threshold
rpm_derivative > threshold
```

Do not generate arbitrary blip events outside a confirmed downshift.

---

### F-05 — Remove duplicate shift ownership

**Priority:** P0

Current code can trigger shift one-shots from both:

- `VehicleAudioEngine::set_state()` gear-change detection;
- `VehicleAudioControllerNative::_physics_process()` gear-change detection.

During the migration, define exactly one owner for shift event triggering.

Preferred:

```text
physics/telemetry publishes shift phase/event
Rust audio runtime owns sound triggering
C++ bridge remains transport-only
```

**Acceptance**

One physical shift produces exactly one logical shift event plus explicitly configured companion layer(s), never duplicate triggers.

---

## EPIC G — Expand `VehicleAudioEngine` state

### G-01 — Replace large positional `set_state`

**Priority:** P0

Prefer:

```rust
pub fn set_telemetry(&mut self, telemetry: VehicleAudioTelemetry)
```

over another extended positional signature.

Store raw/latest:

```text
last_normalized_engine_load
last_normalized_engine_torque
last_torque_sign
last_rpm_derivative
last_throttle_derivative
last_shift_phase
```

---

### G-02 — Preserve smoothing ownership

**Priority:** P1

Differentiate:

```text
physics-derived filtering:
    derivative cleanup

audio interpolation/smoothing:
    block-to-block / sample-to-sample continuity
```

Do not smooth a signal repeatedly without documenting both filters.

---

### G-03 — Add telemetry diagnostics

**Priority:** P1

Expose new fields through existing diagnostics/telemetry tooling.

At minimum:

```text
throttle
engine_load
engine_torque_norm
torque_sign
rpm_dot
throttle_dot
shift_phase
tc_cut
rev_limiter
```

---

## EPIC H — Expand `v10-engine-synth::RuntimeTelemetry`

### H-01 — Replace `load` with explicit fields

**Priority:** P0

Remove semantic ambiguity:

```rust
load
```

→

```rust
normalized_engine_load
normalized_engine_torque
torque_sign
```

Do not leave both `load` and `normalized_engine_load` as separate aliases.

---

### H-02 — Extend validation

**Priority:** P0

Validate:

```text
rpm finite
throttle [0,1]
normalized_engine_load [0,1]
normalized_engine_torque [-1,1]
torque_sign valid enum
rpm_derivative finite
throttle_derivative finite
gear range valid
shift_phase valid enum
dt_seconds valid
```

Invalid telemetry should return an error without corrupting runtime state.

---

### H-03 — Interpolate appropriate fields

**Priority:** P0

Sample-interpolate continuous fields:

```text
rpm
throttle
normalized_engine_load
normalized_engine_torque
rpm_derivative
throttle_derivative
```

Do not numerically interpolate enums:

```text
torque_sign
gear
shift_phase
```

Use target/current discrete state with explicit transition behavior.

---

## EPIC I — Map load into GF509 synthesis

### I-01 — Remove `load = throttle`

**Priority:** P0

Hard acceptance condition:

No production GF509 path may contain semantic logic equivalent to:

```text
load = throttle
```

except explicit test fixtures.

---

### I-02 — Feed physical engine input

Where GF509 currently receives:

```rust
EngineInput {
    rpm,
    throttle,
    load,
}
```

feed:

```text
load = normalized_engine_load
```

---

### I-03 — Preserve signed torque separately

**Priority:** P1

Do not encode coast by making `load` negative.

Keep:

```text
load ∈ [0,1]
torque ∈ [-1,1]
```

This provides a 2D acoustic state:

```text
load magnitude × torque direction
```

---

### I-04 — Add coast/power continuous blend

**Priority:** P1

Introduce a smoothed signed-torque state into the synthesis path.

Initial target:

```text
power:
    positive torque

coast:
    negative torque

unloaded:
    near-zero torque + low load
```

Avoid hard switching.

---

### I-05 — Add derivative-driven transient modulation

**Priority:** P2

Use only after baseline load behavior passes.

Candidate subtle controls:

```text
positive throttle derivative → intake attack
negative throttle derivative → lift-off transient
negative RPM derivative during UpshiftCut → torque-cut character
positive RPM derivative during DownshiftBlip → blip emphasis
```

All gains must start conservatively.

---

## EPIC J — Rev limiter and TC integration

### J-01 — Forward rev limiter state

**Priority:** P1

Physics already tracks:

```text
is_rev_limited
```

Expose it to vehicle audio telemetry.

Do not infer limiter state from:

```text
rpm >= max_rpm
```

inside multiple audio layers if the physics state is available.

---

### J-02 — Forward TC cut ratio

**Priority:** P1

Expose:

```text
tc_cut_ratio_smoothed
```

This enables the synth to distinguish:

```text
full throttle + full delivered torque
full throttle + active torque intervention
```

---

### J-03 — Avoid double application of TC

**Priority:** P1

If `normalized_engine_load` already includes TC delivery reduction, do not multiply base engine loudness by `(1 - tc_cut)` again unless deliberately modeling combustion cuts.

If a separate TC combustion texture is added later, it should be a transient/timbre layer rather than another blind gain reduction.

---

# 14. Tests

## 14.1 Unit tests — physics-to-audio derivation

Create deterministic input cases.

### Case 1 — Full acceleration

Expected:

```text
throttle ≈ 1
torque > 0
torque_sign = +1
load high
shift_phase = None
```

### Case 2 — Lift-off at high RPM

Expected:

```text
throttle ≈ 0
torque < 0
torque_sign = -1
load > 0
throttle_derivative < 0
```

Critical assertion:

```text
load != throttle
```

---

### Case 3 — Free rev / clutch open

Expected:

```text
throttle high
positive engine torque possible
clutch engagement low
normalized_engine_load low
```

---

### Case 4 — TC intervention

Expected:

```text
throttle high
tc_cut > 0
normalized_engine_load reduced
```

---

### Case 5 — Upshift

Expected sequence:

```text
None
→ UpshiftCut
→ UpshiftRecovery
→ None
```

Load should collapse during cut and recover smoothly.

---

### Case 6 — Downshift

Expected sequence:

```text
None
→ DownshiftCut
→ DownshiftBlip
→ DownshiftRecovery
→ None
```

If provisional blip detection is not triggered in a given physical trace:

```text
None
→ DownshiftCut
→ DownshiftRecovery
→ None
```

must remain valid.

---

### Case 7 — Rev limiter

Expected:

```text
rev_limiter_active = true
positive torque suppressed
throttle may remain high
```

Critical assertion:

The audio state must not interpret high throttle as high positive engine load automatically.

---

## 14.2 ABI tests

Test:

- Rust ABI version == C++ expected version.
- Struct size matches.
- Enum numeric values match.
- Known packet pattern is received without field corruption.
- invalid schema version is rejected safely.
- invalid `struct_size` is rejected safely.

---

## 14.3 Audio regression renders

Produce offline renders for the same RPM trajectory with different load states:

```text
A: 15000 RPM, throttle 1.0, load 1.0, positive torque
B: 15000 RPM, throttle 1.0, load 0.25, positive torque
C: 15000 RPM, throttle 0.0, load 0.55, negative torque
D: 15000 RPM, throttle 0.0, load 0.05, near-zero torque
```

Acceptance:

The four renders must not be identical after normalization.

---

# 15. Telemetry logging format

Add an optional debug trace row:

```csv
time_s,
rpm,
throttle,
engine_load,
engine_torque_norm,
torque_sign,
rpm_dot,
throttle_dot,
gear,
target_gear,
shift_phase,
clutch_engagement,
tc_cut,
rev_limiter
```

This trace should be usable by Python scripts for:

```text
plots
event alignment
comparison against onboard audio
regression metrics
```

Do not log from the real-time audio callback.

---

# 16. Performance requirements

The migration must preserve:

- no file I/O in render callback;
- no heap allocation in the per-sample hot path;
- no Godot node access from GF509;
- no string processing per audio sample;
- no mutex acquisition in render callback;
- bounded control-state validation;
- existing max block size assumptions.

A telemetry packet is copied once per physics/control update, not reconstructed per sample.

---

# 17. Suggested file-level change map

## C++ / Godot bridge

```text
native/include/formula90s/presentation/vehicle_audio_controller_native.hpp
native/src/presentation/vehicle_audio_controller_native.cpp
```

Changes:

- ABI version synchronization;
- telemetry POD;
- new function pointer;
- derivative state;
- powertrain snapshot extraction;
- load/torque normalization;
- shift-phase adapter;
- telemetry diagnostics.

---

## Rust audio FFI

```text
game/crates/vehicle-audio-engine/src/ffi.rs
```

Changes:

- ABI bump;
- `#[repr(C)]` telemetry packet;
- `vehicle_audio_set_telemetry`;
- validation;
- compatibility shim if retained.

---

## Rust audio mixer

```text
game/crates/vehicle-audio-engine/src/mixer.rs
```

Changes:

- `set_telemetry`;
- new cached telemetry fields;
- eliminate `load: throttle`;
- one authoritative shift trigger path;
- GF509 forwarding;
- TC/limiter forwarding.

---

## GF509 runtime

```text
game/crates/v10-engine-synth/src/runtime.rs
```

Changes:

- expanded `RuntimeTelemetry`;
- validation;
- interpolation;
- discrete-state handling;
- forward new state to engine/sample layer.

---

## GF509 engine/synthesis

Likely:

```text
game/crates/v10-engine-synth/src/engine.rs
game/crates/v10-engine-synth/src/sample_layer.rs
game/crates/v10-engine-synth/src/acoustics.rs
```

Changes should be incremental:

1. use real `normalized_engine_load`;
2. expose signed torque;
3. add coast/power blend;
4. add transient derivative response.

Do not modify all acoustic subsystems in the same first commit.

---

## Physics source

Primarily read existing state from:

```text
game/crates/vehicle-physics-engine/src/powertrain.rs
```

Only add new physics-facing telemetry APIs if current snapshots do not expose the required canonical data.

Avoid changing actual vehicle dynamics as part of this backlog.

---

# 18. Recommended commit sequence

## Commit 1 — ABI and telemetry scaffolding

```text
fix current ABI mismatch
introduce versioned telemetry packet
no intentional sound change
```

Gate:

```text
build + ABI tests + existing audio baseline
```

---

## Commit 2 — Authoritative torque telemetry

```text
engine_torque
clutch_torque
clutch_engagement
tc_cut
limiter
```

Still allow old load behavior behind a temporary compatibility switch.

Gate:

```text
telemetry trace correctness
```

---

## Commit 3 — Load derivation

Replace:

```text
load = throttle
```

with:

```text
load = normalized coupled load
```

Gate:

```text
scenario unit tests
offline A/B renders
```

---

## Commit 4 — Derivatives

Add:

```text
rpm_derivative
throttle_derivative
```

No major synthesis modulation yet.

Gate:

```text
trace quality
no spikes/reset artifacts
```

---

## Commit 5 — Shift phase

Add:

```text
upshift cut/recovery
downshift cut/blip/recovery
```

Remove duplicate shift trigger ownership.

Gate:

```text
one event per shift
deterministic phase sequence
```

---

## Commit 6 — GF509 timbre mapping

Use:

```text
load
signed torque
shift phase
```

to change timbre.

Gate:

```text
four-state fixed-RPM comparison
Bahrain-reference listening test
```

---

## Commit 7 — Transient refinement

Use derivatives for:

```text
attack
lift-off
RPM flare
shift recovery
```

Gate:

```text
no pumping
no zipper noise
no discontinuities
```

---

# 19. Acceptance criteria for the complete backlog

The implementation is complete when all of the following are true:

- [ ] GF509 no longer receives `load = throttle`.
- [ ] `throttle` remains an independent control.
- [ ] Engine torque comes from authoritative powertrain state.
- [ ] Negative engine braking torque reaches audio as a distinct state.
- [ ] Mechanical load remains nonzero during meaningful engine braking.
- [ ] Free-rev/load-light behavior differs from coupled acceleration.
- [ ] TC can reduce delivered load without reducing throttle.
- [ ] Rev-limiter state is explicit.
- [ ] RPM derivative is available and stable.
- [ ] Throttle derivative is available and stable.
- [ ] Shift phase is explicit and deterministic.
- [ ] Shift events are not double-triggered.
- [ ] C++ and Rust use one matching audio ABI version.
- [ ] ABI packet size/version are validated.
- [ ] GF509 telemetry validation rejects invalid values safely.
- [ ] Continuous fields interpolate smoothly.
- [ ] Enums are not numerically interpolated.
- [ ] No allocation/I/O is introduced into the audio render callback.
- [ ] Fixed-RPM renders with different torque/load conditions are measurably and audibly different.
- [ ] Existing surface, tyre, impact and one-shot behavior remains functional.

---

# 20. Explicit non-goals

Do **not** include these in the same implementation unless required to unblock the telemetry migration:

- redesign of the V10 sample bank;
- Bahrain audio extraction/remastering;
- new convolution/reverb system;
- full engine thermodynamics rewrite;
- new vehicle physics behavior;
- new traction-control physics;
- transmission ratio changes;
- sample RPM anchor redesign;
- spatial audio redesign;
- major mastering/EQ retune.

Those should be evaluated after the runtime can correctly distinguish real engine operating states.

---

# 21. Final target behavior

The final control surface should allow these states to coexist at the same RPM:

```text
15000 RPM + full throttle + positive high load
15000 RPM + full throttle + TC-reduced load
15000 RPM + partial throttle + medium load
15000 RPM + closed throttle + negative high load
15000 RPM + clutch open + near-zero load
15000 RPM + shift torque cut
15000 RPM + downshift recovery/blip
15000 RPM + rev limiter
```

GF509 should produce different acoustic behavior for each case without changing the physical RPM solely to fake a different sound.

That distinction is the core purpose of this backlog.
