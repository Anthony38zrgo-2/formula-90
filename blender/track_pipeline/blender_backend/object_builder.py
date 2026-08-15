"""Object materialization from BuildIR explicit asset instances.

The pure function ``object_card_geometry`` computes a billboard card at the
explicit position, yaw and scale of an asset instance and is unit-testable
without Blender. ``build_object_in_blender`` creates the object in Blender from
that geometry (lazy ``bpy`` import).
"""

from __future__ import annotations

import math
from typing import Any

from .build_ir_loader import ExplicitAssetInstance


def object_card_geometry(
    instance: ExplicitAssetInstance,
    width: float = 0.8,
    height: float = 1.8,
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int, int]]]:
    """Return ``(vertices, faces)`` of a single-face card.

    The card is a billboard rectangle centered on the instance position, rotated
    by ``yaw_rad`` in the X/Z plane and scaled uniformly.
    """
    half = width * 0.5
    corners = [(-half, 0.0), (half, 0.0), (half, height), (-half, height)]
    yaw = instance.yaw_rad
    cos, sin = math.cos(yaw), math.sin(yaw)
    px, pz = instance.position
    scale = instance.scale

    vertices: list[tuple[float, float, float]] = []
    for local_x, local_z in corners:
        x = px + scale * (cos * local_x - sin * local_z)
        z = pz + scale * (sin * local_x + cos * local_z)
        vertices.append((x, 0.0, z))
    return vertices, [(0, 1, 2, 3)]


def build_object_in_blender(instance: ExplicitAssetInstance, material: Any) -> Any:
    """Create an object mesh in Blender from an explicit asset instance."""
    import bpy

    vertices, faces = object_card_geometry(instance)
    mesh = bpy.data.meshes.new(f"ObjectMesh_{instance.instance_id}")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    if material is not None:
        mesh.materials.append(material)
    obj = bpy.data.objects.new(f"Asset_{instance.instance_id}_{instance.asset_id}", mesh)
    scene = bpy.context.scene
    if scene is None and bpy.data.scenes:
        scene = bpy.data.scenes[0]
    if scene is not None:
        scene.collection.objects.link(obj)
    return obj
