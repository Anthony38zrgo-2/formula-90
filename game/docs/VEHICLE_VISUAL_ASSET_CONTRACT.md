# Formula-90 vehicle import standard

The authoritative import contract is `docs/vehicles/VEHICLE_IMPORT_STANDARD.md`.
The standard runtime package uses three GLBs: one chassis, one reusable front
wheel and one reusable rear wheel.

## Runtime hierarchy

GEVP owns four independent `RayCast3D` wheels. Their physical state and scene
instances are independent, while the visual GLB resources are shared by axle:

```text
WheelFrontLeft (RayCast3D)
└── FrontLeftWheel (Node3D assigned to wheel_node)
    └── Visual (vehicle_wheel_front.glb)

WheelFrontRight (RayCast3D)
└── FrontRightWheel
    └── Orientation
        └── Visual (vehicle_wheel_front.glb)

WheelRearLeft (RayCast3D)
└── RearLeftWheel
    └── Visual (vehicle_wheel_rear.glb)

WheelRearRight (RayCast3D)
└── RearRightWheel
    └── Orientation
        └── Visual (vehicle_wheel_rear.glb)
```

`RayCast3D` owns physical placement. The `wheel_node` is the direct child
controlled by GEVP; visual GLBs contain no positional correction offsets. A
right-side presentation node may apply a proper 180-degree yaw with determinant
`+1`. Negative scale is forbidden. Four corner-specific wheel GLBs are allowed
only for documented real asymmetry.

## Asset gate

Before a vehicle is accepted, validate the three standard runtime resources
(one chassis, one front wheel and one rear wheel), four independent wheel scene
instances, explicit `GEO_*` names, required `DATUM_*`/`JNT_*` nodes,
metre scale, `+X` right, `+Y` up and `-Z` front, UV range, outward winding,
finite normals, non-degenerate/non-duplicated faces, simple Godot-compatible
materials and nearest filtering for pixel art. Run the smallest scene smoke
after the structural checks pass.
