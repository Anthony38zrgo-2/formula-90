from __future__ import annotations

import argparse
from pathlib import Path
import math
import numpy as np

from pipeline_common import read_json, write_json, closed_polyline_length, interpolate_at_fraction, min_distance_to_closed_polyline, SpatialHash, Occupant, stable_rng


def weighted_asset(rng, assets):
    total = sum(max(0.0,float(a.get("weight",1.0))) for a in assets)
    if total <= 0:
        return assets[0]
    pick = rng.random()*total
    running = 0.0
    for a in assets:
        running += max(0.0,float(a.get("weight",1.0)))
        if pick <= running:
            return a
    return assets[-1]


def place_category(category, density, config, points, assets, occupancy, seed):
    veg = config["vegetation"]
    profile = veg["density_profiles"][density]
    count_key = f"{category}_per_km"
    target = int(round(profile[count_key] * closed_polyline_length(points) / 1000.0))
    zone = veg["zones"][category]
    min_d, max_d = float(zone["min_track_distance_m"]), float(zone["max_track_distance_m"])
    category_assets = [a for a in assets if a["category"] == category and a.get("valid",False)]
    if target > 0 and not category_assets:
        raise RuntimeError(f"Density {density} requests {category}, but no valid {category} assets were cataloged.")

    rng = stable_rng(seed, category)
    output = []
    attempts = 0
    max_attempts = max(500, target*60)

    while len(output) < target and attempts < max_attempts:
        attempts += 1
        asset = weighted_asset(rng, category_assets)
        fraction = rng.random()
        pos, _, normal = interpolate_at_fraction(points, fraction)
        side = -1.0 if rng.random() < 0.5 else 1.0
        distance = rng.uniform(min_d,max_d)
        candidate = pos + normal * (side*distance)
        global_distance = min_distance_to_closed_polyline(candidate, points)
        if global_distance < min_d - 0.05 or global_distance > max_d + 0.50:
            continue

        scale = rng.uniform(float(asset["scale_min"]), float(asset["scale_max"]))
        radius = max(0.05, float(asset["radius_m"])*scale)
        category_padding = {"grass":0.10,"bushes":0.40,"trees":1.25}[category]
        if not occupancy.can_place(float(candidate[0]),float(candidate[1]),radius,category_padding):
            continue

        yaw = rng.uniform(0.0, math.tau)
        tint_strength = {"grass":0.05,"bushes":0.06,"trees":0.07}[category]
        tint = [round(1.0 + rng.uniform(-tint_strength,tint_strength),4) for _ in range(3)]
        record = {
            "category":category,
            "asset_id":asset["id"],
            "position_xz":[round(float(candidate[0]),4),round(float(candidate[1]),4)],
            "yaw_rad":round(yaw,6),
            "scale":round(scale,5),
            "radius_m":round(radius,4),
            "tint_rgb":tint,
        }
        output.append(record)
        occupancy.add(Occupant(float(candidate[0]),float(candidate[1]),radius,category,asset["id"]))

    if len(output) < target:
        raise RuntimeError(f"Could only place {len(output)}/{target} {category} after {attempts} attempts.")
    return output, attempts


def main() -> int:
    p = argparse.ArgumentParser(description="Generate reproducible non-overlapping vegetation placement.")
    p.add_argument("--config", required=True)
    p.add_argument("--catalog", required=True)
    p.add_argument("--trees-density", choices=["none","very_low","low","medium","high"], default="low")
    p.add_argument("--bushes-density", choices=["none","very_low","low","medium","high"], default="low")
    p.add_argument("--grass-density", choices=["none","very_low","low","medium","high"], default="medium")
    p.add_argument("--seed", type=int, default=1995)
    ns = p.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    center = read_json(repo / config["generated_dir"] / "centerline.json")
    points = np.asarray(center["points_xz"], dtype=float)
    assets = read_json(repo / ns.catalog)["assets"]

    occupancy = SpatialHash(cell_size=8.0)
    placements = []
    stats = {}
    for category, density in (("trees",ns.trees_density),("bushes",ns.bushes_density),("grass",ns.grass_density)):
        placed, attempts = place_category(category,density,config,points,assets,occupancy,ns.seed)
        placements.extend(placed)
        stats[category] = {"density":density,"placed":len(placed),"attempts":attempts}

    output = repo / config["generated_dir"] / "placements.json"
    write_json(output, {"track_id":config["track_id"],"seed":ns.seed,"stats":stats,"placements":placements})
    print(f"[environment] wrote {output}")
    for category,stat in stats.items():
        print(f"[environment] {category}: {stat['placed']} ({stat['density']}) attempts={stat['attempts']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
