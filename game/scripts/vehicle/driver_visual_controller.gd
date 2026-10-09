extends Node

const DRIVER_ARM_INVERSE_KINEMATICS_SCRIPT := preload("res://scripts/vehicle/driver_arm_inverse_kinematics_modifier.gd")
const DRIVER_HEAD_MOTION_SCRIPT := preload("res://scripts/vehicle/driver_head_motion_modifier.gd")
const COCKPIT_CONFIGURATION_SCRIPT := preload("res://scripts/camera/cockpit_camera_configuration.gd")
const GRIP_TRANSFER_START_DEGREES := 60.0
const GRIP_TRANSFER_END_DEGREES := 180.0
const FIRST_HAND_TRANSFER_END := 0.30
const SECOND_HAND_TRANSFER_END := 0.70
const FINAL_HAND_TRANSFER_END := 0.86
const MAXIMUM_GRIP_TRANSFER_SPEED := 6.0
const GRIP_TRANSFER_RESPONSE_RATE := 60.0
const GRIP_TRANSFER_INTEGRATION_STEP_SECONDS := 1.0 / 240.0
const STEERING_GRIP_HALF_WIDTH_METERS := 0.132
const STEERING_GRIP_HALF_HEIGHT_METERS := 0.084
const STEERING_GRIP_DEPTH_METERS := -0.018
const REGRIP_RIM_CLEARANCE_METERS := 0.025

@export var driver_model: PackedScene
@export var chassis_visual: Node3D
@export var steering_wheel_controller: Node
@export var seated_position := Vector3(0.0, -0.011, -0.34)
@export_range(-20.0, 20.0, 0.5) var additional_torso_recline_degrees := 0.0
@export var pelvis_position_offset_meters := Vector3.ZERO
@export_range(-15.0, 15.0, 0.5) var additional_pelvis_recline_degrees := 0.0
@export_file("*.json") var cockpit_configuration_path: String

var driver_instance: Node3D
var driver_skeleton: Skeleton3D
var arm_modifier: SkeletonModifier3D
var hand_targets: Array[Node3D] = []
var elbow_targets: Array[Node3D] = []
var neutral_hand_bases: Array[Basis] = []
var steering_pivot: Node3D
var grip_transfer_progress: float = 0.0
var grip_transfer_direction: float = 1.0
var grip_transfer_velocity: float = 0.0
var previous_steering_angle: float = 0.0
var has_previous_steering_angle := false
var head_motion_modifier: SkeletonModifier3D
var driver_eye_point: Node3D
var driver_head_and_neck: MeshInstance3D

func _ready() -> void:
	if driver_model == null or chassis_visual == null or steering_wheel_controller == null:
		push_error("Driver requires a model, chassis, and steering wheel controller.")
		set_process(false)
		return
	steering_pivot = steering_wheel_controller.get("steering_wheel_pivot") as Node3D
	if steering_pivot == null:
		push_error("Driver cannot attach hands without the steering wheel pivot.")
		set_process(false)
		return
	driver_instance = driver_model.instantiate() as Node3D
	driver_instance.name = "Driver"
	chassis_visual.add_child(driver_instance)
	driver_instance.position = seated_position
	driver_instance.rotation.y = PI
	if not validate_driver_geometry_contract():
		driver_instance.queue_free()
		driver_instance = null
		set_process(false)
		return
	for descendant in driver_instance.find_children("*", "Skeleton3D", true, false):
		driver_skeleton = descendant as Skeleton3D
		break
	if driver_skeleton == null:
		push_error("Driver model requires a skeleton.")
		set_process(false)
		return
	apply_pelvis_and_leg_posture()
	apply_additional_torso_recline()
	arm_modifier = DRIVER_ARM_INVERSE_KINEMATICS_SCRIPT.new()
	arm_modifier.name = "DriverArmInverseKinematics"
	driver_skeleton.add_child(arm_modifier)
	var arm_configurations: Array[Dictionary] = []
	for side in ["Left", "Right"]:
		var side_sign := -1.0 if side == "Left" else 1.0
		var hand_target := Node3D.new()
		hand_target.name = side + "DriverHandTarget"
		chassis_visual.add_child(hand_target)
		hand_targets.append(hand_target)
		var elbow_target := Node3D.new()
		elbow_target.name = side + "DriverElbowTarget"
		chassis_visual.add_child(elbow_target)
		elbow_target.position = seated_position + Vector3(side_sign * 0.20, 0.091, 0.26)
		elbow_targets.append(elbow_target)
		var hand_bone_index := driver_skeleton.find_bone("mixamorig_" + side + "Hand")
		if hand_bone_index < 0:
			hand_bone_index = driver_skeleton.find_bone("mixamorig:" + side + "Hand")
		if hand_bone_index < 0:
			push_error("Driver skeleton is missing the " + side + " hand bone.")
			set_process(false)
			return
		var hand_bone_name := driver_skeleton.get_bone_name(hand_bone_index)
		var prefix := hand_bone_name.trim_suffix(side + "Hand")
		var hand_basis := Basis(Vector3.DOWN, Vector3.FORWARD, Vector3.RIGHT) if side == "Left" else Basis(Vector3.UP, Vector3.FORWARD, Vector3.LEFT)
		neutral_hand_bases.append(hand_basis)
		var finger_chains: Array[Dictionary] = []
		for finger_name in ["Index", "Middle", "Ring", "Little", "Thumb"]:
			var finger_bone_indices: Array[int] = []
			var segment_names := ["Metacarpal", "Proximal", "Distal"] if finger_name == "Thumb" else ["Proximal", "Middle", "Distal"]
			for segment_name in segment_names:
				var finger_bone_index := driver_skeleton.find_bone("Driver" + side + finger_name + segment_name)
				if finger_bone_index < 0:
					push_error("Driver skeleton is missing an articulated finger bone.")
					set_process(false)
					return
				finger_bone_indices.append(finger_bone_index)
			finger_chains.append({"bone_indices": finger_bone_indices, "is_thumb": finger_name == "Thumb", "finger_name": finger_name})
		arm_configurations.append({
			"root_bone_name": prefix + side + "Arm",
			"middle_bone_name": prefix + side + "ForeArm",
			"end_bone_name": hand_bone_name,
			"hand_target": hand_target,
			"pole_target": elbow_target,
			"palm_offset": Vector3(0.0, 0.066, 0.013),
			"finger_chains": finger_chains,
			"finger_closure": 1.0,
		})
	arm_modifier.set("arm_configurations", arm_configurations)
	configure_head_motion()
	process_priority = 10
	update_driver_hand_targets()

func _process(elapsed_seconds: float) -> void:
	update_driver_hand_targets(elapsed_seconds)

func validate_driver_geometry_contract() -> bool:
	var geometry_contract := driver_instance.find_child("DriverOriginalUniformGeometryContract", true, false)
	var contract_properties: Dictionary = geometry_contract.get_meta("extras", {}) if geometry_contract != null else {}
	if geometry_contract == null or int(contract_properties.get("contract_version", 0)) != 1 or str(contract_properties.get("source_sha256", "")) != "28eded787a300e10bae2f955431f49b9a7a62be9dcc81dbee4e1f011dcaab12b" or float(contract_properties.get("uniform_scale", 0.0)) <= 0.0 or str(contract_properties.get("neutral_geometry_sha256", "")).length() != 64:
		push_error("Driver rejected: only verified uniformly scaled original geometry is supported. Regenerate the canonical driver asset.")
		return false
	return true

func find_driver_bone(bone_name: String) -> int:
	var bone_index := driver_skeleton.find_bone("mixamorig_" + bone_name)
	if bone_index < 0:
		bone_index = driver_skeleton.find_bone("mixamorig:" + bone_name)
	return bone_index

func apply_pelvis_and_leg_posture() -> void:
	if pelvis_position_offset_meters.is_zero_approx() and is_zero_approx(additional_pelvis_recline_degrees):
		return
	var pelvis_index := find_driver_bone("Hips")
	var neck_index := find_driver_bone("Neck")
	if pelvis_index < 0 or neck_index < 0:
		push_error("Driver pelvis posture requires hips and neck bones.")
		return
	var original_neck_basis := driver_skeleton.get_bone_global_pose(neck_index).basis
	var original_foot_poses: Dictionary = {}
	for side in ["Left", "Right"]:
		var foot_index := find_driver_bone(side + "Foot")
		if foot_index < 0 or find_driver_bone(side + "UpLeg") < 0 or find_driver_bone(side + "Leg") < 0:
			push_error("Driver pelvis posture requires both complete leg chains.")
			return
		original_foot_poses[side] = driver_skeleton.get_bone_global_pose(foot_index)
	var skeleton_lateral_direction := (driver_skeleton.global_basis.inverse() * chassis_visual.global_basis.x).normalized()
	var pelvis_pose := driver_skeleton.get_bone_global_pose(pelvis_index)
	pelvis_pose.origin += driver_skeleton.global_basis.inverse() * chassis_visual.global_basis * pelvis_position_offset_meters
	pelvis_pose.basis = Basis(skeleton_lateral_direction, deg_to_rad(additional_pelvis_recline_degrees)) * pelvis_pose.basis
	driver_skeleton.set_bone_global_pose(pelvis_index, pelvis_pose)
	for side in ["Left", "Right"]:
		var thigh_index := find_driver_bone(side + "UpLeg")
		var lower_leg_index := find_driver_bone(side + "Leg")
		var foot_index := find_driver_bone(side + "Foot")
		var thigh_pose := driver_skeleton.get_bone_global_pose(thigh_index)
		var lower_leg_pose := driver_skeleton.get_bone_global_pose(lower_leg_index)
		var current_foot_pose := driver_skeleton.get_bone_global_pose(foot_index)
		var original_foot_pose: Transform3D = original_foot_poses[side]
		var thigh_length := thigh_pose.origin.distance_to(lower_leg_pose.origin)
		var lower_leg_length := lower_leg_pose.origin.distance_to(current_foot_pose.origin)
		var hip_to_ankle := original_foot_pose.origin - thigh_pose.origin
		var target_distance := clampf(hip_to_ankle.length(), absf(thigh_length - lower_leg_length) + 0.0001, thigh_length + lower_leg_length - 0.0001)
		var leg_direction := hip_to_ankle.normalized()
		var knee_projection := (thigh_length * thigh_length - lower_leg_length * lower_leg_length + target_distance * target_distance) / (2.0 * target_distance)
		var knee_height := sqrt(maxf(0.0, thigh_length * thigh_length - knee_projection * knee_projection))
		var knee_direction := lower_leg_pose.origin - thigh_pose.origin
		knee_direction = (knee_direction - leg_direction * knee_direction.dot(leg_direction)).normalized()
		var desired_knee := thigh_pose.origin + leg_direction * knee_projection + knee_direction * knee_height
		thigh_pose.basis = Basis(Quaternion((lower_leg_pose.origin - thigh_pose.origin).normalized(), (desired_knee - thigh_pose.origin).normalized())) * thigh_pose.basis
		driver_skeleton.set_bone_global_pose(thigh_index, thigh_pose)
		lower_leg_pose = driver_skeleton.get_bone_global_pose(lower_leg_index)
		current_foot_pose = driver_skeleton.get_bone_global_pose(foot_index)
		lower_leg_pose.basis = Basis(Quaternion((current_foot_pose.origin - lower_leg_pose.origin).normalized(), (original_foot_pose.origin - lower_leg_pose.origin).normalized())) * lower_leg_pose.basis
		driver_skeleton.set_bone_global_pose(lower_leg_index, lower_leg_pose)
		current_foot_pose = driver_skeleton.get_bone_global_pose(foot_index)
		current_foot_pose.basis = original_foot_pose.basis
		driver_skeleton.set_bone_global_pose(foot_index, current_foot_pose)
	var neck_pose := driver_skeleton.get_bone_global_pose(neck_index)
	neck_pose.basis = original_neck_basis
	driver_skeleton.set_bone_global_pose(neck_index, neck_pose)

func apply_additional_torso_recline() -> void:
	if is_zero_approx(additional_torso_recline_degrees):
		return
	var spine_bone_index := driver_skeleton.find_bone("mixamorig_Spine")
	var neck_bone_index := driver_skeleton.find_bone("mixamorig_Neck")
	if spine_bone_index < 0:
		spine_bone_index = driver_skeleton.find_bone("mixamorig:Spine")
	if neck_bone_index < 0:
		neck_bone_index = driver_skeleton.find_bone("mixamorig:Neck")
	if spine_bone_index < 0 or neck_bone_index < 0:
		push_error("Driver torso recline requires spine and neck bones.")
		return
	var skeleton_lateral_direction := (driver_skeleton.global_basis.inverse() * chassis_visual.global_basis.x).normalized()
	var torso_rotation := Basis(skeleton_lateral_direction, deg_to_rad(additional_torso_recline_degrees))
	var spine_pose := driver_skeleton.get_bone_global_pose(spine_bone_index)
	spine_pose.basis = torso_rotation * spine_pose.basis
	driver_skeleton.set_bone_global_pose(spine_bone_index, spine_pose)
	var neck_pose := driver_skeleton.get_bone_global_pose(neck_bone_index)
	neck_pose.basis = torso_rotation.inverse() * neck_pose.basis
	driver_skeleton.set_bone_global_pose(neck_bone_index, neck_pose)

func configure_head_motion() -> void:
	if cockpit_configuration_path.is_empty():
		return
	var cockpit_configuration := COCKPIT_CONFIGURATION_SCRIPT.load_from_path(cockpit_configuration_path)
	if cockpit_configuration == null:
		return
	driver_eye_point = driver_instance.find_child("DriverEyePoint", true, false) as Node3D
	driver_head_and_neck = driver_instance.find_child("DriverHeadAndNeck", true, false) as MeshInstance3D
	if driver_eye_point == null or driver_head_and_neck == null:
		push_error("Cockpit driver requires an authored eye point and separate head and neck geometry.")
		return
	head_motion_modifier = DRIVER_HEAD_MOTION_SCRIPT.new() as SkeletonModifier3D
	head_motion_modifier.name = "DriverHeadMotion"
	head_motion_modifier.set("vehicle", chassis_visual.get_parent())
	head_motion_modifier.set("configuration", cockpit_configuration)
	head_motion_modifier.set("driver_eye_point", driver_eye_point)
	driver_skeleton.add_child(head_motion_modifier)

func get_driver_eye_point() -> Node3D:
	return driver_eye_point

func set_cockpit_view_active(view_active: bool) -> void:
	if is_instance_valid(driver_head_and_neck):
		driver_head_and_neck.visible = not view_active

func set_cockpit_force_response_strength(response_strength: float) -> void:
	if head_motion_modifier == null:
		return
	var cockpit_configuration := head_motion_modifier.get("configuration") as CockpitCameraConfiguration
	cockpit_configuration.force_response_strength = clampf(response_strength, 0.0, 2.0)
	if cockpit_configuration.force_response_strength == 0.0:
		head_motion_modifier.call("reset_motion")

func update_driver_hand_targets(elapsed_seconds: float = 1.0 / 60.0) -> void:
	if steering_pivot == null or hand_targets.size() != 2:
		return
	var telemetry_vehicle := steering_wheel_controller.get("vehicle") as Node3D
	var steering_angle := clampf(float(telemetry_vehicle.call("get_true_steering_amount")), -1.0, 1.0) * deg_to_rad(float(steering_wheel_controller.get("total_rotation_degrees")) * 0.5)
	var steering_rotation_speed_degrees := 0.0
	if has_previous_steering_angle and elapsed_seconds > 0.0:
		steering_rotation_speed_degrees = rad_to_deg(absf(steering_angle - previous_steering_angle)) / elapsed_seconds
	previous_steering_angle = steering_angle
	has_previous_steering_angle = true
	arm_modifier.set("steering_rotation_speed_degrees", steering_rotation_speed_degrees)
	var release_progress := clampf((absf(steering_angle) - deg_to_rad(GRIP_TRANSFER_START_DEGREES)) / deg_to_rad(GRIP_TRANSFER_END_DEGREES - GRIP_TRANSFER_START_DEGREES), 0.0, 1.0)
	update_grip_transfer_progress(release_progress * signf(steering_angle), elapsed_seconds)
	var hand_transfer_velocity := grip_transfer_velocity
	if grip_transfer_progress >= SECOND_HAND_TRANSFER_END:
		hand_transfer_velocity *= (1.0 - SECOND_HAND_TRANSFER_END) / (FINAL_HAND_TRANSFER_END - SECOND_HAND_TRANSFER_END)
	arm_modifier.set("grip_transfer_velocity", hand_transfer_velocity)
	release_progress = grip_transfer_progress
	for hand_index in range(2):
		var finger_opening := 0.0
		var leading_hand_index := 0 if grip_transfer_direction >= 0.0 else 1
		var hand_progress := clampf((release_progress - FIRST_HAND_TRANSFER_END) / (SECOND_HAND_TRANSFER_END - FIRST_HAND_TRANSFER_END), 0.0, 1.0)
		var smooth_progress := smoothstep(0.0, 1.0, hand_progress)
		var initial_side_sign := -1.0 if hand_index == 0 else 1.0
		var transfer_angle := PI * smooth_progress
		var grip_orientation_angle := -grip_transfer_direction * transfer_angle
		var grip_position := steering_rim_grip_position(initial_side_sign * cos(transfer_angle), -sin(transfer_angle))
		if hand_index == leading_hand_index:
			var first_transfer_progress := clampf(release_progress / FIRST_HAND_TRANSFER_END, 0.0, 1.0)
			var final_transfer_progress := clampf((release_progress - SECOND_HAND_TRANSFER_END) / (FINAL_HAND_TRANSFER_END - SECOND_HAND_TRANSFER_END), 0.0, 1.0)
			if release_progress < FIRST_HAND_TRANSFER_END:
				finger_opening = sin(first_transfer_progress * PI)
				var upper_transfer_angle := PI * 0.5 * smoothstep(0.0, 1.0, first_transfer_progress)
				grip_orientation_angle = -grip_transfer_direction * upper_transfer_angle
				grip_position = steering_rim_grip_position(initial_side_sign * cos(upper_transfer_angle), sin(upper_transfer_angle))
				grip_position.z += sin(first_transfer_progress * PI) * 0.025
			else:
				finger_opening = sin(final_transfer_progress * PI)
				var upper_transfer_angle := PI * 0.5 * smoothstep(0.0, 1.0, final_transfer_progress)
				grip_orientation_angle = -grip_transfer_direction * (PI * 0.5 + upper_transfer_angle)
				grip_position = steering_rim_grip_position(-initial_side_sign * sin(upper_transfer_angle), cos(upper_transfer_angle))
				grip_position.z += sin(final_transfer_progress * PI) * 0.025
		else:
			finger_opening = sin(hand_progress * PI)
			grip_position.z += sin(hand_progress * PI) * 0.030
		var rim_outward_direction := Vector3(grip_position.x, grip_position.y, 0.0).normalized()
		grip_position += rim_outward_direction * REGRIP_RIM_CLEARANCE_METERS * finger_opening
		var wrist_angle := steering_angle + grip_orientation_angle
		hand_targets[hand_index].global_basis = chassis_visual.global_basis * Basis(Vector3.BACK, wrist_angle) * neutral_hand_bases[hand_index]
		var arm_configurations: Array = arm_modifier.get("arm_configurations")
		arm_configurations[hand_index]["finger_closure"] = 1.0 - finger_opening
		var palm_offset := Vector3(0.0, 0.066, 0.013)
		hand_targets[hand_index].global_position = steering_pivot.global_transform * grip_position - hand_targets[hand_index].global_basis * palm_offset
		var palm_chassis_position := steering_pivot.transform * grip_position
		var side_sign := -1.0 if hand_index == 0 else 1.0
		var elbow_height := clampf((palm_chassis_position.y - steering_pivot.position.y) * 0.4, -0.04, 0.04)
		var desired_elbow_position := seated_position + Vector3(side_sign * 0.20, 0.091 + elbow_height, 0.26)
		elbow_targets[hand_index].position = elbow_targets[hand_index].position.lerp(desired_elbow_position, 1.0 - exp(-10.0 * maxf(elapsed_seconds, 0.0)))

func update_grip_transfer_progress(target_progress: float, elapsed_seconds: float) -> void:
	var remaining_seconds := maxf(elapsed_seconds, 0.0)
	var signed_progress := grip_transfer_progress * grip_transfer_direction
	var signed_velocity := grip_transfer_velocity * grip_transfer_direction
	while remaining_seconds > 0.0000001:
		var integration_seconds := minf(remaining_seconds, GRIP_TRANSFER_INTEGRATION_STEP_SECONDS)
		var displacement := signed_progress - target_progress
		var spring_velocity := signed_velocity + GRIP_TRANSFER_RESPONSE_RATE * displacement
		var response_decay := exp(-GRIP_TRANSFER_RESPONSE_RATE * integration_seconds)
		var next_displacement := (displacement + spring_velocity * integration_seconds) * response_decay
		var next_velocity := (signed_velocity - GRIP_TRANSFER_RESPONSE_RATE * spring_velocity * integration_seconds) * response_decay
		var next_progress := target_progress + next_displacement
		if absf(next_velocity) > MAXIMUM_GRIP_TRANSFER_SPEED:
			next_velocity = clampf(next_velocity, -MAXIMUM_GRIP_TRANSFER_SPEED, MAXIMUM_GRIP_TRANSFER_SPEED)
			next_progress = signed_progress + (signed_velocity + next_velocity) * 0.5 * integration_seconds
		signed_progress = clampf(next_progress, -1.0, 1.0)
		signed_velocity = next_velocity
		if (signed_progress <= -1.0 and signed_velocity < 0.0) or (signed_progress >= 1.0 and signed_velocity > 0.0):
			signed_velocity = 0.0
		remaining_seconds -= integration_seconds
	if absf(signed_progress) > 0.0000001:
		grip_transfer_direction = signf(signed_progress)
	grip_transfer_progress = absf(signed_progress)
	grip_transfer_velocity = signed_velocity * grip_transfer_direction

func steering_rim_grip_position(horizontal_direction: float, vertical_direction: float) -> Vector3:
	return Vector3(signf(horizontal_direction) * sqrt(absf(horizontal_direction)) * STEERING_GRIP_HALF_WIDTH_METERS, signf(vertical_direction) * sqrt(absf(vertical_direction)) * STEERING_GRIP_HALF_HEIGHT_METERS, STEERING_GRIP_DEPTH_METERS)
