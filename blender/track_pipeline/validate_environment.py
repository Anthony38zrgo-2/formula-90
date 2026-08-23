from __future__ import annotations

import argparse
from pathlib import Path

from pipeline_common import read_json, SpatialHash, Occupant
from generate_environment import barrier_minimum_center_distance
from vegetation_distribution import COLORS, load_distribution
from guardrail_layout import guardrail_conflict, load_guardrail_layout


def _minimum_center_distance(config: dict, category: str, radius: float) -> float:
    road_half = float(config["road"]["width_m"]) * 0.5
    zone = config["procedural_environment"]["zones"][category]
    if "min_edge_clearance_m" in zone:
        return road_half + float(zone["min_edge_clearance_m"]) + float(radius)
    return float(zone["min_track_distance_m"]) + float(radius)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate procedural environment placement before Blender assembly.")
    parser.add_argument("--config", required=True)
    ns = parser.parse_args()
    cp = Path(ns.config).resolve()
    repo = cp.parents[3]
    config = read_json(cp)
    generated = repo / config["generated_dir"]
    placements_doc = read_json(generated / "placements.json")
    items = placements_doc["placements"]
    distribution, catalogs = load_distribution(repo, config)
    guardrails = load_guardrail_layout(repo, config)

    failures = 0
    occupancy = SpatialHash(cell_size=10.0)
    overlap_count = 0
    clearance_failures = 0

    padding_by_category = {"grass": .04, "bushes": .30, "trees": .95, "fake_buildings": 2.5}
    for item in items:
        category = item["category"]
        x, z = map(float, item["position_xz"])
        radius = float(item["radius_m"])
        d = float(item.get("distance_to_track_m", item["distance_from_center_m"]))
        minimum = _minimum_center_distance(config, category, radius)
        if d + 1e-4 < minimum:
            print(f"FAIL clearance {item['variant_id']} d={d:.3f} required={minimum:.3f}")
            clearance_failures += 1
        if category in {"trees", "bushes"}:
            barrier_minimum = barrier_minimum_center_distance(config, category, radius)
            if d + 1e-4 < barrier_minimum:
                print(f"FAIL barrier clearance {item['variant_id']} d={d:.3f} required={barrier_minimum:.3f}")
                clearance_failures += 1

        padding = padding_by_category[category]
        if not occupancy.can_place(x, z, radius, padding):
            overlap_count += 1
        occupancy.add(Occupant(x, z, radius, category, item["variant_id"]))

    if clearance_failures:
        failures += clearance_failures
    else:
        print("PASS road-edge clearance")
    if overlap_count:
        print(f"FAIL overlaps={overlap_count}")
        failures += overlap_count
    else:
        print("PASS overlaps=0")

    props = placements_doc.get("trackside_props", [])
    tire = config.get("tire_barriers", {})
    props_cfg = config.get("trackside_props", {})
    road_half = float(config["road"]["width_m"]) * 0.5
    minimum_prop_distance = road_half + float(tire.get("separation_from_edge_m", 5.0)) + float(props_cfg.get("outside_barrier_offset_m", 1.8))
    prop_failures = 0
    for prop in props:
        distance = float(prop.get("distance_from_center_m", 0.0))
        if distance + 1e-4 < minimum_prop_distance or prop.get("collision", False):
            prop_failures += 1
    if prop_failures:
        print(f"FAIL trackside props={prop_failures} required_distance={minimum_prop_distance:.3f}m")
        failures += prop_failures
    else:
        print(f"PASS trackside props={len(props)} non-collidable outside perimeter")

    expected = sum(int(v["placed"]) for v in placements_doc["stats"].values())
    if expected != len(items):
        print(f"FAIL manifest count expected={expected} actual={len(items)}")
        failures += 1
    else:
        print(f"PASS manifest count={len(items)}")

    if distribution:
        for category in ("trees", "bushes"):
            category_items = [item for item in items if item["category"] == category]
            unique = {item["variant_id"] for item in category_items}
            expected_assets = {asset.spec.id for asset in catalogs[category]}
            if unique != expected_assets:
                print(f"FAIL {category} catalog coverage={len(unique)}/{len(expected_assets)}")
                failures += 1
            else:
                print(f"PASS {category} catalog coverage={len(unique)}/{len(expected_assets)}")
            counts = {color: sum(item.get("color_id") == color for item in category_items) for color in COLORS}
            if counts["original"] != max(counts.values()):
                print(f"FAIL {category} dominant color is not original: {counts}")
                failures += 1
            else:
                print(f"PASS {category} dominant=original colors={counts}")

    guardrail_conflicts = sum(
        guardrail_conflict(
            guardrails, item["category"], float(item["track_fraction"]), int(item["side"]),
            float(item.get("distance_to_track_m", item["distance_from_center_m"])), float(item["radius_m"]),
        )
        for item in items if item["category"] in {"trees", "bushes"}
    )
    if guardrail_conflicts:
        print(f"FAIL guardrail/vegetation conflicts={guardrail_conflicts}")
        failures += guardrail_conflicts
    else:
        print("PASS guardrail/vegetation conflicts=0")

    print(f"biome={placements_doc.get('biome')} seed={placements_doc['seed']}")
    return 0 if failures == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
