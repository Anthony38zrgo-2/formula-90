# Formula90s — Current Project State

> Canonical current-state handoff for humans and AI agents.
>
> This file is intentionally different from `docs/architecture/project-direction.md` and
> `docs/engineering/common-errors-and-fixes.md`.
>
> - `docs/architecture/project-direction.md` defines where the project is going.
> - `docs/engineering/common-errors-and-fixes.md` records reusable failure patterns and fixes.
> - `PROJECT_STATE.md` defines where the project is **right now**.
>
> Any agent that is about to modify vehicle physics, telemetry, track generation,
> environment generation, or generated content should read this file first.

---

## 0. Status notation

Every important statement should use one of these labels when ambiguity is possible.

| Label | Meaning |
|---|---|
| `[FROZEN]` | Known-good reference that must not be changed unless the task explicitly reopens it. |
| `[VALIDATED]` | Tested successfully and accepted as the current working state. |
| `[ACTIVE]` | Current implementation or current focus. |
| `[PLANNED]` | Approved next work, but not yet implemented or validated. |
| `[EXPERIMENTAL]` | Temporary or exploratory work. |
| `[DEPRECATED]` | Historical implementation that must not be used as current authority. |
| `[UNKNOWN]` | State is intentionally not assumed; inspect runtime/repository before acting. |

Do not silently convert `[PLANNED]` values into `[VALIDATED]` values.

Do not silently reopen `[FROZEN]` work.

## 0.1 Current F1-94 handoff — 2026-08-20

Current canonical branch/commit:

```text
branch: f1-94
commit: dc33d8d feat(f1-94): integrate scalable brake thermal pipeline
```

The accepted F1-94 brake-thermal feature is `[VALIDATED]`. It includes the
per-axle `scaled_two_node_v1` model, material/mass/geometry-based thermal
resolution, speed- and area-dependent cooling, implicit surface-to-bulk disc
integration, resolved brake telemetry, ABI synchronization, HUD/smoke coverage,
and the canonical debug/release build path. Validation accepted:

- `cargo test --workspace`;
- `scripts/run_f1_94.ps1 -Smoke`;
- `scripts/run_f1_94.ps1 -SmokeAudio`;
- debug and release builds through `scripts/build_windows.ps1`.

The next work sequence is intentionally ordered as follows:

1. `[PLANNED]` Add tyre/tire degradation on top of the existing per-wheel tire
   force, slip, pressure, and thermal state without replacing those models.
2. `[PLANNED]` Run the mechanical validation matrix and close the mechanical
   phase: straight-line load/traction, braking, cornering, transient weight
   transfer, thermal soak, repeatability, and telemetry/setup provenance.
3. `[PLANNED]` Only after mechanical sign-off, move to the V10 powertrain
   phase: torque delivery, engine braking, gears, clutch, and differential
   interaction.

Powertrain tuning must not be used to hide an unresolved tire, suspension,
brake, or mechanical-balance issue.

---

# 1. Repository identity

- Project: **Formula90s**
- Repository: `Anthony38zrgo-2/formula-90`
- Active development branch: `f1-94`
- Current project goal: compact 1990s-inspired formula racing game with convincing,
  readable, mechanically expressive handling and a late-1990s console visual identity.
- Current development car: **F1-94 (3.5L V10 NA)** `[CANONICAL per handoff 2026-08-17; Rust GEVP 3-raycast physics]`
- Previous canonical aliases (HISTORICAL, superseded by F1-94 Rust): **Jordan 197**
  (`game/scenes/vehicles/jordan_197/`, GEVP-era tuning, frozen/legacy), **Jordan 191**
  (`game/scenes/vehicles/jordan_191/` retained for history, physics frozen per HAN-001/PHY-003..007),
  and **Jordan 1995** (`game/scenes/vehicles/jordan_1995/` historical)
- Current handling-development circuit: **La Chutana** `[VALIDATED ENOUGH]`
- Physics architecture: **3-layer Rust GEVP migration** — deterministic 6-DOF Rust core
  (`game/crates/vehicle-physics-engine/`, Physics FFI `ABI v12`) → C++ GDExtension (`native/`,
  `libformula90s.windows.template_*.x86_64.dll`) → GDScript gameplay/controllers
  (`game/addons/formula90s/scripts/`). See §1.2.

### Current development order

```text
Phase A — physical geometry/layout                COMPLETE
Phase B/C — mechanical integration and validation ACTIVE — close-out pending
Brake thermal pipeline                            COMPLETE / VALIDATED
Tyre/tire degradation                             NEXT IMPLEMENTATION
Mechanical validation and freeze                  NEXT GATE
Phase E — V10 powertrain / transmission           PLANNED AFTER MECHANICAL SIGN-OFF
Phase F — aerodynamics
Phase G — assists / true no-assists behavior
Phase RUST — GEVP Physics Rust Migration (3-Raycast, deterministic core, telemetry parity) [VALIDATED BASELINE — F1-94 canonical]
```

---

## 1.2 F1-94 Rust Physics — Current Architecture (ACTIVE canonical)

Status: `[ACTIVE — canonical runtime as of 2026-08-17 handoff]`

The Jordan/GEVP-era gameplay vehicle was superseded by a Rust-ported deterministic
physics core. All F1-94 handling now flows through this 3-layer stack.

### 1.2.1 Layer map

```text
Rust core            game/crates/vehicle-physics-engine/        (cargo, Physics FFI ABI v12)
  ├─ vehicle_config.rs   JSON schema (deny_unknown_fields) + f1_94_canonical()
  ├─ ffi.rs              F1_94_PHYSICS_ABI_VERSION = 12, get/apply_runtime_config
  ├─ simulation.rs       6-DOF solver, inertia from inertia_multipliers
  └─ build → vehicle_physics_engine.*.dll  (Rust core loaded by the C++ wrapper)

C++ GDExtension      native/                                   (scons → libformula90s.*.dll)
  ├─ f1_94_rust_vehicle.cpp/.hpp   F194RustVehicle, core_driver_ (F90Core) takes
  │                                 precedence over sim_bridge_ inside _integrate_forces;
  │                                 update_wheel_visuals, runtime-config sync
  ├─ core/f90_core.cpp/.hpp        F90Core orchestrator facade (registers in
  │                                 register_types.cpp); Core ABI v7; drive_integrate +
  │                                 apply_runtime_config + reset_core_at + pump_audio
  ├─ sim/f90_sim_bridge.cpp/.hpp   F90SimBridge (LEGACY secondary driver; only used
  │                                 when core_driver_ is null)
  └─ include/.../formula90_physics.h  F90RuntimeConfig (Physics ABI v12, inertia + suspension)

Rust orchestrator   game/crates/formula90-core/                (cdylib formula90_core.dll, Core ABI v7)
  └─ CoreFacade owns physics (game_sim + vehicle_physics_engine) + audio
     (vehicle_audio_engine) + ModuleRegistry (SimModule); single handshake
     f90_core_abi_version() == 7. Replaces the three separately-loaded modules.

GDScript gameplay   game/addons/formula90s/scripts/
  ├─ f1_94_rust_vehicle.gd            F194RustVehicleGD wrapper (telemetry/HUD/audio)
  ├─ f1_94_rust_input_controller.gd   linear throttle (throttle_exponent = 1.0)
  ├─ driving_aids.gd                  aids[0] = automatic_transmission from JSON
  └─ vehicle_tunable_contract.gd      typed/bounded property access
```

### 1.2.2 Canonical entry path

```text
scripts/run_f1_94.ps1
  → scenes/runtime/vehicle_test_session.tscn
    → scenes/runtime/world_hud_compositor.tscn
    → data/race_sessions/f1_94_la_chutana.tres
  → scenes/vehicles/f1_94/f1_94_rust.tscn
      VehicleRigidBody (F194RustVehicle) + F194RustInputController + VehicleAudio
```

### 1.2.3 Runtime physics authority

Per §3.1, the single source of truth for F1-94 tuning is the JSON:

```text
game/data/vehicles/f1_94/f1_94_physics.json   (schema_version 2)
```

The Rust `f1_94_canonical()` defaults are only a fallback when JSON parse fails
(`[F194RustVehicle] CRITICAL JSON PARSE ERROR … Falling back to default canonical`).
The C++/Rust reject unknown JSON fields (`deny_unknown_fields`), so typos surface
explicitly instead of silently reverting to defaults.

### 1.2.4 Handoff alignment — 5 discrepancy factors (VALIDATED as of 2026-08-17)

| # | Factor | Resolution | Status |
|---|---|---|---|
| 1 | Silent JSON fallback (deny_unknown_fields) | `deny_unknown_fields` on all JSON substructs in `vehicle_config.rs`; explicit C++ printerr | `[VALIDATED]` |
| 2 | Driving aids overwriting params | `driving_aids.gd` reads `automatic_transmission` dynamically → `aids[0]`; restores baseline | `[VALIDATED]` |
| 3 | RigidBody vs Rust inertia | FFI `inertia_multiplier_x/y/z` (JSON→Rust→C++); `configured_inertia` in `solve_forces_for_state` (replaces fixed 1.10) | `[VALIDATED]` |
| 4 | Visual suspension vs forces | FFI `suspension_front/rear_spring_length` + `resting_ratio`; `update_wheel_visuals` uses them; fallback steer uses `max_steering_angle_` | `[VALIDATED]` |
| 5 | Input modulation | `throttle_exponent = 1.0` (linear) in `f1_94_rust_input_controller.gd`; F1-94 scene uses this controller | `[VALIDATED]` |

### 1.2.4b Runtime-config forwarding (F90Core orchestrator)

`F194RustVehicle::apply_runtime_config()` now forwards the same `F90RuntimeConfig`
to the orchestrator via `core_driver_->apply_runtime_config(cfg)` (when `core_driver_`
is set) and to the legacy bridge otherwise. The Rust side shares one sanitized
applier (`apply_runtime_config_to_sim` in `ffi.rs`, reused by both the legacy
`f1_94_physics_apply_runtime_config` FFI and `formula90_core`), so every tunable
JSON parameter (diff preload, aids mask, aero, suspension, steering…) keeps
applying at runtime on BOTH integration paths. `[VALIDATED — build green; runtime
pending in real env]`

### 1.2.5 Accepted brake-thermal handoff (2026-08-20)

Status: `[VALIDATED]`

The canonical F1-94 JSON now resolves front and rear brake thermal behavior from
per-axle material, rotor mass/geometry, ventilation, installation airflow, and
cooling profile fields. Resolved capacity, surface-to-bulk conductance, natural
cooling, speed cooling, and surface-to-bulk heat flow are available in the
runtime snapshot and CSV telemetry. The legacy two-node path remains available
for compatibility with future vehicle profiles.

The next thermal-related work is tyre/tire degradation. It must consume the
existing force/slip/load/temperature state and remain a separate, testable wear
family; do not retune the accepted brake thermal model as a substitute for wear.

### 1.2.6 Known divergences (runtime-authority vs `f1_94_canonical()` / scene)

Reconciled to the JSON on 2026-08-17 where the Rust default diverged (`vehicle_mass`,
`automatic_transmission`). Remaining rows are either runtime-overridden (JSON wins) or
out of scope (the `f1_94_canonical()` fallback defaults and scene visual anchors).

| Field | JSON (runtime) | Rust `f1_94_canonical()` / scene | Note |
|---|---|---|---|
| `vehicle_mass` | 575.0 | 575.0 (Rust default, aligned 2026-08-17) AND `mass = 505.0` in `f1_94_rust.tscn` RigidBody | Godot body mass (505) is runtime-overridden by the JSON/Rust sim mass (575); low-priority cleanup |
| `front_track` / `rear_track` | 1.670 / 1.590 | 1.5925 / 1.5246 | JSON wins at runtime |
| `max_steering_angle` | 0.418879 (~24°) | 0.436332 (~25°) | JSON wins at runtime |
| `automatic_transmission` | false | false (Rust default, aligned 2026-08-17) | [RESOLVED 2026-08-17] — no longer contradicts AUTO=ON baseline |
| wheel visual position | scene `FrontLeftWheel.x = -0.79625` | JSON `wheel_hubs.FL.x = -0.835` | Scene raycast anchor diverges from JSON hub |

See §5.0 for the full F1-94 physical-state snapshot and §45 for next work.

---

# 2. Mandatory reading order for agents

Before modifying the project:

1. `PROJECT_STATE.md`
2. `docs/architecture/project-direction.md`
3. `docs/engineering/common-errors-and-fixes.md`
4. Relevant local `AGENTS.md`
5. Relevant subsystem source files
6. Recent commits touching the same subsystem

Do not work from chat memory alone if the repository is available.

---

# 3. Source-of-truth hierarchy

When values disagree, use the following authority order.

## 3.1 Runtime physics

```text
runtime node/resource values
    >
current vehicle scene/resource
    >
project-owned Formula90s controller/config
    >
documentation
    >
chat history
```

A value observed in runtime is more authoritative than a manually written note.

## 3.2 Telemetry setup

```text
<telemetry_session>_setup.json
    >
runtime scene/resource values
    >
PROJECT_STATE.md
```

The setup snapshot is an immutable record of the configuration that produced that
specific telemetry capture.

## 3.3 Track geometry

```text
deterministic reference/config
    >
track pipeline output
    >
generated Blender source
    >
canonical generated GLB
    >
Godot wrapper/import
```

The runtime GLB is not the design source.

## 3.4 Generated textures

```text
source/generated base
    >
Texture Forge recipe/config
    >
manifest
    >
generated PNG output
```

Generated PNGs are outputs, not hand-edited source authority.

## 3.5 Vendor code

GEVP vendor code is upstream-owned.

Formula90s-specific behavior should normally be implemented through:

- configuration,
- subclassing,
- adapters,
- scene composition,
- project-owned controller code,
- project-owned surface logic.

---

# 4. Frozen GEVP baseline

> **F1-94 note:** This §4 baseline is the legacy **GEVP** reference. The F1-94 (V10)
> canonical is the Rust core loaded from `game/data/vehicles/f1_94/f1_94_physics.json`
> via `F194RustVehicle` (see §1.2 / §5.0). Do not use the GEVP frozen baseline as
> authority for F1-94 tuning.

## 4.1 Known-good baseline

`[FROZEN]`

Known-good baseline commit:

```text
feab26c3df1e7eaabb7674b2e40cfd12fa299b8e
```

Known-good baseline vehicle scene:

```text
game/scenes/vehicles/baseline_2026/baseline_2026.tscn
```

Known-good baseline test scene:

```text
game/scenes/tracks/test_field/gevp_baseline.tscn
```

The frozen baseline exists to answer:

> Does the underlying GEVP vehicle still work independently from Formula90s tuning?

It is not the place to implement Jordan-specific behavior.

## 4.2 Vendor controller

`[FROZEN]`

Vendor file:

```text
game/addons/gevp/scripts/vehicle_controllergd.gd
```

Known upstream commit:

```text
172cfc8536f02f2568d7e8f530a83e73ffbd7796
```

Do not modify this vendor file to solve Formula90s-specific tuning problems.

## 4.3 Other known vendor-related facts

- Physics tick target: **120 Hz**
- `vehicle.gd` is not treated as a pristine upstream copy because Formula90s retains
  project-specific engine configuration behavior there.
- `wheel.gd` contains safe surface fallbacks and debug behavior used during earlier
  wheel/contact debugging.

Changing these areas requires explicit justification.

---

## 5.0 F1-94 (V10) — current physical state (ACTIVE canonical)

Status: `[VALIDATED GOOD — runtime authority = game/data/vehicles/f1_94/f1_94_physics.json (schema_version 2)]`

> **2026-08-18 state:** Core vehicle configuration and mechanics (chassis, suspension,
> powertrain, differential, tires, brakes, TCS, and the newly implemented ESP yaw-stability
> aid) are in a **good, drivable state**. Steering has been re-blended to a responsive-but-
> correctable feel. Remaining work is **detail polishing** (fine steering-rate feel, ESP
> strength/engage tuning per surface, and optional telemetry-validated sign-off against the
> §7.1 acceptance matrix). No architectural rework pending.

All values below are read by the Rust core at `VehicleRigidBody._ready()` and override
the Rust `f1_94_canonical()` defaults. Numbers are transcribed from the JSON (authority
per §3.1). Divergences flagged `[VERIFY]` in §1.2.6 are NOT re-litigated here.

### Chassis

```text
vehicle_mass              = 575.0        [VERIFY: scene RigidBody mass = 505.0 — runtime-overridden, low priority]
front_weight_distribution = 0.43
cg_vertical_offset_m      = -0.12
inertia_multipliers       = (x 1.05, y 1.26, z 1.05)   (applied live)
wheelbase_m              = 2.92065
front_track_m            = 1.670        [Rust default 1.5925 — JSON wins at runtime]
rear_track_m             = 1.590        [Rust default 1.5246 — JSON wins at runtime]
```

### Steering

```text
max_steering_angle   = 0.418879   (~24.0°)  [VERIFY runtime JSON = 0.436332; JSON wins]
front_steering_ratio = 1.0
rear_steering_ratio  = 0.0
steering_speed       = 5.50   [2026-08-18 blend of 3.80/7.0: responsive entry]
countersteer_speed   = 6.50   [>= steering_speed so corrections are crisp, not laggy]
steering_speed_decay = 0.18   [blend of 0.28/0.10]
steering_slip_assist = 0.55
countersteer_assist  = 0.85
steering_exponent    = 1.00   [near-linear; blend of 1.35/0.85]
ackermann            = 0.20
# Steering model: linear move_toward ramp in simulation.rs filter_inputs (no ease-out tail);
# countersteer rate >= steer rate so corrections unwind predictably. GOOD state, pending polish.
```

### Powertrain

```text
max_torque              = 455.0 N·m
max_rpm                 = 15000
idle_rpm                = 4500
motor_moment            = 0.12
torque_curve            = 9-point (0.0→0.10 … 0.30→0.35 … 0.45→0.72 … 0.60→1.00 … 0.72→0.97 … 0.82→0.93 … 0.90→0.89 … 0.953→0.86 … 1.0→0.75)
gear_ratios             = [2.45, 1.95, 1.72, 1.50, 1.32, 1.18]
final_drive             = 5.00
reverse_ratio           = 3.00
shift_time              = 0.10
automatic_transmission  = false
front_torque_split      = 0.0  (RWD)
gear_inertia            = 0.02
max_clutch_torque_ratio = 1.10
clutch_out_rpm_offset   = 900.0
idle_disengagement_hysteresis_rpm = 100.0
variable_drag_ratio     = 0.12
constant_brake_ratio    = 0.02
handbrake_torque_fraction = 0.0
automatic_shift         = present (upshift/downshift RPM bands, kickdown, etc.)
```

### Differential (Salisbury clutch-pack LSD)

```text
preload_nm                       = 30.0
power_ramp_angle_deg             = 58.0
coast_ramp_angle_deg             = 68.0
clutches                         = 6.0
clutch_friction_coefficient      = 0.10
slip_transition_threshold_rad_s  = 0.90
```

### Suspension

```text
front: spring_length 0.250  resting_ratio 0.280  damping_ratio 0.74
       bump_damp 0.92  rebound_damp 1.18  arb 0.10
       toe -0.0017453  camber -0.0383972  bump_stop 2.1
rear:  spring_length 0.200  resting_ratio 0.350  damping_ratio 0.76
       bump_damp 0.96  rebound_damp 1.22  arb 0.09
       toe 0.0020944  camber -0.0314159  bump_stop 3.0
tri_ray_spacing_ratio = 0.40
```

### Tires & contact (per-axle)

```text
front: radius 0.31695  width 0.30030  wheel_mass 12.0
       contact_patch 0.35  braking_grip_multiplier 1.03  airborne_spin_decay_torque 1.0
rear:  radius 0.32901  width 0.36832  wheel_mass 16.0
       contact_patch 0.35  braking_grip_multiplier 1.03  airborne_spin_decay_torque 1.0
NOTE: contact_patch / braking_grip_multiplier / airborne_spin_decay_torque are now per-axle
      (front/rear) in f1_94_physics.json; top-level globals remain as fallbacks for missing
      axles. airborne_spin_decay_torque was previously hard-coded (2.0) and ignored from JSON;
      it is now wired from config (F1-94 JSON value 1.0 => slower airborne wheel decay).
surfaces: Road / Curb / Dirt / Grass / Gravel
  Road:   friction 2.60  stiffness 7.20  roll_res 1.0   lat_assist 0.04  long_ratio 0.64
  Curb:   friction 2.10  stiffness 5.90  roll_res 1.5   lat_assist 0.015 long_ratio 0.56
  Dirt:   friction 1.20  stiffness 0.45  roll_res 2.4   lat_assist 0.0   long_ratio 0.42
  Grass:  friction 0.75  stiffness 0.35  roll_res 4.2   lat_assist 0.0   long_ratio 0.35
  Gravel: friction 1.00  stiffness 0.45  roll_res 3.0   lat_assist 0.0   long_ratio 0.40
```

### Brakes

```text
front_brake_bias = 0.59
max_brake_torque = 2800.0
enable_abs       = false
abs_pulse_time   = 0.03
abs_spin_diff_threshold = 12.0
```

### Aerodynamics

```text
drag_coefficient        = 0.78
frontal_area            = 1.25
downforce_coefficient   = 1.90
split: front 0.24 / diffuser 0.36 / rear 0.40
air_density             = 1.225
lag_tau_s               = 0.030
blend_min_speed_mps     = 5.556
blend_full_speed_mps    = 41.667
yaw_decay_exponent      = 0.80
flex_coefficient        = 0.0008
```

### Driving aids policy (JSON)

```text
traction_control: available, default ON, selectable  [tuned: slip_threshold 0.08, cut_gain 1.2]
abs:                available, default OFF, selectable
stability (ESP):    available, default ON in TCS+ESP presets, selectable
                     [2026-08-18 IMPLEMENTED in Rust core: simulation.rs::stability_yaw_torque,
                      over-rotation limiter only (damps yaw that exceeds commanded yaw + engage);
                      params engage 0.15 / strength 6.0 / grounded_mult 2.0]
steering_slip_assist_default_enabled = true
countersteer_default_enabled         = true
auto_clutch_default_enabled          = true (player not selectable)
launch_control:     available OFF, default OFF, selectable OFF
brake_assist:       available OFF, default OFF, selectable OFF
handbrake:          rear_only OFF / abs_interlock OFF / clutch_coupling OFF
input_smoothing:    ON (steering 6.00 / throttle 7.50 / brake 14.0)
# Presets: f1_94_physics.json = TCS+ESP (stability_default_enabled true);
#          f1_94_physics_esp.json = TCS+ESP variant (identical simcade values).
# GOOD state — detail polish: ESP strength/engage per surface, §7.1 telemetry sign-off.
# Aids routing (Rust path): driving_aids.gd now sets the aids_enabled_mask bits on
#   F194RustVehicle instead of mutating unsupported VehicleTunableContract properties:
#   ESTAB=bit2 (stability), FRENOS=bit7 (brake-assist), GRIP/TCS=bit1. GEVP Vehicle keeps
#   the property-mutation path. The full mask is applied every step by CoreFacade::step
#   (P0 fix), so ESP/TCS/stability are no longer frozen or drop on the first frame.
#   [VALIDATED — build green; runtime pending in real env]
```

### Wheel visual architecture (F1-94)

Per `f1_94_rust.tscn`: single `F1_94_chassis_geometry.glb`, reused `wheel_front.glb` /
`wheel_rear.glb` on both sides (opposite side via 180° `Orientation` node, not negative
scale). Physics uses 12 RayCasts (FL/FR/RL/RR × In/Mid/Out), `target_position = (0,-0.65,0)`.
Visual wheel nodes are parented to `VehicleRigidBody`; their transforms are driven by the
Rust suspension compression (`update_wheel_visuals`) — NOT by Godot's built-in vehicle.

---

# 5. Jordan 191 — current physical state (canonical per HAN-001)

Status: `[DEPRECATED — HISTORICAL, superseded by F1-94 Rust (see §5.0) on 2026-08-17]`

> **J197-001 canonical selection (replaces HAN-001):** `game/scenes/vehicles/jordan_197/jordan_197.tscn` is the sole authoritative vehicle (3-GLB split: `jordan_197_chassis.glb` / `wheel_front/rear.glb` from `Jordan197_LOD0_Historical.glb` via `3d-asset-generation`). `game/scenes/tracks/test_field/test_field.tscn` and `jordan_197_handling_test.tscn` route through `jordan_197.tscn` via `VehiclePathResolver` (no `f1_1996`/`jordan_191` remains in bootstrap). `jordan_191/` and `jordan_1995/` retained as historical (frozen per HAN-001/PHY-003..007). `jordan_197` inherits PHY-006/007 steering/suspension tuning; `PHY-003` envelope re-derived from 197 model.

# 5.1 Phase A — geometry and physical layout

Status: `[VALIDATED — J197-001 FROZEN]`

Canonical scene:

```text
game/scenes/vehicles/jordan_197/jordan_197.tscn
```

Historical aliases (do not use as authority):

```text
game/scenes/vehicles/jordan_191/jordan_191.tscn
game/scenes/vehicles/jordan_1995/jordan_1995.tscn
game/scenes/vehicles/jordan_191/jordan_191_chassis.glb (dims 1.437x4.539, radius 0.3473)
```

Values below are **runtime authority** from `jordan_197.tscn:58-100` and `Jordan197_LOD0_Historical` measurement `chassis dims x=1.567 y=1.185 z=4.621 / whole vehicle 2.105x1.255x4.621 / wheel_front x=0.307 y=0.648 z=0.648 / wheel_rear x=0.358 y=0.648 z=0.648`. Verified 2026-08-13 via `3d-asset-generation` `analyze_mesh.py` + `spatial_query`. No change to this block during `PHY-012/008` unless J197-001 is explicitly reopened.

## Chassis

```text
mass_kg               = 505
front_weight_ratio    = 0.45
cg_vertical_offset_m  = -0.12
inertia_multiplier    = 1.10
wheelbase_m           = 3.073
front_track_m         = 1.762
rear_track_m          = 1.748
chassis_width_m       = 1.567
whole_width_m         = 2.105
chassis_length_m      = 4.621
```

## Tire dimensions

```text
tire_radius_m         = 0.324
visual_diameter_m     = 0.648

front_tire_width_mm   = 308
rear_tire_width_mm    = 358

front_wheel_mass_kg   = 12
rear_wheel_mass_kg    = 16
```

Measured GLB vs physics match: `front width 0.307 vs 308mm / rear 0.358 vs 358mm / diameter 0.648 vs radius 0.324*2=0.648` — exact model-constrained via single-side `LP_TYRE_LF/LR` extraction. Front track `1.762` and rear `1.748` (+19% vs 191 `1.478/1.437`) reflect 197 model width 2.105; historical 1990s max track ~2.0m would still be wider but 197 is faithful to asset.

## Physics wheel positions

Front left:

```text
(-0.881, 0.32, -1.433)
```

Front right:

```text
(+0.881, 0.32, -1.433)
```

Rear left:

```text
(-0.874, 0.32, 1.64)
```

Rear right:

```text
(+0.874, 0.32, 1.64)
```

These RayCast positions are physical authority.

Visual wheel hierarchy must follow physics, not the reverse.

## Collision

Current tub/nose/rear collision design from the validated Phase A state should not
be retuned during steering work.

---

# 5.2 Wheel visual architecture

Status: `[VALIDATED]`

A Formula90s car should normally use only:

```text
chassis.glb
wheel_front.glb
wheel_rear.glb
```

Physics still has four distinct wheel RayCasts.

The same front visual wheel is reused on both sides.

The same rear visual wheel is reused on both sides.

For the opposite side:

```text
Orientation node
→ 180 degree rotation
```

Do not solve wheel orientation with negative scale.

Do not create four permanent duplicated wheel meshes only for left/right orientation.

---

# 5.3 Phase B — mechanical grip, braking, differential

Status: `[VALIDATED]`

Canonical scene:

```text
game/scenes/vehicles/jordan_191/jordan_191_phase_b.tscn
```

Historical alias (do not use as authority):

```text
game/scenes/vehicles/jordan_1995/jordan_1995_phase_b.tscn
```

Phase B inherits the Phase A physical layout.

## Brakes

```text
front_brake_bias = 0.57
```

## Differential

```text
rear_locking_diff_torque = 170
```

## Tire/contact behavior (per-axle)

```text
contact_patch / braking_grip_multiplier / airborne_spin_decay_torque are now per-axle:
  front_contact_patch / rear_contact_patch
  front_braking_grip  / rear_braking_grip
  front_airborne_decay / rear_airborne_decay
(VehicleConfig fields). Legacy globals contact_patch / braking_grip_multiplier kept for
back-compat and used as fallbacks when an axle value is absent in JSON.
```

## Surface stiffness

```text
Road     = 8.75
Curb     = 7.00
Gravel   = 0.50
Grass    = 0.50
```

## Surface friction

```text
Road     = 2.65
Curb     = 2.20
Gravel   = 1.10
Grass    = 0.90
```

## Rolling resistance / rolling modifier

```text
Road     = 1.0
Curb     = 1.5
Gravel   = 2.0
Grass    = 4.0
```

## Lateral assist

```text
Road     = 0.02
other surfaces = 0
```

## Longitudinal ratio

```text
Road     = 0.48
other surfaces = 0.45
```

### Phase B rule

`[FROZEN FOR PHASE C]`

Do not change these values during Phase C unless telemetry proves that a supposed
steering problem is actually a mechanical-grip defect and the phase boundary is
explicitly reopened.

---

# 6. Current test controls

Status: `[ACTIVE — F1-94 Rust path]`

Known F1-94 test controls (InputMap actions consumed by `f1_94_rust_input_controller.gd`):

```text
Throttle / Brakes / Steer Left / Steer Right   analog (keyboard/controller)
Shift Up / Shift Down                          sequential gear change
Toggle Transmission                            flip automatic_transmission (aids[0])
Clutch                                        clutch (merged with handbrake)
Handbrake                                     handbrake
Reset                                         reset to spawn
```

Notes:
- Throttle is **linear** (`throttle_exponent = 1.0`); the old quadratic `pow(..., 2.0)`
  deadzone lives only in the legacy `formula_vehicle_controller.gd` (NOT on the F1-94 path).
- Reverse is engaged from gear 0 when `|speed| < 3 km/h`; in reverse, throttle/brake swap.
- `DrivingAidsController` (`driving_aids.gd`) is NOT instantiated in `f1_94_rust.tscn`;
  automatic-transmission toggle is handled directly by the input controller.

Current intended initial aid state:

```text
AUTOMATIC_TRANSMISSION = false   (per f1_94_physics.json; [VERIFY — old baseline said AUTO = ON])

stability = OFF
steering  = OFF   (vehicle-level slip/countersteer assist still applied per JSON aids policy)
braking   = OFF
grip      = OFF
```

Aid toggles (DrivingAidsController, keys `aid_1`..`aid_5`): on the Rust `F194RustVehicle`
route they flip the `aids_enabled_mask` bits (ESTAB=2, FRENOS=7, GRIP/TCS=1) via
`driving_aids.gd` `_set_aids_mask_bit`, then `CoreFacade::step` applies the full mask
each tick. AUTO (aid_1) still mutates `automatic_transmission`; DIRECC (aid_3) still
mutates `steering_exponent` on both paths. Previously these toggles were silent no-ops
on the Rust vehicle — that gap is closed. `[VALIDATED — build green; runtime pending]`

See §5.0 for the full F1-94 aids policy and §1.2.4 for the handoff alignment.

### 6.1 Canonical assist policy (PHY-014)

Status: `[VALIDATED — PHY-014]`

`game/scenes/vehicles/jordan_191/jordan_191.tscn:57` now sets `enable_stability = false` — yaw stabilization is **OFF** at baseline (previously hidden `vehicle.gd:90` default `true` captured by `DrivingAidsController._capture_baseline()` `driving_aids.gd:24` meant `aid OFF` still restored `true`). `DrivingAidsController` `driving_aids.gd:7` `aids = [true,false,false,false,false]` is now truthful: aid toggle reproducibly flips `enable_stability` via `_apply_aid(1)/_restore(1)` `driving_aids.gd:56` with `MULT_STABILITY 2.0 / FLOOR 4.0`.

Mild intentional Monaco-GP2 steering assistance remains at the **vehicle** level (not as a hidden aid): `steering_slip_assist = 0.45` `countersteer_assist = 0.30` `steering_exponent = 1.5` `jordan_191.tscn:39` — allowed per `backlog_seed.json: PHY-014`.

| Domain | Baseline (OFF) authority | ON via DrivingAids (reproducible, no code change) | Explicit value |
|---|---|---|---|
| **Yaw** | `enable_stability = false` `jordan_191.tscn:57` | `aid 2` → `true` + `stability_yaw_strength *2.0 (floor 4.0)` `driving_aids.gd:58` | `stability_yaw_engage_angle 0.0`, `ground_multiplier 2.0`, `upright 1.0/1000` remain vendor defaults until explicit |
| **Wheelspin** | `traction_control_max_slip = 8.0` `vehicle.gd:69` (default, now documented — not togglable via DrivingAids; future `PHY-008` may disable to `-1`) / `rear_locking_diff 120` default via `vehicle.gd:171` (jordan_191 inherits) | `grip aid 5` scales `coefficient_of_friction *1.3 floor 1.5` + `lateral_grip_assist +0.15` `driving_aids.gd:64` | Reproducible via `aid 5` |
| **Braking** | `brake_force_multiplier = 1.0` `front_brake_bias = 0.58` `jordan_191.tscn:71` / `ABS pulse 0.03 / threshold 12.0` `vehicle.gd:74` | `braking aid 4` → `*1.5` `driving_aids.gd:62` | Toggle `aid 4` |
| **Steering** | `steering_exponent 1.5` baseline | `steering aid 3` → `*1.5` → `2.25` `driving_aids.gd:60` | Explicit |

No other `autopilot` torque is applied at baseline; acceptance matrix `§7.1` is evaluated with all four aids `OFF` except `AUTO ON`. Changing `enable_stability` alone satisfies `PHY-014` without retuning grip/brake/diff.

Validation: `smoke_test_jordan_191_handling_scene` checks `Jordan191/VehicleRigidBody` `enable_stability = false` at runtime; `DrivingAids` baseline capture verified via headless `get("enable_stability") == false`.

Agents must inspect runtime before assuming that input or aid bindings have not changed.

---

# 7. Handling objective

Status: `[ACTIVE DIRECTION]`

The target is not a hardcore modern simulator and not a generic arcade vehicle.

Desired behavior:

- progressive steering;
- no immediate full lateral adhesion from a keyboard tap;
- understandable weight transfer;
- recoverable oversteer when corrected in time;
- late/excessive correction can still produce a spin;
- lifting throttle can help recover grip;
- braking behavior should remain readable;
- low rear stability should create intuitive oversteer;
- strong rear stability / forward brake bias should be capable of producing understeer;
- off-track surfaces should degrade behavior without becoming invisible walls;
- aerodynamics will later modify high-speed behavior but must not hide poor mechanical handling.

Historical inspiration:

```text
Monaco Grand Prix: Racing Simulation 2 / late-1990s formula handling feel
```

This is an inspiration target, not a requirement to duplicate proprietary physics.

### 7.1 Canonical handling acceptance matrix (HAN-001)

Status: `[VALIDATED — definition]` `epic: canonical-f1-handling` `item: HAN-001`

All subsequent handling changes (`PHY-003 .. PHY-015`) are judged against this matrix, not against a single corner feeling.

| # | Criterion | Observable | Pass condition (behavior-first, human + telemetry if available) |
|---|---|---|---|
| 1 | **Progressive turn-in** | steering input (step/tap) vs yaw/lateral-G | Small tap produces proportional small yaw; full tap builds lateral demand, not instant max grip |
| 2 | **Recoverable oversteer** | throttle oversteer + early countersteer | Early countersteer + lift recovers without spin in >80% of intentional low/medium-speed drifts |
| 3 | **Entry understeer when overdriven** | high entry speed, full lock | Front washes wide if entry speed/lock excessive; not auto-rotated by rear |
| 4 | **Stable straight-line braking** | hard braking from ~200 km/h, hands off | Car stays straight without yaw snap; no persistent bottom-out or oscillation >2 cycles |
| 5 | **Short transient settling** | chicane / direction change / brake release | Body roll settles within 1-2 oscillations; GT3-like long settling is FAIL |
| 6 | **Manageable curbs** | curb strike at normal racing line, 80-220 km/h | No random launch, no chassis G-spike without suspension explanation; RayCast not blind (`godot.gevp.raycast_curb_blindness`) |
| 7 | **Greater high-speed stability than low-speed stability** | 60 km/h vs 180 km/h constant-radius awareness | High-speed feels distinctly more planted than low-speed; low-speed remains lively/mechanical, not velcro |

Canonical routes for the matrix:

```text
Primary:   game/scenes/tracks/test_field/test_field.tscn  -> Jordan191/VehicleRigidBody (via GameBootstrap -> WorldHudCompositor)
Reference: game/scenes/tracks/test_field/jordan_191_handling_test.tscn -> Jordan191/VehicleRigidBody (direct)
Frozen diagnostic: game/scenes/tracks/test_field/gevp_baseline.tscn -> baseline_2026
```

No `f1_1996_car.tscn` remains in either canonical path. `WorldHudCompositor` resolves `Jordan191/VehicleRigidBody` then legacy `VehicleController/VehicleRigidBody` (`game/scenes/runtime/world_hud_compositor.gd:8`).

---

# 8. Phase C — steering and countersteer

Status: `[ACTIVE — PHY-006 applied]`

Only the steering/countersteer family should be modified.

## 8.1 Candidate starting values

These are **starting candidates**, not validated values.

```text
steering_speed              ≈ 3.7
countersteer_speed          ≈ 9.0
steering_decay              ≈ 0.26
slip_assist                 ≈ 0.11
countersteer_assist         ≈ 0.70
steering_exponent           ≈ 1.70
max_steering_angle_deg      ≈ 25
```

Do not record these later as final values unless telemetry and human testing accept them.

### 8.1.1 Active steering family (PHY-006)

Status: `[ACTIVE — applied 2026-08-13, pending §7.1 human/telemetry validation]`

`game/scenes/vehicles/jordan_191/jordan_191.tscn:36` now sets exactly the candidate family:

```text
max_steering_angle  = 0.436332 (25deg)
front_steering_ratio = 1.0
steering_speed       = 3.7
countersteer_speed   = 9.0
steering_speed_decay = 0.26
steering_slip_assist = 0.11
countersteer_assist  = 0.70
steering_exponent    = 1.70
```

Tightly coupled single behavior family per `PHY-006`; no tire/suspension/brake/diff/aero retuned. Yaw stabilization remains `OFF` per `PHY-014` (`enable_stability false`). Validated structural `headless editor DONE` + `smoke_test_jordan_191_handling_scene PASS` + `smoke_test_bootstrap_world_hud_compositor PASS`. Full `§7.1` matrix sign-off deferred to `PHY-015` freeze — human must confirm progressive turn-in + early countersteer recovery before `Phase C [VALIDATED]`.

## 8.2 Allowed changes during Phase C

- steering input shaping;
- steering build-up rate;
- steering unwind/decay;
- countersteer response;
- steering exponent/nonlinearity;
- steering assistance directly related to countersteer;
- maximum steering angle if required by the steering model;
- instrumentation needed to measure steering behavior.

## 8.3 Forbidden simultaneous tuning

Do not retune these families during Phase C:

```text
tire friction
contact patch
brake bias
differential baseline
springs
dampers
anti-roll behavior
engine torque
engine braking
gear ratios
aerodynamics
track collision
surface classification
```

If one of these must change, stop and explicitly reclassify the task.

---

# 9. Phase D — suspension

Status: `[ACTIVE — PHY-007 applied]`

Target family:

- springs;
- damping;
- anti-roll behavior if used;
- transient weight transfer;
- braking pitch;
- direction-change behavior;
- curb/bump response.

Do not increase tire grip to conceal poor suspension behavior.

### 9.1 Active suspension family (PHY-007)

Status: `[ACTIVE — applied 2026-08-13, pending §7.1 curb/transient validation]`

`jordan_191.tscn:64` now sets single suspension family (no tire/brake/diff/aero change):

```text
front_spring_length 0.10  front_resting 0.50  front_damping 0.80  bump 1.3  rebound 1.1  arb 0.25  bump_stop 2.2  toe 0.002
rear_spring_length  0.12  rear_resting  0.50  rear_damping  0.85  bump 1.3  rebound 1.1  arb 0.10  bump_stop 2.2  toe 0.001
```

vs baseline `front_damping 0.75/arb 0.35` `rear_damping 0.70/arb 0.30` (GT3-like long settling). Short travel retained (0.10/0.12), damping +0.05/+0.15 for 1-2 oscillation settle, arb softened front 0.35→0.25 rear 0.30→0.10 to reduce roll without masking grip, bump_stop 1.0→2.2 to prevent persistent bottom-out/launch per `§24` curb contract 0.022m rise. Validated `headless editor DONE` + `smoke_test_jordan_191 PASS` + `bootstrap PASS`; curb G-spike vs suspension compression still requires human telemetry per `§7.1` #5/#6 deferred to `PHY-015`.

---

# 10. Phase E — V10 powertrain and transmission

Status: `[PLANNED — gated by mechanical sign-off]`

Target family:

- intended 1990s V10 torque delivery;
- RPM range;
- engine braking;
- throttle response;
- gear ratios;
- differential interaction.

Powertrain tuning must not begin until tyre degradation is implemented and the
mechanical validation matrix is accepted. Mechanical handling, braking, thermal
behavior, and tire wear must be understood before powertrain values are changed.

---

# 11. Phase F — aerodynamics

Status: `[PLANNED]`

Target family:

- front downforce;
- rear downforce;
- aero balance;
- drag;
- speed-dependent behavior.

Mechanical handling must already work before aero is used to shape balance.

---

# 12. Phase G — assists and true no-assists behavior

Status: `[PLANNED]`

The project must clearly separate:

```text
natural vehicle physics
GEVP baseline stabilization
Formula90s optional aids
```

Eventually `OFF` must have a precise behavioral meaning.

---

# 13. Telemetry contract

Status: `[ACTIVE — MUST BE EXTENDED BEFORE PHASE C]`

Telemetry is the primary evidence source for handling decisions.

Human comments such as:

```text
"the car feels too nervous"
"countersteer feels late"
"rear grip disappears too quickly"
```

are valid observations, but physics changes should be correlated with telemetry where possible.

---

# 14. Telemetry file pairing

Status: `[PLANNED — REQUIRED BEFORE PHASE C]`

Every telemetry capture must produce exactly one immutable setup snapshot.

Example:

```text
telemetry_20260809_112500_001.csv
telemetry_20260809_112500_001_setup.json
```

Naming rule:

```text
<telemetry-basename>.csv
<telemetry-basename>_setup.json
```

The two files are inseparable diagnostic artifacts.

---

# 15. Setup snapshot rule

The setup JSON must be generated automatically from **actual runtime values** when
the telemetry session starts.

Do not:

- manually type the JSON after testing;
- reconstruct the setup from memory;
- infer values from documentation;
- copy values from another run.

The setup file must represent what the running vehicle actually used.

---

# 16. Proposed setup JSON schema

Status: `[PLANNED]`

Recommended structure:

```json
{
  "schema_version": 1,

  "session": {
    "telemetry_file": "telemetry_20260809_112500_001.csv",
    "timestamp": "2026-08-09T11:25:00-05:00",
    "development_phase": "phase_c",
    "physics_hz": 120
  },

  "provenance": {
    "git_commit": "<runtime commit sha>",
    "git_branch": "refactor/gevp-clean-baseline",
    "vehicle_scene": "res://...",
    "track_scene": "res://..."
  },

  "vehicle": {
    "id": "jordan_1995",
    "configuration": "phase_c"
  },

  "chassis": {},
  "tires": {},
  "steering": {},
  "brakes": {},
  "differential": {},
  "suspension": {},
  "engine": {},
  "transmission": {},
  "aerodynamics": {},
  "assists": {},
  "surfaces": {}
}
```

Empty sections are acceptable if the subsystem is not yet explicitly configured.

Invented values are not acceptable.

---

# 17. Minimum setup JSON contents

The logger should capture, when available:

## Session

- telemetry filename;
- timestamp;
- physics tick rate;
- development phase;
- test identifier if one exists.

## Provenance

- Git commit SHA;
- Git branch;
- vehicle scene/resource path;
- track scene/resource path;
- relevant configuration/resource paths.

## Chassis

- mass;
- center of gravity;
- weight distribution;
- inertia-related parameters;
- wheelbase/track values if configurable.

## Tires

- radius;
- width;
- contact patch;
- grip multipliers;
- tire-related assistance values.

## Steering

- steering speed;
- countersteer speed;
- steering decay;
- steering exponent;
- maximum angle;
- slip assist;
- countersteer assist;
- any speed-sensitive steering parameter.

## Brakes

- front/rear bias;
- braking multipliers;
- lock-related parameters.

## Differential

- locking torque;
- current differential mode/configuration.

## Suspension

- spring values;
- damping;
- travel;
- anti-roll settings if present.

## Engine

- torque configuration;
- RPM limits;
- engine braking;
- throttle response parameters.

## Transmission

- gear ratios;
- final drive;
- automatic/manual state;
- shift parameters.

## Aerodynamics

- front aero;
- rear aero;
- drag;
- balance-related values.

## Assists

- automatic transmission;
- stability;
- steering aid;
- braking aid;
- grip aid;
- any hidden/default stabilization relevant to interpretation.

## Surfaces

For each surface:

- stiffness;
- friction;
- rolling resistance;
- lateral assist;
- longitudinal ratio;
- any other runtime coefficient affecting tire behavior.

---

# 18. Mid-session setup changes

Status: `[PLANNED RULE]`

A telemetry session should represent one physical setup.

If a relevant physical parameter changes during a test:

```text
close current CSV
close/finalize current setup snapshot
start a new telemetry session
write a new setup snapshot
```

Example:

```text
telemetry_..._001.csv
telemetry_..._001_setup.json

# steering parameter changed

telemetry_..._002.csv
telemetry_..._002_setup.json
```

This prevents a CSV from containing physically incompatible segments.

Aid toggles should also trigger a new session when the aid affects handling analysis.

---

# 19. Telemetry comparison philosophy

The purpose of setup snapshots is to enable comparisons such as:

```text
Run A
steering_speed = 3.7
countersteer_speed = 9.0
exponent = 1.70

vs

Run B
steering_speed = 3.4
countersteer_speed = 8.0
exponent = 1.55
```

and relate those differences to measured behavior:

```text
lateral G
front slip
rear slip
steering input
steering output
yaw response
vehicle speed
suspension compression
spin/recovery behavior
```

Never compare two telemetry CSV files without first checking the paired setup JSON.

---

# 20. La Chutana — current circuit role

Status: `[VALIDATED ENOUGH FOR PHASE C]`

La Chutana is the current handling-development circuit.

Public/reconstruction anchors used by the project include approximately:

```text
length             ≈ 2.420 km
main straight      ≈ 800 m
turn count         ≈ 7
location           = Lima / San Bartolo area, Peru
```

The implementation is a gameplay-oriented reconstruction, not survey-grade CAD.

---

# 21. La Chutana generated runtime architecture

Status: `[ACTIVE]`

Godot wrapper:

```text
game/scenes/tracks/test_field/la_chutana_generated.tscn
```

Canonical generated runtime asset:

```text
res://assets/generated/tracks/la_chutana/la_chutana.glb
```

The canonical GLB is generated locally and should be treated as a build output.

The old procedural Godot prototype remains historical:

```text
game/scenes/tracks/test_field/la_chutana_track.gd
game/scenes/tracks/test_field/la_chutana_track.tscn
```

Status of old prototype: `[DEPRECATED]`

Do not make it current authority again without an explicit architectural decision.

---

# 22. Track surface groups

Status: `[VALIDATED]`

Expected surface categories:

```text
Road
Curb
Grass
Wall
```

Project-owned group restoration/tagging:

```text
game/addons/formula90s/scripts/generated_track_surface_groups.gd
```

If driving behavior changes unexpectedly after track regeneration, inspect group
classification before changing vehicle grip.

---

# 23. La Chutana terrain/collision contract

Status: `[VALIDATED]`

The current generator uses:

- continuous terrain heightfield;
- upward-facing winding validation after coordinate conversion;
- collision underlay beneath road;
- local collision bridges where required;
- large invisible safety floor as catastrophic-fall failsafe;
- separate visual/collision treatment;
- visual terrain sink where needed to avoid road clipping.

Important coordinate convention:

```text
Godot/data:
(x, z, height)

Blender:
(x, -z, height)
```

The sign inversion affects triangle winding.

Do not reintroduce broad independently offset terrain ribbons around the track.

---

# 24. Current curb contract

Status: `[VALIDATED]`

Current La Chutana curb direction:

```text
width_m        ≈ 0.58
maximum rise   ≈ 0.022 m
```

Representative profile:

```text
(0.00, 0.000)
(0.12, 0.005)
(0.29, 0.022)
(0.46, 0.012)
(0.58, 0.002)
```

Curbs must remain low and gradual enough that a low Formula chassis can touch them
without treating them as ramps.

Do not soften vehicle suspension to hide malformed curb geometry.

---

# 25. Deterministic Blender track pipeline

Status: `[ACTIVE]`

Main directory:

```text
blender/track_pipeline/
```

Important files:

```text
configs/la_chutana.json
data/la_chutana_reference.json

pipeline_common.py
prepare_track.py
validate_track.py

terrain_grid.py

generate_procedural_textures.py
texture_forge.py
validate_texture_forge.py

procedural_catalog.py
procedural_materials_blender.py
procedural_assets_blender.py

generate_environment.py
validate_environment.py

build_track_blender.py
build_environment_blender.py
blender_output.py

requirements.txt
README.md
```

Runner:

```text
scripts/run_track_pipeline.ps1
```

Setup script:

```text
scripts/setup_track_pipeline.ps1
```

Agent skill:

```text
.agents/skills/track-reconstruction/SKILL.md
```

---

# 26. Base / Procedural generation gate

Status: `[ACTIVE]`

Base stage:

```powershell
.\scripts\run_track_pipeline.ps1 `
  -Mode Base `
  -Track la_chutana `
  -Seed 1995
```

Procedural stage:

```powershell
.\scripts\run_track_pipeline.ps1 `
  -Mode Procedural `
  -Track la_chutana `
  -TreesDensity medium `
  -BushesDensity medium `
  -GrassDensity medium `
  -BuildingsDensity medium `
  -Seed 1995
```

Allowed density labels:

```text
none
very_low
low
medium
high
```

A procedural run must begin from clean Base output.

It must not decorate the previous environment output.

---

# 27. Generated Blender/output contract

Base source:

```text
blender/generated/la_chutana/track_base.blend
```

Base GLB output:

```text
game/assets/generated/tracks/la_chutana/la_chutana_base.glb
```

Environment source:

```text
blender/generated/la_chutana/track_environment.blend
```

Environment GLB:

```text
game/assets/generated/tracks/la_chutana/la_chutana_environment.glb
```

Canonical published runtime:

```text
game/assets/generated/tracks/la_chutana/la_chutana.glb
```

Publishing must be atomic:

```text
export temporary .new.glb
→ verify export
→ replace canonical GLB
```

Never delete the last known-good canonical GLB before a replacement has succeeded.

---

# 28. Current La Chutana configuration highlights

Status: `[ACTIVE]`

Important current configuration concepts:

## Road

```text
width_m                = 12.0
thickness_m            = 0.16
edge_line_width_m      = 0.12
surface_elevation_m    = 0.025
```

## Terrain

```text
grid_cell_m                    = 6.0
texture_world_size_m           = 96.0
shoulder_falloff_m             = 18.0

collision_underlay_drop_m      = 0.12
collision_underlay_blend_m     = 1.0

visual_under_road_drop_m       = 0.22
visual_under_road_blend_m      = 1.4
visual_edge_blend_m            = 1.25
visual_sink_m                  = 0.003

roadside_visual_width_m        = 1.4
far_ground_z_m                 = -0.1
far_ground_safety_m            = 0.05
far_ground_margin_m            = 220.0

safety_floor_z_m               = -6.0
safety_floor_thickness_m       = 0.6
safety_floor_margin_m          = 80.0

roadside_collision_width_m     = 2.0
```

## Start/finish

```text
line_width_m       = 2.2
spawn_before_m     = 18.0
```

Agents must inspect `configs/la_chutana.json` before relying on this snapshot if the
configuration has changed since this file was updated.

---

# 29. Procedural environment taxonomy

Status: `[ACTIVE]`

Biome model:

```text
continent
+
longitudinal band
    west | center | east
+
altitude band
    low | medium | high
```

Current implemented continent family:

```text
South America
```

Current La Chutana biome:

```text
south_america / west / low
```

Artistic meaning:

- Peruvian coastal/semi-arid direction;
- dry-biased but not monochrome;
- green, ochre, brown and exposed soil can coexist;
- location variation is artistic rather than strict ecological simulation.

---

# 30. Procedural environment placement rules

Status: `[ACTIVE]`

Representative La Chutana placement zones:

```text
grass:
    min edge clearance ≈ 1.4 m
    max track distance ≈ 30 m

bushes:
    min edge clearance ≈ 3.0 m
    max track distance ≈ 44 m

trees:
    min edge clearance ≈ 8.0 m
    max track distance ≈ 100 m

fake buildings:
    min edge clearance ≈ 65 m
    max track distance ≈ 240 m
```

Placement must account for the asset footprint, not only centerline distance.

Vegetation distribution should use deterministic clustering rather than uniform scatter.

---

# 31. Vegetation geometry contract

Status: `[ACTIVE]`

Trees:

```text
3 crossed textured planes
6 visible directional faces
no gameplay collision
```

Bushes:

```text
2 crossed textured planes
4 visible directional faces
no gameplay collision
```

Grass:

```text
1 double-sided textured card
no gameplay collision
```

Structures:

```text
simple low-poly 3D shells
basic roof/top
medium/far scenery
```

Visual richness should come primarily from texture, silhouette, clustering and composition.

---

# 31.1 Vegetation v2 — La Chutana tree set + 2x visual scale

Status: `[VALIDATED]`

The La Chutana vegetation v2 tree set integrates the four uploaded
`new_tree*.png` sources (see `blender/vegetation_v2_upload_bundle/source_manifest.json`):

```text
tree_v2_a -> tree_v2_01   new_tree.png  1152x2048  tall warm broad-canopy
tree_v2_b -> tree_v2_02   new_tree2.png 1600x1600  wide flowering/willow-like
tree_v2_c -> tree_v2_03   new_tree3.png 1184x2096  tall cool drooping
tree_v2_d -> tree_v2_04   new_tree4.png 1184x2096  tall warm broad-canopy
```

All four trees: `planes: 3`, `collision: false`, transparent RGBA, bottom
anchored, 256x256 prepared cards.

`tree_visual_scale: 2.0` lives in
`blender/track_pipeline/layouts/la_chutana/layout_config.json` and is applied
in `semantic_layout_common.py` **only** when `category == "trees"`. It scales
the final semantic target height and footprint radius (and therefore barrier
clearance and tree-to-tree occupancy) in the compiler contract — the GLB
geometry is unchanged, so doubling geometry alone would be cancelled by
`scale = target_height_m / asset_height_m`.

Validated result:

```text
trees:      130  target height 12.22-22.9 m (was 6.0-11.5)  footprint 3.74-12.16 m
bushes:     110  unchanged
grass:      932  unchanged
tree-to-tree overlaps: 0
negative barrier margin: 0
deterministic compile:  yes (hash-compared twice)
```

Runtime published to `game/assets/generated/tracks/la_chutana/la_chutana.glb`
(source/runtime SHA-256 match verified).

Offline 3xBRZ upscale: **blocked at the license gate** (xBRZ reference by Zenju
is GPLv3; repository is MIT). No compatible MIT implementation or approved
clean-room route exists in this increment; see `THIRD_PARTY.md`. No 3xBRZ code
was written. A compatible implementation or an authorized non-derivative
derivation is a prerequisite for that feature.

---

# 32. Current art direction

Status: `[ACTIVE DIRECTION]`

Target visual family:

```text
late-1990s PS1 rally/racing
photo-derived / prerendered texture character
strong silhouettes
low geometry density
rich but controlled texture detail
fake/baked-looking shadow information
macro terrain variation
retro readability
```

Current stronger direction:

- more autumnal;
- richer texture;
- tree canopies with greater perceived mass;
- balanced palette leaning dry;
- photo-derived rather than illustrative;
- clear regional variation while keeping one coherent visual family.

La Chutana must not become a lush wet forest.

For La Chutana, interpret the autumnal/rich style through:

```text
dusty olive
straw
ochre
muted green
burnt umber
dry soil
weathered surfaces
```

---

# 33. Texture Forge

Status: `[ACTIVE]`

Main implementation:

```text
blender/track_pipeline/texture_forge.py
```

Validation:

```text
blender/track_pipeline/validate_texture_forge.py
```

Texture generation:

```text
blender/track_pipeline/generate_procedural_textures.py
```

Current style identifier:

```text
ps1_rally_clean
```

Current known forge version:

```text
1
```

Current known deterministic seed:

```text
1995
```

Texture Forge is the final deterministic art compiler.

Intended flow:

```text
generated / AI / source base
        ↓
Texture Forge
        ↓
alpha cleanup
silhouette cleanup
palette control
fake lighting
fake AO
lower/contact shadow
posterization
subtle ordered dithering
manifest + hashes
        ↓
runtime texture
```

Do not hand-edit generated runtime PNGs as a permanent source-of-truth workflow.

---

# 34. Current generated texture bank

Status: `[ACTIVE]`

Generated texture root:

```text
blender/generated/la_chutana/textures/
```

Active biome:

```text
blender/generated/la_chutana/textures/biomes/south_america/west/low/
```

Active manifest:

```text
blender/generated/la_chutana/textures/active_manifest.json
```

Current active biome contains:

```text
terrain
shoulder
bark

4 tree variants
4 bush variants
4 grass variants
4 building/facade variants
```

Shared textures include:

```text
asphalt
guardrail
start_finish
```

The broader bank contains the nine South America combinations:

```text
west   / low
west   / medium
west   / high

center / low
center / medium
center / high

east   / low
east   / medium
east   / high
```

---

# 35. Current texture refinement strategy

Status: `[ACTIVE / RECENT]`

The approved refinement strategy is:

```text
First refine only:
south_america / west / low

Include:
trees
bushes
grass
buildings
terrain
shoulder
bark
```

Three-pass supervision strategy:

```text
Round 1
conservative refinement
preserve strengths
increase photo-derived richness

Round 2
stronger artistic push
more PS1 rally character
more autumnal/prerendered presence

Round 3
directed correction
analyze weaknesses in R1/R2
correct without over-stylizing
```

Selection must happen **per category**, not per complete batch.

Example:

```text
trees      ← best round for trees
bushes     ← best round for bushes
grass      ← best round for grass
buildings  ← best round for buildings
terrain    ← best round for terrain
```

Once the active biome style is accepted, it can be propagated to the other biome combinations.

---

# 36. Visual scale notes

Status: `[ACTIVE]`

Trees:

```text
current scene scale accepted
do not globally resize
```

Perceived improvement should come from:

- fuller silhouette;
- internal texture contrast;
- fake light/shadow;
- richer canopy texture.

Bushes:

Historical art direction requested a narrower horizontal presentation.

If geometry-scale changes are revisited, inspect current repository values before acting;
do not assume an older percentage is still current without checking.

---

# 37. Fog / atmosphere

Status: `[UNKNOWN — INSPECT CURRENT RUNTIME]`

A previous visual goal was to reduce excessive scene haze/fog.

Do not assume the current fog implementation/value without inspecting:

- `WorldEnvironment`,
- `Environment` resources,
- track/test scene environment configuration,
- generated scene overrides.

Do not change handling parameters to compensate for visibility problems.

---

# 38. OpenTopography / future terrain direction

Status: `[PLANNED / ARCHITECTURAL OPTION]`

OpenTopography or equivalent DEM data may later be used for two separate problems.

## 38.1 Real circuit elevation

For circuits where elevation is part of circuit identity:

```text
DEM
→ sample centerline elevation
→ filter/smooth macro profile
→ drive road elevation spline
→ generate authoritative road/curb/collision from that spline
```

The DEM must not directly become the fine road collider.

Road-scale details remain project-generated.

## 38.2 Source-style 3D skybox

For distant scenery:

```text
DEM
→ aggressively simplified distant terrain
→ prerendered / Texture Forge art
→ cheap 3D skybox-style geometry
```

No gameplay collision.

This is a future option, not current La Chutana handling work.

---

# 39. Track debugging invariants

If the car behaves incorrectly on a generated track, diagnose in this order:

```text
1. reproduce
2. inspect telemetry
3. inspect wheel contact
4. inspect terrain/collision continuity
5. inspect surface groups
6. inspect vehicle setup
7. only then tune physics
```

Do not immediately change:

```text
mass
grip
suspension
steering
```

to hide a track-generation bug.

---

# 40. Known track failures that must not return

Status: `[FORBIDDEN REGRESSIONS]`

- infinite fall after leaving asphalt;
- wrong triangle winding after coordinate conversion;
- broad grass collision ribbons folding into invisible wedges;
- visual terrain clipping through road;
- grass cards intruding into asphalt due to footprint ignorance;
- visual guardrail mesh being used as detailed collision;
- procedural runs accumulating duplicate scenery;
- failed export deleting the last known-good GLB;
- wrong surface classification after Godot import;
- spawn sign errors due to coordinate conversion mismatch;
- curb shapes that launch the car.

If one of these symptoms appears, consult:

```text
docs/engineering/common-errors-and-fixes.md
```

before inventing a new fix.

---

# 41. Context garbage collection

Status: `[MANDATORY PROCESS]`

`PROJECT_STATE.md` must remain a current-state snapshot, not a chat transcript.

When information becomes reusable general knowledge:

```text
move it to:
docs/engineering/common-errors-and-fixes.md
```

When information becomes durable product/design direction:

```text
move it to:
docs/architecture/project-direction.md
```

When information becomes obsolete:

```text
remove it from PROJECT_STATE.md
or mark it [DEPRECATED] only if the historical warning is still useful
```

Do not accumulate abandoned hypotheses.

---

# 42. Context update triggers

The agent responsible for a change must update this file when any of the following occurs:

- development phase completed;
- validated physics setup changed;
- canonical vehicle scene changed;
- telemetry schema changed;
- track collision contract changed;
- active circuit changed;
- generated texture authority changed;
- active biome changed;
- pipeline architecture changed;
- important bug resolution changes a project invariant;
- frozen baseline intentionally reopened;
- a `[PLANNED]` value becomes `[VALIDATED]`.

Prefer updating `PROJECT_STATE.md` in the same work session/commit family.

---

# 43. How to update this file safely

Before editing:

1. inspect current branch;
2. inspect current HEAD;
3. inspect changed subsystem files;
4. compare against this snapshot;
5. update only facts that actually changed.

For each changed item:

```text
old status
→ new status
→ canonical path/value
→ validation evidence
```

Do not rewrite large unrelated sections merely for style.

---

# 44. Agent handoff rule

At the beginning of a new agent/chat session, the recommended bootstrap instruction is:

```text
Read PROJECT_STATE.md first.

Then read:
- docs/architecture/project-direction.md
- docs/engineering/common-errors-and-fixes.md
- relevant AGENTS.md files

Treat PROJECT_STATE.md as the current handoff snapshot.
Verify any value marked [UNKNOWN].
Do not reopen [FROZEN] or [VALIDATED] work without evidence.
```

This minimizes dependence on previous chat history.

---

# 45. Immediate next work

Status: `[ACTIVE PRIORITY]`

## 45.0 Current F1-94 priority (2026-08-20)

The previous Rust handoff items below are retained for historical traceability;
the accepted baseline is now commit `dc33d8d` on branch `f1-94`. The active
sequence is:

1. **Tyre/tire degradation `[PLANNED]`** — add a deterministic per-wheel wear
   state driven by measured slip/load/temperature history and expose its effect
   through telemetry. Preserve the existing thermal, pressure, and force nodes.
2. **Mechanical closure tests `[PLANNED]`** — repeatable straight-line,
   braking, cornering, transient load-transfer, thermal, and regression tests;
   compare CSV data with the paired `Setup_JSON` and record accepted limits.
3. **Powertrain transition `[PLANNED]`** — begin only after the mechanical
   matrix is accepted and the tire/suspension/brake behavior is frozen for the
   test baseline.

Do not combine tyre-degradation tuning with powertrain tuning in the same test
family. One major physics family remains the unit of change.

## 45.0 F1-94 Rust handoff follow-up (2026-08-17)

The physics-alignment handoff (`instrucciones.txt`) is applied and validated end-to-end:
Rust unit tests pass, both GDExtension DLLs build, and the F1-94 scene loads on La Chutana
(headless smoke PASS, inertia multipliers confirmed live). Open items before deeper tuning:

1. **Reconcile `[VERIFY]` divergences (§1.2.6 / §5.0)** — especially `vehicle_mass`
   505 (scene RigidBody) vs 575 (JSON) and `automatic_transmission` false vs AUTO=ON intent.
   Decide the single authoritative value per family.
2. Extend telemetry setup-snapshot (`<session>.csv` + `_setup.json`) to capture the F1-94
   Rust runtime config (read from `f1_94_physics.json` at load), per §13-§18.
3. Re-run the full validation suite (`scripts/test_windows.ps1`) on a clean commit and
   record the commit SHA as the new known-good baseline for F1-94.
4. **Launch traction diagnosis + fix (2026-08-17)** — Latest telemetry (`rust_launch_head_120hz.csv`,
   `rust_physics_telemetry_sample.csv`, on-track `godot_f1_94_chutana_telemetry`) showed rear-slip
   saturation (0.5–2.0 rad) with `Long_G` oscillating +0.9/−0.9 → wheelspin limit-cycle, **not** an
   engine/entrega-de-potencia fault (RPM and speed climb normally). Root cause: (a) TC off by default
   (`traction_control_default_enabled=false`) and (b) launch clutch torque too high
   (`max_clutch_torque_ratio=1.70` → ~773 N·m, overwhelming rear grip). Fix applied (plan "C: Ambos"):
   - `f1_94_physics.json`: `traction_control_default_enabled=false→true`, `max_clutch_torque_ratio=1.70→1.15`,
     `clutch_out_rpm_offset=1200→900`.
   - `powertrain.rs`: replaced bang-bang TC cut (saturated to 100% → launch stall) with a **proportional
     slip-limiter** (`scale=threshold/slip`, floored at `MIN_TC_TORQUE_SCALE=0.10`) so TC limits slip
     instead of killing torque. Removed the low-speed velocity gate (proportional limiter handles
     standstill gracefully).
   - `f1_94_rust_input_controller.gd`: added `action_toggle_traction_control` (key "Toggle Traction
     Control") toggling bit 1 of `aids_enabled_mask` so TC remains player-selectable / OFF driveable.
   - `telemetry.rs`: added `TC_Active` + `DriveTorque` columns (Rust headless launch); `telemetry_manager.gd`
     appends `TC_Active` (reads `aids_enabled_mask & 2`) for in-engine captures.
   - `physics_cli.rs`: `--config <json>` arg + fixed `phys_hz` positional index (was reading wrong argv).
   Validation: headless launch 0→60 km/h in 3 s, `Long_G` now **monotonic positive** (+0.19…+0.87), rear
   slip bounded ~0.05–0.42 (no stall, no negative-accel surge). In-engine integration test PASSED
   (ABI=7, JSON loaded with TC on). Residual low-speed slip ripple is minor; further smoothing optional
   (lower `max_clutch_torque_ratio` or PI TC). Note: `corr_04` baseline `max_torque=340` vs JSON `455`
   still diverges — reconcile per item 1.
   5. **Rear-grip abruptness + front-heavy diagnosis & follow-up fix (2026-08-17)** — Re-analyzed
      telemetry against config after the launch fix. Two findings:
      - *Front-heavy feel*: real in-game trace (`godot_f1_94_chutana_telemetry_B120_fixed_20260816.csv`)
        shows static compression `FL/FR≈66.6 mm` vs `RL/RR≈30.6 mm` (front ≈2.2× rear) → rear axle
        statically under-loaded (car sits nose-down/tail-up). Config contributor: `front_weight_distribution=0.46`
        (high) + shorter/stiffer rear spring. Scene contributor: rear rides ~half its design compression
        in-engine vs the Rust headless baseline. Fix: `front_weight_distribution=0.46→0.43`, rear
        `spring_length=0.180→0.200` + `resting_ratio` kept `0.350` (rear design compression now 70 mm,
        matching front) → headless rest state balanced (`FL=RL=70 mm`) and rear compresses under accel.
        In-game confirmation still required (scene ride-height may need a separate tweak).
      - *Abrupt rear-grip changes*: the proportional slip-limiter from item 4 applied its cut instantly
        across the 120 Hz step → a ~15 Hz torque/slip limit-cycle (`DriveTorque` ~400↔2100 N·m,
        `Rear_Slip` 0.05↔0.42). Fix: added `tc_cut_ratio_smoothed` state and low-pass the cut
        (`alpha = 1-exp(-rate*dt)`, `rate = traction_control_cut_gain*20` clamped 5-50) so the limiter no
        longer hunts. `traction_control_cut_gain` (was a dead knob) now drives smoothing responsiveness.
        Validation: headless launch `Rear_Slip` settles ~0.10 (no hunt), `DriveTorque` rises smoothly
        ~1000→1450 N·m; all 9 Rust test suites pass.
      - *Incident*: `physics_cli` wrote telemetry over the config path (positional misuse without
        `--config`) and corrupted `f1_94_physics.json` (restored from git, edits re-applied). Hardened
        `physics_cli.rs` to refuse overwriting the config file, and fixed `scripts/build_windows.ps1`
        (`$ErrorActionPreference='Stop'` turned cargo stderr into a fatal error, blocking the DLL copy) to
        use `Continue` + explicit exit-code checks.
   6. **Snapshot-server architecture decision + `game_sim` core (Fase 0/1, 2026-08-17)** — Evaluated
      moving the whole simulation into Rust with Godot as a pure render mirror. Decision: **adopt the
      authoritative Rust core emitting serializable snapshots** (not gdext; the host C++ stays a thin
      bridge). Rationale: perfect sync by construction (ONE simulator; headless and in-engine share the
      same code path + identical inputs — this prevents the FL/RL desync class we hit). Implemented
      `game/sim` crate (`game_sim`): owns world/scene graph, vehicles, aids (`driving_aids.gd` ported to
      `AidsState`), session, and emits `Snapshot` (bincode) each tick. `World::step` calls the physics
      crate directly (Rust to Rust, no C-ABI). `game_cli` runs it headless; `World` unit test asserts
      byte-identical snapshots across identical runs (deterministic). Phase 1 done: core + contracts
      (snapshot/input/step) + headless harness. Pending: Phase 3 Godot bridge (minimal C++ node reads
      snapshot / pushes input), Phase 4 race rules/AI, Phase 5 retire GDScript mirrors + physics C-ABI FFI.
      Note: residual in-engine rear ride-height desync is a scene spawn issue, addressed when wiring the
      bridge (or separately), not by the core itself.
   7. **Fase 3 — Bridge nativo Godot (`F90SimBridge`) + C-ABI de `game_sim` (2026-08-17)** —
      `game_sim` ahora expone un C-ABI (`c_abi.rs`, crate-type rlib+cdylib) con
      `sim_world_create/destroy/spawn_from_json/spawn_canonical/set_input/step/pose/telemetry`
      y structs `#[repr(C)]` (`CSimPose`, `CSimTelemetry`, `CSimRaycastHit`,
      `CSimTriRaycastSample`). Nuevo nodo `F90SimBridge` (godot-cpp,
      `native/src/sim/f90_sim_bridge.{hpp,cpp}` + header C `f90_sim_bridge.h`): carga
      `game_sim.dll` (mismo patrón LoadLibraryW/GetProcAddress que F194RustVehicle),
      crea el `World`, spawnea el F1-94, y cada `_physics_process` lee el InputMap
      (`Throttle`/`Brakes`/`Steer Left`/`Steer Right`/`Handbrake`), llama
      `set_input`+`step`, y REFLEJA la pose en `set_global_transform` + imprime
      telemetría. Godot es espejo puro: el core es el único simulador. Build verde
      (`build_windows.ps1` ahora también compila/copia `game_sim.dll`,
      `SConstruct` globs `native/src/sim/*.cpp`). Test `c_abi::c_abi_roundtrip`
      valida el C-ABI headless. PENDIENTE: validación in-engine (no se puede correr
      el editor aquí) y, como sub-fase, cablear `F90SimBridge` para que conduzca el
      `F194RustVehicle` real (reemplazar la integración por fuerzas del body por la
      transform del snapshot) — hoy el bridge mueve su propio nodo de forma aislada
      y segura.
    8. **Cablear `F90SimBridge` al `F194RustVehicle` real (2026-08-17)** — El core ahora
      CONDUCE el auto real. `F90SimBridge` auto-descubre el primer `F194RustVehicle` del árbol
      y, durante el `_integrate_forces` del vehículo (`F194RustVehicle::drive_integrate`), (a)
      lee el InputMap desde los campos del vehículo (`throttle_amount_`/`steering_input_`/
      `brake_amount_`/`handbrake_amount_`, poblados por `F194RustInputController`), (b) muestrea
      las 12 RayCast3D (`collect_core_samples` -> `CSimTriRaycastSample[4]`), (c) pasa la
      cinemática del body (pose+velocidad) + muestras + input al core vía
      `sim_world_solve_external`, y (d) aplica las fuerzas/torques mundo que devuelve el core con
      `apply_central_force`/`apply_torque` (GODOT integra gravedad + colisiones). Telemetría +
      visuals vía `apply_core_telemetry`. ESTE es el mismo camino estable que el dll legado
      `vehicle_physics_engine` (fuerzas + Godot integra), así que la integración es robusta.
      C-ABI: `sim_world_solve_external` (nuevo; combina sembrar cinemática + `solve_external`
      y devuelve fuerza/torque), `sim_world_set_input`, `sim_world_telemetry`, `sim_world_step`,
      `sim_world_flat_samples`, `sim_world_spawn_*`. `F194RustVehicle`: `bridge_controlled_` +
      `drive_integrate`/`collect_core_samples`/`apply_core_telemetry`; `set_bridge_controlled`
      pone `gravity_scale=1.0` (GODOT aporta gravedad porque `solve_external` la excluye) y
      mantiene el body despierto (`set_can_sleep(false)`/`set_sleeping(false)`).
      **BUG RAÍZ CORREGIDO (2026-08-17):** el GDScript `f1_94_rust_vehicle.gd` sobreescribía
      `_integrate_forces` y llamaba SIEMPRE a `solve_forces_for_state` (dll legado), ocultando
      el override C++ que delega al bridge -> el drive del bridge era código muerto y el auto se
      movía con el dll. Se eliminó el override GDScript para que el C++ corra y delegue.
      Además el bridge leía `Input::get_action_strength` (signo de steering invertido vs el
      controlador) -> ahora lee los campos del vehículo (signo correcto). Y el primer intento de
      drive (velocity-drive / pose-mirror sobre `sim_world_step_with_samples` + integrador
      standalone del core) EXPLOTABA (el integrador propio del core es inestable cuando se siembra
      cada frame) -> se cambió a la ruta de fuerzas `solve_external` (estable, idéntica al dll).
       **VALIDACIÓN HEADLESS:** estable en reposo (v=0, posY≈-0.06) y ACELERA con `debug_throttle=1`
       (17→146 km/h, rpm sube, altura estable, sin explosión). Sigue sin validar headless el giro
       fino ni colisión de muros (el core solo muestrea suspensión, no muros). PENDIENTE:
       validación visual in-engine (correr `run_f1_94.ps1`: throttle acelera, steer gira en
       dirección correcta; muros pueden atravesarse en esta 1ª integración).
       **DRIFT LATERAL CORREGIDO (2026-08-17):** el bridge reconstruía la orientación del cuerpo
       desde `yaw` vía `Mat3::from_euler_yxz(yaw,0,0)`, pero `yaw_from_transform` y `from_euler_yxz`
       NO son inversos (el round-trip invierte el signo de X del forward) -> el empuje se aplicaba
       en un ángulo espejo y el auto derivaba de costado al acelerar. FIX: `sim_world_solve_external`
       ahora recibe el quaternion completo `(qx,qy,qz,qw)` (`gt.basis.get_rotation_quaternion()`) y
       reconstruye la basis con `Quat::to_mat3()` (igual que el dll legado, que pasa el quaternion
       completo). Validado headless en `sim_bridge_quick_test.tscn` (escena mínima plana):
       con `debug_throttle=1` el auto acelera en línea recta, **posX se mantiene 0.0** de 4→67 km/h
       (posZ 0→-24.9, posY estable ~0.32). Sin drift lateral.
       Nota: physics headless corre ~1 Hz (overhead de arranque/carga), por eso la validación usa
       `sim_bridge_quick_test.tscn` y se corre con `--quit-after 240`. El print de telemetría del
        bridge ahora incluye `posX/posY/posZ` para monitorear deriva lateral.

    7. **Keymap decoupling + gears-stuck fix (2026-08-17)** — User reported "gears stuck
       on 1st" and asked to verify the keymap is fully decoupled. Diagnosis: the keymap
       itself was fine/decentralized (actions live only in `project.godot [input]`, read by
       `F194RustInputController` by name, no runtime InputMap re-registration) — the real
       bug was in the bridge glue. `F90SimBridge::drive_integrate` hardcoded
       `clutch=0.0, gear_request=0, toggle_tc=false` into `fn_solve_external_`, so the
       controller's `gear_request` (set via Shift Up/Down) was never forwarded; and
       `c_abi.rs` mapped `gear_request==0 -> None` (no-change), so even a forwarded 0
       couldn't select Neutral (`types.rs:539` contract: Some(0)=Neutral, Some(-1)=Reverse,
       None=no-change).
       Fixes applied:
       - `native/src/sim/f90_sim_bridge.cpp` `drive_integrate`: now reads
         `get_gear_request()`, `get_clutch_amount()`, `get_aids_enabled_mask()`; added
         `last_aids_mask_` member (`f90_sim_bridge.hpp`) to edge-detect TC toggle; passes
         `clutch` and `(int8_t)gr` to `fn_solve_external_` instead of the literals.
       - `game/sim/src/c_abi.rs` (3 sites): `if gear_request < -1 { None } else { Some(...) }`
         so -2=no-request, -1=Reverse, 0=Neutral, 1..6=gears.
       - Keymap extraction (user chose full extraction, autoload = single source):
         created `game/addons/formula90s/scripts/input_bindings.gd` autoload that registers
         ALL actions in `_init()` via `InputMap.add_action`/`action_add_event` with canonical
         name constants (THROTTLE, STEER_LEFT, SHIFT_UP, …). Registered in `project.godot
         [autoload]` BEFORE `TelemetryManager`; **removed the entire `[input]` section** so
         the autoload is the single authoritative source (no drift between two definitions).
       - `f1_94_rust_input_controller.gd`: `@export` action strings now default to
         `InputBindings.XXX` (kept `@export` for per-instance override); added
         `action_reset_vehicle` and wired `Reset Vehicle` -> `vehicle.reset_vehicle(spawn_pos,
         spawn_yaw)` (spawn pose captured at `_ready`).
       - Added the previously-MISSING `Toggle Traction Control` binding (referenced by the
         controller but never defined in `[input]`); the unreferenced `Reset Vehicle` action
         is now actually wired.
       Validation: `cargo build` + `scons` both green; headless straight-line launch
       (`sim_bridge_quick_test.tscn`, `debug_throttle=1.0`) still holds `posX=0.0` (lateral
       drift fix intact). Gears: headless upshift 1->6 confirmed via auto-shift script; a
       deterministic Rust unit test (`simulation.rs::gear_request_sentinel_neutral_and_reverse`)
       proves gear_request Some(2)->gear 2, Some(0)->Neutral, Some(-1)->Reverse, and
       None->no-change (all PASS). NOTE: headless physics runs ~1 Hz so a full shift
       (needs `shift_time` of consecutive ticks) can't complete in the headless window — the
        Rust unit test is the authoritative gear validation.

     8. **JSON = single source of truth for F1-94 (2026-08-17)** — Reconciled the four
        flagged divergences to `game/data/vehicles/f1_94/f1_94_physics.json` (canonical
        source), deprecating the hardcoded defaults that contradicted it. Values aligned:
        `vehicle_mass=575`, `max_torque=455`, `max_rpm=15000`,
        `automatic_transmission=false`. Edits: `game/physics/engine/src/vehicle_config.rs`
        (`f1_94_canonical()` + `default_*` serde fns),
        `native/include/formula90s/vehicle/f1_94_rust_vehicle.hpp` member defaults,
        `engine_spec.gd`/`chassis_spec.gd`, `f1_94_engine.tres`/`f1_94_chassis.tres`,
        `game/addons/gevp/scripts/vehicle.gd`. Also hardened the silent `create_default`
        fallback (`f1_94_rust_vehicle.cpp`) to log a loud WARNING on JSON load failure, and
        added a HARD rev limiter in `powertrain.rs` (RPM clamped at `max_rpm`, torque cut at
        `max_rpm`, neutral free-rev also capped). Validated: all engine + sim unit tests pass
        (incl. new `rev_limiter_caps_rpm_at_max`, `manual_transmission_does_not_auto_shift`,
        `c_abi_roundtrip`); headless build loads JSON with NO fallback/parse warning.
        DEPRECATION: the three conflicting docs (`game/docs/CANONICAL_F1_HANDLING_ACCEPTANCE.md`,
        `game/addons/formula90s/PARAMETROS_F1_1996_GEVP.md`,
        `docs/GEVP_RUST_PHYSICS_MIGRATION_BACKLOG.md`) were superseded by the JSON and
        **DELETED on 2026-08-17** (docs-only reconciliation; runtime already read the JSON).

    9. **F1-94 vehicle audio — Rust core + native GDExtension node (2026-08-18) [DONE — 2026-08-18]**
       Implemented: pure-Rust sample-accurate mixer (`game/audio/engine/`, cdylib
       `vehicle_audio_engine`) driven by a new `VehicleAudioControllerNative` C++ GDExtension
       node; both F1-94 scenes (`f1_94_rust.tscn`, `f1_94.tscn`) swapped from the GDScript
       `VehicleAudioController` to the native node. Same `trigger()` one-shot API and
       `last_*`/`surface`/`_active_bed`/`ENGINE_BAND_NATIVE_RPM` telemetry contract, so
       `audio_telemetry.gd` is unchanged except resolving the `vehicle` NodePath. Rust core
       validated (`cargo test` green; real bank at `game/sounds/banks/v10_vehicle` loads at
       runtime — headless smoke confirms `is_engine_loaded=true` and live `last_rpm`). Build
       script (`scripts/build_windows.ps1`) builds/copies the audio DLL; both GDExtension
       debug+release DLLs rebuilt with the node.
               - **Latency + clipping fixes (2026-08-18) [DONE]:** With audio confirmed working, user reported a
          large output delay and clipping/distortion at RPM changes. Fixed: (a) `VehicleAudioControllerNative`
          `set_buffer_length` 0.12→0.06 s and added `audio/output_latency/buffer_size_ms=10` to `project.godot`
          (WASAPI shared mode — Godot 4.7 has **no exclusive mode**, so device format/latency is OS-driven;
          bit depth is irrelevant). (b) Rust `mixer.rs` clipping root cause: the band-crossfade sum sat on the
          soft `tanh` limiter knee (no headroom), injecting odd-harmonic distortion perceived as clipping on
          the dry asphalt signal. Added `engine_headroom = 0.62` (mirrors the offline oracle `engine * 0.62`)
          applied before the limiter, and a per-sample one-pole RPM glide (`RPM_SMOOTH_TAU = 0.025 s`) so band
          weights + pitch scale continuously instead of stepping per frame (kills the zipper/warble at RPM
          changes). Bumped `shift_gain` 0.40→0.80 for one-shot presence. Added regression tests
          `rpm_ramp_stays_below_headroom_ceiling` + `pitch_glides_instead_of_stepping` (`cargo test` green,
          30 tests). (c) Added `ensure_vehicle_bus.gd` autoload creating the `Vehicle` bus + `AudioEffectLimiter`
          (ceiling −0.3 dB) as a DAC-clip safety net. Performance was **not** the cause (render is trivial
          per-sample arithmetic; also reused the C++ mix buffers to drop a per-tick allocation).
        - **OPEN ISSUE — asphalt / surface beds [STILL OPEN — separate parity gap]:** In the F1-94
         **rust** variant, `detect_surface()` currently always returns `"asphalt"`, so the
         grass/rumble/sand bed layers never play (only the dry engine layer is audible). Cause:
         `F194RustVehicle` exposes neither `axle.wheels[i].surface_type` (the GEVP primary
         source the GDScript used) nor `WheelFrontLeft/…` RayCasts (the native fallback names),
         so both detection paths yield nothing. The GEVP `f1_94.tscn` variant DOES get surface
         beds via `axle`. This is a parity gap (the old GDScript had the same limitation on the
         rust variant, so no *new* regression, but the surface beds are effectively dead on
         F1-94-rust). Fix path: expose the rust vehicle's per-wheel `surface_type` (or add
         `WheelFrontLeft/WheelFrontRight/WheelRearLeft/WheelRearRight` RayCasts) so
         `VehicleAudioControllerNative::detect_surface` can read it. Track explicitly.
        - **No-audio fix (2026-08-18) [RESOLVED — in-game audible confirmed by user]:** User reported
         zero audio after the native swap. Root cause: `create_audio_nodes` used `AudioStreamPlayer3D`,
         which is **silent without a current `AudioListener3D`** — and the project has none, with the
         F1-94 rendered inside a `SubViewport` (`WorldViewport`). The original GDScript
         `VehicleAudioController` used a **non-positional `AudioStreamPlayer`** (line 119), which needs
         no listener — that is why it worked. This godot-cpp build does NOT wrap the non-3D
         `AudioStreamPlayer` class, so the node now instantiates it generically via
         `ClassDB::instantiate("AudioStreamPlayer")` and drives it through the `Object`/`Variant` API
         (`set("stream"/"bus")`, `call("play"/"get_stream_playback"/"push_frame"/"get_frames_available")`).
         Removed the earlier `unit_size(10000)/max_distance(0)` 3D workaround. Also: (a) added
         `windows.editor.dev.x86_64` + `windows.editor.x86_64` keys to `formula90s.gdextension` so the
         extension loads in the editor/headless binary (previously failed "Cannot get class"); (b)
         `set_physics_process(true)` in `_ready`. **Structurally validated headless:** node loads,
         `is_engine_loaded=true`, `last_rpm=4500` (telemetry live), `surface=asphalt` (confirms the
         separate surface issue below), `is_audio_active=false` only because headless uses the Dummy
          driver (`create_audio_nodes` early-returns). In-game audible **confirmed by user** (audio plays);
          subsequent latency + clipping passes applied (see above). The `--audio-driver WASAPI` launch net
          remains in `scripts/run_f1_94.ps1` as a safety measure. No special Godot 4.7 parameter is required
          for the compiled-Rust pipeline itself.

     10. **[HIGH PRIORITY] Stutter / frame-pacing analysis & refactor — Rust physics core vs 3D Godot presentation (2026-08-18) [NEW]:** User reports intermittent stutters during gameplay. Scope: profile the authoritative Rust `game_sim` / `game/physics/engine` core (per-tick step cost, snapshot bincode (de)serialization, allocation churn) AND the 3D Godot presentation path (`F194RustVehicle`, `VehicleVisual3DController`, `DirectionalVehicleSprite`/mesh updates, camera) to locate the jitter source. Hypotheses: (a) per-tick snapshot bincode (de)serialization cost; (b) physics-vs-render cadence/threading mismatch; (c) presentation lacks interpolation across the 120 Hz physics tick; (d) per-frame allocation/GC in presentation scripts. First reversible step: instrument frame-time + step-time (min/max/percentile) on both sides, reproduce headlessly via `game_cli` + a timing harness, then decide whether to optimize the Rust core or restructure the Godot 3D update/interpolation. This is a perf/architecture task — do NOT change physics tuning.

     11. **Fachada-orquestador `formula90_core` + fix P0 de ayudas (2026-08-18) [VALIDATED P0 — P1/P2 pendientes]:** Se migró el runtime a UN solo orquestador Rust (`game/core` crate, cdylib `formula90_core.dll`) que posee física (`game_sim`+`vehicle_physics_engine`) + audio (`vehicle_audio_engine`) y un `ModuleRegistry` expandible (`SimModule`), expuesto a Godot por un único nodo C++ `F90Core` con un solo handshake (`f90_core_abi_version()`, hoy `2`). Reemplaza los 3 módulos cargados por separado. Detalles:
        - **ABI v2**: `f90_core_step` lleva `aids_mask: u32` (los 8 bits) en vez del pulso `toggle_tc`; se añadió `f90_core_apply_runtime_config` (espejo de `F90RuntimeConfig`). Headers: `native/include/formula90s/core/f90_core.{h,hpp}`; `F90Core::drive_integrate` + `apply_runtime_config` + `reset_core_at` + `pump_audio` (push_buffer en lote) en `native/src/core/f90_core.cpp`.
        - **FIX P0 (ayudas/ESP/TCS en la facada)**: el mask completo se aplica cada frame (`CoreFacade::step` -> `ent.sim.aids = AidsMask::from_bits(aids_mask)`), eliminado el toggle espurio de primer frame (`last_aids_mask_`), y el tuning del vehículo (diff_preload, bias, aero...) se enruta al core en modo bridge (`apply_runtime_config_to_sim`, extraído a `vehicle_physics_engine::ffi`). **Antes**: el ABI perdía el mask (ESP congelado, TCS apagado por el toggle del primer frame, tuning descartado) — por eso la sesión facade parecía "sin ESP/TCS".
        - **Audio operativo** (confirmado por el usuario): `F90Core::_ready` crea los nodos de audio (bus `Vehicle`, generador 44.1kHz/60ms, `AudioStreamPlayer` no posicional) y `pump_audio()` hace un `push_buffer` en lote (eliminado el `push_frame` por muestra). Se quitó el `F90Core` duplicado del prefab `f1_94_rust.tscn` (había DOS fachadas alternando drivers -> "config de física rara").
        - **Auditoría JSON (subagente, read-only)**: casi todo el `f1_94_physics.json` se usa; claves muertas/hardcodeadas pendientes (P1): `surfaces.*.lateral_grip_assist` (0.04 vs tire.rs 0.05), `surfaces.*.longitudinal_grip_ratio` (0.64 vs tire.rs 0.5 → más patinaje), `differential.slip_transition_threshold_rad_s` (0.90 vs powertrain.rs:505 0.50), `input_smoothing_*` (simulation.rs:501/506 hardcode 20/10), `automatic_shift`(15 claves)+`gear_inertia` (parseados, no usados), `launch_control_*`/bit `auto_clutch`/`stability_upright_*`/`handbrake_*`/`wheel_hubs` (no usados), `brakes.enable_abs` vs bit del mask (dos fuentes). Valores a decidir con el usuario: `diff_preload` 40 vs canónico 170, `contact_patch` 0.35 vs 0.21.
        - **Validación**: tests Rust (physics + core, incl. `facade_applies_full_aids_mask_each_step`, `facade_apply_runtime_config_is_accepted`), build C++ debug OK, ambos cdylib (debug y template_release) a ABI v2, headless `sim_bridge_quick_test`/`vehicle_test_session` con `[F90Core] facade ready (ABI=2)` y vehículo conducido.
        - **PENDIENTE — Próxima tarea en `instrucciones.txt §4`**: **P1** cablear/eliminar las claves muertas (empezando por grip ratios/slip_threshold/input_smoothing; decidir automatic_shift/gear_inertia y diff_preload/contact_patch) y **P2** telemetría para la ruta Rust (`telemetry_manager.gd` solo captura GEVP; columna `TC_Active` 26 vs 25) + rebuild de la extensión native release.

---

     12. **P2 — Telemetría para la ruta Rust (`telemetry_manager.gd`) (2026-08-18) [IMPLEMENTADO — pendiente validación runtime]:** `telemetry_manager.gd` es un autoload que solo encontraba la clase GEVP `Vehicle` (`find_children("*","Vehicle",...)`), así que la ruta Rust (`F194RustVehicle`, conducida por la fachada `F90Core`) nunca producía CSV. Cambios (archivo `game/addons/formula90s/scripts/telemetry_manager.gd`):
        - `vehicle` ahora es `untyped` (acepta tanto `Vehicle` GEVP como `F194RustVehicle`); `_try_find_vehicle` busca primero `F194RustVehicle` y cae a `Vehicle`.
        - `_format_line` ramifica por `_is_rust`: la ruta Rust lee `get_wheel_compressions()` / `get_wheel_slips()` (orden [FL,FR,RL,RR] en el C++) para `FL/FR/RL/RR_Comp` y `Front/Rear_Slip`; la ruta GEVP conserva `front_axle`/`rear_axle`.
        - **Fix columna `TC_Active`**: `CSV_COLUMNS` declara 26 columnas pero el `format` solo tenía 25 placeholders → el valor `TC_Active` (bit 1 del `aids_enabled_mask`) se descartaba silenciosamente. Se añadió el 26º `%d`, así ahora se emiten las 26 columnas (26 placeholders ↔ 26 valores verificados).
        - Nota: el `_setup_json` de la ruta Rust queda con nulls en props GEVP (aceptable; capturar el JSON de config del core es trabajo aparte, ver item 2 de §45.0).
        - **Validación**: NO verificada end-to-end en este sandbox (Godot headless crashea con signal 11 al cargar la extensión native — limitación del entorno, no del cambio). Requiere rebuild de la extensión native release (P2 item 8 implícito) y correr en entorno real: headless `sim_bridge_quick_test` debe escribir `game/telemetry/*.csv` con 26 columnas y datos del `F194RustVehicle`. No se hizo commit (per instrucciones).
         - **ESTADO 2026-08-19 (working tree):** el cambio sigue sin commit; la ruta Rust ahora es la ruta primaria bajo `F90Core`. El codigo esta en el arbol de trabajo y compila (debug+release DLL rebuild verde). La validacion runtime sigue pendiente en entorno real (headless bloqueado en sandbox).

---

     13. **P1 — Fidelidad del JSON: cablear parámetros muertos/hardcodeados (2026-08-18) [IMPLEMENTADO — tests verdes]:** Se sustituyeron hardcodes por los valores de `f1_94_physics.json` (vía `VehicleConfig`, ya disponibles):
        - `tire.rs` `process_wheel_forces`: `lateral_grip_assist`/`longitudinal_grip_ratio` ahora se leen de `config.surface_lateral_grip_assist`/`surface_longitudinal_grip_ratio` (HashMap por superficie, poblado del JSON por `build_surface_assist_map`); se eliminaron las funciones hardcodeadas 0.05/0.5. Efecto: Road `longitudinal_grip_ratio` 0.5→0.64 (más agarre longitudinal → menos patinaje/“sensación sin TCS”) y `lateral_grip_assist` 0.05→0.04.
        - `powertrain.rs` `solve_salisbury_differential`: `delta_omega_threshold` 0.50→`config.diff_slip_transition_threshold_rad_s` (0.90); se añadió el parámetro a la firma, al call site y al test unitario `differential_test.rs`.
        - `simulation.rs` `filter_inputs`: throttle/brake smoothing 20.0/10.0→`cfg.aids.input_smoothing_throttle_rate`/`_brake_rate` (JSON 7.5/14.0).
        - Fix de acompañamiento: `f1_94_canonical()` (fallback) tenía `surface_lateral_grip_assist` uniforme 0.05 (road==grass) → rompía `test_surface_interaction_parity`; se diferenció (Road/Curb 0.05, Dirt/Grass/Gravel 0.0) reproduciendo el mapeo hardcodeado previo. El path JSON (autoritativo) ya diferenciaba vía `build_surface_assist_map`.
        - **Validación**: `cargo test` game/physics/engine (OK) y game/core (OK) → EXIT 0; rebuild debug+release de todos los DLL (item 14).
        - **PENDIENTE (decisiones del usuario, NO cambiadas para no alterar el feel sin validación)**: (a) `diff_preload` JSON 40 vs canónico 170 y `contact_patch` 0.35 vs 0.21 — ya cableados desde config, solo diverge el valor canónico; (b) `automatic_shift` (15 claves)+`gear_inertia`: parseados pero NO usados (auto-shift hardcodeado en `powertrain.rs`); (c) P1-coherencia `brakes.enable_abs` vs bit del mask y `driving_aids.gd` toggles ESTAB/FRENOS/GRIP no-op (UNSUPPORTED) — requieren decisión/handing aparte.

     14. **P2 item 8 — Rebuild RELEASE native + formula90_core template_release (2026-08-18) [DONE — EXIT 0]:** `scripts/build_windows.ps1 -Configuration debug` y `-Configuration release` → ambos EXIT 0. Recompila `libformula90s` (C++ GDExtension, ahora CON `F90Core`, target template_debug/template_release) vía SCons y los 4 crates Rust (vehicle_physics_engine, game_sim, vehicle_audio_engine, formula90_core) en debug+release, copiando todos a `game/addons/formula90s/bin` (incl. `formula90_core.windows.template_release.x86_64.dll` y `libformula90s.windows.template_release.x86_64.dll` actualizados a ABI v2). Tras P1 (Rust) las DLL template_release ya reflejan los parámetros del JSON. NOTA: headless no se pudo correr en este sandbox (Godot crashea signal 11 cargando la extensión native) — la validación runtime queda pendiente en entorno real.

---

     15. **P1 — Implementar `automatic_shift` (JSON autoritativo) (2026-08-18) [IMPLEMENTADO — tests verdes, DLLs rebuild]**: El `automatic_shift` (15 claves en `f1_94_physics.json`) estaba parseado (`JsonAutomaticShift`) pero NO usado. Se añadió `AutomaticShift` al runtime `VehicleConfig` (con `Default` = valores del JSON y `from_json`), poblado en `f1_94_canonical` / `jordan_197_canonical` / `to_config`. `powertrain.rs` reescribió la selección de marchas para usar las 15 claves (umbrales RPM normalizados por throttle/coast, blend wheel/road-spin por peso, redline margin, kickdown con agresividad+delay, reverse/park speeds) en lugar de las constantes hardcodeadas (0.92/0.98/0.40/0.45/6.0). `gear_inertia` (antes muerto) ahora escala `shift_timer`. **Validación**: `cargo test` game/physics/engine + game/core → EXIT 0; rebuild debug+release (item 14) ya incluye el cambio. NOTA: el JSON F1-94 tiene `automatic_transmission=false`, así que la lógica auto solo se ejerce si se habilita; requiere validación en-engine (headless bloqueado en este sandbox).

     16. **P1-coherencia — Enrutar toggles de aids por `set_aids_enabled_mask` (2026-08-18) [IMPLEMENTADO]**: `driving_aids.gd` (ESTAB/FRENOS/GRIP, índices 1/3/4) tocaba propiedades UNSUPPORTED en el Rust `F194RustVehicle` (`vehicle_tunable_contract.gd` las bloquea → no-op). Ahora, cuando la propiedad no es soportada (Rust), enruta por la máscara de aids: ESTAB→bit2 (stability), FRENOS→bit7 (brake-assist), GRIP→bit1 (TC). El path GEVP (mutación de propiedades) queda intacto vía `VehicleTunableContract.is_property_supported`. GDScript puro, sin rebuild de DLL. Asimismo `.agents/AGENTS.md` ahora lleva la instrucción corta **JSON SOT**: `f1_94_physics.json` es la única fuente de verdad para el tuning F1-94 (siempre gana sobre defaults de código / fallback canónico / escena).

---

## 45.1 Legacy Jordan/GEVP Phase C work (DEFERRED — historical)

The tasks below were written for the Jordan/GEVP era and are deferred until the F1-94
Rust path is signed off. Do not reopen them against F1-94 without reclassifying.

Before Phase C tuning (Jordan-era, historical):

## Task 1 — telemetry setup snapshot

Implement:

```text
<session>.csv
<session>_setup.json
```

Requirements:

- same basename;
- runtime snapshot;
- JSON;
- provenance;
- physical configuration;
- aids;
- surface configuration;
- new session when relevant setup changes.

## Task 2 — validate telemetry pairing

Prove that:

- every CSV receives a setup JSON;
- setup filename matches;
- values reflect runtime;
- missing optional families remain explicit rather than invented;
- changing steering setup starts a new run or produces a new paired snapshot.

## Task 3 — start Phase C

Only after telemetry provenance is trustworthy.

Then tune:

```text
steering
countersteer
unwind
steering exponent
steering assistance directly related to steering behavior
```

Do not simultaneously retune mechanical grip, suspension, powertrain or aero.

---

# 46. Definition of a valid Phase C test

A useful Phase C run should record:

```text
vehicle setup
track
Git commit
aid state
steering parameters
speed
steering input/output
yaw-related response if available
front/rear slip
lateral G
wheel contact/suspension state
```

Human test notes should describe:

- turn-in progression;
- steering buildup;
- unwind;
- initial oversteer response;
- countersteer timing;
- whether recovery feels understandable;
- whether a late correction still produces a spin;
- whether behavior changes materially with speed.

Do not judge Phase C from a single corner or one uncontrolled run.

---

# 47. Definition of done for Phase C

Phase C is not complete because a candidate setup merely feels better once.

Completion requires:

- paired telemetry/setup records;
- repeatable steering behavior;
- no obvious regressions to Phase B;
- progressive keyboard response;
- predictable unwind;
- recoverable but not omniscient countersteer;
- retained spin threshold;
- human validation on multiple corners/speeds;
- accepted final values recorded here as `[VALIDATED]`.

After Phase C closes:

1. replace candidate values with final validated values;
2. mark Phase C `[VALIDATED]`;
3. freeze those values for Phase D;
4. record any reusable debugging lesson in `docs/engineering/common-errors-and-fixes.md`.

---

# 48. Forbidden interpretation shortcuts

Agents must not make these assumptions:

```text
"latest chat value" = validated value
"scene looks correct" = collision is correct
"generated texture exists" = source pipeline is correct
"same filename" = same physical setup
"aid OFF" = zero hidden stabilization
"current branch" = current file contents without inspection
"old telemetry" = usable without setup provenance
```

Always inspect the relevant authority.

---

# 49. Compact project invariants

The following principles should survive all future phases:

```text
1. Change one major physics family at a time.

2. Telemetry is paired with immutable setup provenance.

3. Runtime values beat remembered values.

4. Frozen GEVP baseline remains a diagnostic reference.

5. Vendor code is not the default place for Formula90s-specific fixes.

6. Track bugs are not solved by retuning vehicle physics.

7. Visual geometry and collision geometry are separate responsibilities.

8. Generated assets must be reproducible.

9. Texture Forge is the final deterministic texture compiler.

10. Procedural environment generation always starts from clean Base.

11. Generated runtime publication is atomic.

12. Current project state belongs in PROJECT_STATE.md,
    durable direction belongs in docs/architecture/project-direction.md,
    reusable troubleshooting belongs in docs/engineering/common-errors-and-fixes.md.

13. Candidate values must never be silently promoted to validated values.

14. A new chat/agent must be able to continue the project from repository context
    without requiring the previous conversation.
```

---

# 49.1 Runtime world/HUD presentation

Status: [VALIDATED]

Gameplay is composed through:

    GameBootstrap
    -> WorldHudCompositor
       -> WorldViewport (640x360 3D world)
       -> WorldPresenter (ViewportTexture in the root canvas)
       -> HudLayer (root CanvasLayer)

Canonical files:

    native/src/core/game_bootstrap.cpp
    native/include/formula90s/core/game_bootstrap.hpp
    game/scenes/runtime/world_hud_compositor.tscn
    game/scenes/runtime/world_hud_compositor.gd

The compositor moves only direct root Control nodes from the instantiated
world scene to HudLayer at runtime and retargets their vehicle/aids NodePaths.
Vehicle/track serialized node_paths remain inside the world scene unchanged.

This creates the required boundary for future world-only Super xBR processing:

    Super xBR shader -> WorldPresenter only
    HUD / minimap / menus -> never sampled by the world shader

Validation:

    smoke_test_world_hud_compositor.gd                    PASS
    smoke_test_bootstrap_world_hud_compositor.gd          PASS
    smoke_test_jordan_skybox_runtime.gd                   PASS
    smoke_test_la_chutana_skybox.gd                       PASS
    GPU visual composition capture at 1280x720            PASS

No final Super xBR shader, 3xBRZ texture generation, or player graphics toggle
is validated yet.

---

# 49.2 Arcade HUD and real-time La Chutana map

Status: [VALIDATED]

The normal gameplay HUD is a 16:9 late-1990s arcade presentation. It is still
drawn in `HudLayer`, never inside `WorldViewport`.

Canonical files:

    game/scenes/ui/debug_hud.tscn
    game/scenes/ui/la_chutana_hud_map.tres
    game/addons/formula90s/scripts/arcade_race_hud.gd
    game/addons/formula90s/scripts/arcade_speed_gauge.gd
    game/addons/formula90s/scripts/track_minimap_controller.gd
    game/addons/formula90s/scripts/track_map_data.gd
    game/addons/formula90s/scripts/driving_aids.gd

Presentation contract:

    left-middle: simple canonical La Chutana outline
                 + green start/finish marker
                 + oriented blue player marker derived from vehicle transform

    bottom-right: color-segment speed arc + runtime KPH + runtime gear

    top-right: transient white aid-state message, no panel/background

`TrackMapData` is presentation data only. It samples the canonical generated
La Chutana centerline; it must not be used as lap progress or race position
authority.

`DrivingAidsController` emits:

    signal aid_toggled(aid_label: String, enabled: bool)

`ArcadeRaceHud` consumes that signal and shows `AYUDA <label>
ACTIVADA/DESACTIVADA` for a short duration, then fades it away.

The prior cyan gameplay diagnostics panel is no longer shown in normal play.
`WheelDiagnostics` remains available but defaults to hidden in the Jordan
handling scene.

Future boundary:

    UI-002 / FUTURE_UI-002
    -> race-session authority for lap, position, countdown, and opponents.

Do not display invented LAP/POS/countdown/rival values before that authority
exists. The HUD scene documents this with semicolon-prefixed `.tscn` comments.

Validation:

    smoke_test_arcade_hud_scene.gd                      PASS
    smoke_test_arcade_hud_runtime.gd                    PASS
    smoke_test_world_hud_compositor.gd                  PASS
    smoke_test_bootstrap_world_hud_compositor.gd        PASS
    smoke_test_jordan_skybox_runtime.gd                 PASS
    smoke_test_la_chutana_skybox.gd                     PASS
    GPU capture arcade_hud_16x9.png at 1280x720         PASS
    GPU world-tint capture confirms HUD remains crisp   PASS

Default-route (GameBootstrap) evidence:

    The default bootstrap track (test_field.tscn) required a DrivingAids
    controller for the full HUD contract: without it the visual capture could
    not toggle the aid notification. Added the same DrivingAids node/script
    used by jordan_handling_test.tscn. Re-ran and captured:

        smoke_test_bootstrap_world_hud_compositor.gd   PASS
        (default route: start_game -> compositor -> minimap tracks the
         injected vehicle; projected map position changes after movement)

        visual_test_arcade_hud_capture.gd              PASS
        capture: user://arcade_hud_16x9.png (1280x720)
        pixel analysis: minimap track outline + green start marker + blue
        player marker present; speed gauge yellow/orange/red/purple segments
        and white KPH text present; no HUD filtering by WorldPresenter.

    The compact widget sizes match the acceptance contract at 1280x720
    (2x canvas scale): minimap 216x256, speed gauge 352x220 physical pixels.

    Test hardening: vehicle teleports in the default-route tests use
    PhysicsServer3D.body_set_state instead of assigning global_position,
    because the physics server can revert a direct transform set on a
    sleeping RigidBody3D (raced against the 60 Hz physics step). Both
    tests are deterministic (5/5 and 3/3 runs).

    smoke_test_jordan_skybox_runtime.gd: the camera-follow assertion waits
    two process frames (SourceSkyboxRig._process updates after the
    process_frame signal), fixing a one-frame race. Deterministic 5/5.

Known unrelated validation output:

    EngineAudioController requires EngineAudioConfig
    -> existing test_field default-scene configuration warning.

---

# 50. Maintenance footer

When updating this document, update this footer.

```text
Last reviewed:
Branch:
Commit:
Current active phase:
Next required gate:
Reviewed by:
```

Recommended current values at the time this snapshot is created:

```text
Last reviewed:
2026-08-20

Branch:
f1-94

Commit:
dc33d8d (branch f1-94; scalable brake thermal pipeline accepted and pushed)

Current active phase:
F1-94 `F90Core` is the primary driver; `f1_94_physics.json` is the single source of truth. The scalable per-axle brake thermal pipeline and resolved telemetry are `[VALIDATED]` with Physics ABI 12, Core ABI 7, debug/release builds, workspace tests, and canonical Godot smoke/audio smokes accepted.

Next required gate:
Implement tyre/tire degradation, then run the mechanical validation matrix and accept the mechanical freeze. Only after that gate should the V10 powertrain phase begin.

Next physics phase:
Tyre/tire degradation `[PLANNED]`, followed by repeatable mechanical tests and sign-off; powertrain remains gated until those tests pass.

Reviewed by:
Codex + user acceptance — F1-94 scalable brake thermal pipeline, telemetry, ABI synchronization, and smoke/build validation.
```

The exact commit SHA must be refreshed from Git before this file is treated as a
new canonical repository snapshot.

---

# End of current-state handoff
