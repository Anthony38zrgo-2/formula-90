extends Node

const DRIVER_ARM_INVERSE_KINEMATICS_SCRIPT := preload("res://scripts/vehicle/driver_arm_inverse_kinematics_modifier.gd")

@export var driver_model: PackedScene
@export var chassis_visual: Node3D
@export var steering_wheel_controller: Node
@export var seated_position := Vector3(0.0, -0.011, -0.34)

var driver_instance: Node3D
var driver_skeleton: Skeleton3D
var arm_modifier: SkeletonModifier3D
var hand_targets: Array[Node3D] = []
var elbow_targets: Array[Node3D] = []
var neutral_hand_bases: Array[Basis] = []
var neutral_grip_positions: Array[Vector3] = []
var steering_pivot: Node3D
var grip_transfer_progress: float = 0.0
var grip_transfer_direction: float = 1.0

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
		elbow_target.position = Vector3(side_sign * 0.20, 0.08, -0.08)
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
		neutral_hand_bases.append(chassis_visual.global_basis * hand_basis)
		neutral_grip_positions.append(Vector3(side_sign * 0.116, 0.0, 0.010))
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
			finger_chains.append({"bone_indices": finger_bone_indices, "is_thumb": finger_name == "Thumb"})
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
	process_priority = 10
	update_driver_hand_targets()

func _process(elapsed_seconds: float) -> void:
	update_driver_hand_targets(elapsed_seconds)

func update_driver_hand_targets(elapsed_seconds: float = 1.0 / 60.0) -> void:
	if steering_pivot == null or hand_targets.size() != 2:
		return
	var telemetry_vehicle := steering_wheel_controller.get("vehicle") as Node3D
	var steering_angle := clampf(float(telemetry_vehicle.call("get_true_steering_amount")), -1.0, 1.0) * deg_to_rad(float(steering_wheel_controller.get("total_rotation_degrees")) * 0.5)
	var release_progress := clampf((absf(steering_angle) - deg_to_rad(85.0)) / deg_to_rad(65.0), 0.0, 1.0)
	if grip_transfer_progress <= 0.0001:
		grip_transfer_direction = signf(steering_angle)
	if signf(steering_angle) != grip_transfer_direction:
		release_progress = 0.0
	grip_transfer_progress = move_toward(grip_transfer_progress, release_progress, maxf(elapsed_seconds, 0.0) / 0.35)
	release_progress = grip_transfer_progress
	for hand_index in range(2):
		var finger_opening := 0.0
		var leading_hand_index := 0 if grip_transfer_direction >= 0.0 else 1
		var hand_progress := clampf((release_progress - 0.25) * 2.0, 0.0, 1.0)
		var smooth_progress := smoothstep(0.0, 1.0, hand_progress)
		var grip_orientation_angle := -grip_transfer_direction * PI * smooth_progress
		var grip_position := neutral_grip_positions[hand_index].lerp(neutral_grip_positions[1 - hand_index], smooth_progress)
		if hand_index == leading_hand_index:
			var upper_rim_grip := Vector3(0.0, 0.068, 0.010)
			var first_transfer_progress := clampf(release_progress * 4.0, 0.0, 1.0)
			var final_transfer_progress := clampf((release_progress - 0.75) * 4.0, 0.0, 1.0)
			if release_progress < 0.25:
				grip_orientation_angle = -grip_transfer_direction * PI * 0.5 * smoothstep(0.0, 1.0, first_transfer_progress)
				finger_opening = sin(first_transfer_progress * PI)
				grip_position = neutral_grip_positions[hand_index].lerp(upper_rim_grip, smoothstep(0.0, 1.0, first_transfer_progress))
				grip_position.z += sin(first_transfer_progress * PI) * 0.07
			else:
				grip_orientation_angle = -grip_transfer_direction * PI * 0.5 * (1.0 + smoothstep(0.0, 1.0, final_transfer_progress))
				finger_opening = sin(final_transfer_progress * PI)
				grip_position = upper_rim_grip.lerp(neutral_grip_positions[1 - hand_index], smoothstep(0.0, 1.0, final_transfer_progress))
				grip_position.z += sin(final_transfer_progress * PI) * 0.07
		else:
			finger_opening = sin(hand_progress * PI)
			grip_position.y -= sin(hand_progress * PI) * 0.08
			grip_position.z += sin(hand_progress * PI) * 0.09
		var wrist_angle := clampf(steering_angle + grip_orientation_angle, -deg_to_rad(85.0), deg_to_rad(85.0))
		hand_targets[hand_index].global_basis = chassis_visual.global_basis * Basis(Vector3.BACK, wrist_angle) * chassis_visual.global_basis.inverse() * neutral_hand_bases[hand_index]
		var arm_configurations: Array = arm_modifier.get("arm_configurations")
		arm_configurations[hand_index]["finger_closure"] = 1.0 - finger_opening
		var palm_offset := Vector3(0.0, 0.066, 0.013)
		hand_targets[hand_index].global_position = steering_pivot.global_transform * grip_position - hand_targets[hand_index].global_basis * palm_offset
