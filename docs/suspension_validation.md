# SUS-GEO-10 — Suspension validation matrix, A/B protocol, retrospective

Backlog: `instrucciones.md`. All work on `main-clean`, atomic commits, explicit
`git add` paths only; pre-existing dirt (`game/BUILD_SOURCE`, 8 DLLs,
`formula90-core/src/ffi.rs`, `game-sim/src/c_abi.rs`, gear-sentinel fix) was
never incorporated, reverted or overwritten by this track.

## 1. Validation matrix

| ID | Item | Gate | Evidence |
|---|---|---|---|
| 01 | Audit config/geometry/consumers | ✅ human | `suspension_geometric_test` (legacy profile behaviour); audit numbers in chat history |
| 02 | Versioned physical schema | ✅ human (`f42f8b63`) | `geometric_round_trip…`, `legacy_profiles_preserve…`, `invalid_physical_fails…` |
| 03 | Deterministic kinematics + Jacobians | ✅ human (`f42f8b63`) | `kinematics_lengths…`, `mirror_symmetry`, `jacobian_matches…`, `travel_envelope…`, `suspension_kinematics_test` |
| 04 | Spring/damper/stops via virtual work | ✅ human (`af6dcfd2`) | `suspension_geometric_forces_test` 6/6 (analytic, tangent, dissipation, equilibrium, push/pull, stops) |
| 05 | Reactions + chassis dynamics | ✅ human (`767232ae`+`09d186f6`) | `suspension_loads` 3/3 + `suspension_chassis_reactions_test` 5/5 (symmetry, transfer, ARB torque, hanging, energy) |
| 06 | Wheel orientation/trajectory/contact | ✅ tests | `suspension_geometric_tire_test` 4/4 (rest statics, continuity/mirror, toe 1:1, legacy untouched) |
| 07 | Unified snapshot + visuals | ✅ tests + review (tables not connected in-engine) | `suspension_table` parity (hub ≤1 mm, angle ≤4 mrad), `build_geo_table` example, `suspension_table.gd` reader exists but has **no consumer**: `f1_2030_v10_tables.json` is never loaded and `f1_wheel_visual_controller.gd` still solves `SuspensionGeometry` (PBD) per physics frame |
| 08 | Telemetry + comparative bench | ✅ report (numbers corrected by SUS-GEO-11) | `suspension_bench` example + `docs/suspension_bench.md`: r sweeps, tangent ±35–40 %, c·r², push/pull Δ=0.0 exact, mechanism µs + whole-tick before/after in §7 |
| 09 | F1 2030 migration + calibration | ✅ tests | `suspension_f1_2030_geometric_test` 6/6 on audited hardpoints; legacy file/scene untouched; tables checked in (see 07: not consumed) |
| 10 | Final validation + A/B + retro | ⏳ human A/B (protocol §3) | scripted A/B test below + this matrix |
| 11 | Hot-path performance fix (SUS-GEO-11) | ✅ tests + scripted evidence; ⏳ human gate | `PreparedGeometricSuspension` + pose reuse; new unit tests (exact equivalence, 0 envelope searches, rebuild on config change, legacy untouched); FFI tick 89.8→1.18 ms debug / 26.5→0.33 ms release; bit-identical 640-tick traces; memory flat; Godot smoke PASS |

Full suite at close: lib 139/139 (135 + 4 SUS-GEO-11) + all integration files green;
legacy parity/regression/steering suites byte-identical behaviour. Known
pre-existing failures unrelated to this track: `aero_test` 4 cases (tau/balance/drag —
present before, untouched to keep commits atomic).

SUS-GEO-11 addendum: the geometric integration files that took 13.7/12.6/29.0 s
in debug now take 0.20/0.15/0.40 s, and the FFI whole-tick benchmark plus
640-tick equivalence traces are archived with binary SHA256 in
`docs/suspension_bench.md` §7. The runtime session (Godot 4.7.1, debug)
went from ~103 ms/frame (1.1 FPS) to 120 Hz pacing with 2.6–5.5 ms physics
monitor; windowed FPS remains render-bound in this environment.

## 2. Scripted (objective) A/B — DONE in-repo

`suspension_f1_2030_geometric_test::ab_legacy_vs_geometric_static_matches_dynamic_differs_sanely`:
- Static stance identical within 5 % on all wheels (same car).
- 20 mm front step: peak ΔFz differs (>0.5 %, geometry matters) but stays
  sane (<50 %, same car). No NaN on any channel.

## 3. Human (subjective) A/B protocol — geometric is default since activation

The geometric profile ships as the default (`f1_2030_v10_geometric.json` in
scene/session/resource/manifest); legacy stays as the reference file.
Preconditions: clean rebuild per AGENTS.md (same-branch sources, no reused
DLLs/caches), `BUILD` matches `HEAD`, Fuji contract v1.
1. Back up the dirty worktree files if still present; do not mix with this track.
2. Duplicate `f1_2030_v10_rust.tscn` → `_geo.tscn` pointing
   `physics_config_path` at `f1_2030_v10_geometric.json` (+ tables path for
   `suspension_table.gd` hookup: sample hub/upright per wheel from q/rack).
   **Status:** the tables hookup is NOT implemented; the visual controller
   still uses `SuspensionGeometry` (PBD). Do not read step 2 as done for the
   tables path.
3. Run `run_f1_94.ps1` sessions A (legacy) / B (geometric), blinded order,
   same driver/track/Fuji: slow ramp, kerb strikes, high-speed sweeps.
4. Score: predictability at the limit, kerb behaviour, heave/roll damping,
   jumping/landing, FFB/rattle anomalies. Decision: ship geometric as
   opt-in default only if B ≥ A with no anomaly; otherwise file findings
   against the 09 calibration (rocker curves §6 bench) and keep legacy.
5. Record the verdict + driver notes here (append §5).

## 4. Retrospective

- **Sprint A (01+02):** read-only audit paid off — every later bug (vertical
  rocker, over-centre arms, missing ARB/stop paths) traced to audit findings.
- **Sprint B (03):** fixed-iteration PBD from rest gives a deterministic
  branch; the parity probe initially compared clamped-table vs unclamped
  solve — test bug, not solver bug. Evidence first.
- **Sprint C (04):** per-substep solves were 60 s+ in debug; tick-start
  linearization + secant tangent cut it ~half with negligible error.
- **Sprint D (05+06):** the 6×6 needs the ROD AS UNKNOWN (predetermining it
  from the rocker side double-counts); two sign slips caught by the
  balance=<5 N test. Test rockers kept failing (digressive past peak) until
  long-arm trailing rockers (r≈0.5) — geometry design, not solver bugs.
- **Sprint E (07+08+09):** no-Godot environment → tables + reader + parity
  instead of in-engine validation; bench caught the front toggle past
  +0.08 m, which sized the 09 travel limits (0.04/0.05, F1-realistic).
- **Sprint F (SUS-GEO-11):** the 0.3–0.5 ms bench estimate was mechanism-only
  and hid 9 envelope searches/wheel/tick in the shipped flow; measuring the
  FFI tick (not isolated solves) exposed the 26.5 ms release regression. Fix:
  prepare per-wheel envelopes once, reuse the exact final pose solve. Scripted
  equivalence (bit-identical traces) proved zero behaviour change, so no
  calibration retuning was needed.
- **Process:** transient `Edit`/`WriteFile` failures on large files cost many
  cycles; python-via-file + small spans worked. Debug-build PBD dominates
  test time — keep sim-level tick counts minimal, helper-level asserts first.
- **Debt carried:** shipped vertical-axis rocker stays blocked (correctly);
  rear test rockers in 02/03/04 fixtures are digressive (documented, 09 uses
  healthy arms); `aero_test` 4 pre-existing failures; GDScript parse-check is
  green in this environment (Godot 4.7.1 available); controller hookup to the
  pose tables + human A/B still pending.

## 5. Human A/B verdict

_Pending — fill after §3 runs._
