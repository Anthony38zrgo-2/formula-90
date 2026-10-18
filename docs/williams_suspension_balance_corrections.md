# Williams suspension and contact balance corrections

Date: 2026-10-18. Source baseline: `main-clean`, `b1629807fb1d3e3288c1f2727662e1735f0d40b2`.

The geometric Williams profile could sustain a yaw oscillation at rest and convert asymmetric vertical wheel loading into a yaw moment. Its virtual unsprung masses also contributed gravity a second time even though the host rigid body already carried the complete vehicle mass.

## Implemented behavior

- Static lateral contact now solves all active wheel impulses together with a projected iterative solve. The contact response includes off-diagonal coupling through the same rigid body, rotated principal inertia, friction bounds and the existing low-speed blend. It no longer applies four independent whole-body stopping impulses.
- Anchor reactions now carry a force and a moment. Transferring the rod and damper reactions to the rocker pivot includes the corresponding moment, so reducing the linkage to the integrated body preserves the complete resultant. The prepared suspension backend checks both force and moment residuals before accepting an interpolated response.
- The normal vehicle route continues to integrate the total mass, including fuel, wheels and uprights, in one rigid body. The suspension's virtual unsprung coordinate remains a contact filter. Its gravity and relative acceleration are therefore not subtracted again when transmitting loads to that total body. Low-level reaction helpers retain their explicit separate-mass arguments for callers that actually separate those masses.

No vehicle setup values or profile digests changed. This repair does not claim that the virtual filter is a complete constrained multibody suspension. It preserves the existing single-body integration contract. The opt-in coupled world retains its own mass matrix and integration ownership.

## Native validation

The new contact regression checks total kinetic energy after simultaneous impulses, including friction saturation, partial blend and rotated body inertia. The load-transmission regression checks individual asymmetric forces and free couples on all four wheels at three suspension positions. The Williams rest gate additionally requires normal support within one percent of total vehicle weight and yaw deviation below 0.001 rad/s. The airborne and settled-anchor tests now enforce the total-body mass convention.

Twelve selected integration suites pass: **59 tests**, with two diagnostic tests remaining ignored. The library suite reports **169 passes and one unrelated failure**.

The failure is `aero::tests::f1_2030_wheel_drag_reference_matches_profile_geometry`. The baseline HEAD already specifies wheel reference area 0.0792 m², while that test's formula yields 0.0804 m². Neither the aerodynamics code nor those profile values changed. Evidence: `scratch/williams_balance_correction_20261018/preexisting_aerodynamics_failure.json`.

Matching ten-second native experiments use the Williams profile, simplified tire/aerodynamic/slow-state backends and an impulse of 0.01 rad/s at three seconds. Measurements below cover seconds eight to ten on planar ground:

| Measurement | Before | After |
|---|---:|---:|
| Simplified suspension, 120 Hz, yaw RMS | 0.152449 rad/s | 0.000002847 rad/s |
| Standard suspension, 120 Hz, yaw RMS | 0.152446 rad/s | 0.000000369 rad/s |
| Simplified suspension, 240 Hz, yaw RMS | 0.096103 rad/s | 0.000001443 rad/s |
| Simplified suspension, 480 Hz, yaw RMS | 0.054125 rad/s | 0.000000768 rad/s |
| Simplified 120 Hz, mean normal support | 6656.496 N | 6007.427 N |
| Initial total vehicle weight | 6007.554 N | 6007.554 N |
| Rear-left yaw response to another 100 N vertical load near settled travel | -18.770449 N·m | 0.000000430 N·m |

Fuel consumption and remaining small transients account for the small difference between initial weight and measured mean support. No extra normal-force scale was introduced.

The native rest regression over twenty seconds reports yaw standard deviation approximately `7.32e-7 rad/s` and maximum compression range `0.0237 mm`. These are automated bench results, not a rendered-track handling approval.

## Reproduction and delivery boundaries

Native suite compilation and execution logs, individual integration outputs, baseline hashes and build evidence are retained under `scratch/williams_balance_correction_20261018/`. The original audit remains under `scratch/williams_suspension_equation_audit_20261018/`.

The diagnostic program's `STATIC_OPERATOR` output exercises the retained isolated-contact compatibility helper. It is not the new runtime multirrueda solver; runtime passivity is checked by `vehicle_contact_balance_test` and by the full simulator experiments.

The source commit precedes the canonical rebuild. Rebuild and runtime evidence are recorded separately under the delivery directory so generated DLLs and `game/BUILD_SOURCE` stay outside the source commit. Pre-existing modified source and installed binaries were backed up before cleaning Cargo, SCons Debug/Release, Godot imports and DSP Debug/Release. Pre-existing unrelated modifications remain outside this commit and must be recorded separately from the HEAD build stamp.

The local-to-world hub-offset defect and thermal work accounting identified by the earlier audit are outside these three approved corrections. A driving test with large steering reversals and the actual irregularities of Fuji's last curve remains required for human handling approval.
