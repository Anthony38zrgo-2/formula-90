extends SceneTree

const SCENE := preload("res://scenes/tests/vehicle_track_combinations/jordan_197_handling_test.tscn")


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene_root := SCENE.instantiate()
	root.add_child(scene_root)
	await process_frame

	var controller := scene_root.get_node("Jordan197") as Node
	controller.set_script(null)

	var vehicle := scene_root.get_node("Jordan197/VehicleRigidBody") as RigidBody3D
	vehicle.can_sleep = false
	vehicle.sleeping = false

	var max_speed := 0.0
	var max_lat_g := 0.0
	var max_long_g := 0.0
	var standstill_bottom_out_frames := 0
	var transient_bottom_out_frames := 0
	var contact_frames := 0
	var previous_position := vehicle.global_position
	var previous_velocity := Vector3.ZERO

	for i in 1440:
		vehicle.throttle_input = 1.0
		vehicle.steering_input = 0.0
		vehicle.brake_input = 0.0
		await physics_frame
		var velocity: Vector3 = (vehicle.global_position - previous_position) * 120.0
		previous_position = vehicle.global_position
		var speed_mps := velocity.length()
		max_speed = maxf(max_speed, speed_mps * 3.6)
		var acceleration := (velocity - previous_velocity) * 120.0
		previous_velocity = velocity
		var lat_accel := absf(acceleration.x)
		var long_accel := absf(acceleration.z)
		max_lat_g = maxf(max_lat_g, lat_accel / 9.81)
		max_long_g = maxf(max_long_g, long_accel / 9.81)
		for wheel_name in ["WheelFrontLeft", "WheelFrontRight", "WheelRearLeft", "WheelRearRight"]:
			var wheel := vehicle.get_node(wheel_name)
			var spring_length: float = wheel.get("spring_length")
			var current_length: float = wheel.get("spring_current_length")
			var compression := (spring_length - current_length) * 1000.0
			if compression > 110.0:
				if speed_mps < 1.0:
					standstill_bottom_out_frames += 1
				else:
					transient_bottom_out_frames += 1
		if (vehicle.get_node("WheelRearLeft") as RayCast3D).is_colliding():
			contact_frames += 1

	var failures: Array[String] = []
	print("max_speed_kph=%.1f" % max_speed)
	print("max_lat_g=%.2f max_long_g=%.2f" % [max_lat_g, max_long_g])
	print("bottom_out_at_standstill=%d transient=%d" % [standstill_bottom_out_frames, transient_bottom_out_frames])
	print("rear_ray_contact_frames=%d/1440" % contact_frames)
	if max_speed < 10.0:
		failures.append("car did not move under full throttle: %.1f kph" % max_speed)
	if standstill_bottom_out_frames > 10:
		failures.append("suspension bottomed out at standstill in %d frames" % standstill_bottom_out_frames)
	if max_lat_g > 6.0 or max_long_g > 6.0:
		failures.append("G spikes: lat %.2f long %.2f" % [max_lat_g, max_long_g])

	scene_root.queue_free()
	if failures.is_empty():
		print("PASS: jordan_197 La Chutana scripted drive")
		quit(0)
	else:
		push_error("; ".join(failures))
		quit(1)
