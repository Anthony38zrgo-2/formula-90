"""Load and validate BuildIR for the Blender materialization backend.

This module is pure Python and must not import ``bpy``. It parses the
``track.build.json`` produced by the Rust ``track-build`` compiler and applies
the same materialization-readiness gate as the Rust side, so a backend never
receives an IR that is not self-contained and explicit.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class RoadSample:
    station: float
    position: tuple[float, float]
    tangent: tuple[float, float]
    normal: tuple[float, float]
    width_left: float
    width_right: float
    elevation: float
    bank: float


@dataclass(frozen=True)
class ExplicitAssetInstance:
    instance_id: str
    asset_id: str
    position: tuple[float, float]
    yaw_rad: float
    scale: float
    kind: str | None = None
    source: str | None = None


@dataclass(frozen=True)
class TerrainCell:
    x: float
    z: float
    height: float


@dataclass(frozen=True)
class VegetationInstance:
    instance_id: str
    asset_id: str
    position: tuple[float, float]
    yaw_rad: float
    scale: float


@dataclass
class BuildIR:
    build_ir_version: int
    compiler_version: str
    source_project_hash: str
    road_samples: list[RoadSample] = field(default_factory=list)
    asset_instances: list[ExplicitAssetInstance] = field(default_factory=list)
    terrain_cell_m: float = 0.0
    terrain_heightfield: list[TerrainCell] = field(default_factory=list)
    vegetation_instances: list[VegetationInstance] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "BuildIR":
        return cls(
            build_ir_version=int(data.get("build_ir_version", 0)),
            compiler_version=str(data.get("compiler_version", "")),
            source_project_hash=str(data.get("source_project_hash", "")),
            road_samples=[_road_sample(item) for item in data.get("road_samples", [])],
            asset_instances=[_asset(item) for item in data.get("asset_instances", [])],
            terrain_cell_m=float(data.get("terrain_cell_m", 0.0)),
            terrain_heightfield=[_terrain_cell(item) for item in data.get("terrain_heightfield", [])],
            vegetation_instances=[
                _vegetation(item) for item in data.get("vegetation_instances", [])
            ],
        )


def _terrain_cell(item: dict[str, Any]) -> TerrainCell:
    return TerrainCell(
        x=float(item["x"]),
        z=float(item["z"]),
        height=float(item["height"]),
    )


def _vegetation(item: dict[str, Any]) -> VegetationInstance:
    return VegetationInstance(
        instance_id=str(item["instance_id"]),
        asset_id=str(item["asset_id"]),
        position=(float(item["position"][0]), float(item["position"][1])),
        yaw_rad=float(item.get("yaw_rad", 0.0)),
        scale=float(item.get("scale", 1.0)),
    )


def _road_sample(item: dict[str, Any]) -> RoadSample:
    return RoadSample(
        station=float(item["station"]),
        position=(float(item["position"][0]), float(item["position"][1])),
        tangent=(float(item["tangent"][0]), float(item["tangent"][1])),
        normal=(float(item["normal"][0]), float(item["normal"][1])),
        width_left=float(item["width_left"]),
        width_right=float(item["width_right"]),
        elevation=float(item["elevation"]),
        bank=float(item["bank"]),
    )


def _asset(item: dict[str, Any]) -> ExplicitAssetInstance:
    return ExplicitAssetInstance(
        instance_id=str(item["instance_id"]),
        asset_id=str(item["asset_id"]),
        position=(float(item["position"][0]), float(item["position"][1])),
        yaw_rad=float(item.get("yaw_rad", 0.0)),
        scale=float(item.get("scale", 1.0)),
        kind=item.get("kind"),
        source=item.get("source"),
    )


def load_build_ir(path: str | Path) -> BuildIR:
    """Load ``track.build.json`` from disk."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    return BuildIR.from_dict(data)


def materialization_prerequisites(build_ir: BuildIR) -> list[str]:
    """List of materialization prerequisites that are missing."""
    missing: list[str] = []
    if not build_ir.road_samples:
        missing.append("road_samples are empty")
    for index, sample in enumerate(build_ir.road_samples):
        for name, values in (
            ("position", sample.position),
            ("tangent", sample.tangent),
            ("normal", sample.normal),
        ):
            if any(not _isfinite(v) for v in values):
                missing.append(f"road sample {index} has non-finite {name}")
        if sample.width_left <= 0.0 or sample.width_right <= 0.0:
            missing.append(f"road sample {index} has non-positive half width")
    return missing


def materialization_ready(build_ir: BuildIR) -> bool:
    return not materialization_prerequisites(build_ir)


def _isfinite(value: float) -> bool:
    import math

    return math.isfinite(value)
