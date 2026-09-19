"""Capture eight non-rendered Blender viewport views of the F1 2030 car.

The images are produced with bpy.ops.render.opengl from a solid/material 3D
viewport. No scene camera, render engine, or render lights are used.
"""

from __future__ import annotations

import sys

import bpy
from mathutils import Vector
from pathlib import Path


SOURCE = Path(r"D:\Formula90s\game\assets\models\vehicles\f1-2030\source\f1_2030.blend")
OUTPUT_DIR = Path(r"C:\Users\bill\Downloads")


def find_view3d():
    for window in bpy.context.window_manager.windows:
        for area in window.screen.areas:
            if area.type == "VIEW_3D":
                region = next((item for item in area.regions if item.type == "WINDOW"), None)
                if region is not None:
                    return window, area, region, area.spaces.active
    raise RuntimeError("Blender has no visible VIEW_3D area")


def select_car_meshes():
    bpy.ops.object.select_all(action="DESELECT")
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    for obj in meshes:
        obj.select_set(True)
    if meshes:
        bpy.context.view_layer.objects.active = next(
            (obj for obj in meshes if obj.name == "GEO_CHASSIS_BODY"), meshes[0]
        )


def configure_viewport(space):
    space.shading.type = "MATERIAL"
    space.shading.light = "STUDIO"
    space.shading.studio_light = "studio.exr"
    space.shading.show_shadows = True
    space.shading.show_cavity = True
    space.shading.cavity_type = "BOTH"
    space.shading.curvature_ridge_factor = 1.25
    space.shading.curvature_valley_factor = 1.0
    space.overlay.show_floor = False
    space.overlay.show_axis_x = False
    space.overlay.show_axis_y = False
    space.overlay.show_axis_z = False
    space.overlay.show_relationship_lines = False
    space.overlay.show_outline_selected = False
    space.overlay.show_text = False
    space.overlay.show_stats = False
    space.overlay.show_cursor = False
    space.region_3d.view_distance = 7.0


def set_axis(window, area, region, space, axis):
    with bpy.context.temp_override(window=window, area=area, region=region, space_data=space):
        bpy.ops.view3d.view_axis(type=axis, align_active=False)
        bpy.ops.view3d.view_selected(use_all_regions=False)
    space.region_3d.view_distance = 2.65


def orbit(window, area, region, space, direction, angle):
    with bpy.context.temp_override(window=window, area=area, region=region, space_data=space):
        bpy.ops.view3d.view_orbit(type=direction, angle=angle)
        bpy.ops.view3d.view_selected(use_all_regions=False)
    space.region_3d.view_distance = 2.65


def capture(window, area, region, space, filename):
    destination = OUTPUT_DIR / filename
    destination.parent.mkdir(parents=True, exist_ok=True)
    scene = bpy.context.scene
    scene.render.filepath = str(destination)
    with bpy.context.temp_override(window=window, area=area, region=region, space_data=space):
        bpy.ops.wm.redraw_timer(type="DRAW_WIN_SWAP", iterations=2)
        result = bpy.ops.render.opengl(write_still=True, view_context=True)
    if result != {"FINISHED"} or not destination.exists():
        raise RuntimeError(f"Viewport capture failed: {destination} ({result})")
    print(f"F90_VIEWPORT_CAPTURE={destination}")


bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
window, area, region, space = find_view3d()
configure_viewport(space)
select_car_meshes()

views = (
    ("01_front_left.png", "LEFT", None),
    ("02_rear_right.png", "RIGHT", None),
    ("03_left_side.png", "FRONT", None),
    ("04_right_side.png", "BACK", None),
    ("05_top.png", "TOP", None),
    ("06_bottom.png", "BOTTOM", None),
    ("07_front_left_three_quarter.png", "LEFT", (("ORBITUP", 0.30),)),
    ("08_rear_right_three_quarter.png", "RIGHT", (("ORBITDOWN", 0.30),)),
    ("09_front_three_quarter.png", "LEFT", (("ORBITLEFT", 0.58), ("ORBITUP", 0.24))),
    ("10_front_three_quarter_down.png", "LEFT", (("ORBITLEFT", 0.58), ("ORBITDOWN", 0.30))),
)

if "--only-front-three-quarter" in sys.argv:
    views = (views[-2],)
elif "--only-front-three-quarter-down" in sys.argv:
    views = (views[-1],)

for filename, axis, orbit_spec in views:
    set_axis(window, area, region, space, axis)
    if orbit_spec is not None:
        for direction, angle in orbit_spec:
            orbit(window, area, region, space, direction, angle)
    if filename == "10_front_three_quarter_down.png":
        space.region_3d.view_distance = 3.35
    capture(window, area, region, space, filename)

print(f"F90_VIEWPORT_CAPTURE_COUNT={len(views)}")
bpy.ops.wm.quit_blender()
