extends SceneTree

# Canonical audio smoke. The runtime owns audio through F90Core; the old
# VehicleAudio scene is legacy and is intentionally not used here.
const SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"

func _fail(msg: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + msg)
	failures.append(msg)

func _run() -> void:
	var failures: Array[String] = []
	var packed := load(SCENE_PATH) as PackedScene
	if packed == null:
		_fail("Canonical F1-94 runtime scene could not load.", failures)
		quit(1)
		return

	var runtime := packed.instantiate()
	root.add_child(runtime)
	for _frame in 4:
		await process_frame

	var core := runtime.get_node_or_null("F90Core")
	if core == null:
		_fail("F90Core node missing from canonical runtime.", failures)
	elif not core.has_method("is_engine_loaded") or not core.is_engine_loaded():
		_fail("F90Core failed to initialize; audio cannot run. Check GDExtension/Rust core ABI.", failures)
	else:
		# AudioStream playback is deliberately disabled by Godot in --headless.
		# The Rust audio mixer must nevertheless expose its frame readouts.
		for _frame in 180:
			await physics_frame
		var weights: PackedFloat32Array = core.get("last_weights")
		var pitches: PackedFloat32Array = core.get("last_pitches")
		if weights.size() != 5 or pitches.size() != 5:
			_fail("F90Core did not publish canonical audio mixer readouts.", failures)
		if core.get("last_engine_gain") < 0.0:
			_fail("F90Core published an invalid engine gain.", failures)

	runtime.queue_free()
	if failures.is_empty():
		print("[PASS] Canonical F90Core audio integration initialized and publishes mixer telemetry.")
	quit(failures.size())

func _init() -> void:
	call_deferred("_run")
