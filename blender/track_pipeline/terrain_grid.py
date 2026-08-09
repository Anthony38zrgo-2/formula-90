from __future__ import annotations

from dataclasses import dataclass
import math
from typing import Sequence


@dataclass(frozen=True)
class NearestTrackSample:
    distance_m: float
    fraction: float
    side: float
    signed_offset_m: float


@dataclass
class SegmentSpatialIndex:
    points: list[tuple[float, float]]
    cell_size: float
    cells: dict[tuple[int, int], list[int]]

    @classmethod
    def build(cls, points: Sequence[Sequence[float]], cell_size: float = 32.0) -> "SegmentSpatialIndex":
        pts = [(float(p[0]), float(p[1])) for p in points]
        cells: dict[tuple[int, int], list[int]] = {}
        n = len(pts)
        for i in range(n):
            ax, az = pts[i]
            bx, bz = pts[(i + 1) % n]
            min_x, max_x = sorted((ax, bx))
            min_z, max_z = sorted((az, bz))
            ix0 = math.floor(min_x / cell_size)
            ix1 = math.floor(max_x / cell_size)
            iz0 = math.floor(min_z / cell_size)
            iz1 = math.floor(max_z / cell_size)
            for ix in range(ix0, ix1 + 1):
                for iz in range(iz0, iz1 + 1):
                    cells.setdefault((ix, iz), []).append(i)
        return cls(pts, float(cell_size), cells)

    def candidates(self, x: float, z: float, radius_m: float) -> set[int]:
        r = max(float(radius_m), self.cell_size)
        ix0 = math.floor((x - r) / self.cell_size)
        ix1 = math.floor((x + r) / self.cell_size)
        iz0 = math.floor((z - r) / self.cell_size)
        iz1 = math.floor((z + r) / self.cell_size)
        result: set[int] = set()
        for ix in range(ix0, ix1 + 1):
            for iz in range(iz0, iz1 + 1):
                result.update(self.cells.get((ix, iz), ()))
        return result


def bank_degrees_at_fraction(config: dict, fraction: float) -> float:
    result = 0.0
    f = float(fraction) % 1.0
    for zone in config.get("banking", []):
        center = float(zone["center_fraction"]) % 1.0
        half = max(float(zone["half_width_fraction"]), 1e-6)
        d = abs((f - center + 0.5) % 1.0 - 0.5)
        if d <= half:
            weight = 0.5 * (1.0 + math.cos(math.pi * d / half))
            result += float(zone["degrees"]) * weight
    return result


def effective_far_ground_z(config: dict) -> float:
    road_half = float(config["road"]["width_m"]) * 0.5
    surface_z = float(config["road"].get("surface_elevation_m", 0.025))
    requested = float(config.get("terrain", {}).get("far_ground_z_m", -0.10))
    max_bank = max((abs(float(zone.get("degrees", 0.0))) for zone in config.get("banking", [])), default=0.0)
    lowest_banked_edge = surface_z - math.tan(math.radians(max_bank)) * road_half
    safety = float(config.get("terrain", {}).get("far_ground_safety_m", 0.05))
    return min(requested, lowest_banked_edge - safety)


def nearest_track_sample(index: SegmentSpatialIndex, x: float, z: float, search_radius_m: float) -> NearestTrackSample | None:
    candidates = index.candidates(float(x), float(z), search_radius_m)
    if not candidates:
        return None

    best_d2 = float("inf")
    best_fraction = 0.0
    best_side = 1.0
    best_signed = 0.0
    n = len(index.points)

    for i in candidates:
        ax, az = index.points[i]
        bx, bz = index.points[(i + 1) % n]
        vx, vz = bx - ax, bz - az
        denom = vx * vx + vz * vz
        if denom <= 1e-12:
            continue
        t = ((x - ax) * vx + (z - az) * vz) / denom
        t = max(0.0, min(1.0, t))
        qx, qz = ax + vx * t, az + vz * t
        dx, dz = x - qx, z - qz
        d2 = dx * dx + dz * dz
        if d2 >= best_d2:
            continue

        length = math.sqrt(denom)
        tx, tz = vx / length, vz / length
        nx, nz = tz, -tx
        signed = dx * nx + dz * nz
        best_d2 = d2
        best_fraction = (i + t) / n
        best_side = 1.0 if signed >= 0.0 else -1.0
        best_signed = signed

    if not math.isfinite(best_d2):
        return None
    return NearestTrackSample(math.sqrt(best_d2), best_fraction % 1.0, best_side, best_signed)


def road_surface_height(config: dict, fraction: float, signed_offset_m: float) -> float:
    surface_z = float(config["road"].get("surface_elevation_m", 0.025))
    bank = math.radians(bank_degrees_at_fraction(config, fraction))
    return surface_z + math.tan(bank) * float(signed_offset_m)


def terrain_height_from_sample(config: dict, sample: NearestTrackSample | None, *, visual: bool = False) -> float:
    terrain = config.get("terrain", {})
    far_z = effective_far_ground_z(config)
    if sample is None:
        return far_z - (float(terrain.get("visual_sink_m", 0.002)) if visual else 0.0)

    road_half = float(config["road"]["width_m"]) * 0.5
    shoulder_width = max(float(terrain.get("shoulder_falloff_m", 18.0)), 0.1)
    visual_sink = float(terrain.get("visual_sink_m", 0.002)) if visual else 0.0

    if sample.distance_m <= road_half:
        height = road_surface_height(config, sample.fraction, sample.signed_offset_m)
        return height - visual_sink

    edge_signed = road_half * sample.side
    edge_height = road_surface_height(config, sample.fraction, edge_signed)
    outside = sample.distance_m - road_half
    t = min(1.0, max(0.0, outside / shoulder_width))
    t = t * t * (3.0 - 2.0 * t)
    height = edge_height * (1.0 - t) + far_z * t
    return height - visual_sink


def build_heightfield(points: Sequence[Sequence[float]], config: dict, *, visual: bool = False) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int]], dict]:
    terrain = config.get("terrain", {})
    cell = max(2.0, float(terrain.get("grid_cell_m", 8.0)))
    margin = max(20.0, float(terrain.get("far_ground_margin_m", 220.0)))
    road_half = float(config["road"]["width_m"]) * 0.5
    shoulder_width = max(float(terrain.get("shoulder_falloff_m", 18.0)), 0.1)
    seam_overlap = max(0.0, float(terrain.get("collision_seam_overlap_m", 0.15)))

    pts = [(float(p[0]), float(p[1])) for p in points]
    min_x = math.floor((min(p[0] for p in pts) - margin) / cell) * cell
    max_x = math.ceil((max(p[0] for p in pts) + margin) / cell) * cell
    min_z = math.floor((min(p[1] for p in pts) - margin) / cell) * cell
    max_z = math.ceil((max(p[1] for p in pts) + margin) / cell) * cell

    nx = int(round((max_x - min_x) / cell)) + 1
    nz = int(round((max_z - min_z) / cell)) + 1
    index = SegmentSpatialIndex.build(pts, cell_size=max(24.0, cell * 4.0))
    influence = road_half + shoulder_width + cell * 2.0

    vertices: list[tuple[float, float, float]] = []
    distances: list[float] = []
    for iz in range(nz):
        z = min_z + iz * cell
        for ix in range(nx):
            x = min_x + ix * cell
            sample = nearest_track_sample(index, x, z, influence)
            distance = sample.distance_m if sample is not None else float("inf")
            y = terrain_height_from_sample(config, sample, visual=visual)
            vertices.append((x, z, y))
            distances.append(distance)

    faces: list[tuple[int, int, int]] = []
    skipped_inside = 0
    for iz in range(nz - 1):
        for ix in range(nx - 1):
            a = iz * nx + ix
            b = a + 1
            c = a + nx + 1
            d = a + nx
            ds = (distances[a], distances[b], distances[c], distances[d])
            if max(ds) < road_half - seam_overlap:
                skipped_inside += 1
                continue
            faces.append((a, b, c))
            faces.append((a, c, d))

    stats = {
        "cell_m": cell,
        "vertices": len(vertices),
        "triangles": len(faces),
        "nx": nx,
        "nz": nz,
        "skipped_inside_quads": skipped_inside,
        "far_ground_z_m": effective_far_ground_z(config),
    }
    return vertices, faces, stats


def validate_heightfield(points: Sequence[Sequence[float]], config: dict) -> dict[str, object]:
    vertices, faces, stats = build_heightfield(points, config, visual=False)
    finite = all(all(math.isfinite(v) for v in vertex) for vertex in vertices)
    nondegenerate = True
    for ia, ib, ic in faces:
        ax, az, _ = vertices[ia]
        bx, bz, _ = vertices[ib]
        cx, cz, _ = vertices[ic]
        area2 = abs((bx - ax) * (cz - az) - (bz - az) * (cx - ax))
        if area2 <= 1e-9:
            nondegenerate = False
            break

    road_half = float(config["road"]["width_m"]) * 0.5
    max_seam_error = 0.0
    samples = 64
    for i in range(samples):
        fraction = i / samples
        for side in (-1.0, 1.0):
            signed = road_half * side
            synthetic = NearestTrackSample(road_half, fraction, side, signed)
            terrain_h = terrain_height_from_sample(config, synthetic, visual=False)
            road_h = road_surface_height(config, fraction, signed)
            max_seam_error = max(max_seam_error, abs(terrain_h - road_h))

    return {
        **stats,
        "finite_vertices": finite,
        "nondegenerate_triangles": nondegenerate,
        "max_collision_seam_error_m": max_seam_error,
    }
