import math

import bmesh
import bpy
from mathutils import Matrix, Vector


FINGER_SEGMENTS = ("Proximal", "Middle", "Distal")
THUMB_SEGMENTS = ("Metacarpal", "Proximal", "Distal")


def replace_original_gloves(armature, body):
    body_to_armature = armature.matrix_world.inverted() @ body.matrix_world
    armature_to_body = body_to_armature.inverted()
    removed_vertex_indices = set()
    for side in ("Left", "Right"):
        hand_name = "mixamorig:" + side + "Hand"
        hand_group_index = body.vertex_groups[hand_name].index
        hand_transform = armature.data.bones[hand_name].matrix_local.copy()
        for vertex in body.data.vertices:
            if not any(group.group == hand_group_index and group.weight > 0.5 for group in vertex.groups):
                continue
            hand_position = hand_transform.inverted() @ body_to_armature @ vertex.co
            if hand_position.y > 0.035:
                removed_vertex_indices.add(vertex.index)
            else:
                hand_position.x *= 0.70
                hand_position.z *= 0.70
                vertex.co = armature_to_body @ hand_transform @ hand_position
    editable_mesh = bmesh.new()
    editable_mesh.from_mesh(body.data)
    editable_mesh.verts.ensure_lookup_table()
    bmesh.ops.delete(editable_mesh, geom=[editable_mesh.verts[index] for index in removed_vertex_indices], context="VERTS")
    editable_mesh.to_mesh(body.data)
    editable_mesh.free()
    body.data.update()


def create_articulated_gloves(armature, body):
    replace_original_gloves(armature, body)
    hand_transforms = {side: armature.data.bones["mixamorig:" + side + "Hand"].matrix_local.copy() for side in ("Left", "Right")}
    vertices = []
    faces = []
    vertex_weights = []
    finger_definitions = []

    def append_vertex(position, weights):
        vertices.append(tuple(position))
        vertex_weights.append(weights)
        return len(vertices) - 1

    def append_rounded_palm(hand_transform, hand_name):
        previous_ring = None
        for station_index in range(12):
            palm_progress = station_index / 11
            longitudinal_position = palm_progress * 0.086
            width = 0.020 + 0.013 * math.sin(math.pi * 0.9 * palm_progress)
            thickness = 0.016 - 0.004 * math.sin(math.pi * 0.5 * palm_progress)
            ring = []
            for radial_index in range(12):
                angle = radial_index * math.tau / 12
                position = Vector((width * math.cos(angle), longitudinal_position, thickness * math.sin(angle)))
                ring.append(append_vertex(hand_transform @ position, {hand_name: 1.0}))
            if previous_ring is not None:
                for radial_index in range(12):
                    next_index = (radial_index + 1) % 12
                    faces.append((previous_ring[radial_index], ring[radial_index], ring[next_index], previous_ring[next_index]))
            else:
                faces.append(tuple(ring))
            previous_ring = ring
        faces.append(tuple(reversed(previous_ring)))

    for side, hand_transform in hand_transforms.items():
        hand_name = "mixamorig:" + side + "Hand"
        append_rounded_palm(hand_transform, hand_name)
        thumb_side = -1.0 if side == "Left" else 1.0
        for finger_name, lateral_position, lengths, radius in (
            ("Index", thumb_side * 0.023, (0.029, 0.020, 0.016), 0.0080),
            ("Middle", thumb_side * 0.008, (0.032, 0.023, 0.017), 0.0082),
            ("Ring", thumb_side * -0.008, (0.029, 0.021, 0.016), 0.0078),
            ("Little", thumb_side * -0.023, (0.023, 0.017, 0.014), 0.0065),
            ("Thumb", thumb_side * 0.027, (0.023, 0.022, 0.018), 0.0090),
        ):
            root_position = Vector((lateral_position, 0.079, 0.0))
            finger_direction = Vector((0.0, 1.0, 0.0))
            if finger_name == "Thumb":
                root_position = Vector((lateral_position, 0.045, 0.003))
                finger_direction = Vector((thumb_side * 0.55, 0.65, 0.40)).normalized()
            finger_definitions.append((side, finger_name, hand_transform, root_position, finger_direction, lengths, radius))

    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    for side, finger_name, hand_transform, root_position, finger_direction, lengths, radius in finger_definitions:
        parent_bone = armature.data.edit_bones["mixamorig:" + side + "Hand"]
        joint_position = root_position.copy()
        segment_names = THUMB_SEGMENTS if finger_name == "Thumb" else FINGER_SEGMENTS
        for segment_index, (segment_name, segment_length) in enumerate(zip(segment_names, lengths)):
            bone = armature.data.edit_bones.new("Driver" + side + finger_name + segment_name)
            bone.head = hand_transform @ joint_position
            joint_position += finger_direction * segment_length
            bone.tail = hand_transform @ joint_position
            bone.parent = parent_bone
            bone.use_connect = segment_index > 0
            bone.align_roll(hand_transform.to_3x3() @ Vector((0.0, 0.0, 1.0)))
            parent_bone = bone
    bpy.ops.object.mode_set(mode="OBJECT")

    for side, finger_name, hand_transform, root_position, finger_direction, lengths, radius in finger_definitions:
        segment_names = ["Driver" + side + finger_name + segment for segment in (THUMB_SEGMENTS if finger_name == "Thumb" else FINGER_SEGMENTS)]
        finger_basis = armature.data.bones[segment_names[0]].matrix_local.to_3x3()
        previous_ring = None
        stations = (0.0, lengths[0] * 0.45, lengths[0], lengths[0] + lengths[1] * 0.5, lengths[0] + lengths[1], sum(lengths) - radius * 0.65, sum(lengths))
        for station_index, distance in enumerate(stations):
            segment_index = 0 if distance < lengths[0] else (1 if distance < lengths[0] + lengths[1] else 2)
            weights = {segment_names[segment_index]: 1.0}
            if station_index == 2:
                weights = {segment_names[0]: 0.5, segment_names[1]: 0.5}
            elif station_index == 4:
                weights = {segment_names[1]: 0.5, segment_names[2]: 0.5}
            ring_radius = radius * (1.0 - 0.23 * distance / sum(lengths))
            if station_index == len(stations) - 1:
                ring_radius *= 0.25
            ring = []
            center = hand_transform @ (root_position + finger_direction * distance)
            for radial_index in range(10):
                angle = radial_index * math.tau / 10
                offset = finger_basis @ Vector((ring_radius * math.cos(angle), 0.0, ring_radius * 0.88 * math.sin(angle)))
                ring.append(append_vertex(center + offset, weights))
            if previous_ring is not None:
                for radial_index in range(10):
                    next_index = (radial_index + 1) % 10
                    faces.append((previous_ring[radial_index], ring[radial_index], ring[next_index], previous_ring[next_index]))
            else:
                faces.append(tuple(ring))
            previous_ring = ring
        faces.append(tuple(reversed(previous_ring)))

    glove_mesh = bpy.data.meshes.new("DriverArticulatedGloveGeometry")
    glove_mesh.from_pydata(vertices, [], faces)
    glove_mesh.update()
    gloves = bpy.data.objects.new("DriverArticulatedGloves", glove_mesh)
    bpy.context.collection.objects.link(gloves)
    gloves.parent = armature
    gloves.matrix_parent_inverse = Matrix.Identity(4)
    gloves.matrix_basis = Matrix.Identity(4)
    modifier = gloves.modifiers.new("DriverGloveSkinning", "ARMATURE")
    modifier.object = armature
    for vertex_index, weights in enumerate(vertex_weights):
        for bone_name, weight in weights.items():
            vertex_group = gloves.vertex_groups.get(bone_name) or gloves.vertex_groups.new(name=bone_name)
            vertex_group.add([vertex_index], weight, "REPLACE")
    material = bpy.data.materials.new("DriverGloveFabric")
    material.diffuse_color = (0.76, 0.78, 0.79, 1.0)
    material.use_nodes = True
    surface = material.node_tree.nodes.get("Principled BSDF")
    surface.inputs["Base Color"].default_value = material.diffuse_color
    surface.inputs["Roughness"].default_value = 0.87
    gloves.data.materials.append(material)
    for polygon in gloves.data.polygons:
        polygon.use_smooth = True
    gloves.select_set(True)
    return gloves
