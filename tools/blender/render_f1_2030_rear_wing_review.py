"""Render a rear orthographic review of the currently open F1 2030 blend."""

from __future__ import annotations

import sys
from pathlib import Path

import bpy
from mathutils import Vector


def output_path() -> Path:
    if "--" not in sys.argv:
        raise SystemExit("Expected output PNG after --")
    return Path(sys.argv[sys.argv.index("--") + 1]).resolve()


def requested_view() -> str:
    if "--" not in sys.argv:
        return "rear"
    args = sys.argv[sys.argv.index("--") + 1 :]
    return args[1] if len(args) > 1 else "rear"


def point_at(obj: bpy.types.Object, target: tuple[float, float, float]) -> None:
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()


scene = bpy.context.scene
world = scene.world or bpy.data.worlds.new("RearWingReviewWorld")
scene.world = world
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.035, 0.045, 0.06, 1.0)
world.node_tree.nodes["Background"].inputs[1].default_value = 0.35

for name, position, energy, size in (
    ("RearWingReview_Key", (4.5, -2.4, 3.0), 1350.0, 3.0),
    ("RearWingReview_Fill", (4.0, 2.7, 1.7), 950.0, 2.5),
    ("RearWingReview_Rim", (1.0, 0.0, 3.5), 1100.0, 2.0),
):
    light_data = bpy.data.lights.new(name, "AREA")
    light_data.energy = energy
    light_data.shape = "DISK"
    light_data.size = size
    light = bpy.data.objects.new(name, light_data)
    scene.collection.objects.link(light)
    light.location = position
    point_at(light, (2.25, 0.0, 0.1))

camera_data = bpy.data.cameras.new("RearWingReviewCamera")
camera_data.type = "ORTHO"
camera = bpy.data.objects.new("RearWingReviewCamera", camera_data)
scene.collection.objects.link(camera)
if requested_view() == "three-quarter":
    camera_data.ortho_scale = 1.55
    camera.location = (4.65, -2.15, 1.05)
    point_at(camera, (2.17, 0.0, 0.04))
else:
    camera_data.ortho_scale = 1.45
    camera.location = (5.4, 0.0, 0.25)
    point_at(camera, (2.15, 0.0, 0.08))
scene.camera = camera

scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 1200
scene.render.resolution_y = 900
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = "PNG"
scene.view_settings.look = "AgX - Medium High Contrast"
destination = output_path()
destination.parent.mkdir(parents=True, exist_ok=True)
scene.render.filepath = str(destination)
bpy.ops.render.render(write_still=True)
print(f"F90_REAR_WING_RENDER={destination}")
