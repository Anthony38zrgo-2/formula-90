"""Deterministic La Chutana -> F90 canonical SVG importer (Stage 5 migration).

Consumes the compiled semantic authority
(``blender/generated/la_chutana/semantic_layout/compiled_layout.json``), the
sampled centerline, the object catalog and the layout config, and renders an
F90-profile canonical source SVG (``la_chutana.source.svg``) containing:

* a closed centerline polyline (SVG x -> world X, SVG y -> world Z, identity
  axis mapping per ``svg_normalizer``);
* the road element with ``data-width-m`` from the layout bootstrap;
* flat banking controls (degrees = 0) and a flat elevation control distributed
  along centerline arc-length ``s``;
* a grass ``terrain-zone`` polygon covering ``world_bounds_xz``;
* guardrail barrier segments centred on each compiled object's barrier
  reference point and oriented along that object's barrier tangent;
* every compiled object and vegetation item as an ``asset-instance`` circle
  with stable ``data-instance-id`` values reused from the compiled layout and
  ``data-asset-id`` values resolved from the registry.

After rendering, the importer compiles the source through
``compile_svg_track.compile_track`` against the real Asset Registry, verifies
category/asset parity against the Stage 5 acceptance targets and reports the
centerline length and the terrain-grid collision validation.

Determinism contract: identical inputs produce byte-identical SVG, canonical
SVG, normalized JSON and manifest. No RNG, no timestamps and fixed-precision
number formatting.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import cv2
import numpy as np

from asset_registry import load_registry
from compile_svg_track import (
    CANONICAL_ARTIFACT,
    MANIFEST_ARTIFACT,
    NORMALIZED_ARTIFACT,
    CompileError,
    compile_track,
)
from svg_profile import (
    VEGETATION_BOUNDARY_ROLE,
    VEGETATION_CATEGORY_ATTR,
    VEGETATION_GENERATED_ATTR,
    VEGETATION_REGION_ROLE,
)
from semantic_layout_common import WorldRasterTransform
from vegetation_regions import (
    assign_instances_to_regions,
    derive_regions,
    region_group_element,
)

_REPO_ROOT = Path(__file__).resolve().parents[2]

SOURCE_FILENAME = "la_chutana.source.svg"
COMPILED_SUBDIR = "compiled"

ROAD_SURFACE_ELEVATION_M = 0.025
ASSET_PLACEHOLDER_RADIUS = 0.4
BARRIER_HALF_LENGTH_M = 4.0
BANKING_CONTROL_COUNT = 4
BANKING_DEGREES = 0.0
OBJECT_SCALE = 1.0

# Stage 5 acceptance parity targets; mirrors validate_semantic_layout.py.
OBJECT_CATEGORY_TARGET = {
    "spectator": 42,
    "marshal": 17,
    "photographer": 13,
    "sign": 19,
    "flag": 33,
}
VEGETATION_CATEGORY_TARGET = {"bushes": 110, "grass": 9320, "trees": 130}

_KIND_MAP = {"card": "card", "procedural_flag": "flag"}

# Zone names in the semantic layout per vegetation category (used to derive the
# palette colour, spacing and seed from the catalog/config).
_VEGETATION_ZONE = {"grass": "grass", "bushes": "bushes", "trees": "trees"}


def _points_attr(points: list) -> str:
    return " ".join(f"{_num(v)},{_num(w)}" for v, w in points)


def _boundary_element(region_id: str, category: str, boundary_xz: list) -> str:
    return (
        f'<polygon data-role="{VEGETATION_BOUNDARY_ROLE}" '
        f'data-region-id="{region_id}" points="{_points_attr(boundary_xz)}"/>'
    )


class ImportFailure(ValueError):
    """Raised when the compiled authority cannot be represented as an SVG."""


def _num(value: float, digits: int = 4) -> str:
    return f"{float(value):.{digits}f}"


def _centerline_d(centerline: dict) -> str:
    """Render the sampled centerline as one closed SVG path (``... L first Z``)."""
    points = centerline["points_xz"]
    if len(points) < 3:
        raise ImportFailure(f"centerline needs at least 3 points, got {len(points)}")
    tokens = ["M", _num(points[0][0]), _num(points[0][1])]
    for x, z in points[1:]:
        tokens += ["L", _num(x), _num(z)]
    tokens += ["L", _num(points[0][0]), _num(points[0][1]), "Z"]
    return " ".join(tokens)


def _banking_controls(centerline_length_m: float) -> list[str]:
    """Distribute flat banking controls along the centreline (all degrees = 0)."""
    length = float(centerline_length_m)
    controls = []
    for index in range(BANKING_CONTROL_COUNT):
        s_m = round(length * index / BANKING_CONTROL_COUNT, 1)
        controls.append(
            f'<path data-role="banking" data-s-m="{_num(s_m, 1)}" '
            f'data-degrees="{_num(BANKING_DEGREES, 1)}"/>'
        )
    return controls


def _terrain_zone(world_bounds_xz: list) -> str:
    min_x, min_z, max_x, max_z = (float(v) for v in world_bounds_xz)
    points = " ".join(
        f"{_num(v)},{_num(w)}"
        for v, w in ((min_x, min_z), (max_x, min_z), (max_x, max_z), (min_x, max_z))
    )
    return (
        f'<polygon data-role="terrain-zone" data-kind="grass" data-name="infiel" '
        f'points="{points}"/>'
    )


def _barrier_elements(objects: list[dict]) -> list[str]:
    """Guardrail segments centred on compiled barrier reference points.

    Each object's ``yaw_rad`` is the barrier tangent direction
    ``atan2(tangent_z, tangent_x)`` in world (x, z), so a segment of length
    ``2 * BARRIER_HALF_LENGTH_M`` is oriented along the real guardrail line.
    """
    ordered = sorted(objects, key=lambda obj: (obj.get("track_fraction", 0.0), obj["instance_id"]))
    elements = []
    for obj in ordered:
        reference = obj.get("barrier_position_xz")
        if not reference:
            continue
        yaw = float(obj.get("yaw_rad", 0.0))
        dx, dy = math.cos(yaw), math.sin(yaw)
        x1 = float(reference[0]) - BARRIER_HALF_LENGTH_M * dx
        y1 = float(reference[1]) - BARRIER_HALF_LENGTH_M * dy
        x2 = float(reference[0]) + BARRIER_HALF_LENGTH_M * dx
        y2 = float(reference[1]) + BARRIER_HALF_LENGTH_M * dy
        elements.append(
            f'<line data-role="barrier" data-kind="guardrail" '
            f'x1="{_num(x1)}" y1="{_num(y1)}" x2="{_num(x2)}" y2="{_num(y2)}"/>'
        )
    if not elements:
        raise ImportFailure("no barrier segments could be derived from compiled objects")
    return elements


def _data_kind(raw_kind: str) -> str:
    return _KIND_MAP.get(raw_kind, raw_kind or "card")


def _object_instances(objects: list[dict], catalog: dict) -> list[str]:
    catalog_objects = catalog.get("objects", {})
    elements = []
    for obj in sorted(objects, key=lambda item: item["instance_id"]):
        spec = catalog_objects.get(obj["catalog_index"])
        if spec is not None and spec.get("asset_id") != obj["asset_id"]:
            raise ImportFailure(
                f"object {obj['instance_id']!r}: catalog asset_id {spec.get('asset_id')!r} "
                f"!= compiled asset_id {obj['asset_id']!r}"
            )
        kind = _data_kind((spec or {}).get("kind") or obj.get("kind"))
        cx, cz = (float(v) for v in obj["position_xz"])
        elements.append(
            f'<circle data-role="asset-instance" data-instance-id="{obj["instance_id"]}" '
            f'data-asset-id="{obj["asset_id"]}" data-kind="{kind}" '
            f'cx="{_num(cx)}" cy="{_num(cz)}" r="{_num(ASSET_PLACEHOLDER_RADIUS)}" '
            f'data-scale="{_num(OBJECT_SCALE)}" data-yaw-rad="{_num(obj["yaw_rad"], 7)}"/>'
        )
    return elements


def _region_groups(
    regions: list[dict],
    vegetation: list[dict],
) -> list[str]:
    """Emit ``vegetation-region`` groups, each with boundary + member instances.

    Instances are grouped by their assigned region id and sorted for determinism.
    Instances without a region are emitted as plain instances (caller handles).
    """
    by_region: dict[str, list[dict]] = {}
    for item in vegetation:
        by_region.setdefault(item.get("region_id"), []).append(item)

    groups = []
    for region in sorted(regions, key=lambda r: r["region_id"]):
        members = sorted(by_region.get(region["region_id"], []), key=lambda m: m["instance_id"])
        if not members:
            continue
        group = (
            f'<g data-role="{VEGETATION_REGION_ROLE}" data-region-id="{region["region_id"]}" '
            f'{VEGETATION_CATEGORY_ATTR}="{region["category"]}" '
            f'data-seed="{region["seed"]}" data-spacing-m="{_num(region["spacing_m"])}" '
            f'data-target-count="{len(members)}">'
        )
        group += "\n" + _boundary_element(region["region_id"], region["category"], region["boundary_xz"])
        for member in members:
            asset_id = Path(member["asset_path"]).stem
            cx, cz = (float(v) for v in member["position_xz"])
            group += (
                "\n"
                + f'<circle data-role="asset-instance" data-instance-id="{member["instance_id"]}" '
                f'data-asset-id="{asset_id}" data-kind="vegetation" '
                f'cx="{_num(cx)}" cy="{_num(cz)}" r="{_num(ASSET_PLACEHOLDER_RADIUS)}" '
                f'data-scale="{_num(member["scale"], 7)}" data-yaw-rad="{_num(member["yaw_rad"], 7)}" '
                f'{VEGETATION_GENERATED_ATTR}="{region["region_id"]}" '
                f'{VEGETATION_CATEGORY_ATTR}="{region["category"]}"/>'
            )
        group += "\n</g>"
        groups.append(group)
    return groups


def _loose_vegetation_instances(vegetation: list[dict]) -> list[str]:
    """Emit vegetation instances that belong to no region (no semantic image)."""
    elements = []
    for item in sorted(vegetation, key=lambda veg: veg["instance_id"]):
        asset_id = Path(item["asset_path"]).stem
        cx, cz = (float(v) for v in item["position_xz"])
        elements.append(
            f'<circle data-role="asset-instance" data-instance-id="{item["instance_id"]}" '
            f'data-asset-id="{asset_id}" data-kind="vegetation" '
            f'cx="{_num(cx)}" cy="{_num(cz)}" r="{_num(ASSET_PLACEHOLDER_RADIUS)}" '
            f'data-scale="{_num(item["scale"], 7)}" data-yaw-rad="{_num(item["yaw_rad"], 7)}"/>'
        )
    return elements


def render_source_svg(compiled: dict, centerline: dict, catalog: dict, config: dict) -> str:
    """Render the compiled authority as a canonical F90 source SVG (deterministic)."""
    track_id = config.get("track_id") or compiled.get("track_id")
    if not track_id:
        raise ImportFailure("missing track_id in layout config / compiled layout")
    world_bounds = compiled.get("world_bounds_xz") or config.get("world_bounds_xz")
    if not world_bounds or len(world_bounds) != 4:
        raise ImportFailure("missing world_bounds_xz in compiled layout / layout config")
    min_x, min_z, max_x, max_z = (float(v) for v in world_bounds)
    view_box = f"{_num(min_x)} {_num(min_z)} {_num(max_x - min_x)} {_num(max_z - min_z)}"
    road_width = float(config.get("bootstrap", {}).get("road_width_m", 12.0))
    centerline_length = float(centerline.get("length_m", 0.0) or 0.0)

    vegetation = compiled["vegetation"]

    lines = ['<?xml version="1.0" encoding="UTF-8"?>']
    lines.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" '
        f'data-track-id="{track_id}" '
        f'data-road-surface-elevation-m="{_num(ROAD_SURFACE_ELEVATION_M, 3)}">'
    )
    lines.append('  <g data-layer="centerline">')
    lines.append(f'    <path data-role="centerline" d="{_centerline_d(centerline)}"/>')
    lines.append("  </g>")
    lines.append('  <g data-layer="road">')
    lines.append(f'    <path data-role="road" data-width-m="{_num(road_width)}"/>')
    lines.extend("    " + control for control in _banking_controls(centerline_length))
    lines.append('    <path data-role="elevation" data-s-m="0.0" data-height-m="0.0"/>')
    lines.append("  </g>")
    lines.append('  <g data-layer="terrain">')
    lines.append("    " + _terrain_zone(world_bounds))
    lines.append("  </g>")
    lines.append('  <g data-layer="barrier">')
    lines.extend("    " + barrier for barrier in _barrier_elements(compiled["objects"]))
    lines.append("  </g>")
    lines.append('  <g data-layer="assets">')
    lines.extend("    " + asset for asset in _object_instances(compiled["objects"], catalog))
    regions = _derive_vegetation_regions(compiled, catalog, config)
    if regions:
        for group in _region_groups(regions, vegetation):
            for subline in group.splitlines():
                lines.append("    " + subline)
    else:
        lines.extend("    " + asset for asset in _loose_vegetation_instances(vegetation))
    lines.append("  </g>")
    lines.append("</svg>")
    return "\n".join(lines) + "\n"


def _derive_vegetation_regions(compiled: dict, catalog: dict, config: dict) -> list[dict]:
    """Derive grass/bush/tree regions from the semantic masks and assign instances."""
    semantic_path = config.get("semantic_image")
    if not semantic_path:
        return []
    root = _REPO_ROOT
    image_path = root / semantic_path
    if not image_path.is_file():
        raise ImportFailure(f"semantic image missing: {image_path}")
    semantic = cv2.cvtColor(cv2.imread(str(image_path), cv2.IMREAD_COLOR), cv2.COLOR_BGR2RGB)
    transform = WorldRasterTransform.from_metadata(config)
    if semantic.shape[:2] != (transform.height, transform.width):
        raise ImportFailure(
            f"semantic image dimensions {semantic.shape[:2]} do not match config "
            f"{transform.width}x{transform.height}"
        )

    catalog_zones = catalog.get("zones", {})
    seed = int(config.get("seed", 0))
    all_regions: list[dict] = []
    for category, zone_name in _VEGETATION_ZONE.items():
        zone = catalog_zones.get(zone_name)
        if zone is None:
            continue
        palette = config.get("semantic_palette", {}).get(zone_name)
        if palette is None:
            continue
        spacing = float(zone.get("spacing_m", 4.0))
        target_count = int(zone.get("max_count", 0))
        regions, labels, label_of_region = derive_regions(
            semantic,
            tuple(palette),
            category,
            transform=transform,
            seed=seed,
            spacing_m=spacing,
            target_count=target_count,
            region_prefix=category,
            min_area_px=8,
        )
        assignment = assign_instances_to_regions(
            [item for item in compiled["vegetation"] if item["category"] == category],
            regions,
            category,
            transform=transform,
            labels=labels,
            label_of_region=label_of_region,
        )
        region_map = {r.region_id: r for r in regions}
        for item in compiled["vegetation"]:
            if item["category"] == category:
                item["region_id"] = assignment.get(item["instance_id"])
        for region in regions:
            all_regions.append(
                {
                    "region_id": region.region_id,
                    "category": region.category,
                    "boundary_xz": region.boundary_xz,
                    "seed": region.seed,
                    "spacing_m": region.spacing_m,
                    "target_count": region.target_count,
                }
            )
    return all_regions


def _category_counts(compiled: dict) -> tuple[dict[str, int], dict[str, int]]:
    object_counts: dict[str, int] = {}
    vegetation_counts: dict[str, int] = {}
    for obj in compiled["objects"]:
        object_counts[obj["category"]] = object_counts.get(obj["category"], 0) + 1
    for item in compiled["vegetation"]:
        vegetation_counts[item["category"]] = vegetation_counts.get(item["category"], 0) + 1
    return object_counts, vegetation_counts


def _compiled_asset_counts(compiled: dict) -> dict[str, int]:
    counts: dict[str, int] = {}
    for obj in compiled["objects"]:
        counts[obj["asset_id"]] = counts.get(obj["asset_id"], 0) + 1
    for item in compiled["vegetation"]:
        asset_id = Path(item["asset_path"]).stem
        counts[asset_id] = counts.get(asset_id, 0) + 1
    return counts


def _normalized_asset_counts(normalized: dict) -> dict[str, int]:
    counts: dict[str, int] = {}
    for asset in normalized["assets"]:
        counts[asset["asset_id"]] = counts.get(asset["asset_id"], 0) + 1
    return counts


def verify_parity(
    compiled: dict,
    normalized: dict,
    *,
    object_category_target: dict[str, int] = OBJECT_CATEGORY_TARGET,
    vegetation_category_target: dict[str, int] = VEGETATION_CATEGORY_TARGET,
) -> list[str]:
    """Return parity diagnostics; an empty list means full parity.

    Checks object/vegetation counts by category against the Stage 5 acceptance
    targets, that the compiled per-asset-id counts survive into the normalized
    SVG, and that every ``data-instance-id`` is unique.
    """
    diagnostics: list[str] = []
    object_counts, vegetation_counts = _category_counts(compiled)
    if object_counts != dict(object_category_target):
        diagnostics.append(
            f"object category counts changed: {object_counts} != {dict(object_category_target)}"
        )
    if vegetation_counts != dict(vegetation_category_target):
        diagnostics.append(
            f"vegetation category counts changed: {vegetation_counts} != {dict(vegetation_category_target)}"
        )
    compiled_assets = _compiled_asset_counts(compiled)
    normalized_assets = _normalized_asset_counts(normalized)
    if compiled_assets != normalized_assets:
        diagnostics.append(
            f"compiled asset-id counts do not match the generated SVG: {compiled_assets} != {normalized_assets}"
        )
    ids = [asset["instance_id"] for asset in normalized["assets"]]
    if len(ids) != len(set(ids)):
        diagnostics.append("duplicate data-instance-id in generated SVG")
    if normalized.get("vegetation_regions"):
        unassigned = [
            asset["instance_id"]
            for asset in normalized["assets"]
            if asset.get("region_id") is None and asset.get("category") in _VEGETATION_ZONE
        ]
        if unassigned:
            diagnostics.append(
                f"{len(unassigned)} vegetation instance(s) have no region "
                f"(e.g. {unassigned[:3]})"
            )
    return diagnostics


def _collision_flags(normalized: dict) -> dict[str, object]:
    validation = normalized.get("collision_validation", {})
    return {
        "finite_vertices": validation.get("finite_vertices"),
        "nondegenerate_triangles": validation.get("nondegenerate_triangles"),
        "blender_winding_upward": validation.get("blender_winding_upward"),
        "continuous_collision_grid": validation.get("continuous_collision_grid"),
        "safety_floor_valid": validation.get("safety_floor_valid"),
        "max_collision_seam_error_m": validation.get("max_collision_seam_error_m"),
    }


def _load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise ImportFailure(f"cannot read JSON {path}: {exc}") from exc


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Import the La Chutana compiled layout into an F90 canonical SVG "
                    "and compile/verify it against the Asset Registry."
    )
    parser.add_argument("--compiled", default="blender/generated/la_chutana/semantic_layout/compiled_layout.json")
    parser.add_argument("--centerline", default="blender/generated/la_chutana/centerline.json")
    parser.add_argument("--catalog", default="blender/track_pipeline/layouts/la_chutana/object_catalog.json")
    parser.add_argument("--config", default="blender/track_pipeline/layouts/la_chutana/layout_config.json")
    parser.add_argument("--registry", default="blender/track_pipeline/configs/asset_registry.json")
    parser.add_argument("--output-dir", default="blender/generated/la_chutana/svg_source")
    parser.add_argument("--check", action="store_true", help="Fail with exit code 2 on parity mismatch.")
    args = parser.parse_args(argv)

    def resolve(path: str) -> Path:
        candidate = Path(path)
        return candidate if candidate.is_absolute() else (_REPO_ROOT / candidate)

    try:
        compiled = _load_json(resolve(args.compiled))
        centerline = _load_json(resolve(args.centerline))
        catalog = _load_json(resolve(args.catalog))
        config = _load_json(resolve(args.config))
        registry_path = resolve(args.registry)
        registry = load_registry(registry_path, repo_root=_REPO_ROOT)
    except (ImportFailure, OSError, ValueError) as exc:
        print(f"FAIL {exc}", file=sys.stderr)
        return 2

    source_text = render_source_svg(compiled, centerline, catalog, config)
    output_dir = resolve(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    source_path = output_dir / SOURCE_FILENAME
    source_path.write_text(source_text, encoding="utf-8")

    compile_dir = output_dir / COMPILED_SUBDIR
    try:
        manifest = compile_track(
            source_text.encode("utf-8"),
            source_name=SOURCE_FILENAME,
            registry=registry,
            registry_path=registry_path,
            output_dir=compile_dir,
        )
    except CompileError as exc:
        print("FAIL compile:", file=sys.stderr)
        for diagnostic in exc.diagnostics:
            print(f"- {diagnostic}", file=sys.stderr)
        return 2

    normalized = _load_json(compile_dir / NORMALIZED_ARTIFACT)
    centerline_data = normalized["centerline"]
    assets = normalized["assets"]
    objects = compiled["objects"]
    vegetation = compiled["vegetation"]
    object_counts, vegetation_counts = _category_counts(compiled)

    print(f"track: {manifest['track_id']}")
    print(f"source:    {source_path.resolve()}")
    print(f"canonical: {(compile_dir / CANONICAL_ARTIFACT).resolve()}")
    print(f"normalized: {(compile_dir / NORMALIZED_ARTIFACT).resolve()}")
    print(f"manifest:  {(compile_dir / MANIFEST_ARTIFACT).resolve()}")
    print(f"asset instances: {len(assets)} (objects {len(objects)}, vegetation {len(vegetation)})")
    print(f"objects by category: {object_counts}")
    print(f"vegetation by category: {vegetation_counts}")
    print(f"by asset_id: {_normalized_asset_counts(normalized)}")
    print(f"centerline: closed={centerline_data['closed']} length_m={centerline_data['length_m']} "
          f"point_count={centerline_data['point_count']}")
    flags = _collision_flags(normalized)
    collision_ok = all(flags[key] for key in ("finite_vertices", "nondegenerate_triangles",
                                              "blender_winding_upward", "continuous_collision_grid",
                                              "safety_floor_valid"))
    print(f"collision validation: ok={collision_ok} {flags}")

    parity = verify_parity(compiled, normalized)
    if parity:
        for diagnostic in parity:
            print(f"FAIL parity: {diagnostic}")
        return 2 if args.check else 0
    print("parity objects: PASS")
    print("parity vegetation: PASS")
    print("parity asset_ids: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
