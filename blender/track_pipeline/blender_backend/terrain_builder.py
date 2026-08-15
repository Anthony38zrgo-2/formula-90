"""Terrain materialization from the BuildIR heightfield.

``terrain_grid`` derives a heightfield mesh (vertices + quads) from the explicit
terrain cells and is unit-testable without Blender. ``build_terrain_in_blender``
creates the mesh in Blender (lazy ``bpy`` import).
"""

from __future__ import annotations

from typing import Any

from .build_ir_loader import TerrainCell


def terrain_grid(
    cells: list[TerrainCell],
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int, int]]]:
    """Return ``(vertices, faces)`` of the terrain heightfield mesh.

    Vertices use (x, height, z) so Y is up. Faces are quads between adjacent
    grid cells in row-major order.
    """
    if not cells:
        return [], []
    xs = sorted({cell.x for cell in cells})
    zs = sorted({cell.z for cell in cells})
    heights = {(cell.x, cell.z): cell.height for cell in cells}

    nz = len(zs)
    nx = len(xs)
    vertices: list[tuple[float, float, float]] = []
    for z in zs:
        for x in xs:
            vertices.append((x, heights[(x, z)], z))

    faces: list[tuple[int, int, int, int]] = []
    for row in range(nz - 1):
        for col in range(nx - 1):
            i0 = row * nx + col
            i1 = i0 + 1
            i2 = (row + 1) * nx + col + 1
            i3 = (row + 1) * nx + col
            faces.append((i0, i1, i2, i3))
    return vertices, faces


def build_terrain_in_blender(cells: list[TerrainCell], material: Any) -> Any:
    """Create the terrain heightfield mesh in Blender."""
    import bpy

    vertices, faces = terrain_grid(cells)
    mesh = bpy.data.meshes.new("TerrainMesh")
    mesh.from_pydata(vertices, [], [list(face) for face in faces])
    mesh.update()
    if material is not None:
        mesh.materials.append(material)
    obj = bpy.data.objects.new("GrassTerrainVisual", mesh)
    scene = bpy.context.scene
    if scene is None and bpy.data.scenes:
        scene = bpy.data.scenes[0]
    if scene is not None:
        scene.collection.objects.link(obj)
    return obj
