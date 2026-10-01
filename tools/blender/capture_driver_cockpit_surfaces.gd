extends "res://capture_driver_seated_surface.gd"

func capture_seated_surfaces() -> void:
	var capture_arguments := OS.get_cmdline_user_args()
	var output_path := capture_arguments[0]
	var seated_position := Vector3(float(capture_arguments[1]), float(capture_arguments[2]), float(capture_arguments[3]))
	var world := Node3D.new()
	root.add_child(world)
	var chassis := (load("res://assets/chassis.glb") as PackedScene).instantiate() as Node3D
	world.add_child(chassis)
	var telemetry := SteeringTelemetryVehicle.new()
	world.add_child(telemetry)
	var steering_controller: Node = load("res://scripts/vehicle/steering_wheel_visual_controller.gd").new()
	steering_controller.set("vehicle", telemetry)
	steering_controller.set("chassis_visual", chassis)
	world.add_child(steering_controller)
	driver_controller = load("res://scripts/vehicle/driver_visual_controller.gd").new()
	driver_controller.set("driver_model", load("res://assets/driver.glb"))
	driver_controller.set("chassis_visual", chassis)
	driver_controller.set("steering_wheel_controller", steering_controller)
	driver_controller.set("seated_position", seated_position)
	world.add_child(driver_controller)
	var arm_modifier := driver_controller.get("arm_modifier") as SkeletonModifier3D
	arm_modifier.modification_processed.connect(capture_current_surface)
	for settle_frame in range(60):
		await process_frame
	for step_size in [1, 3]:
		var steering_steps := range(0, 181, step_size)
		steering_steps.append_array(range(180 - step_size, -181, -step_size))
		steering_steps.append_array(range(-180 + step_size, 1, step_size))
		for steering_step in steering_steps:
			current_steering_degrees = float(steering_step)
			telemetry.steering_amount = current_steering_degrees / 180.0
			steering_controller.call("update_steering_wheel_pose")
			capture_next_modification = true
			await arm_modifier.modification_processed
	for steering_degrees in [180.0, -180.0, 0.0]:
		current_steering_degrees = steering_degrees
		telemetry.steering_amount = steering_degrees / 180.0
		for settle_frame in range(300):
			capture_next_modification = settle_frame >= 270
			await arm_modifier.modification_processed
	await process_frame
	var output_file := FileAccess.open(output_path, FileAccess.WRITE)
	if output_file == null:
		quit(1)
		return
	output_file.store_string(JSON.stringify({"samples": captured_samples, "seated_position": seated_position, "steering_pivot_position": steering_controller.get("steering_wheel_pivot").position, "validation_sequences": {"steering_speeds_degrees_per_second": [60.0, 180.0], "maximum_steering_degrees": 180.0, "stationary_pose_settle_seconds": 4.5}}))
	output_file.close()
	print("COCKPIT_SURFACE_SAMPLES=" + str(captured_samples.size()))
	quit(0)

func capture_current_surface() -> void:
	if not capture_next_modification:
		return
	super.capture_current_surface()
	var skeleton := driver_controller.get("driver_skeleton") as Skeleton3D
	var arm_modifier := driver_controller.get("arm_modifier") as SkeletonModifier3D
	var arm_configurations: Array = arm_modifier.get("arm_configurations")
	var wrist_errors: Array[float] = []
	var finger_closures: Array[float] = []
	for arm_configuration in arm_configurations:
		var hand_bone_index := skeleton.find_bone(arm_configuration["end_bone_name"])
		var wrist_position := skeleton.global_transform * skeleton.get_bone_global_pose(hand_bone_index).origin
		var hand_target := arm_configuration["hand_target"] as Node3D
		wrist_errors.append(wrist_position.distance_to(hand_target.global_position))
		finger_closures.append(arm_configuration["finger_closure"])
	captured_samples[-1]["wrist_target_errors"] = wrist_errors
	captured_samples[-1]["finger_closures"] = finger_closures
	captured_samples[-1]["grip_transfer_progress"] = driver_controller.get("grip_transfer_progress")
