"""Deterministic normalizer for the F90 Track SVG profile.

Compiles the canonical SVG into a normalized JSON contract plus a ``track_config``
that the existing ``terrain_grid`` collision code can validate directly, without
running Blender. Output is byte-deterministic for identical canonical input.
"""

from __future__ import annotations

from typing import Any
from xml.etree import ElementTree as ET
import json
import math

from svg_profile import (
    BANKING_RANGE_DEG,
    ELEVATION_HEIGHT_RANGE_M,
    INSTANCE_ID_RE,
    PROFILE_VERSION,
    SCHEMA_VERSION,
    SVG_NS,
    VEGETATION_BOUNDARY_ROLE,
    VEGETATION_CATEGORY_ATTR,
    VEGETATION_GENERATED_ATTR,
    VEGETATION_REGION_ROLE,
    local_name,
)
from terrain_grid import validate_heightfield

SAMPLE_SPACING_M = 2.0
CURVE_FLATTEN_TOLERANCE_M = 0.1
HALF_WIDTH_FRACTION = 0.03

DEFAULT_TERRAIN = {
    "grid_cell_m": 6.0,
    "shoulder_falloff_m": 18.0,
    "collision_underlay_drop_m": 0.12,
    "collision_underlay_blend_m": 1.0,
    "visual_under_road_drop_m": 0.22,
    "visual_under_road_blend_m": 1.4,
    "visual_edge_blend_m": 1.25,
    "visual_sink_m": 0.003,
    "far_ground_z_m": -0.1,
    "far_ground_safety_m": 0.05,
    "far_ground_margin_m": 220.0,
    "safety_floor_z_m": -6.0,
    "safety_floor_thickness_m": 0.6,
}


class NormalizeError(ValueError):
    pass


# ---------------------------------------------------------------------------
# Path parsing / flattening (deterministic; never browser tessellation)
# ---------------------------------------------------------------------------

def _tokenize_path(d: str) -> list[str]:
    tokens: list[str] = []
    number = ""
    for char in d.replace("\n", " ").replace(",", " "):
        if char in "MmLlHhVvCcQqZz":
            if number.strip():
                tokens.extend(number.strip().split())
                number = ""
            tokens.append(char)
        else:
            number += char
    if number.strip():
        tokens.extend(number.strip().split())
    return tokens


# Defensive arity check: canonical SVG is sanitized, but flattening must never
# crash with an IndexError on hand-edited or corrupted input.
_PATH_COMMAND_PARAMS = {
    "M": 2, "m": 2, "L": 2, "l": 2,
    "H": 1, "h": 1, "V": 1, "v": 1,
    "C": 6, "c": 6, "Q": 4, "q": 4,
    "Z": 0, "z": 0,
}


def _validate_path_arity(tokens: list[str]) -> None:
    if not tokens or tokens[0][0] not in "Mm":
        raise NormalizeError("path must start with an M/m moveto command")
    command: str | None = None
    count = 0
    for token in tokens:
        if token[0] in "MmLlHhVvCcQqZz":
            if command is not None:
                _check_params(command, count)
            command = token[0]
            count = 0
        else:
            count += 1
    if command is not None:
        _check_params(command, count)


def _check_params(command: str, count: int) -> None:
    step = _PATH_COMMAND_PARAMS[command]
    if step == 0:
        if count:
            raise NormalizeError(f"path command {command!r} must not take parameters (got {count})")
        return
    if count == 0:
        raise NormalizeError(f"path command {command!r} is missing its parameters")
    if count % step:
        raise NormalizeError(f"path command {command!r} has {count} parameters; expected a multiple of {step}")


def _flatten_cubic(p0, p1, p2, p3, tolerance: float, out: list[tuple[float, float]]) -> None:
    # Recursive subdivision until the chord deviation is below the flatness tolerance.
    def flat(p0, p1, p2, p3):
        x = p1[0] - p0[0]
        y = p1[1] - p0[1]
        distance2 = (p2[0] - (3.0 * p1[0] - 2.0 * p0[0])) ** 2 + (p2[1] - (3.0 * p1[1] - 2.0 * p0[1])) ** 2
        distance = math.sqrt(distance2)
        return distance <= tolerance and (x * x + y * y) <= tolerance * tolerance

    if flat(p0, p1, p2, p3):
        out.append((p3[0], p3[1]))
        return
    x01 = (p0[0] + p1[0]) * 0.5
    y01 = (p0[1] + p1[1]) * 0.5
    x12 = (p1[0] + p2[0]) * 0.5
    y12 = (p1[1] + p2[1]) * 0.5
    x23 = (p2[0] + p3[0]) * 0.5
    y23 = (p2[1] + p3[1]) * 0.5
    x012 = (x01 + x12) * 0.5
    y012 = (y01 + y12) * 0.5
    x123 = (x12 + x23) * 0.5
    y123 = (y12 + y23) * 0.5
    x0123 = (x012 + x123) * 0.5
    y0123 = (y012 + y123) * 0.5
    _flatten_cubic((p0[0], p0[1]), (x01, y01), (x012, y012), (x0123, y0123), tolerance, out)
    _flatten_cubic((x0123, y0123), (x123, y123), (x23, y23), (p3[0], p3[1]), tolerance, out)


def flatten_path(d: str, tolerance: float = CURVE_FLATTEN_TOLERANCE_M) -> list[list[tuple[float, float]]]:
    """Flatten an SVG path into polylines (one per subpath)."""
    tokens = _tokenize_path(d)
    _validate_path_arity(tokens)
    subpaths: list[list[tuple[float, float]]] = []
    current: list[tuple[float, float]] = []
    cursor = (0.0, 0.0)
    subpath_start = (0.0, 0.0)
    index = 0

    def number(i: int) -> float:
        value = float(tokens[i])
        if not math.isfinite(value):
            raise NormalizeError(f"non-finite path number {tokens[i]!r}")
        return value

    while index < len(tokens):
        cmd = tokens[index][0]
        index += 1
        params = []
        while index < len(tokens) and tokens[index][0] not in "MmLlHhVvCcQqZz":
            params.append(number(index))
            index += 1

        if cmd in "Mm":
            for i in range(0, len(params), 2):
                x, y = params[i], params[i + 1]
                if cmd == "m":
                    x, y = cursor[0] + x, cursor[1] + y
                cursor = (x, y)
                subpath_start = cursor
                if current:
                    subpaths.append(current)
                current = [cursor]
        elif cmd in "Ll" and len(params) >= 2:
            for i in range(0, len(params), 2):
                x, y = params[i], params[i + 1]
                if cmd == "l":
                    x, y = cursor[0] + x, cursor[1] + y
                current.append((x, y))
                cursor = (x, y)
        elif cmd == "H":
            current.append((params[-1], cursor[1]))
            cursor = (params[-1], cursor[1])
        elif cmd == "h":
            current.append((cursor[0] + params[-1], cursor[1]))
            cursor = (cursor[0] + params[-1], cursor[1])
        elif cmd == "V":
            current.append((cursor[0], params[-1]))
            cursor = (cursor[0], params[-1])
        elif cmd == "v":
            current.append((cursor[0], cursor[1] + params[-1]))
            cursor = (cursor[0], cursor[1] + params[-1])
        elif cmd in "Cc":
            for i in range(0, len(params), 6):
                c1 = (params[i], params[i + 1])
                c2 = (params[i + 2], params[i + 3])
                end = (params[i + 4], params[i + 5])
                if cmd == "c":
                    c1 = (cursor[0] + c1[0], cursor[1] + c1[1])
                    c2 = (cursor[0] + c2[0], cursor[1] + c2[1])
                    end = (cursor[0] + end[0], cursor[1] + end[1])
                _flatten_cubic(cursor, c1, c2, end, tolerance, current)
                cursor = end
        elif cmd in "Qq":
            for i in range(0, len(params), 4):
                q = (params[i], params[i + 1])
                end = (params[i + 2], params[i + 3])
                if cmd == "q":
                    q = (cursor[0] + q[0], cursor[1] + q[1])
                    end = (cursor[0] + end[0], cursor[1] + end[1])
                c1 = (cursor[0] + 2.0 / 3.0 * (q[0] - cursor[0]), cursor[1] + 2.0 / 3.0 * (q[1] - cursor[1]))
                c2 = (end[0] + 2.0 / 3.0 * (q[0] - end[0]), end[1] + 2.0 / 3.0 * (q[1] - end[1]))
                _flatten_cubic(cursor, c1, c2, end, tolerance, current)
                cursor = end
        elif cmd in "Zz":
            if current and current[-1] != subpath_start:
                current.append(subpath_start)
            cursor = subpath_start
    if current:
        subpaths.append(current)
    return subpaths


def _polyline_length(points: list[tuple[float, float]]) -> float:
    length = 0.0
    for i in range(len(points) - 1):
        length += math.hypot(points[i + 1][0] - points[i][0], points[i + 1][1] - points[i][1])
    return length


def resample_closed(points: list[tuple[float, float]], spacing: float = SAMPLE_SPACING_M) -> list[tuple[float, float]]:
    """Resample a closed polyline at fixed arc-length spacing (deterministic)."""
    if len(points) < 3:
        raise NormalizeError("centerline needs at least 3 points")
    if points[0] != points[-1]:
        points = list(points) + [points[0]]

    cumulative = [0.0]
    for i in range(len(points) - 1):
        cumulative.append(cumulative[-1] + math.hypot(points[i + 1][0] - points[i][0], points[i + 1][1] - points[i][1]))
    total = cumulative[-1]
    if total <= 1e-9:
        raise NormalizeError("centerline has zero length")

    segments = len(points) - 1
    n = max(4, int(round(total / max(float(spacing), 0.1))))
    out: list[tuple[float, float]] = []
    for i in range(n):
        target = total * i / n
        seg = 0
        while seg < segments - 1 and cumulative[seg + 1] < target:
            seg += 1
        length0 = cumulative[seg]
        length1 = cumulative[seg + 1]
        t = 0.0 if length1 - length0 <= 1e-12 else (target - length0) / (length1 - length0)
        x = points[seg][0] + (points[seg + 1][0] - points[seg][0]) * t
        y = points[seg][1] + (points[seg + 1][1] - points[seg][1]) * t
        out.append((x, y))
    return out


def _proper_segment_intersect(a: tuple[float, float], b: tuple[float, float],
                              c: tuple[float, float], d: tuple[float, float]) -> bool:
    """True when segments ab and cd strictly cross (endpoint touch excluded)."""

    def ccw(p, q, r):
        return (q[1] - p[1]) * (r[0] - q[0]) - (q[0] - p[0]) * (r[1] - q[1])

    o1 = ccw(a, b, c)
    o2 = ccw(a, b, d)
    o3 = ccw(c, d, a)
    o4 = ccw(c, d, b)
    if o1 == 0.0 or o2 == 0.0 or o3 == 0.0 or o4 == 0.0:
        return False
    return (o1 > 0.0) != (o2 > 0.0) and (o3 > 0.0) != (o4 > 0.0)


def _point_on_segment(p: tuple[float, float], a: tuple[float, float],
                      b: tuple[float, float], tol: float = 1e-9) -> bool:
    cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    if abs(cross) > tol:
        return False
    length2 = (b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2
    if length2 <= tol:
        return math.hypot(p[0] - a[0], p[1] - a[1]) <= tol
    dot = (p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])
    return -tol <= dot <= length2 + tol


def closed_polyline_self_intersects(points: list[tuple[float, float]]) -> bool:
    """Detect strict crossings or touches between non-adjacent segments."""
    n = len(points)
    if n < 4:
        return False
    for i in range(n):
        a, b = points[i], points[(i + 1) % n]
        for j in range(i + 1, n):
            if j == i + 1 or (i == 0 and j == n - 1):
                continue
            c, d = points[j], points[(j + 1) % n]
            if _proper_segment_intersect(a, b, c, d):
                return True
            if _point_on_segment(a, c, d) or _point_on_segment(b, c, d):
                return True
            if _point_on_segment(c, a, b) or _point_on_segment(d, a, b):
                return True
    return False


# ---------------------------------------------------------------------------
# Extraction
# ---------------------------------------------------------------------------

def _children(root: ET.Element) -> list[ET.Element]:
    return list(root)


def _role(element: ET.Element) -> str | None:
    return element.get("data-role")


def _first(root: ET.Element, role: str, tag: str | None = None) -> ET.Element | None:
    for element in root.iter():
        if _role(element) == role:
            if tag is None or local_name(element.tag) == tag:
                return element
    return None


def _floats(element: ET.Element, name: str, default: float = 0.0) -> float:
    raw = element.get(name)
    if raw is None:
        return default
    context = _role(element) or local_name(element.tag)
    try:
        value = float(raw)
    except ValueError as exc:
        raise NormalizeError(f"{context}: attribute {name} is not numeric: {raw!r}") from exc
    if not math.isfinite(value):
        raise NormalizeError(f"{context}: attribute {name} must be finite, got {raw!r}")
    return value


def _collect_regions(root: ET.Element) -> dict[str, str]:
    """Map each generated vegetation instance id -> its region id.

    Reads the ``vegetation-region`` groups emitted by the importer. An instance
    referenced by ``data-generated-by-region`` that does not belong to a region
    is an error, because it would make the derived-placement contract ambiguous.
    """
    region_of: dict[str, str] = {}
    for group in root.iter():
        if group.get("data-role") != VEGETATION_REGION_ROLE:
            continue
        region_id = group.get("data-region-id")
        if not region_id:
            raise NormalizeError("vegetation-region: missing data-region-id")
        for element in group:
            if element.get("data-role") != "asset-instance":
                continue
            instance_id = element.get("data-instance-id")
            if instance_id:
                region_of[instance_id] = region_id
    return region_of


def _collect_vegetation_regions(root: ET.Element) -> list[dict[str, Any]]:
    regions: list[dict[str, Any]] = []
    for group in root.iter():
        if group.get("data-role") != VEGETATION_REGION_ROLE:
            continue
        boundary = None
        for element in group:
            if element.get("data-role") == VEGETATION_BOUNDARY_ROLE:
                boundary = element
                break
        if boundary is None:
            raise NormalizeError(
                f"vegetation-region {group.get('data-region-id')!r}: missing "
                f"data-role='{VEGETATION_BOUNDARY_ROLE}' polygon"
            )
        points_text = boundary.get("points", "")
        numbers = []
        for token in points_text.replace(",", " ").split():
            try:
                value = float(token)
            except ValueError as exc:
                raise NormalizeError(
                    f"vegetation-region {group.get('data-region-id')!r}: "
                    f"malformed boundary coordinate {token!r}"
                ) from exc
            if not math.isfinite(value):
                raise NormalizeError(
                    f"vegetation-region {group.get('data-region-id')!r}: "
                    f"non-finite boundary coordinate {token!r}"
                )
            numbers.append(value)
        if len(numbers) % 2 != 0:
            raise NormalizeError(
                f"vegetation-region {group.get('data-region-id')!r}: odd boundary points"
            )
        polygon = [[round(numbers[i], 6), round(numbers[i + 1], 6)] for i in range(0, len(numbers), 2)]
        if len(polygon) < 3:
            raise NormalizeError(
                f"vegetation-region {group.get('data-region-id')!r}: boundary needs >= 3 points"
            )
        regions.append(
            {
                "region_id": group.get("data-region-id"),
                "category": group.get(VEGETATION_CATEGORY_ATTR, "vegetation"),
                "seed": int(group.get("data-seed", "0")),
                "spacing_m": round(float(group.get("data-spacing-m", "0.0")), 4),
                "target_count": int(group.get("data-target-count", "0")),
                "boundary_xz": polygon,
            }
        )
    return regions


def normalize(root: ET.Element) -> dict[str, Any]:
    track_id = root.get("data-track-id", "untitled")
    viewbox = root.get("viewBox", "")
    try:
        vb = [float(v) for v in viewbox.replace(",", " ").split()] if viewbox else [0.0, 0.0, 0.0, 0.0]
    except ValueError as exc:
        raise NormalizeError(f"svg: viewBox not numeric: {viewbox!r}") from exc
    if not all(math.isfinite(v) for v in vb):
        raise NormalizeError(f"svg: viewBox must contain only finite numbers: {viewbox!r}")

    center_element = _first(root, "centerline", "path")
    if center_element is None:
        raise NormalizeError("missing required data-role='centerline' path")
    subpaths = flatten_path(center_element.get("d", ""))
    if not subpaths:
        raise NormalizeError("centerline path produced no geometry")
    if len(subpaths) != 1:
        raise NormalizeError("centerline must be a single closed subpath")
    raw_points = subpaths[0]
    if len(raw_points) < 3:
        raise NormalizeError("centerline needs at least 3 points")
    if raw_points[0] != raw_points[-1]:
        raise NormalizeError(
            "centerline must be closed: end the path with Z (or make its last point "
            "coincide with its first)"
        )
    points = resample_closed(raw_points, SAMPLE_SPACING_M)
    if closed_polyline_self_intersects(points):
        raise NormalizeError("centerline must not self-intersect")
    length_m = _polyline_length(points + [points[0]])
    points_xz = [[round(x, 6), round(z, 6)] for x, z in points]

    road_element = _first(root, "road")
    road_width = _floats(road_element, "data-width-m", 12.0) if road_element is not None else 12.0
    if road_width <= 0.0:
        raise NormalizeError(f"road width must be a positive number, got {road_width!r}")
    surface = _floats(root, "data-road-surface-elevation-m", 0.025)

    banking = []
    for element in root.iter():
        if _role(element) != "banking":
            continue
        s_m = _floats(element, "data-s-m")
        degrees = _floats(element, "data-degrees")
        if s_m < 0.0:
            raise NormalizeError(f"banking: data-s-m must be >= 0, got {s_m!r}")
        if not (BANKING_RANGE_DEG[0] <= degrees <= BANKING_RANGE_DEG[1]):
            raise NormalizeError(
                f"banking: data-degrees {degrees!r} outside allowed range {BANKING_RANGE_DEG}"
            )
        center_fraction = round((s_m % length_m) / length_m, 7)
        banking.append({
            "s_m": round(s_m, 4),
            "degrees": round(degrees, 5),
            "center_fraction": center_fraction,
            "half_width_fraction": HALF_WIDTH_FRACTION,
        })
    if not banking:
        raise NormalizeError("missing required banking controls (data-role='banking')")
    banking.sort(key=lambda item: item["center_fraction"])

    elevation = []
    for element in root.iter():
        if _role(element) != "elevation":
            continue
        s_m = _floats(element, "data-s-m")
        height_m = _floats(element, "data-height-m")
        if s_m < 0.0:
            raise NormalizeError(f"elevation: data-s-m must be >= 0, got {s_m!r}")
        if not (ELEVATION_HEIGHT_RANGE_M[0] <= height_m <= ELEVATION_HEIGHT_RANGE_M[1]):
            raise NormalizeError(
                f"elevation: data-height-m {height_m!r} outside allowed range {ELEVATION_HEIGHT_RANGE_M}"
            )
        elevation.append({"s_m": round(s_m, 4), "height_m": round(height_m, 5)})
    elevation.sort(key=lambda item: item["s_m"])

    terrain_zones = []
    for element in root.iter():
        if _role(element) != "terrain-zone":
            continue
        numbers = []
        for token in element.get("points", "").replace(",", " ").split():
            try:
                value = float(token)
            except ValueError as exc:
                raise NormalizeError(
                    f"terrain-zone {element.get('data-name', '?')}: malformed coordinate {token!r}"
                ) from exc
            if not math.isfinite(value):
                raise NormalizeError(
                    f"terrain-zone {element.get('data-name', '?')}: non-finite coordinate {token!r}"
                )
            numbers.append(value)
        if len(numbers) % 2 != 0:
            raise NormalizeError(f"terrain-zone {element.get('data-name', '?')}: odd number of points")
        polygon = [[round(numbers[i], 6), round(numbers[i + 1], 6)] for i in range(0, len(numbers), 2)]
        terrain_zones.append({
            "name": element.get("data-name", "zone"),
            "kind": element.get("data-kind", "grass"),
            "polygon_xz": polygon,
        })

    barriers = []
    for element in root.iter():
        if _role(element) != "barrier":
            continue
        for name in ("x1", "y1", "x2", "y2"):
            if element.get(name) is None:
                raise NormalizeError(f"barrier: missing required attribute {name}")
        start = [_floats(element, "x1"), _floats(element, "y1")]
        end = [_floats(element, "x2"), _floats(element, "y2")]
        barriers.append({
            "kind": element.get("data-kind", "guardrail"),
            "points_xz": [
                [round(start[0], 6), round(start[1], 6)],
                [round(end[0], 6), round(end[1], 6)],
            ],
        })
    if not barriers:
        raise NormalizeError("missing required barrier (data-role='barrier')")

    assets = []
    seen_ids: set[str] = set()
    region_of = _collect_regions(root)
    for element in root.iter():
        if _role(element) != "asset-instance":
            continue
        instance_id = element.get("data-instance-id")
        if not instance_id:
            raise NormalizeError("asset-instance: missing data-instance-id in canonical SVG")
        if not INSTANCE_ID_RE.match(instance_id):
            raise NormalizeError(f"asset-instance: invalid data-instance-id {instance_id!r}")
        if instance_id in seen_ids:
            raise NormalizeError(f"asset-instance: duplicate data-instance-id {instance_id!r}")
        seen_ids.add(instance_id)
        scale = _floats(element, "data-scale", 1.0)
        if scale <= 0.0:
            raise NormalizeError(f"asset {instance_id!r}: data-scale must be positive, got {scale!r}")
        generated_by = element.get(VEGETATION_GENERATED_ATTR)
        if generated_by is not None and instance_id not in region_of:
            raise NormalizeError(
                f"asset-instance {instance_id!r}: generated by region {generated_by!r} "
                "but no such vegetation-region group contains it"
            )
        category = element.get(VEGETATION_CATEGORY_ATTR) or element.get("data-kind", "")
        assets.append({
            "instance_id": instance_id,
            "asset_id": element.get("data-asset-id", ""),
            "kind": element.get("data-kind", ""),
            "category": category,
            "region_id": region_of.get(instance_id),
            "position_xz": [
                round(_floats(element, "cx", 0.0), 6),
                round(_floats(element, "cy", 0.0), 6),
            ],
            "scale": round(scale, 6),
            "yaw_rad": round(_floats(element, "data-yaw-rad", 0.0), 6),
            "collision": False,
        })
    if not assets:
        raise NormalizeError("missing required asset instance (data-role='asset-instance')")

    vegetation_regions = _collect_vegetation_regions(root)

    track_config: dict[str, Any] = {
        "track_id": track_id,
        "sample_spacing_m": SAMPLE_SPACING_M,
        "road": {
            "width_m": road_width,
            "surface_elevation_m": surface,
        },
        "terrain": dict(DEFAULT_TERRAIN),
        "banking": banking,
        "elevation": elevation,
        "centerline_length_m": round(length_m, 6),
    }

    heightfield = validate_heightfield(points_xz, track_config)

    normalized = {
        "schema_version": SCHEMA_VERSION,
        "profile_version": PROFILE_VERSION,
        "track_id": track_id,
        "coordinate_system": {
            "units": "metre",
            "svg_x_to_world_x": "identity",
            "svg_y_to_world_z": "identity",
            "view_box": vb,
        },
        "centerline": {
            "closed": True,
            "length_m": round(length_m, 6),
            "sample_spacing_m": SAMPLE_SPACING_M,
            "point_count": len(points_xz),
            "points_xz": points_xz,
        },
        "road": {"width_m": road_width, "surface_elevation_m": surface},
        "banking": banking,
        "elevation": elevation,
        "terrain_zones": terrain_zones,
        "barriers": barriers,
        "assets": assets,
        "vegetation_regions": vegetation_regions,
        "collision_validation": heightfield,
        "track_config": track_config,
    }
    return normalized


def normalized_json_bytes(root: ET.Element, *, indent: int | None = 2) -> bytes:
    return json.dumps(normalize(root), indent=indent, sort_keys=True, separators=(",", ":")).encode("utf-8")
