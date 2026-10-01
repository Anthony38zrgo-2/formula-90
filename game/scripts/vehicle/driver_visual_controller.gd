extends Node

const DRIVER_ARM_INVERSE_KINEMATICS_SCRIPT := preload("res://scripts/vehicle/driver_arm_inverse_kinematics_modifier.gd")
const DRIVER_HEAD_MOTION_SCRIPT := preload("res://scripts/vehicle/driver_head_motion_modifier.gd")
const COCKPIT_CONFIGURATION_SCRIPT := preload("res://scripts/camera/cockpit_camera_configuration.gd")
const GRIP_TRANSFER_START_DEGREES := 60.0
const GRIP_TRANSFER_END_DEGREES := 180.0
const FIRST_HAND_TRANSFER_END := 0.30
const SECOND_HAND_TRANSFER_END := 0.70
const MAXIMUM_GRIP_TRANSFER_SPEED := 0.75
const GRIP_TRANSFER_ACCELERATION := 4.0
const STEERING_GRIP_HALF_WIDTH_METERS := 0.132
const STEERING_GRIP_HALF_HEIGHT_METERS := 0.084
const STEERING_GRIP_DEPTH_METERS := -0.018
const REGRIP_RIM_CLEARANCE_METERS := 0.025

@export var driver_model: PackedScene
@export var chassis_visual: Node3D
@export var steering_wheel_controller: Node
@export var seated_position := Vector3(0.0, -0.011, -0.34)
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
	for descendant in driver_instance.find_children("*", "Skeleton3D", true, false):
		driver_skeleton = descendant as Skeleton3D
		break
	if driver_skeleton == null:
		push_error("Driver model requires a skeleton.")
		set_process(false)
		return
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
	var release_progress := clampf((absf(steering_angle) - deg_to_rad(GRIP_TRANSFER_START_DEGREES)) / deg_to_rad(GRIP_TRANSFER_END_DEGREES - GRIP_TRANSFER_START_DEGREES), 0.0, 1.0)
	if grip_transfer_progress <= 0.0001:
		grip_transfer_direction = -1.0 if steering_angle < 0.0 else 1.0
	if signf(steering_angle) != grip_transfer_direction:
		release_progress = 0.0
	var remaining_progress := release_progress - grip_transfer_progress
	var desired_velocity := clampf(remaining_progress * 12.0, -MAXIMUM_GRIP_TRANSFER_SPEED, MAXIMUM_GRIP_TRANSFER_SPEED)
	grip_transfer_velocity = move_toward(grip_transfer_velocity, desired_velocity, maxf(elapsed_seconds, 0.0) * GRIP_TRANSFER_ACCELERATION)
	var progress_step := grip_transfer_velocity * maxf(elapsed_seconds, 0.0)
	if absf(progress_step) >= absf(remaining_progress) and progress_step * remaining_progress >= 0.0:
		grip_transfer_progress = release_progress
		grip_transfer_velocity = 0.0
	else:
		grip_transfer_progress = clampf(grip_transfer_progress + progress_step, 0.0, 1.0)
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
			var final_transfer_progress := clampf((release_progress - SECOND_HAND_TRANSFER_END) / (1.0 - SECOND_HAND_TRANSFER_END), 0.0, 1.0)
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
		grip_position += rim_outward_direction * REGRIP_RIM_CLEARANCE_METERS * sin(release_progress * PI)
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

func steering_rim_grip_position(horizontal_direction: float, vertical_direction: float) -> Vector3:
	return Vector3(signf(horizontal_direction) * sqrt(absf(horizontal_direction)) * STEERING_GRIP_HALF_WIDTH_METERS, signf(vertical_direction) * sqrt(absf(vertical_direction)) * STEERING_GRIP_HALF_HEIGHT_METERS, STEERING_GRIP_DEPTH_METERS)
