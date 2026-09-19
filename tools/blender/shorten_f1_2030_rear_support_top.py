"""Shorten the F1 2030 rear support so its arm grips the intermediate plane.

The original swan neck carries the upper wing with its rear arm resting on the
upper plane top, embedded a few millimetres (the authored grip). This script
optionally restores that original mesh from a donor blend and then compresses
the support vertically about its floor base until the arm grips the TOP of
GEO_CHASSIS_REAR_WING_MID with the same embed, leaving the arm shape intact.

Usage:
  blender --background --python shorten_f1_2030_rear_support_top.py -- TARGET.blend OUTPUT.blend [DONOR.blend]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree


SUPPORT_NAME = "GEO_CHASSIS_REARSUPPORT"
PLANE_NAME = "GEO_CHASSIS_REAR_WING_MID"
UPPER_NAME = "GEO_CHASSIS_REAR_WING"
BEAM_NAME = "GEO_CHASSIS_AEROPART3"
ENDPLATE_NAME = "GEO_CHASSIS_ENDPLATE"
EXPECTED_TOPOLOGY = (1752, 1730)
CONTACT_EMBED_M = 0.008
SCALE_MIN = 0.20
SCALE_MAX = 1.00
SCALE_ITERATIONS = 60
ARM_MIN_ABOVE_PLANE_M = 0.02


def arguments():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) not in (2, 3):
        raise SystemExit(
            "usage: blender --background --python shorten_f1_2030_rear_support_top.py -- TARGET.blend OUTPUT.blend [DONOR.blend]"
        )
    target = Path(values[0]).resolve()
    output = Path(values[1]).resolve()
    donor = Path(values[2]).resolve() if len(values) == 3 else None
    return target, output, donor


def world_points(obj):
    return [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]


def world_bvh(obj):
    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        vertices = [evaluated.matrix_world @ vertex.co for vertex in mesh.vertices]
        polygons = [tuple(polygon.vertices) for polygon in mesh.polygons]
        return BVHTree.FromPolygons(vertices, polygons, all_triangles=False)
    finally:
        evaluated.to_mesh_clear()


def restore_support_from_donor(support, donor_path):
    with bpy.data.libraries.load(str(donor_path), link=False) as (data_from, data_to):
        if SUPPORT_NAME not in data_from.objects:
            raise RuntimeError(f"donor {donor_path} has no {SUPPORT_NAME}")
        data_to.objects = [SUPPORT_NAME]
    donor = data_to.objects[0]
    donor_matrix = donor.matrix_world.copy()
    donor_points = [donor_matrix @ vertex.co for vertex in donor.data.vertices]
    if (len(support.data.vertices), len(support.data.polygons)) != (
        len(donor.data.vertices),
        len(donor.data.polygons),
    ):
        raise RuntimeError("donor support topology does not match the target")
    inverse = support.matrix_world.inverted()
    for vertex, point in zip(support.data.vertices, donor_points):
        vertex.co = inverse @ point
    support.data.update()
    donor_mesh = donor.data
    bpy.data.objects.remove(donor, do_unlink=True)
    if donor_mesh.users == 0:
        bpy.data.meshes.remove(donor_mesh)


def main():
    target_path, output_path, donor_path = arguments()
    bpy.ops.wm.open_mainfile(filepath=str(target_path))
    support = bpy.data.objects.get(SUPPORT_NAME)
    plane = bpy.data.objects.get(PLANE_NAME)
    if support is None or support.type != "MESH":
        raise RuntimeError(f"required rear support missing: {SUPPORT_NAME}")
    if plane is None or plane.type != "MESH":
        raise RuntimeError(f"required intermediate plane missing: {PLANE_NAME}")
    if (len(support.data.vertices), len(support.data.polygons)) != EXPECTED_TOPOLOGY:
        raise RuntimeError(
            "rear support topology changed from the expected base: "
            + str((len(support.data.vertices), len(support.data.polygons)))
        )

    protected = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in bpy.data.objects.items()
        if name != SUPPORT_NAME
    }

    restored_from = None
    if donor_path is not None:
        restore_support_from_donor(support, donor_path)
        restored_from = str(donor_path)
    before = world_points(support)

    plane_tree = world_bvh(plane)
    plane_points = world_points(plane)
    plane_top_max = max(p.z for p in plane_points)
    plane_x = (min(p.x for p in plane_points), max(p.x for p in plane_points))

    def top_at(x, y):
        hit = plane_tree.ray_cast(
            Vector((x, y, plane_top_max + 0.5)), Vector((0.0, 0.0, -1.0)), 2.5
        )
        if hit[0] is None:
            return None
        return hit[0].z

    z_min = min(p.z for p in before)
    candidates = [
        (index, p.x, p.y, p.z)
        for index, p in enumerate(before)
        if p.z > plane_top_max + ARM_MIN_ABOVE_PLANE_M
        and plane_x[0] - 0.01 <= p.x <= plane_x[1] + 0.01
        and abs(p.y) <= 0.06
    ]
    if not candidates:
        raise RuntimeError("no rear-support arm geometry over the intermediate plane")

    def min_gap(scale):
        gaps = []
        for _index, x, y, z in candidates:
            top = top_at(x, y)
            if top is None:
                continue
            gaps.append(z_min + scale * (z - z_min) - top)
        if not gaps:
            raise RuntimeError("no surface samples under the support arm")
        return min(gaps)

    low, high = SCALE_MIN, SCALE_MAX
    if min_gap(high) + CONTACT_EMBED_M <= 0.0:
        raise RuntimeError("support already embeds at full height; refusing to guess")
    if min_gap(low) + CONTACT_EMBED_M >= 0.0:
        raise RuntimeError("support cannot reach the intermediate plane within the tested range")
    scale = low
    for _ in range(SCALE_ITERATIONS):
        scale = (low + high) * 0.5
        if min_gap(scale) + CONTACT_EMBED_M > 0.0:
            high = scale
        else:
            low = scale

    inverse = support.matrix_world.inverted()
    for vertex, point in zip(support.data.vertices, before):
        vertex.co = inverse @ Vector((point.x, point.y, z_min + scale * (point.z - z_min)))
    support.data.update()

    for name, matrix_values in protected.items():
        current = tuple(round(value, 9) for row in bpy.data.objects[name].matrix_world for value in row)
        if current != matrix_values:
            raise RuntimeError(f"protected object transform changed: {name}")

    final = world_points(support)
    base_indices = [index for index, p in enumerate(before) if p.z <= z_min + 1e-6]
    base_drift = max(
        (abs(final[index].z - before[index].z) for index in base_indices), default=0.0
    )
    near_base_shift = max(
        (
            abs(final[index].z - before[index].z)
            for index, p in enumerate(before)
            if p.z < z_min + 0.001
        ),
        default=0.0,
    )
    gaps = []
    for index, x, y, _z in candidates:
        top = top_at(final[index].x, final[index].y)
        if top is not None:
            gaps.append(final[index].z - top)
    min_final_gap = min(gaps) if gaps else float("nan")

    overlaps = {}
    support_tree = world_bvh(support)
    for name in (UPPER_NAME, BEAM_NAME, ENDPLATE_NAME, PLANE_NAME):
        other = bpy.data.objects.get(name)
        overlaps[name] = len(support_tree.overlap(world_bvh(other))) if other is not None else None

    support["f1_2030_rear_support_role"] = "carries_intermediate_plane_from_above"
    support["f1_2030_rear_support_vertical_scale"] = round(scale, 9)
    support["f1_2030_rear_support_contact_embed_m"] = CONTACT_EMBED_M
    support["f1_2030_rear_support_previous_z_max_m"] = max(p.z for p in before)
    for stale in (
        "f1_2030_rear_support_compression",
        "f1_2030_rear_support_band_vertices",
        "f1_2030_rear_support_fit",
        "f1_2030_rear_support_contact_stations",
        "f1_2030_rear_support_arm_drop_m",
    ):
        if stale in support:
            del support[stale]

    report = {
        "restored_from": restored_from,
        "z_min_m": round(z_min, 6),
        "vertical_scale": round(scale, 6),
        "top_after_m": round(max(p.z for p in final), 6),
        "base_drift_m": base_drift,
        "near_base_shift_m": near_base_shift,
        "arm_candidates": len(candidates),
        "min_gap_to_top_m": min_final_gap,
        "overlaps": overlaps,
    }
    if base_drift > 1e-9:
        raise RuntimeError(f"floor base moved: {report}")
    if near_base_shift > 0.001:
        raise RuntimeError(f"floor base shifted more than 1 mm: {report}")
    if not (-0.012 <= min_final_gap <= -0.002):
        raise RuntimeError(f"support arm grip is not on the intermediate plane top: {report}")
    if overlaps.get(UPPER_NAME):
        raise RuntimeError(f"support overlaps the upper wing: {report}")
    if overlaps.get(BEAM_NAME):
        raise RuntimeError(f"support overlaps the beam wing: {report}")
    if overlaps.get(ENDPLATE_NAME):
        raise RuntimeError(f"support overlaps the endplate: {report}")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    print("F90_REAR_SUPPORT_SHORTEN=" + json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
