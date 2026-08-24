"""Sanitize, unwrap and style an assembled Formula 1 GLB for Godot staging."""
from __future__ import annotations

import argparse
from hashlib import sha256
import json
from math import radians
from pathlib import Path
import sys

import bmesh
import bpy
from mathutils import Vector

SENTINEL = "__FORMULA90_GODOT_LIVERY_JSON__"


def file_hash(path: Path) -> str:
    digest = sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest().upper()


def material(name: str, color: tuple[float, float, float, float], *,
             metallic: float = 0.0, roughness: float = 0.4,
             emission: tuple[float, float, float, float] | None = None):
    value = bpy.data.materials.new(name)
    value.use_nodes = True
    node = value.node_tree.nodes.get("Principled BSDF")
    node.inputs["Base Color"].default_value = color
    metallic_socket = node.inputs.get("Metallic IOR Level") or node.inputs.get("Metallic")
    if metallic_socket is not None:
        metallic_socket.default_value = metallic
    node.inputs["Roughness"].default_value = roughness
    if emission is not None:
        emission_socket = node.inputs.get("Emission Color") or node.inputs.get("Emission")
        if emission_socket is not None:
            emission_socket.default_value = emission
        strength_socket = node.inputs.get("Emission Strength")
        if strength_socket is not None:
            strength_socket.default_value = 2.5
    return value


def library() -> dict[str, bpy.types.Material]:
    return {
        "blue": material("F90_LIVERY_BLUE", (0.012, 0.055, 0.32, 1), metallic=0.05, roughness=0.26),
        "blue_light": material("F90_LIVERY_BLUE_LIGHT", (0.02, 0.22, 0.65, 1), roughness=0.24),
        "white": material("F90_LIVERY_WHITE", (0.82, 0.86, 0.92, 1), roughness=0.3),
        "yellow": material("F90_LIVERY_YELLOW", (0.95, 0.64, 0.015, 1), roughness=0.3),
        "red": material("F90_LIVERY_RED", (0.72, 0.015, 0.025, 1), roughness=0.28),
        "carbon": material("F90_CARBON", (0.012, 0.016, 0.022, 1), metallic=0.15, roughness=0.34),
        "rubber": material("F90_RUBBER", (0.008, 0.009, 0.011, 1), roughness=0.72),
        "metal": material("F90_METAL", (0.28, 0.31, 0.35, 1), metallic=0.9, roughness=0.22),
        "helmet": material("F90_HELMET", (0.92, 0.92, 0.94, 1), metallic=0.05, roughness=0.2),
        "lcd": material("F90_LCD_GREEN", (0.005, 0.03, 0.015, 1), roughness=0.25,
                        emission=(0.01, 0.85, 0.2, 1)),
    }


def sanitize_and_unwrap(obj: bpy.types.Object) -> dict:
    mesh = obj.data
    original = {"vertices": len(mesh.vertices), "edges": len(mesh.edges), "polygons": len(mesh.polygons)}
    bm = bmesh.new()
    bm.from_mesh(mesh)
    before_non_manifold = sum(1 for edge in bm.edges if not edge.is_manifold)
    before_degenerate = sum(1 for face in bm.faces if face.calc_area() <= 1e-12)
    merge_result = bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=1e-6)
    if bm.faces:
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    after_non_manifold = sum(1 for edge in bm.edges if not edge.is_manifold)
    after_degenerate = sum(1 for face in bm.faces if face.calc_area() <= 1e-12)
    bm.to_mesh(mesh)
    bm.free()
    mesh.validate(verbose=False, clean_customdata=True)
    mesh.update()
    sanitized = {"vertices": len(mesh.vertices), "edges": len(mesh.edges), "polygons": len(mesh.polygons)}
    for layer in list(mesh.uv_layers):
        mesh.uv_layers.remove(layer)
    mesh.uv_layers.new(name="UVMap")
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=radians(66.0), island_margin=0.02,
                             area_weight=0.0, correct_aspect=True, scale_to_bounds=True)
    bpy.ops.object.mode_set(mode="OBJECT")
    obj.select_set(False)
    uv = mesh.uv_layers.active
    finite_uv = all(abs(loop.uv.x) < 1e8 and abs(loop.uv.y) < 1e8 for loop in uv.data)
    return {
        "original": original, "sanitized": sanitized,
        "merged_vertices": original["vertices"] - sanitized["vertices"],
        "non_manifold_edges_before": before_non_manifold,
        "non_manifold_edges_after": after_non_manifold,
        "degenerate_faces_before": before_degenerate,
        "degenerate_faces_after": after_degenerate,
        "uv_loops": len(uv.data), "finite_uv": finite_uv,
    }


def assign_slots(obj: bpy.types.Object, names: list[str], lib: dict):
    obj.data.materials.clear()
    for name in names:
        obj.data.materials.append(lib[name])


def polygon_world_center(obj, polygon):
    return obj.matrix_world @ polygon.center


def style(obj: bpy.types.Object, lib: dict) -> None:
    name = obj.name.upper()
    if "WHEEL_TIRE" in name or "WHEEL_TREAD" in name:
        assign_slots(obj, ["rubber"], lib)
    elif "WHEEL_HUB" in name:
        assign_slots(obj, ["metal"], lib)
    elif "SUSPENSION" in name or "BODY_INTERIOR" in name:
        assign_slots(obj, ["carbon"], lib)
    elif "COCKPIT_LCD" in name:
        assign_slots(obj, ["lcd"], lib)
    elif "DRIVER_HELMET" in name:
        assign_slots(obj, ["helmet", "blue", "yellow"], lib)
        for polygon in obj.data.polygons:
            center = polygon_world_center(obj, polygon)
            polygon.material_index = 1 if center.y < 0.15 else (2 if center.z < 0 else 0)
    elif name == "GEO_CHASSIS":
        assign_slots(obj, ["blue", "white", "red", "blue_light"], lib)
        for polygon in obj.data.polygons:
            center = polygon_world_center(obj, polygon)
            if center.y > 0.18 and abs(center.x) < 0.48:
                polygon.material_index = 1
            elif 0.02 < center.y < 0.22 and abs(center.x) > 0.38:
                polygon.material_index = 2
            elif center.y > 0.05 and abs(center.x) > 0.55:
                polygon.material_index = 3
            else:
                polygon.material_index = 0
    elif name == "GEO_NOSE":
        assign_slots(obj, ["white", "blue", "yellow", "red"], lib)
        centers = [polygon_world_center(obj, polygon) for polygon in obj.data.polygons]
        front_z = min((point.z for point in centers), default=-2.0)
        for polygon, center in zip(obj.data.polygons, centers):
            if center.z < front_z + 0.32:
                polygon.material_index = 2
            elif center.y < 0.02:
                polygon.material_index = 1
            elif abs(center.x) > 0.22:
                polygon.material_index = 3
            else:
                polygon.material_index = 0
    elif "AERO_FRONT_WING" in name:
        assign_slots(obj, ["blue", "white", "yellow"], lib)
        for polygon in obj.data.polygons:
            center = polygon_world_center(obj, polygon)
            polygon.material_index = 2 if abs(center.x) > 0.72 else (1 if center.y > 0 else 0)
    elif "AERO_REAR_WING" in name:
        assign_slots(obj, ["blue", "white", "yellow"], lib)
        for polygon in obj.data.polygons:
            center = polygon_world_center(obj, polygon)
            polygon.material_index = 2 if abs(center.x) > 0.62 else (1 if center.y > 0.65 else 0)
    else:
        assign_slots(obj, ["carbon"], lib)


def add_render_scene(output: Path) -> None:
    ground_y = -0.323
    bpy.ops.mesh.primitive_plane_add(size=30, location=(0, ground_y - 0.002, 0))
    plane = bpy.context.object
    plane.name = "RENDER_GROUND_NOT_EXPORTED"
    plane.data.materials.append(material("RENDER_GROUND", (0.025, 0.03, 0.04, 1), roughness=0.65))
    camera_data = bpy.data.cameras.new("RenderCamera")
    camera = bpy.data.objects.new("RenderCamera", camera_data)
    bpy.context.scene.collection.objects.link(camera)
    camera.location = (6.6, 2.7, -6.6)
    target = Vector((0.0, 0.12, 0.0))
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()
    camera_data.lens = 54
    bpy.context.scene.camera = camera
    for index, (location, energy, size) in enumerate((
        ((3.5, 6.0, -4.0), 1500, 5.0),
        ((-4.0, 3.0, -1.0), 900, 4.0),
        ((1.0, 4.0, 6.0), 1200, 3.0),
    )):
        data = bpy.data.lights.new(f"StudioLight{index}", "AREA")
        data.energy, data.shape, data.size = energy, "DISK", size
        light = bpy.data.objects.new(f"StudioLight{index}", data)
        light.location = location
        light.rotation_euler = (target - light.location).to_track_quat("-Z", "Y").to_euler()
        bpy.context.scene.collection.objects.link(light)
    world = bpy.data.worlds.new("StudioWorld")
    world.color = (0.008, 0.012, 0.02)
    bpy.context.scene.world = world
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1600
    scene.render.resolution_y = 1000
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.filepath = str(output)
    scene.view_settings.look = "AgX - Medium High Contrast"
    bpy.ops.render.render(write_still=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    args.output_dir.mkdir(parents=True, exist_ok=True)
    before_hash = file_hash(args.source)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(args.source))
    lib = library()
    mesh_objects = sorted((obj for obj in bpy.data.objects if obj.type == "MESH"), key=lambda obj: obj.name)
    audit = {}
    for obj in mesh_objects:
        audit[obj.name] = sanitize_and_unwrap(obj)
        style(obj, lib)
    if any(not item["finite_uv"] for item in audit.values()):
        raise ValueError("non-finite UV coordinates detected")
    output_glb = args.output_dir / "formula90-williams94-inspired-godot.glb"
    output_blend = args.output_dir / "formula90-williams94-inspired.blend"
    output_render = args.output_dir / "formula90-williams94-inspired.png"
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_blend), check_existing=False)
    bpy.ops.export_scene.gltf(filepath=str(output_glb), export_format="GLB", use_selection=False,
                              export_yup=True, export_apply=False)
    if file_hash(args.source) != before_hash:
        raise ValueError("source GLB changed")
    faces_before_export = {name: item["sanitized"]["polygons"] for name, item in audit.items()}
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(output_glb))
    imported_faces = {obj.name: len(obj.data.polygons)
                      for obj in bpy.data.objects if obj.type == "MESH"}
    missing_uv = sorted(obj.name for obj in bpy.data.objects if obj.type == "MESH" and not obj.data.uv_layers)
    if missing_uv:
        raise ValueError(f"exported GLB lost UV layers: {missing_uv[0]}")
    for name, face_count in faces_before_export.items():
        if imported_faces.get(name) != face_count:
            raise ValueError(f"face-count mismatch after GLB reimport: {name}")
    add_render_scene(output_render)
    report = {
        "schema_version": "formula90-godot-vehicle-package/v1",
        "source": {"path": str(args.source), "sha256": before_hash},
        "outputs": {
            "glb": {"path": str(output_glb), "sha256": file_hash(output_glb)},
            "blend": {"path": str(output_blend), "sha256": file_hash(output_blend)},
            "render": {"path": str(output_render), "sha256": file_hash(output_render)},
        },
        "mesh_count": len(audit),
        "material_count": len(lib),
        "topology_preserved": False,
        "topology_repaired": True,
        "surface_faces_preserved_after_export": True,
        "uv_rebuilt": True,
        "source_unchanged": True,
        "audit": audit,
        "livery": "1994-inspired blue/white/yellow/red, no sponsor marks",
    }
    report_path = args.output_dir / "godot-package-report.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    print(SENTINEL + json.dumps(report, sort_keys=True, separators=(",", ":")), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
