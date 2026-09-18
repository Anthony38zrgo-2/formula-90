"""Fit the existing F1 2030 active rear flap to the lower rear-wing contour.

This correction preserves the active flap's topology, chord section, span,
materials, UVs and endpoint placement. Each lateral row is translated only in
world Z so its centerline keeps the original closed-flap edge clearance from
GEO_CHASSIS_AEROPART2 across the full span.
"""

import bpy
import sys
from collections import defaultdict
from pathlib import Path


ACTIVE_NAME = "GEO_CHASSIS_ACTIVEREAR"
LOWER_NAME = "GEO_CHASSIS_AEROPART2"
GROUP_PRECISION = 6
MODIFIED_OBJECTS = {ACTIVE_NAME}


def arguments():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) != 2:
        raise SystemExit("usage: blender --background --python SCRIPT -- INPUT.blend OUTPUT.blend")
    return Path(values[0]).resolve(), Path(values[1]).resolve()


def lateral_rows(obj):
    rows = defaultdict(list)
    for vertex in obj.data.vertices:
        point = obj.matrix_world @ vertex.co
        rows[round(point.y, GROUP_PRECISION)].append((vertex, point))
    return rows


def row_center(row):
    return sum(point.z for _vertex, point in row) / len(row)


def fit_active_flap(active, lower):
    active_rows = lateral_rows(active)
    lower_rows = lateral_rows(lower)
    if set(active_rows) != set(lower_rows):
        raise RuntimeError("active and lower rear-wing planes do not share lateral rows")

    half_span = max(abs(y) for y in active_rows)
    edge_rows = [y for y in active_rows if abs(abs(y) - half_span) < 1.0e-6]
    edge_clearance = sum(
        row_center(active_rows[y]) - row_center(lower_rows[y]) for y in edge_rows
    ) / len(edge_rows)

    inverse = active.matrix_world.inverted()
    shifts = []
    final_gaps = []
    for y, row in active_rows.items():
        active_center = row_center(row)
        lower_center = row_center(lower_rows[y])
        shift = lower_center + edge_clearance - active_center
        shifts.append(shift)
        for vertex, point in row:
            point.z += shift
            vertex.co = inverse @ point
        final_gaps.append(active_center + shift - lower_center)

    active.data.update()
    active["f1_2030_active_rear_fit"] = "closed_parallel_to_lower_plane"
    active["f1_2030_active_rear_clearance_m"] = edge_clearance
    return {
        "lateral_rows": len(active_rows),
        "edge_clearance_m": edge_clearance,
        "minimum_vertical_shift_m": min(shifts),
        "maximum_vertical_shift_m": max(shifts),
        "minimum_final_center_gap_m": min(final_gaps),
        "maximum_final_center_gap_m": max(final_gaps),
    }


def main():
    input_path, output_path = arguments()
    bpy.ops.wm.open_mainfile(filepath=str(input_path))
    protected = {
        name: (
            tuple(round(value, 9) for row in obj.matrix_world for value in row),
            len(obj.data.vertices) if obj.type == "MESH" else None,
            len(obj.data.polygons) if obj.type == "MESH" else None,
        )
        for name, obj in bpy.data.objects.items()
        if name not in MODIFIED_OBJECTS
    }

    active = bpy.data.objects.get(ACTIVE_NAME)
    lower = bpy.data.objects.get(LOWER_NAME)
    if active is None or active.type != "MESH" or lower is None or lower.type != "MESH":
        raise RuntimeError("required rear-wing planes are missing")
    report = fit_active_flap(active, lower)

    for name, expected in protected.items():
        obj = bpy.data.objects[name]
        current = (
            tuple(round(value, 9) for row in obj.matrix_world for value in row),
            len(obj.data.vertices) if obj.type == "MESH" else None,
            len(obj.data.polygons) if obj.type == "MESH" else None,
        )
        if current != expected:
            raise RuntimeError(f"protected object changed: {name}")

    bpy.context.scene["f1_2030_active_rear_scope"] = (
        "GEO_CHASSIS_ACTIVEREAR vertical row fit to GEO_CHASSIS_AEROPART2; "
        "closed edge clearance preserved"
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    print(report)


if __name__ == "__main__":
    main()
