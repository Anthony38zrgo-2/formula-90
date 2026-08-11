extends SceneTree

const SCENE := preload("res://scenes/tracks/test_field/jordan_191_handling_test.tscn")


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene_root := SCENE.instantiate()
	root.add_child(scene_root)
	await process_frame

	var vehicle := scene_root.get_node_or_null("Jordan191/VehicleRigidBody")
	var camera := scene_root.get_node_or_null("CameraRig") as Node
	var aids := scene_root.get_node_or_null("DrivingAids") as Node
	var tuner := scene_root.get_node_or_null("DebugHud/HandlingTuningPanel") as Node
	var failures: Array[String] = []
	if vehicle == null:
		failures.append("canonical Jordan191/VehicleRigidBody is missing")
	if camera == null:
		failures.append("CameraRig is missing")
	if aids == null:
		failures.append("DrivingAids is missing")
	if tuner == null:
		failures.append("HandlingTuningPanel is missing")
	else:
		if str(tuner.get("vehicle_path")) != "../Jordan191/VehicleRigidBody":
			failures.append("HandlingTuningPanel vehicle_path is invalid")
		var expected_tunables := [
			"front_brake_bias",
			"max_steering_angle",
			"max_torque",
			"motor_drag",
			"coefficient_of_drag",
			"frontal_area",
			"air_density",
		]
		if tuner.call("get_tunable_ids") != expected_tunables:
			failures.append("HandlingTuningPanel allowlist changed unexpectedly")
		var panel := tuner.get_node_or_null("LiveTuningPanel") as Control
		if panel == null:
			failures.append("HandlingTuningPanel UI was not created")
		else:
			var key_event := InputEventKey.new()
			key_event.keycode = KEY_F10
			key_event.pressed = true
			Input.parse_input_event(key_event)
			await process_frame
			if not panel.visible:
				failures.append("F10 did not open HandlingTuningPanel")
	if vehicle != null:
		for wheel_name in ["WheelFrontLeft", "WheelFrontRight", "WheelRearLeft", "WheelRearRight"]:
			if vehicle.get_node_or_null(wheel_name) == null:
				failures.append("missing wheel: %s" % wheel_name)
	scene_root.queue_free()
	if failures.is_empty():
		print("PASS: canonical Jordan 191 handling scene")
		quit(0)
	else:
		push_error("; ".join(failures))
		quit(1)
