"""Region operations on a canonical F90 Track SVG (no Blender).

Applies a single deterministic operation to a ``vegetation-region`` group in an
already-sanitized canonical SVG: move / scale / extend. Existing instance
positions and IDs are preserved; extension samples only the newly added area.
Returns a new canonical ``ElementTree`` so the authoring server can re-validate
and persist the result.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any
from xml.etree import ElementTree as ET

from svg_profile import (
    VEGETATION_BOUNDARY_ROLE,
    VEGETATION_CATEGORY_ATTR,
    VEGETATION_GENERATED_ATTR,
    VEGETATION_REGION_ROLE,
)
from vegetation_regions import (
    Region,
    sample_new_placements,
    scale_boundary,
    translate_boundary,
)

SVG_NS = "http://www.w3.org/2000/svg"
ASSET_ROLE = "asset-instance"


class RegionOperationError(ValueError):
    """Raised when a region operation cannot be applied safely."""


def _local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _find_region(root: ET.Element, region_id: str) -> ET.Element:
    for element in root.iter():
        if element.get("data-role") == VEGETATION_REGION_ROLE and element.get("data-region-id") == region_id:
            return element
    raise RegionOperationError(f"region not found: {region_id!r}")


def _boundary_of(region: ET.Element) -> list[list[float]]:
    for element in region:
        if element.get("data-role") == VEGETATION_BOUNDARY_ROLE:
            points = element.get("points", "")
            nums = [float(v) for v in points.replace(",", " ").split()]
            return [[nums[i], nums[i + 1]] for i in range(0, len(nums), 2)]
    raise RegionOperationError(f"region {region.get('data-region-id')!r} has no boundary")


def _set_boundary(region: ET.Element, boundary: list[list[float]]) -> None:
    for element in region:
        if element.get("data-role") == VEGETATION_BOUNDARY_ROLE:
            element.set(
                "points",
                " ".join(f"{v[0]:.4f},{v[1]:.4f}" for v in boundary),
            )
            return
    raise RegionOperationError(f"region {region.get('data-region-id')!r} has no boundary")


def _region_members(region: ET.Element) -> list[ET.Element]:
    return [element for element in region if element.get("data-role") == ASSET_ROLE]


def move_region(root: ET.Element, region_id: str, dx: float, dz: float) -> ET.Element:
    region = _find_region(root, region_id)
    boundary = _boundary_of(region)
    new_boundary = translate_boundary(boundary, float(dx), float(dz))
    for member in _region_members(region):
        member.set("cx", f"{float(member.get('cx', 0)) + float(dx):.4f}")
        member.set("cy", f"{float(member.get('cy', 0)) + float(dz):.4f}")
    _set_boundary(region, new_boundary)
    return root


def scale_region(root: ET.Element, region_id: str, factor: float, anchor: list[float] | None = None) -> ET.Element:
    factor = float(factor)
    if factor <= 0:
        raise RegionOperationError("scale factor must be positive")
    region = _find_region(root, region_id)
    boundary = _boundary_of(region)
    if anchor is None:
        xs = [p[0] for p in boundary]
        zs = [p[1] for p in boundary]
        anchor = [(min(xs) + max(xs)) * 0.5, (min(zs) + max(zs)) * 0.5]
    new_boundary = scale_boundary(boundary, factor, anchor)
    ax, az = float(anchor[0]), float(anchor[1])
    for member in _region_members(region):
        cx = ax + (float(member.get("cx", 0)) - ax) * factor
        cz = az + (float(member.get("cy", 0)) - az) * factor
        member.set("cx", f"{cx:.4f}")
        member.set("cy", f"{cz:.4f}")
        scale = float(member.get("data-scale", 1.0)) * factor
        member.set("data-scale", f"{scale:.4f}")
    _set_boundary(region, new_boundary)
    return root


def extend_region(
    root: ET.Element,
    region_id: str,
    new_boundary: list[list[float]],
    *,
    asset_pool: list[str],
    centerline_points: list[list[float]] | None = None,
    road_half_width_m: float = 6.0,
    max_count: int = 400,
) -> ET.Element:
    """Extend a region to a new boundary, generating placements in the added area."""
    region = _find_region(root, region_id)
    old_boundary = _boundary_of(region)
    if len(new_boundary) < 3:
        raise RegionOperationError("new boundary needs at least 3 points")

    existing = []
    for member in _region_members(region):
        existing.append(
            {
                "instance_id": member.get("data-instance-id"),
                "position_xz": [float(member.get("cx", 0)), float(member.get("cy", 0))],
                "asset_path": member.get("data-asset-id"),
                "scale": float(member.get("data-scale", 1.0)),
                "yaw_rad": float(member.get("data-yaw-rad", 0.0)),
            }
        )

    region_spec = Region(
        region_id=region_id,
        category=region.get(VEGETATION_CATEGORY_ATTR, "vegetation"),
        boundary_xz=new_boundary,
        seed=int(region.get("data-seed", "0")),
        spacing_m=float(region.get("data-spacing-m", "4.0")),
        target_count=len(existing),
    )

    placements = sample_new_placements(
        new_polygon=new_boundary,
        old_polygon=old_boundary,
        existing=existing,
        region=region_spec,
        asset_pool=asset_pool,
        seed=region_spec.seed,
        spacing_m=region_spec.spacing_m,
        min_separation_m=1.0,
        centerline_points=centerline_points,
        road_half_width_m=road_half_width_m,
        max_count=max_count,
    )

    index = 0
    for placement in placements:
        while any(
            m.get("data-instance-id") == f"{region_id}_{index:06d}"
            for m in region.iter()
        ):
            index += 1
        el = ET.SubElement(region, f"{{{SVG_NS}}}circle")
        el.set("data-role", ASSET_ROLE)
        el.set("data-instance-id", f"{region_id}_{index:06d}")
        el.set("data-asset-id", Path(placement["asset_path"]).stem)
        el.set("data-kind", "vegetation")
        el.set("cx", f"{placement['position_xz'][0]:.4f}")
        el.set("cy", f"{placement['position_xz'][1]:.4f}")
        el.set("r", "0.4")
        el.set("data-scale", f"{placement['scale']:.4f}")
        el.set("data-yaw-rad", f"{placement['yaw_rad']:.4f}")
        el.set(VEGETATION_GENERATED_ATTR, region_id)
        el.set(VEGETATION_CATEGORY_ATTR, region.get(VEGETATION_CATEGORY_ATTR, "vegetation"))
        index += 1

    _set_boundary(region, new_boundary)
    region.set("data-target-count", str(len(existing) + len(placements)))
    return root


def apply_region_operation(
    root: ET.Element,
    operation: str,
    region_id: str,
    params: dict[str, Any],
) -> ET.Element:
    """Dispatch a region operation on an in-memory canonical SVG tree."""
    if operation == "move":
        return move_region(root, region_id, float(params["dx"]), float(params["dz"]))
    if operation == "scale":
        return scale_region(root, region_id, float(params["factor"]), params.get("anchor"))
    if operation == "extend":
        return extend_region(
            root,
            region_id,
            [list(map(float, p)) for p in params["new_boundary"]],
            asset_pool=[str(a) for a in params.get("asset_pool", [])],
            centerline_points=params.get("centerline_points"),
            road_half_width_m=float(params.get("road_half_width_m", 6.0)),
            max_count=int(params.get("max_count", 400)),
        )
    raise RegionOperationError(f"unsupported region operation: {operation!r}")
