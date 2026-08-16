extends SceneTree

const SCENE := preload("res://scenes/tests/vehicle_track_combinations/jordan_197_assembly_smoke_test.tscn")
const WHEEL_NAMES := [&"WheelFrontLeft", &"WheelFrontRight", &"WheelRearLeft", &"WheelRearRight"]
const SETTLE_FRAMES := 240
# Keep the car inside the 30 m-long assembly surface. A longer run conflates
# wheelspin with the inevitable loss of contact after leaving the test field.
const ACCELERATION_FRAMES := 180


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene_root := SCENE.instantiate()
	root.add_child(scene_root)
	var vehicle := scene_root.get_node("Jordan197/VehicleRigidBody") as Vehicle
	var requested_tcs_threshold := _requested_tcs_threshold()
	if requested_tcs_threshold >= 0.0:
		vehicle.traction_control_max_slip = requested_tcs_threshold

	for _frame in range(SETTLE_FRAMES):
		await physics_frame

	var summary := {
		"settled": _snapshot(vehicle),
		"max_speed_kmh": 0.0,
		"max_rpm": 0.0,
		"min_signed_rear_slip": 0.0,
		"max_abs_rear_slip": 0.0,
		"max_compression_mm": {"FL": 0.0, "FR": 0.0, "RL": 0.0, "RR": 0.0},
		"contact_loss_frames": {"FL": 0, "FR": 0, "RL": 0, "RR": 0},
		"minimum_up_dot": 1.0,
		"tcs_threshold": vehicle.traction_control_max_slip,
		"tcs_active_frames": 0,
	}

	Input.action_press("Throttle", 1.0)
	for _frame in range(ACCELERATION_FRAMES):
		await physics_frame
		_capture(vehicle, summary)
	Input.action_release("Throttle")

	summary["final"] = _snapshot(vehicle)
	print("JORDAN197_STRAIGHT_PROBE ", JSON.stringify(summary))
	scene_root.queue_free()
	await process_frame
	quit(0)


func _capture(vehicle: Vehicle, summary: Dictionary) -> void:
	summary["max_speed_kmh"] = maxf(summary["max_speed_kmh"], absf(vehicle.speed) * 3.6)
	summary["max_rpm"] = maxf(summary["max_rpm"], vehicle.motor_rpm)
	summary["minimum_up_dot"] = minf(summary["minimum_up_dot"], vehicle.global_transform.basis.y.normalized().dot(Vector3.UP))
	if vehicle.tcs_active:
		summary["tcs_active_frames"] += 1

	var rear_slips: Array[float] = []
	for index in WHEEL_NAMES.size():
		var wheel := vehicle.get_node(String(WHEEL_NAMES[index])) as Wheel
		var short_name: String = ["FL", "FR", "RL", "RR"][index]
		summary["max_compression_mm"][short_name] = maxf(
			summary["max_compression_mm"][short_name],
			wheel.previous_compression
		)
		if not wheel.is_colliding():
			summary["contact_loss_frames"][short_name] += 1
		if index >= 2:
			rear_slips.append(wheel.slip_vector.y)

	for slip in rear_slips:
		summary["min_signed_rear_slip"] = minf(summary["min_signed_rear_slip"], slip)
		summary["max_abs_rear_slip"] = maxf(summary["max_abs_rear_slip"], absf(slip))


func _snapshot(vehicle: Vehicle) -> Dictionary:
	var wheels := {}
	for wheel_name in WHEEL_NAMES:
		var wheel := vehicle.get_node(String(wheel_name)) as Wheel
		wheels[String(wheel_name)] = {
			"contact": wheel.is_colliding(),
			"compression_mm": wheel.previous_compression,
			"signed_longitudinal_slip": wheel.slip_vector.y,
			"spin_rad_s": wheel.spin,
		}
	return {
		"speed_kmh": absf(vehicle.speed) * 3.6,
		"rpm": vehicle.motor_rpm,
		"gear": vehicle.current_gear,
		"position": [vehicle.global_position.x, vehicle.global_position.y, vehicle.global_position.z],
		"up_dot": vehicle.global_transform.basis.y.normalized().dot(Vector3.UP),
		"wheels": wheels,
	}


func _requested_tcs_threshold() -> float:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--tcs-threshold="):
			return argument.trim_prefix("--tcs-threshold=").to_float()
	return -1.0
