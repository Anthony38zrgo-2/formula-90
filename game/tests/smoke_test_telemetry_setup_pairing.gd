extends SceneTree

const TELEMETRY_DIR := "res://telemetry/"
const JORDAN_SCENE := "res://scenes/tracks/test_field/jordan_handling_test.tscn"
const TELEMETRY_MANAGER_SCRIPT := preload("res://addons/formula90s/scripts/telemetry_manager.gd")

var telemetry_manager: Node

func _init() -> void:
	call_deferred("_run")

func _run() -> void:
	var failures := 0
	var project_autoload = root.get_node_or_null("TelemetryManager")
	if project_autoload != null:
		project_autoload.enabled = false
		project_autoload._close_session()
	telemetry_manager = TELEMETRY_MANAGER_SCRIPT.new()
	root.add_child(telemetry_manager)
	telemetry_manager.enabled = true
	telemetry_manager.vehicle = null
	telemetry_manager._close_session()
	var before := _setup_files()

	var packed := load(JORDAN_SCENE) as PackedScene
	if packed == null:
		printerr("[FAIL] Jordan handling scene could not load.")
		quit(1)
		return
	var world := packed.instantiate()
	root.add_child(world)
	var vehicle := world.get_node_or_null("VehicleController/VehicleRigidBody") as Vehicle
	if vehicle == null:
		printerr("[FAIL] Jordan handling vehicle could not resolve.")
		world.queue_free()
		quit(1)
		return

	await create_timer(1.25).timeout
	if telemetry_manager.vehicle != vehicle:
		printerr("[FAIL] TelemetryManager did not resolve the active Jordan vehicle.")
		failures += 1
	await create_timer(0.15).timeout
	var first_session := _new_setup_file(before)
	if first_session.is_empty():
		printerr("[FAIL] TelemetryManager did not create a setup snapshot.")
		failures += 1
	elif not _validate_pair(first_session):
		printerr("[FAIL] First telemetry setup snapshot is incomplete or unpaired.")
		failures += 1

	var after_first := _setup_files()
	vehicle.brake_force_multiplier += 0.125
	await create_timer(0.15).timeout
	var second_session := _new_setup_file(after_first)
	if second_session.is_empty() or second_session == first_session:
		printerr("[FAIL] Physical setup change did not start a new telemetry session.")
		failures += 1
	elif not _validate_pair(second_session):
		printerr("[FAIL] Rotated telemetry setup snapshot is incomplete or unpaired.")
		failures += 1

	telemetry_manager._close_session()
	telemetry_manager.vehicle = null
	telemetry_manager.queue_free()
	world.queue_free()
	if failures == 0:
		print("[PASS] Telemetry CSV captures are paired with immutable runtime setup snapshots.")
	quit(failures)

func _setup_files() -> Dictionary:
	var files := {}
	var directory := DirAccess.open(TELEMETRY_DIR)
	if directory == null:
		return files
	for file_name in directory.get_files():
		if file_name.ends_with("_setup.json"):
			files[file_name] = true
	return files

func _new_setup_file(before: Dictionary) -> String:
	for file_name in _setup_files():
		if not before.has(file_name):
			return file_name
	return ""

func _validate_pair(setup_file_name: String) -> bool:
	var raw := FileAccess.get_file_as_string(TELEMETRY_DIR + setup_file_name)
	var snapshot = JSON.parse_string(raw)
	if not snapshot is Dictionary:
		return false
	var csv_file = snapshot.get("csv_file", "")
	if csv_file.is_empty() or not FileAccess.file_exists(TELEMETRY_DIR + csv_file):
		return false
	var setup = snapshot.get("setup", {})
	var provenance = snapshot.get("provenance", {})
	return snapshot.get("schema_version") == 1 and setup.has("chassis") and setup.has("tires_and_surfaces") and \
		setup.has("transmission") and provenance.has("vehicle_node_path") and snapshot.has("assists")
