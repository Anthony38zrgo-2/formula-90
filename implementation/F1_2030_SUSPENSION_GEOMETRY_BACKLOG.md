# F1 2030 Suspension Geometry Backlog

## Purpose

Simulate the visual suspension linkage of the F1 2030 V10 so the whole
**chassis → suspension → wheel** assembly is animated by, and connected to, the
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

Solver invariants (PBD, 32 iterations + 12 constraint-only passes):

- `LBJ`/`UBJ` stay on their wishbone circles (arm radii constant).
- The upright triangle `|LBJ-UBJ|`, `|LBJ-hub|`, `|UBJ-hub|` stays rigid.
- Only the vertical component of the hub is pulled to the telemetry target; the
  mechanism owns the lateral track change.
- The rocker angle is solved from the pushrod-length circle intersection; the
  damper telescopes; the driveshaft (rear) telescopes and spins with the wheel.

## Validation evidence

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

## Build / parity

Rust changed ⇒ `scripts/build_windows.ps1` republished the runtime and refreshed
`game/BUILD_SOURCE`. `run_f1_94.ps1 -ValidateRuntimeOnly` confirms BUILD/HEAD
parity.

## Out of scope (Physics V2 debt)

- Linkage-generated forces / multibody coupling.
- Blender-authored per-element meshes and `JNT_SUSP_*` nodes (hardpoints are
  JSON-driven today; the import standard's `JNT_*`/`DATUM_*` remain the target
  for authored vehicles).
- Vehicle `f1_2026_2008`.