from __future__ import annotations

import argparse
import copy
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from blender_output import atomic_export_glb, atomic_save_blend
from procedural_assets_blender import (
    create_tire_barrier_collision,
    create_tire_barrier_visual,
    godot_xz_to_blender,
    sample_centerline,
    terrain_height,
    _mesh_object,
    create_trackside_card,
)
from procedural_materials_blender import flat_material, texture_material


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def collection(name: str, parent=None):
    existing = bpy.data.collections.get(name)
    if existing:
        for obj in list(existing.objects):
            bpy.data.objects.remove(obj, do_unlink=True)
        return existing
    out = bpy.data.collections.new(name)
    (parent or bpy.context.scene.collection).children.link(out)
    return out


def move_to_collection(obj, target):
    for link in list(obj.users_collection):
        link.objects.unlink(obj)
    target.objects.link(obj)


def import_asset_instances(path: Path, placements, target_collection, asset_key: str):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    imported = [obj for obj in bpy.data.objects if obj not in before]
    meshes = [obj for obj in imported if obj.type == "MESH"]
    if not meshes:
        for obj in imported:
            bpy.data.objects.remove(obj, do_unlink=True)
        raise RuntimeError(f"No mesh objects in raw asset: {path}")
    corners = [obj.matrix_world @ Vector(corner) for obj in meshes for corner in obj.bound_box]
    min_z = min(v.z for v in corners)
    center_x = (min(v.x for v in corners) + max(v.x for v in corners)) * 0.5
    center_y = (min(v.y for v in corners) + max(v.y for v in corners)) * 0.5
    normalized = [(obj, Matrix.Translation((-center_x, -center_y, -min_z)) @ obj.matrix_world.copy()) for obj in meshes]
    for number, item in enumerate(placements):
        instance_id = item.get("instance_id", f"{number:04d}")
        root = bpy.data.objects.new(f"Raw_{item['category']}_{instance_id}_{asset_key}", None)
        target_collection.objects.link(root)
        x, z = item["position_xz"]
        root.location = godot_xz_to_blender(float(x), float(z), float(item.get("ground_m", 0.0)))
        root.rotation_euler[2] = -float(item.get("yaw_rad", 0.0))
        scale = float(item.get("scale", 1.0))
        root.scale = (scale, scale, scale)
        root["formula90s_raw_asset"] = str(path.as_posix())
        root["formula90s_category"] = item["category"]
        root["formula90s_instance_id"] = instance_id
        root["formula90s_collision"] = False
        for source, matrix in normalized:
            copy = source.copy()
            copy.data = source.data
            target_collection.objects.link(copy)
            copy.parent = root
            copy.matrix_basis = matrix
    for obj in imported:
        bpy.data.objects.remove(obj, do_unlink=True)


def create_card(name, image_path: Path, center, tangent, ground, collection_target, side=1, target_width=None, target_height=None):
    image = bpy.data.images.load(str(image_path.resolve()), check_existing=True)
    aspect = float(image.size[1]) / max(1.0, float(image.size[0]))
    if target_height is not None:
        height = float(target_height)
        width = height / max(aspect, 1e-6)
    elif target_width is not None:
        width = float(target_width)
        height = width * aspect
    else:
        raise RuntimeError(f"Card {name} requires target_width or target_height")
    tangent_b = Vector((float(tangent[0]), -float(tangent[1]), 0.0)).normalized()
    origin = godot_xz_to_blender(center[0], center[1], ground)
    half = width * 0.5
    verts = [tuple(origin - tangent_b * half), tuple(origin + tangent_b * half), tuple(origin + tangent_b * half + Vector((0, 0, height))), tuple(origin - tangent_b * half + Vector((0, 0, height)))]
    mesh = bpy.data.meshes.new(name + "Mesh")
    face = (0, 1, 2, 3) if int(side) > 0 else (3, 2, 1, 0)
    mesh.from_pydata(verts, [], [face])
    mesh.uv_layers.new(name="UVMap")
    for loop, uv in zip(mesh.polygons[0].loop_indices, ((0, 0), (1, 0), (1, 1), (0, 1))):
        mesh.uv_layers[0].data[loop].uv = uv
    mat = texture_material("F90_RawCard_" + image_path.stem, image_path, roughness=1.0, alpha=True)
    mat.use_backface_culling = True
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    collection_target.objects.link(obj)
    obj["formula90s_raw_card"] = True
    obj["formula90s_collision"] = False
    return obj


def create_barrier_base(name, points, side, config, track_config, white, navy, collection_target):
    module = float(config["barrier"]["module_length_m"])
    road_half = float(track_config["road"]["width_m"]) * 0.5
    distance = road_half + float(config["barrier"]["distance_from_edge_m"])
    lap = sum(math.hypot(float(points[(i + 1) % len(points)][0]) - float(points[i][0]), float(points[(i + 1) % len(points)][1]) - float(points[i][1])) for i in range(len(points)))
    count = max(1, int(math.ceil(lap / module)))
    vertices = []
    faces = []
    materials = []
    for index in range(count):
        fraction = (index + 0.5) / count
        pos, tangent, normal = sample_centerline(points, fraction)
        ground = terrain_height(track_config, fraction, side, distance)
        center = godot_xz_to_blender(pos[0] + normal[0] * side * distance, pos[1] + normal[1] * side * distance, ground + 0.12)
        along = Vector((tangent[0], -tangent[1], 0.0)).normalized() * (module * 0.5)
        across = Vector((side * normal[0], -side * normal[1], 0.0)).normalized() * 0.28
        up = Vector((0.0, 0.0, 0.24))
        start = len(vertices)
        vertices.extend(tuple(center + a * along + b * across + c * up) for a, b, c in ((-1, -1, -1), (1, -1, -1), (1, 1, -1), (-1, 1, -1), (-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)))
        faces.extend([(start + 0, start + 1, start + 2, start + 3), (start + 4, start + 7, start + 6, start + 5), (start + 0, start + 4, start + 5, start + 1), (start + 1, start + 5, start + 6, start + 2), (start + 2, start + 6, start + 7, start + 3), (start + 4, start + 0, start + 3, start + 7)])
        materials.extend([index % 2] * 6)
    obj = _mesh_object(name, vertices, faces, [white, navy], materials)
    move_to_collection(obj, collection_target)
    obj["formula90s_raw_barrier_base"] = True
    return obj


def build_indexed_objects(config, track_config, center, compiled, collection_target, repo: Path):
    created = 0
    points = center["points_xz"]
    flag_material = flat_material("F90_RawTrackFlag", (0.72, 0.055, 0.035), roughness=0.92)
    for item in compiled["objects"]:
        if item.get("collision"):
            raise RuntimeError(f"Indexed visual object cannot have collision: {item['instance_id']}")
        position = tuple(float(v) for v in item["position_xz"])
        _, tangent, normal = sample_centerline(points, float(item["track_fraction"]))
        ground = terrain_height(track_config, float(item["track_fraction"]), int(item["side"]), float(item["distance_from_center_m"]))
        if item["kind"] == "card":
            path = repo / item["source"]
            create_card(
                f"Indexed_{item['instance_id']}_{item['asset_id']}", path, position, tangent, ground,
                collection_target, side=int(item["side"]),
                target_width=item.get("target_width_m"), target_height=item.get("target_height_m"),
            )
        elif item["kind"] == "procedural_flag":
            flag = create_trackside_card(
                f"Indexed_{item['instance_id']}_flag", position, tangent, normal,
                int(item["side"]), ground, "flag", flag_material,
            )
            move_to_collection(flag, collection_target)
        else:
            raise RuntimeError(f"Unsupported indexed object kind: {item['kind']}")
        created += 1
    return created


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    ns = parser.parse_args(args_after_double_dash())
    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    generated = repo / "blender/generated/la_chutana"
    base = repo / config["base_blend"]
    center = read_json(repo / config["centerline"])
    track_config = read_json(repo / config["track_config"])
    compiled_path = repo / config["compiled_layout"]
    if not compiled_path.exists():
        raise RuntimeError(f"Compiled semantic layout missing: {compiled_path}. Run compile_semantic_layout.py first.")
    compiled = read_json(compiled_path)
    if compiled.get("track_id") != config["track_id"]:
        raise RuntimeError(f"Compiled layout track mismatch: {compiled.get('track_id')}")
    out = repo / config["raw_output_dir"]
    out.mkdir(parents=True, exist_ok=True)
    (out / "raw_placements.json").write_text(json.dumps(compiled, indent=2) + "\n", encoding="utf-8")
    bpy.ops.wm.open_mainfile(filepath=str(base))
    env = collection("F90_RAW_ENVIRONMENT")
    library = collection("F90_RAW_LIBRARY", env)
    cards = collection("F90_RAW_CARDS", env)
    barrier_cfg = copy.deepcopy(track_config)
    barrier_cfg["tire_barriers"] = dict(config["barrier"])
    barrier_cfg["tire_barriers"]["separation_from_edge_m"] = config["barrier"]["distance_from_edge_m"]
    barrier_cfg["tire_barriers"]["procedural"] = True
    barrier_cfg["tire_barriers"]["both_sides"] = False
    barrier_cfg["tire_barriers"]["collision_height_m"] = config["barrier"]["collision_height_m"]
    barrier_cfg["tire_barriers"]["collision_thickness_m"] = config["barrier"]["collision_thickness_m"]
    points = center["points_xz"]
    side = int(config["outer_side"])
    white = flat_material("F90_RawBarrierWhite", (0.92, 0.94, 0.96), roughness=0.92)
    navy = flat_material("F90_RawBarrierLightNavy", (0.12, 0.18, 0.28), roughness=0.94)
    visual, visual_count = create_tire_barrier_visual("Raw_Outer_TireBarrier", points, side, barrier_cfg, white)
    visual.data.materials.append(navy)
    for poly in visual.data.polygons:
        poly.material_index = (poly.index // 32) % 2
    move_to_collection(visual, env)
    create_barrier_base("Raw_Outer_TireBarrierBase", points, side, config, track_config, white, navy, env)
    collision, collision_count = create_tire_barrier_collision("Raw_Outer_TireBarrier", points, side, barrier_cfg)
    move_to_collection(collision, env)
    collision["formula90s_raw_collision"] = True
    groups = {}
    for item in compiled["vegetation"]:
        item["ground_m"] = terrain_height(track_config, float(item["track_fraction"]), int(item["side"]), float(item["distance_from_center_m"]))
        groups.setdefault(item["asset_path"], []).append(item)
    imported = 0
    for asset_path, items in sorted(groups.items()):
        path = repo / asset_path
        if not path.exists():
            raise RuntimeError(f"Compiled vegetation asset missing: {path}")
        import_asset_instances(path, items, library, Path(asset_path).stem)
        imported += len(items)
    indexed_count = build_indexed_objects(config, track_config, center, compiled, cards, repo)
    building_like = [obj.name for obj in env.all_objects if "fake_building" in obj.name.lower() or "building_" in obj.name.lower()]
    blend = out / "la_chutana_raw_environment.blend"
    glb = out / "la_chutana_raw_environment.glb"
    atomic_save_blend(blend, out / "backups")
    atomic_export_glb(glb)
    print(json.dumps({"semantic_vegetation": len(compiled["vegetation"]), "indexed_objects": indexed_count, "barrier_visual_modules": visual_count, "barrier_collision_segments": collision_count, "buildings": len(building_like), "glb": str(glb), "blend": str(blend)}, indent=2))


if __name__ == "__main__":
    main()
