from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import csv
import hashlib
import json
import math

import cv2
import numpy as np


MARKER_RED = 224


def marker_rgb(index: int) -> tuple[int, int, int]:
    if not 1 <= int(index) <= 65535:
        raise ValueError(f"Marker index must be in 1..65535, got {index}")
    return MARKER_RED, (int(index) >> 8) & 255, int(index) & 255


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


@dataclass(frozen=True)
class WorldRasterTransform:
    width: int
    height: int
    min_x: float
    min_z: float
    max_x: float
    max_z: float

    @classmethod
    def from_metadata(cls, metadata: dict) -> "WorldRasterTransform":
        width, height = (int(v) for v in metadata["resolution"])
        min_x, min_z, max_x, max_z = (float(v) for v in metadata["world_bounds_xz"])
        return cls(width, height, min_x, min_z, max_x, max_z)

    @property
    def metres_per_pixel(self) -> tuple[float, float]:
        return (
            (self.max_x - self.min_x) / max(1, self.width - 1),
            (self.max_z - self.min_z) / max(1, self.height - 1),
        )

    def pixel_to_world(self, x: float, y: float) -> tuple[float, float]:
        world_x = self.min_x + float(x) / max(1, self.width - 1) * (self.max_x - self.min_x)
        world_z = self.max_z - float(y) / max(1, self.height - 1) * (self.max_z - self.min_z)
        return world_x, world_z

    def world_to_pixel(self, x: float, z: float) -> tuple[int, int]:
        px = round((float(x) - self.min_x) / (self.max_x - self.min_x) * (self.width - 1))
        py = round((self.max_z - float(z)) / (self.max_z - self.min_z) * (self.height - 1))
        return int(np.clip(px, 0, self.width - 1)), int(np.clip(py, 0, self.height - 1))


def nearest_centerline(point, points):
    px, pz = (float(v) for v in point)
    best = None
    count = len(points)
    for index in range(count):
        ax, az = (float(v) for v in points[index])
        bx, bz = (float(v) for v in points[(index + 1) % count])
        dx, dz = bx - ax, bz - az
        length2 = dx * dx + dz * dz
        t = 0.0 if length2 <= 1e-12 else np.clip(((px - ax) * dx + (pz - az) * dz) / length2, 0.0, 1.0)
        qx, qz = ax + dx * t, az + dz * t
        distance2 = (px - qx) ** 2 + (pz - qz) ** 2
        if best is None or distance2 < best[0]:
            length = max(math.hypot(dx, dz), 1e-9)
            tangent = (dx / length, dz / length)
            normal = (tangent[1], -tangent[0])
            signed = (px - qx) * normal[0] + (pz - qz) * normal[1]
            best = (distance2, (qx, qz), tangent, normal, (index + float(t)) / count, signed)
    distance2, position, tangent, normal, fraction, signed = best
    return {
        "position_xz": position,
        "tangent_xz": tangent,
        "normal_xz": normal,
        "track_fraction": fraction % 1.0,
        "distance_from_center_m": math.sqrt(distance2),
        "side": 1 if signed >= 0.0 else -1,
    }


def extract_markers(marker_rgba: np.ndarray, catalog: dict, minimum_area_px: int = 4) -> list[dict]:
    output = []
    rgb = marker_rgba[..., :3]
    alpha = marker_rgba[..., 3] if marker_rgba.shape[2] == 4 else np.full(rgb.shape[:2], 255, dtype=np.uint8)
    for key in sorted(catalog, key=lambda value: int(value)):
        index = int(key)
        expected = np.array(marker_rgb(index), dtype=np.uint8)
        mask = np.all(rgb == expected, axis=-1) & (alpha == 255)
        labels_count, _, stats, centroids = cv2.connectedComponentsWithStats(mask.astype(np.uint8), connectivity=8)
        for label in range(1, labels_count):
            area = int(stats[label, cv2.CC_STAT_AREA])
            if area < minimum_area_px:
                continue
            if area > 36:
                raise RuntimeError(f"Marker {index:03d} has ambiguous connected area={area}px; separate overlapping instances")
            output.append({"catalog_index": f"{index:03d}", "pixel_xy": [float(v) for v in centroids[label]], "area_px": area})
    output.sort(key=lambda item: (int(item["catalog_index"]), item["pixel_xy"][1], item["pixel_xy"][0]))
    return output


def _nearest_world_point(target, candidates: np.ndarray) -> tuple[float, float]:
    delta = candidates - np.asarray(target, dtype=np.float64)[None, :]
    index = int(np.argmin(np.einsum("ij,ij->i", delta, delta)))
    return tuple(float(v) for v in candidates[index])


def _barrier_frame(marker_pixel, barrier_pixels: np.ndarray, semantic: np.ndarray, config: dict, transform: WorldRasterTransform):
    target = np.asarray(marker_pixel, dtype=np.float64)
    delta = barrier_pixels - target[None, :]
    nearest_index = int(np.argmin(np.einsum("ij,ij->i", delta, delta)))
    center = barrier_pixels[nearest_index]
    local_delta = barrier_pixels - center[None, :]
    local = barrier_pixels[np.einsum("ij,ij->i", local_delta, local_delta) <= 35.0 ** 2]
    if len(local) < 4:
        raise RuntimeError(f"Could not estimate barrier tangent near pixel={marker_pixel}")
    covariance = np.cov((local - local.mean(axis=0)).T)
    values, vectors = np.linalg.eigh(covariance)
    tangent_px = vectors[:, int(np.argmax(values))]
    normal_a = np.array([-tangent_px[1], tangent_px[0]], dtype=np.float64)
    normal_b = -normal_a
    interior = {
        tuple(int(v) for v in config["semantic_palette"][name])
        for name in ("grass", "runoff", "asphalt")
    }

    def interior_score(direction):
        score = 0
        for distance in (5, 10, 15, 20, 25):
            sample = np.rint(center + direction * distance).astype(int)
            x = int(np.clip(sample[0], 0, transform.width - 1))
            y = int(np.clip(sample[1], 0, transform.height - 1))
            score += int(tuple(int(v) for v in semantic[y, x]) in interior)
        return score

    score_a, score_b = interior_score(normal_a), interior_score(normal_b)
    outward_px = normal_a if score_a < score_b else normal_b
    mx, mz = transform.metres_per_pixel
    outward_world = np.array([outward_px[0] * mx, -outward_px[1] * mz], dtype=np.float64)
    outward_world /= max(float(np.linalg.norm(outward_world)), 1e-9)
    tangent_world = np.array([tangent_px[0] * mx, -tangent_px[1] * mz], dtype=np.float64)
    tangent_world /= max(float(np.linalg.norm(tangent_world)), 1e-9)
    return transform.pixel_to_world(center[0], center[1]), outward_world, tangent_world


def _stable_float(key: str, low: float, high: float) -> float:
    integer = int.from_bytes(hashlib.sha256(key.encode("utf-8")).digest()[:8], "big")
    t = integer / float((1 << 64) - 1)
    return low + (high - low) * t


def _sample_zone_pixels(mask: np.ndarray, transform: WorldRasterTransform, spacing_m: float, seed: int, max_count: int):
    mx, mz = transform.metres_per_pixel
    cell_x = max(1, int(round(spacing_m / mx)))
    cell_y = max(1, int(round(spacing_m / mz)))
    rng = np.random.default_rng(int(seed))
    chosen = []
    for y0 in range(0, transform.height, cell_y):
        for x0 in range(0, transform.width, cell_x):
            ys, xs = np.nonzero(mask[y0:min(y0 + cell_y, transform.height), x0:min(x0 + cell_x, transform.width)])
            if len(xs) == 0:
                continue
            pick = int(rng.integers(0, len(xs)))
            chosen.append((x0 + int(xs[pick]), y0 + int(ys[pick])))
    rng.shuffle(chosen)
    return chosen[: int(max_count)]


def _asset_repo_path(raw_path: str) -> str:
    normalized = str(raw_path).replace("\\", "/")
    if normalized.startswith("blender/") or normalized.startswith("assets-lowpoly-python/"):
        return normalized
    return f"assets-lowpoly-python/{normalized}"


def _nearest_barrier_distance(world: tuple[float, float], barrier_world: np.ndarray) -> float:
    delta = barrier_world - np.asarray(world, dtype=np.float64)[None, :]
    return math.sqrt(float(np.min(np.einsum("ij,ij->i", delta, delta))))


def load_asset_dimensions(manifest_csv: Path) -> dict[str, dict]:
    dimensions = {}
    with manifest_csv.open("r", encoding="utf-8", newline="") as handle:
        for row in csv.DictReader(handle):
            norm = row["file"].replace("\\", "/")
            data = {
                "width_m": float(row["width_x"]),
                "height_m": float(row["height_y"]),
                "depth_m": float(row["depth_z"]),
                "category": row["category"],
            }
            dimensions[norm] = data
            stem_name = Path(norm).name
            dimensions[f"assets-lowpoly-python/nature/{row['category']}/glb/{stem_name}"] = data
            dimensions[stem_name] = data
    return dimensions


def compile_layout(layout_config_path: Path) -> dict:
    layout_config_path = layout_config_path.resolve()
    root = layout_config_path.parents[4]
    config = json.loads(layout_config_path.read_text(encoding="utf-8"))
    transform = WorldRasterTransform.from_metadata(config)
    semantic_path = root / config["semantic_image"]
    marker_path = root / config["marker_image"]
    catalog_path = root / config["object_catalog"]
    centerline_path = root / config["centerline"]
    asset_manifest_path = root / config["asset_dimensions_manifest"]
    semantic = cv2.cvtColor(cv2.imread(str(semantic_path), cv2.IMREAD_COLOR), cv2.COLOR_BGR2RGB)
    marker_bgra = cv2.imread(str(marker_path), cv2.IMREAD_UNCHANGED)
    if marker_bgra is None or semantic is None:
        raise RuntimeError("Semantic layout images are missing or unreadable")
    marker_rgba = cv2.cvtColor(marker_bgra, cv2.COLOR_BGRA2RGBA)
    if semantic.shape[:2] != (transform.height, transform.width) or marker_rgba.shape[:2] != semantic.shape[:2]:
        raise RuntimeError(f"Layout dimensions do not match metadata: semantic={semantic.shape} markers={marker_rgba.shape}")
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    centerline = json.loads(centerline_path.read_text(encoding="utf-8"))
    points = centerline["points_xz"]
    dimensions = load_asset_dimensions(asset_manifest_path)

    barrier_mask = np.zeros(semantic.shape[:2], dtype=bool)
    for name, col in config["semantic_palette"].items():
        if name == "barrier" or name.startswith("barrier_"):
            barrier_mask |= np.all(semantic == np.array(col, dtype=np.uint8), axis=-1)
    by, bx = np.nonzero(barrier_mask)
    if len(bx) < 8:
        raise RuntimeError("Semantic barrier contains fewer than eight pixels")
    barrier_pixels = np.asarray([(x, y) for x, y in zip(bx, by)], dtype=np.float64)
    barrier_world = np.asarray([transform.pixel_to_world(x, y) for x, y in barrier_pixels], dtype=np.float64)
    asphalt_rgb = np.array(config["semantic_palette"]["asphalt"], dtype=np.uint8)
    asphalt_mask = np.all(semantic == asphalt_rgb, axis=-1)
    distance_from_asphalt_px = cv2.distanceTransform((~asphalt_mask).astype(np.uint8), cv2.DIST_L2, 5)
    metres_per_pixel = sum(transform.metres_per_pixel) * 0.5

    markers = extract_markers(marker_rgba, catalog["objects"])
    compiled_objects = []
    failures = []
    per_catalog_count = {}
    for marker in markers:
        key = marker["catalog_index"]
        spec = catalog["objects"][key]
        marker_world = transform.pixel_to_world(*marker["pixel_xy"])
        requested = marker_world
        barrier_reference = None
        configured_offset = 0.0
        if spec["placement_mode"] == "barrier_relative":
            barrier_point, outward, barrier_tangent = _barrier_frame(marker["pixel_xy"], barrier_pixels, semantic, config, transform)
            configured_offset = float(spec.get("guardrail_offset_m", 0.0))
            resolved = tuple(np.asarray(barrier_point) + outward * configured_offset)
            barrier_reference = barrier_point
        elif spec["placement_mode"] == "absolute":
            resolved = requested
        else:
            failures.append(f"marker {key}: unsupported placement_mode={spec['placement_mode']}")
            continue
        correction = math.dist(requested, resolved)
        if correction > float(spec.get("max_snap_distance_m", 30.0)):
            failures.append(f"marker {key}: correction {correction:.2f}m exceeds max_snap_distance_m")
            continue
        track = nearest_centerline(resolved, points)
        occurrence = per_catalog_count.get(key, 0)
        per_catalog_count[key] = occurrence + 1
        compiled_objects.append({
            "instance_id": f"marker_{key}_{occurrence:03d}",
            "catalog_index": key,
            "asset_id": spec["asset_id"],
            "kind": spec["kind"],
            "category": spec["category"],
            "source": spec.get("source"),
            "position_xz": [round(v, 4) for v in resolved],
            "requested_position_xz": [round(v, 4) for v in requested],
            "snap_distance_m": round(correction, 4),
            "track_fraction": round(track["track_fraction"], 7),
            "distance_from_center_m": round(track["distance_from_center_m"], 4),
            "side": int(config["outer_side"]) if spec["placement_mode"] == "barrier_relative" else track["side"],
            "yaw_rad": round(math.atan2(barrier_tangent[1], barrier_tangent[0]), 7) if spec["placement_mode"] == "barrier_relative" else round(math.atan2(track["tangent_xz"][1], track["tangent_xz"][0]), 7),
            "barrier_position_xz": [round(v, 4) for v in barrier_reference] if barrier_reference else None,
            "guardrail_offset_m": configured_offset,
            "target_height_m": spec.get("target_height_m"),
            "target_width_m": spec.get("target_width_m"),
            "collision": bool(spec.get("collision", False)),
        })

    compiled_vegetation = []
    for zone_name, zone_spec in catalog["zones"].items():
        zone_rgb = np.array(config["semantic_palette"][zone_name], dtype=np.uint8)
        mask = np.all(semantic == zone_rgb, axis=-1)
        density_bands = []
        near_spec = zone_spec.get("near_track_density")
        if near_spec:
            near_mask = mask & ((distance_from_asphalt_px * metres_per_pixel) <= float(near_spec["max_distance_from_asphalt_m"]))
            far_mask = mask & ~near_mask
            near_spacing = float(zone_spec["spacing_m"]) / math.sqrt(float(near_spec["multiplier"]))
            density_bands.extend((pixel, "near") for pixel in _sample_zone_pixels(
                near_mask, transform, near_spacing,
                int(config["seed"]) + int(zone_spec["seed_offset"]) + 1009,
                int(near_spec["max_count"]),
            ))
            density_bands.extend((pixel, "far") for pixel in _sample_zone_pixels(
                far_mask, transform, float(zone_spec["spacing_m"]),
                int(config["seed"]) + int(zone_spec["seed_offset"]),
                int(near_spec["far_max_count"]),
            ))
        else:
            candidate_multiplier = 4 if zone_spec["category"] in {"trees", "bushes"} else 1
            density_bands.extend((pixel, "default") for pixel in _sample_zone_pixels(
                mask, transform, float(zone_spec["spacing_m"]),
                int(config["seed"]) + int(zone_spec["seed_offset"]),
                int(zone_spec["max_count"]) * candidate_multiplier,
            ))
        pool = [_asset_repo_path(path) for path in zone_spec["asset_pool"]]
        accepted = 0
        near_accepted = 0
        accepted_trees = []  # (x, z, footprint_radius) for tree-to-tree rejection
        for pixel, density_band in density_bands:
            if accepted >= int(zone_spec["max_count"]):
                break
            world = transform.pixel_to_world(*pixel)
            track = nearest_centerline(world, points)
            asset_index = int.from_bytes(hashlib.sha256(f"{config['seed']}:{zone_name}:{pixel}".encode()).digest()[:4], "big") % len(pool)
            asset = pool[asset_index]
            if asset not in dimensions:
                failures.append(f"zone {zone_name}: missing dimensions for {asset}")
                continue
            low, high = (float(v) for v in zone_spec["target_height_m"])
            target_height = _stable_float(f"height:{config['seed']}:{zone_name}:{pixel}", low, high)
            # tree_visual_scale is owned by the semantic contract, not the GLB
            # geometry: it scales the final target height AND the footprint so the
            # runtime world tree is larger. Applied only to category "trees".
            visual_scale = 1.0
            if zone_spec["category"] == "trees":
                visual_scale = float(config.get("tree_visual_scale", 1.0))
            scale = target_height / max(dimensions[asset]["height_m"], 1e-6)
            scale *= visual_scale
            target_height *= visual_scale
            yaw = _stable_float(f"yaw:{config['seed']}:{zone_name}:{pixel}", -math.pi, math.pi)
            footprint_radius = max(dimensions[asset]["width_m"], dimensions[asset]["depth_m"]) * scale * 0.5
            barrier_distance = _nearest_barrier_distance(world, barrier_world)
            required_barrier_distance = 0.0
            if zone_spec["category"] in {"trees", "bushes"}:
                required_barrier_distance = (
                    footprint_radius
                    + float(config.get("barrier_visual_half_thickness_m", 0.5))
                    + float(zone_spec.get("barrier_clearance_m", 0.0))
                )
                if int(track["side"]) != int(config["outer_side"]) or barrier_distance + 1e-6 < required_barrier_distance:
                    continue
                # Perimeter guard: the inner edge of the footprint must stay
                # outside the real guardrail line, not merely clear of the
                # thin barrier pixels in the semantic map. A tree sampled from
                # the interior side of the tree-zone discs (24-26 m from the
                # centerline) used to pass the pixel-distance check above.
                guardrail_center_m = float(config.get("bootstrap", {}).get("barrier_distance_from_center_m", 26.0))
                inner_edge_m = track["distance_from_center_m"] - footprint_radius
                if inner_edge_m < guardrail_center_m - 0.01:
                    continue
            if zone_spec["category"] == "trees":
                overlap = any(
                    math.hypot(world[0] - other[0], world[1] - other[1]) < footprint_radius + other[2] - 1e-3
                    for other in accepted_trees
                )
                if overlap:
                    continue
                accepted_trees.append((world[0], world[1], footprint_radius))
            compiled_vegetation.append({
                "instance_id": f"{zone_name}_{accepted:04d}",
                "category": zone_spec["category"],
                "asset_path": asset,
                "position_xz": [round(v, 4) for v in world],
                "track_fraction": round(track["track_fraction"], 7),
                "distance_from_center_m": round(track["distance_from_center_m"], 4),
                "side": track["side"],
                "yaw_rad": round(yaw, 7),
                "scale": round(scale, 7),
                "target_height_m": round(target_height, 4),
                "footprint_radius_m": round(footprint_radius, 4),
                "barrier_distance_m": round(barrier_distance, 4),
                "required_barrier_distance_m": round(required_barrier_distance, 4),
                "density_band": density_band,
                "collision": False,
            })
            accepted += 1
            near_accepted += int(density_band == "near")
        minimum = int(zone_spec.get("min_count", 0))
        if accepted < minimum:
            failures.append(f"zone {zone_name}: accepted {accepted}/{minimum} placements after constraints")
        if near_spec and near_accepted < int(near_spec["min_count"]):
            failures.append(f"zone {zone_name}: near-track density {near_accepted}/{near_spec['min_count']}")
    if failures:
        raise RuntimeError("Semantic layout validation failed:\n- " + "\n- ".join(failures))
    result = {
        "schema_version": 1,
        "track_id": config["track_id"],
        "seed": int(config["seed"]),
        "provenance": {
            "semantic_image": config["semantic_image"],
            "semantic_sha256": sha256_file(semantic_path),
            "marker_image": config["marker_image"],
            "marker_sha256": sha256_file(marker_path),
            "object_catalog": config["object_catalog"],
            "catalog_sha256": sha256_file(catalog_path),
        },
        "world_bounds_xz": config["world_bounds_xz"],
        "resolution": config["resolution"],
        "objects": compiled_objects,
        "vegetation": compiled_vegetation,
        "counts": {
            "objects": len(compiled_objects),
            "vegetation": len(compiled_vegetation),
            "by_object_category": {category: sum(1 for item in compiled_objects if item["category"] == category) for category in sorted({item["category"] for item in compiled_objects})},
            "by_vegetation_category": {category: sum(1 for item in compiled_vegetation if item["category"] == category) for category in sorted({item["category"] for item in compiled_vegetation})},
        },
    }
    return result
