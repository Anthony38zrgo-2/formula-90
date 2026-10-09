import math

import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree


FINGER_LATERAL_POSITIONS = {"Index": 0.030, "Middle": 0.010, "Ring": -0.009, "Little": -0.025}


def smooth_weight(value, lower_limit, upper_limit):
    fraction = min(1.0, max(0.0, (value - lower_limit) / (upper_limit - lower_limit)))
    return fraction * fraction * (3.0 - 2.0 * fraction)


def prepare_original_glove_rig(armature, gloves):
    hand_vertices = {}
    palm_surfaces = {}
    gloves.data.calc_loop_triangles()
    for side in ("Left", "Right"):
        hand_name = "mixamorig:" + side + "Hand"
        hand_group = gloves.vertex_groups[hand_name]
        hand_inverse = armature.data.bones[hand_name].matrix_local.inverted()
        positions = {vertex.index: hand_inverse @ vertex.co for vertex in gloves.data.vertices if any(group.group == hand_group.index and group.weight > 0.5 for group in vertex.groups)}
        hand_vertices[side] = positions
        indices = list(positions)
        index_mapping = {original_index: index for index, original_index in enumerate(indices)}
        triangles = [[index_mapping[index] for index in triangle.vertices] for triangle in gloves.data.loop_triangles if all(index in positions for index in triangle.vertices)]
        surface = BVHTree.FromPolygons([positions[index] for index in indices], triangles, all_triangles=True)
        mirror_sign = 1.0 if side == "Left" else -1.0
        palm_position, palm_normal, triangle_index, distance = surface.ray_cast(Vector((mirror_sign * 0.025, 0.090, 0.0)), Vector((-mirror_sign, 0.0, 0.0)), 0.1)
        if palm_position is None or palm_normal.x * mirror_sign < 0.5:
            raise RuntimeError("Original glove has no verified inward palm surface")
        palm_surfaces[side] = {"position": list(palm_position), "normal": list(palm_normal)}
    bpy.context.view_layer.objects.active = armature
    armature.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    for side in ("Left", "Right"):
        mirror_sign = 1.0 if side == "Left" else -1.0
        hand = armature.data.edit_bones["mixamorig:" + side + "Hand"]
        hand_transform = hand.matrix.copy()
        for finger_name, lateral_position in FINGER_LATERAL_POSITIONS.items():
            bone = armature.data.edit_bones["Driver" + side + finger_name + "Proximal"]
            bone.use_connect = False
            bone.head = hand_transform @ Vector((-mirror_sign * 0.015, 0.095, lateral_position))
            bone.tail = hand_transform @ Vector((mirror_sign * 0.015, 0.130, lateral_position))
            bone.align_roll(hand_transform.to_3x3() @ Vector((0.0, 0.0, 1.0)))
        thumb_base = armature.data.edit_bones["Driver" + side + "ThumbMetacarpal"]
        thumb_base.use_connect = False
        thumb_base.head = hand_transform @ Vector((mirror_sign * 0.039, 0.063, 0.035))
        thumb_base.tail = hand_transform @ Vector((mirror_sign * 0.050, 0.107, 0.027))
        thumb_base.align_roll(hand_transform.to_3x3() @ Vector((0.0, 0.0, 1.0)))
        thumb_tip = armature.data.edit_bones["Driver" + side + "ThumbProximal"]
        thumb_tip.use_connect = False
        thumb_tip.head = thumb_base.tail.copy()
        thumb_tip.tail = hand_transform @ Vector((mirror_sign * 0.050, 0.140, 0.018))
        thumb_tip.align_roll(hand_transform.to_3x3() @ Vector((0.0, 0.0, 1.0)))
    bpy.ops.object.mode_set(mode="OBJECT")
    for side, positions in hand_vertices.items():
        mirror_sign = 1.0 if side == "Left" else -1.0
        hand_group = gloves.vertex_groups["mixamorig:" + side + "Hand"]
        for vertex_index, position in positions.items():
            lateral_position = position.z
            thumb_weight = smooth_weight(position.x * mirror_sign, 0.028, 0.043) * smooth_weight(position.z, -0.002, 0.016) * smooth_weight(position.y, 0.065, 0.090)
            finger_weight = smooth_weight(position.y, 0.085, 0.108) * (1.0 - thumb_weight)
            palm_weight = 1.0 - thumb_weight - finger_weight
            cuff_follow_weight = 1.0 - smooth_weight(position.y, -0.050, 0.025)
            weights = {hand_group.name: palm_weight * (1.0 - cuff_follow_weight), "mixamorig:" + side + "ForeArm": palm_weight * cuff_follow_weight}
            thumb_tip_weight = smooth_weight(position.y, 0.090, 0.120)
            weights["Driver" + side + "ThumbMetacarpal"] = thumb_weight * (1.0 - thumb_tip_weight)
            weights["Driver" + side + "ThumbProximal"] = thumb_weight * thumb_tip_weight
            nearest_fingers = sorted(FINGER_LATERAL_POSITIONS, key=lambda name: abs(lateral_position - FINGER_LATERAL_POSITIONS[name]))[:2]
            finger_influences = {name: math.exp(-((lateral_position - FINGER_LATERAL_POSITIONS[name]) / 0.009) ** 2) for name in nearest_fingers}
            influence_sum = sum(finger_influences.values())
            for name, influence in finger_influences.items():
                weights["Driver" + side + name + "Proximal"] = finger_weight * influence / influence_sum
            weights = dict(sorted(weights.items(), key=lambda item: item[1], reverse=True)[:4])
            total_weight = sum(weights.values())
            weights = {name: weight / total_weight for name, weight in weights.items()}
            hand_group.remove([vertex_index])
            for bone_name, weight in weights.items():
                if weight > 0.00001:
                    group = gloves.vertex_groups.get(bone_name) or gloves.vertex_groups.new(name=bone_name)
                    group.add([vertex_index], weight, "REPLACE")
    contract = bpy.data.objects.new("DriverOriginalGloveGripContract", None)
    bpy.context.collection.objects.link(contract)
    contract.parent = armature
    contract["contract_version"] = 1
    contract["added_bones"] = 0
    contract["deforming_finger_bones_per_hand"] = 6
    contract["closed_grip_is_rest_pose"] = True
    for side, surface in palm_surfaces.items():
        contract[side.lower() + "_palm_position"] = surface["position"]
        contract[side.lower() + "_palm_normal"] = surface["normal"]
    bpy.context.view_layer.update()
    return {"added_bones": 0, "deforming_finger_bones_per_hand": 6, "palm_surfaces": palm_surfaces, "closed_grip_is_rest_pose": True}
