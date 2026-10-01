extends "res://scripts/runtime/wheel_gun_arm_inverse_kinematics_modifier.gd"

const SHOULDER_REACH_RESERVE_METERS := 0.045
const MAXIMUM_SHOULDER_SPEED_DEGREES := 75.0
const SHOULDER_RESPONSE_SPEED := 12.0
const THUMB_BASE_OPPOSITION_DEGREES := 145.0
const MAXIMUM_HAND_ROTATION_SPEED_DEGREES := 325.0
const MAXIMUM_TARGET_WRIST_BEND_DEGREES := 30.0

var shoulder_rotations: Dictionary = {}
var hand_rotations: Dictionary = {}

func _process_modification_with_delta(elapsed_seconds: float) -> void:
	var skeleton := get_skeleton()
	if skeleton == null:
		return
	for arm_configuration in arm_configurations:
		prepare_shoulder_reach(skeleton, arm_configuration, elapsed_seconds)
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
			var allowed_fraction := minf(1.0, deg_to_rad(MAXIMUM_TARGET_WRIST_BEND_DEGREES) / maxf(wrist_bend, 0.0001))
			var constrained_hand_direction := forearm_direction.slerp(desired_hand_direction, allowed_fraction).normalized()
			hand_target.global_basis = Basis(Quaternion(desired_hand_direction, constrained_hand_direction)) * desired_hand_basis
			hand_target.global_position = palm_world_position - hand_target.global_basis * palm_offset
			_solve_arm_to_hand_target(skeleton, arm_configuration)
		var chassis := hand_target.get_parent() as Node3D
		var desired_hand_rotation := (chassis.global_basis.inverse() * hand_target.global_basis).get_rotation_quaternion()
		var previous_hand_rotation: Quaternion = hand_rotations.get(hand_bone_index, desired_hand_rotation)
		var hand_rotation_distance := previous_hand_rotation.angle_to(desired_hand_rotation)
		var maximum_hand_rotation_step := deg_to_rad(MAXIMUM_HAND_ROTATION_SPEED_DEGREES) * maxf(elapsed_seconds, 0.0)
		var hand_rotation := previous_hand_rotation.slerp(desired_hand_rotation, minf(1.0, maximum_hand_rotation_step / maxf(hand_rotation_distance, 0.0001)))
		hand_rotations[hand_bone_index] = hand_rotation
		hand_target.global_basis = chassis.global_basis * Basis(hand_rotation)
		hand_target.global_position = palm_world_position - hand_target.global_basis * palm_offset
		_solve_arm_to_hand_target(skeleton, arm_configuration)
		var hand_pose := skeleton.get_bone_global_pose(hand_bone_index)
		hand_pose.basis = skeleton.global_basis.inverse() * hand_target.global_basis
		skeleton.set_bone_global_pose(hand_bone_index, hand_pose)
		update_finger_poses(skeleton, arm_configuration)

func _solve_arm_to_hand_target(skeleton: Skeleton3D, arm_configuration: Dictionary) -> void:
	var shoulder_index := skeleton.find_bone(arm_configuration["root_bone_name"])
	var elbow_index := skeleton.find_bone(arm_configuration["middle_bone_name"])
	var wrist_index := skeleton.find_bone(arm_configuration["end_bone_name"])
	var hand_target := arm_configuration["hand_target"] as Node3D
	var shoulder_position := skeleton.get_bone_global_pose(shoulder_index).origin
	var target_position := skeleton.global_transform.affine_inverse() * hand_target.global_position
	var shoulder_to_target := target_position - shoulder_position
	var maximum_reach := skeleton.get_bone_rest(elbow_index).origin.length() + skeleton.get_bone_rest(wrist_index).origin.length() - 0.001
	if shoulder_to_target.length() > maximum_reach:
		hand_target.global_position = skeleton.global_transform * (shoulder_position + shoulder_to_target.normalized() * maximum_reach)
	super._solve_arm_to_hand_target(skeleton, arm_configuration)

func prepare_shoulder_reach(skeleton: Skeleton3D, arm_configuration: Dictionary, elapsed_seconds: float) -> void:
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
	var arm_reach := skeleton.get_bone_rest(elbow_index).origin.length() + skeleton.get_bone_rest(hand_index).origin.length() - SHOULDER_REACH_RESERVE_METERS
	if shoulder_length < 0.001 or target_distance < 0.001:
		return
	var required_alignment := clampf((target_distance * target_distance + shoulder_length * shoulder_length - arm_reach * arm_reach) / (2.0 * target_distance * shoulder_length), -1.0, 1.0)
	var current_angle := shoulder_direction.angle_to(shoulder_to_target)
	var rotation_angle := minf(maxf(current_angle - acos(required_alignment), 0.0), deg_to_rad(35.0))
	var new_direction := shoulder_direction.normalized().slerp(shoulder_to_target.normalized(), rotation_angle / maxf(current_angle, 0.0001))
	var desired_rotation := Quaternion(shoulder_direction.normalized(), new_direction.normalized())
	var previous_rotation: Quaternion = shoulder_rotations.get(shoulder_index, desired_rotation)
	var rotation_distance := previous_rotation.angle_to(desired_rotation)
	var maximum_rotation_step := deg_to_rad(MAXIMUM_SHOULDER_SPEED_DEGREES) * maxf(elapsed_seconds, 0.0)
	var interpolation_fraction := minf(1.0 - exp(-SHOULDER_RESPONSE_SPEED * maxf(elapsed_seconds, 0.0)), maximum_rotation_step / maxf(rotation_distance, 0.0001))
	var shoulder_rotation := previous_rotation.slerp(desired_rotation, interpolation_fraction)
	shoulder_rotations[shoulder_index] = shoulder_rotation
	shoulder_pose.basis = Basis(shoulder_rotation) * shoulder_pose.basis
	skeleton.set_bone_global_pose(shoulder_index, shoulder_pose)

func update_finger_poses(skeleton: Skeleton3D, arm_configuration: Dictionary) -> void:
	var finger_closure := clampf(float(arm_configuration["finger_closure"]), 0.0, 1.0)
	for finger_chain in arm_configuration["finger_chains"]:
		var flexion_degrees := Vector3(THUMB_BASE_OPPOSITION_DEGREES, 35.0, 30.0) if finger_chain["is_thumb"] else Vector3(45.0, 70.0, 35.0)
		if finger_chain["finger_name"] == "Index":
			flexion_degrees.z = 20.0
		for segment_index in range(3):
			var bone_index: int = finger_chain["bone_indices"][segment_index]
			var parent_index := skeleton.get_bone_parent(bone_index)
			var local_pose := skeleton.get_bone_rest(bone_index)
			var flexion_angle := flexion_degrees[segment_index] * finger_closure
			if finger_chain["is_thumb"] and segment_index == 0:
				flexion_angle = flexion_degrees[segment_index]
			local_pose.basis *= Basis(Vector3.RIGHT, deg_to_rad(flexion_angle))
			skeleton.set_bone_global_pose(bone_index, skeleton.get_bone_global_pose(parent_index) * local_pose)
