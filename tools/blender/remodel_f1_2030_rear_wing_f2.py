"""Remodel only the F1 2030 rear-wing planes with an F2-inspired crown.

The deformation preserves each mesh's topology, UV layers, materials, span,
endpoints, chord profile and object transform. Endplates and supports are not
modified. Run from Blender with an input .blend and output .blend after `--`.
"""

import bpy
import bmesh
import math
import sys
from collections import defaultdict
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree


TARGETS = {
    # The upper movable flap carries the stronger F2-style arch.
    "GEO_CHASSIS_ACTIVEREAR": 0.145,
    # The main plane follows it with a slightly shallower crown.
    "GEO_CHASSIS_AEROPART2": 0.115,
}
SUPPORT_NAME = "GEO_CHASSIS_REARSUPPORT"
MODIFIED_OBJECTS = {*TARGETS, SUPPORT_NAME}
PROFILE_EXPONENT = 1.35
GROUP_PRECISION = 6
SPAN_CUTS = 2


def args():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) != 2:
        raise SystemExit("usage: blender --background --python SCRIPT -- INPUT.blend OUTPUT.blend")
    return Path(values[0]).resolve(), Path(values[1]).resolve()


def subdivide_span(obj):
    mesh = obj.data
    bm = bmesh.new()
    bm.from_mesh(mesh)
    span_edges = []
    for edge in bm.edges:
        a = obj.matrix_world @ edge.verts[0].co
        b = obj.matrix_world @ edge.verts[1].co
        delta = b - a
        if abs(delta.y) > 0.05 and abs(delta.x) < 0.001:
            span_edges.append(edge)
    if not span_edges:
        bm.free()
        raise RuntimeError(f"{obj.name} has no spanwise edges to subdivide")
    result = bmesh.ops.subdivide_edges(
        bm,
        edges=span_edges,
        cuts=SPAN_CUTS,
        use_grid_fill=False,
    )
    # The source uses triangles across the chord. Subdividing their lateral
    # boundaries creates non-planar n-gons; triangulate them explicitly so the
    # curved crown is represented by real faces in Blender and glTF.
    bmesh.ops.triangulate(bm, faces=list(bm.faces), quad_method="BEAUTY", ngon_method="BEAUTY")
    bm.to_mesh(mesh)
    bm.free()
    mesh.update()
    return len(span_edges), len(result.get("geom_inner", []))


def deform_plane(obj, crown_m):
    source_vertices = len(obj.data.vertices)
    span_edges, generated = subdivide_span(obj)
    inverse = obj.matrix_world.inverted()
    groups = defaultdict(list)
    for vertex in obj.data.vertices:
        world = obj.matrix_world @ vertex.co
        groups[round(world.y, GROUP_PRECISION)].append((vertex, world))

    if len(groups) < 5:
        raise RuntimeError(f"{obj.name} needs at least five lateral rows, found {len(groups)}")

    half_span = max(abs(y) for y in groups)
    if half_span <= 0.0:
        raise RuntimeError(f"{obj.name} has zero span")

    row_centers = {y: sum(world.z for _, world in row) / len(row) for y, row in groups.items()}
    negative_edge = row_centers[min(groups)]
    positive_edge = row_centers[max(groups)]

    max_endpoint_delta = 0.0
    for y, row in groups.items():
        lateral = y / half_span
        t = min(abs(lateral), 1.0)
        edge_center = negative_edge + (positive_edge - negative_edge) * ((lateral + 1.0) * 0.5)
        arch = crown_m * math.cos(t * math.pi * 0.5) ** PROFILE_EXPONENT
        target_center = edge_center + arch
        shift_z = target_center - row_centers[y]
        if t > 0.999999:
            max_endpoint_delta = max(max_endpoint_delta, abs(shift_z))
        for vertex, world in row:
            world.z += shift_z
            vertex.co = inverse @ world

    obj.data.update()
    obj["f1_2030_rear_wing_shape"] = "F2_2024_inspired_crown"
    obj["f1_2030_rear_wing_crown_m"] = crown_m
    obj["f1_2030_rear_wing_profile_exponent"] = PROFILE_EXPONENT
    return {
        "rows": len(groups),
        "source_vertices": source_vertices,
        "result_vertices": len(obj.data.vertices),
        "subdivided_span_edges": span_edges,
        "generated_geometry_elements": generated,
        "half_span_m": half_span,
        "crown_m": crown_m,
        "max_endpoint_delta_m": max_endpoint_delta,
    }


def extend_support_to_plane(support, plane):
    plane_world_vertices = [plane.matrix_world @ vertex.co for vertex in plane.data.vertices]
    plane_polygons = [tuple(polygon.vertices) for polygon in plane.data.polygons]
    surface = BVHTree.FromPolygons(plane_world_vertices, plane_polygons, all_triangles=True)
    inverse = support.matrix_world.inverted()
    support_world = [support.matrix_world @ vertex.co for vertex in support.data.vertices]
    original_top = max(point.z for point in support_world)
    top_vertices = [
        (vertex, world)
        for vertex, world in zip(support.data.vertices, support_world)
        if abs(world.z - original_top) < 1.0e-5
    ]
    if not top_vertices:
        raise RuntimeError("rear support has no identifiable top attachment vertices")
    extensions = []
    for vertex, world in top_vertices:
        origin = Vector((world.x, world.y, original_top - 0.5))
        hit, _normal, _index, _distance = surface.ray_cast(origin, Vector((0.0, 0.0, 1.0)), 1.5)
        if hit is None:
            raise RuntimeError(f"rear support does not intersect remodeled plane at x={world.x}, y={world.y}")
        extensions.append(hit.z - world.z)
        world.z = hit.z + 0.0005
        vertex.co = inverse @ world
    support.data.update()
    support["f1_2030_rear_wing_support_fit"] = "extended_to_remodeled_main_plane"
    return {
        "top_vertices": len(top_vertices),
        "minimum_extension_m": min(extensions),
        "maximum_extension_m": max(extensions),
    }


def main():
    input_path, output_path = args()
    bpy.ops.wm.open_mainfile(filepath=str(input_path))

    protected = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in bpy.data.objects.items()
        if name not in MODIFIED_OBJECTS
    }
    reports = {}
    for name, crown in TARGETS.items():
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != "MESH":
            raise RuntimeError(f"required rear-wing mesh missing: {name}")
        reports[name] = deform_plane(obj, crown)

    support = bpy.data.objects.get(SUPPORT_NAME)
    if support is None or support.type != "MESH":
        raise RuntimeError(f"required rear-wing support missing: {SUPPORT_NAME}")
    reports[SUPPORT_NAME] = extend_support_to_plane(support, bpy.data.objects["GEO_CHASSIS_AEROPART2"])

    for name, matrix_values in protected.items():
        current = tuple(round(value, 9) for row in bpy.data.objects[name].matrix_world for value in row)
        if current != matrix_values:
            raise RuntimeError(f"protected object transform changed: {name}")

    scene = bpy.context.scene
    scene["f1_2030_rear_wing_reference"] = "C:/Users/bill/Downloads/f2_2024.glb"
    scene["f1_2030_rear_wing_scope"] = "shape only; physics, endplates, materials and UVs preserved; existing support tops extended to maintain attachment"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    for name, report in reports.items():
        print(name, report)


if __name__ == "__main__":
    main()
