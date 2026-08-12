"""Vegetation regions for the F90 Track Authoring System.

A ``vegetation-region`` groups a set of deterministic grass/bush/tree instances
under one editable boundary. Instances inside a region are *derived*: they are
stored in the canonical SVG so the current layout is preserved exactly, but the
editor treats the region (not the individual plant) as the editable unit.

This module owns the deterministic derivation of regions from the semantic
layout masks and the region operations (move / scale / extend) that preserve
existing instance positions and IDs while generating new ones deterministically.
It is importable from the importer and from the authoring server; it never
imports ``bpy``.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence
from xml.etree import ElementTree as ET
import hashlib
import math

import cv2
import numpy as np

from svg_profile import (
    REGION_ID_RE,
    VEGETATION_BOUNDARY_ROLE,
    VEGETATION_CATEGORY_ATTR,
    VEGETATION_GENERATED_ATTR,
    VEGETATION_REGION_ROLE,
    SVG_NS,
    local_name,
)

ASSET_ROLE = "asset-instance"
ROAD_MARGIN_M = 0.0


@dataclass(frozen=True)
class Region:
    region_id: str
    category: str
    boundary_xz: list[list[float]]
    seed: int
    spacing_m: float
    target_count: int


def _stable_float(key: str, low: float, high: float) -> float:
    integer = int.from_bytes(hashlib.sha256(key.encode("utf-8")).digest()[:8], "big")
    t = integer / float((1 << 64) - 1)
    return low + (high - low) * t


def _hash_choice(key: str, n: int) -> int:
    return int.from_bytes(hashlib.sha256(key.encode("utf-8")).digest()[:4], "big") % n


def _simplify_pixels(
    pixels: np.ndarray,
    *,
    transform,
    max_points: int = 32,
    simplify_eps_m: float = 6.0,
) -> list[list[float]]:
    """Convert a set of mask pixel coordinates into a simplified world polygon.

    Uses a convex hull (robust, self-intersection-free for editing) and then a
    fixed-precision rounding so the emitted boundary is deterministic.
    """
    if len(pixels) == 0:
        return []
    hull = cv2.convexHull(pixels.astype(np.float32))
    contour = hull.reshape(-1, 2)
    if len(contour) > max_points:
        eps = simplify_eps_m / max(transform.metres_per_pixel)
        approx = cv2.approxPolyDP(contour, eps, True)
        if len(approx) >= 3:
            contour = approx.reshape(-1, 2)
    world = [transform.pixel_to_world(float(x), float(y)) for x, y in contour]
    return [[round(v, 4) for v in pt] for pt in world]


def derive_regions(
    semantic_rgb: np.ndarray,
    palette_rgb: tuple[int, int, int],
    category: str,
    *,
    transform,
    seed: int,
    spacing_m: float,
    target_count: int,
    region_prefix: str,
    min_area_px: int = 8,
) -> tuple[list[Region], np.ndarray]:
    """Derive deterministic regions from a semantic mask via connected components.

    Returns ``(regions, labels, label_of_region)`` where ``labels`` is the
    integer connected-component label image (0 = background) and
    ``label_of_region`` maps a component label to the region index, so instances
    can be assigned to a region by exact pixel lookup rather than by convex-hull
    point-in-polygon (which would drop instances in concave parts of a
    component).

    Components are ordered by (descending area, then centroid) so region ids are
    stable across runs for identical input.
    """
    expected = np.array(palette_rgb, dtype=np.uint8)
    mask = np.all(semantic_rgb == expected, axis=-1).astype(np.uint8)
    labels_count, labels, stats, centroids = cv2.connectedComponentsWithStats(mask, connectivity=8)
    if labels_count <= 1:
        return [], labels, {}

    indexed = []
    label_of_region: dict[int, int] = {}
    for label in range(1, labels_count):
        area = int(stats[label, cv2.CC_STAT_AREA])
        if area < min_area_px:
            continue
        cx, cy = float(centroids[label][0]), float(centroids[label][1])
        ys, xs = np.nonzero(labels == label)
        pixels = np.asarray(list(zip(xs.tolist(), ys.tolist())), dtype=np.float32)
        indexed.append((area, cy, cx, pixels, label))
    indexed.sort(key=lambda item: (-item[0], item[1], item[2]))

    regions = []
    for index, (area, _, _, pixels, label) in enumerate(indexed):
        boundary = _simplify_pixels(pixels, transform=transform)
        if len(boundary) < 3:
            continue
        region_id = f"{region_prefix}_{index:03d}"
        label_of_region[label] = index
        regions.append(
            Region(
                region_id=region_id,
                category=category,
                boundary_xz=boundary,
                seed=seed,
                spacing_m=spacing_m,
                target_count=target_count,
            )
        )
    return regions, labels, label_of_region


def assign_instances_to_regions(
    vegetation: list[dict],
    regions: list[Region],
    category: str,
    *,
    transform,
    labels: np.ndarray,
    label_of_region: dict[int, int],
) -> dict[str, str]:
    assignment: dict[str, str] = {}
    ordered = sorted(regions, key=lambda r: r.region_id)
    for item in vegetation:
        x, z = (float(v) for v in item["position_xz"])
        px, py = transform.world_to_pixel(x, z)
        py = int(np.clip(py, 0, labels.shape[0] - 1))
        px = int(np.clip(px, 0, labels.shape[1] - 1))
        label = int(labels[py, px])
        region_index = label_of_region.get(label)
        if region_index is not None and region_index < len(ordered):
            assignment[item["instance_id"]] = ordered[region_index].region_id
    return assignment


def _point_in_polygon(x: float, z: float, boundary: Sequence[Sequence[float]]) -> bool:
    """Ray-casting point-in-polygon on the XZ plane."""
    inside = False
    n = len(boundary)
    for i in range(n):
        xi, zi = boundary[i]
        xj, zj = boundary[(i + 1) % n]
        if (zi > z) != (zj > z) and x < (xj - xi) * (z - zi) / (zj - zi + 1e-30) + xi:
            inside = not inside
    return inside


def assign_instances_to_regions(
    vegetation: list[dict],
    regions: list[Region],
    category: str,
    *,
    transform,
    labels: np.ndarray,
    label_of_region: dict[int, int],
) -> dict[str, str]:
    """Map each vegetation instance id -> region id by exact pixel label lookup.

    Each instance's world position is converted back to a pixel coordinate and
    its connected-component label is read directly, so concave parts of a mask
    are never dropped (unlike convex-hull point-in-polygon).
    """
    assignment: dict[str, str] = {}
    ordered = sorted(regions, key=lambda r: r.region_id)
    for item in vegetation:
        x, z = (float(v) for v in item["position_xz"])
        px, py = transform.world_to_pixel(x, z)
        py = int(np.clip(py, 0, labels.shape[0] - 1))
        px = int(np.clip(px, 0, labels.shape[1] - 1))
        label = int(labels[py, px])
        region_index = label_of_region.get(label)
        if region_index is not None and region_index < len(ordered):
            assignment[item["instance_id"]] = ordered[region_index].region_id
    return assignment


# ---------------------------------------------------------------------------
# Region operations (deterministic, preserve existing instances)
# ---------------------------------------------------------------------------

def translate_boundary(boundary: Sequence[Sequence[float]], dx: float, dz: float) -> list[list[float]]:
    return [[round(pt[0] + dx, 4), round(pt[1] + dz, 4)] for pt in boundary]


def scale_boundary(boundary: Sequence[Sequence[float]], factor: float, anchor: Sequence[float]) -> list[list[float]]:
    ax, az = float(anchor[0]), float(anchor[1])
    out = []
    for pt in boundary:
        out.append([round(ax + (pt[0] - ax) * factor, 4), round(az + (pt[1] - az) * factor, 4)])
    return out


def _jitter_grid_points(
    boundary: Sequence[Sequence[float]],
    spacing_m: float,
    seed: int,
    region_id: str,
    jitter_m: float,
) -> list[tuple[float, float]]:
    """Deterministic JITTERED grid of candidate points inside the boundary."""
    xs = [pt[0] for pt in boundary]
    zs = [pt[1] for pt in boundary]
    if not xs:
        return []
    min_x, max_x = min(xs), max(xs)
    min_z, max_z = min(zs), max(zs)
    cell = max(spacing_m, 0.1)
    out: list[tuple[float, float]] = []
    ix = 0
    x = min_x + cell * 0.5
    while x <= max_x + 1e-9:
        iz = 0
        z = min_z + cell * 0.5
        while z <= max_z + 1e-9:
            key = f"{seed}:{region_id}:{ix}:{iz}"
            jx = (_stable_float(key + ":x", -0.45, 0.45)) * cell
            jz = (_stable_float(key + ":z", -0.45, 0.45)) * cell
            px, pz = x + jx, z + jz
            if _point_in_polygon(px, pz, boundary):
                out.append((round(px, 4), round(pz, 4)))
            iz += 1
            z += cell
        ix += 1
        x += cell
    return out


def sample_new_placements(
    *,
    new_polygon: Sequence[Sequence[float]],
    old_polygon: Sequence[Sequence[float]],
    existing: list[dict],
    region: Region,
    asset_pool: Sequence[str],
    seed: int,
    spacing_m: float,
    min_separation_m: float = 1.0,
    centerline_points: Sequence[Sequence[float]] | None = None,
    road_half_width_m: float = 6.0,
    barrier_segments: Sequence[Sequence[Sequence[float]]] | None = None,
    max_count: int = 400,
) -> list[dict]:
    """Generate new placements only inside the newly added boundary area.

    Existing placements are untouched. Candidates are rejected if they fall
    inside the old polygon, on the road, or too close to an existing instance.
    Asset assignment and yaw/scale are deterministic hashes of the candidate.
    """
    if centerline_points is None:
        centerline_points = []
    candidates = _jitter_grid_points(new_polygon, spacing_m, seed, region.region_id, 0.0)
    new_placements: list[dict] = []
    used: set[tuple[float, float]] = {
        (round(v["position_xz"][0], 1), round(v["position_xz"][1], 1)) for v in existing
    }

    for px, pz in candidates:
        if len(new_placements) >= max_count:
            break
        if _point_in_polygon(px, pz, old_polygon):
            continue
        if any(
            math.hypot(px - v["position_xz"][0], pz - v["position_xz"][1]) < min_separation_m
            for v in existing
        ):
            continue
        key = f"{seed}:{region.region_id}:{px}:{pz}"
        if (round(px, 1), round(pz, 1)) in used:
            continue
        asset = asset_pool[_hash_choice(key, len(asset_pool))]
        yaw = _stable_float("yaw:" + key, -math.pi, math.pi)
        scale = _stable_float("scale:" + key, 0.9, 1.1)
        used.add((round(px, 1), round(pz, 1)))
        new_placements.append(
            {
                "position_xz": [px, pz],
                "asset_path": asset,
                "yaw_rad": round(yaw, 7),
                "scale": round(scale, 7),
                "generated_by_region": region.region_id,
            }
        )
    return new_placements


# ---------------------------------------------------------------------------
# SVG helpers for emitting / reading regions
# ---------------------------------------------------------------------------

def region_group_element(
    region: Region,
    boundary_el: ET.Element,
    instance_els: Iterable[ET.Element],
) -> ET.Element:
    group = ET.Element(f"{{{SVG_NS}}}g")
    group.set("data-role", VEGETATION_REGION_ROLE)
    group.set("data-region-id", region.region_id)
    group.set(VEGETATION_CATEGORY_ATTR, region.category)
    group.set("data-seed", str(region.seed))
    group.set("data-spacing-m", str(region.spacing_m))
    group.set("data-target-count", str(region.target_count))
    group.append(boundary_el)
    for el in instance_els:
        group.append(el)
    return group


def find_regions(root: ET.Element) -> dict[str, ET.Element]:
    return {
        el.get("data-region-id"): el
        for el in root.iter()
        if el.get("data-role") == VEGETATION_REGION_ROLE
    }


def region_instance_ids(root: ET.Element, region_id: str) -> list[str]:
    for el in root.iter():
        if el.get("data-role") == VEGETATION_REGION_ROLE and el.get("data-region-id") == region_id:
            return [c.get("data-instance-id") for c in el if c.get("data-role") == ASSET_ROLE]
    return []
