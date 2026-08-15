#!/usr/bin/env python3
"""Read-only Blender vehicle inspection for Formula-90 source assets.

Usage:
  blender --background source.blend --python inspect_blend_vehicle.py -- --report report.json

The script never saves the opened blend. It records world-space bounds, mesh
topology, loose components, transforms, materials, UV coverage and normals so
the source can be classified before any export or normalization is attempted.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def component_count(mesh: bpy.types.Mesh) -> int:
    """Count vertex-connected face islands without modifying the mesh."""
    if not mesh.polygons:
        return 0
    parent = list(range(len(mesh.vertices)))

    def find(index: int) -> int:
        while parent[index] != index:
            parent[index] = parent[parent[index]]
            index = parent[index]
        return index

    def union(left: int, right: int) -> None:
        left_root, right_root = find(left), find(right)
        if left_root != right_root:
            parent[right_root] = left_root

    used: set[int] = set()
    for polygon in mesh.polygons:
        indices = polygon.vertices
        if not indices:
            continue
        first = indices[0]
        used.update(indices)
        for index in indices[1:]:
            union(first, index)
    return len({find(index) for index in used})


def largest_components(mesh: bpy.types.Mesh, obj: bpy.types.Object, limit: int = 48) -> list[dict[str, object]]:
    """Summarize the largest vertex-connected face islands for spatial classification."""
    parent = list(range(len(mesh.vertices)))

    def find(index: int) -> int:
        while parent[index] != index:
            parent[index] = parent[parent[index]]
            index = parent[index]
        return index

    def union(left: int, right: int) -> None:
        left_root, right_root = find(left), find(right)
        if left_root != right_root:
            parent[right_root] = left_root

    for polygon in mesh.polygons:
        if not polygon.vertices:
            continue
        first = polygon.vertices[0]
        for index in polygon.vertices[1:]:
            union(first, index)

    groups: dict[int, dict[str, object]] = {}
    for polygon in mesh.polygons:
        if not polygon.vertices:
            continue
        root = find(polygon.vertices[0])
        group = groups.setdefault(root, {"faces": 0, "vertices": set()})
        group["faces"] += 1
        group["vertices"].update(polygon.vertices)

    summaries = []
    for group in sorted(groups.values(), key=lambda entry: entry["faces"], reverse=True)[:limit]:
        points = [obj.matrix_world @ mesh.vertices[index].co for index in group["vertices"]]
        low = [min(point[axis] for point in points) for axis in range(3)]
        high = [max(point[axis] for point in points) for axis in range(3)]
        summaries.append({
            "faces": group["faces"],
            "vertices": len(group["vertices"]),
            "bounds_world_m": {"min": low, "max": high, "extents": [high[axis] - low[axis] for axis in range(3)]},
            "center_world_m": [(high[axis] + low[axis]) * 0.5 for axis in range(3)],
        })
    return summaries


def object_report(obj: bpy.types.Object) -> dict[str, object]:
    mesh = obj.data
    world_vertices = [obj.matrix_world @ vertex.co for vertex in mesh.vertices]
    if world_vertices:
        low = Vector((min(v.x for v in world_vertices), min(v.y for v in world_vertices), min(v.z for v in world_vertices)))
        high = Vector((max(v.x for v in world_vertices), max(v.y for v in world_vertices), max(v.z for v in world_vertices)))
    else:
        low = high = Vector((0.0, 0.0, 0.0))

    uv_bounds = None
    if mesh.uv_layers.active is not None and mesh.uv_layers.active.data:
        values = [loop.uv for loop in mesh.uv_layers.active.data]
        uv_bounds = {
            "min": [min(value.x for value in values), min(value.y for value in values)],
            "max": [max(value.x for value in values), max(value.y for value in values)],
        }

    material_faces: dict[int, list[int]] = {}
    for polygon in mesh.polygons:
        material_faces.setdefault(polygon.material_index, []).append(polygon.index)
    material_report = []
    for material_index, polygon_indices in material_faces.items():
        used_vertices = {vertex_index for polygon_index in polygon_indices for vertex_index in mesh.polygons[polygon_index].vertices}
        points = [obj.matrix_world @ mesh.vertices[index].co for index in used_vertices]
        material = obj.material_slots[material_index].material if material_index < len(obj.material_slots) else None
        material_report.append({
            "material_index": material_index,
            "material": material.name if material else None,
            "faces": len(polygon_indices),
            "bounds_world_m": {
                "min": [min(point[axis] for point in points) for axis in range(3)],
                "max": [max(point[axis] for point in points) for axis in range(3)],
            },
        })

    return {
        "name": obj.name,
        "type": obj.type,
        "vertices": len(mesh.vertices),
        "faces": len(mesh.polygons),
        "triangles": sum(len(poly.vertices) - 2 for poly in mesh.polygons if len(poly.vertices) >= 3),
        "connected_components": component_count(mesh),
        "bounds_world_m": {"min": list(low), "max": list(high), "extents": list(high - low)},
        "location": list(obj.location),
        "rotation_euler": list(obj.rotation_euler),
        "scale": list(obj.scale),
        "transform_applied": all(abs(value - 1.0) < 1e-6 for value in obj.scale) and all(abs(value) < 1e-6 for value in obj.rotation_euler),
        "materials": [slot.material.name if slot.material else None for slot in obj.material_slots],
        "material_regions": material_report,
        "largest_components": largest_components(mesh, obj),
        "uv_layers": len(mesh.uv_layers),
        "uv_bounds": uv_bounds,
        "color_attributes": [attribute.name for attribute in mesh.color_attributes],
        "normals": {"available": bool(mesh.polygons), "custom": bool(getattr(mesh, "has_custom_normals", False))},
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else [])

    source = Path(bpy.data.filepath).resolve()
    objects = [object_report(obj) for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if not objects:
        raise RuntimeError("Blend contains no mesh objects")

    mins = [entry["bounds_world_m"]["min"] for entry in objects]
    maxs = [entry["bounds_world_m"]["max"] for entry in objects]
    low = [min(point[axis] for point in mins) for axis in range(3)]
    high = [max(point[axis] for point in maxs) for axis in range(3)]
    report = {
        "source": str(source),
        "sha256": sha256(source),
        "mesh_object_count": len(objects),
        "world_bounds_m": {"min": low, "max": high, "extents": [high[axis] - low[axis] for axis in range(3)]},
        "objects": objects,
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({"report": str(args.report), "mesh_object_count": len(objects)}, indent=2))


if __name__ == "__main__":
    main()
