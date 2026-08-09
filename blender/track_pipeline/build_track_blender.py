from __future__ import annotations

import argparse
from pathlib import Path
import json
import math
import sys

import bpy
from mathutils import Vector

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from blender_output import atomic_export_glb, atomic_publish, atomic_save_blend
from procedural_assets_blender import effective_far_ground_z
from procedural_materials_blender import build_material_library


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)


def godot_xz_to_blender(x: float, z: float, height: float = 0.0) -> Vector:
    return Vector((float(x), -float(z), float(height)))


def bank_degrees_at_fraction(config, fraction: float) -> float:
    result = 0.0
    f = fraction % 1.0
    for zone in config.get("banking", []):
        center = float(zone["center_fraction"]) % 1.0
        half = max(float(zone["half_width_fraction"]), 1e-6)
        d = abs((f - center + 0.5) % 1.0 - 0.5)
        if d <= half:
            weight = 0.5 * (1.0 + math.cos(math.pi * d / half))
            result += float(zone["degrees"]) * weight
    return result


def cross_frame(points, i, config):
    n = len(points)
    prev = godot_xz_to_blender(*points[(i - 1) % n])
    nxt = godot_xz_to_blender(*points[(i + 1) % n])
    tangent = (nxt - prev).normalized()
    normal = Vector((-tangent.y, tangent.x, 0.0))
    bank = math.radians(bank_degrees_at_fraction(config, i / n))
    return tangent, normal, bank


def mesh_object(name, verts, faces, materials=None, planar_uv_scale_m: float | None = None):
    mesh = bpy.data.meshes.new(name + "Mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)

    for material in materials or []:
        obj.data.materials.append(material)

    if planar_uv_scale_m:
        uv_layer = mesh.uv_layers.new(name="UVMap")
        scale = max(float(planar_uv_scale_m), 0.001)
        for polygon in mesh.polygons:
            for loop_index in polygon.loop_indices:
                vertex_index = mesh.loops[loop_index].vertex_index
                co = mesh.vertices[vertex_index].co
                uv_layer.data[loop_index].uv = (co.x / scale, co.y / scale)
    return obj


def build_ribbon(points, half_width, top_z, name, material, collision_name, config):
    n = len(points)
    left = []
    right = []
    for i, p in enumerate(points):
        _, normal, bank = cross_frame(points, i, config)
        center = godot_xz_to_blender(p[0], p[1], top_z)
        rise = math.tan(bank) * half_width
        left.append(center - normal * half_width + Vector((0, 0, -rise)))
        right.append(center + normal * half_width + Vector((0, 0, rise)))

    verts = []
    for a, b in zip(left, right):
        verts.extend([tuple(a), tuple(b)])

    faces = []
    for i in range(n):
        j = (i + 1) % n
        faces.append((2 * i, 2 * j, 2 * j + 1, 2 * i + 1))

    road = mesh_object(name, verts, faces, [material], planar_uv_scale_m=5.0)
    collision = mesh_object(collision_name, verts, faces)
    collision.hide_render = True
    collision.display_type = "WIRE"
    return road


def build_edge_line(points, side, road_half, line_width, config, material, top_z):
    n = len(points)
    side_sign = 1.0 if side == "right" else -1.0
    verts = []
    faces = []
    inner = road_half - line_width
    outer = road_half
    for i, p in enumerate(points):
        _, normal, bank = cross_frame(points, i, config)
        center = godot_xz_to_blender(p[0], p[1], top_z + 0.003)
        for offset in (inner, outer):
            signed = side_sign * offset
            rise = math.tan(bank) * signed
            verts.append(tuple(center + normal * signed + Vector((0, 0, rise))))
    for i in range(n):
        j = (i + 1) % n
        faces.append((2 * i, 2 * j, 2 * j + 1, 2 * i + 1))
    return mesh_object(("Right" if side_sign > 0 else "Left") + "EdgeLine", verts, faces, [material])


def curb_indices(n, start_fraction, end_fraction):
    start = int(math.floor(start_fraction * n)) % n
    end = int(math.ceil(end_fraction * n)) % n
    if start <= end:
        return list(range(start, end + 1))
    return list(range(start, n)) + list(range(0, end + 1))


def build_curb(points, segment, road_half, profile, materials, config, top_z):
    indices = curb_indices(len(points), float(segment["start_fraction"]), float(segment["end_fraction"]))
    side = 1.0 if segment["side"] == "right" else -1.0
    verts = []
    faces = []
    rows = len(profile)

    for idx in indices:
        _, normal, bank = cross_frame(points, idx, config)
        normal *= side
        p = points[idx]
        center = godot_xz_to_blender(p[0], p[1], top_z)
        road_edge_rise = math.tan(bank) * road_half * side
        edge = center + normal * road_half + Vector((0, 0, road_edge_rise))
        for offset, height in profile:
            cross_rise = math.tan(bank) * float(offset) * side
            verts.append(tuple(edge + normal * float(offset) + Vector((0, 0, float(height) + cross_rise))))

    for r in range(len(indices) - 1):
        for c in range(rows - 1):
            a = r * rows + c
            b = (r + 1) * rows + c
            faces.append((a, b, b + 1, a + 1))

    obj = mesh_object(
        "Curb_" + segment["name"],
        verts,
        faces,
        [materials["curb_red"], materials["curb_white"]],
        planar_uv_scale_m=1.0,
    )

    stripe_length = max(0.5, float(config["curb"].get("stripe_length_m", 2.0)))
    sample_spacing = float(config.get("sample_spacing_m", 2.0))
    faces_per_row = max(1, rows - 1)
    for polygon_index, polygon in enumerate(obj.data.polygons):
        longitudinal_row = polygon_index // faces_per_row
        distance = longitudinal_row * sample_spacing
        polygon.material_index = int(distance // stripe_length) % 2

    col = mesh_object("CurbCollision_" + segment["name"] + "-colonly", verts, faces)
    col.hide_render = True
    col.display_type = "WIRE"
    return obj


def build_grass_shoulders(points, config, material):
    n = len(points)
    road_half = float(config["road"]["width_m"]) * 0.5
    surface_z = float(config["road"]["surface_elevation_m"])
    terrain = config["terrain"]
    clearance = float(terrain["road_clearance_m"])
    shoulder_width = float(terrain["shoulder_width_m"])
    far_z = effective_far_ground_z(config)

    for side_name, side in (("Left", -1.0), ("Right", 1.0)):
        verts = []
        faces = []
        for i, p in enumerate(points):
            _, normal, bank = cross_frame(points, i, config)
            center = godot_xz_to_blender(p[0], p[1], surface_z)
            edge_rise = math.tan(bank) * road_half * side
            inner = center + normal * (road_half * side) + Vector((0, 0, edge_rise - clearance))
            outer = center + normal * ((road_half + shoulder_width) * side)
            outer.z = far_z
            verts.extend([tuple(inner), tuple(outer)])

        for i in range(n):
            j = (i + 1) % n
            faces.append((2 * i, 2 * j, 2 * j + 1, 2 * i + 1))

        mesh_object(f"GrassShoulder{side_name}", verts, faces, [material], planar_uv_scale_m=4.0)
        col = mesh_object(f"GrassShoulderCollision{side_name}-colonly", verts, faces)
        col.hide_render = True
        col.display_type = "WIRE"


def build_far_ground(points, config, material):
    terrain = config["terrain"]
    margin = float(terrain.get("far_ground_margin_m", 240.0))
    far_z = effective_far_ground_z(config)

    xs = [p[0] for p in points]
    zs = [p[1] for p in points]
    sx = (max(xs) - min(xs)) + margin * 2.0
    sy = (max(zs) - min(zs)) + margin * 2.0
    cx = (max(xs) + min(xs)) * 0.5
    godot_cz = (max(zs) + min(zs)) * 0.5
    cy = -godot_cz

    bpy.ops.mesh.primitive_plane_add(size=2.0, location=(cx, cy, far_z - 0.003))
    ground = bpy.context.object
    ground.name = "GrassFarVisual"
    ground.scale = (sx * 0.5, sy * 0.5, 1.0)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    ground.data.materials.append(material)

    uv_layer = ground.data.uv_layers.active or ground.data.uv_layers.new(name="UVMap")
    for polygon in ground.data.polygons:
        for loop_index in polygon.loop_indices:
            vertex_index = ground.data.loops[loop_index].vertex_index
            co = ground.data.vertices[vertex_index].co
            uv_layer.data[loop_index].uv = (co.x / 5.0, co.y / 5.0)

    bpy.ops.mesh.primitive_plane_add(size=2.0, location=(cx, cy, far_z - 0.005))
    ground_col = bpy.context.object
    ground_col.name = "GrassFarCollision-colonly"
    ground_col.scale = (sx * 0.5, sy * 0.5, 1.0)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    ground_col.hide_render = True
    ground_col.display_type = "WIRE"


def build_start_finish(config, material):
    width = float(config["road"]["width_m"])
    line_width = float(config["start_finish"]["line_width_m"])
    surface_z = float(config["road"]["surface_elevation_m"])
    bpy.ops.mesh.primitive_cube_add(location=(0, 0, surface_z + 0.008))
    line = bpy.context.object
    line.name = "StartFinish"
    line.scale = (width * 0.5, line_width * 0.5, 0.006)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    line.data.materials.append(material)


def build_spawn_marker(config):
    spawn = float(config["start_finish"]["spawn_before_m"])
    surface_z = float(config["road"]["surface_elevation_m"])
    marker = bpy.data.objects.new("PlayerSpawn", None)
    marker.location = godot_xz_to_blender(0.0, spawn, surface_z)
    bpy.context.scene.collection.objects.link(marker)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    ns = parser.parse_args(args_after_double_dash())

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    center = read_json(repo / config["generated_dir"] / "centerline.json")
    points = center["points_xz"]

    clear_scene()
    generated = repo / config["generated_dir"]
    runtime = repo / config["runtime_dir"]
    texture_dir = generated / "textures"
    materials = build_material_library(texture_dir)

    surface_z = float(config["road"]["surface_elevation_m"])
    road_half = float(config["road"]["width_m"]) * 0.5

    build_far_ground(points, config, materials["ground"])
    build_grass_shoulders(points, config, materials["ground"])
    build_ribbon(points, road_half, surface_z, "RoadVisual", materials["asphalt"], "RoadCollision-colonly", config)

    line_width = float(config["road"]["edge_line_width_m"])
    build_edge_line(points, "left", road_half, line_width, config, materials["edge_line"], surface_z)
    build_edge_line(points, "right", road_half, line_width, config, materials["edge_line"], surface_z)

    for segment in config["curb"]["segments"]:
        build_curb(points, segment, road_half, config["curb"]["profile"], materials, config, surface_z)

    build_start_finish(config, materials["start_finish"])
    build_spawn_marker(config)

    generated.mkdir(parents=True, exist_ok=True)
    runtime.mkdir(parents=True, exist_ok=True)

    blend = generated / "track_base.blend"
    glb = runtime / f"{config['track_id']}_base.glb"
    current_glb = runtime / f"{config['track_id']}.glb"
    backup_dir = generated / "backups" / "base"

    atomic_save_blend(blend, backup_dir)
    atomic_export_glb(glb)
    atomic_publish(glb, current_glb)

    print(f"[blender] far ground z={effective_far_ground_z(config):.3f}m")
    print(f"[blender] base blend: {blend}")
    print(f"[blender] base glb: {glb}")
    print(f"[blender] published runtime: {current_glb}")


if __name__ == "__main__":
    main()
