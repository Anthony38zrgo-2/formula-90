# Fuji aerodynamics comparison with one player vehicle

This candidate keeps the existing `vehicle_test_session.tscn` composition, player inputs, cameras, HUD, audio, physical profile, tires, suspension and powertrain. There is no artificial driver or additional vehicle. Human driving acceptance and performance results remain pending.

## Run

From the repository root, after rebuilding the current source:

```powershell
./scripts/run_fuji_aerodynamics_comparison.ps1 -Backend simplified
./scripts/run_fuji_aerodynamics_comparison.ps1 -Backend standard
```

Run each session separately and close the game normally to finish its capture. The wrapper delegates to `scripts/run_f1_94.ps1`; all asset, profile and BUILD/HEAD checks remain active. The scene remains the same in both modes. Normal launches default to `standard`. An optional `-MaximumSeconds` ends a capture automatically; its default is an interactive session without a time limit.

Each capture gets a fresh directory under `scratch/aerodynamics_comparison_<backend>_<timestamp>/`, containing run metadata, `physics_samples.csv` and a closing `session_summary.json`. The overlay identifies the selected backend. The first ten simulated seconds are excluded from timing samples.

## Backend separation

`aero.rs` retains the standard solver. `simplified_aerodynamics.rs` contains the alternative solver. `VehicleSimulator` selects exactly one function when computing aerodynamic forces. The process selects its mode before vehicle creation using `FORMULA90_AERODYNAMICS_BACKEND`; it does not run both solvers or hot-switch an existing car.

The simplified solver uses one common air-relative flow calculation, instantaneous front and rear wing polar coefficients, and one floor/diffuser force with a longitudinal center of pressure. Existing load limits, density, wind, aerodynamic setup, floor map and body/wheel drag coefficients remain authoritative. It removes separate wing response states, floor attachment hysteresis, pressure lag and repeated airflow evaluations at each element and wheel. Wheel drag is aggregated at the center of mass; rotational airflow and individual wheel drag moments are approximated. Ground-map pressure shift remains active.

Only the front-center probe at `(0, -0.205, -0.90)` and diffuser-throat probe at `(0, -0.165, 0.85)` are enabled and explicitly queried in simplified mode. The other three probes are disabled and are not force-updated. The twelve wheel probes remain unchanged. The floor model removes the fixed 40 millimeter probe-origin offset before deriving road-relative heights and rake. If either required measurement is unavailable, it removes the ground-effect increment and retains only the configured free-air floor contribution; it does not invent nominal ground measurements.

Rigid floor contacts, scrape audio and supplemental bottoming remain active. The contact solver uses the front-center position rather than the former left-front lever. With two longitudinal probes, lateral floor height is unavailable and reported roll is zero. The fixed legacy telemetry slot zero carries the front-center sample in simplified mode; `get_underfloor_state_snapshot` explicitly reports the probe layout and this alias. Missing probes keep their validity bits clear. No extra measured lateral samples are fabricated.

## What the measurements mean

Native contact timing covers explicit wheel/floor sample collection and raw contact collection. Rust timing covers the entire facade step, including all physical systems and module work; it is not an isolated aerodynamic timer. Godot physics/process monitors are recorded separately and can repeat across physics ticks within one rendered frame. These monitors are elapsed durations, not operating-system CPU-utilization percentages.

Use matching build binaries, physics frequency, graphics settings, assists, weather and comparable routes. Repeat each mode several times, alternating order. Human-controlled sessions do not reproduce identical inputs or trajectories, and a single vehicle does not establish full-grid savings.

```powershell
python tools/physics_diagnostics/compare_aerodynamics_sessions.py <standard-directory> <simplified-directory>
```

The analyzer reports p50/p95/p99 and relative reductions, rejects incompatible source/processor/frequency metadata, requires at least 120 recorded ticks, and requires the player to be moving in at least 90% of samples. It rejects samples where the player simulation did not advance.

## Validation boundaries

The new Rust aerodynamic tests cover equilibrium load/drag/balance against standard, absent measurements, wind, nonfinite inputs and timestep independence. Core underfloor tests cover centered bottoming torque and rake with the two-probe layout, alongside existing contact and scrape tests.

The original `aero_test` suite has four failures reproduced against the pre-candidate source at `e4418957`: fixed lag expectation, reverse drag, historical balance definition, and an unconstrained quadratic-scaling expectation. The baseline log is under `scratch/aerodynamics_single_player_validation/standard_baseline_aero_tests.log`. These failures are not silently changed or treated as passing.

```powershell
./scripts/run_f1_94.ps1 -SmokeAerodynamics -AerodynamicsBackend standard
./scripts/run_f1_94.ps1 -SmokeAerodynamics -AerodynamicsBackend simplified
```

This dedicated headless smoke loads the same Fuji session and checks one vehicle, probe count, centered position and layout metadata. It does not certify driving feel, visuals, lap times or measured performance improvement. Player driving approval must precede any conclusion about the new model.
