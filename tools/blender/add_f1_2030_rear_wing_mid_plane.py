"""Build the complete fixed intermediate rear-wing plane for F1 2030.

The generated wing is continuous from end to end and has a shallow-W spanwise
crown: a raised center apex, a soft valley around half span and tips lifted to
the endplate mid-height so the arms meet the endplates almost horizontally. It
matches the measured chord inclination of AeroPart3. Its tips are projected
onto the endplate inner walls, trimming any excess that would clip. Running the
script again replaces only the generated intermediate plane.
"""

from __future__ import annotations

import json
import math

import bpy
from mathutils import Euler, Matrix, Vector
from mathutils.bvhtree import BVHTree


SOURCE_NAME = "GEO_CHASSIS_REAR_WING"
BEAM_NAME = "GEO_CHASSIS_AEROPART3"
TARGET_NAME = "GEO_CHASSIS_REAR_WING_MID"

# The endpoint profile is projected against the existing lateral surfaces.
# A 0.2 mm inset prevents numerical penetration while remaining visually joined.
SIDE_OBJECT_NAMES = (SOURCE_NAME, "GEO_CHASSIS_ENDPLATE")
SIDE_CONTACT_INSET_M = 0.0002
CHORD_M = 0.220
THICKNESS_RATIO = 0.115
CAMBER_RATIO = 0.020
# Shallow-W crown in the build frame: raised center apex, a subtle valley around
# half span, and tips lifted to the endplate mid-height so the arms meet the
# endplates almost horizontally (zero slope at both the center and the tips).
TIP_CENTER_Z_M = 0.139
CENTER_CENTER_Z_M = 0.260
W_DIP_FACTOR = 0.5
CHORD_CENTER_X_M = 2.260
SPAN_SEGMENTS = 64
PROFILE_SEGMENTS = 32

# Manual pose authored by the user in f1_2030.blend (rotation plus the final
# raised position). It is codified here so reruns reproduce the exact same
# placement, and the lateral tip projection samples the endplate inner walls at
# the final height.
TARGET_ROTATION_DEG = (
    0.40144308780645327,
    -1.4034940578003547,
    0.07669287247687054,
)
TARGET_TRANSLATION_M = (0.0, 0.0, -0.120784)


def world_bounds(obj: bpy.types.Object) -> list[list[float]]:
    points = [obj.matrix_world @ Vector(corner) for corner in obj.bound_box]
    return [[min(point[axis] for point in points), max(point[axis] for point in points)] for axis in range(3)]


def measured_chord_angle(obj: bpy.types.Object) -> float:
    """Return the principal X/Z inclination of the beam wing in radians."""
    points = [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]
    mean_x = sum(point.x for point in points) / len(points)
    mean_z = sum(point.z for point in points) / len(points)
    covariance_xx = sum((point.x - mean_x) ** 2 for point in points) / len(points)
    covariance_zz = sum((point.z - mean_z) ** 2 for point in points) / len(points)
    covariance_xz = sum((point.x - mean_x) * (point.z - mean_z) for point in points) / len(points)
    return 0.5 * math.atan2(2.0 * covariance_xz, covariance_xx - covariance_zz)


def crown_height(normalized: float) -> float:
    normalized = min(abs(normalized), 1.0)
    # Shallow W: smoothstep gets the center and tip heights with zero slope at
    # both ends, and the sin^2 dip pushes the arms below the tip level so the
    # spanwise profile valleys softly around half span before rising into the
    # endplates.
    rounded = normalized * normalized * (3.0 - 2.0 * normalized)
    dip = W_DIP_FACTOR * math.sin(math.pi * normalized) ** 2
    return CENTER_CENTER_Z_M - (CENTER_CENTER_Z_M - TIP_CENTER_Z_M) * (rounded + dip)


def world_bvh(obj: bpy.types.Object) -> BVHTree:
    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        vertices = [evaluated.matrix_world @ vertex.co for vertex in mesh.vertices]
        polygons = [tuple(poly.vertices) for poly in mesh.polygons]
        return BVHTree.FromPolygons(vertices, polygons, all_triangles=False)
    finally:
        evaluated.to_mesh_clear()


def profile_point(center_z: float, profile_index: int, chord_angle: float) -> tuple[float, float]:
    profile_angle = 2.0 * math.pi * profile_index / PROFILE_SEGMENTS
    local_x = CHORD_M * 0.5 * math.cos(profile_angle)
    normalized_x = local_x / (CHORD_M * 0.5)
    local_z = CHORD_M * THICKNESS_RATIO * 0.5 * math.sin(profile_angle)
    local_z += CHORD_M * CAMBER_RATIO * (1.0 - normalized_x * normalized_x)
    cosine = math.cos(chord_angle)
    sine = math.sin(chord_angle)
    x = CHORD_CENTER_X_M + local_x * cosine - local_z * sine
    z = center_z + local_x * sine + local_z * cosine
    return x, z


def lateral_contact_profiles(chord_angle: float, target_matrix_world) -> tuple[list[float], list[float], dict[str, object]]:
    bvhs = []
    for name in SIDE_OBJECT_NAMES:
        obj = bpy.data.objects.get(name)
        if obj is not None and obj.type == "MESH":
            bvhs.append((name, world_bvh(obj)))
    if not bvhs:
        raise RuntimeError("No lateral rear-wing surfaces available for endpoint projection")

    left: list[float] = []
    right: list[float] = []
    hits_by_object = {name: 0 for name, _ in bvhs}
    tip_z = crown_height(1.0)
    for profile_index in range(PROFILE_SEGMENTS):
        x, z = profile_point(tip_z, profile_index, chord_angle)
        for sign, destination in ((-1.0, left), (1.0, right)):
            origin = target_matrix_world @ Vector((x, 0.0, z))
            direction = (target_matrix_world.to_3x3() @ Vector((0.0, sign, 0.0))).normalized()
            candidates = []
            for name, tree in bvhs:
                location, _normal, _index, distance = tree.ray_cast(origin, direction, 1.5)
                if location is not None:
                    candidates.append((distance, location.y, name))
            if not candidates:
                raise RuntimeError(
                    f"Could not project lateral contact at profile={profile_index}, side={sign:+.0f}, x={x:.6f}, z={z:.6f}"
                )
            _distance, hit_y, hit_name = min(candidates, key=lambda item: item[0])
            hits_by_object[hit_name] += 1
            destination.append(sign * (_distance - SIDE_CONTACT_INSET_M))

    metrics = {
        "contact_inset_m": SIDE_CONTACT_INSET_M,
        "left_contact_y_range_m": [min(left), max(left)],
        "right_contact_y_range_m": [min(right), max(right)],
        "contact_rays_by_object": hits_by_object,
    }
    return left, right, metrics


def remove_existing_target() -> None:
    existing = bpy.data.objects.get(TARGET_NAME)
    if existing is None:
        return
    mesh = existing.data if existing.type == "MESH" else None
    bpy.data.objects.remove(existing, do_unlink=True)
    if mesh is not None and mesh.users == 0:
        bpy.data.meshes.remove(mesh)


def resolve_lateral_penetration(target, step: float = 0.002, max_steps: int = 200) -> dict[str, object]:
    """Shrink the span until no triangle crosses the endplate inner walls.

    Point projection alone is not enough: the endplate wall is concave between
    sampled profile points, so flat cap facets can cross it even with every
    vertex inset. A uniform span scale keeps the wing smooth while trimming
    the excess that clips.
    """

    side_trees = []
    for name in SIDE_OBJECT_NAMES:
        obj = bpy.data.objects.get(name)
        if obj is not None and obj.type == "MESH":
            side_trees.append(world_bvh(obj))
    if not side_trees:
        raise RuntimeError("No lateral surfaces available for penetration trimming")

    def mid_tree() -> BVHTree:
        vertices = [target.matrix_world @ vertex.co for vertex in target.data.vertices]
        polygons = []
        for polygon in target.data.polygons:
            indices = list(polygon.vertices)
            for index in range(1, len(indices) - 1):
                polygons.append((indices[0], indices[index], indices[index + 1]))
        return BVHTree.FromPolygons(vertices, polygons, all_triangles=True)

    tree = mid_tree()
    for iteration in range(max_steps + 1):
        overlaps = sum(len(tree.overlap(other)) for other in side_trees)
        if overlaps == 0:
            span_scale = 1.0 - step * iteration
            target["f1_2030_mid_trim_iterations"] = iteration
            target["f1_2030_mid_trim_span_scale"] = span_scale
            return {"trim_iterations": iteration, "trim_span_scale": span_scale, "trim_overlaps": 0}
        for vertex in target.data.vertices:
            vertex.co.y *= 1.0 - step
        target.data.update()
        tree = mid_tree()
    raise RuntimeError("Could not trim the intermediate plane out of the endplates")


def build_complete_mesh(chord_angle: float, target_matrix_world) -> tuple[bpy.types.Mesh, dict[str, object]]:
    vertices: list[tuple[float, float, float]] = []
    faces: list[tuple[int, ...]] = []
    left_contacts, right_contacts, contact_metrics = lateral_contact_profiles(chord_angle, target_matrix_world)

    for span_index in range(SPAN_SEGMENTS + 1):
        span_normalized = -1.0 + 2.0 * span_index / SPAN_SEGMENTS
        center_z = crown_height(span_normalized)
        for profile_index in range(PROFILE_SEGMENTS):
            x, z = profile_point(center_z, profile_index, chord_angle)
            if span_normalized < 0.0:
                y = -span_normalized * left_contacts[profile_index]
            else:
                y = span_normalized * right_contacts[profile_index]
            vertices.append((x, y, z))

    for span_index in range(SPAN_SEGMENTS):
        current = span_index * PROFILE_SEGMENTS
        following = (span_index + 1) * PROFILE_SEGMENTS
        for profile_index in range(PROFILE_SEGMENTS):
            next_profile = (profile_index + 1) % PROFILE_SEGMENTS
            faces.append(
                (
                    current + profile_index,
                    current + next_profile,
                    following + next_profile,
                    following + profile_index,
                )
            )

    left_center = len(vertices)
    vertices.append((CHORD_CENTER_X_M, max(left_contacts), crown_height(1.0)))
    right_center = len(vertices)
    vertices.append((CHORD_CENTER_X_M, min(right_contacts), crown_height(1.0)))
    right_start = SPAN_SEGMENTS * PROFILE_SEGMENTS
    for profile_index in range(PROFILE_SEGMENTS):
        next_profile = (profile_index + 1) % PROFILE_SEGMENTS
        faces.append((left_center, next_profile, profile_index))
        faces.append((right_center, right_start + profile_index, right_start + next_profile))

    mesh = bpy.data.meshes.new(f"{TARGET_NAME}_MESH")
    mesh.from_pydata(vertices, [], faces)
    mesh.update(calc_edges=True)

    uv_layer = mesh.uv_layers.new(name="UVMap")
    for polygon in mesh.polygons:
        polygon.use_smooth = polygon.loop_total == 4
        for loop_index in polygon.loop_indices:
            vertex_index = mesh.loops[loop_index].vertex_index
            if vertex_index >= (SPAN_SEGMENTS + 1) * PROFILE_SEGMENTS:
                uv_layer.data[loop_index].uv = (0.5, 0.0 if vertex_index == left_center else 1.0)
                continue
            span_index, profile_index = divmod(vertex_index, PROFILE_SEGMENTS)
            uv_layer.data[loop_index].uv = (
                profile_index / PROFILE_SEGMENTS,
                span_index / SPAN_SEGMENTS,
            )

    metrics = {
        "vertices": len(vertices),
        "polygons": len(faces),
        "half_span_m": max(max(abs(value) for value in left_contacts), max(abs(value) for value in right_contacts)),
        "chord_m": CHORD_M,
        "thickness_ratio": THICKNESS_RATIO,
        "camber_ratio": CAMBER_RATIO,
        "tip_center_z_m": TIP_CENTER_Z_M,
        "center_center_z_m": CENTER_CENTER_Z_M,
        "crown_rise_m": CENTER_CENTER_Z_M - TIP_CENTER_Z_M,
        "crown_law": "shallow_w",
        "w_dip_factor": W_DIP_FACTOR,
        "chord_angle_deg": math.degrees(chord_angle),
        **contact_metrics,
    }
    return mesh, metrics


def build_mid_plane() -> dict[str, object]:
    source = bpy.data.objects.get(SOURCE_NAME)
    beam = bpy.data.objects.get(BEAM_NAME)
    if source is None or source.type != "MESH":
        raise RuntimeError(f"Missing mesh object {SOURCE_NAME}")
    if beam is None or beam.type != "MESH":
        raise RuntimeError(f"Missing mesh object {BEAM_NAME}")

    chord_angle = measured_chord_angle(beam)
    target_matrix_world = Matrix.Translation(TARGET_TRANSLATION_M) @ Euler(
        tuple(math.radians(value) for value in TARGET_ROTATION_DEG),
        "XYZ",
    ).to_matrix().to_4x4()
    remove_existing_target()
    mesh, metrics = build_complete_mesh(chord_angle, target_matrix_world)
    target = bpy.data.objects.new(TARGET_NAME, mesh)
    bpy.context.collection.objects.link(target)
    target.parent = source.parent
    target.matrix_world = target_matrix_world
    if source.data.materials:
        mesh.materials.append(source.data.materials[0])
    metrics.update(resolve_lateral_penetration(target))

    target["formula90_role"] = "fixed_rear_wing_intermediate_plane"
    target["source_angle_object"] = BEAM_NAME
    target["continuous_profile"] = True
    target["shallow_w_crown"] = True
    target["drs"] = False
    for key, value in metrics.items():
        target[key] = value

    return {
        "object": TARGET_NAME,
        **metrics,
        "bounds": world_bounds(target),
        "beam_bounds": world_bounds(beam),
        "materials": [material.name if material else None for material in mesh.materials],
    }


report = build_mid_plane()
bpy.ops.wm.save_as_mainfile(filepath=bpy.data.filepath)
print("F90_REAR_WING_MID=" + json.dumps(report, sort_keys=True))
