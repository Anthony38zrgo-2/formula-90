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
from vegetation_distribution import COLORS, choose_asset, commit_asset, load_distribution, sector_for_fraction
from safety_barrier_layout import barrier_conflict, load_safety_barrier_layout

DENSITIES = ("none", "very_low", "low", "medium", "high")
TRACKSIDE_PROP_TYPES = ("spectator", "marshal", "photographer", "flag", "sign")


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


def barrier_minimum_center_distance(config: dict, category: str, radius: float) -> float:
    if category not in {"trees", "bushes"}:
        return 0.0
    road_half = float(config["road"]["width_m"]) * 0.5
    tire = config.get("tire_barriers", {})
    if not tire.get("procedural", False):
        return 0.0
    barrier_center = road_half + float(tire.get("separation_from_edge_m", 5.0))
    barrier_half = float(tire.get("collision_thickness_m", 0.28)) * 0.5
    clearance = float(config.get("vegetation_barrier_clearance_m", {}).get(category, 0.0))
    return barrier_center + barrier_half + clearance + float(radius)


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


def place_category(category, density, config, points, track_index, occupancy, seed, distribution=None, asset_catalog=None, safety_barrier_layout=None):
    env = config["procedural_environment"]
    if category == "fake_buildings" and not env.get("fake_buildings", {}).get("enabled", True):
        return [], 0, 0
    if category == "grass" and not env.get("grass_cards", {}).get("enabled", True):
        return [], 0, 0
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
    usage = {"colors": {color: 0 for color in COLORS}, "assets": {}}

    while len(output) < target and attempts < max_attempts:
        attempts += 1
        if clusters and rng.random() < cluster_probability:
            anchor = clusters[int(rng.random() * len(clusters)) % len(clusters)]
            fraction, side, distance = _candidate_from_cluster(rng, category, anchor, min_d, max_d)
        else:
            fraction = rng.random()
            side = -1.0 if rng.random() < 0.5 else 1.0
            distance = rng.uniform(min_d, max_d)

        selected_asset = None
        if distribution and asset_catalog and category in {"trees", "bushes"}:
            selected_asset = choose_asset(rng, category, fraction, distribution, asset_catalog, usage)
            spec = selected_asset.spec
        else:
            spec = weighted_choice(rng, specs)

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

        if category in {"trees", "bushes"}:
            required_outside = barrier_minimum_center_distance(config, category, radius)
            if global_distance + 1e-6 < required_outside:
                continue
            if safety_barrier_layout and barrier_conflict(
                safety_barrier_layout, category, fraction, int(side), global_distance, radius
            ):
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
            "barrier_clearance_required_m": round(float(required_outside), 4) if category in {"trees", "bushes"} else 0.0,
            "tint_rgb": tint,
        }
        if selected_asset is not None:
            record["color_id"] = selected_asset.color
            record["family"] = selected_asset.family
            record["asset_glb"] = selected_asset.glb
            record["sector_id"] = sector_for_fraction(distribution, fraction)["id"]
        output.append(record)
        if selected_asset is not None:
            commit_asset(selected_asset, usage)
        occupancy.add(Occupant(float(candidate[0]), float(candidate[1]), radius, category, spec.id))

    if len(output) < target:
        raise RuntimeError(f"Could only place {len(output)}/{target} {category} after {attempts} attempts.")
    return output, attempts, len(clusters)


def generate_trackside_props(config: dict, points: np.ndarray) -> list[dict]:
    """Place lightweight non-collidable 2D trackside cards outside the tire perimeter."""
    props_config = config.get("trackside_props", {})
    tire_config = config.get("tire_barriers", {})
    if not props_config.get("procedural", True):
        return []
    lap_length = closed_polyline_length(points)
    road_half = float(config["road"]["width_m"]) * 0.5
    barrier_distance = road_half + float(tire_config.get("separation_from_edge_m", 5.0))
    outside_offset = float(props_config.get("outside_barrier_offset_m", 1.8))
    max_cards = max(0, int(props_config.get("max_visible_cards", 260)))
    spacing = props_config.get("spacing_m", {})
    output: list[dict] = []
    for type_index, prop_type in enumerate(TRACKSIDE_PROP_TYPES):
        step = max(12.0, float(spacing.get(prop_type, 80.0)))
        count = max(1, int(math.ceil(lap_length / step)))
        for index in range(count):
            if len(output) >= max_cards:
                return output
            fraction = ((index + 0.37 + type_index * 0.11) / count) % 1.0
            side = -1 if (index + type_index) % 2 else 1
            distance = barrier_distance + outside_offset
            pos, _, _ = interpolate_at_fraction(points, fraction)
            output.append({
                "prop_type": prop_type,
                "prop_id": f"{prop_type}_{index:03d}",
                "position_xz": [round(float(pos[0]), 4), round(float(pos[1]), 4)],
                "track_fraction": round(float(fraction), 7),
                "side": side,
                "distance_from_center_m": round(distance, 4),
                "collision": False,
            })
    return output


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
    distribution, color_catalogs = load_distribution(repo, config)
    safety_barriers = load_safety_barrier_layout(repo, config)
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
        placed, attempts, clusters = place_category(
            category, density, config, points, track_index, occupancy, ns.seed,
            distribution, color_catalogs.get(category), safety_barriers,
        )
        placements.extend(placed)
        stats[category] = {"density": density, "placed": len(placed), "attempts": attempts, "clusters": clusters}
        if category in {"trees", "bushes"} and distribution:
            stats[category]["colors"] = {
                color: sum(1 for item in placed if item.get("color_id") == color) for color in COLORS
            }
            stats[category]["unique_assets"] = len({item["variant_id"] for item in placed})

    trackside_props = generate_trackside_props(config, points)

    output = repo / config["generated_dir"] / "placements.json"
    write_json(output, {
        "track_id": config["track_id"],
        "biome": biome.id,
        "seed": ns.seed,
        "stats": stats,
        "placements": placements,
        "trackside_props": trackside_props,
    })
    print(f"[environment] biome={biome.id} wrote {output}")
    for category, stat in stats.items():
        print(f"[environment] {category}: {stat['placed']} ({stat['density']}) clusters={stat['clusters']} attempts={stat['attempts']}")
    print(f"[environment] trackside_props: {len(trackside_props)} non-collidable cards")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
