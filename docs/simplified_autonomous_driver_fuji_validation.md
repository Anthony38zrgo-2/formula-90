# Fuji autonomous hotlap with all simplified physics

This document records the earlier controller baseline. The subsequent continuous-pedal implementation and 0.80/0.85/0.90 comparison are documented in `docs/autonomous_driver_longitudinal_control.md`.

Validated on 2026-10-16, branch `main-clean`, native source HEAD `304101829536cb63fb0c712fe4ae1020c4a130c0`. The test adapter and canonical diagnostic entry are pending source changes; no vehicle, force, thermal or AI tuning was modified for the result.

## Result

The existing Rust autonomous driver completed one standing lap and one flying lap on the actual Fuji Godot scene, using simplified aerodynamics, suspension, tires and slow states. Mechanical updates were 120 Hz and driver decisions 60 Hz. The unrestricted driver tuning has `maximum_speed_metres_per_second: null`; its normal corner and braking planner remains active.

| Measurement | Result |
| --- | ---: |
| Standing lap | 1:15.567 |
| Flying lap | 1:10.350 |
| Maximum speed | 296.379 km/h |
| Maximum forward gear | 7 |
| Route length | 4310.294 m |
| Total route progress | 8620.909 m |
| Full-throttle fraction, whole test | 79.169% |
| Full-throttle fraction, flying lap | 78.115% |
| Braking fraction, whole test | 14.402% |
| Maximum lateral deviation, whole test | 2.115 m |
| Maximum lateral deviation, flying lap | 1.944 m |
| Maximum body inclination | 5.435 degrees |
| Recorded control samples | 8756 |
| Simulated duration | 145.917 s |
| Wall duration | 112.005 s |

Both laps passed the route-width footprint gate, projection continuity, motion, finite-state and rollover checks. All telemetry samples are finite. No script/runtime errors appeared in the successful log. All fifteen installed optimized Rust copies and three DSP copies were verified after the run, with BUILD/HEAD parity. Report times are obtained from full-route progress sampled at 60 Hz (16.667 ms resolution), rather than race-menu timing certification.

The canonical profile's stability aid and automatic clutch stayed enabled (mask 36); traction control and ABS stayed disabled. Automatic gear selection was enabled only on the validation vehicle, without changing the player profile. The driver retains its nominal grip estimate and cornering envelope, so this proves completion at unrestricted straight-line pace, not the ultimate achievable lap time or an unassisted lap.

## Adapter correction before the accepted run

The previous Fuji AI harness drives `CoupledVehicleWorld`. The three existing simplified force backends instead belong to canonical Godot rigid-body integration. Reusing the old harness would test a different physics owner.

The new `game/tests/test_simplified_autonomous_driver_fuji.gd` drives the real canonical vehicle using the existing Rust driver process. It is launched only through `scripts/run_f1_94.ps1 -AutonomousDriverOutputDirectory <directory>`, preserving native parity, asset checks and import validation. The driver must already be listening locally and the output directory must contain the source-derived steering conversion table.

The first adapter attempt passed coupled rack commands directly to the canonical steering input. It stopped after 11.877 m because the steering conventions differ in sign and scale. This attempt is preserved but is not accepted as a vehicle-physics finding. Source inspection confirmed: a desired right wheel angle of +0.1 rad maps to coupled command +0.534034, while positive canonical steering rotates the wheel left. The accepted adapter converts the driver's rack command back to wheel angle and then to the corresponding canonical command, including the configured steering exponent and average front-wheel Ackermann mapping. It changes no physical parameter.

Calibration sources and executable are preserved as `derive_steering_mapping.rs` / `.exe` under the evidence root; the 1001-entry mapping is in the accepted run directory. The mapping was checked for size and monotonicity, and the final GDScript passed Godot MCP validation before execution. User-facing input direction is unchanged; conversion exists only in this test adapter.

## Evidence and limits

Accepted result: `scratch/fuji_simplified_hotlap_20261016/calibrated_attempt/fuji_report.json`.

Verification: `scratch/fuji_simplified_hotlap_20261016/calibrated_attempt/verified_summary.json`.

That directory contains complete CSV telemetry, canonical runtime and bridge logs, source/binary hashes, steering mapping, acceptance criteria and execution identity. The parent directory preserves the first attempt, full clean rebuild logs, pre-existing-change inventory and installed-binary backup. `run_calibrated_validation.ps1` records the exact orchestration used; use a fresh output directory when reproducing so evidence is preserved.

Headless single-car validation; no live debugging bridge was installed. No screenshots, human visual acceptance, traffic/racecraft, ten-lap endurance, full-grid performance, all-standard A/B or maximum-pace optimization are established by this result. Historical coupled-world blockers remain separate and are not closed by this canonical-path success.
