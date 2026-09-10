"""Render the five runtime GLBs from orthographic diagnostic views."""

import bpy
import json
import math
from pathlib import Path
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "game/assets/models/vehicles/f1-2030"
REPORT = ROOT / "reports/2026-09-09-f1-2030-wheel-import"
PHYSICS = ROOT / "game/data/vehicles/f1_2030/f1_2030_v10_physics.json"


def import_at(filename, location):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(ASSETS / filename))
    imported = set(bpy.data.objects) - before
    for obj in imported:
        if obj.parent is None:
            obj.location += Vector(location)


bpy.ops.wm.read_factory_settings(use_empty=True)
physics = json.loads(PHYSICS.read_text(encoding="utf-8"))
halfbase = float(physics["geometry"]["wheelbase"]) * 0.5
front_half = float(physics["geometry"]["front_track"]) * 0.5
rear_half = float(physics["geometry"]["rear_track"]) * 0.5

import_at("f1_2030_v10_chassis.glb", (0, 0, 0))
for short, position in {
    "FL": (-front_half, halfbase, 0),
    "FR": (front_half, halfbase, 0),
    "RL": (-rear_half, -halfbase, 0),
    "RR": (rear_half, -halfbase, 0),
}.items():
    import_at(f"f1_2030_v10_wheel_{short}.glb", position)

bpy.ops.mesh.primitive_plane_add(size=30, location=(0, 0, -0.33))
ground = bpy.context.object
ground_material = bpy.data.materials.new("DiagnosticGround")
ground_material.diffuse_color = (0.12, 0.14, 0.17, 1.0)
ground.data.materials.append(ground_material)

world = bpy.data.worlds.new("DiagnosticWorld")
bpy.context.scene.world = world
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.22, 0.25, 0.30, 1)
world.node_tree.nodes["Background"].inputs[1].default_value = 0.55
for name, position, energy, size in (
    ("Key", (4, 3, 7), 1800, 5),
    ("Fill", (-4, 1, 5), 1200, 5),
    ("Rear", (0, -5, 4), 1400, 4),
):
    light_data = bpy.data.lights.new(name, "AREA")
    light_data.energy = energy
    light_data.shape = "DISK"
    light_data.size = size
    light = bpy.data.objects.new(name, light_data)
    bpy.context.scene.collection.objects.link(light)
    light.location = position
    light.rotation_euler = (-light.location).to_track_quat("-Z", "Y").to_euler()

camera_data = bpy.data.cameras.new("DiagnosticCamera")
camera_data.type = "ORTHO"
camera = bpy.data.objects.new("DiagnosticCamera", camera_data)
bpy.context.scene.collection.objects.link(camera)
bpy.context.scene.camera = camera

scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 1400
scene.render.resolution_y = 1000
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = "PNG"
scene.view_settings.look = "AgX - Medium High Contrast"
REPORT.mkdir(parents=True, exist_ok=True)

for name, position, target, scale in (
    ("top", (0, 0, 8), (0, 0, 0), 5.2),
    ("rear", (0, -8, 1.2), (0, 0, 0.05), 3.4),
    ("front", (0, 8, 1.2), (0, 0, 0.05), 3.4),
    ("left", (-8, 0, 1.2), (0, 0, 0.05), 5.2),
):
    camera.location = position
    camera.rotation_euler = (Vector(target) - camera.location).to_track_quat("-Z", "Y").to_euler()
    camera.data.ortho_scale = scale
    scene.render.filepath = str(REPORT / f"{name}.png")
    bpy.ops.render.render(write_still=True)
