"""Sanitize and remaster Formula-90 vehicle topology for a 2004-era game asset."""
from __future__ import annotations

import argparse
from hashlib import sha256
import json
from math import radians
from pathlib import Path
import sys

import bmesh
import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from prepare_godot_livery import library, style


REFINEMENT_CUTS = {
    "GEO_NOSE": 1,
    "GEO_AERO_FRONT_WING": 1,
    "GEO_DRIVER_HELMET": 1,
    "GEO_WHEEL_TIRE": 1,
}

CAP_BOUNDARIES = {"GEO_CHASSIS", "GEO_NOSE"}

SOLIDIFY_THICKNESS = {
    "GEO_CHASSIS": 0.001,
    "GEO_NOSE": 0.001,
    "GEO_AERO_FRONT_WING": 0.0005,
    "GEO_AERO_REAR_WING": 0.0005,
    "GEO_BODY_INTERIOR": 0.0005,
    "GEO_WHEEL_HUB": 0.001,
}

NO_BEVEL = {
    "GEO_CHASSIS", "GEO_NOSE", "GEO_AERO_FRONT_WING", "GEO_AERO_REAR_WING",
    "GEO_BODY_INTERIOR",
}


def file_hash(path: Path) -> str:
    digest = sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest().upper()


def activate(obj: bpy.types.Object) -> None:
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj


def bounds(obj: bpy.types.Object) -> dict:
    points = [obj.matrix_world @ Vector(corner) for corner in obj.bound_box]
    return {
        "min": [min(point[axis] for point in points) for axis in range(3)],
        "max": [max(point[axis] for point in points) for axis in range(3)],
    }


def topology(obj: bpy.types.Object, *, weld: bool = False) -> dict:
    mesh = obj.data
    mesh.calc_loop_triangles()
    bm = bmesh.new()
    bm.from_mesh(mesh)
    raw_vertices = len(bm.verts)
    if weld:
        bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=1e-6)
        bm.normal_update()
    result = {
        "vertices": len(bm.verts),
        "raw_vertices": raw_vertices,
        "edges": len(bm.edges),
        "polygons": len(mesh.polygons),
        "triangles": len(mesh.loop_triangles),
        "boundary_edges": sum(1 for edge in bm.edges if edge.is_boundary),
        "boundary_perimeter_m": sum(edge.calc_length() for edge in bm.edges if edge.is_boundary),
        "non_manifold_edges": sum(1 for edge in bm.edges if not edge.is_manifold),
        "degenerate_faces": sum(1 for face in bm.faces if face.calc_area() <= 1e-12),
        "bounds": bounds(obj),
    }
    bm.free()
    return result


def weld_and_clean(obj: bpy.types.Object) -> int:
    for layer in list(obj.data.uv_layers):
        obj.data.uv_layers.remove(layer)
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    before = len(bm.verts)
    bmesh.ops.remove_doubles(bm, verts=list(bm.verts), dist=1e-6)
    degenerate = [face for face in bm.faces if face.calc_area() <= 1e-12]
    if degenerate:
        bmesh.ops.delete(bm, geom=degenerate, context="FACES")
    if bm.faces:
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.validate(clean_customdata=True)
    obj.data.update()
    return before - len(obj.data.vertices)


def split_nonmanifold_junctions(obj: bpy.types.Object) -> int:
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    junctions = [edge for edge in bm.edges if len(edge.link_faces) > 2]
    count = len(junctions)
    if junctions:
        bmesh.ops.split_edges(bm, edges=junctions)
        bm.to_mesh(obj.data)
        obj.data.update()
    bm.free()
    return count


def finalize_closed_mesh(obj: bpy.types.Object) -> None:
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.dissolve_degenerate(bm, dist=1e-8, edges=list(bm.edges))
    degenerate = [face for face in bm.faces if face.calc_area() <= 1e-12]
    if degenerate:
        bmesh.ops.delete(bm, geom=degenerate, context="FACES")
    boundary = [edge for edge in bm.edges if edge.is_boundary]
    if boundary:
        bmesh.ops.holes_fill(bm, edges=boundary, sides=0)
    bmesh.ops.dissolve_degenerate(bm, dist=1e-8, edges=list(bm.edges))
    if bm.faces:
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.validate(clean_customdata=True)
    obj.data.update()


def seal_residual_cap(obj: bpy.types.Object) -> None:
    if obj.name not in CAP_BOUNDARIES:
        return
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    for distance in (1e-6, 1e-5, 1e-4):
        degenerate = [face for face in bm.faces if face.calc_area() <= 1e-12]
        if degenerate:
            bmesh.ops.delete(bm, geom=degenerate, context="FACES")
        boundary = [edge for edge in bm.edges if edge.is_boundary]
        if not boundary:
            break
        boundary_vertices = list({vertex for edge in boundary for vertex in edge.verts})
        bmesh.ops.remove_doubles(bm, verts=boundary_vertices, dist=distance)
        boundary = [edge for edge in bm.edges if edge.is_boundary]
        if boundary:
            bmesh.ops.holes_fill(bm, edges=boundary, sides=0)
        bmesh.ops.dissolve_degenerate(bm, dist=distance, edges=list(bm.edges))
    residual = [edge for edge in bm.edges if edge.is_boundary]
    if residual:
        bmesh.ops.edgenet_fill(bm, edges=residual, mat_nr=0, use_smooth=False, sides=0)
    if bm.faces:
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.validate(clean_customdata=True)
    obj.data.update()


def join_tire_surfaces() -> None:
    tires = sorted(
        (obj for obj in bpy.data.objects if obj.type == "MESH" and "WHEEL_TIRE" in obj.name),
        key=lambda item: item.name,
    )
    tread = bpy.data.objects.get("GEO_WHEEL_TREAD")
    if tread is not None:
        tires.append(tread)
    if not tires:
        return
    bpy.ops.object.select_all(action="DESELECT")
    for obj in tires:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = tires[0]
    bpy.ops.object.join()
    tires[0].name = "GEO_WHEEL_TIRE"
    tires[0].data.name = "GEO_WHEEL_TIRE"


def solidify(obj: bpy.types.Object, thickness: float) -> bool:
    before = topology(obj)
    if before["boundary_edges"] == 0:
        return False
    activate(obj)
    modifier = obj.modifiers.new("F90_Remaster_Solidify", "SOLIDIFY")
    modifier.thickness = thickness
    modifier.offset = -0.5
    modifier.use_even_offset = True
    modifier.use_quality_normals = True
    bpy.ops.object.modifier_apply(modifier=modifier.name)
    return True


def cap_boundaries(obj: bpy.types.Object) -> int:
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    boundary = [edge for edge in bm.edges if edge.is_boundary]
    count = len(boundary)
    if boundary:
        bmesh.ops.holes_fill(bm, edges=boundary, sides=0)
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
        bm.to_mesh(obj.data)
        obj.data.update()
    bm.free()
    return count


def refine_topology(obj: bpy.types.Object, cuts: int) -> bool:
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.subdivide_edges(
        bm, edges=list(bm.edges), cuts=cuts,
        use_grid_fill=True, smooth=0.0,
    )
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()
    return True


def bevel_closed_details(obj: bpy.types.Object) -> bool:
    if obj.name in NO_BEVEL or "LCD" in obj.name:
        return False
    activate(obj)
    modifier = obj.modifiers.new("F90_Remaster_Bevel", "BEVEL")
    modifier.width = 0.0002 if "LCD" in obj.name else (0.0008 if "WHEEL" in obj.name else 0.0012)
    modifier.segments = 2
    modifier.limit_method = "ANGLE"
    modifier.angle_limit = 0.7
    modifier.affect = "EDGES"
    bpy.ops.object.modifier_apply(modifier=modifier.name)
    return True


def create_provisional_uv(obj: bpy.types.Object) -> None:
    for layer in list(obj.data.uv_layers):
        obj.data.uv_layers.remove(layer)
    obj.data.uv_layers.new(name="UVMap")
    activate(obj)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(
        angle_limit=radians(66.0), island_margin=0.015,
        area_weight=0.0, correct_aspect=True, scale_to_bounds=True,
    )
    bpy.ops.object.mode_set(mode="OBJECT")


def remaster(source: Path, output: Path, report_path: Path) -> dict:
    source_hash = file_hash(source)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(source))
    join_tire_surfaces()
    objects = sorted((obj for obj in bpy.data.objects if obj.type == "MESH"), key=lambda item: item.name)
    audit = {}
    for obj in objects:
        original = topology(obj, weld=True)
        welded = weld_and_clean(obj)
        split_junctions = split_nonmanifold_junctions(obj)
        welded_topology = topology(obj)
        thickness = SOLIDIFY_THICKNESS.get(
            obj.name,
            0.001 if "LCD" in obj.name else (0.002 if obj.name == "GEO_WHEEL_TIRE" else 0.003),
        )
        capped_edges = cap_boundaries(obj) if obj.name in CAP_BOUNDARIES else 0
        was_solidified = False if capped_edges else solidify(obj, thickness)
        refined = False
        cuts = REFINEMENT_CUTS.get(obj.name)
        if cuts is not None:
            refined = refine_topology(obj, cuts)
        beveled = bevel_closed_details(obj)
        finalize_closed_mesh(obj)
        seal_residual_cap(obj)
        final = topology(obj)
        cap_residual_ok = (
            obj.name in CAP_BOUNDARIES
            and final["boundary_edges"] <= 12
            and final["degenerate_faces"] <= 10
        )
        if (final["degenerate_faces"] or final["boundary_edges"]) and not cap_residual_ok:
            raise ValueError(f"invalid topology remains in {obj.name}: {final}")
        for axis in range(3):
            old_min, old_max = original["bounds"]["min"][axis], original["bounds"]["max"][axis]
            new_min, new_max = final["bounds"]["min"][axis], final["bounds"]["max"][axis]
            tolerance = max(0.025, (old_max - old_min) * 0.06)
            if abs(new_min - old_min) > tolerance or abs(new_max - old_max) > tolerance:
                raise ValueError(
                    f"silhouette deviation exceeded for {obj.name} axis {axis}: "
                    f"old=({old_min},{old_max}) new=({new_min},{new_max}) tolerance={tolerance}"
                )
        audit[obj.name] = {
            "original": original,
            "after_weld": welded_topology,
            "final": final,
            "welded_vertices": welded,
            "split_nonmanifold_junctions": split_junctions,
            "solidified": was_solidified,
            "capped_boundary_edges": capped_edges,
            "solidify_thickness_m": thickness if was_solidified else 0.0,
            "linear_refinement": refined,
            "refinement_cuts": cuts,
            "beveled": beveled,
        }
    lib = library()
    for obj in objects:
        style(obj, lib)
        create_provisional_uv(obj)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.export_scene.gltf(
        filepath=str(output), export_format="GLB", use_selection=False,
        export_yup=True, export_apply=False,
    )
    if file_hash(source) != source_hash:
        raise ValueError("source GLB was modified")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(output))
    reimport = {
        obj.name: topology(obj)
        for obj in sorted((item for item in bpy.data.objects if item.type == "MESH"), key=lambda item: item.name)
    }
    report = {
        "schema_version": "formula90-topology-remaster/v1",
        "source": {"path": str(source), "sha256": source_hash},
        "output": {"path": str(output), "sha256": file_hash(output)},
        "mesh_count": len(reimport),
        "triangles": sum(item["triangles"] for item in reimport.values()),
        "boundary_edges_after_reimport": sum(item["boundary_edges"] for item in reimport.values()),
        "non_manifold_edges_after_reimport": sum(item["non_manifold_edges"] for item in reimport.values()),
        "audit": audit,
        "reimport": reimport,
        "source_unchanged": True,
        "uv_policy": "provisional Smart Project UV0; semantic UV rebuild follows topology human gate",
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    print("__FORMULA90_TOPOLOGY_REMASTER__" + json.dumps(report, separators=(",", ":")), flush=True)
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    remaster(args.source, args.output, args.report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
