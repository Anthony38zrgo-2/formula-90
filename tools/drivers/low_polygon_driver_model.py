import hashlib
import json
import math
import shutil
import tempfile
from pathlib import Path

import bpy
from mathutils import Matrix, Vector
from mathutils.kdtree import KDTree

from common.output_policy import validate_output_path


SOURCE_BODY_JOINTS = (
    (0.10, "Hips"),
    (0.48, "Spine"),
    (0.85, "Spine1"),
    (1.22, "Spine2"),
    (1.52, "Neck"),
)
SOURCE_PIECE_NAMES = {
    "DRIVER:polymsh_detached1": ("Left", "Arm"),
    "DRIVER:polymsh_detached5": ("Right", "Arm"),
    "DRIVER:polymsh_extracted": ("Left", "ForeArm"),
    "DRIVER:polymsh_extracted4": ("Right", "ForeArm"),
    "DRIVER:Driver_Body": ("Left", "UpLeg"),
    "DRIVER:Driver_Body1": ("Right", "UpLeg"),
    "DRIVER:polymsh_extracted8": ("Left", "Leg"),
    "DRIVER:polymsh_extracted6": ("Right", "Leg"),
    "DRIVER:polymsh_extracted9": ("Left", "Foot"),
    "DRIVER:polymsh_extracted7": ("Right", "Foot"),
    "DRIVER:polymsh_extracted3": ("Left", "Hand"),
    "DRIVER:polymsh_extracted5": ("Right", "Hand"),
}
SOURCE_LIMB_JOINTS = {
    "Arm": ((0.43, -0.16, 1.28), (0.78, -0.18, 0.53)),
    "ForeArm": ((0.78, -0.18, 0.53), (0.98, -0.22, 0.10)),
    "Hand": ((0.98, -0.22, 0.10), (1.09, -0.22, -0.20)),
    "UpLeg": ((0.22, -0.22, 0.02), (0.26, -0.22, -1.04)),
    "Leg": ((0.26, -0.22, -1.04), (0.27, -0.22, -1.96)),
    "Foot": ((0.27, -0.22, -1.96), (0.27, -0.49, -2.09)),
}


def build_reference_skeleton(project_directory, destination, output_mode):
    from articulated_driver_gloves import create_articulated_gloves

    character_path = project_directory / "game/assets/models/pit_crew/racer/source/Racer.fbx"
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(character_path))
    armature = next(character for character in bpy.data.objects if character.type == "ARMATURE")
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.select_all(action="SELECT")
    armature.scale *= 10.0
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    armature.animation_data_clear()
    for pose_bone in armature.pose.bones:
        pose_bone.matrix_basis.identity()
    bpy.context.view_layer.update()
    create_articulated_gloves(armature, bpy.data.objects["Stig4"])
    for character_object in list(bpy.data.objects):
        if character_object != armature:
            bpy.data.objects.remove(character_object, do_unlink=True)
    armature.name = "DriverSkeleton"
    skeleton_path = validate_output_path(project_directory, destination / "reference_skeleton.blend", output_mode).path
    bpy.ops.wm.save_as_mainfile(filepath=str(skeleton_path))
    return skeleton_path


def make_segment_basis(direction):
    longitudinal_direction = direction.normalized()
    front_direction = Vector((0.0, -1.0, 0.0))
    lateral_direction = longitudinal_direction.cross(front_direction).normalized()
    front_direction = lateral_direction.cross(longitudinal_direction).normalized()
    return Matrix((lateral_direction, longitudinal_direction, front_direction)).transposed()


def adapt_skeleton_to_source_anatomy(armature, uniform_scale):
    source_origin = Vector((0.0, -0.22, 0.10))
    inverse_transform = armature.matrix_world.inverted()
    def joint_position(position):
        return inverse_transform @ ((Vector(position) - source_origin) * uniform_scale)
    anatomical_segments = {
        "Hips": ((0.0, -0.22, 0.10), (0.0, -0.22, 0.48)),
        "Spine": ((0.0, -0.22, 0.48), (0.0, -0.22, 0.85)),
        "Spine1": ((0.0, -0.22, 0.85), (0.0, -0.22, 1.22)),
        "Spine2": ((0.0, -0.22, 1.22), (0.0, -0.22, 1.42)),
        "Neck": ((0.0, -0.22, 1.42), (0.0, -0.27, 1.52)),
        "Head": ((0.0, -0.27, 1.52), (0.0, -0.27, 2.10)),
    }
    for side in ("Left", "Right"):
        side_sign = 1.0 if side == "Left" else -1.0
        anatomical_segments[side + "Shoulder"] = ((0.0, -0.16, 1.28), (side_sign * 0.43, -0.16, 1.28))
        for limb_name, segment in SOURCE_LIMB_JOINTS.items():
            anatomical_segments[side + limb_name] = tuple((side_sign * position[0], position[1], position[2]) for position in segment)
    previous_hand_transforms = {side: armature.data.bones["mixamorig:" + side + "Hand"].matrix_local.copy() for side in ("Left", "Right")}
    previous_finger_positions = {bone.name: (bone.head_local.copy(), bone.tail_local.copy()) for bone in armature.data.bones if any(token in bone.name for token in ("Thumb", "Index", "Middle", "Ring", "Little", "Pinky"))}
    bpy.context.view_layer.objects.active = armature
    armature.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    for bone_name, segment in anatomical_segments.items():
        bone = armature.data.edit_bones["mixamorig:" + bone_name]
        bone.use_connect = False
        bone.head = joint_position(segment[0])
        bone.tail = joint_position(segment[1])
        bone.align_roll(inverse_transform.to_3x3() @ Vector((0.0, -1.0, 0.0)))
    for bone_name, positions in previous_finger_positions.items():
        side = "Left" if "Left" in bone_name else "Right"
        hand = armature.data.edit_bones["mixamorig:" + side + "Hand"]
        previous_inverse = previous_hand_transforms[side].inverted()
        bone = armature.data.edit_bones[bone_name]
        bone.use_connect = False
        bone.head = hand.matrix @ (previous_inverse @ positions[0])
        bone.tail = hand.matrix @ (previous_inverse @ positions[1])
    bpy.ops.object.mode_set(mode="OBJECT")
    for bone in armature.pose.bones:
        bone.matrix_basis.identity()
    return source_origin


def smooth_joint_progress(value, lower_limit, upper_limit):
    progress = min(1.0, max(0.0, (value - lower_limit) / (upper_limit - lower_limit)))
    return progress * progress * (3.0 - 2.0 * progress)


def source_anatomical_weights(source_position, limb_configuration):
    side = "Left" if source_position.x >= 0.0 else "Right"
    if limb_configuration:
        side, limb_name = limb_configuration
        if limb_name in ("Hand", "Foot"):
            return {"mixamorig:" + side + limb_name: 1.0}
        if limb_name in ("Arm", "ForeArm"):
            progress = smooth_joint_progress(0.53 - source_position.z, -0.10, 0.10)
            return {"mixamorig:" + side + "Arm": 1.0 - progress, "mixamorig:" + side + "ForeArm": progress}
        progress = smooth_joint_progress(-1.04 - source_position.z, -0.13, 0.13)
        return {"mixamorig:" + side + "UpLeg": 1.0 - progress, "mixamorig:" + side + "Leg": progress}
    for joint_index in range(len(SOURCE_BODY_JOINTS) - 1):
        lower_height, lower_name = SOURCE_BODY_JOINTS[joint_index]
        upper_height, upper_name = SOURCE_BODY_JOINTS[joint_index + 1]
        if source_position.z <= upper_height:
            progress = smooth_joint_progress(source_position.z, lower_height, upper_height)
            upper_name = "Spine2" if upper_name == "Neck" else upper_name
            if lower_name == upper_name:
                return {"mixamorig:" + lower_name: 1.0}
            return {"mixamorig:" + lower_name: 1.0 - progress, "mixamorig:" + upper_name: progress}
    return {"mixamorig:Spine2": 1.0}


def prepare_materials(source_directory):
    material_images = {node.image for material in bpy.data.materials if material.node_tree for node in material.node_tree.nodes if node.type == "TEX_IMAGE" and node.image}
    for character_image in material_images:
        texture_name = Path(character_image.name).stem + ".png"
        texture_path = source_directory / "textures" / texture_name
        if not texture_path.is_file():
            raise RuntimeError("Driver texture is missing: " + str(texture_path))
        if character_image.packed_file:
            character_image.unpack(method="REMOVE")
        character_image.filepath = str(texture_path)
        character_image.reload()
        character_image.pack()
    for material in bpy.data.materials:
        surface = next((node for node in material.node_tree.nodes if node.type == "BSDF_PRINCIPLED"), None) if material.node_tree else None
        if surface is None:
            continue
        surface.inputs["Roughness"].default_value = 0.75
        if material.name == "RT_HELMET_Glass":
            surface.inputs["Alpha"].default_value = 0.32
            surface.inputs["Roughness"].default_value = 0.18
            surface.inputs["Metallic"].default_value = 0.15


def verify_original_uniform_geometry(source_path, uniform_scale, source_origin):
    bpy.context.view_layer.update()
    prepared_positions = [character.matrix_world @ vertex.co for character in bpy.data.objects if character.type == "MESH" for vertex in character.data.vertices]
    prepared_triangle_count = 0
    for character in bpy.data.objects:
        if character.type == "MESH":
            character.data.calc_loop_triangles()
            prepared_triangle_count += len(character.data.loop_triangles)
    with tempfile.TemporaryDirectory(prefix="driver_original_geometry_") as reference_directory:
        reference_path = Path(reference_directory) / "original_driver.blend"
        shutil.copyfile(source_path, reference_path)
        with bpy.data.libraries.load(str(reference_path), link=False) as (available_data, loaded_data):
            loaded_data.objects = available_data.objects
    original_objects = [character for character in loaded_data.objects if character]
    for character in original_objects:
        bpy.context.collection.objects.link(character)
    bpy.context.view_layer.update()
    original_positions = [(character.matrix_world @ vertex.co - source_origin) * uniform_scale for character in original_objects if character.type == "MESH" for vertex in character.data.vertices]
    original_triangle_count = 0
    for character in original_objects:
        if character.type == "MESH":
            character.data.calc_loop_triangles()
            original_triangle_count += len(character.data.loop_triangles)
    maximum_error = 0.0
    for reference_positions, measured_positions in ((original_positions, prepared_positions), (prepared_positions, original_positions)):
        reference_tree = KDTree(len(reference_positions))
        for position_index, position in enumerate(reference_positions):
            reference_tree.insert(position, position_index)
        reference_tree.balance()
        maximum_error = max(maximum_error, max(reference_tree.find(position)[2] for position in measured_positions))
    for character in original_objects:
        bpy.data.objects.remove(character, do_unlink=True)
    if len(prepared_positions) != len(original_positions) or prepared_triangle_count != original_triangle_count or maximum_error > 0.000001:
        raise RuntimeError("Prepared driver no longer preserves the uniformly scaled original geometry: " + str((len(prepared_positions), len(original_positions), prepared_triangle_count, original_triangle_count, maximum_error)))
    geometry_digest = hashlib.sha256(json.dumps(sorted(tuple(round(coordinate, 6) for coordinate in position) for position in prepared_positions)).encode()).hexdigest()
    return {"verified": True, "vertex_count": len(prepared_positions), "triangle_count": prepared_triangle_count, "maximum_error_meters": maximum_error, "neutral_geometry_sha256": geometry_digest}


def join_character_parts(parts, name, armature):
    bpy.ops.object.select_all(action="DESELECT")
    for character_object in parts:
        character_object.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.join()
    character_object = bpy.context.object
    character_object.name = name
    character_object.parent = armature
    character_object.matrix_parent_inverse = Matrix.Identity(4)
    character_object.matrix_basis = Matrix.Identity(4)
    modifier = character_object.modifiers.new("DriverSkeletalDeformation", "ARMATURE")
    modifier.object = armature
    return character_object


def generate_low_polygon_driver(project_directory, destination, output_mode, source_directory, torso_recline_degrees):
    from generate_driver_model import move_bone_towards, measure_seat_recline

    source_directory = Path(source_directory).resolve()
    source_path = source_directory / "source/driver.blend"
    chassis_path = project_directory / "game/assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb"
    vehicle_scene_source = (project_directory / "game/scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn").read_text(encoding="utf-8")
    seated_position_property = next(line for line in vehicle_scene_source.splitlines() if line.startswith("seated_position = Vector3("))
    seated_position = tuple(float(coordinate) for coordinate in seated_position_property.split("(", 1)[1].rstrip(")").split(","))
    seat_recline_degrees = measure_seat_recline(chassis_path)
    destination.mkdir(parents=True, exist_ok=True)
    skeleton_path = source_directory / "reference_skeleton.blend"
    if not skeleton_path.is_file():
        skeleton_path = destination / "reference_skeleton.blend"
        if not skeleton_path.is_file():
            skeleton_path = build_reference_skeleton(project_directory, destination, output_mode)
    bpy.ops.wm.open_mainfile(filepath=str(source_path), load_ui=False, use_scripts=False)
    prepare_materials(source_directory)
    with bpy.data.libraries.load(str(skeleton_path), link=False) as (available_data, loaded_data):
        loaded_data.objects = [name for name in available_data.objects if name == "DriverSkeleton"]
    armature = loaded_data.objects[0]
    bpy.context.collection.objects.link(armature)
    bpy.context.view_layer.update()
    original_positions = [character.matrix_world @ vertex.co for character in bpy.data.objects if character.type == "MESH" for vertex in character.data.vertices]
    original_height = max(position.z for position in original_positions) - min(position.z for position in original_positions)
    uniform_scale = 1.75 / original_height
    source_origin = adapt_skeleton_to_source_anatomy(armature, uniform_scale)
    source_triangle_count = 0
    maximum_uniform_scale_error = 0.0
    body_parts = []
    head_parts = []
    glove_parts = []
    visor_positions = []
    for character_object in list(bpy.data.objects):
        if character_object.type != "MESH":
            if character_object != armature:
                bpy.data.objects.remove(character_object, do_unlink=True)
            continue
        piece_name = character_object.name
        positions = [character_object.matrix_world @ vertex.co for vertex in character_object.data.vertices]
        character_object.matrix_world = Matrix.Identity(4)
        is_head = piece_name == "DRIVER:head" or "HELMET" in piece_name
        limb_configuration = SOURCE_PIECE_NAMES.get(piece_name)
        character_object.data.calc_loop_triangles()
        source_triangle_count += len(character_object.data.loop_triangles)
        for vertex, source_position in zip(character_object.data.vertices, positions):
            expected_position = (source_position - source_origin) * uniform_scale
            vertex.co = armature.matrix_world.inverted() @ expected_position
            maximum_uniform_scale_error = max(maximum_uniform_scale_error, (armature.matrix_world @ vertex.co - expected_position).length)
            if is_head:
                weights = {"mixamorig:Head": 1.0}
                if piece_name == "DRIVER:HELMET_GLASS_SUB1":
                    visor_positions.append(vertex.co.copy())
            elif piece_name == "DRIVER:COLLARE_HANS1":
                weights = {"mixamorig:Spine2": 1.0}
            else:
                weights = source_anatomical_weights(source_position, limb_configuration)
            for bone_name, weight in weights.items():
                if weight > 0.00001:
                    vertex_group = character_object.vertex_groups.get(bone_name) or character_object.vertex_groups.new(name=bone_name)
                    vertex_group.add([vertex.index], weight, "REPLACE")
        for extra_layer in list(character_object.data.uv_layers)[1:]:
            character_object.data.uv_layers.remove(extra_layer)
        character_object.data.uv_layers[0].name = "DriverTextureCoordinates"
        for polygon in character_object.data.polygons:
            polygon.use_smooth = True
        if is_head:
            head_parts.append(character_object)
        elif limb_configuration and limb_configuration[1] == "Hand":
            glove_parts.append(character_object)
        else:
            body_parts.append(character_object)
    body = join_character_parts(body_parts, "DriverBody", armature)
    head = join_character_parts(head_parts, "DriverHeadAndNeck", armature)
    gloves = join_character_parts(glove_parts, "DriverArticulatedGloves", armature)
    original_geometry_verification = verify_original_uniform_geometry(source_path, uniform_scale, source_origin)
    geometry_contract = bpy.data.objects.new("DriverOriginalUniformGeometryContract", None)
    bpy.context.collection.objects.link(geometry_contract)
    geometry_contract.parent = armature
    geometry_contract["contract_version"] = 1
    geometry_contract["source_sha256"] = hashlib.sha256(source_path.read_bytes()).hexdigest()
    geometry_contract["neutral_geometry_sha256"] = original_geometry_verification["neutral_geometry_sha256"]
    geometry_contract["uniform_scale"] = uniform_scale
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
    armature.location -= armature.matrix_world @ hips.head
    bpy.context.view_layer.update()
    lower_leg_floor_fit = {"adjusted_vertices": 0, "maximum_displacement_meters": 0.0, "method": "skeletal_pose_without_geometry_edits"}
    head_bone = armature.pose.bones["mixamorig:Head"]
    head_inverse = armature.data.bones[head_bone.name].matrix_local.inverted()
    local_visor_positions = [head_inverse @ position for position in visor_positions]
    minimum_position = Vector(tuple(min(position[axis] for position in local_visor_positions) for axis in range(3)))
    maximum_position = Vector(tuple(max(position[axis] for position in local_visor_positions) for axis in range(3)))
    eye_position = (minimum_position + maximum_position) * 0.5
    eye_position.z = maximum_position.z - 0.035
    eye_point = bpy.data.objects.new("DriverEyePoint", None)
    bpy.context.collection.objects.link(eye_point)
    eye_point.parent = armature
    eye_point.parent_type = "BONE"
    eye_point.parent_bone = head_bone.name
    eye_point.matrix_world = armature.matrix_world @ head_bone.matrix @ Matrix.Translation(eye_position) @ Matrix.Rotation(math.pi, 4, "Y") @ Matrix.Rotation(-math.pi * 0.5, 4, "X")
    bpy.ops.object.select_all(action="SELECT")
    prepared_source_directory = destination / "source" if output_mode == "promote" else destination
    validate_output_path(project_directory, prepared_source_directory, output_mode)
    prepared_source_directory.mkdir(parents=True, exist_ok=True)
    prepared_source_path = validate_output_path(project_directory, prepared_source_directory / "prepared_driver.blend", output_mode).path
    bpy.data.orphans_purge(do_local_ids=True, do_linked_ids=True, do_recursive=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(prepared_source_path))
    model_path = validate_output_path(project_directory, destination / "driver.glb", output_mode).path
    bpy.ops.export_scene.gltf(filepath=str(model_path), export_format="GLB", use_selection=True, export_animations=False, export_skins=True, export_yup=True, export_current_frame=True, export_rest_position_armature=False, export_extras=True)
    triangle_count = 0
    for character_object in (body, head, gloves):
        character_object.data.calc_loop_triangles()
        triangle_count += len(character_object.data.loop_triangles)
    source_files = {"source/driver.blend": hashlib.sha256(source_path.read_bytes()).hexdigest(), "reference_skeleton.blend": hashlib.sha256(skeleton_path.read_bytes()).hexdigest()}
    source_files.update({"textures/" + texture_path.name: hashlib.sha256(texture_path.read_bytes()).hexdigest() for texture_path in sorted((source_directory / "textures").glob("*.png"))})
    manifest = {
        "character": "low_polygon_race_car_driver",
        "source": "game/assets/models/drivers/source/source/driver.blend",
        "source_sha256": source_files["source/driver.blend"],
        "source_files": source_files,
        "model_sha256": hashlib.sha256(model_path.read_bytes()).hexdigest(),
        "bone_count": len(armature.data.bones),
        "triangle_count": triangle_count,
        "pose": "seated",
        "hand_geometry": "authored_fixed_grip_following_wrist",
        "finger_bones_per_hand": 15,
        "finger_geometry_deformation": False,
        "torso_recline_degrees": torso_recline_degrees,
        "seat_recline_degrees": seat_recline_degrees,
        "seat_model_sha256": hashlib.sha256(chassis_path.read_bytes()).hexdigest(),
        "separate_head_and_neck": True,
        "eye_point": "DriverEyePoint",
        "eye_position_in_head_space_meters": list(eye_position),
    }
    if triangle_count != source_triangle_count or maximum_uniform_scale_error > 0.000001:
        raise RuntimeError("Uniform original geometry fidelity verification failed")
    manifest["uniform_source_geometry"] = {
        "uniform_scale": uniform_scale,
        "neutral_height_meters": 1.75,
        "source_origin": list(source_origin),
        "maximum_position_error_meters": maximum_uniform_scale_error,
        "source_triangle_count": source_triangle_count,
        "preserved_triangle_count": triangle_count,
        "individual_part_scaling": False,
        "geometry_sculpting": False,
        "skeleton_adapted_to_original_anatomy": True,
    }
    manifest["original_geometry_verification"] = original_geometry_verification
    manifest["geometry_contract_version"] = 1
    manifest["prepared_source_sha256"] = hashlib.sha256(prepared_source_path.read_bytes()).hexdigest()
    manifest["lower_leg_floor_fit"] = lower_leg_floor_fit
    manifest["seated_position_meters"] = seated_position
    provenance_path = source_directory / "source_provenance.json"
    if provenance_path.is_file():
        manifest["source_provenance"] = json.loads(provenance_path.read_text(encoding="utf-8"))
    manifest_path = validate_output_path(project_directory, destination / "driver_manifest.json", output_mode).path
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("DRIVER_MODEL=" + str(model_path))
