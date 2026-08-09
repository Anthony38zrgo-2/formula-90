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
    instantiate_prototype,
    sample_centerline,
    terrain_height,
)
from procedural_materials_blender import build_material_library


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def build_guardrails(config, points, materials):
    sources, module_length = create_guardrail_prototype(config, materials)
    road_half = float(config["road"]["width_m"]) * 0.5
    lap_length = float(config["_centerline_length_m"])

    created = 0
    for segment in config["guardrails"]["segments"]:
        start = float(segment["start_fraction"])
        end = float(segment["end_fraction"])
        span = (end - start) % 1.0
        if span <= 1e-9:
            continue

        approximate_length = span * lap_length
        count = max(1, int(math.ceil(approximate_length / module_length)))
        for i in range(count):
            fraction = (start + (i + 0.5) / count * span) % 1.0
            pos, tangent, normal = sample_centerline(points, fraction)
            side = 1.0 if segment["side"] == "right" else -1.0
            center_distance = road_half + float(segment["offset_from_edge_m"])
            pos = (
                pos[0] + normal[0] * side * center_distance,
                pos[1] + normal[1] * side * center_distance,
            )
            ground_z = terrain_height(config, fraction, side, center_distance)
            yaw = math.atan2(float(tangent[1]), float(tangent[0]))

            instantiate_prototype(
                sources,
                f"Guardrail_{segment['name']}_{i:03d}",
                pos[0],
                pos[1],
                ground_z,
                yaw,
                1.0,
            )
            create_guardrail_collision(
                f"GuardrailCollision_{segment['name']}_{i:03d}",
                pos,
                tangent,
                module_length * 1.02,
                ground_z,
                config,
            )
            created += 1
    return created


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    ns = parser.parse_args(args_after_double_dash())

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    generated = repo / config["generated_dir"]
    runtime = repo / config["runtime_dir"]
    base_blend = generated / "track_base.blend"
    if not base_blend.exists():
        raise RuntimeError(f"Base track missing: {base_blend}. Run Base mode and validate it first.")

    center = read_json(generated / "centerline.json")
    placements = read_json(generated / "placements.json")
    config["_centerline_length_m"] = center["length_m"]
    points = center["points_xz"]

    bpy.ops.wm.open_mainfile(filepath=str(base_blend))

    materials = build_material_library(generated / "textures")
    region = str(config["procedural_environment"]["region"])
    prototypes = create_prototypes(materials, region)

    placed_count = 0
    for idx, item in enumerate(placements["placements"]):
        variant_id = item["variant_id"]
        if variant_id not in prototypes:
            raise RuntimeError(f"No Blender procedural prototype for {variant_id!r} in region {region!r}")
        x, z = item["position_xz"]
        fraction = float(item["track_fraction"])
        side = float(item["side"])
        distance = float(item["distance_from_center_m"])
        height = terrain_height(config, fraction, side, distance)
        instantiate_prototype(
            prototypes[variant_id],
            f"{item['category']}_{idx:04d}_{variant_id}",
            float(x),
            float(z),
            height,
            float(item["yaw_rad"]),
            float(item["scale"]),
        )
        placed_count += 1

    guardrail_count = build_guardrails(config, points, materials)

    blend = generated / "track_environment.blend"
    glb = runtime / f"{config['track_id']}_environment.glb"
    current_glb = runtime / f"{config['track_id']}.glb"
    backup_dir = generated / "backups" / "environment"

    atomic_save_blend(blend, backup_dir)
    atomic_export_glb(glb)
    atomic_publish(glb, current_glb)

    print(f"[blender] procedural placements={placed_count}")
    print(f"[blender] guardrail modules={guardrail_count}")
    print(f"[blender] environment blend: {blend}")
    print(f"[blender] environment glb: {glb}")
    print(f"[blender] published runtime: {current_glb}")


if __name__ == "__main__":
    main()
