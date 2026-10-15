# Fuji suspension comparison with twelve contact rays

The suspension candidate runs in the existing Fuji session with one player vehicle at 120 Hz. Human driving approval and the later 22-car Rust performance comparison remain pending.

## Run

After rebuilding the current sources, run each session separately:

```powershell
./scripts/run_fuji_suspension_comparison.ps1 -Backend simplified
./scripts/run_fuji_suspension_comparison.ps1 -Backend standard
```

The wrapper delegates to `scripts/run_f1_94.ps1`, preserving asset validation and BUILD/HEAD checks. Both commands retain simplified aerodynamics so that only suspension changes. Use `-AerodynamicsBackend standard` on both commands to compare suspension with the original aerodynamics instead. Normal canonical launches still default to standard suspension and standard aerodynamics. The optional coupled physical world rejects simplified suspension rather than mixing integration owners.

Each launch creates `scratch/suspension_comparison_<backend>_<timestamp>/` with metadata, `physics_samples.csv` and a closing `session_summary.json`. The overlay identifies both backends. Timing capture excludes the first ten simulated seconds. Close normally to finish the summary. No additional cars, artificial driver, altered input mapping or driving assistance are introduced.

## What changes

`suspension.rs` retains the standard suspension mechanics. Its separate child module, `simplified_suspension.rs`, prepares 129 travel samples per corner and 33 steering samples between -0.8 and +0.8 radians. These are numerical interpolation settings, not replacement vehicle tuning. Tables are built once from the current physical hardpoints and shared immutably by cloned states. Configuration replacement invalidates both geometric caches; deserialized states rebuild their skipped caches before stepping.

In B, interpolation replaces repeated linkage solves and motion-ratio Jacobians. Prepared linear link-force responses replace per-tick matrix elimination. Force balance, anchor reactions, moments about the centre of gravity, unsprung weight/inertia and load-path diagnostics are still evaluated from the current loads. The response remains dependent on travel and steering, retaining the geometric effects of the authored suspension rather than applying every force at the tire contact patch.

Both modes keep all twelve wheel raycasts, their 1:2:1 contact weighting, partial contact, effective contact normal, surface filtering, pressure/temperature-dependent radial tire mechanics, independent unsprung movement, spring preload, asymmetric slow/fast damping, anti-roll bars, bump/droop stops, mechanical substeps, camber/toe variation and hub offsets. Presentation retains the existing validated geometry tables.

The approximation is interpolation between prepared geometry and force-response samples. Invalid or singular table samples, unavailable geometry, nonfinite or out-of-range travel and steering use the original calculation. Interpolated wrenches also fall back when the measured force-balance residual exceeds 1 N or the equilibrium check reports a singular response. Diagnostic residuals are calculated from the reconstructed reactions, not interpolated from nominal zero-error samples.

Kinematic residual telemetry in B carries the worse of the two source-node residual sets. It describes the prepared solver samples, not a new full constraint audit of the interpolated pose. Off-grid geometry accuracy is checked separately by the candidate tests.

The B backend is serialized with suspension state and selected at creation through `FORMULA90_SUSPENSION_BACKEND`; a process runs one selected backend. Runtime logs emit `SUSPENSION_BACKEND_READY backend=simplified contact_rays=12 prepared=true` after constructing the B tables.

## Automated evidence

The suspension unit suites passed 27 tests, including three new candidate tests. Seven relevant integration suites passed 39 tests. These validate the standard reference, kinematics, elements, tire coupling, chassis reactions and geometric profile parity.

The candidate test evaluates more than 1,000 off-grid travel states against direct geometry, spring and damping calculations. A separate grounded/airborne wrench sweep evaluated 200 cases with nonzero body rotation, translated body coordinates, steering, tire forces, aligning torque and unsprung acceleration. Maximum differences were 1.793 N in force and 0.374 Nm in torque; 187 cases satisfied the interpolation acceptance gate, with the remaining cases delegated to the standard runtime path.

A ten-second, 120 Hz vertical dynamics comparison includes unequal transverse ray distances, changing road height and staggered losses of contact. Both modes preserve identical ray validity and contact fractions. The maximum normal-force difference was 0.080 N. Direct linkage calls in the vertical solver dropped from 28,800 to 714 for that test. This is an operation count, not a measured CPU improvement, and excludes the table preparation cost and chassis-wrench path.

Evidence logs are under `scratch/suspension_single_player_validation/`. Runtime smoke checks confirm a single vehicle and twelve enabled wheel rays; driving feel, rendered geometry and real performance require the player's test.

## Later analysis

Once both driving sessions have been recorded:

```powershell
python tools/physics_diagnostics/compare_aerodynamics_sessions.py --system suspension <standard-directory> <simplified-directory>
```

The analyzer requires matching source, source-file hashes, binaries, physical profile, processor, frequency, retained aerodynamics and contact resolution. It requires at least 120 measured ticks and driving during at least 90% of them. Rust timing covers the complete facade step, not suspension alone. Human-driven trajectories differ; the later controlled Rust comparison with 22 cars remains a separate task.
