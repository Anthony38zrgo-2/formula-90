from __future__ import annotations

import argparse
from pathlib import Path
import numpy as np

from pipeline_common import read_json, min_distance_to_closed_polyline


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--config", required=True)
    p.add_argument("--catalog", required=True)
    ns = p.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    center = read_json(repo / config["generated_dir"] / "centerline.json")
    placements = read_json(repo / config["generated_dir"] / "placements.json")
    points = np.asarray(center["points_xz"], dtype=float)

    failures = 0
    for item in placements["placements"]:
        zone = config["vegetation"]["zones"][item["category"]]
        d = min_distance_to_closed_polyline(item["position_xz"],points)
        if d + 1e-4 < float(zone["min_track_distance_m"]):
            print(f"FAIL clearance {item['asset_id']} d={d:.2f}m")
            failures += 1

    items = placements["placements"]
    overlap_count = 0
    for i,a in enumerate(items):
        ax,az = a["position_xz"]
        for b in items[i+1:]:
            bx,bz = b["position_xz"]
            r = float(a["radius_m"]) + float(b["radius_m"])
            if (ax-bx)**2 + (az-bz)**2 < r*r:
                overlap_count += 1

    if overlap_count:
        print(f"FAIL overlaps={overlap_count}")
        failures += overlap_count
    else:
        print("PASS overlaps=0")
    print(f"placements={len(items)} seed={placements['seed']}")
    return 0 if failures == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
