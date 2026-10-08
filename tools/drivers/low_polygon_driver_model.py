import hashlib
import json
import math
from pathlib import Path

import bpy
import bmesh
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

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
    "ForeArm": ((0.78, -0.18, 0.53), (0.92, -0.22, 0.28)),
    "Hand": ((0.92, -0.22, 0.28), (1.09, -0.22, -0.17)),
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


def map_limb_position(armature, source_position, side, limb_name):
    side_sign = 1.0 if side == "Left" else -1.0
    source_head, source_tail = (Vector(position) for position in SOURCE_LIMB_JOINTS[limb_name])
    source_head.x *= side_sign
    source_tail.x *= side_sign
    source_basis = make_segment_basis(source_tail - source_head)
    local_position = source_basis.transposed() @ (source_position - source_head)
    target_bone = armature.data.bones["mixamorig:" + side + limb_name]
    if limb_name == "Hand":
        local_position.x *= 0.22
        local_position.y *= 0.27
        local_position.z *= 0.18
        local_position.z += 0.008
        if local_position.y > 0.065:
            curl_angle = min(math.pi * 0.85, (local_position.y - 0.065) * math.pi * 0.85 / 0.085)
            local_position.y = 0.065 + 0.030 * math.sin(curl_angle)
            local_position.z += 0.030 * (1.0 - math.cos(curl_angle))
        return target_bone.matrix_local @ local_position
    if limb_name == "Foot":
        target_direction = (target_bone.tail_local - target_bone.head_local).normalized()
        target_basis = make_segment_basis(armature.matrix_world.to_3x3() @ target_direction)
        local_position *= 0.25
        target_world_position = armature.matrix_world @ target_bone.head_local + target_basis @ local_position
        target_world_position.x -= side_sign * 0.040
        return armature.matrix_world.inverted() @ target_world_position
    local_position.x *= 0.23
    local_position.z *= 0.23
    local_position.y *= target_bone.length / (source_tail - source_head).length
    target_direction = armature.matrix_world.to_3x3() @ (target_bone.tail_local - target_bone.head_local)
    target_basis = make_segment_basis(target_direction)
    target_world_position = armature.matrix_world @ target_bone.head_local + target_basis @ local_position
    if limb_name in ("UpLeg", "Leg"):
        target_world_position.x -= side_sign * 0.040
    return armature.matrix_world.inverted() @ target_world_position


def map_body_position(armature, source_position):
    source_height = source_position.z
    for joint_index in range(len(SOURCE_BODY_JOINTS) - 1):
        lower_height, lower_name = SOURCE_BODY_JOINTS[joint_index]
        upper_height, upper_name = SOURCE_BODY_JOINTS[joint_index + 1]
        if source_height <= upper_height or joint_index == len(SOURCE_BODY_JOINTS) - 2:
            position_progress = (source_height - lower_height) / (upper_height - lower_height)
            progress = min(1.0, max(0.0, position_progress))
            lower_bone = armature.data.bones["mixamorig:" + lower_name]
            upper_bone = armature.data.bones["mixamorig:" + upper_name]
            center = lower_bone.head_local.lerp(upper_bone.head_local, position_progress)
            radial_position = armature.matrix_world.inverted().to_3x3() @ Vector((source_position.x * 0.28, (source_position.y + 0.22) * 0.18, 0.0))
            upper_weight_name = "mixamorig:Spine2" if upper_name == "Neck" else upper_bone.name
            weights = {lower_bone.name: 1.0 - progress}
            weights[upper_weight_name] = weights.get(upper_weight_name, 0.0) + progress
            return center + radial_position, weights
    raise RuntimeError("Driver torso requires a supported source height")


def smooth_joint_progress(value, lower_limit, upper_limit):
    progress = min(1.0, max(0.0, (value - lower_limit) / (upper_limit - lower_limit)))
    return progress * progress * (3.0 - 2.0 * progress)


def map_character_position(armature, source_position, limb_configuration):
    side = "Left" if source_position.x >= 0.0 else "Right"
    if limb_configuration and limb_configuration[1] in ("Hand", "Foot"):
        side, limb_name = limb_configuration
        return map_limb_position(armature, source_position, side, limb_name), {"mixamorig:" + side + limb_name: 1.0}
    if limb_configuration and limb_configuration[1] in ("Arm", "ForeArm"):
        side = limb_configuration[0]
        elbow_progress = smooth_joint_progress(0.53 - source_position.z, -0.16, 0.16)
        upper_position = map_limb_position(armature, source_position, side, "Arm")
        lower_position = map_limb_position(armature, source_position, side, "ForeArm")
        position = upper_position.lerp(lower_position, elbow_progress)
        weights = {"mixamorig:" + side + "Arm": 1.0 - elbow_progress, "mixamorig:" + side + "ForeArm": elbow_progress}
        shoulder_progress = smooth_joint_progress(abs(source_position.x), 0.28, 0.52)
        if source_position.z > 0.90 and shoulder_progress < 1.0:
            body_position, body_weights = map_body_position(armature, source_position)
            position = body_position.lerp(position, shoulder_progress)
            weights = {bone_name: weight * shoulder_progress for bone_name, weight in weights.items()}
            for bone_name, weight in body_weights.items():
                weights[bone_name] = weights.get(bone_name, 0.0) + weight * (1.0 - shoulder_progress)
        return position, weights
    body_position, body_weights = map_body_position(armature, source_position)
    if source_position.z < 0.20:
        knee_progress = smooth_joint_progress(-1.04 - source_position.z, -0.20, 0.20)
        thigh_position = map_limb_position(armature, source_position, side, "UpLeg")
        shin_position = map_limb_position(armature, source_position, side, "Leg")
        leg_position = thigh_position.lerp(shin_position, knee_progress)
        leg_weights = {"mixamorig:" + side + "UpLeg": 1.0 - knee_progress, "mixamorig:" + side + "Leg": knee_progress}
        hip_progress = smooth_joint_progress(source_position.z, -0.25, 0.20)
        weights = {bone_name: weight * (1.0 - hip_progress) for bone_name, weight in leg_weights.items()}
        for bone_name, weight in body_weights.items():
            weights[bone_name] = weights.get(bone_name, 0.0) + weight * hip_progress
        return leg_position.lerp(body_position, hip_progress), weights
    if source_position.z > 0.90 and abs(source_position.x) > 0.28:
        shoulder_progress = smooth_joint_progress(abs(source_position.x), 0.28, 0.52)
        arm_position = map_limb_position(armature, source_position, side, "Arm")
        weights = {bone_name: weight * (1.0 - shoulder_progress) for bone_name, weight in body_weights.items()}
        weights["mixamorig:" + side + "Arm"] = shoulder_progress
        return body_position.lerp(arm_position, shoulder_progress), weights
    return body_position, body_weights


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


def fit_lower_legs_to_cockpit_floor(armature, body, chassis_path, seated_position):
    editable_mesh = bmesh.new()
    editable_mesh.from_mesh(body.data)
    deformation_weights = editable_mesh.verts.layers.deform.active
    lower_leg_groups = {group.index for group in body.vertex_groups if group.name in ("mixamorig:LeftLeg", "mixamorig:RightLeg")}
    lower_leg_edges = [edge for edge in editable_mesh.edges if edge.calc_length() > 0.14 and all(sum(weight for group_index, weight in vertex[deformation_weights].items() if group_index in lower_leg_groups) > 0.4 for vertex in edge.verts)]
    bmesh.ops.subdivide_edges(editable_mesh, edges=lower_leg_edges, cuts=7, use_grid_fill=True)
    editable_mesh.to_mesh(body.data)
    editable_mesh.free()
    body.data.update()
    existing_objects = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(chassis_path))
    interior = bpy.data.objects["GEO_CHASSIS_INTERIOR"]
    floor_surface = BVHTree.FromPolygons([interior.matrix_world @ vertex.co for vertex in interior.data.vertices], [list(polygon.vertices) for polygon in interior.data.polygons])
    for imported_object in set(bpy.data.objects) - existing_objects:
        bpy.data.objects.remove(imported_object, do_unlink=True)
    bpy.context.view_layer.update()
    adjusted_vertices = 0
    maximum_displacement = 0.0
    for vertex in body.data.vertices:
        if not any(body.vertex_groups[group.group].name in ("mixamorig:LeftLeg", "mixamorig:RightLeg") and group.weight > 0.5 for group in vertex.groups):
            continue
        skin_transform = Matrix(((0.0,) * 4,) * 4)
        for group in vertex.groups:
            bone_name = body.vertex_groups[group.group].name
            skin_transform += (armature.pose.bones[bone_name].matrix @ armature.data.bones[bone_name].matrix_local.inverted()) * group.weight
        posed_position = armature.matrix_world @ skin_transform @ vertex.co
        chassis_position = Vector((seated_position[0] - posed_position.x, -seated_position[2] - posed_position.y, posed_position.z + seated_position[1]))
        floor_hit = floor_surface.ray_cast(Vector((chassis_position.x, chassis_position.y, 0.15)), Vector((0.0, 0.0, -1.0)), 0.5)[0]
        if floor_hit is None:
            continue
        displacement = max(0.0, floor_hit.z + 0.010 - chassis_position.z)
        if displacement > 0.0:
            posed_position.z += displacement
            vertex.co = skin_transform.inverted() @ armature.matrix_world.inverted() @ posed_position
            adjusted_vertices += 1
            maximum_displacement = max(maximum_displacement, displacement)
    body.data.update()
    return {"adjusted_vertices": adjusted_vertices, "maximum_displacement_meters": maximum_displacement, "floor_separation_meters": 0.010}


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
        for vertex, source_position in zip(character_object.data.vertices, positions):
            if is_head:
                target_head = armature.data.bones["mixamorig:Head"].head_local
                displacement = Vector((source_position.x * 0.38, (source_position.y + 0.27) * 0.38, (source_position.z - 1.50) * 0.38))
                vertex.co = target_head + armature.matrix_world.inverted().to_3x3() @ displacement
                weights = {"mixamorig:Head": 1.0}
                if piece_name == "DRIVER:HELMET_GLASS_SUB1":
                    visor_positions.append(vertex.co.copy())
            elif piece_name == "DRIVER:COLLARE_HANS1":
                vertex.co, weights = map_body_position(armature, source_position)
                vertex.co += armature.matrix_world.inverted().to_3x3() @ Vector((0.0, -0.075, 0.0))
            else:
                vertex.co, weights = map_character_position(armature, source_position, limb_configuration)
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
    lower_leg_floor_fit = fit_lower_legs_to_cockpit_floor(armature, body, chassis_path, seated_position)
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
    bpy.ops.export_scene.gltf(filepath=str(model_path), export_format="GLB", use_selection=True, export_animations=False, export_skins=True, export_yup=True, export_current_frame=True, export_rest_position_armature=False)
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
    manifest["lower_leg_floor_fit"] = lower_leg_floor_fit
    manifest["seated_position_meters"] = seated_position
    provenance_path = source_directory / "source_provenance.json"
    if provenance_path.is_file():
        manifest["source_provenance"] = json.loads(provenance_path.read_text(encoding="utf-8"))
    manifest_path = validate_output_path(project_directory, destination / "driver_manifest.json", output_mode).path
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("DRIVER_MODEL=" + str(model_path))
