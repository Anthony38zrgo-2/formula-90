#!/usr/bin/env python3
"""Build the five normalized Jordan 191 replacement GLBs from supplied Blend files.

This is a reproducible, staging-only exporter.  It never saves source blends.
Coordinates are normalized to Formula-90: right +X, up +Y, front -Z.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

import bpy
from mathutils import Vector


# Measured from the source-component preflight, then mapped to the current
# Jordan 191 physical axle contract. These are source-space wheel centers.
FRONT_AXLE_SOURCE_Y = -4.00
REAR_AXLE_SOURCE_Y = 4.29
SOURCE_AXLE_MID_Y = (FRONT_AXLE_SOURCE_Y + REAR_AXLE_SOURCE_Y) * 0.5
SOURCE_GROUND_Z = 3.25
TARGET_WHEELBASE = 2.8788
CHASSIS_SCALE = TARGET_WHEELBASE / (REAR_AXLE_SOURCE_Y - FRONT_AXLE_SOURCE_Y)

WHEEL_TARGETS = {
    "fl": (0.5304, 0.5304, 0.5304),
    "fr": (0.5304, 0.5304, 0.5304),
    # The rear source is non-uniform. These factors restore its round rolling
    # envelope and the canonical 0.42 m rear visual width.
    "rl": (0.1888, 0.3557, 0.2743),
    "rr": (0.1888, 0.3557, 0.2743),
}

WHEEL_DATUMS = {
    "fl": (-0.739479, 0.0, -1.4394),
    "fr": (0.739257, 0.0, -1.4394),
    "rl": (-0.718479, 0.0, 1.4394),
    "rr": (0.718479, 0.0, 1.4394),
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def clear_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for mesh in list(bpy.data.meshes):
        bpy.data.meshes.remove(mesh)


def load_mesh(source: Path) -> bpy.types.Object:
    clear_scene()
    bpy.ops.wm.open_mainfile(filepath=str(source))
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if len(meshes) != 1:
        raise RuntimeError(f"expected exactly one mesh in {source}, found {len(meshes)}")
    return meshes[0]


def source_world(obj: bpy.types.Object, coordinate: Vector) -> Vector:
    return obj.matrix_world @ coordinate


def formula_transform(point: Vector) -> Vector:
    """Source Blender: X left, Z up, -Y front -> Formula-90 runtime contract."""
    return Vector((
        -point.x * CHASSIS_SCALE,
        -(point.y - SOURCE_AXLE_MID_Y) * CHASSIS_SCALE,
        (point.z - SOURCE_GROUND_Z) * CHASSIS_SCALE,
    ))


def wheel_transform(point: Vector, factors: tuple[float, float, float]) -> Vector:
    # Raw Blender -> Formula-90; separate scale corrects the source wheel envelope.
    return Vector((-point.x * factors[0], -point.y * factors[2], point.z * factors[1]))


def runtime_to_blender(point: tuple[float, float, float]) -> tuple[float, float, float]:
    """Inverse of Blender glTF's (x, z, -y) coordinate export conversion."""
    return (point[0], -point[2], point[1])


def in_source_wheel_envelope(point: Vector) -> bool:
    # Remove visual geometry in the four runtime wheel bays. The source is one
    # joined mesh, so spatial classification is safer than its anonymous mesh
    # name; wheel-bay envelopes may intentionally trim nearby visual wishbones.
    runtime = formula_transform(point)
    return abs(runtime.x) >= 0.50 and abs(runtime.z) <= 2.35 and -0.60 <= runtime.y <= 0.60


def rebuild_mesh(obj: bpy.types.Object, transform, remove_wheels: bool) -> dict[str, int]:
    old_mesh = obj.data
    world_vertices = [source_world(obj, vertex.co) for vertex in old_mesh.vertices]
    vertices = [transform(point) for point in world_vertices]
    faces: list[tuple[int, ...]] = []
    material_indices: list[int] = []
    removed = 0
    degenerate = 0
    for polygon in old_mesh.polygons:
        centroid = sum((world_vertices[index] for index in polygon.vertices), Vector()) / len(polygon.vertices)
        if remove_wheels and in_source_wheel_envelope(centroid):
            removed += 1
            continue
        polygon_vertices = tuple(polygon.vertices)
        anchor = vertices[polygon_vertices[0]]
        if not any((vertices[polygon_vertices[index]] - anchor).cross(vertices[polygon_vertices[index + 1]] - anchor).length_squared > 1e-16 for index in range(1, len(polygon_vertices) - 1)):
            degenerate += 1
            continue
        faces.append(polygon_vertices)
        material_indices.append(polygon.material_index)

    mesh = bpy.data.meshes.new("GEO_CHASSIS" if remove_wheels else "GEO_WHEEL")
    mesh.from_pydata(vertices, [], faces)
    mesh.materials.clear()
    for material in old_mesh.materials:
        mesh.materials.append(material)
    for polygon, material_index in zip(mesh.polygons, material_indices):
        polygon.material_index = material_index
        polygon.use_smooth = True
    mesh.update(calc_edges=True)
    obj.data = mesh
    obj.matrix_world.identity()
    obj.location = (0.0, 0.0, 0.0)
    obj.rotation_euler = (0.0, 0.0, 0.0)
    obj.scale = (1.0, 1.0, 1.0)
    obj.name = mesh.name
    return {"wheel_faces": removed, "degenerate_faces": degenerate}


def add_empty(name: str, location: tuple[float, float, float]) -> bpy.types.Object:
    bpy.ops.object.empty_add(type="PLAIN_AXES", location=location)
    empty = bpy.context.object
    empty.name = name
    empty.empty_display_size = 0.02
    return empty


def export_selected(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_materials="EXPORT",
    )


def export_chassis(source: Path, output: Path) -> dict[str, object]:
    obj = load_mesh(source)
    removed = rebuild_mesh(obj, formula_transform, remove_wheels=True)
    obj.name = "GEO_CHASSIS"
    add_empty("DATUM_VEHICLE_ORIGIN", runtime_to_blender((0.0, 0.0, 0.0)))
    add_empty("DATUM_FRONT_AXLE_CENTER", runtime_to_blender((0.0, 0.0, -1.4394)))
    add_empty("DATUM_REAR_AXLE_CENTER", runtime_to_blender((0.0, 0.0, 1.4394)))
    for corner, position in WHEEL_DATUMS.items():
        add_empty(f"JNT_WHEEL_{corner.upper()}", runtime_to_blender(position))
    add_empty("JNT_FRONT_WING_MOUNT", runtime_to_blender((0.0, 0.0, -1.9)))
    add_empty("JNT_REAR_WING_MOUNT", runtime_to_blender((0.0, 0.65, 1.9)))
    export_selected(output)
    return {"source": str(source), "source_sha256": sha256(source), "removed_embedded_wheel_faces": removed["wheel_faces"], "removed_degenerate_faces": removed["degenerate_faces"]}


def export_wheel(corner: str, source: Path, output: Path) -> dict[str, object]:
    obj = load_mesh(source)
    cleanup = rebuild_mesh(obj, lambda point: wheel_transform(point, WHEEL_TARGETS[corner]), remove_wheels=False)
    obj.name = f"GEO_WHEEL_{corner.upper()}_ASSEMBLY"
    add_empty(f"JNT_WHEEL_{corner.upper()}", (0.0, 0.0, 0.0))
    add_empty("DATUM_WHEEL_ORIGIN", (0.0, 0.0, 0.0))
    export_selected(output)
    return {"source": str(source), "source_sha256": sha256(source), "scale": list(WHEEL_TARGETS[corner]), "removed_degenerate_faces": cleanup["degenerate_faces"]}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--formula", required=True, type=Path)
    parser.add_argument("--wheel-fl", required=True, type=Path)
    parser.add_argument("--wheel-fr", required=True, type=Path)
    parser.add_argument("--wheel-rl", required=True, type=Path)
    parser.add_argument("--wheel-rr", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else [])

    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)
    result = {
        "coordinate_contract": {"right": "+X", "up": "+Y", "front": "-Z", "units": "meters"},
        "chassis_scale": CHASSIS_SCALE,
        "source_axles_y": {"front": FRONT_AXLE_SOURCE_Y, "rear": REAR_AXLE_SOURCE_Y},
        "wheel_datums_m": WHEEL_DATUMS,
        "files": {},
    }
    result["files"]["chassis"] = export_chassis(args.formula, output_dir / "jordan_191_chassis.glb")
    for corner, source in (("fl", args.wheel_fl), ("fr", args.wheel_fr), ("rl", args.wheel_rl), ("rr", args.wheel_rr)):
        result["files"][corner] = export_wheel(corner, source, output_dir / f"jordan_191_wheel_{corner}.glb")
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
