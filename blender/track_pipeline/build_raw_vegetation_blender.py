from __future__ import annotations

import argparse
import copy
import json
import math
import struct
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
    create_tire_barrier_card_visual,
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
        raise RuntimeError(f"Raw vegetation asset contains no meshes: {path}")
    corners = [obj.matrix_world @ Vector(corner) for obj in meshes for corner in obj.bound_box]
    min_z = min(v.z for v in corners)
    center_x = (min(v.x for v in corners) + max(v.x for v in corners)) * 0.5
    center_y = (min(v.y for v in corners) + max(v.y for v in corners)) * 0.5
    normalize = Matrix.Translation((-center_x, -center_y, -min_z))
    for number, item in enumerate(placements):
        instance_id = item.get("instance_id", f"{number:04d}")
        x, z = item["position_xz"]
        position = godot_xz_to_blender(float(x), float(z), float(item.get("ground_m", 0.0)))
        transform = Matrix.Translation(position) @ Matrix.Rotation(-float(item.get("yaw_rad", 0.0)), 4, "Z")
        scale = float(item.get("scale", 1.0))
        transform @= Matrix.Diagonal((scale, scale, scale, 1.0))
        for part_index, source in enumerate(meshes):
            root = source.copy()
            root.data = source.data
            root.name = f"Raw_{item['category']}_{instance_id}_{asset_key}_{part_index:02d}"
            target_collection.objects.link(root)
            root.matrix_world = transform @ normalize @ source.matrix_world
            root["formula90s_raw_asset"] = str(path.as_posix())
            root["formula90s_category"] = item["category"]
            root["formula90s_instance_id"] = instance_id
            root["formula90s_ground_m"] = float(item.get("ground_m", 0.0))
            root["formula90s_collision"] = False
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


def import_mesh_prop(name: str, path: Path, position_xz: tuple[float, float], tangent_xz: tuple[float, float], ground_m: float, collection_target, side: int = 1, yaw_offset: float = 0.0):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    imported = [obj for obj in bpy.data.objects if obj not in before]
    if not imported:
        return None
    yaw = math.atan2(-tangent_xz[1], tangent_xz[0]) + (math.pi * 0.5 if side > 0 else -math.pi * 0.5) + yaw_offset
    center_b = godot_xz_to_blender(position_xz[0], position_xz[1], ground_m)
    root = None
    for obj in imported:
        if obj.parent is None:
            root = obj
            break
    if root is None and imported:
        root = imported[0]

    if root:
        root.name = name
        root.location = center_b
        root.rotation_euler = (0, 0, yaw)
    
    for obj in imported:
        move_to_collection(obj, collection_target)
        obj["formula90s_collision"] = False
    return root


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
        elif item["kind"] in ("glb", "mesh"):
            path = repo / item["source"]
            import_mesh_prop(
                f"Indexed_{item['instance_id']}_{item['asset_id']}", path, position, tangent, ground,
                collection_target, side=int(item["side"]),
            )
        else:
            raise RuntimeError(f"Unsupported indexed object kind: {item['kind']}")
        created += 1
    return created


def export_runtime_part(target: Path, objects):
    bpy.ops.object.select_all(action="DESELECT")
    selected = 0
    for obj in objects:
        if obj.hide_get():
            continue
        obj.select_set(True)
        selected += 1
    if selected == 0:
        raise RuntimeError(f"Runtime GLB part has no visible objects: {target}")
    atomic_export_glb(target, use_selection=True, export_extras=False)
    bpy.ops.object.select_all(action="DESELECT")
    return selected


def is_collision_proxy(obj) -> bool:
    return bool(
        obj.name.endswith("-colonly")
        or obj.get("formula90s_collision")
        or obj.get("formula90s_raw_collision")
    )


def glb_node_names(path: Path) -> set[str]:
    with path.open("rb") as handle:
        if handle.read(4) != b"glTF":
            raise RuntimeError(f"Invalid GLB magic: {path}")
        handle.read(8)
        json_length = struct.unpack("<I", handle.read(4))[0]
        if handle.read(4) != b"JSON":
            raise RuntimeError(f"GLB JSON chunk missing: {path}")
        document = json.loads(handle.read(json_length).decode("utf-8").rstrip("\x00 \t\r\n"))
    return {str(node.get("name", "")) for node in document.get("nodes", [])}


def validate_runtime_split(environment_glb: Path, vegetation_glb: Path, collision_names: set[str]):
    environment_names = glb_node_names(environment_glb)
    vegetation_names = glb_node_names(vegetation_glb)
    missing = sorted(collision_names - environment_names)
    leaked = sorted(collision_names & vegetation_names)
    if missing:
        raise RuntimeError(f"Physical runtime omitted collision proxies: {missing}")
    if leaked:
        raise RuntimeError(f"Vegetation runtime contains collision proxies: {leaked}")


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
    prepared_barrier = read_json(repo / config["barrier"]["prepared_manifest"])
    front_entry = prepared_barrier["sources"][prepared_barrier["module"]["front_source"]]
    front_texture = Path(front_entry["texture"])
    side_texture = Path(prepared_barrier["sources"][prepared_barrier["module"]["side_source"]]["texture"])
    top_texture = Path(prepared_barrier["sources"][prepared_barrier["module"]["top_source"]]["texture"])
    front_material = texture_material("F90_RawBarrierCardFront", front_texture, roughness=1.0, metallic=0.0, alpha=False)
    side_material = texture_material("F90_RawBarrierCardSide", side_texture, roughness=1.0, metallic=0.0, alpha=False)
    top_material = texture_material("F90_RawBarrierCardTop", top_texture, roughness=1.0, metallic=0.0, alpha=False)
    x0, y0, x1, y1 = front_entry["metrics"]["output_bbox"]
    width, height = front_entry["metrics"]["output_size"]
    front_uv_bounds = (x0 / width, 1.0 - y1 / height, x1 / width, 1.0 - y0 / height)

    barriers_manifest_path = repo / "assets-lowpoly-python" / "track_props" / "barriers" / "source_manifest.json"
    multi_materials = {}
    if barriers_manifest_path.exists():
        bm = read_json(barriers_manifest_path)
        bdir = barriers_manifest_path.parent
        for b_type, b_info in bm.get("types", {}).items():
            f_tex = bdir / b_info["front"]
            t_tex = bdir / b_info["top"]
            e_tex = bdir / b_info["end"]
            r_val = 0.6 if b_type == "guardrail_armco" else 1.0
            m_val = 0.7 if b_type == "guardrail_armco" else 0.0
            f_mat = texture_material(f"F90_Barrier_{b_type}_Front", f_tex, roughness=r_val, metallic=m_val, alpha=False)
            s_mat = texture_material(f"F90_Barrier_{b_type}_Side", e_tex, roughness=r_val, metallic=m_val, alpha=False)
            t_mat = texture_material(f"F90_Barrier_{b_type}_Top", t_tex, roughness=r_val, metallic=m_val, alpha=False)
            multi_materials[b_type] = (f_mat, s_mat, t_mat)
    layout_cfg_path = repo / config.get("semantic_layout_config", "blender/track_pipeline/layouts/la_chutana/layout_config.json")
    barrier_sectors = None
    if layout_cfg_path.exists():
        lcfg = read_json(layout_cfg_path)
        barrier_sectors = lcfg.get("legacy_barrier_sectors")

    visual, visual_count = create_tire_barrier_card_visual(
        "Raw_Outer_TireBarrier", points, side, barrier_cfg, front_material, side_material, top_material,
        front_uv_bounds=front_uv_bounds,
        multi_materials=multi_materials if multi_materials else None,
        barrier_sectors=barrier_sectors,
    )
    move_to_collection(visual, env)
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
    environment_glb = out / "la_chutana_raw_environment.glb"
    vegetation_glb = out / "la_chutana_raw_vegetation.glb"
    atomic_save_blend(blend, out / "backups")
    scene_objects = [obj for obj in bpy.context.scene.objects if not obj.hide_get()]
    vegetation_objects = [obj for obj in scene_objects if obj.get("formula90s_raw_asset") and not obj.hide_render]
    environment_objects = [
        obj for obj in scene_objects
        if not obj.get("formula90s_raw_asset") and (not obj.hide_render or is_collision_proxy(obj))
    ]
    collision_names = {obj.name for obj in environment_objects if is_collision_proxy(obj)}
    if not collision_names:
        raise RuntimeError("Physical runtime selection contains no collision proxies")
    environment_object_count = export_runtime_part(environment_glb, environment_objects)
    vegetation_object_count = export_runtime_part(vegetation_glb, vegetation_objects)
    validate_runtime_split(environment_glb, vegetation_glb, collision_names)
    print(json.dumps({
        "semantic_vegetation": len(compiled["vegetation"]),
        "indexed_objects": indexed_count,
        "barrier_visual_modules": visual_count,
        "barrier_collision_segments": collision_count,
        "buildings": len(building_like),
        "environment_objects": environment_object_count,
        "vegetation_objects": vegetation_object_count,
        "collision_proxies": len(collision_names),
        "environment_glb": str(environment_glb),
        "vegetation_glb": str(vegetation_glb),
        "blend": str(blend),
    }, indent=2))


if __name__ == "__main__":
    main()
