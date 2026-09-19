"""Raise GEO_CHASSIS_REAR_WING_MID rigidly to a requested world top height.

Usage:
  blender --background --python raise_f1_2030_mid_plane.py -- TARGET.blend OUTPUT.blend TARGET_TOP_M
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import bpy


PLANE_NAME = "GEO_CHASSIS_REAR_WING_MID"


def arguments():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) != 3:
        raise SystemExit(
            "usage: blender --background --python raise_f1_2030_mid_plane.py -- TARGET.blend OUTPUT.blend TARGET_TOP_M"
        )
    return Path(values[0]).resolve(), Path(values[1]).resolve(), float(values[2])


def main():
    target_path, output_path, target_top = arguments()
    bpy.ops.wm.open_mainfile(filepath=str(target_path))
    plane = bpy.data.objects.get(PLANE_NAME)
    if plane is None or plane.type != "MESH":
        raise RuntimeError(f"required intermediate plane missing: {PLANE_NAME}")

    protected = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in bpy.data.objects.items()
        if name != PLANE_NAME
    }

    current_top = max((plane.matrix_world @ vertex.co).z for vertex in plane.data.vertices)
    delta = target_top - current_top
    plane.matrix_world.translation.z += delta
    plane.data.update()

    for name, matrix_values in protected.items():
        current = tuple(round(value, 9) for row in bpy.data.objects[name].matrix_world for value in row)
        if current != matrix_values:
            raise RuntimeError(f"protected object transform changed: {name}")

    new_top = max((plane.matrix_world @ vertex.co).z for vertex in plane.data.vertices)
    report = {
        "previous_top_m": round(current_top, 6),
        "target_top_m": round(target_top, 6),
        "delta_m": round(delta, 6),
        "new_top_m": round(new_top, 6),
    }
    if abs(new_top - target_top) > 1e-6:
        raise RuntimeError(f"mid plane top did not land on target: {report}")

    plane["f1_2030_mid_plane_top_m"] = round(new_top, 6)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    print("F90_MID_PLANE_RAISE=" + json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
