# Rust suspension comparison: 22 vehicles at 120 Hz

The simplified suspension reduces steady-state Rust computation in this controlled flat-road benchmark. It also increases preparation time, and free integration exhibits transient numerical divergence. These results do not establish in-game frame-rate improvement or exact handling equivalence.

## Experiment

- Date: 2026-10-15, America/Bogota.
- Branch: `main-clean`; source base: `432ffd989d0645830daad4249434f30ee2d6e511`, plus the pending benchmark runner and opt-in diagnostic counters.
- Runner: `game/crates/vehicle-physics-engine/examples/compare_suspension_grid.rs`.
- Actual modular geometric F1 2030 profile, 22 independent simulators, fixed 120 Hz, twelve wheel rays per vehicle. Simplified aerodynamics is identical in A and B.
- AMD Ryzen 5 5500; one thread pinned to logical processor 2 with affinity mask 4. Cargo Release, optimization level 3 and workspace LTO.
- Five paired repetitions for each workload, alternating execution order. Each run warms up for 3 simulated seconds and measures 12 seconds without real-time sleeping.
- Only automatic clutch is enabled. The same driver policy is used in both modes. Free integration derives gear requests from each trajectory, so commands can diverge; ForceReplay supplies identical prescribed motion and commands.
- Timed runs disable coverage counters. Three additional simplified runs collect coverage separately, including warmup. Numerical trace collection and serialization occur outside the timed region.
- Acceptance was declared before execution: finite outputs; flat-road tilt below 45 degrees, positive height and contact fraction above 0.9; repeated workload checksums; paired timing interval excluding zero before claiming a gain. No exact-trajectory equivalence threshold was declared.

## Timings

All times are for a batch of 22 vehicles. The p95 column is the mean of five per-run p95 values. Positive reduction means B is faster.

| Workload | Standard mean, ms | Simplified mean, ms | Reduction | Standard p95, ms | Simplified p95, ms |
|---|---:|---:|---:|---:|---:|
| Vertical suspension and analytic wheel sampling | 5.767308 | 0.044662 | 99.226% | 6.028808 | 0.045984 |
| Complete force solve, prescribed motion | 7.001246 | 0.194075 | 97.228% | 7.493627 | 0.204805 |
| Complete standalone simulation, free integration | 7.059052 | 0.200061 | 97.166% | 7.775781 | 0.232909 |

Approximate 95% paired t intervals for percentage reductions, using five pairs:

- Vertical suspension and analytic wheel sampling: [99.209%, 99.242%].
- Complete force solve, prescribed motion: [97.089%, 97.361%].
- Complete standalone simulation, free integration: [96.910%, 97.403%].

All three intervals exclude zero. The small sample assumes approximately normal paired percentage differences; operating-system outliers remain in the raw data. These intervals do not describe variation across tracks, hardware or driving scenarios.

Vertical suspension includes the existing contact aggregation, unsprung dynamics, springs, damping, anti-roll bars and stops. It excludes chassis-wrench transmission, tires/powertrain stepping and chassis integration; it is not an isolated timing of every suspension operation. Complete workloads include suspension wrench evaluation.

## Preparation and numerical behavior

| Workload | Standard preparation, ms | Simplified preparation, ms | Maximum normal-load difference, N | Maximum travel difference, mm |
|---|---:|---:|---:|---:|
| Vertical suspension and analytic wheel sampling | 125.676 | 2872.356 | 0.024569 | 0.000131 |
| Complete force solve, prescribed motion | 127.079 | 2835.291 | 0.022571 | 0.000162 |
| Complete standalone simulation, free integration | 127.015 | 2804.799 | 488.876089 | 2.329371 |

Preparation measures construction of 22 independent simulators and explicit backend selection, including the standard construction and subsequent cache rebuild performed by the runner. It excludes profile loading. No cross-vehicle cache sharing is introduced. Do not describe these values as minimum possible startup cost.

All timed runs had finite outputs. Both complete workloads retained contact on every measured wheel tick. Free integration reached maximum chassis tilt of approximately 0.704 degrees and minimum body height above 0.272 m; every motion gate passed. There were 950,400 measured vehicle updates in timed runs, plus warmup and separate coverage runs.

Free integration has maximum paired differences of 0.000704699 m in body height, 0.421954 m/s in speed, 488.876089 N in individual normal load and 0.002329371 m in suspension compression. The maximum normal-load difference occurred at 14.675 simulated seconds: rear-right load was 4102.209982 N in A and 4591.086071 N in B. These are corresponding-time differences along independently evolved trajectories, not errors from evaluating an identical instantaneous state. Their cause was not isolated by this benchmark; do not label them harmless, a confirmed defect, or exact equivalence.

The earlier player reported no defects and explicitly authorized this comparison. That closes the prerequisite player-feedback gate; it does not authorize default-backend promotion. The transient free-integration difference remains a review point before any claim of strict behavior parity.

Every workload/backend reproduced its checksum across five repetitions. Vertical checksums sum normal loads; complete-workload checksums sum speed and aerodynamic downforce, so ForceReplay checksum equality does not independently prove complete suspension-state repeatability. First-repetition traces compare all four normal loads and suspension compressions at every measured tick.

Each coverage run recorded 158,400 tick-start geometry evaluations, all interpolated. Both complete coverage workloads also recorded 158,400 wheel-wrench evaluations, all accepted through interpolation, with zero dispatcher fallbacks in this scenario. The vertical-only workload makes no wrench calls. Counters do not cover the final geometry/element evaluation separately; these percentages are not a universal fallback rate. Instrumented coverage checksums matched their uninstrumented counterparts.

## Evidence and reproduction

Local evidence directory: `scratch/suspension_grid_22_120_hertz_20261015_204653/`. It contains `report.json`, `summary.json`, `grid_tick_timings.csv`, first-repetition numerical traces, `execution_metadata.json`, `provenance_verification.json` and a source snapshot. Build, test and launch logs are also retained in `scratch/suspension_grid_preparation_20261015/`. The summary script and exact launch script are copied into the evidence directory.

Measured executable SHA-256: `E4F3E236A40F0F87999CE0F1F6B7F003501988965482B8ECBB03C725C5CFB436`. All recorded source/profile hashes and the executable were verified unchanged after execution. Metadata records the source base, dirty status, resolved configuration and toolchain. The installed Godot libraries and `BUILD_SOURCE` were not rebuilt by this standalone experiment.

```powershell
cargo build --release --manifest-path game/crates/Cargo.toml -p vehicle_physics_engine --example compare_suspension_grid
game/crates/target/release/examples/compare_suspension_grid.exe D:/Formula90s <fresh-output-directory> 12 5 3
```

Set process affinity to mask 4 for the recorded scheduling arrangement. The arguments are measured seconds, paired repetitions and warmup seconds. Initialization and output are excluded from steady-state timings.

The analytic flat road includes no Fuji collision geometry, car-to-car collisions, AI, native facade, Godot raycast execution, audio or rendering. It does not establish full-race performance. Native BUILD/HEAD parity remains required before the next canonical game launch.

## Validation and delivery

The 27 suspension unit tests passed with the optional counters present. All 39 integration tests across seven relevant suites also passed; results are retained in `integration_tests.log`. Diagnostic counters are disabled by default and omitted from serialization; the physical algorithms and vehicle configuration remain unchanged. The benchmark source and this report are included in the requested source publication. The direct Rust default remains standard; by explicit user request the canonical launcher now defaults aero, suspension and tires to simplified.
