extends "res://scripts/runtime/wheel_gun_arm_inverse_kinematics_modifier.gd"

const SHOULDER_REACH_RESERVE_METERS := 0.045
const MAXIMUM_SHOULDER_SPEED_DEGREES := 75.0
const SHOULDER_RESPONSE_SPEED := 12.0
const THUMB_BASE_OPPOSITION_DEGREES := 145.0
const MAXIMUM_HAND_ROTATION_SPEED_DEGREES := 325.0
const STEERING_FOLLOW_START_DEGREES_PER_SECOND := 90.0
const STEERING_FOLLOW_FULL_DEGREES_PER_SECOND := 180.0
const MAXIMUM_TARGET_WRIST_BEND_DEGREES := 25.0
const MAXIMUM_ORIGINAL_THUMB_BASE_OPENING_DEGREES := 25.0
const MAXIMUM_ORIGINAL_THUMB_TIP_OPENING_DEGREES := 20.0
const MAXIMUM_ORIGINAL_FINGER_OPENING_DEGREES := 12.0
const PALM_ROTATION_BUDGET_FRACTION := 0.98

var shoulder_rotations: Dictionary = {}
var hand_rotations: Dictionary = {}
var grip_transfer_velocity: float = 0.0
var steering_rotation_speed_degrees: float = 0.0

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
		var steering_follow_speed := steering_rotation_speed_degrees * smoothstep(STEERING_FOLLOW_START_DEGREES_PER_SECOND, STEERING_FOLLOW_FULL_DEGREES_PER_SECOND, steering_rotation_speed_degrees)
		var hand_rotation_speed := maxf(MAXIMUM_HAND_ROTATION_SPEED_DEGREES, steering_follow_speed + MAXIMUM_HAND_ROTATION_SPEED_DEGREES * absf(grip_transfer_velocity))
		var maximum_hand_rotation_step := deg_to_rad(hand_rotation_speed) * maxf(elapsed_seconds, 0.0)
		var hand_rotation := previous_hand_rotation.slerp(desired_hand_rotation, minf(1.0, maximum_hand_rotation_step / maxf(hand_rotation_distance, 0.0001)))
		hand_rotations[hand_bone_index] = hand_rotation
		hand_target.global_basis = chassis.global_basis * Basis(hand_rotation)
		hand_target.global_position = palm_world_position - hand_target.global_basis * palm_offset
		_solve_arm_to_hand_target(skeleton, arm_configuration)
		for adjustment_index in range(12):
			var constrained_hand_pose := skeleton.get_bone_global_pose(hand_bone_index)
			var constrained_elbow_pose := skeleton.get_bone_global_pose(elbow_bone_index)
			var forearm_direction := (skeleton.global_basis * (constrained_hand_pose.origin - constrained_elbow_pose.origin)).normalized()
			var hand_direction := hand_target.global_basis.y.normalized()
			var wrist_bend := forearm_direction.angle_to(hand_direction)
			if wrist_bend <= deg_to_rad(MAXIMUM_TARGET_WRIST_BEND_DEGREES) + 0.0001:
				break
			var constrained_direction := forearm_direction.slerp(hand_direction, deg_to_rad(MAXIMUM_TARGET_WRIST_BEND_DEGREES) / wrist_bend).normalized()
			hand_target.global_basis = Basis(Quaternion(hand_direction, constrained_direction)) * hand_target.global_basis
			hand_target.global_position = palm_world_position - hand_target.global_basis * palm_offset
			_solve_arm_to_hand_target(skeleton, arm_configuration)
		var final_hand_direction := hand_target.global_basis.y.normalized()
		var palm_surface_normal: Vector3 = arm_configuration.get("palm_surface_normal", Vector3.BACK)
		var desired_palm_normal := (desired_hand_basis * palm_surface_normal).normalized()
		var final_palm_normal := (desired_palm_normal - final_hand_direction * desired_palm_normal.dot(final_hand_direction)).normalized()
		if final_palm_normal.length_squared() > 0.5:
			var previous_hand_basis := chassis.global_basis * Basis(previous_hand_rotation)
			var direction_rotation := Quaternion(previous_hand_basis.y.normalized(), final_hand_direction)
			var carried_basis := Basis(direction_rotation) * previous_hand_basis
			var carried_palm_normal := carried_basis * palm_surface_normal
			carried_palm_normal = (carried_palm_normal - final_hand_direction * carried_palm_normal.dot(final_hand_direction)).normalized()
			var direction_rotation_angle := previous_hand_basis.y.angle_to(final_hand_direction)
			var allowed_rotation_angle := maximum_hand_rotation_step * PALM_ROTATION_BUDGET_FRACTION
			var remaining_roll_step := 0.0
			if direction_rotation_angle < allowed_rotation_angle:
				remaining_roll_step = 2.0 * acos(clampf(cos(allowed_rotation_angle * 0.5) / maxf(cos(direction_rotation_angle * 0.5), 0.0001), -1.0, 1.0))
			var palm_roll_angle := carried_palm_normal.signed_angle_to(final_palm_normal, final_hand_direction)
			final_palm_normal = carried_palm_normal.rotated(final_hand_direction, clampf(palm_roll_angle, -remaining_roll_step, remaining_roll_step))
			var perpendicular_direction := final_hand_direction.cross(final_palm_normal).normalized()
			var palm_projection := Vector2(palm_surface_normal.x, palm_surface_normal.z).normalized()
			hand_target.global_basis = Basis(palm_projection.y * perpendicular_direction + palm_projection.x * final_palm_normal, final_hand_direction, palm_projection.y * final_palm_normal - palm_projection.x * perpendicular_direction)
			hand_target.global_position = palm_world_position - hand_target.global_basis * palm_offset
			_solve_arm_to_hand_target(skeleton, arm_configuration)
		hand_rotations[hand_bone_index] = (chassis.global_basis.inverse() * hand_target.global_basis).get_rotation_quaternion()
		align_forearm_with_hand(skeleton, elbow_bone_index, hand_bone_index, skeleton.global_basis.inverse() * hand_target.global_basis)
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
	if bool(arm_configuration.get("original_closed_glove", false)):
		update_original_glove_opening(skeleton, arm_configuration, 1.0 - finger_closure)
		return
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

func update_original_glove_opening(skeleton: Skeleton3D, arm_configuration: Dictionary, opening_fraction: float) -> void:
	for finger_chain in arm_configuration["finger_chains"]:
		for segment_index in range(3):
			var bone_index: int = finger_chain["bone_indices"][segment_index]
			var parent_index := skeleton.get_bone_parent(bone_index)
			var local_pose := skeleton.get_bone_rest(bone_index)
			if finger_chain["is_thumb"]:
				if segment_index == 0:
					local_pose.basis *= Basis(Vector3.BACK, deg_to_rad(MAXIMUM_ORIGINAL_THUMB_BASE_OPENING_DEGREES * float(arm_configuration["thumb_opening_sign"]) * opening_fraction))
				elif segment_index == 1:
					local_pose.basis *= Basis(Vector3.RIGHT, deg_to_rad(MAXIMUM_ORIGINAL_THUMB_TIP_OPENING_DEGREES * opening_fraction))
			elif segment_index == 0:
				local_pose.basis *= Basis(Vector3.BACK, deg_to_rad(-MAXIMUM_ORIGINAL_FINGER_OPENING_DEGREES * float(arm_configuration["thumb_opening_sign"]) * opening_fraction))
			skeleton.set_bone_global_pose(bone_index, skeleton.get_bone_global_pose(parent_index) * local_pose)

func align_forearm_with_hand(skeleton: Skeleton3D, forearm_bone_index: int, hand_bone_index: int, hand_basis: Basis) -> void:
	var forearm_pose := skeleton.get_bone_global_pose(forearm_bone_index)
	var hand_position := skeleton.get_bone_global_pose(hand_bone_index).origin
	var forearm_direction := (hand_position - forearm_pose.origin).normalized()
	var current_lateral_direction := forearm_pose.basis.x.normalized()
	current_lateral_direction = (current_lateral_direction - forearm_direction * current_lateral_direction.dot(forearm_direction)).normalized()
	var desired_lateral_direction := hand_basis.x.normalized()
	desired_lateral_direction = (desired_lateral_direction - forearm_direction * desired_lateral_direction.dot(forearm_direction)).normalized()
	var forearm_twist_angle := current_lateral_direction.signed_angle_to(desired_lateral_direction, forearm_direction)
	forearm_pose.basis = Basis(forearm_direction, forearm_twist_angle) * forearm_pose.basis
	skeleton.set_bone_global_pose(forearm_bone_index, forearm_pose)
