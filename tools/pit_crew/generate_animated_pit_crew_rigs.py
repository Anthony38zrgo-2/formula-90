import hashlib
import json
import math
from pathlib import Path

import bpy
from mathutils import Vector


PROJECT_ROOT = Path(__file__).resolve().parents[2]
PIT_CREW_ASSET_DIRECTORY = PROJECT_ROOT / "game/assets/models/pit_crew/racer"
BASE_CHARACTER_SOURCE_PATH = PIT_CREW_ASSET_DIRECTORY / "source/Racer.fbx"
POSE_DIRECTORY = PIT_CREW_ASSET_DIRECTORY / "poses"
VEHICLE_WHEEL_DIRECTORY = PROJECT_ROOT / "game/assets/models/vehicles/f1-2030"

WHEEL_PATHS = {
    "front_left": VEHICLE_WHEEL_DIRECTORY / "f1_2030_v10_wheel_FL.glb",
    "front_right": VEHICLE_WHEEL_DIRECTORY / "f1_2030_v10_wheel_FR.glb",
    "rear_left": VEHICLE_WHEEL_DIRECTORY / "f1_2030_v10_wheel_RL.glb",
    "rear_right": VEHICLE_WHEEL_DIRECTORY / "f1_2030_v10_wheel_RR.glb",
}

WHEEL_TIMING_OFFSETS = {
    "front_left": 0.00,
    "front_right": 0.06,
    "rear_left": 0.10,
    "rear_right": 0.14,
}

ANIMATION_CLIPS = ("idle_wait", "service_sequence")
CLIP_FRAMES = 31
CLIP_FRAME_RATE = 30
IK_TARGET_SUFFIX = "IkTarget"
LEG_BEND_BONES = {
    "left_upper_leg_bend": "mixamorig:LeftUpLeg",
    "right_upper_leg_bend": "mixamorig:RightUpLeg",
    "left_knee_bend": "mixamorig:LeftLeg",
    "right_knee_bend": "mixamorig:RightLeg",
}

POSE_DEFINITIONS = [
    {"name": "front_left_wheel_carrier", "role": "wheel_carrier", "wheel": "front_left"},
    {"name": "front_left_wheel_change_mechanic", "role": "wheel_change_mechanic", "wheel": "front_left"},
    {"name": "front_right_wheel_carrier", "role": "wheel_carrier", "wheel": "front_right"},
    {"name": "front_right_wheel_change_mechanic", "role": "wheel_change_mechanic", "wheel": "front_right"},
    {"name": "rear_left_wheel_carrier", "role": "wheel_carrier", "wheel": "rear_left"},
    {"name": "rear_left_wheel_change_mechanic", "role": "wheel_change_mechanic", "wheel": "rear_left"},
    {"name": "rear_right_wheel_carrier", "role": "wheel_carrier", "wheel": "rear_right"},
    {"name": "rear_right_wheel_change_mechanic", "role": "wheel_change_mechanic", "wheel": "rear_right"},
    {"name": "front_jack_operator", "role": "jack_operator"},
    {"name": "pit_signaler", "role": "signaler"},
    {"name": "fuel_hose_operator", "role": "fuel_hose_operator"},
]


def standing_rest_pose(definition):
    is_rear_wheel = definition.get("wheel", "").startswith("rear")
    role = definition["role"]
    if role == "wheel_carrier":
        grip_height = 0.89 if is_rear_wheel else 0.92
        return {
            "hips": Vector((0.0, 0.0, 0.0)),
            "spine_lean": 0.05,
            "spine1_lean": 0.05,
            "left_hand": Vector((0.34, -0.45, grip_height + 0.03)),
            "right_hand": Vector((-0.34, -0.45, grip_height + 0.03)),
        }
    if role == "signaler":
        return {
            "hips": Vector((0.0, 0.0, 0.0)),
            "spine_lean": 0.0,
            "spine1_lean": 0.0,
            "left_hand": Vector((0.16, -0.45, 1.83)),
            "right_hand": Vector((0.15, -0.45, 1.30)),
        }
    if role == "fuel_hose_operator":
        return {
            "hips": Vector((0.0, 0.0, 0.0)),
            "spine_lean": 0.15,
            "spine1_lean": 0.02,
            "left_hand": Vector((0.12, -0.55, 1.08)),
            "right_hand": Vector((-0.12, -0.55, 1.08)),
        }
    return {
        "hips": Vector((0.0, 0.0, 0.0)),
        "spine_lean": 0.0,
        "spine1_lean": 0.0,
        "left_hand": Vector((0.25, -0.03, 0.90)),
        "right_hand": Vector((-0.25, -0.03, 0.90)),
    }


def clone_pose(pose):
    return {
        "hips": pose["hips"].copy(),
        "spine_lean": pose["spine_lean"],
        "spine1_lean": pose["spine1_lean"],
        "left_hand": pose["left_hand"].copy(),
        "right_hand": pose["right_hand"].copy(),
        "left_upper_leg_bend": pose.get("left_upper_leg_bend", 0.0),
        "right_upper_leg_bend": pose.get("right_upper_leg_bend", 0.0),
        "left_knee_bend": pose.get("left_knee_bend", 0.0),
        "right_knee_bend": pose.get("right_knee_bend", 0.0),
    }


def pose_with(pose, **changes):
    updated = clone_pose(pose)
    for key, value in changes.items():
        if key in ("hips", "left_hand", "right_hand"):
            updated[key] = Vector(value)
        else:
            updated[key] = value
    return updated


def nudge(pose, horizontal=0.0, vertical=0.0):
    return pose_with(
        pose,
        left_hand=pose["left_hand"] + Vector((horizontal * 0.4, 0.0, vertical)),
        right_hand=pose["right_hand"] + Vector((-horizontal * 0.4, 0.0, vertical)),
    )


def build_wheel_change_mechanic_keys(definition):
    is_rear_wheel = definition["wheel"].startswith("rear")
    working_height = 0.56 if is_rear_wheel else 0.53
    offset = WHEEL_TIMING_OFFSETS[definition["wheel"]]
    rest = standing_rest_pose(definition)
    grip_left = Vector((0.12, -0.60, working_height + 0.07))
    grip_right = Vector((0.08, -0.55, working_height + 0.15))
    crouch_pose = pose_with(
        rest,
        hips=(0.0, -0.20, -0.02),
        spine_lean=0.80,
        spine1_lean=0.35,
        left_upper_leg_bend=0.75,
        right_upper_leg_bend=0.75,
        left_knee_bend=-0.20,
        right_knee_bend=-0.20,
        left_hand=grip_left,
        right_hand=grip_right,
    )
    return [
        (0.00, clone_pose(rest)),
        (0.06 + offset * 0.5, nudge(rest, 0.0, -0.01)),
        (0.12 + offset, pose_with(
            rest,
            hips=(0.0, -0.10, -0.04),
            spine_lean=0.36,
            spine1_lean=0.16,
            left_upper_leg_bend=0.30,
            right_upper_leg_bend=0.30,
            left_knee_bend=-0.10,
            right_knee_bend=-0.10,
            left_hand=Vector((0.22, -0.18, 0.92)),
            right_hand=Vector((-0.20, -0.16, 0.94)),
        )),
        (0.18 + offset, crouch_pose),
        (0.24 + offset * 0.3, pose_with(
            crouch_pose,
            hips=(0.0, -0.21, -0.03),
            left_hand=grip_left + Vector((0.0, -0.08, 0.0)),
            right_hand=grip_right + Vector((0.0, -0.08, 0.0)),
        )),
        (0.32, pose_with(
            crouch_pose,
            hips=(0.0, -0.20, -0.03),
            left_hand=grip_left + Vector((0.0, 0.04, 0.02)),
            right_hand=grip_right + Vector((0.0, 0.02, 0.04)),
        )),
        (0.38, pose_with(
            crouch_pose,
            left_hand=grip_left + Vector((0.0, -0.02, 0.01)),
            right_hand=grip_right + Vector((0.0, 0.05, -0.01)),
        )),
        (0.46, pose_with(
            crouch_pose,
            hips=(0.0, -0.21, -0.03),
            spine_lean=0.42,
            spine1_lean=0.25,
            left_hand=Vector((0.18, -0.48, working_height + 0.10)),
            right_hand=Vector((-0.18, -0.46, working_height + 0.12)),
        )),
        (0.54, pose_with(
            crouch_pose,
            left_hand=grip_left + Vector((0.0, -0.04, 0.0)),
            right_hand=grip_right + Vector((0.0, -0.04, 0.0)),
        )),
        (0.62, pose_with(
            crouch_pose,
            hips=(0.0, -0.20, -0.03),
            left_hand=grip_left + Vector((0.0, -0.08, -0.01)),
            right_hand=grip_right + Vector((0.0, -0.08, 0.02)),
        )),
        (0.70, pose_with(crouch_pose, right_hand=grip_right + Vector((0.0, -0.06, 0.05)))),
        (0.74, pose_with(
            crouch_pose,
            hips=(0.0, -0.19, -0.03),
            spine_lean=0.50,
            right_hand=grip_right + Vector((0.0, -0.03, 0.01)),
        )),
        (0.78, pose_with(
            crouch_pose,
            hips=(0.0, -0.21, -0.03),
            spine_lean=0.60,
            right_hand=grip_right + Vector((0.0, -0.07, 0.06)),
        )),
        (0.84, pose_with(
            crouch_pose,
            hips=(0.0, -0.12, -0.03),
            spine_lean=0.50,
            spine1_lean=0.20,
            left_upper_leg_bend=0.45,
            right_upper_leg_bend=0.45,
            left_knee_bend=-0.15,
            right_knee_bend=-0.15,
            left_hand=Vector((0.14, -0.42, working_height + 0.16)),
            right_hand=Vector((-0.14, -0.40, working_height + 0.18)),
        )),
        (0.90, pose_with(
            rest,
            hips=(0.0, -0.12, -0.02),
            spine_lean=0.20,
            spine1_lean=0.08,
            left_upper_leg_bend=0.35,
            right_upper_leg_bend=0.35,
            left_knee_bend=-0.18,
            right_knee_bend=-0.18,
            left_hand=Vector((0.20, -0.22, 1.00)),
            right_hand=Vector((-0.18, -0.20, 1.00)),
        )),
        (0.96, pose_with(
            rest,
            hips=(0.0, -0.04, -0.01),
            spine_lean=0.08,
            spine1_lean=0.05,
            left_upper_leg_bend=0.10,
            right_upper_leg_bend=0.10,
            left_knee_bend=-0.05,
            right_knee_bend=-0.05,
            left_hand=Vector((0.28, -0.12, 1.00)),
            right_hand=Vector((-0.28, -0.12, 1.00)),
        )),
        (1.00, clone_pose(rest)),
    ]


def build_wheel_carrier_keys(definition):
    is_rear_wheel = definition["wheel"].startswith("rear")
    grip_height = 0.89 if is_rear_wheel else 0.92
    offset = WHEEL_TIMING_OFFSETS[definition["wheel"]]
    rest = standing_rest_pose(definition)
    present_pose = pose_with(
        rest,
        hips=(0.0, -0.03, -0.03),
        spine_lean=0.30,
        spine1_lean=0.18,
        left_hand=Vector((0.32, -0.56, grip_height - 0.02)),
        right_hand=Vector((-0.32, -0.56, grip_height - 0.02)),
    )
    return [
        (0.00, clone_pose(rest)),
        (0.10 + offset, pose_with(rest, hips=(0.0, 0.02, -0.02), spine_lean=0.12)),
        (0.16 + offset, present_pose),
        (0.20, clone_pose(rest)),
        (0.34, nudge(present_pose, -0.02, -0.01)),
        (0.42 + offset, pose_with(
            present_pose,
            hips=(0.0, -0.04, -0.03),
            spine_lean=0.38,
            spine1_lean=0.22,
            left_hand=Vector((0.30, -0.60, grip_height - 0.04)),
            right_hand=Vector((-0.30, -0.60, grip_height - 0.04)),
        )),
        (0.62, pose_with(
            present_pose,
            hips=(0.0, -0.04, -0.03),
            left_hand=Vector((0.31, -0.59, grip_height - 0.03)),
            right_hand=Vector((-0.31, -0.59, grip_height - 0.03)),
        )),
        (0.74, present_pose),
        (0.86, pose_with(
            rest,
            hips=(0.0, -0.02, -0.04),
            spine_lean=0.16,
            left_hand=Vector((0.33, -0.50, grip_height + 0.05)),
            right_hand=Vector((-0.33, -0.50, grip_height + 0.05)),
        )),
        (0.96, clone_pose(rest)),
        (1.00, clone_pose(rest)),
    ]


def build_jack_operator_keys(definition):
    rest = standing_rest_pose(definition)
    grip_pose = pose_with(
        rest,
        hips=(0.0, -0.03, -0.03),
        spine_lean=0.50,
        spine1_lean=0.34,
        left_hand=Vector((0.14, -0.57, 0.66)),
        right_hand=Vector((-0.14, -0.57, 0.66)),
    )
    keys = [
        (0.00, clone_pose(rest)),
        (0.03, pose_with(rest, hips=(0.0, -0.01, -0.02), spine_lean=0.10)),
        (0.06, grip_pose),
    ]
    pump_amplitudes = [0.16, 0.14, 0.12, 0.10, 0.08, 0.06]
    pump_start = 0.08
    for pump_index, amplitude in enumerate(pump_amplitudes):
        mid_time = pump_start + pump_index * 0.045
        keys.append((mid_time, pose_with(
            grip_pose,
            left_hand=Vector((0.14, -0.58, 0.66 - amplitude * 0.5)),
            right_hand=Vector((-0.14, -0.58, 0.66 - amplitude * 0.5)),
        )))
        keys.append((mid_time + 0.022, pose_with(
            grip_pose,
            hips=(0.0, -0.04, -0.10 - amplitude * 0.35),
            spine_lean=0.50 + amplitude * 0.30,
            left_hand=Vector((0.14, -0.59, 0.66 - amplitude)),
            right_hand=Vector((-0.14, -0.59, 0.66 - amplitude)),
        )))
    keys.extend([
        (0.42, pose_with(grip_pose, hips=(0.0, -0.04, -0.04), left_hand=Vector((0.14, -0.59, 0.50)), right_hand=Vector((-0.14, -0.59, 0.50)))),
        (0.55, pose_with(
            grip_pose,
            hips=(0.0, -0.02, -0.03),
            spine_lean=0.30,
            left_hand=Vector((0.14, -0.52, 0.62)),
            right_hand=Vector((-0.14, -0.52, 0.62)),
        )),
        (0.80, pose_with(
            rest,
            hips=(0.0, 0.0, -0.03),
            spine_lean=0.20,
            left_hand=Vector((0.16, -0.44, 0.80)),
            right_hand=Vector((-0.16, -0.44, 0.80)),
        )),
        (0.90, nudge(rest, 0.01, -0.01)),
        (1.00, clone_pose(rest)),
    ])
    return keys


def build_signaler_keys(definition):
    rest = standing_rest_pose(definition)
    return [
        (0.00, clone_pose(rest)),
        (0.25, pose_with(rest, hips=(0.02, 0.0, 0.0), left_hand=Vector((0.20, -0.46, 1.86)), right_hand=Vector((0.19, -0.46, 1.33)))),
        (0.50, pose_with(rest, hips=(0.0, 0.0, -0.01), left_hand=Vector((0.15, -0.44, 1.81)), right_hand=Vector((0.14, -0.44, 1.28)))),
        (0.75, pose_with(rest, hips=(-0.01, 0.0, 0.0), left_hand=Vector((0.17, -0.45, 1.84)), right_hand=Vector((0.16, -0.45, 1.31)))),
        (0.90, pose_with(rest, spine_lean=0.10, left_hand=Vector((0.16, -0.30, 1.55)), right_hand=Vector((0.15, -0.30, 1.15)))),
        (1.00, pose_with(rest, left_hand=Vector((0.24, -0.05, 1.05)), right_hand=Vector((-0.10, -0.05, 0.95)))),
    ]


def build_fuel_hose_operator_keys(definition):
    rest = standing_rest_pose(definition)
    return [
        (0.00, clone_pose(rest)),
        (0.15, pose_with(rest, hips=(0.0, -0.02, -0.02), spine_lean=0.20, left_hand=Vector((0.13, -0.58, 1.06)), right_hand=Vector((-0.13, -0.58, 1.06)))),
        (0.35, pose_with(rest, hips=(0.01, -0.02, -0.03), left_hand=Vector((0.12, -0.56, 1.09)), right_hand=Vector((-0.12, -0.56, 1.08)))),
        (0.55, pose_with(rest, hips=(0.0, -0.03, -0.02), spine_lean=0.18, left_hand=Vector((0.13, -0.59, 1.07)), right_hand=Vector((-0.13, -0.59, 1.07)))),
        (0.75, pose_with(rest, hips=(0.0, -0.02, -0.03), left_hand=Vector((0.12, -0.57, 1.08)), right_hand=Vector((-0.12, -0.57, 1.09)))),
        (1.00, clone_pose(rest)),
    ]


SERVICE_KEY_BUILDERS = {
    "wheel_change_mechanic": build_wheel_change_mechanic_keys,
    "wheel_carrier": build_wheel_carrier_keys,
    "jack_operator": build_jack_operator_keys,
    "signaler": build_signaler_keys,
    "fuel_hose_operator": build_fuel_hose_operator_keys,
}


def build_idle_keys(definition):
    rest = standing_rest_pose(definition)
    return [
        (0.00, clone_pose(rest)),
        (0.30, nudge(rest, 0.01, -0.006)),
        (0.55, pose_with(rest, hips=(0.008, 0.0, -0.012), spine_lean=rest["spine_lean"] + 0.03)),
        (0.80, nudge(rest, -0.01, 0.004)),
        (1.00, clone_pose(rest)),
    ]


def create_ik_rig(armature):
    rig_targets = {}
    for target_name in ("LeftHand", "RightHand"):
        target = bpy.data.objects.new(target_name + IK_TARGET_SUFFIX, None)
        bpy.context.scene.collection.objects.link(target)
        pose_bone = armature.pose.bones["mixamorig:" + target_name]
        constraint = pose_bone.constraints.new("IK")
        constraint.target = target
        constraint.chain_count = 3
        rig_targets[target_name] = target
    return rig_targets


def apply_pose_targets(armature, rig_targets, pose):
    rig_targets["LeftHand"].location = pose["left_hand"]
    rig_targets["RightHand"].location = pose["right_hand"]
    armature.pose.bones["mixamorig:Hips"].location = pose["hips"]
    armature.pose.bones["mixamorig:Spine"].rotation_euler.x = pose["spine_lean"]
    armature.pose.bones["mixamorig:Spine1"].rotation_euler.x = pose["spine1_lean"]
    for pose_property, bone_name in LEG_BEND_BONES.items():
        armature.pose.bones[bone_name].rotation_euler.x = pose.get(pose_property, 0.0)


def keyframe_drivers(armature, rig_targets, keys):
    scene = bpy.context.scene
    scene.frame_set(1)
    bpy.context.view_layer.update()
    for time_fraction, pose in keys:
        frame = 1.0 + time_fraction * (CLIP_FRAMES - 1)
        apply_pose_targets(armature, rig_targets, pose)
        rig_targets["LeftHand"].keyframe_insert("location", frame=frame)
        rig_targets["RightHand"].keyframe_insert("location", frame=frame)
        armature.pose.bones["mixamorig:Hips"].keyframe_insert("location", frame=frame)
        armature.pose.bones["mixamorig:Spine"].keyframe_insert("rotation_euler", index=0, frame=frame)
        armature.pose.bones["mixamorig:Spine1"].keyframe_insert("rotation_euler", index=0, frame=frame)
        for bone_name in LEG_BEND_BONES.values():
            armature.pose.bones[bone_name].keyframe_insert("rotation_euler", index=0, frame=frame)
def capture_bone_bases(armature):
    scene = bpy.context.scene
    captured = {pose_bone.name: [] for pose_bone in armature.pose.bones}
    for frame in range(1, CLIP_FRAMES + 1):
        scene.frame_set(frame)
        depsgraph = bpy.context.evaluated_depsgraph_get()
        bpy.context.view_layer.update()
        evaluated_armature = armature.evaluated_get(depsgraph)
        for pose_bone in evaluated_armature.pose.bones:
            parent_bone = pose_bone.parent
            if parent_bone:
                basis = pose_bone.bone.convert_local_to_pose(
                    pose_bone.matrix,
                    pose_bone.bone.matrix_local,
                    parent_matrix=parent_bone.matrix,
                    parent_matrix_local=parent_bone.bone.matrix_local,
                    invert=True,
                )
            else:
                basis = pose_bone.bone.convert_local_to_pose(
                    pose_bone.matrix,
                    pose_bone.bone.matrix_local,
                    invert=True,
                )
            captured[pose_bone.name].append(basis)
    return captured


def bake_captured_bases(armature, captured, clip_name):
    baked_action = bpy.data.actions.new(clip_name)
    armature.animation_data.action = baked_action
    for pose_bone in armature.pose.bones:
        for constraint in list(pose_bone.constraints):
            pose_bone.constraints.remove(constraint)
        pose_bone.matrix_basis.identity()
    for frame in range(1, CLIP_FRAMES + 1):
        for pose_bone in armature.pose.bones:
            pose_bone.matrix_basis = captured[pose_bone.name][frame - 1]
            pose_bone.keyframe_insert("location", frame=frame)
            pose_bone.keyframe_insert("rotation_euler", frame=frame)
            pose_bone.keyframe_insert("scale", frame=frame)
    bpy.context.scene.frame_set(1)
    bpy.context.view_layer.update()
    return baked_action


def validate_character_geometry(armature, character_body, clip_name):
    for frame in range(1, CLIP_FRAMES + 1):
        bpy.context.scene.frame_set(frame)
        depsgraph = bpy.context.evaluated_depsgraph_get()
        evaluated_armature = armature.evaluated_get(depsgraph)
        evaluated_body = character_body.evaluated_get(depsgraph)
        evaluated_mesh = evaluated_body.to_mesh()
        world_positions = [evaluated_body.matrix_world @ vertex.co for vertex in evaluated_mesh.vertices]
        minimum_position = Vector(tuple(min(position[axis] for position in world_positions) for axis in range(3)))
        maximum_position = Vector(tuple(max(position[axis] for position in world_positions) for axis in range(3)))
        evaluated_body.to_mesh_clear()
        foot_heights = [
            (evaluated_armature.matrix_world @ evaluated_armature.pose.bones["mixamorig:" + side_name + "Foot"].head).z
            for side_name in ("Left", "Right")
        ]
        if not (
            -1.2 <= minimum_position.x <= 0.0
            and 0.0 <= maximum_position.x <= 1.2
            and -1.5 <= minimum_position.y <= 0.0
            and 0.0 <= maximum_position.y <= 1.5
            and -0.06 <= minimum_position.z <= 0.18
            and 1.0 <= maximum_position.z <= 2.3
            and all(-0.06 <= foot_height <= 0.22 for foot_height in foot_heights)
        ):
            raise RuntimeError(
                f"Invalid character geometry in {clip_name} frame {frame}: "
                f"minimum={tuple(minimum_position)} maximum={tuple(maximum_position)} "
                f"foot_heights={foot_heights}"
            )


def create_colored_material(name, color, roughness=0.65):
    material = bpy.data.materials.new(name)
    material.diffuse_color = color
    material.use_nodes = True
    surface_shader = material.node_tree.nodes.get("Principled BSDF")
    surface_shader.inputs["Base Color"].default_value = color
    surface_shader.inputs["Roughness"].default_value = roughness
    return material


def create_cylinder_between(name, first_position, second_position, radius, material):
    start = Vector(first_position)
    end = Vector(second_position)
    direction = end - start
    bpy.ops.mesh.primitive_cylinder_add(vertices=16, radius=radius, depth=direction.length)
    cylinder = bpy.context.object
    cylinder.name = name
    cylinder.location = (start + end) * 0.5
    cylinder.rotation_euler = direction.to_track_quat("Z", "Y").to_euler()
    cylinder.data.materials.append(material)
    return cylinder


def attach_carried_wheel(wheel_name):
    existing_objects = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(WHEEL_PATHS[wheel_name]))
    imported_objects = set(bpy.data.objects) - existing_objects
    for imported_object in imported_objects:
        if imported_object.type == "MESH" and not imported_object.name.endswith(("_RIM_04", "_TIRE")):
            bpy.data.objects.remove(imported_object, do_unlink=True)
    imported_objects = set(bpy.data.objects) - existing_objects
    carried_wheel = bpy.data.objects.new("CarriedWheel", None)
    bpy.context.scene.collection.objects.link(carried_wheel)
    for imported_object in imported_objects:
        if imported_object.parent not in imported_objects:
            original_world_matrix = imported_object.matrix_world.copy()
            imported_object.parent = carried_wheel
            imported_object.matrix_world = original_world_matrix
    carried_wheel.location = (0.0, -0.49, 0.93 if wheel_name.startswith("front") else 0.90)
    carried_wheel.rotation_euler.z = math.pi * 0.5
    bpy.context.view_layer.update()


def attach_wheel_change_tool(definition):
    black_material = create_colored_material("WheelToolBlack", (0.035, 0.04, 0.05, 1.0))
    metallic_material = create_colored_material("WheelToolMetal", (0.32, 0.35, 0.37, 1.0), 0.3)
    height = 0.72 if definition["wheel"].startswith("rear") else 0.69
    create_cylinder_between(
        "WheelChangeToolHandle",
        (0.10, -0.55, height + 0.02),
        (0.10, -0.72, height + 0.02),
        0.045,
        black_material,
    )
    create_cylinder_between(
        "WheelChangeToolSocket",
        (0.10, -0.72, height + 0.02),
        (0.10, -0.81, height + 0.02),
        0.025,
        metallic_material,
    )
    create_cylinder_between(
        "WheelChangeToolGrip",
        (0.10, -0.60, height),
        (0.10, -0.60, height - 0.20),
        0.035,
        black_material,
    )


def attach_jack_handle():
    metallic_material = create_colored_material("JackHandleMetal", (0.22, 0.25, 0.27, 1.0), 0.35)
    create_cylinder_between(
        "JackHandle",
        (0.0, -0.60, 0.78),
        (0.0, -1.04, 0.13),
        0.025,
        metallic_material,
    )


def attach_pit_signal():
    black_material = create_colored_material("SignalHandleBlack", (0.03, 0.035, 0.04, 1.0))
    red_material = create_colored_material("SignalFaceRed", (0.7, 0.025, 0.018, 1.0))
    create_cylinder_between("SignalHandle", (0.15, -0.48, 1.18), (0.15, -0.48, 2.15), 0.025, black_material)
    bpy.ops.mesh.primitive_cylinder_add(vertices=32, radius=0.17, depth=0.025)
    signal_face = bpy.context.object
    signal_face.name = "PitSignalFace"
    signal_face.location = (0.15, -0.48, 2.15)
    signal_face.rotation_euler.x = math.pi * 0.5
    signal_face.data.materials.append(red_material)


def attach_fuel_hose():
    black_material = create_colored_material("FuelHoseBlack", (0.015, 0.018, 0.02, 1.0))
    metallic_material = create_colored_material("FuelNozzleMetal", (0.28, 0.31, 0.33, 1.0), 0.3)
    hose_curve = bpy.data.curves.new("FuelHoseCurve", "CURVE")
    hose_curve.dimensions = "3D"
    hose_curve.bevel_depth = 0.035
    hose_curve.bevel_resolution = 4
    hose_path = hose_curve.splines.new("BEZIER")
    hose_path.bezier_points.add(3)
    positions = [(0.62, 0.15, 0.08), (0.66, -0.28, 0.28), (0.36, -0.72, 0.82), (0.0, -0.58, 1.08)]
    for path_point, position in zip(hose_path.bezier_points, positions):
        path_point.co = position
        path_point.handle_left_type = "AUTO"
        path_point.handle_right_type = "AUTO"
    hose_object = bpy.data.objects.new("FuelHose", hose_curve)
    bpy.context.scene.collection.objects.link(hose_object)
    hose_object.data.materials.append(black_material)
    bpy.context.view_layer.objects.active = hose_object
    for scene_object in bpy.data.objects:
        scene_object.select_set(False)
    hose_object.select_set(True)
    bpy.ops.object.convert(target="MESH")
    hose_object.select_set(False)
    create_cylinder_between("FuelNozzle", (0.0, -0.58, 1.08), (0.0, -0.78, 1.08), 0.055, metallic_material)


def attach_props(definition):
    role = definition["role"]
    if role == "wheel_carrier":
        attach_carried_wheel(definition["wheel"])
    elif role == "wheel_change_mechanic":
        attach_wheel_change_tool(definition)
    elif role == "jack_operator":
        attach_jack_handle()
    elif role == "signaler":
        attach_pit_signal()
    elif role == "fuel_hose_operator":
        attach_fuel_hose()


def prepare_armature():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(BASE_CHARACTER_SOURCE_PATH))
    armature = next(character for character in bpy.data.objects if character.type == "ARMATURE")
    character_body = next(character for character in bpy.data.objects if character.name == "Stig4")
    bpy.ops.object.select_all(action="DESELECT")
    armature.select_set(True)
    character_body.select_set(True)
    bpy.context.view_layer.objects.active = armature
    armature.scale *= 10.0
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    for existing_action in list(bpy.data.actions):
        bpy.data.actions.remove(existing_action)
    armature.animation_data_clear()
    for pose_bone in armature.pose.bones:
        pose_bone.rotation_mode = "XYZ"
        pose_bone.matrix_basis.identity()
    scene = bpy.context.scene
    scene.frame_start = 1
    scene.frame_end = CLIP_FRAMES
    scene.render.fps = CLIP_FRAME_RATE
    return armature, character_body


def keyframe_driver_clip(armature, rig_targets, definition, clip_name):
    for target_object in rig_targets.values():
        if hasattr(target_object, "animation_data"):
            target_object.animation_data_clear()
    driver_action = bpy.data.actions.new(clip_name + "Driver")
    armature.animation_data_create()
    armature.animation_data.action = driver_action
    keys = build_idle_keys(definition) if clip_name == "idle_wait" else SERVICE_KEY_BUILDERS[definition["role"]](definition)
    keyframe_drivers(armature, rig_targets, keys)
    captured = capture_bone_bases(armature)
    bpy.data.actions.remove(driver_action)
    return captured


def export_rigged_member(definition):
    armature, character_body = prepare_armature()
    rig_targets = create_ik_rig(armature)
    captured_clips = {}
    for clip_name in ANIMATION_CLIPS:
        captured_clips[clip_name] = keyframe_driver_clip(armature, rig_targets, definition, clip_name)
    for clip_name in ANIMATION_CLIPS:
        bake_captured_bases(armature, captured_clips[clip_name], clip_name)
        validate_character_geometry(armature, character_body, clip_name)
    armature.animation_data.action = bpy.data.actions[ANIMATION_CLIPS[1]]
    attach_props(definition)
    for driver_object in list(bpy.data.objects):
        if driver_object.name.endswith(IK_TARGET_SUFFIX):
            bpy.data.objects.remove(driver_object, do_unlink=True)
    bpy.ops.object.select_all(action="DESELECT")
    for scene_object in bpy.data.objects:
        if scene_object.type in {"MESH", "ARMATURE"} or scene_object.name == "CarriedWheel":
            scene_object.select_set(True)
    bpy.context.view_layer.objects.active = armature
    bpy.ops.export_scene.gltf(
        filepath=str(POSE_DIRECTORY / (definition["name"] + ".glb")),
        export_format="GLB",
        use_selection=True,
        export_animations=True,
        export_skins=True,
        export_yup=True,
    )


def write_manifest():
    manifest = {
        "base_character": BASE_CHARACTER_SOURCE_PATH.name,
        "character_source": "racer.zip/source/Racer.rar/Racer.fbx",
        "character_source_sha256": hashlib.sha256(BASE_CHARACTER_SOURCE_PATH.read_bytes()).hexdigest(),
        "vehicle": "f1_2030_v10",
        "track_first_use": "fuji76_77",
        "rigged": True,
        "animation_clips": list(ANIMATION_CLIPS),
        "clip_seconds": 1.0,
        "members": POSE_DEFINITIONS,
        "wheel_sources": {position: path.name for position, path in WHEEL_PATHS.items()},
        "wheel_source_sha256": {
            position: hashlib.sha256(path.read_bytes()).hexdigest()
            for position, path in WHEEL_PATHS.items()
        },
    }
    manifest_path = PIT_CREW_ASSET_DIRECTORY / "pose_manifest.json"
    with manifest_path.open("w", encoding="utf-8", newline="\n") as manifest_file:
        manifest_file.write(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")


def main():
    POSE_DIRECTORY.mkdir(parents=True, exist_ok=True)
    for definition in POSE_DEFINITIONS:
        export_rigged_member(definition)
    write_manifest()


if __name__ == "__main__":
    main()
