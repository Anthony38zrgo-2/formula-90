extends SceneTree

## Issue A diagnostic: sustained audio production vs consumption.
##
## The F90Core node owns the AudioStreamGenerator pump in `_process`. This probe
## measures, for a sustained window, whether the pump produces at least the
## configured mix rate (no starvation), plus ring occupancy, ring skips
## (underruns) and per-pump DSP cost. It A/Bs `audio_pump_mode`:
##   0 = legacy fixed cap (kPumpBudgetFrames = 1024)
##   1 = elapsed-delta budget (rate * delta + bounded catch-up)
##
## Must run WINDOWED with a real audio driver: `create_audio_nodes()` bails out
## under `--headless` / the Dummy driver, so the pump never runs there.
##
## Usage:
##   godot --path game --script res://tests/audio_sustained_capture_f1_2030.gd -- \
##     --frames=600 --warmup=180 [--throttle=0.0] \
##     [--max-fps-list=0,30,20] [--modes=0,1] [--stall-ms=0] [--out=res://..]

const DEFAULT_SCENE_PATH := "res://scenes/runtime/vehicle_test_session.tscn"

func _init() -> void:
	call_deferred("_run")

func _argument(key: String, fallback: String) -> String:
	var prefix := key + "="
	for argument in OS.get_cmdline_user_args():
		if argument == key:
			return ""
		if argument.begins_with(prefix):
			return argument.trim_prefix(prefix)
	return fallback

func _flag(key: String) -> bool:
	for argument in OS.get_cmdline_user_args():
		if argument == key:
			return true
		if argument.begins_with(key + "="):
			var value := argument.trim_prefix(key + "=").to_lower()
			return value == "true" or value == "1"
	return false

func _int_list(value: String, fallback: Array) -> Array:
	if value.is_empty():
		return fallback
	var out: Array = []
	for token in value.split(","):
		var trimmed := token.strip_edges()
		if trimmed.is_valid_int():
			out.append(int(trimmed))
	return out if not out.is_empty() else fallback

func _avg(samples: Array) -> float:
	var total := 0.0
	for value in samples:
		total += value
	return total / maxf(float(samples.size()), 1.0)

func _percentile(sorted_values: Array, fraction: float) -> float:
	if sorted_values.is_empty():
		return 0.0
	var index := int(round(float(sorted_values.size() - 1) * fraction))
	return sorted_values[index]

## Run one sustained window for a given scheduler mode and frame-rate cap.
func _capture(core: Node, vehicle: Node, mode: int, max_fps: int, window_frames: int, throttle: float, stall_ms: int) -> Dictionary:
	Engine.max_fps = max_fps
	core.call("set_audio_pump_mode", mode)
	# Let the ring settle under the requested scheduler before measuring.
	for _frame in 120:
		await process_frame
	# Read the live generator skip counter before zeroing the other counters.
	var skips_before := int(core.call("get_audio_skips"))
	core.call("reset_audio_stats")
	var frame_ms: Array = []
	var start_usec := Time.get_ticks_usec()
	for frame_index in window_frames:
		if throttle > 0.0 and frame_index == 0 and vehicle.get("throttle_amount") != null:
			vehicle.set("throttle_amount", throttle)
		if stall_ms > 0 and frame_index == window_frames / 2:
			OS.delay_msec(stall_ms)
		var before_usec := Time.get_ticks_usec()
		await process_frame
		frame_ms.append(float(Time.get_ticks_usec() - before_usec) / 1000.0)
	var elapsed_s := maxf(float(Time.get_ticks_usec() - start_usec) / 1_000_000.0, 0.0001)
	var calls := int(core.call("get_audio_pump_calls"))
	var pushed := int(core.call("get_audio_frames_pushed"))
	var render_us := int(core.call("get_audio_render_usec_total"))
	var skips_after := int(core.call("get_audio_skips"))
	var mix_rate := int(core.call("get_audio_mix_rate"))
	frame_ms.sort()
	var produced_per_s := float(pushed) / elapsed_s
	return {
		"mode": mode,
		"max_fps": max_fps,
		"window_frames": window_frames,
		"elapsed_s": elapsed_s,
		"fps": float(window_frames) / elapsed_s,
		"mix_rate": mix_rate,
		"pump_calls": calls,
		"pump_per_s": float(calls) / elapsed_s,
		"samples_pushed": pushed,
		"produced_per_s": produced_per_s,
		"consumed_per_s": float(mix_rate),
		"deficit_per_s": float(mix_rate) - produced_per_s,
		"render_ms_avg": float(render_us) / 1000.0 / maxf(float(calls), 1.0),
		"max_available": int(core.call("get_audio_max_available")),
		"buffer_length": float(core.call("get_audio_buffer_length")),
		"skips_delta": skips_after - skips_before,
		"skips_total": skips_after,
		"frame_ms_p50": _percentile(frame_ms, 0.50),
		"frame_ms_p95": _percentile(frame_ms, 0.95),
		"frame_ms_max": frame_ms[frame_ms.size() - 1] if not frame_ms.is_empty() else 0.0,
	}

## Verify the runtime audio gate: disabling stops production, re-enabling
## resumes it without touching the immutable scheduler state.
func _toggle_check(core: Node) -> bool:
	core.call("set_audio_pump_mode", 1)
	core.set("enable_audio", true)
	for _frame in 45:
		await process_frame
	core.call("reset_audio_stats")
	for _frame in 60:
		await process_frame
	var calls_on := int(core.call("get_audio_pump_calls"))
	core.set("enable_audio", false)
	core.call("reset_audio_stats")
	for _frame in 60:
		await process_frame
	var calls_off := int(core.call("get_audio_pump_calls"))
	core.set("enable_audio", true)
	core.call("reset_audio_stats")
	for _frame in 60:
		await process_frame
	var calls_reenabled := int(core.call("get_audio_pump_calls"))
	var ok := calls_on > 0 and calls_off == 0 and calls_reenabled > 0
	print("[AUDCAP] toggle check: on=%d off=%d re-enabled=%d -> %s" % [
		calls_on, calls_off, calls_reenabled, ("PASS" if ok else "FAIL")])
	return ok

func _run() -> void:
	var scene_path := _argument("--scene", DEFAULT_SCENE_PATH)
	var packed := load(scene_path) as PackedScene
	if packed == null:
		printerr("[AUDCAP][FAIL] Could not load %s" % scene_path)
		quit(1)
		return
	var compositor := packed.instantiate()
	root.add_child(compositor)
	for _frame in 10:
		await process_frame

	var vehicle := compositor.find_child("VehicleRigidBody", true, false)
	if vehicle == null:
		printerr("[AUDCAP][FAIL] VehicleRigidBody not found")
		quit(1)
		return
	vehicle.set("enable_player_input", false)

	var core := compositor.get_node_or_null("F90Core")
	if core == null:
		printerr("[AUDCAP][FAIL] F90Core node not found")
		quit(1)
		return
	core.set("enable_audio", true)

	var driver := AudioServer.get_driver_name()
	print("[AUDCAP] audio driver=%s mix_rate=%d buffer=%.3f" % [
		driver, int(core.call("get_audio_mix_rate")), float(core.call("get_audio_buffer_length"))])
	if driver == "Dummy":
		printerr("[AUDCAP][FAIL] Dummy audio driver: run windowed without --headless")
		quit(2)
		return

	var warmup := int(_argument("--warmup", "180"))
	for _frame in warmup:
		await process_frame

	if _flag("--toggle-check"):
		await _toggle_check(core)

	var window_frames := int(_argument("--frames", "600"))
	var throttle := float(_argument("--throttle", "0.0"))
	var stall_ms := int(_argument("--stall-ms", "0"))
	var modes := _int_list(_argument("--modes", ""), [0, 1])
	var max_fps_list := _int_list(_argument("--max-fps-list", ""), [0, 30, 20])

	var results: Array = []
	for mode in modes:
		for max_fps in max_fps_list:
			var stats := await _capture(core, vehicle, mode, max_fps, window_frames, throttle, stall_ms)
			results.append(stats)
			print("[AUDCAP] mode=%d max_fps=%-3d fps=%.1f pump/s=%.1f produced/s=%.0f deficit/s=%.0f render_ms=%.3f max_avail=%d skips_delta=%d skips_total=%d frame_ms p50=%.2f p95=%.2f max=%.2f" % [
				stats["mode"], stats["max_fps"], stats["fps"], stats["pump_per_s"],
				stats["produced_per_s"], stats["deficit_per_s"], stats["render_ms_avg"],
				stats["max_available"], stats["skips_delta"], stats["skips_total"],
				stats["frame_ms_p50"], stats["frame_ms_p95"], stats["frame_ms_max"]])

	var out_path := _argument("--out", "")
	if not out_path.is_empty():
		var payload := {
			"scene": scene_path,
			"driver": driver,
			"throttle": throttle,
			"stall_ms": stall_ms,
			"results": results,
		}
		var file := FileAccess.open(out_path, FileAccess.WRITE)
		if file != null:
			file.store_string(JSON.stringify(payload, "  "))
			print("[AUDCAP] wrote %s" % out_path)

	compositor.queue_free()
	quit(0)
