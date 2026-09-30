extends "res://scripts/runtime/wheel_gun_arm_inverse_kinematics_modifier.gd"

func _process_modification_with_delta(elapsed_seconds: float) -> void:
	var skeleton := get_skeleton()
	if skeleton == null:
		return
	for arm_configuration in arm_configurations:
		prepare_shoulder_reach(skeleton, arm_configuration)
	super._process_modification_with_delta(elapsed_seconds)
	for arm_configuration in arm_configurations:
		var hand_bone_index := skeleton.find_bone(arm_configuration["end_bone_name"])
		var hand_target := arm_configuration["hand_target"] as Node3D
		if hand_bone_index < 0 or hand_target == null:
			continue
		var elbow_bone_index := skeleton.find_bone(arm_configuration["middle_bone_name"])
		var palm_offset: Vector3 = arm_configuration["palm_offset"]
		var palm_world_position := hand_target.global_transform * palm_offset
		var desired_hand_basis := hand_target.global_basis
		for adjustment_index in range(6):
			var current_hand_pose := skeleton.get_bone_global_pose(hand_bone_index)
			var current_elbow_pose := skeleton.get_bone_global_pose(elbow_bone_index)
			var forearm_direction := (skeleton.global_basis * (current_hand_pose.origin - current_elbow_pose.origin)).normalized()
			var desired_hand_direction := desired_hand_basis.y.normalized()
			var wrist_bend := forearm_direction.angle_to(desired_hand_direction)
			var allowed_fraction := minf(1.0, deg_to_rad(30.0) / maxf(wrist_bend, 0.0001))
			var constrained_hand_direction := forearm_direction.slerp(desired_hand_direction, allowed_fraction).normalized()
			hand_target.global_basis = Basis(Quaternion(desired_hand_direction, constrained_hand_direction)) * desired_hand_basis
			hand_target.global_position = palm_world_position - hand_target.global_basis * palm_offset
			_solve_arm_to_hand_target(skeleton, arm_configuration)
		var hand_pose := skeleton.get_bone_global_pose(hand_bone_index)
		hand_pose.basis = skeleton.global_basis.inverse() * hand_target.global_basis
		skeleton.set_bone_global_pose(hand_bone_index, hand_pose)
		update_finger_poses(skeleton, arm_configuration)

func prepare_shoulder_reach(skeleton: Skeleton3D, arm_configuration: Dictionary) -> void:
	var arm_index := skeleton.find_bone(arm_configuration["root_bone_name"])
	var elbow_index := skeleton.find_bone(arm_configuration["middle_bone_name"])
	var hand_index := skeleton.find_bone(arm_configuration["end_bone_name"])
	if arm_index < 0 or elbow_index < 0 or hand_index < 0:
		return
	var shoulder_index := skeleton.get_bone_parent(arm_index)
	if shoulder_index < 0:
		return
	var shoulder_pose := skeleton.get_bone_global_pose(shoulder_index)
	var arm_origin := skeleton.get_bone_global_pose(arm_index).origin
	var shoulder_direction := arm_origin - shoulder_pose.origin
	var shoulder_length := shoulder_direction.length()
	var hand_target := arm_configuration["hand_target"] as Node3D
	var target_position := skeleton.global_transform.affine_inverse() * hand_target.global_position
	var shoulder_to_target := target_position - shoulder_pose.origin
	var target_distance := shoulder_to_target.length()
	var arm_reach := skeleton.get_bone_rest(elbow_index).origin.length() + skeleton.get_bone_rest(hand_index).origin.length() - 0.025
	if shoulder_length < 0.001 or target_distance < 0.001 or arm_origin.distance_to(target_position) <= arm_reach:
		return
	var required_alignment := clampf((target_distance * target_distance + shoulder_length * shoulder_length - arm_reach * arm_reach) / (2.0 * target_distance * shoulder_length), -1.0, 1.0)
	var current_angle := shoulder_direction.angle_to(shoulder_to_target)
	var rotation_angle := minf(maxf(current_angle - acos(required_alignment), 0.0), deg_to_rad(35.0))
	var new_direction := shoulder_direction.normalized().slerp(shoulder_to_target.normalized(), rotation_angle / maxf(current_angle, 0.0001))
	shoulder_pose.basis = Basis(Quaternion(shoulder_direction.normalized(), new_direction.normalized())) * shoulder_pose.basis
	skeleton.set_bone_global_pose(shoulder_index, shoulder_pose)

func update_finger_poses(skeleton: Skeleton3D, arm_configuration: Dictionary) -> void:
	var finger_closure := clampf(float(arm_configuration["finger_closure"]), 0.0, 1.0)
	for finger_chain in arm_configuration["finger_chains"]:
		var flexion_degrees := Vector3(35.0, 75.0, 45.0)
		if finger_chain["is_thumb"]:
			flexion_degrees = Vector3(25.0, 60.0, 45.0)
		for segment_index in range(3):
			var bone_index: int = finger_chain["bone_indices"][segment_index]
			var parent_index := skeleton.get_bone_parent(bone_index)
			var local_pose := skeleton.get_bone_rest(bone_index)
			local_pose.basis *= Basis(Vector3.RIGHT, deg_to_rad(flexion_degrees[segment_index] * finger_closure))
			skeleton.set_bone_global_pose(bone_index, skeleton.get_bone_global_pose(parent_index) * local_pose)
