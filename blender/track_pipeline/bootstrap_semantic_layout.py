from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

import cv2
import numpy as np

from raw_vegetation_common import outer_side
from semantic_layout_common import WorldRasterTransform, marker_rgb


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def sample_centerline(points, fraction):
    count = len(points)
    value = (float(fraction) % 1.0) * count
    index = int(math.floor(value)) % count
    t = value - math.floor(value)
    ax, az = points[index]
    bx, bz = points[(index + 1) % count]
    x, z = ax + (bx - ax) * t, az + (bz - az) * t
    dx, dz = bx - ax, bz - az
    length = max(math.hypot(dx, dz), 1e-9)
    tangent = (dx / length, dz / length)
    return (x, z), tangent, (tangent[1], -tangent[0])


def offset_position(points, fraction, side, distance):
    position, _, normal = sample_centerline(points, fraction)
    return position[0] + normal[0] * side * distance, position[1] + normal[1] * side * distance


def rgb_to_bgr(rgb):
    return tuple(int(v) for v in reversed(rgb))


def draw_world_polyline(image, transform, points, color_rgb, thickness_m):
    pixels = np.asarray([transform.world_to_pixel(x, z) for x, z in points], dtype=np.int32).reshape((-1, 1, 2))
    mx, mz = transform.metres_per_pixel
    thickness = max(1, int(round(float(thickness_m) / ((mx + mz) * 0.5))))
    cv2.polylines(image, [pixels], True, rgb_to_bgr(color_rgb), thickness=thickness, lineType=cv2.LINE_8)


def main() -> int:
    parser = argparse.ArgumentParser(description="Create the editable initial semantic and indexed-marker maps for a track.")
    parser.add_argument("--layout-config", required=True)
    args = parser.parse_args()
    config_path = Path(args.layout_config).resolve()
    repo = config_path.parents[4]
    config = read_json(config_path)
    transform = WorldRasterTransform.from_metadata(config)
    centerline = read_json(repo / config["centerline"])
    placements = read_json(repo / config["source_placements"])
    points = centerline["points_xz"]
    exterior = outer_side(points)
    palette = config["semantic_palette"]
    bootstrap = config["bootstrap"]
    semantic = np.full((transform.height, transform.width, 3), rgb_to_bgr(palette["background"]), dtype=np.uint8)

    # Broad editable zones. Later hand edits to this lossless PNG become authoritative.
    draw_world_polyline(semantic, transform, points, palette["grass"], bootstrap["grass_band_width_m"])
    for item in placements.get("placements", []):
        category = str(item.get("category"))
        if category not in {"trees", "bushes"}:
            continue
        minimum = float(bootstrap["barrier_distance_from_center_m"]) + (8.0 if category == "trees" else 4.0)
        distance = max(minimum, float(item.get("distance_from_center_m", minimum)))
        x, z = offset_position(points, float(item["track_fraction"]), exterior, distance)
        px, py = transform.world_to_pixel(x, z)
        radius_m = bootstrap["tree_zone_radius_m"] if category == "trees" else bootstrap["bush_zone_radius_m"]
        radius_px = max(2, int(round(radius_m / sum(transform.metres_per_pixel) * 2.0)))
        cv2.circle(semantic, (px, py), radius_px, rgb_to_bgr(palette[category]), thickness=-1, lineType=cv2.LINE_8)

    draw_world_polyline(semantic, transform, points, palette["runoff"], bootstrap["runoff_width_m"])
    draw_world_polyline(semantic, transform, points, palette["asphalt"], bootstrap["road_width_m"])
    barrier_sectors = config.get("barrier_sectors", [])
    if barrier_sectors:
        n_samples = max(200, len(points) * 4)
        for sector in barrier_sectors:
            start_f = float(sector["start_fraction"])
            end_f = float(sector["end_fraction"])
            color_rgb = sector.get("color", palette.get("barrier", [32, 56, 100]))
            if start_f <= end_f:
                count_s = max(2, int((end_f - start_f) * n_samples) + 2)
                fractions = [start_f + (end_f - start_f) * i / (count_s - 1) for i in range(count_s)]
            else:
                len_wrapped = (1.0 - start_f) + end_f
                count_s = max(2, int(len_wrapped * n_samples) + 2)
                fractions = [(start_f + len_wrapped * i / (count_s - 1)) % 1.0 for i in range(count_s)]
            sector_points = [offset_position(points, f, exterior, float(bootstrap["barrier_distance_from_center_m"])) for f in fractions]
            draw_world_polyline(semantic, transform, sector_points, color_rgb, 1.2)
    else:
        barrier_points = [offset_position(points, index / len(points), exterior, bootstrap["barrier_distance_from_center_m"]) for index in range(len(points))]
        draw_world_polyline(semantic, transform, barrier_points, palette["barrier"], 1.2)

    marker_bgra = np.zeros((transform.height, transform.width, 4), dtype=np.uint8)
    occupied = np.zeros((transform.height, transform.width), dtype=bool)
    marker_records = []
    sign_cycle = [5, 6, 7, 8]
    spectator_cycle = [1, 2]
    counters = {"sign": 0, "spectator": 0}
    marker_size = int(bootstrap["marker_size_px"])
    for item in placements.get("trackside_props", []):
        prop = str(item["prop_type"])
        if prop == "spectator":
            index = spectator_cycle[counters["spectator"] % len(spectator_cycle)]
            counters["spectator"] += 1
        elif prop == "marshal":
            index = 3
        elif prop == "photographer":
            index = 4
        elif prop == "sign":
            index = sign_cycle[counters["sign"] % len(sign_cycle)]
            counters["sign"] += 1
        elif prop == "flag":
            index = 9
        else:
            continue
        x, z = offset_position(points, float(item["track_fraction"]), exterior, bootstrap["barrier_distance_from_center_m"])
        px, py = transform.world_to_pixel(x, z)
        half = marker_size // 2
        offsets = [(0, 0)] + [(dx, dy) for radius in (4, 8, 12, 16, 20) for dx, dy in ((radius, 0), (-radius, 0), (0, radius), (0, -radius), (radius, radius), (-radius, radius), (radius, -radius), (-radius, -radius))]
        selected = None
        for dx, dy in offsets:
            cx, cy = px + dx, py + dy
            if cx - half < 0 or cy - half < 0 or cx + half >= transform.width or cy + half >= transform.height:
                continue
            if not occupied[cy - half:cy + half + 1, cx - half:cx + half + 1].any():
                selected = (cx, cy)
                break
        if selected is None:
            raise RuntimeError(f"Could not place marker {index:03d} near pixel {(px, py)} without overlap")
        px, py = selected
        occupied[py - half:py + half + 1, px - half:px + half + 1] = True
        marker_records.append((index, px, py, half))

    # Labels are human-facing; exact colored cores are drawn last and remain machine-authoritative.
    for index, px, py, half in marker_records:
        cv2.putText(marker_bgra, f"{index:02d}", (px + half + 2, py - half - 1), cv2.FONT_HERSHEY_PLAIN, 0.8, (255, 255, 255, 255), 1, cv2.LINE_8)
    for index, px, py, half in marker_records:
        rgb = marker_rgb(index)
        cv2.rectangle(marker_bgra, (px - half, py - half), (px + half, py + half), (*rgb_to_bgr(rgb), 255), thickness=-1, lineType=cv2.LINE_8)

    semantic_path = repo / config["semantic_image"]
    marker_path = repo / config["marker_image"]
    preview_path = repo / config["preview_image"]
    semantic_path.parent.mkdir(parents=True, exist_ok=True)
    if not cv2.imwrite(str(semantic_path), semantic):
        raise RuntimeError(f"Could not write {semantic_path}")
    if not cv2.imwrite(str(marker_path), marker_bgra):
        raise RuntimeError(f"Could not write {marker_path}")
    preview = cv2.cvtColor(semantic, cv2.COLOR_BGR2BGRA)
    alpha = marker_bgra[..., 3:4].astype(np.float32) / 255.0
    preview[..., :3] = np.rint(marker_bgra[..., :3] * alpha + preview[..., :3] * (1.0 - alpha)).astype(np.uint8)
    if not cv2.imwrite(str(preview_path), preview):
        raise RuntimeError(f"Could not write {preview_path}")
    print(json.dumps({"semantic": str(semantic_path), "markers": str(marker_path), "preview": str(preview_path), "outer_side": exterior}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
