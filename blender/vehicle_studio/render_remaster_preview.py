"""Assemble and render remastered Formula-90 canonical modules."""
from __future__ import annotations

import argparse
from math import pi
from pathlib import Path
import sys

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from materialize_build_ir import add_wheel_instance, export_glb
from prepare_godot_livery import add_render_scene


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--chassis", required=True)
    parser.add_argument("--front", required=True)
    parser.add_argument("--rear", required=True)
    parser.add_argument("--output-glb", required=True)
    parser.add_argument("--output-png", required=True)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=args.chassis)
    placements = (
        ("WHEEL_FL", args.front, (-0.820138, 0.025669, -1.533342), False),
        ("WHEEL_FR", args.front, (0.820138, 0.025669, -1.533342), True),
        ("WHEEL_RL", args.rear, (-0.747060, -0.010424, 1.533342), False),
        ("WHEEL_RR", args.rear, (0.747060, -0.010424, 1.533342), True),
    )
    for name, source, location, rotate_right in placements:
        add_wheel_instance(source, name, location, rotate_right)
    export_glb(args.output_glb)
    add_render_scene(Path(args.output_png))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
