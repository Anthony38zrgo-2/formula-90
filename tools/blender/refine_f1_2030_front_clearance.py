"""Raise the F1 2030 nose and deepen the fixed lower front-wing plane.

This is an incremental refinement of the existing R25-inspired front assembly.
Only GEO_CHASSIS_BODY, GEO_CHASSIS_FRONTSUPPORT and GEO_CHASSIS_FRONTWING are
edited. The lower plane keeps its object transform and outer position; its
central span is depressed smoothly to add concavity and clearance.
"""

import json
import math
import sys
from pathlib import Path

import bpy


BODY_NAME = "GEO_CHASSIS_BODY"
SUPPORT_NAME = "GEO_CHASSIS_FRONTSUPPORT"
LOWER_PLANE_NAME = "GEO_CHASSIS_FRONTWING"
MODIFIED_OBJECTS = {BODY_NAME, SUPPORT_NAME, LOWER_PLANE_NAME}

REVISION = "nose_up_lower_plane_concave_v2"
NOSE_BLEND_START_X = -1.85
NOSE_BLEND_LENGTH_M = 0.92
NOSE_EXTRA_TIP_RISE_M = 0.025
NOSE_FULL_WIDTH_M = 0.20
NOSE_BLEND_HALF_WIDTH_M = 0.34
LOWER_PLANE_EXTRA_DEPRESSION_M = 0.018
LOWER_PLANE_BLEND_HALF_WIDTH_M = 0.44
SUPPORT_TOP_OVERLAP_M = 0.001


def arguments():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) != 2:
        raise SystemExit("usage: blender --background --python SCRIPT -- INPUT.blend OUTPUT.blend")
    return Path(values[0]).resolve(), Path(values[1]).resolve()


def smoothstep(value):
    t = max(0.0, min(1.0, value))
    return t * t * (3.0 - 2.0 * t)


def nose_lift(x, y):
    if x >= NOSE_BLEND_START_X or abs(y) >= NOSE_BLEND_HALF_WIDTH_M:
        return 0.0
    longitudinal = smoothstep((NOSE_BLEND_START_X - x) / NOSE_BLEND_LENGTH_M)
    if abs(y) <= NOSE_FULL_WIDTH_M:
        lateral = 1.0
    else:
        lateral = smoothstep(
            (NOSE_BLEND_HALF_WIDTH_M - abs(y))
            / (NOSE_BLEND_HALF_WIDTH_M - NOSE_FULL_WIDTH_M)
        )
    return NOSE_EXTRA_TIP_RISE_M * longitudinal * lateral


def lower_plane_depression(y):
    normalized = abs(y) / LOWER_PLANE_BLEND_HALF_WIDTH_M
    if normalized >= 1.0:
        return 0.0
    return -LOWER_PLANE_EXTRA_DEPRESSION_M * math.cos(normalized * math.pi * 0.5) ** 2


def world_bounds(obj):
    points = [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]
    return [[min(point[axis] for point in points), max(point[axis] for point in points)] for axis in range(3)]


def connected_components(obj):
    adjacency = [set() for _vertex in obj.data.vertices]
    for edge in obj.data.edges:
        a, b = edge.vertices
        adjacency[a].add(b)
        adjacency[b].add(a)
    seen = set()
    components = []
    for start in range(len(obj.data.vertices)):
        if start in seen:
            continue
        stack = [start]
        seen.add(start)
        indices = []
        while stack:
            current = stack.pop()
            indices.append(current)
            for neighbor in adjacency[current]:
                if neighbor not in seen:
                    seen.add(neighbor)
                    stack.append(neighbor)
        components.append(indices)
    return components


def raise_nose(body):
    before = world_bounds(body)
    inverse = body.matrix_world.inverted()
    changed = 0
    maximum = 0.0
    for vertex in body.data.vertices:
        point = body.matrix_world @ vertex.co
        shift = nose_lift(point.x, point.y)
        if shift <= 1.0e-9:
            continue
        point.z += shift
        vertex.co = inverse @ point
        changed += 1
        maximum = max(maximum, shift)
    if changed == 0:
        raise RuntimeError("nose refinement selection was empty")
    body.data.update()
    body["f1_2030_nose_clearance_revision"] = REVISION
    body["f1_2030_nose_extra_tip_rise_m"] = NOSE_EXTRA_TIP_RISE_M
    return {
        "changed_vertices": changed,
        "maximum_tip_rise_m": maximum,
        "bounds_before": before,
        "bounds_after": world_bounds(body),
    }


def deepen_lower_plane(wing):
    before = world_bounds(wing)
    inverse = wing.matrix_world.inverted()
    changed = 0
    minimum = 0.0
    for vertex in wing.data.vertices:
        point = wing.matrix_world @ vertex.co
        shift = lower_plane_depression(point.y)
        if abs(shift) <= 1.0e-9:
            continue
        point.z += shift
        vertex.co = inverse @ point
        changed += 1
        minimum = min(minimum, shift)
    if changed == 0:
        raise RuntimeError("lower-plane concavity selection was empty")
    wing.data.update()
    wing["f1_2030_lower_plane_clearance_revision"] = REVISION
    wing["f1_2030_lower_plane_extra_center_depression_m"] = LOWER_PLANE_EXTRA_DEPRESSION_M
    wing["f1_2030_lower_plane_outer_position_preserved"] = True
    return {
        "changed_vertices": changed,
        "maximum_center_depression_m": abs(minimum),
        "bounds_before": before,
        "bounds_after": world_bounds(wing),
    }


def stretch_supports(support):
    before = world_bounds(support)
    inverse = support.matrix_world.inverted()
    components = connected_components(support)
    if len(components) != 2:
        raise RuntimeError(f"expected two front-support components, found {len(components)}")
    clearance_increases = []
    for indices in components:
        points = {index: support.matrix_world @ support.data.vertices[index].co for index in indices}
        minimum_z = min(point.z for point in points.values())
        maximum_z = max(point.z for point in points.values())
        span = maximum_z - minimum_z
        if span <= 1.0e-6:
            raise RuntimeError("front-support component has no vertical span")
        for index, point in points.items():
            top_weight = (point.z - minimum_z) / span
            top_shift = nose_lift(point.x, point.y)
            bottom_shift = lower_plane_depression(point.y)
            point.z += (
                bottom_shift * (1.0 - top_weight)
                + top_shift * top_weight
                + SUPPORT_TOP_OVERLAP_M * top_weight
            )
            support.data.vertices[index].co = inverse @ point
        center = sum((points[index] for index in indices), points[indices[0]] * 0.0) / len(indices)
        clearance_increases.append(nose_lift(center.x, center.y) - lower_plane_depression(center.y))
    support.data.update()
    support["f1_2030_front_support_clearance_revision"] = REVISION
    support["f1_2030_front_support_clearance_increase_m"] = sum(clearance_increases) / len(clearance_increases)
    support["f1_2030_front_support_top_overlap_m"] = SUPPORT_TOP_OVERLAP_M
    return {
        "components": len(components),
        "average_clearance_increase_m": sum(clearance_increases) / len(clearance_increases),
        "bounds_before": before,
        "bounds_after": world_bounds(support),
    }


def main():
    input_path, output_path = arguments()
    bpy.ops.wm.open_mainfile(filepath=str(input_path))
    if bpy.context.scene.get("f1_2030_front_clearance_revision") == REVISION:
        raise RuntimeError(f"refinement {REVISION} is already applied")

    objects = {}
    for name in MODIFIED_OBJECTS:
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != "MESH":
            raise RuntimeError(f"required mesh missing: {name}")
        objects[name] = obj

    protected_transforms = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in bpy.data.objects.items()
        if name not in MODIFIED_OBJECTS
    }
    edited_transforms = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in objects.items()
    }

    reports = {
        BODY_NAME: raise_nose(objects[BODY_NAME]),
        LOWER_PLANE_NAME: deepen_lower_plane(objects[LOWER_PLANE_NAME]),
        SUPPORT_NAME: stretch_supports(objects[SUPPORT_NAME]),
    }

    for name, matrix_values in {**protected_transforms, **edited_transforms}.items():
        current = tuple(round(value, 9) for row in bpy.data.objects[name].matrix_world for value in row)
        if current != matrix_values:
            raise RuntimeError(f"object transform changed unexpectedly: {name}")

    bpy.context.scene["f1_2030_front_clearance_revision"] = REVISION
    bpy.context.scene["f1_2030_front_clearance_scope"] = (
        "natural incremental nose pitch, stretched twin supports, fixed-position lower plane with extra center concavity"
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    print("F90_FRONT_CLEARANCE=" + json.dumps(reports, sort_keys=True))


if __name__ == "__main__":
    main()
