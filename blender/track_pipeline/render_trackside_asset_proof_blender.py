from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from procedural_assets_blender import _mesh_object
from procedural_materials_blender import flat_material, texture_material


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def aim(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()


def setup(output: Path, camera_location, target):
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1280
    scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.filepath = str(output)
    if scene.world is None:
        scene.world = bpy.data.worlds.new("ProofWorld")
    scene.world.color = (0.06, 0.08, 0.11)
    bpy.ops.object.light_add(type="AREA", location=(2, -4, 12))
    bpy.context.object.data.energy = 1800
    bpy.context.object.data.shape = "DISK"
    bpy.context.object.data.size = 8
    bpy.ops.object.light_add(type="SUN", location=(0, 0, 10))
    bpy.context.object.rotation_euler = (math.radians(25), math.radians(-20), math.radians(-25))
    bpy.context.object.data.energy = 2.0
    bpy.ops.object.camera_add(location=camera_location)
    camera = bpy.context.object
    camera.data.lens = 52
    aim(camera, target)
    scene.camera = camera
    bpy.ops.mesh.primitive_plane_add(size=80, location=(0, 0, -0.02))
    bpy.context.object.data.materials.append(flat_material("ProofGround", (0.15, 0.13, 0.10), roughness=1.0))
    return camera


def render_barrier(repo: Path, output: Path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    prepared = json.loads((repo / "blender/generated/la_chutana/tire_barrier_cards/prepared_manifest.json").read_text(encoding="utf-8"))
    front_entry = prepared["sources"][prepared["module"]["front_source"]]
    side_entry = prepared["sources"][prepared["module"]["side_source"]]
    front = texture_material("ProofBarrierFront", Path(front_entry["texture"]), 1.0, 0.0, True)
    side = texture_material("ProofBarrierSide", Path(side_entry["texture"]), 1.0, 0.0, True)
    top = texture_material("ProofBarrierTop", Path(prepared["sources"][prepared["module"]["top_source"]]["texture"]), 1.0, 0.0, False)
    x0, y0, x1, y1 = front_entry["metrics"]["output_bbox"]
    tw, th = front_entry["metrics"]["output_size"]
    front_uv = [(x0 / tw, 1 - y1 / th), (x1 / tw, 1 - y1 / th), (x1 / tw, 1 - y0 / th), (x0 / tw, 1 - y0 / th)]
    width, depth, height = 0.68, 0.68, 1.45
    verts, faces, mats, uvs = [], [], [], []
    for module in range(9):
        cx = (module - 4) * width
        corners = [(cx-width/2,-depth/2,0),(cx+width/2,-depth/2,0),(cx+width/2,depth/2,0),(cx-width/2,depth/2,0)]
        for indices, mat, uv in (((0,1,1,0),0,front_uv),((3,2,2,3),0,front_uv),((0,3,3,0),1,[(0,0),(1,0),(1,5),(0,5)]),((2,1,1,2),1,[(0,0),(1,0),(1,5),(0,5)])):
            start = len(verts)
            a,b,c,d = indices
            verts += [corners[a],corners[b],(corners[c][0],corners[c][1],height),(corners[d][0],corners[d][1],height)]
            faces.append((start,start+1,start+2,start+3)); mats.append(mat); uvs.append(uv)
        start = len(verts); verts += [(x,y,height) for x,y,_ in corners]
        faces.append((start,start+1,start+2,start+3)); mats.append(2); uvs.append([(0,0),(1,0),(1,1),(0,1)])
        start = len(verts); verts += list(reversed(corners))
        faces.append((start,start+1,start+2,start+3)); mats.append(2); uvs.append([(0,0),(1,0),(1,1),(0,1)])
    _mesh_object("ProofBarrier", verts, faces, [front, side, top], mats, uvs)
    camera = setup(output, (4.7, -6.5, 2.8), (0, 0, 0.72))
    bpy.ops.render.render(write_still=True)
    front_output = output.with_name("tire_barrier_rectangular_front_proof.png")
    bpy.context.scene.render.filepath = str(front_output)
    camera.location = (0, -8.5, 1.3)
    aim(camera, (0, 0, 0.72))
    bpy.ops.render.render(write_still=True)
    end_output = output.with_name("tire_barrier_rectangular_end_proof.png")
    bpy.context.scene.render.filepath = str(end_output)
    camera.location = (-5.0, -4.7, 2.35)
    aim(camera, (-2.72, 0, 0.72))
    bpy.ops.render.render(write_still=True)


def render_trees(repo: Path, output: Path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    positions = (-12.0, 0.0, 12.0)
    for index, x in enumerate(positions, 1):
        before = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str(repo / f"blender/generated/la_chutana/raw_vegetation/assets_v2/glb/tree_v2_0{index}.glb"))
        imported = [obj for obj in bpy.data.objects if obj not in before]
        for obj in imported:
            obj.location.x += x
    setup(output, (29, -42, 16), (0, 0, 7.5))
    bpy.ops.render.render(write_still=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True)
    parser.add_argument("--output-dir", required=True)
    ns = parser.parse_args(args_after_double_dash())
    repo = Path(ns.repo).resolve()
    output_dir = Path(ns.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    render_barrier(repo, output_dir / "tire_barrier_cards_proof.png")
    render_trees(repo, output_dir / "trees_two_cards_proof.png")
    print(json.dumps({"barrier": str(output_dir / "tire_barrier_cards_proof.png"), "trees": str(output_dir / "trees_two_cards_proof.png")}, indent=2))


if __name__ == "__main__":
    main()
