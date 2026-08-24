"""Render a deterministic side-by-side Vehicle Studio GLB comparison."""
from __future__ import annotations

import argparse
from math import pi
from pathlib import Path
import sys

import bpy
from mathutils import Vector


def imported_roots(path: Path):
    existing = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    imported = [obj for obj in bpy.data.objects if obj not in existing]
    return [obj for obj in imported if obj.parent is None]


def add_vehicle(path: Path, z_offset: float, label: str):
    parent = bpy.data.objects.new(label, None)
    bpy.context.scene.collection.objects.link(parent)
    parent.location.z = z_offset
    for root in imported_roots(path):
        root.parent = parent
    return parent


def add_label(text: str, z: float):
    curve = bpy.data.curves.new(text, "FONT")
    curve.body = text
    curve.align_x = "CENTER"
    curve.size = 0.34
    obj = bpy.data.objects.new(text, curve)
    bpy.context.scene.collection.objects.link(obj)
    obj.location = (0.0, 1.35, z)
    obj.rotation_euler = (pi / 2.0, 0.0, pi / 2.0)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    bpy.ops.wm.read_factory_settings(use_empty=True)
    add_vehicle(args.before, -3.5, "ANTES")
    add_vehicle(args.after, 3.5, "DESPUES")
    add_label("ANTES  |  WHEELBASE 2.921 m", -3.5)
    add_label("DESPUES  |  WHEELBASE 3.359 m (+15%)", 3.5)

    camera_data = bpy.data.cameras.new("ComparisonCamera")
    camera = bpy.data.objects.new("ComparisonCamera", camera_data)
    bpy.context.scene.collection.objects.link(camera)
    camera.location = (10.0, 2.2, 0.0)
    direction = Vector((0.0, 0.25, 0.0)) - camera.location
    camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    camera_data.type = "ORTHO"
    camera_data.ortho_scale = 12.0
    bpy.context.scene.camera = camera

    world = bpy.data.worlds.new("ComparisonWorld")
    world.color = (0.008, 0.012, 0.018)
    bpy.context.scene.world = world
    for location, energy, size in (((5, 6, -5), 1400, 5), ((4, 3, 6), 1000, 4)):
        light_data = bpy.data.lights.new("Area", "AREA")
        light_data.energy = energy
        light_data.shape = "DISK"
        light_data.size = size
        light = bpy.data.objects.new("Area", light_data)
        light.location = location
        bpy.context.scene.collection.objects.link(light)
        light.rotation_euler = (0.0, pi / 2.0, 0.0)

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1920
    scene.render.resolution_y = 900
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.filepath = str(args.output)
    scene.render.film_transparent = False
    scene.view_settings.look = "AgX - Medium High Contrast"
    bpy.ops.render.render(write_still=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
