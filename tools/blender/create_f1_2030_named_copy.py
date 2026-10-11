import bpy

import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "blender"))
from formula_one_2030_object_names import enable_legacy_source_names, canonical_object_name
enable_legacy_source_names()

import hashlib
import json
import re


CHASSIS_NAMES = {
    "ActiveFront": "GEO_FRONT_WING_INTERMEDIATE_ELEMENT",
    "ActiveRear": "GEO_REAR_WING_MAIN_ELEMENT",
    "Aero2": "GEO_FRONT_WING_UPPER_ELEMENT",
    "AeroPart1": "GEO_SIDEPOD_FRONT_FLOW_DEFLECTORS",
    "AeroPart2": "GEO_REAR_WING_UPPER_ELEMENT",
    "AeroPart3": "GEO_REAR_WING_LOWER_BEAM_ELEMENT",
    "Body": "GEO_BODYWORK_NOSE_SIDEPODS_AND_ENGINE_COVER",
    "Carbon3": "GEO_ENGINE_AIR_INTAKE_INLET_SURFACE",
    "Carbon4": "GEO_COCKPIT_MIRROR_REFLECTIVE_SURFACES",
    "Carbon5": "GEO_ENGINE_COVER_REAR_CLOSURE",
    "Cylinder.001": "GEO_STEERING_WHEEL_CENTER_FASTENER",
    "Endplate": "GEO_FRONT_WING_ENDPLATES",
    "Floor": "GEO_UNDERBODY_FLOOR_AND_DIFFUSER",
    "Front Suspension": "GEO_SUSPENSION_FRONT_LINK_ASSEMBLY",
    "FrontSupport": "GEO_FRONT_WING_MOUNTING_PYLONS",
    "FrontWing": "GEO_FRONT_WING_MAIN_ELEMENT",
    "Interior": "GEO_COCKPIT_INTERIOR_SHELL",
    "LateralCam": "GEO_ENGINE_COVER_SIDE_CAMERA_HOUSINGS",
    "Mirrors": "GEO_COCKPIT_MIRROR_HOUSINGS_AND_SUPPORTS",
    "OnboardCam": "GEO_ENGINE_COVER_TOP_CAMERA_HOUSING",
    "Rear Suspension": "GEO_SUSPENSION_REAR_LINK_ASSEMBLY",
    "Rear Wing": "GEO_REAR_WING_ENDPLATE_ASSEMBLY",
    "RearLight": "GEO_REAR_SAFETY_LIGHT",
    "RearSupport": "GEO_REAR_WING_CENTRAL_SUPPORT",
    "Screen": "GEO_STEERING_WHEEL_DISPLAY",
    "Seat": "GEO_COCKPIT_DRIVER_SEAT",
    "ShifterBrak": "GEO_STEERING_WHEEL_PADDLE_LEFT",
    "ShifterThro": "GEO_STEERING_WHEEL_PADDLE_RIGHT",
    "Steer": "GEO_STEERING_WHEEL_BODY",
    "SteerColum": "GEO_STEERING_COLUMN_SHAFT",
}

CONTROL_NAMES = {
    "CARAXIS": "DATUM_VEHICLE_ORIGIN",
    "FL": "WHEEL_FRONT_LEFT_CONTROL",
    "FR": "WHEEL_FRONT_RIGHT_CONTROL",
    "RL": "WHEEL_REAR_LEFT_CONTROL",
    "RR": "WHEEL_REAR_RIGHT_CONTROL",
}

WHEEL_POSITIONS = {
    "FL": "FRONT_LEFT",
    "FR": "FRONT_RIGHT",
    "RL": "REAR_LEFT",
    "RR": "REAR_RIGHT",
}


def file_sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def snapshot():
    result = {}
    for obj in bpy.data.objects:
        mesh = obj.data if obj.type == "MESH" else None
        result[obj.name] = {
            "type": obj.type,
            "parent": obj.parent.name if obj.parent else None,
            "vertices": len(mesh.vertices) if mesh else None,
            "edges": len(mesh.edges) if mesh else None,
            "polygons": len(mesh.polygons) if mesh else None,
            "location": [float(value) for value in obj.location],
            "rotation_euler": [float(value) for value in obj.rotation_euler],
            "scale": [float(value) for value in obj.scale],
            "materials": [slot.material.name if slot.material else None for slot in obj.material_slots],
        }
    return result


def target_name(old_name):
    if old_name in CONTROL_NAMES:
        return CONTROL_NAMES[old_name]
    if old_name in CHASSIS_NAMES:
        return CHASSIS_NAMES[old_name]
    match = re.fullmatch(r"GEO_INDY2010_(FL|FR|RL|RR)_(.+)", old_name)
    if match:
        position, component = match.groups()
        return canonical_object_name(f"GEO_WHEEL_{WHEEL_POSITIONS[position]}_{component}")
    raise RuntimeError(f"No naming rule for object: {old_name}")


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    if len(args) != 3:
        raise SystemExit("usage: blender --background COPY.blend --python SCRIPT -- REFERENCE.blend OUTPUT.blend MANIFEST.json")

    reference_path = Path(args[0]).resolve()
    output_path = Path(args[1]).resolve()
    manifest_path = Path(args[2]).resolve()
    input_copy_path = Path(bpy.data.filepath).resolve()
    input_copy_sha256 = file_sha256(input_copy_path)
    before = snapshot()

    mapping = {name: target_name(name) for name in before}
    if len(set(mapping.values())) != len(mapping):
        raise RuntimeError("Naming map contains duplicate target object names")

    # Rename through temporary values so Blender never adds numeric suffixes.
    objects_by_old_name = {obj.name: obj for obj in bpy.data.objects}
    for index, (old_name, obj) in enumerate(objects_by_old_name.items()):
        obj.name = f"__F1_2030_RENAME_{index:03d}__"

    for old_name, obj in objects_by_old_name.items():
        new_name = mapping[old_name]
        obj.name = new_name
        if obj.type == "MESH":
            obj.data.name = f"{new_name}_MESH"

    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)

    after = snapshot()
    object_records = []
    for old_name, new_name in sorted(mapping.items()):
        old = before[old_name]
        new = after[new_name]
        invariant_keys = ("type", "vertices", "edges", "polygons", "location", "rotation_euler", "scale", "materials")
        unchanged = all(old[key] == new[key] for key in invariant_keys)
        object_records.append(
            {
                "old_name": old_name,
                "new_name": new_name,
                "data_name": bpy.data.objects[new_name].data.name if bpy.data.objects[new_name].data else None,
                "type": new["type"],
                "parent_before": old["parent"],
                "parent_after": new["parent"],
                "vertices": new["vertices"],
                "polygons": new["polygons"],
                "geometry_transform_materials_unchanged": unchanged,
            }
        )

    manifest = {
        "asset": "f1_2030",
        "reference_blend": str(reference_path),
        "reference_convention": {
            "chassis_geometry": "GEO_<ASSEMBLY>_<COMPONENT>",
            "wheel_geometry": "GEO_WHEEL_<POSITION>_<COMPONENT>",
            "wheel_controls": "WHEEL_<POSITION>_CONTROL",
            "datums": "DATUM_*",
            "mesh_datablocks": "<OBJECT_NAME>_MESH",
            "note": "Position tokens were added to split wheel components so all objects remain uniquely and semantically named.",
        },
        "input_copy_sha256_before_rename": input_copy_sha256,
        "output_blend": str(output_path),
        "output_sha256": file_sha256(output_path),
        "object_count_before": len(before),
        "object_count_after": len(after),
        "mesh_count_before": sum(value["type"] == "MESH" for value in before.values()),
        "mesh_count_after": sum(value["type"] == "MESH" for value in after.values()),
        "all_objects_named": len(mapping) == len(before),
        "all_invariants_unchanged": all(record["geometry_transform_materials_unchanged"] for record in object_records),
        "objects": object_records,
    }
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
