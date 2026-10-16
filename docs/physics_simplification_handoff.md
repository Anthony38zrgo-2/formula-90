# Handoff: aerodynamics and suspension simplification

## Latest continuation: tire simplification experiment

The user authorized a tire simplification experiment after the suspension comparison. Read `docs/tire_simplification_experiment.md` and the updated backlog before continuing. `tire.rs` now has Standard and two explicit Rust experimental backends. The selected candidate removes post-peak force memory, peak migration, asymmetric loss/recovery delays, minimum relaxation time and contact force ramp while retaining all requested force-envelope mechanics.

One symmetric distance-based slip relaxation is justified by measurement: without it, 5/9 wheel-feedback cases fail at 120 Hz; with it, all tested cases pass at 120/240/480 Hz. The controlled 22-car comparison measures 21.337 percent lower isolated tire cost, but no demonstrated whole-simulation gain for the selected candidate. All 54 reference unit, 47 existing integration and 10 candidate tests passed.

The user requested commit and native rebuild. The tire source commit now includes the canonical `-TireBackend standard|simplified` selection and `-SmokeTires` startup check; simplified maps to the single-relaxation candidate. By explicit user request, the canonical launcher now defaults aero, suspension and tires to simplified; direct Rust defaults remain standard. Rebuild/startup evidence at `3f1194c0` is retained under `scratch/tire_native_rebuild/`: fifteen Rust library copies and three DSP copies matched; simplified/default, canonical Fuji and standard-reference smokes passed. The first font-import crash was resolved by a canonical retry. Publication changes HEAD, so refresh native identity and verify parity before another drive. Human handling feedback for the tire candidate remains pending. Do not claim runtime, visual, strict handling-parity or FPS validation from this experiment. Existing suspension and unrelated dirty work were preserved.


## Current continuation: controlled suspension comparison completed

On 2026-10-15 the player explicitly reported no defects and authorized the 22-car A/B comparison. The player-feedback prerequisite is resolved. The comparison was executed on `main-clean` at source base `432ffd98` plus pending benchmark/counter changes, with 22 cars at 120 Hz, twelve rays and simplified aero held constant.

Read `docs/suspension_rust_grid_benchmark.md` and the updated machine-readable backlog first. Complete standalone simulation mean fell from 7.059052 ms to 0.200061 ms per 22-car tick (97.166 percent); the paired 95 percent reduction interval is [96.910, 97.403] percent. Preparation rose from about 127 ms to 2805 ms for the grid. These are Rust flat-road results, not measured game frame-rate gains.

All finite-state/contact/motion gates passed. Free integration nevertheless showed transient paired differences up to 488.876 N in wheel load, 2.329371 mm in travel and 0.421954 m/s in speed. Prescribed-motion force replay had only 0.022571 N maximum normal-load difference. The cause of free-trajectory divergence is not established; review it before asserting strict handling equivalence or promoting B. No physical tuning was made by the benchmark. The user subsequently requested simplified aero, suspension and tires as the canonical launcher defaults.

The runner, optional coverage counters, report and continuation updates are included in the user-requested source publication. Preserve the dirty checkout and installed binaries. Do not rerun the benchmark merely because the historical instructions below still describe it as deferred. Next steps are result review and any explicitly requested diagnosis or source publication. A canonical game launch still requires native rebuild and source parity.

## Historical handoff snapshot

The remaining sections record the previous delivery state and its original evidence. Pending-player and deferred-benchmark statements below are superseded by the current continuation above.


Updated 2026-10-15. Workspace: `D:/Formula90s`. Continue on `main-clean`; implementation commit: `5192b56e8f9801fa5da23849a8ba02acc9d41c31`. This handoff is committed immediately after it for the requested remote review. Resolve current HEAD again before continuing; the validation build was recorded at base `1baa06d0d4e2fa9f70bb7cd487714b83bd95f41b` with the matching implementation then uncommitted. Read the repository and affected-folder `AGENTS.md` instructions before editing.

## Start here

The current task is to simplify suspension using two selectable A/B backends while preserving all twelve wheel raycasts. Implementation, native rebuild and automated validation are complete. Suspension source changes are **committed in `5192b56e`**; the player and benchmark gates remain open. Explicit player driving feedback is still pending; do not mark B approved, tune it without a reported reason, or run the 22-car suspension benchmark as part of this handoff.

The user required the same branch, a first Fuji test with only the player car, and twelve suspension rays. The later suspension comparison is intended to use Rust only, 22 cars and 120 Hz, following the previous aero experiment. Obtain the player's findings before proceeding to that phase. No user authorization to message another chat or create another task was given.

The machine-readable continuation backlog is `docs/physics_simplification_backlog.json`. It is an independent workstream from the optional coupled Rust world. The historical suspension backlog links here; none of its coupled-world acceptance states have been promoted by this work. `instrucciones.md` currently contains an audio-bank path, not this handoff; preserve that unrelated content.

## Verified current state

- Aero A/B is committed in `ae325a61`; its Rust benchmark and results are committed in `1baa06d0`.
- Suspension B and its launcher, tests and comparison support are committed in `5192b56e`. Normal canonical launches still default to standard aero and suspension.
- The successful player session was launched with simplified suspension and simplified aero, one car and 120 Hz. It has since closed; no Godot process was present at handoff inspection. Do not tell the next user it is still open without checking.
- Capture: `scratch/suspension_comparison_simplified_20261015_172354/`. Its closing summary reports 108 seconds and 11,761 measured ticks. All rows advanced; 9,832 were moving, or 83.5983 percent. The analyzer requires 90 percent, so this capture is not eligible for a performance comparison as a whole. No paired standard-suspension capture has been validated.
- A closed capture does not establish player approval. No explicit driving assessment has been received.

## Implementation map

| File | Responsibility |
|---|---|
| `game/crates/vehicle-physics-engine/src/suspension.rs` | Owns `SuspensionBackend`, prepared-cache lifecycle, shared vertical dynamics and A/B wrench dispatch. Standard remains default. Backend survives serialization; skipped immutable caches rebuild before stepping. |
| `game/crates/vehicle-physics-engine/src/simplified_suspension.rs` | Separate B module: 129 travel samples per corner, 33 steering samples over ±0.8 radians, interpolated poses/motion ratios and prepared linear link-force responses. Includes three candidate tests. |
| `game/crates/vehicle-physics-engine/src/suspension_loads.rs` | Shared anchor reactions and real equilibrium checks; optional prepared response replaces matrix elimination in B. Standard calls retain the original solve. |
| `game/crates/vehicle-physics-engine/src/simulation.rs` | Both grounded and airborne geometric force paths call the selected suspension wrench backend. |
| `scripts/run_f1_94.ps1` | Adds `-SuspensionBackend standard\|simplified`, sets the execution environment, rejects simplified suspension with `-PhysicalWorldPackage`, records source hashes and twelve-ray metadata. Keeps canonical validation. |
| `scripts/run_fuji_suspension_comparison.ps1` | Fuji wrapper, defaults to B suspension and B aero; delegates to the canonical launcher and enables existing timing capture. |
| `game/scripts/runtime/aerodynamics_comparison_session.gd` | Shared capture/overlay now identifies and records both backends. Rust timing measures the whole facade step. |
| `game/tests/smoke_test_aerodynamics_backends.gd` | Existing smoke additionally asserts twelve enabled wheel rays and reports suspension selection. |
| `tools/physics_diagnostics/compare_aerodynamics_sessions.py` | Adds `--system suspension`; validates the selected suspension pair, retained aero and source compatibility. Default aero comparison remains available. |
| `docs/suspension_comparison_single_player.md` | Detailed behavior, limitations, commands and automated evidence. |

B retains three transverse contacts per wheel and their weighting, partial coverage, normals and surface filtering. It preserves pressure/thermal tire mechanics, four unsprung degrees of freedom, preload, asymmetric slow/fast damping, anti-roll bars, stops, substeps, camber/toe and hub offsets. It approximates geometric evaluation and load transmission through interpolation rather than deleting those effects. Existing visual geometry tables remain in use.

Singular/unavailable samples and out-of-range inputs fall back to A. Interpolated chassis wrenches also fall back if force balance exceeds 1 N or the reconstructed equilibrium is flagged singular. Load-path residuals are calculated from actual reconstructed reactions. Kinematic residual telemetry carries source-node solver residuals, not a fresh constraint audit of the interpolated pose; off-grid pose accuracy is tested separately. Do not describe every geometric diagnostic as an exact per-tick solve.

Caches are immutable and shared through `Arc` when state is cloned. Keep that property; deep-cloning the response tables in a per-frame path would defeat the optimization. Configuration replacement must rebuild caches through the existing lifecycle hook.

## Validation and limits

Evidence lives under `scratch/suspension_single_player_validation/`:

- `unit_tests.log`: 27 suspension unit tests passed, including the three new candidate tests.
- `integration_tests.log`: 39 tests across geometric forces, chassis reactions, F1 2030 geometry, tire coupling, kinematics, parity and profile resolution passed.
- Candidate geometry sweeps cover more than 1,000 off-grid states. A 200-case grounded/airborne wrench sweep measured maximum force difference 1.7922 N and torque difference 0.3732 Nm. 187 cases met the B interpolation acceptance gate; the dispatcher uses A for rejected cases.
- Ten seconds of vertical dynamics at 120 Hz with unequal transverse rays and contact loss produced a maximum normal-force difference of 0.0792 N. Direct geometry calls in that vertical-solver test dropped from 28,800 to 714. This is an operation count, **not a CPU saving or complete-simulation benchmark**.
- `build_final_release.log` and `build_debug_bridge.log`: optimized Rust and native bridge builds. DSP Debug/Release checks passed through the canonical builder.
- `installed_binary_hashes.json`: fifteen canonical/template Rust-library copies verified against final release outputs. `source_hashes.json` identifies the nine implementation/control files used for validation. `before_build.json` records provenance; `validation_report.json` summarizes results.
- `smoke_standard.log` and `smoke_simplified.log`: canonical A/B startup checks passed with simplified aero held constant, one car, two floor probes and twelve wheel rays; no runtime error markers were found.
- `interactive_stdout.log` confirms both simplified backends, one car and 120 Hz. `interactive_stderr.log` includes the Rust `prepared=true` markers. `interactive_session.json` records the successful launch and capture directory.
- The first interactive attempt used legacy Windows PowerShell and failed to resolve `Get-FileHash` in the inherited environment. It did not become the validated session. Its details are retained in `interactive_session_failed_legacy_shell.json` and `legacy_shell_error.log`; the successful retry used PowerShell 7.

Godot MCP inspected the project/scene and validated the modified scripts without diagnostics. Its live inspection bridge was not connected. Runtime logs and smoke passes do not certify rendered geometry or driving feel. Do not install a bridge or change normal gameplay configuration merely to claim visual acceptance.

The earlier aero Rust benchmark found about 60.169 percent lower isolated aero cost, but no demonstrated whole-simulation gain outside variation. See `docs/aerodynamics_rust_grid_benchmark.md`; do not transfer that percentage to suspension. Four legacy `aero_test` failures were reproduced before the aero candidate and are documented separately in `docs/aerodynamics_comparison_single_player.md`.

## Resume procedure

1. Record current branch, HEAD and status again. Preserve all dirty work; do not switch, reset, clean, stage everything or overwrite generated/foreign changes. Compare current implementation hashes against `source_hashes.json` before relying on the recorded build.
2. Read the player's findings from the completed Fuji test. If none exist, ask for the driving assessment. If a defect is reported, diagnose it and present a correction plan before changing physical behavior. Do not infer acceptance from session closure or timing data.
3. For an approved repeat drive, use the existing wrapper in PowerShell 7. Hold aero constant when comparing suspension, and keep one player vehicle until the player phase is resolved.
4. When proceeding to the controlled Rust benchmark, extend or adapt `game/crates/vehicle-physics-engine/examples/compare_aerodynamics_grid.rs`. It currently switches **aerodynamics**, not suspension. Explicitly select suspension with `SuspensionSystem::select_backend` for each simulator and hold aero identical. Merely rerunning the existing example does not test suspension A/B.
5. Use 22 cars at 120 Hz, the real resolved profile, equal assists and inputs, alternating paired runs, warmup, release builds and full source/executable provenance. Separate initialization cost, isolated suspension work and full simulation work. Record interpolation/fallback coverage and numerical behavior. The analytic flat-road runner does not include Fuji collision geometry, racing AI, audio, Godot or rendering.
6. Update this backlog with actual evidence and remaining gates. Publish source only when requested/approved, staging explicit paths and reviewing the cached diff. Committing changes HEAD; rebuild/validate source parity before the next game launch.

### Commands

From `D:/Formula90s`, in PowerShell 7, after confirming BUILD/source parity:

```powershell
./scripts/run_fuji_suspension_comparison.ps1 -Backend simplified
./scripts/run_fuji_suspension_comparison.ps1 -Backend standard
```

Run separately. Both keep aero B. To retain aero A instead, pass `-AerodynamicsBackend standard` to both. The successful PowerShell host was `C:/Users/bill/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe`.

```powershell
./scripts/run_f1_94.ps1 -SmokeAerodynamics -AerodynamicsBackend simplified -SuspensionBackend standard
./scripts/run_f1_94.ps1 -SmokeAerodynamics -AerodynamicsBackend simplified -SuspensionBackend simplified
python tools/physics_diagnostics/compare_aerodynamics_sessions.py --system suspension <standard-directory> <simplified-directory>
```

Do not bypass the canonical launcher to avoid a parity failure. Rebuild through `scripts/build_windows.ps1` when required; verify the final installed canonical and template libraries after all Cargo builds finish. Keep a source-hash record for uncommitted physics: HEAD/BUILD equality alone does not identify pending source contents.

## Publication inventory

The implementation map lists the ten candidate source/document files. This handoff adds `docs/physics_simplification_backlog.json`, `docs/physics_simplification_handoff.md` and a related-workstream link in `docs/suspension_coupled_dynamics_backlog.json`. Review those explicit paths if publication is requested. The user subsequently requested publication to `main-clean` for review. Implementation was committed separately as `5192b56e`, followed by these continuation documents. Generated binaries and unrelated changes remain excluded. Verify the actual remote tip rather than treating this document as proof of a completed push.

Preserve and exclude from source commits: `game/BUILD_SOURCE`, installed DLLs, `.godot`/Cargo/SCons outputs, captures under `scratch/`, generated `.import`/`.uid` files, asset reports and unrelated audio/media. `game/crates/formula90-core/src/ffi.rs` was already marked modified before suspension work while showing no textual diff; do not stage it as part of this feature. No model hardpoints, vehicle tuning documents or visual assets were edited for suspension B.

Publication changes HEAD; the existing installed BUILD remains stamped with the validation base. Rebuild and revalidate parity before another game launch; do not rewrite BUILD_SOURCE to bypass the check.

Pending user gates: driving feedback; any requested corrective plan; later controlled suspension performance comparison. Remote publication was requested for source review and does not approve driving behavior. There is no measured suspension CPU improvement to report yet.
