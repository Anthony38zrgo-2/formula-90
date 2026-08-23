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
    create_guardrail_collision,
    create_guardrail_prototype,
    create_prototypes,
    create_tire_barrier_collision,
    create_tire_barrier_card_visual,
    create_tire_barrier_visual,
    create_trackside_card,
    instantiate_prototype,
    sample_centerline,
    terrain_height,
)
from procedural_catalog import biome_from_config
from procedural_materials_blender import build_material_library
from guardrail_layout import load_guardrail_layout


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
    aerial = output_dir / f"{prefix}_review_aerial.png"
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
        target = output_dir / f"{prefix}_review_{name}.png"
        scene.render.filepath = str(target)
        bpy.ops.render.render(write_still=True)
        captures.append(target)
    return captures


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def build_guardrails(config, points, materials):
    if not config.get("guardrails", {}).get("procedural", True):
        return 0
    repo = Path(config["_repo_root"])
    layout = load_guardrail_layout(repo, config)
    module_length = float(layout.get("module_length_m", config["guardrails"].get("module_length_m", 4.0)))
    terminal_length = float(layout.get("terminal_length_m", 10.0))
    terminal_flare = float(layout.get("terminal_flare_m", 2.5))
    prototype_cache = {}
    lap = float(config["_centerline_length_m"])
    created = 0
    for segment in layout["segments"]:
        start = float(segment["start_fraction"])
        end = float(segment["end_fraction"])
        span = (end - start) % 1.0
        if span <= 1e-9:
            continue
        count = max(1, int(math.ceil(span * lap / module_length)))
        rail_count = int(segment["rail_count"])
        if rail_count not in prototype_cache:
            prototype_cache[rail_count] = create_guardrail_prototype(config, materials, rail_count=rail_count)[0]
        sources = prototype_cache[rail_count]
        for i in range(count):
            fraction = (start + (i + .5) / count * span) % 1.0
            pos, tangent, normal = sample_centerline(points, fraction)
            side = 1 if segment["side"] == "right" else -1
            distance = float(segment["center_distance_m"])
            along_m = (i + 0.5) * span * lap / count
            remaining_m = span * lap - along_m
            flare = terminal_flare * max(0.0, 1.0 - min(along_m, remaining_m) / terminal_length)
            distance += flare
            pos = (
                pos[0] + normal[0] * side * distance,
                pos[1] + normal[1] * side * distance,
            )
            ground = terrain_height(config, fraction, side, distance)
            yaw = math.atan2(float(tangent[1]), float(tangent[0]))
            instantiate_prototype(
                sources,
                f"Guardrail_{segment['name']}_{i:03d}",
                pos[0], pos[1], ground, yaw, 1.0,
            )
            create_guardrail_collision(
                f"GuardrailCollision_{segment['name']}_{i:03d}",
                pos, tangent, module_length * 1.02, ground, config,
            )
            created += 1
    return created


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
            barrier_sectors = lcfg.get("barrier_sectors")
    for side in sides:
        side_name = "Right" if side > 0 else "Left"
        if card_materials:
            visual, modules = create_tire_barrier_card_visual(
                f"TireBarrierVisual{side_name}", points, side, config, *card_materials,
                front_uv_bounds=front_uv_bounds,
                multi_materials=multi_materials if multi_materials else None,
                barrier_sectors=barrier_sectors,
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
        card_pos = (pos[0] + normal[0] * side * distance, pos[1] + normal[1] * side * distance)
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

    review_mode = ns.vegetation_review or ns.guardrail_review
    guards = 0 if ns.vegetation_review else build_guardrails(config, points, materials)
    tire_barriers = {"visual_modules": 0, "collision_segments": 0} if review_mode else build_tire_barriers(config, points, materials)
    trackside_props = 0 if review_mode else build_trackside_props(config, points, placements.get("trackside_props", []), materials)
    review_name = "guardrail" if ns.guardrail_review else "vegetation"
    blend = generated / (f"track_{review_name}_review.blend" if review_mode else "track_environment.blend")
    glb = runtime / (f"{config['track_id']}_{review_name}_review.glb" if review_mode else f"{config['track_id']}_environment.glb")
    live = runtime / f"{config['track_id']}.glb"
    atomic_save_blend(blend, generated / "backups" / "environment")
    atomic_export_glb(glb)
    if review_mode:
        viewpoints = {"t1": 0.055, "t4": 0.36, "chicane": 0.535, "t6": 0.735} if ns.guardrail_review else None
        captures = render_review_captures(config, points, generated / "review", review_name, viewpoints)
        print(f"[blender] human-gate captures: {' '.join(map(str, captures))}")
    else:
        atomic_publish(glb, live)
    print(f"[blender] biome={biome.id} procedural placements={placed}")
    print(f"[blender] guardrail modules={guards}")
    print(f"[blender] tire barriers visual_modules={tire_barriers['visual_modules']} collision_segments={tire_barriers['collision_segments']}")
    print(f"[blender] trackside cards={trackside_props} collision=False")
    print(f"[blender] {'review only; runtime not published' if review_mode else f'published runtime: {live}'}")


if __name__ == "__main__":
    main()
