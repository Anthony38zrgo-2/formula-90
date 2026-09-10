import bpy
import hashlib
import json
import sys
from mathutils import Matrix, Vector
from pathlib import Path


ASSET_ID = "f1_2030_v10"
WHEELS = {
    "FL": "FRONT_L",
    "FR": "FRONT_R",
    "RL": "REAR_L",
    "RR": "REAR_R",
}

# NUT_05 is the red caliper and RIM_06 is the inboard brake-duct cover. Both
# follow hub travel, steering and camber, but never longitudinal wheel spin.
STATIC_WHEEL_SUFFIXES = ("_NUT_05", "_RIM_06")


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()


def clear_export_objects():
    for obj in list(bpy.data.objects):
        if obj.get("f1_2030_export_temp", False):
            bpy.data.objects.remove(obj, do_unlink=True)


def duplicate_mesh_object(source, source_name, matrix_world):
    duplicate = source.copy()
    duplicate.data = source.data.copy()
    duplicate.data.transform(matrix_world)
    duplicate.data.update()
    duplicate.name = source_name
    duplicate.data.name = source_name + "_MESH"
    duplicate.parent = None
    duplicate.matrix_world = Matrix.Identity(4)
    duplicate["f1_2030_export_temp"] = True
    bpy.context.scene.collection.objects.link(duplicate)
    return duplicate


def duplicate_group(sources, matrices):
    records = [(obj, obj.name, obj.data.name) for obj in sources]
    for obj, old_name, _ in records:
        obj.name = "__SOURCE_" + old_name
    duplicates = [duplicate_mesh_object(obj, old_name, matrix) for (obj, old_name, _), matrix in zip(records, matrices)]
    return duplicates, records


def restore_sources(records):
    for obj, old_name, old_data_name in records:
        obj.name = old_name
        obj.data.name = old_data_name


def export_selected(objects, output):
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.export_scene.gltf(
        filepath=str(output),
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_apply=True,
        export_cameras=False,
        export_lights=False,
        export_animations=False,
    )


def bbox_center_world(obj):
    corners = [obj.matrix_world @ Vector(corner) for corner in obj.bound_box]
    minimum = Vector((min(v.x for v in corners), min(v.y for v in corners), min(v.z for v in corners)))
    maximum = Vector((max(v.x for v in corners), max(v.y for v in corners), max(v.z for v in corners)))
    return (minimum + maximum) * 0.5


def export_chassis(output, vertical_offset):
    # Source axes: X longitudinal, Y lateral, Z up. Rotate -90 degrees around
    # Blender Z so the glTF Y-up conversion yields Godot X right/Y up/Z rear.
    conversion = Matrix.Translation((0.0, 0.0, vertical_offset)) @ Matrix.Rotation(-1.5707963267948966, 4, "Z")
    sources = sorted((obj for obj in bpy.data.objects if obj.type == "MESH" and obj.name.startswith("GEO_CHASSIS_")), key=lambda obj: obj.name)
    duplicates, records = duplicate_group(sources, [conversion @ obj.matrix_world for obj in sources])
    export_selected(duplicates, output)
    stats = {obj.name: {"vertices": len(obj.data.vertices), "polygons": len(obj.data.polygons)} for obj in duplicates}
    clear_export_objects()
    restore_sources(records)
    return stats


def export_wheel(position, output, tire_radius, tire_width):
    prefix = f"GEO_WHEEL_{position}_"
    sources = sorted((obj for obj in bpy.data.objects if obj.type == "MESH" and obj.name.startswith(prefix)), key=lambda obj: obj.name)
    tire = next((obj for obj in sources if obj.name.endswith("_TIRE")), None)
    if not sources or tire is None:
        raise RuntimeError(f"Incomplete wheel group: {position}")
    center = bbox_center_world(tire)
    corners = [tire.matrix_world @ Vector(corner) for corner in tire.bound_box]
    extents = Vector((
        max(v.x for v in corners) - min(v.x for v in corners),
        max(v.y for v in corners) - min(v.y for v in corners),
        max(v.z for v in corners) - min(v.z for v in corners),
    ))
    # The source objects carry a +90 degree X rotation. Consequently their
    # evaluated world-space axle is Y (not the mesh-local Z axis), while X/Z
    # form the radial plane.
    radial_scale = (2.0 * tire_radius) / max(extents.x, extents.z)
    width_scale = tire_width / extents.y
    translate = Matrix.Translation(-center)
    # Reorient the evaluated source Y axle to Godot X. Preserve source Z as up
    # and map source X to the vehicle's longitudinal axis. The glTF exporter
    # performs the final Blender Z-up to Godot/glTF Y-up conversion.
    wheel_to_blender = Matrix(((0, 1, 0, 0), (-1, 0, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1)))
    target_scale = Matrix.Diagonal((width_scale, radial_scale, radial_scale, 1.0))
    # target_scale already derives the exact wheel dimensions from evaluated
    # world bounds. Do not normalize the tire a second time: that can make its
    # AABB plausible while leaving the rim topology on the wrong axle.
    duplicates, records = duplicate_group(sources, [target_scale @ wheel_to_blender @ translate @ obj.matrix_world for obj in sources])
    spin_root = bpy.data.objects.new("SpinVisual", None)
    brake_root = bpy.data.objects.new("BrakeStatic", None)
    for root in (spin_root, brake_root):
        root["f1_2030_export_temp"] = True
        bpy.context.scene.collection.objects.link(root)
    for duplicate in duplicates:
        duplicate.parent = brake_root if duplicate.name.endswith(STATIC_WHEEL_SUFFIXES) else spin_root
    export_selected([spin_root, brake_root, *duplicates], output)
    stats = {obj.name: {"vertices": len(obj.data.vertices), "polygons": len(obj.data.polygons)} for obj in duplicates}
    stats["visual_groups"] = {
        "spin": [obj.name for obj in duplicates if obj.parent == spin_root],
        "static_brake": [obj.name for obj in duplicates if obj.parent == brake_root],
    }
    clear_export_objects()
    restore_sources(records)
    return stats


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    if len(args) != 2:
        raise SystemExit("usage: blender --background SOURCE.blend --python SCRIPT -- OUTPUT_DIR PHYSICS.json")
    output_dir = Path(args[0]).resolve()
    physics_path = Path(args[1]).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    physics = json.loads(physics_path.read_text(encoding="utf-8"))

    wheel_centers = {}
    for short, position in WHEELS.items():
        tire = bpy.data.objects.get(f"GEO_WHEEL_{position}_TIRE")
        if tire is None:
            raise RuntimeError(f"Missing wheel datum tire: {position}")
        wheel_centers[short] = bbox_center_world(tire)
    front_center_x = (wheel_centers["FL"].x + wheel_centers["FR"].x) * 0.5
    rear_center_x = (wheel_centers["RL"].x + wheel_centers["RR"].x) * 0.5
    authored_geometry = {
        "front_track": abs(wheel_centers["FR"].y - wheel_centers["FL"].y),
        "rear_track": abs(wheel_centers["RR"].y - wheel_centers["RL"].y),
        "wheelbase": abs(rear_center_x - front_center_x),
    }
    for key, value in authored_geometry.items():
        configured = float(physics["geometry"][key])
        if abs(configured - value) > 0.001:
            raise RuntimeError(f"Physics {key}={configured:.6f} does not match authored wheel datums {value:.6f}")
    wheel_center_z = sum(center.z for center in wheel_centers.values()) / len(wheel_centers)
    chassis_vertical_offset = -wheel_center_z

    files = {"chassis": output_dir / f"{ASSET_ID}_chassis.glb"}
    stats = {"chassis": export_chassis(files["chassis"], chassis_vertical_offset)}
    for short, position in WHEELS.items():
        key = f"wheel_{short}"
        files[key] = output_dir / f"{ASSET_ID}_wheel_{short}.glb"
        axle = "front" if short.startswith("F") else "rear"
        tire_spec = physics["tires"][axle]
        stats[key] = export_wheel(position, files[key], float(tire_spec["radius"]), float(tire_spec["width"]))

    geometry_assets = {
        key: {"path": path.name, "sha256": sha256(path), "size_bytes": path.stat().st_size}
        for key, path in files.items()
    }
    manifest = {
        "schema_version": 8,
        "schema": "formula90s/vehicle-import/v1",
        "vehicle_id": ASSET_ID,
        "standard": "Formula-90 GEVP decoupled visual asset",
        "asset": "F1_2030_V10",
        "physics_profile": "res://data/vehicles/f1_2030/f1_2030_v10_physics.json",
        "physics_sha256": sha256(physics_path),
        "coordinate_contract": {"right": "+X", "left": "-X", "up": "+Y", "front": "-Z", "rear": "+Z", "units": "meters", "transformations_applied": True},
        "authored_alignment": {
            "front_track": authored_geometry["front_track"],
            "rear_track": authored_geometry["rear_track"],
            "wheelbase": authored_geometry["wheelbase"],
            "chassis_vertical_offset": chassis_vertical_offset,
        },
        "wheel_visual_contract": {
            "SpinVisual": "tire, rim, hub and brake disc rotate about local X",
            "BrakeStatic": "brake caliper and inboard brake duct follow suspension, steering and camber but do not rotate",
        },
        "geometry_assets": geometry_assets,
        "validation": {"status": "EXPORT_PASS", "note": "Five canonical GLBs; every wheel contains separate SpinVisual and BrakeStatic groups."},
    }
    report = {
        "operation": "Export f1_2030_v10 as chassis plus four independent wheel GLBs",
        "passed": True,
        "files": geometry_assets,
        "mesh_stats": stats,
        "authored_alignment": manifest["authored_alignment"],
    }
    (output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    (output_dir / "export_report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
