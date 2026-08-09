from __future__ import annotations

import argparse
from pathlib import Path

from pipeline_common import read_json, write_json, reference_pixels_to_world, fit_periodic_centerline, closed_polyline_length, tangents_and_normals, signed_curvature


def main() -> int:
    parser = argparse.ArgumentParser(description="Deterministically fit and resample a Formula90s track centerline.")
    parser.add_argument("--config", required=True)
    args = parser.parse_args()

    config_path = Path(args.config).resolve()
    repo_root = config_path.parents[3]
    config = read_json(config_path)
    reference = read_json(repo_root / config["reference_file"])

    raw = reference_pixels_to_world(reference)
    target = float(reference["target_length_m"])
    preliminary = fit_periodic_centerline(raw, float(config["sample_spacing_m"]))
    scale = target / closed_polyline_length(preliminary)
    fitted = fit_periodic_centerline(preliminary * scale, float(config["sample_spacing_m"]))

    tangents, normals = tangents_and_normals(fitted)
    curvature = signed_curvature(fitted)
    length = closed_polyline_length(fitted)

    output = repo_root / config["generated_dir"] / "centerline.json"
    write_json(output, {
        "track_id": config["track_id"],
        "target_length_m": target,
        "length_m": length,
        "sample_spacing_m": float(config["sample_spacing_m"]),
        "points_xz": fitted.round(6).tolist(),
        "tangents_xz": tangents.round(8).tolist(),
        "normals_xz": normals.round(8).tolist(),
        "curvature_1pm": curvature.round(10).tolist(),
    })
    print(f"[track] wrote {output}")
    print(f"[track] length={length:.3f}m target={target:.3f}m error={length-target:+.3f}m")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
