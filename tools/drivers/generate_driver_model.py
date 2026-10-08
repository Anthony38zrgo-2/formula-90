import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
import bmesh
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


def separate_driver_head_and_neck(body):
    head_and_neck_groups = {group.index for group in body.vertex_groups if "Head" in group.name or "Neck" in group.name}
    head_and_neck_faces = set()
    for polygon in body.data.polygons:
        total_weight = sum(group.weight for vertex_index in polygon.vertices for group in body.data.vertices[vertex_index].groups if group.group in head_and_neck_groups)
        if total_weight / len(polygon.vertices) >= 0.5:
            head_and_neck_faces.add(polygon.index)
    head_and_neck = body.copy()
    head_and_neck.data = body.data.copy()
    head_and_neck.name = "DriverHeadAndNeck"
    bpy.context.collection.objects.link(head_and_neck)
    for target_object, keep_head_and_neck in ((body, False), (head_and_neck, True)):
        editable_mesh = bmesh.new()
        editable_mesh.from_mesh(target_object.data)
        editable_mesh.faces.ensure_lookup_table()
        discarded_faces = [face for face in editable_mesh.faces if (face.index in head_and_neck_faces) != keep_head_and_neck]
        bmesh.ops.delete(editable_mesh, geom=discarded_faces, context="FACES")
        editable_mesh.to_mesh(target_object.data)
        editable_mesh.free()
        target_object.data.update()
        target_object.select_set(True)
    return head_and_neck


def create_driver_eye_point(armature, body):
    head_bone = armature.pose.bones["mixamorig:Head"]
    body_to_head = armature.data.bones[head_bone.name].matrix_local.inverted() @ armature.matrix_world.inverted() @ body.matrix_world
    head_group_index = body.vertex_groups["mixamorig:Head"].index
    visor_material_index = next(index for index, material in enumerate(body.data.materials) if material.name == "Blue_Mat")
    visor_vertex_indices = {vertex_index for polygon in body.data.polygons if polygon.material_index == visor_material_index for vertex_index in polygon.vertices}
    visor_positions = [body_to_head @ body.data.vertices[vertex_index].co for vertex_index in visor_vertex_indices if any(group.group == head_group_index and group.weight >= 0.5 for group in body.data.vertices[vertex_index].groups)]
    if not visor_positions:
        raise RuntimeError("Driver helmet requires a visor to locate the eye point")
    minimum_position = Vector(tuple(min(position[axis] for position in visor_positions) for axis in range(3)))
    maximum_position = Vector(tuple(max(position[axis] for position in visor_positions) for axis in range(3)))
    eye_position = (minimum_position + maximum_position) * 0.5
    eye_position.z = maximum_position.z - 0.035
    eye_point = bpy.data.objects.new("DriverEyePoint", None)
    bpy.context.collection.objects.link(eye_point)
    eye_point.parent = armature
    eye_point.parent_type = "BONE"
    eye_point.parent_bone = head_bone.name
    eye_point.matrix_world = armature.matrix_world @ head_bone.matrix @ Matrix.Translation(eye_position) @ Matrix.Rotation(math.pi, 4, "Y") @ Matrix.Rotation(-math.pi * 0.5, 4, "X")
    eye_point.select_set(True)
    return eye_position


def main():
    arguments = sys.argv[sys.argv.index("--") + 1:]
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-directory", required=True)
    parser.add_argument("--mode", choices=["preview", "promote"], required=True)
    parser.add_argument("--torso-recline-degrees", type=float, default=CALIBRATED_TORSO_RECLINE_DEGREES)
    parser.add_argument("--character-model", choices=["original", "low_polygon"], default="low_polygon")
    parser.add_argument("--source-directory", type=Path)
    options = parser.parse_args(arguments)
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, options.mode).path
    if options.character_model == "low_polygon":
        from low_polygon_driver_model import generate_low_polygon_driver
        source_directory = options.source_directory or PROJECT_DIRECTORY / "game/assets/models/drivers/source"
        generate_low_polygon_driver(PROJECT_DIRECTORY, destination, options.mode, source_directory, options.torso_recline_degrees)
        return
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
    eye_position = create_driver_eye_point(armature, body)
    separate_driver_head_and_neck(body)
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
        "separate_head_and_neck": True,
        "eye_point": "DriverEyePoint",
        "eye_position_in_head_space_meters": list(eye_position),
    }
    manifest_path = destination / "driver_manifest.json"
    validate_output_path(PROJECT_DIRECTORY, manifest_path, options.mode)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("DRIVER_MODEL=" + str(model_path))


if __name__ == "__main__":
    main()
