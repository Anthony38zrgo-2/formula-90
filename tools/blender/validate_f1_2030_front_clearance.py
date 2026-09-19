"""Validate the refined F1 2030 nose, supports and fixed lower plane."""

import json

import bpy
from mathutils.bvhtree import BVHTree


BODY_NAME = "GEO_CHASSIS_BODY"
SUPPORT_NAME = "GEO_CHASSIS_FRONTSUPPORT"
WING_NAME = "GEO_CHASSIS_FRONTWING"


def world_points(obj):
    return [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]


def world_bvh(obj):
    return BVHTree.FromPolygons(
        world_points(obj),
        [tuple(polygon.vertices) for polygon in obj.data.polygons],
        all_triangles=False,
    )


def components(obj):
    adjacency = [set() for _vertex in obj.data.vertices]
    for edge in obj.data.edges:
        a, b = edge.vertices
        adjacency[a].add(b)
        adjacency[b].add(a)
    result = []
    seen = set()
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
        result.append(indices)
    return result


body = bpy.data.objects.get(BODY_NAME)
support = bpy.data.objects.get(SUPPORT_NAME)
wing = bpy.data.objects.get(WING_NAME)
if any(obj is None or obj.type != "MESH" for obj in (body, support, wing)):
    raise RuntimeError("required front assembly meshes are missing")

body_tree = world_bvh(body)
wing_tree = world_bvh(wing)
support_tree = world_bvh(support)
support_points = world_points(support)
component_reports = []
for indices in components(support):
    points = [support_points[index] for index in indices]
    minimum_z = min(point.z for point in points)
    maximum_z = max(point.z for point in points)
    top = [point for point in points if point.z >= maximum_z - 0.004]
    bottom = [point for point in points if point.z <= minimum_z + 0.004]
    top_distances = [body_tree.find_nearest(point, 0.02)[3] for point in top]
    bottom_distances = [wing_tree.find_nearest(point, 0.02)[3] for point in bottom]
    if any(distance is None for distance in top_distances + bottom_distances):
        raise RuntimeError("support cap could not be matched to its anchor surface")
    component_reports.append(
        {
            "center_y_m": sum(point.y for point in points) / len(points),
            "vertical_span_m": maximum_z - minimum_z,
            "maximum_top_surface_distance_m": max(top_distances),
            "maximum_bottom_surface_distance_m": max(bottom_distances),
        }
    )

body_wing_overlaps = len(body_tree.overlap(wing_tree))
support_body_overlaps = len(support_tree.overlap(body_tree))
support_wing_overlaps = len(support_tree.overlap(wing_tree))
symmetry_error = abs(component_reports[0]["vertical_span_m"] - component_reports[1]["vertical_span_m"])
errors = []
if len(component_reports) != 2:
    errors.append(f"expected two supports, found {len(component_reports)}")
if body_wing_overlaps != 0:
    errors.append(f"nose intersects lower plane in {body_wing_overlaps} triangle pairs")
if support_wing_overlaps == 0 and any(
    report["maximum_bottom_surface_distance_m"] > 0.002 for report in component_reports
):
    errors.append("support lower interface is detached")
if any(report["maximum_top_surface_distance_m"] > 0.002 for report in component_reports):
    errors.append("support upper interface is detached")
if symmetry_error > 1.0e-5:
    errors.append(f"support vertical spans are asymmetric by {symmetry_error:.9f} m")
if any(report["maximum_top_surface_distance_m"] > 0.004 for report in component_reports):
    errors.append("support top cap is too far from the nose underside")
if any(report["maximum_bottom_surface_distance_m"] > 0.004 for report in component_reports):
    errors.append("support bottom cap is too far from the lower plane")

report = {
    "passed": not errors,
    "errors": errors,
    "components": component_reports,
    "body_wing_triangle_overlaps": body_wing_overlaps,
    "support_body_triangle_overlaps": support_body_overlaps,
    "support_wing_triangle_overlaps": support_wing_overlaps,
    "support_span_symmetry_error_m": symmetry_error,
}
print("F90_FRONT_CLEARANCE_VALIDATION=" + json.dumps(report, sort_keys=True))
if errors:
    raise SystemExit(1)
