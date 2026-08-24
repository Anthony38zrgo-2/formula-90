"""Deterministic Blender worker for VS-032..VS-035 staged variants."""
from __future__ import annotations

import argparse
from hashlib import sha256
import json
from pathlib import Path
import sys
from math import pi
import re

import bpy
from mathutils import Vector

SENTINEL = "__FORMULA90_VEHICLE_STUDIO_MATERIALIZE_JSON__"
SUPPORTED_ROLES = {
    "wheelbase", "front_track", "rear_track", "front_tire_radius",
    "rear_tire_radius", "front_tire_width", "rear_tire_width",
}


def bind_albedo_materials(material_sources: list[dict]) -> dict:
    if not material_sources:
        return {"bound_count": 0, "bindings": []}
    source_by_slot = {item["material_slot"]: Path(item["absolute_path"])
                      for item in material_sources}
    bound = []
    missing = []
    for material in sorted(bpy.data.materials, key=lambda item: item.name):
        slot = re.sub(r"\.\d{3}$", "", material.name)
        source = source_by_slot.get(slot)
        if source is None:
            missing.append(slot)
            continue
        material.use_nodes = True
        nodes = material.node_tree.nodes
        links = material.node_tree.links
        principled = nodes.get("Principled BSDF")
        if principled is None:
            raise ValueError(f"Principled BSDF missing for {material.name}")
        texture = nodes.get("Formula90 Albedo") or nodes.new("ShaderNodeTexImage")
        texture.name = "Formula90 Albedo"
        texture.label = source.name
        texture.interpolation = "Closest"
        texture.image = bpy.data.images.load(str(source), check_existing=True)
        base_color = principled.inputs.get("Base Color")
        if base_color is None:
            raise ValueError(f"Base Color input missing for {material.name}")
        for link in list(base_color.links):
            links.remove(link)
        links.new(texture.outputs["Color"], base_color)
        bound.append({"material_slot": slot, "albedo": source.name})
    if missing:
        raise ValueError(f"albedo source missing for material: {missing[0]}")
    return {"bound_count": len(bound), "bindings": bound}


def file_hash(path: Path) -> str:
    digest = sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest().upper()


def parameter_map(build_ir: dict) -> dict[str, tuple[float, float]]:
    return {item["semantic_role"]: (float(item["baseline_value"]), float(item["absolute_value"]))
            for item in build_ir.get("parameters", []) if item.get("semantic_role") in SUPPORTED_ROLES}


def axle_blend(z: float, wheelbase: float) -> float:
    return min(1.0, max(0.0, (z + wheelbase / 2.0) / wheelbase))


def deform_chassis_point(point: Vector, parameters: dict[str, tuple[float, float]]) -> Vector:
    base_wb, target_wb = parameters.get("wheelbase", (1.0, 1.0))
    base_ft, target_ft = parameters.get("front_track", (1.0, 1.0))
    base_rt, target_rt = parameters.get("rear_track", (1.0, 1.0))
    base_fr, target_fr = parameters.get("front_tire_radius", (0.0, 0.0))
    base_rr, target_rr = parameters.get("rear_tire_radius", (0.0, 0.0))
    blend = axle_blend(point.z, base_wb)
    lateral_scale = (target_ft / base_ft) * (1.0 - blend) + (target_rt / base_rt) * blend
    height_delta = (target_fr - base_fr) * (1.0 - blend) + (target_rr - base_rr) * blend
    half_delta = (target_wb - base_wb) / 2.0
    if point.z <= -base_wb / 2.0:
        new_z = point.z - half_delta
    elif point.z >= base_wb / 2.0:
        new_z = point.z + half_delta
    else:
        new_z = point.z * target_wb / base_wb
    return Vector((point.x * lateral_scale, point.y + height_delta, new_z))


def apply_chassis(parameters: dict[str, tuple[float, float]]) -> None:
    for obj in sorted((item for item in bpy.data.objects if item.type == "MESH"), key=lambda item: item.name):
        world, inverse = obj.matrix_world.copy(), obj.matrix_world.inverted()
        for vertex in obj.data.vertices:
            vertex.co = inverse @ deform_chassis_point(world @ vertex.co, parameters)
        obj.data.update()
    empties = [item for item in bpy.data.objects if item.type == "EMPTY"]
    original = {item.name: item.matrix_world.translation.copy() for item in empties}
    def depth(item):
        value, parent = 0, item.parent
        while parent is not None:
            value, parent = value + 1, parent.parent
        return value
    for obj in sorted(empties, key=lambda item: (depth(item), item.name)):
        matrix = obj.matrix_world.copy()
        matrix.translation = deform_chassis_point(original[obj.name], parameters)
        obj.matrix_world = matrix


def topology_signature() -> list[dict]:
    return sorted(({"name": obj.name, "vertices": len(obj.data.vertices),
                    "polygons": len(obj.data.polygons), "uv_layers": len(obj.data.uv_layers)}
                   for obj in bpy.data.objects if obj.type == "MESH"), key=lambda item: item["name"])


def export_glb(path: str) -> None:
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", use_selection=False,
                              export_yup=True, export_apply=False)


def build_wheel(source: Path, output: str, radius: tuple[float, float],
                width: tuple[float, float], policy: str, material_sources: list[dict]) -> dict:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(source))
    materials = bind_albedo_materials(material_sources)
    before = topology_signature()
    radial_scale = radius[1] / radius[0]
    width_scale = width[1] / width[0]
    half_width = width[0] / 2.0
    pivot = {"centered": 0.0, "inboard_fixed": half_width,
             "outboard_fixed": -half_width}[policy]
    for obj in sorted((item for item in bpy.data.objects if item.type == "MESH"), key=lambda item: item.name):
        for vertex in obj.data.vertices:
            vertex.co.x = pivot + (vertex.co.x - pivot) * width_scale
            vertex.co.y *= radial_scale
            vertex.co.z *= radial_scale
        obj.data.update()
    after = topology_signature()
    if before != after:
        raise ValueError("wheel topology or UV membership changed")
    export_glb(output)
    return {"radius_m": radius[1], "width_m": width[1], "width_policy": policy,
            "contact_patch_y_m": -radius[1], "anchor_translation_m": [0.0, 0.0, 0.0],
            "topology_signature": after, "materials": materials}


def source_by_suffix(config: dict, suffix: str) -> Path:
    for item in config["sources"]:
        if item["path"].endswith(suffix):
            return Path(item["absolute_path"])
    raise ValueError(f"required modular source missing: {suffix}")


def add_wheel_instance(source: str, name: str, location: tuple[float, float, float],
                       rotate_right: bool) -> None:
    existing = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=source)
    imported = [obj for obj in bpy.data.objects if obj not in existing]
    top_level = [obj for obj in imported if obj.parent is None]
    anchor = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(anchor)
    anchor.location = location
    if rotate_right:
        anchor.rotation_euler[1] = pi
    for obj in top_level:
        obj.parent = anchor


def build_preview(config: dict, parameters: dict[str, tuple[float, float]], ground_y: float) -> dict:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=config["output_chassis_glb"])
    wheelbase = parameters["wheelbase"][1]
    front_track = parameters["front_track"][1]
    rear_track = parameters["rear_track"][1]
    front_radius = parameters["front_tire_radius"][1]
    rear_radius = parameters["rear_tire_radius"][1]
    placements = (
        ("WHEEL_FL", config["output_front_wheel_glb"], (-front_track / 2.0, ground_y + front_radius, -wheelbase / 2.0), False),
        ("WHEEL_FR", config["output_front_wheel_glb"], (front_track / 2.0, ground_y + front_radius, -wheelbase / 2.0), True),
        ("WHEEL_RL", config["output_rear_wheel_glb"], (-rear_track / 2.0, ground_y + rear_radius, wheelbase / 2.0), False),
        ("WHEEL_RR", config["output_rear_wheel_glb"], (rear_track / 2.0, ground_y + rear_radius, wheelbase / 2.0), True),
    )
    for name, source, location, rotate_right in placements:
        add_wheel_instance(source, name, location, rotate_right)
    export_glb(config["output_preview_glb"])
    return {"wheel_placements": [
        {"name": name, "translation_m": list(location), "right_side_rotation_y_rad": pi if rotate_right else 0.0}
        for name, _, location, rotate_right in placements
    ]}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    config = json.loads(args.config.read_text(encoding="utf-8"))
    for item in config["sources"]:
        if file_hash(Path(item["absolute_path"])) != item["sha256"]:
            raise ValueError(f"source hash mismatch inside Blender: {item['path']}")
    parameters = parameter_map(config["build_ir"])
    policies = config["build_ir"].get("width_policies", {})

    bpy.ops.wm.read_factory_settings(use_empty=True)
    chassis = Path(config["source"])
    bpy.ops.import_scene.gltf(filepath=str(chassis))
    chassis_materials = bind_albedo_materials(config["material_sources"])
    before = topology_signature()
    apply_chassis(parameters)
    after = topology_signature()
    if before != after:
        raise ValueError("chassis topology or UV membership changed")
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.file.pack_all()
    bpy.ops.wm.save_as_mainfile(filepath=config["output_blend"], check_existing=False)
    export_glb(config["output_chassis_glb"])

    front = build_wheel(
        source_by_suffix(config, "F1_94_wheel_front_geometry.glb"),
        config["output_front_wheel_glb"],
        parameters["front_tire_radius"], parameters["front_tire_width"],
        policies.get("front_tire_width", "centered"),
        config["material_sources"],
    )
    rear = build_wheel(
        source_by_suffix(config, "F1_94_wheel_rear_geometry.glb"),
        config["output_rear_wheel_glb"],
        parameters["rear_tire_radius"], parameters["rear_tire_width"],
        policies.get("rear_tire_width", "centered"),
        config["material_sources"],
    )
    ground_y = float(config["build_ir"]["operations"][0]["inputs"]["ground_y_m"])
    preview = build_preview(config, parameters, ground_y)
    base_wb, target_wb = parameters["wheelbase"]
    front_delta = parameters["front_tire_radius"][1] - parameters["front_tire_radius"][0]
    rear_delta = parameters["rear_tire_radius"][1] - parameters["rear_tire_radius"][0]
    measured = {role: target for role, (_, target) in sorted(parameters.items())}
    report = {
        "ok": True, "worker_version": 4, "mesh_count": len(after),
        "topology_signature": after, "measured_dimensions_m": measured,
        "materials": chassis_materials,
        "ground_contact": {"front_y_m": ground_y, "rear_y_m": ground_y},
        "preview": preview,
        "chassis_height": {"front_delta_m": front_delta, "rear_delta_m": rear_delta,
                           "rake_delta_rad": (rear_delta - front_delta) / target_wb},
        "wheels": {"front": front, "rear": rear},
    }
    print(SENTINEL + json.dumps(report, sort_keys=True, separators=(",", ":")), flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(SENTINEL + json.dumps({"ok": False, "code": "MATERIALIZER_WORKER_FAILED",
              "error": str(exc)}, sort_keys=True, separators=(",", ":")), flush=True)
        raise
