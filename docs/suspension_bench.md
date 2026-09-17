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

## 8. Post-handoff: packaging/visual fixes, provenance, and the audio stall

This section was added after the SUS-GEO handoff; unless stated otherwise the
numbers below were **reproduced in this session** (source inspection + rebuilt
binaries + windowed runtime), not copied from commit messages.

### 8.1 Provenance restored (blocker cleared)

At handoff, `game/BUILD_SOURCE` held `c755b8d3` while HEAD was `49006e61`, the
installed DLLs disagreed with each other and with `target/debug`, and the
pre-existing Rust gear-sentinel edit was uncommitted. Resolution on
`main-clean`, atomic commits:

| commit | change |
|---|---|
| `2df6d94e` | `fix(gearbox)`: map the C-ABI gear sentinel so N/R are reachable (uncommitted dirt from before the handoff, isolated before touching suspension/audio) |
| `d0619fb6` | `chore(godot)`: register UID for `perf_capture_f1_2030_windowed.gd` |
| `b09e94f7` | `build`: republish Windows runtime for `d0619fb6` |
| `e3c52406` | `fix(audio)`: delta-budget pump scheduler + counters (see §8.3) |
| `a374e431` | `build`: republish Windows runtime for `e3c52406` |

Current state: HEAD `a374e431`, `BUILD_SOURCE` = `e3c52406` (publish parent,
accepted by `run_f1_94.ps1`). Stamped artifacts embed `e3c52406`
(`formula90_core`, `vehicle_audio_engine`, `libformula90s`, `f90_audio_dsp`);
`game_sim`/`vehicle_physics_engine`/`psx_art_plugin` carry no embedded stamp and
are outputs of the same clean build. `run_f1_94.ps1 -ValidateRuntimeOnly` →
`Paridad BUILD/HEAD validada: a374e431`. Rebuild used a clean
Cargo/SCons/`.godot` wipe (no cross-branch caches).

Known ambient quirk (not a source bug): `scripts/build_windows.ps1` sets
`$ErrorActionPreference='Continue'` and prints "copiada" after `Copy-Item`,
which can fail transiently with "user-mapped section open" (AV/first-run lock)
and leave a stale `bin` copy; the publish above re-synced every Rust pair from
`target/debug` and verified `bare == template_debug`.

### 8.2 Suspension visual fixes (commit-reported, not re-verified visually here)

`87d08c4d` (nose damper packaging), `b661283b` (front blade re-anchor),
`c755b8d3` (runtime visual-disable switch) and `49006e61` (audio cap + pose
cache) are in the tree. Their visual result (damper inside the nose, blade
attachment through travel/steering) still needs the human cockpit/exterior
gate; only the solver/regression tests were inspected here.

### 8.3 Audio scheduling (Issue A) — measured and fixed

Architecture: `F90Core::_process` calls `pump_audio`, which asks the
`AudioStreamGeneratorPlayback` (`WASAPI`, 44.1 kHz, buffer now 0.1 s) for free
frames, renders them through the Rust mixer and pushes one batched buffer.

Baseline defect: the `SUS-GEO-12` fixed cap `kPumpBudgetFrames = 1024` bounds
per-pump DSP but caps production at `1024 · render_FPS` samples/s, which is
**below** the 44100 Hz consumption whenever render FPS < ~43.07.

Mixer cost, headless bench (`gf509_rt_bench`, this machine; deadline = 1024
frames at 44.1 kHz):

| config | 1024-frame median | per-sample | CPU for 44.1 kHz |
|---|---|---|---|
| debug | 9.76 ms | ~9.5 µs | ~420 ms/s (~42 %) |
| release | 2.08 ms | ~2.0 µs | ~90 ms/s (~9 %) |

Windowed runtime, `game/tests/audio_sustained_capture_f1_2030.gd`, 300-frame
windows (mode 0 = legacy cap, mode 1 = delta budget):

| mode | max_fps | fps | produced/s | deficit/s | new ring skips |
|---|---|---|---|---|---|
| 0 | native | 45.1 | 44103 | −3 | 148 |
| 0 | 30 | 30.0 | 30720 | 13380 | 1545 |
| 0 | 20 | 20.0 | 20481 | 23619 | 5412 |
| 1 | native | 46.2 | 44122 | −22 | 0 |
| 1 | 30 | 30.0 | 44107 | −7 | 0 |
| 1 | 20 | 20.0 | 44067 | 33 | 0 |

Fix: `audio_pump_mode` (default 1). Mode 1 renders
`ceil(44100 · delta) + 256` frames, clamped to the ring's free space and a
hard ceiling, so production never falls below the steady-state consumption
rate while a single pump stays bounded. Per-pump cost is higher in debug at
low FPS (~18.6 ms at 30 FPS, ~27.8 ms at 20 FPS; ~2 µs/sample) because the
required DSP work is proportional to samples consumed — the cap never removed
that work, it only underproduced. Mode 0 is retained for diagnostics.

Stalls (windowed, buffer 0.1 s): 60 ms hitch → 0 new skips in both modes;
150 ms hitch → 6/14 skips (exceeds the ring; only a larger buffer/latency would
absorb it). Runtime gate: `enable_audio=false` stops the pump (0 calls/60
frames), re-enabling resumes it (60 calls/60 frames) — `toggle check: PASS`.

Limitations / open: only the **debug** binary was rebuilt here; release audio
was measured through the headless bench, not the full runtime. Issue B (visual
pose cache omitting driveshaft spin) is not addressed in this section. No
human listening verdict has been recorded.

### 8.4 Dedicated-core audio worker (phases 1–2)

The delta budget fixed underproduction but left the required DSP on the render
thread, where in debug it is a fixed ~42 % of one core and still scales the
frame (lower FPS -> larger delta -> more samples per pump). The worker removes
that coupling: the mixer moves to its own OS thread.

- **Design.** Rust `AudioWorker` (`game/crates/formula90-core/src/audio_worker.rs`)
  exclusively owns the `AudioModule`. Telemetry packets and trigger/ambient/reset
  commands cross threads in bounded mutex queues (in order — gear one-shots can
  never be skipped); PCM crosses through a lock-free SPSC ring (worker writes,
  host drains); counters are atomics. `F90Core` creates the OS thread, joins it
  in `_exit_tree`/`unload_dll`, and `_process` only drains the ring and issues one
  batched `push_buffer` (no DSP). `audio_worker_enabled=false` falls back to the
  unchanged inline pump (`audio_pump_mode`).
- **Thread policy.** Affinity to the resolved logical processor (auto prefers a
  P-core's highest index; `audio_worker_core` overrides), above-normal priority
  (`audio_worker_priority`) and MMCSS "Pro Audio" via dynamically loaded
  avrt.dll. Note: the process-mask reservation from the plan is not implementable
  as written — Windows intersects process and thread affinity, so excluding the
  core from the process mask would also evict the worker. Isolation is therefore
  pin + priority, not hard exclusion.
- **Correctness.** PCM parity test: 400 telemetry ticks rendered through the
  worker are bit-identical to the inline pump; ring wrap, command survival and an
  end-to-end FFI lifecycle (real thread, pull>0, healthy, 0 drops, clean join)
  are covered by `cargo test -p formula90_core`. Diagnostic switches
  `--no-audio-worker` (and the fixed init-time ordering of `--no-audio`) were
  added to both windowed captures.

Debug, 400 frames accelerating (`--throttle=0.7`, geometry ON):

| config | fps | frame p50 | process p50 | fps buckets |
|---|---|---|---|---|
| inline pump (worker off) | 27 | 39.3 ms | 33.0 ms | 28.4 → 27.5/18.5 (unstable) |
| worker on | 151 | 7.1 ms | 6.5 ms | 127 → 150 (flat) |
| no audio | 138 | 7.3 ms | 7.7 ms | 127 → 150 (flat) |

Worker ON sustained sweep (WASAPI, 44.1 kHz): 44046–44113 samples/s with **0 new
ring skips, 0 starved iterations, 0 dropped packets** at native/30/20 FPS caps;
core 3 (affinity mask 0x8); worker CPU 62–65 % of one core in debug. Stalls:
60 ms and 150 ms absorbed with 0 new skips; 300 ms exceeds the 0.1 s generator
buffer (44 skips). Runtime gate: on=53, off=0, re-enabled=36 (PASS).

Release (QA runner `scripts/run_release_runtime.ps1`, which swaps the
`windows.editor.*` entries to the template_release extension and restores the
file hash-verified): worker on 119.7 FPS at **16 %** of one core, 0 skips;
worker off 114.2 FPS with 1.66–7.8 ms/pump inline (release DSP is ~4x cheaper,
so the inline fallback is viable there). Debug and release are reported
separately; no blended numbers.

Commits: `2364dbbe` (worker+FFI), `ceccfbd2` (host thread), `8c7054c9`
(publish), `1c8520e1` (captures), `b202b187` (release QA). Pending: human
listening gate; Issue B still open.

### 8.5 Regression found by listening: stale sibling DLL silenced the runtime

After the release QA experiment the user reported "no audio". Objective
evidence: the windowed probe showed `pump/s=0`, `max_avail=0` and (after adding
the meter) `rms=0.0000` — the pump never ran at all, in **both** worker and
inline modes, so this was not a worker bug.

Root cause: the release build left
`formula90_core.windows.template_release.x86_64.dll` (stamp `1c8520e1`) in
`bin`. The loader prefers `template_release`; it loaded that stale file, its
BUILD no longer matched `res://BUILD_SOURCE` (`d4632fc3`) and `load_dll` fataled
on the **first** candidate, so the facade never came up.

Fix (`7e06cffe`, published `35c34862`):

- `load_dll` validates ABI + BUILD **per candidate** and skips mismatched ones
  (`[F90Core] skipped stale candidate: ... (ABI=.. BUILD=..)`), fataling only
  when no candidate matches. Verified by planting a `c755b8d3` release sibling:
  it is skipped and the current debug DLL loads with `rms=0.07`.
- `run_release_runtime.ps1` removes the QA-only `*template_release*` artifacts
  and restores the tracked debug publish + `BUILD_SOURCE` to HEAD after every
  run, so a release experiment can no longer leave stale shadows.
- New diagnostics: `push_audio_batch()` records the pushed-block RMS and
  `push_buffer` rejections (`get_audio_output_rms`, `audio_push_rejections`,
  mirrored in `audio_worker_stats` and printed by `audio_sustained_capture`).

Healthy windowed baseline after the fix (worker on, core 3): `rms≈0.07`,
0 push rejections, 0 ring skips, produced/s ≈ 44.1 kHz. The lesson matches the
AGENTS rebuild rule: never mix configurations' artifacts; prefer/validate per
candidate instead of trusting the first file that loads.
