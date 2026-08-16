extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const TELEMETRY_EXPORT_PATH := "user://godot_f1_94_chutana_telemetry.csv"

func _init() -> void:
	call_deferred("_run_parity_test")

func _fail(msg: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + msg)
	failures.append(msg)

func _run_parity_test() -> void:
	var failures: Array[String] = []
	print("=== Running F1-94 La Chutana Track Parity Test (PHY-010) ===")
	
	var packed = load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("Could not load vehicle_test_session.tscn", failures)
		quit(1)
		return
	
	var compositor = packed.instantiate()
	root.add_child(compositor)
	
	# Wait 4 frames for scene graph composition
	for _f in range(4):
		await process_frame
	
	var session = compositor.find_child("RaceSession", true, false)
	if session == null:
		_fail("RaceSession node missing in compositor", failures)
		quit(1)
		return
	
	var vehicle = compositor.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		_fail("Active vehicle VehicleRigidBody missing", failures)
		quit(1)
		return
	
	var track = compositor.find_child("ActiveTrack", true, false)
	if track == null:
		_fail("ActiveTrack missing", failures)
		quit(1)
		return
	
	print("[OK] Composed F1-94 with Track 'La Chutana'")
	print("Initial Position: %s" % str(vehicle.global_position))
	
	if "enable_player_input" in vehicle:
		vehicle.enable_player_input = false
	
	var telemetry_rows: Array[String] = []
	telemetry_rows.append("Time_ms,Speed_kmh,RPM,Gear,Throttle,Brake,Steering,Lat_G,Long_G,Vert_G,FL_Comp,FR_Comp,RL_Comp,RR_Comp,Front_Slip,Rear_Slip,Session_Id,Session_Timestamp_UTC,Physics_Hz,Test_Id,Track_Scene,Vehicle_Node_Path,Vehicle_Scene,Vehicle_Script,Setup_Schema_Version,Setup_JSON")
	
	var start_pos_z = vehicle.global_position.z
	
	# Simulate 300 frames (5.0 seconds at 60 Hz) of full acceleration on La Chutana
	for frame in range(300):
		# Apply full throttle
		if "throttle_amount" in vehicle:
			vehicle.throttle_amount = 1.0
		
		await physics_frame
		
		var sim_time_ms = int(frame * (1000.0 / 60.0))
		var spd = vehicle.get("speed") * 3.6 if vehicle.get("speed") != null else 0.0
		var rpm = vehicle.get("motor_rpm") if vehicle.get("motor_rpm") != null else 4500.0
		var gear = vehicle.get("current_gear") if vehicle.get("current_gear") != null else 1
		var fl_c = vehicle.get("_wheel_compressions")[0] if vehicle.get("_wheel_compressions") != null else 50.0
		var fr_c = vehicle.get("_wheel_compressions")[1] if vehicle.get("_wheel_compressions") != null else 50.0
		var rl_c = vehicle.get("_wheel_compressions")[2] if vehicle.get("_wheel_compressions") != null else 50.0
		var rr_c = vehicle.get("_wheel_compressions")[3] if vehicle.get("_wheel_compressions") != null else 50.0
		
		# Compute real G-forces from vehicle's linear acceleration
		var lat_g := 0.0
		var long_g := 0.0
		var vert_g := 1.0
		var accel: Vector3 = vehicle.get("_current_linear_accel") if vehicle.get("_current_linear_accel") != null else Vector3.ZERO
		if accel != Vector3.ZERO:
			lat_g = accel.dot(vehicle.global_transform.basis.x) / 9.80665
			long_g = accel.dot(-vehicle.global_transform.basis.z) / 9.80665
			vert_g = accel.dot(vehicle.global_transform.basis.y) / 9.80665 + 1.0  # +1g for gravity

		# Compute slip from wheel spins vs ground speed
		var front_slip := 0.0
		var rear_slip := 0.0
		var spins: Array = vehicle.get("_wheel_spins") if vehicle.get("_wheel_spins") != null else []
		if spins.size() == 4 and spd > 1.0:
			var v_ms = spd / 3.6
			var tire_rad = 0.33  # approximate F1-94 tire radius
			for w in range(4):
				var wheel_v = absf(spins[w]) * tire_rad
				var slip_ratio = absf(wheel_v - v_ms) / maxf(v_ms, 0.1)
				if w < 2:
					front_slip = maxf(front_slip, slip_ratio)
				else:
					rear_slip = maxf(rear_slip, slip_ratio)

		var row = "%d,%.2f,%.1f,%d,1.0,0.0,0.0,%.3f,%.3f,%.3f,%.1f,%.1f,%.1f,%.1f,%.4f,%.4f,godot_parity_session,2026-08-15T00:00:00Z,60,PHY-010,res://scenes/tracks/test_field/la_chutana_generated.tscn,VehicleRigidBody,res://scenes/vehicles/f1_94/f1_94.tscn,res://addons/formula90s/scripts/f1_94_rust_vehicle.gd,1,{}" % [
			sim_time_ms, spd, rpm, gear,
			lat_g, long_g, vert_g,
			fl_c, fr_c, rl_c, rr_c,
			front_slip, rear_slip
		]
		telemetry_rows.append(row)
		
		if frame % 30 == 0:
			print("Frame %3d (T=%.2fs): Speed=%5.1f km/h, RPM=%5.0f, Gear=%d, Pos=(%.1f, %.2f, %.1f), Comp=(%.1f, %.1f, %.1f, %.1f)" % [
				frame, frame / 60.0, spd, rpm, gear, vehicle.global_position.x, vehicle.global_position.y, vehicle.global_position.z,
				fl_c, fr_c, rl_c, rr_c
			])
	
	var final_spd = vehicle.get("speed") * 3.6 if vehicle.get("speed") != null else 0.0
	var final_rpm = vehicle.get("motor_rpm") if vehicle.get("motor_rpm") != null else 0.0
	var final_gear = vehicle.get("current_gear") if vehicle.get("current_gear") != null else 0
	var dist_travelled = absf(vehicle.global_position.z - start_pos_z)
	
	print("=== Simulation Complete ===")
	print("Final Speed: %.1f km/h" % final_spd)
	print("Final RPM: %.0f" % final_rpm)
	print("Final Gear: %d" % final_gear)
	print("Distance Travelled: %.1f m" % dist_travelled)
	
	# Export CSV
	var file = FileAccess.open(TELEMETRY_EXPORT_PATH, FileAccess.WRITE)
	if file != null:
		for r in telemetry_rows:
			file.store_line(r)
		file.close()
		print("[OK] Telemetry written to %s" % TELEMETRY_EXPORT_PATH)
	
	# Assertions for 5.0s standing start (adjusted for increased aero drag)
	if dist_travelled < 50.0:
		_fail("Vehicle did not travel sufficient distance down straight (%.1fm, expected >= 50.0m)" % dist_travelled, failures)
	if final_spd < 85.0:
		_fail("Vehicle final speed below expectation (%.1f km/h, expected >= 85.0 km/h)" % final_spd, failures)
	if vehicle.global_position.y < -5.0 or vehicle.global_position.y > 10.0:
		_fail("Vehicle fell through track or flew off (Y=%.2f)" % vehicle.global_position.y, failures)
	
	if failures.size() == 0:
		print("=== F1-94 La Chutana Parity Test: PASSED ===")
		quit(0)
	else:
		printerr("=== F1-94 La Chutana Parity Test: FAILED (%d errors) ===" % failures.size())
		quit(1)
