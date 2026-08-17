extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const FRAMES := 900
const TARGETS := [80.0, 130.0]

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		printerr("[FAIL] scene missing")
		quit(1)
		return
	var output := ["target_kmh,frame,time_ms,speed_kmh,rpm,gear,throttle,lat_g,long_g,rear_slip,rl_comp,rr_comp"]
	for target in TARGETS:
		var root_node = packed.instantiate()
		root.add_child(root_node)
		for _i in range(4):
			await process_frame
		var vehicle = root_node.find_child("VehicleRigidBody", true, false)
		if vehicle == null:
			printerr("[FAIL] VehicleRigidBody missing")
			quit(1)
			return
		vehicle.enable_player_input = false
		for frame in range(FRAMES):
			var throttle := 1.0 if vehicle.speed_kmh < target else 0.0
			vehicle.throttle_amount = throttle
			await physics_frame
			if frame % 10 == 0:
				var comp = vehicle.get_wheel_compressions()
				var slips = vehicle.get_wheel_slips()
				output.append("%.1f,%d,%d,%.3f,%.1f,%d,%.1f,%.5f,%.5f,%.5f,%.3f,%.3f" % [
					target, frame, int(frame * 1000.0 / Engine.physics_ticks_per_second),
					vehicle.speed_kmh, vehicle.motor_rpm, vehicle.current_gear, throttle,
					vehicle.lat_g, vehicle.long_g,
					maxf(absf(slips[2]), absf(slips[3])), comp[2], comp[3]])
		root_node.queue_free()
		await process_frame
	var file := FileAccess.open("user://godot_f1_94_aero_ab.csv", FileAccess.WRITE)
	if file != null:
		for line in output:
			file.store_line(line)
		file.close()
	print("[OK] aero A/B telemetry written to user://godot_f1_94_aero_ab.csv")
	quit(0)
