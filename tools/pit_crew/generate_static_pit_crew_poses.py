import hashlib
import json
import math
from pathlib import Path

import bpy
from mathutils import Matrix, Vector


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


def create_position_target(name, position):
    target = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(target)
    target.location = Vector(position)
    return target


def constrain_bone_to_position(armature, bone_name, position, chain_length):
    target = create_position_target(bone_name + "Target", position)
    constraint = armature.pose.bones["mixamorig:" + bone_name].constraints.new("IK")
    constraint.target = target
    constraint.chain_count = chain_length


def configure_character_pose(armature, definition):
    for pose_bone in armature.pose.bones:
        pose_bone.matrix_basis.identity()

    role = definition["role"]
    wheel_position = definition.get("wheel", "")
    is_rear_wheel = wheel_position.startswith("rear")

    if role in {"wheel_change_mechanic", "jack_operator"}:
        armature.pose.bones["mixamorig:Spine"].rotation_mode = "XYZ"
        armature.pose.bones["mixamorig:Spine"].rotation_euler.x = 0.60
        armature.pose.bones["mixamorig:Spine1"].rotation_mode = "XYZ"
        armature.pose.bones["mixamorig:Spine1"].rotation_euler.x = 0.35

    if role == "wheel_carrier":
        height = 0.89 if is_rear_wheel else 0.92
        constrain_bone_to_position(armature, "LeftHand", (0.34, -0.49, height), 3)
        constrain_bone_to_position(armature, "RightHand", (-0.34, -0.49, height), 3)
    elif role == "wheel_change_mechanic":
        working_height = 0.56 if is_rear_wheel else 0.53
        constrain_bone_to_position(armature, "LeftHand", (0.12, -0.62, working_height), 3)
        constrain_bone_to_position(armature, "RightHand", (-0.12, -0.56, working_height + 0.08), 3)
    elif role == "jack_operator":
        constrain_bone_to_position(armature, "LeftHand", (0.14, -0.57, 0.55), 3)
        constrain_bone_to_position(armature, "RightHand", (-0.14, -0.57, 0.55), 3)
    elif role == "signaler":
        constrain_bone_to_position(armature, "LeftHand", (0.16, -0.45, 1.83), 3)
        constrain_bone_to_position(armature, "RightHand", (0.15, -0.45, 1.30), 3)
    elif role == "fuel_hose_operator":
        armature.pose.bones["mixamorig:Spine"].rotation_mode = "XYZ"
        armature.pose.bones["mixamorig:Spine"].rotation_euler.x = 0.15
        constrain_bone_to_position(armature, "LeftHand", (0.12, -0.55, 1.08), 3)
        constrain_bone_to_position(armature, "RightHand", (-0.12, -0.55, 1.08), 3)


def freeze_character_body(character_body):
    bpy.context.view_layer.update()
    evaluation_context = bpy.context.evaluated_depsgraph_get()
    evaluated_character = character_body.evaluated_get(evaluation_context)
    posed_mesh = bpy.data.meshes.new_from_object(
        evaluated_character,
        preserve_all_data_layers=True,
        depsgraph=evaluation_context,
    )
    posed_body = bpy.data.objects.new("PitCrewBody", posed_mesh)
    bpy.context.scene.collection.objects.link(posed_body)
    posed_body.matrix_world = character_body.matrix_world.copy()
    bpy.context.view_layer.update()
    posed_body.data.transform(posed_body.matrix_world)
    posed_body.matrix_world = Matrix.Identity(4)

    for existing_object in list(bpy.data.objects):
        if existing_object != posed_body:
            bpy.data.objects.remove(existing_object, do_unlink=True)

    return posed_body


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


def attach_carried_wheel(wheel_position):
    existing_objects = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(WHEEL_PATHS[wheel_position]))
    imported_objects = set(bpy.data.objects) - existing_objects
    carried_wheel = bpy.data.objects.new("CarriedWheel", None)
    bpy.context.scene.collection.objects.link(carried_wheel)
    for imported_object in imported_objects:
        if imported_object.parent not in imported_objects:
            original_world_matrix = imported_object.matrix_world.copy()
            imported_object.parent = carried_wheel
            imported_object.matrix_world = original_world_matrix
    carried_wheel.location = (0.0, -0.49, 0.93 if wheel_position.startswith("front") else 0.90)
    carried_wheel.rotation_euler.z = math.pi * 0.5


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
    hose_object.select_set(True)
    bpy.ops.object.convert(target="MESH")
    hose_object.select_set(False)
    create_cylinder_between("FuelNozzle", (0.0, -0.58, 1.08), (0.0, -0.78, 1.08), 0.055, metallic_material)


def export_pose(definition):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=str(BASE_CHARACTER_SOURCE_PATH))
    armature = next(character for character in bpy.data.objects if character.type == "ARMATURE")
    character_body = next(character for character in bpy.data.objects if character.name == "Stig4")
    armature.animation_data_clear()
    armature.scale *= 10.0
    configure_character_pose(armature, definition)
    freeze_character_body(character_body)

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

    for scene_object in bpy.data.objects:
        scene_object.select_set(True)
    bpy.ops.export_scene.gltf(
        filepath=str(POSE_DIRECTORY / (definition["name"] + ".glb")),
        export_format="GLB",
        use_selection=True,
        export_animations=False,
    )


def main():
    POSE_DIRECTORY.mkdir(parents=True, exist_ok=True)
    for definition in POSE_DEFINITIONS:
        export_pose(definition)
    manifest = {
        "base_character": BASE_CHARACTER_SOURCE_PATH.name,
        "character_source": "racer.zip/source/Racer.rar/Racer.fbx",
        "character_source_sha256": hashlib.sha256(BASE_CHARACTER_SOURCE_PATH.read_bytes()).hexdigest(),
        "vehicle": "f1_2030_v10",
        "track_first_use": "fuji76_77",
        "members": POSE_DEFINITIONS,
        "wheel_sources": {position: path.name for position, path in WHEEL_PATHS.items()},
        "wheel_source_sha256": {
            position: hashlib.sha256(path.read_bytes()).hexdigest()
            for position, path in WHEEL_PATHS.items()
        },
    }
    (PIT_CREW_ASSET_DIRECTORY / "pose_manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
