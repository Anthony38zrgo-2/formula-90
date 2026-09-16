"""Extract source suspension profiles, preserving the .blend unchanged.

blender --background f1_2030.blend --python SCRIPT -- OUTPUT.json
Coordinate conversion matches export_f1_2030_v10.py.
"""
import hashlib
import json
import sys
from pathlib import Path

import bpy
from math import copysign
from mathutils import Vector


def extract(obj):
    points = [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]
    parents = list(range(len(points)))

    def find(index):
        while parents[index] != index:
            parents[index] = parents[parents[index]]
            index = parents[index]
        return index

    # Source faces have split vertices along seams. Weld for grouping only;
    # output retains the original evaluated coordinates and triangles.
    seen = {}
    for index, point in enumerate(points):
        key = tuple(round(value, 4) for value in point)
        if key in seen:
            parents[find(index)] = find(seen[key])
        seen[key] = index
    for edge in obj.data.edges:
        first, second = edge.vertices
        parents[find(first)] = find(second)
    groups = {}
    for index in range(len(points)):
        groups.setdefault(find(index), []).append(index)
    obj.data.calc_loop_triangles()
    result = []
    for indices in groups.values():
        lookup = {vertex: index for index, vertex in enumerate(indices)}
        vertices = [
            [round(points[i].y, 7), round(points[i].z + 0.1050000228, 7), round(points[i].x, 7)]
            for i in indices
        ]
        triangles = []
        for triangle in obj.data.loop_triangles:
            if triangle.vertices[0] in lookup:
                triangles.extend(lookup[index] for index in triangle.vertices)
        result.append({"vertices": vertices, "indices": triangles})
    return result


def append_blade(part, start, end, width=0.065, thickness=0.014):
    start, end = Vector(start), Vector(end)
    direction = (end - start).normalized()
    across = direction.cross(Vector((0, 1, 0))).normalized()
    normal = across.cross(direction).normalized()
    base = len(part["vertices"])
    for point in (start, end):
        for x, y in [(-1, -1), (-1, 1), (1, 1), (1, -1)]:
            part["vertices"].append(list(point + across * x * width * 0.5 + normal * y * thickness * 0.5))
    faces = [0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4,
             1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7]
    part["indices"].extend(base + index for index in faces)


def retarget_front_pushrod(part, outer, arm):
    """Re-anchor the authored pushrod blade onto the physical mechanism.

    The authored blade runs from the measured outboard hardpoint to the legacy
    vertical-axis rocker arm (`legacy_arm`, the authored `suspension.geometry`
    `rocker.pushrod_arm`). The geometric mechanism keeps the same outboard
    hardpoint but the rocker arm sits further forward with a longer pushrod,
    so the authored blade would render detached from the rocker. Map every
    blade vertex by rotating the authored axis onto the new one about the
    outboard hardpoint and stretching the axial coordinate by the length
    ratio; radial offsets keep their authored shape, and the inboard ring
    lands exactly on the physical rocker arm.
    """
    outer, arm = Vector(outer), Vector(arm)
    legacy_arm = Vector([copysign(0.182718, arm.x), 0.2204355, -1.3562085])
    old_axis = legacy_arm - outer
    old_length = old_axis.length
    new_axis = arm - outer
    new_length = new_axis.length
    if old_length < 1e-9 or new_length < 1e-9:
        raise RuntimeError("Degenerate pushrod axis")
    old_dir = old_axis / old_length
    new_dir = new_axis / new_length
    rotation = old_dir.rotation_difference(new_dir).to_matrix()
    points = [Vector(v) for v in part["vertices"]]
    remapped = []
    for point in points:
        rotated = rotation @ (point - outer)
        offset = rotated
        along = new_dir * offset.dot(new_dir)
        final = outer + along * (new_length / old_length) + (offset - along)
        remapped.append([round(c, 7) for c in final])
    part["vertices"] = remapped


FRONT_PUSHROD = {
    "FL": {
        "outer": [-0.549739, -0.019274, -1.4389295],
        "arm": [-0.162718, 0.2204355, -1.6062085],
    },
    "FR": {
        "outer": [0.549739, -0.019274, -1.4389295],
        "arm": [0.162718, 0.2204355, -1.6062085],
    },
}


def main():
    output = Path(sys.argv[sys.argv.index("--") + 1]).resolve()
    result = {
        "source": "game/assets/models/vehicles/f1-2030/source/f1_2030.blend",
        "sha256": hashlib.sha256(Path(bpy.data.filepath).read_bytes()).hexdigest(),
        "conversion": "Godot = (source Y, source Z + 0.1050000228, source X)",
        "corners": {},
    }
    for axle, keys, roles in [
        ("FRONT", ("FL", "FR"), ("upper", "lower", "pushrod", "trackrod")),
        ("REAR", ("RL", "RR"), ("driveshaft", "upper", "lower")),
    ]:
        parts = extract(bpy.data.objects[f"GEO_CHASSIS_{axle}_SUSPENSION"])
        if len(parts) != len(keys) * len(roles):
            raise RuntimeError(f"Source topology changed: inspect {axle} before assigning component roles")
        for index, part in enumerate(parts):
            key = keys[index // len(roles)]
            expected_sign = -1 if key.endswith("L") else 1
            if any(vertex[0] * expected_sign < 0 for vertex in part["vertices"]):
                raise RuntimeError(f"Unexpected source component order: {key}")
            result["corners"].setdefault(key, {})[roles[index % len(roles)]] = part
    for key in ("FL", "FR"):
        anchor = FRONT_PUSHROD[key]
        retarget_front_pushrod(result["corners"][key]["pushrod"], anchor["outer"], anchor["arm"])
    for key, sign in [("RL", 1), ("RR", -1)]:
        def mirror(value):
            return [value[0] * sign, value[1], value[2]]
        part = result["corners"][key]["lower"]
        joint = [-0.505193, -0.0596035, 1.474073]
        append_blade(part, mirror([-0.510495, -0.0596035, 1.347062]), mirror(joint))
        append_blade(part, mirror([-0.073654, -0.1203565, 1.624112]), mirror(joint))
    result["reconstructed"] = [
        "Rear lower outer bridge and rear leg; original front leg retained",
        "Rear inboard pullrod/rocker/damper and toe link are completed in the visual hardpoints, not measured from the source",
        "Front pushrod blades re-anchored onto the physical rocker arm (station rotation + axial stretch about the measured outboard hardpoint); the authored blade targeted the rejected vertical-axis rocker",
    ]
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, separators=(",", ":")), encoding="utf-8")
    print(f"Extracted original suspension profiles: {output}")


if __name__ == "__main__":
    main()
