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
	
	vehicle.enable_player_input = false
	vehicle.throttle_amount = 1.0
	
	var telemetry_rows: Array[String] = []
	telemetry_rows.append("Time_ms,Speed_kmh,RPM,Gear,Throttle,Brake,Steering,Lat_G,Long_G,Vert_G,FL_Comp,FR_Comp,RL_Comp,RR_Comp,Front_Slip,Rear_Slip,Session_Id,Session_Timestamp_UTC,Physics_Hz,Test_Id,Track_Scene,Vehicle_Node_Path,Vehicle_Scene,Vehicle_Script,Setup_Schema_Version,Setup_JSON")
	
	var start_pos_z = vehicle.global_position.z
	
	# Simulate 600 frames (5.0 seconds at 60 Hz) of full acceleration on La Chutana
	for frame in range(600):
		vehicle.throttle_amount = 1.0
		await physics_frame
		
		var sim_time_ms = int(frame * (1000.0 / 60.0))
		var spd = vehicle.get("speed_kmh") if vehicle.get("speed_kmh") != null else 0.0
		var rpm = vehicle.get("motor_rpm") if vehicle.get("motor_rpm") != null else 4500.0
		var gear = vehicle.get("current_gear") if vehicle.get("current_gear") != null else 1
		var comp = vehicle.get_wheel_compressions() if vehicle.has_method("get_wheel_compressions") else [50.0, 50.0, 50.0, 50.0]
		var fl_c = comp[0] if comp.size() > 0 else 50.0
		var fr_c = comp[1] if comp.size() > 1 else 50.0
		var rl_c = comp[2] if comp.size() > 2 else 50.0
		var rr_c = comp[3] if comp.size() > 3 else 50.0
		
		# G-forces directly from vehicle telemetry
		var lat_g: float = vehicle.get("lat_g") if vehicle.get("lat_g") != null else 0.0
		var long_g: float = vehicle.get("long_g") if vehicle.get("long_g") != null else 0.0
		var vert_g: float = vehicle.get("vert_g") if vehicle.get("vert_g") != null else 1.0

		# Slip ratios from wheel slips
		var slips = vehicle.get_wheel_slips() if vehicle.has_method("get_wheel_slips") else [0.0, 0.0, 0.0, 0.0]
		var front_slip: float = maxf(absf(slips[0]), absf(slips[1])) if slips.size() >= 2 else 0.0
		var rear_slip: float = maxf(absf(slips[2]), absf(slips[3])) if slips.size() >= 4 else 0.0

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
	
	var final_spd = vehicle.get("speed_kmh") if vehicle.get("speed_kmh") != null else 0.0
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
