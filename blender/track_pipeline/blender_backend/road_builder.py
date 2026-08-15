"""Road materialization from BuildIR road samples.

The pure function ``road_mesh_geometry`` derives the planar ribbon geometry
(left/right edges from the sample position, normal and half widths) and is
unit-testable without Blender. The Blender mesh/material assignment lives in
``build_road_in_blender``, which imports ``bpy`` lazily and is part of the
Blender-gated remainder of TS-110.
"""

from __future__ import annotations

from typing import Any

from .build_ir_loader import RoadSample


def road_mesh_geometry(samples: list[RoadSample]) -> tuple[list[tuple[float, float]], list[tuple[int, int, int, int]]]:
    """Return ``(vertices, faces)`` of the closed road ribbon.

    Each sample contributes a left and right edge point in (x, z). Faces are
    quads between consecutive samples plus a closing quad. Elevation is not
    applied here; Blender applies Y from the sample elevation/bank.
    """
    vertices: list[tuple[float, float]] = []
    for sample in samples:
        nx, nz = sample.normal
        left = (
            sample.position[0] - nx * sample.width_left,
            sample.position[1] - nz * sample.width_left,
        )
        right = (
            sample.position[0] + nx * sample.width_right,
            sample.position[1] + nz * sample.width_right,
        )
        vertices.append(left)
        vertices.append(right)

    count = len(samples)
    faces: list[tuple[int, int, int, int]] = []
    for index in range(count - 1):
        base = index * 2
        faces.append((base, base + 2, base + 3, base + 1))
    if count > 2:
        base = (count - 1) * 2
        faces.append((base, 0, 1, base + 1))
    return vertices, faces


def build_road_in_blender(samples: list[RoadSample], material: Any) -> Any:
    """Create the road mesh and assign a material in Blender.

    Requires an active Blender context (``bpy``). The mesh is built from the
    explicit BuildIR samples only; no semantic reinterpretation occurs here.
    """
    import bpy

    vertices_2d, faces = road_mesh_geometry(samples)
    vertices = [(x, 0.0, z) for x, z in vertices_2d]
    mesh = bpy.data.meshes.new("RoadMesh")
    mesh.from_pydata(vertices, [], [list(face) for face in faces])
    mesh.update()
    if material is not None:
        mesh.materials.append(material)
    obj = bpy.data.objects.new("RoadVisual", mesh)
    scene = bpy.context.scene
    if scene is None and bpy.data.scenes:
        scene = bpy.data.scenes[0]
    if scene is not None:
        scene.collection.objects.link(obj)
    return obj
