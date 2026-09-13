# F1 2030 Suspension Geometry Backlog

## Purpose

Simulate the visual suspension linkage of the F1 2030 V10 so the whole
**chassis â†’ suspension â†’ wheel** assembly is animated by, and connected to, the
physics. Each wheel resolves a per-corner mechanism composed of
`UPPER_WISHBONE`, `LOWER_WISHBONE`, `PUSHROD`, `TRACKROD`, `UPRIGHT`, `ROCKER`
and (rear only) `DRIVESHAFT`.

## Authority model (approved)

- The Rust 1-DOF solver remains the only suspension **physics**. Its telemetry
  (`compression`, `steer`, `camber`) parametrises the mechanism.
- The linkage is **visual kinematics only**: a position-based solver keeps the
  arms rigid and the upright triangle closed while the wheel centre tracks the
  telemetry height (soft vertical target). The mechanism owns the lateral track
  change (real scrub).
- Geometry source is **procedural in Godot** from a per-corner hardpoint table
  declared in the vehicle physics JSON (`suspension.geometry`). No Blender
  re-export is required.
- Scope: `f1_2030_v10` only.

## Contract

`game/data/vehicles/f1_2030/f1_2030_v10_physics.json`:

```jsonc
"suspension": {
  "geometry": {
    "FL": {
      "hub_center": [x, y, z],
      "lower_wishbone": { "inner_front": [..], "inner_rear": [..], "outer": [..] },
      "upper_wishbone": { "inner_front": [..], "inner_rear": [..], "outer": [..] },
      "trackrod": { "inner": [..], "outer": [..] },
      "pushrod": { "outer": [..] },
      "rocker": { "pivot": [..], "axis": [..], "pushrod_arm": [..], "damper_arm": [..] },
      "damper": { "chassis": [..] },
      "driveshaft": { "inner": [..], "outer": [..] }   // rear only
    },
    "FR": { "mirror_of": "FL" },
    "RL": { ... },
    "RR": { "mirror_of": "RL" }
  }
}
```

Coordinates are chassis-local meters, `+X` right, `+Y` up, `-Z` front (the
vehicle `coordinate_contract`). `mirror_of` mirrors the base corner across X.

Rust accepts the block as a **parser-only** `Option<serde_json::Value>` field on
`JsonSuspension` (`deny_unknown_fields` keeps rejecting typos); it never reaches
runtime forces and is dropped by `to_json_value`.

## Implementation

| Area | Files |
|---|---|
| Parser | `game/crates/vehicle-physics-engine/src/vehicle_config.rs` |
| Hardpoints | `game/data/vehicles/f1_2030/f1_2030_v10_physics.json` (+ `manifest.json` sha) |
| Solver | `game/scripts/vehicle/suspension_geometry.gd` |
| Visual builder | `game/scripts/vehicle/suspension_link_visual.gd` |
| Integration | `game/scripts/vehicle/f1_wheel_visual_controller.gd` |
| Tests | `game/tests/test_f1_2030_suspension_geometry.gd`, `game/tests/test_f1_2030_suspension_runtime.gd` |

Solver invariants (PBD, 48 iterations + 80 constraint-only passes):

- `LBJ`/`UBJ` stay on their wishbone circles (arm radii constant).
- The upright triangle `|LBJ-UBJ|`, `|LBJ-hub|`, `|UBJ-hub|` stays rigid.
- Only the vertical component of the hub is pulled to the telemetry target; the
  mechanism owns the lateral track change.
- The rocker angle is solved from the pushrod-length circle intersection; the
  damper telescopes; the driveshaft (rear) telescopes and spins with the wheel.

## Historical validation evidence (2026-09-12; superseded for visual acceptance)

- `test_f1_2030_suspension_geometry.gd`: hardpoint validity, rear-only
  driveshaft, rest hub placement, rigid-arm travel sweep, steering sweep.
- `test_f1_2030_suspension_runtime.gd`: all element nodes present per wheel,
  baked `GEO_CHASSIS_*_SUSPENSION` hidden, FL lower wishbone moves ~75 mm and
  upright ~57 mm under braking.
- `test_f1_wheel_visual_positions.gd`, `test_f1_2030_v10_rust_physics.gd`,
  canonical Fuji smoke and `test_fast.ps1` (Vehicle, Runtime) all pass.
- Rust: `suspension_geometry_parser_only_accepted_and_ignored` passes; the full
  crate suite has 4 **pre-existing** `aero_test` failures (unrelated to this
  change; they assert `f1_94_canonical()` values).

## Historical build / parity (2026-09-12)

Rust changed â‡’ `scripts/build_windows.ps1` republished the runtime and refreshed
`game/BUILD_SOURCE`. `run_f1_94.ps1 -ValidateRuntimeOnly` confirms BUILD/HEAD
parity.

## Out of scope (Physics V2 debt)

- Linkage-generated forces / multibody coupling.
- Blender-authored per-element meshes and `JNT_SUSP_*` nodes (hardpoints are
  JSON-driven today; the import standard's `JNT_*`/`DATUM_*` remain the target
  for authored vehicles).
- Vehicle `f1_2026_2008`.

## SUSPGEO visual repair — 2026-09-13

Baseline: main-clean / `27856b5b0327580cd5b73c6097c6c2e1d89f44f6`, clean.

Scope: shared rendered wheel/linkage pose, rigid upright and steering links,
axis-correct rocker, and meshes built around actual attachment points. Two
visual rocker pushrod attachments were recalibrated within `suspension.geometry`;
all force-facing JSON values and Rust sources remain identical to baseline.
The physics-profile SHA-256 in the asset manifest was refreshed. Source GLBs
are unchanged.

- Compression and rack command are smoothed before solving. The wheel hub and
  bearing orientation are applied from that same pose without a second lerp.
- The upright is transported from its authored triangle and steers as a rigid
  body about the ball joints, including the offset wheel center. Static camber
  and toe are mirrored bearing alignment; travel camber/bump steer come from
  the visual mechanism rather than the old independent wheel approximation.
- Both front trackrods have fixed lengths and share one calibrated rack travel.
  Rear trackrods remain anchored to the chassis. Steering closure is solved by
  a circle/sphere intersection, choosing the branch nearest the command.
- The pushrod attachment retains its complete 3D offset on the lower wishbone.
  The rocker circle retains its axial offset and its mesh connects the pivot,
  pushrod and damper arms. Uprights are extruded attachment triangles with a
  steering arm; rods/shafts are cylindrical, joints spherical, and dampers use
  fixed-length body/piston meshes with changing overlap. Lower arms include
  supports to the offset pushrod lug; upright bearing housings use the same
  alignment as the wheel. Fixed sleeves connect the hardpoints to seats measured on the visible
  chassis mesh using native triangle BVHs once after readiness.
- The original rocker rest arms were near a toggle limit (front droop under
  1 mm). Repositioning their visual pushrod attachments produces an approximately
  100 mm lever, oriented across the pushrod direction at rest. Chassis pivots,
  wishbones, wheel centers and all force parameters remain unchanged.
- Unreachable travel is exposed as `travel_limited`, `requested_compression`,
  `solved_compression`; the wheel/linkage stop together rather than stretching.
  The calibrated visual compression intervals are front 0.000959–0.210231 m
  (rest 0.045 m), rear 0.032916–0.200495 m (rest 0.0594 m). These are visual
  mechanical limits, not new physical bump stops.

Validation: isolated Godot 4.7.1 project, no native DLLs loaded. Run
`tools/physics_diagnostics/preview_suspension_visuals.ps1 -OutputDirectory <path> -Animate`.
This copies explicit inputs into a fresh temporary project, imports/parses the
scripts and five GLBs, runs both native-free regression suites, renders four
captures plus optional animation frames, and saves source hashes and logs.

The new `test_f1_2030_suspension_visual_pose.gd` covers all four corners, 25
travel samples from rest -20 mm to +100 mm, and steering -25/0/+25 degrees.
No sample in that envelope is limited. It checks rigid bases, upright/rod/
rocker attachment closure, damper overlap, mechanical stops and the actual
controller/builder over a smoothed input step. Original geometry invariants
also pass. Relative visual pose checks replace the old runtime test's global
movement measurement as evidence of articulation; that historical measurement
could be satisfied by motion of the entire car.

Review: PNGs with the actual GLBs were inspected; the detail views hide wheels
so attachments can be reviewed. Generated evidence includes `provenance.json`.
Full driving/runtime validation was not run: installed `BUILD_SOURCE` is
`38b4149a216cbf1853912d37b22994ae4caa4768`, different from current HEAD. No native
binaries, build stamps or runtime caches were changed for this repair.

Status: implementation and isolated automated review complete; human visual
acceptance and driving validation pending. Retrospective/done follow that gate.
