from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np

from pipeline_common import read_json, min_distance_to_closed_polyline


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate deterministic procedural environment placement.")
    parser.add_argument("--config", required=True)
    ns = parser.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    center = read_json(repo / config["generated_dir"] / "centerline.json")
    placements = read_json(repo / config["generated_dir"] / "placements.json")
    points = np.asarray(center["points_xz"], dtype=float)
    zones = config["procedural_environment"]["zones"]

    failures = 0
    for item in placements["placements"]:
        category = item["category"]
        zone = zones[category]
        distance = min_distance_to_closed_polyline(item["position_xz"], points)
        min_d = float(zone["min_track_distance_m"])
        max_d = float(zone["max_track_distance_m"])
        if distance + 1e-4 < min_d:
            print(f"FAIL min-clearance {item['variant_id']} d={distance:.2f}m min={min_d:.2f}m")
            failures += 1
        if distance - 0.75 > max_d:
            print(f"FAIL max-zone {item['variant_id']} d={distance:.2f}m max={max_d:.2f}m")
            failures += 1

    items = placements["placements"]
    overlap_count = 0
    closest_gap = float("inf")
    for i, a in enumerate(items):
        ax, az = a["position_xz"]
        for b in items[i + 1:]:
            bx, bz = b["position_xz"]
            center_distance = ((ax - bx) ** 2 + (az - bz) ** 2) ** 0.5
            required = float(a["radius_m"]) + float(b["radius_m"])
            gap = center_distance - required
            closest_gap = min(closest_gap, gap)
            if gap < -1e-6:
                overlap_count += 1

    if overlap_count:
        print(f"FAIL overlaps={overlap_count}")
        failures += overlap_count
    else:
        print("PASS overlaps=0")

    if closest_gap != float("inf"):
        print(f"closest_object_gap={closest_gap:.3f}m")
    print(f"placements={len(items)} seed={placements['seed']} region={placements.get('region')}")
    return 0 if failures == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
