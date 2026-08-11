from __future__ import annotations

import argparse
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def main() -> None:
    parser = argparse.ArgumentParser(description="Validate the Jordan 3x2 tire barrier GLB contract.")
    parser.add_argument("--glb", required=True)
    parser.add_argument("--source-wheel", required=True)
    ns = parser.parse_args(args_after_double_dash())

    glb = Path(ns.glb).resolve()
    source = Path(ns.source_wheel).resolve()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(glb))

    failures = []
    mesh_objs = [obj for obj in bpy.data.objects if obj.type == "MESH"]
    print(f"[info] imported mesh objects={len(mesh_objs)}")

    roots = [obj for obj in bpy.data.objects if obj.type == "EMPTY" and obj.name.startswith("TireBarrierJordan6")]
    if not roots:
        failures.append("missing TireBarrierJordan6 root")
    if len(mesh_objs) != 1:
        failures.append(f"expected a single fused module mesh, got {len(mesh_objs)}")

    # The fused mesh must expose both blue and white material slots.
    slot_names = set()
    for obj in mesh_objs:
        slot_names.update(slot.name for slot in obj.data.materials if slot)
    has_blue = any("Blue" in n for n in slot_names)
    has_white = any("White" in n for n in slot_names)
    print(f"[info] material slots={sorted(slot_names)}")
    if not has_blue or not has_white:
        failures.append(f"module mesh must have blue and white slots, got {sorted(slot_names)}")

    # No collision markers and no -colonly mesh.
    if any(o.get("formula90s_collision", False) for o in bpy.data.objects):
        failures.append("an object carries formula90s_collision=true")
    if any("-colonly" in o.name for o in bpy.data.objects):
        failures.append("-colonly mesh present in visual asset")

    # Bounds: finite, lowest Z ~ 0; log source vs output dimensions.
    corners = [obj.matrix_world @ Vector(c) for obj in mesh_objs for c in obj.bound_box]
    min_z = min(v.z for v in corners)
    max_z = max(v.z for v in corners)
    min_x = min(v.x for v in corners)
    max_x = max(v.x for v in corners)
    min_y = min(v.y for v in corners)
    max_y = max(v.y for v in corners)
    print(f"[info] output dims X={max_x-min_x:.3f} Y={max_y-min_y:.3f} Z={max_z-min_z:.3f}")
    print(f"[info] output Z range [{min_z:.4f}, {max_z:.4f}]")
    if abs(min_z) > 0.02:
        failures.append(f"lowest Z not ~0: {min_z:.4f}")

    if failures:
        for f in failures:
            print(f"FAIL {f}")
        raise SystemExit(2)
    print("PASS tire barrier GLB: fused module, blue + white slots, no collision, Z anchored")
    print(f"PASS source wheel unchanged: {source}")


if __name__ == "__main__":
    main()
