extends SceneTree

# Canonical audio smoke. The runtime owns audio through F90Core; the old
# VehicleAudio scene is legacy and is intentionally not used here.
const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"

func _scene_path() -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--scene="):
			return argument.trim_prefix("--scene=")
	return DEFAULT_SCENE_PATH

func _fail(msg: String, failures: Array[String]) -> void:
	printerr("[FAIL] " + msg)
	failures.append(msg)

func _run() -> void:
	var failures: Array[String] = []
	var scene_path := _scene_path()
	var packed := load(scene_path) as PackedScene
	if packed == null:
		_fail("Runtime scene could not load: %s" % scene_path, failures)
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
		print("[PASS] F90Core audio integration initialized for %s and publishes mixer telemetry." % scene_path)
	quit(failures.size())

func _init() -> void:
	call_deferred("_run")
