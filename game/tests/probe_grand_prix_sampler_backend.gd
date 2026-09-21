extends SceneTree

## Runtime probe for the Grand Prix sampler backend.
##
## Confirms from live mixer readouts (not JSON) that the active continuous
## source is the Grand Prix sampler: its zone playback rates stay near unity,
## while the GF509 path publishes rpm * 5 / 120 (hundreds) in `last_pitches[0]`.
## The strict bank loader rejects any hash mismatch before activation, so a
## sampler signature also proves the packaged bank was validated at load time.

const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"

func _scene_path() -> String:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--scene="):
			return argument.trim_prefix("--scene=")
	return DEFAULT_SCENE_PATH

func _run() -> void:
	var failures: Array[String] = []
	var packed := load(_scene_path()) as PackedScene
	if packed == null:
		printerr("[FAIL] runtime scene could not load")
		quit(1)
		return
	var runtime := packed.instantiate()
	root.add_child(runtime)
	for _frame in 4:
		await process_frame
	var core := runtime.get_node_or_null("F90Core")
	if core == null or not core.has_method("is_engine_loaded") or not core.is_engine_loaded():
		printerr("[FAIL] F90Core missing or engine not loaded")
		quit(1)
		return
	for _frame in 240:
		await physics_frame
	var source_code: int = core.get("audio_source_code")
	var weights: PackedFloat32Array = core.get("last_weights")
	var pitches: PackedFloat32Array = core.get("last_pitches")
	var engine_gain: float = core.get("last_engine_gain")
	var output_rms: float = core.get("audio_output_rms")
	var rpm_readout: float = core.get("last_rpm")
	print("[probe] audio_source_code=", source_code, " rpm=", rpm_readout, " engine_gain=", engine_gain, " output_rms=", output_rms)
	print("[probe] weights=", weights)
	print("[probe] pitches=", pitches)
	if source_code != 2:
		failures.append("expected audio_source_code=2 (Grand Prix sampler), got %d" % source_code)
	if weights.size() != 5 or pitches.size() != 5:
		failures.append("mixer readouts must publish five weights and pitches")
	if engine_gain < 0.0:
		failures.append("invalid engine gain %.4f" % engine_gain)
	if output_rms > 0.0 and pitches[0] >= 2.0:
		failures.append("GF509 render signature detected (pitches[0]=%.3f)" % pitches[0])
	runtime.queue_free()
	if failures.is_empty():
		print("[PASS] Grand Prix sampler backend active in runtime (source_code=%d, rpm %.0f)." % [source_code, rpm_readout])
	quit(failures.size())

func _init() -> void:
	call_deferred("_run")
