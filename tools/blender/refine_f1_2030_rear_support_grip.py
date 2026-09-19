"""Conform the shortened rear-support arm onto the intermediate plane top.

The shortened swan neck only touches the mid plane near its rear tip. This
script bends the arm cross-sections vertically (smooth falloff into the neck)
until the arm underside follows the plane top along the whole connection run,
keeping the authored grip embed, the floor base and every other object intact.

Usage:
  blender --background --python refine_f1_2030_rear_support_grip.py -- TARGET.blend OUTPUT.blend [EMBED_M]
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
STATION_STEP_M = 0.004
SMOOTH_RADIUS = 1
NECK_BLEND_START_M = 2.10
RUN_START_M = 2.145
MIN_VERTEX_Z_M = 0.10
MAX_ABS_Y_M = 0.03
PASSES = 3
UNION_MAX_GAP_M = 0.012
UNION_RAMP_M = 0.012


def arguments():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) not in (2, 3):
        raise SystemExit(
            "usage: blender --background --python refine_f1_2030_rear_support_grip.py -- TARGET.blend OUTPUT.blend [EMBED_M]"
        )
    embed = float(values[2]) if len(values) == 3 else 0.008
    return Path(values[0]).resolve(), Path(values[1]).resolve(), embed


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


def main():
    target_path, output_path, embed = arguments()
    bpy.ops.wm.open_mainfile(filepath=str(target_path))
    support = bpy.data.objects.get(SUPPORT_NAME)
    plane = bpy.data.objects.get(PLANE_NAME)
    if support is None or support.type != "MESH":
        raise RuntimeError(f"required rear support missing: {SUPPORT_NAME}")
    if plane is None or plane.type != "MESH":
        raise RuntimeError(f"required intermediate plane missing: {PLANE_NAME}")
    if (len(support.data.vertices), len(support.data.polygons)) != EXPECTED_TOPOLOGY:
        raise RuntimeError("rear support topology changed from the expected base")

    protected = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in bpy.data.objects.items()
        if name != SUPPORT_NAME
    }

    plane_tree = world_bvh(plane)
    plane_points = world_points(plane)
    plane_x = (min(p.x for p in plane_points), max(p.x for p in plane_points))
    plane_top_max = max(p.z for p in plane_points)

    def top_at(x, y):
        hit = plane_tree.ray_cast(
            Vector((x, y, plane_top_max + 0.5)), Vector((0.0, 0.0, -1.0)), 2.5
        )
        if hit[0] is None:
            return None
        return hit[0].z

    inverse = support.matrix_world.inverted()
    z_min_start = min(p.z for p in world_points(support))
    base_indices = [
        index
        for index, p in enumerate(world_points(support))
        if p.z <= z_min_start + 1e-6
    ]

    run_start = max(RUN_START_M, plane_x[0])
    run_end = min(plane_x[1], max(p.x for p in world_points(support)))
    stations = []
    x = run_start
    while x <= run_end:
        stations.append(x)
        x += STATION_STEP_M
    if len(stations) < 3:
        raise RuntimeError("connection run is too short for a station fit")

    before_passes = world_points(support)
    union_indices = []
    for _pass in range(PASSES):
        points = world_points(support)
        deltas = [0.0] * len(stations)
        active = [False] * len(stations)
        for index, station in enumerate(stations):
            column = [
                p
                for p in points
                if abs(p.x - station) <= STATION_STEP_M
                and abs(p.y) <= MAX_ABS_Y_M
                and p.z > MIN_VERTEX_Z_M
            ]
            top = top_at(station, 0.0)
            if not column or top is None:
                continue
            gap = min(p.z for p in column) - top
            if 0.0 < gap <= UNION_MAX_GAP_M:
                deltas[index] = -gap
                active[index] = True
        smoothed = list(deltas)
        for index in range(len(deltas)):
            if not active[index]:
                continue
            window = [
                deltas[other]
                for other in range(max(0, index - SMOOTH_RADIUS), min(len(deltas), index + SMOOTH_RADIUS + 1))
                if active[other]
            ]
            smoothed[index] = sum(window) / len(window)

        active_indices = [index for index, flag in enumerate(active) if flag]
        if _pass == 0:
            union_indices = list(active_indices)
        if not active_indices:
            break
        first_active = stations[active_indices[0]]

        def delta_at(x):
            if x <= first_active - UNION_RAMP_M:
                return 0.0
            if x < first_active:
                t = (x - (first_active - UNION_RAMP_M)) / UNION_RAMP_M
                return smoothed[active_indices[0]] * t
            if x >= stations[-1]:
                return smoothed[active_indices[-1]]
            for index in range(len(stations) - 1):
                if stations[index] <= x <= stations[index + 1]:
                    t = (x - stations[index]) / (stations[index + 1] - stations[index])
                    return smoothed[index] * (1.0 - t) + smoothed[index + 1] * t
            return 0.0

        for vertex, point in zip(support.data.vertices, points):
            if point.z <= MIN_VERTEX_Z_M:
                continue
            drop = delta_at(point.x)
            if drop == 0.0:
                continue
            vertex.co = inverse @ Vector((point.x, point.y, point.z + drop))
        support.data.update()

    for name, matrix_values in protected.items():
        current = tuple(round(value, 9) for row in bpy.data.objects[name].matrix_world for value in row)
        if current != matrix_values:
            raise RuntimeError(f"protected object transform changed: {name}")

    final = world_points(support)
    base_drift = max(
        (abs(final[index].z - z_min_start) for index in base_indices), default=0.0
    )
    if not union_indices:
        raise RuntimeError("no floating union stations were found; nothing to seat")
    ramp_start = stations[union_indices[0]] - UNION_RAMP_M
    arch_drift = max(
        (
            (final[index] - before_passes[index]).length
            for index, point in enumerate(before_passes)
            if point.x < ramp_start
        ),
        default=0.0,
    )
    grips = []
    seats = []
    for index, station in enumerate(stations):
        column = [
            p
            for p in final
            if abs(p.x - station) <= STATION_STEP_M
            and abs(p.y) <= MAX_ABS_Y_M
            and p.z > MIN_VERTEX_Z_M
        ]
        top = top_at(station, 0.0)
        if not column or top is None:
            continue
        gap = min(p.z for p in column) - top
        grips.append(gap)
        if index in union_indices:
            seats.append(gap)
    min_grip = min(grips) if grips else float("nan")
    max_grip = max(grips) if grips else float("nan")
    max_seat = max((abs(gap) for gap in seats), default=float("nan"))

    overlaps = {}
    support_tree = world_bvh(support)
    for name in (UPPER_NAME, BEAM_NAME, ENDPLATE_NAME, PLANE_NAME):
        other = bpy.data.objects.get(name)
        overlaps[name] = len(support_tree.overlap(world_bvh(other))) if other is not None else None

    support["f1_2030_rear_support_role"] = "carries_intermediate_plane_from_above"
    support["f1_2030_rear_support_union_stations"] = len(union_indices)
    support["f1_2030_rear_support_union_gap_m"] = round(max_seat, 6)
    support["f1_2030_rear_support_contact_stations"] = len(stations)
    support["f1_2030_rear_support_grip_range_m"] = [round(min(grips), 6), round(max(grips), 6)]

    report = {
        "stations": len(stations),
        "run_m": [round(run_start, 4), round(run_end, 4)],
        "union_stations": len(union_indices),
        "union_first_m": round(stations[union_indices[0]], 4),
        "max_seat_gap_m": round(max_seat, 6),
        "grip_min_m": round(min_grip, 6),
        "grip_max_m": round(max_grip, 6),
        "arch_drift_m": arch_drift,
        "base_drift_m": base_drift,
        "overlaps": overlaps,
    }
    if base_drift > 1e-6:
        raise RuntimeError(f"floor base moved: {report}")
    if arch_drift > 1e-9:
        raise RuntimeError(f"arch geometry moved outside the union: {report}")
    if max_seat > 0.0025:
        raise RuntimeError(f"union still floats after seating: {report}")
    if min_grip < -0.015:
        raise RuntimeError(f"union over-penetrates the plane: {report}")
    if overlaps.get(UPPER_NAME):
        raise RuntimeError(f"support overlaps the upper wing: {report}")
    if overlaps.get(BEAM_NAME) or overlaps.get(ENDPLATE_NAME):
        raise RuntimeError(f"support overlaps another aero part: {report}")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    print("F90_REAR_SUPPORT_GRIP=" + json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
