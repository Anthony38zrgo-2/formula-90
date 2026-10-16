# Autonomous driver: continuous longitudinal control

Implemented above `main-clean` source base `304101829536cb63fb0c712fe4ae1020c4a130c0`. AI-only changes; native physics sources, vehicle profiles and installed physical binaries remain fixed. Existing HUD and Fuji adapter work is preserved.

## Controller and ownership

`local_driver/longitudinal_control.rs` owns a single signed pedal state. Positive values command accelerator, negative values command brake, and zero crosses through coasting. Normal transitions respect configured pedal slew rates; no positive brake command can reset the accelerator instantaneously. Pedals remain mutually exclusive.

Braking entry and release use separate 2.0% and 0.6% demand thresholds. A continuous speed-error response replaces the old 0.5 m/s Boolean braking gate. The existing integral regulator has saturation protection. Drag deceleration is distinguished from pedal-generated deceleration so the driver can coast instead of requesting the brake for drag already available.

`local_driver/longitudinal_profile.rs` samples the original piecewise speed-squared envelope on a closed lap. The speed target stays at the current path location. Only acceleration demand is anticipated by 0.5 s. Spatial finite-window differentiation regularizes acceleration across waypoint segments; the more conservative of the current and anticipated accelerations is used. Feedforward acceleration changes are limited to 120 m/s cubed. This bounds planned demand, not measured chassis jerk or gearbox transients.

Both tuning JSON files explicitly own the new longitudinal settings. Normal accelerator increase/decrease rates are 2.5/4.0 per second, and brake increase/decrease rates 4.0/3.5. Overspeed above 4 m/s permits the separately configured faster 8.0/s response toward braking. Feedback gain is 3.0 for the acceleration-domain controller. Steering, AT logic, player profile, tire grip, power, suspension, aerodynamics and thermal behavior are unchanged.

## Validation

Eight new continuity, hysteresis, anticipation, speed-envelope, urgency and configuration tests pass. Sixteen existing control, pursuit, recovery, geometry, scheduling and transmission tests pass. The launch regression now requires full throttle after the configured ramp duration rather than the former fixed eight ticks.

The broader coupled flat-ground regression rejects vehicle/world overlap. The original driver and original tuning reproduced the same physical rejection in an isolated baseline crate against the identical physics source. Evidence is in `scratch/ai_longitudinal_smoothing_20261016/reference_closed_loop_regression_verified.log`; this separate physical-world gate remains open.

The initial candidate eliminated abrupt accelerator cuts and reduced short brake episodes from 42 to 6, but its 82.333 s flying lap lost too much pace. That pilot advanced both speed target and acceleration. The retained implementation keeps the original current-position target and anticipates only acceleration. Its comparison is recorded separately below; pilot and interrupted captures are retained.

## Controlled utilization comparison

The completed campaign uses 0.80 / 0.85 / 0.90 utilization, with nominal grip calibration fixed at 0.65, null global speed cap, all four simplified physical backends, 120 Hz mechanics, 60 Hz driver control, AT, stability and automatic clutch. Each variant starts with the same vehicle profile and initial state and attempts one standing and one flying lap.


| Controller / utilization | Flying lap | Brake episodes | Episodes below 0.25 s | Abrupt accelerator cuts above 20 points |
| --- | ---: | ---: | ---: | ---: |
| Previous / 0.80 | 1:10.350 | 53 | 42 | 39 |
| Continuous / 0.80 | 1:12.083 | 13 | 3 | 0 |
| Continuous / 0.85 | 1:10.383 | 11 | 4 | 0 |
| Continuous / 0.90 | 1:09.100 | 9 | 3 | 0 |

All three completed both laps without footprint exits, overturning, invalid states or runtime/script errors. Commands never overlap accelerator and brake. Maximum measured signed pedal slew is 4.000/s in each run, versus 68.000/s in the earlier controller. The planned-acceleration jerk limit measured 120.000 m/s cubed. At 0.80, short brake episodes fall 92.86%; at 0.90, pedal total variation falls 75.41% against the previous 0.80 baseline. The cross-utilization comparison combines improved control and changed planner reserve, so its lap-time difference must not be attributed solely to smoothing.

The 0.90 variant is fastest in this short comparison, 1.250 s ahead of the earlier baseline. The 0.85 variant is within 0.033 s of the earlier baseline while using substantially smoother pedals. At fixed 0.80 utilization, smoothing costs 1.733 s per flying lap. The existing default remains 0.80; selecting a higher default is separate from the requested comparison. No statistical pace conclusion or endurance claim is made from one flying lap per setting.

Only `grip_utilization_fraction` differs among candidate tuning files. All candidate source hashes and physical source/profile/installed-binary hashes match their pre-capture values; matching baseline physical source and profile hashes were also verified. The AI executable is rebuilt and available for the canonical observation adapter. Native BUILD/HEAD parity remains at 30410182; native rebuilding was unnecessary because this task changed only AI and diagnostic source.

Evidence: `scratch/ai_longitudinal_smoothing_20261016/refined_campaign/comparison_summary.json`. Source snapshots, immutable driver executable, individual tuning documents, per-control telemetry, runtime logs and fixed physics hashes are preserved. The existing utilization 0.80 remains the default while results await review. Utilization is a planner reserve, not a measurement of actual remaining tire grip.

This is a headless single-car comparison. No traffic, human driving approval, visual acceptance, endurance or all-standard physical comparison is inferred.
