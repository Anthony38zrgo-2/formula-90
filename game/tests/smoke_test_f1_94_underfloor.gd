extends SceneTree

const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"
const PROBE_NAMES := [
	"UnderfloorFrontLeft",
	"UnderfloorFrontRight",
	"UnderfloorCenter",
	"DiffuserThroat",
	"DiffuserExit",
]

func _fail(message: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + message)
	failures.append(message)

func _run() -> void:
	var failures: Array[String] = []
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("Canonical F1-94 runtime scene could not load.", failures)
		quit(1)
		return

	var runtime := packed.instantiate()
	root.add_child(runtime)
	for _frame in 180:
		await physics_frame

	var vehicles := root.find_children("*", "F194RustVehicle", true, false)
	if vehicles.is_empty():
		_fail("F194RustVehicle is missing from the canonical runtime.", failures)
	else:
		var vehicle: Node = vehicles[0]
		for probe_name in PROBE_NAMES:
			var probe := vehicle.find_child(probe_name, true, false)
			if probe == null or not probe is RayCast3D:
				_fail("Underfloor probe %s is missing or is not a RayCast3D." % probe_name, failures)
		if not vehicle.has_method(&"get_underfloor_state_snapshot"):
			_fail("Underfloor telemetry snapshot is not exposed.", failures)
		else:
			var snapshot: Dictionary = vehicle.call(&"get_underfloor_state_snapshot")
			var clearances_value: Variant = snapshot.get("clearance_m", {})
			if not clearances_value is Dictionary or clearances_value.size() != 5:
				_fail("Underfloor telemetry does not contain five clearance channels.", failures)
			else:
				for probe_name in clearances_value:
					var clearance := float(clearances_value[probe_name])
					if not is_finite(clearance) or clearance < 0.0 or clearance > 0.3501:
						_fail("Invalid clearance for %s: %s" % [probe_name, clearance], failures)
			if int(snapshot.get("valid_mask", 0)) == 0:
				_fail("No underfloor ray detected the settled road surface.", failures)
			for key in ["minimum_clearance_m", "rake_rad", "roll_rad", "contact_confidence", "scrape_intensity", "audio_scrape_gain", "audio_scrape_pitch"]:
				if not is_finite(float(snapshot.get(key, NAN))):
					_fail("Underfloor telemetry field %s is not finite." % key, failures)

	runtime.queue_free()
	if failures.is_empty():
		print("[PASS] Five underfloor raycasts and synchronized Rust/audio telemetry are live.")
	quit(failures.size())

func _init() -> void:
	call_deferred("_run")
