"""Extract source suspension profiles, preserving the .blend unchanged.

blender --background f1_2030.blend --python SCRIPT -- OUTPUT.json
Coordinate conversion matches export_f1_2030_v10.py.
"""
import hashlib
import json
import sys
from pathlib import Path

import bpy
sys.path.insert(0,str(Path(__file__).resolve().parent))
from formula_one_2030_object_names import enable_legacy_source_names
enable_legacy_source_names()
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


def retarget_front_pushrod(component_mesh, outboard_attachment, rocker_attachment):
    unique_positions = [Vector(position) for position in sorted(set(tuple(position) for position in component_mesh["vertices"]))]
    minimum_lateral_distance = min(abs(position.x) for position in unique_positions)
    maximum_lateral_distance = max(abs(position.x) for position in unique_positions)
    endpoint_band = (maximum_lateral_distance - minimum_lateral_distance) * 0.02
    inboard_ring = [position for position in unique_positions if abs(position.x) <= minimum_lateral_distance + endpoint_band]
    outboard_ring = [position for position in unique_positions if abs(position.x) >= maximum_lateral_distance - endpoint_band]
    if len(inboard_ring) != 4 or len(outboard_ring) != 4:
        raise RuntimeError("Front pushrod must have two measured four-corner attachment rings")
    measured_inboard_center = sum(inboard_ring, Vector()) / len(inboard_ring)
    measured_outboard_center = sum(outboard_ring, Vector()) / len(outboard_ring)
    original_axis = measured_inboard_center - measured_outboard_center
    target_outboard_center = Vector(outboard_attachment)
    target_axis = Vector(rocker_attachment) - target_outboard_center
    if original_axis.length < 1e-9 or target_axis.length < 1e-9:
        raise RuntimeError("Degenerate front pushrod attachment axis")
    original_direction = original_axis.normalized()
    target_direction = target_axis.normalized()
    alignment_rotation = original_direction.rotation_difference(target_direction).to_matrix()
    longitudinal_scale = target_axis.length / original_axis.length
    remapped_positions = []
    for position in component_mesh["vertices"]:
        rotated_offset = alignment_rotation @ (Vector(position) - measured_outboard_center)
        longitudinal_offset = target_direction * rotated_offset.dot(target_direction)
        remapped_position = target_outboard_center + longitudinal_offset * longitudinal_scale + rotated_offset - longitudinal_offset
        remapped_positions.append([round(coordinate, 7) for coordinate in remapped_position])
    component_mesh["vertices"] = remapped_positions


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
        parts = extract(bpy.data.objects[f"GEO_SUSPENSION_{axle}_LINK_ASSEMBLY"])
        if len(parts) != len(keys) * len(roles):
            raise RuntimeError(f"Source topology changed: inspect {axle} before assigning component roles")
        for index, part in enumerate(parts):
            key = keys[index // len(roles)]
            expected_sign = -1 if key.endswith("L") else 1
            if any(vertex[0] * expected_sign < 0 for vertex in part["vertices"]):
                raise RuntimeError(f"Unexpected source component order: {key}")
            result["corners"].setdefault(key, {})[roles[index % len(roles)]] = part
    physical_profile_path = Path(__file__).resolve().parents[2] / "game/data/vehicles/f1_2030/profiles/geometric/suspension.json"
    result["physical_suspension_source"] = physical_profile_path.relative_to(Path(__file__).resolve().parents[2]).as_posix()
    result["physical_suspension_source_digest"] = hashlib.sha256(physical_profile_path.read_bytes()).hexdigest()
    physical_corners = json.loads(physical_profile_path.read_text(encoding="utf-8"))["suspension"]["geometry_physical"]["corners"]
    for wheel_key in ("FL", "FR"):
        physical_corner = physical_corners[wheel_key]
        mirrored_corner = "mirror_of" in physical_corner
        if mirrored_corner:
            physical_corner = physical_corners[physical_corner["mirror_of"]]
        target_outboard_attachment = list(physical_corner["rod"]["outer"])
        target_rocker_attachment = list(physical_corner["rocker"]["pushrod_arm"])
        if mirrored_corner:
            target_outboard_attachment[0] *= -1
            target_rocker_attachment[0] *= -1
        retarget_front_pushrod(result["corners"][wheel_key]["pushrod"], target_outboard_attachment, target_rocker_attachment)
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
        "Front pushrod attachment rings measured from the source surface and mapped onto the current physical suspension profile; rotation and longitudinal scaling preserve the authored blade section",
    ]
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, separators=(",", ":")), encoding="utf-8")
    print(f"Extracted original suspension profiles: {output}")


if __name__ == "__main__":
    main()
