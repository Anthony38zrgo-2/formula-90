# Formula90s canonical vehicle visual asset contract

## Purpose

Formula90s vehicles must not carry four independently-authored wheel meshes when the left and right wheels of an axle are geometrically equivalent. Lateral symmetry is a scene-composition responsibility, not an asset-authoring responsibility.

The default runtime visual contract is exactly three canonical geometries per vehicle:

- `chassis.glb`
- `wheel_front.glb`
- `wheel_rear.glb`

The front wheel geometry is instantiated twice and the rear wheel geometry is instantiated twice.

## Required runtime hierarchy

GEVP still owns four independent `RayCast3D` wheel simulations. Only their visual geometry is shared.

```text
Vehicle (RigidBody3D / GEVP)
├── ChassisVisual -> chassis.glb
├── WheelFrontLeft (RayCast3D)
│   └── Pivot                    # owned dynamically by GEVP
│       └── Orientation          # static Formula90s presentation transform
│           └── Visual -> wheel_front.glb
├── WheelFrontRight (RayCast3D)
│   └── Pivot
│       └── Orientation (Y = 180 deg)
│           └── Visual -> wheel_front.glb
├── WheelRearLeft (RayCast3D)
│   └── Pivot
│       └── Orientation
│           └── Visual -> wheel_rear.glb
└── WheelRearRight (RayCast3D)
    └── Pivot
        └── Orientation (Y = 180 deg)
            └── Visual -> wheel_rear.glb
```

`Pivot` remains the `wheel_node` assigned to GEVP. Formula90s must not place static correction transforms on that node because GEVP changes its vertical position and wheel-spin rotation at runtime.

Static side/orientation corrections belong below `Pivot`, normally on `Orientation`.

## Symmetry invariants

For each axle:

- left and right must instantiate the same `PackedScene` wheel geometry;
- left and right RayCast X positions must be exact opposites around vehicle X=0;
- left and right RayCast Y positions must be identical;
- left and right RayCast Z positions must be identical;
- tire radius, width, mass and suspension parameters are axle-level values, never side-specific values;
- no runtime scale correction may make the two sides geometrically different.

A vehicle requiring intentionally different left/right wheel geometry is an explicit exception and must document why the asymmetry exists.

## Asset coordinate convention

Canonical wheel assets should be authored as reusable axle components:

- local X: axle / wheel-width axis;
- local Y: vertical axis;
- local Z: vehicle forward/back axis;
- wheel rotation center: local origin;
- transform: rotation zero and scale `(1,1,1)` after export;
- geometry centered on the intended wheel rotation axis.

The right-hand visual should normally be produced by a static 180-degree presentation rotation around Y. Do not use negative runtime scale as the standard mirroring mechanism because negative scale can complicate normals, tangents, culling and shader behavior.

## Import/materialization rule

Source bundles may contain legacy `FL/FR/RL/RR` meshes. The runtime importer/materializer must choose one verified canonical source for the front axle and one for the rear axle, then expose only:

```text
<vehicle>_chassis.glb
<vehicle>_wheel_front.glb
<vehicle>_wheel_rear.glb
```

Unused side-specific meshes must not remain in the generated runtime asset directory. This prevents Godot from importing redundant or inconsistent geometry.

For the Jordan 1995 prototype:

- chassis source: `jordan_191_1995_chassis.glb`
- canonical front source: `jordan_191_1995_wheel_fl.glb`
- canonical rear source: `jordan_191_1995_wheel_rl.glb`

The generated runtime names are:

- `jordan_191_1995_chassis.glb`
- `jordan_191_1995_wheel_front.glb`
- `jordan_191_1995_wheel_rear.glb`

## What this removes

The canonical contract makes these patterns unnecessary for normal symmetric cars:

- four independently-generated wheel GLBs;
- per-side wheel AABB normalization;
- runtime scripts that repair left/right visual differences;
- manually approximated left/right wheel heights;
- independent left/right visual scaling.

Asset defects should be corrected once in the canonical source asset rather than hidden through physics, grip, suspension or steering tuning.

## Validation checklist

Before accepting a new Formula90s vehicle:

1. Only three canonical visual geometries are required at runtime.
2. Front left/right reference the same front wheel scene.
3. Rear left/right reference the same rear wheel scene.
4. RayCast positions are mirrored exactly in X and equal in Y/Z per axle.
5. Right-side orientation is static below the GEVP `Pivot`.
6. No negative scale is used for standard side orientation.
7. No per-wheel visual calibrator is required.
8. Straight-line wheel diagnostics show comparable contact/compression behavior left vs right.
9. Any remaining asymmetry is investigated as physics/raycast/chassis geometry, not compensated with tire grip values.
