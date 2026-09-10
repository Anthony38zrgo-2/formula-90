import bpy
import json
import sys
from mathutils import Vector
from pathlib import Path


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)


def bounds(objects):
    points = [obj.matrix_world @ Vector(corner) for obj in objects if obj.type == "MESH" for corner in obj.bound_box]
    minimum = Vector((min(p.x for p in points), min(p.y for p in points), min(p.z for p in points)))
    maximum = Vector((max(p.x for p in points), max(p.y for p in points), max(p.z for p in points)))
    return minimum, maximum, maximum - minimum


def object_details(obj):
    minimum, maximum, dimensions = bounds([obj])
    return {
        "materials": [slot.material.name if slot.material else "" for slot in obj.material_slots],
        "bounds_min": [round(v, 6) for v in minimum],
        "bounds_max": [round(v, 6) for v in maximum],
        "dimensions": [round(v, 6) for v in dimensions],
        "center": [round(v, 6) for v in (minimum + maximum) * 0.5],
    }


def inspect(path):
    clear_scene()
    bpy.ops.import_scene.gltf(filepath=str(path))
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    minimum, maximum, dimensions = bounds(meshes)
    tire_meshes = [obj for obj in meshes if obj.name.endswith("_TIRE")]
    tire_minimum, tire_maximum, tire_dimensions = bounds(tire_meshes) if tire_meshes else (Vector(), Vector(), Vector())
    groups = {}
    for group_name in ("SpinVisual", "BrakeStatic"):
        group = bpy.data.objects.get(group_name)
        groups[group_name] = sorted(obj.name for obj in group.children_recursive if obj.type == "MESH") if group else []
    return {
        "mesh_count": len(meshes),
        "vertices": sum(len(obj.data.vertices) for obj in meshes),
        "polygons": sum(len(obj.data.polygons) for obj in meshes),
        "bounds_min": [round(v, 6) for v in minimum],
        "bounds_max": [round(v, 6) for v in maximum],
        "dimensions": [round(v, 6) for v in dimensions],
        "center": [round(v, 6) for v in (minimum + maximum) * 0.5],
        "tire_dimensions": [round(v, 6) for v in tire_dimensions],
        "tire_center": [round(v, 6) for v in (tire_minimum + tire_maximum) * 0.5],
        "objects": {obj.name: object_details(obj) for obj in sorted(meshes, key=lambda item: item.name)},
        "visual_groups": groups,
    }


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    if len(args) != 2:
        raise SystemExit("usage: blender --background --python SCRIPT -- ASSET_DIR PHYSICS.json")
    asset_dir = Path(args[0]).resolve()
    physics = json.loads(Path(args[1]).read_text(encoding="utf-8"))
    manifest = json.loads((asset_dir / "manifest.json").read_text(encoding="utf-8"))
    names = ["chassis", "wheel_FL", "wheel_FR", "wheel_RL", "wheel_RR"]
    results = {name: inspect(asset_dir / f"f1_2030_v10_{name}.glb") for name in names}
    errors = []
    chassis_dims = results["chassis"]["dimensions"]
    # Blender re-import maps glTF/Godot Z back to Blender Y.
    if not (chassis_dims[1] > chassis_dims[0] and chassis_dims[1] > chassis_dims[2]):
        errors.append("chassis longitudinal axis is not Godot Z")
    alignment = manifest.get("authored_alignment", {})
    for key in ("front_track", "rear_track", "wheelbase"):
        if abs(float(alignment.get(key, -1.0)) - float(physics["geometry"][key])) > 0.001:
            errors.append(f"authored {key} does not match physics geometry")
    floor = results["chassis"]["objects"].get("GEO_CHASSIS_FLOOR")
    if floor is None or abs(float(floor["bounds_min"][2]) + float(physics["tires"]["front"]["radius"])) > 0.02:
        errors.append("chassis floor is not vertically aligned to the wheel contact plane")
    for suspension_name in ("GEO_CHASSIS_FRONT_SUSPENSION", "GEO_CHASSIS_REAR_SUSPENSION"):
        suspension = results["chassis"]["objects"].get(suspension_name)
        if suspension is None or not (suspension["bounds_min"][2] <= 0.0 <= suspension["bounds_max"][2]):
            errors.append(f"{suspension_name} does not cross the wheel-center plane")
    for short in ("FL", "FR", "RL", "RR"):
        result = results[f"wheel_{short}"]
        axle = "front" if short.startswith("F") else "rear"
        expected = physics["tires"][axle]
        dims = result["tire_dimensions"]
        center = result["tire_center"]
        if abs(dims[0] - float(expected["width"])) > 0.002:
            errors.append(f"wheel {short} width mismatch: {dims[0]}")
        if abs(dims[1] * 0.5 - float(expected["radius"])) > 0.002 or abs(dims[2] * 0.5 - float(expected["radius"])) > 0.002:
            errors.append(f"wheel {short} radius mismatch: {dims}")
        if max(abs(v) for v in center) > 0.002:
            errors.append(f"wheel {short} is not centered: {center}")
        if result["mesh_count"] != 7:
            errors.append(f"wheel {short} expected 7 meshes, got {result['mesh_count']}")
        wheel_prefix = f"GEO_WHEEL_{'FRONT' if short.startswith('F') else 'REAR'}_{'L' if short.endswith('L') else 'R'}"
        expected_static = [f"{wheel_prefix}_NUT_05", f"{wheel_prefix}_RIM_06"]
        if result["visual_groups"].get("BrakeStatic") != expected_static:
            errors.append(f"wheel {short} static brake group is wrong: {result['visual_groups'].get('BrakeStatic')}")
        if any(name in result["visual_groups"].get("SpinVisual", []) for name in expected_static):
            errors.append(f"wheel {short} caliper or brake duct still inherits wheel spin")
        if len(result["visual_groups"].get("SpinVisual", [])) != 5:
            errors.append(f"wheel {short} spin group expected 5 meshes")
        rim_key = next((name for name in result["objects"] if name.endswith("_RIM_03")), None)
        if rim_key is None:
            errors.append(f"wheel {short} reference rim disc is missing")
        else:
            rim_dimensions = result["objects"][rim_key]["dimensions"]
            if rim_dimensions[0] >= min(rim_dimensions[1], rim_dimensions[2]):
                errors.append(f"wheel {short} rim axle is not Godot X: {rim_dimensions}")
    report = {"passed": not errors, "errors": errors, "assets": results}
    (asset_dir / "validation_report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    if errors:
        raise RuntimeError("; ".join(errors))


if __name__ == "__main__":
    main()
