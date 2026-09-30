import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector


PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
CALIBRATED_TORSO_RECLINE_DEGREES = 79.86680690888585
sys.path.insert(0, str(PROJECT_DIRECTORY / "tools"))
sys.path.insert(0, str(PROJECT_DIRECTORY / "tools/drivers"))
from common.output_policy import validate_output_path
from articulated_driver_gloves import create_articulated_gloves


def move_bone_towards(armature, bone_name, direction):
    pose_bone = armature.pose.bones[bone_name]
    current_direction = pose_bone.tail - pose_bone.head
    rotation = current_direction.normalized().rotation_difference(direction.normalized())
    pose_bone.matrix = Matrix.Translation(pose_bone.head) @ rotation.to_matrix().to_4x4() @ Matrix.Translation(-pose_bone.head) @ pose_bone.matrix
    bpy.context.view_layer.update()


def measure_seat_recline(chassis_path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(chassis_path))
    seat = bpy.data.objects["GEO_CHASSIS_SEAT"]
    profile = {}
    for vertex in seat.data.vertices:
        position = seat.matrix_world @ vertex.co
        longitudinal_position = round(-position.y, 4)
        if abs(position.x) < 0.035 and 0.06 < longitudinal_position < 0.27:
            profile[longitudinal_position] = max(profile.get(longitudinal_position, 0.0), position.z)
    profile_stations = sorted(profile)
    lower_station = profile_stations[0]
    upper_station = profile_stations[-1]
    return math.degrees(math.atan2(upper_station - lower_station, profile[upper_station] - profile[lower_station]))


def slim_character_body(armature, body, radial_scale):
    body_to_armature = armature.matrix_world.inverted() @ body.matrix_world
    armature_to_body = body_to_armature.inverted()
    for vertex in body.data.vertices:
        dominant_group = max(vertex.groups, key=lambda group: group.weight, default=None)
        if dominant_group is None:
            continue
        bone_name = body.vertex_groups[dominant_group.group].name
        if any(part in bone_name for part in ("Head", "Neck", "Hand", "Foot", "Toe")):
            continue
        bone = armature.data.bones.get(bone_name)
        if bone is None:
            continue
        position = body_to_armature @ vertex.co
        bone_direction = (bone.tail_local - bone.head_local).normalized()
        center_on_bone = bone.head_local + bone_direction * (position - bone.head_local).dot(bone_direction)
        vertex.co = armature_to_body @ (center_on_bone + (position - center_on_bone) * radial_scale)
    body.data.update()


def main():
    arguments = sys.argv[sys.argv.index("--") + 1:]
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-directory", required=True)
    parser.add_argument("--mode", choices=["preview", "promote"], required=True)
    parser.add_argument("--torso-recline-degrees", type=float, default=CALIBRATED_TORSO_RECLINE_DEGREES)
    options = parser.parse_args(arguments)
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, options.mode).path
    source_path = PROJECT_DIRECTORY / "game/assets/models/pit_crew/racer/source/Racer.fbx"
    chassis_path = PROJECT_DIRECTORY / "game/assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb"
    seat_recline_degrees = measure_seat_recline(chassis_path)
    torso_recline_degrees = options.torso_recline_degrees
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(source_path))
    armature = next(character for character in bpy.data.objects if character.type == "ARMATURE")
    body = bpy.data.objects["Stig4"]
    for scene_object in list(bpy.data.objects):
        if scene_object not in (armature, body):
            bpy.data.objects.remove(scene_object, do_unlink=True)
    bpy.ops.object.select_all(action="SELECT")
    bpy.context.view_layer.objects.active = armature
    armature.scale *= 10.0
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    armature.animation_data_clear()
    for action in list(bpy.data.actions):
        bpy.data.actions.remove(action)
    for pose_bone in armature.pose.bones:
        pose_bone.matrix_basis.identity()
    bpy.context.view_layer.update()
    slim_character_body(armature, body, 0.85)
    create_articulated_gloves(armature, body)
    armature.name = "DriverSkeleton"
    body.name = "DriverBody"
    hips = armature.pose.bones["mixamorig:Hips"]
    hips.rotation_mode = "XYZ"
    hips.rotation_euler.x = math.radians(-torso_recline_degrees)
    neck = armature.pose.bones["mixamorig:Neck"]
    neck.rotation_mode = "XYZ"
    neck.rotation_euler.x = math.radians(torso_recline_degrees)
    bpy.context.view_layer.update()
    for side in ("Left", "Right"):
        move_bone_towards(armature, "mixamorig:" + side + "UpLeg", Vector((0.0, 0.10, 0.45)))
        move_bone_towards(armature, "mixamorig:" + side + "Leg", Vector((0.0, -0.10, 0.42)))
        move_bone_towards(armature, "mixamorig:" + side + "Foot", Vector((0.0, 0.08, 0.10)))
    hips_world = armature.matrix_world @ hips.head
    armature.location -= hips_world
    bpy.context.view_layer.update()
    destination.mkdir(parents=True, exist_ok=True)
    model_path = destination / "driver.glb"
    validate_output_path(PROJECT_DIRECTORY, model_path, options.mode)
    bpy.ops.export_scene.gltf(
        filepath=str(model_path), export_format="GLB", use_selection=True,
        export_animations=False, export_skins=True, export_yup=True,
        export_current_frame=True, export_rest_position_armature=False,
    )
    manifest = {
        "character": "driver", "source": str(source_path.relative_to(PROJECT_DIRECTORY)),
        "source_sha256": hashlib.sha256(source_path.read_bytes()).hexdigest(),
        "model_sha256": hashlib.sha256(model_path.read_bytes()).hexdigest(),
        "bone_count": len(armature.data.bones), "pose": "seated", "separate_finger_bones": True,
        "finger_bones_per_hand": 15, "glove_geometry": "articulated_palm_four_fingers_opposable_thumb",
        "thumb_segments": ["Metacarpal", "Proximal", "Distal"],
        "body_radial_scale": 0.85, "seat_recline_degrees": seat_recline_degrees,
        "torso_recline_degrees": torso_recline_degrees, "head_pitch_degrees": 0.0,
        "seat_model_sha256": hashlib.sha256(chassis_path.read_bytes()).hexdigest(),
    }
    manifest_path = destination / "driver_manifest.json"
    validate_output_path(PROJECT_DIRECTORY, manifest_path, options.mode)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("DRIVER_MODEL=" + str(model_path))


if __name__ == "__main__":
    main()
