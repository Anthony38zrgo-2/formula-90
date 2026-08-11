from __future__ import annotations

import argparse
import math
import subprocess
import sys
from pathlib import Path

import bpy
from mathutils import Vector

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from blender_output import atomic_export_glb, atomic_save_blend


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def repo_root() -> Path:
    marker = SCRIPT_DIR.parents[1]
    return marker


TIRE_DIAMETER = 0.695   # measured from the Jordan rear-wheel GLB torus (Y/Z)
TIRE_WIDTH = 0.377      # measured along the wheel axis (X)
ROWS = 2
PER_ROW = 3
TIRE_COUNT = ROWS * PER_ROW


def build_material(name: str, color) -> bpy.types.Material:
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = 0.98
    bsdf.inputs["Metallic"].default_value = 0.0
    return mat


def import_wheel(source: Path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(source))
    imported = [obj for obj in bpy.data.objects if obj not in before]
    meshes = [obj for obj in imported if obj.type == "MESH"]
    if not meshes:
        raise RuntimeError("No mesh objects in source wheel GLB")
    empties = [obj for obj in imported if obj.type == "EMPTY"]
    return meshes, empties


def make_tire_unit(source_meshes: list, mat_blue, mat_white, blue: bool) -> list:
    """Duplicate the wheel geometry into one tire instance standing in a barrier.
    The wheel circle (Y-Z plane) becomes the vertical wall plane; the wheel axis
    (X) becomes the wall depth. Blue/white applied to all faces of the unit.
    Returns the mesh copies (already rotated onto the wall plane); each copy has
    its own mesh data so material assignment never mutates the shared source."""
    mat = mat_blue if blue else mat_white
    copies = []
    for src in source_meshes:
        copy = src.copy()
        copy.data = src.data.copy()
        bpy.context.scene.collection.objects.link(copy)
        copy.matrix_basis = src.matrix_basis.copy()
        # Rotate the wheel so the Y-Z circle lands on the vertical X-Z wall plane.
        copy.rotation_euler[2] = math.pi * 0.5
        for index in range(len(copy.data.materials)):
            copy.data.materials[index] = mat
        copies.append(copy)
    return copies


def assemble_module(wheel_source: Path, blue_mat, white_mat) -> bpy.types.Object:
    meshes, _empties = import_wheel(wheel_source)
    module = bpy.data.objects.new("TireBarrierJordan6", None)
    bpy.context.scene.collection.objects.link(module)
    half = (PER_ROW - 1) * 0.5
    for row in range(ROWS):
        for col in range(PER_ROW):
            copies = make_tire_unit(meshes, blue_mat, white_mat, blue=(row == 0))
            for copy in copies:
                copy.parent = module
                copy.location = Vector((
                    (col - half) * TIRE_DIAMETER,
                    0.0,
                    row * TIRE_DIAMETER + TIRE_DIAMETER * 0.5,
                ))
    # Fuse all wheel meshes into a single module mesh with two material slots
    # (blue lower, white upper). One object per module keeps instancing cheap
    # (thousands of modules along the perimeter).
    bpy.ops.object.select_all(action="DESELECT")
    for copy in [o for o in bpy.data.objects if o.type == "MESH" and o.parent == module]:
        copy.select_set(True)
    bpy.context.view_layer.objects.active = [o for o in bpy.data.objects if o.type == "MESH" and o.parent == module][0]
    bpy.ops.object.join()
    fused = bpy.context.active_object
    fused.parent = module
    fused.location = Vector((0.0, 0.0, 0.0))
    fused["formula90s_collision"] = False
    return module


def main() -> None:
    parser = argparse.ArgumentParser(description="Build the reusable Jordan 3x2 tire barrier module.")
    parser.add_argument("--source-wheel", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--render-proof", default="")
    ns = parser.parse_args(args_after_double_dash())

    repo = Path(ns.repo).resolve()
    wheel = (repo / ns.source_wheel).resolve()
    if not wheel.exists():
        raise RuntimeError(f"Source wheel GLB missing: {wheel}")

    generated = repo / "blender/generated/la_chutana/tire_barrier"
    generated.mkdir(parents=True, exist_ok=True)
    runtime = repo / "game/assets/trackside"
    runtime.mkdir(parents=True, exist_ok=True)

    bpy.ops.wm.read_factory_settings(use_empty=True)
    blue = build_material("F90_TireBarrierBlue", (0.10, 0.16, 0.55))
    white = build_material("F90_TireBarrierWhite", (0.93, 0.94, 0.96))
    module = assemble_module(wheel, blue, white)
    module.name = "TireBarrierJordan6"
    module["formula90s_collision"] = False
    module["formula90s_barrier_kind"] = "jordan_rear_tire_stack_6"
    module["formula90s_tire_count"] = 6
    module["formula90s_rows"] = 2

    # Apply transforms down the hierarchy so the exported GLB is baked.
    bpy.ops.object.select_all(action="DESELECT")
    module.select_set(True)
    bpy.context.view_layer.objects.active = module
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)

    # Center horizontally (Y depth) and put lowest contact at Z=0.
    def world_corners():
        return [
            mesh.matrix_world @ Vector(c)
            for mesh in bpy.data.objects
            if mesh.type == "MESH" and mesh.parent == module
            for c in mesh.bound_box
        ]

    corners = world_corners()
    min_z = min(v.z for v in corners)
    max_z = max(v.z for v in corners)
    center_x = (min(v.x for v in corners) + max(v.x for v in corners)) * 0.5
    center_y = (min(v.y for v in corners) + max(v.y for v in corners)) * 0.5
    module.location = Vector((-center_x, -center_y, -min_z))
    bpy.context.view_layer.objects.active = module
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    corners = world_corners()
    print(f"[blender] module Z range after anchor [{min(v.z for v in corners):.4f}, {max(v.z for v in corners):.4f}]")

    # Flatten: detach the fused mesh keeping its (already anchored) world
    # transform, then reparent it under a clean root at the origin so the
    # exported GLB bakes world-space positions directly (glTF does not apply an
    # empty parent transform to its children on export).
    meshes_under_module = [o for o in bpy.data.objects if o.type == "MESH" and o.parent == module]
    bpy.ops.object.select_all(action="DESELECT")
    for mesh in meshes_under_module:
        mesh.select_set(True)
    bpy.ops.object.parent_clear(type="CLEAR_KEEP_TRANSFORM")
    bpy.ops.object.select_all(action="DESELECT")
    bpy.data.objects.remove(module, do_unlink=True)
    # Remove the source-import leftovers (the original wheel objects and the
    # imported scene root) so the export contains only the module.
    for leftover in [o for o in bpy.data.objects if o not in meshes_under_module]:
        bpy.data.objects.remove(leftover, do_unlink=True)
    root = bpy.data.objects.new("TireBarrierJordan6", None)
    bpy.context.scene.collection.objects.link(root)
    for mesh in meshes_under_module:
        mesh.parent = root
    corners = [
        mesh.matrix_world @ Vector(c)
        for mesh in bpy.data.objects
        if mesh.type == "MESH" and mesh.parent == root
        for c in mesh.bound_box
    ]
    print(f"[blender] final root Z range [{min(v.z for v in corners):.4f}, {max(v.z for v in corners):.4f}]")
    root["formula90s_collision"] = False
    root["formula90s_barrier_kind"] = "jordan_rear_tire_stack_6"
    root["formula90s_tire_count"] = 6
    root["formula90s_rows"] = 2

    blend = generated / "tire_barrier_jordan_6.blend"
    atomic_save_blend(blend, generated / "backups")
    glb = runtime / "tire_barrier_jordan_6.glb"
    atomic_export_glb(glb)

    if ns.render_proof:
        proof = (repo / ns.render_proof).resolve()
        proof.parent.mkdir(parents=True, exist_ok=True)
        _render_proof(root, proof)

    print(f"[blender] tire barrier module built: {glb}")
    print(f"[blender] source wheel unchanged: {wheel}")

    blend = generated / "tire_barrier_jordan_6.blend"
    atomic_save_blend(blend, generated / "backups")
    glb = runtime / "tire_barrier_jordan_6.glb"
    atomic_export_glb(glb)

    if ns.render_proof:
        proof = (repo / ns.render_proof).resolve()
        proof.parent.mkdir(parents=True, exist_ok=True)
        _render_proof(module, proof)

    print(f"[blender] tire barrier module built: {glb}")
    print(f"[blender] source wheel unchanged: {wheel}")


def _render_proof(module: bpy.types.Object, out: Path) -> None:
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1280
    scene.render.resolution_y = 720
    scene.render.film_transparent = False
    sun = bpy.data.objects.new("ProofSun", bpy.data.lights.new("ProofSun", type="SUN"))
    scene.collection.objects.link(sun)
    sun.rotation_euler = Vector((math.radians(55.0), 0.0, math.radians(-30.0)))
    sun.data.energy = 3.0
    cam = bpy.data.objects.new("ProofCam", bpy.data.cameras.new("ProofCam"))
    scene.collection.objects.link(cam)
    cam.location = Vector((0.0, 2.4, 1.0))
    cam.data.lens = 50
    track = bpy.data.objects.new("ProofTarget", None)
    scene.collection.objects.link(track)
    track.location = Vector((0.0, 0.0, 0.8))
    constraint = cam.constraints.new(type="TRACK_TO")
    constraint.target = track
    constraint.track_axis = "TRACK_NEGATIVE_Z"
    constraint.up_axis = "UP_Y"
    scene.camera = cam
    scene.render.filepath = str(out)
    bpy.ops.render.render(write_still=True)


if __name__ == "__main__":
    main()
