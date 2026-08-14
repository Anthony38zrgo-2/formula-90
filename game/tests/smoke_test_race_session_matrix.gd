extends SceneTree

const SESSION := preload("res://scenes/runtime/race_session.tscn")
const VEHICLES := {
	"jordan_191": preload("res://data/vehicles/jordan_191.tres"),
	"f1_94": preload("res://data/vehicles/f1_94.tres"),
}
const TRACKS := {
	"test_field": preload("res://data/tracks/test_field.tres"),
	"la_chutana": preload("res://data/tracks/la_chutana.tres"),
}

var failures: Array[String] = []

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	for track_id in TRACKS:
		for vehicle_id in VEHICLES:
			await _check_case(track_id, vehicle_id)
	if failures.is_empty():
		print("[PASS] RaceSession matrix: TestField/LaChutana x Jordan191/F1-94")
	else:
		for failure in failures:
			printerr("[FAIL] " + failure)
	quit(failures.size())

func _check_case(track_id: String, vehicle_id: String) -> void:
	var config := RaceSessionConfig.new()
	config.selected_track = TRACKS[track_id]
	config.selected_vehicle = VEHICLES[vehicle_id]
	var session := SESSION.instantiate() as RaceSession
	session.config = config
	root.add_child(session)
	for _frame in 8:
		await process_frame
	var label := "%s + %s" % [track_id, vehicle_id]
	if session.active_track == null or session.active_vehicle == null:
		failures.append(label + ": composition missing")
	else:
		var spawn := session.active_track.find_child("VehicleSpawn", true, false) as Marker3D
		if spawn == null:
			failures.append(label + ": VehicleSpawn missing")
		if session.active_vehicle_root.global_position.distance_to(spawn.global_position) > 0.001:
			failures.append(label + ": vehicle root not placed at VehicleSpawn")
		for wheel_name in ["WheelFrontLeft", "WheelFrontRight", "WheelRearLeft", "WheelRearRight"]:
			if session.active_vehicle.get_node_or_null(wheel_name) as RayCast3D == null:
				failures.append(label + ": missing " + wheel_name)
		if session.get_node_or_null("CameraRig/Camera3D") == null or session.driving_aids == null:
			failures.append(label + ": runtime systems missing")
	session.queue_free()
	await process_frame
