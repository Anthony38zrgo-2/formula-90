from __future__ import annotations

import argparse
from pathlib import Path
import math

import numpy as np

from pipeline_common import read_json, closed_polyline_length, count_self_intersections, signed_curvature


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate deterministic track geometry before Blender generation.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--max-length-error-percent", type=float, default=0.5)
    args = parser.parse_args()

    config_path = Path(args.config).resolve()
    repo_root = config_path.parents[3]
    config = read_json(config_path)
    center = read_json(repo_root / config["generated_dir"] / "centerline.json")
    points = np.asarray(center["points_xz"], dtype=float)

    length = closed_polyline_length(points)
    target = float(center["target_length_m"])
    error_pct = abs(length - target) / target * 100.0
    intersections = count_self_intersections(points)
    curvature = np.abs(signed_curvature(points))
    nonzero = curvature[curvature > 1e-7]
    min_radius = float(1.0 / nonzero.max()) if len(nonzero) else float("inf")

    curb_profile = config["curb"]["profile"]
    curb_max = max(float(p[1]) for p in curb_profile)
    curb_width = float(config["curb"]["width_m"])
    road_width = float(config["road"]["width_m"])
    surface_z = float(config["road"].get("surface_elevation_m", 0.0))
    terrain_clearance = float(config.get("terrain", {}).get("road_clearance_m", 0.0))
    max_bank = max((abs(float(zone.get("degrees", 0.0))) for zone in config.get("banking", [])), default=0.0)
    lowest_banked_edge = surface_z - math.tan(math.radians(max_bank)) * road_width * 0.5
    requested_far_z = float(config.get("terrain", {}).get("far_ground_z_m", -0.05))

    checks = {
        "length_error_percent": error_pct <= args.max_length_error_percent,
        "self_intersections": intersections == 0,
        "road_width_positive": road_width >= 8.0,
        "surface_elevation_small_positive": 0.005 <= surface_z <= 0.08,
        "terrain_clearance_small_positive": 0.005 <= terrain_clearance <= 0.04,
        "curb_width_reasonable": 0.20 <= curb_width <= 1.20,
        "curb_height_reasonable": 0.0 <= curb_max <= 0.050,
    }
    passed = all(checks.values())

    print("TRACK VALIDATION")
    print(f" length: {length:.3f} m / target {target:.3f} m ({error_pct:.4f}% error)")
    print(f" self intersections: {intersections}")
    print(f" minimum sampled radius: {min_radius:.2f} m")
    print(f" road width: {road_width:.2f} m")
    print(f" road surface elevation: {surface_z*1000:.1f} mm")
    print(f" terrain clearance at edge: {terrain_clearance*1000:.1f} mm")
    print(f" lowest banked road edge estimate: {lowest_banked_edge:.3f} m")
    print(f" requested far ground z: {requested_far_z:.3f} m (builder may lower it automatically)")
    print(f" curb: width={curb_width:.3f} m max_height={curb_max*1000:.1f} mm")
    for name, ok in checks.items():
        print(f" {'PASS' if ok else 'FAIL'} {name}")
    return 0 if passed else 2


if __name__ == "__main__":
    raise SystemExit(main())
