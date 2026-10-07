extends SceneTree

const DEFAULT_SESSION_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const EXPECTED_SEVENTH_GEAR := 7
const UPSHIFT_REVOLUTIONS_PER_MINUTE := 16000.0
const MAXIMUM_PHYSICS_FRAMES := 3600

func _init() -> void:
	call_deferred("_run")

func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)

func _session_scene_path() -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--session="):
			return argument.trim_prefix("--session=")
	return DEFAULT_SESSION_SCENE_PATH

func _run() -> void:
	var failures: Array[String] = []
	var session_scene_path := _session_scene_path()
	var packed := load(session_scene_path) as PackedScene
	if packed == null:
		_fail("could not load %s" % session_scene_path, failures)
		quit(1)
		return
	var session := packed.instantiate()
	root.add_child(session)
	current_scene = session

	var controller_nodes := session.find_children("*", "VehicleRustInputController", true, false)
	if controller_nodes.is_empty():
		_fail("VehicleRustInputController was not found in the session", failures)
		quit(1)
		return
	var controller: Node = controller_nodes[0]
	controller.call("_resolve_max_gear")

	for _frame in 10:
		await process_frame

	var resolved_maximum_gear := int(controller.get("_max_gear"))
	if resolved_maximum_gear != EXPECTED_SEVENTH_GEAR:
		_fail("input controller resolved %d gears instead of %d" % [resolved_maximum_gear, EXPECTED_SEVENTH_GEAR], failures)

	var vehicle := session.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		_fail("VehicleRigidBody was not found in the session", failures)
		quit(1)
		return
	vehicle.set("enable_player_input", false)
	if vehicle.has_method("set_automatic_transmission"):
		vehicle.set("automatic_transmission", false)
	vehicle.call("set_throttle_amount", 1.0)
	vehicle.call("set_brake_amount", 0.0)

	var seventh_gear_reached := false
	for _frame in MAXIMUM_PHYSICS_FRAMES:
		vehicle.call("set_throttle_amount", 1.0)
		var current_gear := int(vehicle.call("get_current_gear"))
		if current_gear > 0 and current_gear < EXPECTED_SEVENTH_GEAR:
			var revolutions_per_minute := float(vehicle.call("get_motor_rpm"))
			if revolutions_per_minute >= UPSHIFT_REVOLUTIONS_PER_MINUTE:
				vehicle.call("set_gear_request", current_gear + 1)
		if current_gear == EXPECTED_SEVENTH_GEAR:
			seventh_gear_reached = true
			break
		await physics_frame

	if not seventh_gear_reached:
		_fail("seventh gear was not reached with the profile-driven input controller active", failures)
		vehicle.call("set_throttle_amount", 0.0)
		quit(1)
		return
	print("[PASS] seventh gear engaged at %.1f km/h" % float(vehicle.call("get_speed_kmh")))

	vehicle.call("set_gear_request", 8)
	for _frame in 10:
		await physics_frame
	if int(vehicle.call("get_current_gear")) != EXPECTED_SEVENTH_GEAR:
		_fail("an eighth-gear request changed the gear state", failures)

	var telemetry: Dictionary = vehicle.call("get_telemetry_snapshot")
	if int(telemetry.get("gear", -99)) != EXPECTED_SEVENTH_GEAR:
		_fail("telemetry snapshot reports gear %s" % str(telemetry.get("gear")), failures)

	var speed_gauge := session.find_child("SpeedGauge", true, false)
	if speed_gauge == null:
		_fail("SpeedGauge HUD node was not found", failures)
	elif str(speed_gauge.get("gear_label")) != str(EXPECTED_SEVENTH_GEAR):
		_fail("HUD gear label is '%s'" % str(speed_gauge.get("gear_label")), failures)

	vehicle.call("set_throttle_amount", 0.0)
	vehicle.call("set_brake_amount", 1.0)
	for _frame in 240:
		await physics_frame
	vehicle.call("set_brake_amount", 0.0)
	var downshift_safe_speed := float(vehicle.call("get_speed_kmh"))
	if downshift_safe_speed < 240.0:
		vehicle.call("set_gear_request", 6)
		for _frame in 60:
			await physics_frame
		if int(vehicle.call("get_current_gear")) != 6:
			_fail("seventh-to-sixth downshift did not engage at %.1f km/h" % downshift_safe_speed, failures)

	if failures.is_empty():
		print("=== F1 2030 V10 seven-gear integration capture: PASSED ===")
		quit(0)
	else:
		printerr("=== F1 2030 V10 seven-gear integration capture: FAILED (%d errors) ===" % failures.size())
		quit(1)
