# Rust aerodynamics comparison: 22 vehicles at 120 Hz

The simplified aerodynamic solver reduces its isolated computation time, but this benchmark does not demonstrate a reduction in total vehicle simulation time outside measurement variation. This result does not establish an in-game frame-rate improvement.

## Recorded experiment

- Date: 2026-10-15.
- Source base: `ae325a613e621ccad23475393e87a24b4abbcca5`, on `main-clean`, plus the standalone aerodynamic-environment entry point and benchmark example committed with this document.
- Profile: `game/data/vehicles/f1_2030/profiles/geometric/vehicle.json`, resolved from the real modular documents.
- Processor: AMD Ryzen 5 5500; one execution thread pinned to logical processor 2 using affinity mask 4.
- Compilation: Cargo Release, optimization level 3, with workspace LTO enabled.
- Each workload advances a batch of 22 independent vehicles with a fixed timestep of 1/120 second. The runner does not sleep to enforce real-time pacing.
- Five paired repetitions per backend and workload, alternating standard/simplified execution order between repetitions.
- Each repetition has 3 simulated seconds of warmup followed by 12 simulated seconds of measurement.
- Stability, traction control, ABS and steering assistance are disabled. Automatic clutch remains enabled for both modes.

## Results

All times below are for the whole batch of 22 vehicles. Positive reduction means the simplified mode took less time.

| Workload | Standard mean, ms | Simplified mean, ms | Mean reduction | Standard p95, ms | Simplified p95, ms |
|---|---:|---:|---:|---:|---:|
| Aerodynamic solver only | 0.016692 | 0.006648 | 60.169% | 0.024200 | 0.010200 |
| Complete force solve with prescribed identical motion | 7.313269 | 7.259037 | 0.742% | 7.718615 | 7.659305 |
| Complete standalone simulation with free chassis integration | 7.280935 | 7.328594 | -0.655% | 7.715305 | 7.735630 |

The approximate 95% paired t intervals for mean percentage reductions are respectively `[51.342%, 67.765%]`, `[-1.600%, 3.032%]` and `[-2.362%, 1.068%]`. These intervals assume approximately normal paired differences and contain only five independent pairs. Both complete-simulation intervals include zero; their small point estimates should not be described as demonstrated gains or regressions.

The 120 Hz interval is 8.333333 ms. This experiment excludes several runtime costs, so the remaining interval is not a complete game budget.

## Numerical checks and boundaries

There were 950,400 measured vehicle updates across the 30 runs, plus warmup. Each backend/workload reproduced its numerical checksum exactly across all five repetitions. All finite-output and flat-road motion gates passed. Free integration retained wheel contact in every measured wheel tick, and maximum chassis tilt was approximately 0.710 degrees.

`AerodynamicsOnly` excludes contact sampling. `ForceReplay` supplies identical deterministic body trajectories to both backends while evolving their internal vehicle systems. `StandaloneFlatRoad` also integrates the chassis through the existing Rust standalone integrator, allowing trajectories to differ as a consequence of aerodynamic forces. Wheel and floor samples use analytic ray/plane intersections; the simplified path queries its two floor probes while the standard path queries five.

Timing excludes trace preparation, diagnostic aggregation and file output. The experiment does not include Fuji geometry, vehicle-to-vehicle collisions, racecraft, audio, native facade work, Godot raycast queries, scrape/contact fusion or rendering. It validates this controlled computational comparison, not a full race or human driving acceptance.

## Reproduction and provenance

```powershell
cargo run --release --manifest-path game/crates/Cargo.toml -p vehicle_physics_engine --example compare_aerodynamics_grid -- D:/Formula90s <fresh-output-directory> 12 5 3
```

For the same scheduling setup, set the runner process affinity to mask 4. The arguments after the output directory are measured seconds, paired repetitions and warmup seconds. Vehicle count and simulation frequency are fixed at 22 and 120 Hz in the runner.

Original evidence is retained locally under `scratch/aerodynamics_grid_22_120_hertz_20261015_162858/`: `report.json`, `summary.json`, `grid_tick_timings.csv`, `execution_metadata.json`, `execution.log`, `runner_source.rs` and `standalone_entry_point.patch`. These generated captures are excluded from the source commit. The report includes the resolved physical configuration and all repetition results.

The recorded executable SHA-256 is `58D3C447E1F0D7EF87EDD7BF1664BE6153F3C3B46D771FC8848B7B755F1BC71C`. The benchmark example SHA-256 is `57A3E118A0F49129A13C3CED4A5B9DF11519173F059526F75833F737F8241B10`. The simulation source SHA-256 is `22F930E9DEFE8004F46B44EDCB2B81802D8E5AC9EB06902499673E829B598F57`. These were rechecked against the measured executable and source files after the run.

The earlier single-player A/B candidate, its validation boundaries and canonical launcher commands are documented in `docs/aerodynamics_comparison_single_player.md`. A new source commit changes HEAD and requires the canonical BUILD/HEAD rebuild before the next game launch.
