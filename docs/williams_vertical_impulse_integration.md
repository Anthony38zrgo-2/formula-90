# Williams vertical integration and normal impulse transfer

The Williams telemetry `telemetry_20261018_035306_518.csv` exposed severe vertical responses after asymmetric suspension travel reached the stops. A native flat-ground experiment using its recorded controls rolled over at 120 Hz while remaining upright at higher external frequencies. That experiment is a diagnostic, not an exact reconstruction of the Godot session.

## Implemented contract

The simplified suspension backend used by the Williams launcher now solves the geometric and legacy virtual wheel filters with backward Euler. Tire compliance, the digressive damper, spring and nonlinear stops are evaluated at the solved endpoint. The existing anti-roll snapshots are retained: the legacy axle uses its common tick-start force, while geometric mode evaluates its own solved compression against the opposite wheel's tick-start compression.

`next_compression = previous_compression + duration * next_velocity`

`mass * (next_velocity - previous_velocity) = tire_impulse - suspension_impulse + travel_limit_impulse`

The scalar solve brackets the velocity inside the existing travel envelope, including the existing allowed overtravel. It uses deterministic bisection. When a travel boundary is active, its constraint impulse is recorded explicitly instead of silently losing that momentum through a position clamp.

Normal impulses are accumulated across all mechanical substeps. In the simplified backend, `total_normal_force` is their sum divided by the external step duration; the body and tire force calculation consume that interval mean. `tire_vertical_force` remains the endpoint diagnostic. Positive interval impulses are retained without the previous one-newton grounded deadband. Body mass continues to include wheel masses once; virtual wheel masses remain contact filters, not extra gravitational mass.

For the simplified lumped-mass route, the aggregate geometric wrench is constrained to the known external contact force and its moment about the actual center of mass, including the aligning couple. Interpolated anchor reactions remain diagnostic approximations with their original residuals. An unreachable linkage solution retains its singular flag but cannot erase the external contact impulse. This also prevents small interpolation residuals from leaking into the body impulse balance. Airborne contacts contribute zero external wrench. Separately integrated nonzero unsprung masses are not subject to this projection.

Damper dissipation is accumulated from each solved substep instead of extrapolating the final sample across the external interval. In the implicit route, the extra velocity-sign-dependent force previously added when a geometric stop was flagged has been removed: the configured nonlinear stop already opposes penetration, and that additional discontinuity prevented a continuous momentum residual at zero velocity. Invalid geometry remains flagged and the travel constraint remains active.

The standard backend retains its reference explicit integration, force sampling and grounded threshold. Its accumulated impulse diagnostics describe the mechanical substeps, while its body force still follows the reference last-sample contract. Applying the new integrator globally changed an all-standard driving trace and introduced rollover; that candidate was rejected. Numerical integration selection is separate from geometry approximation internally so the existing geometry parity test can compare both geometry implementations under the same implicit method, without relaxing its travel or force tolerances.

No spring rates, damper rates, masses, steering inputs, assists or profile data were retuned. The original mechanical substep duration and external 120 Hz frequency are preserved.

## Regression evidence

`williams_vertical_integration_test.rs` contains a native plane test driven by the recorded input fixture at 120, 240 and 480 Hz, with the four simplified backends selected explicitly to match `scripts/run_f1_97.ps1`. The fixture identifies its source telemetry and SHA-256 digest. Plane rays are intersected individually, including their transverse offsets.

A second test covers the simplified legacy F1 94 and geometric Williams routes under an 80 mm road-target step. It checks virtual wheel momentum closure and the agreement between normal impulse and external body force. A rotated-body test injects degenerate linkage geometry and verifies that the failed solve preserves contact force and moment while reporting the singularity; its airborne case verifies zero external wrench. Unit tests check stiff-mode energy decay, momentum at travel constraints and the difference between an integrated transient impulse and the last force multiplied by the full external duration.

The regression plane produced these maxima, completing every frequency without rollover:

| External frequency | Vertical speed | Combined inclination | Sum of normal forces |
|---|---:|---:|---:|
| 120 Hz | 0.644 m/s | 7.684 degrees | 17.697 kN |
| 240 Hz | 0.893 m/s | 9.835 degrees | 14.588 kN |
| 480 Hz | 0.988 m/s | 10.674 degrees | 14.675 kN |

Peak speed remained above 256 km/h in all three tests. The original diagnostic's matched 120 Hz comparison changed from rollover, 5.596 m/s vertical speed and 90.959 degrees to no rollover, 0.567 m/s and 7.124 degrees. The independent all-standard 120 Hz trace retained exactly the original measured maxima: 207.5295 km/h, 0.74838 m/s vertical speed and 5.59699 degrees inclination.

Automated selection: 172 library tests and 35 integration tests passed. The only failing test is the previously recorded `aero::tests::f1_2030_wheel_drag_reference_matches_profile_geometry`; the profile/reference aerodynamic-area mismatch is unchanged and outside this patch. The overall Cargo invocation therefore exits nonzero; it must not be described as an entirely passing library suite.

The earlier diagnostic uses a slightly different transverse-ray approximation. Its before/after results should only be compared within that same diagnostic; values from the geometrically intersected regression plane are recorded separately. Automated test logs and rebuild evidence are under `scratch/williams_vertical_impulse_fix_20261018/`.

## Acceptance limits

This is a correction to the virtual vertical integrator and impulse accounting. It is not a fully coupled body-and-four-wheel implicit solve: road targets, tick-start geometry linearization and the opposite-wheel anti-roll snapshot still update at the external frequency. The optional coupled-world integration owner is unchanged.

The requested driving reproduction uses the simplified runtime route. The all-standard-backend route is checked independently against its original reference behavior and must not be presented as receiving the implicit integration fix. General library and suspension tests remain separate gates.

Neither the local hub-offset transformation nor the underfloor rigid-contact coordinate diagnosis is changed here. Full-rate Godot orientation/contact telemetry and human driving acceptance on the original track remain necessary; native plane stability does not certify all bumps, collisions, inputs, profiles or backend combinations.
