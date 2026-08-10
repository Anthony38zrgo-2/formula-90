from __future__ import annotations

import argparse
from pathlib import Path
import json
import math
import sys

import bpy

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from blender_output import atomic_export_glb, atomic_publish, atomic_save_blend
from procedural_assets_blender import (
    create_guardrail_collision,
    create_guardrail_prototype,
    create_prototypes,
    create_tire_barrier_collision,
    create_tire_barrier_visual,
    create_trackside_card,
    instantiate_prototype,
    sample_centerline,
    terrain_height,
)
from procedural_catalog import biome_from_config
from procedural_materials_blender import build_material_library


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def build_guardrails(config, points, materials):
    sources, module_length = create_guardrail_prototype(config, materials)
    road_half = float(config["road"]["width_m"]) * 0.5
    lap = float(config["_centerline_length_m"])
    created = 0
    for segment in config["guardrails"]["segments"]:
        start = float(segment["start_fraction"])
        end = float(segment["end_fraction"])
        span = (end - start) % 1.0
        if span <= 1e-9:
            continue
        count = max(1, int(math.ceil(span * lap / module_length)))
        for i in range(count):
            fraction = (start + (i + .5) / count * span) % 1.0
            pos, tangent, normal = sample_centerline(points, fraction)
            side = 1 if segment["side"] == "right" else -1
            distance = road_half + float(segment["offset_from_edge_m"])
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
    for side in sides:
        side_name = "Right" if side > 0 else "Left"
        visual, modules = create_tire_barrier_visual(
            f"TireBarrierVisual{side_name}", points, side, config, materials["tire_barrier"]
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
    ns = parser.parse_args(args_after_double_dash())
    cp = Path(ns.config).resolve()
    repo = cp.parents[3]
    config = read_json(cp)
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

    guards = build_guardrails(config, points, materials)
    tire_barriers = build_tire_barriers(config, points, materials)
    trackside_props = build_trackside_props(config, points, placements.get("trackside_props", []), materials)
    blend = generated / "track_environment.blend"
    glb = runtime / f"{config['track_id']}_environment.glb"
    live = runtime / f"{config['track_id']}.glb"
    atomic_save_blend(blend, generated / "backups" / "environment")
    atomic_export_glb(glb)
    atomic_publish(glb, live)
    print(f"[blender] biome={biome.id} procedural placements={placed}")
    print(f"[blender] guardrail modules={guards}")
    print(f"[blender] tire barriers visual_modules={tire_barriers['visual_modules']} collision_segments={tire_barriers['collision_segments']}")
    print(f"[blender] trackside cards={trackside_props} collision=False")
    print(f"[blender] published runtime: {live}")


if __name__ == "__main__":
    main()
