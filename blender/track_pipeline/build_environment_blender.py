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
from procedural_assets_blender import (
    create_prototypes,
    create_safety_barrier_ribbon_collision,
    create_safety_barrier_prototype,
    create_tire_barrier_collision,
    create_tire_barrier_card_visual,
    create_tire_barrier_visual,
    create_trackside_card,
    instantiate_prototype,
    sample_centerline,
    terrain_height,
)
from procedural_catalog import biome_from_config
from procedural_materials_blender import add_safety_barrier_materials, build_material_library
from safety_barrier_layout import compile_layout, load_safety_barrier_layout


def load_glb_vegetation_prototypes(repo, placement_items):
    prototypes = {}
    for item in placement_items:
        asset_glb = item.get("asset_glb")
        variant_id = item["variant_id"]
        if not asset_glb or variant_id in prototypes:
            continue
        before = set(bpy.data.objects)
        bpy.ops.import_scene.gltf(filepath=str(repo / asset_glb))
        sources = [obj for obj in bpy.data.objects if obj not in before and obj.type == "MESH"]
        if not sources:
            raise RuntimeError(f"GLB vegetation asset has no mesh objects: {asset_glb}")
        for source in sources:
            source.hide_render = True
            source.hide_viewport = True
            source.hide_set(True)
        prototypes[variant_id] = sources
    return prototypes


def _aim(camera, target):
    camera.rotation_euler = (Vector(target) - camera.location).to_track_quat("-Z", "Y").to_euler()


def render_review_captures(config, points, output_dir, prefix="vegetation", viewpoints=None):
    output_dir.mkdir(parents=True, exist_ok=True)
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1280
    scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.world.color = (0.08, 0.10, 0.13)
    bpy.ops.object.light_add(type="SUN", location=(0, 0, 120))
    sun = bpy.context.object
    sun.rotation_euler = (math.radians(28), math.radians(-18), math.radians(24))
    sun.data.energy = 2.4
    bpy.ops.object.camera_add()
    camera = bpy.context.object
    camera.data.lens = 42
    scene.camera = camera

    xs = [float(point[0]) for point in points]
    zs = [float(point[1]) for point in points]
    center = ((min(xs) + max(xs)) * 0.5, -(min(zs) + max(zs)) * 0.5, 0.0)
    span = max(max(xs) - min(xs), max(zs) - min(zs))
    camera.location = (center[0], center[1], span * 0.92)
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = span * 1.18
    _aim(camera, center)
    review_token = "" if prefix == "safety_barrier" else "_review"
    aerial = output_dir / f"{prefix}{review_token}_aerial.png"
    scene.render.filepath = str(aerial)
    bpy.ops.render.render(write_still=True)

    camera.data.type = "PERSP"
    camera.data.lens = 46
    captures = [aerial]
    for name, fraction in (viewpoints or {"trackside": 0.56}).items():
        pos, _, _ = sample_centerline(points, fraction)
        look, _, _ = sample_centerline(points, fraction + 0.025)
        camera.location = (float(pos[0]), -float(pos[1]), 2.2)
        _aim(camera, (float(look[0]), -float(look[1]), 1.0))
        target = output_dir / f"{prefix}{review_token}_{name}.png"
        scene.render.filepath = str(target)
        bpy.ops.render.render(write_still=True)
        captures.append(target)
    return captures


def render_safety_barrier_catalog(output_dir):
    representatives = {}
    for obj in bpy.data.objects:
        if obj.get("formula90s_safety_barrier") and obj.get("barrier_type") not in representatives:
            representatives[obj.get("barrier_type")] = obj
    order = ("guardrail_2", "guardrail_3", "tire_stack", "tecpro", "concrete_wall", "concrete_jersey")
    clones = []
    original_visibility = {obj: obj.hide_render for obj in bpy.data.objects}
    for obj in original_visibility:
        obj.hide_render = True
    for index, barrier_type in enumerate(order):
        source = representatives.get(barrier_type)
        if source is None:
            continue
        root = source.copy()
        root.data = None
        root.location = ((index - 2.5) * 7.0, 0.0, 0.0)
        root.rotation_euler = (0.0, 0.0, 0.0)
        root.scale = (1.0, 1.0, 1.0)
        root.hide_render = False
        bpy.context.scene.collection.objects.link(root)
        clones.append(root)
        for child in source.children:
            copy = child.copy()
            copy.data = child.data
            copy.parent = root
            copy.matrix_parent_inverse.identity()
            copy.hide_render = False
            copy.hide_viewport = False
            copy.hide_set(False)
            bpy.context.scene.collection.objects.link(copy)
            clones.append(copy)
    scene = bpy.context.scene
    scene.world.color = (0.08, 0.10, 0.13)
    bpy.ops.object.light_add(type="SUN", location=(0.0, -8.0, 12.0))
    catalog_sun = bpy.context.object
    catalog_sun.rotation_euler = (math.radians(32), math.radians(-18), math.radians(24))
    catalog_sun.data.energy = 3.0
    clones.append(catalog_sun)
    camera = scene.camera
    if camera is None:
        bpy.ops.object.camera_add()
        camera = bpy.context.object
        scene.camera = camera
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 48.0
    camera.hide_render = False
    camera.location = (0.0, -38.0, 7.5)
    _aim(camera, (0.0, 0.0, 0.55))
    target = output_dir / "safety_barrier_catalog.png"
    scene.render.filepath = str(target)
    bpy.ops.render.render(write_still=True)
    for obj in reversed(clones):
        bpy.data.objects.remove(obj, do_unlink=True)
    for obj, hidden in original_visibility.items():
        if obj.name in bpy.data.objects:
            obj.hide_render = hidden
    return target


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _pattern_color(manifest, module):
    pattern = manifest["patterns"][module["material_style"]]
    mode = pattern["mode"]
    if mode == "longitudinal_band":
        return pattern["base"]
    if mode == "sparse_panels":
        period = max(1, int(pattern["period_modules"]))
        if module["module_index"] % period:
            return pattern["base"]
        colors = pattern["colors"]
        return colors[(module["pattern_phase"] // period) % len(colors)]
    spans = [max(1, int(value)) for value in pattern["span_modules"]]
    colors = pattern["colors"]
    position = (module["module_index"] + module["pattern_phase"]) % sum(spans)
    for color, span in zip(colors, spans):
        if position < span:
            return color
        position -= span
    return colors[-1]


def build_safety_barriers(config, points, materials):
    if not config.get("safety_barriers", {}).get("procedural", True):
        return {"modules": 0, "collisions": 0, "counts": {}, "sha256": ""}
    repo = Path(config["_repo_root"])
    manifest = load_safety_barrier_layout(repo, config)
    compiled = compile_layout(manifest, float(config["_centerline_length_m"]))
    guardrail_bitmap = (repo / "assets-lowpoly-python" / "track_props" / "barriers" /
                        "guardrail_armco" / "textures" / "front_card_rgba_256x128.png")
    tire_bitmap = (repo / "assets-lowpoly-python" / "track_props" / "barriers" /
                   "tire_black" / "textures" / "tirewall_front_albedo_128x256.png")
    add_safety_barrier_materials(materials, manifest["palette"], guardrail_bitmap, tire_bitmap)
    prototype_cache = {}
    counts = {}
    segment_lookup = {segment["id"]: segment for segment in compiled["segments"]}
    modules_by_segment = {}
    for module in compiled["modules"]:
        segment = segment_lookup[module["segment_id"]]
        fraction = float(module["fraction"])
        side = 1 if module["side"] == "right" else -1
        distance = float(module["center_distance_m"])
        prototype = manifest["prototypes"][module["type"]]
        if prototype["geometry"] == "guardrail_armco":
            along = module["module_index"] * float(module["length_m"])
            remaining = float(segment["compiled_length_m"]) - along - float(module["length_m"])
            terminal_length = float(manifest["defaults"]["terminal_length_m"])
            terminal_flare = float(manifest["defaults"]["terminal_flare_m"])
            if segment["terminal_start"] == "flare_out":
                distance += terminal_flare * max(0.0, 1.0 - along / terminal_length)
            if segment["terminal_end"] == "flare_out":
                distance += terminal_flare * max(0.0, 1.0 - remaining / terminal_length)
        color = _pattern_color(manifest, module)
        cache_key = (module["type"], color)
        if cache_key not in prototype_cache:
            prototype_cache[cache_key] = create_safety_barrier_prototype(
                module["type"], prototype, materials, color,
            )
        pos, tangent, normal = sample_centerline(points, fraction)
        pos = (pos[0] + normal[0] * side * distance,
               pos[1] + normal[1] * side * distance)
        ground = terrain_height(config, fraction, side, distance)
        yaw = math.atan2(float(tangent[1]), float(tangent[0]))
        root = instantiate_prototype(
            prototype_cache[cache_key],
            f"Safety_{module['segment_id']}_{module['module_index']:04d}_{module['type']}",
            pos[0], pos[1], ground, yaw, float(module["length_m"]) / float(prototype["module_length_m"]),
        )
        root["formula90s_safety_barrier"] = True
        root["segment_id"] = module["segment_id"]
        root["barrier_type"] = module["type"]
        root["pattern_phase"] = int(module["pattern_phase"])
        segment_module_count = int(segment["module_count"])
        if prototype["geometry"] == "jersey_profile":
            ramp = 1.0
            if segment["terminal_start"] == "jersey_end" and module["module_index"] < 3:
                ramp = min(ramp, (module["module_index"] + 1) / 3.0)
            from_end = segment_module_count - int(module["module_index"])
            if segment["terminal_end"] == "jersey_end" and from_end <= 3:
                ramp = min(ramp, from_end / 3.0)
            root.scale.z *= ramp
        counts[module["type"]] = counts.get(module["type"], 0) + 1
        collision_module = dict(module)
        collision_module["_position_xz"] = pos
        collision_module["_ground_z"] = ground
        modules_by_segment.setdefault(module["segment_id"], []).append(collision_module)

    collisions = 0
    for segment_id, segment_modules in modules_by_segment.items():
        first = segment_modules[0]
        profile = manifest["collision_profiles"][first["collision_profile"]]
        samples = [(item["_position_xz"], float(item["_ground_z"]))
                   for item in segment_modules]
        create_safety_barrier_ribbon_collision(
            f"SafetyCollision_{segment_id}", samples, profile,
        )
        collisions += 1
    prototype_objects = [obj for objects in prototype_cache.values() for obj in objects]
    unique_meshes = {obj.data for obj in prototype_objects if obj.data is not None}
    vertices = sum(len(mesh.vertices) for mesh in unique_meshes)
    triangles = sum(sum(max(0, len(poly.vertices) - 2) for poly in mesh.polygons)
                    for mesh in unique_meshes)
    return {"modules": len(compiled["modules"]), "collisions": collisions,
            "counts": counts, "sha256": compiled["sha256"],
            "segments": compiled["segments"], "prototype_count": len(prototype_cache),
            "material_count": 6, "vertices": vertices, "triangles": triangles}


def build_tire_barriers(config, points, materials):
    settings = config.get("tire_barriers", {})
    if not settings.get("procedural", True):
        return {"visual_modules": 0, "collision_segments": 0}
    sides = (1, -1) if settings.get("both_sides", True) else (1,)
    visual_modules = 0
    collision_segments = 0
    visual_mode = settings.get("visual_mode", "legacy")
    asset_glb = settings.get("visual_asset_glb")
    card_materials = None
    front_uv_bounds = (0.0, 0.0, 1.0, 1.0)
    if visual_mode == "rectangular_prism":
        manifest_path = Path(settings["prepared_manifest"])
        if not manifest_path.is_absolute():
            manifest_path = Path(config["_repo_root"]) / manifest_path
        prepared = read_json(manifest_path)
        front_entry = prepared["sources"][prepared["module"]["front_source"]]
        front = Path(front_entry["texture"])
        side_texture = Path(prepared["sources"][prepared["module"]["side_source"]]["texture"])
        top_texture = Path(prepared["sources"][prepared["module"]["top_source"]]["texture"])
        x0, y0, x1, y1 = front_entry["metrics"]["output_bbox"]
        width, height = front_entry["metrics"]["output_size"]
        front_uv_bounds = (x0 / width, 1.0 - y1 / height, x1 / width, 1.0 - y0 / height)
        from procedural_materials_blender import texture_material
        card_materials = (
            texture_material("F90_TireBarrierCardFront", front, roughness=1.0, metallic=0.0, alpha=False),
            texture_material("F90_TireBarrierCardSide", side_texture, roughness=1.0, metallic=0.0, alpha=False),
            texture_material("F90_TireBarrierCardTop", top_texture, roughness=1.0, metallic=0.0, alpha=False),
        )
        # Check for multi-barrier materials and layout sectors
        repo = Path(config["_repo_root"])
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
        layout_cfg_path = repo / "blender" / "track_pipeline" / "layouts" / config.get("track_id", "la_chutana") / "layout_config.json"
        barrier_sectors = None
        if layout_cfg_path.exists():
            lcfg = read_json(layout_cfg_path)
            barrier_sectors = lcfg.get("barrier_sectors", lcfg.get("legacy_barrier_sectors"))
    for side in sides:
        side_name = "Right" if side > 0 else "Left"
        if card_materials:
            visual, modules = create_tire_barrier_card_visual(
                f"TireBarrierVisual{side_name}", points, side, config, *card_materials,
                front_uv_bounds=front_uv_bounds,
                multi_materials=multi_materials if multi_materials else None,
                barrier_sectors=barrier_sectors,
                exclude_spans=settings.get("exclude_spans", []),
            )
        else:
            visual, modules = create_tire_barrier_visual(
                f"TireBarrierVisual{side_name}", points, side, config, materials["tire_barrier"],
                asset_glb=asset_glb,
            )
        collision, segments = create_tire_barrier_collision(
            f"TireBarrierCollision{side_name}", points, side, config
        )
        visual_modules += modules
        collision_segments += segments
    return {"visual_modules": visual_modules, "collision_segments": collision_segments}


def build_trackside_props(config, points, props, materials):
    created = 0
    for item in props:
        prop_type = str(item["prop_type"])
        if prop_type not in {"spectator", "marshal", "photographer", "flag", "sign"}:
            raise RuntimeError(f"Unknown trackside prop type: {prop_type}")
        pos, tangent, normal = sample_centerline(points, float(item["track_fraction"]))
        side = int(item["side"])
        distance = float(item["distance_from_center_m"])
        authored = item.get("position_xz")
        card_pos = ((float(authored[0]), float(authored[1])) if authored else
                    (pos[0] + normal[0] * side * distance,
                     pos[1] + normal[1] * side * distance))
        ground = terrain_height(config, float(item["track_fraction"]), side, distance)
        create_trackside_card(
            f"Trackside_{prop_type}_{item['prop_id']}",
            card_pos,
            tangent,
            normal,
            side,
            ground,
            prop_type,
            materials[prop_type],
        )
        created += 1
    return created


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    parser.add_argument("--vegetation-review", action="store_true")
    parser.add_argument("--guardrail-review", action="store_true")
    parser.add_argument("--safety-barrier-review", action="store_true")
    ns = parser.parse_args(args_after_double_dash())
    cp = Path(ns.config).resolve()
    repo = cp.parents[3]
    config = read_json(cp)
    config["_repo_root"] = str(repo)
    generated = repo / config["generated_dir"]
    runtime = repo / config["runtime_dir"]
    base = generated / "track_base.blend"
    if not base.exists():
        raise RuntimeError(f"Base track missing: {base}. Run Base mode and validate it first.")

    center = read_json(generated / "centerline.json")
    placements = read_json(generated / "placements.json")
    config["_centerline_length_m"] = center["length_m"]
    points = center["points_xz"]
    bpy.ops.wm.open_mainfile(filepath=str(base))

    materials = build_material_library(generated / "textures")
    for key in ("spectator", "marshal", "photographer", "flag", "sign"):
        materials[key].use_backface_culling = True
    biome = biome_from_config(config)
    prototypes = create_prototypes(materials, config)
    prototypes.update(load_glb_vegetation_prototypes(repo, placements["placements"]))
    placed = 0
    for idx, item in enumerate(placements["placements"]):
        variant_id = item["variant_id"]
        if variant_id not in prototypes:
            raise RuntimeError(f"No Blender procedural prototype for {variant_id!r} in biome {biome.id!r}")
        x, z = item["position_xz"]
        h = terrain_height(
            config,
            float(item["track_fraction"]),
            float(item["side"]),
            float(item["distance_from_center_m"]),
        )
        instantiate_prototype(
            prototypes[variant_id],
            f"{item['category']}_{idx:04d}_{variant_id}",
            float(x), float(z), h,
            float(item["yaw_rad"]),
            float(item["scale"]),
            item.get("tint_rgb"),
            float(item.get("width_scale", 1.0)),
            float(item.get("height_scale", 1.0)),
        )
        placed += 1

    review_mode = ns.vegetation_review or ns.guardrail_review or ns.safety_barrier_review
    safety = ({"modules": 0, "collisions": 0, "counts": {}, "sha256": ""}
              if ns.vegetation_review else build_safety_barriers(config, points, materials))
    tire_barriers = ({"visual_modules": 0, "collision_segments": 0}
                     if ns.vegetation_review or ns.guardrail_review
                     else build_tire_barriers(config, points, materials))
    trackside_props = (0 if ns.vegetation_review or ns.guardrail_review
                       else build_trackside_props(config, points, placements.get("trackside_props", []), materials))
    review_name = ("safety_barrier" if ns.safety_barrier_review else
                   "guardrail" if ns.guardrail_review else "vegetation")
    blend = generated / (f"track_{review_name}_review.blend" if review_mode else "track_environment.blend")
    glb = runtime / (f"{config['track_id']}_{review_name}_review.glb" if review_mode else f"{config['track_id']}_environment.glb")
    live = runtime / f"{config['track_id']}.glb"
    atomic_save_blend(blend, generated / "backups" / "environment")
    atomic_export_glb(glb)
    if review_mode:
        viewpoints = ({"main_straight": 0.96, "t1": 0.08, "t4": 0.40,
                       "chicane": 0.57, "t6": 0.79}
                      if ns.safety_barrier_review else
                      {"t1": 0.055, "t4": 0.36, "chicane": 0.535, "t6": 0.735}
                      if ns.guardrail_review else None)
        captures = render_review_captures(config, points, generated / "review", review_name, viewpoints)
        if ns.safety_barrier_review:
            catalog = render_safety_barrier_catalog(generated / "review")
            captures.append(catalog)
            report = generated / "review" / "safety_barrier_report.json"
            report.write_text(json.dumps(safety, indent=2), encoding="utf-8")
            print(f"[blender] safety barrier report: {report}")
        print(f"[blender] human-gate captures: {' '.join(map(str, captures))}")
    else:
        atomic_publish(glb, live)
    print(f"[blender] biome={biome.id} procedural placements={placed}")
    print(f"[blender] safety barriers modules={safety['modules']} collisions={safety['collisions']} counts={safety['counts']} sha256={safety['sha256']}")
    print(f"[blender] tire barriers visual_modules={tire_barriers['visual_modules']} collision_segments={tire_barriers['collision_segments']}")
    print(f"[blender] trackside cards={trackside_props} collision=False")
    print(f"[blender] {'review only; runtime not published' if review_mode else f'published runtime: {live}'}")


if __name__ == "__main__":
    main()
