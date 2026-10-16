# Tire simplification experiment

The candidate removes post-peak force memory, cross-axis peak migration, asymmetric loss/recovery delays and the contact force ramp. It retains one symmetric distance-based slip relaxation because removing that final relaxation caused sustained wheel-feedback oscillation at the intended 120 Hz. The selected candidate is `TireBackend::SimplifiedWithRelaxation`; the no-relaxation variant remains available for reproducible ablation only.

## Scope and source

- Date: 2026-10-15, America/Bogota; branch `main-clean`; source base `432ffd989d0645830daad4249434f30ee2d6e511` plus pending tire changes and the pre-existing suspension benchmark instrumentation.
- Rust implementation: `game/crates/vehicle-physics-engine/src/tire.rs`.
- Tests: `game/crates/vehicle-physics-engine/tests/tire_simplification_test.rs`.
- Experiment runner: `game/crates/vehicle-physics-engine/examples/compare_tire_grid.rs`.
- The real modular geometric F1 2030 physical profile is loaded from its authorized repository root. No physical profile, tire pressure/thermal/wear configuration, hardpoint, visual asset, native launcher or default backend was changed.
- `Standard` remains the direct Rust API and legacy deserialization default; by explicit user request the canonical launcher defaults to simplified aero, suspension and tires. The canonical launcher now accepts `-TireBackend standard|simplified`; simplified maps only to the validated single-relaxation candidate. The instantaneous ablation variant remains accessible through the Rust API only.

## Retained and removed behavior

Both candidate variants preserve pure longitudinal/lateral pre-peak curves, the smooth post-peak decline and sliding plateau, one shared combined-slip friction budget with its existing smoothing band, vertical-load sensitivity, mechanical/pressure/thermal/wear modifiers, camber thrust, pneumatic-trail self-aligning torque and wheel torque/brake mechanics.

Post-peak coefficients are evaluated directly from the current effective slips. Existing decay telemetry fields report the instantaneous envelope decay and do not feed a stored sliding-memory state in the candidate. Effective peaks remain the configured per-axis peaks; opposite-axis slip no longer migrates them. Combined forces can therefore differ from Standard by design even at steady state, while the pure-axis envelopes are preserved.

The candidate bypasses loss/recovery time constants, the minimum relaxation time, the recovery multiplier and the separate contact ramp. It clears effective slips immediately when unloaded and returns zero tire forces before consuming fictitious zero body velocity. A new contact is evaluated from its measured load and contact fraction without an extra time filter. Suspension contact sampling and surface filtering, brake-lock regularization and pressure/temperature/wear evolution remain separate existing mechanics.

The retained relaxation uses the existing mechanically derived relaxation length, contact-patch and pressure modifiers. Its time scale is length divided by planar speed (with the existing low-speed regularization), and the same exponential fraction advances both slip axes. It adds no post-peak memory, minimum-time floor or separate loss/recovery lag. Progressive transition in slip space remains present even in the rejected instantaneous variant.

## Relaxation necessity test

The controlled probe holds chassis speed constant and starts the front-left wheel 1 percent above rolling speed. It applies zero drive torque, zero brake torque and zero rolling resistance; the actual wheel inertia responds to tire reaction torque. It runs for two simulated seconds and evaluates the final second. Speeds are 5, 20 and 60 m/s; loads are 0.5, 1 and 2 times the profile-derived static wheel load; frequencies are 120, 240 and 480 Hz. Each backend therefore has 27 cases, for 81 cases total.

The acceptance criterion was declared before execution: final-second slip root mean square must stay below 0.01 in every 120 Hz case. Prefer no relaxation if it passes; retain one relaxation only if it resolves the instantaneous failures. This tests numerical necessity within the existing explicit wheel-feedback integration, not an independently measured real-tire time constant.

| Backend | Failed / 9 at 120 Hz | Maximum final-second slip RMS at 120 Hz | Failed at 240 Hz | Failed at 480 Hz |
|---|---:|---:|---:|---:|
| Standard | 0 | 0.000041599 | 0 | 0 |
| SimplifiedWithoutRelaxation | 5 | 0.442487470 | 4 | 2 |
| SimplifiedWithRelaxation | 0 | 0.000037853 | 0 | 0 |

Without relaxation, five 120 Hz cases fail and the worst final-second slip RMS is 44.249 percent, with repeated sign reversals. One symmetric relaxation passes all tested cases at all three frequencies; its worst 120 Hz slip RMS is 0.003785 percent. This is the measured reason for retaining that single transient layer. It does not justify retaining the removed hysteresis layers.

## Controlled 22-car timings

The runner evaluates 22 independent vehicles at fixed 120 Hz. Standard tires, no-relaxation tires and single-relaxation tires share simplified aero and simplified suspension. Five paired repetitions use forward/reversed backend order, 3 simulated warmup seconds and 12 measured seconds. Release compilation uses workspace optimization level 3 and LTO. Execution is pinned to logical processor 2, affinity mask 4, on an AMD Ryzen 5 5500. No real-time sleeping is used.

Isolated tire stepping prescribes identical load, slip, camber and neutral mechanical modifiers in every backend; pressure/thermal/wear state is held fixed there. Complete workloads start from equal physical configuration and internal state but evolve temperature/pressure/wear as a consequence of their forces. Stability assistance, traction control, ABS and steering assistance are disabled; automatic clutch is common to every mode. ForceReplay prescribes equal body motion and driver commands; free integration uses the same speed-dependent driver policy, so trajectories and gear requests can diverge.

All times below are per batch of 22 vehicles. Percentage reduction compares the selected single-relaxation candidate with Standard tires in this same experiment.

| Workload | Standard mean, ms | Single-relaxation mean, ms | Mean reduction | Paired 95% reduction interval |
|---|---:|---:|---:|---|
| TiresOnly | 0.020118 | 0.015825 | 21.337% | [14.742%, 27.437%] |
| ForceReplay | 0.198245 | 0.201075 | -1.428% | [-9.791%, 7.032%] |
| StandaloneFlatRoad | 0.210910 | 0.202941 | 3.779% | [-3.479%, 10.722%] |

The isolated tire cost falls by 21.337 percent. Both complete-workload intervals include zero: no whole-simulation timing gain or regression is demonstrated for the selected candidate. The rejected no-relaxation mode has a larger isolated saving, but that does not override its wheel-feedback failures. The approximate paired t intervals use five independent repetitions and assume approximately normal paired percentage differences. Raw outliers remain in the evidence.

All 45 timed runs have finite outputs and repeat their workload checksum exactly across five repetitions. Both complete workloads retain contact on every measured wheel tick and pass the declared motion gates (tilt below 45 degrees, positive height, contact fraction above 0.9). The checksums cover isolated tire forces/aligning torque or complete speed/downforce/tire force sums; they are not complete serialized-state determinism proofs. There are 1,425,600 measured vehicle updates plus warmup and the separate wheel-feedback probes.

Timing excludes prescribed-motion preparation, numerical trace collection, diagnostic aggregation and file output. Simulator construction/backend preparation is recorded separately in each run and includes the common simplified suspension tables; it is not a tire-only startup measurement.

This analytic flat-road runner does not include Fuji collision geometry, car-to-car interaction, AI, the native facade, Godot raycast execution, audio or rendering. The numerical contact/stability gates do not establish identical handling or rendered behavior. No game frame-rate gain is claimed.

## Automated validation and pending gates

- 54 standard tire-related unit tests pass; the same 54 passed before the tire edit.
- 47 existing integration tests pass across seven suites covering tire baseline, transients, thermal/wear mechanics, suspension coupling and coupled tire hysteresis.
- 10 new candidate tests pass: pure-curve steady envelopes across axle/load/sign cases; absence of instantaneous post-peak history; combined budget and fixed peaks; load sensitivity, camber and aligning torque; peak continuity and sliding plateau; unload/landing; symmetric frequency-consistent single relaxation; serialization/default compatibility; free-wheel settling; brake lock/release recovery.
- The candidate remains experimental. The user requested commit and native rebuild for a canonical one-player drive. The launcher rejects simplified tires with the optional coupled world. Startup smoke and binary parity are recorded separately under `scratch/tire_native_rebuild/`; human driving feedback remains pending.
- Native binaries, `BUILD_SOURCE`, previous suspension work and unrelated dirty changes were preserved. A standalone Cargo example build is not a native runtime rebuild.
- Tire source, tests, runner, launcher and this report are included in the requested source commit. Generated files and unrelated pending work are excluded. No push was requested.

## Reproduction and evidence

```powershell
cargo test --manifest-path game/crates/Cargo.toml -p vehicle_physics_engine --test tire_simplification_test
cargo build --release --manifest-path game/crates/Cargo.toml -p vehicle_physics_engine --example compare_tire_grid
game/crates/target/release/examples/compare_tire_grid.exe D:/Formula90s <fresh-output-directory> 12 5 3
```

Local evidence: `scratch/tire_grid_22_120_hertz_20261015_220219/`. It contains the report and summary JSON, grid timing CSV, first-repetition numerical snapshots, the 81-case relaxation ablation, launch/analysis scripts, source snapshot and patch, build/test logs, metadata and post-run provenance verification.

Executable SHA-256: `3EAF3853C8F906455C8D2AF9073D84AF2D4B13F756F776205346332DBB793288`. Source/profile hashes and the measured executable were rechecked after execution with no mismatch. Base HEAD, branch, dirty status, resolved configuration, toolchain and affinity are recorded in `execution_metadata.json`. The complete pre-edit dirty diff and status were retained under `scratch/tire_simplification_validation/`.

## Canonical player test

```powershell
./scripts/run_f1_94.ps1 -TireBackend simplified
```

The canonical launcher now defaults all three backends to simplified. The tire flag is optional. An explicit equivalent command is:

```powershell
./scripts/run_f1_94.ps1 -TireBackend simplified -AerodynamicsBackend simplified -SuspensionBackend simplified
```

`-SmokeTires -TireBackend simplified` performs the canonical headless startup check. Confirm the Rust marker `TIRE_BACKEND_READY backend=simplified relaxation=single_distance_based` as well as the smoke PASS marker. The public launcher deliberately excludes the rejected instantaneous tire mode.

The user subsequently requested simplified aero, suspension and tires as the normal launcher defaults. For a complete reference comparison, pass `-AerodynamicsBackend standard -SuspensionBackend standard -TireBackend standard`. The optional physical-world package requires those standard selections explicitly.
