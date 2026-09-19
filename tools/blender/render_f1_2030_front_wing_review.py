"""Render focused front-wing review views from the currently open blend."""

import sys
from pathlib import Path

import bpy
from mathutils import Vector


if "--" not in sys.argv:
    raise SystemExit("Expected output PNG and optional view after --")
args = sys.argv[sys.argv.index("--") + 1 :]
destination = Path(args[0]).resolve()
view = args[1] if len(args) > 1 else "three-quarter"


def point_at(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()


scene = bpy.context.scene
world = scene.world or bpy.data.worlds.new("FrontWingReviewWorld")
scene.world = world
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.035, 0.045, 0.06, 1.0)
world.node_tree.nodes["Background"].inputs[1].default_value = 0.35

for name, position, energy, size in (
    ("FrontWingReview_Key", (-4.7, -2.7, 2.6), 1350.0, 3.0),
    ("FrontWingReview_Fill", (-4.3, 2.8, 1.6), 950.0, 2.5),
    ("FrontWingReview_Rim", (-1.0, 0.0, 3.4), 1100.0, 2.0),
):
    light_data = bpy.data.lights.new(name, "AREA")
    light_data.energy = energy
    light_data.shape = "DISK"
    light_data.size = size
    light = bpy.data.objects.new(name, light_data)
    scene.collection.objects.link(light)
    light.location = position
    point_at(light, (-2.4, 0.0, -0.12))

camera_data = bpy.data.cameras.new("FrontWingReviewCamera")
camera_data.type = "ORTHO"
camera = bpy.data.objects.new("FrontWingReviewCamera", camera_data)
scene.collection.objects.link(camera)
if view == "side":
    camera_data.ortho_scale = 1.35
    camera.location = (-2.45, -4.5, 0.45)
    point_at(camera, (-2.45, 0.0, -0.12))
elif view == "front":
    camera_data.ortho_scale = 2.2
    camera.location = (-5.2, 0.0, 0.25)
    point_at(camera, (-2.42, 0.0, -0.12))
else:
    camera_data.ortho_scale = 1.8
    camera.location = (-4.4, -2.3, 1.15)
    point_at(camera, (-2.42, 0.0, -0.14))
scene.camera = camera

scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 1200
scene.render.resolution_y = 900
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = "PNG"
scene.view_settings.look = "AgX - Medium High Contrast"
destination.parent.mkdir(parents=True, exist_ok=True)
scene.render.filepath = str(destination)
bpy.ops.render.render(write_still=True)
print(f"F90_FRONT_WING_RENDER={destination}")
