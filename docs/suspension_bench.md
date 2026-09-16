# SUS-GEO-08 — Suspension bench report

Source: `game/crates/vehicle-physics-engine/examples/suspension_bench.rs`
(run: `cargo run -p vehicle_physics_engine --example suspension_bench --release`).
Fixture: healthy trailing-rocker test geometry (NOT the shipped car);
numbers below are mechanism evidence, not F1-2030 calibration.

## 1. Motion ratio r = ds/dq

| corner | r0 (rest) | min | max | shape |
|---|---|---|---|---|
| FL | 0.478 | 0.012 | 0.822 | rising-rate, then toggle collapse past q≈+0.08 |
| RL | 0.383 | 0.221 | 0.575 | falling-rate (digressive) across travel |

Consequences (SUS-GEO-04/09):
- Front bump travel MUST stay clear of the toggle (q≲+0.07 for this rocker);
  the travel envelope + `r<=0` guard enforce it, but calibration should place
  max bump on the rising slope, not past the peak.
- Rear is digressive everywhere: `F·dr/dq < 0` subtracts from the tangent
  rate (see §2). A rising-rate rear rocker is 09 design work.

## 2. Rest wheel rates (tangent k·r² + F·dr/dq, linear k·r², damper c·r²)

| corner | k tangent | k linear | geometric term | c bump | c rebound |
|---|---|---|---|---|---|
| FL | 35822 | 26705 | +34 % | 2693 | 3636 |
| RL | 16412 | 27118 | −39 % | 3005 | 4057 |

The F·dr/dq term is NOT negligible (±35–40 %): linear-rate models are wrong
by that margin on these rockers. Legacy-matched wheel rates hold by
construction (preload·r0 = static load), so static stance matches legacy
while the travel response is geometric.

## 3. Stops, reactions, balance

- Travel stops engage on wheel limits + damper stroke + mechanism clamp
  (SUS-GEO-04 tests: 120 mm slam engages, travel bounded, no NaN).
- Whole-wheel balance residual 0.00 N on the unit check; <2 % Fz settled
  in sim (SUS-GEO-05 tests). Load-path split at rest ≈ 99 % element /
  1 % wishbones (the spring carries body weight; wishbones carry
  lateral/moment loads) — registered per wheel, symmetric L/R.
- Anchor moments replace contact-patch moments in geometric mode; total
  force matches legacy within unsprung terms (axle totals agree).

## 4. Push/pull equivalence

Relabelling pushrod↔pullrod without touching geometry: max hub/damper
delta over the full sweep = **0.0 m exact**. No artificial grip by label.

## 5. Cost per tick (release, this machine) — corrected for the real flow

**Correction (SUS-GEO-11):** the original 0.3–0.5 ms figure below counted the
mechanism solves only (~6/wheel) and missed the 9 `travel_envelope` searches
per wheel and tick performed by the shipped flow (stops + travel bounds inside
every substep). Measured with the full FFI tick, the shipped geometric profile
cost **89.8 ms/tick in debug and 26.5 ms/tick in release** at 120 Hz, which is
the performance regression fixed by SUS-GEO-11 (§7).

Original mechanism-only numbers, kept for the record:
- `solve_corner`: ~11 µs; `jacobian` (2 solves): ~22 µs.
- Mechanism-only tick ≈ 6 solves/wheel → ~70 µs/wheel; anchor wrench +1 solve.
- 4 wheels ≈ 0.3–0.5 ms/tick at 120 Hz — **does not include the envelope
  searches of the shipped path; do not use as a whole-tick estimate**.
- Debug builds are ~10–20× slower (PBD iterations unoptimized); CI timing
  uses small tick counts for that reason. No HOST/GPU cost (pure Rust).

## 6. Legacy vs geometric deltas (matched static stance)

- Static Fz identical (preload·r0 = legacy static load by calibration).
- Wheel rate at rest identical by construction; travel response differs:
  geometric adds progressivity (F·dr/dq), bump-steer/camber from linkage
  (SUS-GEO-06, replaces linear gains), anchor moments (jacking/anti-dive
  emerge, SUS-GEO-05), physical stops (SUS-GEO-04).
- No silent fallback anywhere: invalid geometry fails with wheel+field.

## Conclusions for SUS-GEO-09

1. Calibrate bump/droop INSIDE the rising slope (front q≲+0.07 on these
   arms; design arms for the travel actually used).
2. Prefer near-constant-r rockers (long arms, small angles) so the linear
   intuition holds and the F·dr/dq term stays a correction, not the rate.
3. Keep matched preload (equilibrium at design) + legacy ARB ratios as the
   starting calibration, then tune from telemetry (SUS-GEO-10 A/B).

## 7. SUS-GEO-11 — hot-path fix: prepared envelopes (F1 2030 geometric)

### Root cause

`solve_force_geometric` re-derived the reachable travel envelope from inside
every substep: `geometric_stop_force` and `geometric_travel_bounds` each called
`geometric_q_limits → travel_envelope → 2 × bisect_limit`, up to 48 full
`solve_corner` calls (128 PBD iterations each) per limit. With 4 substeps at
120 Hz that is 9 searches/wheel/tick (36/vehicle) plus the exact start/final
solves. The envelope depends only on hardpoints + droop/bump at zero direction,
so the work was invariant per configuration.

### Fix

- `PreparedGeometricSuspension` (`suspension.rs`): per-wheel `q_min`, `q_max`
  and `k_wheel_rest_n_per_m`, built once per `SuspensionSystem` (sim
  create/reset) from the validated zero-direction geometry.
- Stops and travel bounds consume the prepared limits; the stable per-substep
  flow performs **0 envelope searches**.
- Linkage pose reuse: the exact solve from `geometric_wheel_forces` feeds
  camber/toe/hub telemetry instead of a second identical `solve_corner`
  (same inputs, side-independent upright basis).
- Invalidation: `SuspensionSystem::new` / explicit
  `rebuild_geometric_prepared`; no runtime path mutates
  `VehicleConfig::geometric_suspension`, no per-substep hashing or cloning.

### Whole-tick measurements (FFI, ctypes probe, same machine/profiles)

`probe.py` (TEMP audit tool): `create_from_json` + `solve_forces` at 1/120 s,
5 warmup ticks + 5 × 20-tick batches, medians. Build `d6dc82be` sources;
binaries archived with SHA256 (fixed debug `1ad0614f…`, release `50a4fba6…`).

| profile | build | before (ms/tick) | after (ms/tick) | speedup |
|---|---|---|---|---|
| legacy `f1_2030_v10_physics` | debug | 0.0405 | 0.0403 | — |
| legacy `f1_2030_v10_physics` | release | 0.00703 | 0.00709 | — |
| geometric `f1_2030_v10_geometric` | debug | 89.79 | **1.176** | 76× |
| geometric `f1_2030_v10_geometric` | release | 26.51 | **0.329** | 80× |

- Budget: 8.33 ms at 120 Hz; internal target 0.5 ms release / 2 ms debug — met.
- Creation cost grows by the one-time preparation: ~1.9→11.7 ms debug,
  ~0.5→3.8 ms release.
- Per-tick exact solves after the fix: 6/wheel (24/vehicle); envelope searches
  0 (asserted by the `#[cfg(test)]` counters in `suspension_kinematics.rs`).
- Equivalence: 640-tick force traces (rest, 40 mm one-wheel bump, 160 mm drop
  with contact loss, recuperation) are **bit-identical** before/after in debug
  and release; the final force vectors match at all 17 digits.
- Memory: private bytes flat over 20 000 ticks release (10 809 344 B) and
  5 000 ticks debug (10 895 360 B); no continuous growth observed.

### In-engine (Godot 4.7.1, RTX 3050, debug template)

Runtime session physics monitor (`Performance.TIME_PHYSICS_PROCESS`):

| state | before | after |
|---|---|---|
| idle | ~103 ms/frame (1.1 FPS) | 2.6–5.5 ms |
| driving | ~104 ms/frame (1.1 FPS) | 2.6–5.5 ms |

- Tick pacing after the fix: `physics_frame` wall avg ≈ 8.33 ms (120 Hz) in
  headless and windowed runs; headless FPS 123–134.
- Isolated vehicle scene (no F90Core): monitor 2.4 ms idle / 4.6 ms driving.
- Windowed FPS in this session (18–23) is render/DWM-bound (physics monitor
  ≤ 4 ms), not physics-bound; interactive FPS belongs to the human gate.
- `game/tests/smoke_test_f1_2030_v10_perf.gd` reproduces the runtime numbers:
  `godot --headless --path game --script res://tests/smoke_test_f1_2030_v10_perf.gd`
  (options `--frames=N`, `--no-visual-suspension=true` for attribution).

### Tables / visual controller status (unchanged by this fix)

- `suspension_table.rs` has no runtime consumers: the only user is the
  `build_geo_table` example; `f1_2030_v10_tables.json` is not loaded by the
  game.
- `SuspensionTable` (GDScript) is never instantiated; `f1_wheel_visual_controller.gd`
  still solves the visual linkage with `SuspensionGeometry` (PBD) per physics
  frame. The tables are generated/validated data, **not** an active runtime
  path; earlier "unified pose" wording must not be read as in-engine integration.

### Follow-up: visual linkage single-source (visual misalignment fix)

The visual solver used to read the authored `suspension.geometry` block while
physics used `suspension.geometry_physical.corners`. For the F1 2030 profile
these are different mechanisms: front rocker axis `[0,1,0]` (visual) vs
`[1,0,0]` (physics) — 90° apart — with rocker arms 55/49.5 mm vs 80/70 mm,
different arm directions and front damper chassis 0.28 m lower; solving both
at the same travel put `rocker_end`/`damper_end` 97–111 mm apart.

`SuspensionGeometry.from_json_dict` now prefers the physical corners (single
source; `rod` → `pushrod` + `attachment`, `mirror_of` and `_visual_meshes`
preserved) and falls back whole to the legacy block when no physical geometry
exists. Telemetry had already shown the physics side aligned (straight-line
L-R asymmetry +0.06/+0.07 mm at speed, ±0.01 mm at rest); the misalignment was
purely visual. `smoke_test_f1_2030_v10_wheel_visual.gd` had a stale ±10 mm
planar tolerance and was already red at HEAD: the real rear hub recedes
~13.6 mm at full droop, so the check now uses a documented 20 mm detachment
guard instead of the design axle box.

### Known pre-existing failures (not attributed to this fix)

- `aero_test`: 4 cases fail before and after (`test_aerodynamic_lag_exponential_decay`,
  `test_reverse_and_zero_velocity_gives_no_downforce`,
  `test_aero_distribution_and_balance_from_elements`,
  `test_wing_cl_speed_independent_no_flex`).
