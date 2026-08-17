extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const SETTLE_FRAMES := 180
const LAUNCH_FRAMES := 600

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] scene missing")
		quit(1)
		return
	var file := FileAccess.open("user://f1_94_clean_diff_ab_forensics.csv", FileAccess.WRITE)
	file.store_line("mode,frame,time_ms,forward_kmh,lateral_kmh,rpm,gear,engine_torque,clutch_engagement,lat_g,long_g,yaw_rate_rad_s,rear_slip_max,rl_comp,rr_comp,rl_spin,rr_spin,rl_slip,rr_slip,diff_preload,diff_mu")
	for mode in ["BASE", "GE170"]:
		var root_node = packed.instantiate()
		root.add_child(root_node)
		for _i in range(4):
			await process_frame
		var vehicle = root_node.find_child("VehicleRigidBody", true, false)
		if vehicle == null:
			printerr("[FAIL] VehicleRigidBody missing for %s" % mode)
			quit(1)
			return
		vehicle.enable_player_input = false
		vehicle.throttle_amount = 0.0
		for _i in range(SETTLE_FRAMES):
			await physics_frame
		if mode == "GE170":
			vehicle.set_rear_locking_differential_engage_torque(170.0)
			await physics_frame
		var max_lat: float = 0.0
		var max_rear: float = 0.0
		var max_yaw_rate: float = 0.0
		for frame in range(LAUNCH_FRAMES):
			vehicle.throttle_amount = 1.0
			await physics_frame
			var basis: Basis = vehicle.global_transform.basis
			var linear_velocity: Vector3 = vehicle.linear_velocity
			var forward_velocity: float = -basis.z.dot(linear_velocity)
			var lateral_velocity: float = basis.x.dot(linear_velocity)
			var slips = vehicle.get_wheel_slips()
			var spins = vehicle.get_wheel_spins()
			var comp = vehicle.get_wheel_compressions()
			var rear_slip: float = maxf(absf(float(slips[2])), absf(float(slips[3])))
			var lat_g: float = vehicle.lat_g
			var yaw_rate: float = vehicle.angular_velocity.y
			max_lat = maxf(max_lat, absf(lat_g))
			max_rear = maxf(max_rear, rear_slip)
			max_yaw_rate = maxf(max_yaw_rate, absf(yaw_rate))
			if frame % 5 == 0 or frame < 10:
				file.store_line("%s,%d,%d,%.6f,%.6f,%.1f,%d,%.6f,%.6f,%.6f,%.6f,%.6f,%.6f,%.3f,%.3f,%.6f,%.6f,%.6f,%.6f,%.3f,%.3f" % [
					mode, frame, int(frame * 1000.0 / Engine.physics_ticks_per_second),
					forward_velocity * 3.6, lateral_velocity * 3.6, vehicle.motor_rpm, vehicle.current_gear,
					float(vehicle.get("engine_torque")), float(vehicle.get("clutch_engagement")), lat_g, vehicle.long_g, yaw_rate,
					rear_slip, comp[2], comp[3], spins[2], spins[3], slips[2], slips[3],
					float(vehicle.get_diff_preload()), float(vehicle.get_diff_clutch_friction_coeff())])
		print("[OK] %s clean A/B: max_abs_lat_g=%.6f max_rear_slip=%.6f max_abs_yaw_rate=%.6f final_forward_kmh=%.3f diff=%.3f/%.3f" % [
				mode, max_lat, max_rear, max_yaw_rate,
				(-vehicle.global_transform.basis.z.dot(vehicle.linear_velocity)) * 3.6,
				float(vehicle.get_diff_preload()), float(vehicle.get_diff_clutch_friction_coeff())])
		root_node.queue_free()
		await process_frame
	file.close()
	quit(0)
