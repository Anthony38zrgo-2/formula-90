extends SceneTree

const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const SERVICE_WAIT_FRAMES := 1600
const IMMOBILIZED_POSITION_TOLERANCE_M := 0.6

var _service_started_frame := -1
var _service_position := Vector3.ZERO


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	var failures: Array[String] = []
	var scene_path := DEFAULT_SCENE_PATH
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--scene="):
			scene_path = argument.trim_prefix("--scene=")
	var packed := load(scene_path) as PackedScene
	if packed == null:
		printerr("[FAIL] pit stop runtime scene could not load: " + scene_path)
		quit(1)
		return
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 8:
		await process_frame

	var race_session := compositor.find_child("RaceSession", true, false)
	var vehicle := compositor.find_child("VehicleRigidBody", true, false)
	if race_session == null:
		failures.append("RaceSession is missing")
	if vehicle == null:
		failures.append("VehicleRigidBody is missing")
	var pit_stop: PitStopController = race_session.pit_stop if race_session != null else null
	if pit_stop == null:
		failures.append("RaceSession does not expose a PitStopController")
	else:
		await _exercise_pit_stop(vehicle, pit_stop, failures)
	var panel := compositor.find_child("PitStopPanel", true, false)
	if panel == null:
		failures.append("PitStopPanel is missing from the HUD")
	elif not panel.visible:
		failures.append("PitStopPanel must be visible while the car is inside the pit lane")

	compositor.queue_free()
	if failures.is_empty():
		print("[PASS] Pit stop box, immobilization and service complete on Fuji 76-77.")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())


func _exercise_pit_stop(vehicle: Node, pit_stop: PitStopController, failures: Array[String]) -> void:
	if pit_stop.get_box_count() != 16:
		failures.append("Fuji must declare 16 pit boxes, got %d" % pit_stop.get_box_count())
	if pit_stop.get_assigned_box_index() != 0:
		failures.append("the default player box must be the first one, got %d" % pit_stop.get_assigned_box_index())
	if pit_stop.get_selection().get("fuel_laps") != 15:
		failures.append("the default fuel target must be 15 laps")
	var markers_root := pit_stop.get_node_or_null("PitBoxMarkers")
	if markers_root == null or markers_root.get_child_count() != 16:
		failures.append("the asphalt must show one marker per mapped box")
	elif (markers_root.get_child(0) as Node3D).global_position.distance_to(Vector3(-267.82, 0.02, 9.365)) > 0.05:
		failures.append("the first asphalt marker must follow the Fuji metadata")
	if not vehicle.has_method("set_fuel_kg") or not vehicle.has_method("replace_tires"):
		failures.append("the vehicle must expose set_fuel_kg and replace_tires")
		return
	vehicle.call("reset_vehicle", Vector3(-267.82, 0.35, 9.365), 0.0)
	if "enable_player_input" in vehicle:
		vehicle.set("enable_player_input", false)
	for _settle_frame in 3:
		await physics_frame
	var fuel_before: Dictionary = vehicle.call("get_fuel_state_snapshot")
	var remaining_before := float(fuel_before.get("remaining_kg", -1.0))
	if remaining_before < 0.0:
		failures.append("fuel snapshot is missing before the pit stop")
		return
	for frame in SERVICE_WAIT_FRAMES:
		if not pit_stop.in_pit_lane:
			failures.append("the pit lane strip must trigger on box entry")
			return
		if pit_stop.is_servicing() and _service_started_frame < 0:
			_service_started_frame = frame
			_service_position = (vehicle as Node3D).global_position
			if "enable_player_input" in vehicle and bool(vehicle.get("enable_player_input")):
				failures.append("the vehicle must be immobilized while servicing")
			if absf(float(vehicle.get("brake_amount"))) < 0.99:
				failures.append("the service must hold the brake")
		if _service_started_frame >= 0:
			var drift := (vehicle as Node3D).global_position.distance_to(_service_position)
			if drift > IMMOBILIZED_POSITION_TOLERANCE_M:
				failures.append("the vehicle drifted %.2f m during the service" % drift)
				return
			if not pit_stop.is_servicing():
				break
		await physics_frame
	if _service_started_frame < 0:
		failures.append("stopping inside the assigned box did not start the service")
		return
	var fuel_after: Dictionary = vehicle.call("get_fuel_state_snapshot")
	var remaining_after := float(fuel_after.get("remaining_kg", -1.0))
	var expected_target := pit_stop.get_fuel_target_kg()
	if absf(remaining_after - expected_target) > 0.05:
		failures.append("the tank must hold %.2f kg after the refuel, got %.2f" % [expected_target, remaining_after])
	if "enable_player_input" in vehicle and not bool(vehicle.get("enable_player_input")):
		failures.append("the vehicle must regain control after the service")
	if absf(float(vehicle.get("brake_amount"))) > 0.01:
		failures.append("the release must clear the brake")
	print("[PIT] fuel %.2f -> %.2f kg, service started at frame %d" % [remaining_before, remaining_after, _service_started_frame])
