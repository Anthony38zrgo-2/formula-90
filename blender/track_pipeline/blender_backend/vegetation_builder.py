"""Vegetation materialization from BuildIR explicit vegetation instances.

The Blender side reuses the object card geometry. The pure geometry helper is
exposed here so unit tests can validate vegetation placement independently.
"""

from __future__ import annotations

from typing import Any

from .build_ir_loader import ExplicitAssetInstance, VegetationInstance
from .object_builder import build_object_in_blender, object_card_geometry


def vegetation_geometry(
    instance: VegetationInstance,
    width: float = 0.8,
    height: float = 1.8,
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int, int]]]:
    """Return the card geometry for a vegetation instance."""
    explicit = ExplicitAssetInstance(
        instance_id=instance.instance_id,
        asset_id=instance.asset_id,
        position=instance.position,
        yaw_rad=instance.yaw_rad,
        scale=instance.scale,
    )
    return object_card_geometry(explicit, width=width, height=height)


def build_vegetation_in_blender(instance: VegetationInstance, material: Any) -> Any:
    """Create a vegetation card in Blender from an explicit instance."""
    explicit = ExplicitAssetInstance(
        instance_id=instance.instance_id,
        asset_id=instance.asset_id,
        position=instance.position,
        yaw_rad=instance.yaw_rad,
        scale=instance.scale,
    )
    return build_object_in_blender(explicit, material)
