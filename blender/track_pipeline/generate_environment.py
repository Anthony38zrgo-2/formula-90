from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import math

import numpy as np

from pipeline_common import (
    read_json,
    write_json,
    closed_polyline_length,
    interpolate_at_fraction,
    SpatialHash,
    Occupant,
    stable_rng,
)
from procedural_catalog import biome_from_config, specs_for_biome, weighted_choice
from terrain_grid import SegmentSpatialIndex, nearest_track_sample

DENSITIES = ("none", "very_low", "low", "medium", "high")


@dataclass(frozen=True)
class ClusterAnchor:
    fraction: float
    side: float
    distance_m: float


CLUSTER_DEFAULTS = {
    "grass": {"probability": 0.86, "per_km": 7.0, "fraction_sigma": 0.0060, "distance_sigma_m": 3.4},
    "bushes": {"probability": 0.76, "per_km": 4.0, "fraction_sigma": 0.0080, "distance_sigma_m": 5.0},
    "trees": {"probability": 0.62, "per_km": 2.8, "fraction_sigma": 0.0120, "distance_sigma_m": 8.0},
    "fake_buildings": {"probability": 0.52, "per_km": 1.4, "fraction_sigma": 0.0180, "distance_sigma_m": 12.0},
}


def _zone_distances(config: dict, category: str) -> tuple[float, float]:
    env = config["procedural_environment"]
    zone = env["zones"][category]
    road_half = float(config["road"]["width_m"]) * 0.5
    if "min_edge_clearance_m" in zone:
        min_d = road_half + float(zone["min_edge_clearance_m"])
    else:
        min_d = float(zone["min_track_distance_m"])
    max_d = float(zone["max_track_distance_m"])
    if max_d <= min_d:
        raise ValueError(f"Invalid {category} zone: max distance must exceed minimum")
    return min_d, max_d


def _make_clusters(rng, category: str, lap_length: float, min_d: float, max_d: float) -> list[ClusterAnchor]:
    settings = CLUSTER_DEFAULTS[category]
    count = max(3, int(round(float(settings["per_km"]) * lap_length / 1000.0)))
    anchors = []
    for _ in range(count):
        anchors.append(ClusterAnchor(
            fraction=rng.random(),
            side=-1.0 if rng.random() < 0.5 else 1.0,
            distance_m=rng.uniform(min_d, max_d),
        ))
    return anchors


def _candidate_from_cluster(rng, category: str, anchor: ClusterAnchor, min_d: float, max_d: float):
    settings = CLUSTER_DEFAULTS[category]
    fraction = (anchor.fraction + rng.gauss(0.0, float(settings["fraction_sigma"]))) % 1.0
    distance = min(max_d, max(min_d, anchor.distance_m + rng.gauss(0.0, float(settings["distance_sigma_m"]))))
    return fraction, anchor.side, distance


def place_category(category, density, config, points, track_index, occupancy, seed):
    env = config["procedural_environment"]
    profile = env["density_profiles"][density]
    lap_length = closed_polyline_length(points)
    target = int(round(float(profile[f"{category}_per_km"]) * lap_length / 1000.0))
    min_d, max_d = _zone_distances(config, category)
    biome = biome_from_config(config)
    specs = specs_for_biome(biome, category)
    rng = stable_rng(seed, f"{biome.id}:{category}")
    cluster_rng = stable_rng(seed, f"{biome.id}:{category}:clusters")
    clusters = _make_clusters(cluster_rng, category, lap_length, min_d, max_d)
    cluster_probability = float(CLUSTER_DEFAULTS[category]["probability"])

    output = []
    attempts = 0
    max_attempts = max(1000, target * 140)
    road_half = float(config["road"]["width_m"]) * 0.5
    edge_clearance = min_d - road_half

    while len(output) < target and attempts < max_attempts:
        attempts += 1
        spec = weighted_choice(rng, specs)
        if clusters and rng.random() < cluster_probability:
            anchor = clusters[int(rng.random() * len(clusters)) % len(clusters)]
            fraction, side, distance = _candidate_from_cluster(rng, category, anchor, min_d, max_d)
        else:
            fraction = rng.random()
            side = -1.0 if rng.random() < 0.5 else 1.0
            distance = rng.uniform(min_d, max_d)

        pos, tangent, normal = interpolate_at_fraction(points, fraction)
        candidate = pos + normal * (side * distance)
        nearest = nearest_track_sample(track_index, float(candidate[0]), float(candidate[1]), max_d + 40.0)
        if nearest is None:
            continue
        global_distance = float(nearest.distance_m)

        scale = rng.uniform(spec.scale_min, spec.scale_max)
        radius = max(.05, spec.radius_m * scale)
        minimum_center_distance = road_half + edge_clearance + radius
        if global_distance + 1e-6 < minimum_center_distance or global_distance > max_d + 1.0:
            continue

        padding = {"grass": .04, "bushes": .30, "trees": .95, "fake_buildings": 2.5}[category]
        if not occupancy.can_place(float(candidate[0]), float(candidate[1]), radius, padding):
            continue

        if category == "fake_buildings":
            yaw = math.atan2(float(tangent[1]), float(tangent[0])) + rng.uniform(-.14, .14)
            width_scale = rng.uniform(.88, 1.14)
            height_scale = rng.uniform(.92, 1.12)
        elif category == "trees":
            yaw = rng.uniform(0.0, math.tau)
            width_scale = rng.uniform(.90, 1.14)
            height_scale = rng.uniform(.96, 1.14)
        elif category == "bushes":
            yaw = rng.uniform(0.0, math.tau)
            width_scale = rng.uniform(1.02, 1.24)
            height_scale = rng.uniform(.94, 1.08)
        else:
            yaw = rng.uniform(0.0, math.tau)
            width_scale = rng.uniform(.92, 1.12)
            height_scale = rng.uniform(.90, 1.10)

        strength = {"grass": .020, "bushes": .026, "trees": .022, "fake_buildings": .014}[category]
        tint = [round(1.0 + rng.uniform(-strength, strength), 4) for _ in range(3)]

        record = {
            "category": category,
            "variant_id": spec.id,
            "position_xz": [round(float(candidate[0]), 4), round(float(candidate[1]), 4)],
            "track_fraction": round(float(fraction), 7),
            "side": int(side),
            "distance_from_center_m": round(float(distance), 4),
            "distance_to_track_m": round(float(global_distance), 4),
            "yaw_rad": round(float(yaw), 6),
            "scale": round(float(scale), 5),
            "width_scale": round(float(width_scale), 5),
            "height_scale": round(float(height_scale), 5),
            "radius_m": round(float(radius), 4),
            "tint_rgb": tint,
        }
        output.append(record)
        occupancy.add(Occupant(float(candidate[0]), float(candidate[1]), radius, category, spec.id))

    if len(output) < target:
        raise RuntimeError(f"Could only place {len(output)}/{target} {category} after {attempts} attempts.")
    return output, attempts, len(clusters)


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate reproducible clustered procedural environment placement.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--trees-density", choices=DENSITIES, default="low")
    parser.add_argument("--bushes-density", choices=DENSITIES, default="low")
    parser.add_argument("--grass-density", choices=DENSITIES, default="medium")
    parser.add_argument("--buildings-density", choices=DENSITIES, default="very_low")
    parser.add_argument("--seed", type=int, default=1995)
    ns = parser.parse_args()

    cp = Path(ns.config).resolve()
    repo = cp.parents[3]
    config = read_json(cp)
    center = read_json(repo / config["generated_dir"] / "centerline.json")
    points = np.asarray(center["points_xz"], dtype=float)
    biome = biome_from_config(config)
    occupancy = SpatialHash(cell_size=10.0)
    track_index = SegmentSpatialIndex.build(points.tolist(), cell_size=32.0)
    placements = []
    stats = {}

    for category, density in (
        ("fake_buildings", ns.buildings_density),
        ("trees", ns.trees_density),
        ("bushes", ns.bushes_density),
        ("grass", ns.grass_density),
    ):
        placed, attempts, clusters = place_category(category, density, config, points, track_index, occupancy, ns.seed)
        placements.extend(placed)
        stats[category] = {"density": density, "placed": len(placed), "attempts": attempts, "clusters": clusters}

    output = repo / config["generated_dir"] / "placements.json"
    write_json(output, {
        "track_id": config["track_id"],
        "biome": biome.id,
        "seed": ns.seed,
        "stats": stats,
        "placements": placements,
    })
    print(f"[environment] biome={biome.id} wrote {output}")
    for category, stat in stats.items():
        print(f"[environment] {category}: {stat['placed']} ({stat['density']}) clusters={stat['clusters']} attempts={stat['attempts']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
