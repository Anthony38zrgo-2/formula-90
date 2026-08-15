#!/usr/bin/env python3
"""Render a read-only studio preview of the currently opened Blend file."""

from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def look_at(obj: bpy.types.Object, target: Vector) -> None:
    obj.rotation_euler = (target - obj.location).to_track_quat("-Z", "Y").to_euler()


def bounds() -> tuple[Vector, Vector]:
    vertices = []
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            vertices.extend(obj.matrix_world @ vertex.co for vertex in obj.data.vertices)
    if not vertices:
        raise RuntimeError("Blend contains no mesh vertices")
    return (
        Vector((min(v.x for v in vertices), min(v.y for v in vertices), min(v.z for v in vertices))),
        Vector((max(v.x for v in vertices), max(v.y for v in vertices), max(v.z for v in vertices))),
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--view", choices=("iso", "front", "side", "top"), default="iso")
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else [])

    if args.input is not None:
        bpy.ops.object.select_all(action="SELECT")
        bpy.ops.object.delete(use_global=False)
        bpy.ops.import_scene.gltf(filepath=str(args.input))

    low, high = bounds()
    center = (low + high) * 0.5
    extent = max(high - low)
    distance = max(extent * 2.2, 1.0)

    bpy.ops.object.camera_add()
    camera = bpy.context.object
    if args.view == "front":
        camera.location = center + Vector((0.0, -distance, 0.0))
    elif args.view == "side":
        camera.location = center + Vector((distance, 0.0, 0.0))
    elif args.view == "top":
        camera.location = center + Vector((0.0, 0.0, distance))
    else:
        camera.location = center + Vector((distance, -distance, distance * 0.65))
    look_at(camera, center)
    bpy.context.scene.camera = camera

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "MATERIAL"
    scene.display.shading.show_shadows = False
    scene.display.shading.show_cavity = True
    scene.render.resolution_x = 640
    scene.render.resolution_y = 480
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = False
    scene.world.color = (0.04, 0.04, 0.04)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    scene.render.filepath = str(args.output)
    bpy.ops.render.render(write_still=True)


if __name__ == "__main__":
    main()
