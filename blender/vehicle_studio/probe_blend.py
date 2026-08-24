#!/usr/bin/env python3
"""Read-only Vehicle Studio Blend probe executed inside Blender."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import sys


SENTINEL = "__FORMULA90_VEHICLE_STUDIO_BLEND_JSON__"


def emit(payload):
    print(
        SENTINEL
        + json.dumps(
            payload,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
            allow_nan=False,
        )
    )


def matrix_rows(matrix):
    return [[float(matrix[row][col]) for col in range(4)] for row in range(4)]


def determinant3(matrix):
    return (
        matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
    )


def json_property(value, depth=0):
    if depth > 8:
        return "<max-depth>"
    if value is None or isinstance(value, (bool, int, float, str)):
        if isinstance(value, float) and not math.isfinite(value):
            return "<non-finite>"
        return value
    if hasattr(value, "to_list"):
        return json_property(value.to_list(), depth + 1)
    if isinstance(value, dict) or hasattr(value, "items"):
        return {
            str(key): json_property(child, depth + 1)
            for key, child in sorted(value.items(), key=lambda item: str(item[0]))
        }
    if isinstance(value, (list, tuple)) or hasattr(value, "__iter__"):
        try:
            return [json_property(child, depth + 1) for child in value]
        except TypeError:
            pass
    return str(value)


def custom_properties(owner):
    return {
        key: json_property(owner[key])
        for key in sorted(owner.keys())
        if key != "_RNA_UI"
    }


def mesh_facts(obj, depsgraph):
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        mesh.calc_loop_triangles()
        local_min = [math.inf, math.inf, math.inf]
        local_max = [-math.inf, -math.inf, -math.inf]
        finite_vertices = True
        for vertex in mesh.vertices:
            coordinates = [float(value) for value in vertex.co]
            if not all(math.isfinite(value) for value in coordinates):
                finite_vertices = False
                continue
            for axis in range(3):
                local_min[axis] = min(local_min[axis], coordinates[axis])
                local_max[axis] = max(local_max[axis], coordinates[axis])
        bounds = None
        if not math.isinf(local_min[0]):
            bounds = {"min": local_min, "max": local_max}
        uv_layers = []
        for layer in sorted(mesh.uv_layers, key=lambda item: item.name):
            uv_min = [math.inf, math.inf]
            uv_max = [-math.inf, -math.inf]
            finite_uv = True
            for datum in layer.data:
                uv = [float(datum.uv[0]), float(datum.uv[1])]
                if not all(math.isfinite(value) for value in uv):
                    finite_uv = False
                    continue
                for axis in range(2):
                    uv_min[axis] = min(uv_min[axis], uv[axis])
                    uv_max[axis] = max(uv_max[axis], uv[axis])
            uv_bounds = None
            if not math.isinf(uv_min[0]):
                uv_bounds = {"min": uv_min, "max": uv_max}
            uv_layers.append(
                {
                    "name": layer.name,
                    "active": mesh.uv_layers.active == layer,
                    "loop_count": len(layer.data),
                    "finite": finite_uv,
                    "bounds": uv_bounds,
                }
            )
        return {
            "vertex_count": len(mesh.vertices),
            "triangle_count": len(mesh.loop_triangles),
            "loop_count": len(mesh.loops),
            "finite_vertices": finite_vertices,
            "local_bounds": bounds,
            "uv_layers": uv_layers,
            "material_slots": [slot.material.name if slot.material else None for slot in obj.material_slots],
        }
    finally:
        evaluated.to_mesh_clear()


def object_facts(obj, depsgraph):
    world = matrix_rows(obj.matrix_world)
    result = {
        "name": obj.name,
        "type": obj.type,
        "parent": obj.parent.name if obj.parent else None,
        "collections": sorted(collection.name for collection in obj.users_collection),
        "world_transform": world,
        "world_determinant": determinant3([row[:3] for row in world[:3]]),
        "modifiers": [
            {
                "name": modifier.name,
                "type": modifier.type,
                "show_viewport": bool(modifier.show_viewport),
                "show_render": bool(modifier.show_render),
            }
            for modifier in sorted(obj.modifiers, key=lambda item: item.name)
        ],
        "custom_properties": custom_properties(obj),
        "mesh": None,
    }
    if obj.type == "MESH":
        result["mesh"] = mesh_facts(obj, depsgraph)
    return result


def material_facts(material):
    nodes = []
    image_paths = []
    if material.use_nodes and material.node_tree:
        for node in sorted(material.node_tree.nodes, key=lambda item: item.name):
            nodes.append({"name": node.name, "type": node.bl_idname})
            image = getattr(node, "image", None)
            if image is not None:
                image_paths.append(image.filepath)
    return {
        "name": material.name,
        "use_nodes": bool(material.use_nodes),
        "nodes": nodes,
        "image_paths": sorted(set(image_paths)),
        "custom_properties": custom_properties(material),
    }


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1 :]
    return parser.parse_args(argv)


def main():
    args = parse_args()
    try:
        config = json.loads(Path(args.config).read_text(encoding="utf-8"))
        source = Path(config["source"])
        requested = config.get("objects", [])
        if not source.is_absolute() or source.suffix.lower() != ".blend":
            raise ValueError("source must be an absolute .blend path")
        if not isinstance(requested, list) or not all(isinstance(item, str) for item in requested):
            raise ValueError("objects must be an array of names")
    except (KeyError, OSError, ValueError, json.JSONDecodeError) as exc:
        emit({"ok": False, "class": "precondition", "error": str(exc)})
        return 2

    try:
        import bpy

        bpy.ops.wm.open_mainfile(filepath=str(source), load_ui=False)
        depsgraph = bpy.context.evaluated_depsgraph_get()
        if requested:
            names = sorted(set(requested))
        else:
            names = sorted(obj.name for obj in bpy.data.objects)
        objects = []
        for name in names:
            obj = bpy.data.objects.get(name)
            if obj is None:
                objects.append({"name": name, "exists": False})
            else:
                fact = object_facts(obj, depsgraph)
                fact["exists"] = True
                objects.append(fact)
        used_material_names = sorted(
            {
                slot.material.name
                for obj in bpy.data.objects
                for slot in obj.material_slots
                if slot.material is not None
            }
        )
        materials = [material_facts(bpy.data.materials[name]) for name in used_material_names]
        emit(
            {
                "ok": True,
                "blender_version": bpy.app.version_string,
                "objects": objects,
                "object_count": len(objects),
                "materials": materials,
                "material_count": len(materials),
                "scene_custom_properties": custom_properties(bpy.context.scene),
            }
        )
        return 0
    except Exception as exc:
        emit({"ok": False, "class": "corrupt", "error": str(exc)})
        return 3


if __name__ == "__main__":
    raise SystemExit(main())

