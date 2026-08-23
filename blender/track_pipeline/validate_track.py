from __future__ import annotations

import argparse
from pathlib import Path
import numpy as np

from curb_manifest import load_curb_manifest
from pipeline_common import read_json, closed_polyline_length, count_self_intersections, signed_curvature
from terrain_grid import validate_heightfield, effective_far_ground_z


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate deterministic track and terrain geometry before Blender generation.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--max-length-error-percent", type=float, default=.5)
    args = parser.parse_args()

    cp = Path(args.config).resolve()
    repo = cp.parents[3]
    config = read_json(cp)
    curb_manifest = load_curb_manifest(cp, config)
    center = read_json(repo / config["generated_dir"] / "centerline.json")
    points = np.asarray(center["points_xz"], dtype=float)

    length = closed_polyline_length(points)
    target = float(center["target_length_m"])
    error = abs(length-target) / target * 100
    intersections = count_self_intersections(points)
    curvature = np.abs(signed_curvature(points))
    nz = curvature[curvature > 1e-7]
    min_radius = float(1 / nz.max()) if len(nz) else float("inf")

    curb_profiles = curb_manifest["profiles"]
    curb_max = max(float(point[1]) for profile in curb_profiles.values() for point in profile["points"])
    curb_widths = {float(profile["width_m"]) for profile in curb_profiles.values()}
    road_width = float(config["road"]["width_m"])
    surface = float(config["road"].get("surface_elevation_m", 0))
    terrain = validate_heightfield(points.tolist(), config)
    grid = float(config.get("terrain", {}).get("grid_cell_m", 6))
    sink = float(config.get("terrain", {}).get("visual_sink_m", 0))
    underlay = float(config.get("terrain", {}).get("collision_underlay_drop_m", 0))
    roadside_collision = float(config.get("terrain", {}).get("roadside_collision_width_m", 2.0))
    road_half = road_width * 0.5

    checks = {
        "length_error_percent": error <= args.max_length_error_percent,
        "centerline_self_intersections": intersections == 0,
        "road_width_positive": road_width >= 8,
        "surface_elevation_small_positive": .005 <= surface <= .08,
        "curb_widths_30_percent_wider": curb_widths == {1.508},
        "curb_height_reasonable": 0 <= curb_max <= .175,
        "terrain_grid_cell_reasonable": 3 <= grid <= 10,
        "terrain_visual_sink_visual_only": 0 <= sink <= .010,
        "terrain_collision_underlay": .05 <= underlay <= .25,
        "roadside_collision_offset_safe": road_half + roadside_collision < min_radius * .90,
        "terrain_vertices_finite": bool(terrain["finite_vertices"]),
        "terrain_triangles_nondegenerate": bool(terrain["nondegenerate_triangles"]),
        "terrain_collision_winding_upward": bool(terrain["blender_winding_upward"]),
        "terrain_collision_grid_continuous": bool(terrain["continuous_collision_grid"]),
        "terrain_collision_seam_continuous": float(terrain["max_collision_seam_error_m"]) <= 1e-6,
        "terrain_safety_floor_valid": bool(terrain["safety_floor_valid"]),
        "terrain_triangle_budget": int(terrain["triangles"]) <= 140000,
    }

    print("TRACK VALIDATION")
    print(f" length: {length:.3f} m / target {target:.3f} m ({error:.4f}% error)")
    print(f" centerline self intersections: {intersections}")
    print(f" minimum sampled radius: {min_radius:.2f} m")
    print(f" road width: {road_width:.2f} m surface elevation={surface*1000:.1f} mm")
    print(f" curb catalog: profiles={','.join(sorted(curb_profiles))} width=1.508 m max_height={curb_max*1000:.1f} mm")
    print(f" terrain grid: {terrain['vertices']} vertices / {terrain['triangles']} triangles / cell={terrain['cell_m']:.2f} m")
    print(f" terrain collision seam max error: {float(terrain['max_collision_seam_error_m'])*1000:.4f} mm")
    print(f" terrain collision winding upward: {terrain['blender_winding_upward']}")
    print(f" terrain continuous collision grid: {terrain['continuous_collision_grid']}")
    print(f" terrain far z: {effective_far_ground_z(config):.3f} m")
    print(f" terrain safety floor z: {config['terrain']['safety_floor_z_m']:.3f} m")
    for name, ok in checks.items():
        print(f" {'PASS' if ok else 'FAIL'} {name}")
    return 0 if all(checks.values()) else 2


if __name__ == "__main__":
    raise SystemExit(main())
