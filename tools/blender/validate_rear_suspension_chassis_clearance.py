import json
import math
from pathlib import Path

import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree


repository_directory = Path(__file__).resolve().parents[2]
vehicle_profile_path = repository_directory / "game/data/vehicles/f1_2030/f1_2030_v10_geometric.json"
chassis_model_path = repository_directory / "game/assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb"
vehicle_profile = json.loads(vehicle_profile_path.read_text(encoding="utf-8"))
rear_corner = vehicle_profile["suspension"]["geometry_physical"]["corners"]["RL"]

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(chassis_model_path))
body_object = bpy.data.objects["GEO_CHASSIS_BODY"]
body_surface = BVHTree.FromObject(body_object, bpy.context.evaluated_depsgraph_get())
inverse_body_transform = body_object.matrix_world.inverted()
vertical_ray = inverse_body_transform.to_3x3() @ Vector((0.0, 0.0, -1.0))


def body_height_at(lateral_position, longitudinal_position):
    ray_origin = inverse_body_transform @ Vector((lateral_position, -longitudinal_position, 2.0))
    intersection, _, _, _ = body_surface.ray_cast(ray_origin, vertical_ray, 5.0)
    if intersection is None:
        raise AssertionError("Chassis body has no surface above the rear damper")
    return (body_object.matrix_world @ intersection).z


def rotated_damper_arm(rocker_angle):
    pivot = Vector(rear_corner["rocker"]["pivot"])
    rest_arm = Vector(rear_corner["rocker"]["damper_arm"])
    relative_arm = rest_arm - pivot
    return Vector((
        rest_arm.x,
        pivot.y - relative_arm.z * math.sin(rocker_angle),
        pivot.z + relative_arm.z * math.cos(rocker_angle),
    ))


chassis_attachment = Vector(rear_corner["damper"]["chassis"])
minimum_skin_clearance = float("inf")
for mirror_direction in (-1.0, 1.0):
    for rocker_step in range(54):
        rocker_angle = -0.27 + rocker_step * 0.01
        rocker_attachment = rotated_damper_arm(rocker_angle)
        for segment_step in range(21):
            segment_fraction = segment_step / 20.0
            damper_center = rocker_attachment.lerp(chassis_attachment, segment_fraction)
            body_height = body_height_at(damper_center.x * mirror_direction, damper_center.z)
            skin_clearance = body_height - damper_center.y - 0.04
            minimum_skin_clearance = min(minimum_skin_clearance, skin_clearance)
            if skin_clearance < 0.01:
                raise AssertionError(
                    f"Rear damper leaves chassis at angle {rocker_angle:.3f}, "
                    f"segment {segment_fraction:.2f}: clearance {skin_clearance:.4f} m"
                )

print(f"Rear damper minimum chassis skin clearance: {minimum_skin_clearance:.4f} m")
