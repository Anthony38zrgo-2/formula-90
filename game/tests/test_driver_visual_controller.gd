extends SceneTree

class SteeringTelemetryVehicle extends Node3D:
	var effective_steering_amount: float = 0.0

	func get_true_steering_amount() -> float:
		return effective_steering_amount

var failures: Array[String] = []
var maximum_wrist_error: float = 0.0
var maximum_bone_length_error: float = 0.0
var minimum_hand_separation: float = INF
var maximum_elbow_flexion_degrees: float = 0.0
var maximum_wrist_bend_degrees: float = 0.0
var maximum_finger_bone_length_error: float = 0.0
var minimum_closed_palm_alignment: float = 1.0
var maximum_closed_thumb_extension_alignment: float = -1.0
var minimum_finger_closure: float = 1.0
var maximum_simultaneously_open_hands: int = 0
var maximum_shoulder_protraction_degrees: float = 0.0
var maximum_shoulder_bone_length_error: float = 0.0
var driver_controller: Node
var modification_samples: int = 0
var previous_bone_rotations: Dictionary = {}
var maximum_hand_rotation_step_degrees: float = 0.0
var maximum_shoulder_rotation_step_degrees: float = 0.0
var measure_rotation_steps := false
var maximum_final_supporting_hand_rim_error_meters: float = 0.0

func _init() -> void:
	call_deferred("run_validation")

func run_validation() -> void:
	var vehicle_scene := load("res://scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn") as PackedScene
	if vehicle_scene == null:
		printerr("Driver vehicle scene failed to load.")
		quit(1)
		return
	var instance := vehicle_scene.instantiate()
	root.add_child(instance)
	var vehicle := instance.get_node("VehicleRigidBody") as Node3D
	vehicle.set("freeze", true)
	instance.get_node("F12030V10InputController").set_process(false)
	driver_controller = vehicle.get_node("DriverVisualController")
	var driver_skeleton := driver_controller.get("driver_skeleton") as Skeleton3D
	if driver_skeleton == null:
		printerr("Driver has no imported skeleton.")
		quit(1)
		return
	if driver_skeleton.get_bone_count() != 55:
		failures.append("Driver does not contain the thirty added finger and thumb bones.")
	var arm_modifier := driver_controller.get("arm_modifier") as SkeletonModifier3D
	arm_modifier.modification_processed.connect(validate_arm_pose)
	var steering_telemetry := SteeringTelemetryVehicle.new()
	root.add_child(steering_telemetry)
	var steering_controller := vehicle.get_node("SteeringWheelVisualController")
	steering_controller.set("vehicle", steering_telemetry)
	var head_index := driver_skeleton.find_bone("mixamorig_Head")
	var head_pose := driver_skeleton.global_transform * driver_skeleton.get_bone_global_pose(head_index)
	var chassis := vehicle.get_node("ChassisVisual") as Node3D
	var helmet_center := chassis.global_transform.affine_inverse() * head_pose * Vector3(0.0, 0.1279602, 0.045034)
	var steering_pivot := steering_controller.get("steering_wheel_pivot") as Node3D
	var helmet_height_offset := helmet_center.y - steering_pivot.position.y
	if absf(helmet_height_offset) > 0.02:
		failures.append("Driver helmet is not at steering wheel height.")
	if (chassis.global_basis.inverse() * head_pose.basis.z).dot(Vector3.FORWARD) < 0.95:
		failures.append("Driver head is not facing forward.")
	print("DRIVER_HELMET_HEIGHT_OFFSET_METERS=" + str(helmet_height_offset))
	validate_grip_transfer_response(steering_telemetry, steering_controller)
	validate_final_hand_transfer(steering_telemetry, steering_controller)
	for settle_frame in range(30):
		await process_frame
	measure_rotation_steps = true
	var steering_steps := range(0, 181)
	steering_steps.append_array(range(179, -181, -1))
	steering_steps.append_array(range(-179, 1))
	for steering_step in steering_steps:
		var steering_amount := float(steering_step) / 180.0
		steering_telemetry.effective_steering_amount = steering_amount
		steering_controller.call("update_steering_wheel_pose")
		await process_frame
	measure_rotation_steps = false
	var fast_steering_steps := range(0, 181, 3)
	fast_steering_steps.append_array(range(177, -181, -3))
	fast_steering_steps.append_array(range(-177, 1, 3))
	for steering_step in fast_steering_steps:
		steering_telemetry.effective_steering_amount = float(steering_step) / 180.0
		steering_controller.call("update_steering_wheel_pose")
		await process_frame
	await validate_fast_final_hand_transfer(steering_telemetry, steering_controller)
	await validate_lock_to_lock_transitions(steering_telemetry, steering_controller)
	for settle_frame in range(120):
		await process_frame
	if float(driver_controller.get("grip_transfer_progress")) > 0.001:
		failures.append("Driver does not recover its neutral grip after returning to center.")
	driver_controller.call("update_driver_hand_targets", 0.0)
	var neutral_target_bases: Array[Basis] = []
	for hand_target in driver_controller.get("hand_targets"):
		neutral_target_bases.append(chassis.global_basis.inverse() * hand_target.global_basis)
	for chassis_heading in [45.0, 90.0, 180.0, 270.0, 360.0]:
		vehicle.rotation.y = deg_to_rad(chassis_heading)
		driver_controller.call("update_driver_hand_targets", 0.0)
		for hand_index in range(2):
			var target: Node3D = driver_controller.get("hand_targets")[hand_index]
			var local_hand_rotation := (chassis.global_basis.inverse() * target.global_basis).get_rotation_quaternion()
			if local_hand_rotation.angle_to(neutral_target_bases[hand_index].get_rotation_quaternion()) > deg_to_rad(0.1):
				failures.append("Driver hand orientation depends on the world heading of the chassis.")
		await process_frame
	if maximum_hand_rotation_step_degrees > 5.5:
		failures.append("Driver hands rotate too abruptly during the slow steering sweep.")
	if maximum_shoulder_rotation_step_degrees > 1.4:
		failures.append("Driver shoulders rotate too abruptly during the slow steering sweep.")
	if modification_samples < 16:
		failures.append("Driver arm modifier did not process the steering sweep.")
	if maximum_wrist_error > 0.005:
		failures.append("Driver hand targets exceed anatomical arm reach.")
	if maximum_bone_length_error > 0.001:
		failures.append("Driver steering stretches arm bones.")
	if minimum_hand_separation < 0.060:
		failures.append("Driver hands overlap during regrip.")
	if maximum_elbow_flexion_degrees > 150.0:
		failures.append("Driver elbows exceed the allowed flexion.")
	if maximum_wrist_bend_degrees > 35.0:
		failures.append("Driver wrists exceed the allowed bend.")
	if maximum_finger_bone_length_error > 0.0001:
		failures.append("Driver grip stretches finger bones.")
	if minimum_closed_palm_alignment < 0.70:
		failures.append("Driver closed palms do not face the steering rim.")
	if maximum_closed_thumb_extension_alignment > -0.50:
		failures.append("Driver closed thumbs do not fold around the steering rim.")
	if minimum_finger_closure > 0.15:
		failures.append("Driver fingers do not open while changing grip.")
	if maximum_simultaneously_open_hands > 1:
		failures.append("Driver releases both hands at once.")
	if maximum_final_supporting_hand_rim_error_meters > 0.001:
		failures.append("Driver supporting hand separates from the steering rim during the final regrip.")
	if maximum_shoulder_protraction_degrees > 35.1:
		failures.append("Driver shoulders exceed the allowed protraction.")
	if maximum_shoulder_bone_length_error > 0.0001:
		failures.append("Driver grip stretches shoulder bones.")
	var livery_scene := load("res://scenes/vehicles/f1_2030_v10/f1_2030_v10_rust_mp4_6_senna_1.tscn") as PackedScene
	var livery_instance := livery_scene.instantiate()
	if livery_instance.get_node("VehicleRigidBody").has_node("DriverVisualController"):
		failures.append("Driver was added to the unrequested livery variant.")
	livery_instance.free()
	print("DRIVER_MAXIMUM_WRIST_ERROR_METERS=" + str(maximum_wrist_error))
	print("DRIVER_MAXIMUM_BONE_LENGTH_ERROR_METERS=" + str(maximum_bone_length_error))
	print("DRIVER_MINIMUM_HAND_SEPARATION_METERS=" + str(minimum_hand_separation))
	print("DRIVER_MAXIMUM_ELBOW_FLEXION_DEGREES=" + str(maximum_elbow_flexion_degrees))
	print("DRIVER_MAXIMUM_WRIST_BEND_DEGREES=" + str(maximum_wrist_bend_degrees))
	print("DRIVER_MAXIMUM_FINGER_BONE_LENGTH_ERROR_METERS=" + str(maximum_finger_bone_length_error))
	print("DRIVER_MINIMUM_CLOSED_PALM_ALIGNMENT=" + str(minimum_closed_palm_alignment))
	print("DRIVER_MAXIMUM_CLOSED_THUMB_EXTENSION_ALIGNMENT=" + str(maximum_closed_thumb_extension_alignment))
	print("DRIVER_MINIMUM_FINGER_CLOSURE=" + str(minimum_finger_closure))
	print("DRIVER_MAXIMUM_SIMULTANEOUSLY_OPEN_HANDS=" + str(maximum_simultaneously_open_hands))
	print("DRIVER_MAXIMUM_FINAL_SUPPORTING_HAND_RIM_ERROR_METERS=" + str(maximum_final_supporting_hand_rim_error_meters))
	print("DRIVER_MAXIMUM_SHOULDER_PROTRACTION_DEGREES=" + str(maximum_shoulder_protraction_degrees))
	print("DRIVER_MAXIMUM_SHOULDER_BONE_LENGTH_ERROR_METERS=" + str(maximum_shoulder_bone_length_error))
	print("DRIVER_MAXIMUM_HAND_ROTATION_STEP_DEGREES=" + str(maximum_hand_rotation_step_degrees))
	print("DRIVER_MAXIMUM_SHOULDER_ROTATION_STEP_DEGREES=" + str(maximum_shoulder_rotation_step_degrees))
	for failure in failures:
		printerr(failure)
	print("DRIVER_VALIDATION_FAILURES=" + str(failures.size()))
	instance.queue_free()
	steering_telemetry.queue_free()
	quit(0 if failures.is_empty() else 1)

func validate_grip_transfer_response(steering_telemetry: SteeringTelemetryVehicle, steering_controller: Node) -> void:
	driver_controller.set_process(false)
	var settled_progress_by_frame_rate: Array[float] = []
	for frames_per_second in [30, 60, 120]:
		driver_controller.set("grip_transfer_progress", 0.0)
		driver_controller.set("grip_transfer_velocity", 0.0)
		driver_controller.set("grip_transfer_direction", 1.0)
		steering_telemetry.effective_steering_amount = 1.0
		steering_controller.call("update_steering_wheel_pose")
		for response_frame in range(int(0.8 * frames_per_second)):
			driver_controller.call("update_driver_hand_targets", 1.0 / frames_per_second)
		var settled_progress := float(driver_controller.get("grip_transfer_progress"))
		settled_progress_by_frame_rate.append(settled_progress)
		if settled_progress < 0.995:
			failures.append("Driver grip transfer does not settle within 0.8 seconds at " + str(frames_per_second) + " frames per second.")
	if settled_progress_by_frame_rate.max() - settled_progress_by_frame_rate.min() > 0.002:
		failures.append("Driver grip transfer response changes with frame rate.")
	for steering_direction in [-1.0, 1.0]:
		driver_controller.set("grip_transfer_progress", 0.0)
		driver_controller.set("grip_transfer_velocity", 0.0)
		steering_telemetry.effective_steering_amount = steering_direction
		steering_controller.call("update_steering_wheel_pose")
		for response_frame in range(30):
			driver_controller.call("update_driver_hand_targets", 1.0 / 60.0)
			if float(driver_controller.get("grip_transfer_progress")) >= 0.25:
				break
		var interrupted_progress := float(driver_controller.get("grip_transfer_progress"))
		var interrupted_velocity := float(driver_controller.get("grip_transfer_velocity"))
		steering_telemetry.effective_steering_amount = steering_direction * (60.0 + 120.0 * interrupted_progress) / 180.0
		steering_controller.call("update_steering_wheel_pose")
		driver_controller.call("update_driver_hand_targets", 0.0)
		if absf(float(driver_controller.get("grip_transfer_velocity")) - interrupted_velocity) > 0.00001:
			failures.append("Driver grip transfer discards velocity immediately when steering is corrected.")
		var maximum_progress_after_correction := interrupted_progress
		for braking_frame in range(30):
			driver_controller.call("update_driver_hand_targets", 1.0 / 60.0)
			maximum_progress_after_correction = maxf(maximum_progress_after_correction, float(driver_controller.get("grip_transfer_progress")))
		var braking_travel := maximum_progress_after_correction - interrupted_progress
		if braking_travel < 0.005 or braking_travel > 0.04:
			failures.append("Driver grip transfer does not brake smoothly within a short travel after correction.")
		if absf(float(driver_controller.get("grip_transfer_progress")) - interrupted_progress) > 0.001 or absf(float(driver_controller.get("grip_transfer_velocity"))) > 0.01:
			failures.append("Driver grip transfer does not settle at the corrected steering position.")
		steering_telemetry.effective_steering_amount = -steering_direction
		steering_controller.call("update_steering_wheel_pose")
		for reversal_frame in range(72):
			driver_controller.call("update_driver_hand_targets", 1.0 / 60.0)
		if float(driver_controller.get("grip_transfer_progress")) < 0.99 or float(driver_controller.get("grip_transfer_direction")) != -steering_direction:
			failures.append("Driver grip transfer does not recover when steering reverses during a transfer.")
	print("DRIVER_GRIP_PROGRESS_AFTER_0_8_SECONDS=" + str(settled_progress_by_frame_rate))
	driver_controller.set("grip_transfer_progress", 0.0)
	driver_controller.set("grip_transfer_velocity", 0.0)
	steering_telemetry.effective_steering_amount = 0.0
	steering_controller.call("update_steering_wheel_pose")
	driver_controller.call("update_driver_hand_targets", 0.0)
	driver_controller.set_process(true)

func validate_final_hand_transfer(steering_telemetry: SteeringTelemetryVehicle, steering_controller: Node) -> void:
	driver_controller.set_process(false)
	for steering_direction in [-1.0, 1.0]:
		driver_controller.set("grip_transfer_direction", steering_direction)
		driver_controller.set("grip_transfer_velocity", 0.0)
		for transfer_step in range(70, 101):
			var transfer_progress := float(transfer_step) / 100.0
			driver_controller.set("grip_transfer_progress", transfer_progress)
			steering_telemetry.effective_steering_amount = steering_direction * (60.0 + 120.0 * transfer_progress) / 180.0
			steering_controller.call("update_steering_wheel_pose")
			driver_controller.call("update_driver_hand_targets", 0.0)
			var arm_configurations: Array = driver_controller.get("arm_modifier").get("arm_configurations")
			if transfer_step >= 86:
				for arm_configuration in arm_configurations:
					if float(arm_configuration["finger_closure"]) < 0.999:
						failures.append("Driver final regrip does not finish before the last steering segment ends.")
						break
			var supporting_hand_index := 1 if steering_direction > 0.0 else 0
			if measure_hand_rim_error(arm_configurations[supporting_hand_index]) > 0.001:
				failures.append("Driver last hand transfer leaves its supporting hand outside the rim.")
	driver_controller.set("grip_transfer_progress", 0.0)
	driver_controller.set("grip_transfer_velocity", 0.0)
	steering_telemetry.effective_steering_amount = 0.0
	steering_controller.call("update_steering_wheel_pose")
	driver_controller.call("update_driver_hand_targets", 0.0)
	driver_controller.set_process(true)

func validate_fast_final_hand_transfer(steering_telemetry: SteeringTelemetryVehicle, steering_controller: Node) -> void:
	for steering_direction in [-1.0, 1.0]:
		steering_telemetry.effective_steering_amount = 0.0
		steering_controller.call("update_steering_wheel_pose")
		for settle_frame in range(120):
			await process_frame
		for steering_degrees in range(0, 145, 3):
			steering_telemetry.effective_steering_amount = steering_direction * float(steering_degrees) / 180.0
			steering_controller.call("update_steering_wheel_pose")
			await process_frame
		steering_telemetry.effective_steering_amount = steering_direction * 144.0 / 180.0
		steering_controller.call("update_steering_wheel_pose")
		for settle_frame in range(120):
			await process_frame
		var arm_configurations: Array = driver_controller.get("arm_modifier").get("arm_configurations")
		var leading_hand_index := 0 if steering_direction > 0.0 else 1
		var completed_transfer_frame := -1
		for transfer_frame in range(30):
			steering_telemetry.effective_steering_amount = steering_direction * minf(144.0 + 12.0 * float(transfer_frame + 1), 180.0) / 180.0
			steering_controller.call("update_steering_wheel_pose")
			await process_frame
			if completed_transfer_frame < 0 and float(driver_controller.get("grip_transfer_progress")) > 0.8 and float(arm_configurations[leading_hand_index]["finger_closure"]) > 0.999:
				completed_transfer_frame = transfer_frame + 1
		if completed_transfer_frame < 0 or completed_transfer_frame > 8:
			failures.append("Driver final support handoff takes longer than 0.134 seconds after steering to full lock.")
		print("DRIVER_FINAL_HANDOFF_COMPLETION_FRAMES=" + str(completed_transfer_frame))
		for steering_degrees in range(177, -1, -3):
			steering_telemetry.effective_steering_amount = steering_direction * float(steering_degrees) / 180.0
			steering_controller.call("update_steering_wheel_pose")
			await process_frame
	steering_telemetry.effective_steering_amount = 0.0
	steering_controller.call("update_steering_wheel_pose")

func validate_lock_to_lock_transitions(steering_telemetry: SteeringTelemetryVehicle, steering_controller: Node) -> void:
	for steering_degrees in range(0, 181, 3):
		steering_telemetry.effective_steering_amount = float(steering_degrees) / 180.0
		steering_controller.call("update_steering_wheel_pose")
		await process_frame
	for settle_frame in range(60):
		await process_frame
	var starting_direction := 1.0
	for transition_frame_count in [36, 36, 24, 24]:
		for transition_frame in range(1, transition_frame_count + 1):
			steering_telemetry.effective_steering_amount = lerpf(starting_direction, -starting_direction, float(transition_frame) / float(transition_frame_count))
			steering_controller.call("update_steering_wheel_pose")
			await process_frame
		for settle_frame in range(8):
			await process_frame
		if float(driver_controller.get("grip_transfer_direction")) != -starting_direction or float(driver_controller.get("grip_transfer_progress")) < 0.98:
			failures.append("Driver hands lag behind a complete steering reversal lasting " + str(float(transition_frame_count) / 60.0) + " seconds.")
		print("DRIVER_LOCK_TO_LOCK_SETTLED_PROGRESS=" + str([transition_frame_count, driver_controller.get("grip_transfer_direction"), driver_controller.get("grip_transfer_progress")]))
		starting_direction = -starting_direction
	for reversal_index in range(4):
		for transition_frame in range(1, 25):
			steering_telemetry.effective_steering_amount = lerpf(starting_direction, -starting_direction, float(transition_frame) / 24.0)
			steering_controller.call("update_steering_wheel_pose")
			await process_frame
		starting_direction = -starting_direction
	for steering_degrees in range(177, -1, -3):
		steering_telemetry.effective_steering_amount = starting_direction * float(steering_degrees) / 180.0
		steering_controller.call("update_steering_wheel_pose")
		await process_frame

func measure_hand_rim_error(arm_configuration: Dictionary) -> float:
	var target := arm_configuration["hand_target"] as Node3D
	var steering_pivot := driver_controller.get("steering_pivot") as Node3D
	var palm_position: Vector3 = steering_pivot.global_transform.affine_inverse() * (target.global_transform * arm_configuration["palm_offset"])
	var rim_direction := Vector2(signf(palm_position.x) * pow(absf(palm_position.x) / 0.132, 2.0), signf(palm_position.y) * pow(absf(palm_position.y) / 0.084, 2.0)).normalized()
	var contact_position := Vector3(signf(rim_direction.x) * sqrt(absf(rim_direction.x)) * 0.132, signf(rim_direction.y) * sqrt(absf(rim_direction.y)) * 0.084, -0.018)
	return palm_position.distance_to(contact_position)

func validate_arm_pose() -> void:
	modification_samples += 1
	var skeleton := driver_controller.get("driver_skeleton") as Skeleton3D
	var arm_configurations: Array = driver_controller.get("arm_modifier").get("arm_configurations")
	var wrists: Array[Vector3] = []
	var open_hand_count := 0
	var minimum_supporting_hand_rim_error_meters := INF
	for arm_configuration in arm_configurations:
		var shoulder_index := skeleton.find_bone(arm_configuration["root_bone_name"])
		var elbow_index := skeleton.find_bone(arm_configuration["middle_bone_name"])
		var wrist_index := skeleton.find_bone(arm_configuration["end_bone_name"])
		var shoulder := skeleton.get_bone_global_pose(shoulder_index).origin
		var collarbone_index := skeleton.get_bone_parent(shoulder_index)
		var collarbone_pose := skeleton.get_bone_global_pose(collarbone_index)
		if measure_rotation_steps:
			var hand_rotation := skeleton.get_bone_global_pose(wrist_index).basis.get_rotation_quaternion()
			var collarbone_rotation := collarbone_pose.basis.get_rotation_quaternion()
			if previous_bone_rotations.has(wrist_index):
				maximum_hand_rotation_step_degrees = maxf(maximum_hand_rotation_step_degrees, rad_to_deg(hand_rotation.angle_to(previous_bone_rotations[wrist_index])))
				maximum_shoulder_rotation_step_degrees = maxf(maximum_shoulder_rotation_step_degrees, rad_to_deg(collarbone_rotation.angle_to(previous_bone_rotations[collarbone_index])))
			previous_bone_rotations[wrist_index] = hand_rotation
			previous_bone_rotations[collarbone_index] = collarbone_rotation
		var collarbone_rest := skeleton.get_bone_global_rest(collarbone_index)
		var shoulder_rotation := collarbone_pose.basis * collarbone_rest.basis.inverse()
		maximum_shoulder_protraction_degrees = maxf(maximum_shoulder_protraction_degrees, rad_to_deg(shoulder_rotation.get_rotation_quaternion().get_angle()))
		maximum_shoulder_bone_length_error = maxf(maximum_shoulder_bone_length_error, absf(shoulder.distance_to(collarbone_pose.origin) - skeleton.get_bone_rest(shoulder_index).origin.length()))
		var elbow := skeleton.get_bone_global_pose(elbow_index).origin
		var wrist := skeleton.get_bone_global_pose(wrist_index).origin
		var target := arm_configuration["hand_target"] as Node3D
		maximum_elbow_flexion_degrees = maxf(maximum_elbow_flexion_degrees, rad_to_deg((elbow - shoulder).angle_to(wrist - elbow)))
		var hand_forward := skeleton.get_bone_global_pose(wrist_index).basis.y
		maximum_wrist_bend_degrees = maxf(maximum_wrist_bend_degrees, rad_to_deg(hand_forward.angle_to(wrist - elbow)))
		maximum_wrist_error = maxf(maximum_wrist_error, (skeleton.global_transform * wrist).distance_to(target.global_position))
		var upper_arm_length := skeleton.get_bone_rest(elbow_index).origin.length()
		var forearm_length := skeleton.get_bone_rest(wrist_index).origin.length()
		maximum_bone_length_error = maxf(maximum_bone_length_error, absf(shoulder.distance_to(elbow) - upper_arm_length))
		maximum_bone_length_error = maxf(maximum_bone_length_error, absf(elbow.distance_to(wrist) - forearm_length))
		wrists.append(skeleton.global_transform * wrist)
		var finger_closure: float = arm_configuration["finger_closure"]
		minimum_finger_closure = minf(minimum_finger_closure, finger_closure)
		if finger_closure < 0.95:
			open_hand_count += 1
		if finger_closure > 0.999:
			minimum_supporting_hand_rim_error_meters = minf(minimum_supporting_hand_rim_error_meters, measure_hand_rim_error(arm_configuration))
			var palm_world_position: Vector3 = target.global_transform * arm_configuration["palm_offset"]
			var steering_pivot := driver_controller.get("steering_pivot") as Node3D
			var inward_direction := steering_pivot.global_position - palm_world_position
			inward_direction -= steering_pivot.global_basis.z * inward_direction.dot(steering_pivot.global_basis.z)
			var palm_normal := skeleton.global_basis * skeleton.get_bone_global_pose(wrist_index).basis.z
			minimum_closed_palm_alignment = minf(minimum_closed_palm_alignment, palm_normal.normalized().dot(inward_direction.normalized()))
		for finger_chain in arm_configuration["finger_chains"]:
			if finger_chain["is_thumb"] and finger_closure > 0.999:
				var thumb_root_index: int = finger_chain["bone_indices"][0]
				var thumb_direction := skeleton.get_bone_global_pose(thumb_root_index).basis.y.normalized()
				var thumb_comparison_hand_direction := skeleton.get_bone_global_pose(wrist_index).basis.y.normalized()
				maximum_closed_thumb_extension_alignment = maxf(maximum_closed_thumb_extension_alignment, thumb_direction.dot(thumb_comparison_hand_direction))
			for bone_index in finger_chain["bone_indices"]:
				var parent_index := skeleton.get_bone_parent(bone_index)
				var actual_length := skeleton.get_bone_global_pose(bone_index).origin.distance_to(skeleton.get_bone_global_pose(parent_index).origin)
				maximum_finger_bone_length_error = maxf(maximum_finger_bone_length_error, absf(actual_length - skeleton.get_bone_rest(bone_index).origin.length()))
	minimum_hand_separation = minf(minimum_hand_separation, wrists[0].distance_to(wrists[1]))
	maximum_simultaneously_open_hands = maxi(maximum_simultaneously_open_hands, open_hand_count)
	if float(driver_controller.get("grip_transfer_progress")) >= 0.70:
		maximum_final_supporting_hand_rim_error_meters = maxf(maximum_final_supporting_hand_rim_error_meters, minimum_supporting_hand_rim_error_meters)
