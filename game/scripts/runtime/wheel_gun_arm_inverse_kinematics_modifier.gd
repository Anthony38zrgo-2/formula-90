extends SkeletonModifier3D
class_name WheelGunArmInverseKinematicsModifier

var arm_configurations: Array[Dictionary] = []
var completed_modification_count := 0


func _process_modification_with_delta(delta_seconds: float) -> void:
	completed_modification_count += 1
	var skeleton := get_skeleton()
	if skeleton == null:
		return
	for arm_configuration in arm_configurations:
		_solve_arm_to_hand_target(skeleton, arm_configuration)


func _solve_arm_to_hand_target(skeleton: Skeleton3D, arm_configuration: Dictionary) -> void:
	var root_bone_index := skeleton.find_bone(arm_configuration["root_bone_name"])
	var middle_bone_index := skeleton.find_bone(arm_configuration["middle_bone_name"])
	var end_bone_index := skeleton.find_bone(arm_configuration["end_bone_name"])
	var hand_target := arm_configuration["hand_target"] as Node3D
	var pole_target := arm_configuration["pole_target"] as Node3D
	if root_bone_index < 0 or middle_bone_index < 0 or end_bone_index < 0:
		return
	if hand_target == null or pole_target == null:
		return
	var root_pose := skeleton.get_bone_global_pose(root_bone_index)
	var middle_pose := skeleton.get_bone_global_pose(middle_bone_index)
	var end_pose := skeleton.get_bone_global_pose(end_bone_index)
	var shoulder_position := root_pose.origin
	var elbow_position := middle_pose.origin
	var wrist_position := end_pose.origin
	var upper_arm_length := shoulder_position.distance_to(elbow_position)
	var forearm_length := elbow_position.distance_to(wrist_position)
	if upper_arm_length <= 0.001 or forearm_length <= 0.001:
		return
	var target_position := skeleton.global_transform.affine_inverse() * hand_target.global_position
	var pole_position := skeleton.global_transform.affine_inverse() * pole_target.global_position
	var shoulder_to_target := target_position - shoulder_position
	if shoulder_to_target.length_squared() <= 0.000001:
		return
	var target_direction := shoulder_to_target.normalized()
	var minimum_reach := absf(upper_arm_length - forearm_length) + 0.001
	var maximum_reach := upper_arm_length + forearm_length - 0.001
	var reachable_distance := clampf(shoulder_to_target.length(), minimum_reach, maximum_reach)
	var reachable_wrist_position := shoulder_position + target_direction * reachable_distance
	var elbow_plane_direction := pole_position - shoulder_position
	elbow_plane_direction -= target_direction * elbow_plane_direction.dot(target_direction)
	if elbow_plane_direction.length_squared() <= 0.000001:
		elbow_plane_direction = elbow_position - shoulder_position
		elbow_plane_direction -= target_direction * elbow_plane_direction.dot(target_direction)
	if elbow_plane_direction.length_squared() <= 0.000001:
		elbow_plane_direction = Vector3.UP.cross(target_direction)
	if elbow_plane_direction.length_squared() <= 0.000001:
		elbow_plane_direction = Vector3.RIGHT.cross(target_direction)
	if elbow_plane_direction.length_squared() <= 0.000001:
		return
	var elbow_plane_normal := elbow_plane_direction.normalized()
	var distance_along_target := (
		upper_arm_length * upper_arm_length
		- forearm_length * forearm_length
		+ reachable_distance * reachable_distance
	) / (2.0 * reachable_distance)
	var distance_from_target_line := sqrt(maxf(
		upper_arm_length * upper_arm_length - distance_along_target * distance_along_target,
		0.0))
	var solved_elbow_position := (
		shoulder_position
		+ target_direction * distance_along_target
		+ elbow_plane_normal * distance_from_target_line
	)
	var current_upper_arm_direction := elbow_position - shoulder_position
	var solved_upper_arm_direction := solved_elbow_position - shoulder_position
	var upper_arm_rotation := Quaternion(
		current_upper_arm_direction.normalized(), solved_upper_arm_direction.normalized())
	var current_forearm_direction := wrist_position - elbow_position
	var forearm_direction_after_upper_arm_rotation := upper_arm_rotation * current_forearm_direction
	var solved_forearm_direction := reachable_wrist_position - solved_elbow_position
	var forearm_rotation := Quaternion(
		forearm_direction_after_upper_arm_rotation.normalized(), solved_forearm_direction.normalized())
	var updated_root_pose := root_pose
	updated_root_pose.basis = Basis(upper_arm_rotation) * root_pose.basis
	skeleton.set_bone_global_pose(root_bone_index, updated_root_pose)
	var updated_middle_pose := middle_pose
	updated_middle_pose.origin = solved_elbow_position
	updated_middle_pose.basis = Basis(forearm_rotation * upper_arm_rotation) * middle_pose.basis
	skeleton.set_bone_global_pose(middle_bone_index, updated_middle_pose)
