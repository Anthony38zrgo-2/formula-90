import json

import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree


MID_NAME = "GEO_CHASSIS_REAR_WING_MID"
LATERAL_OBJECTS = (
    "GEO_CHASSIS_REAR_WING",
    "GEO_CHASSIS_ENDPLATE",
)


def world_bvh(obj):
    mesh = obj.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
    try:
        vertices = [obj.matrix_world @ vertex.co for vertex in mesh.vertices]
        polygons = [tuple(poly.vertices) for poly in mesh.polygons]
        return BVHTree.FromPolygons(vertices, polygons, all_triangles=False)
    finally:
        obj.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh_clear()


mid = bpy.data.objects.get(MID_NAME)
if mid is None:
    raise RuntimeError(f"Missing required object: {MID_NAME}")

mid_bvh = world_bvh(mid)
checks = {}
for name in LATERAL_OBJECTS:
    other = bpy.data.objects.get(name)
    if other is None:
        checks[name] = {"present": False, "triangle_overlaps": None}
        continue
    checks[name] = {
        "present": True,
        "triangle_overlaps": len(mid_bvh.overlap(world_bvh(other))),
    }

result = {
    "mid_object": MID_NAME,
    "mid_y_bounds_m": [
        min((mid.matrix_world @ Vector(corner)).y for corner in mid.bound_box),
        max((mid.matrix_world @ Vector(corner)).y for corner in mid.bound_box),
    ],
    "checks": checks,
    "passed": all(
        value["triangle_overlaps"] == 0
        for value in checks.values()
        if value["present"]
    ),
}
print("F90_REAR_WING_MID_CLEARANCE=" + json.dumps(result, sort_keys=True))
if not result["passed"]:
    raise SystemExit(1)
