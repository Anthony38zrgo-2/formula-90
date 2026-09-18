"""Give the F1 2030 front wing an R25-inspired mounting and center profile.

Only vertical coordinates are changed on the existing wing and nose geometry,
so the authored planform width and longitudinal length remain exact. The outer
wing and endplates are lifted rigidly; the central elements receive a smooth
local concavity. The original needle-thin center support is rebuilt as two
short blades fitted between the remodeled wing and nose underside.
"""

import bpy
import bmesh
import math
import sys
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree


WING_LIFT_M = 0.045
CENTER_DEPRESSION_M = 0.030
CENTER_BLEND_HALF_WIDTH_M = 0.42
NOSE_UNDERSIDE_LIFT_M = 0.030
NOSE_LOCAL_RAISE_M = 0.035
WING_OBJECTS = {
    "GEO_CHASSIS_FRONTWING",
    "GEO_CHASSIS_ACTIVEFRONT",
    "GEO_CHASSIS_AERO2",
    "GEO_CHASSIS_ENDPLATE",
}
CENTER_PROFILE_OBJECTS = {
    "GEO_CHASSIS_FRONTWING",
    "GEO_CHASSIS_ACTIVEFRONT",
    "GEO_CHASSIS_AERO2",
}
BODY_NAME = "GEO_CHASSIS_BODY"
SUPPORT_NAME = "GEO_CHASSIS_FRONTSUPPORT"
MODIFIED_OBJECTS = WING_OBJECTS | {BODY_NAME, SUPPORT_NAME}


def arguments():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) != 2:
        raise SystemExit("usage: blender --background --python SCRIPT -- INPUT.blend OUTPUT.blend")
    return Path(values[0]).resolve(), Path(values[1]).resolve()


def smoothstep(value):
    t = max(0.0, min(1.0, value))
    return t * t * (3.0 - 2.0 * t)


def center_weight(lateral_m):
    t = abs(lateral_m) / CENTER_BLEND_HALF_WIDTH_M
    if t >= 1.0:
        return 0.0
    return math.cos(t * math.pi * 0.5) ** 2


def move_wing_object(obj):
    inverse = obj.matrix_world.inverted()
    minimum_x = minimum_y = float("inf")
    maximum_x = maximum_y = float("-inf")
    minimum_shift = float("inf")
    maximum_shift = float("-inf")
    for vertex in obj.data.vertices:
        world = obj.matrix_world @ vertex.co
        minimum_x = min(minimum_x, world.x)
        maximum_x = max(maximum_x, world.x)
        minimum_y = min(minimum_y, world.y)
        maximum_y = max(maximum_y, world.y)
        shift = WING_LIFT_M
        if obj.name in CENTER_PROFILE_OBJECTS:
            shift -= CENTER_DEPRESSION_M * center_weight(world.y)
        minimum_shift = min(minimum_shift, shift)
        maximum_shift = max(maximum_shift, shift)
        world.z += shift
        vertex.co = inverse @ world
    obj.data.update()
    obj["f1_2030_front_wing_reference"] = "R25_2005"
    obj["f1_2030_front_wing_lift_m"] = WING_LIFT_M
    if obj.name in CENTER_PROFILE_OBJECTS:
        obj["f1_2030_front_wing_center_depression_m"] = CENTER_DEPRESSION_M
        obj["f1_2030_front_wing_center_blend_half_width_m"] = CENTER_BLEND_HALF_WIDTH_M
    return {
        "planform_bounds_before_after": [minimum_x, maximum_x, minimum_y, maximum_y],
        "minimum_vertical_shift_m": minimum_shift,
        "maximum_vertical_shift_m": maximum_shift,
    }


def remodel_nose_underside(obj):
    inverse = obj.matrix_world.inverted()
    changed = 0
    maximum_shift = 0.0
    minimum_x = minimum_y = float("inf")
    maximum_x = maximum_y = float("-inf")
    for vertex in obj.data.vertices:
        world = obj.matrix_world @ vertex.co
        if world.x >= -2.05 or abs(world.y) >= 0.38:
            continue
        longitudinal = smoothstep((-2.05 - world.x) / 0.45)
        nose_lateral = smoothstep((0.38 - abs(world.y)) / 0.08)
        shift = NOSE_LOCAL_RAISE_M * longitudinal * nose_lateral
        if abs(world.y) < 0.19 and world.z < -0.10:
            underside = smoothstep((-0.10 - world.z) / 0.18)
            underside_lateral = smoothstep((0.19 - abs(world.y)) / 0.07)
            shift += NOSE_UNDERSIDE_LIFT_M * longitudinal * underside * underside_lateral
        if shift <= 1.0e-8:
            continue
        minimum_x = min(minimum_x, world.x)
        maximum_x = max(maximum_x, world.x)
        minimum_y = min(minimum_y, world.y)
        maximum_y = max(maximum_y, world.y)
        maximum_shift = max(maximum_shift, shift)
        world.z += shift
        vertex.co = inverse @ world
        changed += 1
    if changed == 0:
        raise RuntimeError("nose underside selection was empty")
    obj.data.update()
    obj["f1_2030_front_nose_reference"] = "R25_2005"
    obj["f1_2030_front_nose_local_raise_m"] = NOSE_LOCAL_RAISE_M
    obj["f1_2030_front_nose_max_underside_lift_m"] = NOSE_UNDERSIDE_LIFT_M
    return {
        "changed_vertices": changed,
        "maximum_vertical_shift_m": maximum_shift,
        "changed_world_bounds_xy": [minimum_x, maximum_x, minimum_y, maximum_y],
    }


def world_bvh(obj):
    vertices = [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]
    polygons = [tuple(polygon.vertices) for polygon in obj.data.polygons]
    return BVHTree.FromPolygons(vertices, polygons, all_triangles=False)


def ray_surface_z(surface, x, y, origin_z, direction_z):
    hit, _normal, _index, _distance = surface.ray_cast(
        Vector((x, y, origin_z)), Vector((0.0, 0.0, direction_z)), 3.0
    )
    if hit is None:
        raise RuntimeError(f"surface ray missed at x={x:.4f}, y={y:.4f}")
    return hit.z


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


def extract_body_supports(old_support, wing, body):
    candidates = []
    for indices in connected_components(body):
        points = [body.matrix_world @ body.data.vertices[index].co for index in indices]
        bounds_min = [min(point[axis] for point in points) for axis in range(3)]
        bounds_max = [max(point[axis] for point in points) for axis in range(3)]
        if (
            len(indices) == 112
            and bounds_max[0] < -2.50
            and 0.05 < min(abs(bounds_min[1]), abs(bounds_max[1]))
            and max(abs(bounds_min[1]), abs(bounds_max[1])) < 0.12
            and bounds_max[2] < -0.18
        ):
            candidates.append(indices)
    if len(candidates) != 2:
        raise RuntimeError(f"expected two embedded body supports, found {len(candidates)}")

    old_mesh = old_support.data
    bpy.data.objects.remove(old_support, do_unlink=True)
    if old_mesh.users == 0:
        bpy.data.meshes.remove(old_mesh)

    target_indices = {index for component in candidates for index in component}
    support = body.copy()
    support.data = body.data.copy()
    body.users_collection[0].objects.link(support)

    body_mesh = bmesh.new()
    body_mesh.from_mesh(body.data)
    body_mesh.verts.ensure_lookup_table()
    bmesh.ops.delete(
        body_mesh,
        geom=[vertex for vertex in body_mesh.verts if vertex.index in target_indices],
        context="VERTS",
    )
    body_mesh.to_mesh(body.data)
    body_mesh.free()
    body.data.update()

    support_mesh = bmesh.new()
    support_mesh.from_mesh(support.data)
    support_mesh.verts.ensure_lookup_table()
    bmesh.ops.delete(
        support_mesh,
        geom=[vertex for vertex in support_mesh.verts if vertex.index not in target_indices],
        context="VERTS",
    )
    support_mesh.to_mesh(support.data)
    support_mesh.free()
    support.data.update()
    if len(body.data.vertices) != 8177 or len(support.data.vertices) != 224:
        raise RuntimeError(
            "body support extraction produced unexpected parts: "
            + str([len(body.data.vertices), len(support.data.vertices)])
        )

    support.name = SUPPORT_NAME
    support.data.name = SUPPORT_NAME + "_BODY_EXTRACTED_MESH"

    wing_surface = world_bvh(wing)
    inverse = support.matrix_world.inverted()
    adjusted = 0
    overlaps = []
    for indices in connected_components(support):
        points = [support.matrix_world @ support.data.vertices[index].co for index in indices]
        minimum_z = min(point.z for point in points)
        for index in indices:
            vertex = support.data.vertices[index]
            point = support.matrix_world @ vertex.co
            if point.z > minimum_z + 0.006:
                continue
            wing_z = ray_surface_z(wing_surface, point.x, point.y, point.z + 0.01, -1.0)
            point.z = wing_z - 0.001
            vertex.co = inverse @ point
            adjusted += 1
            overlaps.append(point.z - wing_z)
    support.data.update()
    support["f1_2030_front_support_style"] = "body_extracted_twin_pylons"
    support["f1_2030_front_support_overlap_m"] = 0.001
    return {
        "support_count": len(candidates),
        "source": BODY_NAME,
        "vertices": len(support.data.vertices),
        "polygons": len(support.data.polygons),
        "adjusted_bottom_vertices": adjusted,
        "minimum_overlap_m": -max(abs(value) for value in overlaps),
    }


def main():
    input_path, output_path = arguments()
    bpy.ops.wm.open_mainfile(filepath=str(input_path))
    protected = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in bpy.data.objects.items()
        if name not in MODIFIED_OBJECTS
    }
    reports = {}
    for name in WING_OBJECTS:
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != "MESH":
            raise RuntimeError(f"required front-wing mesh missing: {name}")
        reports[name] = move_wing_object(obj)

    body = bpy.data.objects.get(BODY_NAME)
    support = bpy.data.objects.get(SUPPORT_NAME)
    if body is None or body.type != "MESH" or support is None or support.type != "MESH":
        raise RuntimeError("required nose or front support mesh missing")
    reports[BODY_NAME] = remodel_nose_underside(body)
    reports[SUPPORT_NAME] = extract_body_supports(
        support,
        bpy.data.objects["GEO_CHASSIS_FRONTWING"],
        body,
    )

    for name, matrix_values in protected.items():
        current = tuple(round(value, 9) for row in bpy.data.objects[name].matrix_world for value in row)
        if current != matrix_values:
            raise RuntimeError(f"protected object transform changed: {name}")

    scene = bpy.context.scene
    scene["f1_2030_front_wing_reference"] = "C:/Users/bill/AppData/Local/Temp/f1_2005_renault.glb"
    scene["f1_2030_front_wing_scope"] = "vertical-only planform-preserving R25-inspired front-wing and nose remodel"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    for name, report in reports.items():
        print(name, report)


if __name__ == "__main__":
    main()
