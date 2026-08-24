"""Audit Formula-90 GLB topology inside Blender without modifying the source."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

import bmesh
import bpy


def boundary_loops(bm: bmesh.types.BMesh) -> list[dict]:
    boundary = {edge for edge in bm.edges if edge.is_boundary}
    loops: list[dict] = []
    while boundary:
        seed = boundary.pop()
        component = {seed}
        frontier = [seed]
        vertices = set(seed.verts)
        while frontier:
            edge = frontier.pop()
            for vertex in edge.verts:
                for linked in vertex.link_edges:
                    if linked in boundary:
                        boundary.remove(linked)
                        component.add(linked)
                        frontier.append(linked)
                        vertices.update(linked.verts)
        perimeter = sum(edge.calc_length() for edge in component)
        loops.append({"edges": len(component), "vertices": len(vertices), "perimeter_m": perimeter})
    return sorted(loops, key=lambda item: item["perimeter_m"], reverse=True)


def mesh_audit(obj: bpy.types.Object) -> dict:
    mesh = obj.data
    bm = bmesh.new()
    bm.from_mesh(mesh)
    vertices_before_weld = len(bm.verts)
    bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=1e-6)
    bm.normal_update()
    welded_vertices = len(bm.verts)
    bm.to_mesh(mesh)
    mesh.update()
    mesh.calc_loop_triangles()
    loops = boundary_loops(bm)
    result = {
        "vertices": len(mesh.vertices),
        "vertices_before_weld": vertices_before_weld,
        "welded_vertices": vertices_before_weld - welded_vertices,
        "edges": len(mesh.edges),
        "polygons": len(mesh.polygons),
        "triangles": len(mesh.loop_triangles),
        "boundary_edges": sum(1 for edge in bm.edges if edge.is_boundary),
        "non_manifold_edges": sum(1 for edge in bm.edges if not edge.is_manifold),
        "degenerate_faces": sum(1 for face in bm.faces if face.calc_area() <= 1e-12),
        "boundary_loops": loops,
        "material_slots": [slot.material.name if slot.material else None for slot in obj.material_slots],
    }
    bm.free()
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(args.source))
    meshes = {
        obj.name: mesh_audit(obj)
        for obj in sorted((item for item in bpy.data.objects if item.type == "MESH"), key=lambda item: item.name)
    }
    report = {
        "source": str(args.source),
        "mesh_count": len(meshes),
        "triangles": sum(item["triangles"] for item in meshes.values()),
        "boundary_edges": sum(item["boundary_edges"] for item in meshes.values()),
        "non_manifold_edges": sum(item["non_manifold_edges"] for item in meshes.values()),
        "meshes": meshes,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    print("__FORMULA90_TOPOLOGY_AUDIT__" + json.dumps(report, separators=(",", ":")), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
